# G12：SnarkPack EVM 完整验证可行性审查

## 结论：BLOCKED_G12_FEASIBILITY

日期：2026-11-15（Asia/Shanghai）。本轮是**源码与既有证据审查**，不是新性能实验或完整验证器。
基于最新 J1 提交 `fff01791f51dc87e695523c53ed8e6f7e88baee8` 新建 `experiment/g12-snarkpack-evm-feasibility`；Stage2 `1ade01f12bc95ab00a514a7986a984816fe1741b` 仅作只读参照，不退回旧版本、不丢弃 J1。
**不批准进入 G13/B2 完整 EVM，不自动实施包装或联合证明。** 本轮到此停止，不提交、不推送。

本轮新增 EVM 交易、Solidity/Rust 编译、Setup、Prove、完整密码学重放、群运算探针均为0。
核对基线 2597 个不可变已跟踪文件的 Git blob 与当前字节；旧代码、参数、fixtures、结果与论文不改。
只有 NEXT_STEPS 和 IMPLEMENTATION_STATUS 两个滚动索引更新，历史版本仍可从基线提交取得。
已有 p3_curve_compare.csv 的工作树 CRLF 与 Git LF 不同（core.autocrlf=input），初次原始 blob 检查因此停止且未生成报告。
随后确认其当前 SHA-256 与 J1 历史字节清单一致、仅换行归一化后与基线 Git blob 一致；脚本同时检查这两项并记录例外，**不改写 CSV**。

## 1. 为什么不是“再写一个 Solidity 函数”

