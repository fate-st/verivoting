# G17：zkVM A/B/C 三路线——Z0/Z1 首轮报告

## 结论：BLOCKED_COMPATIBILITY（工具链下载，非密码学不可行）

2026-11-15。分支 `experiment/g17-zkvm-abc-comparison`，基准 `a73d7f59e73317e18f98a90fea7aaf82856eb216`。
本轮只执行用户方案最后一段规定的 Z0/Z1，不开发 SnarkPack guest、累计关系、外层证明或新合约。
**原始证明的原生验证和编码对照通过，但 guest 编译/执行尚未完成。不能称 PASS_GUEST。**

## 实际完成与阻塞

- 新增独立 Cargo 工作区，根 Cargo.toml/Cargo.lock、协议 Manifest、原曲线/VK/SRS/证明、合约、P3 和论文全部不动。
- 复用 W32 最前两张原票；由 G8 EIP VK 重建 Ark VK，SHA-256与原值 `6d00cab6cd02e9926bd8ef05f8e782fbf1f32b5993c134699155accf7b37aa98` 一致。
- 对照两张原192B证明与原512B EIP证明，明确 EIP存的是-A；全部25个公开输入执行LE/BE一致性检查。
- 共享 `verify_batch` 是完整 BLS12-381 Groth16 验证：规范解析/子群、固定VK、全部输入与配对式；不接收 host成功标志，不做随机批检。
- guest源码调用同一函数，只有验证成功才能 commit；但未生成ELF，带zkvm feature的最终host代码也尚未完整typecheck，不能把源码存在当成 guest通过。
- 两次真实 guest构建均到达 risc0-build 后因 `Risc Zero Rust toolchain not found` 退出101；没有密码学拒绝，也不是J1的Fq/Fr问题。

初轮Clippy只因chunks_exact的新lint失败，run-01日志及源码保留；改为等价as_chunks后，run-02原生Release、严格Clippy和fmt通过，并重新执行下列4例。

## 原生对照：不是 zkVM 性能

| 用例 | 原生完整验证 | 原生验证 ms（单样本） | guest | guest cycles |
| --- | --- | --- | --- | --- |
| b1-valid | 接受 | 4.386 | NOT_RUN | null |
| b1-changed-input | 密码学拒绝 | 4.279 | NOT_RUN | null |
| b2-valid | 接受 | 6.377 | NOT_RUN | null |
| b2-changed-input | 密码学拒绝 | 6.509 | NOT_RUN | null |

反例只改最后一张票 input[7] 为原Fr+1，原证明/VK不变；两次分别得到G17_INVALID_GROTH16_0/1，确实到达Groth16拒绝。
上述时间包含本轮适配器的规范解析/prepare/验证等函数开销，只有功能控制意义，不是独立选举统计，不与任何历史或外部数据排名。
没有guest运行时间、周期、内存、program ID或receipt；全部为null。原生控制太快未取得1秒RSS样本，其内存也为null，不填0。

## 环境与工具链：记录失败，不降低版本

机器：Apple M4，32 GiB，10逻辑核；host为aarch64-apple-darwin。
已有 `cargo 1.99.1 (5f94df478 2026-08-27)`、Node `v23.11.0`。默认stable的cargo组件不可用，本轮使用 `cargo +1.99.1`，未修改全局默认工具链。
`rzup 0.6.0`来自官方安装脚本指定地址，下载到target/g17-tools；只检查脚本，不执行其修改shell profile的步骤。
RISC0_HOME固定在项目忽略的工具缓存，不写入shell profile，不用第三方镜像。

[官方安装文档](https://dev.risczero.com/api/zkvm/install)列出arm64 macOS，并要求固定工具版本。
[官方v3.1.0发布](https://github.com/risc0/risc0/releases/tag/v3.1.0)包含安全修复，本轮锁定SDK/r0vm3.1.0、build crate3.0.7及guest Rust1.97.0，不降级到旧教程版本。
SDK crate可取得，但二进制SDK/guest Rust资产没有下载完整；安装目标是本机版本，不是远程证明服务。

| 真实命令尝试 | 退出码 | 墙钟秒 | 证据 |
| --- | --- | --- | --- |
| install-rust | 1 | 92.486 | [日志](../experiments/zkvm_abc_raw/run-20261115-01/logs/install-rust.stderr.log) |
| install-sdk | 1 | 75.794 | [日志](../experiments/zkvm_abc_raw/run-20261115-01/logs/install-sdk.stderr.log) |
| download-rust-api | 28 | 170.023 | [日志](../experiments/zkvm_abc_raw/run-20261115-01/logs/download-rust-api.stderr.log) |
| download-sdk-api | 28 | 170.034 | [日志](../experiments/zkvm_abc_raw/run-20261115-01/logs/download-sdk-api.stderr.log) |

rzup下载分别发生TLS EOF/连接超时；同一官方GitHub API资产入口也因超时只下载部分字节。
官方SDK资产应为51568850 B，Rust资产应为442966921 B；期望SHA来自已归档官方发布API。
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

`node experiments/zkvm_abc/scripts/report.mjs --check`

它只读检查全部原始命令/结果/材料、当前源码与快照、摘要及2603个旧跟踪文件；不启动VM或证明。
`--write`只首次生成新的G17归档，已有不同内容时拒绝覆盖。
报告清单在 [results/g17/SHA256SUMS](../results/g17/SHA256SUMS)，相对路径以仓库根为准。

网络恢复后仅继续以下最小步骤，**不自动执行Z2**：

1. 从已归档官方地址下载同一SDK3.1.0、guest Rust1.97.0并核对发布SHA，使用同一RISC0_HOME安装；固定r0vm3.1.0。
2. 执行 `cargo +1.99.1 build --manifest-path experiments/zkvm_abc/Cargo.toml --release --locked -p g17-host --features zkvm`。
3. 新run目录逐例执行 `g17-host guest INPUT NEW_RESULT_JSON`，每例600秒/单进程16GiB软预算；宿主IPC只使用本地r0vm，不生成receipt。
4. 必须实际得到B1/B2正例、B1错误输入和B2第二票错误输入的guest密码学结果与原生对应；记录cycles/segments、ELF/image ID和实际时间。
5. 四例通过后停止复核，再决定Z2。若失败保留日志；不得用host原生成功、假receipt、RISC0_SKIP_BUILD或资源超时冒充guest通过。

当前代码与结果未自动提交或推送。完整失败日志、两轮原生控制、两轮源码与锁版本均保留，不覆盖G8—G12历史。
