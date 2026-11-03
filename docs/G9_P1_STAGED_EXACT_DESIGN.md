# P1：G9分阶段精确验证设计与资源门槛

日期2026-11-03。基准a520404。**P1尚未通过；P2未启动。**
本轮新增的是具体检查/续算设计、源代码锚点和W32 B2离线transcript执行，不是EVM验证器。
旧G8-D、原Manifest、曲线、VK、SRS、原始SnarkPack域和历史数据均未修改。

## 实际执行与来源

复用fixtures/snarkpack_g1d0_depth5_public_v1.json的B2 case：5236 B原聚合证明、两行各25项输入。
从规范矩阵重建外部transcript，再调用既有独立JS解码器；15个GT做规范往返和子群检查。
仅把第二行field7加1并检查仍为规范标量，重建其自己的transcript，不换proof/VK/SRS。
两个不同声明各派生6项内部挑战；hcom不变，其余5项改变。
这验证了编码与挑战依赖，**不证明GIPA/KZG/全部聚合等式或EVM通过**；没有新Rust原生全验证。
第一次脚本误以为旧B2已归档negative，实际其negative=null（旧D0仅最大B做负例）。
已改为在新脚本内派生单字段变更，不改原fixture；不能把新负例称为旧归档负例。

## 精确检查清单

锁定bellperson 0.27.0，机器可读锚点见results/g9_p1/design_inventory.json。

| 检查 | 固定B2工作 | 实现方向与缺口 |
| --- | --- | --- |
| Wire/parser | 15 GT、9 G1、5 G2、1个u32，共5236 B | 压缩字节绑定；允许展开见证，但须检查规范往返、曲线和子群；不能信任host结果 |
| 原FS | hcom/random-r/gipa seed/round/extra-link/random-z | 保留原GT bincode、LE标量、64位累积拒绝计数；逐字节对齐，不擅改域 |
| GIPA | 一轮；10个GT标量幂、两个G1项、五个GT折叠目标 | 合约自行推进算术寄存器和bit cursor；无可用完整GT成本上界 |
| KZG-V | alpha、beta两条，各两对pairing | 保留乘积形式多项式计算和原SRS；两条独立检查 |
| KZG-W | alpha、beta两条，各两对pairing | 保留r逆移位、z^n和原SRS；两条独立检查 |
| TIPP | Z一对、T两对、U两对 | 三条任意GT目标等式，不是一个终点 |
| MIPP | T一对、U一对，另有G1标量等式 | 两条GT目标等式和final_c*final_r=final_zc都必须验证 |
| 聚合Groth16 | 三对pairing与ip_ab比较 | 所有50项公开输入按原r折叠并绑定原VK |
| 最终GT | 六条任意GT目标等式、共10个pairing项 | 多Miller、最终幂及完整GT比较均缺已测的分阶段实现 |

因此共有10条初等pairing等式、18个逻辑pairing项，另有一条MIPP非pairing等式。
计数来自源关系，不是已执行的EVM操作数。

### 不能直接照搬native accumulator

原生PairingChecks用随机非零系数合并关系，测试ChaCha种子不能作为链上随机性。
候选设计逐条验证全部初等等式，避免引入新的链上随机合并策略；这只是实现候选，
在说明其与目标关系的正确性/健全性对应以前，不宣称等价验证器已实现。
还有一项容易漏掉的语义：原accumulator用Gt::generator作特殊“neutral sentinel”，
并在add_if_not_one_gt中跳过它。不能把KZG的这个源码标记机械当成e(g,h)目标，
也不能任意把真实GT值改成单位元；必须按锁定实现逐个解释检查关系。

## 新合约结构（设计接口，尚未实现）

- StagedSnarkPackVerifier：固定程序版本、VK/SRS标识；管理验证session及不可跳步的pc。
- ExactGTStepper：规范Fp/Fp2/Fp6/Fp12寄存器、幂运算bit cursor、Miller曲线状态和最终幂状态。
  每步只从已认证session推进，不以host提供的中间结果作为真值。
- StagedAuthenticatedReceiver：独立新版本；维持原认证/批次/状态证明/NF约束，不改G8-D。

session envelope规范绑定chain、verifier、receiver、Election ID、proof/VK/SRS/有序矩阵摘要、
B、批次序号、batch root、old CT和proposed new CT。它是外层调度绑定，不加入原FS transcript。
这些摘要只绑定对象，不能用摘要相等代替执行原始验证关系。

| 阶段 | 允许写入 | 必须保持 |
| --- | --- | --- |
| BEGIN | 一个待验证session及固定envelope | 只能绑定当前next batch；NF/已接受CT不变 |
| ADVANCE | 从固定pc算出的验证寄存器/cursor/结果位 | 禁止跳阶段、覆盖已完成检查或采信host成功布尔值 |
| FINISH | 在全部检查后标记verified | 任何prefix成功都不能接受选票 |
| CONSUME | 唯一Receiver原子写NF/CT并消费一次 | 重查当前old CT、序号、根、上下文、NF和原状态证明；任一失败整体回滚 |
| ABORT | 关闭本session，清理活动引用 | 不改变已接受NF/CT，不能复活已消费session |

最小设计每选举只允许一个活动next batch。并发调度、复杂恢复和压力矩阵不在本轮范围。
跨交易资源被耗尽只能阻止该session完成，不能让其获得收票资格；无法完成时按既定策略中止。

## 资源预算：可计算部分和未知部分分开

[EIP-2537](https://eips.ethereum.org/EIPS/eip-2537#pairing-check-operation)的pairing-check成本为37700+32600*k。
四条KZG单位目标检查若各用两对输入，预编译费用公式合计411600 Gas。
这只是标准公式的局部派生，**不是实测Gas，也不是完整交易估计**；它排除了MSM、
规范解码、FS、GT、CALL、calldata、存储和交易费用。
pairing-check只返回是否为单位元，不返回GT，不能完成六条任意GT目标关系。

15个GT展开为12个64 B字段各自768 B，总11520 B；这是中间编码体积，不等于必须全部上链/持久化。
原proof+公开矩阵为6836 B，也不是完整ABI/calldata或实际通信量。

| 预算项 | 当前值/依据 |
| --- | --- |
| 每笔Gas上限 | 20,000,000，沿用本地研究上限 |
| block上下文 | 30,000,000，不是实时公链永久参数 |
| 总交易数/总Gas/总时间上界 | 未闭合，JSON保持null |
| GT解析/子群/幂/最终幂完整续算 | 缺有界实现和测量，不以旧OOG猜测完成成本 |
| P2启动 | 不满足资源和语义前置条件，禁止运行 |

旧full64在20M OOG只说明该次尝试用尽预算，不能换算总次数、完整最终幂成本或总Gas。
不提高cap、不重跑被冻结的Miller、不展开算术扫描。本轮没有Rust/Solidity编译、EVM交易或Setup/Prove。
P0论文的LaTeX编译另行记录，不属于密码学编译或证明实验。

## 后续最小验收（尚未执行）

先补齐所有GT原语的有界续算/存储/总交易及总时间设计，并审查逐条等式与原验证关系的对应。
资源设计能够闭合时，才允许P2用同一W32 B2做一个完整正例和一个单字段负例。
负例必须用自己的重建transcript和session，不能以旧session/root不匹配冒充密码学拒绝。
如只能完成FS或GT子组件，继续标组件，不提升exact标记。

复核：node scripts/g9p1_design.mjs --check。每次调用实际执行两例JS transcript；不调用EVM。
P1状态为BLOCKED_P1_RESOURCE_AND_EQUIVALENCE_GATES，不能把设计清单已列出写成设计已验收。
