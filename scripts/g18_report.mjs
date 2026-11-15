// Immutable G18 evidence report/checker. No proof, VM, contract or Git mutations.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
process.chdir(path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'));
const mode=process.argv[2]??'--check';assert.ok(['--write','--check'].includes(mode));
const run='experiments/g18_raw/run-20261115-01',out='results/g18';
const read=p=>fs.readFileSync(p),load=p=>JSON.parse(read(p));
const sha=b=>createHash('sha256').update(b).digest('hex');
const pin=p=>({path:p,sha256:sha(read(p)),bytes:read(p).length});
const encode=j=>JSON.stringify(j,null,2)+'\n';
const store=(p,b)=>{
 b=Buffer.isBuffer(b)?b:Buffer.from(b);
 if(mode==='--write'&&!fs.existsSync(p)){fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,b,{flag:'wx'});}
 else assert.deepEqual(read(p),b,p);
};
const m=load(`${run}/execution.json`),config=load('configs/experiment.G18-DECISION.json');
assert.equal(m.config.sha256,sha(read(m.config.path)));assert.equal(config.frozen,true);
assert.equal(config.decision.gas_advantage_minimum_fraction,0.15);
assert.equal(config.budgets.proof_seconds,1800);assert.equal(config.budgets.proof_memory_mib,24576);
assert.equal(config.budgets.guest_seconds,600);assert.equal(config.budgets.guest_memory_mib,16384);
assert.equal(m.status,'STOP_G18_1_REQUIRED_GROTH16_ENVIRONMENT_UNAVAILABLE');
assert.equal(m.engineering_repair_retests,0);assert.equal(m.historical_files_unchanged,true);
const historical=load(`${run}/historical_files.json`);
for(const e of historical)assert.equal(sha(read(e.path)),e.sha256,'Historical file changed: '+e.path);
for(const e of m.sources){assert.equal(sha(read(e.path)),e.sha256);assert.deepEqual(read(`${run}/source_snapshot/${e.path}`),read(e.path));}
for(const e of config.input_pins)assert.equal(sha(read(e.path)),e.sha256,e.path);
const lock=load(`${run}/isolated_lock.json`);assert.equal(sha(read(lock.path)),lock.sha256);
assert.deepEqual(read(`${run}/source_snapshot/${lock.path}`),read(lock.path));
for(const e of config.official_smoke.official_files){
 const archived=`${run}/upstream/${e.package}__${e.file.replaceAll('/','__')}`;
 assert.equal(sha(read(archived)),e.sha256,archived);
}
const pre=load(`${run}/environment_preflight.json`),smoke=load(`${run}/official_smoke/result.json`);
assert.equal(pre.required_groth16_wrapper_available,false);assert.equal(pre.environment_admitted,false);
assert.equal(pre.arch,'arm64');assert.equal(pre.platform,'darwin');assert.equal(pre.docker_cli_available,false);
assert.equal(smoke.status,'PASS_REAL_OFFICIAL_COMPOSITE_RECEIPT');assert.equal(smoke.proof_generated,true);
assert.equal(smoke.receipt_verified,true);assert.equal(smoke.journal_verified,true);assert.equal(smoke.dev_mode,false);
assert.equal(smoke.remote_prover,false);assert.equal(smoke.ballot_proof_test,false);assert.equal(smoke.evm_wrapper_test,false);
const receipt=pin(`${run}/official_smoke/receipt.bincode`),program=load(`${run}/official_smoke/program.json`);
assert.equal(receipt.sha256,smoke.receipt_sha256);assert.equal(receipt.bytes,smoke.receipt_bytes);
assert.equal(program.image_id,smoke.image_id);assert.equal(program.program_sha256,smoke.program_sha256);
assert.equal(sha(read(`${run}/official_smoke/program.bin`)),program.program_sha256);
assert.equal(sha(read(`${run}/official_smoke/guest.elf`)),config.official_smoke.user_elf_sha256);
assert.equal(read(`${run}/official_smoke/input.bin`).toString('hex'),'00000000');
assert.equal(m.commands.filter(c=>c.name==='official-real-proof-smoke').length,1);
for(const c of m.commands){assert.equal(c.code,0,c.name);assert.equal(c.termination,null);assert.equal(c.signal,null);assert.equal(c.spawn_error,null);}
const command=m.commands.find(c=>c.name==='official-real-proof-smoke');
assert.equal(command.budget_seconds,config.budgets.proof_seconds);assert.equal(command.budget_memory_mib,config.budgets.proof_memory_mib);
assert.ok(command.wall_ms<1800000);assert.ok(command.sampled_max_process_tree_rss_mib<24576);
const g17=load('results/g17_z1_recovery/guest_execution.json');
const admission={schema:'VV/G18/PROVER-ADMISSION/v1',status:'BLOCKED_REQUIRED_GROTH16_ENVIRONMENT',
 config:m.config,baseline_commit:m.baseline_commit,branch:m.branch,execution:pin(`${run}/execution.json`),
 environment:pre,local_stark_backend_actual_test:'PASS_REAL_OFFICIAL_COMPOSITE_RECEIPT',
 official_smoke:{...smoke,receipt,program:pin(`${run}/official_smoke/program.bin`),
  user_elf:pin(`${run}/official_smoke/guest.elf`),command,
  not_a_ballot_proof:true,not_a_route_a_or_b_benchmark:true,samples:1,
  memory_scope:'Sampled host process RSS only; no exact peak or separate GPU-memory measurement',gpu_memory_bytes:null},
 b1_ballot:{status:'NOT_RUN_REQUIRED_ENVIRONMENT_GATE',receipt:null,prove_ms:null,peak_rss_mib:null,
  receipt_bytes:null,verification_ms:null,
  reason:'Frozen full EVM-capable environment gate is unavailable; no large original B1 proof attempted',
  original_g17_program_reference:g17.program,previous_execute_only:pin('results/g17_z1_recovery/guest_execution.json')},
 groth16_wrapper:{status:'BLOCKED_APPLE_SILICON_UNSUPPORTED',available:false,runtime_attempted:false,
  failure_type:'UNSUPPORTED_FIXED_PLATFORM_AND_MISSING_DOCKER',proof:null,prove_ms:null,bytes:null,
  evidence:{actual_probes:pin(`${run}/environment_preflight.json`),official_sources:config.official_sources,
   locked_source:config.official_smoke.official_files.at(-1)},
  no_dummy_seal_or_old_wrapper_used:true},
 engineering_repair_retests:0,environment_admitted:false,admit_functional_stage:false,
 interpretation:'STARK proving capability is demonstrated only for the official 169-user-cycle control. Full B1 receipt, Groth16 wrapping and A/B feasibility remain untested.'};
