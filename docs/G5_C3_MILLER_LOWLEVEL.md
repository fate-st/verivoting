# G5-C3-B7：低层乘法集成有收益，完整循环仍超出 20M

2026-10-26。已将 B6 的同一 `_mul` 集成到独立 sparse-Miller 版本，并实际执行：
一次部署、同一两步前缀正例、一个错误目标、一次完整 64 步尝试。
前缀完整结果正确，错误目标被拒绝；前缀 Gas 较 B4 下降 30.80%。
但唯一完整尝试仍以根级 `out of gas` 失败。本轮没有提高上限或追加重试。

状态：`PARTIAL_PASS_LOW_LEVEL_MILLER_PREFIX_FULL_LOOP_OOG_20M`。
这表示集成及前缀部分验收完成，不表示完整 Miller、pairing 或 SnarkPack 验证完成。

## 实际对照

使用同一 B=2 公开证明、同一 `snarkpack_g5c3_miller_loop_v1.json` 和编译参数。
三笔调用的 calldata SHA-256 与 B4 分别相同；没有重新生成证明或向量。

| 指标 | B4：专用平方/014 | B7：再集成低层乘法 |
|---|---:|---:|
| 两步正例总 Gas | 1,865,022 | 1,290,640 |
| 两步 execution Gas | 1,828,330 | 1,253,948 |
| 两步活动 EVM 内存 | 5,233 words | 2,829 words |
| 两步 MODEXP 成功次数 | 516 | 516 |
| 两步 MODEXP execution Gas | 103,200 | 103,200 |
| 错误目标 | 拒绝 | 拒绝，1,290,458 总 Gas |
| 完整循环结果 | 20M 上限 OOG | 20M 上限 OOG |

前缀节省 574,382 Gas（30.80%），内存字数下降 45.94%。前缀返回的全部 768B
与冻结参考值一致；负例只改变 expected 的第一个规范 Fp 系数，返回
`INVALID_MILLER_RESULT`。两项前缀均无 MODEXP 失败。

完整尝试花费 20,000,000 Gas，execution 为 19,963,388，根异常为 `out of gas`，
不是把其他 revert 归类为资源不足。失败前 7,848 次 MODEXP 成功，退出活动内存为
3,147 words；B4 对应为 5,556 次 / 5,315 words。调用数只能说明失败前的已执行运算，
不能换算已完成步数、完成百分比或完整循环总成本。

`full_miller_verification_gas` 和完整 SnarkPack 验证 Gas 都为 `null`。
20M 是失败交易的预算花费，不是完成 Miller 所需的测量值，也不构成不可能性结论。

新版本部署总 Gas 1,504,337，runtime 6,711B；B4 为 1,439,661 / 6,412B。
运行收益伴随代码和部署成本增加。上述均为同一公开输入的组件对照，
不是统计性能、端到端选举效率或论文规模实验。

## 改动与验收范围

新增 `contracts/src/Bls12381MillerLoopLowLevel.sol`。运行器核对整份源码等于：

1. B4 的合约名换成新名字；
2. B4 的 `_mul` 精确换成已通过 B6 的完整 `_mul`；
3. 其余所有源码保持逐字一致。

共 52 个函数，其中 51 个保持 B4 原样。域模数、编码、点倍加、NAF、一般 Fp12 平方、
014 线乘和内存 arena 均未变。独立只读审查确认 `_mul` 返回区与 arena 兼容：
临时结果在 rewind 前已深拷贝到持久对象，无 scratch 指针跨边界保存。
全部点更新、线与域运算仍由合约计算；参考结果只用来比较输出，不提供可信中间状态。

两步前缀已覆盖点倍加、第一次负号 mixed addition、平方、三条线及内存复用。
因此本轮只保留这组原正反例及一次完整尝试，不重跑 B6 边界、Rust 高内存电路、
全部历史测试、setup/prove、随机/故障矩阵或规模实验。
编译通过、Node 语法检查和 diff 检查通过。
执行后的独立只读复核确认了 16 项来源文件哈希、完整前缀输出、calldata、
费用与 MODEXP trace 摘要；完整尝试确为根级 OOG，范围标记一致。复核未重跑 EVM。

## 产物与复现

```bash
npm run --prefix contracts g5c3b7
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

- 合约：`contracts/src/Bls12381MillerLoopLowLevel.sol`。
- 运行器：`contracts/scripts/g5c3_miller_lowlevel.mjs`。
- 原始结果：`experiments/g5_c3_raw/miller_lowlevel_evm.json`。
- 保留基准：`miller_sparse_evm.json`（B4）、`field_lowlevel_profile.json`（B6）。

环境为 Apple M4 / arm64、Node 23.11.0、EthereumJS 10.1.3 / Prague，
solc 0.8.30、optimizer 200、无 viaIR；本地签名交易，无 RPC。
交易上限 20M、区块上限 30M，未改变。内存复用依赖固定编译配置，
不是可任意升级编译器的通用库。
Git 基准 `7442414a04e8d06d79d968b3cbf448a6a694347a`，`git_dirty=true`，
不是 clean-commit 论文性能样本；实际源码与依赖通过原始记录中的哈希固定。

| 产物 | SHA-256 |
|---|---|
| 新合约 | `facd572ff1f4f21fe3024c03ba59ed8f4850357687953c6d4921c24ae3e270af` |
| 新运行器 | `9ff7ba64ae70a03862adc27c97fea09d6e081f3e7a5c50a2095a81bb721bc428` |
| 原始结果 | `cdf06a719f4df41a098ce1a2ce7a4b5e4a8e2e4eb61479b044aa717da9caa7e6` |

B2–B6 旧代码/结果未覆盖，Cargo.lock、依赖锁文件、曲线、SRS、Manifest 和论文主张未改。
完整验证状态为 `BLOCKED_SOFTWARE_MILLER_LOOP_AFTER_LOW_LEVEL_MUL_EXCEEDS_20M`。
最终幂、GT 目标绑定和完整 SnarkPack 合约仍未完成，Manifest 不升级能力。

## 下一步只做 B8：一个 Fp2 临时分配路径对照

1. 保留 B6 原语和 B7 集成结果，优先针对 `_mul2` 的临时分配/规范加减路径，
   新增一个隔离实现，保持三次 Fp 乘法恒等式、基础域 `_mul`、曲线和编码不变。
2. 先用原有同一 Fp2 公开输入比较完整输出、窗口 Gas、内存和 MODEXP 数；
   仅补改动直接涉及的必要边界，不增加测试矩阵。
3. 有真实正确性和成本收益后再决定集成；本轮不实现 B8，也不再运行完整循环。

不提高预算、不提前最终幂、不换协议套件、不从前缀推算完整验证 Gas。

后续记录：B8 已完成单一 `_mul2` 临时分配对照，见
[G5_C3_FP2_LOWLEVEL.md](G5_C3_FP2_LOWLEVEL.md)。本页保留 B7 当时测量；
B8 没有运行 Miller，下一步先验收 B9 集成前缀，不自动追加完整循环尝试。
