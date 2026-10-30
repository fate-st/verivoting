# G7 — 同一 G6 证明 / G4-D VK 的真实 Prague EVM 验证

## 结果：PASS，仅验证器组件，未扩展至 G8/G9

在本地 EthereumJS Prague VM 中实际执行七笔签名交易：分别部署新 G4-D VK 和历史
G4-C 固定 123 VK，再执行一个正例和四个负例。正例使用 **G6 已归档的同一份证明**，
原始 60 项输入逐项仅转换 LE→BE；未生成新证明，未执行新 Setup，未修改 Solidity 源码。

| 交易 | receipt status | 实测总 Gas | 交易 data 字节 |
| --- | ---: | ---: | ---: |
| 部署 G4-D VK 验证器 | 1 | 6,631,884 | 11,405 |
| 部署历史固定 123 VK（仅负例） | 1 | 6,631,824 | 11,405 |
| 正确 G6 证明及原始输入 | 1 | 1,331,401 | 2,564 |
| 错误 Selector | 0 | 1,331,491 | 2,564 |
| 错误 Winner | 0 | 1,331,479 | 2,564 |
| 改动密文 C1 | 0 | 1,331,479 | 2,564 |
| 同一证明/输入，实际旧 VK | 0 | 1,331,479 | 2,564 |

四个负例均 revert `INVALID_GROTH16`。每次验证均实际调用 `0x0c` G1 MSM（61 项，
9,760 B 输入，424,560 Gas）和 `0x0f` 四项配对（1,536 B 输入，168,100 Gas）。
两次预编译均成功，配对正例返回 1、负例返回 0；不是非法点、异常或耗尽 Gas 的假拒绝。
旧 VK 负例的 calldata 与正例完全相同，仅目标合约不同。

## 原始证据与哈希

- [实际交易/Gas/输入/预编译记录](../experiments/g7_raw/g7_p3_prague_evm.json)
  SHA-256：`ecb9efb080fb40ece52df403871f72ab6e63e378a74c776fc40e02a63d1dc161`。
- [EIP-2537 公开 fixture](../fixtures/g6_p3_eip2537_g7_v1.json)
  SHA-256：`81daaa9311a507021883e58ce9e0c1adfc46daaf94c578b04295df4fc85ac356`。
- [ABI、创建字节码、两份实际部署 runtime](../experiments/g7_raw/contract_artifact.json)
  SHA-256：`8c037b618fb27bfefb752abde72d69a91f187b1c885508550c118a13ec284b02`。
- [原生验证/公开编码转换报告](../experiments/g7_raw/public_export.json)、
  [执行命令及退出状态](../experiments/g7_raw/commands.json)、
  [运行环境](../experiments/g7_raw/environment.json)、[整体校验清单](../experiments/g7_raw/SHA256SUMS)。

G4-D 原始压缩 VK SHA-256：
`0d37a77cfc8a1cdfed612b397149d96816188d164e08a74994dd2abfca6cff29`。
G6 原始压缩 proof SHA-256：
`fc3854067baf9b766c83d18e9eabd8db90a477f8d65204991c7359e347eeee4c`。
VK 逐字等于 G4-D fixture，proof 逐字来自 G6 fixture；完整源文件及配置哈希保存在 raw。
错误 VK 为真实 G4-C 固定 123 tally-v2 key：
`734d5a6b573b500de5b64dc90fced31036d15d26fda3118ce21e45da6326b238`。

运行基点：`35fc537e9c97f132537387e87060958f548a64fa`，分支
`experiment/g2d-w32-b4-state`。新增实现运行时尚未提交，raw 如实记录 `git_dirty=true`，
不将该基点冒充新增源码的干净提交。G7 manifest 只约束本次组件实验，不替换正式协议 Suite。

## 负例的精确含义

- Selector：只将零基输入 51 从 0（123）改为 1（124），证明及其余输入不变。
- Winner：只将零基输入 59 从 1 改为 2。
- 密文：先检查原 52..57 输入与 G6 冻结 CT2 的三个 C1 编码精确相同，再令候选人 1
  `C1' = C1 + G`，使用原规范编码/字段打包替换 52、53，其他输入及 CP prefix 不变。
  新点仍为合法子群点。该测试证明原证明拒绝改变后的公开声明，**不证明合约已维护冻结状态**。
- VK：把完全相同的证明与输入发送到装有真实旧 VK 的另一个部署，检测实际关系不兼容。
  这不是关于所有不同 VK 都不能互换的普遍定理。

