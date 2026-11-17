# Stage1：多批次 EVM 部署摊销与消融（隔离实现）

当前门槛：A-04，提交后准入修复、编译、静态审计及独立 B 预检重建。B/C/D 未获本轮运行授权，禁止自动进入。
本文件不修改论文或任何 G8-E、G12、Stage2、G18 原始材料。

## 固定范围

- 固定提交链：G18 `72ccc561d8934b0d761e17764ef9012bfe012626` → `2444269b52573d9c2f3e551ef7d6b8bb9d0bd008`（59A）
  → 当前HEAD `f086879c4dc247e528d62d2253d76d6afd603669`（3M+31A）。
  父提交Tree固定为 `92f7331a7d9b534d17108409342851381811aa52`；当前Tree固定为 `3b8b57b375bb0391afe431935057a2711c5608e9`。
  区分HEAD、直接父提交、G18基准，分别校验两段静态硬编码文件清单，不从运行环境推导预期值。
  A-04仅修改runner、auditor及本文档；不修改配置、Rust、Solidity、电路、fixtures、旧结果或论文。
- D5/W32，32 名选民均为 fixture 权重1；B4、3候选人、区域4；原有32份Groth16证明与25项输入完整复用。
- Rust/Cargo `+1.99.1`，Release `opt-level=1`，两编译任务/四Rayon线程；不依赖默认stable入口。
- solc0.8.30、optimizer200、viaIR、Prague、EthereumJS10.1.3；20M交易/30M区块上下文。不更改预算来掩盖失败。
- 三路线均是逐票Groth16回退；没有完整SnarkPack Solidity验证器，不把原生时间换算Gas。

## 三路线与冻结

`Stage1Receivers.sol` 内含共用认证/状态基类和三个独立接收器。
Legacy逐票验证原B4承诺下的两层成员路径、上下文、原选票关系、NF和旧状态，并执行真实Jubjub点加。
每四次成功才递增批次；部分批次不得冻结。Direct Fold与State Proof每批单交易原子接收。
Legacy中途失败可能保留此前成功选票，不能将三者中途失败语义说成相同。
公平对齐的是完整前缀输入、最终CT、32项完整NF布尔集合、认证状态和最终冻结结果。

冻结版本为`VV/STAGE1/FROZEN/v1`：SHA256(ABI(string version, bytes32 bindingDigest,
bytes32 scheduleDigest, uint64 K, uint64 count, bytes32[6] CT))。
scheduleDigest为Keccak256(ABI(bytes32[] roots))，根必须是固定八根序列的K项前缀。
不含路线或地址，三路线同K应一致；不冒充旧G6 CT2摘要，不扩展到P3/Winner结算。

### 三路线成本构成（不是实测Gas结论）

| 成本项目 | Legacy | Direct Fold | State Proof |
|---|---|---|---|
| 接收时认证检查 | 每票一次 | 每批一次 | 每批一次 |
| BallotAccepted事件 | 每票一次 | 无 | 无 |
| BatchAccepted事件 | 每批一次 | 每批一次 | 每批一次 |
| 选票Groth16验证 | 每票一次 | 每批4份 | 每批4份 |
| Jubjub密文点加 | 链上逐票 | 链上批量 | 链下状态证明 |
| 状态证明验证 | 无 | 无 | 每批一次 |

三条路线在 `freeze()` 时都会再执行一次认证检查。认证重复次数、事件、交易拆分和框架存储
属于系统框架开销；选票证明、状态证明的生成与验证属于密码学证明开销，不能将全部差额归因于证明本身。

## 严格门控命令（B/C/D均未执行）

门槛A，仅编译：
```sh
node contracts/scripts/stage1_multibatch_compare.mjs --gate A --run experiments/stage1_multibatch_raw/gate-a-20261117-04
node contracts/scripts/stage1_multibatch_compare.mjs --preflight B --a-run experiments/stage1_multibatch_raw/gate-a-20261117-04 --b-run gate-b-20261117-01
node scripts/stage1_multibatch_audit.mjs --gate A --run experiments/stage1_multibatch_raw/gate-a-20261117-04
```
检查已有归档时给审计脚本追加`--check`，不覆盖记录、不重新编译。

