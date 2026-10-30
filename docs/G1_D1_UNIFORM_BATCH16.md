# G1-D1：冻结 D5/W32 套件下的真实 B=16 聚合

状态：`PASS_G1D1_FIXED_DEPTH5_W32_B16_REUSED8_NEW8_SAME_VK_SRS`。
2026-10-28 已完成一次真实 Release 正反例验收，起始时间 `2026-10-28T13:58:22Z`。
本轮唯一目标是 B=16；验收后停止，没有开发或执行 B=32、状态证明、P3 或 EVM。

## 基准、修改与冻结范围

开始前实际执行 `git ls-remote --heads origin refs/heads/experiment/g08-jubjub-native`，
远端和本地均为 `bfc24a501c98ae3e95d767e3f2738fd7e6e195c3`，工作区初始干净，
没有更新冲突。实际测量时为该基点加本轮修改，`git_dirty=true`，不声称干净提交复测。

新增独立入口 `crates/vv-friendly-dkg/src/bin/g1d1_uniform_batch16.rs`，二进制名
`friendly-g1d1-uniform-batch16`；在已有 Cargo.toml 注册，并扩展原汇总脚本和文档。
未修改密码学库算法、Merkle 编码、PRF、公开输入布局、SnarkPack V2 或旧 D0 入口。

冻结配置 `configs/protocol_manifest.JUBJUB-EVAL-D5-W32.json` 保留全部原始字节，
SHA-256 仍为 `00c3869c4b10a6e48fa5972b2a98767966db2b2fac0de3692eb8cbbc13cafe5e`。
固定深度 5、32 名单位权重注册选民、总权重及上界 32、3 候选人、4 区域、
5 Trustee / 3-of-5。16 名实际投票者是原注册 fixture 的前 16 人，均由原区域函数
选入 region 4，地址、nullifier、密文向量及证明均不同。

## 严格兼容后才复用

1. 固定读取 G1-D0 原始结果和公开 fixture，核对二者已归档 SHA-256、身份及八份
   源证明数量；逐个核对 D0 原测量源码哈希。冻结配置、原源文件及历史材料未改。
2. 使用原始 seed 和原始 election-ID domain 重建认证 DKG、注册关闭及 32 叶树。
   核对公钥、KeyCore、pre-policy、prekey context、DKG session、final context、
   注册根和注册关闭 digest 均与 D0 相同；前八个地址及 25-field 矩阵完全相同。
3. 因旧 PK 未保存，按原 setup seed 重建一次 Groth16 参数。实际重建 Ark VK
   hash 匹配，转换后的 bellperson VK 全部 wire 字节也匹配，不只是宽度相同。
4. 按原 SRS seed 和 size=32 重建研究 SRS，公共 SRS hash 匹配。
5. 上述检查全部通过后，才规范解码并复用 D0 的八份真实证明。只为 indices 8–15
   （第 9–16 人）新生成八份证明；全部 16 份分别在 arkworks 和 bellperson 中验证。
6. 使用一个 size-16 specialize 结果，真实聚合并验证整个 `16 × 25` 矩阵。
   唯一负例保留聚合证明，只改第 16 人 nullifier 的第一个字节并重建匹配的新
   transcript；整个矩阵验证拒绝，变化仅为零基 row 15 / field 7。

兼容性结果和证明来源逐项记录于原始 JSON。复用的八份证明没有重新 Prove，
本次八个新 Prove 的计时也没有混入历史八份的生成时间。

| 兼容项 | 本次实际结果 |
|---|---|
| 冻结配置及 D0 实测源码哈希 | 全部匹配 |
| DKG 公钥、上下文、注册根及关闭 digest | 全部匹配 |
| 前八个地址及公开输入矩阵 | 全部匹配 |
| Ark VK SHA-256 | `6d00cab6cd02e9926bd8ef05f8e782fbf1f32b5993c134699155accf7b37aa98`，匹配 |
| bellperson VK SHA-256 / 原 wire 字节 | `0b189651c3b32d56251afc5e3ced70b670e0f56a6788d5b1cc6c127ce829ce4c`，匹配 |
| size-32 研究 SRS hash | `3a8a75be96e4150c757b85f682be37d4e502e8b45a17943171bd0bbbdfd605aa`，匹配 |

## 本次真实测量

| 项目 | 实测 |
|---|---:|
| 源证明组成 | 8 份复用 + 8 份新生成 = 16 份不同证明 |
| 每票约束 / 公开输入 / witness 变量 | 1,076,095 / 25 / 1,059,760，与 D0 相同 |
| 认证 DKG 重建 | 11.544 ms |
| 一次 Groth16 Setup 重建 | 11,005.745 ms |
| 八份旧证明规范解码及 round-trip 检查合计 | 1.833 ms（不是 Prove） |
| 第 9–16 人新增 Prove | 7,056.792 / 6,941.084 / 7,027.424 / 7,332.364 / 7,274.077 / 7,385.616 / 6,619.395 / 6,712.637 ms |
| 八次增量 Prove 合计 | 56,349.388 ms |
| Ark VK 预处理 | 0.995 ms |
| VK 转换及 bellperson 预处理 | 4.294 ms |
| 16 份证明和公开输入转换 | 1.871 ms |
| size-32 研究 SRS 重建 / size-16 specialize | 4.620 / 10.877 ms |
| bellperson 16 份逐票 Verify 合计 | 6.917166 ms，通过 |
| 真实 B=16 聚合生成 | 28.524625 ms |
| 完整 B=16 Aggregate Verify | 4.990667 ms，通过 |
| 修改第 16 人 nullifier 后 Verify | 4.788292 ms，拒绝 |
| 原生直接累计及状态相等检查 | 0.021041 ms，96 次群点加法，通过 |
| 16 份单票证明 / aggregate | 3,072 / 14,164 B |
| 公开输入矩阵 / 可导出的外部 transcript | 12,800 / 12,910 B |
| 单票证明＋矩阵 / aggregate＋矩阵 | 15,872 / 26,964 B |
| 压缩 PK 大小（未保存） / 压缩 Ark VK | 355,012,368 / 1,592 B |
| 进程累计峰值 RSS | 2,911.922 MiB |
| 不含编译、测至归档前的流程耗时 | 91,769.038 ms |

