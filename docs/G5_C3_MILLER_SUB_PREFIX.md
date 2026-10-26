# G5-C3-B15：低分配 Fp2 减法集成前缀通过

2026-10-26。将 B14 已通过固定用例的同一 `_sub2` 精确集成到独立的 B12-based
Miller 合约，只执行一次部署、原有两步前缀正例和一个错误目标。正例的完整
768B 返回值与冻结参考一致，错误目标以 `INVALID_MILLER_RESULT` 拒绝；本轮没有
构造或执行完整循环交易。

状态：`PASS_SUB2_MILLER_PREFIX_INTEGRATION_ONLY`，仅表示这一固定输入的前缀
集成通过。它不是完整 Miller、最终幂、GT 目标绑定或 exact SnarkPack EVM
验证结果。

## 真实测量

本轮沿用 B12 的公开 B=2 证明、冻结两步向量、正反例 calldata、编译配置和
20M 交易 Gas 上限。负例仍只改变预期结果的第一个规范 Fp 系数。

| 指标 | B12 | B15 |
|---|---:|---:|
| 两步正例总 Gas | 1,043,835 | 985,557 |
| 正例 execution Gas | 1,007,143 | 948,865 |
| 正例 intrinsic Gas | 36,692 | 36,692 |
| 两步活动 EVM 内存 | 2,079 words | 1,905 words |
| MODEXP 成功次数 | 516 | 516 |
| MODEXP execution Gas | 103,200 | 103,200 |
| 错误目标总 Gas | 1,043,653 | 985,375 |
| 新版本完整循环尝试数 | 0 | 0 |

正例节省 58,278 Gas，即 5.5831%；活动内存字数减少 174，即 8.3694%。
正反例各完成 516 次 MODEXP，均无失败，MODEXP trace 摘要保持一致。正例费用
满足 `948,865 + 36,692 = 985,557`；负例满足
`948,671 + 36,704 = 985,375`。两笔交易退款均为零，未受 calldata floor
支配。

上述下降是同一个固定两步前缀的集成测量，不是完整循环 Gas 外推。B14 隔离
窗口下降 40.51%，集成前缀只下降 5.58%，再次表明原语窗口比例不能直接替代
真实调用路径测量。

## 部署权衡

B15 部署总 Gas 为 1,589,758，runtime 为 7,106B；B12 为
1,565,064 / 6,992B。前缀执行和活动内存下降，但部署费用增加 24,694 Gas，
runtime 增加 114B，因此不能声称所有成本均改善。

这是一次固定公开正例与一个对应负例，不是统计性能样本；本地 wall-clock
时间不用于论文规模或吞吐结论。

## 实现范围

新增 `contracts/src/Bls12381MillerLoopSubLowLevel.sol`。运行器按整份源码构造
期望值并比较，确认相对 B12 只改变合约名并完整替换 `_sub2` 为 B14 函数体；
其余常量、结构体、注释和函数均不变，包括 `_add2`、`_mul`、`_mul2`、点倍加、
NAF、专用平方/014 线乘、编解码与 arena。

B14 `_sub2` 的数学公式未变；它使用新的 Fp2 壳和两个独占 Fp 输出，处理直接
低 limb 借位及负差时加回 p 的进位，不覆盖 a/b，也不使用宿主预计算结果替代
合约内算术。原两步前缀实际覆盖该函数在点更新、线计算及 arena 生命周期中的
集成调用；完整输出和错误目标判定均保持一致。

这些变化范围来自运行器的确定性源码比较及保存的 raw 记录；本页不额外声称
完成一般化代码或密码学安全审计。没有重复 B14 边界、随机矩阵或历史算术测试。

## 完整循环与阻塞状态

B15 原始记录明确保存：

```text
full_loop_attempts = 0
full_attempt = null
full_miller_loop_verified_in_evm = false
full_miller_verification_gas = null
evm_final_exponentiation_implemented = false
pairing_to_gt_evm_verified = false
exact_snarkpack_evm_verified = false
snarkpack_verification_gas = null
```

最新实际完整循环尝试仍是 B7 旧实现，在相同 20M 上限下 OOG。该历史记录没有
覆盖 B15，不能写成 B15 再次 OOG。反过来，B15 的前缀 Gas 已降至 985,557，
也不能证明 64 步完整循环已进入预算，因为各步线路、加法次数和内存轨迹并非由
两步样本线性代表。