获得独立人工确认后才可执行B（以下仅列接口，不表示已经运行）：
```sh
node contracts/scripts/stage1_multibatch_compare.mjs --gate B --approved B --run experiments/stage1_multibatch_raw/gate-b-20261117-01 --a-run experiments/stage1_multibatch_raw/gate-a-20261117-04
node scripts/stage1_multibatch_audit.mjs --gate B --run experiments/stage1_multibatch_raw/gate-b-20261117-01
```
B先新建材料目录、串行生成两份新状态证明，再运行K1/K2三路线回归及三个独立负例VM。
Omission-A在首次接收之前测试，State Proof必须真正触发第五次配对返回0。
Legacy恢复顺序为：Alice/Bob成功→Charlie失败原子回滚→Charlie重试成功→David成功→同笔David交易触发批次完成。
所有失败检查完整Receiver可变元组、32项NF、Gate状态/签署者/认证权重和证书摘要；不要求费用或发送者nonce回滚。

再次获得独立确认后才可执行C：
```sh
node contracts/scripts/stage1_multibatch_compare.mjs --gate C --approved C --run experiments/stage1_multibatch_raw/NEW_C_ID --a-run experiments/stage1_multibatch_raw/gate-a-20261117-04 --b-run experiments/stage1_multibatch_raw/gate-b-20261117-01
```
C从全新材料目录开始，重建同一研究Setup，严格核对原Ark/EIP VK，串行新生成全部8份状态证明。
不复用B或历史状态证明。每批证明在独立Rust子进程运行，读取一次该运行的PK；OS getrusage记录工作进程高水位RSS。
RSS包括该批Witness诊断、PK解码、Prove、Verify和导出，不谎称是只覆盖Prove区间的精确峰值。
Witness指标为独立R1CS合成及满足性检查，高层Prove会内部再合成，不能把两者作为互不重叠子项相加。
Setup计时独立，逐批Prove/Verify/VK准备时间独立，证明长度由实际序列化字节动态读取。
C随后创建8规模×3路线的24个独立干净VM/子进程，仅成功路径入正式统计；负例不混入。

再次获准D后：
```sh
node scripts/stage1_multibatch_audit.mjs --gate D --approved D --run results/stage1_multibatch/NEW_C_ID --raw-run experiments/stage1_multibatch_raw/NEW_C_ID
```
输出transactions.csv、cumulative_costs.csv（两种累计口径）、per_vote.csv、offchain.json、summary.json、report.md及SHA256SUMS。
Gate D 明确将 rawRun 作为只读输入，执行前后递归完整快照必须相等，仅向 results 下的 run 写审计结果。
所有 Gas 汇总与差额使用 BigInt；JSON 为十进制字符串。per_vote.csv 使用精确分数，避免把浮点舍入伪装成实测。
各实测量标MEASURED，汇总/差额标DERIVED，未测为NOT_MEASURED/null，失败为BLOCKED并保存日志。
只比较K1..K8全部整数实测点，首次正节省仅是当前范围首次观察；不假定差额单调、不向K>8外推。
部署收据已含子CREATE，调用轨迹不再加到总Gas。认证/接收/冻结同范围统计；拒绝交易独立。
K2检查旧Stage2输入/上下文/CT不变，Gas按部署/认证/接收/冻结分项解释；新冻结记录预期与旧摘要不同。

## 历史门槛A-01/02验收记录（保留）

2026-11-17（Asia/Shanghai），基于G18建立`experiment/stage1-multibatch-gate-a`。以下为当时未提交、未推送的原始运行记录；
该批文件后来已包含于当前固定 HEAD，不将历史状态当作 A-03 的执行结果。
正式验收目录为`experiments/stage1_multibatch_raw/gate-a-20261117-02/`，派生审计为
`results/stage1_multibatch/gate-a-20261117-02/gate_a_audit.json`。

实际执行：`node contracts/scripts/stage1_multibatch_compare.mjs --gate A --run experiments/stage1_multibatch_raw/gate-a-20261117-02`。
Rust `cargo +1.99.1 build --release --locked --offline -p vv-friendly-dkg --bin stage1_multibatch_materials`退出0，
编译命令墙钟44.964秒，opt-level=1。CARGO_TARGET_DIR在本轮raw目录内，不改旧target构建缓存或锁文件。
`rustfmt +1.99.1 --edition 2021 --check`及两个JS文件`node --check`均退出0。
solc `0.8.30+commit.73712a01.Emscripten.clang`编译通过，墙钟12.266秒（MEASURED），optimizer200/viaIR/Prague。

