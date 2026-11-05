// Artifact-linked generation ledger + actual bounded aggregation data; no new prover run.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
const mode=process.argv[2]??'--check';assert.ok(['--write','--check'].includes(mode));
const sha=b=>createHash('sha256').update(b).digest('hex');
const pin=p=>({path:p,sha256:sha(fs.readFileSync(p))});
const paths={d0:'experiments/g1_d0_raw/uniform_depth5_native_pilot.json',d1:'experiments/g1_d1_raw/uniform_depth5_batch16.json',d2:'experiments/g1_d2_raw/uniform_depth5_batch32.json',
 f0:'fixtures/snarkpack_g1d0_depth5_public_v1.json',f1:'fixtures/snarkpack_g1d1_depth5_batch16_public_v1.json',f2:'fixtures/snarkpack_g1d2_depth5_batch32_public_v1.json',
 sd:'experiments/g2_d_raw/state_transition_d5w32_b4_groth16.json',se:'experiments/g2_e_raw/multibatch_state_b4.json',
 fd:'fixtures/jubjub_state_g2d_d5w32_b4_public_v1.json',fe:'fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json',
 g6:'experiments/g6_raw/native_winner_only.json',fg6:'fixtures/jubjub_g6_native_winner_only_public_v1.json',
 b1:'results/g10_b1/summary.json',b1raw:'experiments/g10_b1_raw/matched.json',manifest:'configs/protocol_manifest.JUBJUB-EVAL-D5-W32.json',
 current:'experiments/g10_b2_raw/aggregation_cost.json',execution:'experiments/g10_b2_raw/execution/manifest.json'};
