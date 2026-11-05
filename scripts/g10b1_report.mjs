// Deterministically check/derive the archived experiment; never run cryptography.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
const mode=process.argv[2]??'--check';assert.ok(['--write','--check'].includes(mode));
const root='experiments/g10_b1_raw';const load=p=>JSON.parse(fs.readFileSync(p));
const sha=b=>createHash('sha256').update(b).digest('hex');
const manifest=load(root+'/execution/manifest.json'),raw=load(root+'/matched.json');
const routes=['individual_groth16_per_vote','transaction_batch_individual_groth16','snarkpack_direct_fold','snarkpack_state_proof'];
const titles=['逐票 Groth16 / 逐票累计','B4 提交 / 逐票 Groth16 / Direct Fold','SnarkPack + Direct Fold','SnarkPack + State Proof'];
assert.equal(manifest.status,'PASS_G10_B1_FOUR_ROUTE_INTAKE_AND_ISOLATED_PROCESS_RSS');
assert.equal(manifest.source_base_sha,'b253f60e73f9dec41b80b03d339b2442ce9bfe19');
assert.equal(raw.status,'PASS_FOUR_MATCHED_NATIVE_INTAKE_ROUTES');assert.equal(raw.git_dirty,true);
assert.equal(raw.windows.length,48);assert.equal(raw.method.rounds,12);assert.equal(raw.method.flows_per_window,4);
assert.equal(raw.method.proof_pools,1);assert.equal(raw.method.distinct_ballots,8);assert.equal(raw.rayon_num_threads,'4');
assert.equal(raw.source_version_sha256,sha(Buffer.from(JSON.stringify(raw.sources))));
assert.equal(raw.source_version_sha256,manifest.source_version_sha256);
assert.equal(sha(fs.readFileSync('scripts/g10b1_run.mjs')),manifest.driver_sha256);
assert.equal(sha(fs.readFileSync(root+'/source_snapshot/scripts/g10b1_run.mjs')),manifest.driver_sha256);
for(const s of raw.sources){assert.equal(sha(fs.readFileSync(s.path)),s.sha256,s.path);assert.equal(sha(fs.readFileSync(root+'/source_snapshot/'+s.path)),s.sha256);}
for(const p of raw.fixture_pins.filter(p=>p.path))assert.equal(sha(fs.readFileSync(p.path)),p.sha256,p.path);
assert.equal(manifest.commands.length,9);assert.equal(manifest.runs.length,5);
for(const c of manifest.commands){assert.equal(c.exit_code,0,c.name);assert.equal(c.signal,null);assert.equal(c.error,null);assert.ok(fs.existsSync(root+'/'+c.log));}
assert.match(fs.readFileSync(root+'/execution/tests.log','utf8'),/4 passed; 0 failed/);
for(const entry of manifest.runs){
 const b=fs.readFileSync(root+'/'+entry.file),r=JSON.parse(b);assert.equal(sha(b),entry.sha256);
 assert.equal(r.source_base_sha,manifest.source_base_sha);assert.equal(r.source_version_sha256,manifest.source_version_sha256);
 assert.equal(r.binary_sha256,manifest.binary_sha256);assert.equal(r.all_routes_same_ct1_ct2_and_spent8,true);
 for(const k of ['new_setup','new_proofs','new_aggregates','new_evm'])assert.equal(r[k],0);
 for(const k of ['setup_ms','prove_ms','aggregation_ms','gas','calldata'])assert.equal(r[k],null);
 assert.equal(r.full_lifecycle_comparison,false);assert.equal(r.exact_snarkpack_evm,false);assert.deepEqual(r.final_state,raw.final_state);
 if(entry.route){assert.equal(r.memory_route,entry.route);assert.equal(r.windows.length,0);assert.ok(r.process_peak_rss_mib>0&&r.process_peak_rss_mib<2048);}
}
const fixture=load('fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json');
assert.deepEqual(raw.final_state,fixture.final_state.map(c=>({c0_hex:c.c0_hex,c1_hex:c.c1_hex})));
assert.equal(raw.necessary_negatives.length,4);
for(let i=0;i<4;i++){const n=raw.necessary_negatives[i];assert.equal(n.route,routes[i]);assert.ok(n.checkpoint_unchanged);if(i<2)assert.ok(n.groth16_reached);else assert.ok(n.snarkpack_passed&&n.route_specific_rejected);}
for(let round=1;round<=12;round++){
 const rows=raw.windows.filter(w=>w.round===round);assert.deepEqual(rows.map(w=>w.route),routes.map((_,p)=>routes[(round-1+p)%4]));
 assert.deepEqual(rows.map(w=>w.position),[1,2,3,4]);
 for(const w of rows){const i=routes.indexOf(w.route);assert.equal(w.repetitions,4);assert.equal(w.two_batch_flow_ms,w.window_ms/4);
  assert.equal(w.per_vote_ms,w.two_batch_flow_ms/8);assert.equal(w.validity_calls_per_flow,i<2?8:2);assert.equal(w.checkpoint_writes_per_flow,i===0?8:2);
  for(const k of ['two_batch_flow_ms','packing_ms_per_flow','validity_ms_per_flow','route_specific_ms_per_flow'])assert.ok(Number.isFinite(w[k])&&w[k]>=0);
 }
}
function quantile(a,p){a=[...a].sort((x,y)=>x-y);const i=(a.length-1)*p;return a[Math.floor(i)]+(a[Math.ceil(i)]-a[Math.floor(i)])*(i%1);}
const stats=a=>({n:a.length,median:quantile(a,.5),q1:quantile(a,.25),q3:quantile(a,.75),min:Math.min(...a),max:Math.max(...a)});
const measured=routes.map((route,i)=>({id:route,title:titles[i],two_batch_flow_ms:stats(raw.windows.filter(w=>w.route===route).map(w=>w.two_batch_flow_ms)),
 per_vote_ms:stats(raw.windows.filter(w=>w.route===route).map(w=>w.per_vote_ms)),
 components_ms:Object.fromEntries(['packing_ms_per_flow','validity_ms_per_flow','route_specific_ms_per_flow'].map(k=>[k,stats(raw.windows.filter(w=>w.route===route).map(w=>w[k]))])),
 validity_calls:i<2?8:2,checkpoint_writes:i===0?8:2,process_peak_rss_mib:load(root+'/memory-'+(i+1)+'.json').process_peak_rss_mib,
 communication:raw.communication_components[i]}));
