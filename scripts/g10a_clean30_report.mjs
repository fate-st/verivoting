// Derive clean-source descriptive evidence, never execute or replace a timing run.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
const mode=process.argv[2]??'--check';assert.ok(['--write','--check'].includes(mode));
const BASE='e1e477f94247dc8ee261400cfb2ec381e0f44fc3',dir='experiments/g10_a_clean30_raw';
const sha=b=>createHash('sha256').update(b).digest('hex'),load=p=>JSON.parse(fs.readFileSync(p));
const manifest=load(dir+'/execution/manifest.json');
assert.equal(manifest.source_commit,BASE);assert.equal(manifest.status,'PASS_CLEAN_E1E477F_THREE_PROCESSES_30_PAIRED_ROUNDS');
assert.equal(manifest.before.status,'');assert.equal(manifest.after.status,'');assert.equal(manifest.runs.length,3);
assert.equal(sha(fs.readFileSync('scripts/g10a_clean_reproduce.mjs')),manifest.driver_sha256);
assert.equal(manifest.commands.filter(c=>c.name.startsWith('benchmark-')).length,3);
for(const c of manifest.commands){assert.equal(c.exit_code,0,c.name);assert.equal(c.source_before.status,'');assert.equal(c.source_after.status,'');assert.equal(c.source_before.head,BASE);assert.equal(c.source_after.head,BASE);assert.ok(fs.existsSync(path.join(dir,c.log)));}
const runs=manifest.runs.map((entry,i)=>{
 assert.equal(entry.process,i+1);const p=path.join(dir,entry.path),b=fs.readFileSync(p);assert.equal(sha(b),entry.sha256,p);
 const r=JSON.parse(b);assert.equal(r.source_base_sha,BASE);assert.equal(r.git_dirty,false);assert.equal(r.status,'PASS_MATCHED_TWO_B4_NATIVE_RECEIVE_PATHS');assert.equal(r.binary_sha256,manifest.binary_sha256);
 assert.equal(r.method.paired_rounds,10);assert.equal(r.method.repetitions_per_window,8);assert.equal(r.windows.length,20);assert.equal(r.rayon_num_threads,'4');
 assert.equal(r.setup_count,0);assert.equal(r.proof_generation_count,0);assert.equal(r.aggregate_generation_count,0);assert.equal(r.evm_executed,false);assert.equal(r.gas,null);assert.equal(r.prove_ms,null);assert.equal(r.setup_ms,null);
 assert.ok(r.both_paths_final_ct2_equal);assert.equal(r.necessary_negatives.length,2);assert.ok(r.necessary_negatives.every(n=>n.snarkpack_verified&&n.route_specific_check_rejected&&n.checkpoint_unchanged&&!n.frozen_short_circuit));
 for(const s of r.sources)assert.equal(sha(execFileSync('git',['show',BASE+':'+s.path])),s.sha256,s.path);
 for(const s of r.fixture_pins.filter(s=>s.path))assert.equal(sha(fs.readFileSync(s.path)),s.sha256,s.path);
 for(let round=1;round<=10;round++){
   const w=r.windows.filter(w=>w.round===round);assert.deepEqual(w.map(w=>w.route),round%2?['snarkpack_direct_fold','snarkpack_state_proof']:['snarkpack_state_proof','snarkpack_direct_fold']);
   assert.deepEqual(w.map(w=>w.position),[1,2]);
   for(const x of w){assert.equal(x.repetitions,8);assert.equal(x.two_batch_flow_ms,x.window_ms/8);assert.ok(Number.isFinite(x.two_batch_flow_ms)&&x.two_batch_flow_ms>0);}
 }
 return r;
});
for(const r of runs){assert.deepEqual(r.final_state,runs[0].final_state);assert.deepEqual(r.fixture_pins,runs[0].fixture_pins);assert.deepEqual(r.sources,runs[0].sources);}
const quantile=(a,p)=>{a=[...a].sort((x,y)=>x-y);const i=(a.length-1)*p;return a[Math.floor(i)]+(a[Math.ceil(i)]-a[Math.floor(i)])*(i%1);};
const stats=a=>({n:a.length,median:quantile(a,.5),q1:quantile(a,.25),q3:quantile(a,.75),min:Math.min(...a),max:Math.max(...a)});
const routeStats=rs=>Object.fromEntries(['snarkpack_direct_fold','snarkpack_state_proof'].map(route=>[route,Object.fromEntries(['two_batch_flow_ms','per_vote_ms','votes_per_second','packing_ms_per_flow','snarkpack_ms_per_flow','route_specific_ms_per_flow'].map(k=>[k,stats(rs.flatMap(r=>r.windows.filter(w=>w.route===route).map(w=>w[k])))]))]));
const ratios=runs.flatMap(r=>Array.from({length:10},(_,i)=>{const w=r.windows.filter(w=>w.round===i+1);return w.find(w=>w.route==='snarkpack_state_proof').two_batch_flow_ms/w.find(w=>w.route==='snarkpack_direct_fold').two_batch_flow_ms;}));
const state=load('fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json');
assert.deepEqual(runs[0].final_state,state.final_state.map(c=>({c0_hex:c.c0_hex,c1_hex:c.c1_hex})));
const summary={schema:'VV/G10-A/CLEAN30-SUMMARY/v1',status:manifest.status,source_commit:BASE,all_run_git_dirty_false:true,driver_sha256:manifest.driver_sha256,binary_sha256:manifest.binary_sha256,
 source_manifest:{path:dir+'/execution/manifest.json',sha256:sha(fs.readFileSync(dir+'/execution/manifest.json'))},raw_runs:manifest.runs.map(r=>({...r,path:dir+'/'+r.path})),
 design:{independent_processes:3,paired_rounds_per_process:10,total_paired_rounds:30,total_windows:60,repetitions_per_window:8,total_timed_two_batch_flows:480,distinct_proof_pools:1,distinct_ballots:8,independent_elections:1,one_process_is_not_one_independent_election:true,all_samples_retained:true,old_dirty_pilot_not_pooled:true,window_clustering_by_process:true},
 pooled_routes:routeStats(runs),paired_state_over_direct:stats(ratios),state_slower_window_pairs:ratios.filter(x=>x>1).length,
 per_process:runs.map((r,i)=>({process:i+1,routes:routeStats([r]),peak_rss_mib:r.process_peak_rss_mib,started_utc:r.started_utc})),
 memory_scope:'shared cumulative process including both routes/materials; no per-route RSS or prover-memory claim',fixed_proof_sizes:runs[0].communication_components,
 setup_ms:null,prove_ms:null,aggregation_ms:null,generation_cost_status:'REUSED_NOT_RETIMED_NOT_ZERO',new_setup:0,new_proofs:0,new_aggregates:0,new_evm:0,gas:null,calldata:null,
 two_required_negative_cases_per_process:true,negative_executions:6,matched_final_ct2:true,complete_system_comparison:false,complete_privacy_proof:false,exact_snarkpack_evm_verified:false};
