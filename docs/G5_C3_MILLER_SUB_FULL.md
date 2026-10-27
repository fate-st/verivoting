# G5-C3-B16：最新版软件 Miller full64 在 20M 上限下 OOG

2026-10-26。本轮保持 B15 合约、fixture、编译设置和 20M 交易 Gas 上限完全
不变，只执行一次部署和一笔 full64 正例交易。交易实际以 root out-of-gas 失败，
没有返回完整 Fp12 结果。

状态：`BLOCKED_SOFTWARE_MILLER_SUB_LOWLEVEL_EXCEEDS_20M`。这是最新版软件
Miller 路径在固定预算下的实际失败证据，不是完整循环 Gas 测量，也不是密码学
不可能性结论。

## 唯一 full64 尝试

| 项目 | B16 实测 |
|---|---:|
| 交易 Gas 上限 | 20,000,000 |
| 交易总 Gas | 20,000,000 |
| execution Gas | 19,963,388 |
| intrinsic Gas | 36,612 |
| receipt status | 0 |
| 异常 | root `out of gas` |
| 成功 MODEXP 次数 | 10,628 |
| 失败 MODEXP 次数 | 0 |
| MODEXP execution Gas | 2,125,600 |
| OOG 时活动 EVM 内存 | 1,945 words |
| 返回值 | 空 |
| 完整循环验证 Gas | `null` |

费用恰好满足 `19,963,388 + 36,612 = 20,000,000`，退款为零。最后记录的
root opcode 为 pc 5,374 的 `SHR`，执行前仅余 2 Gas，活动内存为 1,945 words。
这说明该笔固定上限交易耗尽预算；它不定位唯一根因，也不能据此推断提高多少
Gas 才能完成。

10,628 次 MODEXP 全部成功且没有 precompile 失败，但合约在产生返回值前 OOG，
所以没有完整 768B 输出可与参考值比较，`full_miller_loop_completed=false`、
`full_miller_loop_verified_in_evm=false`、`full_return_matches_reference=false`。

完整公式的预期 MODEXP 次数是 14,316。`10,628/14,316` 不能写成完成率：调用
分布、顶层算术、点更新和内存成本并非均匀，且交易没有完成。20M 是失败交易的
封顶支出，不是完成 full64 所需的实际 Gas。

## 与先前证据的关系

B16 使用与 B15 完全相同的合约源码和 runtime；没有新增优化或算术变化。
B15 的两步前缀正例和错误目标没有在本轮重跑，raw 中的
`prefix_correctness_verified_in_evm=true` 与错误目标通过状态来自保存的 B15
基准，`prefix_checks_executed_this_run=false`。

此前最新实际完整尝试是 B7 旧版本，在 20M 上限下于 7,848 次成功 MODEXP、
3,147 个活动内存 words 后 OOG。B16 将最新版三项低分配修改带入 full64，实际
达到 10,628 次成功 MODEXP、1,945 words 后仍 OOG。该对照仅说明执行轨迹发生
变化；两个调用数都不是完成比例，也不能用它们线性估计剩余 Gas。

B16 现在是最新实际完整尝试，B7 只保留为历史基准。最新版前缀仍是 B15 的
985,557 Gas 通过结果，但前缀结果不能线性外推 full64。

## 本次记录的只读归因

在内存中使用相同 solc 0.8.30、optimizer 200、Prague、无 viaIR 设置重编译，
只额外请求 source map 输出。所得 runtime 仍为 7,106B，SHA-256 仍为
`69912b7444e21d7f8f562795063895da8243f1be400bf3333b2b925b0db58e92`，
与 B16 实际部署记录一致。source map 将最后的 pc 5,374 `SHR` 映射到源码
第 355 行，即 `_mul` 局部 `halfMod` 中的 `rh := shr(1, hi)`。

`SHR` 的基础费用为 3 Gas，而该步前只余 2 Gas，因此异常在此落地。但这是累计
预算耗尽的最后落点，不证明 `halfMod` 是热点，也不能把最后一条指令当成根因。
本次 source-map 对照只定位最后 opcode，没有提供函数级 Gas 分摊。

raw 中 MODEXP 子调用执行费用为 2,125,600 Gas，占 19,963,388 execution Gas
的约 10.65%。相减后的 17,837,788 Gas 位于根层 EVM 执行及调用等路径中，
但不能全部笼统称为“算术费用”；当前记录没有按函数拆分这部分成本。

按 1,945 个活动内存 words 计算，EVM 累计内存扩张公式为
`3*1945 + floor(1945^2/512) = 13,223 Gas`，约占 execution 的 0.066%。因此
现有证据不支持把 EVM 内存扩张认定为这次 OOG 的主因。活动 memory words 也
不是主机 RSS 或所有内存相关 opcode 的总成本。

以上仅是相同字节码的只读编译/source-map 映射与保存 raw 的算术核算，没有重跑
EVM，也不等于函数级 profiling。它足以排除两种过早结论：不能因最后落在
`halfMod` 就称其为热点，也不能因内存达到 1,945 words 就称内存膨胀为主因。

