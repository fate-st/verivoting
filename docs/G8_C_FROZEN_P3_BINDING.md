# G8-C：认证接收、冻结状态重建60输入与原 P3 验证

## 结果：G8-C PASS，完整 G8 仍 PARTIAL

修正后的验收顺序已真实执行：原证书认证 → 两批 B4 验证/接收 → CT2 冻结 → 原 Trustee
承诺认证 → 合约重建60项输入 → 原 G6/G4-D P3 验证。没有 Winner 写入或新的 Setup/Prove。
分支 `experiment/g8c-frozen-p3-binding`，基点 `dd7e00a76f52fbd84065045382d34a49401c2caa`，
执行时 `git_dirty=true`，保存实际执行源码快照/哈希，不冒充干净提交复现。

## 实现范围与固定来源

- `auth/VeriVotingCertificateGateV2.sol`：独立版本，保留原 DKG/注册认证群运算、规范编码和消息。
  增加认证后 Expected/系数/身份/authority 读取，以及原注册证书认证后写入的
  `wRegistered/nRegistered/wMax/certificateSha256`。原 Gate 文件不改。
- `VeriVotingAuthenticatedSettlement.sol`：复制原真实接收/冻结逻辑，新入口**内部**强制检查
  Gate 和认证字段，不存在不受门控的继承/包装旁路。认证公钥/KeyCore/context/root/关闭摘要/
  disclosure/authority/certificate hash/注册权重都与固定选举匹配。旧接收器仍是历史组件，
  本轮只使用新 receiver；不能靠调用历史合约改变新 receiver 的状态。
- `VeriVotingTrusteeCommitments.sol`：原 G8-B 源码不变，通过相同 ABI 指向新 Gate/receiver。
  继续验证原 Jubjub 包签名、公开多项式份额、target/唯一性后存储承诺，不是只统计数量。
- `VeriVotingFrozenP3Verifier.sol`：不可修改的 Gate/receiver/store/子验证器地址；构造时交叉核对。
  只接受 `proof,selector,proposedWinner`，不接受输入数组、任意密文/target/承诺或交易选择的 VK。
- `G8CExperimentPins.sol` / `experiment.G8C-FROZEN-P3-W32.json`：原两批 B4 根、单票/状态
  EIP VK SHA-256 和完整旧 binding digest 固定。是固定实验部署配置，不改密码学 Suite。

P3 固定摘要：

| 对象 | 固定值 |
| --- | --- |
| G4-D Arkworks VK SHA-256 | `0d37a77cfc8a1cdfed612b397149d96816188d164e08a74994dd2abfca6cff29` |
| EIP-2537 VK SHA-256 | `93336597070cda6b48775e28cc077ea0027686f0f8e1e588e4867e3cedb3694a` |
| EIP-2537 VK Keccak-256 | `52fe9c94b348ede49b3e789d240edd861827eeafc9e7bcb41777d2505980e1dd` |

合约核对 receiver 绑定的 Arkworks 来源摘要及 EIP 实际字节的两个固定摘要，不能只检查输入数60。
两种 VK/证明格式的对应来自已归档 G7 原生规范解码/验证/编码导出，脚本重新核对来源哈希；
不声称 EVM 重新解压了 Arkworks 编码。原证明192 B、G7对应 EIP proof512 B，实际传入后者。

## 60项输入与来源映射

| 0-based 索引 | 内容 | 实际来源 |
| --- | --- | --- |
| 0–16 | 选中 Trustee 1 的 CP 前缀和承诺 | Gate 认证上下文、receiver C0/target、store 认证份额/承诺 |
| 17–33 | 选中 Trustee 2 | 同上 |
| 34–50 | 选中 Trustee 3 | 同上 |
| 51 | selector | caller 主张，原十子集表派生 IDs，并检查每个 ID 已认证 |
| 52–57 | 三个冻结 C1 | receiver checkpoint 的同一 CT2 |
| 58 | 注册权重32 | Gate 实际认证的 `certifiedWRegistered` |
| 59 | Winner 主张 | caller 提议合法编号，必须由原 P3 证明约束 |

