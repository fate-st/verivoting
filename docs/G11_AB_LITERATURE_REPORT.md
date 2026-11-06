# G11-A/B 文献与架构比较

本轮结论：G11 框架可以独立运行，文献/架构比较目前为**部分完成**，不是五个外部方案全部验收。
Helios、Belenios、Ordinos、Kryvos的相关完整原文已读取；Emami仅取得出版及作者资料，未取得完整协议/安全模型。
五个外部方案均建立条目；Emami的属性保持unknown、架构阶段明确待核实，没有据摘要补全。

## 从哪里看

- `experiments/g11/literature/evidence.json`：人工证据与属性定义的唯一来源。
- `results/g11/literature/security_matrix.csv`：6个系统（含本文原型）×11项属性的宽表。
- `results/g11/literature/attribute_evidence.csv`：66条逐属性状态、限定条件、原文物理PDF页码/章节、URL及SHA。
- `results/g11/literature/trust_model.csv`：公众、Trustee和Combiner分别看到什么。
- `results/g11/literature/architecture_matrix.csv`：统一六阶段与密钥/计票分工。
- `results/g11/architecture_comparison.pdf`：6页机制图，其中Emami一页是资料缺口，不伪装为已核实架构。
- `experiments/g11/architecture/*.mmd`：对应可编辑图源码。

CSV与PDF由同一证据库生成。CSV经artifact-tool写入、重算、导出回读核对；六页PDF已逐页视觉检查。
原文仅临时缓存，没有将第三方论文全文复制进Git。

## 本轮比较能回答什么

1. **接受集合与累计状态。** Helios的已定版公告板、Belenios的last-ballot规则和公开审计、Ordinos的去重后签名列表、Kryvos的公开承诺累计，都是已有的一致性检查机制。不能将“没有独立状态SNARK”写成“不能正确累计”。逐项定位见属性表的state_consistency。
2. **最终结果与同一累计目标。** 外部系统根据各自公告板的最终累计对象验证结果；这不等于本文的Solidity冻结seal，也不能据此否认它们的结果绑定。本文现有原型将CT2、认证承诺和60项P3输入在存储中关联，但精确SnarkPack EVM仍未实现。
3. **Tally对谁隐藏。** Ordinos的MPC目标与Kryvos的public tally-hiding目标不同。后者的计票方知道完整tally；公众隐私要求全部计票方诚实，内部隐私另用nt-1门限。本文授权Combiner同样知道tally，尚不能宣称隐私强于Ordinos或已证明Function-Equal组合定理。

加权能力也不是本文独有功能：Helios选定部署使用类别权重，Belenios规范的同态模式支持权重。
未知项不计为“无此功能”；例如没有找到简洁聚合论证的原文证据，不写成其他系统不支持任何批量处理。
本表不做打分或胜负排名，不把不同硬件/安全参数的论文数字搬到同一性能柱状图。

## 来源版本与访问边界

- Helios：2009 UCL/2.0，15页，已核查相关章节；不是2008原始mixnet版本。
- Belenios：官方规范3.0，40页；当前网站已列3.1平台，故不能把本次3.0协议比较泛化成所有历史/最新版本的实现。
- Ordinos：2020作者扩展版，63页，重点核对§2、§3、§4、§5.3的P1-P4及Theorem 2、§7/8的原型设置。
- Kryvos：CCS 2022作者版15页，重点核对§3.1/3.2、§5.2/5.3、Definition 5.3及Theorem 5.4。
- Emami：JISA 79 (2023), 103645。出版页、作者主页和机构记录确认文献身份；机构页面未提供全文。尚不能定位完整安全模型或论文对应代码。需要作者上传合法可读PDF，或提供作者公开全文及原代码链接。

各下载版本SHA及页数经pypdf独立检查，见 `results/g11/source_access.json`。网页版本可变，复现时必须核对哈希。
Ordinos/Kryvos论文列出的旧GitHub实现地址本次访问返回404；这只说明本次无法访问该历史地址，不说明从未公开过实现。

## 实际执行与范围

新模型和运行器已测试参数合法性、缺失指标保留null、成功/失败/超时日志、禁止以摘要冒充完整证据及文件哈希。
已有G1/G2/G4归档核查、G8历史原版审计、G10-C原生报告和真实交易/配对记录审计已执行。
这里是**只读归档回归**，没有新执行旧密码学证明，也不声称本轮重跑了所有Rust单元测试。

首次G8审计因后续合法Cargo变更触发旧“全源码树不变”断言而失败，保留在run-20261106-01。
适配器在4a2cc163原始源码树运行完全未修改的G8审计，939个历史文件、473条归档记录及原21笔交易检查通过。
当前所有G1–G10文件另与5f170c5核对一致，未通过删除旧断言解决兼容性问题。

本轮没有新的外部协议运行、Setup、Prove、EVM交易或Gas测量。运行器耗时仅是框架运行时间，协议指标为空，不填零。
所有新增文件位于 `verivoting-vscode-starter`；没有改主论文、旧实验配置、Rust/Solidity密码学代码，未提交或推送Git。

## 下一步

先取得Emami完整原文以完成G11-A/B的第五个对象；可以独立开始Belenios原版代码/依赖/版本可行性调查。
本轮不自动开发外部协议、扩大B64/N1024、改论文或重启G9 Miller优化。G9、同上下文独立DKG和T4/T5仍是另外的未完成任务。