const j=Object.fromEntries(Object.entries(paths).map(([k,p])=>[k,JSON.parse(fs.readFileSync(p))]));
// Proof identity, not merely same parameters: trace the exact 8 proofs through D0 -> D1 -> D2 -> G2-E -> B1.
assert.equal(j.d0.public_verifier_fixture_sha256,pin(paths.f0).sha256);
assert.equal(j.d1.baseline_raw_sha256,pin(paths.d0).sha256);assert.equal(j.d1.baseline_fixture_sha256,pin(paths.f0).sha256);
assert.equal(j.d2.baseline_raw_sha256,pin(paths.d1).sha256);assert.equal(j.d2.baseline_fixture_sha256,pin(paths.f1).sha256);
assert.equal(j.d2.public_verifier_fixture_sha256,pin(paths.f2).sha256);
assert.equal(j.sd.public_verifier_fixture_sha256,pin(paths.fd).sha256);assert.equal(j.se.public_verifier_fixture_sha256,pin(paths.fe).sha256);
assert.equal(j.g6.public_fixture_sha256,pin(paths.fg6).sha256);assert.equal(j.fg6.source_g2e_fixture_sha256,pin(paths.fe).sha256);
assert.deepEqual(j.f1.source_proofs_ark_compressed_hex.slice(0,8),j.f0.source_proofs_ark_compressed_hex);
assert.deepEqual(j.f2.source_proofs_ark_compressed_hex.slice(0,8),j.f0.source_proofs_ark_compressed_hex);
assert.equal(new Set(j.f0.source_proofs_ark_compressed_hex).size,8);
assert.equal(j.d0.source_proof_count,8);assert.equal(j.d0.groth16_prove_ms.length,8);
assert.ok(Math.abs(j.d0.groth16_prove_ms.reduce((a,b)=>a+b,0)-j.d0.groth16_prove_total_ms)<1e-6);
for(const k of ['f1','f2']){
 assert.deepEqual(j[k].source_public_records.slice(0,8),j.f0.source_public_records);
 assert.deepEqual(j[k].source_public_inputs_le_hex.slice(0,8),j.f0.source_public_inputs_le_hex);
 assert.equal(j[k].groth16_vk_hex,j.f0.groth16_vk_hex);
 assert.equal(j[k].shared_aggregation_srs_sha256,j.f0.shared_aggregation_srs_sha256);
}
assert.equal(j.fe.source_ballot_vk_hex,j.f2.groth16_vk_hex);assert.equal(j.fd.groth16_vk_hex,j.fe.state_vk_hex);
assert.equal(j.sd.proof_sha256,j.fe.batches[0].state_proof_sha256);assert.equal(j.fd.proof_hex,j.fe.batches[0].state_proof_hex);
assert.equal(j.sd.verifying_key_sha256,j.se.state_vk_sha256);assert.equal(j.se.state_vk_sha256,j.fe.state_vk_sha256);
assert.deepEqual(j.sd.old_state,j.fe.batches[0].old_state);assert.deepEqual(j.sd.new_state,j.fe.batches[0].new_state);
assert.deepEqual(j.fe.batches[1].old_state,j.fe.batches[0].new_state);
assert.deepEqual(j.se.final_state,j.fe.final_state);assert.deepEqual(j.g6.final_checkpoint.state,j.fe.final_state);
assert.deepEqual(j.fg6.final_checkpoint.state,j.fe.final_state);assert.equal(j.g6.verifying_key_sha256,j.fg6.verifying_key_sha256);
assert.equal(sha(Buffer.from(j.fg6.proof_hex,'hex')),j.fg6.proof_sha256);
assert.deepEqual(j.b1raw.final_state,j.fe.final_state.map(c=>({c0_hex:c.c0_hex,c1_hex:c.c1_hex})));
for(let i=0;i<2;i++){
 const b=j.fe.batches[i];assert.deepEqual(b.source_public_records,j.f2.source_public_records.slice(i*4,i*4+4));
 assert.deepEqual(b.source_public_inputs_le_hex,j.f2.source_public_inputs_le_hex.slice(i*4,i*4+4));
 assert.deepEqual(b.source_ballot_proofs_hex,j.f2.source_proofs_ark_compressed_hex.slice(i*4,i*4+4));
 assert.equal(sha(Buffer.from(b.state_proof_hex,'hex')),b.state_proof_sha256);
 assert.equal(sha(Buffer.from(b.source_aggregate_hex,'hex')),b.source_aggregate_sha256);
}
assert.equal(j.sd.source_aggregate_sha256,j.fe.batches[0].source_aggregate_sha256);
for(const r of [j.se,j.g6])for(const p of r.source_artifact_pins)assert.equal(pin(p.path).sha256,p.sha256,p.path);
const raw=j.current,m=j.execution;
assert.equal(raw.status,'PASS_EXACT_TWO_B4_AGGREGATION_GENERATION');assert.equal(m.status,'PASS_G10_B2_BOUNDED_AGGREGATION_COST');
assert.equal(raw.samples.length,24);assert.equal(raw.method.paired_rounds,12);assert.equal(raw.method.proof_pools,1);
assert.equal(raw.new_aggregate_count_including_warmup,26);assert.equal(raw.generated_aggregate_verifications,26);
assert.equal(raw.new_groth16_setups,0);assert.equal(raw.new_ballot_proofs+raw.new_state_proofs+raw.new_p3_proofs,0);
assert.equal(raw.research_srs_rebuilds,1);assert.equal(raw.evm_executed,false);assert.equal(raw.complete_system_cost,null);
assert.equal(raw.generic_srs_sha256,j.f2.shared_aggregation_srs_sha256);assert.equal(raw.research_srs_seed_hex,'4731443053525320');
assert.equal(raw.source_version_sha256,sha(Buffer.from(JSON.stringify(raw.sources))));
assert.equal(raw.source_version_sha256,m.source_version_sha256);assert.equal(pin(paths.current).sha256,m.raw_sha256);
assert.equal(raw.binary_sha256,m.binary_sha256);assert.equal(raw.git_dirty,true);assert.equal(raw.rayon_num_threads,'4');
assert.equal(sha(fs.readFileSync('scripts/g10b2_run.mjs')),m.driver_sha256);
assert.equal(sha(fs.readFileSync('experiments/g10_b2_raw/source_snapshot/scripts/g10b2_run.mjs')),m.driver_sha256);
for(const s of raw.sources){assert.equal(pin(s.path).sha256,s.sha256);assert.equal(pin('experiments/g10_b2_raw/source_snapshot/'+s.path).sha256,s.sha256);}
for(const s of [...raw.fixture_pins,...raw.additional_pins].filter(s=>s.path))assert.equal(pin(s.path).sha256,s.sha256);
assert.equal(m.commands.length,7);for(const c of m.commands){assert.equal(c.exit_code,0,c.name);assert.equal(c.signal,null);assert.equal(c.error,null);assert.ok(fs.existsSync('experiments/g10_b2_raw/'+c.log));}
assert.match(fs.readFileSync('experiments/g10_b2_raw/execution/receiver-regression.log','utf8'),/4 passed; 0 failed/);
for(let round=1;round<=12;round++){
 const w=raw.samples.filter(s=>s.round===round);assert.deepEqual(w.map(s=>s.batch_index),round%2?[2,1]:[1,2]);
 assert.deepEqual(w.map(s=>s.position),[1,2]);for(const s of w){assert.ok(Number.isFinite(s.aggregation_ms)&&s.aggregation_ms>0);assert.ok(s.byte_identical_to_archive&&s.verified);assert.equal(s.aggregate_sha256,j.fe.batches[s.batch_index-1].source_aggregate_sha256);}
}
function quantile(a,p){a=[...a].sort((x,y)=>x-y);const i=(a.length-1)*p;return a[Math.floor(i)]+(a[Math.ceil(i)]-a[Math.floor(i)])*(i%1);}
const stats=a=>({n:a.length,median:quantile(a,.5),q1:quantile(a,.25),q3:quantile(a,.75),min:Math.min(...a),max:Math.max(...a)});
const aggregation={by_batch:[1,2].map(i=>({batch_index:i,aggregation_ms:stats(raw.samples.filter(s=>s.batch_index===i).map(s=>s.aggregation_ms))})),
 two_batch_generation_components_ms:stats(Array.from({length:12},(_,i)=>raw.samples.filter(s=>s.round===i+1).reduce((a,b)=>a+b.aggregation_ms,0))),
 generation_times_only_not_end_to_end:true,process_peak_rss_mib:raw.process_peak_rss_mib,rss_scope:raw.rss_scope,
 srs_rebuild_ms_offline:raw.research_srs_rebuild_ms_offline,srs_specialize_ms_offline:raw.research_srs_specialize_ms_offline,
 matrix_transcript_pack_ms_offline:raw.matrix_and_transcript_pack_ms_offline};
