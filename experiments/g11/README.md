# G11 比较实验

本轮范围：框架和 G11-A/B 文献/架构比较。外部方案尚未运行，没有外部性能结果。

- `literature/evidence.json`：唯一人工维护的属性证据与架构数据。
- `literature/papers.yaml`：版本及原文来源索引，由证据库生成。
- `architecture/*.mmd`：相同六阶段的机制图，非协议安全证明。
- `baselines/README.md`：后续外部适配约定。
- `winner_only/README.md`、`performance/README.md`：后续准入范围，不预填测量。

执行：`node scripts/g11/run.mjs --case all --out /ABS/NEW_RUN_DIR`。
支持 `inventory`、`literature`、`regression`，分别采集环境、导出文献表、只读核查历史证据。输出目录必须不存在。
运行记录含 JSON、CSV、stdout/stderr、退出码、超时状态和 SHA256SUMS；框架耗时不充作协议耗时。
PDF/表格构建另见 `docs/G11_IMPLEMENTATION_PLAN.md`。所有 G1–G10 记录保持原字节。