CP 前缀使用原 Canonical/domain/suite、eid/policy/context/close/disclosure/target、ID、
认证 verification share、C0 和原长度字段；字节串按原31字节 LE 分块入域，不模约减，
不加 Groth16 leading one。原60字段布局/电路/CP 哈希不变，不新增 seal 公共输入。
Gate认证结果、receiver真实CT2和store同一target/seal绑定后才构造输入。

原始 JSON 保存全部60项数值、每项字节范围、分块、来源字段/重叠范围和全局地址映射。
独立 JavaScript 根据**链上 getter 读取值**构造来源映射，与合约输出、G6 LE 输入和
G7 BE 输入均逐项60/60相同。公开数据不包含 opening、usable 解密份额、私包或私有 tally。

## 修正后的实际执行与10项必要拒绝

28笔完整流程交易连续 nonce0..27。4次部署、原三份 DKG/一次注册认证、真实两批接收、冻结/
绑定、五份原承诺认证和一笔 P3正例，共18笔成功；10笔必要负例正常 revert，无事件/部分写入。

| 负例 | 实际原因 | 配对路径 |
| --- | --- | --- |
| 另一把同60宽度的真实旧 VK | PINNED_P3_VK | 0 |
| 未完成认证直接接收 | UNCERTIFIED | 0 |
| 首批 newState 改成另一规范编码密文点，原状态 proof 不变 | INVALID_GROTH16 | 先四份 ballot=1，再 state=0 |
| 两批成功但尚未冻结就验证 P3 | NOT_FROZEN | 0 |
| 冻结后继续接收 | FROZEN | 0，仅证明阶段限制 |
| 只认证 Trustee 1/2，选择123 | MISSING_COMMITMENT | 0，第三份认证缺失 |
| 重复 Trustee 3 | DUPLICATE_TRUSTEE | 0 |
| Winner=2，原证明不变 | INVALID_GROTH16 | 实际 P3 pairing=0 |
| selector1=124，但4尚未认证 | MISSING_COMMITMENT | 0，不能冒称 Groth16 拒绝 |
| 五份原承诺全部认证后，仍使用原 selector0 证明验证 selector1 | INVALID_GROTH16 | 实际 P3 pairing=0 |

**篡改状态发生在首批正式接收和冻结之前**。当时 checkpoint/spent_count/8个NF均未变，
四个真实 ballot 方程已经通过后，状态证明在规范新输入上实际配对拒绝，不是 FROZEN 提前挡住。
每个失败前后保存 Gate checkpoint/注册数据、receiver CT/next/count/8个NF、store计数/
target/seal及五个submitted/commitment/share槽；全部相同。来源审计独立重建 MSM/配对输入
哈希，核对错误状态和错误 Winner/selector 进入的实际方程，而不只相信异常字符串。

主正例仅 G6 已归档 selector0/123/Winner1 的一份 proof。另两份原承诺只用于让错误 selector
到达真实 Groth16；不新建 CP，也不声称十种子集在本次 EVM 全部验收。原生十子集历史结果保持。

## 本次真实成本

| 成功交易 | receipt 总 Gas | transaction data |
| --- | ---: | ---: |
| Gate V2及原签名后端部署 | 8,226,556 | 17,606 B |
| 认证接收器及原 ballot/state 验证器部署 | 10,856,237 | 28,891 B |
| 原承诺存储部署 | 6,339,786 | 11,351 B |
| 冻结 P3组件及固定原验证器部署 | 7,911,595 | 18,832 B |
| DKG签名1/2/3 | 2,652,194 / 2,590,648 / 2,580,463 | 各260 B |
| 注册证书认证 | 2,770,407 | 452 B |
| 两批B4接收 | 3,400,469 / 3,402,641 | 各6,948 B |
| CT2冻结 | 204,028 | 4 B |
| 承诺绑定冻结目标 | 279,723 | 4 B |
| Trustee承诺1/2/3/4/5 | 3,804,497 / 3,779,881 / 3,788,569 / 3,778,878 / 3,767,933 | 各484 B |
| 合约重建输入 + 原 P3 验证 | **2,058,601** | **644 B** |