assert.deepEqual(raw.communication_components.map(c=>c.route),routes);
assert.equal(raw.communication_components[0].membership_path_bytes,512);
assert.ok(raw.sender_membership_path_preparation.every(p=>p.prepare_ms>0&&p.four_paths_bytes===256));
const summary={schema:'VV/G10-B1/SUMMARY/v1',status:raw.status,scope:'NATIVE_INTAKE_ONLY_NOT_FULL_SYSTEM',source_base_sha:raw.source_base_sha,
 source_version_sha256:raw.source_version_sha256,source_was_dirty:true,source_snapshot:root+'/source_snapshot',binary_sha256:raw.binary_sha256,
 experiment_manifest:{path:root+'/execution/manifest.json',sha256:sha(fs.readFileSync(root+'/execution/manifest.json'))},
 measured_raw:{path:root+'/matched.json',sha256:sha(fs.readFileSync(root+'/matched.json'))},
 sample_design:{processes_for_timing:1,matched_rounds:12,route_windows:48,flows_per_window:4,timed_flows:192,proof_pools:1,distinct_ballots:8,
 balanced_position_counts_per_route:[3,3,3,3],all_windows_retained:true,development_attempt_not_pooled:true,independent_election_statistics:false},
 routes:measured,necessary_negative_cases:raw.necessary_negatives,unit_tests_passed:4,release_build:true,strict_clippy:true,fmt:true,
 sender_membership_path_preparation:raw.sender_membership_path_preparation,
 memory_scope:raw.rss_scope,full_lifecycle_comparison:false,five_route_system_comparison:false,full_tally_vs_winner_only:false,
 new_setup:0,new_proofs:0,new_aggregates:0,new_evm:0,setup_ms:null,prove_ms:null,aggregation_ms:null,gas:null,calldata:null,
 G9_P1:'BLOCKED',G9_P2_started:false,
 limitations:['single fixed eight-ballot pool, one timing process; descriptive matched pilot only','working source changes pinned by full source snapshots/hashes, not a clean committed-source reproduction',
 'process-specific RSS includes common material loading; no prover or incremental algorithm memory claim','membership path generation measured once per batch separately; not inside intake timing',
 'no setup/prove/aggregation generation costs retimed','no certificate/freeze/threshold/P3/Winner suffix or exact SnarkPack EVM','component bytes are not a complete transport envelope'],
 next_task:'G10-B2: first audit existing generation-cost evidence and set a bounded matched-cost plan; only fill necessary gaps. Full Tally/Winner-only and full lifecycle remain separate.'};
