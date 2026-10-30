# G1-D2：冻结 D5/W32 套件下的真实 B=32 聚合

状态：`PASS_G1D2_FIXED_DEPTH5_W32_B32_REUSED16_NEW16_SAME_VK_SRS`。
真实 Release 开始时间：`2026-10-30T06:20:13Z`（北京时间 2026-10-30）。
本轮只做 B32 功能验收，完成后停止；没有开发 B64、正式统计/绘图、W32 状态、P3 或 EVM。

## 真实源码版本与干净工作区

开始时远端与本地均为干净的 `01d4067fbac34c38aca8b8314ac5f91c2a2ade4d`。
该提交尚无 B32 入口，因此不能在新增代码后仍声称以 01d4067 干净运行。
本轮先新增入口、完成编译/Clippy/43 项回归并提交源码，再执行实验：

- 实际测量源码提交：`8e72034aa6b97c52ad23adccdff563cc27578001`。
- `git_dirty=false`：在进程开始、创建输出之前读取真实 Git 状态；新入口要求源码已提交且工作区干净，否则直接停止。
- `git_clean_before_archival=true`：全部计算结束、写出任何结果之前，再检查 HEAD 未变且工作区仍干净。
- 新生成 JSON、fixture、日志和之后的文档更新会产生正常的新改动，不能声称它们已存在于上述源码提交，也不将结果文件引起的改动伪装为源码 dirty。

未修改 D0/D1 源文件、历史 JSON/fixture、冻结 Manifest 或密码学库。独立入口为
`crates/vv-friendly-dkg/src/bin/g1d2_uniform_batch32.rs`，二进制为
`friendly-g1d2-uniform-batch32`；仅增加 Cargo 注册，没有更改电路布局或算法。

## 兼容检查与必要正反例

1. 固定校验 D1 原始 JSON/fixture 的 SHA-256，以及 D1 已记录的各个原始源码 hash。
2. 保持 D0 的全部研究 seed、Setup/SRS seed 和原始 election-ID domain，重建认证 DKG、32 叶注册关闭及深度 5 注册路径。使用全部 32 名不同单位权重选民，均由原 PRF 真实筛选入 region 4，总权重与上界 32。
3. 核对公钥、pre-policy、prekey、session、KeyCore、final context、注册根/关闭 digest 与 D1 完全相同；前 16 地址和 `16×25` 公开输入矩阵逐项相同。
4. 旧 PK 未保存，因此只重建一次 Setup；核对 Ark VK hash 及完整 Bellperson VK wire 字节。重建 size-32 研究 SRS 并核对 hash。任一不符即停止，不能重置密钥绕过。
5. 全部兼容检查通过后，规范解码/round-trip 并复用 D1 池的 16 份真实证明（其来源是 8 D0 + 8 D1）。只为第 17–32 人新生成 16 份证明，不能称为重新生成 32 份。
6. 检查地址、nullifier、密文和证明不重复；32 个 R1CS 的约束、输入 packing 均相同且满足，所有 32 份证明在 arkworks 和 Bellperson 中分别验证通过。
7. 完整 `32×25` 矩阵的一次真实 SnarkPack V2 聚合和完整验证通过；32 票原生直接累计及状态相等检查通过。
8. 唯一负例：只翻转第 32 人 nullifier 的首字节，变化仅在零基 row 31 / field 7；重建对应完整 transcript，保留原聚合证明，完整验证拒绝。

## 实际计时与大小

| 项目 | 真实测量 |
|---|---:|
| 来源组成 | 16 份复用 + 16 份新生成 = 32 份不同证明 |
| 每票约束 / 公开输入 / witness 变量 | 1,076,095 / 25 / 1,059,760，与 D0/D1 相同 |
| 认证 DKG 重建 | 12.075 ms |
| 一次 Groth16 Setup 重建 | 10106.341 ms |
| 16 份旧证明解码及 round-trip 合计 | 3.323 ms（不是 Prove） |
| 16 次新增 Prove 合计 | 106863.164 ms |
| Ark VK 预处理 | 2.251 ms |
| VK 转换与 Bellperson 预处理 | 4.536 ms |
| 32 份证明与输入转换 | 3.570 ms |
| size-32 SRS 重建 / specialize | 5.026 / 21.242 ms |
| 32 票逐票 Bellperson Verify 合计 | 14.048915 ms，通过 |
| 真实 B32 聚合生成 | 57.087584 ms |
| 完整 Aggregate Verify | 6.942209 ms，通过 |
| 第 32 人 nullifier 负例 Verify | 7.354333 ms，拒绝 |
| 原生直接累计/状态相等检查 | 0.041917 ms，192 次群点加法 |
| 32 份单票证明 / aggregate | 6144 / 17140 B |
| 公开输入矩阵 / 可导出的 transcript | 25600 / 25774 B |
| 单票证明＋矩阵 / aggregate＋矩阵 | 31744 / 42740 B |
| 累计进程峰值 RSS | 3111.375 MiB |
| 不含编译、测至归档前的流程耗时 | 165057.483 ms |