同次 bellperson 验证中，聚合的 4.991 ms 小于逐票合计 6.917 ms，样本比值约
1.386；但这是单次功能样本，未进行独立重复、专门预热或随机化，不能据此宣布
稳定加速、统计显著优势或交叉点为 B=16。聚合生成另需 28.525 ms，证明大小仍
明显大于 16 份原证明，不存在本次证明压缩收益。

所有 16 份源证明也在 arkworks 中单独验证，原始 JSON 保存其各次实际时间；
不将 arkworks 与 bellperson 验证混合比较。Setup/SRS 重建是未保存研究参数导致
的复现开销，单列，不计入在线验证。历史八份 Prove 时间只作来源记录，未重跑，
也没有把历史与本次加总冒充同轮 16 份 Prove。

直接累计仅测原生算术及相等检查，不包含注册、batch-root、nullifier 或结算状态
检查。上述字节是 proof＋一份矩阵，不重复加计已编码密文，也不再次传输可导出的
transcript；缓存的 VK/SRS、完整 DA/交易 framing 未计入。完整 DA、calldata、
EVM Gas 保留 null。RSS 是整个重建/增量证明进程的累计峰值，不是独立聚合峰值。

## 实际执行命令与检查结果

```bash
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 fmt --all
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none \
cargo +1.99.1 clippy -p vv-friendly-crypto -p vv-friendly-dkg \
  --all-targets --release --locked --offline -- -D warnings
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none \
cargo +1.99.1 test -p vv-friendly-crypto -p vv-friendly-dkg --lib --release --locked --offline
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none \
cargo +1.99.1 run --release --locked --offline \
  -p vv-friendly-dkg --bin friendly-g1d1-uniform-batch16
```

最终 Release 编译、全 target 严格 Clippy、格式检查及 43 项库回归全部通过
（32 + 11，无失败/忽略/过滤）。开发期首次编译发现公钥编码返回 Vec 与固定摘要
数组不能混在同一比较数组，已在新 runner 中改为独立字节比较；没有修改编码或
密码学库。实际重型 B=16 验收只运行一次，兼容检查、16/16 源证明、聚合正例、
直接累计均通过，唯一故意篡改负例拒绝；没有失败的重型样本被删除或覆盖。

## 归档与复现

- 原始结果：`experiments/g1_d1_raw/uniform_depth5_batch16.json`，SHA-256
  `0d6abd826a375a227aeecd97c083de50c05585943356f36e14f9bf964c8ea5b5`。
- 新公开材料：`fixtures/snarkpack_g1d1_depth5_batch16_public_v1.json`，SHA-256
  `ef1dbdc30c9d70d358deb31a19125681044eb2afa7ed107cd39f702c85567f14`。
- D0 原始结果/fixture 仍分别为
  `d62977c93e71dd2c0493dee221c50692995b13197baba0dead18e1ce6a9ee088` /
  `a4ae00a6bf322cbeba70050e4583f297d3fd9a83e6a8cea793b1366f97cef062`，未改写。
- 环境：Apple M4、32 GiB RAM、macOS aarch64、Rust 1.99.1、Release opt-level=1、
  strip=none、构建并发 2。源码、配置、VK/SRS 及基准材料 hash 均在原始 JSON。

复现使用同一命令，但须指定新输出路径：

```bash
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none \
cargo +1.99.1 run --release --locked --offline \
  -p vv-friendly-dkg --bin friendly-g1d1-uniform-batch16 -- \
  --output experiments/g1_d1_reproduce/uniform_depth5_batch16.json \
  --fixture-output fixtures/snarkpack_g1d1_depth5_batch16_reproduce_v1.json
```

公开 fixture 保存完整 16 证明池、来源标记、B=16 aggregate、相同 VK、size-16
verifier SRS、公开矩阵/正反例 transcript 及密文累计结果。不保存 PK、prover SRS、
选民秘密、weight opening 或 Trustee 份额。确定性研究 seed 和 fake SRS 不构成
生产可信设置或真实投票隐私保证。

## 验收后的停止点

G1-D1 验收通过后停止。下一项仅计划 G1-D2：在同一冻结套件下复用匹配的 B16
证明池，只新增第 17–32 人证明；本轮没有实现它。正式重复统计/自动绘图、W32
状态证明、P3 v2 全子集、同一冻结目标的披露比较和完整结算仍待后续实施。
软件 Miller 路线保持冻结，完整 SnarkPack EVM 和 Proposed Gas 仍未测量。
