# G5-C3-B2：软件 Miller 前缀通过，完整循环受本轮预算阻挡

本轮已新增并真实执行 Solidity 软件 Miller-loop 组件。**前两步与参考值完全一致，
错误结果被拒绝；完整64步在20,000,000 Gas交易预算下 OOG。**
状态：`PARTIAL_PASS_MILLER_PREFIX_FULL_LOOP_OOG_20M`。
这不是完整 Miller loop、配对目标比较或 SnarkPack 链上验证通过。

## 只做了哪些必要实验

继续复用同一份真实 B=2 SnarkPack 证明的 `final_a`、`final_b`。
旧证明、C2挑战向量、原生终点向量和旧乘法合约保持不变。
原始证明 SHA-256：`12e25c7edf07eff2d3f211a3cbb74ae4ce43e028ab58702debdef98b0787e64c`。

1. 参考端重算全部6次 C2挑战，确认完整 TIPP 折叠目标与之前原生 blstrs 结果一致。
2. Solidity部署一次；运行前两步的正确结果及一个错误结果，除此不增加反例矩阵。
3. 用同一合约和实际群点，**只尝试一次完整64步**，交易上限20M Gas，区块上限30M Gas。
   预算耗尽后没有上调预算或反复试跑。

### 实际结果

| 项目 | 实际结果 |
|---|---:|
| 部署总 Gas | 1,280,304 |
| 合约 runtime | 5,675 B |
| 两步正例总 / execution Gas | 4,805,196 / 4,768,504 |
| 两步正例 calldata | 1,316 B |
| 两步错误结果 | revert `INVALID_MILLER_RESULT`，4,804,731总 Gas |
| 每次两步调用 MODEXP | 642次，全部成功；执行 Gas合计128,400 |
| 完整64步尝试 | receipt=0，`out of gas`，消耗预算20,000,000 Gas |
| 完整尝试退出时活动 EVM 内存 | 85,860 words（2,747,520 B），不是主机峰值 RSS |
| 完整尝试已执行 MODEXP | 1,530次，全部成功；没有把预编译失败误报为等式拒绝 |

完整尝试的20M是**失败交易实际花费的预算上限**，不是完整循环的成功成本，
不能推算单次配对或完整 SnarkPack Gas。一次未优化实现 OOG 也不证明 EVM 路线不可行。
本轮没有计算完整循环所需的精确总 Gas，没有进行额外分项性能实验。

## 实现与正确性边界

公式移植自固定版本的 [Noble 2.4.0 源码](https://github.com/paulmillr/noble-curves/blob/2.4.0/src/abstract/bls.ts)，
新合约保留完整 MIT 许可。基础域/塔域运算沿用上一轮经验证的算法，但复制到新文件，
不修改历史乘法组件或其源码哈希。

参数为 `|x|=0xd201000000010000`，使用带符号 NAF；共64个循环步骤、69条线。
非零 NAF 索引（从0开始）为 `1:-1, 3:+1, 6:+1, 15:+1, 47:+1`。
每步在 EVM 内执行倍点并构造线；非零位置执行对应的正/负加点。
线函数稀疏槽为0、1、4；目前通过通用 Fp12乘法完成，平方也使用通用乘法，尚未优化。

两步前缀覆盖2次倍点、1次负号加点、1次 Fp12平方、3次线函数乘法。
前缀返回未共轭累积值；只有完整64步才执行负 BLS参数对应的共轭。
**Miller输出不等于最终配对，也不应要求其已经在 GT子群内。**

Solidity只接受384 B仿射点编码、步骤数和待比较结果，不接收预计算线、G2更新状态
或可信累积值作为计算来源。`expected`只用于和合约自行计算的结果比较。
错误用例只将预期结果首个 Fp系数加一模 p；群点、步数不变，仍是规范 Fp编码。

参考端使用 Noble线系数和通用 Fp12乘法手工重放64步，并与其 `pairing(...,false)`
匹配；最终幂只在参考端计算，结果匹配此前原生 TIPP目标。
独立手工点更新公式审查所得参考哈希也一致（12×Fp64BE）：

- 两步未共轭前缀：`8dfe01e9042a7543c023489a4b03bf5c6e4d3abfc170934906eb2aabb289ad92`
- 完整已共轭 Miller：`d2677f402256f0f1e2dfc7f05a2752740e73f4929336780f9067f49b866fb2b2`

这里只能证明实际跑通的两步 EVM结果正确。其余步骤的公式/参考检查不代替完整 EVM执行。
合约只检查坐标规范编码，没有实现完整曲线/子群/单位元策略、最终幂、EVM挑战重算、
目标折叠绑定或完整聚合方程，不能当验证器部署。

## 文件与复现

- `contracts/src/Bls12381MillerLoop.sol`：全部点更新、线和域运算在 EVM 内执行。
- `contracts/scripts/g5c3_miller_loop.mjs`：参考重放、部署、必要调用及原始记录。
- `fixtures/snarkpack_g5c3_miller_loop_v1.json`：公开点、前缀/完整参考输出，不含私有 witness。
- `experiments/g5_c3_raw/miller_loop_evm.json`：真实回执、异常、内存字数、MODEXP计数及哈希。

```bash
npm run --prefix contracts g5c3b2
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

复现命令会重放上述固定实验，不生成新证明。Node语法检查、Solidity真实编译和
`git diff --check`通过；未重跑所有历史测试或Rust高内存证明电路。
环境：Apple M4、Node23.11.0、EthereumJS10.1.3 / Prague、Noble2.4.0、
Solidity0.8.30、optimizer200、无viaIR；本地签名交易，无RPC。
Git基准为 `7442414a04e8d06d79d968b3cbf448a6a694347a`，`git_dirty=true`，
不是 clean-commit 正式论文性能样本。没有改曲线、Manifest、Cargo.lock、SRS或论文安全主张。

## 下一步：G5-C3-B3，只优化当前 Miller 实现

本轮已从“没有软件实现”推进到“前缀真实执行通过、完整实现遇到具体20M预算限制”。
下一步不进入最终幂，而是：

1. 保留此次未优化代码和结果作基准，先改累积内存分配/固定工作区，再考虑专用平方及稀疏线乘。
   退出时内存字数是选取此方向的依据；本轮尚未分离计费，不能声称全部超支均由内存造成。
2. 同一个公开终点、同一个两步正反例检查优化版本，不能用未绑定的宿主结果替换运算。
3. 版本通过后，在**同一20M交易预算**再做一次完整64步尝试；若仍不通过，记录差异并保留 BLOCKED。

只有完整循环通过，才开展最终幂和正确/错误 GT目标比较。此前完整 SnarkPack Gas保持 `null`，
不做规模、故障矩阵、额外 setup/prove或曲线/聚合协议替换。

后续已实施内存专项优化，见 [G5-C3-B3报告](G5_C3_MILLER_MEMORY.md)：同一两步 Gas
降低50.65%，但完整64步仍在20M预算下 OOG。本页基准及其代码/原始结果均保留不变。
