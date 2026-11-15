# G18：最终证明架构决策实验

## 结论：INCONCLUSIVE_RESOURCE_OR_ENVIRONMENT

本轮在 G18-1 的环境准入处停止。**本机真实 Composite 证明器可用，但当前固定 Apple M4 环境不能提供官方支持的 EVM Groth16 包装。**
这不是 A/B 的密码学失败，也不是 C 的性能胜出；没有足够证据选 A、B 或宣称新架构更高效。
本轮未进入 G18-2/3/4，未自动安装 Docker、改用远程证明服务或 Linux，不开启 G19。

## 1. 固定基准、配置与原数据

基准提交：`151ed277d76ed0344c3fb19938319f22a179675c`；独立分支：`experiment/g18-final-architecture-decision`。执行源为该提交加本轮未提交的新文件，不称干净提交复现。
配置：`configs/experiment.G18-DECISION.json`，SHA-256：`d6879aee4027db1073ff701e55d562a3f14b9cc487568a33db2b44bd584ab7b1`。正式记录开始后未改字节。
固定 D5/W32、32名注册选民、原8票、3候选人、两个连续 B4；原 VK、全部25项输入、认证上下文、批次根、CT2及冻结目标均按原归档锁定。
八份原证明/有序输入逐项与 G2-E 及 G8 EIP 编码交叉核对；CT2 一致。未更换数据集、曲线、原 SnarkPack V2 或 Jubjub 累计定义。
SDK/r0vm3.1.0、risc0-build3.0.7、guest Rust1.97.0、host Rust1.99.1、Prague固定。原配置、锁、合约、G8—G17与论文未修改。

固定预算：guest600秒/16GiB、真实proof1800秒/24GiB；每路线最多一次正式实现和一次明确工程修复复测，实用修复复测次数为0。
监控为每秒主机单进程/进程树 RSS 与墙钟软限制，不是内核硬限或GPU内存计量。15%同条件总生命周期Gas门槛未变，未因失败提高预算。

## 2. 本机包装后端预检

机器：Apple M4，32GiB，darwin/arm64。
实际 probes 保留 r0vm版本/能力、系统版本、Docker/CUDA及开发工具结果；原官方 r0vm 二进制摘要与 G17 一致。
Docker 与 nvidia-smi 实际不存在；xcrun 找不到 metal 编译工具，但这**不被用作“普通证明不支持”的推断**，其能力由下面实际证明检验。

[官方3.1本地证明文档](https://dev.risczero.com/api/generating-proofs/local-proving)区分普通CPU/Metal证明与只支持x86的Groth16包装，并明确排除Apple Silicon，即使使用Docker。
[官方v3.1.0发布说明](https://github.com/risc0/risc0/releases/tag/v3.1.0)给出配套新设置、Docker镜像及新链上验证器版本；没有沿用旧包装或旧VK。
锁定 `risc0-groth163.0.6/src/prove/docker.rs` 同样要求x86上的Docker；安装源码逐字节匹配原G17锁中的官方crate归档，副本与摘要已保存。
所以包装标为 `BLOCKED_APPLE_SILICON_UNSUPPORTED`，另有Docker缺失。未执行包装调用、未伪造一个包装失败交易或Gas数字。

## 3. 一次最小官方真实证明自测

采用锁定SDK自带 `examples/loop.bin`，按官方datasheet方式与原 `V1COMPAT_ELF` 组合，输入原始4字节零、循环计数0、预期空journal。
这是官方最小控制程序，不是修改后的选票验证器，也不是A/B功能用例。官方二进制/源码及crate归档摘要均与原锁匹配。
薄host适配器显式调用同一个本地 `ExternalProver`；SDK启用 `disable-dev-mode`，运行环境及ProverOpts也关闭dev-mode，拒绝Fake/非Composite类型。
证明生成后执行SDK `Receipt::verify(image_id)`，并核对空journal，保留真实bincode Receipt。

| 实测项 | 本轮值 |
| --- | --- |
| 真实 Composite 生成 | PASS |
| Receipt / Image ID / journal 验证 | PASS |
| Prove | 1.758055秒 |
| Receipt Verify | 8.399毫秒 |
| Receipt序列化大小 | 198990 B |
| user / total cycles | 169 / 16384 |
| 段数 | 1 |
| 采样单进程 / 进程树RSS | 159.109 / 162.531 MiB |

只有一次功能样本，RSS只有1个采样点，不是精确峰值/GPU内存。Prove不是Execute时间；构建67.024秒单列，不计入Prove。
这个小程序仅169 user cycles，不能外推到G17的数亿cycles选票验证，更不能将其1份Receipt当作B1选票证明。

- 控制程序Image ID：`691fbe91155b80f5edc7d0f3fb91b8065b84f2a3cb392312d7ace283ab23b34d`。
- 控制程序用户ELF SHA：`cc7bdb6579ecf77a6bbaacb886953ac87eb36851609450aec20242395bbe67af`。
- 组合程序SHA：`6a0f59fe40634cebb6554f3e51a610b3794050478bd1d1d6266cbeb696ebae15`。
- 真实Receipt SHA：`55ce587e814f69d5762ba0b4d034a8afb67d2b16aa744c80577f7905ca872b6f`。

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

实际执行入口：`node scripts/g18_run.mjs experiments/g18_raw/run-20261115-01`。这是已完成的唯一正式运行记录，不建议自动重复。
依次完成环境probe、隔离锁生成、locked/offline Release构建、fmt、官方程序准备和一次真实prove/verify。
全部argv、退出状态、起止时间、资源样本和日志在 [execution.json](../experiments/g18_raw/run-20261115-01/execution.json)。原输入、ELF、组合程序和Receipt均保留在official_smoke子目录。
本轮没有工程失败复测或更换证明后端；未自动提交推送。已有block0.1.6未来Rust兼容提示保留在构建日志，未改依赖。

只读检查：`node scripts/g18_report.mjs --check`。它检查冻结配置、输入、源码快照、Receipt字节摘要、结果和2879个原跟踪文件不变；不新生成证明或执行EVM。
它的PASS是归档完整性，不是对A/B架构或新EVM可行性的准入。

输出：[prover_admission.json](../results/g18/prover_admission.json)、[functional_admission.json](../results/g18/functional_admission.json)、
[proof_and_evm_costs.json](../results/g18/proof_and_evm_costs.json)、[final_decision.json](../results/g18/final_decision.json)及SHA256SUMS。

## 6. 决策边界与停止

最终只能给 `INCONCLUSIVE_RESOURCE_OR_ENVIRONMENT`，而不是SELECT_A/SELECT_B/KEEP_C_AND_REDESIGN的效率判断。
C继续作为已有可运行基线保存，并不代表本轮重新测得C优于A/B。未评价15%阈值，没有任何未运行架构的性能数据。
本轮到此停止，不自动G19。若要继续，须另行授权统一、受支持的证明目标环境并建立新的冻结记录；不得改写本次配置/失败边界，不能混用M4与其他机器数字排名。
