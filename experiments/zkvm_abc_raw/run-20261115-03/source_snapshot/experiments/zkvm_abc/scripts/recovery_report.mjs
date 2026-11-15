// First-write archive / read-only checker for the four real Z1 executions.
// Does not execute a VM, download tools, prove, or modify historical reports.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
process.chdir(path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..'));
const mode=process.argv[2]??'--check';
assert.ok(['--write','--check'].includes(mode));
const run='experiments/zkvm_abc_raw/run-20261115-03';
const previous='experiments/zkvm_abc_raw/run-20261115-02';
const out='results/g17_z1_recovery';
const read=p=>fs.readFileSync(p),load=p=>JSON.parse(read(p));
const sha=b=>createHash('sha256').update(b).digest('hex');
const pin=p=>({path:p,sha256:sha(read(p)),bytes:read(p).length});
const encode=j=>JSON.stringify(j,null,2)+'\n';
const store=(p,b)=>{
 b=Buffer.isBuffer(b)?b:Buffer.from(b);
 if(mode==='--write'&&!fs.existsSync(p)){
  fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,b,{flag:'wx'});
 }else assert.deepEqual(read(p),b,p);
};
const config=load('configs/experiment.G17-ZKVM-ABC.json');
const downloads=load(`${run}/download_verified.json`),install=load(`${run}/installation.json`);
assert.equal(downloads.status,'PASS_OFFICIAL_ASSET_SHA256');
assert.equal(install.status,'PASS_PINNED_TOOLCHAIN_EXTRACTED');
const releases=[load('experiments/zkvm_abc_raw/run-20261115-01/upstream/risc0-release.json'),
 load('experiments/zkvm_abc_raw/run-20261115-01/risc0-rust-release.json')];
for(const [i,a]of downloads.assets.entries()){
 const official=releases[i].assets.find(x=>x.browser_download_url===a.official_url);
 assert.ok(official);assert.equal(a.bytes,official.size);
 assert.equal('sha256:'+a.sha256,official.digest);assert.equal(a.sha256,a.expected_sha256);assert.equal(a.match,true);
 // Tool caches are ignored. Archived release digests suffice after cleanup.
 if(fs.existsSync(a.path)){assert.equal(sha(read(a.path)),a.sha256);assert.equal(read(a.path).length,a.bytes);}
}
const command=name=>load(`${run}/${name}.command.json`);
const success=name=>{
 const c=command(name);assert.equal(c.exit_code,0,name);assert.equal(c.termination,null,name);
 assert.equal(c.signal,null,name);assert.equal(c.spawn_error,null,name);return c;
};
for(const n of ['download-sdk-proxy','download-rust-proxy','install-verified','sdk-version-corrected',
 'r0vm-version','guest-rust-version','installed-components','build-host-no-strip','image-id',
 'fmt-workspace','fmt-guest','build-locked-offline','check-historical-g17','check-historical-g12'])success(n);
assert.equal(command('clippy-zkvm').exit_code,101);
const lintError=read(`${run}/logs/clippy-zkvm.stderr.log`).toString();
assert.ok(lintError.includes('error[E0463]')&&lintError.includes("can't find crate for `std`"));
assert.equal(command('sdk-version').exit_code,2); // Initial wrong CLI invocation, retained.
assert.equal(command('build-guest').exit_code,101); // Operator-stopped host strip failure.
assert.equal(load(`${run}/build_diagnostic.json`).first_build_stopped_by_operator,true);
for(const n of ['sdk-version-corrected','r0vm-version'])assert.match(read(`${run}/logs/${n}.stdout.log`).toString(),/3\.1\.0/);
assert.match(read(`${run}/logs/guest-rust-version.stdout.log`).toString(),/rustc 1\.97\.0/);

