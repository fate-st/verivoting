import fs from 'node:fs';
import crypto from 'node:crypto';
import path from 'node:path';
import assert from 'node:assert/strict';
const root='/tmp/vv-pk-reuse-pilot';
const original='/Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter';
const historical=original+'/experiments/stage1_multibatch_raw/gate-b-20261117-02/materials';
const fresh=root+'/worktree/experiments/stage1_multibatch_raw/pk-reuse-pilot-2batch';
const load=p=>JSON.parse(fs.readFileSync(p,'utf8'));
const write=(p,s)=>fs.writeFileSync(root+'/'+p,s,{flag:'wx'});
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const hex=s=>Buffer.from(s.replace(/^0x/,''),'hex');
const p=load(root+'/reuse_pilot_profile.json'),o=load(root+'/outer_wall_rss.json'),c=load(root+'/compile.json');
const integrity=load(root+'/integrity_after.json'),source=load(root+'/diagnostic_source.json'),admission=load(root+'/runtime_admission.json');
const b=load(root+'/before.json'),env=load(root+'/environment.json');
assert.equal(c.returncode,0);assert.equal(o.returncode,0);assert.equal(p.status,'PASS_REUSE_PILOT_ONLY');
assert.equal(integrity.status,'PASS_ORIGINAL_AND_INPUTS_UNCHANGED');
assert.equal(p.pk_load_count,1);assert.deepEqual(p.completed_batches,[1,2]);
assert.equal(p.pk_deserialize_bytes_read,847800624);assert.equal(p.request_sha256_equal,true);
const requestHistorical=sha(fs.readFileSync(historical+'/request.json')),requestPilot=sha(fs.readFileSync(fresh+'/request.json'));
assert.equal(requestHistorical,requestPilot);assert.equal(p.historical_request_sha256,requestHistorical);assert.equal(p.pilot_request_sha256,requestPilot);
assert.equal(sha(fs.readFileSync(root+'/worktree/'+source.source)),source.sha256);assert.equal(source.sha256,admission.source_sha256);
assert.equal(sha(fs.readFileSync(root+'/target/release/stage1_multibatch_pk_reuse_pilot')),admission.binary_sha256);
assert.equal(sha(fs.readFileSync(root+'/reuse_pilot.diff')),source.diff_sha256);
const ns=n=>BigInt(p[n+'_ns']);
assert.equal(BigInt(o.outer_wall_ns),ns('process_config')+ns('worker_total')+BigInt(o.outer_residual_ns));
assert.equal(ns('pk_once_load_total'),p.load_stage_order.map(n=>ns(n)).reduce((a,b)=>a+b,0n)+ns('pk_once_load_unattributed'));
assert.equal(ns('worker_total'),ns('pk_once_load_total')+ns('batch_1_total')+ns('batch_2_total')+ns('unattributed_worker'));
const variable=['witness_ms','prove_ms','verify_ms','prepare_vk_ms','peak_rss_mib','proof_origin','rss_scope'];
function schema(v){if(Array.isArray(v))return v.map(schema);if(v!==null&&typeof v==='object')return Object.fromEntries(Object.keys(v).sort().map(k=>[k,schema(v[k])]));return typeof v;}
const stable=v=>Object.fromEntries(Object.entries(v).filter(([k])=>!variable.includes(k)));
const batches=[];
for(const index of [1,2]){
 assert.equal(p[`batch_${index}_pk_deserialize_ns`],0);assert.equal(p[`batch_${index}_pk_deserialize_ms`],0);
 assert.deepEqual(p[`batch_${index}_completed_stages`],p.batch_stage_order);
 assert.equal(ns(`batch_${index}_total`),p.batch_stage_order.map(n=>ns(`batch_${index}_${n}`)).reduce((a,b)=>a+b,0n)+ns(`batch_${index}_unattributed`));
 const old=load(historical+`/batch_${index}.json`),cur=load(fresh+`/batch_${index}.json`),cmd=load(historical+`/prove-worker-${index}.command.json`);
 assert.deepEqual(schema(old),schema(cur));assert.deepEqual(stable(old),stable(cur));
 const ark=hex(cur.proof_ark_hex),eip=hex(cur.eip.proof_neg_a_eip2537_hex);
 assert.equal(ark.length,192);assert.equal(eip.length,512);
 assert.equal(cur.proof_ark_hex,old.proof_ark_hex);assert.equal(cur.eip.proof_neg_a_eip2537_hex,old.eip.proof_neg_a_eip2537_hex);
 assert.equal(sha(ark),old.proof_ark_sha256);if(index===1)assert.equal(sha(ark),'ff2b7b683b63a172c317612ab63fc093a4fa0599b7fd1faa767050ce059d45ec');
 const stateDigest=x=>sha(Buffer.from(JSON.stringify({root:x.root,old_state:x.old_state,new_state:x.new_state,public_inputs_be_hex:x.eip.public_inputs_be_hex})));
 batches.push({index,status:'MEASURED',historical_wall_ms:cmd.wall_ms,historical_peak_rss_mib:old.peak_rss_mib,
  total_ms:p[`batch_${index}_total_ms`],witness_diagnostic_ms:p[`batch_${index}_witness_diagnostic_ms`],prove_ms:p[`batch_${index}_prove_ms`],ark_verify_ms:p[`batch_${index}_ark_verify_ms`],eip_export_ms:p[`batch_${index}_eip_export_ms`],output_write_ms:p[`batch_${index}_output_write_ms`],pk_deserialize_ms:0,
  ark_bytes:ark.length,eip_bytes:eip.length,historical_ark_sha256:old.proof_ark_sha256,pilot_ark_sha256:sha(ark),historical_eip_sha256:sha(hex(old.eip.proof_neg_a_eip2537_hex)),pilot_eip_sha256:sha(eip),batch_root:cur.root,old_state:cur.old_state,new_state:cur.new_state,
  historical_state_transition_digest:stateDigest(old),pilot_state_transition_digest:stateDigest(cur),public_input_count:cur.eip.public_input_count,schema_match:true,cryptographic_fields_byte_equal:true,process_cumulative_peak_rss_mib:p[`batch_${index}_process_peak_rss_mib`]});
}
assert.deepEqual(batches[0].new_state,batches[1].old_state);
const native=p.worker_peak_rss_mib,physical=o.host_physical_mem_mib;
assert.ok(typeof native==='number'&&native>0&&typeof physical==='number'&&physical>0);
const trace=fs.readFileSync(root+'/rss_samples.jsonl','utf8').trim().split('\n').filter(Boolean).map(l=>JSON.parse(l));
assert.ok(trace.length>0,'no real worker RSS samples');assert.ok(trace.every(x=>x.pid===o.pid));assert.equal(trace.length,o.rss_sample_count);
const peak=trace.reduce((a,x)=>x.rss_mib>a.rss_mib?x:a);assert.equal(peak.rss_mib,o.sampled_peak_rss_mib);assert.equal(peak.timestamp_utc,o.sampled_peak_time_utc);
const baseline=batches.reduce((s,x)=>s+x.historical_wall_ms,0),fixed=p.process_config_ms+p.unattributed_worker_ms+o.outer_residual_ms;
const avg=(batches[0].total_ms+batches[1].total_ms)/2,max=Math.max(...batches.map(x=>x.total_ms));
const grow=Math.max(0,batches[1].process_cumulative_peak_rss_mib-batches[0].process_cumulative_peak_rss_mib);
const swapUsed=s=>{const m=s.match(/used\s*=\s*([0-9.]+)([MG])/);return m?Number(m[1])*(m[2]==='G'?1024:1):null;};
const pageSize=Number(o.vm_stat_before.match(/page size of (\d+) bytes/)?.[1]??0);
const counter=(text,key)=>{const m=text.match(new RegExp(key+':\\s+(\\d+)'));return m?Number(m[1]):null;};
const swapOutBefore=counter(o.vm_stat_before,'Swapouts'),swapOutAfter=counter(o.vm_stat_after,'Swapouts');
const estimate={status:'DERIVED_NOT_EXECUTED',basis:'one measured PK load + 8 times observed mean/max batch time + measured fixed residual; not a bound or acceptance',estimated_8_batch_wall_ms:p.pk_once_load_total_ms+8*avg+fixed,estimated_8_batch_using_slower_observed_batch_ms:p.pk_once_load_total_ms+8*max+fixed,
 measured_peak_if_memory_plateaus_mib:native,linear_highwater_scenario_8batch_mib:native+6*grow,observed_batch2_minus_batch1_native_highwater_mib:grow,physical_minus_measured_peak_mib:physical-native,physical_minus_linear_scenario_mib:physical-(native+6*grow),memory_estimate_is_guarantee:false,
 original_child_seconds:load(root+'/worktree/configs/experiment.STAGE1-MULTIBATCH.json').budgets.child_seconds,gate_c:'NOT_AUTHORIZED_NOT_STARTED'};
