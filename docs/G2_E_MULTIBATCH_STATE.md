# G2-E：两个连续 B=4 批次的状态累计

状态：`PASS_TWO_CONSECUTIVE_B4_TRANSITIONS_AND_NATIVE_CHECKPOINT`。实际开始：`2026-10-30T08:41:35Z`。
本轮落实用户四步路线的第一步；没有同时修改 P3、曲线、哈希、Merkle 深度或合约。

## 1. 真实来源与复用口径

- 基点 HEAD：`61193416f36a4bb4132091a10259ced1282d5978`，当前分支 `experiment/g2d-w32-b4-state`。
- 新入口执行时未提交，`git_dirty=true`；raw 保存当时 git status、八项源码 SHA-256 和八项输入产物 pin。
  测量前后逐文件 hash 和 HEAD 相同。基点不代表已包含新入口；复现须使用源码 hash 相同的版本。
- 配置：`configs/protocol_manifest.JUBJUB-G2E-MULTIBATCH-B4.json`。
- 认证来源：D2 D5/W32 同一 32 选民证明池，选择 indices 0..3 和 4..7；八份原单票证明均真实验证。
- 复用八份选票证明；没有新 DKG、选票 Setup 或选票 Prove。
- 复用 G2-D 第一批 aggregate 和第一批状态证明，仅新增第二批 B4 aggregate 和第二批状态证明。
- G2-D 未保存 PK，因此按原 seed 重建一次研究 Setup；完整 VK 字节必须相同才复用首批 proof。
  Setup seed 与新第二批 proof seed 均保存在 raw；参数和 PK 仅留内存，不保存。

原本计划的“G2-E 单个 B8 电路”没有执行；按用户本次优先级将本轮 G2-E 明确定为两个连续 B4。
累计八票不等于实现了一个 B8 状态电路。

## 2. 本次实际证明的连续关系

1. 固定同一 eid、final context、region=4、公钥、key digest 和 D2 冻结注册根；八个地址/nullifier 不同。
2. 两批记录都逐字段等于各自已认证的 25-field 矩阵。Root_v2 的 batch_index 分别为 1、2。
3. Setup/Prove 之前，本地固定这两个有序 accepted roots；之后不得删除、重排或替换。
4. 第一批由零密文 S0 得到 S1；第二批从 S1 得到 S2，不把 old_state 重置为零。
5. 真实检查第二批非零 old_state R1CS 满足、37 个公开输入与实际 instance assignment 一致。
   全量 A/B/C 矩阵 SHA-256 与 G2-D 完全相同，而非只比较约束数。
6. 重建原 Setup 并比较完整 VK；逐批执行 SnarkPack V2 和状态 Groth16 Verify，通过后更新 next/state/spent。
7. candidate-major 独立参考不调用生产 fold/Ciphertext::add；S2 的三坐标 c0/c1 全部等于一次累计八票的结果。
8. 第一个检查点 next=2、spent=4；第二个 next=3、spent=8。公开 fixture 在独立进程再次完整重放通过。

状态关系仍是原 `VV/JUBJUB/G2-C/STATE-B4-LAYOUT/v1`，2,743,666 约束、37 inputs、2,693,602 witnesses。
通用 B4 电路本身不是新的 W32/深度范围电路；选票资格和冻结注册根由认证来源及接收层相等检查绑定。
SnarkPack 不在状态 R1CS 内递归验证。

## 3. 仅保留必要的拒绝用例

| 用例 | 真实结果 | 检查层 |
|---|---|---|
| 先提交拥有真实 proof 的第二批，跳过首批 | NextIndex 拒绝，状态/spent 不变 | 原生接收层 |
| 第一批接受后再次提交原第一批 | NextIndex 拒绝，状态/spent 不变 | 原生接收层 |
| 第二批 old_state 改为零 | OldState 拒绝，状态/spent 不变 | 原生接收层 |
| 同一第二批 proof 对零 old_state 公开语句验证 | Groth16 拒绝 | 密码学验证器 |

另外两项小型接收层单元测试覆盖“验证失败不写状态”和“不同批次根下重复 nullifier 拒绝”；
其成功回调是 bookkeeping 单元测试，不冒充密码学证明。上述两个正例则都执行了真实完整验证器。
没有给无效状态另行生成证明，没有重跑 G2-D 的遗漏攻击矩阵。

## 4. 一次真实 Release 测量

| 指标 | 实测 |
|---|---:|
| 重建原状态 Setup（离线） | 25314.304084 ms |
| 仅第二批新状态 Prove | 16917.430791 ms |
| 仅第二批新 SnarkPack aggregate | 12.321125 ms |
| 第二批 R1CS 合成/packing/满足性 | 4375.161292 ms |
| 第二批全矩阵指纹（额外） | 2157.055667 ms |
| 第一批 / 第二批 SnarkPack Verify | 6.842292 / 8.751791 ms |
| 第一批 / 第二批 State Verify | 2.566542 / 3.004792 ms |
| 第一批 / 第二批完整接收检查+双验证+写内存 | 9.462042 / 11.817333 ms |
| 第一批 / 第二批独立 Direct Fold 比较 | 0.006167 / 0.032042 ms |
| 每批状态 proof / 状态公开输入 | 192 / 1184 B |
| 每批 aggregate / 选票矩阵 | 8212 / 3200 B |
| 状态 PK 压缩大小（仅测量，不保存） | 847800624 B |
| 累计进程峰值 RSS | 5793.875 MiB |

