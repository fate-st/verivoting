# G5-C1: SnarkPack 公共材料与 Prague 接口检查

本轮结论：**G5-C1 前置检查通过；完整 SnarkPack EVM 验证仍未实现。**
这一步解决的是“下一阶段拿什么真实证明、按什么编码开发验证器”，
不是新增规模实验，也不是用 Groth16 验证冒充 SnarkPack 验证。

## 本轮完成的最小实验

- 在原有 distinct-voter 程序中增加可选 `--snarkpack-fixture-output`，
  不改变投票关系、曲线、密码学套件或 Manifest。
- 使用真实完整 `C_vote` Groth16 证明，为两名不同的注册选民生成 SnarkPack V2 聚合。
- 导出 aggregate wire proof、Groth16 VK、公开 verifier SRS、两条公开 statement
  及完整 external transcript。没有导出 proving key、prover SRS 或私有 witness。
- proof 和 VK 使用正式 binary write/read；检查游标耗尽及对象相等。
  公开 verifier SRS 使用压缩点 JSON 编码并解码；随后用解码后的材料实际验证。
- 同一个 decoded aggregate 在第二名选民 nullifier 被改变后拒绝。
  在当前 25-field 布局中只改变第二条 statement 的输入 7（从 0 开始）。
- JavaScript 重建的 1,654-byte **external statement transcript** 与 Rust 逐字节一致。
  内部 Fiat--Shamir transcript 尚未移植，不能据此声称挑战已在 EVM 内重算。

## 真实运行结果

本次基准为 `7442414a04e8d06d79d968b3cbf448a6a694347a`，运行时
`git_dirty=true`；不是 clean-commit benchmark。环境为 Apple M4、Rust 1.99.1、
Release `opt-level=1` / `strip=none`、Node 23.11.0、EthereumJS VM 10.1.3、Prague。

| 项目 | 本次结果 |
|---|---:|
| 不同选民 / 每票公开输入 | 2 / 25 |
| 两条原始 Groth16 证明合计 | 384 B |
| SnarkPack aggregate proof | 5,236 B |
| 聚合时间 | 4.701 ms |
| 原生 aggregate 验证 | 3.765 ms，通过 |
| decoded 材料验证 / 修改第二 nullifier | 通过 / 拒绝 |
| 峰值进程 RSS | 2,442.406 MiB |

这些是一次功能复现的数据，不是统计性能结论。B=2 的聚合证明比两条原始证明大，
因此不能声称压缩收益。`setup_fake_srs` 仍只用于可复现实验，不是生产仪式。

Prague 中实际调用的必要原语如下：

| 原语 | 返回结果 | 单次 precompile execution gas |
|---|---|---:|
| SHA-256(external transcript) | 与 Rust 摘要一致 | 684 |
| G2 MSM：一个点乘 1 | 与输入点一致 | 22,500 |
| `e(P,Q)e(-P,Q)` | 32-byte true | 102,900 |
| `e(P,Q)` | 32-byte false，而非 GT 元素 | 70,300 |

上述数值来自 `vm.evm.runCall`，不含签名交易开销，**不是 SnarkPack 验证 Gas**。
群点原语探测复用 G5-A 的公开 EIP-2537 点，不是对 SnarkPack 压缩 GT 的解析或验证。

## 为什么完整 G5-C 仍然 BLOCKED

固定的 bellperson 0.27.0 verifier 在 GIPA/TIPP/MIPP 中直接对 GT 元素进行
标量运算和累积，并验证配对结果与非单位 GT 右端相等。
[EIP-2537](https://eips.ethereum.org/EIPS/eip-2537) 的 pairing 接口只判断配对乘积是否为 1，
没有通用 GT 输入、GT 输出或 Fp12 算术接口。当前 Groth16 Solidity 合约不能直接复用。

这是**当前 exact verifier 尚缺实现**，不是证明 EVM 不可能验证。
后续需实现并校验兼容的 GT/配对目标比较路线，或另行评估等价验证关系；
本轮没有替换聚合协议、添加 wrapper SNARK，或把两次单票验证计作聚合验证。
原生 PairingChecks 的确定性实验 RNG 也不能直接变成生产合约的固定批处理系数。

内部 transcript 的移植必须与固定源码一致：

- V2 仍使用 `snarkpack-v1` prefix，内部哈希为 SHA-256。
- `Transcript::write` 使用 bincode serde，不是 proof wire encoding。
- GT 在 transcript 中是完整 Fp12 coefficients，wire proof 则用 288-byte GT 压缩编码。
- counter 为 `usize.to_be_bytes()`（本机 8 bytes），先递增；拒绝后累计更新同一 hash 状态。
- digest 按 little-endian Fr 解释，削掉最高多余 bit，拒绝 0、1 及非规范标量。

参考固定版本源码：
[verify.rs](https://docs.rs/crate/bellperson/0.27.0/source/src/groth16/aggregate/verify.rs)、
[accumulator.rs](https://docs.rs/crate/bellperson/0.27.0/source/src/groth16/aggregate/accumulator.rs)、
[transcript.rs](https://docs.rs/crate/bellperson/0.27.0/source/src/groth16/aggregate/transcript.rs)。

## 复现

仓库根目录运行一次 source artifact 生成（已有 fixture 时不必重复）：

```bash
CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-dkg --bin friendly-ballot-cvote-b1 -- \
  --output experiments/g5_c_raw/source_full_c_vote_r1cs.json \
  --layout-output fixtures/jubjub_ballot_g5c1_public_layout_v1.json \
  --snarkpack-distinct-output experiments/g5_c_raw/source_snarkpack_distinct_voters_batch_2.json \
  --snarkpack-fixture-output fixtures/snarkpack_g5c1_public_v1.json
```

直接重放轻量检查：

```bash
cd contracts
npm run g5c1
cd ..
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

公共 fixture：`fixtures/snarkpack_g5c1_public_v1.json`。
真实报告：`experiments/g5_c_raw/snarkpack_route.json`。
报告记录材料哈希、工具链及 `exact_snarkpack_evm_verified=false`；完整验证 Gas 为 `null`。
不需要重新生成 P3、扩大 B、跑全套崩溃测试或修改论文来复现本步骤。

本轮验证还包括 `cargo fmt --all -- --check`、修改 binary 的 Release
Clippy（`-D warnings`）及 `vv-friendly-dkg --lib` 的 10/10 个回归测试。
没有重跑资源密集的 Ristretto 非原生电路测试或整个历史性能矩阵。

## 下一步只做 G5-C2

更新：本项已在后续 G5-C2 完成，实际结果与当前 G5-C3 计划见
`docs/G5_C2_TRANSCRIPT_CONFORMANCE.md`。下文保留本阶段原定验收要求。

从本轮固定公共 aggregate proof 导出内部 transcript 的 golden vectors，
依次核对 `hcom`、`random-r`、GIPA V2 challenge、extra-link 和 KZG challenge。
JavaScript 必须重建同样的 bincode bytes、rejection counter 和规范标量；
只增加一个修改第二 nullifier 的必要反例。已有证明可直接复用，不重跑 setup/prove。

验收条件是所有真实挑战逐字节一致、修改 statement 改变应绑定的挑战、
原生完整 verifier 仍拒绝错误 statement。达到后再考虑 GT 实现。
这个 gate 本身仍不代表完整 SnarkPack EVM 验证，后续完整结算和公平对照实验另行推进。
