# G8 第一阶段：真实逐票备用验证 + 连续状态接收 + CT2 冻结

## 结论：第一阶段 PASS，完整 G8 仍为 PARTIAL

本轮只完成接收与冻结，没有把合约外壳或 State Proof 单独冒充完整结算。
新增 `VeriVotingSettlement.sol`，在本地 Prague VM 的同一合约实例中真实执行：

`八份原始单票证明 → 两次 B4 状态验证/写入 → CT2 → 一次冻结`。

**选票合法性走真实逐票 Groth16 备用/对照路线，不是 SnarkPack EVM。**
不修改已有曲线、R1CS、公开输入布局或冻结 D5/W32 配置；独立 G8 descriptor 明确该路线。
不重跑 G1-E/G2-E/G4-D/G6/G7 的证明生成；Setup=0、Prove=0。

## 实现及与论文的对应关系

- `interfaces/IBatchVerifier.sol` 定义“失败即 revert”的真实验证接口。
- `IndividualGroth16BatchVerifier.sol` 为每批调用四次原 BLS12-381 Groth16 验证器。
  Settlement 构造器内部创建这个具体适配器和状态验证器，不接收外部 mock 地址。
- `VeriVotingSettlement.sol` 固定 Election ID、DKG/上下文、公钥、注册根/关闭摘要、策略、
  P3 VK hash、两个已接纳批次根及参数；没有可修改这些锚点的 setter。
- 合约检查 next、旧状态、每行上下文/注册根、批内与跨批 NF，再用同一个四行矩阵重算
  七次 SHA-256 的 canonical batch tree。四份合法性证明与该矩阵绑定；随后从合约状态及
  本次声明重建全部 37 个 State Proof 输入，验证通过后才写状态/NF/next。
- `freeze()` 不接受外部 CT 或目标摘要，从已验证的存储生成 CP target 和 seal，拒绝提前、
  重复冻结及冻结后接收。其两份摘要与原始 G6 逐字一致。

这对应论文 C2 的“同一有序表示绑定合法性和状态”以及 `FreezeTallyTarget` 的一部分实现证据。
论文正式路线仍采用 SnarkPack；本轮逐票路线不能用于声称正式聚合路线已上链或获得批量效率。
论文 C3/C4 的认证私包、`AcceptWinnerDisclosure` 及完整隐私/联合健全性结论没有因此完成。
本轮没有修改论文主文件。

## 真实 EVM 数据

| 实际交易 | 总 Gas（receipt） | 实际交易 data |
| --- | ---: | ---: |
| 部署接收合约及三个真实验证组件 | 10,358,066 | 25,697 B |
| 接收首批 B4 | 3,333,283 | 6,948 B |
| 接收第二批 B4 | 3,335,455 | 6,948 B |
| 从存储冻结 CT2 | 144,836 | 4 B |

