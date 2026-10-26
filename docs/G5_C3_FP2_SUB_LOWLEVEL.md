# G5-C3-B14：低分配 Fp2 减法对照通过

2026-10-26。本轮保留 B13 原始成本基准，只替换 `_sub2` 为低分配候选，并执行
同一个公开性能输入及一个必要的模下溢边界。两个完整 128B 结果均与直接模减和
Noble Fp2 参考一致；没有运行 Miller、RPC、setup/prove 或规模实验。

状态：`PASS_LOW_ALLOCATION_FP2_SUB_FIXED_CASES_ONLY`，仅表示该候选在这两个固定
用例上通过，并在同一性能输入上降低窗口、execution 和分配成本。它不是完整
Miller、最终幂或 SnarkPack EVM 验证结果。

## 同一公开输入的真实对照

性能输入与 B13 完全相同，来自公开 B=2 聚合证明 `final_b` 的仿射坐标
`q.x - q.y`。calldata SHA-256 保持一致；完整规范输出也与 B13 相同。

| 项目 | B13 原实现 | B14 候选 |
|---|---:|---:|
| 算术窗口 Gas | 1,286 | 765 |
| execution Gas | 5,296 | 4,790 |
| intrinsic Gas | 24,952 | 24,952 |
| 交易总 Gas | 30,880 | 30,880 |
| calldata floor 补差 | 632 | 1,138 |
| 窗口分配指针增长 | 640B | 384B |
| 窗口内存扩张 Gas | 65 | 39 |
| 窗口内存 | 44 -> 64 words | 44 -> 56 words |
| 退出活动 EVM 内存 | 79 words | 71 words |
| MODEXP 次数 / Gas | 0 / 0 | 0 / 0 |

窗口减少 521 Gas，即 40.5132%；execution 减少 506 Gas；分配增长减少 256B，
即 40%；窗口内存扩张费用从 65 降为 39。窗口外 execution 从 4,010 变为
4,025，因此不能把窗口节省直接当作整笔执行的同额节省。

B14 的 `24,952 + 4,790 = 29,742`，低于 Prague calldata floor 30,880，故
实际总 Gas 仍为 30,880，floor 补差升至 1,138。B13 也受同一 floor 支配。
本轮可以声称固定输入的算术窗口、execution 和分配成本下降，但不能声称该笔
交易总费用下降。补差不属于算术或内存成本，退款为零。

B14 部署总 Gas 为 675,510、runtime 为 2,878B；B13 为 699,328 / 2,988B。
这些部署数值包含 profiling 合约整体，不是 Miller 部署成本。本轮只有一个性能
样本，wall-clock 时间不用于统计结论。

## 唯一必要边界通过

B13 的公开输入覆盖两种直接减法：`c0` 有低 limb 借位、`c1` 无借位，但不触发
模下溢。本轮只增加以下边界，不计入性能样本：

```text
a = (0, 0)
b = (1, 2^256)
a - b = (p-1, p-2^256)
```

该单个 Fp2 用例同时覆盖：

- `c0` modular wrap、低 limb 借位及加回 p 时的低 limb 进位；
- `c1` modular wrap，但直接低 limb 不借位、加回 p 时也不产生低 limb 进位。

完整输出正确。边界交易总 Gas 为 27,574，execution 为 4,902，窗口为 877，
分配增长 384B，MODEXP 为零；它只用于分支正确性，不纳入 B13/B14 性能对照。
本轮不增加随机、别名、全边界或故障矩阵。

## 唯一实现变化

新增 `contracts/src/Bls12381Fp2SubLowLevelProfile.sol`。运行器严格比较 B13
源码，确认除合约名与 `_sub2` 外，其他 18 个既有函数体逐字不变，包括 `_sub`、
`_add2`、`_mul`、`_mul2`、编解码和四个 profile 入口。数学公式、规范输入要求、
曲线、编码和 Suite 均未改变。

新 `_sub2` 在局部 Yul 中申请一个新 Fp2 壳和两个互不重叠的 Fp 输出，共显式
192B；分别计算低 limb、借位后的高 limb，并在负差时加回一次 p，包括加回时的
低 limb 进位。它不覆盖或修改 a/b，也不使用宿主结果替代合约内算术。编译窗口
实测分配增长为 384B，应以实测值而不是显式申请量报告。

上述变化范围由运行器的确定性源码比较和 raw 记录确认；本页不声称超出这些检查
的一般化安全审计。B14 只执行一次部署、一个公开性能调用和一个必要边界调用。

## 完整验证状态不变

B14 原始记录为 `miller_run=false`、`full_loop_attempts=0`、
`full_attempt=null`。最新实际完整循环尝试仍是 B7 旧实现的 20M OOG；B12 是
当前最新前缀，但没有完整循环尝试。

