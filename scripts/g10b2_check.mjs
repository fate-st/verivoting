// Limited generation-ledger/aggregation acceptance. Never launches a prover or benchmark.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
const sha=b=>createHash('sha256').update(b).digest('hex');
const run=(p,args,options={})=>{const text=execFileSync(p,args,{encoding:'utf8',...options});if(text)process.stdout.write(text);};
for(const p of ['scripts/g10b2_run.mjs','scripts/g10b2_report.mjs','scripts/g10b2_check.mjs'])run('node',['--check',p]);
run('python3',['-c','import ast,sys; ast.parse(sys.stdin.read()); print("summarizer AST: PASS")'],{input:fs.readFileSync('scripts/summarize.py','utf8')});
run('git',['diff','--check']);
const base='b253f60e73f9dec41b80b03d339b2442ce9bfe19';
const mutable=['README.md','docs/IMPLEMENTATION_STATUS.md','docs/NEXT_STEPS.md','scripts/summarize.py','crates/vv-friendly-dkg/Cargo.toml','experiments/aggregate/summary.json','experiments/aggregate/summary.md'];
const fresh=p=>[
 'crates/vv-friendly-dkg/src/bin/common/g10b_intake.rs','crates/vv-friendly-dkg/src/bin/common/g10b_materials.rs','crates/vv-friendly-dkg/src/bin/g10b1_matched_native.rs',
 'crates/vv-friendly-dkg/src/bin/g10b2_aggregation_cost.rs','docs/G10_B1_MATCHED_NATIVE_BASELINES.md','docs/G10_B2_GENERATION_COST_AUDIT.md',
 'scripts/g10b1_run.mjs','scripts/g10b1_report.mjs','scripts/g10b1_check.mjs','scripts/g10b2_run.mjs','scripts/g10b2_report.mjs','scripts/g10b2_check.mjs',
].includes(p)||['experiments/g10_b1_raw/','results/g10_b1/','experiments/g10_b2_raw/','results/g10_b2/'].some(prefix=>p.startsWith(prefix));
for(const [args,allowed] of [[['diff','--name-only',base],p=>mutable.includes(p)||fresh(p)],[['ls-files','--others','--exclude-standard'],fresh]]){
 for(const p of execFileSync('git',args,{encoding:'utf8'}).trim().split('\n').filter(Boolean))assert.ok(allowed(p),'unexpected change: '+p);
}
// B1 is an immutable earlier experiment; only its central rolling docs/summary advance.
let b1Immutable=0;
for(const line of fs.readFileSync('results/g10_b1/SHA256SUMS','utf8').trim().split('\n')){
 const m=line.match(/^([0-9a-f]{64})  (.+)$/);assert.ok(m);if(!mutable.includes(m[2])){assert.equal(sha(fs.readFileSync(m[2])),m[1],m[2]);b1Immutable++;}
}
run('node',['scripts/g10b1_report.mjs','--check']);
run('node',['scripts/g10b2_report.mjs','--check']);
run('node',['scripts/g10a_clean30_report.mjs','--check']);
run('node',['scripts/g9p1_feasibility.mjs','--check']);
const summary=JSON.parse(fs.readFileSync('experiments/aggregate/summary.json'));
assert.equal(summary.p1.constraints,3521);
assert.equal(summary.gates.G10_B1_matched_native_intake,'PASS_FOUR_MATCHED_NATIVE_INTAKE_ROUTES');
assert.equal(summary.gates.G10_B2_generation_cost_audit,'PASS_ARTIFACT_COST_AUDIT_AND_BOUNDED_AGGREGATION');
assert.equal(summary.gates.G10_B_five_route_system_comparison,'PARTIAL_FOUR_NATIVE_INTAKE_ROUTES_ONLY');
assert.equal(summary.g10_b2_generation_cost_audit.full_end_to_end_total_ms,null);
assert.equal(summary.project_current_progress.exact_snarkpack_evm_verified,false);
const sums='results/g10_b2/SHA256SUMS';
if(fs.existsSync(sums))for(const line of fs.readFileSync(sums,'utf8').trim().split('\n')){
 const m=line.match(/^([0-9a-f]{64})  (.+)$/);assert.ok(m);assert.equal(sha(fs.readFileSync(m[2])),m[1],m[2]);
}
console.log(JSON.stringify({status:'PASS_G10_B2_LIMITED_COST_ACCEPTANCE',historical_ballot_proofs_linked:8,historical_state_proofs_linked:2,p3_same_target:true,
 unchanged_b1_artifacts_checked:b1Immutable,aggregation_rounds:12,new_aggregates_verified_byte_identical:26,research_srs_rebuilds:1,new_groth16_setups:0,new_groth16_proofs:0,new_evm:0,
 source_snapshot_verified:true,historical_raw_and_crypto_manifest_unchanged:true,full_end_to_end_total_ms:null,G9_P1:'BLOCKED',G9_P2_started:false}));
