# G1-C3：八名不同选民、深度 3 的真实 B=8 聚合

状态：`PASS_G1C3_REAL_SNARKPACK_V2_EIGHT_DISTINCT_SAME_REGION_VOTERS_NONPRODUCTION_SRS`。
2026-10-28 完成一次真实 Release 功能验收。本轮只实施上传方案的第一个增量，
没有同时开展规模矩阵、B=4 状态证明或额外链上实验。

## 实现与验收范围

独立入口 `friendly-g1c3-batch8`；配置为
`configs/protocol_manifest.JUBJUB-G1C3-B8.json`，选票布局为
`VV/JUBJUB/G1-C3/C-VOTE-DEPTH3-LAYOUT/v1`。

1. 在 DKG 前冻结注册政策及 pre-policy，运行一次真实认证 5/3 GJKR-DKG，检查
   证书、KeyCore 与公钥绑定。八张票使用同一最终 context 和认证公钥。
2. 注册关闭前，使用既有 HMAC PRF 和区域函数筛选八名 region 4 选民，地址均不同。
   八人的候选 seed 尝试次数为 `3,6,16,1,3,5,6,6`，未覆盖区域计算结果。
3. 权重为 `(1,1,1,1,1,1,1,3)`、选择为 `(1,2,1,3,2,3,1,2)`，注册总权重及
   上界均为 10。这是新的八选民 fixture，不是复制旧票凑数。保持上界 10，避免
   同时引入不必要的范围参数变化。注册关闭认证覆盖真实八叶树；各人使用独立
   深度 3 路径、权重 opening 和加密随机数。
4. 八个完整 `C_vote` R1CS 均满足，逐项核对实际 instance assignment 与 25-field
   公开输入；同时核对已有 state ballot-record 的公开字段编码。
5. 对深度 3 关系执行一次全新 Groth16 setup，串行生成八份新证明，并分别在
   arkworks 和转换后的 bellperson 中验证。地址、nullifier、密文向量和证明均不同。
6. 真实生成 size-8 实验 SRS，执行一次 SnarkPack V2 聚合及完整八行矩阵验证。
   唯一负例：保留原聚合证明，只修改第八人 nullifier 的第一个字节，同时重新生成
   匹配的新 transcript。完整验证拒绝；矩阵仅第 8 行的 field index 7 改变。

没有用 host-language 结果替代电路群运算。DKG/RA 证书仍在原生 runner 中检查，
不是电路内证书验证。核对 ballot-record 编码也不等于执行了状态证明。

## 真实测量

| 项目 | 实测 |
|---|---:|
| 每票约束 / 公开输入 / witness 变量 | 928,335 / 25 / 913,142 |
| 认证 DKG | 11.946 ms |
| 一次深度 3 Groth16 setup | 7,730.263 ms |
| 八次独立 Prove | 4,411.214 / 4,273.860 / 4,301.030 / 4,634.713 / 4,887.957 / 4,329.965 / 4,287.827 / 4,360.590 ms |
| Prove 合计 | 35,487.154 ms |
| arkworks 逐票 Verify | 2.607 / 2.273 / 2.401 / 3.311 / 2.383 / 2.699 / 2.305 / 2.419 ms |
| 转换为 bellperson（包括转换后的 VK 预处理） | 5.812 ms |
| bellperson 逐票 Verify | 0.556 / 0.511 / 0.435 / 0.442 / 0.704 / 0.540 / 0.515 / 0.498 ms |
| bellperson 逐票 Verify 合计 | 4.200 ms |
| 实验 SRS 生成及 specialize | 10.021 ms |
| SnarkPack V2 聚合 | 16.975 ms |
| 完整 aggregate Verify | 5.276 ms，通过 |
| 修改第八人 nullifier 后 Verify | 4.681 ms，拒绝 |
| 八份单票证明合计 | 1,536 B（8 × 192 B） |
| 聚合证明编码 | 11,188 B |
| 公开输入矩阵 / 外部 transcript | 6,400 / 6,478 B |
| 进程累计峰值 RSS | 3,304.453 MiB |
| PK 压缩大小（只在内存测量） / VK 压缩大小 | 269,492,400 / 1,592 B |

