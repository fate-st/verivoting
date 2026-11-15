# J1 联合证明可行性：停在当前域兼容性检查

结论：`BLOCKED_INNER_RECURSIVE_VERIFIER`。这不是成功的联合证明，也不是已经证明递归在任何方案下都不可行。

## 实际完成

- 新分支 `experiment/j1-joint-proof-feasibility`，基准提交 `1ade01f12bc95ab00a514a7986a984816fe1741b`。
- 用原 D5/W32 第一张真实选票、原 VK 和 25 项公开输入做公开材料核对；Ark 原证明192 B、EIP-2537证明512 B逐字节对应，正确保留 `A` 与 `-A` 区别。原生验证通过仅为输入预检，没有把宿主 bool 写入 witness。
- 在独立工具包启用锁定的 ark-groth16 0.4.0 `r1cs`，完整 Fq 验证器约束函数通过 Release 编译及严格 Clippy，但未调用该函数构建约束。
- 对同一个原 BLS12-381 验证器，要求当前 `SNARKGadget<Fr,Fr,Groth16<Bls12_381>>` 接口；真实编译退出101，报告 `E0271`：FqConfig 与 FrConfig 的 BasePrimeField 不相等。没有使用缺少 feature/import 的报错冒充域不兼容。

## 为什么不能继续 B2

原 Groth16 选票的输入是 Fr，但现成内层验证器的约束系统在 Fq 上；当前外层 BLS12-381 Groth16 和 Jubjub 状态关系都在 Fr 上。

| 对象 | 实际域 | 位数 | 本轮结果 |
| --- | --- | ---: | --- |
| 原选票公开输入 | BLS12-381 Fr | 255 | 原证明预检通过 |
| 现成配对/内层验证 gadget 的约束域 | BLS12-381 Fq | 381 | 仅编译对照通过 |
| 当前外层及状态关系约束域 | BLS12-381 Fr | 255 | 原 gadget 不能满足接口，E0271 |

`BooleanInputVar<Fr,Fq>` 能表示输入标量，不会把 Fq 配对约束自动变为 Fr 约束。`NonNativeFieldVar` 的存在也不等于已有完整的非原生 G1/G2、扩域、Miller Loop、最终幂及 Groth16 验证器。
这是锁定版本下直接复用路线的工程阻塞，不是“Arkworks没有任何配对gadget”，更不是递归证明不可能。
若继续，须另行批准可审计的非原生配对实现或新递归后端/套件设计；不能仅把类型别名改成Fq，或未经说明更换原选票曲线及 Manifest。

