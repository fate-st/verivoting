# G5-C3-B3：Miller 内存优化实测

本轮只优化内存生命周期，不改点/线/域公式。**同一两步前缀 Gas 减少50.65%，
活动 EVM 内存字数减少79.91%；正例通过，错误结果被拒绝。**
完整64步在原20M交易预算下仍 OOG，状态为
`PARTIAL_PASS_MILLER_MEMORY_REDUCED_FULL_LOOP_OOG_20M`，不能宣称完整 Miller 或配对验证通过。

## 相同条件下的真实结果

旧 B2 合约、运行器、fixture及原始结果均保留不变。新版本复用同一实际 B=2证明终点、
同一前缀正反例、同一 solc0.8.30 / optimizer200 / Prague / 无viaIR，以及相同20M预算。
新增一次部署、两次前缀调用及**一次完整循环尝试**，没有扩大测试矩阵。

| 项目 | B2 未优化基准 | B3 仅内存优化 |
|---|---:|---:|
| 两步正例总 Gas | 4,805,196 | 2,371,368 |
| 两步正例 execution Gas | 4,768,504 | 2,334,676 |
| 两步活动 EVM 内存字数 | 35,464 | 7,123 |
| 两步 MODEXP次数 | 642，成功 | 642，成功 |
| 错误结果 | 正确拒绝 | 正确拒绝，2,371,186总 Gas |
| 完整64步 | 20M上限 OOG | 20M上限 OOG |
| 完整失败尝试退出内存字数 | 85,860 | 7,203 |
| 完整失败前已执行 MODEXP | 1,530 | 5,636 |

新部署总 Gas 为1,377,399，runtime6,124 B；调用 calldata仍为1,316 B。
新完整尝试已执行的5,636次 MODEXP全部成功，根异常是 `out of gas`。
两次完整尝试均未完成，执行到的位置不相同；不能把相同20M预算花费解释成相同成功成本，
也不能依据调用次数推算完整循环 Gas或精确已完成步数。

这些百分比是**同一组件、同一输入的一次交易对照**，不是统计性能样本，
不是投票端到端加速，也不是完整 SnarkPack Gas。

## 实现修改

新文件为 `contracts/src/Bls12381MillerLoopMemory.sol`，不覆盖旧基准。
点、线、基础域、塔域、编码函数及 NAF与 B2对应部分逐字一致。

所有常驻对象在临时工作区之前分配：不可变 Q、输入坐标、NAF，以及可变 R、Fp12累积值和线。
R的坐标是 Q的深拷贝，不能共享底层 Fp对象。每次平方/倍点/线乘/带符号加点结束时，
先将全部 `hi/lo`数值复制到常驻对象，再回退临时分配指针；不让临时引用逃逸。
所有线与群状态仍由 EVM运算产生，`expected`仅用于结果比较。

Solidity内存引用赋值会共享指针，不能用普通结构体赋值替代深拷贝。
本实现将回退操作明确留为未标记 `memory-safe` 的 assembly，限制于本轮固定编译配置；
不声称适配任意编译器或 viaIR。相关约束见
[Solidity0.8.30内存布局](https://docs.soliditylang.org/en/v0.8.30/internals/layout_in_memory.html)
和[内联汇编内存安全](https://docs.soliditylang.org/en/v0.8.30/assembly.html#memory-safety)。

回退分配指针**不会回收已经扩展的 EVM内存或返还 Gas**；它限制后续临时分配继续抬高内存高水位。
表中记录的是实际 `memoryWordCount`，不是主机峰值RSS，也不是通过指针值模拟的内存节省。
本轮没有追加内存清零/编译器矩阵/故障矩阵实验。

## 验证及产物

- 新运行器：`contracts/scripts/g5c3_miller_memory.mjs`。
- 新原始结果：`experiments/g5_c3_raw/miller_memory_evm.json`。
- 固定输入继续使用 `fixtures/snarkpack_g5c3_miller_loop_v1.json`，不重新生成。
- 原始结果记录旧基准哈希、新源码/锁/Manifest哈希、实际回执、内存字数及预算失败。

```bash
npm run --prefix contracts g5c3b3
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

实际编译、Node语法检查和diff检查通过；独立审查未发现持久对象/临时指针别名问题。
前缀与固定参考值逐系数一致；642次MODEXP保持不变，错误目标仍返回
`INVALID_MILLER_RESULT`。完整循环未跑通，因此公式检查不能替代完整 EVM正确性证据。

环境：Apple M4、Node23.11.0、EthereumJS10.1.3 / Prague、本地签名交易，无RPC。
Git基准 `7442414a04e8d06d79d968b3cbf448a6a694347a`，`git_dirty=true`，
不是 clean-commit 论文正式性能样本。没有重跑Rust高内存测试、setup/prove或规模实验。
两份Manifest、Cargo.lock、曲线和聚合协议均不改变；完整验证Gas保持 `null`。

## 下一步只做 G5-C3-B4：专用平方及014稀疏线乘法

内存优化已得到实际收益，但当前通用 Fp12乘法仍同时承担平方和稀疏线乘。
下一步保留 B2/B3作为基准，只替换这两种算术实现，保持相同的输入输出和内存复用方案：

1. 专用 Fp12平方与014稀疏线乘法必须与既有参考运算一致，不能使用未绑定的宿主结果。
2. 仍只跑同一两步正反例，再做一次完整64步、同一20M预算尝试。
3. 若仍超预算，记录真实差异并保持 BLOCKED，不盲目上调上限。

最终幂、GT目标绑定、完整聚合验证仍未实现。在完整 Miller通过之前，不进入最终幂、规模实验，
不把组件结果升级为论文完整链上验证主张。

后续B4已实现上述专用算术，见 [B4实测报告](G5_C3_MILLER_SPARSE.md)：同一前缀Gas
再降低21.35%，但完整循环仍在20M预算下OOG。下一步先定位底层Fp/Fp2成本，不重复整段尝试。