本次 B=8 没有显示证明大小压缩或验证时间优势：11,188 B 大于 1,536 B，
5.276 ms 大于同次 bellperson 逐票验证合计 4.200 ms，聚合本身另需 16.975 ms。
arkworks 和 bellperson 分别报告，不能混用时间声称加速。这是单次功能结果，
不是统计结论；历史 B=4 使用深度 2，本次使用深度 3，也不是统一电路下的公平规模对照。

八次约束合成与检查分别约 1,069–1,162 ms，包含 finalize、满足性和 packing
检查，不是纯 witness 赋值时间。运行总计 53,748.335 ms，测至归档写入前，包含
DKG、fixture、约束检查、setup 和证明流程，不含编译，不是完整系统结算延迟。

## 保留历史基线的方式

共享 `BallotCvoteCircuit` 现在使用 const-generic 深度，旧默认值仍为 2；新入口
显式使用 `BallotCvoteCircuit<3>`。旧深度 2 的关系、全局默认深度常量、公开输入
顺序未变。新增共用转换/测量模块供新入口使用，不重写已归档 B=2/B=4 入口。
注册关闭原逻辑未改，新增完整二叉树路径 API，并独立测试全部八条路径与认证根一致。

原 B=2/B=4 入口、配置、原始结果和公开 fixture 保持原字节。共享库源码确实因
参数化而修改，不能声称全部旧源码哈希不变；历史源码及测量仍对应原提交。
本次新 VK 不复用深度 2 的密钥，也不将旧结果中的源码哈希改写成当前哈希。
官方 Manifest、原 prototype Manifest、密码学曲线及哈希 Suite 均未替换。

## 归档、验证与复现

- 原始结果：`experiments/g1_c3_raw/snarkpack_distinct_voters_batch_8.json`，SHA-256
  `2d0784367b1d4865e15c7edf4ef74d20d7b66ae5bf51f5882654587b0b988180`。
- 公开验证材料：`fixtures/snarkpack_g1c3_batch8_public_v1.json`，SHA-256
  `81e24abe9d3a908860cf13ca90e5fe24dbea1ed98c56e4d328a06f2fe6a7d760`。
- 环境：Apple M4 / macOS aarch64，Rust 1.99.1，Release opt-level=1，strip=none，
  构建并发 2。基点提交 `69e9d5a235d6a333fc36bba95bb756cf94ba782c`，实际为
  `git_dirty=true`；配置、实测源码及历史材料的 SHA-256 记录在原始结果中。
- `cargo fmt --all -- --check`、受影响两个 crate 全 target 严格 Release Clippy
  通过；两个 crate 的 42 项库测试通过（31 + 11，含一个新增八叶路径正例）。

```bash
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none \
cargo +1.99.1 run --release --locked --offline \
  -p vv-friendly-dkg --bin friendly-g1c3-batch8 -- \
  --output experiments/g1_c3_reproduce/snarkpack_distinct_voters_batch_8.json \
  --fixture-output fixtures/snarkpack_g1c3_batch8_reproduce_v1.json
```

只允许写入新路径。公开 fixture 保存八份源证明、aggregate、VK、公开 verifier SRS、
完整矩阵及变异矩阵，不保存 PK、aggregation prover SRS、选民 secret、权重 opening
或 Trustee 私有份额。确定性研究 seed 和 `setup_fake_srs` 仅适合功能复现，不构成
生产可信设置或实际投票隐私保证。

## 下一项必要实验

按上传方案转向 **G2-C：B=4 状态证明与遗漏第三票负例**，而非立即扩大到 B=16。
使用已保存的真实 G1-C2 四票材料，建立新的 B=4 状态关系和新密钥，核对同一有序
公开矩阵对应的 batch root，证明四张票全部累计。固定四条合法记录和正确 batch
root，只把提交的新状态改为遗漏第三张票后的累计值；预期该状态关系不满足。

本轮没有运行该 G2-C 实验，没有 B=8 状态证明、正式重复性能矩阵或新 EVM 调用。
软件 Miller 固定预算路线保持冻结；完整 SnarkPack EVM、完整结算和 Proposed Gas
仍未完成、不得据此推算。
