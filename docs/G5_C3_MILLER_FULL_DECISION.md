# G5-C3-B22：一次受限 full64 验收的只读决定

2026-10-27。本轮执行 Node 来源与源码复核，决定下一门 B23 只对保存的 B21
quarter 版本进行一次 20M 预算内完整正例验收。本轮 EVM 交易数为零，没有
重编译 runtime、修改算术、实现 B23 运行器或取得新的性能证据。

状态：`PASS_DERIVED_SINGLE_FULL_CONFORMANCE_DECISION_ONLY`。这里的 PASS
只表示决定及来源核对已记录；目标能力仍为
`BLOCKED_QUARTER_MILLER_FULL_VARIANT_UNTESTED`。

## 为什么只再安排这一次完整验收

B20 的隔离 Fp 算术和四个进位用例已通过；B21 新 runtime 的两步前缀完整
输出及错误目标拒绝也已通过。但新版本没有 full64 记录，后续迭代、arena 的
完整执行及仅在第 64 步末尾进行的负 x conjugation 尚未得到该版本的完整结果
验收。B18 的 OOG 来自不同旧 runtime，不能替代新版本证据。

本次选择 B23 的信息价值是收口这一个版本的完整结果/预算缺口：若成功，取得
固定向量的完整 Miller 结果与真实成本；若失败，冻结该固定预算软件路线的
自动迭代。B21 的 3.41776% 前缀降幅仅保留为历史测量，没有用于预测 full
成功概率、完成率或 Gas，也不保证 B23 能执行到最终 conjugation。

论文当前的 target-verifier compatibility 段明确允许完整聚合验证器待实现，
restricted evaluation 也排除了 SnarkPack EVM 与完整结算。因此 B23 服务于
当前选定的软件组件研发收口，不是支撑论文现有收窄主张的强制实验。

## B23 的冻结范围：未来一部署、一正例

| 项目 | 未来 B23 固定值 |
|---|---|
| 合约 | `contracts/src/Bls12381MillerLoopQuarter.sol` |
| runtime 大小 | 7,078B |
| runtime SHA-256 | `97d85e04447a8ff8cbc46aca4d6e2eae510d04e7c96c34cdd80ffa1b1d0ec83c` |
| 编译配置 | solc 0.8.30、optimizer 200、Prague、无 viaIR，与 B21 相同 |
| 调用 | `verifyMillerLoopOrRevert(points_eip2537_hex, full_miller_fp12_hex)` |
| full calldata SHA-256 | `a64cf5715de2da1fc25fd31897031b969f3bc5396a2da13256c438a5ea037713` |
| 交易 / block 预算 | 20,000,000 / 30,000,000 Gas |
| 交易数 | 一次必要部署 + 一笔 full64 正例，共两笔 |
| 前缀、原语边界、负例矩阵 | 均不重跑 |

固定 fixture 是 `fixtures/snarkpack_g5c3_miller_loop_v1.json`；同一公开 B=2
证明和预期 full 输出继续沿用，不生成新证明，不提供可信预计算中间状态。
预算、Suite、曲线、编码、Manifest 和编译配置都不改变。

B23 只保存 MODEXP 成功/失败与费用、完整返回值、交易费用及活动内存等必要
记录，不开展新的大规模 opcode/source profiling。本轮 B22 没有执行上述
两笔未来交易，`planned_validation.executed=false`。

## 预先确定的结果分类与停止规则

### PASS：只限固定向量的完整 Miller conformance

必须同时满足：

- receipt 为 1，根层无异常；ABI 解码后的全部 768B 规范 payload 与冻结
  full fixture 完全相同。
- 未改变公式预期的 14,316 次 MODEXP 全部成功，失败数为零，返回长度检查
  通过；不能以宿主参考结果代替合约计算。
- 退款为零，execution、intrinsic、Prague calldata floor 与总费用对账通过。

只有这些条件全部满足，才填写新版本实际完成的 Miller 验证 Gas。成功也不
表示最终幂、精确 GT target binding 或完整 SnarkPack verifier 已完成；是否
开展最终幂需要另行决定，不由本门自动展开。

### OOG：明确的根层预算耗尽

记录为本门 OOG 需要根 receipt 为 0、根异常为 `out of gas`、交易花费达到
固定 20M 上限，且已观察到的 MODEXP child 失败数为零，费用对账一致。
记录实际成功调用数、终止位置和内存；完整结果与完成 Gas 继续为 null。
未执行完的 MODEXP 数不构成完成比例。

### OTHER FAILURE：保留实际原因，不改写为 OOG

child 失败、显式 revert、错误/无法解码的返回值、成功执行但调用数不符、
费用或退款检查失败等，都应保留原始证据并单独记录。不能仅因为花费较高，
或未取得完整结果，就把它们记为预算 OOG；存在 child 失败也不能自动归入
上面的纯根层 OOG 分类。尚未完成的 OOG 路径不要求成功次数等于 14,316。

