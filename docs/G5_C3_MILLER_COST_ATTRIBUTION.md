# G5-C3-B18：同程序失败调用的累计成本归因

2026-10-27。本轮只为 B16 的相同程序增加聚合观察器，实际执行一次必要部署和
一笔完整 64 步正例调用，交易预算仍为 20M。调用精确复现 B16 的根层 OOG，
累计费用对账差额为零。没有实施算术优化或提高预算。

状态：`PASS_SAME_PROGRAM_FAILED_ATTEMPT_COST_ATTRIBUTION_ONLY`。这里的 PASS
只表示失败路径复现和费用归因通过；完整 Miller 仍未完成，exact SnarkPack EVM
验证仍为 `BLOCKED_SOFTWARE_MILLER_SUB_LOWLEVEL_EXCEEDS_20M`。

## 相同程序与实际执行

合约仍为 B15/B16 的 `Bls12381MillerLoopSubLowLevel.sol`，源码没有变化。
solc 0.8.30、optimizer 200、Prague、无 viaIR，只额外请求 AST、deployed source
map 和 generated sources。实际部署 runtime 仍为 7,106B，SHA-256 与 B16 相同。
同一公开证明、fixture、calldata、签名发送者及交易序列均保留；B18 的部署交易
和 full64 交易哈希也分别与 B16 一致。

| 项目 | B18 实际结果 |
|---|---:|
| EVM 交易数 | 2：一次部署、一笔 full64 |
| 部署总 Gas | 1,589,758 |
| full64 总 Gas / 交易上限 | 20,000,000 / 20,000,000 |
| full64 execution / intrinsic | 19,963,388 / 36,612 |
| 退款 | 0 |
| receipt / 根异常 | 0 / out of gas |
| 成功 MODEXP / 失败 MODEXP | 10,628 / 0 |
| 退出活动内存 | 1,945 words |
| 根层已执行指令数 | 5,510,618 |
| 最后观察位置 | pc 5,374，SHR，剩余 2 Gas |
| 与 B16 复现差异 | 0 |

最后的 SHR 需要 3 Gas，因此没有实际执行。source map 将其映射到 `_mul` 内
`halfMod` 的 `shr(1, hi)`，源码第 355 行。这只是预算耗尽的落点；本轮另以
累计费用记录描述已执行路径，而不从最后一条指令推定瓶颈。

## 完整费用对账

```text
已收费 root opcode       17,837,786
成功 MODEXP child         2,125,600
终止未执行指令耗尽余量             2
---------------------------------
execution                19,963,388
intrinsic                    36,612
---------------------------------
交易总 Gas               20,000,000
对账差额                          0
```

opcode、Solidity function 和 source leaf 三个分区的总和分别均为 17,837,786。
它们是同一根层费用的三种替代分类，不能相加。MODEXP child 单独计算；root
STATICCALL 费用也不能与 child 的实际计算费用混淆。最后的 SHR 不计入已执行
opcode，终止耗尽余量只能计 2 Gas，不能按其要求的 3 Gas 入账。

动态 opcode 费用已经包含相应内存扩张费用，因此不再额外加上 B17 的内存公式
结果。全部执行费用对账成功不等于全部费用都有可靠的函数归属。

## 源码归因与覆盖率

运行器按 instruction index 解析压缩 source map，跳过 PUSH 的立即数字节。
每个 root opcode 选择包含其源码范围的最短 AST 函数区间；Solidity 函数为
function bucket，内联 Yul 函数可进一步形成 `parent::helper` leaf bucket。
原始 source ID 为 0，generated source ID 为 1。无映射、生成代码和合约框架
保留独立桶，不强行分摊给相邻函数。

| 互斥 function 桶 | 已收费 root Gas | 占已收费 root |
|---|---:|---:|
| `_mul` | 8,051,757 | 45.1388% |
| `<unmapped>` | 3,993,601 | 22.3884% |
| `_add2` | 2,155,414 | 12.0834% |
| `_mul2` | 1,502,129 | 8.4211% |
| `_sub2` | 939,062 | 5.2645% |
| `<compiler-generated:1>` | 2,756 | 0.0155% |
| `<contract-scaffolding>` | 2,751 | 0.0154% |

全部已映射函数共 13,838,678 Gas，动态费用覆盖率为 77.5807%。无映射、生成
代码和框架合计 3,999,108 Gas，即 22.4193%。覆盖率的分母是本次已收费 root
Gas，不是静态 PC 数，也不是包含 child 的整笔 execution Gas。

`_mul` 是当前最大已映射函数桶；这表示优化后的源码映射归属，不是动态调用栈
的 inclusive 成本。它不包含 MODEXP child，也不能直接解释为可消除的费用。
同样，`_pointDouble` 等函数自身桶较小，不表示其调用的全部运算便宜。

leaf 分区中，`_mul::square` 为 2,502,894 Gas（14.0314%），`_mul` 本体为
2,351,511（13.1828%），`_mul::subMod` 为 1,319,696（7.3983%），
`_mul::halfMod` 为 1,080,196（6.0557%），`_mul::addMod` 为 797,460
（4.4706%）。这些细项与其父函数是不同粒度的替代分区，不能再次叠加到函数
总额上。终止于 halfMod 不意味着它是该函数内最大的累计 leaf。

## opcode 排名

| root opcode | 已收费 Gas | 占已收费 root |
|---|---:|---:|
| JUMP | 1,573,656 | 8.8220% |
| PUSH1 | 1,366,119 | 7.6586% |
| ADD | 1,229,610 | 6.8933% |
| STATICCALL | 1,062,800 | 5.9581% |
| SWAP1 | 974,220 | 5.4616% |
| MLOAD | 969,570 | 5.4355% |
| MSTORE | 938,824 | 5.2631% |
| PUSH2 | 923,079 | 5.1749% |