const comparison={status:'DERIVED_FROM_MEASURED_SINGLE_RUN_AND_HISTORICAL_RUNS',historical_two_worker_total_ms:baseline,pilot_outer_wall_ms:o.outer_wall_ms,saved_ms:baseline-o.outer_wall_ms,speedup:baseline/o.outer_wall_ms,
 limitation:'cross-run diagnostic comparison, not repeated independent statistics; old worker timers include log opening/RSS polling, new begins after logs open; both include observed-exit polling.'};
const rss={host_physical_mem_mib:physical,worker_peak_rss_mib:native,sampled_peak_rss_mib:o.sampled_peak_rss_mib,sampled_peak_time_utc:o.sampled_peak_time_utc,historical_reference_mib:5513.1875,native_delta_from_historical_mib:native-5513.1875,per_phase_rss:o.per_phase_rss,
 after_pk_load_peak_rss_mib:p.after_pk_load_peak_rss_mib,batch_1_process_peak_rss_mib:p.batch_1_process_peak_rss_mib,batch_2_process_peak_rss_mib:p.batch_2_process_peak_rss_mib,
 host_swap_before:o.swap_before,host_swap_after:o.swap_after,host_swap_used_before_mib:swapUsed(o.swap_before),host_swap_used_after_mib:swapUsed(o.swap_after),host_swap_used_delta_mib:swapUsed(o.swap_after)-swapUsed(o.swap_before),
 host_swapout_pages_before:swapOutBefore,host_swapout_pages_after:swapOutAfter,host_swapout_delta_mib:swapOutBefore!==null&&swapOutAfter!==null?(swapOutAfter-swapOutBefore)*pageSize/1048576:null,
 limitation:'ru_maxrss is cumulative; physical minus RSS is not available system memory; host-global Swap cannot be exclusively attributed to pilot; two batches do not prove bounded eight-batch residency.'};