原 SnarkPack V2 不只有最终 Groth16 配对式，还包括规范解析、原挑战、GIPA、KZG、TIPP/MIPP、独立 G1 终点及任意 GT 目标。
[EIP-2537](https://eips.ethereum.org/EIPS/eip-2537#abi-for-pairing-check)只返回配对乘积是否为单位元，不返回 GT 或接受任意 GT 目标。
MSM/配对带子群检查，加法不带；不能用加法预编译代替验证输入域。
[EIP-2565](https://eips.ethereum.org/EIPS/eip-2565)是整数模幂，不是 Fp12 或完整 Miller/最终幂验证器。
因此存在原语接口不匹配，但这**不是原生精确验证在所有资源预算下不可能的证明**。

## 2. G9 证据到底完成了什么

- G5-C2：原生/JS 两个用例共12次挑战、编码及拒绝计数匹配；不是 Solidity FS 完成。
- G5-C3：一条 TIPP 终点 native/JS 对照；不是全部 TIPP/MIPP/KZG 验证。
- full64 软件 Miller：原交易 `0xc0a9fa68b027043a6d579a16dc3ad0a7a5ba001f9d10d2216e479fed86e92b8c` 在20,000,000 Gas预算 OOG。
  该数字是**失败封顶支出**，不是完成一次 Miller 或 SnarkPack 的成本；最终幂未完成，本轮不重跑或提高预算。
- G9-P1/v2：已列出10条配对等式、独立 MIPP G1 终点、B2 标量公式、有限寄存器布局提案及 sentinel 风险；状态仍为 `BLOCKED_NO_COMPLETE_GAS_AND_EQUIVALENCE_BOUND`。
- Stage2 两路接收器均使用真实逐票 Groth16 回退，不是精确 SnarkPack EVM；不把其 Gas 当成聚合验证成本。
- J1：原生 Fq(381-bit) gadget 编译控制通过，直接要求当前 Fr(255-bit) 后端时 E0271；未构建 R1CS。
  这仅排除**当前锁定后端直接复用**，不排除其他递归后端或 zkVM。没有新递归性能数据。

来源路径、各 SHA-256、锁定包版本及逐条源码行/anchor 见 [verification_inventory.json](../results/g12/verification_inventory.json)。
上游参考为本机 Cargo.lock 锁定的 bellperson 0.27.0 / blstrs 0.7.0；verify/transcript SHA 与旧 C2 相符，不下载或修改依赖。

## 3. 完整验证清单

下表 native 表示上游存在该实现，不是本轮重新执行或证明了实现安全。EVM 的 PARTIAL 仅表示已有组件/对照证据。

| 项目 | 检查 | 原生源码 | EVM 状态 |
| --- | --- | --- | --- |
| PARSE | 解析、规范编码、子群及参数绑定 | IMPLEMENTED | PARTIAL |
| FS | 完整 Fiat–Shamir | IMPLEMENTED | PARTIAL |
| GIPA | 全部 GIPA 折叠 | IMPLEMENTED | NOT_IMPLEMENTED |
| KZG | KZG-V/W 四项 | IMPLEMENTED | NOT_IMPLEMENTED |
| TIPP | TIPP 三项目标等式 | IMPLEMENTED | PARTIAL |
| MIPP | MIPP 两项与独立 G1 终点 | IMPLEMENTED | NOT_IMPLEMENTED |
| AGGREGATE | 最终聚合与全部公开输入 | IMPLEMENTED | NOT_IMPLEMENTED |
| GT | GT、Miller 与最终幂 | IMPLEMENTED | PARTIAL |
| ACCUMULATOR | 随机累积与替代语义 | IMPLEMENTED | EQUIVALENCE_UNPROVEN |

### 解析与 FS 的不可省略边界

B2 压缩证明为15个 GT、9个 G1、5个 G2及一个 u32，共5236 B；2行×25个 Fr 输入另占1600 B。
需保持字段规范性、曲线与子群、全部字节消费/规范 roundtrip、B和长度一致，固定完整 VK/SRS。
VerifierSRS 的 PartialEq 未覆盖 n/h_alpha_d，不能靠对象相等代替完整参数绑定；本报告另存包含所有字段的规范 JSON 摘要，其编码不冒称标准 SRS wire。
原生 read/parsing_check 与应用加载/绑定分工存在，指出 EVM 缺口不代表声称原生 parser 有新漏洞。

GT 压缩 wire 为6×48 B LE；FS bincode 序列化用完整12个系数；EIP 字段为64 B BE，三者不得混用。
torus 解压、非零分母、子群及原可编码域要逐项保持，不能发明 identity 特例编码而宣称兼容。
V2 仍使用 `snarkpack-v1` 前缀。此64位证据域 nonce 是8 B BE，从1开始，将历次拒绝计数累计到同一 SHA 状态；shave 后按 Fr LE 表示解析，拒绝0/1及非规范值，不能改成取模。
六次 B2 挑战为 hcom、random-r、gipa-0 seed/round、extra-link、random-z；外部 transcript 必须绑定全部非固定顺序输入。
session 的额外绑定独立于原 FS，不能悄悄改变原挑战域。

### B2 的10条配对式与额外终点

记 a=c_inv、c=a^-1，r 为 random-r，z 为 random-z；G1/G2用加法，GT用乘法；A/B/C是 GIPA 最终点。
下表是**来源映射及关系层候选**，不是已经证明等价的 Solidity 实现；行号均对应锁定 verify.rs。

| 关系 | 逻辑配对项 | B2 公式 | 源码行 |
| --- | --- | --- | --- |
| KZG_V_ALPHA | 2 | `e(-g,V_alpha-h*f_v(z))*e(g_alpha-g*z,pi_v_alpha)=1` | 801 |
| KZG_V_BETA | 2 | `e(-g,V_beta-h*f_v(z))*e(g_beta-g*z,pi_v_beta)=1` | 801 |
| KZG_W_ALPHA | 2 | `e(W_alpha-g*f_w(z),-h)*e(pi_w_alpha,h_alpha-h*z)=1` | 876 |
| KZG_W_BETA | 2 | `e(W_beta-g*f_w(z),-h)*e(pi_w_beta,h_beta-h*z)=1` | 876 |
| TIPP_Z | 1 | `e(A,B)=ip_ab*Zab_L^c*Zab_R^a` | 493 |
| TIPP_T | 2 | `e(A,V_alpha)*e(W_alpha,B)=Tab*Tab_L^c*Tab_R^a` | 495 |
| TIPP_U | 2 | `e(A,V_beta)*e(W_beta,B)=Uab*Uab_L^c*Uab_R^a` | 498 |
| MIPP_T | 1 | `e(C,V_alpha)=Tc*Tc_L^c*Tc_R^a` | 508 |
| MIPP_U | 1 | `e(C,V_beta)=Uc*Uc_L^c*Uc_R^a` | 510 |
| GROTH16_AGGREGATE | 3 | `e(alpha*(1+r),beta)*e(IC0*(1+r)+sum_j IC[j+1]*(X0[j]+r*X1[j]),gamma)*e(agg_c,delta)=ip_ab` | 192 |

共18个逻辑配对项；另必须检查 `C*(1+a*r)==agg_c+c*Zc_L+a*Zc_R`（verify.rs:519），不能漏掉或算成第11条配对式。
f_v(z)=1+a*z，f_w(z)=z²*(1+c*r^-1*z)，final_r=1+a*r。完整公开矩阵同时参与原应用 transcript 与最终 IC 线性组合。

### 随机累积不能无论证地确定化

原 PairingChecks 用随机非零 Fr 系数合并多项，最后执行一次最终幂；测试 harness 的已知固定种子不具备自动的对抗性健全性。
实现中 `Gt::generator()` 还承担特殊 neutral sentinel，而 blstrs 实际乘法单位元是 `Gt::identity()`，二者不同。
必须区分 IdentityTarget 与 ActualGT(value)，不能全局把真实 generator 值改写为1。
“逐条检查所有数学等式”是可讨论的替代关系，但不等于对所有坏证明、随机种子、sentinel 角落逐比特接受等价；需明确有效域、完备性及健全性/误差。
本轮不报告新攻击，不修订上游，也不将源码 anchor 检查当成等价性证明。

## 4. 资源：能给出的数字及不能给出的数字

| 数量 | 值 | 性质 |
| --- | --- | --- |
| Historical full64 failed spend | 20,000,000 Gas | HISTORICAL_MEASURED_FAILED_SPEND |
| Four two-term KZG pairing calls only | 411,600 Gas | STANDARD_FORMULA_PARTIAL |
| Archived compressed B2 proof plus scalar matrix | 6,836 bytes | DERIVED_MATERIAL_LENGTH |
| Expanded GT field representation proposal | 11,520 bytes | DESIGN_PROPOSAL |
| G9 partial fixed-exponent operation subtotal | 64,518 square_or_multiply | PARTIAL_REFERENCE_ALGORITHM_UPPER_SUBTOTAL |
| G9 proposed finite register/storage allocation | 41,280 bytes | DESIGN_PROPOSAL |

四项 KZG 的411,600 Gas仅是标准配对调用公式，不含其他计算/调用/交易，也不是新 receipt。
64,518 是旧提案里固定幂子程序的朴素 square/multiply **局部上界小计**，既非完整上界，也非下界，不能换算成完整 Gas。
41,280 B/1,290 slots 是未编译布局提案，不是证明过的最大活跃存储。
每笔20M、block上下文30M仍只是既有本地 Prague 研究策略，不声称当前公链永久限制。

完整解析/FS、GT解压和幂、Miller、最终幂、检查点以及交易开销均未闭合。
**完整每步/总 Gas、最大交易数、最大存储、完成时间、完整操作数上界全部为 null。** 尚未选择并准入总交易/总时间/总Gas上限。
拒绝采样是数据相关循环，没有本轮可证明的实际最坏执行上限；运营性超时只能 ABORT/INCOMPLETE，不能说 INVALID_PROOF 或宣称完整输入域无条件完成。
本机 watchdog 不等于公共链收录或最终性时间界。
完整分类、排除项和各阶段未知值见 [resource_budget.json](../results/g12/resource_budget.json)。

## 5. 多交易路线：已有可审查骨架，尚无完整实现

旧 IBatchVerifier 是同步 view，G8 接收器构造时固定创建逐票回退验证器；不能原地址直接换成异步分阶段证明。
未来必须另建独立版本，原 G8 合约与历史结果不动。

1. BEGIN：唯一 Receiver 绑定 chain/address、suite/version、完整 VK/SRS、proof/matrix/external-transcript、B、eid/context、注册/已接受批次根、区域/index、old/new CT。不写已接受 CT/NF。
2. ADVANCE：从存储的认证状态按 micro-PC 推进 Fp→Fp2→Fp6→Fp12、GT解压/乘幂及 Miller。保存系数/bit/pair游标、G2线状态和有符号x循环位置；调用者提供的结果不能直接信任。
3. FINISH：九类检查、10条配对式和独立 G1 终点全部通过才能 VERIFIED；部分通过不是接收授权。
4. CONSUME：唯一 Receiver 重查当前旧CT、index、root和新鲜 NF，对同一矩阵验证状态证明，一次性原子消费并更新 NF/CT。
5. ABORT：未完成或预算耗尽不改变已接受业务状态，也不得升级为 VERIFIED。

Fp使用规范<p的两u256表示，Fp2:u²=-1，Fp6:v³=u+1，Fp12:w²=v。检查点还需版本、phase、PC、规范寄存器和完成位图；逆元可由见证提供但必须检查乘积为1。
Miller线嵌入、负x共轭、塔域顺序、最终幂约定、每阶段检查点到原关系的归纳对应仍未实现/验证。
有一个有限结构提案不代表已具备逐步 Gas 证明、完整状态恢复实现或最大资源上界，因此路线A仍不准入。

## 6. A/B/C 三路线决策

| 路线 | 本轮决定 | 保留语义与成本瓶颈 |
| --- | --- | --- |
| A 原多交易精确验证 | BLOCKED，不进入 G13 | 保持完整原关系，但缺 GT stepper、随机/typed-target语义、异步接收及全部资源界；不是数学上已证不可能 |
| B 递归/zkVM 包装 | 下一轮**独立可行性审查候选** | 需 guest可移植性、完整内部验证、program/journal/状态绑定、新假设/VK/设置；生成与外层Gas均未知 |
| C 新统一批次证明 | 备用，需新的协议范围 | 新关系/数据流/电路/VK/Manifest与安全论证，不能当无成本优化 |

B 不得假设 bellperson 的 Rayon、blstrs/BLST FFI 和熵源能直接在 guest运行；本轮没有安装后端、guest编译或测量。
即使 guest执行成功，也不能让恶意证明者选择随机数或采用公开固定种子重放随机批检后，未经论证就声称原健全性成立。
需验证完整确定性关系，或另行论证随机化来源与安全误差，并把完整矩阵、VK/SRS、证明、选举/批次/状态与程序身份绑定到公开 journal 和外层合约。
现有 J1 Fq/Fr 不匹配不能仅改类型别名解决，也不能据此宣布 zkVM 不可能；新后端/曲线/VK/可信设置必须单独准入，不沿用旧 Manifest。

C 的聚合器只有选票公开材料，不能假设持有选民私有 witness。必须真正验证旧证明，或明确重设投票者到证明者的数据流/套件；host-language 成功值不是约束。
没有证据表明B或C一定更快、更便宜或已经可兼容。保留现有真实逐票回退能力及非 SnarkPack 声明边界。

## 7. 六项验收回答与后续边界

1. 真正障碍：任意GT目标与预编译接口不匹配，完整 parser/FS/全部关系及接受语义、资源调度未闭合。
2. G9已完成：跨语言编码/挑战、一个终点、10式+G1终点映射、B2公式/材料、有限设计、真实失败证据；未完成验证器。
3. 多交易路线：有可审查骨架，没有完整已实现且证明终态等价的执行路线。
4. 未知界：完整各阶段/总Gas、交易数、存储、运行时间及实际拒绝采样预算；均保持 null。
5. B2完整EVM：**不允许**；G12为 BLOCKED，不把产物自检 PASS 当作可行性 PASS。
6. 替代方向：先另行授权B的可移植性/语义审查，C为备选，不继续盲写原生 Miller 或自动扩大预算。

未来只有语义与资源门槛闭合后才考虑原 W32 B2 正例和单字段密码学反例。反例保持 proof/VK/SRS，仅改第二票 input[7]并重建对应 transcript/session，必须由完整验证拒绝，不能仅以旧session不匹配冒充。
本轮没有执行该反例，原 D0 B2 case 也没有归档 negative 结果；旧 G5-C1 的另一份证明/挑战不可拼接到 W32。
继续公开记录研究 Fake SRS 和确定性实验设置，不宣称生产可信设置、完整链上批量结算或公链部署。

## 8. 产物与复核

`node scripts/g12_feasibility_review.mjs --write` 首次生成本轮独立产物；已存在但不匹配时拒绝覆盖。
`node scripts/g12_feasibility_review.mjs --check` 只读核对源码、历史字节、报告和摘要，未来提交后也可运行，不要求HEAD仍等于基线。

- [verification_inventory.json](../results/g12/verification_inventory.json)：SHA-256 `1b92b7ae4072088397f94088ef84a90ff726ed3ef1f5e6f099d09ba3425ec1b4`。
- [resource_budget.json](../results/g12/resource_budget.json)：SHA-256 `a223453bc9226dc8abbf174007ba29857f6c599eb8282e1223d4b0f308040cd9`。
- [route_decision.json](../results/g12/route_decision.json)：SHA-256 `73b72b54026f57c07740141f60dd4cf4929cd6b14266087cc66623b83b29bd99`。
- [SHA256SUMS](../results/g12/SHA256SUMS)：包括上述JSON、本报告和生成脚本；文件名相对仓库根目录。

复核成功只表示当前审查产物与证据一致。**最终可行性状态仍是 BLOCKED_G12_FEASIBILITY，完整 SnarkPack EVM=false。**
