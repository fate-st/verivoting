# G5-C3-B17：软件 Miller 预算与路线只读复核

2026-10-26。本轮只读取 B16、B15、B7 的已保存证据，并用相同编译设置额外请求
deployed source map。分析器没有创建 VM、部署合约或执行交易，也没有生成新的
性能测量。

状态：`PASS_DERIVED_MILLER_BUDGET_REVIEW_ONLY`。这里的 PASS 只表示派生数据、
源码映射和路线决策记录成功，不能写成新的 EVM、Miller 或性能 PASS。B16 仍是
最新实际 full 尝试，目标验证器仍为
`BLOCKED_SOFTWARE_MILLER_SUB_LOWLEVEL_EXCEEDS_20M`。

## 分析范围与来源

| 项目 | B17 记录 |
|---|---|
| EVM 交易数 | 0 |
| 新 benchmark | 否 |
| Miller/full 尝试数 | 0 |
| 证据来源 | B16 full64 OOG raw |
| 最新前缀 | B15 |
| 先前 full 历史 | B7 |
| 软件/runtime 变化 | 无 |
| 能力或 Manifest 变化 | 无 |

分析器核对 B16/B15/B7 raw、合约、fixture、锁文件和两个 Manifest 的哈希。
solc 仍为 0.8.30、optimizer 200、Prague、无 viaIR，只在 output selection 中
增加 `evm.deployedBytecode.sourceMap`。重编译 runtime 仍为 7,106B，SHA-256
仍为 `69912b7444e21d7f8f562795063895da8243f1be400bf3333b2b925b0db58e92`，
证明 source-map 请求没有改变被分析程序。

## 从旧失败记录派生的结果

source map 将 B16 最后观察到的 root pc 5,374 `SHR` 映射到
`Bls12381MillerLoopSubLowLevel.sol` 第 355 行 `_mul` 局部 `halfMod` 的
`shr(1, hi)`。该步前只余 2 Gas，低于 `SHR` 的 3 Gas 基础费用；这是累计预算
耗尽的终止落点，不是 `halfMod` 累计热点证据。

| 派生项目 | 数值 |
|---|---:|
| B16 observed execution Gas | 19,963,388 |
| 成功 MODEXP child Gas | 2,125,600 |
| MODEXP child 占 execution | 10.6475% |
| 未归因到成功 MODEXP child 的 execution | 17,837,788 |
| 1,945 words 的内存扩张 schedule cost | 13,223 Gas |
| 内存扩张 schedule cost 占 execution | 0.06624% |

这些数字全部来自失败的 B16 封顶交易。17,837,788 Gas 只是
`execution - successful MODEXP children`，包含根层 opcode、调用和其他路径，
不能全部称为算术。13,223 Gas 仅是 EVM 内存扩张公式，不包含全部内存相关
opcode，也不是主机 RSS。当前证据既不支持把最后的 `halfMod` 认定为热点，也
不支持把内存扩张认定为 OOG 主因。

MODEXP 次数、20M 封顶支出和最后 pc 都不能估计完整循环完成比例或完成 Gas。
本轮没有函数级累计 Gas 排名，因此暂停新的算术修改，而不是从终止落点猜测优化。

## 与论文主张边界的一致性

论文当前第 1231 行附近明确指出：SnarkPack 参考实现使用 BLS12--381 目标群，
但不提供 Solidity 验证器或 Ethereum Gas；部署仍需要兼容链能力、专用 precompile
或新的 succinct-wrapper 路线。第 1234 行开始的 batch evaluation 仍将完整比较
标为未测量。第 2433 行附近也把 SnarkPack EVM 验证、完整 DA、状态写入和
public-chain finality 排除在现有受限可行性证据之外。

B16/B17 为“受限软件路线可行性与预算失败”补充了真实证据，但没有完成完整
GT、内部 transcript/target 绑定或结算关系。succinct wrapper 不是免费替换：它
需要新的验证关系、完整 GT/transcript 绑定和单独资源计划。因此本轮暂缓 wrapper，
不更换 Suite，也不靠提高实验 cap 改写现有结论。

20M 是这组实验选择的交易预算，B17 没有证明它是任何目标链的永久硬限制。
`chain_hard_limit_established_by_this_analysis=false`，同时
`automatic_budget_increase=false`。

## 路线决策

当前软件修改状态为 `HOLD_PENDING_CUMULATIVE_COST_ATTRIBUTION`。B16 已证明
最新版在固定预算下 OOG，但只保存终止位置与粗粒度 child/root 分解，没有根层
opcode/source 的累计互斥排名。因此下一门只做 B18，一次相同程序的累计成本归因；
不再先验选择新的算术函数。

B18 的固定范围是：

1. 使用与 B16 相同的 runtime、calldata、compiler 设置和 20M 上限；只执行一次
   部署和一笔 full64，不改变协议或算术实现。
