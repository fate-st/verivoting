import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import {createHash} from 'node:crypto';
import {execFileSync, spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';
export const ROOT = fileURLToPath(new URL('../../', import.meta.url));
export const BASE = '5f170c5a904d825a99c96fd45558a743f2c4e1ea';
export const sha = value => createHash('sha256').update(value).digest('hex');
export const json = p => JSON.parse(fs.readFileSync(p, 'utf8'));
export function command(file, args) {
  try {return {value: execFileSync(file, args, {cwd: ROOT, encoding: 'utf8', timeout: 15000}).trim(), error: null};}
  catch (error) {return {value: null, error: String(error.message).slice(0, 500)};}
}
export function environment(config) {
  const configs = execFileSync('git', ['ls-files', '-z', 'configs'], {cwd: ROOT, encoding: 'utf8'}).split('\0').filter(Boolean);
  const status = command('git', ['status', '--porcelain=v1', '--untracked-files=all']);
  const sources = [...configs, ...filesUnder(path.join(ROOT,'configs/g11')).map(p=>path.relative(ROOT,p)),
    ...filesUnder(path.join(ROOT,'scripts/g11')).map(p=>path.relative(ROOT,p)), 'experiments/g11/literature/evidence.json'];
  return {observed_at: new Date().toISOString(), client_date: '2026-11-06', client_timezone: 'Asia/Shanghai',
    base_commit: BASE, git_head: command('git', ['rev-parse', 'HEAD']), git_branch: command('git', ['branch', '--show-current']),
    git_status: status, git_dirty: status.value === null ? null : Boolean(status.value),
    cpu: os.cpus()[0]?.model ?? null, cpu_count: os.cpus().length, memory_bytes: os.totalmem(),
    os: {platform: os.platform(), release: os.release(), architecture: os.arch(), macos: command('sw_vers', [])},
    node: process.version, rust: command('rustc', ['+1.99.1', '-Vv']), cargo: command('cargo', ['+1.99.1', '-V']),
    solidity: {observed_compiler: command('node', ['--input-type=module', '-e', "import solc from './contracts/node_modules/solc/index.js';console.log(solc.version())"]), invoked_for_g11: false},
    evm: {source: 'contracts/scripts/g10c_winner_chain.mjs', intended_hardfork: 'Prague', invoked_for_g11: false},
    compile_profile: config.compile_profile, random_seed: config.workload.seed,
    hashes: Object.fromEntries([...new Set(sources)].filter(p => fs.existsSync(path.join(ROOT, p))).sort().map(p => [p, sha(fs.readFileSync(path.join(ROOT, p)))]))};
}
export function assertHistoricalUnchanged() {
  const allowed = p => /^(scripts\/g11\/|configs\/g11\/|experiments\/g11\/|results\/g11\/|docs\/G11)/.test(p);
  const changed = execFileSync('git', ['diff', '--name-only', BASE], {cwd: ROOT, encoding: 'utf8'}).trim().split('\n').filter(Boolean);
  for (const p of changed) if (!allowed(p)) throw Error(`Historical file changed: ${p}`);
  return {status: 'unchanged', base_commit: BASE, base_tree: command('git', ['rev-parse', `${BASE}^{tree}`]).value,
    policy: 'No chmod or rewrite of historical files. Git base and content hashes are the read-only guard.'};
}
export function runChild(file, args, {out, name, timeoutMs = 30000, cwd = ROOT} = {}) {
  return new Promise(resolve => {
    const started = performance.now(); const stdout = fs.openSync(path.join(out, `${name}.stdout.log`), 'wx');
    const stderr = fs.openSync(path.join(out, `${name}.stderr.log`), 'wx');
    let timedOut = false, error = null;
    const child = spawn(file, args, {cwd, stdio: ['ignore', stdout, stderr], detached: true});
    child.on('error', e => {error = e.message;});
    const timer = setTimeout(() => {timedOut = true; try {process.kill(-child.pid, 'SIGKILL');} catch {}}, timeoutMs);
    child.on('close', (exitCode, signal) => {clearTimeout(timer); fs.closeSync(stdout); fs.closeSync(stderr);
      resolve({command: [file, ...args], status: timedOut ? 'timeout' : exitCode === 0 ? 'passed' : 'failed',
        exit_code: exitCode, signal, error, timeout_ms: timeoutMs, harness_elapsed_ms: performance.now() - started});});
  });
}
export function filesUnder(dir) {
  return fs.readdirSync(dir, {withFileTypes: true}).sort((a,b) => a.name.localeCompare(b.name)).flatMap(e =>
    e.isDirectory() ? filesUnder(path.join(dir,e.name)) : [path.join(dir,e.name)]);
}
export function seal(dir) {
  const entries = filesUnder(dir).filter(p => path.basename(p) !== 'SHA256SUMS');
  fs.writeFileSync(path.join(dir,'SHA256SUMS'), entries.map(p => `${sha(fs.readFileSync(p))}  ${path.relative(dir,p)}`).join('\n')+'\n', {flag:'wx'});
}
