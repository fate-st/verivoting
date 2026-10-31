# G8-B 补充：五份原承诺与建钥/解密角色分离

## 范围与结果

**PASS：五份原始公开承诺均通过真实 EVM 签名/KeyCore 份额核对；完整 G8 仍 PARTIAL。**

本次上传评估以旧 `ac021610` 为基准，未包含本地已提交 `e3b7cfb` 的 G8-B 三人结果。
因此不重新开发已有模块，只补评估新增的“五名 Trustee 全部通过”和“解密 roster 不限于
建钥认证签名集合”。分支 `experiment/g8b-five-trustees`，执行基点
`e3b7cfb9b1c21161af32e2035b67e774ba4a47b5`、`git_dirty=true`，真实源码快照/哈希归档。

所有 Solidity、Rust、协议参数、原始签名算法与旧 fixture 保持原字节；没有重新编译 Solidity，
而是使用已归档的**完全相同创建交易/运行字节码**。新增的是独立实验脚本与实验配置，
不是新密码学 Suite。

## 必要的真实流程

在新本地 Prague VM 中注入的仅是公开研究发送账户的初始余额，**没有注入合约状态**。
逐笔执行 G8-B 归档 nonce 0..12 的 13 笔已签名原交易：原 gate、receiver、commitment 组件的
真实部署，原三份 DKG/注册认证，两批真实选票/状态证明验证、CT2 冻结及绑定。
其中也包含原前缀的两个拒绝交易。收据状态、Gas、返回字节、SHA/MSM/pairing 轨迹与原归档一致，
四个顶层组件 runtime 逐字相等。不是从旧 JSON 读取 checkpoint 后直接设置为成功。

随后使用 G6 已归档的五份原承诺/签名，按 `4,5,3,1,2` 提交。
原 `JubjubSchnorrVerifier` 和 `VeriVotingTrusteeCommitments` 实际重新计算原消息、挑战和群等式，
每份都检查 `VK_i=A0+i*A1+i²*A2`；输入签名字节与 G6 原记录逐字一致。

| 提交顺序 | Trustee | 是 DKG 认证签名者 | receipt Gas | data |
| ---: | ---: | --- | ---: | ---: |
| 1 | 4 | 否 | 3,795,978 | 484 B |
| 2 | 5 | 否 | 3,767,933 | 484 B |
| 3 | 3 | 是 | 3,788,569 | 484 B |
| 4 | 1 | 是 | 3,787,397 | 484 B |
| 5 | 2 | 是 | 3,779,881 | 484 B |

建钥 certifier 始终为 `{1,2,3}`、`Q_CERT=3`。只有 4/5 提交时 count=2、P3-ready=false；
3 加入后，认证承诺集合为 `{3,4,5}`，count=3，门限检查交易成功（25,616 Gas、4 B）。
此时 1/2 尚未提交承诺，证明合约没有错误要求“解密成员都必须签过 DKG certificate”。
继续接受 1/2 后 count=5、五个 submitted 全 true，原认证 signer mask 仍为
`[true,true,true,false,false]`。`TAU` 和 `Q_CERT` 的不同角色明确保留。

以上是**公开认证承诺就绪集合**的测试，不是为 `{3,4,5}` 新生成 CP/P3 或执行新私密解密。
承诺认证不自动证明打开、CP 和计票正确，仍须在后续现有 P3 内证明。

## 四项新增必要拒绝

| 用例 | 实测拒绝原因 |
| --- | --- |
| 伪造 Trustee 4 原签名的 canonical response | BAD_PACKAGE_SIGNATURE |
| 修改 Trustee 4 的 CP target | CP_TARGET |
| 重复提交 Trustee 4 | DUPLICATE_TRUSTEE |
| 只有非 certifier 4/5 时要求 P3-ready | DECRYPTION_QUORUM |

均为真实交易 revert、非 OOG、无事件；失败前后五人的 submitted/承诺/verification-share 槽、
count、target/seal、认证 signer mask 和 receiver checkpoint 相同。
旧 G8-B 的十项负例保留，不重新扩展完整攻击矩阵。总 23 笔真实交易：13 笔原前缀 +
5 笔承诺 + 4 笔新负例 + 1 笔门限就绪；原前缀另含两笔历史负例重放。

## 命令与来源

```sh
node --check contracts/scripts/g8b_all_trustees.mjs
node --check scripts/g8b_five_record.mjs
node --check scripts/g8b_five_audit.mjs
node scripts/g8b_five_record.mjs evm node contracts/scripts/g8b_all_trustees.mjs
node scripts/g8b_five_record.mjs baseline-audit node scripts/g8b_audit.mjs
node scripts/g8b_five_record.mjs audit node scripts/g8b_five_audit.mjs
```

实际 argv/退出码/log 见 [命令记录](../experiments/g8b_five_raw/commands.json)。
本轮没有 Rust 或合约修改，因此不重跑基点的 48 Rust 回归、Clippy、编译或旧重型证明。
它们仅作为 `e3b7cfb` 已验收基线保留，不能宣称这次重新执行了这些测试。
411 条历史实验/fixture/图表清单项来源检查通过，中央 summary 按新结果更新。

- [原始 EVM 结果](../experiments/g8b_five_raw/all_trustees.json)：
  SHA-256 `d6b468eab53bd7cfe1324b284abb64552d84260562c5072995a8c0f351ca38dd`。
- [原始字节码引用](../experiments/g8b_raw/contract_artifacts.json)、
  [五份公开记录引用](../fixtures/g8b_trustee_commitments_public_v1.json)：不新建或覆盖旧 fixture。
- [来源审计](../experiments/g8b_five_raw/audit.log)、
  [执行快照](../experiments/g8b_five_raw/provenance)、
  [SHA-256 清单](../experiments/g8b_five_raw/SHA256SUMS)。

Apple M4 / 32 GiB / macOS arm64、Node v23.11.0、EthereumJS Prague，依赖锁源码哈希见 raw。
使用 G8-B 已归档 solc 0.8.30/optimizer200/viaIR/Prague 字节码，不重新 setup/prove/编译。
每笔 Gas 上限 20M、block context 30M 未改；Node 峰值 RSS 153.09375 MiB，不是链上内存。
提交顺序影响初次计数槽等存储成本，不能把上表与旧三人样本的差值写成优化收益。
不计算完整系统 Gas，不把单次功能交易当独立统计，不调用公共 RPC。

## 下一步与未完成边界

本轮按最新上传方案只补 G8-B，不自动实施 C/D/E。唯一下一步是 **G8-C**：

1. 新接收器在接收前强制原证书认证和绑定，保护旧接收器/旧结果不改。
2. 从实际冻结 CT2、target、已认证验证份额/承诺和 Election Context 重建全部 60 项 P3 输入。
3. 与 G6 原输入逐项兼容，复用 G6 原证明和 G4-D VK 调用真实 EVM 验证器。

一次性 Winner 留给 G8-D；完整闭环/成本留给 G8-E。当前旧 receiver 仍未在接收时 enforce gate，
CP opening/usable shares/tally 始终不公开。外部授权的身份密钥、authority 和部署地址仍是信任根；
研究型确定性 Setup、本地 VM、未整合多进程私密交付、G9 精确 SnarkPack EVM 和 G10 公平比较
的限制不变。五份承诺认证不等于完整 Winner-only 链上系统完成。
