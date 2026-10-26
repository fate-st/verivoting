# G5-C3-B13：原 Fp2 减法成本定位完成

2026-10-26。本轮保持 `_sub2` 算术不变，只新增一个隔离测量入口，并对保存的
公开 endpoint 坐标执行一次 Fp2 减法。完整 128B 结果与直接模减及 Noble Fp2
参考一致；没有运行其他算术测试、Miller、RPC 或证明实验。

状态：`PASS_ISOLATED_FP2_SUB_COST_PROFILE_ONLY`。这是一条原始实现的固定输入
成本基线，不是优化结果，也不表示完整 Miller 或 SnarkPack 验证取得新进展。

## 真实测量

操作数来自同一公开 B=2 聚合证明的 `final_b` 仿射坐标，计算 `q.x - q.y`；它们
也正是 B11 加法成本对照所用的两个完整 Fp2 输入。

| 项目 | B13 原 `_sub2` |
|---|---:|
| 算术窗口 Gas | 1,286 |
| execution Gas | 5,296 |
| intrinsic Gas | 24,952 |
| 交易总 Gas | 30,880 |
| calldata floor 补差 | 632 |
| 窗口分配指针增长 | 640B |
| 窗口内存扩张 Gas | 65 |
| 窗口内存 | 44 -> 64 words |
| 退出活动 EVM 内存 | 79 words |
| MODEXP 次数 / Gas | 0 / 0 |

费用对账为 `24,952 + 5,296 = 30,248`，低于 Prague calldata floor 的
30,880，因此实际总 Gas 包含 632 Gas 的 floor 补差。补差不是算术执行或内存
成本。两次 GAS 标记的差值为 1,286，窗口内存扩张费用为 65；顶层 opcode
5,296 与 execution 完全对账，退款为零。

窗口从规范输入解码后包围未修改的 `_sub2`，并在输出编码前结束；它仍包含
编译器调用和测量开销，不能当作嵌入 Miller 后的固定减法常数。分配指针增长
640B 也不是 EVM 峰值内存或主机 RSS。本轮只有一个固定公开性能输入，不提供
统计时间、规模或端到端效率结论。

部署总 Gas 为 699,328，runtime 为 2,988B。新增 profiling 入口会影响部署体积；
这些数值不应与完整 Miller 合约部署费用混用。

## 输入覆盖范围

本次两个坐标均走直接减法，不触发 `a < b` 的模下溢分支：

- `c0` 直接减法，低 limb 发生借位；
- `c1` 直接减法，低 limb 不发生借位；
- 两个坐标均没有 modular wrap。

因此本次固定输入同时覆盖“直接减且有低 limb 借位”和“直接减且无低 limb
借位”，但不覆盖模下溢后的 `p - (b-a)` 路径。不能仅凭这个正例认定候选实现
已覆盖全部分支；B14 需要增加一个直接相关的模下溢边界。

## 实现范围

新增 `contracts/src/Bls12381Fp2SubProfile.sol`。运行器确认该文件相对 B11 原语
基准只改变合约名并增加 `profileFp2Sub` 入口；18 个既有函数体均逐字不变，
包括 `_sub`、`_sub2`、`_add2`、`_mul`、`_mul2` 及原有三个 profile 入口。
新增入口与原加法 profile 的结构相同，仅调用 `_sub2`。

运行器还核对 `_sub2` 与 B12 Miller 合约中的函数体完全相同。其实现仍为分别
调用 `_sub(a.c0,b.c0)` 与 `_sub(a.c1,b.c1)`，没有宿主语言结果替代、曲线、
编码、公式或 Manifest 变化。`arithmetic_changed=false`、
`optimization_implemented=false`。

这里的范围结论来自运行器的确定性源码比较和保存的 raw 记录；本页不声称额外
完成一般化安全审计。没有随机矩阵、额外边界、乘法、前缀或完整循环重跑。

## 完整验证状态不变

B13 是原语成本定位，`miller_run=false`、`full_loop_attempts=0`、
`full_attempt=null`。最新实际完整循环尝试仍是 B7 旧实现的 20M OOG；B12
只有前缀通过，未尝试完整循环。

完整 Miller、EVM 最终幂、非单位 GT 目标绑定与 exact SnarkPack EVM 验证仍未
完成，相关验证 Gas 均为 `null`。目标状态继续为
`BLOCKED_SOFTWARE_MILLER_LOOP_AFTER_LOW_LEVEL_MUL_EXCEEDS_20M`。本轮不升级
协议能力，也不修改曲线、Suite、SRS、Manifest 或论文主张。

