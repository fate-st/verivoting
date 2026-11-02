# G8-D：同一权威接收器验证 P3 后一次性写入 Winner

## 结果与范围

G8-D PASS。基于已审计并本地提交的 G8-C `2ce1caedf11a064c36f8322daf65e69b8522bf76`，
建立 `experiment/g8d-winner-once`；执行时源码尚未提交，因此结果如实记为 git_dirty=true，
并保存实际执行源码快照/哈希。不把旧 dd7e00a 当作 G8-C 的干净提交。

一次新建的本地 Prague VM 流程实际完成：原证书认证 → 两批 B4 的8份单票/2份状态证明验证 →
冻结同一 CT2 → 认证原 Trustee 1/2/3 承诺 → 存储重建60项输入 → 原 P3 验证 → Winner=1。
成功写入及 WinnerDisclosed 事件均恰好一次。21笔有效验收流程交易，五项验收（一个正例、四个必要负例）。
本轮到 G8-D 停止；精确 SnarkPack EVM、G8-E完整成本口径和G10公平比较仍未完成。

## 最小代码改动与信任边界

仅新增 `contracts/src/VeriVotingWinnerOnlySettlement.sol`，继承 G8-C 认证接收器：
原接收/Nullifier/累计/冻结函数、原 Gate、承诺认证、P3编码和电路/VK均不修改。
Winner保存在该权威接收器，而非可任意重复部署的独立披露包装器中。

- P3地址由部署者预先计算 CREATE 地址并在构造时固定为 immutable；不是交易调用者选验证器。
- `bindFrozenP3()` 只允许 initializer 在首批接收前确认一次，检查真实 Receiver/Gate/Store
  相互引用及原 G4-D 固定摘要/子验证器；不能改指针、换VK或再次确认绑定。
- 唯一 `discloseWinner(bytes,uint8,uint64)` 入口先检查未披露，再 STATICCALL 固定
  `verifyFrozenP3`。该组件在同一交易从认证冻结存储重建输入并进行真实配对，成功后才
  写 Winner/selector/disclosed、产生一个包含 Election ID及freeze seal的事件。
- 不接受 caller输入数组、密文、承诺、VK或离线成功布尔值；不缓存另一次交易的验证成功。
- 单次写入约束作用于指定 receiver/election 实例。不能宣称阻止全世界部署另一份克隆合约；
  选举所承认的接收器地址、原身份/authority和部署初始化仍是明确的信任根。
- 初始化流程必须先完成绑定再开放选举。没有新增通用可升级、权限体系或另一套签名机制。

## 实际验收顺序

先部署并绑定固定组件；重复绑定正常拒绝。随后认证原证书，真实接收两批B4。
冻结前尝试披露正常拒绝，再冻结CT2并认证原123承诺。

| 项目 | 实际结果 | P3配对 |
| --- | --- | --- |
| 重复绑定P3 | P3_ALREADY_BOUND | 未调用 |
| 冻结前披露 | NOT_FROZEN | 未调用 |
| 正确冻结及承诺后，原证明将Winner改为2 | INVALID_GROTH16 | 实际返回0 |
| 原512 B证明、selector0、Winner1 | 成功写入并发出一次事件 | 实际返回1 |
| 重复正确披露 | ALREADY_DISCLOSED | 未调用 |

错误Winner在成功披露之前测试，不用ALREADY_DISCLOSED冒充证明拒绝。
四项失败均正常revert而非OOG、无事件，前后完整业务检查点相同（不要求发送者nonce/Gas余额不变）。
成功交易仅改变Winner及披露状态；Gate/认证/8个Nullifier/CT2/冻结target/承诺保持不变。
不重复G8-C十项历史负例，也不重新生成十种子集证明。

## 真实测量

环境：Apple M4、Node v23.11.0、Solidity 0.8.30+commit.73712a01.Emscripten.clang；optimizer200、viaIR=true、Prague。
每笔20M Gas上限、30M block上下文；同一VM连续交易，不是把所有交易塞入同一个30M区块。
峰值Node RSS 492.421875 MiB（包括编译和VM），不是独立证明者内存测量。

| 交易 | receipt总Gas | calldata/创建输入字节 |
| --- | ---: | ---: |
| deploy_winner_receiver | 11,604,752 | 32,526 |
| bind_fixed_p3_once | 63,614 | 4 |
| accept_batch_1 | 3,400,500 | 6,948 |
| accept_batch_2 | 3,402,672 | 6,948 |
| freeze_ct2 | 204,081 | 4 |
| disclose_winner_once | 2,073,560 | 644 |

其余部署/认证/承诺/拒绝交易的全部明细见原始JSON。披露成本包含实际输入重建、验证、写状态和事件，
不沿用G8-C或G7组件Gas，也不推测SSTORE附加值。不把本次功能样本称为统计性能优势，不汇总为论文完整系统Gas。
接收器运行字节码12,935 B，实际9个合约均未超过EIP-170；全部交易均低于20M，不依赖关闭代码长度限制。

## 证明复用与隐私边界

复用原8份选票、2份状态、1份P3证明及原认证签名；新Setup/Prove/CP包/重签均为0。
G6原证明192 B；实际EVM使用G7已导出的512 B EIP-2537编码，60项与G6/G7逐项一致。
使用原G4-D VK，SHA-256为 `0d37a77cfc8a1cdfed612b397149d96816188d164e08a74994dd2abfca6cff29`。

未输出私有tally、usable解密份额、opening或CP私包。没有本轮重新解密/证明或分布式私密交付；
不能以复用证明声称本轮验证了生产隐私或完成隐私组合定理。当前EVM批次仍是逐票Groth16备用路线，
不是精确SnarkPack；没有公链部署或生产可信设置。

## 命令、来源与失败记录

实际执行命令、退出码和日志见 [commands.json](../experiments/g8d_raw/commands.json)：

```sh
node scripts/g8d_record.mjs evm node contracts/scripts/g8d_winner_once.mjs
node scripts/g8d_record.mjs audit node scripts/g8d_audit.mjs
node scripts/g8d_record.mjs baseline-audit node scripts/g8c_audit.mjs
node --check contracts/scripts/g8d_winner_once.mjs
node --check scripts/g8d_audit.mjs
node --check scripts/g8d_record.mjs
```

初次运行21笔交易已执行，但最终源码归档错误引用不存在的 `g8d_frozen_p3.mjs` 而退出。
该轮 `evm.log`、全部 `transactions-attempt-1.jsonl` 和实际执行源码在attempt-sources保留。
修正为独立脚本真实文件名后第二次完整执行通过；正式JSON只记录第二次21笔，非42笔统计样本。
审计同时核对两轮交易签名字节、Gas、状态和返回值一致，不改历史记录来冒充首次成功。

当前审核：真实签名/创建输入/实际MSM及配对输入/60字段来源/原子性均通过；G8-C来源回归通过。
473条历史archive条目、939个非状态汇总类基线文件未变。Rust/电路不改，不重跑旧重型证明或Clippy。
六个明确允许维护的README/状态/下一步/中央汇总生成器和产物不计入“旧文件完全不变”。

原始结果：[winner_once.json](../experiments/g8d_raw/winner_once.json)

SHA-256：`20b72b4f23a66dcd8ef06e30c62494ad8f989d51107fb3c64876a9b1c1ff3268`。
合约ABI、创建/实际运行字节码：[contract_artifacts.json](../experiments/g8d_raw/contract_artifacts.json)。

本轮结束。下一轮仅G8-E：固定本版本完整生命周期记录与成本口径，不混加历史组件数据；G9/G10后置。
