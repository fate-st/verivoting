# G17 Z1：工具链恢复与四项真实 guest 执行

## 结论

2026-11-15，`PASS_Z1_GUEST_EXECUTION_NOT_PROOF`。锁定的官方工具链已恢复，真实 guest/host Release 构建成功；原 B1/B2 正例和单字段错误输入四项均符合预期。
本轮到 Z1 停止。未实现 SnarkPack guest、状态累计、外层证明或新合约，未修改论文、原密码学协议或历史结果。
基准提交 `34097bda0e1ed9887a91fd4a721ffc172202f512`；分支 `experiment/g17-zkvm-abc-comparison`。
实际执行使用本轮未提交恢复脚本，不能称为干净提交复现；源码快照及摘要在 [原始归档](../experiments/zkvm_abc_raw/run-20261115-03/source_files.json)。

## 工具链恢复及真实构建

沿用 SDK/r0vm 3.1.0、risc0-build 3.0.7、guest Rust 1.97.0、host Rust 1.99.1，没有降级。
之前终端未使用 macOS 已配置的本地代理。本轮仅在下载命令中显式使用现有代理，仍从官方 GitHub 获取原定版本，未改系统网络设置。

| 官方资产 | 字节数 | SHA-256 |
| --- | --- | --- |
| cargo-risczero-3.1.0-verified.tgz | 51568850 | `3b50b0393b8e4158e7129add4cfe0cbc536e994b5abc6a3a435e1e6b841c0990` |
| rust-toolchain-1.97.0-verified.tar.gz | 442966921 | `85070d82aebbe8e1027e75f2cd0c781f5c82c0ae7e06697218be9d8b71dec15d` |