const functional={schema:'VV/G18/FUNCTIONAL-ADMISSION/v1',status:'NOT_RUN_G18_1_ENVIRONMENT_GATE',
 config_sha256:m.config.sha256,routes:{
  A:{status:'NOT_RUN',snarkpack_semantics_review:'NOT_RUN',full_checks:null,tests:null,functionally_admitted:false},
  B:{status:'NOT_RUN',g17_execute_only_reference:pin('results/g17_z1_recovery/guest_execution.json'),
   authenticated_context_and_jubjub_fold:null,tests:null,functionally_admitted:false}},
 guest_sizes_tested_this_stage:null,guest_cycles:null,guest_ms:null,guest_rss_mib:null,input_bytes:null,
 no_new_guest_or_cryptographic_simplification:true,admit_gas_comparison:false};
const metrics=()=>({deployment_gas:null,authentication_gas:null,two_batch_receive_gas:null,
 proof_verifier_gas:null,freeze_gas:null,total_matched_lifecycle_gas:null,amortized_gas_per_ballot:null,
 calldata_bytes:null,data_availability_bytes:null,ballot_prove_ms:null,snarkpack_aggregate_ms:null,
 guest_execute_ms:null,zkvm_prove_ms:null,wrapper_prove_ms:null,prover_memory_mib:null,
 communication_bytes:null,reused_deployment_cost:null});
const costs={schema:'VV/G18/PROOF-AND-EVM-COSTS/v1',status:'NOT_RUN_G18_1_ENVIRONMENT_GATE',
 config_sha256:m.config.sha256,A:{status:'NOT_RUN',metrics:metrics()},B:{status:'NOT_RUN',metrics:metrics()},
 C:{status:'NOT_REEXECUTED_NO_ADMITTED_CANDIDATE',metrics:metrics(),historical_reference:config.historical_c_reference,
  historical_numbers_not_used_for_new_matched_ranking:true},
 new_evm_transactions:0,signed_transaction_receipts:null,official_verifier_deployed:null,
 official_smoke_not_used_as_application_cost:true,gas_savings_fraction:null,threshold:0.15,
 literal_zero_scope:'Zero is an actual execution count, not an unmeasured Gas/time/cost value'};
