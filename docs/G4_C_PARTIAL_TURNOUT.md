# G4-C：P3 部分投票正确性修复（tally relation v2）

状态：`PASS_G4C_REAL_GROTH16_PARTIAL_TURNOUT_TALLY_V2`。2026-10-28 已真实执行完成。

## 本轮只解决什么

论文 `BoundedTallyCorrect` 不要求全部注册选民投票。旧 P3-C1 共用关系却要求
`sum(t_j) = W_registered`，因而拒绝合法的部分投票。本轮只修正这个语义错误，
不扩展批次大小、Trustee 子集矩阵、SnarkPack EVM 或完整结算合约。

三个 P3-C1 变体及其上层 P3-D0 均改为：

```text
0 <= t_j <= W_registered
plaintext_j = t_j * G
sum(t_j) + private_u64_slack = W_registered
```

Slack 在电路内部进行 64 位分解，并非未约束的 host 计算结果。三个 tally 和
slack 的总和小于 `2^66`，远小于 R1CS 字段模数；等式因此也是非负整数等式。
生成 witness 时用 `u128` 求和，越界时仍由电路拒绝，不能利用 `u64` 回绕。
不新增公开输入，不公开实际投票总权重或 slack。

## 最小真实实验

使用同一轮真实 5/3 GJKR-DKG、同一个 RA 注册关闭证书、同一组注册权重
`(3,2,1,4)`、公开注册总权重 `10`。测试 fixture 的预期值如下；这些值用于
说明固定科研用例，并不是向真实选举公开 tally 的接口。

| 用例 | 实际加密并累计的选票 | 内部预期 tally | 公开 winner |
|---|---|---|---|
| 满额投票 | 四张，选择 `(1,2,1,3)` | `(4,2,4)` | `1`（平票取最小编号） |
| 部分投票 | 只取前两张，选择 `(1,2)` | `(3,2,0)` | `1` |

两例均由 Trustee `{1,2,3}` 生成真实 CP 和私有 package，并原生验证、门限解密。
两例完整 R1CS 的约束数、公开输入数量和 witness 数量一致；实际公开输入向量
还必须逐项等于电路的 instance assignment。**公开输入数量相同不表示值相同**：
冻结密文、CP transcript 和 package commitment 随用例变化。

只执行一次全新 Groth16 setup，并用同一套新参数串行生成、验证两份证明。
两个必要负例是：

1. 在部分投票有效证明上仅把公开 winner 改为 `2`，Groth16 验证拒绝。
2. 保留部分投票 CP 材料和组点，只将独立公开 bound 改为 `4`；每个 tally 仍不
   大于 `4`，但总和为 `5`，完整 R1CS 拒绝。这是总和约束测试，不是有效注册状态。

轻量隔离测试另覆盖零票、整数回绕和错误组点；**没有声称生成了完整零票选举证明**。
不运行恶意 Trustee/重放/故障恢复矩阵，也不重复已有重型 CP 测试。

## 实测结果

| 指标 | 满额投票 | 部分投票 |
|---|---:|---:|
| 完整 R1CS 满足 / 公开输入逐项匹配 | 通过 / 通过 | 通过 / 通过 |
| 约束数 | 6,232,037 | 6,232,037 |
| 公开输入 / witness 变量 | 60 / 6,005,170 | 60 / 6,005,170 |
| R1CS 合成及检查 | 14,964.562 ms | 13,986.579 ms |
| 真实 Groth16 Prove | 46,432.630 ms | 43,131.654 ms |
| 真实 Groth16 Verify | 5.649 ms | 5.884 ms |
| 压缩证明 | 192 B | 192 B |
| 证明验证 | 通过 | 通过 |

共用的新 setup 实测 `68,220.170 ms`；两用例使用同一 VK，其 SHA-256 为
`734d5a6b573b500de5b64dc90fced31036d15d26fda3118ce21e45da6326b238`。
PK 压缩大小 `1,843,909,056 B`（只在内存测量，不保存），VK 为 `3,272 B`。
进程最终累计峰值 `9,250.156 MiB`。错误 winner 验证拒绝，耗时 `6.310 ms`；
总和越界的完整 R1CS 不满足，且各单项仍在该负例 bound 内。

