# G5-C3-B21：除四乘法集成的 Miller 前缀实测

2026-10-27。本轮将 B20 已测完整 `_mul` 精确集成到独立 B15-based Miller
合约，只执行一次部署和原两步前缀正例、一个错误目标。完整 768B 前缀输出正确，
错误目标被拒绝；正例总 Gas 从 985,557 降至 951,873。

状态：`PASS_QUARTER_MILLER_PREFIX_INTEGRATION_ONLY`。这是固定输入的前缀
集成通过。新 quarter 版本尚未运行完整 64 步，目标状态为
`BLOCKED_QUARTER_MILLER_FULL_VARIANT_UNTESTED`。

## 唯一实现变化

新增 `contracts/src/Bls12381MillerLoopQuarter.sol`。整份源码严格等于 B15 的
合约名替换，加上将其 `_mul` 完整替换为 B20 profile 的同一函数。其余 51 个
Solidity 函数体及其他源码字节保持不变，包括 `_add2`、`_sub2`、`_mul2`、
点倍加、NAF、一般 Fp12 平方、稀疏线乘和 scratch arena。

固定模数除四的规范性及两个独立进位见
[B19 推导](G5_C3_MILLER_ROUTE_REVIEW.md)，隔离实测见
[B20 报告](G5_C3_FP_MUL_QUARTER.md)。本轮没有重跑原语边界或 Fp profile，
没有向合约提供可信中间状态；全部点、线和域运算仍由合约计算。

## 同一前缀的真实对照

使用同一公开 B=2 证明、同一冻结 Miller fixture、相同编译配置和原正反例
calldata。两步覆盖前缀点更新、第一次负号 mixed addition、平方、三条线乘
及 arena 复用；完整结果逐字匹配冻结参考值。

| 项目 | B15 原版本 | B21 quarter 版本 |
|---|---:|---:|
| 正例交易总 Gas | 985,557 | 951,873 |
| 正例 execution Gas | 948,865 | 915,181 |
| 正例 intrinsic Gas | 36,692 | 36,692 |
| 退出活动 EVM 内存 | 1,905 words | 1,905 words |
| MODEXP 成功次数 | 516 | 516 |
| MODEXP child Gas | 103,200 | 103,200 |
| 错误目标总 Gas | 985,375 | 951,691 |
| runtime | 7,106B | 7,078B |
| 部署总 Gas | 1,589,758 | 1,583,679 |

正例实际减少 33,684 Gas，即 3.41776%；活动内存没有下降。隔离 Fp 的
6.6451% 窗口收益与本次前缀收益不是同一指标，不相互替代，也不能用于线性
估计完整 64 步 Gas。

错误目标仅沿用原先的规范预期值改动，结果为根层 `revert`、原因
`INVALID_MILLER_RESULT`。其 execution 为 914,987、intrinsic 为 36,704，
合计 951,691 Gas。它检查比较结果正确拒绝，不是额外构造一份 SnarkPack proof。

两笔调用均为零退款，516 次 MODEXP 全部成功。MODEXP trace SHA-256 和各自
calldata SHA-256 都与 B15 对应调用一致。正例 `915,181 + 36,692 = 951,873`，
负例 `914,987 + 36,704 = 951,691`，均高于各自 calldata floor，无最低费用
补差。部署 `1,417,077 + 166,602 = 1,583,679`。

runtime 减少 28B、部署总费用减少 6,079 Gas；这些是本轮实际部署记录。
活动 EVM 内存不是主机 RSS，也不能与隔离原语的分配指针增长混为一谈。
本轮只有一组固定正反例，没有统计时间、规模或端到端性能结论。

## 新版本完整循环未测试，历史失败另行保留

本轮仅三笔交易：一次部署和两笔前缀调用。raw 明确记录：

```text
full_miller_loop_run = false
full_loop_attempts = 0
full_attempt = null
new_full_variant_tested = false
full_miller_loop_verified_in_evm = false
full_miller_verification_gas = null
estimated_full_gas = null
completion_percent = null
```

最新实际完整尝试仍是 B18 的归因运行，但它使用旧 B15-derived 合约，20M 下
OOG。它与 B21 的 quarter runtime 不同；不能将该历史失败写成 B21 已测 OOG，
也不能因前缀通过就将 B21 记为完整循环可行。

| 记录 | 已执行范围 | 正确状态 |
|---|---|---|
| B21 当前 quarter 版本 | 原两步前缀正/错对 | PREFIX PASS；full 未测试 |
| B18 历史旧版本 | 一笔 20M full64 失败调用的归因 | 历史 OOG |
| exact SnarkPack EVM verifier | 尚未完成 | BLOCKED，Gas 为 null |

