# G5-D：P3 tally-v2 本地 EVM 验收

## 结论与范围

**PASS：G4-C 归档的部分投票和满额投票两份真实 Groth16 证明，均能使用其新 v2 VK，在既有 Solidity 验证器中通过本地 Prague / EIP-2537 验证。** 部分投票证明仅修改公开 winner，或原样改用归档 v1 VK，均被拒绝。

本轮只完成修复后的验证路径回归：公开材料转换及原生复核、2 次合约部署、4 次验证调用，共 6 笔本地签名交易。没有重新 setup/prove，没有网络或公开链交易，没有重开已冻结的软件 Miller 路线，也未展开规模矩阵。

- 关系：`VV/JUBJUB/P3/BOUNDED-TALLY/v2`；固定 Trustee 子集 `{1,2,3}`。
- 两份证明来自 G4-C 的同一次新 setup；本轮复用归档证明和新 VK，不复用旧 v1 VK 来验证正常 v2 证明。
- 每份证明：arkworks 压缩编码 192 B；EIP-2537 证明编码 512 B。
- 公开输入 60 个、IC 点 61 个；VK 的 EIP-2537 编码 8,704 B。
- 证明与公开标量的逻辑字节数为 2,432 B；下面的 2,564 B 是实际 ABI 交易 calldata，均不是完整批次 DA。

## 实际结果

| 验证用例 | 使用的 VK | receipt | 总 Gas | execution Gas | Calldata | 结果 / 错误原因 |
| --- | --- | --- | ---: | ---: | ---: | --- |
| 部分投票归档证明 | v2 | 1 | 1,331,365 | 1,278,293 | 2,564 B | 通过 |
| 同一部分投票证明，仅 winner 输入 59 从 1 改为 2 | v2 | 0 | 1,331,443 | 1,278,371 | 2,564 B | `revert: INVALID_GROTH16` |
| 同一部分投票证明及全部原公开输入 | 旧 v1 | 0 | 1,331,443 | 1,278,371 | 2,564 B | `revert: INVALID_GROTH16` |
| 满额投票归档证明 | 同一 v2 | 1 | 1,331,377 | 1,278,293 | 2,564 B | 通过 |

旧 VK 负例与部分投票正例的 calldata SHA-256 完全相同，仅交易目标地址不同；没有将输入偷换成旧实验的输入。此结果说明这两把实际归档 VK 与证明不能混用，不是“任意旧 VK 必然拒绝”的普适安全证明。

4 次验证均真实调用 G1 MSM `0x0c` 与 pairing `0x0f`，输入分别为 9,760 B、1,536 B，返回分别为 128 B、32 B。两预编译本身均成功；pairing 在两个正例返回 1、两个负例返回 0。拒绝不是 OOG、长度错误或预编译执行失败。

v2 部署为 **6,631,824 Gas**，旧 v1 VK 部署为 **6,631,812 Gas**，部署 calldata 均为 11,405 B。两个实例的 runtime 均为 1,807 B，但哈希不同，因为源码中的 `verificationKeyDigest` 是随 VK 固定的 immutable；本轮复用的是相同合约源码和编译配置，而不是要求两个 runtime 字节完全相同。

Prague 记账已逐笔复核：refund 均为 0，`total = max(intrinsic + execution, calldata floor)`，6 笔交易均无 floor 补差。两份正例 12 Gas 的总量差来自 calldata 非零字节数量，不代表验证计算性能差异。每用例仅一次执行，不报告中位数、P95 或性能优势。

## 证据与来源

实测时间为 `2026-10-28T08:25:22.345Z`（北京时间 16:25）。环境为 Apple M4 / macOS arm64、Node `v23.11.0`、Solidity `0.8.30`、optimizer 200、`evmVersion=prague`，EthereumJS 相关包为 `10.1.3`。

运行基点为 `6551be4369a44928c2777602e07bfcb3b5a151ab`，**`git_dirty=true`**：新 exporter、runner 等修改当时尚未提交，不能将其描述为干净提交复现。原始记录另存源码及输入 SHA-256，明确绑定实际运行版本。

