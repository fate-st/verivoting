# G5-C2：SnarkPack 内部编码与挑战一致性

本轮结论：**G5-C2 已通过。完整 SnarkPack EVM 验证仍未完成。**
复用 G5-C1 保存的真实 B=2 聚合证明，没有重新 setup/prove，没有扩大批次，
也没有新增故障矩阵。本轮只解决内部 transcript 是否能被另一语言准确重建。

## 实际完成了什么

1. Rust 从原有公共 fixture 解码 proof、VK、公开 verifier SRS 和两条 statement，
   重算 external transcript，再调用完整官方原生 SnarkPack V2 verifier。
2. 在临时目录复制固定 bellperson 0.27.0，仅为 `Transcript` 增加日志字段和输出。
   追踪实际 verifier 的完整哈希 base、最终 counter 和 challenge；不是自写镜像互相验证。
3. JavaScript 使用已锁定的 `@noble/curves 2.4.0`，从同一 5,236-byte wire proof
   独立解压 15 个 GT 元素、重建规范 GT 编码、bincode 字段和 SHA-256 挑战。
   它不接受 Rust 提供的 GT 系数或预拼接 preimage。
4. 只保留一个必要反例：第二名选民的 nullifier 改动，其他公开输入、proof、VK、SRS 不变。

## 真实结果

正例与反例各有 6 次官方 challenge 调用，**12/12** 的完整 base、最终拒绝采样
counter、规范 Fr 标量均逐字节一致。正例的调用如下；payload 不含 prefix/tag/nonce。

| 调用 | 序列化 payload | 实际尝试次数 |
|---|---:|---:|
| `hcom` | 2,304 B | 1 |
| `random-r` | 1,694 B | 1 |
| `gipa-0` seed | 688 B | 3 |
| `gipa-0` V2 round | 5,888 B | 2 |
| `gipa-extra-link` | 6,080 B | 1 |
| `random-z` | 512 B | 1 |

真实正例自然覆盖了 3 次被拒绝的候选，因此没有添加人为制造的拒绝采样实验。
官方日志记录每次调用最终接受的 counter/scalar；逐次候选 digest 由 JavaScript 重算。
不能将其描述为“官方输出了每个被拒绝候选的 digest”。

完整原生 verifier 接受正例、拒绝同一证明的错误 statement。反例只改变第二条
25-field statement 的索引 7，即展平索引 32。`hcom` 保持不变，后续 5 个挑战全部改变。
两次同名 `gipa-0` 调用分别保存为 seed 和 round，没有按 tag 覆盖。

## 版本和执行边界

- 基准 Git SHA 为 `7442414a04e8d06d79d968b3cbf448a6a694347a`，运行时
  `git_dirty=true`，不是 clean-commit 性能复测。
- 环境：Apple M4、Rust 1.99.1、Release `opt-level=1` / `strip=none`、Node 23.11.0。
- bellperson 0.27.0、blstrs 0.7.0、blst 0.3.10、bincode 1.3.3、SHA-256 依赖版本均核对。
  临时与原项目实际启用的 bellperson features 都只有 `groth16`。
- 原始 `verify.rs`、已安装依赖源码、原项目 `Cargo.lock`、G5-C1 公共 fixture 未被改动。
  官方 `transcript.rs`、`verify.rs`、日志版本及两种 manifest 的哈希均随材料记录。
- 临时 path dependency 会解析未启用的 CUDA-supraseal 分支，该分支与固定 CPU
  blst 版本冲突。因此只在临时 manifest 移除此可选依赖并保留空的特性名。
  没有升级 blst、启用 GPU、运行第三方基准或修改正式协议清单。
- `contracts/package.json` 显式声明已安装的 Noble 版本；只有测试依赖锁定变更，
  不是投票曲线或密码学套件迁移。

限制为 B=2、每票25个公开输入、64-bit counter ABI。内部 V2 prefix 仍是
`snarkpack-v1`；GT transcript 用576-byte完整 Fp12，wire 用288-byte torus压缩编码；
拒绝后 nonce 累计追加到同一 SHA 状态，extra-link 使用 round challenge 的乘法逆。
这些细节均由真实官方调用对照，而不是替换哈希或取模近似。

本轮没有验证 Solidity 中的内部挑战、GT 算术、GIPA/KZG/聚合配对关系或完整结算。
`exact_snarkpack_evm_verified=false`，聚合验证 Gas 仍为 `null`，Manifest 能力不提升。
原生确定性 verifier RNG 仅用于功能复现；日志化验证时间不作为公平性能基线。

## 怎样查看和复现

先看 `experiments/g5_c2_raw/transcript_conformance.json`：
`matched_call_count=12`、三个 `rust_js_*_match=true`，以及
`official_complete_native_changed_nullifier_rejected=true` 即为本轮验收结果。

完整规范向量在 `fixtures/snarkpack_g5c2_transcript_v1.json`，其中既有独立 JS
重建结果，也有官方实际调用日志。`native_replay.json` 保存原生重放结果。
这些文件只含公共验证材料，不包含私有票、私有 witness 或 proving key。

```bash
cd contracts
# 新环境先 npm ci --ignore-scripts --no-audit --no-fund
npm run g5c2
cd ..
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

脚本自动建立临时参考副本并校验版本；首次可能需要联网补齐 Cargo 索引元数据。
不需要重新生成 G5-C1/G4-B/P3 证明。格式检查和新增 Rust binary 的严格 Release
Clippy（`-D warnings`）通过。

## 下一步只做 G5-C3

更新：后续已完成 C3-A 的真实 Fp12 乘法组件；配对目标比较仍未完成，
当前下一步为 C3-B，见 `docs/G5_C3_FP12_PRODUCT.md`。以下保留 C2 阶段的原定计划。

下一项瓶颈是 **GT 运算及配对到非单位 GT 目标的 exact EVM 路线**，不是继续扩规模。

1. 从同一公共证明选出一个真实 GT 运算及对应的目标比较，固定原生预期输出。
2. 先确定可验证的 EVM 实现路线；不能把 host 算出的 GT 直接当作可信结果，
   也不能将 pairing-product-to-one 布尔接口当作一般 GT 输出。
3. 如路线可实现，仅跑一个真实正例和一个错误目标反例，记录实际资源/Gas及其范围。
   若缺少必要实现，记录具体 BLOCKED 原因，不填估算数据、不换套件或添加 wrapper 冒充原协议。

达到这一最小门槛后才扩展完整 SnarkPack verifier；本轮不提前实施 G5-C3，
也不修改论文结论、正式 Manifest、任意子集支持或完整 settlement 状态。
