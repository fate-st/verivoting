// Functional evidence only; no optimization benchmarks, no cryptographic rerun.
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';import {createHash} from 'node:crypto';
const mode=process.argv[2]??'--check';assert.ok(['--write','--check'].includes(mode));
const root='experiments/g10_c_raw',sha=b=>createHash('sha256').update(b).digest('hex');
const load=p=>JSON.parse(fs.readFileSync(p)),map=load(root+'/archive_map.json');
const resolve=p=>p.startsWith(map.runtime_root+'/')?path.join(root,p.slice(map.runtime_root.length+1)):p;
const n=load(root+'/native/report.json'),c=load(root+'/chain/winner_once.json'),complete=load(root+'/functional_completion.json');
const nm=load(root+'/execution/manifest.json'),cm=load(root+'/chain_bridge/manifest.json'),p=load(root+'/native/winner_only/public.json');
const publicReplay=load(root+'/winner_public_process.json'),fullReplay=load(root+'/full_tally_public_process.json');
assert.equal(n.status,'PASS_CONNECTED_NATIVE_FIVE_WORKER_NEW_P3_WINNER_ONCE_AND_FULL_TALLY_DIAGNOSTIC');
assert.equal(c.status,'PASS_CURRENT_NATIVE_P3_AND_COMMITMENTS_EVM_WINNER_ONCE');
assert.equal(complete.status,'PASS_IMPLEMENTED_NATIVE_AND_INDIVIDUAL_GROTH16_EVM_FALLBACK_FLOW');
assert.equal(nm.status,'PASS_CONNECTED_NATIVE_NEW_P3_AND_SEPARATE_PUBLIC_RELEASE_VERIFIERS');
assert.equal(cm.status,'PASS_SAME_NATIVE_PROOF_AND_WORKER_COMMITMENTS_CONNECTED_TO_EVM_FALLBACK');
for(const [manifest,count,base] of [[nm,9,root],[cm,6,root+'/chain_bridge']]){
 assert.equal(manifest.commands.length,count);for(const cmd of manifest.commands){assert.equal(cmd.exit_code,0,cmd.name);assert.equal(cmd.signal,null);assert.equal(cmd.error,null);assert.ok(fs.existsSync(cmd.log.startsWith('/')?resolve(cmd.log):path.join(base,cmd.log)));}
}
assert.equal(sha(fs.readFileSync('scripts/g10c_run.mjs')),nm.driver_sha256);assert.equal(sha(fs.readFileSync('scripts/g10c_chain.mjs')),cm.driver_sha256);
assert.equal(sha(fs.readFileSync(root+'/native/report.json')),cm.native_report_sha256);
assert.equal(sha(fs.readFileSync(root+'/chain/winner_once.json')),cm.chain_report_sha256);
assert.equal(n.source_version_sha256,sha(Buffer.from(JSON.stringify(n.sources))));assert.equal(n.source_version_sha256,nm.source_version_sha256);
for(const s of n.sources){assert.equal(sha(fs.readFileSync(s.path)),s.sha256,s.path);assert.equal(sha(fs.readFileSync(root+'/source_snapshot/'+s.path)),s.sha256);}
assert.equal(n.binary_sha256,nm.binary_sha256);assert.equal(n.new_p3_proofs,1);assert.equal(n.new_groth16_setup_rebuilds,1);
assert.equal(n.new_ballot_proofs,0);assert.equal(n.new_state_proofs,0);assert.equal(n.new_cp_packages,5);
assert.equal(n.worker_evidence.unique_process_ids,5);assert.equal(new Set(n.worker_evidence.workers.map(w=>w.process_id)).size,5);
assert.ok(n.worker_evidence.workers.every(w=>w.exited_successfully&&w.private_input_bytes>0&&w.private_response_bytes>0));
assert.equal(n.worker_evidence.independent_distributed_dkg,false);assert.equal(n.worker_evidence.centralized_research_dkg_bootstrap,true);
assert.equal(n.five_cp_and_commitment_openings_verified,true);assert.equal(n.new_p3_proof_from_current_private_witness,true);
assert.equal(n.all_60_inputs_from_live_frozen_state,true);assert.equal(n.same_ct2_and_freeze_as_g6,true);
assert.ok(n.batches.every(b=>b.snarkpack_verified&&b.state_proof_verified));assert.equal(n.batches.length,2);
assert.equal(n.winner_writes,1);assert.equal(p.final_checkpoint.winner_write_count,1);assert.equal(p.public_winner,1);
assert.equal(p.verifying_key_sha256,'0d37a77cfc8a1cdfed612b397149d96816188d164e08a74994dd2abfca6cff29');
assert.equal(sha(Buffer.from(p.proof_hex,'hex')),n.p3_proof_sha256);assert.equal(n.p3_proof_sha256,c.ark_proof_sha256);
assert.notEqual(n.p3_proof_sha256,load('fixtures/jubjub_g6_native_winner_only_public_v1.json').proof_sha256);
assert.deepEqual(p.public_inputs_le_hex.map(s=>'0x'+Buffer.from(s,'hex').reverse().toString('hex')),c.public_inputs_be_hex);
assert.equal(publicReplay.status,'PASS_PUBLIC_WINNER_REPLAY_WITHOUT_PRIVATE_DATA');assert.equal(publicReplay.private_dkg_or_cp_or_tally_loaded,false);
assert.equal(fullReplay.status,'PASS_PUBLIC_FULL_TALLY_DIAGNOSTIC_VERIFICATION');assert.ok(fullReplay.claimed_wrong_tally_rejected&&fullReplay.same_winner&&fullReplay.no_secret_keys_loaded);
assert.ok(Object.values(n.necessary_negatives).every(Boolean));
assert.equal(n.release_comparison.full_tally_policy_authorized,false);assert.equal(n.release_comparison.full_tally_payload_not_in_public_archive,true);
assert.equal(map.full_tally_usable_data_published,false);assert.ok(!fs.existsSync(root+'/private_diagnostic'));
const forbidden=new Set(['secret_share_hex','authentication_rng_word_pos','opening','released_packages','tally','cp']);
let jsonFiles=0;
function checkObject(o,p){if(o&&typeof o==='object'){for(const [k,v] of Object.entries(o)){assert.ok(!forbidden.has(k),'private/full-tally payload escaped into '+p+': '+k);checkObject(v,p);}}}
function walk(dir){for(const e of fs.readdirSync(dir,{withFileTypes:true})){const p=path.join(dir,e.name);if(e.isDirectory()){if(e.name!=='source_snapshot'&&e.name!=='provenance')walk(p);}else if(e.name.endsWith('.json')){checkObject(load(p),p);jsonFiles++;}}}
walk(root);
const successful=c.transactions.filter(t=>t.receipt_status===1),deploy=successful.filter(t=>t.label.startsWith('deploy_'));
const sum=a=>a.reduce((s,t)=>s+t.total_gas,0),disclose=c.transactions.find(t=>t.label==='disclose_winner_once');
assert.equal(complete.successful_gas_including_deployment,sum(successful));assert.equal(successful.length,17);
const summary={schema:'VV/G10-C/FUNCTIONAL-INTEGRATION/v1',status:complete.status,scope:'CONNECTED_IMPLEMENTED_NATIVE_AND_AUTHENTICATED_EVM_FALLBACK_NOT_FULL_TARGET',
 source_base_sha:n.source_base_sha,source_version_sha256:n.source_version_sha256,source_was_dirty:true,binary_sha256:n.binary_sha256,
 native_proof_sha256:n.p3_proof_sha256,evm_uses_same_proof_and_worker_commitments:true,matched_public_input_count:60,
 completed:{authenticated_dkg_registration_reconstruction:true,two_actual_native_snarkpack_and_state_batches:true,live_ct2_freeze:true,
 five_separate_cp_package_processes:true,private_pipe_delivery_and_cp_opening_authentication:true,private_threshold_decode:true,
 one_fresh_fixed_vk_p3:true,native_one_time_winner:true,independent_public_winner_replay:true,separate_full_tally_diagnostic_public_verifier:true,
 evm_certificate_authentication:true,evm_individual_ballot_and_state_verification:true,evm_ct2_freeze_and_commitment_authentication:true,
 evm_storage_rebuild_60_inputs_and_same_new_p3:true,evm_winner_write_once_and_replay_rejection:true},
 native:{trustee_process_ids:n.worker_evidence.workers.map(w=>w.process_id),new_cp_packages:5,new_p3_proofs:1,setup_rebuilds:1,
 setup_rebuild_ms:n.setup_rebuild_ms,prove_ms:n.p3_prove_ms,total_wall_ms:n.total_wall_ms,process_peak_rss_mib:n.process_peak_rss_mib,
 reused_ballot_proofs:8,reused_aggregates:2,reused_state_proofs:2,functional_run_not_statistics:true},
 chain:{environment:'local_Prague',signed_transactions:21,successful:17,rejected:4,successful_gas_including_deployment:sum(successful),
 deployment_gas:sum(deploy),non_deployment_success_gas:sum(successful.filter(t=>!t.label.startsWith('deploy_'))),
 disclose_winner_gas:disclose.total_gas,disclose_calldata_bytes:disclose.calldata_bytes,one_winner_event:true,exact_snarkpack_evm:false,
 gas_scope:'same new functional run receipts; successes only, deployment separated; not a statistical or exact-SnarkPack full-system efficiency result'},
 disclosure:{winner_only_fixture_json_bytes:n.release_comparison.winner_only_fixture_json_bytes,full_tally_diagnostic_json_bytes:n.release_comparison.full_tally_diagnostic_json_bytes,
 winner_ark_proof_bytes:192,winner_eip_proof_bytes:512,winner_derived_inputs_bytes:1920,cached_ark_vk_bytes:n.release_comparison.cached_p3_vk_bytes,
 full_tally_counterfactual_not_policy_authorized:true,usable_full_tally_payload_withheld:true,public_data_json_files_scanned:jsonFiles,
 comparable_transport_envelope:false,privacy_theorem_proved:false},
 remaining_required:complete.remaining_required,
 target_architecture_complete:false,independent_distributed_dkg:false,production_setup:false,production_deployment:false,full_privacy_proof:false,
 no_optimization_performed:true,no_batch_scale_expansion:true,no_g1e_repeat_campaign:true,
 one_command_driver_available:'node scripts/run_functional_experiment.mjs /ABS/NEW_RUN_ROOT',
 current_execution_was_two_sequential_drivers_then_finalize:true,current_one_command_proving_invocation_claimed:false,
 next_priority:'Close exact SnarkPack EVM verification/receiver functionality under an explicit bounded route decision; connect genuinely independent DKG/key custody to this same full-flow context. No further microbenchmark/optimization gates.',
 sources:[['native/report.json',n],['chain/winner_once.json',c],['functional_completion.json',complete]].map(([p])=>({path:root+'/'+p,sha256:sha(fs.readFileSync(root+'/'+p))}))};