原生 Arkworks 同样执行 1 正 + 4 负验证：当前 VK 准备 1.184 ms、正例验证 5.645 ms。
这些是转换阶段单次主机测量，不是 EVM Gas 或性能统计。只覆盖 G6 的 123 证明；
没有重复 G4-D 十子集证明生成，也没有宣称十个子集都重新在 EVM 上测试。

## Gas / 字节码口径

原始总 Gas 直接取 signed transaction 的 metered receipt，不挪用 G5-D 的旧关系数据。
Prague 的 calldata floor 同时记录；本次均由 intrinsic+execution 主导，refund=0。
正例：`53,108 intrinsic + 1,278,293 execution = 1,331,401`；
标准 calldata Gas 为 32,108，零/非零字节为 743/1,821，floor 最低值 101,270。

压缩 proof 192 B；EIP-2537 proof 512 B；60 个字段输入 1,920 B；
逻辑 proof+inputs 为 2,432 B，实际 ABI calldata 为 2,564 B（含 selector/offset/length）。
VK 以 EIP 格式为 8,704 B，随一次性部署传入，不混入每次验证的 calldata。
创建字节码 2,605 B，运行字节码 1,807 B；两种 VK 的 runtime hash 不同，因为 VK digest
是构造时 immutable。完整创建和部署字节码均保存，不仅记录大小。

Apple M4 / 32 GiB / macOS arm64，Node v23.11.0；EthereumJS 10.1.3、ethers 6.15.0、
solc `0.8.30+commit.73712a01.Emscripten.clang`；optimizer=200、evmVersion=prague，
配置和依赖锁未修改。交易 gas limit=20M，VM block context limit=30M。
这是同一测试 VM 中顺序独立提交的交易调用，不是把七笔交易声明为一个真实已出块区块。
Node 进程峰值 RSS 370.9375 MiB，包括编译器/VM/验证活动；VM wall time 不是链上确认延迟。

## 实际执行与检查

以下 Rust 命令使用 `CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none`：

```sh
cargo +1.99.1 fmt --all -- --check
cargo +1.99.1 clippy -p vv-friendly-crypto -p vv-friendly-dkg --all-targets --release --locked --offline -- -D warnings
cargo +1.99.1 test -p vv-friendly-crypto -p vv-friendly-dkg --lib --release --locked --offline
cargo +1.99.1 test -p vv-friendly-dkg --bin friendly-g6-native-winner-only --release --locked --offline
cargo +1.99.1 build -p vv-friendly-dkg --bin friendly-g7-p3-eip2537-export --release --locked --offline
target/release/friendly-g7-p3-eip2537-export
node contracts/scripts/g7_p3_verifier.mjs
node scripts/g7_audit.mjs
```

45 项库测试 + 3 项 G6 接收/冻结状态测试通过；严格 Clippy 和格式检查通过。
首次导出因旧 fixture 字段名 `verifying_key_ark_compressed_hex` 与 G6 不同而失败；
仅修正新适配器后重新编译、Clippy、转换、EVM 验收通过，首轮失败日志未删除或伪装为通过。
299 条历史数据/图表清单项、G6 19 项运行源码来源均验证；Cargo.toml 因注册新入口变更，
其历史 hash 对照 `35fc537` 中原字节，不改写 G6 历史 raw/hash。

导出器拒绝覆盖已归档结果。现有公开转换材料可直接重跑本地 EVM，选用新的输出目录：

```sh
node contracts/scripts/g7_p3_verifier.mjs experiments/g7_replay_new
```

该命令不重新 Setup/Prove；原始 G7 结果不被覆盖。完整干净版本统一复现属于后续 G10，
不通过重复重型证明生成来冒充本轮必要验收。

## 仍未完成 / 下一步边界

本轮到 G7 为止。未修改 G1-E、G2-E、G4-D、G6 的旧数据、旧配置、旧电路或论文主文件。
仍未实现：G8 合约批次顺序/旧状态/NF、注册认证、不可变冻结目标、Trustee 签名认证、
从存储重建 P3 输入及一次性 Winner 写入；多进程 Trustee 私密交付也仍待整合。
G9 的完整 SnarkPack EVM 路线仍受阻，未重启冻结的软件 Miller 优化。
没有公链部署、生产可信设置、完整结算 Gas、Full Tally 公平系统比较或统计效率结论。
后续只应按授权进入 G8，不把本轮通过写成完整系统已上链。