G7 独立验证历史数据为1,331,401 Gas/2,564 B。G8-C减少60×32=1,920 B的本笔输入传输，
但增加认证读取/存储重建，**本次 Gas 没有更低**。这是功能路径描述，不是公平系统总成本或
优化统计；承诺/部署/此前批次的数据不能因此省略。没有 Winner结算，不汇总为完整系统Gas。
子验证器的通用入口仍是单独的 view 组件；它不写结算状态，不能绕过 binder 改 receiver/Winner。

## 可复现记录与实际检查

```sh
node scripts/g8c_record.mjs evm node contracts/scripts/g8c_frozen_p3.mjs
node scripts/g8c_record.mjs baseline-audit node scripts/g8b_five_audit.mjs
node scripts/g8c_record.mjs audit node scripts/g8c_audit.mjs
node --check contracts/scripts/g8c_frozen_p3.mjs
node --check scripts/g8c_record.mjs
node --check scripts/g8c_audit.mjs
```

实际命令/日志/退出码保存在 [commands.json](../experiments/g8c_raw/commands.json)。
首次脚本因误引用公开 fixture 不存在的 ciphertext_records 字段，在部署后中断；未产出
合格完整结果，失败日志与对应源码保留。改为规范解码原 ballot 公开输入后，第二次完整执行通过；
原始有效28笔明细属于第二次执行，首次部分部署没有独立保存交易明细，不混入性能表。
首次来源审计仅因证书哈希0x前缀比较失败，修正并保留版本/日志，未改任何链上测量结果。
本轮 Solidity编译/新脚本语法/实际EVM/配对轨迹与来源审计通过。Rust/电路未改，不重复运行
原48项回归、Clippy或重型证明。440条历史数据/图表项未变；旧tracked文件除6个明确的
README/状态/下一步/中央摘要生成器及产物外不变，旧源码、配置、fixture、raw不覆盖。

- [本次完整结果](../experiments/g8c_raw/frozen_p3_binding.json)：
  SHA-256 `12a519caf8f6446c79f0ba6867f426621fba1455da566c4a75a3726d7d9cbcec`。
- [全部9个实际合约 ABI/创建及运行字节码](../experiments/g8c_raw/contract_artifacts.json)：
  SHA-256 `3755c1c50fce69b37d539e11513f574f955b4e6803dd1b559592d26c3142f6e3`。
- [来源与轨迹审计](../experiments/g8c_raw/audit-attempt-2.log)、
  [执行源码](../experiments/g8c_raw/provenance)、[SHA清单](../experiments/g8c_raw/SHA256SUMS)。

Apple M4/32GiB/macOS arm64、Node v23.11.0、solc0.8.30、optimizer200、viaIR=true、Prague。
Node峰值455.46875MiB包含编译/VM，不是链上内存。每笔20M cap、30M block context未改；
交易共享本地VM状态，但不是把所有交易Gas塞进一个区块，也没有公共RPC或公链部署。
各runtime均小于24,576 B。默认输出拒绝覆盖，可用新目录重跑
`node contracts/scripts/g8c_frozen_p3.mjs experiments/g8c_replay_new`。

## 停止边界与下一步

到G8-C验收停止：P3方法是view，没有Winner接口/写入/成功缓存，重复view验证不等于重复披露。
下一轮只做G8-D：通过本轮固定绑定的真实验证后，在同一交易写Winner一次，必要错误Winner/
重复披露拒绝。G8-E完整结算成本、G9精确SnarkPack EVM、G10公平比较/干净源码复现仍未完成。
当前投票接收仍是真实逐票Groth16备用路线，不是SnarkPack链上聚合。
原身份公钥/registration authority/部署授权仍是信任根；认证不证明RA诚实求和，也不消除
研究型确定性Setup、授权combiner知悉tally、未整合多进程私密交付和未完成隐私组合定理的限制。
