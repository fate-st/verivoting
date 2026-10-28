# G1-D0：同一深度 5 / 权重 32 / VK 的 B=2、4、8 原生预实验

状态：`PASS_G1D0_FIXED_DEPTH5_W32_SAME_VK_NATIVE_B2_B4_B8_PILOT`。
2026-10-28 完成一次真实 Release 运行，起始时间 `2026-10-28T13:33:41Z`。
这是调整后实验方案的第一个增量，不是正式重复统计或完整 Proposed 结算实验。

## 固定配置与本次执行量

- 配置：`configs/protocol_manifest.JUBJUB-EVAL-D5-W32.json`。
- 关系：`VV/JUBJUB/EVALUATION/C-VOTE-D5-W32-LAYOUT/v1`，Jubjub / BLS12-381
  Groth16，既有 HMAC-SHA256 PRF、区域规则及 SnarkPack V2 不变。
- 固定 5 名 Trustee、t=2、tau=3、q_cert=3，3 名候选人、4 个区域，注册深度 5。
- 32 名不同单位权重选民经真实 PRF/区域函数筛选进入 region 4。注册前冻结政策，
  执行一次认证 GJKR-DKG、一次 32 叶注册关闭并验证证书；注册总权重和上界均为 32。
  不是沿用无法容纳 32 名正权重选民的旧上界 10，也未手工覆盖区域计算结果。
- 只生成前八名选民的真实选票证明，另外 24 名只注册、不生成证明。各人使用独立
  secret、nullifier、加密随机数和深度 5 路径；选择按注册 fixture 的 1/2/3 循环。
- 八个 R1CS 均满足，实际 instance assignment 精确等于 25-field 公开输入，且与
  state ballot-record 编码相同。只执行一套新的选票 setup；八份证明在 arkworks
  下验证，再转换并在 bellperson 的各规模逐票基线中验证。
- 使用同一八证明池的有序前缀形成 B=2/4/8。各批次内部没有复制证明凑人数；
  跨规模明确复用，共生成八份而非十四份证明。
- 一次 size-32 的研究 `setup_fake_srs`，分别 specialize 到 2/4/8；每个 B 执行
  一次真实聚合及一次完整矩阵验证。唯一负例在 B=8 修改最后一人 nullifier，
  重建匹配的新 transcript，完整验证拒绝。

官方及旧 Manifest、旧入口和历史原始数据未改。新配置的 planned sizes 16/32
只是后续计划，不代表已经测量；本轮也未升级正式协议能力。

## 同一次运行的真实对照

| B | bellperson 逐票 Verify 合计（ms） | Aggregate Verify（ms） | 聚合生成（ms） | 原生直接累计及比较（ms） | 单票证明合计 / aggregate（B） |
|---|---:|---:|---:|---:|---:|
| 2 | 1.329791 | 3.922875 | 5.708416 | 0.003292 | 384 / 5,236 |
| 4 | 1.846958 | 3.970959 | 9.568125 | 0.006459 | 768 / 8,212 |
| 8 | 3.948791 | 5.210167 | 16.832083 | 0.010583 | 1,536 / 11,188 |

三个点确实使用相同选票电路、注册根、认证公钥、上下文和 VK，不再混用历史
深度 2 与深度 3 的测量。VK 预处理不计入表中的 Verify；转换/预处理单独记录。
八份源证明的 arkworks 与 bellperson 压缩 wire 字节逐字节相同，每份为 192 B。

本次三个 B 都没有显示聚合验证时间或证明大小收益。验证速度比仅为该次样本的
约 `0.339 / 0.465 / 0.758`，不是稳定加速比或交叉点结论。测试按 B=2、4、8
顺序执行，无专门 warmup、无随机化、每个 B 仅一次；正式统计仍未进行。

## 直接累计基线的正确含义

每个 B 还实际运行了现有 `fold_state_for_batch`：从零旧状态累计该前缀的全部
真实密文，并比较提交的新状态。分别执行 12、24、48 次 Jubjub 点加法。
表中直接累计列只含原生算术和状态相等比较，预先准备提交状态不计时。

公平的后续比较应为“SnarkPack＋直接累计”与“SnarkPack＋状态证明”，而不是
刻意去掉所有累计检查后宣称只有状态证明能保证一致性。本轮只补齐直接累计的
原生组成成本，未测注册检查、根计算、nullifier 处理、完整接收/结算或 EVM Gas。
不能将旧权重 10 的 G2-C 状态证明时间加进本次权重 32 的数据，冒充同场 Proposed。
这些原生算术时间也不能推算为目标链 Gas 或目标链上的状态证明收益。

## 共同成本与资源

