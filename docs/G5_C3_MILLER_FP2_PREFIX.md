# G5-C3-B9：Fp2 优化集成前缀通过

2026-10-26。将 B8 的同一 `_mul2` 集成到独立 Miller 版本，真实执行一次部署和原有
两步前缀正反例。完整前缀输出正确，错误目标拒绝；Gas 较 B7 下降 12.23%。
本轮没有构建或执行完整循环交易，也没有新增测试矩阵。

状态：`PASS_FP2_MILLER_PREFIX_INTEGRATION_ONLY`，仅表示集成前缀通过。
最新完整尝试仍是 B7 旧版本的 20M OOG；B9 新版尚未做完整循环验收，不能称为新版又 OOG。

## 真实测量

同一公开 B=2 证明、同一冻结两步向量、同一编译设置和 20M 交易上限。
正反例 calldata SHA-256 分别与 B7 完全相同；负例只改变预期结果的第一个规范 Fp 系数。

| 指标 | B7 | B9 |
|---|---:|---:|
| 两步正例总 Gas | 1,290,640 | 1,132,763 |
| 正例 execution Gas | 1,253,948 | 1,096,071 |
| 正例 intrinsic Gas | 36,692 | 36,692 |
| 两步活动 EVM 内存 | 2,829 words | 2,411 words |
| MODEXP 成功次数 | 516 | 516 |
| MODEXP execution Gas | 103,200 | 103,200 |
| 错误目标总 Gas | 1,290,458 | 1,132,581 |
| 本轮完整循环尝试数 | B7 当时为 1 | B9 为 0 |

前缀节省 157,877 Gas（12.23%），活动内存字数减少 14.78%。
两笔调用各完成 516 次 MODEXP，无失败；正例返回的全部 768B 与冻结参考值相同，
负例返回 `INVALID_MILLER_RESULT`。这同时检验了新 `_mul2` 的输入别名平方、
上层点更新、线乘及深拷贝后内存回退的集成使用。

费用对账：正例 `1,096,071 + 36,692 = 1,132,763`，
负例 `1,095,877 + 36,704 = 1,132,581`；退款为零，未受 calldata floor 影响。
原始结果保留返回字节、calldata 哈希、MODEXP trace 摘要和内存记录。

新版本部署总 Gas 1,562,736，runtime 6,981B；B7 为 1,504,337 / 6,711B。
因此前缀执行成本下降，但部署成本和代码体积增加，并非所有成本都改善。
这是同一组件、同一固定公开输入的对照，不是统计时间、完整选举性能或论文规模实验。

## 具体改动与必要验收

新增 `contracts/src/Bls12381MillerLoopFp2LowLevel.sol`。运行器严格核对整份源码：
仅把 B7 的合约名改成新名字，且 `_mul2` 完整替换为 B8 已通过对照的函数。
其余源码逐字不变，包括 `_mul`、点倍加、NAF、专用平方/014 线乘、编码和 arena。

新 `_mul2` 只原地更新本次新分配的 t0/t2，不覆盖 a/b；结果在 operation 结束后
按原实现深拷贝到持久对象，再回退 arena。独立只读审查确认该生命周期兼容。
原两步前缀已覆盖平方、两次点倍加、首次负号加点、三条线及内存复用，
本轮不重跑 B8 三个 Fp2 边界、随机矩阵或全部历史测试。

真实 Solidity 编译、Node 语法和 diff 检查通过。执行后独立只读复核确认 15 项
来源文件哈希、完整输出、费用、计数及范围标记一致，没有重复执行 EVM。
合约计算全部点更新和域运算，宿主值仅用于输出比较，不提供可信中间状态。

## 完整循环状态必须单独看

本次原始记录明确：`full_loop_attempts=0`、`full_attempt=null`，
`full_miller_loop_verified_in_evm=false`，完整 Miller 和 SnarkPack 验证 Gas 都为 `null`。
`target_verifier_status_source` 指向 B7 的 `miller_lowlevel_evm.json`，该历史记录没有覆盖。

B7 的实际 OOG 不等于证明 B9 新版仍超预算；B9 前缀收益也不等于证明新版已能通过。
本轮决定不追加完整尝试，先定位一个尚未单独测量的路径，不从前缀外推完整 Gas。
最终幂、GT 目标绑定及完整 SnarkPack 合约仍未完成，Manifest 能力不升级。

## 产物与复现

```bash
npm run --prefix contracts g5c3b9
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

- 合约：`contracts/src/Bls12381MillerLoopFp2LowLevel.sol`。
- 运行器：`contracts/scripts/g5c3_miller_fp2_prefix.mjs`。
- 本次结果：`experiments/g5_c3_raw/miller_fp2_prefix_evm.json`。
- 原集成基准：`miller_lowlevel_evm.json`；原语基准：`fp2_lowlevel_profile.json`。

环境：Apple M4 / arm64，Node 23.11.0，EthereumJS 10.1.3 / Prague，
solc 0.8.30、optimizer 200、无 viaIR；本地签名交易，无 RPC。
Git 基准 `7442414a04e8d06d79d968b3cbf448a6a694347a`，`git_dirty=true`，
不是 clean-commit 论文性能样本。内存复用依赖固定编译配置，不作为任意编译器通用库。

| 产物 | SHA-256 |
|---|---|
| 新合约 | `5196fa33d5523b1fe4767a410e23250471434258add0daaceeb8d12f13a9fadc` |
| 新运行器 | `62a7ae875e36a2c58db3440431b6ea041e5e77b759fc944fa7da642a77669f33` |
| 原始结果 | `e9228b36212a5b7685abd5cb962edefc2b234b4c539676bb9795a118706becf4` |

旧实现、结果、向量、锁文件、曲线、SRS、Manifest 和论文主张均未更改。
没有 setup/prove、规模实验、最终幂或完整循环重试。

## 下一步只做 B10：Fp2 加法成本定位

1. 复用原公开操作数，给现有 `_add2` 增加隔离测量入口；先保持加法实现不变。
2. 只测一个固定 Fp2 加法输入，核对完整结果、测量窗口、分配量和实际 MODEXP 次数
   （加法预期为零）。不重跑已有乘法、前缀或完整循环。
3. 根据真实成本再判断是否值得优化，不预先认定它是唯一瓶颈；若收益空间有限则再审视路线。

本轮不实施 B10，不提高预算、不换协议套件、不提前进入最终幂。

后续记录：B10 已完成原 `_add2` 的单次成本测量，见
[G5_C3_FP2_ADD_COST_PROFILE.md](G5_C3_FP2_ADD_COST_PROFILE.md)。本页保留 B9 数据；
B10 没有重跑 Miller，下一步仅作 B11 隔离低分配候选对照。
