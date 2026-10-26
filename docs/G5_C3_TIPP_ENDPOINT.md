# G5-C3-B：真实 TIPP 终点一致性及 EVM 路线缺口

本轮完成了**同一真实 B=2 证明的一条 TIPP 终点等式的 Rust/JavaScript 一致性检查**，
并在本地 Prague EVM 用实际终点群点检查了现有配对接口。
结果为 `PARTIAL_PASS_TIPP_ENDPOINT_CONFORMANCE_EVM_TARGET_BLOCKED`：
**不是完整 G5-C3-B 通过，也不是 SnarkPack 链上验证通过。**

## 本轮真实执行

复用 `fixtures/snarkpack_g5c1_public_v1.json` 中的公开证明，不生成新证明。
证明 SHA-256：`12e25c7edf07eff2d3f211a3cbb74ae4ce43e028ab58702debdef98b0787e64c`。

固定等式为

\[
e(A_{\rm final}, B_{\rm final})
= Z_{\rm final}
= \operatorname{ip}_{ab}\, Z_L^c\, Z_R^{c^{-1}}.
\]

这对应 pinned bellperson 0.27.0 `verify.rs` 的真实检查：493行提交配对终点；
646–661行初始化 `ip_ab`；726–727行分别使用 `c`、`c_inv` 缩放左右项；778行合并。
这里使用 V2 的**第二次** `gipa-0` 挑战，而不是初始 seed。
blstrs 的 GT API 使用加法表示乘法，标量乘表示 GT 幂，不能照字面混淆。

必要检查只有以下几项：

| 检查 | 实际结果 | 边界 |
|---|---|---|
| Rust 从官方 wire reader 解码并计算配对、折叠目标 | 相等，PASS | 只验证这条终点等式 |
| JS 从原始 wire 和外部 statement 重算 C2 全部6次挑战 | 与已保存官方记录完全一致 | 非 EVM 挑战重算 |
| JS 独立解压 GT、重算目标和真实 pairing | 与 Rust 的12个 Fp 系数全部一致 | 不是只比较 pairing 是否为1 |
| 只将目标乘以公开 `ip_ab` | Rust/JS 均拒绝 | 点、挑战不变；两个目标均为非单位 GT 子群元素 |
| 实际 `final_a`、`final_b` 调用 EVM `0x0f` | 成功返回32-byte false；70,300 primitive Gas | 只说明配对不是1，不比较目标 |

Rust 复用 C2 已捕获的官方挑战记录，并验证其来源 fixture/证明哈希及 `c*c_inv=1`；
JS 独立重算完整 preimage、拒绝采样、挑战并匹配记录。因此单独生成 Rust fixture
不代表挑战跨语言检查已经重跑，完整本轮检查必须继续执行 `npm run g5c3b`。

错误目标采用 `Z_wrong = Z_final * ip_ab`，不使用越界系数或非子群值来制造拒绝。
原生 reader 的 GT 验证和 JS 子群检查均通过，拒绝原因是**等式不相等**。
它是目标比较组件的负例，不是构造另一份完整 SnarkPack proof。

## 为什么仍是 BLOCKED

[EIP-2537 的 pairing ABI](https://eips.ethereum.org/EIPS/eip-2537#abi-for-pairing-check)
只接收 `384*k` 字节的 G1/G2 对并返回配对乘积是否为1；没有任意 GT 目标的输入槽，
也不返回 Fp12。此次实际终点输入384 B，输出32 B，而目标的组件编码为768 B。
有效终点返回 false 是正常结果，不能把它解释成正确目标被拒绝，更不能声称错误目标在 EVM 被拒绝。

在该轮结束时，Solidity只有上一轮的 Fp12乘法，没有 Miller loop、最终幂及完整 GT标量运算；
尚不能在 EVM 内完成这条等式。不得用宿主计算的 `Z_final` 或可信布尔值补洞。
也不能仅为布尔接口提供一对声称“对应目标”的群点，却不验证其与目标绑定。

这属于**实现及接口缺口**，不是“Mac 内存不足”、已测得的 OOG 或“EVM 不可能实现”。
本轮只给接口 probe 1,000,000 Gas 预算，实际消耗70,300；没有运行完整软件配对，
也不根据上一轮的乘法 Gas 推算完整验证成本。`snarkpack_verification_gas=null`，
`pairing_to_gt_evm_verified=false`，`evm_wrong_gt_target_rejected=null`。

## 文件与复现

新增：

- `crates/vv-friendly-dkg/src/bin/g5c3_tipp_endpoint.rs`：原生终点向量生成。
- `fixtures/snarkpack_g5c3_tipp_endpoint_v1.json`：仅公开输入、GT 目标和原生结果。
- `contracts/scripts/g5c3_tipp_endpoint.mjs`：独立重算及一次真实 EVM 接口 probe。
- `experiments/g5_c3_raw/tipp_endpoint_route.json`：实际结果、Gas、输入、源码/锁/Manifest 哈希。

```bash
CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-dkg --bin friendly-g5c3-tipp-endpoint
npm run --prefix contracts g5c3b
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

环境为 Apple M4、Node 23.11.0、Noble 2.4.0、EthereumJS VM 10.1.3 / Prague；
Rust 1.99.1、workspace Release opt-level 1。
Git 基准 `7442414a04e8d06d79d968b3cbf448a6a694347a`，`git_dirty=true`；
不是 clean-commit 正式性能样本。EVM 本轮是 `runCall` primitive probe，无 RPC、无部署或签名交易。

新增 binary 的严格 Release Clippy、workspace fmt 检查和 Node 语法检查通过。
没有重复全部历史测试、高内存 P3 或 setup/prove，也没有扩大 B 或选民规模。
旧 C2/C3-A 向量、两份 Manifest、Cargo.lock、曲线和聚合协议均不修改。

## 下一步只做 G5-C3-B2：软件 Miller loop 最小组件

1. 继续复用同一终点和公开参考结果，在现有 Fp12 算术上实现软件 Miller loop；
   先交叉核对参考实现及配对约定，不把宿主提供的中间值当 EVM 运算。
2. 为单组件设置明确执行预算并记录真实结果；若预算或实现尚不满足，准确保留缺口。
   Miller loop 通过也不等于完成配对，还需要最终幂及目标比较。
3. 最终幂和目标绑定完成后，才运行 EVM 的一个正确目标和一个错误 GT 目标；
   在此之前不填写完整 pairing-to-GT 或 SnarkPack Gas。

本轮止于端点一致性和具体路线检查。下一步不新增 wrapper SNARK、不换曲线、不扩大批次，
不做故障矩阵、规模实验或重复证明生成。

后续进展见 [G5-C3-B2 软件 Miller报告](G5_C3_MILLER_LOOP.md)：两步前缀已在 EVM
通过，完整64步在固定20M预算下 OOG。精确配对目标比较依然未完成，不能用本页历史
“实现缺口”替代后续已测得的具体预算结果。