| 合约 | 创建字节码B（MEASURED） | 运行字节码B（MEASURED） |
|---|---:|---:|
| Stage1LegacySettlement |15149|9798|
| Stage1DirectFoldSettlement |17266|10621|
| Stage1StateProofSettlement |17959|8634|

仅核对编译字节码大小，不代表实际部署通过，也不提供Gas估算。
编译日志在raw目录的`logs/`；编译器设置、全部ABI/字节码和源文件哈希在`compiled.json`；
命令、退出码、二进制摘要及门控结果在`gate_a.json`。
首次`gate-a-20261117-01`保留了Rust成功、Solidity声明语法错误的失败记录；仅修正本轮新增合约后使用全新02目录复测，未覆盖01。

门槛A产物审计PASS；2947个原有文件（包括先前未提交的G19材料）及外部论文正文哈希不变。
本轮Setup数、状态/选票Prove数、新EVM交易数均为0（实际执行计数）。
功能测试、负例、恢复、RSS/证明时间/Gas/正式24VM结果均为NOT_MEASURED/null，不以编译成功代替功能通过。
只读核对命令：
```sh
# 历史 A-02 审计命令，仅作来源记录；A-03脚本锁定新归档，不重新运行或改写旧记录：
# node scripts/stage1_multibatch_audit.mjs --gate A --run experiments/stage1_multibatch_raw/gate-a-20261117-02 --check
```

未解决事项：B/C的实际功能、资源及负例尚未执行；确定性研究Setup不是生产可信设置；
完整SnarkPack EVM及P3/Winner集成不在本阶段范围。当前停在A，下一步必须由人工单独确认B。

## 历史A-03：工作区与历史档案保护

`validateConfig({phase,run,rawRun})` 分别检查 staged、unstaged 和未跟踪文件。
未跟踪文件使用 `git ls-files --others --exclude-standard`，避免将既有 node_modules 当作新增研究文件；
被忽略的历史构建缓存仍由下述完整递归快照保护，不能以 Git ignore 为由排除。
A仅允许3个批准补丁和A-03两个新目录；B/C仅允许批准补丁及当前运行的raw/results；
D仅允许预先存在的批准补丁、只读C原始目录和当前审计输出，执行前后还核对3个补丁内容不变。

递归快照记录完整相对路径、文件类型、数量及内容SHA-256，包括目录、符号链接及ignored build-target：
`experiments/stage1_multibatch_raw/gate-a-20261117-01`、`.../gate-a-20261117-02`、
`results/stage1_multibatch/gate-a-20261117-02`。
显式记录 `results/stage1_multibatch/gate-a-20261117-01` 不存在，前后必须仍不存在。
不再整体排除Stage1历史目录；A-03仅排除当前03目录。

预检只是 READY_FOR_HUMAN_APPROVAL，不是 B 通过。A-03运行时归档在工作区未跟踪；当时按指令保持
B/C“仅当前run”的严格白名单，不自动放宽。实际启动B前需人工明确如何保留/纳入A-03准入材料，
否则应拒绝执行。不会擅自提交、删除归档或借此启动Setup/Prove。

## 存储打包、多区块与Prague计费

solc输出完整 `storageLayout`，逐一检查基类及三条继承路线：`nextBatchIndex`为8字节、offset0；
`spentCount`为8字节、offset8；`positionInBatch`为1字节、offset16；`frozen`为1字节、offset17。
四项共18字节共享slot S，`frozenTarget`在slot S+1、offset0、32字节。
第四票后 `(1,3,3,0) -> (2,4,0,0)` 使该槽保持非零，不能归因为整槽清零退款；
这不等于整个交易退款恒零，仍完整执行通用退款计量。

未来每个VM保持状态与部署者nonce连续，每笔交易独立分配从1开始递增的区块，交易索引0、
区块交易前Gas为0，区块上限30M、交易上限20M，timestamp=1750000000+blockNumber。
不会在同一个区块塞入全部交易，也不会把不同K接续在同一个VM。
Common静态检查Mainnet/Prague激活EIP-2537、3529、7623，不激活8037；
加载与VM构造函数相同的官方 `paramsVM` 参数表后校验 maxRefundQuotient=5，不为此创建VM。

