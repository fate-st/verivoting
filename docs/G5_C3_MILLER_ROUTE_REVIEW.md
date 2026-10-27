# G5-C3-B19：固定模数除四的源内路线复核

2026-10-27。本轮读取 B18 归因、B6 隔离乘法基线和现有 `_mul`，执行 Node 源码
与代数核对，选择一个待测候选：将乘法末尾两次 `halfMod` 合为一次固定模数
`quarterMod`。本轮没有创建 VM、部署合约、修改 Solidity 或测量新 Gas。

状态：`PASS_SOURCE_DERIVED_MUL_QUARTER_ROUTE_REVIEW_ONLY`。它只表示来源、
代数与最小验收方案已核对，不表示优化实现或成本收益已经通过。

## 从自动筛选暂停到单一人工审查候选

B18 的最大已映射函数 `_mul` 为 8,051,757 root Gas，下一已映射函数 `_add2`
为 2,155,414；未映射、生成代码和框架合计 3,999,108 Gas，占已收费 root 的
22.4193%。B18 自动候选要求该比例低于 10%，因此当时候选为空。

10% 是自动筛选的工作启发式，不是禁止进一步源内数学审查的安全条件。
本轮使用更保守的排名界：即使把全部未归入函数的费用都给下一已映射函数，仍有

```text
2,155,414 + 3,999,108 = 6,154,522 < 8,051,757
差额 = 1,897,235 Gas
```

因此 `_mul` 在这条失败路径上的最大源码归属桶位置仍稳健。这个界只涉及互斥
语法归属，不是动态调用栈成本，也不是可以消除的 Gas 保证。本轮选择候选还依赖
下面的具体恒等式，未把未知费用强行分配给任何函数。

`_mul::halfMod` 自身在 B18 中为 1,080,196 Gas，仅占已收费 root 的 6.0557%。
即使候选有收益，也不能把它视为已解决完整循环预算或完整 verifier 的方案。

## 当前乘法与合并依据

现有 `_mul` 仍使用两次真实 MODEXP 平方计算

```text
t = ((a+b)^2 - (a-b)^2) mod p
result = halfMod(halfMod(t))
```

`subMod` 保证传入除法阶段的 t 是规范值 `0 <= t < p`。B19 核对确认 B6 隔离
profile 合约与当前 Miller 合约的完整 `_mul` 函数体逐字相同，因此已有隔离
基线可以用于下一轮对照，无需在 Miller 内先行试改。

固定 BLS12-381 基础域模数满足 `p mod 4 = 3`。令 `k = t & 3`，则
`t + k*p = t - k (mod 4) = 0`，所以可定义

```text
quarterMod(t) = (t + k*p) / 4
```

由于 `0 <= t < p`、`0 <= k <= 3`，分子小于 4p，结果小于 p；同时
`4*quarterMod(t) = t (mod p)`，故它与两次规范模除二相等，无需额外约减。
不能直接把 t 右移两位，也不能先将 `t+k*p` 模 p 约减后再右移。

## 两个 limb 的无分支候选伪代码

令 `B = 2^256`，`p = P_HI*B + P_LO`。由当前固定常量逐项核对：

```text
p mod 4 = 3
2*P_LO < B < 3*P_LO < 2*B
4*p < B^2
```

所以 `floor(k*P_LO/B)` 恰为 `k == 3`。下面的伪代码中所有 limb 加、乘和左移
均采用 EVM uint256 截断语义；比较产生 0 或 1，k 必须从原始 lo 取得：

```text
k        = lo & 3
lowProd  = k * P_LO                         // 截断后的低 limb
prodHi   = k * P_HI + (k == 3)              // kp 的低乘积进位
sumLo    = lo + lowProd                     // 截断后的和
sumHi    = hi + prodHi + (sumLo < lo)       // 相加产生的另一项进位
resultHi = sumHi >> 2
resultLo = (sumLo >> 2) | (sumHi << 254)
```

`k == 3` 与 `sumLo < lo` 是两种独立进位，可能同时为 1，必须相加，不能用 OR
合并。遗漏 `k*P_LO` 的乘积高位也会出错。这一简化依赖上述固定模数的不等式，
不是适用于任意模数或曲线的通用除四函数。

源层将两个 helper 调用合为一个、两轮跨 limb 位移合为一轮，避免原两次条件
加 p 的分支；同时引入小整数乘法和进位计算。编译器、栈调度及不同 residue
会影响实际成本，当前不报告 Gas 节省，也不保证编译后的净收益。
两次 MODEXP、乘法恒等式、规范编码和原内存布局均应保持不变。

## 下一门 B20：一个性能输入、四个必要正确性值

B20 尚未执行。计划只在独立 Fp `_mul` profile 中更换除四阶段，一次部署，
一个已保存公开性能调用，四个正确性调用，共六笔本地交易；不运行 Miller。

性能输入继续使用 B6 的同一公开 `final_b` 坐标分量。B6 的历史窗口为
2,167 Gas，总交易为 27,606 Gas，分配指针增长为 384B。这些是旧实现的真实
基准，不能当作新候选结果；四个正确性调用也不纳入性能样本。

