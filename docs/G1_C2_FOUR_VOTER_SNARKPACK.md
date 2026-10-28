# G1-C2：四名不同选民、同一区域的真实 B=4 聚合

状态：`PASS_G1C2_REAL_SNARKPACK_V2_FOUR_DISTINCT_SAME_REGION_VOTERS_NONPRODUCTION_SRS`。
2026-10-28 已完成一次真实 Release 功能验收，不是规模矩阵或正式性能重复实验。

## 本轮范围

独立入口 `friendly-g1c2-batch4`，没有修改原 `ballot_cvote_b1.rs`、G1-C1 B=2 原始
结果或旧 fixtures。使用独立配置 `configs/protocol_manifest.JUBJUB-G1C2-B4.json`，
不将官方或旧 prototype Manifest 的全局 batch-size 能力升级为 4。

实际执行顺序：

1. 冻结注册规则与 pre-policy，进行一轮真实认证 5/3 GJKR-DKG，验证证书及公钥与
   已认证系数的绑定。最终 context 绑定该 KeyCore；四张票共用同一公钥和上下文。
2. 注册关闭前，用既有 HMAC PRF 生成地址，再按既有区域函数筛选 region 4 的四名
   不同选民。各人的候选 seed 尝试次数为 `1,6,1,8`；没有手动覆盖区域位。
   256 次上限只是测试 fixture 的采样预算，不改变协议哈希算法或电路。
3. 注册权重 `(3,2,1,4)`，选择 `(1,2,1,3)`，RA 关闭并认证真实四叶、深度 2 树，
   注册总权重为 10。为各人建立各自的 Merkle 路径及独立加密随机数。
4. 四个完整 `C_vote` R1CS 分别满足，公开输入逐项等于 R1CS instance assignment，
   并与现有公开 ballot-record 编码一致。每票 854,454 约束、25 个公开输入。
5. 一次新 Groth16 setup 后，串行生成四份独立证明，在 arkworks 和转换后的
   bellperson 中逐个验证；四个地址、nullifier、密文向量、证明均不同。
6. 使用 size-4 的实验 `setup_fake_srs` 和真实 SnarkPack V2 聚合，验证整个四行
   公开输入矩阵。唯一新增负例：仅修改第四人的 nullifier，重建匹配的新 transcript，
   使用原 aggregate 验证全部四行，结果拒绝。变化仅在第 4 行的 field index 7。

认证 DKG/RA 的检查在原生 runner 中执行，不是新增的电路内证书验证或完整系统结算。
原 ballot 电路、3 名候选人、4 个区域、4 叶/深度 2 及固定权重上界 10 保持不变。

## 真实结果

| 项目 | 实测 |
|---|---:|
| 认证 DKG | 25.738 ms |
| 一次 ballot Groth16 setup | 6,973.912 ms |
| 四次独立 Prove | 4,052.956 / 4,059.996 / 4,020.042 / 4,073.292 ms |
| Prove 合计 | 16,206.286 ms |
| arkworks 逐票 Verify | 2.253 / 2.269 / 2.444 / 2.445 ms |
| 转换为 bellperson | 4.524 ms |
| bellperson 逐票 Verify | 0.424 / 0.466 / 0.433 / 0.437 ms |
| bellperson 逐票 Verify 合计 | 1.760 ms |
| 实验 SRS 生成及 specialize | 5.223 ms |
| 真正的 V2 聚合 | 7.217 ms |
| 完整 aggregate Verify | 3.402 ms，通过 |
| 修改第四人 nullifier 后 Verify | 3.402 ms，拒绝 |
| 四份单票证明合计 | 768 B（4 × 192 B） |
| 聚合证明编码 | 8,212 B |
| 公开输入矩阵 | 3,200 B（4 × 25 × 32 B） |
| 外部 transcript | 3,262 B |
| 进程累计峰值 RSS | 3,026.562 MiB |
| 每票 witness 变量 | 839,832 |
| PK 压缩大小（只在内存测量） | 251,898,000 B |
| VK 压缩大小 | 1,592 B |

本次 B=4 **没有显示证明大小压缩或验证时间优势**：8,212 B 大于 768 B，
aggregate Verify 3.402 ms 大于同次 bellperson 逐票验证合计 1.760 ms，聚合本身还
需 7.217 ms。arkworks 与 bellperson 的验证时间分别报告，不能混用后声称加速。
这些是单次功能用例，不是统计结论，也不证明更大批次一定有或没有优势。

Setup 与在线阶段分开记录。`total_run_ms_excluding_compilation` 为 28,126.280 ms，
包括 DKG、fixture、约束检查、setup 和证明流程，测到归档写入前；不是完整选举端到端延迟。
`shapes[].synthesis_and_check_ms` 包括合成、finalize、满足性和 packing 检查，不是纯赋值时间。

## 归档与复现

- 原始结果：`experiments/g1_c2_raw/snarkpack_distinct_voters_batch_4.json`，SHA-256
  `5958e7368326d4c96b07ef07085440cb114efb96d10cc06c111fdf919e0b5ca9`。
- 公开验证材料：`fixtures/snarkpack_g1c2_batch4_public_v1.json`，SHA-256
  `84e3c7748dffa8eeece45ddfdadfe64fcb4c563a3e549a788b0321744f87d024`。
- 环境：Apple M4 / macOS aarch64、Rust 1.99.1、Release opt-level=1、strip=none。
  基点提交 `b562ead`，实际 `git_dirty=true`；源码、配置及旧 B=2 材料的哈希均有记录，
  不宣称干净提交独立复现。

```bash
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none \
cargo +1.99.1 run --release --locked --offline \
  -p vv-friendly-dkg --bin friendly-g1c2-batch4 -- \
  --output experiments/g1_c2_reproduce/snarkpack_distinct_voters_batch_4.json \
  --fixture-output fixtures/snarkpack_g1c2_batch4_reproduce_v1.json
```

输出必须是新路径。公开 fixture 包含四份源证明、aggregate、VK、verifier SRS、
公开矩阵与记录，不保存 PK、aggregation prover SRS、选民 seed、DKG 私有份额、
投票随机数或权重承诺 opening。公开确定性研究 seed 和 fake SRS 只适合功能复现，
不构成生产可信设置或真实投票隐私保证。

本轮格式检查、入口严格 Clippy 检查通过；两个现有密码学 crate 的 41 项库回归通过。
没有新增故障矩阵，没有 B=4 状态证明、EVM、P3 重跑或规模重复测量。

## 下一步

仅推进 B=8：单独版本化深度 3 的注册树/选票电路、八名不同同区域选民、一个新
setup 和八份新证明，真实 size-8 聚合及最后一人 nullifier 负例。当前深度 2 的
VK 不可冒充支持八叶关系。B=4 状态证明和恶意遗漏用例仍是后续独立任务；冻结的
Miller 路线不解冻，完整 SnarkPack EVM / Proposed Gas 仍未测量。