14项十进制字符串字段为：`block_number`、`transaction_index_in_block`、`block_gas_used_before`、
`gas_pre_refund`、`refund_counter_raw`、`refund_cap`、`refund_applied_before_floor`、
`gas_after_refund_before_floor`、`minimum_gas_limit`、`eip7623_floor_gas`、`floor_adjustment`、
`refund_effective_in_receipt`、`gas_used`、`receipt_cumulative_gas`。
全部Gas计算保持BigInt：pre=intrinsic+execution；原始退款来自 `execResult.gasRefund`（当前Legacy交易路径）；
cap=pre/5；applied=min(raw,cap)；after=pre-applied；floor由 `getCalldataFloorGas(tx,sender)` 独立取得，
不混同 `getMinimumGasLimit()`；adjustment=max(0,floor-after)；实际Gas以 `totalGasSpent` 为权威值。
审计验证 used=max(after,floor)、effective=max(0,applied-adjustment)、
receipt.cumulative=blockBefore+used及区块上限，不再假设退款为零。
这些交易级运行时断言本轮未执行，待B/C验收。

## Final Context、VM实例与C-VOTE来源边界

Final Context读取固定 `fixtures/jubjub_g6_native_winner_only_public_v1.json` 的
`public_context.final_context_hex`。37项状态电路输入绑定该上下文摘要，未单独传入Chain ID或Receiver地址。
未来运行输出记录route、chain_id、deployer_address、初始/最终nonce、Gate/Receiver部署nonce与地址、
两份实际initcode SHA256、VM实际Receiver runtime code SHA256和final_context。
跨VM审计覆盖B的9实例/C的24正向实例：固定链/部署者/初始nonce产生相同确定性合约地址，
但三路线字节码不同；不同K的构造参数/immutable不同，不能错误要求其initcode/runtime哈希全部相同。
这些实例复用同源证明材料是公平比较手段，不是跨链或跨合约重放安全测试；
不得等同于形式化协议中包含完整实例域的上下文派生。

原有32份C-VOTE证明来自 `fixtures/snarkpack_g1d2_depth5_batch32_public_v1.json`：
16份复用G1-D1池、16份G1-D2新生成，`private_witness_emitted=false`。
Stage1仅验证、复用这些原证明；后续B/C仅新生成状态转移证明，不重新测量32份选票证明生成耗时。
C-VOTE的私有见证约束权重范围和Pedersen承诺，但当前固定均匀权重：32名选民、每人1、
总登记权重32、单人上界32；不声称覆盖任意非均匀权重分布。

## 历史A-03验收记录

A-03执行范围：仅编译、静态核查与独立B预检；当时新档案均位于 `gate-a-20261117-03`。
实际编译、11项核查及命令退出码以新 `gate_a.json`、`static_checks.json`、`storage_layout.json`、
`preflight_b.json`、`results/.../gate_a_audit.json` 与 `report.md` 为准。
Setup=0、Prove=0、VM创建=0、EVM交易=0；不得把静态接线检查标为运行时通过。
不提交、不推送；输出后停止，等待B的单独确认及工作区交接。

2026-11-17实际编译：Rust `+1.99.1` 的锁定/离线Release构建退出0，命令墙钟48.226秒，
`opt-level=1`；solc `0.8.30+commit.73712a01.Emscripten.clang` 墙钟12.431秒。
rustfmt与两个JS语法检查均退出0。三路线运行字节码分别9798、10621、8634字节；仅编译，未部署。
完整布局实测 S=22，四个字段offset为0/8/16/17，`frozenTarget`在slot23。
递归保护覆盖A-01的1628文件、A-02的1629文件及A-02结果的2文件，另有2987项原有Git可见文件与论文哈希；
不存在的A-01结果目录仍不存在。重复记录文件不应相加后声称为去重总数。
`preflight_b.json`确认6正向/3负向任务配置及两个B目标目录不存在；状态为READY_FOR_HUMAN_APPROVAL。
本轮无运行时Gas、功能正负例或性能优势数据；全部交易级检查仍待B/C。

## A-04：动态运行路径与只读交接

`resultsPathForRun(rawRun)`从传入路径派生results；`verifyA`、`gateA`、`preflightGateB`不再把A-03当作当前目录。
`validateConfig({phase,run,aRun,bRun,rawRun})`和全部调用点统一更新，`phaseCallSites()`检查实际函数体，
包括Gate B/C分支、VM worker从request读取的A/B来源，以及audit main的源文件哈希冻结。

