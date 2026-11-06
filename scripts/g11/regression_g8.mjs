// Run the unchanged historical audit in its original source revision.
// Its final whole-tree assertion predates later legitimate Cargo registrations.
import fs from 'node:fs';import os from 'node:os';import path from 'node:path';
import assert from 'node:assert/strict';import {execFileSync} from 'node:child_process';
import {ROOT,sha,assertHistoricalUnchanged} from './common.mjs';
const revision='4a2cc163cd8b0d805d4a582c6e0d91586fc2627e';
assertHistoricalUnchanged();
const work=fs.mkdtempSync(path.join(os.tmpdir(),'vv-g11-g8-historical-'));
const archive=execFileSync('git',['archive','--format=tar',revision],{cwd:ROOT,maxBuffer:128*1024*1024});
execFileSync('tar',['-xf','-','-C',work],{input:archive});
fs.symlinkSync(path.join(ROOT,'contracts/node_modules'),path.join(work,'contracts/node_modules'),'dir');
assert.equal(sha(fs.readFileSync(path.join(work,'scripts/g8d_audit.mjs'))),sha(fs.readFileSync(path.join(ROOT,'scripts/g8d_audit.mjs'))));
const output=execFileSync(process.execPath,['scripts/g8d_audit.mjs'],{cwd:work,encoding:'utf8',timeout:25000,
  env:{...process.env,GIT_DIR:path.join(ROOT,'.git'),GIT_WORK_TREE:work}});
process.stdout.write(output);assertHistoricalUnchanged();
console.log(JSON.stringify({status:'PASS_UNMODIFIED_G8D_HISTORICAL_SOURCE_AUDIT',revision,
  reason:'Original audit freezes its complete historical source tree. Current source evolution is separately checked against 5f170c5.',
  unchanged_audit_script:true,temporary_source:work,new_evm_transactions:0}));