完整排名见 raw。本次最多只能说明这条失败路径上存在大量指令、控制流和
调用开销，不能把所有 JUMP、PUSH、内存操作或无映射费用都归到某个算术函数。

## 路线选择与停止条件

运行器预先设定的候选筛选是：最大已映射函数占已收费 root 至少 20%，且无映射、
生成代码、框架费用合计低于 10%。这是选择下一项只读审查的工作启发式，不是
安全定理、归因误差上界或完整验证器的验收标准。

虽然 `_mul` 达到第一项条件，但本次未归入函数的费用为 22.4193%，未满足第二项。
因此 raw 明确记录：

```text
candidate_for_read_only_review = null
optimization_implemented = false
next_gate = B19: pause arithmetic changes; read-only route decision
```

本轮止于排名和路线选择，暂停新的算术修改。B19 只读取现有归因和实现，说明
在既定预算下继续软件路线需要什么具体依据，并形成单一、可审查的路线决定；
不自动指定 `_xi`、`halfMod` 或其他原语作为优化项，不追加 VM 调用、提高 cap、
实施 wrapper 或更换 Suite。wrapper 仍需要独立定义完整 verifier 关系和资源计划。

20M 是当前组件实验的交易预算，30M 是本地 block 预算；本次没有证明目标链
永久硬上限，也没有测得提高预算后即可完成。10,628 次 MODEXP、20M 失败支出和
上述占比均不能换算完整 64 步的完成率或总成本。

## 产物、复现与来源

```bash
npm run --prefix contracts g5c3b18
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

以上为复现指令；当前 raw 已存在时，运行器会拒绝覆盖。本轮归档直接核对已保存
结果，没有再次运行 EVM。

- 观察器：`contracts/scripts/g5c3_miller_attribution.mjs`。
- 实际记录：`experiments/g5_c3_raw/miller_cost_attribution.json`。
- B16 来源：`experiments/g5_c3_raw/miller_sub_full_evm.json`。
- B17 只读复核：`experiments/g5_c3_raw/miller_budget_review.json`。
- 合约：`contracts/src/Bls12381MillerLoopSubLowLevel.sol`。

环境为 Apple M4 / arm64、Node 23.11.0、EthereumJS / Prague，Git 基准
`e80aa2b9fb8770653d7eb23351d27fc89b0d466f`，`git_dirty=true`。
观察器记录的 wall time 包含逐步观察开销，不作无观察器性能或统计样本。
本轮没有保存逐步大 trace，没有前缀、边界或负例重跑，没有 setup/prove、最终幂
或规模实验。源码、来源 raw、fixture、锁文件、两个 Manifest 和解释器实现的
13 项哈希已逐项核对。

| 产物或来源 | SHA-256 |
|---|---|
| B18 观察器 | `25a4da7619346a46b4aa41a024d728c44264a4146eb9427aea9ada8483b00ac9` |
| B18 raw | `6645a9dbdc8c363944723edc89b09a947e5533b95d3a507076b8086c2fb9ec37` |
| B16 raw | `1bace132207dd13d4fa492283ccb0d37bd6d65d53e02cc15e0044f39cde941e9` |
| B17 派生记录 | `027f1587054d515b4ae70b499921a4757b824cb28ad684e896bde64d32758c42` |
| 合约源码 | `8f13f1c53c5cf3ddc8f04a8259b3944d0a9fc642897f04a3cd8e5659c596eb06` |
| deployed runtime | `69912b7444e21d7f8f562795063895da8243f1be400bf3333b2b925b0db58e92` |
| deployed source map | `60b66307f9fbce831f5df2c671d46d8d96bc690d06789d17aa16595ac49a1b39` |

完整 Miller、最终幂、GT 绑定和 exact SnarkPack EVM 验证仍未通过。
`full_miller_verification_gas`、`estimated_full_gas`、`completion_percent` 和
`snarkpack_verification_gas` 均为 `null`；协议能力及 Manifest 没有升级。

## 后续记录：B19 已完成源内路线复核

2026-10-27，B19 只读取本页归因、B6 隔离乘法基线和现有源码，并执行 Node
代数核对；EVM 交易数为零，未修改 Solidity。B18 的 10% 自动筛选启发式仍保留
为当时的历史决策，它不禁止针对明确数学恒等式继续只读审查。

B19 另给出保守排名界：`_mul` 的 8,051,757 Gas 仍高于下一已映射函数
`_add2` 的 2,155,414 加全部未归入函数的 3,999,108，即高出 1,897,235 Gas。
这只保证失败路径的互斥语法归属排名，不代表可消除的费用或动态调用栈成本。

唯一待测候选是在 `_mul` 中把两次 `halfMod` 合为固定模数 `quarterMod`：
`k=t&3; quarter(t)=(t+k*p)/4`。分析核对了 p 的低两位、kp 乘积进位及随后
相加进位，两项进位可能同时为 1，不能用 OR 合并。`halfMod` 自身仅占本次
root 的 6.0557%，因此没有承诺该候选可解决 20M 限额或取得实际 Gas 节省。

下一门 B20 仅计划一次独立 Fp `_mul` 对照：一个 B6 原公开性能样本和四个
校正/进位正确性值，含部署共六笔未来交易。本轮执行数仍为零，最新 actual
full 仍是 B18 OOG；不自动进行 Miller 集成或 full 重试。
详见 [B19 源内路线复核](G5_C3_MILLER_ROUTE_REVIEW.md)。

派生记录：`experiments/g5_c3_raw/miller_route_review.json`，SHA-256
`0f95968de0efdb6f41af221e113b3545f757d48e6a9a284e2b980bc4c113061b`。