白名单分开检查staged/unstaged/untracked：A只允许3个补丁及A-04新目录；
B允许当前运行raw/results及只读A输入双目录；C再加入只读B输入双目录。
A/B只读输入执行前后递归校验，包括ignored build-target；C不能修改B回归记录或复用B状态证明。
三代历史A-01/02/03目录与缓存均固定纳入完整快照，A-01 results仍须不存在。
D在启动时冻结3个批准补丁的SHA256，结束必须相同；C原始输入只读、D审计结果独立写入。

B除独立人工 `--approved B` 外，还必须检验预检两份报告字节一致、状态、a_run、b_run、
config/compiled/binary SHA256与完整source_pins，后者同时与A记录和当前源文件逐项一致。
预检中的B目录名必须与实际目标匹配；执行和审计B均使用同一目标。
C除 `--approved C` 外，必须同时取得B执行报告和独立B审计报告：6个正向VM、3个负向VM会话、
9个独立实例和gas_receipt_audit=PASS。3个负向会话分别覆盖多项攻击，不等于仅3个攻击用例。
两份报告config/source_pins必须一致，不能只凭B执行状态放行C。

Setup实际RSS字段为 `m.setup.worker_peak_rss_mib`，逐批为 `b.peak_rss_mib`；
都必须大于0且不超过既有 `cfg.budgets.child_rss_mib=16384`，不修改配置或使用错误字段。
本轮只审查这些运行时接线，未执行B/C，更没有测得本轮证明RSS。

## A-04：本机二进制与编译准入

使用当前运行自己的 `CARGO_TARGET_DIR=<aRun>/build-target`，检查真实文件
`<aRun>/build-target/release/stage1_multibatch_materials`存在，实际SHA256同时匹配本轮gate_a.json
及A-03固定基准 `da5c0f46378af23ac61905e5f4f96d42d174dfc45e3711eec53863901fc1a488`。
比对结果独立保存为 `binary_reproducibility.json`。若不一致立即停止，不改历史记录、不继续预检或证明。
仅新建A-04 raw/results归档；原始命令与编译器产物、历史快照、11项审计及独立预检都在对应目录。
预检状态只允许READY_FOR_HUMAN_APPROVAL，不代表Gate B完成。

本轮必须保持 Setup=0、Prove=0、VM创建=0、交易=0，不提交、不推送；A-04输出后停止等待确认。

2026-11-17实际执行：`cargo +1.99.1 build --release --locked --offline -p vv-friendly-dkg --bin stage1_multibatch_materials`
退出0，命令墙钟49.065秒，Release opt-level=1；Solidity 0.8.30编译墙钟12.228秒。
rustfmt与两个JS语法检查均退出0。A-04本机二进制存在，SHA256与上述A-03完整基准逐字相同；未调整历史基准。
完整storageLayout继续为slot22、offset0/8/16/17，frozenTarget为slot23。
历史递归文件数分别为A-01 raw 1628、A-02 raw 1629/results 2、A-03 raw 1638/results 4；
A-01 results仍不存在，另核对3018项原有Git可见文件与论文哈希。这些覆盖存在重叠，不相加冒充去重总数。
独立B预检与11项审计结果见A-04对应JSON、report.md和SHA256SUMS；状态与未执行项目以原始报告为准。

## 最终报告六项必答问题

1. 同K三路线的最终密文、完整NF集合、认证和冻结结果是否一致？
2. Legacy五步恢复和原子性差异是否被真实验收？
3. 仅部署＋接收在哪个实测K首次观察到净节省（或未观察到）？
4. 部署＋认证＋接收＋冻结在哪个实测K首次观察到净节省？
5. 本次同源Setup、逐批Witness/Prove/Verify、RSS和通信成本是多少，如何划定口径？
6. 哪些结论只属于回退原型，哪些完整SnarkPack/生产部署指标仍然未测？

## A-05 当前执行规范（上方A-01至A-04全部保留为历史）

本节为当前有效准入说明；上方A-04命令与“当前门槛”描述均属于当轮历史，不应作为本轮运行入口。
固定四代提交链：`72ccc561d8934b0d761e17764ef9012bfe012626`
→ `2444269b52573d9c2f3e551ef7d6b8bb9d0bd008`（59A）
→ `f086879c4dc247e528d62d2253d76d6afd603669`（3M+31A）
→ `3f8813c9d69d0e45631e9acb7bbb247a1119f511`（3M+32A）。
对应grandparent/parent/HEAD的Tree分别固定为
`92f7331a7d9b534d17108409342851381811aa52`、`3b8b57b375bb0391afe431935057a2711c5608e9`、
`a21e84e05a2c653218a50ec99769b81766d12b91`，三段静态清单与版本校验仅在runner定义，auditor复用共享函数。