const decision={schema:'VV/G18/FINAL-ARCHITECTURE-DECISION/v1',
 decision:'INCONCLUSIVE_RESOURCE_OR_ENVIRONMENT',stage_reached:'G18-1',config_sha256:m.config.sha256,
 baseline_commit:m.baseline_commit,branch:m.branch,
 basis:['One official 3.1.0 Composite receipt was actually generated and verified locally with dev mode disabled',
  'The fixed Apple M4/arm64 environment lacks the required supported official Groth16 shrink-wrap backend; no Docker/CUDA',
  'The frozen environment admission failed, so original B1 proving, A/B full-function guests and matched EVM comparison were not started'],
 evidence:{prover_admission:`${out}/prover_admission.json`,functional_admission:`${out}/functional_admission.json`,
  proof_and_evm_costs:`${out}/proof_and_evm_costs.json`,execution:`${run}/execution.json`},
 functionality_evidence:'Official control proof only. Existing G17 B1/B2 execution is historical reference, not full G18 B validity/fold/state admission.',
 performance_evidence:{official_control_only:{prove_ms:smoke.proof_ms,verify_ms:smoke.verify_ms,receipt_bytes:receipt.bytes,
  sampled_process_tree_rss_mib:command.sampled_max_process_tree_rss_mib},
  route_a_costs:null,route_b_costs:null,new_route_c_costs:null,gas_savings_fraction:null},
 limitations:['No original ballot real receipt was generated in G18',
  'No official Groth16 wrapper or EVM transaction was run',
  'No A/B SnarkPack/full Jubjub state guest was implemented or admitted',
  'The tiny control receipt cannot predict B1/B2/B4 proving time, memory or Gas',
  'RSS is sampled host memory; GPU allocation and exact peak were not measured',
  'The 15% full-lifecycle Gas threshold has not been evaluated',
  'No final architecture selection or cryptographic impossibility claim is justified'],
 current_implemented_baseline:'C preserved unchanged; not selected as a measured performance winner',
 budget_or_threshold_changed_after_start:false,original_protocol_or_paper_changed:false,
 old_tracked_files_preserved:historical.length,engineering_repair_retests:0,
 stopped:true,g19_started:false,automatic_hardware_change:false,automatic_backend_change:false,
 auto_commit:false,auto_push:false,
 continuation_requires:'New explicit authorization and a separately frozen common supported target environment; do not revise this run/config or splice different-machine numbers'};
