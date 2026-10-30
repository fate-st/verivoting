# 第一阶段：文献功能与安全属性对比

核对日期：2026-10-30。仅比较明确版本和安全目标，不把外部论文计时与本机结果排名。
下表是对原文的归纳，不是对这些系统的新安全审计；各安全结论仍受原文完整模型约束。
VeriVoting 一行区分“设计目标”和“已执行的组件证据”，不能与已证明的完整协议直接画等号。

## 1. 功能及结果披露

| 系统 / 比较版本 | 选票及认证 | 聚合与结果计算 | 公开披露 | 批次状态证明维度 |
|---|---|---|---|---|
| Helios 2.0，2009 [H] | Exponential ElGamal、选票合法性 ZKP；选举认证服务 | 同态合计与分布式解密；部署支持类别权重 | 候选人总票数 | 本文未以独立 SNARK 状态转换为研究目标 |
| Belenios 3.0 规范 [B] | ElGamal + 合法性证明 + credential 签名 | 同态模式支持权重；非同态模式使用混洗 | 同态模式公开总票数；非同态模式公开匿名选票列表 | 有事件哈希链；不能等同于我们的状态 SNARK |
| Ordinos，2020 [O] | 加密选票及合法性证明、认证服务和公告板 | 同态累计后用 MPC 比较；实例采用 threshold Paillier | 可只公布 winner 或指定排名 | 已有完整选票至结果协议；不是缺少累计正确性 |
| Kryvos，2022 [K] | 票值全门限分享、Pedersen 向量承诺、加密打开信息及 Groth16 | 同态承诺合计，计票方重建 tally 后证明结果函数 | 可仅公开结果函数值 | 公开可重算承诺合计；未单独评估本文的批次状态证明 |
| PQKryvos，2026 [P] | 格基 BDLOP 承诺、加密分享及 Ligero 证明 | Kryvos 框架的后量子实例，支持灵活 choice space / result function | 可仅公开结果函数值 | 非本文 SnarkPack + 密文状态证明路线 |
| VeriVoting 当前原型 [V] | Jubjub 加权 ElGamal、深度 5 注册证明；研究型设置 | B2–B32 SnarkPack；两个 B4 状态转换；同一 P3 VK 的十子集 | P3 组件只输出 winner；实时完整流程尚未串联 | 已有两次连续原生转换；完整链上结算未完成 |

“未单独研究这种状态证明”不表示系统无法安全合计选票。直接重算完整认证集合的同态和也是有效验证方法。
Helios 2.0 和 Belenios 已有权重功能，不能把“加权”本身称为 VeriVoting 独有贡献。

## 2. 安全属性、计票方权限和信任边界

| 系统 | tally 对谁隐藏 | 关键依赖（摘要，非穷举） | 设置 / 后量子属性 |
|---|---|---|---|
| Helios 2.0 [H] | 不隐藏公开的候选人总票数；目标是单票隐私 | 认证/选民审计及解密方信任；2009 部署采用所有 key holder 参与，不是任意 3-of-5 | 经典离散对数；不使用 Groth16 型电路 CRS |
| Belenios 3.0 [B] | 标准同态模式不隐藏总票数 | credential authority 与 trustees 的选定角色分离、审计；可配置 Pedersen 门限 DKG | 经典群方案；无本文 Groth16/SnarkPack 设置要求；不是 BeleniosRF |
| Ordinos [O] | 在原隐私条件下，也对少于门限的恶意 trustees 隐藏结果以外信息 | 隐私定理的 P1–P4、MPC、公告板/设备和 risk-avoiding 条件，不能简化成无条件隐私 | 原实例为经典 Paillier；原型集中生成 key shares，需区分协议与部署 |
| Kryvos [K] | public tally-hiding；计票方获知 tally | 公共隐私假定所有 talliers 诚实；内部隐私允许至多 n_t−1 恶意 talliers、只保护 tally 以外信息；诚实 BB/调度者 | 经典 Groth16，需正确生成电路 CRS |
| PQKryvos [P] | 同属 public tally-hiding，非对计票方隐藏 tally | 公共/内部隐私区分同上，另须满足其量子对手下原语条件；不是“任一诚实 tallier 就向公众隐藏 tally” | 所选 Ligero 透明，无可信 CRS；后量子安全是完整所选原语的条件结论 |
| VeriVoting 当前原型 [V] | 目标是向公众隐藏 tally；授权 combiner 私有重建 tally | 不保护 tally 免受 combiner 泄露；认证注册/DKG、冻结目标及联合健全性/隐私组合仍需完整论证 | 经典 Jubjub / Groth16 / SnarkPack；确定性研究 Setup、Fake SRS，非生产设置 |