const md=`# G10-C：先完成流程，不优化

状态：${summary.status}。本轮运行的是实际连接的原生流程及**已有逐票Groth16 EVM备用路线**，不是完整精确SnarkPack链上架构。

## 已实际连通的流程

认证DKG/注册重建 → 两个B4的真实SnarkPack/状态证明验证 → CT1/CT2 → 一次冻结 → 五个独立OS进程生成CP/承诺签名 → 私管道交付 → CP/打开/身份核查 → 私密3-of-5解密 → 一份新P3 → 原生Winner一次写入。
随后使用**同一份新P3、同一组新承诺、同一CT2**，在新本地Prague VM中实际部署原合约，认证证书、逐票验证两批、验证状态证明、冻结、认证承诺、从存储重建60输入，再验证新P3并写入Winner=1一次。
不是拼接历史JSON；链上交易、配对调用输入、返回值、公开输入和证明哈希都有本轮独立核对。错误Winner在披露前实际得到pairing=0；正确证明pairing=1；重放不进入pairing。

| 本轮功能验收 | 实际结果 |
| --- | --- |
| 私包进程 | 5个不同PID：${summary.native.trustee_process_ids.join(', ')} |
| 新私包 / 新P3 | 5 / 1；不是复用G6旧P3 |
| P3 VK | 原G4-D VK不变，${p.verifying_key_sha256} |
| 新证明 SHA-256 | ${n.p3_proof_sha256} |
| 公开输入 | 原生新证明与合约存储重建60/60相同 |
| 原生/链上Winner | 均为1，分别只写一次 |
| EVM交易 | 21笔，17成功、4项必要原子拒绝 |
| 新披露交易 | ${disclose.total_gas.toLocaleString('en-US')} Gas，${disclose.calldata_bytes} B calldata |

原生本次PK重建 ${(n.setup_rebuild_ms/1000).toFixed(3)} s、Prove ${(n.p3_prove_ms/1000).toFixed(3)} s、总执行 ${(n.total_wall_ms/1000).toFixed(3)} s、进程累计峰值 ${(n.process_peak_rss_mib/1024).toFixed(3)} GiB。
这是一次功能验收，不是统计性能结论；未改电路/曲线/合约算法或优化参数，未扩B64+，未重跑G1-E。
本轮EVM成功路径总计${sum(successful).toLocaleString('en-US')} Gas（含部署${sum(deploy).toLocaleString('en-US')}），非部署业务${summary.chain.non_deployment_success_gas.toLocaleString('en-US')}；失败验收费用另列原始交易，不计成功路径。
该Gas只属于本次逐票备用路线，不是精确SnarkPack费用，也不与原生Setup/Prove相加。

## Full Tally 对照的边界

使用同一CT2/DKG和同一三份有效份额，对照验证公开完整计票与仅公开Winner的材料。
Full Tally分支包含可用解密包/打开，**不能被Winner-only证书授权**，本轮明确将它定义为反事实公开解密诊断，不是给真实协议增加泄露入口。
诊断JSON留在运行目录private_diagnostic/full_tally.json（目录0700/文件0600），未复制到Git归档；归档只含验证成功、错误计票拒绝、大小及hash。Winner公共重放在独立进程完成，不加载私钥/CP打开/计票。
Winner公共fixture ${n.release_comparison.winner_only_fixture_json_bytes} B、Full Tally诊断fixture ${n.release_comparison.full_tally_diagnostic_json_bytes} B口径不同：Winner还包含缓存VK和60项派生输入，不能直接把两数排名当通信优势。
公开payload扫描未发现CP包/opening/tally/秘密份额字段。研究种子公开，因此这仍不构成运行隐私或Function-Equal定理证明。

## 仍然没有完成的目标功能

1. **精确SnarkPack EVM验证及其接收器整合。** 原生已真实验证SnarkPack，EVM仍走逐票Groth16备用验证，不能悄悄跳过此缺口。
2. **同完整流程的独立DKG/私钥保管。** 五个worker是真进程，但密钥/份额来自集中、确定性的研究bootstrap；并非五个独立Trustee完成本次DKG，不是跨机器生产部署。
3. T4/T5数学安全组合、最终统一比较与干净提交复现。功能通过不替代安全证明。

下一步只围绕上述功能闭环推进，不继续增加小型优化/成本Gate。若精确链上路线预算/语义尚不能闭合，如实保留阻塞，不能把已完成备用路线称为完整目标架构。

## 一条命令运行现有完整功能

\`\`\`sh
# 新目录必须不存在。一次新Setup/Prove，随后完整本地EVM；日志逐阶段保留。
node scripts/run_functional_experiment.mjs /ABS/NEW_RUN_ROOT
# 以下只检查本次归档，不重跑证明或EVM
node scripts/g10c_chain_audit.mjs
node scripts/g10c_report.mjs --check
node scripts/g10c_check.mjs
\`\`\`

本次先实际顺序执行g10c_run.mjs和g10c_chain.mjs，再用--finalize-existing核对衔接；没有伪称新总入口已重新跑过证明阶段。
归档experiments/g10_c_raw，汇总results/g10_c/functional_summary.json；绝对临时路径通过archive_map.json映射，原始结果字节不改。
本轮新增代码未修改旧协议Manifest、旧源码/图表/论文；实际源版本${n.source_version_sha256}，运行时工作区有未提交修改，不声称最终干净源码复现。
`;
function put(p,b){if(mode==='--check'||fs.existsSync(p))assert.equal(fs.readFileSync(p,'utf8'),b,p);else{fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,b,{flag:'wx'});}}
put('results/g10_c/functional_summary.json',JSON.stringify(summary,null,2)+'\n');put('docs/G10_C_CONNECTED_FUNCTIONAL_FLOW.md',md);
console.log(JSON.stringify({status:summary.status,mode,workers:5,new_p3:1,evm_transactions:21,matched_inputs:60,native_proof_matches_chain:true,
 winner_once:true,full_tally_diagnostic_withheld:true,exact_snarkpack_evm:false,independent_distributed_dkg:false,target_architecture_complete:false}));