| 项目 | 实测 |
|---|---:|
| 每票约束 / 公开输入 / witness 变量 | 1,076,095 / 25 / 1,059,760 |
| 认证 DKG | 11.463 ms |
| 一次新的选票 Setup | 11,312.937 ms |
| 八次真实 Prove | 7,628.730 / 6,970.812 / 6,929.006 / 6,835.703 / 6,826.820 / 6,739.683 / 7,001.994 / 6,769.657 ms |
| 八次 Prove 合计 | 55,702.404 ms |
| 转换及 bellperson VK 预处理 | 5.497 ms |
| 一次 size-32 研究 SRS 生成 | 5.299 ms |
| B=2/4/8 SRS specialize | 3.712 / 5.008 / 7.132 ms |
| 第八人 nullifier 负例 Verify | 4.456 ms，拒绝 |
| 进程累计峰值 RSS | 3,077.672 MiB |
| PK 压缩大小（只在内存测量） / VK 压缩大小 | 355,012,368 / 1,592 B |
| 不含编译、测至归档前的流程耗时 | 78,993.687 ms |

约束合成检查每票约 1,237–1,294 ms，包括 finalize、满足性及 packing，不是纯
witness 赋值时间。Setup、源证明和进程峰值是共同成本，不为每个 B 重复计费；
也没有将累计 RSS 当成各 B 的独立内存曲线，逐规模独立进程 RSS 保留 null。

通信只报告定义清楚的实际编码：

| B | 字段矩阵（B） | 单票证明＋矩阵（B） | aggregate＋矩阵（B） |
|---|---:|---:|---:|
| 2 | 1,600 | 1,984 | 6,836 |
| 4 | 3,200 | 3,968 | 11,412 |
| 8 | 6,400 | 7,936 | 17,588 |

矩阵已经包含编码后的密文与公开字段，不再另加相同密文制造重复计量。外部
transcript 可由矩阵导出，分别为 1,654 / 3,262 / 6,478 B，不另算第二份传输。
以上只是 proof＋matrix 的明确子载荷，不含缓存的 VK/SRS、交易/DA framing；
完整 calldata、DA、Gas 都未测量，保存为 null，不填 0。

## 可追溯材料与复现

- 原始结果：`experiments/g1_d0_raw/uniform_depth5_native_pilot.json`，SHA-256
  `d62977c93e71dd2c0493dee221c50692995b13197baba0dead18e1ce6a9ee088`。
- 公开 fixture：`fixtures/snarkpack_g1d0_depth5_public_v1.json`，SHA-256
  `a4ae00a6bf322cbeba70050e4583f297d3fd9a83e6a8cea793b1366f97cef062`。
- 冻结配置 SHA-256：`00c3869c4b10a6e48fa5972b2a98767966db2b2fac0de3692eb8cbbc13cafe5e`。
- 同一 Ark VK SHA-256：`6d00cab6cd02e9926bd8ef05f8e782fbf1f32b5993c134699155accf7b37aa98`。
- 公共研究 SRS hash：`3a8a75be96e4150c757b85f682be37d4e502e8b45a17943171bd0bbbdfd605aa`。
- 环境：Apple M4 / 32 GiB RAM / macOS aarch64、Rust 1.99.1、Release opt-level=1、
  strip=none、构建并发 2。基点 `f080b7b7abe466f510ce658feecf358ec7860d0c`，实际
  `git_dirty=true`；源码及配置哈希已记录，不声称干净提交复现。

```bash
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none \
cargo +1.99.1 run --release --locked --offline \
  -p vv-friendly-dkg --bin friendly-g1d0-uniform-pilot -- \
  --output experiments/g1_d0_reproduce/uniform_depth5_native_pilot.json \
  --fixture-output fixtures/snarkpack_g1d0_depth5_reproduce_v1.json
```

输出必须是新路径。fixture 保存八份源证明、三份 aggregate、同一 VK、各 B 的
公开 verifier SRS、矩阵、匹配 transcript 和密文累计结果。不保存 PK、prover SRS、
选民秘密、权重 opening 或 Trustee 份额。研究 seed 与 fake SRS 不适于生产隐私
或可信设置；这是实现功能证据，不是完整隐私/健全性证明。

格式检查、两个受影响 crate 的全 target 严格 Release Clippy 和 43 项库测试通过。
没有修改密码学库算法，没有新故障矩阵，没有复跑 G2-C、P3 或 EVM，也没有正式
统计图；已有汇总脚本直接读取这份新 JSON 生成当前预实验表。

## 下一项最小任务

只推进 G1-D1：同一冻结 D5/W32 配置的 B=16 功能增量。保留本次八份公开证明；
因未保存 PK，需要按相同 seed 重建参数，并先核对 VK、上下文、注册根、前八行
矩阵及 SRS hash 完全一致，再复用旧八份证明，仅新生成第 9–16 份。只做一次
B=16 聚合/逐票对照和一个最后 nullifier 负例，通过后再另做 B=32。
本轮未执行该任务；正式重复统计、W32 状态/P3、完整结算与论文图表仍未完成。