const built='experiments/zkvm_abc/target/riscv-guest/g17-methods/g17-guest-b/riscv32im-risc0-zkvm-elf/release/g17-guest-b';
const metadata='experiments/zkvm_abc/target/release/build/g17-methods-267ba0c8b10630cf/out/methods.rs';
const artifacts=[['guest-b.elf',built],['guest-b.bin',built+'.bin'],['methods.rs',metadata]];
for(const [name,src]of artifacts){
 const dest=`${run}/artifacts/${name}`;
 if(mode==='--write')store(dest,read(src));
 else if(fs.existsSync(src))assert.equal(sha(read(dest)),sha(read(src)),src);
}
assert.equal(read(`${run}/artifacts/guest-b.elf`).subarray(0,4).toString('hex'),'7f454c46');
const methods=read(`${run}/artifacts/methods.rs`).toString();
const words=JSON.parse('['+methods.match(/G17_GUEST_B_ID: \[u32; 8\] = \[([^\]]+)\]/)[1]+']');
const idBytes=Buffer.alloc(32);words.forEach((w,i)=>idBytes.writeUInt32LE(w,4*i));
const image=idBytes.toString('hex');
const imageLog=read(`${run}/logs/image-id.stdout.log`).toString().trim().split('\n').at(-1);
assert.equal(imageLog,image);
const programPin=pin(`${run}/artifacts/guest-b.bin`),elfPin=pin(`${run}/artifacts/guest-b.elf`);
const materials=load(`${previous}/materials/materials.json`);
const cases=materials.cases.map(c=>{
 const id=c.file.replace(/\.json$/,''),p=`${run}/guest-${id}.json`,j=load(p),cmd=success('guest-'+id);
 const input=`${previous}/materials/${c.file}`;
 assert.equal(sha(read(input)),c.input_sha256);assert.equal(j.input_sha256,c.input_sha256);
 assert.equal(j.guest_executed,true);assert.equal(j.guest_execution_attempted,true);
 assert.equal(j.native_accept,c.expected_accept);assert.equal(j.guest_accept,c.expected_accept);
 assert.equal(j.program_id,image);assert.equal(j.guest_elf_sha256,programPin.sha256);
 assert.equal(j.real_proof_generated,false);assert.equal(j.receipt_kind,'NONE_EXECUTION_ONLY');
 assert.equal(j.evm_gas,null);assert.equal(j.state_binding_implemented,false);
 assert.ok(j.guest_wall_ms>0&&cmd.wall_ms>=j.guest_wall_ms);
 assert.ok(cmd.sampled_max_single_process_rss_mib>0&&cmd.sampled_max_single_process_rss_mib<16384);
 if(c.expected_accept){
  assert.equal(j.status,'PASS_GUEST');assert.ok(j.guest_cycles>0);assert.ok(j.segments>0);assert.equal(j.guest_error,null);
 }else{
  assert.equal(j.status,'PASS_GUEST_CRYPTOGRAPHIC_REJECTION');
  assert.equal(j.guest_error,`Guest panicked: G17_INVALID_GROTH16_${c.batch_size-1}`);
  assert.equal(j.guest_cycles,null);assert.equal(j.segments,null);
 }
 return {id,batch_size:c.batch_size,status:j.status,expected_accept:c.expected_accept,guest_accept:j.guest_accept,
  result:p,input:pin(input),mutation:c.mutation,guest_cycles:j.guest_cycles,segments:j.segments,
  executor_wall_ms:j.guest_wall_ms,command_wall_ms:cmd.wall_ms,
  sampled_max_single_process_rss_mib:cmd.sampled_max_single_process_rss_mib,
  sampled_max_process_tree_rss_mib:cmd.sampled_max_process_tree_rss_mib,
  rss_samples:cmd.rss_sample_count,guest_error:j.guest_error,journal_bytes:j.journal_bytes};
});
assert.equal(cases.length,4);
const relativeSources=['Cargo.toml','Cargo.lock','common/Cargo.toml','common/src/lib.rs','host/Cargo.toml','host/src/main.rs',
 'methods/Cargo.toml','methods/build.rs','methods/src/lib.rs','guest_b_groth16/Cargo.toml',
 'guest_b_groth16/Cargo.lock','guest_b_groth16/src/main.rs','scripts/command.mjs','scripts/recovery_exec.mjs',
 'scripts/recovery_install.mjs','scripts/recovery_report.mjs'];
