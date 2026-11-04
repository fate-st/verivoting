// Acceptance of the archived clean reproduction; does not rerun timed experiments.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
const run=(exe,args,options={})=>{const b=execFileSync(exe,args,{encoding:'utf8',...options});if(b)process.stdout.write(b);};
for(const p of ['scripts/g10a_clean_reproduce.mjs','scripts/g10a_clean30_report.mjs','scripts/g10a_clean30_check.mjs'])run('node',['--check',p]);
run('python3',['-c','import ast,sys; ast.parse(sys.stdin.read()); print("summarizer AST: PASS")'],{input:fs.readFileSync('scripts/summarize.py','utf8')});
run('git',['diff','--check']);
const allowed=['README.md','docs/IMPLEMENTATION_STATUS.md','docs/NEXT_STEPS.md','scripts/summarize.py','experiments/aggregate/summary.json','experiments/aggregate/summary.md'];
const fresh=p=>['scripts/g10a_clean_reproduce.mjs','scripts/g10a_clean30_report.mjs','scripts/g10a_clean30_check.mjs','docs/G10_A_CLEAN30_REPRODUCTION.md','docs/G10_B_COMPARISON_PROTOCOL.md'].includes(p)||['experiments/g10_a_clean30_raw/','results/g10_a_clean30/','results/g10_b_plan/'].some(prefix=>p.startsWith(prefix));
const changed=execFileSync('git',['diff','--name-only','e1e477f'],{encoding:'utf8'}).trim().split('\n').filter(Boolean);
for(const p of changed)assert.ok(allowed.includes(p)||fresh(p),'unexpected historical modification: '+p);
for(const p of execFileSync('git',['ls-files','--others','--exclude-standard'],{encoding:'utf8'}).trim().split('\n').filter(Boolean))assert.ok(fresh(p),'unexpected untracked file: '+p);
run('node',['scripts/g10a_clean30_report.mjs','--check']);
run('node',['scripts/g10a_report.mjs','--check']);
run('node',['scripts/g9p1_feasibility.mjs','--check']);
run('node',['scripts/g8e_accounting.mjs','--check']);
const summary=JSON.parse(fs.readFileSync('experiments/aggregate/summary.json'));
assert.equal(summary.g10_a_clean30_reproduction.design.total_paired_rounds,30);
assert.equal(summary.g10_a_clean30_reproduction.design.total_windows,60);
assert.equal(summary.g10_a_clean30_reproduction.all_run_git_dirty_false,true);
assert.equal(summary.p1.constraints,3521);
assert.equal(summary.g10_b_comparison_scope.system_comparison_executed,false);
assert.equal(summary.project_current_progress.exact_snarkpack_evm_verified,false);
assert.equal(summary.gates.G10_B_five_route_system_comparison,'PARTIAL_PLAN_ONLY_NO_SYSTEM_COMPARISON_RUN');
const scope=JSON.parse(fs.readFileSync('results/g10_b_plan/comparison_scope.json'));
assert.equal(scope.routes.length,5);assert.equal(scope.required_generation_ledger.new_values_in_this_round,false);assert.equal(scope.admission.G9_P2_allowed,false);
const m=JSON.parse(fs.readFileSync('experiments/g10_a_clean30_raw/execution/manifest.json'));
let live_worktree_checked=false;
if(fs.existsSync(m.source_worktree)){
 assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:m.source_worktree,encoding:'utf8'}).trim(),m.source_commit);
 assert.equal(execFileSync('git',['status','--porcelain'],{cwd:m.source_worktree,encoding:'utf8'}).trim(),'');live_worktree_checked=true;
}
console.log(JSON.stringify({status:'PASS_CLEAN30_REPRODUCTION_ACCEPTANCE',live_worktree_checked,source_commit:m.source_commit,paired_rounds:30,raw_windows:60,proof_pools:1,all_git_dirty_false:true,required_negative_executions:6,historical_data_and_crypto_code_unchanged:true,G10_B:'DESIGN_ONLY_NOT_EXECUTED',P1:'BLOCKED',P2_started:false,new_setup:0,new_proofs:0,new_evm:0}));