本地依赖源码及许可证在 `dependency_evidence/`，包括直接的 `BasePrimeField` 和 `SNARKGadget` 类型定义。官方对应版本：[Groth16 v0.4.0 constraints.rs](https://github.com/arkworks-rs/groth16/blob/v0.4.0/src/constraints.rs)。实际依据以归档的锁文件、安装源码 SHA 和编译诊断为准，不引用 latest API 代替0.4。

## 没有执行、没有伪造的项目

原证明在当前 Fr 电路内的正例、错误 Nullifier、替换密文、不同上下文和不匹配 VK 全部为 `NOT_RUN`；编译失败不是密码学负例。
R1CS未构建，因此约束数、满足性、外层 Setup/Prove/Verify、联合证明大小、证明者RSS及EVM Gas全部为null；没有用0表示未测。
新Setup/Prove/DKG/P3次数确实为0。没有提取状态关系、构建 B2/B4 JointBatchCircuit、生成新联合VK或修改Solidity。

## 真实运行及资源记录

预算：每工作进程树16GiB、每阶段600秒、总计1200秒；若能够进入R1CS才适用1000万约束准入上限。本轮在域检查处停止，未触及约束/内存上限，不能报告为OOM或约束超限。
内存栏是1秒采样的整个编译进程树高水位，非证明者内存；短于采样间隔的阶段留null。

| 阶段（最终缓存复验） | 退出码 | 耗时 ms | 采样RSS MiB |
| --- | ---: | ---: | ---: |
| fmt-workspace | 0 | 391.340 | 未测（null） |
| fmt-isolated | 0 | 48.307 | 未测（null） |
| build-public-preflight | 0 | 73.241 | 未测（null） |
| clippy-public-preflight | 0 | 140.696 | 未测（null） |
| public-fixture-preflight | 0 | 495.189 | 未测（null） |
| build-native-fq-control | 0 | 47.360 | 未测（null） |
| clippy-native-fq-control | 0 | 109.655 | 未测（null） |
| native-fq-control | 0 | 216.201 | 未测（null） |
| current-outer-fr-compile | 101 | 86.932 | 未测（null） |

前两次开发尝试分别保留，不混入最终复验耗时或改写状态：

- `run-20261114-01`：开发错误：公开来源输出的 JSON 宏语法错误；不是递归不可行证据。 总10080.672 ms；日志和运行源快照保留。
- `run-20261114-02`：原证明核对和 Fq 对照编译通过，Fr 编译实际报 E0271；分类器错误地只期待 E0277，原 FAIL 保留，不改写为 PASS。 总18013.353 ms；日志和运行源快照保留。
- `run-20261114-03`：修正结果分类后缓存复验；接受真实 E0271 域不兼容作为 J1 BLOCKED，非密码学负例通过。 总1722.868 ms；日志和运行源快照保留。

run-02的Fq对照编译含尚未命中的依赖编译，实际8141.696ms/采样进程树峰值386.9375MiB；它不是内层验证执行时间或联合证明性能。最终run-03复用编译缓存。

## 按方案逐项回答

1. 本轮入口只读公开材料，不读取选民私钥、选择、随机数、权重打开或Merkle witness；尚不能以此宣称联合证明已经实现。
2. 原Groth16验证没有在当前Fr约束系统中执行；Fq函数仅完成类型检查。
3. 合法性和累计的共享变量绑定尚未实施，J2未开始。
4. B2未生成联合证明，不能称成功聚合或递归验证。
5. 目标电路约束和Prove/RSS尚不可测；仅编译/预检数据有真实来源。
6. 不扩B4、不测Joint EVM；保留已有SnarkPack、State Proof、P3及Winner-only基线。

## 执行与复核

实际运行命令：`node scripts/joint_proof_run_j1.mjs experiments/joint_proof_raw/run-20261114-03`。
以下为最终尝试逐阶段真实命令（完整输出在各日志）：

- `cargo +1.99.1 fmt --all -- --check` — `logs/fmt-workspace.log`.
- `cargo +1.99.1 fmt --manifest-path tools/j1-inner-verifier-probe/Cargo.toml --all -- --check` — `logs/fmt-isolated.log`.
- `cargo +1.99.1 build --release --locked --offline -p vv-friendly-dkg --bin j1_inner_verifier_probe` — `logs/build-public-preflight.log`.
- `cargo +1.99.1 clippy --release --locked --offline -p vv-friendly-dkg --bin j1_inner_verifier_probe -- -D warnings` — `logs/clippy-public-preflight.log`.
- `cargo +1.99.1 run --release --locked --offline -p vv-friendly-dkg --bin j1_inner_verifier_probe -- /Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter/experiments/joint_proof_raw/run-20261114-03/public_preflight.json` — `logs/public-fixture-preflight.log`.
- `cargo +1.99.1 build --release --locked --offline --manifest-path tools/j1-inner-verifier-probe/Cargo.toml --bin j1-native-fq-compile` — `logs/build-native-fq-control.log`.
- `cargo +1.99.1 clippy --release --locked --offline --manifest-path tools/j1-inner-verifier-probe/Cargo.toml --bin j1-native-fq-compile -- -D warnings` — `logs/clippy-native-fq-control.log`.
- `cargo +1.99.1 run --release --locked --offline --manifest-path tools/j1-inner-verifier-probe/Cargo.toml --bin j1-native-fq-compile` — `logs/native-fq-control.log`.
- `cargo +1.99.1 build --release --locked --offline --manifest-path tools/j1-inner-verifier-probe/Cargo.toml --bin j1-current-outer-fr-rejected --message-format=json` — `logs/current-outer-fr-compile.log`.

只读复核：`node scripts/joint_proof_check.mjs experiments/joint_proof_raw/run-20261114-03`。
新运行必须使用全新目录；运行器拒绝覆盖旧尝试。隔离工具包有故意不能编译的 `j1-current-outer-fr-rejected` 目标，不能把该包的all-targets成功当验收要求；它不属于原workspace，原workspace构建不受影响。

主Cargo.toml/Cargo.lock、ballot_identity_gadget.rs、state_gadget.rs、EVM exporter、原VK/fixture/Manifest和历史实验归档均未修改。只运行必要的探针/fmt/严格Clippy及原始证明预检，未重跑全workspace的重型电路测试。
运行源码快照SHA：`77d62c7e9695b7f2eea97c4627f57164cd570179505f59db3febfd7103989ac2`；原始证据见 `experiments/joint_proof_raw/run-20261114-03`。

本轮到J1停止；没有自动提交或推送。