const sourcePaths=['configs/experiment.G17-ZKVM-ABC.json',...relativeSources.map(p=>'experiments/zkvm_abc/'+p)];
const sources=sourcePaths.map(pin);
for(const p of sourcePaths)store(`${run}/source_snapshot/${p}`,read(p));
store(`${run}/source_files.json`,encode(sources));
const commands=fs.readdirSync(run).filter(p=>p.endsWith('.command.json')).sort().map(p=>{
 const file=`${run}/${p}`,j=load(file);return {record:pin(file),stdout:pin(`${run}/${j.stdout}`),stderr:pin(`${run}/${j.stderr}`)};
});
const hardware=read('experiments/zkvm_abc_raw/run-20261115-01/logs/hardware.stdout.log').toString().trim().split('\n');
const tests={schema:'VV/G17/Z1/GUEST-RECOVERY/v1',status:'PASS_Z1_GUEST_EXECUTION_NOT_PROOF',
 run,baseline_commit:'34097bda0e1ed9887a91fd4a721ffc172202f512',branch:'experiment/g17-zkvm-abc-comparison',
 execution_source:'Uncommitted recovery scripts on baseline; exact source snapshot/hash below, not a clean-commit run',
 source_version_sha256:sha(Buffer.from(encode(sources))),sources,commands,
 environment:{cpu:hardware[0],physical_memory_bytes:Number(hardware[1]),logical_cpus:Number(hardware[2]),
  sdk:'3.1.0',r0vm:'3.1.0',guest_rust:'1.97.0',host_rust:'1.99.1',risc0_build:'3.0.7',
  executor:'local r0vm via IPC',dev_mode:false,cargo_jobs:2,host_release_strip:'none',
  global_defaults_changed:false,remote_prover_used:false},
 quality_checks:{fmt_workspace:'PASS',fmt_guest:'PASS',locked_offline_build:'PASS',
  clippy_zkvm:'BLOCKED_NESTED_HOST_CLIPPY_DRIVER_TARGET_STD',
  clippy_reason:'RUSTC_WORKSPACE_WRAPPER set by host Clippy leaks into nested guest compilation; archived E0463. Normal pinned guest build/execution succeeds. No skip-build workaround used.'},
 official_downloads:downloads,installation:install,program:{image_id:image,user_elf:elfPin,combined_program:programPin,
  legacy_guest_elf_sha256_field_means:'SDK G17_GUEST_B_ELF embeds ProgramBinary (user ELF + kernel), not bare user ELF'},
 original_curve:'BLS12-381',original_vk_sha256:config.original_ballot_vk_sha256,public_inputs_per_ballot:25,
 reused_original_ballot_proofs:2,new_ballot_setup_count:0,new_ballot_proof_count:0,
 guest_executions:4,successful_positive_executions:2,cryptographic_rejections:2,cases,
 measurement_scope:{samples_per_case:1,statistics_or_performance_advantage_claim:false,
  cycles:'SDK SessionInfo.cycles(): sum of user cycles; excludes padding/continuation overhead',
  negative_cycles:'null: existing panic-on-invalid guest yields SDK error, no SessionInfo. No estimate or rerun added.',
  executor_wall_time:'Local IPC executor call, no proving. Does not include native reference/image-id computation.',
  memory:'1-second polled host-process and process-tree RSS; not exact peak, guest allocation or proof-generation memory.'},
 proof:{generated:false,receipt:null,prove_ms:null,proof_bytes:null},evm_gas:null,
 state_fold_implemented:false,snarkpack_guest_implemented:false,
 historical_results:'Prior G17 BLOCKED reports/raw runs are preserved; they describe previous attempts, not current execution.'};
const decision={schema:'VV/G17/Z1/DECISION/v1',status:'PASS_Z1_STOP_FOR_REVIEW_BEFORE_Z2',
 pinned_toolchain_restored:true,real_guest_compiled:true,four_existing_cases_passed:true,
 next:'Z2 design/admission review only: exact SnarkPack checks, FS, BLST/Rayon portability and random accumulator soundness must be settled before implementation.',
 automatic_z2_started:false,zkvm_proof_generation_started:false,new_contracts_started:false,
 inference:'Complete pure-Rust original BLS12-381 Groth16 verification executes on this guest. This does not establish original SnarkPack portability or proving/EVM feasibility.',
 selected_protocol_replacement:null,efficiency_winner:null,auto_commit:false,auto_push:false};