const historical=({id,title,key,field,stage,applies,artifact,...other})=>({id,title,classification:'HISTORICAL_EXACT_ARTIFACT_COST_NOT_CURRENT_MATCHED_MEASUREMENT',source:pin(paths[key]),field,
 measured_ms:j[key][field],stage,applies_to_native_intake_routes:applies,artifact,...other});
const all=['individual_groth16_per_vote','transaction_batch_individual_groth16','snarkpack_direct_fold','snarkpack_state_proof'];
const ledger=[
 historical({id:'ballot_setup',title:'选票 Groth16 Setup',key:'d0',field:'groth16_setup_ms',stage:'OFFLINE',applies:all,artifact:{vk_ark_sha256:j.d0.ballot_vk_ark_compressed_sha256,vk_wire_sha256:j.d0.ballot_vk_bellperson_wire_sha256},count_in_original_execution:1}),
 historical({id:'eight_ballot_proving',title:'原八票 Groth16 Prove',key:'d0',field:'groth16_prove_total_ms',stage:'PROVER',applies:all,artifact:{proof_sha256:j.f0.source_proofs_ark_compressed_hex.map(h=>sha(Buffer.from(h,'hex')))},per_ballot_ms:j.d0.groth16_prove_ms,proof_indices:[0,1,2,3,4,5,6,7]}),
 historical({id:'aggregate1_original',title:'第一批 B4 原聚合生成',key:'sd',field:'new_source_aggregation_ms',stage:'AGGREGATOR',applies:all.slice(2),artifact:{aggregate_sha256:j.fe.batches[0].source_aggregate_sha256}}),
 historical({id:'aggregate2_original',title:'第二批 B4 原聚合生成',key:'se',field:'new_source_aggregation_ms',stage:'AGGREGATOR',applies:all.slice(2),artifact:{aggregate_sha256:j.fe.batches[1].source_aggregate_sha256}}),
 historical({id:'state_setup',title:'状态关系 Setup',key:'sd',field:'setup_ms',stage:'OFFLINE',applies:[all[3]],artifact:{vk_sha256:j.fe.state_vk_sha256},count_in_original_execution:1}),
 historical({id:'state_setup_reconstruction',title:'后续状态 PK 重建',key:'se',field:'setup_rebuild_ms',stage:'OFFLINE_RECONSTRUCTION',applies:[all[3]],artifact:{vk_sha256:j.fe.state_vk_sha256},not_an_additional_online_per_batch_cost:true}),
 historical({id:'state1_proving',title:'第一批状态 Prove',key:'sd',field:'prove_ms',stage:'PROVER',applies:[all[3]],artifact:{proof_sha256:j.fe.batches[0].state_proof_sha256,batch_root:j.fe.batches[0].batch_root_hex},process_peak_mib:j.sd.peak_rss_after_prove_mib,memory_scope:'cumulative process includes Setup and matrix instrumentation, not isolated Prove allocation'}),
 historical({id:'state2_proving',title:'第二批状态 Prove',key:'se',field:'new_second_prove_ms',stage:'PROVER',applies:[all[3]],artifact:{proof_sha256:j.fe.batches[1].state_proof_sha256,batch_root:j.fe.batches[1].batch_root_hex},process_peak_mib:j.se.peak_rss_mib,memory_scope:'whole G2-E execution, not isolated Prove allocation'}),
 historical({id:'p3_setup',title:'P3 PK 重建（接收范围外）',key:'g6',field:'setup_rebuild_ms',stage:'OFFLINE_SUFFIX_ONLY',applies:[],artifact:{vk_sha256:j.fg6.verifying_key_sha256}}),
 historical({id:'p3_proving',title:'P3 Prove（接收范围外）',key:'g6',field:'prove_ms',stage:'PROVER_SUFFIX_ONLY',applies:[],artifact:{proof_sha256:j.fg6.proof_sha256,target_fixture_sha256:pin(paths.fe).sha256},process_peak_mib:j.g6.peak_rss_mib,memory_scope:'whole G6 flow including Setup, not isolated P3 Prove allocation'}),
];
for(const row of ledger)assert.ok(Number.isFinite(row.measured_ms)&&row.measured_ms>0);
const compatibility=['d0','d1','d2','sd','se','g6'].map(key=>({source:pin(paths[key]),recorded_git:j[key].git_commit,recorded_dirty:j[key].git_dirty,
 cpu:j[key].cpu,rustc:j[key].rustc_version,profile:j[key].build_profile,recorded_rayon_threads:null,
 archived_source_hashes_different_from_current:j[key].source_hashes.filter(s=>pin(s.path).sha256!==s.sha256).map(s=>({...s,current_sha256:pin(s.path).sha256})),
 same_binary_as_current:false,current_prover_measurement:false}));