仅修改runner、auditor和本文档；仅新建 `gate-a-20261117-05` 的raw/results归档。
完整保护A-01至A-04旧归档及ignored构建缓存，A-01 results仍须不存在；不改Solidity、Rust、电路、VK、
研究Setup、fixtures、Cargo.lock、历史数据或论文。当前仅允许编译/静态核查/独立预检，
必须保持Setup=0、Prove=0、VM=0、交易=0。

### 全局不提交与源文件锁定

Gate A-05、B、C、D全部结束前，严禁 `git commit` 或 `git push`，不能因归档上传再触发版本锁定循环。
3个批准文件保留为未提交补丁，其哈希由 `currentSourcePins()` 对 `sourceFiles ∪ approvedPatches` 去重后持久化；
本文件也必须纳入 `gate_a.json`、`preflight_b.json` 及后续门控的source_pins。
因此本文档必须在A-05编译前定稿，此后不再追加实测时间或编辑任何补丁；真实执行结果保存在新运行JSON与日志。
Gate B/C开始前与结束后都必须与A-05的完整source_pins一致，不能修改文档后悄然更新基准。
不同于补丁，独立运行目录允许按各阶段规则生成，但A/B只读输入与所有历史档案仍须递归前后相同。

### 完整合法选票替换与现有负例的区别

保留 `replacement_ballot_proof`：仅用其他证明替换V1的Groth16证明，保留V1输入，预期INVALID_GROTH16。
新增 `complete_valid_ballot_replacement`：用第二批的合法V5（`m.ballots[4]`）同时替换第一批V1的公开输入和证明，
保持原位置0及原BatchRoot/成员路径；Legacy预期BATCH_MEMBERSHIP，Direct Fold与State Proof预期BATCH_ROOT。
不能使用V2替换V1：批内已经含V2，合约先执行重复Nullifier检查，会触发DUPLICATE_NULLIFIER而无法独立测根绑定。
V5与第一批其他票的Nullifier不同，材料生成阶段仍需验证全部原始选票证明，不能用非法选票冒充“完整合法替换”。
三条路线均通过统一reject帮助函数记录完整before/after状态、revert原因、收据与零事件回滚；独立auditor再次强制核对。
本轮只接入与静态检查该用例，不声称已经在EVM执行并拒绝，运行时结果待Gate B。

`legacy_incomplete_freeze`测试的是Alice/Bob已成功、批次未满额时提前冻结被拒绝，不能冒充批次内漏计证明。
Direct Fold/State Proof的Omission-A则在接受第一批之前修改newState以少计批内选票，测试直接折叠或状态证明的拒绝。
Legacy仍保留Alice/Bob成功→Charlie首次失败回滚→Charlie重试→David成功→批次完成的五步恢复检查。

### 本轮严格执行顺序

```sh
node contracts/scripts/stage1_multibatch_compare.mjs --gate A --run experiments/stage1_multibatch_raw/gate-a-20261117-05
node contracts/scripts/stage1_multibatch_compare.mjs --preflight B --a-run experiments/stage1_multibatch_raw/gate-a-20261117-05 --b-run gate-b-20261117-01
node scripts/stage1_multibatch_audit.mjs --gate A --run experiments/stage1_multibatch_raw/gate-a-20261117-05
```

三条命令不可颠倒；A审计依赖预检文件，所有写入使用全新05目录，不覆盖01至04。
Rust固定 `+1.99.1`、Release opt-level=1、独立CARGO_TARGET_DIR；Solidity 0.8.30配置不变。
新二进制必须真实存在，SHA256与A-03基准
`da5c0f46378af23ac61905e5f4f96d42d174dfc45e3711eec53863901fc1a488`一致；不一致立即停止。
只有PASS_GATE_A_COMPILE_ONLY、PASS_GATE_A_ARTIFACT_AUDIT和READY_FOR_HUMAN_APPROVAL全部满足后，
才输出摘要并停止。READY不是Gate B通过，也不构成本轮运行Gate B的授权；B命令待人工另行下发。