每批实际调用四次 25-input 单票验证和一次 37-input 状态验证：五次 G1 MSM、五次四项配对，
配对均返回 1。共八份单票与两份状态证明，独立审计将十次预编译输入 hash 与原 fixture 的
VK、proof、每一行公开输入逐一对应，不只是检查“有调用过配对”。
EIP-2537 编码/预编译依据：[官方规范](https://eips.ethereum.org/EIPS/eip-2537)；
外层参数依据 [Solidity ABI](https://docs.soliditylang.org/en/v0.8.30/abi-spec.html)，不混淆 Arkworks 压缩编码。

两批 execution Gas 都为 3,240,031；intrinsic 分别为 93,252/95,424，差额来自实际 data 字节。
Prague calldata floor 已计入原始记录：一些快速拒绝由 floor 主导，不能简单把 execution 当总 Gas。
仅记录本次组件流程，不把 G7 或旧 W10 数据相加成完整系统 Gas。

## 必要的负例（13/13，全部原子拒绝）

| 情形 | 合约拒绝原因 |
| --- | --- |
| 未完成两批就冻结 | INCOMPLETE |
| 跳过第一批 | NEXT_INDEX |
| 替换旧状态 | OLD_STATE |
| 选票注册根不同于初始化根 | REGISTRATION_ROOT |
| 修改预定批次根 | ACCEPTED_ROOT |
| 重排真实合法选票及对应证明 | BATCH_ROOT |
| 批内重复同一合法选票/NF | DUPLICATE_NULLIFIER |
| 第一行配错另一份真实单票证明 | INVALID_GROTH16 |
| 四票均有效，但配错真实状态证明 | INVALID_GROTH16 |
| 首批重放 | NEXT_INDEX |
| 第二批重用首批真实合法票/NF | SPENT_NULLIFIER |
| 重复冻结 | FROZEN |
| 冻结后接收 | FROZEN |

每次拒绝前后检查完整 checkpoint 和八个 NF 标记相同，没有接受事件。
错误单票负例的配对返回 0；错误状态负例先有四次正确配对，最后状态配对返回 0。
这些不是非法点、OOG 或链下 success flag 制造的假验证。

“不同于初始化根被拒绝”**不等于已认证注册关闭证书**：当前证书/DKG/注册/接纳日程锚点
仍由部署者从已检查的 G6 公开材料预置，并没有在 EVM 上验证其原 Schnorr 证书。

## 原始证据与来源

- [实际交易与 checkpoint](../experiments/g8_raw/receiver_freeze.json)
  SHA-256：`d60ef561cccb88d0ddc9abdc9c58bb4426e32a8a714269dfa664a6a6d5e5f45a`。
- [公开 EIP-2537 fixture](../fixtures/g8_receiver_eip2537_v1.json)
  SHA-256：`a2952ff452dd40e920360e473eea1ea2c3a374b1e7704902f16bb9ec18ce55bb`。
- [ABI/创建字节码/四份实际部署 runtime](../experiments/g8_raw/contract_artifacts.json)
  SHA-256：`65e615e8a46e1ab511989dc8c8ef6ec96d819eb459a2e2cf31b2c44d42c661fb`。
- [原生重放与编码报告](../experiments/g8_raw/public_export.json)、[执行命令](../experiments/g8_raw/commands.json)、
  [环境](../experiments/g8_raw/environment.json)、[来源/trace 审计](../experiments/g8_raw/audit.json)、
  [整体 SHA-256 清单](../experiments/g8_raw/SHA256SUMS)。

单票 Ark VK SHA-256：`6d00cab6cd02e9926bd8ef05f8e782fbf1f32b5993c134699155accf7b37aa98`，
从 D2 原 Bellperson VK 转换后核对历史 Ark hash；状态 VK：
`92871cac5c3452ab0a1647939507d7b041dcccf189e3b60ea2b19c1d6e1e1856`。
没有新的密钥设置。冻结 seal：
`05a2ce08ef1edc92398fb7990b5896387aa2efcef5c7d885c5fab676b7e338c0`，与 G6 相同。

基点 `9baed4793f9ef634a2aa7d88ac65c72518ccf266`，分支
`experiment/g8-winner-only-settlement`；新增实现运行时 `git_dirty=true`，源码快照/逐文件 hash
记录实际来源，不把 G7 基点冒充干净 G8 提交。324 条历史数据/图表清单项保持不变。
Cargo/package 仅新增入口，历史 G7 的相关源码 hash 对照 `9baed479`，不篡改旧清单。

Apple M4 / 32 GiB / macOS arm64，Node v23.11.0，EthereumJS 10.1.3，solc 0.8.30，
optimizer=200、**viaIR=true**、evmVersion=prague。本阶段 IR 设置不同于 G7，不能直接作二者
效率排名。原通用验证器源文件未改。创建字节码 15,041 B；接收器/适配器/单票验证器/状态验证器
runtime 分别 7,293/851/1,764/1,764 B。Node 进程峰值 RSS 424.46875 MiB 包含编译和 VM，
不是链上内存。17 笔独立签名交易在本地固定区块上下文执行，不是公链部署/真实出块或统计样本。
每笔交易 cap=20M；未放宽历史预算，也未重启软件 Miller 路线。

## 实际命令与最小复核

以下 Rust 命令环境为 `CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none`：

```sh
cargo +1.99.1 fmt --all -- --check
cargo +1.99.1 clippy -p vv-friendly-crypto -p vv-friendly-dkg --all-targets --release --locked --offline -- -D warnings
cargo +1.99.1 test -p vv-friendly-crypto -p vv-friendly-dkg --lib --release --locked --offline
cargo +1.99.1 test -p vv-friendly-dkg --bin friendly-g6-native-winner-only --release --locked --offline
cargo +1.99.1 build -p vv-friendly-dkg --bin friendly-g8-receiver-export --release --locked --offline
target/release/friendly-g8-receiver-export
node contracts/scripts/g8_settlement.mjs
node scripts/g8_audit.mjs
```

45 项库 + 3 项 G6 接收/冻结回归通过；格式、严格 Clippy、原生 8+2 重放通过。
首轮 EVM 因测试工具 `runCall` 只读查询增加 caller nonce 而中止；改为 journal checkpoint/revert
隔离只读查询后重新执行，17 个实际交易 nonce 连续为 0..16。首轮失败日志仍保留。
原始输出不覆盖；可用已归档公开材料重跑到新目录（无需 Setup/Prove）：

```sh
node contracts/scripts/g8_settlement.mjs experiments/g8_receiver_replay_new
```

## 下一步：仍在 G8 内，只补认证与结果写入

当前没有 Winner 写入接口；伪造 Trustee 签名、篡改承诺、披露时替换冻结目标、正确 Winner
证明和重复 Winner 的验收均为 **NOT_IMPLEMENTED**，不是通过或跳过后算完成。
下一轮先解决原 DKG/注册及 Trustee **Jubjub Schnorr** 认证的一致验证，不能直接用新 secp256k1
签名冒充原签名；然后从同一冻结合约存储和已认证承诺重建 60 项 P3 输入，调用同一 G4-D VK，
只写 Winner 一次并执行相应必要负例。此闭环完成前不宣称完整 Winner-only 合约。

G9 精确 SnarkPack EVM 仍受阻；本轮真实逐票备用路线不是其完成。G10 的公平 Full Tally/
Winner-only、完整 Gas/通信量、独立统计、干净提交复现仍待后续。研究型确定性 Setup 不是
生产可信设置；多进程 Trustee 私密交付也仍未接入 G6。本轮到接收/冻结阶段停止。
