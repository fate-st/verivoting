// Derive descriptive statistics only from the actual matched-path Release run.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
const mode=process.argv[2]??'--check';assert.ok(['--write','--check'].includes(mode));
const path='experiments/g10_a_raw/native_direct_fold_b4.json';
const bytes=fs.readFileSync(path),raw=JSON.parse(bytes);
assert.equal(raw.status,'PASS_MATCHED_TWO_B4_NATIVE_RECEIVE_PATHS');assert.equal(raw.windows.length,20);
assert.ok(raw.both_paths_final_ct2_equal);assert.equal(raw.evm_executed,false);
assert.equal(raw.setup_count,0);assert.equal(raw.proof_generation_count,0);assert.equal(raw.aggregate_generation_count,0);
assert.equal(raw.gas,null);assert.equal(raw.necessary_negatives.length,2);
assert.ok(raw.necessary_negatives.every(n=>n.snarkpack_verified&&n.route_specific_check_rejected&&n.checkpoint_unchanged&&!n.frozen_short_circuit));
const sha=b=>createHash('sha256').update(b).digest('hex');
for(const p of raw.sources)assert.equal(sha(fs.readFileSync(p.path)),p.sha256,p.path);
for(const p of raw.fixture_pins.filter(p=>p.path))assert.equal(sha(fs.readFileSync(p.path)),p.sha256,p.path);
const quantile=(a,p)=>{a=[...a].sort((x,y)=>x-y);const i=(a.length-1)*p;return a[Math.floor(i)]+(a[Math.ceil(i)]-a[Math.floor(i)])*(i%1);};
const stats=a=>({n:a.length,median:quantile(a,.5),q1:quantile(a,.25),q3:quantile(a,.75),min:Math.min(...a),max:Math.max(...a)});
const routes={};
for(const route of ['snarkpack_direct_fold','snarkpack_state_proof']){
 const w=raw.windows.filter(x=>x.route===route);assert.equal(w.length,10);
 for(const x of w){assert.equal(x.repetitions,8);assert.ok(x.two_batch_flow_ms>0);assert.equal(x.two_batch_flow_ms,x.window_ms/8);assert.equal(x.per_vote_ms,x.two_batch_flow_ms/8);}
 routes[route]=Object.fromEntries(['two_batch_flow_ms','per_vote_ms','votes_per_second','packing_ms_per_flow','snarkpack_ms_per_flow','route_specific_ms_per_flow'].map(k=>[k,stats(w.map(x=>x[k]))]));
}
const ratios=[];
for(let round=1;round<=10;round++){
 const w=raw.windows.filter(x=>x.round===round);assert.equal(w.length,2);
 assert.deepEqual(w.map(x=>x.route),round%2?['snarkpack_direct_fold','snarkpack_state_proof']:['snarkpack_state_proof','snarkpack_direct_fold']);
 ratios.push(w.find(x=>x.route==='snarkpack_state_proof').two_batch_flow_ms/w.find(x=>x.route==='snarkpack_direct_fold').two_batch_flow_ms);
}
const report={schema:'VV/G10-A/SUMMARY/v1',status:raw.status,source:{path,sha256:sha(bytes),base_sha:raw.source_base_sha,dirty:raw.git_dirty},routes,paired_state_over_direct:stats(ratios),process_peak_rss_mib:raw.process_peak_rss_mib,
 sample_unit:'one window averages eight complete two-B4 receive flows; ten paired windows/route, fixed proofs, not independent elections',
 no_outlier_filter:true,full_native_receive_path_measured:true,total_time_is_not_component_sum:true,
 new_setup:0,new_proofs:0,new_aggregates:0,new_evm:0,generation_costs:'REUSED_NOT_RETIMED_NOT_ZERO',gas:null,
 full_system_or_privacy_claim:false,exact_snarkpack_evm_verified:false};
