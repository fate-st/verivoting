# Stage1：多批次 EVM 部署摊销与消融（隔离实现）

当前门槛：A，实现与编译。B/C/D 未获本轮运行授权，禁止自动进入。
本文件不修改论文或任何 G8-E、G12、Stage2、G18 原始材料。

## 固定范围

- 基准 G18 `72ccc561d8934b0d761e17764ef9012bfe012626`；仅新增本轮六个文件及独立运行目录。
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
node contracts/scripts/stage1_multibatch_compare.mjs --gate A --run experiments/stage1_multibatch_raw/NEW_A_ID
node scripts/stage1_multibatch_audit.mjs --gate A --run experiments/stage1_multibatch_raw/NEW_A_ID
```
检查已有归档时给审计脚本追加`--check`，不覆盖记录、不重新编译。

获得独立人工确认后才可执行B（以下仅列接口，不表示已经运行）：
```sh
node contracts/scripts/stage1_multibatch_compare.mjs --gate B --approved B --run experiments/stage1_multibatch_raw/NEW_B_ID --a-run experiments/stage1_multibatch_raw/gate-a-20261117-02
node scripts/stage1_multibatch_audit.mjs --gate B --run experiments/stage1_multibatch_raw/NEW_B_ID
```
B先新建材料目录、串行生成两份新状态证明，再运行K1/K2三路线回归及三个独立负例VM。
Omission-A在首次接收之前测试，State Proof必须真正触发第五次配对返回0。
Legacy恢复顺序为：Alice/Bob成功→Charlie失败原子回滚→Charlie重试成功→David成功→同笔David交易触发批次完成。
所有失败检查完整Receiver可变元组、32项NF、Gate状态/签署者/认证权重和证书摘要；不要求费用或发送者nonce回滚。

再次获得独立确认后才可执行C：
```sh
node contracts/scripts/stage1_multibatch_compare.mjs --gate C --approved C --run experiments/stage1_multibatch_raw/NEW_C_ID --a-run experiments/stage1_multibatch_raw/gate-a-20261117-02 --b-run experiments/stage1_multibatch_raw/NEW_B_ID
```
C从全新材料目录开始，重建同一研究Setup，严格核对原Ark/EIP VK，串行新生成全部8份状态证明。
不复用B或历史状态证明。每批证明在独立Rust子进程运行，读取一次该运行的PK；OS getrusage记录工作进程高水位RSS。
RSS包括该批Witness诊断、PK解码、Prove、Verify和导出，不谎称是只覆盖Prove区间的精确峰值。
Witness指标为独立R1CS合成及满足性检查，高层Prove会内部再合成，不能把两者作为互不重叠子项相加。
Setup计时独立，逐批Prove/Verify/VK准备时间独立，证明长度由实际序列化字节动态读取。
C随后创建8规模×3路线的24个独立干净VM/子进程，仅成功路径入正式统计；负例不混入。

再次获准D后：
```sh
node scripts/stage1_multibatch_audit.mjs --gate D --approved D --run experiments/stage1_multibatch_raw/NEW_C_ID
```
输出transactions.csv、cumulative_costs.csv（两种累计口径）、per_vote.csv、offchain.json、summary.json、report.md及SHA256SUMS。
各实测量标MEASURED，汇总/差额标DERIVED，未测为NOT_MEASURED/null，失败为BLOCKED并保存日志。
只比较K1..K8全部整数实测点，首次正节省仅是当前范围首次观察；不假定差额单调、不向K>8外推。
部署收据已含子CREATE，调用轨迹不再加到总Gas。认证/接收/冻结同范围统计；拒绝交易独立。
K2检查旧Stage2输入/上下文/CT不变，Gas按部署/认证/接收/冻结分项解释；新冻结记录预期与旧摘要不同。

## 门槛A验收记录

2026-11-17（Asia/Shanghai），基于G18建立`experiment/stage1-multibatch-gate-a`，未提交、未推送。
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
node scripts/stage1_multibatch_audit.mjs --gate A --run experiments/stage1_multibatch_raw/gate-a-20261117-02 --check
```

未解决事项：B/C的实际功能、资源及负例尚未执行；确定性研究Setup不是生产可信设置；
完整SnarkPack EVM及P3/Winner集成不在本阶段范围。当前停在A，下一步必须由人工单独确认B。

## 最终报告六项必答问题

1. 同K三路线的最终密文、完整NF集合、认证和冻结结果是否一致？
2. Legacy五步恢复和原子性差异是否被真实验收？
3. 仅部署＋接收在哪个实测K首次观察到净节省（或未观察到）？
4. 部署＋认证＋接收＋冻结在哪个实测K首次观察到净节省？
5. 本次同源Setup、逐批Witness/Prove/Verify、RSS和通信成本是多少，如何划定口径？
6. 哪些结论只属于回退原型，哪些完整SnarkPack/生产部署指标仍然未测？
