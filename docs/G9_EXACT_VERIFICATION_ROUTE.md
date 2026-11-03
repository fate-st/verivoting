# G9：精确 SnarkPack EVM 验证路线决策

## 结论：路线审查完成，不是验证器通过

日期：2026-11-02（Asia/Shanghai）。G8-E先保存为本地提交 `4421dd9be2487195db07d91085d941b3aa51a8f6`。
本轮只读核对既有源码/失败证据并归档设计，没有新EVM交易、编译、Setup/Prove、CP或签名。
有条件选择**多交易分阶段精确验证**作为后续设计候选；资源可行性未证明，不批准直接启动完整B2原型。
原单交易软件Miller路线继续冻结；wrapper只做候选比较，不开发、不换套件。当前完整精确验证仍BLOCKED，Gas为null。

## 已有证据及真正缺口

- [B23](../experiments/g5_c3_raw/miller_quarter_full_evm.json)：一次full64在20M预算下OOG，支出20,000,000 Gas；它是失败封顶支出，不是完整Miller完成成本。最终幂和完整聚合验证未完成。
- [C2](../experiments/g5_c2_raw/transcript_conformance.json)：原生/JS的12次挑战一致，只能证明该跨语言检查；不能说Solidity已经重算挑战。
- [TIPP终点](../experiments/g5_c3_raw/tipp_endpoint_route.json)：一条终点等式原生/JS一致，不代表全部TIPP/MIPP/KZG关系完成。
- [EIP-2537官方ABI](https://eips.ethereum.org/EIPS/eip-2537#abi-for-pairing-check)仅接受G1/G2配对并返回乘积是否等于1，无任意GT目标槽或GT返回值。本轮已查阅官方页面，脚本不联网。

核对已安装且Cargo.lock锁定的bellperson 0.27.0：verify.rs/transcript.rs与C2原始SHA一致，accumulator.rs也记录独立SHA。
旧实验Cargo.lock与当前整文件不同：Git diff仅在vv-friendly-dkg增加ark-relations/sha2工作区依赖边；外部包身份表一致。分别核对旧Git提交的锁和当前基线锁，不要求二者整文件相等。
以下为原验证关系的完整实现清单，不是把一条Miller等式当整个验证器：

| 项目 | 必须保持的检查 | 当前EVM边界 |
| --- | --- | --- |
| PARSE | Original V2 proof size/count and canonical point/GT parsing, subgroup and SRS/VK binding | MISSING_COMPLETE_PROOF_PARSER |
| FS | Original bincode/GT transcript encoding, SHA-256 cumulative rejection counters and all V2 challenges | RUST_JS_CONFORMANCE_ONLY |
| GIPA | All GIPA folding of group/GT/scalar terms and extra-link challenge | NOT_IMPLEMENTED |
| KZG_V | Commitment-key v opening verification | NOT_IMPLEMENTED |
| KZG_W | Commitment-key w opening and original r-inverse shift | NOT_IMPLEMENTED |
| TIPP | All z/tab/uab pairing-to-GT equalities, not just the one archived endpoint | ONE_ENDPOINT_NATIVE_JS_ONLY |
| MIPP | tc/uc pairings plus final_c scalar/group relation to zc | NOT_IMPLEMENTED |
| AGGREGATE | Final Groth16 aggregate equation binds every ordered public-input row | NOT_IMPLEMENTED |
| ACCUMULATOR | Sound complete equation checking; no fixed native harness RNG on chain | STRATEGY_UNRESOLVED |
| FINAL_GT | Complete Miller loops, final exponentiation and exact GT target equality | SOFTWARE_ROUTE_FROZEN_OOG |

所有清单项在JSON中定位到固定包文件/行/anchor；脚本核对源码片段存在，但这不代替形式化等价性证明。
PairingChecks原实现使用随机系数合并检查；不能把功能测试的固定ChaCha20种子照搬链上。
逐条验证全部等式或采用另经论证的合并策略需先说明正确性/健全性，当前保持未解决，不能悄悄替换。

## 三条路线的决定

| 路线 | 决定 | 原因 |
| --- | --- | --- |
| 原单交易软件 | 保持冻结 | 已有预算失败；不提高cap、不重复full或扫描算术 |
| 多交易分阶段精确验证 | 仅选为设计候选 | 可能分摊每笔计算，但仍缺GT/最终幂/完整关系及状态存储测量；不预言收益 |
| 额外证明封装 | 暂不实施 | 必须约束完整原验证关系、设置及声明绑定；新增证明假设/电路成本尚无证据 |

## 接收器不能直接替换：拟议协议边界

现有IBatchVerifier.verifyBatch是同步view；Receiver构造时创建固定B4逐票验证器，并在验证后原子写入NF/CT。
分阶段执行是有状态的异步协议，不能简单把它实现成当前view接口，也不修改旧G8-D合约。未来需要独立新版本。

1. BEGIN：由指定Receiver绑定proof、顺序矩阵、B、VK/SRS、选举上下文、batch index、old/new CT，建立session。
2. ADVANCE：合约从已认证存储推进固定阶段，实际计算中间值；只传session和受约束数据，不信任host预计算。
3. FINISH：完整清单全部通过才标记verified，任何组件/前缀通过均不授权收票。
4. CONSUME：唯一Receiver重检当前批次/旧CT/根/NF，对同一矩阵执行原状态证明检查，原子一次消费并更新NF/CT。

最小设计仅允许一个待验证next batch，未完成/中止不得改变已接受NF/CT；未实现重入、多并发或故障压力矩阵。
session envelope可包含chain/address等防混用信息，但不能擅自改动原SnarkPack外部/内部Fiat–Shamir域或编码。

## 已有B2材料的选择：不重新生成证明

旧G5-C1 proof `12e25c7edf07eff2d3f211a3cbb74ae4ce43e028ab58702debdef98b0787e64c` 仅作为历史GT组件来源，旧挑战/目标不可用于别的proof。
找到[W32 B2归档](../fixtures/snarkpack_g1d0_depth5_public_v1.json)的batch_size=2 case：

- aggregate SHA-256：`072156c814f1ce3ae909223eeaad5d06274c163a5843dd05b5742e4cf529447c`。
- 两行共50个字段转成同一BE表示后，与G8第一批的前两行逐项一致。
- Ark VK摘要与G8 ballot_vk_ark_sha256相同：`6d00cab6cd02e9926bd8ef05f8e782fbf1f32b5993c134699155accf7b37aa98`。
- 共享研究SRS：`3a8a75be96e4150c757b85f682be37d4e502e8b45a17943171bd0bbbdfd605aa`；固定该case的n=2 verifier SRS。

未来完整B2验证优先复用该材料，但需为这份proof派生/核对自己的挑战/GT向量。当前未重新执行原生完整验证。
B2是独立组件验收，不是现有B4接收器的收票用例；将来接入固定两批B4须用各自真实B4聚合及对应绑定，不能拼接或冒充。

## 资源预算与下一次最小验收

本轮预算：0 EVM交易、0编译、0证明；只做来源及设计核对。
后续候选保持本地Prague每笔20M、block上下文30M研究配置；不说它们是当前公链永久限制。
总交易数/总Gas/运行时间尚无可信调度依据，继续为null；**必须先确定完整操作/续算清单和总交易及时间上限，才允许原型执行**，不得自动扩预算。

只有这些先决条件满足且另行授权后，最小完整验证只做两例：

- 正例：原W32 B2 proof、VK、SRS，原声明全部真实验证通过。
- 反例：只改第二行input[7]的nullifier，原proof/VK/SRS不变，重建外部transcript及相应session绑定，让完整验证拒绝；不能用过期session-ID/root不匹配冒充证明拒绝。

必须实测每笔和总Gas、交易数、calldata、存储与完整关系覆盖。若只完成GT或其他组件，只能报告组件，不能提升exact标记。
若关键关系/预算不能闭合，则停止并保留BLOCKED，报告已完成的逐票备用路线；不因此自动修改论文或协议。

## 复核与产物

```sh
node scripts/g9_route_review.mjs --write
node scripts/g9_route_review.mjs --check
node scripts/g8e_accounting.mjs --check
```

脚本不运行VM、不编译、不生成新GT/挑战向量；--check重算并逐字节核对JSON和本报告。G8-E清单33项现值及6项已提交状态字节通过。
首次脚本将旧Cargo锁哈希直接与当前文件比较而退出；确认上述合法工作区依赖变更后改为分别核对历史/当前锁及包身份。失败日志和原脚本保留，没有修改任何锁或旧raw。
决策JSON：[route_review.json](../results/g9/route_review.json)，SHA-256：`d473d93c309c1f37038cce51707be18d6f5004e841e26ba29343b1defb93f9b4`。

到本轮路线审查停止。G9完整精确EVM仍未实现，G10公平系统比较仍后置，禁止以审查PASS冒充验证器PASS。
