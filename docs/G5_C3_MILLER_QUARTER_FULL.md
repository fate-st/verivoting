# G5-C3-B23：quarter 版本 full64 在 20M 下 OOG，软件路线冻结

2026-10-27。本轮按 B22 决定，只执行一次必要部署和一笔完整 64 步正例调用，
使用与 B21 完全相同的 quarter runtime、编译配置和冻结 full calldata。
唯一完整调用在固定 20M 交易预算下发生根层 OOG；此前 11,076 次 MODEXP
全部成功，child 失败数为零。依照预先规定的停止规则，冻结这条固定预算软件
路线的自动算术优化和 full 重试。

实际结果分类为 `OOG`，状态为
`BLOCKED_QUARTER_SOFTWARE_MILLER_EXCEEDS_20M_FROZEN`。当前 quarter 版本已经
测试完整调用，不能再写成 full 未测试；它也没有通过完整 Miller 验收。

## 固定程序与实际执行

合约仍为 `contracts/src/Bls12381MillerLoopQuarter.sol`，源码没有变化。
重编译及实际部署的 runtime 均与 B21 哈希一致，大小为 7,078B。solc 0.8.30、
optimizer 200、Prague、无 viaIR，仍使用原公开 B=2 证明及冻结 Miller fixture。
full calldata SHA-256 与 B22 计划和旧 full 记录一致。

| 项目 | B23 实际结果 |
|---|---:|
| 已提交 / 已执行 EVM 交易 | 2 / 2 |
| 部署 / full64 调用数 | 1 / 1 |
| 部署总 Gas | 1,583,679 |
| full64 总 Gas / 交易上限 | 20,000,000 / 20,000,000 |
| full64 execution Gas | 19,963,388 |
| full64 intrinsic Gas | 36,612 |
| full64 退款 | 0 |
| receipt / 根异常 | 0 / out of gas |
| MODEXP 成功 / 失败 | 11,076 / 0 |
| MODEXP child Gas | 2,215,200 |
| 退出活动 EVM 内存 | 1,945 words |
| full 返回值 | 空，0x |

没有重跑前缀、原语边界、负例矩阵、setup/prove 或规模实验，也没有改变
算术、编译器、Suite、曲线、编码、Manifest、SRS 或预算。

## 为什么分类为 OOG

前置核对和唯一部署成功；full 调用取得实际执行结果。其根 receipt 为 0，
异常明确为 `out of gas`，交易花费达到 20M，已观察到的 child 没有失败。
所有 MODEXP 返回长度为 64B、每次 child 费用为 200 Gas，调用栈检查也通过。
因此满足 B22 的 OOG 条件，而非将其他 revert 或算术返回错误改记为资源不足。

费用满足：

```text
19,963,388 execution + 36,612 intrinsic = 20,000,000 总 Gas
11,076 MODEXP * 200 = 2,215,200 child Gas
退款 = 0
```

Prague 最低交易费用为 60,030，实际费用高于该值；费用方程检查通过。
部署的 `1,417,077 execution + 166,602 intrinsic = 1,583,679` 也与保存
记录一致。上述 20M 是失败交易的封顶支出，不是完成 Miller 所需的测量值。

由于返回为空，完整 payload 字节数为 null，没有获得可与冻结参考值比较的
完整结果。`full_return_matches_reference=false` 不能解释为合约已算出一份
错误的完整结果；本轮终止原因是 OOG。

raw 保存的 `root_post_execution_state` 为 program counter 6,119、
last opcode byte 87、执行后剩余 Gas 为 0。这是 VM 执行后状态，不是收费前
step 事件，也不是累计源码热点；不据此认定某条源码或函数是主导瓶颈。

## 停止规则已触发，范围仅限本软件路线

```text
result_class = OOG
new_full_variant_tested = true
latest_prefix_gate = G5-C3-B21
latest_full_attempt_gate = G5-C3-B23
software_route_frozen_by_failure = true
full_miller_loop_completed = false
full_miller_loop_verified_in_evm = false
full_miller_verification_gas = null
estimated_full_gas = null
completion_percent = null
```

当前目标状态从 B21 的“新版本 full 未测试”更新为本页的“新版本在固定预算下
已测 OOG 并冻结”。B18 仍保留为不同旧版本的历史记录；最新实际 full 由 B23
提供，最新正确前缀仍为 B21。

冻结仅覆盖这条固定 20M 软件路线的自动算术优化和 full 重试，并非停止整个
实验项目。下一步只读对齐已有证据与论文主张，并安排其他尚未完成的必要实验；
不自动增加 cap、实现 wrapper、进入最终幂或启动新原语优化。

20M 是本组组件实验选择的交易预算，30M 是本地 block 预算。结果不证明目标链
永久硬限制，也不证明兼容验证器不可能实现。成功 MODEXP 数不能换算已完成
比例或完整成本，前缀降幅也不能补足未测得的完整 Gas。

最终幂、精确 GT target binding、完整 SnarkPack EVM 验证和结算仍未完成；
相关能力标记没有升级，`snarkpack_verification_gas=null`。

## 观察器与性能口径

本轮没有 root step listener，也没有保存大 step trace；仅保留必要的 child
计数、返回值、交易费用和执行后状态。wall time 的观察开销与 B18 累计源码
profiling 不同，不能拿两次 wall time 宣称加速，亦不作为统计性能结果。
活动 EVM 内存不等于主机 RSS。本轮只有一个固定公开完整调用。

## 产物、复现与来源

```bash
npm run --prefix contracts g5c3b23
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

以上仅记录历史复现入口。原始结果已存在，本轮归档未再次运行 VM；既定失败
停止规则已触发，不执行另一次部署或完整重试。

- 运行器：`contracts/scripts/g5c3_miller_quarter_full.mjs`。
- 实测记录：`experiments/g5_c3_raw/miller_quarter_full_evm.json`。
- 前置决定：`experiments/g5_c3_raw/miller_full_decision.json`。
- 新版本前缀：`experiments/g5_c3_raw/miller_quarter_prefix_evm.json`。

环境为 Apple M4 / arm64、Node 23.11.0、本地 EthereumJS / Prague，无 RPC。
raw 保留执行时 Git 基准 `6bad1b0384ef18f525b85c86a9342447eda94b96` 与
`git_dirty=true`，不是 clean-commit 统计样本。归档只读核对了十八项来源、
源码、锁文件、Manifest 和解释器哈希、B21 runtime 一致性及费用分类。

| 产物或来源 | SHA-256 |
|---|---|
| B23 运行器 | `5b1a93cbf80625948cce891bb043db25a40877040c3cee22ef3b8235f777f3ed` |
| B23 raw | `ea725ce6aa8e1f0aebf0b8ca44f273cdec2aaffa25d579610ab21e090eea7ccf` |
| B22 决定 | `b862ff2296763dee4d68aa848f04adebdeb77dae32191389c273d5d629412b48` |
| B21 前缀 raw | `afab1dbf4d32d5ed1a0066e049512fde22839ce612e02deec7f9398c6474eb01` |
| 合约源码 | `90e573ae450b9a75c0dbe4a21242cc07113ccb041f56c12370e43a194725958e` |
| 部署 runtime | `97d85e04447a8ff8cbc46aca4d6e2eae510d04e7c96c34cdd80ffa1b1d0ec83c` |
