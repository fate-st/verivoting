# G5-C3-B11：低分配 Fp2 加法对照通过

2026-10-26。本轮新增一个隔离 `_add2` 候选，只运行原公开成本输入及一个必要边界。
完整输出均正确，窗口 Gas、execution Gas 和实际分配增长均下降。
交易总 Gas 仍为 30,880，不能将本轮收益写成这笔交易费用下降。

状态：`PASS_LOW_ALLOCATION_FP2_ADD_FIXED_CASES_ONLY`，仅表示原语对照通过。
没有集成或重跑 Miller，没有完成新的完整验证。

## 实际对照

沿用 B10 的同一公开 `final_b` 仿射 x/y、同一加法测量入口与 calldata。
完整 128B 返回值同时与直接 BigInt 模加法和 Noble 参考结果比较。

| 项目 | B10 原实现 | B11 候选 |
|---|---:|---:|
| 算术窗口 Gas | 1,752 | 849 |
| execution Gas | 5,740 | 4,852 |
| 窗口分配指针增长 | 896B | 384B |
| 窗口内存扩张 Gas | 91 | 39 |
| 退出活动 EVM 内存 | 87 words | 71 words |
| intrinsic Gas | 24,952 | 24,952 |
| 交易总 Gas | 30,880 | 30,880 |
| calldata floor 补差 | 188 | 1,076 |
| MODEXP 次数 / Gas | 0 / 0 | 0 / 0 |

窗口节省 903 Gas（51.54%），execution 节省 888 Gas，分配增长减少 57.14%。
窗口外 execution 从 3,988 变为 4,003；不能把窗口下降当作整个调用的同额下降。
两版本都触发了本地 Prague calldata floor：

```text
B10: 24,952 + 5,740 = 30,692; 补 188  至 30,880
B11: 24,952 + 4,852 = 29,804; 补 1,076 至 30,880
```

补差不属于算术或内存费用。新版本顶层 opcode 合计 4,852，与 execution 一致；
两次 GAS 标记差为 849，实际内存扩张费用为 39，退款为零。
这是真实的执行成本下降，但不是本次交易总费用下降，也不是 Miller/端到端效率结论。

部署总 Gas 为 614,832、runtime 2,597B；B10 为 678,411 / 2,891B。
每版只有一个固定公开性能输入，没有统计时间或规模性能结论。

## 唯一实现变化

新增 `contracts/src/Bls12381Fp2AddLowLevelProfile.sol`，整文件严格等于 B10
仅替换合约名与 `_add2` 完整函数。其余 17 个顶层函数，包括三个测量入口、
`_mul`、`_mul2`、`_add` 和编解码，全部逐字不变。

新 `_add2` 在局部 Yul 中直接读取规范输入，处理低 limb 进位、`>=p` 单次约减和借位。
显式申请 192B，包含 Fp2 指针壳及两个独占 Fp 结果，完整初始化六个 word；
不修改 a/b、不回退分配指针，不使用宿主值替代计算。输入即使同指针也只被读取，
不会被新输出覆盖。模数、逐坐标模加法公式及编码未变。

显式 192B 不是整个编译窗口的分配增长；实测是 384B，应以分配指针差值报告。
该数值也不是 EVM 峰值内存或主机 RSS。窗口包含编译调用及测量开销，
不是嵌入 Miller 后的固定操作成本。

## 一个必要边界通过

原公开输入已包含普通加法及带进位/约减借位的分量。本轮只额外运行：
`a=(p−1,0), b=(1,0)`，预期结果 `(0,0)`。
它检查“和恰等 p 必须归零”及 `0+0`，完整输出正确；不计入性能样本。
边界交易总 Gas 为 28,120，execution 为 4,884，无 floor 补差，MODEXP 为零。

不增加随机、别名专用入口、故障或规模矩阵。独立只读审查核对了进借位、
新写区与输入区不交叠、整份源码变化范围以及费用记录。执行后复核确认 16 项
来源文件哈希、两个完整结果和费用/内存对账一致，未重复执行 EVM。
编译、Node 语法及 diff 检查通过。

## 产物与复现

```bash
npm run --prefix contracts g5c3b11
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

- 合约：`contracts/src/Bls12381Fp2AddLowLevelProfile.sol`。
- 运行器：`contracts/scripts/g5c3_fp2_add_lowlevel.mjs`。
- 原始结果：`experiments/g5_c3_raw/fp2_add_lowlevel_profile.json`。
- 原实现基准：`fp2_add_cost_profile.json`，未覆盖。

环境：Apple M4 / arm64，Node 23.11.0，EthereumJS 10.1.3 / Prague，
solc 0.8.30、optimizer 200、无 viaIR；本地签名交易，无 RPC。
Git 基准 `7442414a04e8d06d79d968b3cbf448a6a694347a`，`git_dirty=true`；
不是 clean-commit 论文性能样本。

| 产物 | SHA-256 |
|---|---|
| 新合约 | `475182d503caa8e57400cfcf82941b8d9cb76742e1a00294b0f2a0446f7612a2` |
| 新运行器 | `0f48e67f3cb6460f0b41c5f65c1d3a4804669bc1f47359e760b0753d62499c10` |
| 原始结果 | `92c9f44ce56fdf9ed5e303a347b9594caff0c5ffd46bba64107be5def1e9bd0a` |

旧实现、结果、向量、锁文件、曲线、SRS、Manifest 和论文主张均未更改。
没有乘法、前缀、完整循环、setup/prove 或规模重跑。
最新集成前缀仍为 B9；最新完整尝试仍是 B7 旧版 20M OOG。
B9 新版完整循环未验收，完整验证保持 BLOCKED，聚合验证 Gas 为 `null`。

## 下一步只做 B12：先验收集成前缀

1. 新增独立 B9-based Miller 版本，精确集成本轮 `_add2`，其余源码保持原样。
2. 仅执行原两步前缀正例及一个错误目标，核对完整输出和真实 Gas/内存。
3. 依据集成后的实际结果再决定后续，不从本轮 51.54% 推算完整循环收益，
   不自动追加完整循环尝试。

本轮不实施 B12，不提高上限、不提前最终幂、不改变套件或 Manifest。

## 后续记录：B12 前缀集成已完成

B12 已将本页的同一 `_add2` 集成到独立 B9-based Miller 合约，并只运行原两步
前缀正例及一个错误目标。正例 Gas 从 1,132,763 降至 1,043,835（7.85%），
活动内存从 2,411 降至 2,079 words（13.77%）；错误目标正确拒绝，516 次
MODEXP 保持不变。详见
[G5_C3_MILLER_ADD_PREFIX.md](G5_C3_MILLER_ADD_PREFIX.md)。

B12 的 `full_loop_attempts=0`、`full_attempt=null`；最新完整尝试仍是 B7 旧版
20M OOG，不能把它归因于 B12，也不能由前缀收益推断完整循环 Gas。下一步 B13
仅测原 `_sub2` 的一个固定公开输入成本，不在同一门同时优化、重跑 Miller 或进入
最终幂。