const allRows=[...p.load_stage_order,'pk_once_load_unattributed'].map(n=>({scope:'load',stage:n,ns:p[n+'_ns'],ms:p[n+'_ms']}));
for(const i of [1,2])for(const n of [...p.batch_stage_order,'unattributed'])allRows.push({scope:'batch_'+i,stage:n,ns:p[`batch_${i}_${n}_ns`],ms:p[`batch_${i}_${n}_ms`]});
allRows.push({scope:'worker',stage:'unattributed_worker',ns:p.unattributed_worker_ns,ms:p.unattributed_worker_ms});
const result={status:'PASS_REUSE_PILOT_ONLY',created_at:new Date().toISOString(),gate_c:'NOT_AUTHORIZED_NOT_STARTED',environment:env,compile:c,source,request_sha256_equal:true,historical_request_sha256:requestHistorical,pilot_request_sha256:requestPilot,pk_load_count:p.pk_load_count,
 pk_once_load_total_ms:p.pk_once_load_total_ms,batch_1_pk_deserialize_ms:0,batch_2_pk_deserialize_ms:0,profile:p,outer:o,batches,comparison,rss,gate_c_estimate:estimate,stage_rows:allRows,integrity,
 invocation_counts:{setup:0,proving_processes:1,groth16_proofs:2,gate_c:0,evm:0,git_commit:0,git_push:0}};