环境：Apple M4 / macOS aarch64，Rust `1.99.1`，Release `opt-level=1`，
`CARGO_PROFILE_RELEASE_STRIP=none`。基准提交 `69c7199`，`git_dirty=true`；
没有退回附件中的旧提交，也没有宣称本次是干净提交的独立复现。
原始 JSON 的 SHA-256：
`4979a9dc777f39a916711d85ab5e75bca48481bb066cd7b5fdaa1073290d8c1d`。

## 版本与参数边界

- 新关系：`VV/JUBJUB/P3/BOUNDED-TALLY/v2`。
- 新实验配置：`configs/protocol_manifest.JUBJUB-P3-WEIGHT-V2.json`，协议版本
  `VERIVOTING-JUBJUB-PROTOTYPE-G4C-3.0`，公开布局 `VV/JUBJUB/G4-C/INPUT-LAYOUT/v2`。
- 官方 Manifest 和旧 Jubjub prototype Manifest 保持不变；新配置只描述本次实验，
  不代表生产 Suite 或新链上能力已获验收。
- 即使公开输入仍为 60 个，R1CS 已变化，旧 proving/verifying key **不兼容**。
  历史 P3-C1/D0/D1、G4-A/B、G5-B 原始数据保持不变，仅适用于旧 exact-sum 关系。
- 旧 P3 实验入口现在拒绝缺少 v2 标识的旧配置，以免悄悄覆盖历史结果。历史复现
  应使用对应旧提交；当前 v2 复现使用下方新入口和全新输出路径。
- 当前仅对 `{1,2,3}` 的 v2 正例生成证明，不能把旧 v1 的第二子集测量算作 v2 覆盖。
- 确定性公开 seed 仅用于功能实验；不是生产可信设置或真实选举隐私保证。

## 复现

```bash
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none \
cargo +1.99.1 run --release --locked --offline \
  -p vv-friendly-dkg --bin friendly-g4c-partial-turnout -- \
  --output experiments/g4_c_reproduce/partial_turnout_groth16.json \
  --fixture-output fixtures/jubjub_g4c_partial_turnout_reproduce_v2.json
```

输出路径必须不存在。新 runner 不读取旧 PK/VK，也不会覆盖旧 G4/G5 fixtures。
首次验收结果归档在 `experiments/g4_c_raw/partial_turnout_groth16.json`，公开验证
材料在 `fixtures/jubjub_g4c_partial_turnout_v2.json`。后者使用 arkworks 压缩编码，
**不是 EIP-2537 calldata**；不含 tally、slack、解密份额、私有 package、opening 或 PK。

`shape.witness_ms` 包括约束合成、finalize、满足性及公开输入 packing 检查，
不是仅 witness 赋值耗时。RSS 是当前进程累计峰值，并非各用例独立峰值。
两份证明是两个正确性用例，不是性能重复样本，不据此声称加速或统计显著性。
记录保留真实 `git_dirty`、Git SHA、源文件 SHA-256、工具链和硬件，不伪称干净复现。

## 本轮代码验收

- 新增 6 个轻量隔离测试通过。
- Workspace Release 回归测试通过；只按既定范围过滤
  `real_dalek_equation_satisfies_non_native_r1cs_and_mutation_fails` 这一高内存测试。
- `cargo +1.99.1 fmt --all -- --check` 通过。
- 两个受影响 crate 的 `cargo +1.99.1 clippy ... --all-targets --release --locked --offline -- -D warnings` 通过。
- 上述测试不能替代真实证明；真实证明的两个正例和两个负例已单独实测并归档。

## 下一步（尚未运行）

只将新 v2 证明和新 VK 接入**已有**本地 Groth16 EVM verifier，做一次有效证明和
一次错误 winner 验收。不能复用旧 G5-B VK 或把其 `1,331,473 gas` 当作 v2 测量。
这不解冻 B23 已冻结的固定 20M 软件 Miller/SnarkPack 路线，也不等于实现完整结算。
