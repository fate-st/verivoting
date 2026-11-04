# P0：本地论文证据与视觉验收

2026-11-04，基准7475f56。本轮未重新运行任何P0密码学实验，也未改论文正文。

- 独立论文与Git同步快照字节一致；SHA-256 `a2a3903afadc70982ad16f62830223aa12e08fb93c7e9de8ec9428e0d4023935`。
- 原9组G1-E/G2-E/G4-D/G6/G7/G8-C/D/E/G9来源JSON逐文件SHA复核不变，复现入口仍由原claim_evidence映射。
- 冻结D5/W32 Manifest SHA为`00c3869c4b10a6e48fa5972b2a98767966db2b2fac0de3692eb8cbbc13cafe5e`。
- 已有编译PDF共67页；使用本机现有Ghostscript渲染72dpi全页，8张九宫格总览均视觉检查，46/47页当前实验表及成本口径另以120dpi放大检查。
- 未观察到裁切、重叠、公式/表格溢出或引用占位符；编译log无Overfull/未定义引用/LaTeX错误。这不是逐字符校对、参考文献事实核验或数学审稿。
- Introduction三项贡献是设计目标，不是“已完成精确SnarkPack EVM”的实测宣称；Abstract/实验/Discussion/Conclusion明确保留备用逐票链上路线和T4/T5开放边界。本轮不改Introduction或形式化环境。

未安装Poppler或新依赖，使用现有Ghostscript及系统PDFKit/AppKit制作只读核查材料；PDF原字节未编辑。
`results/p0_visual_review/sn-article.checked.pdf`为已核查PDF的原字节副本，SHA-256 `06d6df6f5b9f72ced5b649edd22ebe029bc8fb81f293699c723d8339afdfe6cd`。
机器记录`results/p0_visual_review/review.json`保存PDF/论文/配置/代码基准、67页渲染SHA和人工查看范围。
临时PNG/九宫格未作为论文图表，不将它们冒充实验性能图片。

源快照仍依赖原模板类和参考文献工程，不是独立模板发行包。代码执行基准SHA是7475f56，
G10-A本轮新增源码执行时dirty=true，由raw的源码/二进制SHA准确追踪；不能把它写成最终干净源码独立复现。
目前PDF不包含本轮新G10-A结果，它们在独立报告中；待后续最终实验章节统一采纳，避免单次修改污染原P0归档。