对于每个指定 t，使用 `a=1, b=(t+(t&3)*p)/4`，真实 `_mul` 的内部规范差值
满足 `4ab mod p = t`，期望乘积就是 b：

| 指定内部 t | 校正 k | kp 低乘积进位 | sumLo 相加进位 |
|---|---:|---:|---:|
| 0 | 0 | 0 | 0 |
| p-2 | 1 | 0 | 0 |
| p-1 | 2 | 0 | 1 |
| B-1 | 3 | 1 | 1 |

第四例同时触发两项进位，直接检查不能以 OR 取代加法。四例覆盖所有低两位
校正及此次实现涉及的进位；raw 中的规范 64B 输入和期望值均为代数推导，不是
模拟或已执行的 EVM 结果。本轮只核对这些推导，不宣称内部指令覆盖跟踪。

B20 最小验收要求为：五个完整规范结果均正确，每次乘法恰有两次成功 MODEXP，
分配量保持基线口径不变、费用对账通过，且同一公开性能输入的隔离窗口严格小于
B6 的 2,167 Gas。必须分别报告窗口、execution、交易总费用和分配量，不能用
窗口下降代替交易总费用下降。

若正确性或窗口收益不满足，保留原实现并停止该候选；通过后是否集成也属于
独立门槛，不自动追加前缀、full、最终幂、随机矩阵或规模实验。编译配置和
20M 交易 / 30M block 预算均不变，不实施 wrapper 或更换 Suite。

## 产物、范围和哈希

```bash
npm run --prefix contracts g5c3b19
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

- 源码/代数分析器：`contracts/scripts/g5c3_miller_route.mjs`。
- 派生记录：`experiments/g5_c3_raw/miller_route_review.json`。
- 归因来源：`experiments/g5_c3_raw/miller_cost_attribution.json`。
- 隔离基线：`experiments/g5_c3_raw/field_lowlevel_profile.json`。

本轮记录 `evm_transactions_executed=0`、`optimization_implemented=false`，
并保持合约和 Manifest 不变。九项来源、源码、依赖锁和 Manifest 哈希及四项
推导输入已逐项复核。Git 基准为 `e80aa2b9fb8770653d7eb23351d27fc89b0d466f`，
`git_dirty=true`；本轮没有新性能样本。

| 产物或来源 | SHA-256 |
|---|---|
| B19 分析器 | `5dece43da88e91c8c451192bc8102665dd58cd2146473281d648faeef6ed972b` |
| B19 派生记录 | `0f95968de0efdb6f41af221e113b3545f757d48e6a9a284e2b980bc4c113061b` |
| B18 raw | `6645a9dbdc8c363944723edc89b09a947e5533b95d3a507076b8086c2fb9ec37` |
| B6 隔离基线 | `4753d634e56d59986817e61846bb6c9d6252ed60477eb225c08cc2d95b95d318` |
| B6 profile 合约 | `9b2c03c91f3dbab325be969f675a62e9ba0b8a0e4f214f90ab2420644ca29786` |
| 当前 Miller 合约 | `8f13f1c53c5cf3ddc8f04a8259b3944d0a9fc642897f04a3cd8e5659c596eb06` |

最新实际 full 仍是 B18 的 20M OOG，目标状态继续为
`BLOCKED_SOFTWARE_MILLER_SUB_LOWLEVEL_EXCEEDS_20M`。完整 Miller、最终幂、GT
绑定和 exact SnarkPack EVM 验证没有完成。候选实测 Gas、完整循环 Gas、完成率
及 SnarkPack 验证 Gas 均保持 null，不从本次推导承诺 20M 内成功。

## 后续记录：B20 隔离乘法已真实执行

2026-10-27，B20 在独立 B6-based profile 中实现本页 quarterMod 候选，实际
执行一次部署、一个原公开性能输入及四个校正/进位用例，六笔交易全部完成。
五个完整规范结果及每笔费用对账通过，两次 MODEXP / 每次乘法、384B 分配及
37 Gas 窗口内存扩张保持不变。

同一公开输入的窗口为 `2,167 -> 2,023 Gas`，下降 144 Gas（6.6451%）；
execution 为 `4,318 -> 4,174`，交易总费用为 `27,606 -> 27,462`，没有
calldata floor 补差。runtime 为 `2,519 -> 2,491B`，部署总 Gas 为
`597,983 -> 591,914`。四个正确性用例只用于检查，不作为额外性能样本。

候选尚未集成 Miller，最新 full 仍是 B18 的 20M OOG，完整验证器继续 BLOCKED。
本页前文保留 B19 当时的源内推导及计划背景；实测结果见
[B20 隔离乘法报告](G5_C3_FP_MUL_QUARTER.md)。下一门 B21 仅规划将相同完整
`_mul` 集成到独立 B15-based 版本并验收原前缀正/错对，不自动追加 full 重试。

B20 raw：`experiments/g5_c3_raw/fp_mul_quarter_profile.json`，SHA-256
`0d44c5b76b33816cf37f85aaacd52ff7170afc09cd5e92a987b10c198b5cb52c`。
