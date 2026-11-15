// Evidence-derived report/sealing only. Never reruns cryptography or compilation.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {check} from './joint_proof_check.mjs';
const [runArg,resultsArg]=process.argv.slice(2);assert.ok(runArg&&resultsArg,'usage: node scripts/joint_proof_report.mjs J1_RUN NEW_RESULTS_DIR');
const run=path.resolve(runArg),output=path.resolve(resultsArg);assert.ok(!fs.existsSync(output),'never overwrite an archived report');
const checked=check(run),sha=b=>createHash('sha256').update(b).digest('hex');
const read=(dir,name)=>JSON.parse(fs.readFileSync(path.join(dir,name+'.json')));
const m=read(run,'execution'),n=read(run,'native_fq_control'),f=read(run,'public_preflight'),d=read(run,'outer_fr_compile_diagnostics');
const attempts=fs.readdirSync(path.dirname(run),{withFileTypes:true}).filter(e=>e.isDirectory()&&/^run-20261114-\d+$/.test(e.name))
 .map(e=>({dir:path.join(path.dirname(run),e.name),m:read(path.join(path.dirname(run),e.name),'execution')}));
const s={schema:'VV/JOINT/J1-SUMMARY/v1',status:m.status,reason:m.reason,run_directory:path.relative(process.cwd(),run),
 baseline_commit:m.baseline_commit,branch:m.branch,source_version_sha256:m.source_version_sha256,config_sha256:m.config_sha256,audit:checked,
 fixed_outer_constraint_field:'BLS12-381 Fr',fixed_outer_constraint_field_bits:255,native_gadget_constraint_field:'BLS12-381 Fq',native_gadget_constraint_field_bits:381,
 public_fixture_preflight_passed:true,original_ark_proof_sha256:f.proof_ark_sha256,original_ark_vk_sha256:f.vk_ark_sha256,original_eip_proof_sha256:f.proof_eip_sha256,
 r1cs_synthesized:false,r1cs_constraints:null,r1cs_public_input_count:null,r1cs_satisfied:null,
 inner_circuit_crypto_negative_tests:m.recursive_crypto_negative_tests,setup_count:0,proof_generation_count:0,
 joint_proof_bytes:null,joint_vk_sha256:null,joint_setup_ms:null,joint_prove_ms:null,joint_verify_ms:null,joint_prover_peak_rss_mib:null,joint_evm_gas:null,
 j2_j5_started:false,suite_changed:false,voter_secret_material_read:false,
 compiler_errors:d.errors.map(e=>({code:e.code.code,message:e.message})),
 attempts:attempts.map(a=>({directory:path.relative(process.cwd(),a.dir),status:a.m.status,error:a.m.error??null,wall_ms:a.m.wall_ms,commands:a.m.commands})),
};
fs.mkdirSync(output,{recursive:true});
fs.writeFileSync(path.join(output,'summary.json'),JSON.stringify(s,null,2)+'\n',{flag:'wx'});
const fmt=v=>v===null?'未测（null）':Number(v).toFixed(3);
const attemptNotes={
 'run-20261114-01':'开发错误：公开来源输出的 JSON 宏语法错误；不是递归不可行证据。',
 'run-20261114-02':'原证明核对和 Fq 对照编译通过，Fr 编译实际报 E0271；分类器错误地只期待 E0277，原 FAIL 保留，不改写为 PASS。',
 'run-20261114-03':'修正结果分类后缓存复验；接受真实 E0271 域不兼容作为 J1 BLOCKED，非密码学负例通过。',
};
const md=`# J1 联合证明可行性：停在当前域兼容性检查\n\n结论：\`${m.status}\`。这不是成功的联合证明，也不是已经证明递归在任何方案下都不可行。\n\n`+
 `## 实际完成\n\n- 新分支 \`${m.branch}\`，基准提交 \`${m.baseline_commit}\`。\n- 用原 D5/W32 第一张真实选票、原 VK 和 25 项公开输入做公开材料核对；Ark 原证明192 B、EIP-2537证明512 B逐字节对应，正确保留 \`A\` 与 \`-A\` 区别。原生验证通过仅为输入预检，没有把宿主 bool 写入 witness。\n- 在独立工具包启用锁定的 ark-groth16 0.4.0 \`r1cs\`，完整 Fq 验证器约束函数通过 Release 编译及严格 Clippy，但未调用该函数构建约束。\n- 对同一个原 BLS12-381 验证器，要求当前 \`SNARKGadget<Fr,Fr,Groth16<Bls12_381>>\` 接口；真实编译退出101，报告 \`E0271\`：FqConfig 与 FrConfig 的 BasePrimeField 不相等。没有使用缺少 feature/import 的报错冒充域不兼容。\n\n`+
 `## 为什么不能继续 B2\n\n原 Groth16 选票的输入是 Fr，但现成内层验证器的约束系统在 Fq 上；当前外层 BLS12-381 Groth16 和 Jubjub 状态关系都在 Fr 上。\n\n| 对象 | 实际域 | 位数 | 本轮结果 |\n| --- | --- | ---: | --- |\n| 原选票公开输入 | BLS12-381 Fr | 255 | 原证明预检通过 |\n| 现成配对/内层验证 gadget 的约束域 | BLS12-381 Fq | 381 | 仅编译对照通过 |\n| 当前外层及状态关系约束域 | BLS12-381 Fr | 255 | 原 gadget 不能满足接口，E0271 |\n\n`+
 `\`BooleanInputVar<Fr,Fq>\` 能表示输入标量，不会把 Fq 配对约束自动变为 Fr 约束。\`NonNativeFieldVar\` 的存在也不等于已有完整的非原生 G1/G2、扩域、Miller Loop、最终幂及 Groth16 验证器。\n这是锁定版本下直接复用路线的工程阻塞，不是“Arkworks没有任何配对gadget”，更不是递归证明不可能。\n若继续，须另行批准可审计的非原生配对实现或新递归后端/套件设计；不能仅把类型别名改成Fq，或未经说明更换原选票曲线及 Manifest。\n\n`+
 `本地依赖源码及许可证在 \`dependency_evidence/\`，包括直接的 \`BasePrimeField\` 和 \`SNARKGadget\` 类型定义。官方对应版本：[Groth16 v0.4.0 constraints.rs](https://github.com/arkworks-rs/groth16/blob/v0.4.0/src/constraints.rs)。实际依据以归档的锁文件、安装源码 SHA 和编译诊断为准，不引用 latest API 代替0.4。\n\n`+
 `## 没有执行、没有伪造的项目\n\n原证明在当前 Fr 电路内的正例、错误 Nullifier、替换密文、不同上下文和不匹配 VK 全部为 \`NOT_RUN\`；编译失败不是密码学负例。\nR1CS未构建，因此约束数、满足性、外层 Setup/Prove/Verify、联合证明大小、证明者RSS及EVM Gas全部为null；没有用0表示未测。\n新Setup/Prove/DKG/P3次数确实为0。没有提取状态关系、构建 B2/B4 JointBatchCircuit、生成新联合VK或修改Solidity。\n\n`+
 `## 真实运行及资源记录\n\n预算：每工作进程树16GiB、每阶段600秒、总计1200秒；若能够进入R1CS才适用1000万约束准入上限。本轮在域检查处停止，未触及约束/内存上限，不能报告为OOM或约束超限。\n内存栏是1秒采样的整个编译进程树高水位，非证明者内存；短于采样间隔的阶段留null。\n\n`+
 '| 阶段（最终缓存复验） | 退出码 | 耗时 ms | 采样RSS MiB |\n| --- | ---: | ---: | ---: |\n'+m.commands.map(c=>`| ${c.name} | ${c.code} | ${fmt(c.elapsed_ms)} | ${fmt(c.polled_process_tree_peak_rss_mib)} |`).join('\n')+'\n\n'+
 `前两次开发尝试分别保留，不混入最终复验耗时或改写状态：\n\n`+attempts.map(a=>`- \`${path.basename(a.dir)}\`：${attemptNotes[path.basename(a.dir)]??a.m.status} 总${fmt(a.m.wall_ms)} ms；日志和运行源快照保留。`).join('\n')+'\n\n'+
 `run-02的Fq对照编译含尚未命中的依赖编译，实际8141.696ms/采样进程树峰值386.9375MiB；它不是内层验证执行时间或联合证明性能。最终run-03复用编译缓存。\n\n`+
 `## 按方案逐项回答\n\n1. 本轮入口只读公开材料，不读取选民私钥、选择、随机数、权重打开或Merkle witness；尚不能以此宣称联合证明已经实现。\n2. 原Groth16验证没有在当前Fr约束系统中执行；Fq函数仅完成类型检查。\n3. 合法性和累计的共享变量绑定尚未实施，J2未开始。\n4. B2未生成联合证明，不能称成功聚合或递归验证。\n5. 目标电路约束和Prove/RSS尚不可测；仅编译/预检数据有真实来源。\n6. 不扩B4、不测Joint EVM；保留已有SnarkPack、State Proof、P3及Winner-only基线。\n\n`+
 `## 执行与复核\n\n实际运行命令：\`node scripts/joint_proof_run_j1.mjs ${s.run_directory}\`。\n以下为最终尝试逐阶段真实命令（完整输出在各日志）：\n\n`+
 m.commands.map(c=>'- `'+[c.program,...c.args].join(' ')+'` — `'+c.log+'`.').join('\n')+'\n\n'+
 `只读复核：\`node scripts/joint_proof_check.mjs ${s.run_directory}\`。\n新运行必须使用全新目录；运行器拒绝覆盖旧尝试。隔离工具包有故意不能编译的 \`j1-current-outer-fr-rejected\` 目标，不能把该包的all-targets成功当验收要求；它不属于原workspace，原workspace构建不受影响。\n\n`+
 `主Cargo.toml/Cargo.lock、ballot_identity_gadget.rs、state_gadget.rs、EVM exporter、原VK/fixture/Manifest和历史实验归档均未修改。只运行必要的探针/fmt/严格Clippy及原始证明预检，未重跑全workspace的重型电路测试。\n运行源码快照SHA：\`${m.source_version_sha256}\`；原始证据见 \`${s.run_directory}\`。\n\n本轮到J1停止；没有自动提交或推送。\n`;