store(`${out}/guest_execution.json`,encode(tests));store(`${out}/route_decision.json`,encode(decision));
const rows=cases.map(c=>`| ${c.id} | ${c.guest_accept?'接受':'密码学拒绝'} | ${c.guest_cycles??'null'} | ${(c.executor_wall_ms/1000).toFixed(3)} | ${c.sampled_max_single_process_rss_mib.toFixed(2)} | ${c.sampled_max_process_tree_rss_mib.toFixed(2)} |`).join('\n');
const doc=`# G17 Z1：工具链恢复与四项真实 guest 执行

## 结论

2026-11-15，\`PASS_Z1_GUEST_EXECUTION_NOT_PROOF\`。锁定的官方工具链已恢复，真实 guest/host Release 构建成功；原 B1/B2 正例和单字段错误输入四项均符合预期。
本轮到 Z1 停止。未实现 SnarkPack guest、状态累计、外层证明或新合约，未修改论文、原密码学协议或历史结果。
基准提交 \`34097bda0e1ed9887a91fd4a721ffc172202f512\`；分支 \`experiment/g17-zkvm-abc-comparison\`。
实际执行使用本轮未提交恢复脚本，不能称为干净提交复现；源码快照及摘要在 [原始归档](../${run}/source_files.json)。

## 工具链恢复及真实构建

沿用 SDK/r0vm 3.1.0、risc0-build 3.0.7、guest Rust 1.97.0、host Rust 1.99.1，没有降级。
之前终端未使用 macOS 已配置的本地代理。本轮仅在下载命令中显式使用现有代理，仍从官方 GitHub 获取原定版本，未改系统网络设置。

| 官方资产 | 字节数 | SHA-256 |
| --- | --- | --- |
${downloads.assets.map(a=>`| ${path.basename(a.path)} | ${a.bytes} | \`${a.sha256}\` |`).join('\n')}

