# G8-B：原 Trustee 承诺认证与冻结 CT2 绑定

## 验收结论

**G8-B 组件 PASS；完整 G8 仍 PARTIAL。** 本轮严格止于原公开承诺认证/存储，
不开发新 P3 电路、不生成新 CP 包/签名、不重跑 G1-E 或十种子集证明。
新分支 `experiment/g8b-trustee-commitments`，基点
`ac0216105a2e8ed859ea2b136a8a76708177fecf`；实际执行时 `git_dirty=true`，有源码快照/哈希。

## 实现与绑定

- `g8b_trustee_export.rs` 只读取 G6 和 G8-A 公开 fixture，严格解码原五份 66 B 签名，
  通过原 `package.rs::commitment_message(v5)` 和原 Jubjub Schnorr 验证；不持有私钥或 RNG。
  五个身份公钥/verification shares 与 G8-A 匹配，五次原生公开系数多项式核对通过。
- `VeriVotingTrusteeCommitments.sol` 固定原 gate/receiver 地址，重算完整原 KeyCore 字节和 digest，
  认证身份公钥与三个公开系数不可用另一组值替换。所有坐标先经过原规范点/子群检查。
- `bindFrozen()` 必须先通过原 gate 的 DKG/注册认证；原注册 root/close/disclosure 均匹配，
  实际 receiver 已接收两批且冻结。它读取存储中的六个密文点并重算原 CP target，
  保存 CT2、CP target、freeze seal；不是接收任意 caller target 或复制历史 JSON 当状态。
- 每次提交检查 Election ID、final context、冻结 target、唯一合法 Trustee ID，
  在 EVM 执行 `VK_i = A0 + i*A1 + i²*A2` 并与提交份额核对。
  新 `JubjubVerificationShare.sol` 使用原 complete extended Edwards 群运算；
  没有使用未检查的 host 结果替代群运算。
- 消息完整复用原 `VV/JUBJUB/PRIVATE-PACKAGE/COMMIT/v5` 和
  `PRIVATE-PACKAGE-COMMITMENT-SIGNATURE`，绑定 eid、policy-pre、final context、close digest、
  disclosure digest、CP target、Trustee ID、commitment。原消息没有另加 KeyCore 或 seal 字段；
  KeyCore 通过原 final context 及组件显式校验绑定，密文通过 CP target/实际冻结 receiver 绑定。
  不改变原签名消息后重新签名。
- 原 G8-A Schnorr/SHA-512/H2S-v2 验证器保持源字节不变。认证后存储承诺、已核对的公开
  verification share 与 submitted 标记。解密 `TAU=3` 和建钥 `Q_CERT=3` 是不同协议角色。

`requireP3Ready()` 只检查已有三个不同认证公开记录。它**不**验证私密 CP/opening、
不做门限解密、不验证 P3，更没有 Winner 写入功能；这些关系仍必须由后续 P3 证明。

## 真实同流程 Prague 执行

一次新本地 VM 里依次执行：部署原 gate/真实 receiver/新组件 → 原三份 DKG 签名与注册认证 →
两批各四份原单票 Groth16 和一份 State Proof 验证 → 真实 CT2 冻结 → 认证冻结目标绑定 →
三名 Trustee 原承诺认证 → 门限就绪检查。25 笔有签名的独立交易（含 10 笔负例）连续 nonce 0..24，
序列化交易、哈希、calldata、事件、收据和 SHA-256/MSM/pairing 轨迹全部归档。
五份 package signature 挑战/counter 与原生计算一致；其中三份实际执行签名验证交易，
**不声称五份都做了链上认证**。认证包按字节复用 G6，而不是只复用 G6 中的成功标记。

| 本次交易 | receipt 总 Gas | transaction data |
| --- | ---: | ---: |
| 部署原证书 gate/后端 | 7,750,294 | 16,271 B |
| 部署真实 receiver/两个验证器及适配器 | 10,358,066 | 25,697 B |
| 部署 Trustee 承诺组件 | 6,339,060 | 11,351 B |
| DKG certifier 1 / 2 / 3 | 2,651,994 / 2,590,448 / 2,580,263 | 各 260 B |
| 原注册证书认证 | 2,725,613 | 452 B |
| 接收 B4 第一批 / 第二批 | 3,333,283 / 3,335,455 | 各 6,948 B |
| CT2 冻结 | 144,836 | 4 B |
| 认证冻结目标绑定 | 279,303 | 4 B |
| Trustee 1 承诺 | 3,804,497 | 484 B |
| Trustee 2 承诺 | 3,779,881 | 484 B |
| Trustee 3 承诺 | 3,788,569 | 484 B |
| 三名 Trustee 达门限检查 | 25,616 | 4 B |

全部数值来自本次执行，不从历史结果拼接；部署数据包含构造参数。
原源码以同一 compiler/settings 在新 source-unit 路径编译，metadata/部署 calldata 的非零字节
可不同于 G8-A 原部署，故保留本次实测成本而不强行沿用旧值。它们是功能样本，
**不是完整系统总 Gas、独立重复统计或效率优势**。Setup/Prove 均为 0，不虚构缺失的系统成本。

## 10 项必要原子拒绝

