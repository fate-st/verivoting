# G5-C3-B8：Fp2 临时分配优化对照通过

2026-10-26。本轮只做一个隔离改动：减少 `_mul2` 求和、模减时的临时对象。
真实执行一次部署、原有两个成本输入及三个必要 Fp2 边界，完整输出全部正确。
Fp2 测量窗口从 9,965 降至 8,086 Gas（18.86%）；没有运行 Miller 或证明生成。

状态：`PASS_FP2_FIXED_CASES_AND_COST_REDUCTION_ONLY`，仅表示原语对照通过。
最新完整 Miller 尝试仍是 B7 的 20M OOG，完整验证能力不升级。

## 实现范围

新增 `contracts/src/Bls12381Fp2LowLevelProfile.sol`，除合约名和 `_mul2` 外，
整份源码与 B6 逐字一致。特别是基础域 `_mul`、规范编码和测量入口完全保留。
仍使用三次基础域乘法的同一 Karatsuba 公式：

```text
t0 = a0*b0; t1 = a1*b1; t2 = (a0+a1)*(b0+b1)
c0 = t0-t1; c1 = t2-t0-t1             (mod p)
```

两个规范和使用独占 128B 缓冲区；局部 Yul 处理进位、一次减 p 和原地模减。
三个 `_mul` 结果彼此独占。先在 t2 完成 c1，再覆盖 t0 为 c0，最后分配 Fp2 指针壳。
不修改输入、不回退分配指针，不把宿主计算结果当作合约算术。
三次 `_mul` 仍执行六次真实 MODEXP，未换公式、曲线、模数或协议套件。

## 真实成本对照

使用原 B6 同一公开证明的 `final_b` 仿射 x/y；Fp 控制项取 `x.c0 * y.c0`，
Fp2 取完整 `x * y`。输入、预期输出、编译参数、锁文件和 Manifest 均固定。
返回的全部字节同时与直接 BigInt 模运算及 Noble 参考结果比较。

| 指标 | Fp 控制：B6 → B8 | Fp2：B6 → B8 |
|---|---:|---:|
| 算术窗口 Gas | 2,167 → 2,167 | 9,965 → 8,086 |
| 窗口降幅 | 0% | 18.86% |
| 交易总 Gas | 27,606 → 27,614 | 38,887 → 37,022 |
| execution Gas | 4,318 → 4,326 | 13,935 → 12,070 |
| MODEXP 次数 | 2 → 2 | 6 → 6 |
| MODEXP execution Gas | 400 → 400 | 1,200 → 1,200 |
| 窗口分配指针增长 | 384B → 384B | 2,816B → 1,664B |
| 窗口内存扩张 Gas | 37 → 37 | 295 → 171 |
| 退出活动 EVM 内存 | 39 → 39 words | 147 → 111 words |

Fp2 窗口节省 1,879 Gas，分配指针增长减少 40.91%。Fp 控制的窗口及分配量不变，
但交易总 Gas 增加 8；变化发生在测量窗口之外，不能声称所有调用都变快。
Fp2 总交易节省 1,865 Gas，不应与窗口节省混淆。

费用对账：

- Fp：顶层 opcode 3,926 + MODEXP 400 = execution 4,326；加 intrinsic 23,288 = 总 27,614。
- Fp2：顶层 opcode 10,870 + MODEXP 1,200 = execution 12,070；加 intrinsic 24,952 = 总 37,022。
- 首尾 GAS 标记与合约返回窗口一致；两项均未触发 calldata floor，退款为零。

部署总 Gas 为 564,461，runtime 2,364B（B6 为 597,983 / 2,519B）。
窗口包含编译调用及测量开销，不是内联到 Miller 后的固定操作成本。
分配指针差也不是 EVM 峰值内存或主机 RSS；STATICCALL 与内存费用不能简单重复归因。
每种操作仅一个固定公开成本输入，没有统计时间或端到端性能结论。

## 三个必要边界：3/3 通过

令 `B=2^256`、`p` 为原 BLS12-381 基础域模数，Fp2 按 `(c0,c1)` 表示：

| a、b | 检查重点 |
|---|---|
| `a=b=(0,1)` | 纯虚平方得到 `(-1,0)`，负差修正及零分量 |
| `a=b=(p−1,p−1)` | 两个和均需减 p；c0 为零；先算 c1 再覆盖 t0 的依赖 |
| `a=(p−1,B−1), b=(B+1,p−2)` | 求和低 limb 进位、约减借位、正差跨 limb 借位及 c1 两次负差 |

三个 EVM 调用各执行六次成功 MODEXP，完整 128B 输出均正确。
运行器记录的分支条件由输入推导，减 p 借位使用约减前 raw sum 的低 limb；
这些条件不是内部指令覆盖或穷尽正确性证明。边界不计入性能样本。
不再重跑 B6 的四个 Fp 边界，也不增加随机、故障或规模矩阵。
独立源码审查确认规范求和、原地模减、c1/c0 覆写顺序及返回指针安全。

## 产物与复现

```bash
npm run --prefix contracts g5c3b8
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

- 合约：`contracts/src/Bls12381Fp2LowLevelProfile.sol`。
- 运行器：`contracts/scripts/g5c3_fp2_lowlevel.mjs`。
- 原始结果：`experiments/g5_c3_raw/fp2_lowlevel_profile.json`。
- 成本基准：`field_lowlevel_profile.json`；最新完整循环记录：`miller_lowlevel_evm.json`。

环境：Apple M4 / arm64、Node 23.11.0、EthereumJS 10.1.3 / Prague，
solc 0.8.30、optimizer 200、无 viaIR；本地签名交易，无 RPC。
Git 基准 `7442414a04e8d06d79d968b3cbf448a6a694347a`，`git_dirty=true`；
不是 clean-commit 论文性能样本，真实源码由哈希固定：

| 产物 | SHA-256 |
|---|---|
| 新合约 | `f89380291847399f78d73e942fee09cfe4644d47041ccd64461d0d9203545583` |
| 新运行器 | `1f5e910440e13c6155282005cafc86dcae9f2dbe6949e57b7fe22a56393e0fbd` |
| 原始结果 | `da0a6838302d08cb09fa3075e257e2c678f75ffc2e574c543acf1d7438e438b8` |

旧代码、结果和向量均保留；没有改 Cargo.lock、依赖锁文件、Manifest、SRS 或论文主张。
没有运行 setup/prove、Miller 或规模实验。完整验证仍为
`BLOCKED_SOFTWARE_MILLER_LOOP_AFTER_LOW_LEVEL_MUL_EXCEEDS_20M`，聚合验证 Gas 为 `null`。

## 下一步只做 B9：先验收集成前缀

1. 新增独立 Miller 变体，在 B7 中精确集成此次 `_mul2`，其他函数保持原样。
2. 只先执行同一两步前缀正例及一个错误目标，比较完整输出与真实 Gas/内存。
3. 依据集成后的实测收益，再决定是否值得一次相同 20M 上限的完整尝试；
   不从隔离原语降幅推算完整循环成本，也不自动追加重试。

本轮不实施 B9，不提高预算、不提前做最终幂、不改变协议套件。

后续记录：B9 集成前缀已通过，见
[G5_C3_MILLER_FP2_PREFIX.md](G5_C3_MILLER_FP2_PREFIX.md)。本页保留 B8 当时数据；
B9 未运行完整循环，下一步仅做 B10 的现有 Fp2 加法成本定位。
