# G5-C3-B6：低层 Fp 乘法对照通过

2026-10-26。本轮完成一个必要的小步骤：保留 B5 基准，新增低层 `_mul`，
只执行两项固定公开输入成本对照和四项确定性算术边界。所有完整输出均正确，
两个测量窗口均有实际成本下降。没有重跑完整 Miller、setup/prove 或规模实验。

状态：`PASS_LOW_LEVEL_FP_FIXED_CASES_AND_COST_REDUCTION_ONLY`。
这不是完整 Miller、pairing-to-GT 或 SnarkPack 链上验证通过。

## 改动范围

新增 `contracts/src/Bls12381FieldLowLevelProfile.sol`，只改变 B5 的 `_mul` 实现。
其余 14 个辅助函数及两个测量入口的函数体由运行器逐字核对。
保留原 381 位模数、64B 大端 Fp 编码、Fp2 塔结构和同一乘法恒等式：

`ab = ((a+b)^2 - (a-b)^2) / 4 (mod p)`。

低层实现以内联 Yul 处理规范加减、跨 limb 进借位、两次模除二，减少临时对象；
两次平方仍是真实 MODEXP 调用，不使用宿主语言结果替代 EVM 算术。
64B 结果槽和 256B MODEXP 输入区分开，后者重复使用；不回退分配指针。
每次调用均检查成功和 64B 返回长度。恒等式的背景见
[EIP-198](https://eips.ethereum.org/EIPS/eip-198)。

## 真实结果

沿用 B5 的公开 `final_b` 仿射坐标：Fp 使用 `x.c0 * y.c0`，Fp2 使用 `x * y`。
原证明、输入/预期结果、编译参数、锁文件和两个 Manifest 的哈希均与 B5 对照。
完整返回值同时匹配直接 BigInt 模运算与 Noble 参考实现。

| 指标 | Fp：B5 → B6 | Fp2：B5 → B6 |
|---|---:|---:|
| 算术窗口 Gas | 4,372 → 2,167 | 16,468 → 9,965 |
| 窗口降幅 | 50.43% | 39.49% |
| 交易总 Gas | 29,813 → 27,606 | 45,399 → 38,887 |
| execution Gas | 6,525 → 4,318 | 20,447 → 13,935 |
| MODEXP 次数 | 2 → 2 | 6 → 6 |
| MODEXP 子调用 Gas | 400 → 400 | 1,200 → 1,200 |
| 窗口内存扩张 Gas | 211 → 37 | 899 → 295 |
| 窗口分配指针增长 | 2,112B → 384B | 7,872B → 2,816B |
| 退出活动 EVM 内存 | 93 → 39 words | 305 → 147 words |

新版本部署总 Gas 为 597,983，runtime 2,519B（B5 为 565,535 Gas / 2,369B）：
运行成本下降，但代码及部署成本有所增加，不能只报告收益。
两项成本调用均未触发 calldata floor。Yul 工作区的 320B 不等于整个编译窗口的
实际分配量；表格采用真实分配指针差值，不把它等同 EVM 峰值内存或主机 RSS。

费用对账：

- Fp：顶层 opcode 3,918 + MODEXP 400 = execution 4,318；加 intrinsic 23,288 = 总 27,606。
- Fp2：顶层 opcode 12,735 + MODEXP 1,200 = execution 13,935；加 intrinsic 24,952 = 总 38,887。
- 首尾 GAS 标记与合约返回窗口一致；窗口内存字数对应 37 / 295 扩张 Gas。

窗口仍包含编译调用及测量开销，不是内联到 Miller 后不变的操作常数。
STATICCALL 与内存费用不是互斥归因；扣除 MODEXP 的剩余费用不能全归于某一种开销。
每项只有一个固定公开性能输入，没有统计时间或端到端效率结论。

## 四个必要边界：4/4 通过

令 `B = 2^256`，`p` 为同一 BLS12-381 基础域模数：

| 输入 `(a,b)` | 本次检查重点 |
|---|---|
| `(0,p−1)` | 零积及负差路径 |
| `(p−1,p−1)` | 和的模约减、相等差与模数附近 |
| `(1,(3p+2B−1)/4)` | 两次模除二均为奇数且加 p 均产生低 limb 进位 |
| `(B+1,B−1)` | 低 limb 加法进位及非负差借位 |

四笔 EVM 调用均返回完整正确乘积，每笔均执行两次成功 MODEXP。
这些边界不计入上表性能对照；没有增加随机测试、故障矩阵或规模测试。
运行器的分支条件来自输入的 BigInt 推导，用于说明所选边界，
不是内部指令覆盖跟踪，更不是对所有域元素的形式化正确性证明。
独立源码审查检查了进借位、先加 p 再右移及临时区的返回值生命周期。
执行后的独立只读核对也确认六笔算术调用费用对账、完整输出、11 项来源/锁文件哈希
及范围标记一致，没有额外运行 EVM 实验。

## 复现与产物

```bash
npm run --prefix contracts g5c3b6
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

- 合约：`contracts/src/Bls12381FieldLowLevelProfile.sol`。
- 运行器：`contracts/scripts/g5c3_field_lowlevel.mjs`。
- 原始结果：`experiments/g5_c3_raw/field_lowlevel_profile.json`。
- 历史基准：`experiments/g5_c3_raw/field_cost_profile.json`，未覆盖。

环境：Apple M4 / arm64，Node 23.11.0，EthereumJS 10.1.3 / Prague，
solc 0.8.30、optimizer 200、无 viaIR。一次部署、六笔本地签名调用，无 RPC。
Git 基准 `7442414a04e8d06d79d968b3cbf448a6a694347a`，`git_dirty=true`；
实际代码通过以下 SHA-256 及原始记录中的依赖/Manifest 哈希固定：

| 产物 | SHA-256 |
|---|---|
| 新合约 | `9b2c03c91f3dbab325be969f675a62e9ba0b8a0e4f214f90ab2420644ca29786` |
| 新运行器 | `10f66b2b27863544576eed02831c473f68158bc9cb313d336b3033094fb0e495` |
| 本次原始结果 | `4753d634e56d59986817e61846bb6c9d6252ed60477eb225c08cc2d95b95d318` |

没有改曲线、编码、SRS、Manifest、旧实现或论文主张。
完整验证状态保留 `BLOCKED_SOFTWARE_MILLER_LOOP_AFTER_SPARSE_OPT_EXCEEDS_20M`，
完整 SnarkPack 验证 Gas 仍为 `null`。在 B6 完成时，B4 是最新实际完整循环尝试，
B6 不替代该记录；后续完整尝试见本页末尾的 B7 链接。

## 下一步仅做 B7：集成后再验收

1. 新增独立 sparse-Miller 版本，仅集成此次相同 `_mul`；保留 B4 和 B6 的代码/数据。
2. 使用同一公开证明和前两步 prefix，核对全部系数及一个错误目标拒绝，并测量真实 Gas。
3. 上述通过后，仅尝试一次同样 20M 上限的完整循环。失败则保留具体原因和 BLOCKED；
   成功也仅表示 Miller 部分通过，不能称为完整 pairing 或 SnarkPack 验证。

本轮不执行这一步，不提高预算、不提前做最终幂、不替换套件、不推算完整验证 Gas。

后续记录：B7 已完成集成验收和一次原预算完整尝试，见
[G5_C3_MILLER_LOWLEVEL.md](G5_C3_MILLER_LOWLEVEL.md)。前缀成本下降，但完整循环仍 OOG；
本页保留 B6 当时结果，当前下一步以 B7 报告的 B8 隔离对照为准。
