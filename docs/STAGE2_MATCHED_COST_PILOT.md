# 阶段二：同条件原生成成本试运行

日期：2026-11-14。范围仅任务0证据索引与任务1最小生成/接收试运行，不是第二阶段全部完成。

机器：Apple M4，32.000 GiB，darwin/arm64。
Rust 1.99.1；沿用仓库Release配置opt-level=1，不是默认opt-level=3；jobs=2、Rayon=4。

## 结果

固定 D5/W32、32名注册/实际8票、三候选人、同一重建选举、两个连续B4。
本次成功试运行真实生成8份新选票Groth16证明、2份新状态证明、2份SnarkPack V2聚合。
四条接收路线使用这些新材料，均达到相同CT1、CT2和8个已用Nullifier；接收时不复用历史证明。
选票25项、状态37项公开布局不变，重建VK/SRS与原档案完全一致。

| 实际生成阶段 | 本次测量 | 归属/口径 |
| --- | ---: | --- |
| Ballot Setup | 14.586 s | 离线独立记录 |
| 8票 Ballot Prove | 65.005 s | 选民证明计算之和，真实新生成 |
| State Setup | 35.617 s | Proposed额外离线Setup |
| 两批 State Prove | 39.119 s | 状态证明者额外计算之和 |
| 两批 SnarkPack Aggregation | 19.054 ms | 聚合者额外计算之和 |

| 接收路线 | 两批接收 ms | 合法性验证 ms | Fold/State Verify ms |
| --- | ---: | ---: | ---: |
| individual_groth16_per_vote | 4.534 | 4.388 | —（未单独计时） |
| transaction_batch_individual_groth16 | 6.152 | 6.032 | 0.011 |
| snarkpack_direct_fold | 13.590 | 13.453 | 0.020 |
| snarkpack_state_proof | 18.180 | 13.066 | 4.995 |

逐票路线在接收器内部直接累计，累计已包含在接收总时间中。原始extra计数器的0表示该分项未单独计时，
不是零累计成本；本表不将它当作测量值。其他路线的不适用生成操作为0次，而不是缺失数据填零。

这些是同次成功运行的**组件测量**，每路线仅一个冷启动样本，没有做独立选举统计。
cost_summary.json中的sequential_measured_component_sum_ms只是组件之和，**不是单独计时的完整端到端延迟**。
共同业务检查/状态写入包含在接收总时间内，不再重复相加。Setup、见证重建、缓存I/O、转换/序列化、
发送端路径准备、证书认证、Freeze/解密/P3/EVM不在该组件和中；原始分项和进程墙钟另有记录。
不同角色的这些计算不能冒充并行系统墙钟或完整投票系统优势。保留原G10-B1统计面板，不用本次单样本替换它。

## 内存与运行器修正

| 独立工作进程 | 进程峰值 MiB | 整进程墙钟 s |
| --- | ---: | ---: |
| ballot-setup | 2516.938 | 15.871 |
| ballot-prove | 3144.875 | 148.448 |
| state-setup | 4589.141 | 38.611 |
| state-prove | 6290.391 | 234.876 |
| aggregate-receive | 10.078 | 0.121 |

峰值来自各新进程getrusage高水位，不相减推算“增量内存”。Prove进程不运行Setup，但包含公共档案加载、
PK加载与证明后的验证，不能称为算法专属内存。子进程每1秒采样RSS作16GiB预算保护，不代替高水位数据。
每工作进程上限600s、整轮1800s、构建jobs=2、Rayon=4；未提高协议参数或更换密码学库。

首次run-20261114-01保留为中止诊断：生成2份Setup和8份选票证明，因重复校验本地压缩PK的开销主动停止state-prove，
并非无效证明或达到预算上限的证据。其选票PK加载约249.518s，与实际8票Prove约65.046s分开记录。
修正仅针对本流水线刚生成的私有PK缓存：先检查完整SHA-256，再解码，并与冻结VK逐字节比较；
不用于外部PK、证明、声明或群见证。外部证明仍规范解析，所有实际Groth16/SnarkPack运算不变。
增加缓存变更拒绝测试；没有以未约束host群结果代替R1CS。

本轮两次尝试累计4次Groth16 Setup、16份选票证明、2份状态证明、2份聚合；
正式成功面板仅使用run-02的8+2+2，不混入run-01样本。P3/EVM新执行均为0。

## 验收与复现

成功执行命令：

```sh
node scripts/stage2_matched_cost.mjs experiments/stage2_matched_cost_raw/run-20261114-02
node scripts/stage2_matched_cost.mjs --check experiments/stage2_matched_cost_raw/run-20261114-02
node scripts/stage2_inventory.mjs --check experiments/stage2_matched_cost_raw/experiment_inventory.json
```

首次命令只允许新目录；再次运行请更换目录，不覆盖历史数据。完整构建/审计/测试命令与日志在execution.json和logs/。
Release build、fmt、严格Clippy、5项相关测试通过。4项实际密码学负例通过：两个逐票路径分别用另一选民证明，
进入Groth16后拒绝；Direct Fold和State路线篡改第一批newState，在接收/冻结之前通过SnarkPack，
再被相应累计检查/状态Groth16拒绝。失败均保持checkpoint和NF不变。

代码基准：c160a430c0aa971250626e520209621c7673d389；分支：experiment/stage2-matched-cost-pilot。执行时存在新文件，**不是干净提交复现**。
执行源码版本SHA-256：1f610fc6c2c485bdd7daf4146f97f748aae6d99f31cae9704abbea6f0800fb12。
二进制SHA-256：5a7dbc029fada6644efe7a85e0145cd647fb2add46d014c6dec4d4569827e0e0。
配置SHA-256：00c3869c4b10a6e48fa5972b2a98767966db2b2fac0de3692eb8cbbc13cafe5e。
Arkworks选票VK：6d00cab6cd02e9926bd8ef05f8e782fbf1f32b5993c134699155accf7b37aa98；状态VK：92871cac5c3452ab0a1647939507d7b041dcccf189e3b60ea2b19c1d6e1e1856。
Bellperson兼容VK摘要与Arkworks序列化摘要口径不同，原始pins分别保存，不能混称同一编码。
全部原有tracked文件在运行前后逐字节哈希一致；source_snapshot/保存实际执行源码。

公开材料：ballot-prove.json、state-prove.json、aggregate-receive.json保存新证明与摘要；
原矩阵/CT/认证上下文复用已固定的G2-E fixture，通过pins关联。没有发布私钥、选民见证或可恢复tally的份额。
私有临时PK不进入Git，均为可重建研究参数；非生产可信设置，重建同一旧选举而非新的独立DKG。

## 当前边界及下一步

任务0：已索引66类历史材料、184份主要JSON，
核验主要报告与来源，不声称把所有历史密码学实验重新执行。Belenios的105份旧记录通过审计，但临时可执行文件失效，
尚未恢复工具链；不同权重、门限模型和计时边界仍不作跨系统排名。

任务1：最小同条件生成面板完成；独立工作负载重复、完整业务时延与最终论文图表尚未完成。
下一项仅设计/实现同一逐票Groth16回退后端下的B4 Jubjub Direct Fold与State Proof公平EVM比较。
先保持现有两批、同认证/根/Nullifier/状态写入语义，不扩B/N、不重做P3、不改冻结电路。
精确SnarkPack EVM继续BLOCKED，Gas为null；独立DKG、持久化注册、部分树、通用多区域和T4/T5仍保留。
本轮不修改论文，不提交或推送Git。
