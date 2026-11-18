import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
const root='/tmp/vv-pk-profile';
const original='/Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter';
const load=p=>JSON.parse(fs.readFileSync(p,'utf8'));
const write=(p,s)=>fs.writeFileSync(path.join(root,p),s,{flag:'wx'});
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const before=load(root+'/before.json'), integrity=load(root+'/integrity_after.json');
const prof=load(root+'/batch1_profile.json'), outer=load(root+'/outer_wall.json'), compile=load(root+'/compile.json');
const source=load(root+'/diagnostic_source.json'), env=load(root+'/environment.json');
const historical=load(original+'/experiments/stage1_multibatch_raw/gate-b-20261117-02/materials/batch_1.json');
const historicalCommand=load(original+'/experiments/stage1_multibatch_raw/gate-b-20261117-02/materials/prove-worker-1.command.json');
const current=load(root+'/worktree/experiments/stage1_multibatch_raw/pk-profile-batch1/batch_1.json');
assert.equal(compile.returncode,0);assert.equal(outer.returncode,0);assert.equal(prof.status,'PASS_PROFILE_ONLY');
assert.equal(integrity.status,'PASS_ORIGINAL_AND_PINNED_INPUTS_UNCHANGED');
assert.equal(prof.completed_stages.length,16);assert.deepEqual(prof.completed_stages,prof.stage_order);
const total=BigInt(prof.worker_total_ns), sum=prof.stage_order.map(n=>BigInt(prof[n+'_ns'])).reduce((a,b)=>a+b,0n);
assert.equal(total,sum+BigInt(prof.unattributed_worker_ns));
assert.equal(BigInt(outer.outer_wall_ns),BigInt(prof.process_config_ns)+total+BigInt(outer.outer_residual_ns));
assert.ok(prof.worker_peak_rss_mib<=16384);
assert.equal(prof.pk_deserialize_bytes_read,847800624);
const hex=s=>Buffer.from(s.replace(/^0x/,''),'hex');
const proof=hex(current.proof_ark_hex),eipProof=hex(current.eip.proof_neg_a_eip2537_hex);
assert.equal(proof.length,192);assert.equal(eipProof.length,512);
assert.equal(hash(proof),'ff2b7b683b63a172c317612ab63fc093a4fa0599b7fd1faa767050ce059d45ec');
assert.equal(hash(proof),historical.proof_ark_sha256);
assert.equal(current.eip.proof_neg_a_eip2537_hex,historical.eip.proof_neg_a_eip2537_hex);
assert.equal(current.root,'0x91f6c687850b799e58e413c6c9c3b67e1acb1f2cb8a03e2b4d88a3729a9a2134');
function schema(v){if(Array.isArray(v))return v.map(schema);if(v!==null&&typeof v==='object')return Object.fromEntries(Object.keys(v).sort().map(k=>[k,schema(v[k])]));return typeof v;}
assert.deepEqual(schema(current),schema(historical));
const volatile=['witness_ms','prove_ms','verify_ms','prepare_vk_ms','peak_rss_mib'];
const stable=v=>Object.fromEntries(Object.entries(v).filter(([k])=>!volatile.includes(k)));
assert.deepEqual(stable(current),stable(historical));
const rows=[...prof.stage_order,'unattributed_worker'].map(name=>({name,ns:prof[name+'_ns'],ms:prof[name+'_ns']/1e6,seconds:prof[name+'_ns']/1e9,worker_percent:prof[name+'_ns']/prof.worker_total_ns*100,status:'MEASURED'}));
const outerRows=['process_config','worker_total','outer_residual'].map(name=>({name,ns:outer[name+'_ns'],ms:outer[name+'_ns']/1e6,seconds:outer[name+'_ns']/1e9,outer_percent:outer[name+'_ns']/outer.outer_wall_ns*100,status:'MEASURED'}));
const largest=rows.filter(r=>r.name!=='unattributed_worker').sort((a,b)=>b.ns-a.ns)[0];
const warningLog=fs.readFileSync(root+'/compile.stderr.log','utf8');
const warnings=[...warningLog.matchAll(/warning: (?:function|constant) `([^`]+)` is never used/g)].map(m=>m[1]);
const result={status:'PASS_PROFILE_ONLY',created_at:new Date().toISOString(),gate_c:'NOT_AUTHORIZED_NOT_STARTED',
  compile,environment:env,diagnostic_source:source,outer,worker_stages:rows,
  proof_checks:{status:'PASS_BYTE_IDENTICAL_PROOFS',ark_bytes:proof.length,ark_sha256:hash(proof),eip_bytes:eipProof.length,eip_sha256:hash(eipProof),ark_vk_sha256:load(root+'/worktree/experiments/stage1_multibatch_raw/pk-profile-batch1/setup.json').vk_sha256,eip_vk_sha256:hash(hex(current.eip.vk_eip2537_hex)),batch_root:current.root,public_input_count:current.eip.public_input_count,json_schema_unchanged:true,non_timing_non_rss_fields_identical:true,volatile_fields:volatile},
  cpu:{user_ms:prof.pk_deserialize_user_cpu_ms,sys_ms:prof.pk_deserialize_sys_cpu_ms,cpu_to_deserialize_wall_ratio:(prof.pk_deserialize_user_cpu_ms+prof.pk_deserialize_sys_cpu_ms)/prof.pk_deserialize_ms,cpu_not_summed_into_wall:true,bytes_read:prof.pk_deserialize_bytes_read},
  comparison:{historical_wall_ms:historicalCommand.wall_ms,current_wall_ms:outer.outer_wall_ms,difference_ms:outer.outer_wall_ms-historicalCommand.wall_ms,identical_timing_boundary:false,reason:'Historical timer starts before log open/current_exe and includes ps calls per loop; current starts after logs open, before Popen, and polls without ps; both stop at observed exit, nominal 250 ms polling.'},
  largest_measured_stage:largest,integrity,compile_warnings:warnings,invocation_counts:{setup:0,prove_worker_batch_1:1,prove_worker_batch_2:0,gate_c:0,evm:0,git_commit:0,git_push:0},
  limitations:['Single diagnostic, active page cache, not statistical benchmarking or cold-storage measurement.','Canonical deserialization still includes streamed I/O, point decoding and unchanged validation; no validation subcomponent causality claimed.','Deterministic research RNG used only for byte-level reproduction; not production randomness.','Historical timing boundaries are not exactly identical; no controlled speedup or opt-level causal claim.','No PK reuse optimization, Batch 2, EVM or Gate C performed.']};
write('report.json',JSON.stringify(result,null,2)+'\n');
write('stage_timings.csv','stage,ns,ms,seconds,worker_percent,status\n'+rows.map(r=>[r.name,r.ns,r.ms,r.seconds,r.worker_percent,r.status].join(',')).join('\n')+'\n');
const fmt=(n,d=6)=>n.toFixed(d);
const snapshotTable=integrity.protected_paths.map(p=>`| \`${p.path}\` | ${p.status} | ${p.sha256??'—'} | ${p.status==='EXISTS'?'MATCH':'STILL_ABSENT'} |`).join('\n');
const report=`# Batch 1 PK Profile 完整执行报告

最终状态：**PASS_PROFILE_ONLY**。Gate C：**NOT AUTHORIZED / NOT STARTED**。本轮已结束，不自动继续。

## 1. 前置、移植与诊断源码核验

原始根目录：\`${original}\`。临时根目录：\`${root}\`。

HEAD 保持 \`${before.head.trim()}\`；原始 Rust Blob 为 \`${before.original_rust_blob}\`。
三个批准补丁与 A-06 引脚允许路径交集，仅按已获确认的严格字节/哈希一致规则移植；移植前后全部等于 A-06 固定值。未覆盖任何历史记录。

${before.approved_patches.map(p=>`- \`${p.path}\`：\`${p.sha256}\``).join('\n')}

