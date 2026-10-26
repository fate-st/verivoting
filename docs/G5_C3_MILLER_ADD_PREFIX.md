# G5-C3-B12：低分配 Fp2 加法集成前缀通过

2026-10-26。将 B11 已通过固定用例的同一 `_add2` 集成到独立的 B9-based
Miller 合约，只执行一次部署、原有两步前缀正例和一个错误目标。正例的全部
768B 返回值与冻结参考一致，错误目标以 `INVALID_MILLER_RESULT` 拒绝；本轮没有
构造或执行完整循环交易。

状态：`PASS_ADD2_MILLER_PREFIX_INTEGRATION_ONLY`，仅表示这一固定输入的前缀集成
通过。它不是完整 Miller、最终幂、GT 目标绑定或 SnarkPack EVM 验证结果。

## 真实测量

本轮沿用 B9 的公开 B=2 证明、冻结两步向量、正反例 calldata、编译配置和
20M 交易 Gas 上限。负例仍只改变预期结果的第一个规范 Fp 系数。

| 指标 | B9 | B12 |
|---|---:|---:|
| 两步正例总 Gas | 1,132,763 | 1,043,835 |
| 正例 execution Gas | 1,096,071 | 1,007,143 |
| 正例 intrinsic Gas | 36,692 | 36,692 |
| 两步活动 EVM 内存 | 2,411 words | 2,079 words |
| MODEXP 成功次数 | 516 | 516 |
| MODEXP execution Gas | 103,200 | 103,200 |
| 错误目标总 Gas | 1,132,581 | 1,043,653 |
| 新版本完整循环尝试数 | 0 | 0 |

正例节省 88,928 Gas，即 7.8505%；活动内存字数减少 332，即 13.7702%。
正反例各完成 516 次 MODEXP，均无失败，MODEXP trace 摘要保持一致。正例费用
满足 `1,007,143 + 36,692 = 1,043,835`；负例满足
`1,006,949 + 36,704 = 1,043,653`。两笔交易退款均为零，未受 calldata floor
支配。上述下降是同一固定前缀的集成测量，不是完整循环或聚合验证器 Gas 推算。

B12 部署总 Gas 为 1,565,064，runtime 为 6,992B；B9 为 1,562,736 / 6,981B。
因此前缀执行和活动内存下降，但部署费用增加 2,328 Gas、runtime 增加 11B，
不能声称所有成本均下降。本轮只有一个固定公开正例，不构成统计时间或规模结论。

## 实现范围

新增 `contracts/src/Bls12381MillerLoopAddLowLevel.sol`。运行器在执行前比较整份
源码，确认相对 B9 只改变合约名并完整替换 `_add2`；其余 51 个顶层函数逐字
不变，包括 `_mul`、`_mul2`、点倍加、NAF、专用平方/014 线乘、编解码与 arena。

所集成 `_add2` 与 B11 候选一致：在新的输出区直接处理低 limb 进位、`>=p`
单次约减和借位，完整初始化 Fp2 壳及两个 Fp 结果，不修改输入，也不使用宿主
预计算结果替代合约内算术。Miller 路径仍先深拷贝存活结果，再按原实现回退
arena；本轮用原有前缀正反例实际覆盖这一集成路径。

这些源码范围判断来自运行器的确定性比较及保存的原始记录；本页不额外声称完成
超出该检查范围的独立安全审计。没有重复执行 B11 边界、随机矩阵、历史算术测试
或任何重型证明实验。

## 完整循环与阻塞状态

B12 原始记录明确保存：

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

最新实际完整循环尝试仍是 B7 旧实现，在相同 20M 上限下 OOG。该历史结果只说明
B7 实现超出当时预算，不能写成 B12 再次 OOG；反过来，B12 的 7.85% 前缀收益
也不能证明新版本能够完成 64 步循环。目标状态继续引用 B7 原始记录：
`BLOCKED_SOFTWARE_MILLER_LOOP_AFTER_LOW_LEVEL_MUL_EXCEEDS_20M`。

最终幂、非单位 GT 目标比较、完整 SnarkPack 验证、结算绑定和链上写入均未实现。
Manifest、曲线、编码、SRS 和论文能力边界不升级，聚合验证 Gas 继续为 `null`。