16 次新 Prove 各自时间（按第 17–32 人顺序，ms）：
`7010.866 / 6673.819 / 6711.158 / 6573.681 / 6701.926 / 6539.736 / 6658.291 / 6669.051 / 6747.016 / 6594.803 / 6579.099 / 6678.105 / 6779.716 / 6808.487 / 6594.984 / 6542.427`。

当前 B=2/4/8/16/32 只是共享配置的单次功能样本，来自 D0/D1/D2 不同运行。
B32 本次聚合 Verify 较逐票合计小，不等于稳定加速或统计交叉点；本次样本比值约
2.024，不能作为论文最终统计结论。aggregate 仍比单票证明总量大，另有聚合生成开销。
历史 16 份 Prove 时间只作为来源保存，未重新执行，也不与本次 Prove 加总冒充同轮新工作。
Setup、转换和 SRS specialize 单列，不计入在线验证；不同验证后端不混算。

直接累计只测原生算术和状态相等，不包含注册/nullifier/accepted-root/结算检查。
矩阵已经包含密文编码，不再重复计算密文；transcript 可导出，不作为第二份必传材料。
proof＋matrix 是明确口径的部分载荷，不含缓存 VK/SRS、交易/DA framing。
完整 DA、calldata、Gas 都未运行，保持 null。RSS 是整个重建及增量证明进程累计峰值，不是每规模独立内存曲线。

## 实际命令与日志

下面命令按顺序实际执行。格式、严格全 target Clippy、Release build 均通过；
库测试 32 + 11 = 43 项全部通过，没有失败、忽略或过滤。源码提交之后的重型 B32
正反例验收只执行一次，没有模拟或覆盖旧结果。

```bash
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 fmt --all
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 clippy -p vv-friendly-crypto -p vv-friendly-dkg --all-targets --release --locked --offline -- -D warnings
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 test -p vv-friendly-crypto -p vv-friendly-dkg --lib --release --locked --offline
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 build --release --locked --offline -p vv-friendly-dkg --bin friendly-g1d2-uniform-batch32
git add -- crates/vv-friendly-dkg/Cargo.toml crates/vv-friendly-dkg/src/bin/g1d2_uniform_batch32.rs
git commit -m "Add clean-source same-suite B32 aggregation runner"
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 run --release --locked --offline -p vv-friendly-dkg --bin friendly-g1d2-uniform-batch32
```

完整 stdout/stderr：`experiments/g1_d2_raw/build.log`、`clippy.log`、`tests.log`、`release.log`。
执行时日志先写到仓库外临时目录，避免日志文件本身污染干净源码检查，结束后复制归档。

## 归档与哈希

| 材料 | SHA-256 / 标识 |
|---|---|
| 原始 JSON：`experiments/g1_d2_raw/uniform_depth5_batch32.json` | `1a8c11c29ec89b8cf4a58dc963d83f408cd6f3fef6ef28a91159bb6985952372` |
| 公开材料：`fixtures/snarkpack_g1d2_depth5_batch32_public_v1.json` | `6b9a1ca042e3ab1d637fb3b554029cc7a36a5b867430b2707539d1ea61928596` |
| 冻结 Manifest | `00c3869c4b10a6e48fa5972b2a98767966db2b2fac0de3692eb8cbbc13cafe5e` |
| Ark VK | `6d00cab6cd02e9926bd8ef05f8e782fbf1f32b5993c134699155accf7b37aa98` |
| Bellperson VK | `0b189651c3b32d56251afc5e3ced70b670e0f56a6788d5b1cc6c127ce829ce4c` |
| size-32 研究 SRS | `3a8a75be96e4150c757b85f682be37d4e502e8b45a17943171bd0bbbdfd605aa` |
| B32 aggregate | `857edce6cc6fbd7941e8d5e992bf118b4b1889dbc410a763fd04e717b7106362` |

源码及基准材料 hash 在原始 JSON；原始数据/fixture/全部日志的 SHA-256 另列于
`experiments/g1_d2_raw/SHA256SUMS`。环境：Apple M4 / 32 GiB / macOS aarch64，
Rust 1.99.1，Release opt-level=1、strip=none、构建并发 2。

公开 fixture 包含 32 份证明池及来源、B32 aggregate、相同 VK、size-32 verifier SRS、
公开矩阵、正反例 transcript 和累计密文。不保存 PK、prover SRS、选民秘密、weight opening 或 Trustee 份额。
确定性研究 seed 与 setup_fake_srs 均非生产可信设置，也不提供生产投票隐私保证。

## 复现与停止边界

先切到包含 B32 入口、且源码 hash 匹配的干净提交，指定两个全新输出路径：

```bash
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none \
cargo +1.99.1 run --release --locked --offline \
  -p vv-friendly-dkg --bin friendly-g1d2-uniform-batch32 -- \
  --output experiments/g1_d2_reproduce/uniform_depth5_batch32.json \
  --fixture-output fixtures/snarkpack_g1d2_depth5_batch32_reproduce_v1.json
```

新输出会正常使结果目录出现未跟踪文件；不要在第二次执行前误把它当成干净工作区。
本轮至此停止。下一项只计划固定 B=2/4/8/16/32 的重复测量和自动图表；W32 状态、
P3 v2 十子集、披露比较、完整冻结绑定/结算及 SnarkPack EVM 仍未完成。软件 Miller 路线保持冻结。