15 个引脚（9 个配置输入 + 6 个 A-06 文件）在原工作区及隔离 Worktree 全部匹配。Cargo.toml/Cargo.lock 未修改；libc 已为直接依赖；autobins 未禁用。已捕获 ${before.untracked.length} 个未跟踪文件的完整字节副本及逐项哈希。单一移植补丁为 \`evidence/approved_patches.diff\`；历史未跟踪文件副本保留于 \`evidence/untracked_bytes/\`，不作为本次 Worker 输入。只给 Worker 原样 request/setup 和只读 PK 符号链接。

独立诊断源码：\`${source.diagnostic_source}\`；SHA-256：\`${source.source_sha256}\`。
\`instrumentation.diff\` SHA-256：\`${source.instrumentation_diff_sha256}\`。差异包括 16 段计时、字节计数、getrusage、独立诊断输出和仅允许 B1 的入口保护；输出 JSON 构造与文件写出分开计时。电路、密码学调用、规范验证、编码、种子、选票顺序不变。

worker 返回 \`Result<()>\`，不返回 ProvingKey/R1CS 大对象；main 中外层计时在 worker 真正返回后停止，涵盖其局部 Drop。完整报告中的结论来自实际单次运行，不是模拟。

## 2. 编译与外层时间分解

编译命令：\`${compile.command.join(' ')}\`，CARGO_TARGET_DIR=\`${root}/target\`，CARGO_BUILD_JOBS=2，RAYON_NUM_THREADS=4，RUSTUP_TOOLCHAIN=1.99.1，Release opt-level=1。

编译退出码 ${compile.returncode}；compile_wall_ms = ${compile.compile_wall_ms}（${fmt(compile.compile_wall_ms/1000)} s），独立于运行墙钟。完整逻辑副本产生 5 条未使用项警告（1 个常量、4 个函数）：${warnings.join(', ')}；没有编译错误。

运行命令：\`${outer.command.join(' ')}\`，cwd=\`${root}/worktree\`；只启动一次，PID=${outer.pid}，退出码=${outer.returncode}。

outer_wall_semantics=\`spawn_to_observed_exit\`，nominal_poll_interval_ms=250，polling_overhead_bound_ms=null。日志在计时前打开；计时从 Popen 前到 poll 观察到退出为止。

outer_wall_ns=${outer.outer_wall_ns}；outer_wall_ms=${outer.outer_wall_ms}；outer_wall_seconds=${fmt(outer.outer_wall_ms/1000)}。

| 分项 | ns | ms | 秒 | 占 outer % |
|---|---:|---:|---:|---:|
${outerRows.map(r=>`| ${r.name} | ${r.ns} | ${fmt(r.ms)} | ${fmt(r.seconds)} | ${fmt(r.outer_percent)} |`).join('\n')}

process_config 从 main 入口至 config 返回，包含参数、路径和配置 9 项引脚校验，不含 OS 创建/动态链接。outer_residual 已用整数纳秒断言非负，含进程创建、动态链接、分发、Profile JSON 写出、退出与轮询误差，不能全部解释为 OS 启动成本。

## 3. Worker 内部 16 阶段与残差

| 阶段 | ns | ms | 秒 | 占 worker % |
|---|---:|---:|---:|---:|
${rows.map(r=>`| ${r.name} | ${r.ns} | ${fmt(r.ms)} | ${fmt(r.seconds)} | ${fmt(r.worker_percent)} |`).join('\n')}

16 阶段互斥；整数断言 \`worker_total_ns = sum(16 stages) + unattributed_worker_ns\` 通过。内部残差包括 setup.json 读取、JSON 对象构造、显式 R1CS Drop、返回前其他局部对象 Drop 及少量未埋点记账。

## 4. PK SHA-256

pk_sha256_ms=${prof.pk_sha256_ms}，为独立顺序读文件及完整性哈希检查，不与反序列化合并。

## 5. PK 反序列化墙钟与边界

pk_deserialize_ms=${prof.pk_deserialize_ms}，包含流式读取、压缩点解码和原有 Validate 检查。没有使用 Validate::No，也未复用内存 PK。不能从本次总耗时推断其中子群检查的独立占比。

## 6. 反序列化 CPU 辅助指标

user_cpu_ms=${prof.pk_deserialize_user_cpu_ms}；sys_cpu_ms=${prof.pk_deserialize_sys_cpu_ms}；累计 CPU / 该阶段墙钟=${fmt(result.cpu.cpu_to_deserialize_wall_ratio)}。
采用 getrusage(RUSAGE_SELF) 区间差值，累计本进程所有线程 CPU 时间，可能大于墙钟；这些指标不加入墙钟求和。

## 7. 实际消费字节数

pk_deserialize_bytes_read=${prof.pk_deserialize_bytes_read}，CountingReader<BufReader<File>> 在 deserialize_compressed 返回后、尾部检查前取值。它记录反序列化器实际消费的逻辑字节，非 OS 物理读盘量。与 PK ${before.inputs[0].byte_size} bytes 一致。

## 8. VK Ark 哈希

vk_ark_hash_ms=${prof.vk_ark_hash_ms}。Ark VK SHA-256：\`${result.proof_checks.ark_vk_sha256}\`；EIP VK SHA-256：\`${result.proof_checks.eip_vk_sha256}\`，均保持既有固定值。

## 9. Witness 与 Prove

witness_diagnostic_ms=${prof.witness_diagnostic_ms}；prove_ms=${prof.prove_ms}，独立报告，不将二者合并为 Prove。不包含 prepare 阶段的 32 份原选票证明验证；本轮没有调用 prepare/Setup。

## 10. Ark Verify 与 EIP 导出

ark_verify_ms=${prof.ark_verify_ms}；eip_export_ms=${prof.eip_export_ms}。仅执行本地 Ark 验证和 EIP-2537 导出检查，没有执行任何 EVM 验证/部署/交易；Gas=NOT_MEASURED。

## 11. 历史对照、归因与 Page Cache

历史 B-02 Batch 1 外层墙钟 ${historicalCommand.wall_ms} ms（约 593.399 s）；本次 ${outer.outer_wall_ms} ms；差值 ${outer.outer_wall_ms-historicalCommand.wall_ms} ms。

历史 spawn_worker 计时开始于打开日志和 current_exe/spawn 之前，每次轮询还包含 ps RSS 检查；新计时开始于日志打开后、Popen 前，轮询不调用 ps。二者均在观察到退出后结束，名义轮询间隔 250 ms，但边界/轮询开销不完全相同，不能作为严格同边界速度提升或单一编译参数因果证据。

本次最大阶段为 **${largest.name}，${fmt(largest.seconds)} s，占 worker ${fmt(largest.worker_percent)}%**；未归因残差为 ${fmt(prof.unattributed_worker_ns/1e9)} s。

执行前 16 项快照及输入哈希读取了 B-02 大型 PK，Worker 又先执行 PK SHA-256 顺序读取。因此本轮是**当前活跃缓存状态下的测量，不是冷缓存磁盘性能**。CPU/墙钟数据可以辅助判别计算与等待，但没有进一步区分解码和群检查；不得将整个阶段全部归因于其中单一算法。

这是一次诊断，不是统计优势结论，也不把新阶段值倒填为历史 593 秒的分项实测。是否采用单次安全加载、多批只读 PK 复用，须下一轮单独授权；本轮未优化，也不推导链上 Gas 收益。

## 12. 证明字节级核验

- Ark：${proof.length} B，SHA-256=\`${hash(proof)}\`，与历史原证明逐字节相同。
- EIP-2537：${eipProof.length} B，SHA-256=\`${hash(eipProof)}\`，与历史导出逐字节相同。
- 批次根：\`${current.root}\`；公开输入数量 ${current.eip.public_input_count}。
- batch_1.json 完整递归字段结构与原版相同；除 ${volatile.join(', ')} 五个测量字段外，其余内容全部一致。没有添加诊断字段到 batch_1.json。

确定性 ChaCha20Rng 仅为重现既有 Batch 1 证明字节、验证埋点零扰动；这是研究复现设置，不是生产安全随机数方案。

## 13. 原工作区与输入最终完整性

HEAD/status/index、全部已跟踪差量及 ${before.untracked.length} 个未跟踪文件内容哈希前后一致。完整前后记录见 before.json / integrity_after.json；所有快照在原始主工作区根目录读取，包含根目录、空目录、ignored 缓存、文件权限/大小/hash、符号链接目标，没有忽略空目录。

| 只读输入 | Bytes | 前后相同 SHA-256 |
|---|---:|---|
${integrity.inputs.map(p=>`| ${path.basename(p.path)} | ${p.byte_size} | ${p.sha256} |`).join('\n')}

| 保护路径 | 状态 | 前后 SHA-256 | 核验 |
|---|---|---|---|
${snapshotTable}

原始 git status（前后逐字相同）：

\`\`\`text
${before.status_short.trimEnd()}
\`\`\`

运行数量：Setup=0，Batch 1 Prove Worker=1，Batch 2=0，EVM=0，commit=0，push=0。Gate C **NOT AUTHORIZED / NOT STARTED**。完成后停止。
`;
write('REPORT.md',report);
const artifacts=['batch1_profile.json','outer_wall.json','instrumentation.diff','diagnostic_source.json','runtime_admission.json','environment.json','compile.json','compile.stdout.log','compile.stderr.log','before.json','integrity_after.json','transplant.json','single_execution_intent.json','batch1.stdout.log','batch1.stderr.log','report.json','stage_timings.csv','REPORT.md','control.mjs','execute.py','report.mjs','worktree/crates/vv-friendly-dkg/src/bin/stage1_multibatch_pk_profile.rs','worktree/experiments/stage1_multibatch_raw/pk-profile-batch1/batch_1.json'];
write('SHA256SUMS',artifacts.map(p=>hash(fs.readFileSync(root+'/'+p))+'  '+p).join('\n')+'\n');
console.log(JSON.stringify({status:result.status,largest_stage:largest,proof_checks:result.proof_checks,outer_wall_s:outer.outer_wall_ms/1000,worker_peak_rss_mib:prof.worker_peak_rss_mib,report:root+'/REPORT.md',gate_c:result.gate_c},null,2));
