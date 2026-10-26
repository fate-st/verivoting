# G5-C3-A：真实证明衍生输入的 Fp12 乘法

本轮完成 **G5-C3 的乘法组件（G5-C3-A）**。原生、独立 JavaScript 与
Solidity/EVM 的完整乘积一致，一个规范错误目标被拒绝。
**非单位 GT 配对目标比较仍未实现，G5-C3 整体及完整 SnarkPack EVM 仍未完成。**

## 本轮只做了什么

复用 G5-C1 保存的同一份真实 B=2 SnarkPack 证明，取
`proof.com_ab.0` 与 `proof.com_c.0` 两个 GT 元素。Rust 使用官方 wire reader
解码后由 blstrs 计算原生乘积；JavaScript 从原始压缩 wire 的偏移 0、576
独立解压、校验子群并重算乘积；Solidity 在 EVM 内执行完整 Fp12 乘法。

这两个 GT 元素的乘积是**proof-derived 算术组件测试**，并不是 GIPA 中一条
完整的真实相邻子表达式，也不是 GIPA、TIPP、KZG 或聚合验证通过。

仅部署一个组件合约，执行一个正例和一个错误目标反例。负例只将预期乘积的
首个 Fp 系数加一再模 p，仍是规范 Fp；两个乘数保持完全相同。
实际 calldata 仅偏移 1795 的一字节变化。没有重跑 setup/prove，没有扩大批次或故障矩阵。

## 如何在 EVM 内正确计算 381 位乘法

基础域模数超过256位，不能直接用 `uint256 mulmod` 代替。
本实现将 Fp 表示为高、低两个256位 limbs，显式处理加减进位/借位，利用

\[
xy=((x+y)^2-(x-y)^2)/4\pmod p
\]

通过两次真正的 MODEXP 平方完成基础域乘法，再用模除二两次实现除四。
奇数除二前先加 p，不能先约简。MODEXP 接口来自
[EIP-198](https://eips.ethereum.org/EIPS/eip-198)；Gas 由本次 Prague 执行实际记录。

塔结构与现有 BLS12-381 库一致：`u²=-1`、`v³=1+u`、`w²=v`。
Fp2/Fp6/Fp12 分别使用3/6/3次下层乘法，总计54次基础域乘法、108次 MODEXP。
合约会重算全部12个系数，并返回规范乘积，不把宿主语言提供的乘积当可信运算结果。

组件 ABI 为 `VV/G5-C3/Fp12-12xFp64BE/v1`：12个 Fp 系数，各64-byte BE，合计768 B。
它仅是测试组件的输入编码，不是新的 SnarkPack wire、正式 Manifest 编码或曲线迁移。

## 实际测量

环境：Apple M4、Node 23.11.0、EthereumJS VM 10.1.3、Prague、
Solidity 0.8.30，optimizer 200，无 viaIR；本地内存 VM 中的真实签名交易，不连接 RPC。

| 项目 | 实际结果 |
|---|---:|
| 部署总 Gas | 793,421 |
| 正例总 Gas / execution Gas | 609,940 / 558,168 |
| 错误目标总 Gas | 609,708，revert |
| 每次调用 MODEXP 数 | 108，全部成功、每次返回64 B |
| 每次调用 MODEXP 执行 Gas 合计 | 21,600 |
| 调用 calldata | 2,500 B |
| 合约 runtime | 3,423 B |

这些是**一次 Fp12 乘法组件交易**的成本，不是完整 SnarkPack、结算、数据可用性
或公链 finality 成本。不由这些数值推算聚合验证总 Gas。
本次 Git 基准为 `7442414a04e8d06d79d968b3cbf448a6a694347a`，运行时
`git_dirty=true`；不是 clean-commit 性能对照。

原始报告：`experiments/g5_c3_raw/fp12_product_evm.json`。报告包含交易回执、
108次 MODEXP 追踪、源码/输入/依赖锁/Manifest 哈希及明确的未完成状态。

## 边界与未完成项

Solidity 组件检查乘数的 Fp 规范编码，但不进行 GT 子群检查，也不证明这些乘数
来自某份 SnarkPack proof。宿主侧本轮输入的 wire/子群校验不替代未来 EVM 检查。
标量幂、完整 GIPA/KZG、内部挑战的 EVM 实现及配对到非单位 GT 目标比较均未完成。

[EIP-2537](https://eips.ethereum.org/EIPS/eip-2537) 的 pairing 接口仍只返回配对乘积
是否为1；这个乘法合约没有补齐一般配对输出或目标比较。
因此报告保持 `complete_g5c3=false`、`pairing_to_gt_verified=false`、
`exact_snarkpack_evm_verified=false`，完整聚合验证 Gas 为 `null`。
正式/原型 Manifest 和原项目 `Cargo.lock` 均未修改，没有提升任何完整链上能力。

## 复现

首次生成公共原生向量（已有 fixture 时可跳过，不生成新证明）：

```bash
CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-dkg --bin friendly-g5c3-fp12-fixture
```

重放两笔必要调用并更新汇总：

```bash
npm run --prefix contracts g5c3
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

本轮格式检查、新增 Rust binary 严格 Release Clippy、Solidity 编译和
Node 语法检查通过。未重复已有高内存电路或整个历史实验矩阵。

## 下一步只推进剩余的 G5-C3-B

目标是同一真实 B=2 proof 的一条实际 TIPP 终点等式
`e(final_a, final_b) = final_zab`，而不是继续添加乘法性能样本。

1. 复用 C2 的规范挑战，固定该等式的实际公开群点和原生 GT 目标。
2. 明确能够检查配对到 GT 目标的 exact EVM 路线；现有布尔 pairing 接口不能自动完成它。
   若需软件 Miller loop/final exponentiation，先判断具体实现与资源预算是否可行。
3. 路线可执行时只跑一个真实正例和一个错误目标反例；否则记录具体实现缺口，保留 BLOCKED。

不自动替换曲线/聚合协议、不增加 wrapper SNARK，不将宿主结果当可信目标验证，
也不在此之前跑规模、完整结算或公平性能对照。以上为 C3-A 当时的下一步计划。
后续 C3-B 已完成真实终点的 Rust/JS 一致性及实际 EVM 接口 probe，但精确 EVM
目标比较仍 BLOCKED；见 [G5-C3-B 报告](G5_C3_TIPP_ENDPOINT.md)。
