// Read-only verification of the completed G11 framework and partial literature stage.
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {ROOT,BASE,filesUnder,json,sha,assertHistoricalUnchanged} from './common.mjs';
import {validateEvidence,DATA_PATH} from './literature.mjs';
const root=path.join(ROOT,'results/g11');
const summary=json(path.join(root,'final/acceptance.json'));
assertHistoricalUnchanged();
for(const file of filesUnder(path.join(ROOT,'scripts/g11')).filter(p=>p.endsWith('.mjs')))
  execFileSync(process.execPath,['--check',file]);
for(const file of filesUnder(path.join(ROOT,'scripts/g11')).filter(p=>p.endsWith('.py')))
  execFileSync('python3',['-c','import ast,sys;ast.parse(sys.stdin.read())'],{input:fs.readFileSync(file)});
const evidence=json(DATA_PATH);assert.equal(validateEvidence(evidence).length,66);
assert.equal(summary.base_commit,BASE);assert.equal(summary.source_evidence_sha256,sha(fs.readFileSync(DATA_PATH)));
const run=json(path.join(ROOT,summary.accepted_run,'runs.json'));
assert.ok(run.executions.every(e=>e.status==='passed'));assert.equal(run.results.length,6);
assert.ok(run.results.every(r=>Object.values(r.measurements).every(v=>v===null)));
assert.equal(summary.literature_status,'PARTIAL_FOUR_FULLTEXTS_ONE_ACCESS_GAP');
assert.equal(summary.external_protocol_runs,0);assert.equal(summary.new_setup,0);assert.equal(summary.new_proofs,0);assert.equal(summary.new_evm,0);
const data=json(path.join(ROOT,summary.accepted_run,'literature/evidence.json'));assert.deepEqual(data,evidence);
const papers=json(path.join(root,'source_access.json'));assert.equal(papers.sources.length,4);
for(const p of papers.sources)assert.equal(p.sha256,evidence.systems.find(s=>s.id===p.system).source_sha256);
let hashes=0;
for(const p of filesUnder(root).filter(p=>path.basename(p)==='SHA256SUMS')) {
  for(const line of fs.readFileSync(p,'utf8').trim().split('\n')) {
    const m=line.match(/^([0-9a-f]{64})  (.+)$/);assert.ok(m);
    const candidate=path.resolve(path.dirname(p),m[2]);
    const allowedRoot=p===path.join(root,'final/SHA256SUMS')?ROOT.replace(/\/$/,''):path.dirname(p);
    assert.ok(candidate.startsWith(allowedRoot+path.sep));
    assert.equal(sha(fs.readFileSync(candidate)),m[1],candidate);hashes++;
  }
}
for(const p of filesUnder(path.join(ROOT,'experiments/g11')))assert.ok(!/\.(pdf|pem|key)$|private_diagnostic/.test(p));
console.log(JSON.stringify({status:'PASS_G11_FRAMEWORK_AND_PARTIAL_LITERATURE_ACCEPTANCE',accepted_run:summary.accepted_run,
  passed_run_cases:6,attribute_rows:66,verified_hash_entries:hashes,original_experiments_unchanged:true,
  fulltext_external_sources:4,emami_fulltext_pending:true,external_protocol_runs:0,new_setup:0,new_proofs:0,new_evm:0,
  full_g11_complete:false,exact_snarkpack_evm_complete:false}));
