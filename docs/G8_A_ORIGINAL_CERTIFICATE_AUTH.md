# G8-A：原 Jubjub Schnorr 的 DKG / 注册证书认证组件

## 结果：独立认证组件 PASS；完整 G8 仍 PARTIAL

本轮按 Phase 2 的第一步，只做原认证材料的真实 EVM 验证。
新增 `auth/VeriVotingCertificateGate.sol`、`JubjubSchnorrVerifier.sol`、`Sha512.sol`、`CanonicalVV.sol`。
门控合约在构造时创建真实验证后端，未采用 secp256k1、外部 success flag 或调用者提供的挑战。
**第一阶段接收器保持原字节，尚未强制调用此认证门控**；不要将两个组件合并描述为已完成认证结算。

## 实际通过的流程

1. 对五个身份公钥、注册 authority 和三个公开多项式系数进行原 Jubjub 编码/曲线/子群检查。
2. 在 EVM 重算原注册政策、pre-policy、prekey、DKG session、完整 KeyCore 和 final context，
   与 G6 的同一上下文匹配。当前只支持原实验 all-five QUAL、n=5/t=2、W32 配置。
3. 三个不同 Trustee 的**原算法 DKG 签名**逐笔验证，达到 `q_cert=n-t=3` 后才认证公钥。
4. 验证原注册 authority 的 Schnorr 签名，重建 canonical certificate、关闭摘要和 disclosure 摘要；
   三份值与 G6 原证书一致后才写 `registrationCertified=true`。
5. 独立门控的 `requireAuthorized()` 仅在上述两项都通过后接受。

`q_cert` 是建钥认证 quorum，`tau=t+1` 是后续私密解密门限，使用不同常量/字段；
它们只是本配置中同为 3，不能据此视为同一协议角色。

## 原签名兼容性与来源澄清

采用原始 `VV/JUBJUB/SCHNORR/TRANSCRIPT/v2` 和 H2S/v2 编码。
签名挑战用完整 SHA-512 的前 32 B，按原规范清除最后字节高 nibble，然后作为 LE 标量做严格
`< q` 检查，必须取 counter=0..127 中**第一个**合法候选；不是 CP 的 wide H2F，也不做模约减替代。
规范点编码遵循原 Ark TEFlags；身份点、非子群点、非规范标量不能通过原签名验证。

