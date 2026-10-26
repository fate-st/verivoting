# G5-C3-B5：Fp/Fp2 最小成本定位完成

本轮只实际执行**一次Fp乘法、一次Fp2乘法**，两项结果均正确；没有重跑Miller循环。
状态为 `PASS_ISOLATED_FP_FP2_COST_PROFILE_ONLY`，只表示成本测量完成，不表示完整验证通过。

## 实际测量

复用原B=2公开证明的 `final_b` 仿射坐标：Fp取`x.c0`与`y.c0`，Fp2取完整`x`与`y`。
底层15个辅助函数与B4逐字相同；只新增外层成本测量，不修改数学公式、旧向量或旧基准。
合约自行计算乘积并返回，运行器将完整结果与直接BigInt模运算、Noble参考结果比较。

| 项目 | Fp乘法 | Fp2乘法 |
|---|---:|---:|
| 交易总Gas | 29,813 | 45,399 |
| intrinsic Gas | 23,288 | 24,952 |
| 合约execution Gas | 6,525 | 20,447 |
| 算术测量窗口Gas | 4,372 | 16,468 |
| MODEXP次数 | 2 | 6 |
| MODEXP子调用execution Gas | 400 | 1,200 |
| MODEXP占测量窗口 | 9.15% | 7.29% |
| 窗口内存扩张Gas | 211 | 899 |
| 窗口STATICCALL opcode Gas | 200 | 600 |
| 窗口分配指针增长 | 2,112B | 7,872B |
| 退出时活动EVM内存 | 93words | 305words |

部署总Gas为565,535，runtime2,369B；两笔交易均成功，所有MODEXP均成功返回64B。
未触发calldata floor。每项只使用一个固定公开输入，不是统计性能或端到端投票效率样本。

## 怎样解释这些数字

`gasleft()`窗口位于规范输入解析之后、输出编码之前，记录的是**带测量代码的编译算术窗口**。
它包含内部调用/控制流、临时对象、打包、STATICCALL设置及测量本身的固定开销，
不是任意调用位置下不变的“Fp乘法Gas常数”。参考
[Solidity0.8.30的gasleft说明](https://docs.soliditylang.org/en/v0.8.30/units-and-global-variables.html#block-and-transaction-properties)。

MODEXP使用现有64B基础域输入、指数2，本次每次实测200execution Gas；定价背景见
[EIP-2565](https://eips.ethereum.org/EIPS/eip-2565)。两个占比只针对本轮独立窗口，
不能推成整个Miller的开销占比，也不能直接乘次数推算完整循环Gas。

本轮对账包括：

- Fp：顶层opcode6,125 + 子调用400 = execution6,525；加intrinsic23,288 = 总Gas29,813。
- Fp2：顶层opcode19,247 + 子调用1,200 = execution20,447；加intrinsic24,952 = 总Gas45,399。
- 窗口首尾GAS标记差值与合约返回值一致；标记内存字数按固定Prague内存公式得到211/899。

STATICCALL opcode费用可能已经包含其内存收费，因此表中两行不能简单相加归因。
窗口扣除MODEXP后的3,972/15,268Gas是**尚未完全拆分的其余开销**，不能全部称为某一类
Solidity算术或内存扩张成本。分配指针增长也不等于实际收费内存或主机RSS。
部署的execution还含代码存储成本，不使用上述纯计算调用的opcode对账公式。

数据支持下一步尝试降低底层乘法的对象/打包/调用开销，而不是继续只减少MODEXP次数；
这是有测量依据的工程方向，不是已经证明了唯一瓶颈或优化后必能满足20M预算。

## 产物与复现

- 合约：`contracts/src/Bls12381FieldCostProfile.sol`。
- 运行器：`contracts/scripts/g5c3_field_profile.mjs`。
- 原始结果：`experiments/g5_c3_raw/field_cost_profile.json`。

```bash
npm run --prefix contracts g5c3b5
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

原始结果保存公开输入/完整输出、边界GAS标记、逐次MODEXP、内存、源码及基准哈希。
Solidity编译、Node语法和diff检查通过。独立审查确认费用对账和范围说明无实质问题。
本轮没有新增负例矩阵：底层算法未修改，本次任务是成本定位而不是新的密码学实现验收。

环境为Apple M4、Node23.11.0、EthereumJS10.1.3 / Prague、solc0.8.30、optimizer200、无viaIR。
本地签名交易，无RPC。Git基准`7442414a04e8d06d79d968b3cbf448a6a694347a`，`git_dirty=true`。
没有修改Manifest、Cargo.lock、曲线、SRS、旧代码/结果或论文主张；没有setup/prove或规模实验。
完整Miller、最终幂、GT比较及SnarkPack仍未完成，聚合验证Gas保持`null`。

## 下一步只做 G5-C3-B6：底层Fp乘法的小范围实现对照

1. 保留本轮为成本基准，新增一个低层Fp乘法版本；保持381位模数、编码及原乘法恒等式不变，
   只减少临时对象、MODEXP输入打包和内部调用开销，不把宿主结果当可信计算。
2. 先用同一Fp/Fp2输入对照完整结果、测量窗口、内存和MODEXP调用数；新的低层实现仍须核对
   进借位/模除二等正确性，不能仅凭Gas降低宣称实现正确。
3. 此阶段继续不重跑完整Miller；只有原语实现正确且得到实际成本收益后，再决定是否集成。

不扩大规模、不提高原Gas上限、不引入新证明或改曲线；不提前开展最终幂。

后续记录：B6 已完成同输入的低层实现对照与四个必要算术边界，
见 [G5_C3_FIELD_LOWLEVEL.md](G5_C3_FIELD_LOWLEVEL.md)。本页保留 B5 原始测量及当时计划，
当前下一步以新报告的 B7 集成验收为准。