这里是真实接收函数的单次 elapsed；包含 preflight/root 检查、两个验证器及内存更新，不包括 Setup、
Prove、输入 JSON 解析、源单票验证或 SRS 重建。Direct Fold 是额外独立算术检查，不是另一条完整接收事务。
RSS 包含关系指纹、Setup 和 Prove，不是阶段独立峰值。仅一个功能样本，不据此作性能优势或统计结论。

## 5. 命令与公开复现

均使用 CARGO_BUILD_JOBS=2、CARGO_PROFILE_RELEASE_STRIP=none；Release 实际 opt-level=1，locked/offline。
完整命令、退出码、wall time 见 `experiments/g2_e_raw/commands.json`，工具链/硬件/二进制 hash 见 environment.json。

```bash
cargo +1.99.1 fmt --all -- --check
cargo +1.99.1 clippy -p vv-friendly-crypto -p vv-friendly-dkg --all-targets --release --locked --offline -- -D warnings
cargo +1.99.1 test -p vv-friendly-crypto -p vv-friendly-dkg --lib --release --locked --offline
cargo +1.99.1 test -p vv-friendly-dkg --bin friendly-g2e-multibatch-state --release --locked --offline
cargo +1.99.1 build -p vv-friendly-dkg --bin friendly-g2e-multibatch-state --release --locked --offline
cargo +1.99.1 run --release --locked --offline -p vv-friendly-dkg --bin friendly-g2e-multibatch-state
# 只读取公开材料：零 Setup/Prove；output 必须是新路径
cargo +1.99.1 run --release --locked --offline -p vv-friendly-dkg --bin friendly-g2e-multibatch-state -- \
  --verify-fixture fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json \
  --output experiments/g2_e_replay/public_replay.json
```

43 项库回归 + 6 项新入口测试 = 49 项通过；fmt/严格 Clippy/Release 构建均通过。
开发时修正了一个 Clippy 风格提示，之后才进行正式重型运行；没有丢弃或覆盖失败的重型样本。
完整复现可传 `--output NEW_JSON --fixture-output NEW_FIXTURE`，现有产物会拒绝覆盖。

## 6. 原始产物与 SHA-256

- 原始结果：`experiments/g2_e_raw/multibatch_state_b4.json`，`01582f079b95d317c728cca2a62c29939a03fd8425e05edd564c721ba9acda51`。
- 公开 fixture：`fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json`，`42e68fa9bee72fb77e47c292dd3e65386cf4a348192c1ee0a9731cfe2f5eff19`。
- 配置：`69037c5cd893f32fc8c7a06e13077f63b96f70b60513efe619f5e4fa0190b53b`。
- 状态 VK：`92871cac5c3452ab0a1647939507d7b041dcccf189e3b60ea2b19c1d6e1e1856`，与 G2-D 完整字节相同。
- 非零旧状态的完整矩阵：`361195013588cde48bf43e1c5b0071f9cbe8462675246fe3d6f88dec41c6315e`。
- 本地有序 accepted schedule JSON：`97192e74b0ec2f66e38552516fd3489aa2c43c3b265d6d59c15e7f4613f75e6b`；这是 serde_json 实际序列化字节摘要，不是新协议级 Root 编码。
- batch 1 / 2 Root_v2：`91f6c687850b799e58e413c6c9c3b67e1acb1f2cb8a03e2b4d88a3729a9a2134` / `4e3a7649aa9d7455509a0b052d85c6745b38cb1f2416045c08ffd25094837fcf`。

fixture 含两份状态 proof、共同状态 VK、两份实际 aggregate、公开 ballot VK/verifier SRS、八份原单票 proof、
矩阵/记录和前后密文状态；不含 PK、选民秘密、Trustee 份额、私有 package 或 tally。
独立 Node 审计另行重建两个 Root_v2、LE/BE 打包、37 个公开字段和 SnarkPack transcript；公开 Rust 重放实际验证密码学证明。
G2-D 的 10 项和 G1-E 的 228 项历史 SHA 清单保持通过。

## 7. 四步路线与边界

1. **本轮完成**：两个连续 B4 原生状态转换及最小接收检查点。
2. **下一步**：完整 P3 tally-v2 的十种 3-of-5 子集，同一新电路/新 Manifest/同一新 VK；不能将已有原生十子集测试算作完整 P3。
3. **之后**：把本轮冻结 S2、同一认证 DKG、公钥/注册关闭记录、真实 CP/package、门限解密及 winner proof 串为同一个 W32 目标。
4. **最后**：真实合约固定状态、写一次结果及防重放，然后才做完整链上/Direct Fold 公平比较。完整 SnarkPack EVM 仍 BLOCKED，不能用独立状态/P3 verifier Gas 冒充完整结算。

接收层仅为单进程原生内存串行更新，未测试崩溃持久化或并发；本地接受根计划不是独立 RA/intake log，
不保证聚合器接收的每张票都获纳入。研究 deterministic setup/fake SRS 不是生产设置。
没有新增 B8 状态电路、P3 十子集、EVM、Miller 重试、规模扫描或图表。Gas/calldata 为 null，不是零。