write('report.json',JSON.stringify(result,null,2)+'\n');
write('batch_timings.csv','batch,total_ms,witness_diagnostic_ms,prove_ms,ark_verify_ms,eip_export_ms,output_write_ms,pk_deserialize_ms,status\n'+batches.map(x=>[x.index,x.total_ms,x.witness_diagnostic_ms,x.prove_ms,x.ark_verify_ms,x.eip_export_ms,x.output_write_ms,x.pk_deserialize_ms,x.status].join(',')).join('\n')+'\n');
write('stage_timings.csv','scope,stage,ns,ms,status\n'+allRows.map(x=>[x.scope,x.stage,x.ns,x.ms,'MEASURED'].join(',')).join('\n')+'\n');
const f=(x,d=6)=>x.toFixed(d);
const stageTable=allRows.map(x=>`| ${x.scope} | ${x.stage} | ${x.ns} | ${f(x.ms)} |`).join('\n');
const snapshotTable=integrity.protected_paths.map(x=>`| \`${x.path}\` | ${x.status} | ${x.sha256??'—'} | ${x.status==='EXISTS'?'MATCH':'STILL_ABSENT'} |`).join('\n');
const report=`# PK-Reuse Pilot：两批单次执行完整报告

状态：**PASS_REUSE_PILOT_ONLY**。Gate C：**NOT AUTHORIZED / NOT STARTED**。只运行一次证明进程，顺序生成 Batch 1、Batch 2；完成后停止。没有 Setup、EVM、提交或推送。

## 执行与源码边界

原工作区 \`${original}\`，HEAD \`${b.head.trim()}\`；全部输出在 \`${root}\`。16 项保护路径的前后快照均从原工作区读取。
9 项配置输入和 6 项 A-06 源码引脚在原工作区/独立 Worktree 均匹配。三个批准补丁只按已授权的哈希一致方式移植；Cargo 清单、锁文件、电路、原材料生成器及原始 Setup/证明种子规则未变。

独立源码：\`${source.source}\`，SHA-256 \`${source.sha256}\`；\`reuse_pilot.diff\` SHA-256 \`${source.diff_sha256}\`。
diff 的功能变化仅限新入口中的一次安全加载、两批只读引用复用、计时/CPU/字节采样、进度标签和错误落盘。反序列化调用点只有一处，仍为完整 \`ProvingKey::deserialize_compressed()\`；不使用 unchecked/Validate::No、不克隆 PK。

两份 batch JSON 的字段结构未变。为避免虚假元数据，仅将 proof_origin 如实标为单进程只读复用，rss_scope 如实标为累积进程高水位；其余除原有计时/RSS数值外，所有密码学字段逐字节与历史相同。R1CS 与批内对象在各批函数返回时析构，PK 在整个 worker 返回前析构，均计入相应外层计时。

编译命令：\`${c.command.join(' ')}\`；CARGO_TARGET_DIR=\`${root}/target\`，CARGO_BUILD_JOBS=2，RAYON_NUM_THREADS=4，Rust/Cargo 1.99.1，Release opt-level=1。编译退出码 ${c.returncode}，耗时 ${f(c.compile_wall_ms/1000)} s，未混入运行墙钟。编译日志保留未使用项警告，没有自动 cargo fix 或修改参数。

运行命令：\`${o.command.join(' ')}\`；PID=${o.pid}，退出码 ${o.returncode}。主机 ${env.cpu} / ${physical} MiB / ${env.macos.replaceAll('\n','; ')}。

## 1. 一次性完整 PK 加载

- pk_load_count = **1**。
- pk_once_load_total_ms = **${p.pk_once_load_total_ms}**（${f(p.pk_once_load_total_ms/1000)} s）。
- batch_1_pk_deserialize_ns = **0**，batch_1_pk_deserialize_ms = **0.0**。
- batch_2_pk_deserialize_ns = **0**，batch_2_pk_deserialize_ms = **0.0**。

每批的零表示批内没有反序列化调用，绝不表示共享加载没有成本；真实加载已单独计入 pk_once_load_total。

| 加载指标 | ms |
|---|---:|
${p.load_stage_order.map(n=>`| ${n} | ${f(p[n+'_ms'])} |`).join('\n')}
| load 内部残差 | ${f(p.pk_once_load_unattributed_ms)} |

pk_deserialize_user_cpu_ms=${p.pk_deserialize_user_cpu_ms}，pk_deserialize_sys_cpu_ms=${p.pk_deserialize_sys_cpu_ms}；一次加载总区间 user/sys=${p.pk_once_load_user_cpu_ms}/${p.pk_once_load_sys_cpu_ms} ms。CPU 累计全部线程，不与墙钟相加。
pk_deserialize_bytes_read=${p.pk_deserialize_bytes_read}，在 canonical deserialize 返回后、尾部检查前读取 CountingReader 计数。读取、解码与原有验证均保留；VK/tail 检查均通过。

缓存边界：保护目录快照/输入校验及 Worker 自身 PK SHA-256 均读取了 PK。本轮仍是当前活跃缓存条件；不能宣称冷缓存磁盘性能，也没有独立测量子群检查的耗时占比。

## 2. 两批独立出证与历史对照

| Batch | Witness ms | Prove ms | Ark Verify ms | EIP Export ms | Output ms | 批总计 ms（不含共享加载） | 批内 PK decode ms |
|---|---:|---:|---:|---:|---:|---:|---:|
${batches.map(x=>`| ${x.index} | ${f(x.witness_diagnostic_ms)} | ${f(x.prove_ms)} | ${f(x.ark_verify_ms)} | ${f(x.eip_export_ms)} | ${f(x.output_write_ms)} | ${f(x.total_ms)} | 0.0 |`).join('\n')}

process_config_ms=${p.process_config_ms}；worker_total_ms=${p.worker_total_ms}；unattributed_worker_ms=${p.unattributed_worker_ms}。
outer_wall_ms=${o.outer_wall_ms}，outer_residual_ms=${o.outer_residual_ms}。
外层语义 spawn_to_observed_exit，日志打开后 Popen 前计时，名义 RSS/退出轮询间隔 250 ms，polling_overhead_bound_ms=null。全部整数纳秒残差非负，加载层、批次层、Worker 层和外层的求和恒等式通过。

历史两独立 Worker：${f(batches[0].historical_wall_ms/1000)} s + ${f(batches[1].historical_wall_ms/1000)} s = **${f(baseline/1000)} s**。
本轮含共享加载的完整外层：**${f(o.outer_wall_ms/1000)} s**。
相对历史合计节省 **${f(comparison.saved_ms/1000)} s**，加速比 **${f(comparison.speedup)}×**（DERIVED，自实测值计算）。这不是同轮独立重复统计；旧计时含日志打开与 RSS 探针，新计时起点在日志打开后，不能声称严格配对统计显著优势或推广到完整系统/Gas。

全部互斥子阶段及残差：

| 层级 | 阶段 | ns | ms |
|---|---|---:|---:|
${stageTable}

## 3. RSS、物理内存、Swap 与八批预测

| 项目 | MiB |
|---|---:|
| 主机物理内存 | ${physical} |
| 原生 worker_peak_rss（ru_maxrss） | ${native} |
| 外部 250ms 名义采样峰值 | ${o.sampled_peak_rss_mib} |
| 历史 B-02 Batch 1 原生基准 | 5513.1875（约5513.188） |
| 本轮原生峰值相对历史差值 | ${native-5513.1875} |
| PK 加载后进程累积峰值 | ${p.after_pk_load_peak_rss_mib} |
| Batch 1 返回后进程累积峰值 | ${p.batch_1_process_peak_rss_mib} |
| Batch 2 返回后进程累积峰值 | ${p.batch_2_process_peak_rss_mib} |

外部峰值时间：${o.sampled_peak_time_utc}；共 ${trace.length} 个该 Worker 的有效样本，采样错误 ${o.rss_sampling_errors}。外部采样与 ru_maxrss 为不同口径，不互相替代。

| 外部观测阶段 | 样本数 | 首次 RSS MiB | 末次 RSS MiB | 采样峰值 MiB |
|---|---:|---:|---:|---:|
${Object.entries(o.per_phase_rss).map(([k,x])=>`| ${k} | ${x.sample_count} | ${x.first_rss_mib} | ${x.last_rss_mib} | ${x.sampled_peak_rss_mib} |`).join('\n')}

ru_maxrss 为进程生命周期累积高水位，单独上升不能证明内存泄漏，外部阶段采样也不是精确边界 RSS。仅两个批次不能证明八批常驻内存有界。
主机 Swap 前：\`${o.swap_before}\`；后：\`${o.swap_after}\`。
系统 Swap used 变化 ${rss.host_swap_used_delta_mib} MiB；全机 Swapout 计数变化对应 ${rss.host_swapout_delta_mib} MiB。这些是主机全局指标，运行前已有 Swap 使用，不能把变化独占归因于 Pilot，也不能据此声称进程完全无 Swap。

八批模型（全部 **DERIVED / NOT_EXECUTED**，不是验收数据或严格上界）：

- 按已测一次加载 + 8×两批平均批耗时 + 已测固定残差，预计 ${f(estimate.estimated_8_batch_wall_ms/1000)} s；改用两批较慢者，预计 ${f(estimate.estimated_8_batch_using_slower_observed_batch_ms/1000)} s。
- 若内存驻留平台不再上升，参考已测峰值 ${native} MiB；若仅按两个高水位点的增长 ${grow} MiB/批线性外推，八批场景为 ${estimate.linear_highwater_scenario_8batch_mib} MiB。二者均非承诺/上界。
- 物理内存减已测峰值为 ${estimate.physical_minus_measured_peak_mib} MiB；减线性场景为 ${estimate.physical_minus_linear_scenario_mib} MiB。这只是名义差值，未扣除系统/其他应用，**不是可用内存保证**。不以任意 16 GiB 阈值单独判定成败。
- 原配置每 child 名义预算为 ${estimate.original_child_seconds} s；本次独立 Pilot 未调用旧 spawn_worker。若后续把整个范围证明放入同一 child，应重新审查其计时边界和预算策略，不能自动修改预算或直接放行 Gate C。

## 4. 输入与密码学字节等价

historical_request_sha256=\`${requestHistorical}\`；pilot_request_sha256=\`${requestPilot}\`；request_sha256_equal=**true**。运行前后 request、setup、PK 的历史/Pilot 哈希均一致。

| Batch | Ark 新旧相同 SHA-256（192 B） | EIP-2537 新旧相同 SHA-256（512 B） |
|---|---|---|
${batches.map(x=>`| ${x.index} | ${x.pilot_ark_sha256} | ${x.pilot_eip_sha256} |`).join('\n')}

${batches.map(x=>`Batch ${x.index} 根：\`${x.batch_root}\`；状态/公开输入规范 JSON 摘要：\`${x.pilot_state_transition_digest}\`（新旧相同；该摘要仅为审计，不是新增协议承诺）；公开输入 ${x.public_input_count} 项。`).join('\n\n')}

