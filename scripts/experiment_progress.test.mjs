// Viewer unit fixtures only; no simulated benchmark results or crypto execution.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {spawn, execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {PLAN, findLatest, snapshot, render} from './experiment_progress.mjs';
const viewer = fileURLToPath(new URL('./experiment_progress.mjs', import.meta.url));
const fixtures = [];
function fixture(t, status = 'RUNNING', commands = []) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), 'vv-progress-unit-'));
  fixtures.push(base);
  const dir = path.join(base, 'unit-run'); fs.mkdirSync(path.join(dir, 'logs'), {recursive: true});
  const manifest = {schema: 'VV/STAGE2/EXECUTION/v1', status, started_utc: '2026-11-14T00:00:00Z', commands};
  fs.writeFileSync(path.join(dir, 'execution.json'), JSON.stringify(manifest));
  // Exact disposable test directory created above; never an experiment folder.
  t.after(() => fs.rmSync(base, {recursive: true, force: true}));
  return {base, dir, manifest};
}
test('latest uses start timestamp rather than directory name', t => {
  const f = fixture(t);
  const newer = path.join(f.base, 'aaa'); fs.mkdirSync(newer);
  fs.writeFileSync(path.join(newer, 'execution.json'), JSON.stringify({...f.manifest, started_utc:'2026-11-15T00:00:00Z'}));
  assert.equal(findLatest(f.base).dir, newer);
});
test('completed steps are counted without claiming whole-project completion', t => {
  const f = fixture(t, 'PASS_MATCHED_GENERATION_PILOT', PLAN.map(([name]) => ({name,code:0,elapsed_ms:5})));
  const v = snapshot(f); assert.equal(v.passed, 16); assert.equal(v.failed, 0);
  assert.match(render(v), /16\/16通过/); assert.match(render(v), /不代表整个项目完成/);
});
test('running log identifies pending receipt and actual proof log counts', t => {
  const f = fixture(t, 'RUNNING', PLAN.slice(0,12).map(([name]) => ({name,code:0,elapsed_ms:5})));
  fs.writeFileSync(path.join(f.dir, 'logs', 'ballot-prove.log'), 'fresh ballot 3/8: prove=1.0 ms\n');
  const v = snapshot(f); assert.equal(v.steps.find(s => s.name === 'ballot-prove').state, 'pending_receipt');
  assert.deepEqual(v.counts.ballot, {done:3,total:8}); assert.match(render(v), /未核实进程存活/);
});
test('null exit with SIGTERM is an interruption, never a pass', t => {
  const f = fixture(t, 'FAIL_OR_RESOURCE_BLOCKED', [{name:'state-prove',code:null,signal:'SIGTERM'}]);
  const v = snapshot(f); assert.equal(v.failed, 1); assert.equal(v.passed, 0);
  assert.match(render(v), /已中止/);
});
test('explicit run works outside current directory and does not change records', t => {
  const f = fixture(t, 'PASS_MATCHED_GENERATION_PILOT');
  const file = path.join(f.dir, 'execution.json'), before = fs.readFileSync(file);
  const text = execFileSync(process.execPath, [viewer, '--run', f.dir, '--json'], {cwd:os.tmpdir(),encoding:'utf8'});
  assert.equal(JSON.parse(text).dir, f.dir); assert.deepEqual(fs.readFileSync(file), before);
});
test('watch follows a newer run and Ctrl+C only exits the viewer', async t => {
  const f = fixture(t, 'PASS_MATCHED_GENERATION_PILOT');
  const child = spawn(process.execPath, [viewer,'--watch','--base',f.base,'--interval','0.05']);
  let text = ''; child.stdout.on('data', b => {text += b;});
  const closed = new Promise(resolve => child.on('close', (code,signal) => resolve({code,signal})));
  await new Promise((resolve,reject) => {
    const timeout = setTimeout(() => reject(new Error('viewer did not render first record')), 3000);
    const poll = setInterval(() => {if(text.includes(f.dir)) {clearTimeout(timeout);clearInterval(poll);resolve();}},20);
    t.after(() => {clearTimeout(timeout);clearInterval(poll);});
  });
  const next = path.join(f.base,'next');fs.mkdirSync(path.join(next,'logs'),{recursive:true});
  fs.writeFileSync(path.join(next,'execution.json'),JSON.stringify({...f.manifest,started_utc:'2026-11-15T00:00:00Z'}));
  await new Promise((resolve,reject) => {
    const timeout = setTimeout(() => reject(new Error('viewer did not follow newer record')), 3000);
    const poll = setInterval(() => {if(text.includes(next)) {clearTimeout(timeout);clearInterval(poll);resolve();}},20);
    t.after(() => {clearTimeout(timeout);clearInterval(poll);});
  });
  child.kill('SIGINT');
  assert.deepEqual(await closed,{code:0,signal:null}); assert.match(text,/没有停止或重新运行实验/);
});

test('EVM experiment has its own eight steps, without native proof-generation claims', t => {
  const f = fixture(t, 'PASS_PAIRED_EVM_FOLD_COMPARISON', ['build-rust-reference','fmt','clippy-reference','rust-reference','compile','point-conformance','direct-fold-vm','state-proof-vm'].map(name=>({name,code:0})));
  fs.writeFileSync(path.join(f.dir,'execution.json'),JSON.stringify({...f.manifest,schema:'VV/STAGE2/EVM-FOLD-EXECUTION/v1'}));
  const v=snapshot(f); assert.equal(v.passed,8); assert.equal(v.steps.length,8);
  assert.match(render(v),/8\/8通过/); assert.equal(v.steps.some(s=>s.name==='ballot-prove'),false);
  assert.equal(findLatest([f.base]).dir,f.dir);
});