摘要逐项匹配前轮已归档的官方 release API，完整性通过后才解包。SDK 归档本身包含 cargo-risczero 与 r0vm。
采用 rzup 识别的项目内版本目录；版本查询均通过，不修改全局 rustup 默认、Cargo shim 或 shell profile。工具包留在忽略的 target 缓存，不上传约 500MB 安装包。
[官方安装说明](https://dev.risczero.com/api/zkvm/install)、[SDK v3.1.0](https://github.com/risc0/risc0/releases/tag/v3.1.0)、[guest Rust 发布](https://github.com/risc0/rust/releases/tag/r0.1.97.0)。

实际编译发现另一处主机工具故障：guest 已编译，但 host 的 rust-objcopy 停在 dyld abort_with_payload。采样栈和退出101的首次构建均保留，操作员仅停止该任务的 rustc/objcopy。
本轮命令改为 \`CARGO_PROFILE_RELEASE_STRIP=none\`，不运行故障的 host strip；没有改动 guest 源码、验证算法、优化等级或全局工具链。新建 guest Cargo.lock 固定了首次实际解析的依赖。
第二次构建成功：${(success('build-host-no-strip').wall_ms/1000).toFixed(3)}秒；这包含主机 profile 变化导致的依赖重编译，不作为 guest 性能。
随后锁定/offline 构建及两个工作区 fmt 通过。严格 Clippy 没有通过：其 RUSTC_WORKSPACE_WRAPPER 主机驱动进入 risc0-build 的嵌套 guest 构建，产生 E0463 缺少目标 std；这是工具调用问题，不是上述正常 guest 构建或四项密码学执行失败。
保留 clippy-zkvm 的退出101和完整日志；不使用 RISC0_SKIP_BUILD、不修改 SDK 或 guest 来伪造检查成功。依赖 block 0.1.6 的未来 Rust 兼容提示也保留，未改依赖来掩盖它。

## 程序与原始输入

- 真实用户 RISC-V ELF：\`${elfPin.sha256}\`（${elfPin.bytes} B）。
- SDK 组合程序：\`${programPin.sha256}\`（${programPin.bytes} B）。SDK 的 G17_GUEST_B_ELF 常量实际嵌入 user ELF + kernel ProgramBinary；原结果字段 guest_elf_sha256 指的是这一组合程序，不能混作裸 ELF 摘要。
- Image ID：\`${image}\`。生成 methods.rs、独立 r0vm --id 和四份执行结果一致。
- 原 BLS12-381 VK：\`${config.original_ballot_vk_sha256}\`，每票25项公开输入。复用前轮已有两张真实证明，不做 Setup/Prove。

输入直接复用 run-02 四份文件；两个反例仅把最后一票 input[7] 改为 Fr+1，原证明/VK不动。共享完整 Groth16 验证函数未修改，guest 自行验证，不接收 host 成功标志。

## 四项实际结果（单样本）

| 用例 | guest 结果 | user cycles | executor 秒 | 单进程采样 RSS MiB | 进程树采样 RSS MiB |
| --- | --- | --- | --- | --- | --- |
${rows}

错误输入在 guest 内分别触发 G17_INVALID_GROTH16_0、G17_INVALID_GROTH16_1；不是下载、构建、解析、超时或内存错误。原生计算仅作对照，未填入 guest 性能栏。
正例 cycles 为 SDK SessionInfo 的 user-cycle 合计，不含填充/续算开销；反例沿用已有 panic 拒绝接口，SDK 返回错误而不返回 SessionInfo，因此周期/段数为 null，不估算、不为补数字重写 guest。
时间为本地 IPC execute 调用，不含证明生成，也不等于整个 host 命令时间。内存是每秒轮询的主机进程/树 RSS，不是精确峰值、guest 逻辑内存或证明器内存。
机器 ${hardware[0]}、${Number(hardware[1])/1024**3} GiB、${hardware[2]}逻辑核。四项顺序执行，各600秒/单进程16GiB软预算，均未触发上限。
这些仅是四个功能样本，不能据此推导统计性能优势或真实 zkVM Prove 成本。

## 实际命令与复现

仓库根目录中使用恢复脚本，它只给子进程固定 PATH/RISC0_HOME/本地 r0vm/IPC/dev_mode=0，不调用 prover，也不修改全局环境。
全部实际 argv、起止时间、退出码和 stdout/stderr 见 [原始命令目录](../${run}/)。

\`node experiments/zkvm_abc/scripts/recovery_exec.mjs cargo build --manifest-path experiments/zkvm_abc/Cargo.toml --release --locked -p g17-host --features zkvm\`

四例由现有资源记录器逐一执行，形式如下；将 CASE 换为 b1-valid、b1-changed-input、b2-valid、b2-changed-input，并使用不存在的新输出路径，不能覆盖归档：

\`node experiments/zkvm_abc/scripts/recovery_exec.mjs host guest experiments/zkvm_abc_raw/run-20261115-02/materials/CASE.json /ABS/NEW_RESULT.json\`

只读检查：\`node experiments/zkvm_abc/scripts/recovery_report.mjs --check\`。
旧 G17/G12 检查器也已实际运行通过；旧 BLOCKED 报告及 run-01/02 保持原样，当前结果单独归档在 [guest_execution.json](../${out}/guest_execution.json)。

## 是否进入 SnarkPack guest

可以进入下一轮 Z2 的设计/准入审查，但本轮不自动编码执行。现在只证实原 BLS12-381 Groth16 的纯 Rust 完整验证可运行；原 SnarkPack 的 BLST FFI/Rayon、全部 FS/GT/GIPA/KZG/TIPP/MIPP 与随机累积健全性仍需逐项解决。
不能用固定种子随机批检替代未经证明的原语义，也不能由这次 execute 成功推出证明生成、EVM 包装或完整投票状态绑定可行。
本轮 receipt/prove 时间/证明大小/Gas 均 null。未自动提交推送。
`;
store('docs/G17_Z1_GUEST_EXECUTION.md',doc);
const walk=p=>fs.readdirSync(p,{withFileTypes:true}).flatMap(e=>e.isDirectory()?walk(path.join(p,e.name)):[path.join(p,e.name)]);
const manifest=[...walk(run),`${out}/guest_execution.json`,`${out}/route_decision.json`,'docs/G17_Z1_GUEST_EXECUTION.md',...sourcePaths].sort();
store(`${out}/SHA256SUMS`,manifest.map(p=>`${sha(read(p))}  ${p}`).join('\n')+'\n');
console.log(JSON.stringify({status:'PASS_Z1_EXECUTION_ARCHIVE',guest_cases:4,program_id:image,
 real_proofs:0,new_evm_transactions:0,mode},null,2));