Gate B仅做完整功能与证据回归：材料、6正向/3负向独立会话、状态/NF连续性、回滚、收据计费及只读保护；
不输出性能优劣结论。K=2与历史Stage2（仅Direct Fold/State Proof）的部署、认证、接收、冻结分项成本差异，
统一保留在Gate D的 `deriveD()` 执行，不将其前移到B。完成B后仍需停止并等待C的独立批准。

## A-06 当前执行规范（A-01至A-05原文全部保留为历史归档规范）

本节为当前有效规范；A-05仅为历史归档规范，不再是新执行的准入入口。
本地HEAD保持 `3f8813c9d69d0e45631e9acb7bbb247a1119f511`，禁止commit、push、reset或clean。
只修改runner、auditor和本文档，只新建 `gate-a-20261117-06` raw/results目录；
不改Solidity、Rust、电路、VK、研究Setup、配置、Cargo.lock、fixtures、历史结果或论文。

### B-01失败归档与外层调度预算

`gate-b-20261117-01`已因外层materials总预算600秒触发WALL_BUDGET/SIGTERM，
当时Setup worker墙钟实测38.782秒、峰值RSS 4465.719 MiB，随后仍在第1批证明阶段。
该Setup worker墙钟与仅参数生成的35.465秒不同；不能把半成品视为B通过或正式性能样本。
失败原始日志、Setup与半成品全部只读保留，不复用、不删除、不改写。

`command()`新增独立wallBudgetSeconds参数，默认仍为cfg.budgets.child_seconds（600秒）；
仅nextGate的materials调用使用 `(1 + proofCount) * cfg.budgets.child_seconds + 120`：
Gate B的proofCount=2，对应1920秒；Gate C的proofCount=8，对应5520秒。
120秒涵盖材料预处理与进程调度的运行缓冲，并非数学绝对上限，也不保证证明一定完成。
所有命令日志新增wall_budget_seconds，独立审计核对materials预算、成功退出、无超时原因、
正墙钟耗时及未超过预算，并核对Setup加逐批worker数量为1+proofCount。

Rust内层单个Worker仍使用名义600秒Worker预算与16384 MiB RSS上限，未修改Rust或配置。
超时由后台每250ms轮询 `t.elapsed().as_secs() > 600` 触发，实际终止时刻可能略超600.000秒。
Setup与每批Prove的独立进程RSS仍须在(0,16384] MiB内；本轮不生成这些新的运行时指标。

### 12项历史快照与新源文件哈希

runner递归保护A-01至A-05以及失败B-01的raw/results共12项路径，包括ignored构建缓存。
其中A-01与B-01的results须不存在，其余10项须存在；这些历史目录虽获工作区白名单准入，
仍受完整目录清单与内容SHA-256前后核验，绝不授权修改历史文件。
auditor独立硬编码同一12项清单并核对存在性，不仅依赖runner提供的清单。
原有三代静态变更清单、四代提交链、固定Tree和未授权路径拒绝规则保持不变。

A-06编译前定稿全部3个批准补丁，并在currentSourcePins()中与配置、Rust及Solidity一并持久锁定。
本文档不在编译后追加实测耗时；真实命令、计时、哈希、预检和审计结果只写入新A-06归档。
本机新二进制必须位于A-06自己的build-target，且SHA-256与A-03不可变基准及gate_a.json一致：
`da5c0f46378af23ac61905e5f4f96d42d174dfc45e3711eec53863901fc1a488`。

### 本轮只编译、预检与审计

```sh
node contracts/scripts/stage1_multibatch_compare.mjs --gate A --run experiments/stage1_multibatch_raw/gate-a-20261117-06
node contracts/scripts/stage1_multibatch_compare.mjs --preflight B --a-run experiments/stage1_multibatch_raw/gate-a-20261117-06 --b-run gate-b-20261117-02
node scripts/stage1_multibatch_audit.mjs --gate A --run experiments/stage1_multibatch_raw/gate-a-20261117-06
```

严格按上述顺序逐条执行，前一步成功才进入下一步；不创建B-02 raw/results目录。
最终须分别得到PASS_GATE_A_COMPILE_ONLY、READY_FOR_HUMAN_APPROVAL和PASS_GATE_A_ARTIFACT_AUDIT。
本轮Setup=0、Prove=0、VM=0、交易=0；静态接线通过不等于运行时预算或密码学测试已通过。
输出执行摘要与3个批准文件的完整git diff后立即停止，待人工核验后另行放行B-02，禁止自动进入B/C。
