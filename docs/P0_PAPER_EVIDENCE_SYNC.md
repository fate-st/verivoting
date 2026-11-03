# P0：论文与已归档实验同步

基准：a5204043bdcec5a788f79de2ad79225320845e64；日期2026-11-03。
仅同步已有证据，不产生新的计时、证明或EVM交易。论文Introduction和形式化定义未改。

## 更新内容

- Abstract：五个原生批次规模、两批连续B4、同电路/VK十子集，以及本地逐票备用结算。
- 批次实验：G1-E五规模×30轮中位数；固定池/预热/内存采样和非独立选举边界。
- 门限实验：G4-D 6,236,368约束/60输入/10种子集；G6真实原生串联。
- 合约实验：G7、G8-C、G8-D分别列出，不拼接三个版本的Gas。
- G8-E：同一次执行的部署、非部署业务、拒绝测试和嵌套通信口径。
- Discussion/Conclusion及安全状态表：固定备用路径的绑定已实现；精确SnarkPack EVM及隐私组合未完成。
- 不再要求先扩B64/B128/B256才能报告已有结果。

## 声明分级

| 内容 | 证据等级 | 边界 |
| --- | --- | --- |
| G1-E原生计时 | 重复测量 | 同一证明池，不是独立选举；没有SnarkPack EVM Gas |
| G2-E/G4-D/G6 | 已执行功能实验 | 固定n=5/tau=3、两批B4；同进程私密交付 |
| G8-C/D | 本地真实EVM验收 | 逐票Groth16备用路线；不是完整SnarkPack链上流程 |
| G8-E | 已执行收据的派生统计 | 不是新实验；离线成本未重测不填0 |
| 精确SnarkPack EVM | 尚未实现 | G9/P1仍受资源与验证语义门槛约束 |
| Full Tally/Direct Fold完整公平比较 | 尚未完成 | 不从历史组件拼出结果 |
| 联合健全性和Function-Equal组合 | 开放证明义务 | 不用实验通过替代证明 |

## 可核查材料

results/p0_paper_sync/claim_evidence.json为9组实验声明的原始JSON SHA、报告和复现入口映射。
results/p0_paper_sync/sn-article.synced.tex是本轮修改后的论文快照，原论文仍在用户论文目录。
归档基准是已有代码/结果的固定提交，不冒充本轮未提交脚本的干净执行版本。
各历史raw中的原Git SHA、dirty标记、源码和失败日志均不改。

核对命令（不重做历史实验）：

    node scripts/p0_paper_evidence.mjs --check
    node scripts/p0_paper_evidence.mjs --check --paper /ABS/PATH/sn-article.tex
    node scripts/g8e_accounting.mjs --check

论文使用latexmk在独立临时输出目录编译。真实命令/日志在results/p0p1_execution。
LaTeX类文件和参考文献仍由原论文工程提供；快照本身不是一个独立可编译模板包。
