# G11 本轮复现

基准 5f170c5a904d825a99c96fd45558a743f2c4e1ea；当前开发分支 experiment/g11-comparative-evaluation，运行时包含未提交的G11文件，未声称干净提交复现。

`node scripts/g11/run.mjs --case all --out /ABS/NEW_RUN_DIR`

`node scripts/g11/check.mjs` 仅校验本轮已封存材料，不重新运行密码学。

运行器需要Node和Python；CSV导出依赖Codex独立运行时的artifact-tool，通过G11_ARTIFACT_TOOL_MODULE指定，不安装进原协议依赖。PDF构建需要reportlab，来源版本核查需要pypdf。

每个运行目录保存environment.json、config.json、runs.json、summary.csv、原始stdout/stderr及SHA256SUMS。框架耗时不是投票协议耗时。只读核查记录的历史Gas/Prove不计为新测量。

旧G8审计包含对整个历史源码树的冻结断言，在后续正常Cargo变更后不适用于当前树。首次失败保留；第二次在原4a2cc163版本临时源码中运行同一未修改审计，所有断言通过。另核对整个当前历史部分与5f170c5一致。

公开缓存论文不随Git发布；source_access.json保存四份PDF的SHA-256和物理页码。重新下载时哈希不一致应停止，不接受网站静默换版。Emami条目等待完整原文，不能补猜结论。
