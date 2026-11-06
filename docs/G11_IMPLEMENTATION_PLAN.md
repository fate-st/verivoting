# G11 本轮实施设计

基准：5f170c5a904d825a99c96fd45558a743f2c4e1ea；独立分支 experiment/g11-comparative-evaluation。
按上传方案最后建议，仅任务1/2。设计检查：不改密码学、不读私密 Full Tally、不覆写历史、不把文献属性变成计时数据。

## 文件与数据流

1. configs/g11/framework.json 固定框架准入配置。N=8/C=3/B=4仅用于模型验证，不代表外部方案已支持该权重或已产生选票。
2. scripts/g11/model.mjs 定义 ElectionConfig、WorkloadConfig、BenchmarkResult。未测值为null，失败/超时保留日志。
3. scripts/g11/run.mjs 分别/批量运行环境清点和文献检查。只允许新输出目录，保存源码/配置SHA、工具链、硬件和种子。
4. experiments/g11/literature/evidence.json 是人工证据的唯一来源；literature.mjs 导出 papers.yaml、三份CSV和六份Mermaid图。
5. PDF和CSV均来自同一数据。每个明确属性必须有原文页码/章节；unknown不冒充否定结论。Emami若无完整原文则保留资料阻塞。
6. results/g11 保存本轮日志、摘要、可视化和SHA256SUMS；旧目录只通过只读验收脚本核查。

## 验收顺序

先运行已有G1/G2/G4归档核查和G8/G10真实交易/证明的只读审计，再测试新模型和成功/失败/超时记录。
文献读取相关完整章节，不以摘要填属性；PDF视觉审查后封存。没有改Rust/Solidity，不重做大电路Setup/Prove/Gas。
原始论文只在临时目录读取，归档只保存来源、下载SHA及定位，不上传第三方论文全文。

## 后续准入

本轮不自动进入任务3–6、不扩大B64/N1024、不改论文、不推送Git。
外部复现前先固定原仓库/版本和可比操作；G9精确EVM、独立DKG、T4/T5仍保留未完成状态。