| 材料 | SHA-256 |
| --- | --- |
| [G5-D EVM 原始记录](../experiments/g5_d_raw/g5d_p3_v2_evm.json) | `fecfc53d7115265617f86065ccd8b93835c0a434fe213665832a34e6aba9b19f` |
| [G4-C 证明生成记录](../experiments/g4_c_raw/partial_turnout_groth16.json) | `4979a9dc777f39a916711d85ab5e75bca48481bb066cd7b5fdaa1073290d8c1d` |
| [归档 arkworks 公开材料](../fixtures/jubjub_g4c_partial_turnout_v2.json) | `b9d41ab5cce14b151ca1ad0ba21edf742199189215ea4281a829f57f0d35c4c0` |
| [转换后的 EIP-2537 材料](../fixtures/g4c_eip2537_fixture_v2.json) | `b611894a40ae34d29d5c6799eccc9219d2cb59b7f5516c32bb9d6c7b8c5cd903` |
| [公开材料转换与原生复核记录](../experiments/g5_d_raw/public_export.json) | `83af91b3e3d135634d644c93c0f30b7ee0bcd4a2473f0c16f03ae82c09c53535` |
| [G5-D runner](../contracts/scripts/g5d_p3_v2_verifier.mjs) | `9030b4528c98e779f5c2678ec79ae1e5a268b5a7c5ebdd63bd5c49230d497271` |
| [既有 Solidity 验证器](../contracts/src/Bls12381Groth16Verifier.sol) | `8fffe10ba59e994cacfb103a75fc7158b849437ea63cd19210997b60616f1277` |

已只读核对原始记录中的 16 项输入、源码及工具链哈希，并核对 4 个用例的 receipt、错误、pairing 返回值和 Gas 恒等式。G4-C 证明记录及原公开材料、G5-B EVM 记录及旧公开材料、Solidity 验证器这 5 个历史文件均与上述 Git 基点中的字节一致，未覆盖旧实验数据。新旧 VK 的 digest、两个部署 runtime 哈希及各笔交易完整账目均保存在 G5-D JSON。

公开导出完成后，Clippy 对十六进制解析器提出纯写法建议，因此将
`chunks_exact(2)` 改为 `as_chunks::<2>().0.iter()`；偶数长度检查、解析及所有
密码学关系不变，没有为此重跑 EVM 或证明。原记录中的 exporter 源码哈希指向
**实际执行时的版本**，其完整快照保存在
`experiments/g5_d_raw/provenance/g5d_p3_v2_export.executed.rs.txt`，SHA-256 为
`99c04fb2842de2f3a6ca87ba9799e7bd1c2b200de73a1a305bf47a95622c47b8`。
当前源码仅有上述机械差异；不能把测量时哈希改写成后续版本的哈希。

代码验收：两个受影响 crate 的库回归测试共 **41 项通过**；Rust 格式检查、
`clippy --all-targets --release --locked --offline -- -D warnings` 和 Node 脚本
语法检查通过。真实 EVM 结果依然来自上面的 6 笔交易，不添加测试矩阵或性能重复。

## 复现

以下命令从仓库根目录执行。默认公开材料及转换记录已归档时，**不必重复导出**；可直接运行 EVM 命令。若要独立复核转换，必须使用全新输出路径，且该过程只验证已有公开证明，不生成证明：

```sh
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-g5d-p3-v2-export -- \
  --output fixtures/g4c_eip2537_fixture_v2_reproduce.json \
  --report experiments/g5_d_reproduce/public_export.json
```

当前 runner 固定读取已归档的默认 fixture 和 export report；上面新路径用于独立转换复核，不会自动替换其输入。EVM 复现使用默认归档输入，并将新测量写入另一文件：

```sh
npm run --prefix contracts g5d -- experiments/g5_d_reproduce/g5d_p3_v2_evm.json
```

runner 将输出相对路径解析为仓库根目录下的路径，而不是 `contracts/` 下的路径。输出已存在时会拒绝覆盖；再次复现应另选新文件名。

## 不在本轮结论内

- v2 只实测 `{1,2,3}`，不代表另一个子集或全部 10 种合法子集已经通过 v2。
- 验证器未比较 accepted registry、冻结计票目标、承诺认证或 write-once winner 状态；因此不是完整 `AcceptWinnerDisclosure` 或原子结算。
- 未测完整 SnarkPack EVM、完整 Proposed Gas、完整批次 DA、公开链部署及最终性；不能将本轮 Gas 与单独状态验证 Gas 相加后称作系统总成本。
- 使用公开确定性研究 setup，不能视为生产可信设置。

下一步仅计划 **B=4、不同注册选民且同一区域的原生 SnarkPack 聚合**，复用现有选票关系；该实验尚未执行，不包含 B=4 状态证明或链上聚合能力升级，也不重试冻结的软件 Miller 路线。

后续更新：上述 G5-D 时点的下一步已在 G1-C2 实际完成，见
`docs/G1_C2_FOUR_VOTER_SNARKPACK.md`；当前待实施的是深度 3 / B=8 原生聚合。