## 产物、来源与复现

```bash
npm run --prefix contracts g5c3b13
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

- 合约：`contracts/src/Bls12381Fp2SubProfile.sol`。
- 运行器：`contracts/scripts/g5c3_fp2_sub_profile.mjs`。
- 原始结果：`experiments/g5_c3_raw/fp2_sub_cost_profile.json`。
- 算术来源：`experiments/g5_c3_raw/fp2_add_lowlevel_profile.json`。
- 当前前缀：`experiments/g5_c3_raw/miller_add_prefix_evm.json`。
- 最新历史完整尝试：`experiments/g5_c3_raw/miller_lowlevel_evm.json`。

环境：Apple M4 / arm64，Node 23.11.0，EthereumJS 10.1.3 / Prague，
solc 0.8.30、optimizer 200、无 viaIR；本地签名交易，无 RPC。
Git 基准 `7442414a04e8d06d79d968b3cbf448a6a694347a`，`git_dirty=true`，
不是 clean-commit 论文性能样本。

| 产物或来源 | SHA-256 |
|---|---|
| B13 合约 | `4d92e40727f12d84632f017055e1ad142bb8acfd3a0b70499a76c3edc459defa` |
| B13 运行器 | `e49ecaa454afc1f1dab569310f6d33b6fb40a95309e2d77355598b49c6bd8371` |
| B13 原始结果 | `2995b8bb2b3d2220a8054e49b8ea74614ad9a9477cf77167cd9ef72ea3203b37` |
| `_sub2` 函数体 | `6cb8dd6218aaab609615a25983c4542effd31223ec18699492ec80d88f286c21` |
| B11 算术基准结果 | `92c9f44ce56fdf9ed5e303a347b9594caff0c5ffd46bba64107be5def1e9bd0a` |
| B12 前缀结果 | `a8b993cf9f0e3ea2a631c3c685fbd4facaf0885c6b2e9678838e2b681b527282` |
| B7 完整尝试结果 | `cdf06a719f4df41a098ce1a2ce7a4b5e4a8e2e4eb61479b044aa717da9caa7e6` |
| Miller 冻结向量 | `1079b6fd309f7d11d60fcd2cd5f15a1e4b25895fe7f876bfe3be624d4f989ead` |
| B=2 公开证明向量 | `34180b26bcb316b2bf5d2a6ce810a6348a42cb2d5bcd3916a2f9a2a8dbcfaeb2` |

旧合约、旧结果与所有固定向量均保留。本轮实际范围只有一次部署和一次原
`_sub2` 固定输入交易。

## 下一步只做 B14：低分配 `_sub2` 隔离对照

1. 保留 B13 基准，新建一个只改变合约名和 `_sub2` 的低分配候选。
2. 复用同一固定公开输入，比较完整规范结果、窗口 Gas、execution Gas、分配增长
   和内存扩张；MODEXP 应保持为零。
3. 只增加一个直接相关的模下溢必要边界，例如
   `(0,0) - (1,0) = (p-1,0)`，核对完整 Fp2 结果，不计入性能样本。

B14 仅计划、不在本轮实施；不增加随机或规模矩阵，不集成 Miller，不重试完整
循环，也不提前进入最终幂。

## 后续记录：B14 低分配减法对照已完成

B14 保留本页 B13 基准，只替换 `_sub2` 并运行同一公开性能输入及一个必要的
模下溢边界。公开输入的窗口从 1,286 降至 765 Gas（40.51%），execution 从
5,296 降至 4,790 Gas，分配增长从 640 降至 384B（40%），窗口内存扩张费用
从 65 降至 39；MODEXP 保持为零。交易总 Gas 仍为 30,880，因为 calldata-floor
补差从 632 增至 1,138，不能称为总交易费用下降。详见
[G5_C3_FP2_SUB_LOWLEVEL.md](G5_C3_FP2_SUB_LOWLEVEL.md)。

唯一边界使用 `a=(0,0)`、`b=(1,2^256)`，完整结果为
`(p-1,p-2^256)`，同时覆盖模下溢时有/无低 limb 借位及相应加回 p 的进位路径；
它不计入性能样本。B14 未运行 Miller。下一步 B15 仅把该 `_sub2` 集成到独立
B12-based Miller 版本，并验收原两步前缀正例和错误目标；暂不进行完整循环。
