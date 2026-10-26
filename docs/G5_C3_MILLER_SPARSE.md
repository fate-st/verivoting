# G5-C3-B4：专用平方与014稀疏线乘法

已实现并实际运行两种专用运算。**同一两步前缀 Gas 较B3减少21.35%，MODEXP调用
从642次降到516次；正例通过，错误结果被拒绝。完整64步仍在20M预算下 OOG。**
状态为 `PARTIAL_PASS_SPARSE_MILLER_PREFIX_FULL_LOOP_OOG_20M`，完整配对验证仍未完成。

## 实验范围与真实对照

复用同一公开B=2证明终点与 `snarkpack_g5c3_miller_loop_v1.json`，不重新生成fixture。
B2、B3的代码、运行器及结果保留原样。相同solc0.8.30、optimizer200、Prague、无viaIR，
同一20M交易上限、30M区块上限；只增加一次部署、同一两步正反例及一次完整循环尝试。

| 项目 | B2通用版本 | B3内存优化 | B4专用算术 |
|---|---:|---:|---:|
| 两步正例总Gas | 4,805,196 | 2,371,368 | 1,865,022 |
| 两步MODEXP次数 | 642 | 642 | 516 |
| 两步活动内存words | 35,464 | 7,123 | 5,233 |
| 错误结果 | 拒绝 | 拒绝 | 拒绝，1,864,840总Gas |
| 完整64步 | 20M上限OOG | 20M上限OOG | 20M上限OOG |

新版本正例execution Gas为1,828,330，calldata仍为1,316B；516次MODEXP全部成功，
预编译execution Gas合计103,200。部署总Gas为1,439,661，runtime6,412B。
完整尝试退出时活动内存为5,315words，已执行5,556次成功MODEXP，根异常为 `out of gas`。

上述Gas降幅只针对同一组件、同一输入的一次对照，不是论文端到端效率结论。
三个完整尝试都没有完成，因此20M仅是各自失败交易的预算花费；不能当完整Miller成本。
失败前MODEXP次数也不能直接换算完成步数或比较完整循环总成本。

## 具体修改及核对

新合约为 `contracts/src/Bls12381MillerLoopSparse.sol`，沿用B3的常驻对象深拷贝和临时内存复用。
底层Fp乘法、点倍加、NAF、输入输出编码和群参数不改，只替换：

1. **任意Fp12元素的专用平方**：用2次Fp6乘法代替3次。
   每次平方MODEXP由108降为72；不是要求GT/圆分子群成员的特殊平方。
2. **014稀疏线乘法**：利用Fp6的`mul01`（5次Fp2乘法）、`mul1`（3次）及组合式，
   共13次Fp2乘法，加上点坐标缩放的4次Fp乘法。每条线MODEXP由116降为86。

同一两步前缀有一次平方、三条线，故预计调用数为
`642 - (108-72) - 3*(116-86) = 516`；实际执行计数确实为516。
全部点更新、线和域运算在EVM内部完成，宿主值仅作为待比较结果，不作为可信中间状态。

实现对照固定Noble2.4.0的本地源码
`contracts/node_modules/@noble/curves/abstract/tower.js`及
[`abstract/bls.js`对应源码](https://github.com/paulmillr/noble-curves/blob/2.4.0/src/abstract/bls.ts)，
MIT许可保留，源码哈希记录在原始结果中。
参考端在同一实际轨迹上将新增公式与原通用运算比较：63次平方、69次线乘全部一致，
最终参考输出匹配冻结向量。这是宿主参考运算检查，不代替完整64步EVM执行证据。

## 产物与复现

- 新运行器：`contracts/scripts/g5c3_miller_sparse.mjs`。
- 原始结果：`experiments/g5_c3_raw/miller_sparse_evm.json`。
- 状态与汇总：`docs/IMPLEMENTATION_STATUS.md`、`docs/NEXT_STEPS.md`和`experiments/aggregate/summary.*`。

```bash
npm run --prefix contracts g5c3b4
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

真实Solidity编译、Node语法及diff检查通过。未重跑全部历史测试或Rust高内存电路。
环境：Apple M4、Node23.11.0、EthereumJS10.1.3、本地签名交易，无RPC。
Git基准`7442414a04e8d06d79d968b3cbf448a6a694347a`，`git_dirty=true`；
不是clean-commit论文性能样本。Manifest、Cargo.lock、曲线、SRS和旧向量均未改变；
没有setup/prove、规模或故障矩阵实验。

## 下一步只做 G5-C3-B5：底层Fp/Fp2最小成本定位

本轮专用算术有实际收益，但仍不足以完成64步。下一步先定位底层成本，不继续盲目整段重试：

1. 取现有公开终点衍生的规范系数，分别测一次Fp乘法和一次Fp2乘法，记录总Gas、MODEXP成本与内存。
2. 保持同一底层算法，区分预编译本身与Solidity编码/临时对象/调用开销，再决定低层优化方向。
   103,200预编译Gas不能解释前缀全部1,865,022Gas，但本轮没有完成精细归因，不预先指定唯一瓶颈。
3. 此阶段不再运行完整64步。只有底层改进获得必要正确性及真实成本证据后，才考虑同20M预算复测。

不提高Gas上限、不换曲线、不加wrapper证明、不提前进入最终幂。完整Miller、最终幂、GT目标比较
和SnarkPack验证仍保持BLOCKED；完整聚合验证Gas保持`null`。

后续B5已完成成本定位，见 [Fp/Fp2成本报告](G5_C3_FIELD_COST_PROFILE.md)：窗口Gas为
4,372/16,468，MODEXP部分为400/1,200。没有重新运行完整循环；下一步先做底层实现对照。
