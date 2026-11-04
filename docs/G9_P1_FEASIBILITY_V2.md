# G9-P1 v2：关系对应、有限续算与资源可行性审查

2026-11-04。基准7475f56；只新增审查，不覆盖原P1设计/结果，不修改bellperson、原SRS/VK/Manifest或G8-D。
**结论：BLOCKED_NO_COMPLETE_GAS_AND_EQUIVALENCE_BOUND；P2/P3不启动。**

本轮将“清单有十条”等进一步细化成B2专门化公式和有限寄存器设计，但尚不满足用户要求的全部Gas/总交易数/时间和等价性验收。
记录中所有未知项保持null；下面的操作数/布局是设计推导，不是EVM实测。

## 1. 锁定来源和记号

机器清单：`results/g9_p1_v2/feasibility_review.json`；生成/只读复核：`node scripts/g9p1_feasibility.mjs --write/--check`。
读取Cargo锁定bellperson 0.27.0的verify/prove/accumulator及blstrs 0.7.0的gt.rs，逐文件SHA和行锚点归档。
复用原W32 B2的6项挑战，不生成证明、Setup或EVM交易。

令a为gipa-0-round派生的c_inv，c=a^{-1}，r为random-r，z为random-z。
A/B/C和V/W为原proof的final点；alpha/beta为VK常量，不与挑战a混淆。
G1/G2写加法，GT写乘法，所有标量在BLS12-381 Fr内；X0/X1为两行各25项原输入。

固定B2只有一轮，因此原product-form函数给出：

- f_v(z)=1+a*z，调用原challenges_inv。
- f_w(z)=z^2*(1+c*r^{-1}*z)，调用原challenges和原r^{-1}移位。
- final_r=1+a*r，final_zc=agg_c+c*Zc_L+a*Zc_R。
- r_sum=1+r。原代码用(r^2-1)/(r-1)，原挑战采样排除0、1；该专门化不改变接受域。

脚本实际用归档挑战做Fr求逆/标量计算并记录结果；这是源关系对应，不是完整GT/证明验证。

## 2. 十条配对关系以及独立MIPP终点

| ID | 原数学检查；每条末端都必须执行 | pair数 |
| --- | --- | ---: |
| KZG-V-alpha | e(-g,V_alpha-h*f_v(z))*e(g_alpha-g*z,pi_v_alpha)=1 | 2 |
| KZG-V-beta | 将上行alpha换为beta | 2 |
| KZG-W-alpha | e(W_alpha-g*f_w(z),-h)*e(pi_w_alpha,h_alpha-h*z)=1 | 2 |
| KZG-W-beta | 将上行alpha换为beta | 2 |
| TIPP-Z | e(A,B)=ip_ab*Zab_L^c*Zab_R^a | 1 |
| TIPP-T | e(A,V_alpha)*e(W_alpha,B)=Tab*Tab_L^c*Tab_R^a | 2 |
| TIPP-U | e(A,V_beta)*e(W_beta,B)=Uab*Uab_L^c*Uab_R^a | 2 |
| MIPP-T | e(C,V_alpha)=Tc*Tc_L^c*Tc_R^a | 1 |
| MIPP-U | e(C,V_beta)=Uc*Uc_L^c*Uc_R^a | 1 |
| Aggregate | e(alpha*(1+r),beta)*e(IC0*(1+r)+sum_j IC[j+1]*(X0[j]+r*X1[j]),gamma)*e(agg_c,delta)=ip_ab | 3 |

共18个逻辑pairing项。另有不可省略的G1等式：
`C*(1+a*r) == agg_c+c*Zc_L+a*Zc_R`。
它不是多加一条pairing，也不能用“配对检查完成”代替。
Parser、SRS大小/VK arity、规范编码、所有50项输入、全部FS挑战和五个GIPA GT折叠目标也都必须先绑定。

## 3. “等价”必须说明是哪一种

逐条等式可直接与原verify.rs的kzg_check_v/w、verify_tipp_mipp和aggregate分支对应。
在规范解析/FS/折叠均正确的前提下，所有等式的合取检查了目标代数关系，不存在随机抵消的误接受。
但它不等于对每个畸形proof及每个随机种子都复制原随机批处理器的返回值；随机批处理本身具有错误概率。
因此不能把“数学关系逐项对应”写成“固定原生RNG的接受集合逐位相同”。

特别是原accumulator的`generator`标记：

1. blstrs::Gt::identity()是Fp12::ONE；generator()是e(G1基点,G2基点)，并非ONE。
2. accumulator把该具体generator值当特殊neutral sentinel：跳过随机幂或合并。
3. 本轮独立JS标准群探针实际验证generator不等于ONE，且sentinelMerge(generator,h)与字面GT乘积不同。
4. KZG代码中“=1”的控制标记应显式标注为IdentityTarget；真实GT目标必须作为ActualGT(value)处理，不能全局把数值generator改写为1。

