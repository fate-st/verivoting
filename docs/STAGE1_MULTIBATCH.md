# Stage1：多批次 EVM 部署摊销与消融（隔离实现）

当前门槛：A-03，最小脚本补丁、编译、静态审计及独立 B 预检。B/C/D 未获本轮运行授权，禁止自动进入。
本文件不修改论文或任何 G8-E、G12、Stage2、G18 原始材料。

## 固定范围

- 基准 G18 `72ccc561d8934b0d761e17764ef9012bfe012626`；当前 HEAD `2444269b52573d9c2f3e551ef7d6b8bb9d0bd008`，Tree `92f7331a7d9b534d17108409342851381811aa52`。
  当前 HEAD 的父提交必须为 G18，固定核对全部59个 Added 文件（包括G19来源审查、Stage1、A-01/02归档）。
  A-03仅修改runner、auditor及本文档；不修改配置、Rust、Solidity、电路、fixtures、旧结果或论文。
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

## 严格门控命令（B/C/D均未执行）

门槛A，仅编译：
```sh
node contracts/scripts/stage1_multibatch_compare.mjs --gate A --run experiments/stage1_multibatch_raw/gate-a-20261117-03
node contracts/scripts/stage1_multibatch_compare.mjs --preflight B --a-run experiments/stage1_multibatch_raw/gate-a-20261117-03 --b-run gate-b-20261117-01
node scripts/stage1_multibatch_audit.mjs --gate A --run experiments/stage1_multibatch_raw/gate-a-20261117-03
```
检查已有归档时给审计脚本追加`--check`，不覆盖记录、不重新编译。

获得独立人工确认后才可执行B（以下仅列接口，不表示已经运行）：
```sh
node contracts/scripts/stage1_multibatch_compare.mjs --gate B --approved B --run experiments/stage1_multibatch_raw/NEW_B_ID --a-run experiments/stage1_multibatch_raw/gate-a-20261117-03
node scripts/stage1_multibatch_audit.mjs --gate B --run experiments/stage1_multibatch_raw/NEW_B_ID
```
B先新建材料目录、串行生成两份新状态证明，再运行K1/K2三路线回归及三个独立负例VM。
Omission-A在首次接收之前测试，State Proof必须真正触发第五次配对返回0。
Legacy恢复顺序为：Alice/Bob成功→Charlie失败原子回滚→Charlie重试成功→David成功→同笔David交易触发批次完成。
所有失败检查完整Receiver可变元组、32项NF、Gate状态/签署者/认证权重和证书摘要；不要求费用或发送者nonce回滚。

再次获得独立确认后才可执行C：
```sh
node contracts/scripts/stage1_multibatch_compare.mjs --gate C --approved C --run experiments/stage1_multibatch_raw/NEW_C_ID --a-run experiments/stage1_multibatch_raw/gate-a-20261117-03 --b-run experiments/stage1_multibatch_raw/NEW_B_ID
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

## A-03：工作区与历史档案保护

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

预检只是 READY_FOR_HUMAN_APPROVAL，不是 B 通过。当前 A-03 归档在工作区未跟踪；按指令保持
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

## A-03验收记录

当前执行范围：仅编译、静态核查与独立B预检；新档案均位于 `gate-a-20261117-03`。
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

## 最终报告六项必答问题

1. 同K三路线的最终密文、完整NF集合、认证和冻结结果是否一致？
2. Legacy五步恢复和原子性差异是否被真实验收？
3. 仅部署＋接收在哪个实测K首次观察到净节省（或未观察到）？
4. 部署＋认证＋接收＋冻结在哪个实测K首次观察到净节省？
5. 本次同源Setup、逐批Witness/Prove/Verify、RSS和通信成本是多少，如何划定口径？
6. 哪些结论只属于回退原型，哪些完整SnarkPack/生产部署指标仍然未测？