const summary={schema:'VV/G10-B2/COST-LEDGER/v1',status:'PASS_ARTIFACT_COST_AUDIT_AND_BOUNDED_AGGREGATION',source_base_sha:raw.source_base_sha,
 source_version_sha256:raw.source_version_sha256,binary_sha256:raw.binary_sha256,source_was_dirty:true,
 sources:Object.entries(paths).map(([id,p])=>({id,...pin(p)})),identity_checks:{original_eight_proofs_traced_through_d0_d1_d2:true,same_records_matrix_key_and_context:true,
 both_state_proof_bytes_and_ct1_ct2_match:true,p3_fixture_targets_same_ct2:true,research_srs_matches:true,all_26_generated_aggregates_byte_identical_and_verified:true},
 historical_ledger:ledger,historical_condition_audit:compatibility,current_aggregation:aggregation,
 current_aggregation_raw:pin(paths.current),timed_intake_source:pin(paths.b1),
 scope:'ARTIFACT_LINKED_COMPONENT_COSTS_NOT_MATCHED_FULL_SYSTEM_COST',new_groth16_setups:0,new_groth16_proofs:0,new_aggregates:26,new_evm:0,research_srs_rebuilds:1,
 artifact_linked_historical_generation_available:true,all_current_generation_costs_measured:false,full_end_to_end_total_ms:null,full_system_gas:null,
 historical_costs_not_added_to_B1_timings:true,setup_not_counted_as_online:true,
 gaps:['current matched ballot/state/P3 generation and isolated prover RSS not retimed','whole lifecycle and Full Tally/Winner-only matched comparison not implemented',
 'one fixed proof pool; no independent proof-generation sample statistics','exact SnarkPack EVM and T4/T5 security proof obligations unresolved'],
 decision:{rerun_g1e_scale_matrix:false,rerun_groth16_setup_prove_now:false,reason:'exact artifact-linked historical costs already exist; no need to regenerate merely to fill a missing ledger. Different source/lock/thread conditions prevent current full-system cost claims.',
 next_only:'G10-C1: specify/implement same-CT2 Full Tally vs archived Winner-only public verification/release comparison, with separated output branches and explicit trust scope; no automatic P3 re-proving or public deployment.'},G9_P1:'BLOCKED',G9_P2_started:false};