两个批次的根、old_state/new_state 全部密文分量、公开输入、Ark/EIP VK 和导出字节均逐项一致；Batch 1 new_state 严格等于 Batch 2 old_state。完整字段在 report.json。
两次证明均执行真实 Ark 验证；没有 mock、没有替换验证标志、没有 EVM。每批 RNG 仍为 SHA256(request 原始字节 || batch_index 的 u64 大端字节) 初始化独立 ChaCha20Rng；仅用于研究复现、非生产随机数方案。

## 5. 原工作区完整性与停止

原 HEAD、完整 git status、index、跟踪差量、${b.untracked.length} 个未跟踪文件内容哈希、论文和上一轮 PK Profile 产物均未变。独立 Worktree 中原始材料生成器 Blob 仍为 \`${b.original_rust_blob}\`，15 项引脚与 Cargo 清单/锁文件均未变。

| 保护路径 | 状态 | 前后 SHA-256 | 核验 |
|---|---|---|---|
${snapshotTable}

原始 status 前后逐字一致：

\`\`\`text
${b.status_short.trimEnd()}
\`\`\`

只执行一轮 Pilot，没有自动重试。没有 Setup、8 批扩展、Gate C、EVM、commit 或 push。**Gate C 仍为 NOT AUTHORIZED / NOT STARTED。**
`;
write('REPORT.md',report);
const files=['reuse_pilot_profile.json','outer_wall_rss.json','reuse_pilot.diff','diagnostic_source.json','environment.json','compile.json','compile.stdout.log','compile.stderr.log','runtime_admission.json','single_execution_intent.json','pilot.stdout.log','pilot.stderr.log','rss_samples.jsonl','before.json','input_integrity_before.json','integrity_after.json','report.json','batch_timings.csv','stage_timings.csv','REPORT.md','control.mjs','execute.py','report.mjs','worktree/'+source.source,'worktree/experiments/stage1_multibatch_raw/pk-reuse-pilot-2batch/batch_1.json','worktree/experiments/stage1_multibatch_raw/pk-reuse-pilot-2batch/batch_2.json'];
write('SHA256SUMS',files.map(f=>sha(fs.readFileSync(root+'/'+f))+'  '+f).join('\n')+'\n');
console.log(JSON.stringify({status:result.status,pk_load_count:p.pk_load_count,pk_once_load_total_ms:p.pk_once_load_total_ms,batches,comparison,rss,gate_c_estimate:estimate,report:root+'/REPORT.md'},null,2));