store(`${out}/prover_admission.json`,encode(admission));store(`${out}/functional_admission.json`,encode(functional));
store(`${out}/proof_and_evm_costs.json`,encode(costs));store(`${out}/final_decision.json`,encode(decision));
const doc=`# G18：最终证明架构决策实验

## 结论：INCONCLUSIVE_RESOURCE_OR_ENVIRONMENT

本轮在 G18-1 的环境准入处停止。**本机真实 Composite 证明器可用，但当前固定 Apple M4 环境不能提供官方支持的 EVM Groth16 包装。**
这不是 A/B 的密码学失败，也不是 C 的性能胜出；没有足够证据选 A、B 或宣称新架构更高效。
本轮未进入 G18-2/3/4，未自动安装 Docker、改用远程证明服务或 Linux，不开启 G19。

## 1. 固定基准、配置与原数据

基准提交：\`${m.baseline_commit}\`；独立分支：\`${m.branch}\`。执行源为该提交加本轮未提交的新文件，不称干净提交复现。
配置：\`configs/experiment.G18-DECISION.json\`，SHA-256：\`${m.config.sha256}\`。正式记录开始后未改字节。
固定 D5/W32、32名注册选民、原8票、3候选人、两个连续 B4；原 VK、全部25项输入、认证上下文、批次根、CT2及冻结目标均按原归档锁定。
八份原证明/有序输入逐项与 G2-E 及 G8 EIP 编码交叉核对；CT2 一致。未更换数据集、曲线、原 SnarkPack V2 或 Jubjub 累计定义。
SDK/r0vm3.1.0、risc0-build3.0.7、guest Rust1.97.0、host Rust1.99.1、Prague固定。原配置、锁、合约、G8—G17与论文未修改。

固定预算：guest600秒/16GiB、真实proof1800秒/24GiB；每路线最多一次正式实现和一次明确工程修复复测，实用修复复测次数为0。
监控为每秒主机单进程/进程树 RSS 与墙钟软限制，不是内核硬限或GPU内存计量。15%同条件总生命周期Gas门槛未变，未因失败提高预算。

## 2. 本机包装后端预检

机器：${config.environment.cpu}，${config.environment.physical_memory_bytes/1024**3}GiB，${config.environment.platform}/${config.environment.arch}。
实际 probes 保留 r0vm版本/能力、系统版本、Docker/CUDA及开发工具结果；原官方 r0vm 二进制摘要与 G17 一致。
Docker 与 nvidia-smi 实际不存在；xcrun 找不到 metal 编译工具，但这**不被用作“普通证明不支持”的推断**，其能力由下面实际证明检验。

[官方3.1本地证明文档](https://dev.risczero.com/api/generating-proofs/local-proving)区分普通CPU/Metal证明与只支持x86的Groth16包装，并明确排除Apple Silicon，即使使用Docker。
[官方v3.1.0发布说明](https://github.com/risc0/risc0/releases/tag/v3.1.0)给出配套新设置、Docker镜像及新链上验证器版本；没有沿用旧包装或旧VK。
锁定 \`risc0-groth163.0.6/src/prove/docker.rs\` 同样要求x86上的Docker；安装源码逐字节匹配原G17锁中的官方crate归档，副本与摘要已保存。
所以包装标为 \`BLOCKED_APPLE_SILICON_UNSUPPORTED\`，另有Docker缺失。未执行包装调用、未伪造一个包装失败交易或Gas数字。

## 3. 一次最小官方真实证明自测

采用锁定SDK自带 \`examples/loop.bin\`，按官方datasheet方式与原 \`V1COMPAT_ELF\` 组合，输入原始4字节零、循环计数0、预期空journal。
这是官方最小控制程序，不是修改后的选票验证器，也不是A/B功能用例。官方二进制/源码及crate归档摘要均与原锁匹配。
薄host适配器显式调用同一个本地 \`ExternalProver\`；SDK启用 \`disable-dev-mode\`，运行环境及ProverOpts也关闭dev-mode，拒绝Fake/非Composite类型。
证明生成后执行SDK \`Receipt::verify(image_id)\`，并核对空journal，保留真实bincode Receipt。

| 实测项 | 本轮值 |
| --- | --- |
| 真实 Composite 生成 | PASS |
| Receipt / Image ID / journal 验证 | PASS |
| Prove | ${(smoke.proof_ms/1000).toFixed(6)}秒 |
| Receipt Verify | ${smoke.verify_ms.toFixed(3)}毫秒 |
| Receipt序列化大小 | ${receipt.bytes} B |
| user / total cycles | ${smoke.stats.user_cycles} / ${smoke.stats.total_cycles} |
| 段数 | ${smoke.stats.segments} |
| 采样单进程 / 进程树RSS | ${command.sampled_max_single_process_rss_mib.toFixed(3)} / ${command.sampled_max_process_tree_rss_mib.toFixed(3)} MiB |

只有一次功能样本，RSS只有${command.rss_sample_count}个采样点，不是精确峰值/GPU内存。Prove不是Execute时间；构建${(m.commands.find(c=>c.name==='build-official-smoke-host').wall_ms/1000).toFixed(3)}秒单列，不计入Prove。
这个小程序仅${smoke.stats.user_cycles} user cycles，不能外推到G17的数亿cycles选票验证，更不能将其1份Receipt当作B1选票证明。

- 控制程序Image ID：\`${smoke.image_id}\`。
- 控制程序用户ELF SHA：\`${config.official_smoke.user_elf_sha256}\`。
- 组合程序SHA：\`${program.program_sha256}\`。
- 真实Receipt SHA：\`${receipt.sha256}\`。

## 4. 为什么没有继续原B1及A/B

本轮在执行前冻结了完整EVM可用环境门槛：官方小程序自测可独立运行，但包装环境不满足时停止原B1大工作负载与后续路线。
因此，**本轮没有生成G17原B1的真实Receipt**。其程序、ELF和历史Execute证据只在admission JSON中作为引用，不拿官方小程序替代。
本机普通证明能力已得到正面证据；原B1证明的时间/内存以及完整EVM包装仍未验收。这里的停止属于本次冻结准入策略，不是宣称B1数学上不能证明。

| 阶段 | 结果 | 本轮可支持的结论 |
| --- | --- | --- |
| G18-1普通证明自测 | 实测PASS | 同一官方本地后端能生成并验证最小真实Composite |
| G18-1完整EVM环境 | BLOCKED | 当前M4不具备必需的受支持Groth16包装环境 |
| G17原B1真实Receipt | NOT_RUN | 不推断其Prove能力/成本 |
| G18-2 A/B完整功能 | NOT_RUN | 未开发SnarkPack guest或认证Jubjub累计guest |
| G18-3真实包装/EVM | NOT_RUN | 无signed交易、Gas或calldata实测 |
| G18-4新C与公平比较 | NOT_RUN | 旧C保持不变，不拼接历史数字排名 |

A的随机累积/全部FS、GT、GIPA、KZG、TIPP、MIPP语义尚未在本轮评审，不能把“未开始”写成已经证明不安全。
B的G17完整Groth16执行通过也不等于已绑定认证上下文、batch root、fresh NF和Jubjub状态累计。
所有这些未测指标为null。C仅保留旧实现与有摘要的历史出处，不把旧Gas放入新的同条件三路线比较。

## 5. 命令、原始记录和复核

实际执行入口：\`node scripts/g18_run.mjs experiments/g18_raw/run-20261115-01\`。这是已完成的唯一正式运行记录，不建议自动重复。
依次完成环境probe、隔离锁生成、locked/offline Release构建、fmt、官方程序准备和一次真实prove/verify。
全部argv、退出状态、起止时间、资源样本和日志在 [execution.json](../${run}/execution.json)。原输入、ELF、组合程序和Receipt均保留在official_smoke子目录。
本轮没有工程失败复测或更换证明后端；未自动提交推送。已有block0.1.6未来Rust兼容提示保留在构建日志，未改依赖。

只读检查：\`node scripts/g18_report.mjs --check\`。它检查冻结配置、输入、源码快照、Receipt字节摘要、结果和${historical.length}个原跟踪文件不变；不新生成证明或执行EVM。
它的PASS是归档完整性，不是对A/B架构或新EVM可行性的准入。

输出：[prover_admission.json](../${out}/prover_admission.json)、[functional_admission.json](../${out}/functional_admission.json)、
[proof_and_evm_costs.json](../${out}/proof_and_evm_costs.json)、[final_decision.json](../${out}/final_decision.json)及SHA256SUMS。

## 6. 决策边界与停止

最终只能给 \`INCONCLUSIVE_RESOURCE_OR_ENVIRONMENT\`，而不是SELECT_A/SELECT_B/KEEP_C_AND_REDESIGN的效率判断。
C继续作为已有可运行基线保存，并不代表本轮重新测得C优于A/B。未评价15%阈值，没有任何未运行架构的性能数据。
本轮到此停止，不自动G19。若要继续，须另行授权统一、受支持的证明目标环境并建立新的冻结记录；不得改写本次配置/失败边界，不能混用M4与其他机器数字排名。
`;
store('docs/G18_FINAL_ARCHITECTURE_DECISION.md',doc);
const walk=p=>fs.readdirSync(p,{withFileTypes:true}).flatMap(e=>e.isDirectory()?walk(path.join(p,e.name)):[path.join(p,e.name)]);
const sources=['scripts/g18_prepare.mjs','scripts/g18_preflight.mjs','scripts/g18_run.mjs','scripts/g18_report.mjs',
 'configs/experiment.G18-DECISION.json','experiments/g18_prover_admission/Cargo.toml',
 'experiments/g18_prover_admission/Cargo.lock','experiments/g18_prover_admission/src/main.rs'];
const all=[...walk(run),...sources,'docs/G18_FINAL_ARCHITECTURE_DECISION.md',
 ...['prover_admission','functional_admission','proof_and_evm_costs','final_decision'].map(n=>`${out}/${n}.json`)].sort();
store(`${out}/SHA256SUMS`,all.map(p=>`${sha(read(p))}  ${p}`).join('\n')+'\n');
console.log(JSON.stringify({audit:'PASS_G18_IMMUTABLE_ARCHIVE',decision:decision.decision,
 actual_official_smoke_receipts:1,actual_original_ballot_receipts:0,new_evm_transactions:0,
 old_tracked_files_unchanged:historical.length,mode},null,2));
