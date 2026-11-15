// G17 read-only evidence checker / first-write report generator. No VM/prover.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
process.chdir(path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../../..'));
const mode=process.argv[2]??'--check';assert.ok(['--write','--check'].includes(mode));
const read=p=>fs.readFileSync(p),load=p=>JSON.parse(read(p)),sha=b=>createHash('sha256').update(b).digest('hex');
const encode=j=>JSON.stringify(j,null,2)+'\n',pin=p=>({path:p,sha256:sha(read(p))});
const configPath='configs/experiment.G17-ZKVM-ABC.json',config=load(configPath);
const baseline=config.baseline_commit;
const r1='experiments/zkvm_abc_raw/run-20261115-01',r2='experiments/zkvm_abc_raw/run-20261115-02';
const rolling=new Set(['docs/IMPLEMENTATION_STATUS.md','docs/NEXT_STEPS.md']);
const history=load('experiments/joint_proof_raw/run-20261114-03/historical_files.json');
const oldWorking=new Map(history.map(e=>[e.path,e.sha256]));let unchanged=0;
for(const entry of execFileSync('git',['ls-tree','-r','-z',baseline],{maxBuffer:2*1024*1024}).toString().split('\0').filter(Boolean)){
  const m=/^(\d+) blob ([a-f0-9]{40})\t([\s\S]+)$/.exec(entry);assert.ok(m);
  const [, ,object,p]=m;if(rolling.has(p))continue;
  const bytes=read(p),blob=b=>createHash('sha1').update(`blob ${b.length}\0`).update(b).digest('hex');
  if(blob(bytes)!==object){assert.equal(sha(bytes),oldWorking.get(p),p);assert.equal(blob(Buffer.from(bytes.toString().replaceAll('\r\n','\n'))),object,p);}
  unchanged++;
}
const commandFiles=[r1,r2].flatMap(dir=>fs.readdirSync(dir).filter(p=>p.endsWith('.command.json')).sort().map(p=>path.join(dir,p)));
const commands=commandFiles.map(p=>({record:p,...load(p)}));
const command=(dir,name)=>{const c=commands.find(c=>c.record===`${dir}/${name}.command.json`);assert.ok(c,name);return c;};
for(const name of ['build-native','prepare-materials','clippy-native','fmt-final'])assert.equal(command(r2,name).exit_code,0);
assert.equal(command(r1,'fmt-guest').exit_code,0);
assert.equal(command(r1,'clippy-native').exit_code,101);
assert.equal(command(r2,'cargo-default').exit_code,1);
for(const name of ['install-rust','install-sdk'])assert.equal(command(r1,name).exit_code,1);
for(const name of ['download-rust-api','download-sdk-api'])assert.equal(command(r1,name).exit_code,28);
assert.equal(command(r2,'build-guest').exit_code,101);
const buildError=read(`${r2}/logs/build-guest.stderr.log`).toString();
assert.ok(buildError.includes('Risc Zero Rust toolchain not found'));
const tree=read(`${r1}/logs/dependency-tree.stdout.log`).toString();
assert.ok(!/\b(bellperson|blstrs|blst|rayon) v/.test(tree));
const materials=load(`${r2}/materials/materials.json`);
assert.equal(materials.status,'PASS_NATIVE_PUBLIC_MATERIAL_PREPARATION_NOT_GUEST');
assert.equal(materials.negative_a_conversion_matches,true);
assert.equal(materials.original_vk_sha256,config.original_ballot_vk_sha256);
const tests=materials.cases.map(c=>{
 const id=c.file.replace(/\.json$/,''),p=`${r2}/native-${id}.json`,j=load(p);
 assert.equal(j.input_sha256,c.input_sha256);assert.equal(sha(read(`${r2}/materials/${c.file}`)),c.input_sha256);
 assert.equal(j.native_accept,c.expected_accept);assert.equal(j.guest_executed,false);
 assert.equal(command(r2,'native-'+id).exit_code,0);
 assert.equal(j.guest_cycles,null);assert.equal(j.real_proof_generated,false);
 if(!c.expected_accept)assert.equal(j.native_error,`G17_INVALID_GROTH16_${c.batch_size-1}`);
 const good=load(`${r2}/materials/b${c.batch_size}-valid.json`),actual=load(`${r2}/materials/${c.file}`);
 assert.deepEqual(good.proofs_ark_hex,actual.proofs_ark_hex);assert.equal(good.vk_ark_hex,actual.vk_ark_hex);
 const diffs=[];actual.inputs_le_hex.forEach((row,i)=>row.forEach((f,k)=>{if(f!==good.inputs_le_hex[i][k])diffs.push([i,k])}));
 assert.deepEqual(diffs,c.expected_accept?[]:[[c.batch_size-1,7]]);
 return {id,batch_size:c.batch_size,expected_accept:c.expected_accept,mutation:c.mutation,
  native_status:j.status,native_accept:j.native_accept,native_error:j.native_error,
  native_verification_ms:j.native_verification_ms,native_samples:1,native_result:p,
  status:'NOT_RUN',guest_executed:false,guest_accept:null,guest_cycles:null,guest_execution_ms:null,
  guest_peak_rss_mib:null,program_id:null,guest_elf_sha256:null,
  reason:'Missing guest toolchain after official download failures; native results are NOT guest results.'};
});
// Both valid/mutated materials must be generated from the same frozen W32 proof pool.
const pool=load(config.material_sources[0]),good=load(`${r2}/materials/b2-valid.json`);
assert.deepEqual(good.proofs_ark_hex,pool.source_proofs_ark_compressed_hex.slice(0,2));
assert.deepEqual(good.inputs_le_hex,pool.source_public_inputs_le_hex.slice(0,2));
assert.equal(sha(Buffer.from(good.vk_ark_hex,'hex')),pool.groth16_vk_ark_compressed_sha256);
const hardware=read(`${r1}/logs/hardware.stdout.log`).toString().trim().split('\n');
const env={cpu:hardware[0],physical_memory_bytes:Number(hardware[1]),logical_cpus:Number(hardware[2]),
 host_rust:read(`${r1}/logs/rust-version.stdout.log`).toString().trim(),
 cargo:read(`${r1}/logs/cargo-version.stdout.log`).toString().trim(),node:read(`${r1}/logs/node-version.stdout.log`).toString().trim(),
 rzup:read(`${r2}/logs/rzup-version.stdout.log`).toString().trim(),guest_toolchain_installed:false,
 sdk_cli_installed:false,r0vm_installed:false,global_default_toolchain_changed:false};
const sdkRelease=load(`${r1}/upstream/risc0-release.json`),rustRelease=load(`${r1}/risc0-rust-release.json`);
const sdkAsset=sdkRelease.assets.find(a=>a.name==='cargo-risczero-aarch64-apple-darwin.tgz');
const rustAsset=rustRelease.assets.find(a=>a.name==='rust-toolchain-aarch64-apple-darwin.tar.gz');
assert.equal(sdkRelease.tag_name,'v3.1.0');assert.equal(rustRelease.tag_name,'r0.1.97.0');
const downloads=load(`${r1}/download_integrity.json`);
assert.ok(downloads.assets.every(a=>a.complete_and_hash_matches===false));
const commonPaths=['Cargo.toml','common/Cargo.toml','common/src/lib.rs','host/Cargo.toml','host/src/main.rs',
 'guest_b_groth16/Cargo.toml','guest_b_groth16/src/main.rs','methods/Cargo.toml','methods/build.rs','methods/src/lib.rs',
 'Cargo.lock','scripts/command.mjs','scripts/report.mjs','.gitignore'].map(p=>'experiments/zkvm_abc/'+p);
const sourcePins=[configPath,...commonPaths].map(pin);
const inputPins=[...config.material_sources,...config.reference_only_later_b4_sources,'results/g12/route_decision.json'].map(pin);
const compatibility={schema:'VV/G17/COMPATIBILITY/v1',status:'BLOCKED_COMPATIBILITY',
 reason_code:'OFFICIAL_TOOLCHAIN_DOWNLOAD_UNAVAILABLE',baseline_commit:baseline,branch:config.experiment_branch,
 scope:'Z0 / Z1 only',environment:env,sdk_version:config.sdk_version,build_crate_version:config.build_crate_version,
 official_release:{url:config.official_sources[1],published:sdkRelease.published_at,tag:sdkRelease.tag_name,
  sdk_asset:{url:sdkAsset.browser_download_url,bytes:sdkAsset.size,digest:sdkAsset.digest},
  rust_asset:{url:rustAsset.browser_download_url,bytes:rustAsset.size,digest:rustAsset.digest}},
 categories:[
  {id:'APPLE_SILICON',status:'DOCUMENTED_SUPPORTED_HOST_NOT_GUEST_PROVEN',evidence:config.official_sources[0]},
  {id:'GUEST_TOOLCHAIN',status:'BLOCKED_COMPATIBILITY',reason:'rzup TLS/connect failures; both official API downloads timed out; actual build reports toolchain missing.'},
  {id:'ORIGINAL_BELLPERSON_BLST_FFI',status:'UNKNOWN',reason:'Original blstrs uses BLST C/FFI; no direct guest cross-compilation attempted. Portable flag is not proof of zkVM support.'},
  {id:'ORIGINAL_RAYON_THREADS',status:'UNKNOWN',reason:'Original verifier uses Rayon; scheduling/runtime port not tested. This round shared verifier has no Rayon dependency.'},
  {id:'RNG',status:'AUDITED_DETERMINISTIC_Z1_PATH_GUEST_UNKNOWN',reason:'Pure Groth16 verification needs no random coefficients. SnarkPack random-accumulator semantics are deferred, not replaced by a fixed seed.'},
  {id:'FILE_NETWORK',status:'HOST_ONLY_BY_SOURCE_GUEST_UNKNOWN',reason:'Only host reads public JSON; guest reads env input and commits output. No guest file/network API or remote prover used.'},
  {id:'BLS381_PURE_RUST',status:'NATIVE_REFERENCE_PASS_GUEST_NOT_RUN',reason:'Original Ark 0.4 verifier, full canonical decoding/subgroup checks; four native cases passed, guest target not compiled.'},
  {id:'SERIALIZATION',status:'NATIVE_REFERENCE_PASS_GUEST_NOT_RUN',reason:'Original VK SHA matches; 192B Ark A/B/C equals archived 512B EIP -A/B/C after sign conversion; all 25 LE/BE fields match.'},
  {id:'SHA512_SHA256_FS',status:'NOT_RUN',reason:'Original SnarkPack FS belongs to Z2. Z1 SHA-256 journal digests do not replace the original protocol transcript.'},
  {id:'REAL_PROOF_EVM',status:'NOT_RUN',reason:'Z4 is not authorized this round. No receipt, Docker/CUDA or remote prover was used; local outer-proof feasibility remains unknown.'},
 ],source_pins:sourcePins,input_pins:inputPins,downloads,
 historical_files_preserved:unchanged,guest_build_command:`${r2}/build-guest.command.json`,
 guest_build_failure_is_not_invalid_proof:true,mathematical_infeasibility_claim:false,
 zero_setup_and_proof_generation:true};
const guestB={schema:'VV/G17/GUEST-B-TESTS/v1',status:'BLOCKED_COMPATIBILITY',reason_code:compatibility.reason_code,
 original_curve:'BLS12-381',original_vk_sha256:config.original_ballot_vk_sha256,public_input_count:25,
 common_function:'g17_common::verify_batch',guest_calls_complete_groth16_by_source:true,
 guest_build_succeeded:false,host_with_zkvm_feature_typechecked:false,guest_execution_count:0,
 native_reference_cases_passed:tests.length,native_reference_unique_original_proofs:2,
 cases:tests,real_receipt_generated:false,sample_interpretation:'Single functional native controls only; no guest performance or cross-route comparison.'};
const notRun=(schema,reason)=>({schema,status:'NOT_RUN',reason,measurements:null,real_proof_count:0,evm_transactions:0});
const guestA=notRun('VV/G17/GUEST-A-TESTS/v1','Stop before Z2: route B guest not executable; do not port or alter SnarkPack/random accumulator now.');
const state=notRun('VV/G17/STATE-BINDING-TESTS/v1','Z3 not started; Z1 verifies ballot validity only, not authenticated state transition, fresh NFs or accepted batch root.');
const proof={...notRun('VV/G17/PROOF-METRICS/v1','Z4 not started; no development/fake or real receipt generated.'),
 A:{prove_ms:null,peak_rss_mib:null,receipt_bytes:null,verify_ms:null},B:{prove_ms:null,peak_rss_mib:null,receipt_bytes:null,verify_ms:null}};
const evm={...notRun('VV/G17/EVM-COMPARISON/v1','No new EVM execution. Historical C is not combined with missing A/B measurements.'),
 A:{deployment_gas:null,receive_gas:null,calldata_bytes:null},B:{deployment_gas:null,receive_gas:null,calldata_bytes:null},
 C:{status:'HISTORICAL_REFERENCE_ONLY_NOT_REMEASURED',source:config.reference_only_later_b4_sources[1],
  new_deployment_gas:null,new_receive_gas:null,new_calldata_bytes:null},fair_winner:null};
const decision={schema:'VV/G17/ROUTE-DECISION/v1',status:'BLOCKED_COMPATIBILITY',reason_code:compatibility.reason_code,
 baseline_commit:baseline,source_pins:sourcePins,completed:['isolated branch and project','frozen original public materials','native B1/B2 valid and changed-input controls','actual logged guest build attempt'],
 not_completed:['guest-target compilation','B1/B2 guest execution/cycles','SnarkPack guest','Jubjub fold in guest','outer proof','EVM','three-route comparison'],
 selected_architecture:null,efficiency_winner:null,zkvm_impossibility_proven:false,
 next_only:'Restore the SAME pinned official Rust/SDK/r0vm locally, verify hashes, then build and run four B guest cases with 600s/16GiB soft budgets. Stop for review before Z2.',
 new_setup_count:0,new_ballot_proof_count:0,new_zkvm_proof_count:0,new_evm_transactions:0,
 auto_commit:false,auto_push:false,paper_or_protocol_changed:false,
 original_ffi_portability_unknown:true,J1_failure_does_not_rule_out_zkvm:true};

const outputs=new Map([
 ['results/g17/compatibility.json',encode(compatibility)],['results/g17/guest_a_tests.json',encode(guestA)],
 ['results/g17/guest_b_tests.json',encode(guestB)],['results/g17/state_binding_tests.json',encode(state)],
 ['results/g17/proof_metrics.json',encode(proof)],['results/g17/evm_gas_comparison.json',encode(evm)],
 ['results/g17/route_decision.json',encode(decision)],
]);
const sourceSnapshot=`${r2}/source_snapshot`;
const provenance={schema:'VV/G17/EXECUTION-ARCHIVE/v1',status:'BLOCKED_COMPATIBILITY',baseline_commit:baseline,
 started_utc:commands.map(c=>c.started_utc).sort()[0],finished_commands_utc:commands.map(c=>c.finished_utc).sort().at(-1),
 environment:env,config:pin(configPath),source_pins:sourcePins,input_pins:inputPins,
 source_version_sha256:sha(Buffer.from(encode(sourcePins))),commands,
 rss_scope:'Build/installer polled RSS is not guest memory. Native controls finish before a 1s RSS sample; their RSS remains null.',
 guest_cycles:null,guest_execution_count:0,proof_count:0,evm_transactions:0,historical_files_preserved:unchanged,
 previous_run_notes:'run-01 retains initial chunks_exact lint failure/source snapshot. run-02 uses only as_chunks lint fix; actual native controls rerun; guest remains blocked at missing Rust toolchain.'};
outputs.set(`${r2}/execution.json`,encode(provenance));
outputs.set(`${r2}/source_files.json`,encode(sourcePins));
const table=tests.map(t=>`| ${t.id} | ${t.native_accept?'接受':'密码学拒绝'} | ${t.native_verification_ms.toFixed(3)} | NOT_RUN | null |`).join('\n');
const failures=['install-rust','install-sdk','download-rust-api','download-sdk-api'].map(name=>{
 const c=command(r1,name);return `| ${name} | ${c.exit_code} | ${(c.wall_ms/1000).toFixed(3)} | [日志](../${r1}/${c.stderr}) |`;
}).join('\n');
const doc=`# G17：zkVM A/B/C 三路线——Z0/Z1 首轮报告

## 结论：BLOCKED_COMPATIBILITY（工具链下载，非密码学不可行）

2026-11-15。分支 \`${config.experiment_branch}\`，基准 \`${baseline}\`。
本轮只执行用户方案最后一段规定的 Z0/Z1，不开发 SnarkPack guest、累计关系、外层证明或新合约。
**原始证明的原生验证和编码对照通过，但 guest 编译/执行尚未完成。不能称 PASS_GUEST。**

## 实际完成与阻塞

- 新增独立 Cargo 工作区，根 Cargo.toml/Cargo.lock、协议 Manifest、原曲线/VK/SRS/证明、合约、P3 和论文全部不动。
- 复用 W32 最前两张原票；由 G8 EIP VK 重建 Ark VK，SHA-256与原值 \`${config.original_ballot_vk_sha256}\` 一致。
- 对照两张原192B证明与原512B EIP证明，明确 EIP存的是-A；全部25个公开输入执行LE/BE一致性检查。
- 共享 \`verify_batch\` 是完整 BLS12-381 Groth16 验证：规范解析/子群、固定VK、全部输入与配对式；不接收 host成功标志，不做随机批检。
- guest源码调用同一函数，只有验证成功才能 commit；但未生成ELF，带zkvm feature的最终host代码也尚未完整typecheck，不能把源码存在当成 guest通过。
- 两次真实 guest构建均到达 risc0-build 后因 \`Risc Zero Rust toolchain not found\` 退出101；没有密码学拒绝，也不是J1的Fq/Fr问题。

初轮Clippy只因chunks_exact的新lint失败，run-01日志及源码保留；改为等价as_chunks后，run-02原生Release、严格Clippy和fmt通过，并重新执行下列4例。

## 原生对照：不是 zkVM 性能

| 用例 | 原生完整验证 | 原生验证 ms（单样本） | guest | guest cycles |
| --- | --- | --- | --- | --- |
${table}

反例只改最后一张票 input[7] 为原Fr+1，原证明/VK不变；两次分别得到G17_INVALID_GROTH16_0/1，确实到达Groth16拒绝。
上述时间包含本轮适配器的规范解析/prepare/验证等函数开销，只有功能控制意义，不是独立选举统计，不与任何历史或外部数据排名。
没有guest运行时间、周期、内存、program ID或receipt；全部为null。原生控制太快未取得1秒RSS样本，其内存也为null，不填0。

## 环境与工具链：记录失败，不降低版本

机器：${env.cpu}，${env.physical_memory_bytes/1024**3} GiB，${env.logical_cpus}逻辑核；host为aarch64-apple-darwin。
已有 \`${env.cargo}\`、Node \`${env.node}\`。默认stable的cargo组件不可用，本轮使用 \`cargo +1.99.1\`，未修改全局默认工具链。
\`${env.rzup}\`来自官方安装脚本指定地址，下载到target/g17-tools；只检查脚本，不执行其修改shell profile的步骤。
RISC0_HOME固定在项目忽略的工具缓存，不写入shell profile，不用第三方镜像。

[官方安装文档](https://dev.risczero.com/api/zkvm/install)列出arm64 macOS，并要求固定工具版本。
[官方v3.1.0发布](https://github.com/risc0/risc0/releases/tag/v3.1.0)包含安全修复，本轮锁定SDK/r0vm3.1.0、build crate3.0.7及guest Rust1.97.0，不降级到旧教程版本。
SDK crate可取得，但二进制SDK/guest Rust资产没有下载完整；安装目标是本机版本，不是远程证明服务。

| 真实命令尝试 | 退出码 | 墙钟秒 | 证据 |
| --- | --- | --- | --- |
${failures}

rzup下载分别发生TLS EOF/连接超时；同一官方GitHub API资产入口也因超时只下载部分字节。
官方SDK资产应为${sdkAsset.size} B，Rust资产应为${rustAsset.size} B；期望SHA来自已归档官方发布API。
部分文件不符合长度/摘要，未解包或执行，不宣称已安装。外层r0vm/证明工具没有另行安装或运行。
这说明**当前机器到资产的下载路径受阻**，不是Apple Silicon被证实不支持，也不是zkVM或原Groth16关系数学上不可行。

## 可移植性边界

原bellperson0.27/blstrs0.7依赖BLST C/FFI、Rayon等；尚未在guest编译，不仅凭依赖存在就断言绝对不兼容。
本轮用不启用parallel的Arkworks0.4纯Rust验证原票；实际normal依赖树没有BLST/blstrs/bellperson/Rayon。SDK自身的其他曲线依赖不等于替换内层BLS12-381。
完整Groth16验证路径不需要随机系数；原SnarkPack的随机累积/sentinel/FS移植仍在Z2，未通过也未变更。
host读公开JSON；guest只读env输入、执行验证并commit摘要，不读取选民私钥/见证、文件或网络。
Z1 journal只描述已验证票及完整顺序输入摘要，**没有**认证选举/批次根/全局NF/累计状态证明，不能拿它当完整接收凭据。
program ID应来自真实ELF的image ID并最终由receipt/链上预期程序绑定；本轮ELF和image ID均不存在，不虚填常量。

## A/B/C 现在能比较吗？

不能。A=NOT_RUN；B的guest=NOT_RUN；C仅保留Stage2已归档逐票+Direct Fold参考，没有重测。
Z3状态累计、Z4真实证明/EVM、Z5两批B4均未启动；A+State也未启动。新Setup、Prove、EVM为0是执行次数，不是把未测成本填0。
完整A/B链下证明、Gas、calldata、内存和总成本保持null，无效率获胜者，不据此替换SnarkPack或修改论文架构。
本地真实Groth16 shrink-wrap的Docker/CUDA/内存要求留给Z4单独核验，本轮不能声称Mac已具备生成EVM可验证外层证明的能力。

## 复核与下一步

在仓库根目录运行：

\`node experiments/zkvm_abc/scripts/report.mjs --check\`

它只读检查全部原始命令/结果/材料、当前源码与快照、摘要及${unchanged}个旧跟踪文件；不启动VM或证明。
\`--write\`只首次生成新的G17归档，已有不同内容时拒绝覆盖。
报告清单在 [results/g17/SHA256SUMS](../results/g17/SHA256SUMS)，相对路径以仓库根为准。

网络恢复后仅继续以下最小步骤，**不自动执行Z2**：

1. 从已归档官方地址下载同一SDK3.1.0、guest Rust1.97.0并核对发布SHA，使用同一RISC0_HOME安装；固定r0vm3.1.0。
2. 执行 \`cargo +1.99.1 build --manifest-path experiments/zkvm_abc/Cargo.toml --release --locked -p g17-host --features zkvm\`。
3. 新run目录逐例执行 \`g17-host guest INPUT NEW_RESULT_JSON\`，每例600秒/单进程16GiB软预算；宿主IPC只使用本地r0vm，不生成receipt。
4. 必须实际得到B1/B2正例、B1错误输入和B2第二票错误输入的guest密码学结果与原生对应；记录cycles/segments、ELF/image ID和实际时间。
5. 四例通过后停止复核，再决定Z2。若失败保留日志；不得用host原生成功、假receipt、RISC0_SKIP_BUILD或资源超时冒充guest通过。

当前代码与结果未自动提交或推送。完整失败日志、两轮原生控制、两轮源码与锁版本均保留，不覆盖G8—G12历史。
`;
outputs.set('docs/G17_ZKVM_ABC_COMPARISON.md',doc);

// Snapshot exact final sources; preserve initial snapshot separately.
if(mode==='--write')for(const p of sourcePins){const dest=path.join(sourceSnapshot,p.path);if(!fs.existsSync(dest)){fs.mkdirSync(path.dirname(dest),{recursive:true});fs.copyFileSync(p.path,dest,fs.constants.COPYFILE_EXCL);}}
for(const p of sourcePins)assert.equal(sha(read(path.join(sourceSnapshot,p.path))),p.sha256,'snapshot '+p.path);
for(const p of load(`${r1}/source_files.json`))assert.equal(sha(read(path.join(r1,'source_snapshot',p.path))),p.sha256,'initial snapshot '+p.path);
const rawFiles=[];function walk(dir){for(const e of fs.readdirSync(dir,{withFileTypes:true})){const p=path.join(dir,e.name);if(e.isDirectory())walk(p);else rawFiles.push(p)}}walk(r1);walk(r2);
const manifest=new Map([...rawFiles.filter(p=>!outputs.has(p)).map(p=>[p,sha(read(p))]),...sourcePins.map(p=>[p.path,p.sha256]),...inputPins.map(p=>[p.path,p.sha256]),...outputs.entries()].map(([p,v])=>[p,typeof v==='string'&&/^[a-f0-9]{64}$/.test(v)?v:sha(Buffer.from(v))]));
const sums=[...manifest].sort(([a],[b])=>a.localeCompare(b)).map(([p,h])=>`${h}  ${p}`).join('\n')+'\n';
outputs.set('results/g17/SHA256SUMS',sums);
for(const[p,v]of outputs)if(mode==='--check'||fs.existsSync(p))assert.equal(read(p).toString(),v,p);
if(mode==='--write')for(const[p,v]of outputs)if(!fs.existsSync(p)){fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,v,{flag:'wx'});}
console.log(encode({audit:'PASS_G17_BLOCKED_EVIDENCE_INTEGRITY',status:'BLOCKED_COMPATIBILITY',reason_code:compatibility.reason_code,
 native_cases:tests.length,guest_cases_executed:0,guest_cycles:null,new_proofs:0,new_evm:0,historical_files_unchanged:unchanged,
 full_comparison_available:false,auto_follow_on:false}));
