# 阶段二实验版本与证据索引

本轮基准c160a430c0aa971250626e520209621c7673d389，与执行前git fetch核对的origin/experiment/g11-comparative-evaluation一致。
新分支experiment/stage2-matched-cost-pilot。历史原始数据不改、不重新贴上当前HEAD标签。

机器可读清单：experiments/stage2_matched_cost_raw/experiment_inventory.json。
含66类归档、184个JSON的SHA、原始提交/dirty状态、
Suite/电路/VK字段、实测字段及命令日志位置；原档案缺少的字段保持null，而不是补造。

| 证据 | 本轮处理与用途 | 不可声称 |
| --- | --- | --- |
| G1-E | 保留B2–B32统一布局原始统计/图；索引manifest/source | 不重跑或换成本次生成样本 |
| G2-E | 固定两批B4、有序根、CT1/CT2、37输入；新状态证明仍针对同一关系 | 不代表任意批次或区域 |
| G4-D | 索引同一VK、60输入、十种3-of-5组合来源 | 本轮未重新Prove |
| G6/G7/G8-C/D/E | 索引认证、冻结、P3、一次Winner和本地Gas原档案 | 不是精确SnarkPack EVM或公链部署 |
| G10-B1/B2 | 原报告check通过，分清接收、历史生成账本与聚合实测 | 历史分项不拼成新E2E |
| G10-C | report和chain_audit通过，同新P3/60输入/一次Winner | 仍集中研究bootstrap/逐票链上回退 |
| G11-C1 | 官方Belenios3.0的105场独立选举原始check通过 | 不与B4接收毫秒数直接排名 |
| G9-P1 | feasibility --check保持BLOCKED | 不能启动P2或填造Gas |

原g10b1/g10b2/g10c_check.mjs包含其历史开发阶段的路径白名单，不能用作当前HEAD集成准入。
本轮没有改宽白名单或修改被SHA归档的脚本；新驱动调用原report/chain_audit/来源哈希检查，另验证本轮新材料。
新的inventory --check只核对所列原档案和冻结文件哈希，不因未来HEAD推进或新增实验路径而拒绝。
该验收接口在测量结束后修正；实际运行时版本仍保留于run-02/source_snapshot/，不回写执行记录。
完整命令和输出在run-20261114-02/execution.json与logs/，旧tracked文件前后哈希验证不变。

固定注册快照仍可用于D5/W32成本面板。持久化RegisterVoter/唯一注册台账、部分填充树、通用多区域、
独立DKG、精确SnarkPack EVM及T4/T5不在本次已实现范围。与论文阶段一的规范/代码差异保持一致。

本轮成本结果见docs/STAGE2_MATCHED_COST_PILOT.md。只执行必要的一次成功试运行，首次中止日志独立保留。