## 实验范围与未完成项

本轮只有一次部署和一笔 full64 正例：

- 合约源码、runtime、曲线、编解码、Suite、SRS 和 Manifest 均未改变；
- 没有重跑前缀 pair、错误目标、B14 边界或其他算术测试；
- 没有 RPC、setup/prove、规模实验或负例矩阵；
- 没有最终幂、非单位 GT 目标绑定或 exact SnarkPack EVM 验证。

因此 `full_miller_verification_gas` 与 `snarkpack_verification_gas` 继续为 `null`，
`evm_final_exponentiation_implemented=false`、`pairing_to_gt_evm_verified=false`、
`exact_snarkpack_evm_verified=false`。完整 G5-C3 和目标链结算仍未完成。

## 产物、来源与复现

```bash
npm run --prefix contracts g5c3b16
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

- 合约：`contracts/src/Bls12381MillerLoopSubLowLevel.sol`，与 B15 相同。
- 运行器：`contracts/scripts/g5c3_miller_sub_full.mjs`。
- 本次结果：`experiments/g5_c3_raw/miller_sub_full_evm.json`。
- 最新前缀基准：`experiments/g5_c3_raw/miller_sub_prefix_evm.json`。
- 先前完整尝试：`experiments/g5_c3_raw/miller_lowlevel_evm.json`。

环境：Apple M4 / arm64，Node 23.11.0，EthereumJS 10.1.3 / Prague，
solc 0.8.30、optimizer 200、无 viaIR；本地签名交易，无 RPC。
Git 基准 `e80aa2b9fb8770653d7eb23351d27fc89b0d466f`，`git_dirty=true`。
单次本地 wall-clock 记录不是统计性能结论。

| 产物或来源 | SHA-256 |
|---|---|
| B15/B16 合约 | `8f13f1c53c5cf3ddc8f04a8259b3944d0a9fc642897f04a3cd8e5659c596eb06` |
| B16 运行器 | `0a83fa3aa7b6b6eb381d6639eb85620f829da9502096ddd894018f441068fd6f` |
| B16 原始结果 | `1bace132207dd13d4fa492283ccb0d37bd6d65d53e02cc15e0044f39cde941e9` |
| B15 前缀结果 | `702ca228900fbfb5decd02ad383205b08ddadeeec8b2ebd04eb6cbd4f205ad37` |
| B14 原语结果 | `008d45bff28202b1ede28901a8c65a9ebec69649b9aaed2e65609d07557a023e` |
| B7 历史完整尝试 | `cdf06a719f4df41a098ce1a2ce7a4b5e4a8e2e4eb61479b044aa717da9caa7e6` |
| Miller 冻结向量 | `1079b6fd309f7d11d60fcd2cd5f15a1e4b25895fe7f876bfe3be624d4f989ead` |
| B=2 公开证明向量 | `34180b26bcb316b2bf5d2a6ce810a6348a42cb2d5bcd3916a2f9a2a8dbcfaeb2` |
| 聚合证明字节 | `12e25c7edf07eff2d3f211a3cbb74ae4ce43e028ab58702debdef98b0787e64c` |

部署仍为 1,589,758 Gas、runtime 7,106B，与 B15 一致。旧源码、旧 raw、fixture
和锁文件均保留。

## 下一步只做只读归因与路线决策

先分析这一次保存的 OOG 轨迹、剩余顶层算术和软件路线的预算可行性，再决定一个
有明确验收条件的后续门。当前不自动重试 full64、不提高 Gas 上限、不继续无界
逐原语 profiling，也不提前实现最终幂。

20M 是本研究为该次交易固定的实验预算，不应冒充目标链的永久硬限制。路线评估
可以讨论预算与实现选择，但不会据此自动提高 cap 或更换协议 Suite。

后续若选择新实验，必须以这份 B16 失败证据为基线；不得把 MODEXP 调用数当作
完成率，也不得把 20M 封顶支出报告为完整循环 Gas。

## 后续记录：B17 只读预算复核已完成

B17 只读取本页 B16 raw、B15 前缀与 B7 历史记录，并以相同编译设置额外请求
source map；实际 EVM 交易数为零。派生记录复核了最后 pc 到源码的映射、MODEXP
child 10.6475%、未归因根层 17,837,788 Gas，以及内存扩张 schedule cost
13,223 Gas（execution 的 0.06624%）。详见
[G5_C3_MILLER_BUDGET_REVIEW.md](G5_C3_MILLER_BUDGET_REVIEW.md)。

该 PASS 仅表示只读派生分析成功，不是新的 Miller/EVM/性能 PASS。B16 仍是最新
实际 full 尝试，完整验证 Gas 仍为 `null`。20M 仍是实验预算，不是已证明的链
硬限制。

预算复核选择 B18 作为下一门：同 runtime、calldata、compiler 和 20M 上限，
只执行一次部署与一笔 full64，观察器仅保存累计互斥成本桶，不修改算术。B18
本轮未执行；不自动重试、增 cap、换 Suite、继续逐原语 profiling 或进入最终幂。