| 情形 | 原因 |
| --- | --- |
| 未认证就绑定 | UNCERTIFIED |
| receiver 未冻结就绑定 | NOT_FROZEN |
| 原签名的 canonical response + 1 | BAD_PACKAGE_SIGNATURE |
| 承诺替换为另一合法子群点 | BAD_PACKAGE_SIGNATURE |
| 修改 CP target | CP_TARGET |
| 修改 final context | FINAL_CONTEXT |
| 修改 Election ID | ELECTION_ID |
| Trustee 1 验证份额替换为 Trustee 2 的合法份额 | VERIFICATION_SHARE |
| 重复 Trustee ID | DUPLICATE_TRUSTEE |
| 只有两名认证 Trustee 就要求进入后续 P3 | DECRYPTION_QUORUM |

失败收据必须是 revert，非 OOG，且没有事件。每次失败前后检查组件的全部五个 submitted/
commitment/share 槽、计数、target/seal、receiver checkpoint 和 gate 授权标记相同。
承诺/份额替换使用实际合法的另一点，不依赖非法点编码提前失败。
没有公布 CP opening、可用解密份额、完整私包或私有 tally；公开 verification share
是 DKG 公钥多项式值，不是可用解密份额。公开 Schnorr response 也不是私密 CP response。

## 执行命令与证据

```sh
# env: CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none
cargo +1.99.1 build -p vv-friendly-dkg --bin g8b_trustee_export --release --locked --offline
target/release/g8b_trustee_export
node contracts/scripts/g8b_trustee_commitments.mjs
cargo +1.99.1 fmt --all -- --check
cargo +1.99.1 clippy -p vv-friendly-crypto -p vv-friendly-dkg --all-targets --release --locked --offline -- -D warnings
cargo +1.99.1 test -p vv-friendly-crypto -p vv-friendly-dkg --lib --release --locked --offline
cargo +1.99.1 test -p vv-friendly-dkg --bin friendly-g6-native-winner-only --release --locked --offline
node scripts/g8a_audit.mjs
node scripts/g8b_audit.mjs
```

实际通过 `node scripts/g8b_record.mjs <name> <command...>` 记录 argv/退出码/log。
首次新增导出器编译出现切片类型适配错误，修正后通过；`build.log` 和 `build-attempt-2.log`
均保留，未删除失败项。该错误不涉及密码学协议修改。
45 项库测试 + 3 项原生状态测试、严格 Clippy、格式通过；376 条历史数据/图表清单项未变。

- [真实 EVM 原始结果](../experiments/g8b_raw/trustee_commitments.json)：
  `f067d6cec7df0a040bdc60e383b101d81a83b8073fdae1957dceb51a6f281dea`。
- [原始公开记录导出](../fixtures/g8b_trustee_commitments_public_v1.json)：
  `944c69430864923a43624bffa5de99b6deaf2764a6d7ffc3aaac1bac01c63800`。
- [字节码及 ABI](../experiments/g8b_raw/contract_artifacts.json)：
  `b78d34dfacc3b51190be80f248e1d11139aa6b2a72e91e2b1be21764818539b7`。
- [命令](../experiments/g8b_raw/commands.json)、[原生导出报告](../experiments/g8b_raw/public_export.json)、
  [审计日志](../experiments/g8b_raw/audit.log)、[SHA-256 清单](../experiments/g8b_raw/SHA256SUMS)。

Apple M4 / 32 GiB / macOS arm64、Node v23.11.0、solc 0.8.30、optimizer=200、viaIR=true、
Prague，实际依赖锁哈希见结果。每交易上限 20M，block context 30M，未提高预算。
新组件 creation 9,975 B、runtime 6,109 B；原证书/签名/receiver runtime 分别 4,816/6,257/7,293 B。
Node RSS 峰值 450.6875 MiB 含编译/VM，不是 EVM 链上内存。没有 RPC 或公共测试网部署。
原生导出器由 Cargo 自动发现为 `g8b_trustee_export`，无需改历史 Cargo.toml 或 package.json；
旧合约/冻结配置/历史 raw 和图表保持字节不变。

输出拒绝覆盖。已有公开 fixture 可将 EVM 流程重放到新目录：
`node contracts/scripts/g8b_trustee_commitments.mjs experiments/g8b_replay_new`。
完整的干净源码统一复现仍属于 G10，不能用本轮 dirty-source 功能执行替代。

## 未完成与下一步

原 receiver **尚未在接收时强制证书 gate**。本轮新组件只在冻结绑定前 enforce，不能将此写成
完整认证投票接收。gate/receiver 部署地址、Election/manifest 和原身份密钥/authority 仍需外部
合法授权。当前为固定 all-five QUAL、n5/t2、D5/W32 研究配置，不是生产可信部署或安全定理。

下一轮只做 **G8-C**：新的整合接收器强制认证，从同一认证 KeyCore/冻结存储/承诺重建全部
60 项公开输入，复用 G6 原证明和 G4-D 原 VK 真实验证。私密打开/CP 留在现有 P3 内。
G8-D 一次性 Winner、G8-E 完整闭环和成本、G9 精确 SnarkPack EVM、G10 公平系统比较尚未完成。
不扩大批次、不重跑历史统计/证明、不重启冻结的 Miller 路线。本轮到 G8-B 验收即停止。