功能测试不是隐私证明：相同 winner / 不同 tally 用例也不能代替模拟式或不可区分性论证。
本轮不做 coercion-resistance、任意 n/t、生产部署或全链上实现的“全是/全否”评分。

## 3. 可追溯的一手来源

- [H] Adida、de Marneffe、Pereira、Quisquater，*Electing a University President using Open-Audit Voting: Analysis of real-world use of Helios*，2009。[USENIX 原文 PDF](https://usenix.org/events/evtwote09/tech/full_papers/adida-helios.pdf)，§§1、2.1–2.3、3.2。比较对象是 2.0，不将 2008 mixnet 原版写成同态版本。
- [B] Glondu，*Belenios specification*，version 3.0。[官方规范](https://www.belenios.org/specification.pdf)，§§1（公开内容/权重）、2、3.1.1、3.4–3.5。不是 RF 或 VS 变体，也不把当前网站版本泛称为所有历史版本。[官方说明](https://www.belenios.org/howitworks.html)用于补充角色分离。
- [O] Küsters 等，*Ordinos: A Verifiable Tally-Hiding E-Voting System*，EuroS&P 2020，DOI 10.1109/EuroSP48549.2020.00022。[作者扩展版](https://publ.sec.uni-stuttgart.de/kuestersliedtkemuellerrauschvogt-eurosp-2020.pdf)，§2、§4、§5.3（P1–P4 / Theorem 2）、§7（实例及集中 key-share 设置）。本次下载的 63 页扩展版 SHA-256：`420353b5fea005c0ba9e0bdb8d85d64741ad3cd26a6549fbc16640eaecc826fc`。
- [K] Huber 等，*Kryvos: Publicly Tally-Hiding Verifiable E-Voting*，CCS 2022，DOI 10.1145/3548606.3560701。[作者论文](https://publ.sec.uni-stuttgart.de/huberkuesterskripsliedtkemuellerrauschreisert-ccs-2022.pdf)，§§2.1、3.1–3.2、5.1–5.3（Definition 5.3 / Theorem 5.4）。本次下载的 15 页版本 SHA-256：`8bc2b323b3adf0448f367db0a4a7e5853fa5a62b85d763e1102a5130571c3d6d`。[ePrint 索引](https://eprint.iacr.org/2022/1132)提供扩展版及出版信息。
- [P] Huber、Küsters、Reisert，*PQKryvos: Post-Quantum Secure E-Voting With Flexible Ballot Formats and Public Tally-Hiding*，PoPETs 2026(4)，1149–1171。[出版原文](https://www.petsymposium.org/popets/2026/popets-2026-0164.pdf)，§§2–4（BDLOP / Ligero）、§6.1–6.2、Appendix C.3（Definition 6 / Theorem 6）。[出版信息与 artifact](https://www.petsymposium.org/popets/2026/popets-2026-0164.php)。本轮不引用其性能数字做本机排名。
- [V] 本地已归档执行证据：[G1-E](G1_E_FIXED_SUITE_BENCHMARK.md)、[G2-D](G2_D_W32_B4_STATE.md)、[G2-E](G2_E_MULTIBATCH_STATE.md)、[G4-D](G4_D_TEN_SUBSETS_P3_V2.md)。这不是外部完整安全证明，也不将论文设计中的目标填为全部实现。

完整论文的措辞应是“研究如何将已认证批次、密文状态和结果披露连接”，不是“首次可验证 winner-only”。
本表不证明独立的理论新颖性，需要结合论文定理与相关工作进一步论证。

## 4. 论文使用

英文两表见 [LaTeX snippet](tables/comparison_stage1.tex)，对应独立引用键见 [BibTeX](tables/comparison_stage1.bib)。
这是待作者确认的插入材料；本轮没有自动改写 `sn-article.tex`，也没有覆盖旧表格。
数值图与读图说明见 [第一阶段入口](COMPARISON_STAGE1.md)。
