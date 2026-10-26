# G5-C3-B10：原 Fp2 加法成本定位完成

2026-10-26。本轮保持原 `_add2` 算法完全不变，仅新增测量入口。
真实执行一次部署、一次固定公开输入加法；完整结果正确。没有新增边界矩阵、
重跑乘法/Miller 或进行优化。

状态：`PASS_ISOLATED_FP2_ADD_COST_PROFILE_ONLY`，只表示成本定位完成。
不能称为优化已完成，更不能称为完整验证通过。

## 真实测量

使用原 B8 的同一公开 `final_b` 仿射 x/y，计算 `Fp2(x+y)`。
完整 128B 返回值同时匹配直接 BigInt 模加法与 Noble 参考实现。

| 项目 | 本次实测 |
|---|---:|
| 算术测量窗口 Gas | 1,752 |
| 合约 execution Gas | 5,740 |
| intrinsic Gas | 24,952 |
| 交易总 Gas | 30,880 |
| calldata floor 对应最低费用 | 30,880 |
| floor 补差 | 188 |
| 窗口分配指针增长 | 896B |
| 窗口内存扩张 Gas | 91 |
| 退出活动 EVM 内存 | 87 words |
| MODEXP 次数 / Gas | 0 / 0 |
| STATICCALL opcode Gas | 0 |

部署总 Gas 为 678,411，runtime 2,891B。两笔交易均是本地签名调用，无 RPC。
部署费用不计入单次加法窗口。

## 交易总费用不等于加法执行成本

本次退款为零，实际费用对账为：

```text
intrinsic + execution = 24,952 + 5,740 = 30,692
total = max(30,692, 30,880) = 30,880
floor_extra = 188
```

即本次触发了本地 Prague 模式的 calldata floor。188 Gas 是交易最低费用补差，
不是 `_add2` 算术、内存扩张或测量窗口开销。
顶层 opcode Gas 合计为 5,740，与 execution 一致；两次 GAS 标记差为 1,752。
窗口内存字数由 44 增至 72，对应扩张费用 91；分配指针为 `2,304−1,408=896B`。

因此后续即使算术窗口/执行成本下降，单次调用的总 Gas 也可能仍为 30,880，
不能强行要求交易总费用下降作为优化通过条件。应分别比较窗口、execution、分配量与总费用。
窗口包含编译调用及测量开销，不是嵌入 Miller 后不变的操作常数；
分配指针增长也不是 EVM 峰值内存或主机 RSS。

## 改动与验收范围

新增 `contracts/src/Bls12381Fp2AddProfile.sol`，从 B8 原语测量合约复制。
严格整文件核对仅有合约名变化和新增 `profileFp2Add` 一个入口；原有 17 个顶层函数
逐字保留，`_add2` 也与 B9 集成合约逐字相同。
新入口与原 `profileFp2Mul` 的测量结构相同，只把被测操作改为 `_add2`。
窗口位于规范解析后、结果编码前，不改变原算术算法。

固定输入已经自然包含两个不同分量路径：c0 无需减 p、无低 limb 进位；
c1 需要减 p、有低 limb 进位及约减借位。路径条件由输入推导，
不是内部指令覆盖或穷尽正确性证明。由于算法未改，本轮不另加边界或负例矩阵。

真实 Solidity 编译、Node 语法及 diff 检查通过。独立只读复核确认了 16 项来源文件哈希、
完整结果、费用、窗口、内存及零 MODEXP 记录，未重复执行 EVM。

## 产物与复现

```bash
npm run --prefix contracts g5c3b10
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

- 合约：`contracts/src/Bls12381Fp2AddProfile.sol`。
- 运行器：`contracts/scripts/g5c3_fp2_add_profile.mjs`。
- 原始结果：`experiments/g5_c3_raw/fp2_add_cost_profile.json`。
- 算术来源：B8；最新集成前缀：B9；最新完整循环尝试：B7 旧版 20M OOG。

环境：Apple M4 / arm64，Node 23.11.0，EthereumJS 10.1.3 / Prague，
solc 0.8.30、optimizer 200、无 viaIR。
Git 基准 `7442414a04e8d06d79d968b3cbf448a6a694347a`，`git_dirty=true`；
本轮不是 clean-commit 论文性能样本，真实源码由哈希固定。

| 产物 | SHA-256 |
|---|---|
| 新合约 | `a189ba3f395fc97a5366d9a0a184595fca9eb1faea3008a7ad14772f3bf736b4` |
| 新运行器 | `92bbd43f02a9b87fcae00a9c58ec738fc1ea6f25795fb0aefd696ead6161bed4` |
| 原始结果 | `a8d1e86f1db695f68a4369b5519bd297a521f714092408be89feb8349ebc5554` |

没有修改旧实现、结果、向量、锁文件、曲线、SRS、Manifest 或论文主张。
B9 新版完整循环仍未验收；B10 没有新 Miller 尝试，不能称新版仍超预算。
完整验证保持 BLOCKED，最终幂/GT 目标绑定/完整 SnarkPack 仍未完成，聚合验证 Gas 为 `null`。

## 下一步只做 B11：一个低分配加法候选对照

1. 保留本轮原实现基准，新增一个隔离的 `_add2` 低分配候选，保持模数、公式、编码和输入不变。
2. 先比较同一公开输入的完整输出、窗口、execution、分配量和总 Gas；仅补改动直接需要的边界。
   总费用可能受 floor 限制，不能以“总 Gas 必须降低”代替算术成本比较。
3. 根据真实对照结果再决定是否集成；本测量不能证明加法是唯一瓶颈，也不能证明完整循环可达 20M。

本轮不实施 B11，不重跑 Miller、不提高上限、不提前最终幂、不扩大规模。

后续记录：B11 已完成一个低分配 `_add2` 候选对照，见
[G5_C3_FP2_ADD_LOWLEVEL.md](G5_C3_FP2_ADD_LOWLEVEL.md)。窗口和分配量下降，但受 floor
限制的交易总 Gas 未降；本页保留 B10 基准，下一步仅作 B12 集成前缀验收。