2. 仅改变观察器，保存聚合 counter，不保存逐步巨量 trace。根层已收费 opcode、
   source 桶、MODEXP child 和终止 burn 必须采用互斥口径。
3. 对账要求为：所有 root 已收费桶 + child MODEXP + terminal burn 2 Gas
   `= 19,963,388`。unmapped/generated 代码必须保留独立桶，不能强行归给邻近函数。
4. 最后一条 `SHR` 需要 3 Gas、当时只余 2 Gas，因此没有实际执行；终止 burn
   只能记 2 Gas，不能记成 3 Gas 的已执行 opcode。
5. B18 只回答累计成本归因，不引入新优化、不提高 cap、不换 Suite，也不进入
   最终幂。完成归因后才决定是否还有单一、可验收的软件改动。

B18 本轮未执行。B17 不授权自动 full 重试、无界逐原语 profiling 或从 component
结果推算 complete verifier Gas。

## 产物与复现

```bash
npm run --prefix contracts g5c3b17
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

- 分析器：`contracts/scripts/g5c3_miller_budget.mjs`。
- 派生记录：`experiments/g5_c3_raw/miller_budget_review.json`。
- 实际失败证据：`experiments/g5_c3_raw/miller_sub_full_evm.json`。
- 最新前缀证据：`experiments/g5_c3_raw/miller_sub_prefix_evm.json`。
- 先前 full 历史：`experiments/g5_c3_raw/miller_lowlevel_evm.json`。

Git 基准 `e80aa2b9fb8770653d7eb23351d27fc89b0d466f`。本轮是只读派生分析，
不是 clean/dirty 状态下的新性能运行。

| 产物或来源 | SHA-256 |
|---|---|
| B17 分析器 | `227fba03ca45aeaf40c4ac31771dca276013c5eddf6b066c1bdb5993312aaa0e` |
| B17 派生记录 | `027f1587054d515b4ae70b499921a4757b824cb28ad684e896bde64d32758c42` |
| B16 实际 full 证据 | `1bace132207dd13d4fa492283ccb0d37bd6d65d53e02cc15e0044f39cde941e9` |
| B15 前缀证据 | `702ca228900fbfb5decd02ad383205b08ddadeeec8b2ebd04eb6cbd4f205ad37` |
| B7 历史 full 证据 | `cdf06a719f4df41a098ce1a2ce7a4b5e4a8e2e4eb61479b044aa717da9caa7e6` |
| 合约源码 | `8f13f1c53c5cf3ddc8f04a8259b3944d0a9fc642897f04a3cd8e5659c596eb06` |
| deployed runtime | `69912b7444e21d7f8f562795063895da8243f1be400bf3333b2b925b0db58e92` |
| deployed source map | `60b66307f9fbce831f5df2c671d46d8d96bc690d06789d17aa16595ac49a1b39` |

`full_miller_verification_gas`、`estimated_full_gas`、`completion_percent` 和
`snarkpack_verification_gas` 均继续为 `null`。

## 后续记录：B18 累计归因已执行

2026-10-27，B18 已在相同 B16 runtime、calldata、编译配置及 20M 预算下执行
一次必要部署和一笔 full64。失败调用精确复现 B16：10,628 次 MODEXP 全成功，
1,945 words，根层 pc 5,374 的 SHR 前仅余 2 Gas，最终仍为 OOG。

实际费用对账为 `17,837,786 root + 2,125,600 child + 2 terminal burn
= 19,963,388 execution`，差额为零；再加 36,612 intrinsic 即 20M。
这里将 B17 的 17,837,788 未细分余量拆为 root 已收费指令和 2 Gas 终止耗尽，
没有将未执行 SHR 的 3 Gas 要求计作已收费指令。

最大已映射函数桶 `_mul` 为 8,051,757 Gas（45.1388% root）；全部函数费用
覆盖率为 77.5807%，无映射、生成代码和框架合计仍有 3,999,108 Gas（22.4193%）。
归属是互斥源码映射，不是动态调用栈成本。未归入函数的费用超过候选筛选启发式
的 10% 上限，因此 `candidate_for_read_only_review=null`，暂停算术修改。

最新实际 full 记录由 B16 更新为 B18；本页前文保留 B17 当时的只读分析背景。
归因 PASS 不表示完整 Miller 或 SnarkPack 通过，相关完成标记仍为 false、完整
验证 Gas 仍为 null。下一门 B19 只做路线决策，不追加 VM、优化、增 cap 或 wrapper
实施。详见 [B18 累计归因报告](G5_C3_MILLER_COST_ATTRIBUTION.md)。

B18 raw：`experiments/g5_c3_raw/miller_cost_attribution.json`，SHA-256
`6645a9dbdc8c363944723edc89b09a947e5533b95d3a507076b8086c2fb9ec37`。