这不是对完整SnarkPack的攻击测试，也未证明旧库安全性有误；它是阻止不正确移植的语义警示。
采用合取方案前，仍须说明哪些原实现特殊值分支属于控制标记、哪些属于真实群值，并用原proof/边界向量审查兼容域。
本轮不擅自修补上游库，不更改论文套件或协议假设。

## 4. 有限续算的具体表示与检查点

固定规范塔：Fp2=Fp[u]/(u^2+1)，Fp6=Fp2[v]/(v^3-u-1)，Fp12=Fp6[w]/(w^2-v)。
每Fp存两u256 limb，严格验证值<p；不是直接写入blstrs的Montgomery内部字。
原wire/FS的48B LE系数字节与EIP-2537的64B BE补零字节分别转换，不能混用。

| 原语/阶段 | 状态和有限推进 | 必要校验 |
| --- | --- | --- |
| Fp/Fp2/Fp6/Fp12乘法 | 输入/结果寄存器，系数i/j、乘加/约简micro-PC | 每limb规范；只接受合约自行计算的结果；若使用逆元见证，必须非零并验证x*y=1 |
| GT解析 | 15个压缩值的index、torus展开和q幂bit cursor | 压缩往返、非零、q子群；维持原wire的可编码域，不自创单位元编码 |
| GT幂/五目标折叠 | 原base、acc=1、指数、bit cursor及square/multiply子阶段 | 指数来自已认证Fr/原FS；完整完成后才能推进目标 |
| Miller | 当前pair/bit、G2线点状态、Fp12积和line micro-PC | 首位初始化、负x共轭、线嵌入、Fp12顺序须与锁定实现向量一致；当前未实现 |
| 最终幂 | 固定E=(p^12-1)/q；Fp12 base/acc/bit cursor | 完成全部固定指数位后精确比较目标；未经验证不能改用部分easy/hard片段 |
| 跨交易恢复 | session/version/phase/PC/cursor/寄存器/check bitmap | 只能从上一笔已提交状态恢复；失败交易不推进PC，输入envelope不变 |

BEGIN/ADVANCE/FINISH/CONSUME/ABORT仍是原P1的独立新Receiver设计，不改G8-D。
NF和已接受CT只允许在全部检查后由唯一Receiver消费时原子写入；任一前缀或timeout都不能授权收票。
本轮不加并发调度、复杂故障矩阵或主机提供的“已验证”布尔值。

## 5. 可计算的设计量，不冒充完整预算

采用保守二进制square/multiply，每个b-bit指数至多2b次Fp12乘法。
15个GT q子群幂、10个GIPA幂和6次独立最终幂的已列子程序操作数上界合计64,518。
这是该具体参考算法的**部分上界之和**，不是完整验证器上界，也不是所有算法的下界；尚未计入Miller、解析/折叠和存储。
不能拿旧20M OOG除以某个前缀时间，把它换算成这些操作的Gas。

固定session布局提案：原wire164槽、15个展开GT 360槽、展开点76槽、矩阵50槽、scratch512槽、metadata/envelope128槽，合计1290槽/41,280B。
这是待实现的最大预留布局，不是实际占用测量，也未证明完整编译程序的活跃寄存器一定适配；若活跃范围不适配，P1必须继续失败，不能静默越界扩容。

| 用户要求的资源项 | 当前可核查状态 |
| --- | --- |
| 单笔上限 | 沿用本地研究policy cap 20M；不是已证明每阶段能完成 |
| 各阶段Gas上界 | 未有完整编译实现/静态证明，null |
| 最大实际交易数、完整总Gas | 未闭合，null |
| 最大存储量 | 有1290槽布局提案；未编译审计/实测，验收值仍null |
| 最大执行时间 | null；本地watchdog不能保证链上收录时间 |
| P2准入 | FAIL/BLOCKED |

FS拒绝采样还存在独立资源边界：采样次数依赖输入。固定尝试上限或timeout必须返回ABORT/INCOMPLETE，
不能伪装INVALID_PROOF，也不能声称对所有原有效proof具有无条件完成性。完整等价声明必须包含可支持域/预算和资源中止语义。

官方依据：[EIP-2537](https://eips.ethereum.org/EIPS/eip-2537)的pairing-check没有任意GT输出/目标输入；
[EIP-2565](https://eips.ethereum.org/EIPS/eip-2565)只定义modexp分项计费，不提供Fp12/Miller/状态存储完整成本。
本轮查阅标准，没有新增Gas测量。旧411600仅为四条KZG pairing预编译公式分项，不能冒充总成本。

## 6. 下一步严格限制

若继续G9，唯一必要工作是：确定带类型的目标语义/资源中止支持域，编写完整GT micro-program及其逐步不变式，
完成每phase最坏路径Gas和存储活跃范围审计，给出总tx/Gas/时间停止标准。
在此之前，不启动P2、不重跑被冻结Miller、不增加B4接入或wrapper。
若这些设计最终不满足预算，保留BLOCKED，论文报告当前G8逐票备用链上路线；G10-A原生比较不受此阻塞。