const f=x=>x.toFixed(6),d=routes.snarkpack_direct_fold,s=routes.snarkpack_state_proof;
const md=`# G10-A：同一 W32 两批 B4 的原生 Direct Fold 消融\n\n状态：${raw.status}。基准 ${raw.source_base_sha}，本轮执行时 dirty=${raw.git_dirty}，真实源码和二进制哈希保留在 raw。\n\n## 实测结果\n\n| 路线 | 两批完整接收中位数 ms | Q1/Q3 ms | 每票中位数 ms | 接收吞吐量票/s |\n| --- | ---: | ---: | ---: | ---: |\n| SnarkPack + Direct Fold | ${f(d.two_batch_flow_ms.median)} | ${f(d.two_batch_flow_ms.q1)} / ${f(d.two_batch_flow_ms.q3)} | ${f(d.per_vote_ms.median)} | ${f(d.votes_per_second.median)} |\n| SnarkPack + State Proof | ${f(s.two_batch_flow_ms.median)} | ${f(s.two_batch_flow_ms.q1)} / ${f(s.two_batch_flow_ms.q3)} | ${f(s.per_vote_ms.median)} | ${f(s.votes_per_second.median)} |\n\n配对窗口 State/Direct 比值中位数 ${f(report.paired_state_over_direct.median)}。这是同一固定证明池的描述性结果，不是完整投票系统加速比或统计显著性结论。\n本轮状态证明路线更慢，不能声称原生验证效率优势。首轮两路径均较慢，全部样本保留，不过滤或替换。\n\n## 同环境和计时边界\n\n- 两条路线均实际执行相同 G2-E Receiver：初始化、批次序号/旧CT/上下文/注册根/接受根/Nullifier检查；从真实记录重建矩阵和原transcript；真实SnarkPack验证；原子写checkpoint；最终CT2及8个NF检查。\n- 仅增量不同：candidate-major独立Direct Fold重做六分量累计并比较new CT，或从当前提交对象重建37项输入并验证原Groth16状态证明。旧Receiver、折叠实现和所有密码学关系未改。\n- 每路线2次预热、10个交替AB/BA配对轮；每窗口8次完整两批流程，共20个原始窗口。不把160次流程误报成160场独立选举。两路径对应调用使用相同研究RNG种子，但并行accumulator任务调度不保证随机系数分配逐等式相同；它不是生产随机性策略。\n- 总时间直接包围完整接收流程，不用历史组件时间相加；packing/SP/增量分项只用于解释，不回填total。离线排除文件解码和VK预处理。\n- ${raw.hardware}，${raw.hardware_memory_bytes} B RAM；Rust 1.99.1 Release/locked/offline；RAYON_NUM_THREADS=${raw.rayon_num_threads}。进程累计峰值 ${raw.process_peak_rss_mib.toFixed(3)} MiB，包括共同加载材料和两路线，不能当逐路线内存比较。\n\n## 必要正确性验收\n\n两路径均到达同一G2-E CT2。仅修改第一批new CT的一个分量、保留原aggregate/状态proof/记录/根；在正式接受前提交。\n两次SnarkPack均实际通过，之后Direct Fold或Groth16状态关系拒绝；next=1、spent=0和旧CT均不变。\n这不是冻结后的FROZEN拒绝，也不是session ID不匹配。没有增加攻击/故障矩阵。\n\n## 成本和复用口径\n\n复用两份aggregate、两份state proof、同一ballot VK/state VK及B4 verifier SRS；新Setup/Prove/聚合生成均为0次。\n生成时间未重测，以null和REUSED_NOT_RETIMED标记，不表示生成成本为零。\n每B4公共矩阵3200 B、old+new CT分量384 B；State Proof额外192 B。37项state statement由已绑定数据派生，不能再与矩阵/CT重复相加当通信总量。\n这些仅是二进制分项，未定义完整网络封包/DA/calldata，故Gas和calldata保持null。没有EVM、Full Tally/Winner-only或五方案系统比较。\n\n## 复现\n\n\`\`\`sh\nCARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 build --release --locked --offline -p vv-friendly-dkg --bin friendly-g10a-native-fold\nRAYON_NUM_THREADS=4 target/release/friendly-g10a-native-fold --output experiments/g10_a_reproduce/NEW_RUN.json\nnode scripts/g10a_report.mjs --check\n\`\`\`\n\n源码/原始窗口：${path}；摘要：results/g10_a/summary.json；真实命令/失败构建日志：results/g10_a/execution。\n现有报告脚本核对已归档run；新路径复现实验须单独分析，不能覆盖原始测量。\n本轮初次构建只因JSON宏默认递归限制失败，修正为512后编译、严格Clippy及6项相关测试通过；失败日志保留。\n\n## 停止边界\n\n只完成原生B4 Direct Fold与State Proof公平增量比较。不扩B8状态或B64+，不重跑G1-E，不从旧Gas拼出系统优势。\nG9-P1仍需闭合精确关系和有界EVM资源；G9-P2/P3不启动。\n`;
function put(p,b){if(mode==='--check'||fs.existsSync(p))assert.equal(fs.readFileSync(p,'utf8'),b,p);else{fs.mkdirSync(p.slice(0,p.lastIndexOf('/')),{recursive:true});fs.writeFileSync(p,b,{flag:'wx'});}}
put('results/g10_a/summary.json',JSON.stringify(report,null,2)+'\n');put('docs/G10_A_NATIVE_DIRECT_FOLD.md',md);
console.log(JSON.stringify({status:report.status,mode,windows:20,direct_two_batch_ms:d.two_batch_flow_ms.median,state_two_batch_ms:s.two_batch_flow_ms.median,paired_state_over_direct:report.paired_state_over_direct.median,gas:null}));