完整 Miller、EVM 最终幂、非单位 GT 目标绑定及 exact SnarkPack EVM 验证仍未
完成，相关验证 Gas 均为 `null`。B14 的隔离窗口下降不是嵌入 Miller 后的固定
收益，也不能证明减法是唯一瓶颈或推算完整循环 Gas。Manifest 和论文能力边界
保持不变。

## 产物、来源与复现

```bash
npm run --prefix contracts g5c3b14
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

- 合约：`contracts/src/Bls12381Fp2SubLowLevelProfile.sol`。
- 运行器：`contracts/scripts/g5c3_fp2_sub_lowlevel.mjs`。
- 原始结果：`experiments/g5_c3_raw/fp2_sub_lowlevel_profile.json`。
- B13 基准：`experiments/g5_c3_raw/fp2_sub_cost_profile.json`。
- 当前前缀：`experiments/g5_c3_raw/miller_add_prefix_evm.json`。
- 最新历史完整尝试：`experiments/g5_c3_raw/miller_lowlevel_evm.json`。

环境：Apple M4 / arm64，Node 23.11.0，EthereumJS 10.1.3 / Prague，
solc 0.8.30、optimizer 200、无 viaIR；本地签名交易，无 RPC。
Git 基准 `7442414a04e8d06d79d968b3cbf448a6a694347a`，`git_dirty=true`，
不是 clean-commit 论文性能样本。

| 产物或来源 | SHA-256 |
|---|---|
| B14 合约 | `bf022f566465531873d19f024366ae5167c2412e555489974c1f1160d9215240` |
| B14 运行器 | `34a53a2782048af1a99f25a731fe659151ae105c3c62e2367d4d915765fcdacb` |
| B14 原始结果 | `008d45bff28202b1ede28901a8c65a9ebec69649b9aaed2e65609d07557a023e` |
| B14 `_sub2` 函数体 | `42ea303fc264980cae246aea7bef34227c20c2f89a4596b06b82bff830e83ada` |
| B13 基准结果 | `2995b8bb2b3d2220a8054e49b8ea74614ad9a9477cf77167cd9ef72ea3203b37` |
| B12 前缀结果 | `a8b993cf9f0e3ea2a631c3c685fbd4facaf0885c6b2e9678838e2b681b527282` |
| B7 完整尝试结果 | `cdf06a719f4df41a098ce1a2ce7a4b5e4a8e2e4eb61479b044aa717da9caa7e6` |
| Miller 冻结向量 | `1079b6fd309f7d11d60fcd2cd5f15a1e4b25895fe7f876bfe3be624d4f989ead` |
| B=2 公开证明向量 | `34180b26bcb316b2bf5d2a6ce810a6348a42cb2d5bcd3916a2f9a2a8dbcfaeb2` |
| 聚合证明字节 | `12e25c7edf07eff2d3f211a3cbb74ae4ce43e028ab58702debdef98b0787e64c` |

旧合约、B13 基准、历史结果及固定向量均保留。

## 下一步只做 B15：集成 `_sub2` 后验收同一前缀

1. 新建独立 B12-based Miller 版本，精确集成本轮 `_sub2`；除合约名外，其余
   源码保持 B12 不变。
2. 只执行 B12 的同一两步前缀正例和一个错误目标，核对完整 768B 输出、实际
   Gas、活动内存与 516 次 MODEXP；不重复 B14 边界或其他算术矩阵。
3. 根据集成结果再决定是否值得进行任何完整循环尝试，不从 40.51% 隔离窗口
   降幅推算 Miller 收益。

B15 暂不在本轮实施，不提高 20M 上限，不运行完整循环、最终幂、规模或证明实验。

## 后续记录：B15 减法集成前缀已完成

B15 已将本页的同一 `_sub2` 精确集成到独立 B12-based Miller 合约，并只运行
原两步前缀正例及一个错误目标。正例 Gas 从 1,043,835 降至 985,557
（5.58%），活动内存从 2,079 降至 1,905 words（8.37%）；错误目标正确拒绝，
516 次 MODEXP 保持不变。详见
[G5_C3_MILLER_SUB_PREFIX.md](G5_C3_MILLER_SUB_PREFIX.md)。

部署总 Gas 同时从 1,565,064 升至 1,589,758，runtime 从 6,992 增至
7,106B，不能声称所有成本改善。B15 的 `full_loop_attempts=0`、
`full_attempt=null`；最新完整尝试仍是 B7 旧版 20M OOG，不能据此归因于 B15，
也不能由 5.58% 前缀收益推断完整循环可进入预算。

下一门槛暂不机械设为完整循环或另一项原语 profiling。先复核 B2--B15 的实际
热点、活动内存和旧完整尝试预算，再选择一个有明确验收条件的单一后续门；期间
不提高 20M 上限、不提前最终幂。
