# 第一阶段对比：从哪里看、怎么看

本轮只整理已有实验与一手文献，没有重跑 G1-E、G4-D 或新建重型 Gate；不改密码学算法、冻结配置和主论文。

## 先看这三处

1. [文献功能 / 安全属性两表](LITERATURE_COMPARISON.md)：Helios、Belenios、Ordinos、Kryvos、PQKryvos，以及当前原型。附逐项原文定位及论文可用 LaTeX/BibTeX。
2. [G1-E 六组原图与测量解释](G1_E_FIXED_SUITE_BENCHMARK.md)：复用已有 B2–B32 的正式操作统计；没有覆盖、重复生成或混入其他论文计时。
3. [本轮数值摘要及两张状态图](../results/comparison_stage1/summary.md)：直接展示 G2-D 的匹配组件和 G2-E 的连续接收状态。

## 最重要的读图结论

- G1-E：B16/B32 出现本机验证者收益，但聚合生成另有成本，证明和 proof+matrix 也没有变小。不是全系统效率优势。
- G2-D：同一 B4 下 Direct Fold 路线组件和 8.372208 ms，State Proof 为 10.456375 ms。状态证明尚未显示本机速度优势；它的潜在验证卸载价值与额外证明成本要分开讨论。
- G2-E：两批实际接收/验证/应用为 9.462042 / 11.817333 ms，证明连续状态能工作。右图的独立 Direct Fold 是局部检查，不能拿它和完整接收时间相除。
- G2 两图没有误差棒：都是功能样本，不能把两批当作重复统计。Setup/Prove、在线 Verify、累计 RSS 的边界不混用。
- public tally-hiding 不等于向计票方隐藏 tally。当前授权 combiner 会知道 tally；研究种子和 Fake SRS 也不能支持真实部署隐私。

## 本轮核对与复现

脚本 [comparison_stage1.py](../scripts/comparison_stage1.py) 从已固定 SHA-256 的 JSON 读取；校验 G1-E/G2-D/G2-E/G4-D 归档数据及公开 fixture。
历史源码/构建清单曾正常演进，不把旧测量时的 Cargo hash 当成本轮当前源码的 hash；本轮自身源文件与输入分别记录。
另用 `@oai/artifact-tool` 只读导入 G1-E `summary.csv`，核对五行的中位数、字节数、样本数和缺失 P95；不新增不需要的 Excel 工作簿。

```bash
python scripts/comparison_stage1.py --check-only
# 需要 matplotlib；运行至一个不存在的新目录，不覆盖归档：
MPLCONFIGDIR=/tmp/vv-comparison-mpl python scripts/comparison_stage1.py --out results/comparison_stage1
# 可选：使用已安装的 @oai/artifact-tool；或将 ARTIFACT_TOOL_MODULE 指向其模块路径
node scripts/comparison_stage1_csv_check.mjs
```

本机实际使用仓库外已有 Matplotlib 环境。数据/图版本和 SHA-256 见
[comparison_data.json](../results/comparison_stage1/comparison_data.json) 和
[SHA256SUMS](../results/comparison_stage1/SHA256SUMS)。
`--check-only` 不写文件；正常模式不调用密码学二进制、不采集新性能数据。

### 本轮实际验收

- 归档数据与公开材料 SHA-256：G1-E 227、G2-D 8、G2-E 15、G4-D 28 项，共 278 项通过。此计数刻意不包含后续正常演进的历史源码/状态文档。
- `--check-only` 数值与 suite/manifest/VK/完整矩阵指纹/连续状态核对通过；Python 语法编译与 Node 语法检查通过。
- artifact-tool 只读 CSV/JSON 核对：5 个 B、45 个统计/大小值、20 个空 P95 一致，缺失值未补零。
- `latexmk -pdf -interaction=nonstopmode -halt-on-error table-preview.tex` 在临时目录完成两表及五条引用的独立编译；未编译或修改主论文。
- 两张最终 PDF 及临时表格预览均渲染为 PNG 后目视检查，表格采用左对齐减少窄列空隙，图中区分统计口径及局部/完整计时。
- 无 Rust/Solidity 源码改动，未重复运行与本轮无关的 Clippy、Setup、Prove、G1-E 或 Gas 实验。

## 下一步仍只有一项核心实现

G4-D 十种组合已经完成，下一轮把 G2-E 原生 receiver → 一次冻结 CT2 → 私密 CP 包/门限解密 → P3 → 一次性 winner 写入接通。
只需选一种已合法的 Trustee 子集做集成验收，不重证十种组合。随后再做完整合约及公平端到端对比。
Full Tally vs Winner-only、完整 SnarkPack EVM、完整系统 Gas 仍待完成；不拼接历史 W10/W32 或不同 VK 的 Gas。