目标状态继续引用 B7：
`BLOCKED_SOFTWARE_MILLER_LOOP_AFTER_LOW_LEVEL_MUL_EXCEEDS_20M`。完整 Miller、
最终幂、非单位 GT 目标比较、完整 SnarkPack 验证及结算均未完成，聚合验证 Gas
保持 `null`；Manifest、曲线、编码、SRS 和论文能力边界不升级。

## 产物、来源与复现

```bash
npm run --prefix contracts g5c3b15
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

- 合约：`contracts/src/Bls12381MillerLoopSubLowLevel.sol`。
- 运行器：`contracts/scripts/g5c3_miller_sub_prefix.mjs`。
- 本次结果：`experiments/g5_c3_raw/miller_sub_prefix_evm.json`。
- 集成基准：`experiments/g5_c3_raw/miller_add_prefix_evm.json`。
- 原语来源：`experiments/g5_c3_raw/fp2_sub_lowlevel_profile.json`。
- 最新历史完整尝试：`experiments/g5_c3_raw/miller_lowlevel_evm.json`。

环境：Apple M4 / arm64，Node 23.11.0，EthereumJS 10.1.3 / Prague，
solc 0.8.30、optimizer 200、无 viaIR；本地签名交易，无 RPC。
Git 基准 `7442414a04e8d06d79d968b3cbf448a6a694347a`，`git_dirty=true`，
不是 clean-commit 论文性能样本。

| 产物或来源 | SHA-256 |
|---|---|
| B15 合约 | `8f13f1c53c5cf3ddc8f04a8259b3944d0a9fc642897f04a3cd8e5659c596eb06` |
| B15 运行器 | `a76f06817e2fc645a8e44ea0723a1a3d40427cfc0ac08b54539ba51efc53f7a3` |
| B15 原始结果 | `702ca228900fbfb5decd02ad383205b08ddadeeec8b2ebd04eb6cbd4f205ad37` |
| B12 集成基准结果 | `a8b993cf9f0e3ea2a631c3c685fbd4facaf0885c6b2e9678838e2b681b527282` |
| B14 原语结果 | `008d45bff28202b1ede28901a8c65a9ebec69649b9aaed2e65609d07557a023e` |
| B7 完整尝试结果 | `cdf06a719f4df41a098ce1a2ce7a4b5e4a8e2e4eb61479b044aa717da9caa7e6` |
| Miller 冻结向量 | `1079b6fd309f7d11d60fcd2cd5f15a1e4b25895fe7f876bfe3be624d4f989ead` |
| B=2 公开证明向量 | `34180b26bcb316b2bf5d2a6ce810a6348a42cb2d5bcd3916a2f9a2a8dbcfaeb2` |
| 聚合证明字节 | `12e25c7edf07eff2d3f211a3cbb74ae4ce43e028ab58702debdef98b0787e64c` |

旧合约、B12/B14 基准、历史结果和固定向量均保留。本轮没有 setup/prove、RPC、
规模实验、完整循环重试或最终幂。

## 下一步只做 B16：一次最新版 full64 证据测试

B15 后的只读预算复核已经完成。B7 之后，低分配 `_mul2`、`_add2`、`_sub2`
均已依次集成并通过同一前缀正反例；但包含这三项修改的最新版从未执行 full64，
所以当前缺口不是另一项无界原语 profiling，而是一次受控的最新版完整循环证据。

B16 固定使用 B15 合约、fixture 与编译设置，只执行一次部署和一笔 full64 正例，
交易 Gas 上限保持 20M。不修改实现，不重跑前缀 pair、隔离边界、规模、setup 或
prove，也不进入最终幂。

- 若成功，必须核对完整 768B 返回值、14,316 次 MODEXP 全部成功，并记录实际
  完整循环 Gas 和活动内存；
- 若失败，保存真实异常、成功/失败 MODEXP 次数及活动内存，完整循环验证 Gas
  继续为 `null`，并先分析这一次失败，不继续盲目逐原语优化。

14,316 是完整执行的预期 MODEXP 总数；B7 在 OOG 前的 7,848 次成功调用只是
失败轨迹。二者都不能写成“完成率”。两步前缀也不能线性外推 full64 Gas。

B16 仅为下一门计划，本报告不执行它、不提高上限、不推算完整验证器 Gas，也不
升级 Manifest 能力。