const fmt=x=>x.toFixed(3),a=aggregation;
const md=`# G10-B2：生成成本证据账本与两批 B4 聚合补测

状态：${summary.status}。本轮是**成本来源核对 + 有限聚合生成补测**，不是新的完整系统性能实验。

## 1. 本轮真实补测

同一W32/D5、同一8票/两个连续B4、同一选票VK、原SnarkPack V2和研究SRS。仅重建一次research Fake SRS，不做Groth16 Setup/Prove。
两批各预热1次，再做12轮交替顺序；24个正式样本、26次实际聚合及26次真实验证，全部结果保留，所有聚合字节与G2-E归档相同。

| 项目 | 中位数 ms | Q1 / Q3 ms |
| --- | ---: | ---: |
${a.by_batch.map(b=>`| 第${b.batch_index}批 B4 聚合生成 | ${fmt(b.aggregation_ms.median)} | ${fmt(b.aggregation_ms.q1)} / ${fmt(b.aggregation_ms.q3)} |`).join('\n')}
| 同轮两次聚合运算之和 | ${fmt(a.two_batch_generation_components_ms.median)} | ${fmt(a.two_batch_generation_components_ms.q1)} / ${fmt(a.two_batch_generation_components_ms.q3)} |

只计aggregate_proofs调用；序列化、比对和真实验证在其外记录。两次运算之和不是端到端流程。
离线研究SRS重建 ${fmt(a.srs_rebuild_ms_offline)} ms、specialize ${fmt(a.srs_specialize_ms_offline)} ms、两批matrix/transcript准备 ${fmt(a.matrix_transcript_pack_ms_offline)} ms。
整个聚合进程峰值 ${a.process_peak_rss_mib.toFixed(3)} MiB，包含共同加载/SRS，不是单次聚合分配或Groth16证明者内存。
只有一组固定证明池，不是12次独立选举。此补测不重复G1-E B2–B32矩阵；第一批为当前环境对照，第二批对应非零旧状态的连续结算材料。

## 2. 已有真实生成成本：逐项对上当前证明

| 历史组件 | 已记录耗时 s | 来源 | 口径 |
| --- | ---: | --- | --- |
${ledger.map(r=>`| ${r.title} | ${(r.measured_ms/1000).toFixed(6)} | ${r.source.path} / ${r.field} | ${r.stage} |`).join('\n')}

八份原始选票证明逐字节从D0经D1/D2追踪到当前G2-E/B1，计时对应原8票，不把D1/D2新增16/32票时间重复计入。
两份状态证明对应CT0→CT1、CT1→CT2，状态VK相同。P3仅作同CT2后缀成本参考，尚未接入本轮四路线完整生命周期。
同一合法性证明的生成负担对四路线一致；只有SnarkPack路线承担聚合生成，只有State路线承担状态证明生成。
Setup是离线项。第二次状态Setup是PK未保存所需的历史重建，不能默认把两次历史重建都作为一个部署的在线批次成本。
历史RSS是包含Setup/关系检查的累计高水位，不是每个Prove阶段的隔离内存。

## 3. 为什么不重跑大电路

成本数据不是不存在，而是此前未统一映射到现用证明。本轮已经验证证明、VK、记录、矩阵、上下文、批次根和CT2的对应关系并保存独立SHA。
但历史源文件/锁文件有差异，且历史Rayon线程条件未完整归档；即使同为Apple M4/Rust1.99.1/Release，也不能把它们称为当前同二进制生成测量。
机器清单保存各历史source hash与当前差异，不抹去原git_dirty或替换旧SHA。没有把旧Prove和新B1接收、或本轮Aggregation相加充当端到端结果。
因此当前完整系统总时间/Gas仍为null，匹配生成成本和独立证明者RSS仍未完成；不以生成次数0冒充成本0。
目前B4接收路线已显示逐票验证更快，没有必要为获得预设优势扩B64+或重跑旧规模实验。重型Setup/Prove仅在最终明确需要同环境完整成本时按预算单独补测。

## 4. 来源与复现

执行基准HEAD ${raw.source_base_sha}，工作区有本轮未提交修改；实际源版本SHA ${raw.source_version_sha256}，二进制SHA ${raw.binary_sha256}。
本轮未改变Cargo.toml：使用Cargo自动发现的新bin，保证B1所钉住的旧源码/配置字节不变。执行前后B1归档核查均通过。
命令、退出码、硬件、源/二进制hash见experiments/g10_b2_raw/execution/manifest.json及aggregation_cost.json；源码原字节快照见同目录source_snapshot。
账本见results/g10_b2/cost_ledger.json，含全部来源路径/文件hash、逐证明hash、应用路线、历史环境差异和仍未测项。

\`\`\`sh
node scripts/g10b2_run.mjs /ABS/NEW_OUTPUT_DIRECTORY
node scripts/g10b2_report.mjs --check
node scripts/g10b2_check.mjs
\`\`\`

本轮新增Groth16 Setup=0、选票/状态/P3 Prove=0、研究SRS重建=1、聚合=26、EVM=0。旧实验、图表、配置和论文不改。
G9-P1仍BLOCKED，不进入P2。下一步只推进同CT2下Full Tally/Winner-only的公开验证/披露比较，隔离输出分支；不自动重做P3，不把公开字节差异当成隐私定理。
`;
function put(p,b){if(mode==='--check'||fs.existsSync(p))assert.equal(fs.readFileSync(p,'utf8'),b,p);else{fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,b,{flag:'wx'});}}
put('results/g10_b2/cost_ledger.json',JSON.stringify(summary,null,2)+'\n');put('docs/G10_B2_GENERATION_COST_AUDIT.md',md);
console.log(JSON.stringify({status:summary.status,mode,exact_ballot_proofs:8,exact_state_proofs:2,exact_p3_target:true,paired_aggregation_rounds:12,
 two_batch_aggregation_ms:a.two_batch_generation_components_ms.median,historical_ballot_prove_ms:j.d0.groth16_prove_total_ms,
 historical_state_prove_ms:[j.sd.prove_ms,j.se.new_second_prove_ms],new_groth16_proofs:0,new_aggregates:26,research_srs_rebuilds:1,new_evm:0,full_end_to_end_total_ms:null}));
