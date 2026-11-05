// Archive/current-source acceptance only; no benchmark, prover, setup or EVM run.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
const sha=b=>createHash('sha256').update(b).digest('hex');
const run=(p,args,options={})=>{const text=execFileSync(p,args,{encoding:'utf8',...options});if(text)process.stdout.write(text);};
for(const p of ['scripts/g10b1_run.mjs','scripts/g10b1_report.mjs','scripts/g10b1_check.mjs'])run('node',['--check',p]);
run('python3',['-c','import ast,sys; ast.parse(sys.stdin.read()); print("summarizer AST: PASS")'],{input:fs.readFileSync('scripts/summarize.py','utf8')});
run('git',['diff','--check']);
const base='b253f60e73f9dec41b80b03d339b2442ce9bfe19';
const mutable=['README.md','docs/IMPLEMENTATION_STATUS.md','docs/NEXT_STEPS.md','scripts/summarize.py','crates/vv-friendly-dkg/Cargo.toml','experiments/aggregate/summary.json','experiments/aggregate/summary.md'];
const fresh=p=>['crates/vv-friendly-dkg/src/bin/common/g10b_intake.rs','crates/vv-friendly-dkg/src/bin/common/g10b_materials.rs','crates/vv-friendly-dkg/src/bin/g10b1_matched_native.rs','docs/G10_B1_MATCHED_NATIVE_BASELINES.md','scripts/g10b1_run.mjs','scripts/g10b1_report.mjs','scripts/g10b1_check.mjs'].includes(p)||p.startsWith('experiments/g10_b1_raw/')||p.startsWith('results/g10_b1/');
for(const [args,allowed] of [[['diff','--name-only',base],p=>mutable.includes(p)||fresh(p)],[['ls-files','--others','--exclude-standard'],fresh]]){
 const files=execFileSync('git',args,{encoding:'utf8'}).trim().split('\n').filter(Boolean);for(const p of files)assert.ok(allowed(p),'unexpected historical/untracked change: '+p);
}
run('node',['scripts/g10b1_report.mjs','--check']);
run('node',['scripts/g10a_clean30_report.mjs','--check']);
// The original pilot checker deliberately binds the OLD Cargo.toml bytes.
// Adding a new bin changes only that manifest; validate the archived pilot
// against its pinned Git source, never rewrite history to match today's file.
const oldPilot=JSON.parse(fs.readFileSync('experiments/g10_a_raw/native_direct_fold_b4.json'));
for(const s of oldPilot.sources)assert.equal(sha(execFileSync('git',['show',base+':'+s.path])),s.sha256,s.path);
for(const p of ['experiments/g10_a_raw/native_direct_fold_b4.json','results/g10_a/summary.json','docs/G10_A_NATIVE_DIRECT_FOLD.md']){
 assert.equal(sha(fs.readFileSync(p)),sha(execFileSync('git',['show',base+':'+p])),p);
}
console.log('historical G10-A pilot: source/archive bytes PASS against pinned Git base (not current extended Cargo.toml)');
run('node',['scripts/g9p1_feasibility.mjs','--check']);
run('node',['scripts/g8e_accounting.mjs','--check']);
const summary=JSON.parse(fs.readFileSync('experiments/aggregate/summary.json'));
assert.equal(summary.p1.constraints,3521);
assert.equal(summary.gates.G10_B1_matched_native_intake,'PASS_FOUR_MATCHED_NATIVE_INTAKE_ROUTES');
assert.equal(summary.gates.G10_B_five_route_system_comparison,'PARTIAL_FOUR_NATIVE_INTAKE_ROUTES_ONLY');
assert.equal(summary.project_current_progress.exact_snarkpack_evm_verified,false);
const sums='results/g10_b1/SHA256SUMS';
if(fs.existsSync(sums))for(const line of fs.readFileSync(sums,'utf8').trim().split('\n')){
 const m=line.match(/^([0-9a-f]{64})  (.+)$/);assert.ok(m,'checksum syntax');assert.equal(sha(fs.readFileSync(m[2])),m[1],m[2]);
}
console.log(JSON.stringify({status:'PASS_G10_B1_LIMITED_NATIVE_INTAKE_ACCEPTANCE',routes:4,matched_rounds:12,windows:48,proof_pools:1,negative_cases:4,related_tests:4,source_snapshot_verified:true,source_was_dirty:true,historical_raw_and_manifest_unchanged:true,full_system_comparison:false,new_setup:0,new_proofs:0,new_aggregates:0,new_evm:0,G9_P1:'BLOCKED',G9_P2_started:false}));