摘要逐项匹配前轮已归档的官方 release API，完整性通过后才解包。SDK 归档本身包含 cargo-risczero 与 r0vm。
采用 rzup 识别的项目内版本目录；版本查询均通过，不修改全局 rustup 默认、Cargo shim 或 shell profile。工具包留在忽略的 target 缓存，不上传约 500MB 安装包。
[官方安装说明](https://dev.risczero.com/api/zkvm/install)、[SDK v3.1.0](https://github.com/risc0/risc0/releases/tag/v3.1.0)、[guest Rust 发布](https://github.com/risc0/rust/releases/tag/r0.1.97.0)。

实际编译发现另一处主机工具故障：guest 已编译，但 host 的 rust-objcopy 停在 dyld abort_with_payload。采样栈和退出101的首次构建均保留，操作员仅停止该任务的 rustc/objcopy。
本轮命令改为 `CARGO_PROFILE_RELEASE_STRIP=none`，不运行故障的 host strip；没有改动 guest 源码、验证算法、优化等级或全局工具链。新建 guest Cargo.lock 固定了首次实际解析的依赖。
第二次构建成功：81.527秒；这包含主机 profile 变化导致的依赖重编译，不作为 guest 性能。
随后锁定/offline 构建及两个工作区 fmt 通过。严格 Clippy 没有通过：其 RUSTC_WORKSPACE_WRAPPER 主机驱动进入 risc0-build 的嵌套 guest 构建，产生 E0463 缺少目标 std；这是工具调用问题，不是上述正常 guest 构建或四项密码学执行失败。
保留 clippy-zkvm 的退出101和完整日志；不使用 RISC0_SKIP_BUILD、不修改 SDK 或 guest 来伪造检查成功。依赖 block 0.1.6 的未来 Rust 兼容提示也保留，未改依赖来掩盖它。

## 程序与原始输入

- 真实用户 RISC-V ELF：`e423e639e9810688b0bf0820df046a454bd564ef9ab9bdc686a547cf6be24b7a`（591256 B）。
- SDK 组合程序：`d19a6ab39dc71a5730c3ca60c4ea7a61beacf66f15d91f2ecf6fc61d18ef2f65`（623680 B）。SDK 的 G17_GUEST_B_ELF 常量实际嵌入 user ELF + kernel ProgramBinary；原结果字段 guest_elf_sha256 指的是这一组合程序，不能混作裸 ELF 摘要。
- Image ID：`615c9d1381e2b04edfe1a83428a6d848f1905814c7d1b1b646ed0654b903a321`。生成 methods.rs、独立 r0vm --id 和四份执行结果一致。
- 原 BLS12-381 VK：`6d00cab6cd02e9926bd8ef05f8e782fbf1f32b5993c134699155accf7b37aa98`，每票25项公开输入。复用前轮已有两张真实证明，不做 Setup/Prove。

输入直接复用 run-02 四份文件；两个反例仅把最后一票 input[7] 改为 Fr+1，原证明/VK不动。共享完整 Groth16 验证函数未修改，guest 自行验证，不接收 host 成功标志。

## 四项实际结果（单样本）

| 用例 | guest 结果 | user cycles | executor 秒 | 单进程采样 RSS MiB | 进程树采样 RSS MiB |
| --- | --- | --- | --- | --- | --- |
| b1-valid | 接受 | 481270965 | 3.662 | 48.41 | 93.11 |
| b1-changed-input | 密码学拒绝 | null | 3.825 | 49.45 | 93.44 |
| b2-valid | 接受 | 714456045 | 5.355 | 49.13 | 92.83 |
| b2-changed-input | 密码学拒绝 | null | 5.370 | 49.08 | 93.69 |

错误输入在 guest 内分别触发 G17_INVALID_GROTH16_0、G17_INVALID_GROTH16_1；不是下载、构建、解析、超时或内存错误。原生计算仅作对照，未填入 guest 性能栏。
正例 cycles 为 SDK SessionInfo 的 user-cycle 合计，不含填充/续算开销；反例沿用已有 panic 拒绝接口，SDK 返回错误而不返回 SessionInfo，因此周期/段数为 null，不估算、不为补数字重写 guest。
时间为本地 IPC execute 调用，不含证明生成，也不等于整个 host 命令时间。内存是每秒轮询的主机进程/树 RSS，不是精确峰值、guest 逻辑内存或证明器内存。
机器 Apple M4、32 GiB、10逻辑核。四项顺序执行，各600秒/单进程16GiB软预算，均未触发上限。
这些仅是四个功能样本，不能据此推导统计性能优势或真实 zkVM Prove 成本。

## 实际命令与复现

仓库根目录中使用恢复脚本，它只给子进程固定 PATH/RISC0_HOME/本地 r0vm/IPC/dev_mode=0，不调用 prover，也不修改全局环境。
全部实际 argv、起止时间、退出码和 stdout/stderr 见 [原始命令目录](../experiments/zkvm_abc_raw/run-20261115-03/)。

`node experiments/zkvm_abc/scripts/recovery_exec.mjs cargo build --manifest-path experiments/zkvm_abc/Cargo.toml --release --locked -p g17-host --features zkvm`

四例由现有资源记录器逐一执行，形式如下；将 CASE 换为 b1-valid、b1-changed-input、b2-valid、b2-changed-input，并使用不存在的新输出路径，不能覆盖归档：

`node experiments/zkvm_abc/scripts/recovery_exec.mjs host guest experiments/zkvm_abc_raw/run-20261115-02/materials/CASE.json /ABS/NEW_RESULT.json`

只读检查：`node experiments/zkvm_abc/scripts/recovery_report.mjs --check`。
旧 G17/G12 检查器也已实际运行通过；旧 BLOCKED 报告及 run-01/02 保持原样，当前结果单独归档在 [guest_execution.json](../results/g17_z1_recovery/guest_execution.json)。

## 是否进入 SnarkPack guest

可以进入下一轮 Z2 的设计/准入审查，但本轮不自动编码执行。现在只证实原 BLS12-381 Groth16 的纯 Rust 完整验证可运行；原 SnarkPack 的 BLST FFI/Rayon、全部 FS/GT/GIPA/KZG/TIPP/MIPP 与随机累积健全性仍需逐项解决。
不能用固定种子随机批检替代未经证明的原语义，也不能由这次 execute 成功推出证明生成、EVM 包装或完整投票状态绑定可行。
本轮 receipt/prove 时间/证明大小/Gas 均 null。未自动提交推送。