const comparison=`# 联合证明比较边界（J1结果）\n\n状态：BLOCKED；不是性能比较完成。\n\n当前联合证明路线尚不能在原Fr约束域中实例化内层验证器，B2未开始，因此不能把“一份外层证明”视为实际产物或宣称Gas更低。\nJ1只检查一份原证明的域兼容性，不是B2，更不能与已有两批B4的Stage2数据直接排名。\n\n| 指标 | 联合证明本轮 |\n| --- | --- |\n| 原票/VK/25项输入及-A编码 | 原生公开预检通过 |\n| 当前Fr内层R1CS验证 | 未执行；E0271编译阻塞 |\n| 约束数 / 满足性 | null / null |\n| 外层Setup / Prove / Verify | 未执行 |\n| 联合证明 / 联合VK | 不存在 |\n| 同密文共享绑定 / B2根 / 累计 | J2-J3未开始 |\n| Joint EVM Gas | null |\n| B4准入 | 不通过 |\n\n原SnarkPack+State Proof和Direct Fold现有实验完整保留。State2的27.329%仅是既有逐票回退后端的两批接收Gas节省，部署/认证/接收/冻结总成本反而更高；不能把它与本轮未测的递归路线拼接。\n\n后续只有在另行解决当前Fr非原生配对验证后，才考虑同一B2数据集下的真正联合证明、必要密码学负例及成本比较。此处不自动改变论文构造、加密群、原证明或EVM接收路径。\n`;
fs.writeFileSync(path.join(output,'feasibility.md'),md,{flag:'wx'});fs.writeFileSync(path.join(output,'comparison.md'),comparison,{flag:'wx'});
fs.writeFileSync(path.join(output,'processing_sources.json'),JSON.stringify(['scripts/joint_proof_check.mjs','scripts/joint_proof_report.mjs'].map(p=>({path:p,sha256:sha(fs.readFileSync(p))})),null,2)+'\n',{flag:'wx'});
function files(dir){return fs.readdirSync(dir,{withFileTypes:true}).flatMap(e=>e.isDirectory()?files(path.join(dir,e.name)):[path.join(dir,e.name)]);}
for(const dir of [...attempts.map(a=>a.dir),output]){
 const target=path.join(dir,'SHA256SUMS');assert.ok(!fs.existsSync(target),'archive already sealed');
 const list=files(dir).sort();fs.writeFileSync(target,list.map(p=>sha(fs.readFileSync(p))+'  '+path.relative(dir,p)).join('\n')+'\n',{flag:'wx'});
}
console.log(JSON.stringify({status:m.status,report:output,archived_attempts:attempts.length,recursive_constraints:null,new_proofs:0}));