const pin=p=>({path:p,sha256:sha(fs.readFileSync(p))});
const g2dPath='experiments/g2_d_raw/state_transition_d5w32_b4_groth16.json',g2ePath='experiments/g2_e_raw/multibatch_state_b4.json',g6Path='experiments/g6_raw/native_winner_only.json',g8ePath='results/g8_e/lifecycle_costs.json';
const g2d=load(g2dPath),g2e=load(g2ePath),g6=load(g6Path),g8e=load(g8ePath);
assert.equal(g2d.proof_sha256,state.batches[0].state_proof_sha256);assert.deepEqual(g2e.final_state,state.final_state);
const scope={schema:'VV/G10-B/COMPARISON-SCOPE/v1',status:'DESIGN_ONLY_NO_FIVE_BASELINE_SYSTEM_RUN',source_commit:BASE,not_a_cryptographic_protocol_manifest:true,
 frozen_inputs:[pin('configs/protocol_manifest.JUBJUB-EVAL-D5-W32.json'),pin('fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json'),pin('fixtures/snarkpack_g1d2_depth5_batch32_public_v1.json')],
 params:{W:32,D:5,candidates:3,distinct_ballots:8,batches:2,batch_size:4,trustees:5,threshold:3,winning_rule:'highest-weight-smallest-id-on-tie'},
 accepted_roots:state.batches.map(b=>b.batch_root_hex),state_vk_sha256:state.state_vk_sha256,target_state_json_sha256:sha(Buffer.from(JSON.stringify(state.final_state))),target_hash_scope:'artifact JSON-byte identity only, not a new protocol CT/CP hash',
 routes:[
 {id:'legacy_individual',environment:'native_first_then_separate_EVM_implementation',unit:'8 authenticated individual ballot accept/update operations',verification:'8 real Groth16 proofs',fold:'one-ballot direct fold per acceptance',status:'NOT_IMPLEMENTED_MATCHED_ROUTE'},
 {id:'transaction_batch_individual',environment:'native_first_then_separate_EVM_implementation',unit:'2 authenticated B4 accept/update operations',verification:'8 real Groth16 proofs, no aggregate',fold:'B4 direct fold',status:'NOT_IMPLEMENTED_MATCHED_ROUTE'},
 {id:'snarkpack_direct_fold',environment:'native',unit:'2 authenticated B4 accept/update operations',verification:'2 real SnarkPack V2',fold:'B4 direct fold',status:'MEASURED_INTAKE_ONLY_CLEAN30'},
 {id:'snarkpack_state_proof',environment:'native',unit:'2 authenticated B4 accept/update operations',verification:'2 real SnarkPack V2 + 2 real state proofs',fold:'state-proof verification',status:'MEASURED_INTAKE_ONLY_CLEAN30'},
 {id:'g8_individual_fallback',environment:'local_Prague_EVM',unit:'certificate/init + 2 B4 + freeze + commitments + P3 + winner write-once',verification:'individual Groth16 fallback, NOT exact SnarkPack EVM',status:'ARCHIVED_DIFFERENT_LIFECYCLE_SCOPE_NOT_A_MATCHED_FIVE_ROUTE_RUN'}],
 panels:{A:'Matched intake only: identical source ballots/order, root authorization, context, key, fresh nullifiers, CT0->CT1->CT2, same checks and state writes. Both legacy and transaction-batch routes must be real implementations, not renamed component timing.',B:'Complete lifecycle: add the same certificate/freeze/trustee/threshold/P3 or Full-Tally suffix to every eligible route. Never rank native milliseconds against EVM Gas as if same workload.'},
 required_generation_ledger:{ballot_setup:'OFFLINE_SEPARATE',ballot_proving:'PENDING_MATCHED_MEASUREMENT',aggregation:'PENDING_MATCHED_MEASUREMENT',state_setup:'OFFLINE_SEPARATE',state_proving:'PENDING_MATCHED_MEASUREMENT',P3_setup:'OFFLINE_SEPARATE',P3_proving:'PENDING_MATCHED_MEASUREMENT',new_values_in_this_round:false},
 historical_references_only:{not_added_to_clean30_receive_times:true,not_current_generation_comparison:true,
  state_batch1:{source:pin(g2dPath),prove_ms:g2d.prove_ms,setup_ms:g2d.setup_ms,proof_sha256:g2d.proof_sha256},
  state_batch2:{source:pin(g2ePath),prove_ms:g2e.new_second_prove_ms,setup_rebuild_ms:g2e.setup_rebuild_ms},
  winner_P3:{source:pin(g6Path),prove_ms:g6.prove_ms,setup_rebuild_ms:g6.setup_rebuild_ms},
  fallback_EVM:{source:pin(g8ePath),successful_gas_including_deployment:g8e.totals.successful_including_deployment.total_gas,successful_non_deployment_gas:g8e.totals.non_deployment_success.total_gas,gas_is_archived_receipts_not_new_measurement:true}},
 admission:{G10_B_first_two_native_routes:'implement a shared accepted-record authentication contract before timing; correct per-ballot legacy writes vs B4 atomic writes, same CT2/NF set',exact_snarkpack_EVM:false,G9_P2_allowed:false,Full_Tally_vs_Winner_only:'same CT2/DKG/shares but isolated release branches; full-tally outputs must not contaminate winner-only public fixture',production_privacy_claim:false},
 missing:['two real individual-proof intake routes','matched generation-time and route-specific verifier/prover memory','matched full lifecycle and Full-Tally comparison','exact SnarkPack EVM admission and fair chain comparison','independent proof pools and final clean-source full-system reproduction']};