SHA-512 依据 [NIST FIPS 180-4](https://csrc.nist.gov/pubs/fips/180-4/upd1/final)，
extended Edwards 加法依据 [EFD a=-1 公式](https://www.hyperelliptic.org/EFD/g1p/auto-twisted-extended-1.html)。
曲线、生成元、标量阶和编码同时核对项目锁定的 Arkworks 0.4 源码；数学运算及挑战重算都在 EVM 内。
代码尚未经过独立生产安全审计，不将有限功能测试写成一般密码学安全证明。

重要来源区别：G6 公开 fixture **没有单独保存完整五份 DKG 签名字节**。本轮使用未修改的
`common/g6_materials.rs`、原算法/原研究 RNG 再执行确定性 DKG，原签名计算确实重新执行；
公钥、KeyCore、身份公钥/verification shares 均与 G6 匹配。
注册证书则与 G6 已归档的完整 certificate **逐字相等**。
raw 中 `signatures_resigned=false` 仅指“没有替换授权算法或修改消息重新授权”，
**不表示 DKG 内的原签名计算从未执行**。详见 [来源澄清](../experiments/g8a_raw/provenance_clarification.json)。
没有新 Groth16 Setup/Prove，没有重新生成 P3、十子集证明或 G1-E 数据。

## 真实 EVM 数据

| 实际签名交易 | receipt 总 Gas | transaction data |
| --- | ---: | ---: |
| 部署证书门控及原签名后端 | 7,750,282 | 16,271 B |
| DKG certifier 1 | 2,651,994 | 260 B |
| DKG certifier 2 | 2,590,448 | 260 B |
| DKG certifier 3 | 2,580,263 | 260 B |
| 原注册关闭证书认证 | 2,725,613 | 452 B |
| 认证完成后的授权检查 | 23,403 | 4 B |

每笔 transaction cap=20M，未放宽预算。共 18 笔本地 Prague 签名交易（包含 12 笔负例），
nonce 连续为 0..17。三个 DKG 签名及一个注册签名实际验证；另外两份 DKG 签名只用于原生
重建/挑战一致性检查，不声称它们也完成了 quorum 交易。

13 项 SHA-512 EVM/原生一致性检查覆盖 empty、abc、111/112 B padding 边界、128/257 B
多块输入和全部原证书 hash preimage；六份原签名的挑战/counter 精确匹配，包括实际 counter=1。
另执行身份点与二阶非子群点拒绝检查。
这些只读调用的 execution Gas 单独保存在 raw，**不是**签名交易 Gas 样本或重复性能统计。

## 12 项必要原子拒绝

| 情形 | 拒绝原因 |
| --- | --- |
| 伪造 DKG response | BAD_DKG_SIGNATURE |
| 错误 Election ID | ELECTION_ID |
| 错误 DKG context | DKG_CONTEXT |
| 使用非首个合法 counter | BAD_DKG_SIGNATURE |
| 重复 certifier | DUPLICATE_CERTIFIER |
| 两签名未达建钥 quorum 就认证注册 | DKG_QUORUM |
| 注册尚未认证就要求授权 | UNCERTIFIED |
| 注册证书 Election ID 不符 | REGISTRATION_CONTEXT |
| 注册证书 final context 不符 | REGISTRATION_CONTEXT |
| 注册关闭摘要不符 | REGISTRATION_CLOSE_DIGEST |
| 伪造注册 response | BAD_REGISTRATION_SIGNATURE |
| 重复认证注册证书 | REGISTRATION_ALREADY_CERTIFIED |

每次失败前后完整 checkpoint 和五个 signer 标记均不变。有效签名可以按设计逐步累计到 quorum；
这不是无效交易写入部分认证状态。没有公开任何 usable 解密份额、CP 私包、opening 或 tally。
公开 Schnorr signature response 是认证材料，不是私密 CP response。

## 原始证据、环境与命令

- [真实 EVM 记录](../experiments/g8a_raw/certificate_auth.json)
  SHA-256：`1391a8b8390e6ba0beef370a26aaa1db518267b89fd1b931c6dcf033f25a036c`。
- [原证书公开 fixture](../fixtures/g8a_original_certificates_public_v1.json)
  SHA-256：`75e724ac0644702cbb864fafc8e54ffa0f64042bb08483038ad680db3f021b8e`。
- [实际字节码/ABI](../experiments/g8a_raw/contract_artifacts.json)
  SHA-256：`243a3949ec7c405fe54bceecb5bb47a8d4157b9d0e4c90c53c18b72af0ccb6f8`。
- [原生导出](../experiments/g8a_raw/public_export.json)、[命令](../experiments/g8a_raw/commands.json)、
  [环境](../experiments/g8a_raw/environment.json)、[审计](../experiments/g8a_raw/audit.json)、
  [SHA-256 清单](../experiments/g8a_raw/SHA256SUMS)。

基点 `1c19f811df803a38e3b6ee418f799a557b6d220a`，分支 `experiment/g8-auth-disclosure`，
新增实现执行时 `git_dirty=true`，保留实际源码快照/哈希，不冒充已提交干净源码。
350 条历史数据/图表清单项未变，G7/G8 第一阶段的旧合约、旧原始报告和旧配置不改。

Apple M4 / 32 GiB / macOS arm64，Node v23.11.0、EthereumJS 10.1.3、solc 0.8.30，
optimizer=200、viaIR=true、Prague。创建字节码 15,023 B；门控 runtime 4,816 B，验证后端 6,257 B。
峰值 Node RSS 389.640625 MiB 包含编译/VM 活动，不是链上内存；没有 RPC 或公链部署。

以下 Rust 命令环境为 `CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none`：

```sh
cargo +1.99.1 fmt --all -- --check
cargo +1.99.1 clippy -p vv-friendly-crypto -p vv-friendly-dkg --all-targets --release --locked --offline -- -D warnings
cargo +1.99.1 test -p vv-friendly-crypto -p vv-friendly-dkg --lib --release --locked --offline
cargo +1.99.1 test -p vv-friendly-dkg --bin friendly-g6-native-winner-only --release --locked --offline
cargo +1.99.1 build -p vv-friendly-dkg --bin friendly-g8a-certificate-export --release --locked --offline
target/release/friendly-g8a-certificate-export
node contracts/scripts/g8a_certificate_auth.mjs
node scripts/g8a_audit.mjs
```

45 项库 + 3 项状态回归、格式和严格 Clippy 均通过；本轮正式执行记录没有删除失败/重试项。
输出拒绝覆盖；已有公开材料可重跑到新目录：
`node contracts/scripts/g8a_certificate_auth.mjs experiments/g8a_replay_new`。

## 尚未完成与下一项任务

引导身份公钥/authority、Election ID 和 manifest 仍是外部授权的信任根，不能靠自签证明
身份根本身可信。认证模块证明对应值获得这些已授权密钥认证，不意味着消除了引导信任。

下一步只做 G8-B：让原 Trustee package commitment 绑定同一冻结上下文和**已认证 KeyCore**，
特别是 verification share 必须由已认证系数派生/核对，不能接受无绑定的调用者 VK share。
随后在新的整合接收器中强制证书认证，重建 60 项 P3 输入、复用 G6/G4-D 证明/VK并写 Winner 一次。
CP opening 应保持私密并在 P3 内证明，不把 opening/usable shares 交给合约公开。
现有接收器尚未 enforce gate，Trustee 承诺存储、P3 storage binding、Winner 写入仍未完成。
G9 SnarkPack EVM 受阻、G10 全系统公平比较/干净版本复现继续待办。本轮到 G8-A 组件验收停止。