const f=x=>x.toFixed(6);
const md=`# G10-B1：同一八票集合的四路线原生接收比较

状态：${summary.status}。本轮仅实现并测量原生接收，不是完整投票系统/Gas比较。

## 真实测量

全部使用D5/W32、同一DKG上下文、公钥/注册根、同一8张归档选票和25字段输入、两批B4、同一选票VK/状态VK/SnarkPack V2 SRS。
四路线在**同一个新二进制**中重测，不拼接G10-A历史时间。下表单位均为完成8票接收（CT0→CT1→CT2）的毫秒。

| 路线 | 中位数 ms | Q1 / Q3 ms | 每票中位数 ms | 实际写入次数 | 独立进程峰值 MiB |
| --- | ---: | ---: | ---: | ---: | ---: |
${measured.map(r=>`| ${r.title} | ${f(r.two_batch_flow_ms.median)} | ${f(r.two_batch_flow_ms.q1)} / ${f(r.two_batch_flow_ms.q3)} | ${f(r.per_vote_ms.median)} | ${r.checkpoint_writes} | ${r.process_peak_rss_mib.toFixed(3)} |`).join('\n')}

**当前B4接收中两条逐票验证路线更快，SnarkPack和状态证明没有在本轮显示原生接收优势。**
不能预设Proposed更快；也不能外推到B16/B32、公链或完整系统。逐票写入是内存checkpoint写入，不是8笔EVM交易。

## 验证工作与可比性

- Legacy不是改名的B4 Receiver：真实逐票Groth16验证、逐票六个Jubjub分量累计、逐票NF和cursor更新，共8次原子提交，第4/8票对照CT1/CT2。
- 单票针对原有已接受B4根验证位置和深度2成员路径。原LEAF/NODE v2域、规范25字段编码不变；运行加载和单元测试均与原生产batch_root逐项匹配。路径验证/编码在接收计时内；8条路径512 B。路径构造在发送端预处理，单独计时${raw.sender_membership_path_preparation.map(p=>`batch${p.batch_index}=${f(p.prepare_ms)}ms`).join('、')}，不隐藏为免费。
- Transaction-batch真实验证8份原单票证明，根/旧状态/上下文/NF共同检查及Direct Fold后，两次B4原子提交。不是把组件耗时相加。
- 两条SnarkPack路线继续使用原Receiver的共同检查和真实聚合验证；分别直接累计或验证原两份状态证明。每条成功流程确认CT1/CT2和8个鲜活NF。
- 四个必要密码学负例：两条新路线均用原有效record/root和另一选民的原证明，实际Groth16返回false；两条聚合路线用错误newState，SnarkPack成功后Direct/State分别拒绝。失败时CT/NF/cursor保持不变，不使用冻结或缺失认证短路来冒充密码学拒绝。
- 4项相关单元测试通过，覆盖原根路径/位置、逐票原子写入及旧Receiver的连续性/跨批重复NF。不是新增大规模攻击矩阵。

## 采样、版本及边界

一个计时进程，每路线12窗口，每窗口4次完整8票流程；四路线循环轮换顺序，每个位置各出现3次，各预热2次。48窗口/192次流程仍只是一组8票证明池；无慢样本过滤、无IID选举显著性结论。
另4个新进程分别仅运行一路的1次预热/4次流程记录RSS；共同loader会加载所有路线材料，故该RSS含共同加载，不是增量算法内存，也不是Prove内存。
构建Rust1.99.1 Release/locked/offline，jobs=2、Rayon=4。原始日志记录实际命令、退出码、计时和硬件。

基准Git：${raw.source_base_sha}；执行时工作区存在本轮未提交修改，raw诚实保存git_dirty=true，**不宣称干净已提交源码复现**。
执行源码版本SHA-256：${raw.source_version_sha256}；二进制SHA-256：${raw.binary_sha256}。
全部9项源码/锁文件加驱动原字节快照在${root}/source_snapshot，校验脚本逐项检查，不用基准HEAD冒充新源码版本。

首轮开发试跑保存在${root}/development_attempt1，随后完善实际记录输入绑定、拒绝路径证据和发送端路径成本，才执行最终版本。首轮源码hash不同，故不混入最终统计；不是按速度挑选样本。
本轮没有新Setup/Prove/聚合生成/EVM；相关生成耗时为null，不是0。通信仅分项：单票proof、aggregate、matrix、路径、state proof和CT，不把派生输入重复累加，完整网络封包/calldata未测。
冻结/门限/P3/Winner后缀、Full Tally对比、匹配生成负担、独立证明池和完整系统复现仍未完成。G9-P1仍BLOCKED，P2未启动。旧实验/Manifest/论文和旧图不改。

## 复现与产物

\`\`\`sh
# 新结果目录必须不存在；锁文件依赖已在本机缓存
node scripts/g10b1_run.mjs /ABS/NEW_OUTPUT_DIRECTORY
# 下列只核查本次归档，不重跑证明/性能
node scripts/g10b1_report.mjs --check
node scripts/g10b1_check.mjs
\`\`\`

原始数据：${root}/matched.json及memory-1..4.json；执行命令：${root}/execution/manifest.json；统计：results/g10_b1/summary.json。
下一步先核对已有生成成本及缺口，固定必要预算后补匹配成本；不继续扩B64+、不重跑旧规模实验或软件Miller路线。
`;
function put(p,data){if(mode==='--check'||fs.existsSync(p))assert.equal(fs.readFileSync(p,'utf8'),data,p);else{fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,data,{flag:'wx'});}}
put('results/g10_b1/summary.json',JSON.stringify(summary,null,2)+'\n');put('docs/G10_B1_MATCHED_NATIVE_BASELINES.md',md);
console.log(JSON.stringify({status:summary.status,mode,rounds:12,windows:48,proof_pools:1,medians_ms:measured.map(r=>({route:r.id,ms:r.two_batch_flow_ms.median})),new_setup:0,new_proofs:0,new_evm:0}));