因此当前目标状态改为 `BLOCKED_QUARTER_MILLER_FULL_VARIANT_UNTESTED`，由
B21 raw 提供来源；历史状态 `BLOCKED_SOFTWARE_MILLER_SUB_LOWLEVEL_EXCEEDS_20M`
继续作为不同版本的已测失败证据保留。最终幂、精确 GT 目标绑定和完整 SnarkPack
transcript/KZG 验证仍未完成，Manifest 不升级能力。

## 下一门 B22：只读停止或继续决策

B22 只读取本次前缀和已有失败/归因证据，判断继续投入该软件路线是否有充分
依据，或应保留当前缺口并暂停。它不自动运行 full64、最终幂或新的原语优化。
3.41776% 是本次真实前缀降幅，不构成追加完整尝试的预算证明。

本轮没有变更 20M 交易 / 30M block 上限，没有改变曲线、Suite、SRS、Manifest
或论文能力主张，也没有 setup/prove、规模实验或 RPC 调用。

## 产物、复现与来源

```bash
npm run --prefix contracts g5c3b21
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

原始结果已保存；本页归档直接核对，不再次执行 EVM。

- 合约：`contracts/src/Bls12381MillerLoopQuarter.sol`。
- 运行器：`contracts/scripts/g5c3_miller_quarter_prefix.mjs`。
- 实测记录：`experiments/g5_c3_raw/miller_quarter_prefix_evm.json`。
- 前缀基线：`experiments/g5_c3_raw/miller_sub_prefix_evm.json`。
- 原语来源：`experiments/g5_c3_raw/fp_mul_quarter_profile.json`。
- 不同版本历史 full：`experiments/g5_c3_raw/miller_cost_attribution.json`。

环境为 Apple M4 / arm64、Node 23.11.0、solc 0.8.30、optimizer 200、无 viaIR、
本地 EthereumJS / Prague。执行时 Git 基准为
`6bad1b0384ef18f525b85c86a9342447eda94b96`，`git_dirty=true`；不是 clean-commit
统计样本。手动 arena 依赖固定编译配置，不声称可任意升级编译器。

归档时只读复核了十八项来源/源码/锁文件/Manifest 哈希、整源变化范围、51 个
保留函数、全部 768B 返回值、错误原因、费用以及 MODEXP/calldata 摘要。

| 产物或来源 | SHA-256 |
|---|---|
| B21 合约 | `90e573ae450b9a75c0dbe4a21242cc07113ccb041f56c12370e43a194725958e` |
| B21 运行器 | `b0c07159dc868d6249ff28d66d72dd5f21240ae9e0c9891560d21265ce209c46` |
| B21 raw | `afab1dbf4d32d5ed1a0066e049512fde22839ce612e02deec7f9398c6474eb01` |
| B21 runtime | `97d85e04447a8ff8cbc46aca4d6e2eae510d04e7c96c34cdd80ffa1b1d0ec83c` |
| B15 前缀 raw | `702ca228900fbfb5decd02ad383205b08ddadeeec8b2ebd04eb6cbd4f205ad37` |
| B20 隔离 raw | `0d44c5b76b33816cf37f85aaacd52ff7170afc09cd5e92a987b10c198b5cb52c` |
| B18 不同版本 full raw | `6645a9dbdc8c363944723edc89b09a947e5533b95d3a507076b8086c2fb9ec37` |

## 后续记录：B22 已作一次受限 full 验收的只读决定

2026-10-27，B22 只核对现有证据与源码，EVM 交易数为零，未重编译或修改
runtime。决定未来 B23 仅使用本页保存的 B21 runtime、编译配置、冻结 full
calldata 及同一 20M 上限，执行一次必要部署和一笔 full64 正例。

理由是新版本只有两步证据，B18 的不同旧 runtime 不能替代其后续路径和最终
conjugation 验收；没有用本页 3.41776% 前缀降幅预测 full Gas 或成功概率。
这次计划服务于软件路线的证据收口，也不是当前论文收窄主张的强制实验。

预先停止规则为：前置/部署失败记 ABORT；纯根层 OOG、child/revert/返回值/
计数/费用等其他失败分别保留准确原因；任何失败后冻结该固定预算软件路线的
自动优化和 full 重试，只读诊断。成功也只表示完整 Miller 固定向量验收，
不自动进入最终幂或升级完整 verifier 能力。

B22 未执行 B23；latest actual prefix 仍为 B21、full 仍为 B18，目标仍为
`BLOCKED_QUARTER_MILLER_FULL_VARIANT_UNTESTED`，完整费用、预测及完成率均为
null。详见 [B22 完整验收决定](G5_C3_MILLER_FULL_DECISION.md)。

派生记录：`experiments/g5_c3_raw/miller_full_decision.json`，SHA-256
`b862ff2296763dee4d68aa848f04adebdeb77dae32191389c273d5d629412b48`。
