# G10-B2：生成成本证据账本与两批 B4 聚合补测

状态：PASS_ARTIFACT_COST_AUDIT_AND_BOUNDED_AGGREGATION。本轮是**成本来源核对 + 有限聚合生成补测**，不是新的完整系统性能实验。

## 1. 本轮真实补测

同一W32/D5、同一8票/两个连续B4、同一选票VK、原SnarkPack V2和研究SRS。仅重建一次research Fake SRS，不做Groth16 Setup/Prove。
两批各预热1次，再做12轮交替顺序；24个正式样本、26次实际聚合及26次真实验证，全部结果保留，所有聚合字节与G2-E归档相同。

| 项目 | 中位数 ms | Q1 / Q3 ms |
| --- | ---: | ---: |
| 第1批 B4 聚合生成 | 9.108 | 8.697 / 9.554 |
| 第2批 B4 聚合生成 | 8.776 | 8.513 / 9.329 |
| 同轮两次聚合运算之和 | 18.099 | 17.744 / 18.324 |

只计aggregate_proofs调用；序列化、比对和真实验证在其外记录。两次运算之和不是端到端流程。
离线研究SRS重建 7.766 ms、specialize 5.508 ms、两批matrix/transcript准备 0.049 ms。
整个聚合进程峰值 12.250 MiB，包含共同加载/SRS，不是单次聚合分配或Groth16证明者内存。
只有一组固定证明池，不是12次独立选举。此补测不重复G1-E B2–B32矩阵；第一批为当前环境对照，第二批对应非零旧状态的连续结算材料。

## 2. 已有真实生成成本：逐项对上当前证明

| 历史组件 | 已记录耗时 s | 来源 | 口径 |
| --- | ---: | --- | --- |
| 选票 Groth16 Setup | 11.312937 | experiments/g1_d0_raw/uniform_depth5_native_pilot.json / groth16_setup_ms | OFFLINE |
| 原八票 Groth16 Prove | 55.702404 | experiments/g1_d0_raw/uniform_depth5_native_pilot.json / groth16_prove_total_ms | PROVER |
| 第一批 B4 原聚合生成 | 0.008160 | experiments/g2_d_raw/state_transition_d5w32_b4_groth16.json / new_source_aggregation_ms | AGGREGATOR |
| 第二批 B4 原聚合生成 | 0.012321 | experiments/g2_e_raw/multibatch_state_b4.json / new_source_aggregation_ms | AGGREGATOR |
| 状态关系 Setup | 24.941019 | experiments/g2_d_raw/state_transition_d5w32_b4_groth16.json / setup_ms | OFFLINE |
| 后续状态 PK 重建 | 25.314304 | experiments/g2_e_raw/multibatch_state_b4.json / setup_rebuild_ms | OFFLINE_RECONSTRUCTION |
| 第一批状态 Prove | 15.998831 | experiments/g2_d_raw/state_transition_d5w32_b4_groth16.json / prove_ms | PROVER |
| 第二批状态 Prove | 16.917431 | experiments/g2_e_raw/multibatch_state_b4.json / new_second_prove_ms | PROVER |
| P3 PK 重建（接收范围外） | 69.021922 | experiments/g6_raw/native_winner_only.json / setup_rebuild_ms | OFFLINE_SUFFIX_ONLY |
| P3 Prove（接收范围外） | 41.178248 | experiments/g6_raw/native_winner_only.json / prove_ms | PROVER_SUFFIX_ONLY |

八份原始选票证明逐字节从D0经D1/D2追踪到当前G2-E/B1，计时对应原8票，不把D1/D2新增16/32票时间重复计入。
两份状态证明对应CT0→CT1、CT1→CT2，状态VK相同。P3仅作同CT2后缀成本参考，尚未接入本轮四路线完整生命周期。
同一合法性证明的生成负担对四路线一致；只有SnarkPack路线承担聚合生成，只有State路线承担状态证明生成。
Setup是离线项。第二次状态Setup是PK未保存所需的历史重建，不能默认把两次历史重建都作为一个部署的在线批次成本。
历史RSS是包含Setup/关系检查的累计高水位，不是每个Prove阶段的隔离内存。

## 3. 为什么不重跑大电路

成本数据不是不存在，而是此前未统一映射到现用证明。本轮已经验证证明、VK、记录、矩阵、上下文、批次根和CT2的对应关系并保存独立SHA。
但历史源文件/锁文件有差异，且历史Rayon线程条件未完整归档；即使同为Apple M4/Rust1.99.1/Release，也不能把它们称为当前同二进制生成测量。
机器清单保存各历史source hash与当前差异，不抹去原git_dirty或替换旧SHA。没有把旧Prove和新B1接收、或本轮Aggregation相加充当端到端结果。
因此当前完整系统总时间/Gas仍为null，匹配生成成本和独立证明者RSS仍未完成；不以生成次数0冒充成本0。
目前B4接收路线已显示逐票验证更快，没有必要为获得预设优势扩B64+或重跑旧规模实验。重型Setup/Prove仅在最终明确需要同环境完整成本时按预算单独补测。

## 4. 来源与复现

执行基准HEAD b253f60e73f9dec41b80b03d339b2442ce9bfe19，工作区有本轮未提交修改；实际源版本SHA 917a13ddf0fe7d17d1a9895aa1972d74dc9d7a407816528c0ced9c6e2796185f，二进制SHA 2c176db99171561e864971df701d457040cf7c16b686dfc15099d5d1bb5691a9。
本轮未改变Cargo.toml：使用Cargo自动发现的新bin，保证B1所钉住的旧源码/配置字节不变。执行前后B1归档核查均通过。
命令、退出码、硬件、源/二进制hash见experiments/g10_b2_raw/execution/manifest.json及aggregation_cost.json；源码原字节快照见同目录source_snapshot。
账本见results/g10_b2/cost_ledger.json，含全部来源路径/文件hash、逐证明hash、应用路线、历史环境差异和仍未测项。

```sh
node scripts/g10b2_run.mjs /ABS/NEW_OUTPUT_DIRECTORY
node scripts/g10b2_report.mjs --check
node scripts/g10b2_check.mjs
```

本轮新增Groth16 Setup=0、选票/状态/P3 Prove=0、研究SRS重建=1、聚合=26、EVM=0。旧实验、图表、配置和论文不改。
G9-P1仍BLOCKED，不进入P2。下一步只推进同CT2下Full Tally/Winner-only的公开验证/披露比较，隔离输出分支；不自动重做P3，不把公开字节差异当成隐私定理。