### ABORT：执行前或部署阶段失败

源码/runtime/依赖/fixture 等前置核对失败，或唯一必要部署没有成功，则中止
full 调用并记录 ABORT。此时没有执行 full64，不能报告 Miller 已测 OOG、
填写完成费用或为取得另一个结果自动再次部署。

上述任何失败都冻结这条固定预算软件路线的自动算术优化与 full 重试。后续
只允许先对已保存证据作只读诊断与归档，不提高 cap、不启动新的原语搜索、
wrapper 实现或重复完整调用。该前置停止规则使 B23 成为一次收口，而非又一轮
细碎优化的起点。

## B22 结束时的真实状态

```text
evm_transactions_executed = 0
runtime_recompiled = false
contract_changed = false
optimization_implemented = false
latest_prefix_gate = G5-C3-B21
latest_full_attempt_gate = G5-C3-B18
target_verifier_status = BLOCKED_QUARTER_MILLER_FULL_VARIANT_UNTESTED
```

完整 Miller 与 exact SnarkPack EVM verified 标记仍为 false。完整 Miller
Gas、estimated full Gas、completion percent 和 SnarkPack Gas 均为 null。
B18 继续作为不同旧版本的历史失败来源，B23 目前只是计划。
20M 是组件实验预算，不是本轮证明的目标链永久硬上限。

## 产物、复现与来源

```bash
npm run --prefix contracts g5c3b22
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

- 决策分析器：`contracts/scripts/g5c3_miller_full_decision.mjs`。
- 派生记录：`experiments/g5_c3_raw/miller_full_decision.json`。
- 新版本前缀：`experiments/g5_c3_raw/miller_quarter_prefix_evm.json`。
- 原语来源：`experiments/g5_c3_raw/fp_mul_quarter_profile.json`。
- 不同版本历史 full：`experiments/g5_c3_raw/miller_cost_attribution.json`。

归档只读核对了十二项来源/源码/锁文件/Manifest/fixture 哈希，以及保存 runtime、
编译配置、full calldata 和公式预期调用数的一致性，没有运行 VM。Git 基准为
`6bad1b0384ef18f525b85c86a9342447eda94b96`，`git_dirty=true`；本轮不是新
性能样本。

| 产物或来源 | SHA-256 |
|---|---|
| B22 分析器 | `5ab71177fa82eeff7742b73471b45697b571c56378aca8b3e6dfaad092e787d4` |
| B22 派生记录 | `b862ff2296763dee4d68aa848f04adebdeb77dae32191389c273d5d629412b48` |
| B21 前缀 raw | `afab1dbf4d32d5ed1a0066e049512fde22839ce612e02deec7f9398c6474eb01` |
| B20 原语 raw | `0d44c5b76b33816cf37f85aaacd52ff7170afc09cd5e92a987b10c198b5cb52c` |
| B18 历史 full raw | `6645a9dbdc8c363944723edc89b09a947e5533b95d3a507076b8086c2fb9ec37` |
| 冻结合约源码 | `90e573ae450b9a75c0dbe4a21242cc07113ccb041f56c12370e43a194725958e` |

## 后续记录：B23 唯一完整调用 OOG，停止规则已触发

2026-10-27，B23 已按本页计划执行一次必要部署和一笔 full64，使用原 B21
runtime、编译配置、冻结 calldata 和 20M 上限。部署成功；唯一完整调用为
根层 `out of gas`，总费用 20,000,000，execution 19,963,388、intrinsic
36,612、退款零。11,076 次 MODEXP 全部成功，child 失败为零，费用检查通过。

结果准确分类为 OOG，新 quarter 版本从“full 未测试”更新为
`BLOCKED_QUARTER_SOFTWARE_MILLER_EXCEEDS_20M_FROZEN`。最新实际 full 为 B23，
最新前缀仍为 B21，完整结果和完成 Gas 未取得；预测及完成率继续为 null。
本页前文保留 B22 当时的只读决定背景，实测详情见
[B23 quarter 完整调用报告](G5_C3_MILLER_QUARTER_FULL.md)。

预先规定的失败规则现已触发：冻结这条固定预算软件路线的自动算术优化与
full 重试，之后只读对齐证据、论文主张和其他必要实验的优先级，不自动提高
cap、实现 wrapper 或另开原语优化。冻结不等于停止整个项目，也不是证明
20M 是目标链硬限制。B23 没有 root step listener，wall time 不与 B18 的
大观察器比较，不从调用次数声称完成率或加速。

B23 raw：`experiments/g5_c3_raw/miller_quarter_full_evm.json`，SHA-256
`ea725ce6aa8e1f0aebf0b8ca44f273cdec2aaffa25d579610ab21e090eea7ccf`。