const f=x=>x.toFixed(6),d=summary.pooled_routes.snarkpack_direct_fold.two_batch_flow_ms,s=summary.pooled_routes.snarkpack_state_proof.two_batch_flow_ms;
const md=`# G10-A：干净 e1e477f 的30轮配对复现\n\n状态：${summary.status}。本轮没有修改原Rust入口、原证明池、Manifest或密码学关系。\n\n## 实测\n\n| 路线 | 两批B4完整接收中位数 ms | Q1 / Q3 ms | 每票接收中位数 ms |\n| --- | ---: | ---: | ---: |\n| SnarkPack + Direct Fold | ${f(d.median)} | ${f(d.q1)} / ${f(d.q3)} | ${f(summary.pooled_routes.snarkpack_direct_fold.per_vote_ms.median)} |\n| SnarkPack + State Proof | ${f(s.median)} | ${f(s.q1)} / ${f(s.q3)} | ${f(summary.pooled_routes.snarkpack_state_proof.per_vote_ms.median)} |\n\n配对窗口State/Direct比值中位数 ${f(summary.paired_state_over_direct.median)}，本轮${summary.state_slower_window_pairs}/30对窗口中State路线更慢。\n结论限于当前固定8票/W32/B4和验证者负担，不支持完整系统优势，也不能据此判定状态证明在所有部署中无价值。\n\n## 干净源码及采样单位\n\n- 从完整提交 ${BASE} 建立独立detached worktree。构建、格式、严格Clippy、6项相关测试及每个运行前后均检查HEAD和空git status。三份raw均git_dirty=false。\n- 不修改常量ROUNDS=10：三次顺序独立进程各10轮，共30轮配对/60窗口。每窗口平均8次完整两批接收；共480次计时流程，但只是一组固定8票证明池，不是480场或30场独立选举。\n- 保留每进程原有2次/路线预热、交替AB/BA、相同研究种子。全部样本保留，无阈值删除、重跑替换或跨旧dirty样本拼接。报告列出进程分组统计，不按30个IID选举构造显著性结论。\n- 使用锁定Rust 1.99.1 Release/offline，构建jobs=2、Rayon=4；复用外部依赖/编译缓存。干净源码不等于冷缓存或全新机器。\n- 结果和日志在worktree之外产生；所有运行完成后才导入本分支，不把事后复制结果后的dirty状态倒填到运行时。驱动自身由独立SHA追踪，不冒充e1e477f中的源码。\n\n| 进程 | Direct两批中位数 ms | State两批中位数 ms | 共享进程峰值 MiB |\n| --- | ---: | ---: | ---: |\n${summary.per_process.map(x=>`| ${x.process} | ${f(x.routes.snarkpack_direct_fold.two_batch_flow_ms.median)} | ${f(x.routes.snarkpack_state_proof.two_batch_flow_ms.median)} | ${x.peak_rss_mib.toFixed(3)} |`).join('\n')}\n\nRSS包括共同材料和两路线，不是逐路线内存，更不是Prove内存。独立选票池尚未增加，不能以三个进程冒充三个证明池。\n\n## 正确性和成本边界\n\n三次均到达同一CT2。每次先实际测试两类错误newState，原SnarkPack通过后Direct/Groth16分别拒绝，NF/CT保持不变；共6次执行、仅2种负例。\n没有新Setup/Prove/聚合生成或EVM交易；生成时间均为null/REUSED_NOT_RETIMED，不能填零。\n状态证明生成负担已作为G10-B必需成本登记。G2-D/G2-E/G6历史Prove/Setup仅作带哈希来源的参考，不加到本轮干净接收时间上冒充完整实测。\n未完成多个独立证明池、逐路线RSS、匹配的生成时间或完整系统成本，因此没有完成上传计划的全部性能论证。\n\n## 复现命令\n\n\`\`\`sh\n# 在仓库中建立一个全新、固定提交的worktree；下列均替换为新的绝对路径\ngit worktree add --detach /ABS/NEW_CLEAN_SOURCE ${BASE}\nnode scripts/g10a_clean_reproduce.mjs /ABS/NEW_CLEAN_SOURCE /ABS/EXTERNAL_TARGET_CACHE /ABS/NEW_OUTPUT_DIR\nnode scripts/g10a_clean30_report.mjs --check\n\`\`\`\n\n驱动拒绝脏源码、错误提交、源码内输出和已有输出目录；每个命令/时间/退出码、运行前后状态及源/二进制哈希保存在 ${dir}/execution/manifest.json。\n新输出目录复现须单独导入/分析，不覆盖本次归档；本报告脚本仅复核本次已归档数据。\n原始三份run与全部日志：${dir}；统计：results/g10_a_clean30/summary.json。\n旧 experiments/g10_a_raw、results/g10_a 和原文档均保持原字节。G9-P1仍BLOCKED，P2未启动。\n`;
function put(p,b){if(mode==='--check'||fs.existsSync(p))assert.equal(fs.readFileSync(p,'utf8'),b,p);else{fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,b,{flag:'wx'});}}
put('results/g10_a_clean30/summary.json',JSON.stringify(summary,null,2)+'\n');put('results/g10_b_plan/comparison_scope.json',JSON.stringify(scope,null,2)+'\n');put('docs/G10_A_CLEAN30_REPRODUCTION.md',md);
console.log(JSON.stringify({status:summary.status,mode,dirty:false,paired_rounds:30,windows:60,proof_pools:1,direct_two_batch_ms:d.median,state_two_batch_ms:s.median,paired_ratio:summary.paired_state_over_direct.median,G10_B:'DESIGN_ONLY',new_setup:0,new_proofs:0,new_evm:0}));
