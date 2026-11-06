// Verify the sealed experiment without network, private material or heavy cryptographic setup.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {ROOT, read, sha} from './run.mjs';
const result = path.join(ROOT, 'results/g11/c1');
const a = read(path.join(result, 'acceptance.json'));
assert.equal(a.status, 'PASS_G11_C1_BELENIOS_ORIGINAL_CLI');
let hashes = 0;
for (const dir of [path.join(ROOT, 'experiments/g11/baselines/belenios'), result]) {
  for (const line of fs.readFileSync(path.join(dir, 'SHA256SUMS'), 'utf8').trim().split('\n')) {
    const m = line.match(/^([a-f0-9]{64})  (.+)$/); assert.ok(m);
    const file = path.resolve(dir, m[2]); assert.ok(file.startsWith(dir + path.sep));
    assert.equal(sha(fs.readFileSync(file)), m[1], file); hashes++;
  }
}
assert.equal(sha(fs.readFileSync(path.join(ROOT, a.raw_manifest.path))), a.raw_manifest.sha256);
for (const [file, digest] of Object.entries(a.source_files)) assert.equal(sha(fs.readFileSync(path.join(ROOT, file))), digest, file);
for (const run of ['pilot-20261106-02', 'run-20261106-01'])
  execFileSync(process.execPath, ['scripts/g11/c1/check.mjs', path.join(ROOT, 'experiments/g11/baselines/belenios', run)], {cwd: ROOT});
execFileSync(process.execPath, ['scripts/g11/check.mjs'], {cwd: ROOT});
console.log(JSON.stringify({status: 'PASS_G11_C1_SEALED_ACCEPTANCE', verified_c1_hashes: hashes,
  source_files: Object.keys(a.source_files).length, elections: a.suite_elections, formal_elections: a.formal_elections,
  warmups: a.warmup_elections, public_only_audits: a.public_only_audits.length, historical_data_unchanged: true}));