## 产物、来源与复现

```bash
npm run --prefix contracts g5c3b12
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

- 合约：`contracts/src/Bls12381MillerLoopAddLowLevel.sol`。
- 运行器：`contracts/scripts/g5c3_miller_add_prefix.mjs`。
- 本次结果：`experiments/g5_c3_raw/miller_add_prefix_evm.json`。
- 集成基准：`experiments/g5_c3_raw/miller_fp2_prefix_evm.json`。
- 原语来源：`experiments/g5_c3_raw/fp2_add_lowlevel_profile.json`。
- 最新历史完整尝试：`experiments/g5_c3_raw/miller_lowlevel_evm.json`。

环境：Apple M4 / arm64，Node 23.11.0，EthereumJS 10.1.3 / Prague，
solc 0.8.30、optimizer 200、无 viaIR；本地签名交易，无 RPC。
Git 基准 `7442414a04e8d06d79d968b3cbf448a6a694347a`，`git_dirty=true`，
因此不是 clean-commit 论文性能样本。

| 产物或来源 | SHA-256 |
|---|---|
| B12 合约 | `b43045895f1b125938cbc3315cd6a5e0e690f224650d24424c8acfea40aeae2f` |
| B12 运行器 | `d37db2923103ffd5b51fedbd8ba3d3daf27d6408b6cc63cdbb199ae738f58de8` |
| B12 原始结果 | `a8b993cf9f0e3ea2a631c3c685fbd4facaf0885c6b2e9678838e2b681b527282` |
| B9 集成基准结果 | `e9228b36212a5b7685abd5cb962edefc2b234b4c539676bb9795a118706becf4` |
| B11 原语结果 | `92c9f44ce56fdf9ed5e303a347b9594caff0c5ffd46bba64107be5def1e9bd0a` |
| B7 完整尝试结果 | `cdf06a719f4df41a098ce1a2ce7a4b5e4a8e2e4eb61479b044aa717da9caa7e6` |
| Miller 冻结向量 | `1079b6fd309f7d11d60fcd2cd5f15a1e4b25895fe7f876bfe3be624d4f989ead` |
| B=2 公开证明向量 | `34180b26bcb316b2bf5d2a6ce810a6348a42cb2d5bcd3916a2f9a2a8dbcfaeb2` |
| 聚合证明字节 | `12e25c7edf07eff2d3f211a3cbb74ae4ce43e028ab58702debdef98b0787e64c` |

旧合约、旧原始结果和向量均保留。本轮没有 setup/prove、规模实验、RPC、完整循环
重试或最终幂。

## 下一步只做 B13：原 `_sub2` 单输入成本定位

1. 保持现有 `_sub2` 实现不变，只增加隔离测量入口。
2. 复用同一公开操作数，只执行一次固定 Fp2 减法，核对完整结果、窗口 Gas、
   分配增长、内存扩张和 MODEXP 次数。
3. 根据真实成本再决定是否值得提出候选实现；不在 B13 同时做优化或 Miller 集成。

B13 不重跑前缀或完整循环，不提高 20M 上限，不进入最终幂、规模或证明实验，
也不从 B12 前缀收益外推后续结果。

## 后续记录：B13 原减法成本基线已完成

B13 已保持 `_sub2` 不变，对同一公开 Fp2 操作数执行一次隔离减法。完整结果
正确；窗口为 1,286 Gas，execution 为 5,296 Gas，分配增长 640B，MODEXP 为
零。交易总 Gas 仍为 30,880，其中包含 632 Gas 的 Prague calldata-floor 补差。
详见 [G5_C3_FP2_SUB_COST_PROFILE.md](G5_C3_FP2_SUB_COST_PROFILE.md)。

该固定输入的 `c0` 直接减法发生低 limb 借位、`c1` 直接减法不借位，但两个
坐标都没有触发 `a < b` 的模下溢分支。B13 没有重跑 Miller；B12 前缀和 B7
历史完整尝试的状态不变。下一步 B14 只做一个低分配 `_sub2` 隔离候选对照，
保留 B13 基准及同一性能输入，并只增加一个直接相关的模下溢必要边界；不在
同一门集成 Miller 或重试完整循环。
