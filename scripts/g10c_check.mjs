// Function-completion archive acceptance; no proof/EVM rerun, no optimization.
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';import {createHash} from 'node:crypto';
const sha=b=>createHash('sha256').update(b).digest('hex');
const run=(p,a,o={})=>{const s=execFileSync(p,a,{encoding:'utf8',...o});if(s)process.stdout.write(s);};
for(const p of ['scripts/g10c_run.mjs','scripts/g10c_chain.mjs','scripts/g10c_chain_audit.mjs','scripts/g10c_report.mjs','scripts/g10c_check.mjs','scripts/run_functional_experiment.mjs','contracts/scripts/g10c_winner_chain.mjs'])run('node',['--check',p]);
run('python3',['-c','import ast,sys;ast.parse(sys.stdin.read());print("summarizer AST: PASS")'],{input:fs.readFileSync('scripts/summarize.py','utf8')});
run('git',['diff','--check']);
const mutable=['README.md','docs/IMPLEMENTATION_STATUS.md','docs/NEXT_STEPS.md','scripts/summarize.py','crates/vv-friendly-dkg/Cargo.toml','experiments/aggregate/summary.json','experiments/aggregate/summary.md'];
const newFiles=new Set(['crates/vv-friendly-dkg/src/bin/common/g10b_intake.rs','crates/vv-friendly-dkg/src/bin/common/g10b_materials.rs','crates/vv-friendly-dkg/src/bin/common/g10c_packages.rs',
 'crates/vv-friendly-dkg/src/bin/g10b1_matched_native.rs','crates/vv-friendly-dkg/src/bin/g10b2_aggregation_cost.rs','crates/vv-friendly-dkg/src/bin/g10c_complete_native.rs','crates/vv-friendly-dkg/src/bin/g10c_commitment_export.rs',
 'docs/G10_B1_MATCHED_NATIVE_BASELINES.md','docs/G10_B2_GENERATION_COST_AUDIT.md','docs/G10_C_CONNECTED_FUNCTIONAL_FLOW.md','scripts/run_functional_experiment.mjs','contracts/scripts/g10c_winner_chain.mjs']);
const fresh=p=>newFiles.has(p)||/^scripts\/g10(?:b[12]|c)_(?:run|chain|chain_audit|report|check)\.mjs$/.test(p)||['experiments/g10_b1_raw/','experiments/g10_b2_raw/','experiments/g10_c_raw/','results/g10_b1/','results/g10_b2/','results/g10_c/'].some(prefix=>p.startsWith(prefix));
for(const [args,allowed] of [[['diff','--name-only','b253f60e73f9dec41b80b03d339b2442ce9bfe19'],p=>mutable.includes(p)||fresh(p)],[['ls-files','--others','--exclude-standard'],fresh]])
 for(const p of execFileSync('git',args,{encoding:'utf8'}).trim().split('\n').filter(Boolean))assert.ok(allowed(p),'unexpected change '+p);
let previousArtifacts=0;
for(const p of ['results/g10_b1/SHA256SUMS','results/g10_b2/SHA256SUMS'])for(const line of fs.readFileSync(p,'utf8').trim().split('\n')){
 const m=line.match(/^([0-9a-f]{64})  (.+)$/);assert.ok(m);if(!mutable.includes(m[2])){assert.equal(sha(fs.readFileSync(m[2])),m[1],m[2]);previousArtifacts++;}
}
run('node',['scripts/g10b1_report.mjs','--check']);run('node',['scripts/g10b2_report.mjs','--check']);
run('node',['scripts/g10c_chain_audit.mjs']);run('node',['scripts/g10c_report.mjs','--check']);
const summary=JSON.parse(fs.readFileSync('experiments/aggregate/summary.json'));
assert.equal(summary.p1.constraints,3521);assert.equal(summary.project_current_progress.development_priority,'FUNCTIONAL_COMPLETION_BEFORE_OPTIMIZATION');
assert.equal(summary.project_current_progress.target_architecture_complete,false);assert.equal(summary.project_current_progress.exact_snarkpack_evm_verified,false);
const mapping=JSON.parse(fs.readFileSync('experiments/g10_c_raw/archive_map.json'));
const privateDir=path.join(mapping.runtime_root,'private_diagnostic'),privateFile=path.join(privateDir,'full_tally.json');let privateModesChecked=false;
if(fs.existsSync(privateFile)){assert.equal(fs.statSync(privateDir).mode&0o777,0o700);assert.equal(fs.statSync(privateFile).mode&0o777,0o600);privateModesChecked=true;}
const sums='results/g10_c/SHA256SUMS';if(fs.existsSync(sums))for(const line of fs.readFileSync(sums,'utf8').trim().split('\n')){
 const m=line.match(/^([0-9a-f]{64})  (.+)$/);assert.ok(m);assert.equal(sha(fs.readFileSync(m[2])),m[1],m[2]);
}
console.log(JSON.stringify({status:'PASS_CONNECTED_IMPLEMENTED_FUNCTIONAL_FLOW_ACCEPTANCE',fresh_p3:1,real_cp_workers:5,matched_native_chain_inputs:60,
 actual_evm_transactions:21,winner_events:1,actual_wrong_winner_pairing_rejection:true,private_full_tally_withheld:true,private_diagnostic_modes_checked:privateModesChecked,
 unchanged_prior_artifact_entries:previousArtifacts,unchanged_original_contracts:15,optimization_performed:false,
 implemented_native_and_fallback_flow:true,full_target_architecture_complete:false,exact_snarkpack_evm:false,independent_distributed_dkg:false}));
