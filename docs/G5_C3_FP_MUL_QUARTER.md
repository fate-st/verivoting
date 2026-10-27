# G5-C3-B20：隔离 Fp 乘法的除四合并实测

2026-10-27。本轮在独立 B6-based profile 合约中实现 B19 审查的固定模数
`quarterMod`，只执行一次部署、一个原公开性能输入和四个校正/进位正确性输入。
五个完整规范乘积全部正确，同一公开输入的算术窗口从 2,167 降至 2,023 Gas。

状态：`PASS_FP_QUARTER_FIXED_CASES_AND_WINDOW_REDUCTION_ONLY`。本次收益仅为
隔离 Fp 调用的真实测量；候选尚未集成 Miller，完整验证器仍为 BLOCKED。

## 唯一实现变化

新增 `contracts/src/Bls12381FpMulQuarterProfile.sol`。整份源码与 B6 profile
对照，只有合约名、`_mul` 内部 `halfMod` 改为 `quarterMod`、两次末尾调用
改为一次；其他 16 个 Solidity 函数及其余源码字节保持不变。

乘法仍由两次真实 MODEXP 平方计算规范差值 t，再按固定 p 计算
`k=t&3; quarter(t)=(t+k*p)/4`。实现分别累计 kp 低乘积产生的进位和加 t
产生的进位，两者可能同时为 1，使用加法而不是 OR。模数条件及正确性推导见
[B19 源内路线复核](G5_C3_MILLER_ROUTE_REVIEW.md)。

曲线、规范 64B 大端编码、两次平方、输入校验、MODEXP 失败检查和内存布局均未
改变。合约内执行全部算术；宿主参考值只生成公开测试输入并核对返回结果。

## 同一公开输入的真实对照

性能输入来自原公开 B=2 证明 `final_b` 的仿射坐标 `x.c0`、`y.c0`，与 B6
完全相同；calldata SHA-256 和完整输出也保持一致。只有这一个性能样本。

| 项目 | B6 原实现 | B20 候选 |
|---|---:|---:|
| 算术窗口 Gas | 2,167 | 2,023 |
| execution Gas | 4,318 | 4,174 |
| intrinsic Gas | 23,288 | 23,288 |
| 交易总 Gas | 27,606 | 27,462 |
| 窗口外 execution Gas | 2,151 | 2,151 |
| 窗口分配指针增长 | 384B | 384B |
| 窗口内存扩张 Gas | 37 | 37 |
| 窗口活动内存 | 16 -> 28 words | 16 -> 28 words |
| 退出活动内存 | 39 words | 39 words |
| MODEXP 次数 / child Gas | 2 / 400 | 2 / 400 |

窗口实际减少 144 Gas，即 6.6451%；execution 和交易总费用也各减少 144 Gas。
本次没有触发 Prague calldata floor，退款为零。这里可以报告该固定输入的总
费用下降，但不能据此断言任意输入都有相同节省，或将该窗口视为内联后的常数。

费用对账：root opcode 3,774 + MODEXP child 400 = execution 4,174；加
intrinsic 23,288 = 总计 27,462。首尾 GAS 标记差为 2,023，与合约返回窗口
一致。窗口分配指针 512 -> 896B；它不等同 EVM 活动内存或主机 RSS。

候选 runtime 为 2,491B，较 B6 的 2,519B 减少 28B；部署总 Gas 为 591,914，
较 597,983 减少 6,069。部署 execution 为 `528 + 2,491*200 = 498,728`，
其中包含代码存入费用；再加 intrinsic 93,186 即部署总费用。不能把部署期间
的 root opcode 528 单独称为全部部署 execution。

## 四个必要正确性用例：4/4 真实通过

令 `B=2^256`。四个调用沿用 B19 推导的 `a=1, b=quarter(t)`；对应真实
乘法内部规范差值为 `4ab mod p=t`，完整期望乘积为 b。

| 指定内部 t | k | kp 乘积进位 | 相加进位 | 实际完整输出 |
|---|---:|---:|---:|---|
| 0 | 0 | 0 | 0 | 正确 |
| p-2 | 1 | 0 | 0 | 正确 |
| p-1 | 2 | 0 | 1 | 正确 |
| B-1 | 3 | 1 | 1 | 正确 |

第四例同时需要两项进位，检验此次新实现最直接的易错点。每笔调用均成功、
返回完整规范 64B 乘积、执行两次成功 MODEXP，并保持 384B 窗口分配和零退款。
根层/child execution、交易总费用、首尾窗口标记均分别对账通过。

四笔正确性调用的窗口分别为 1,955、2,091、2,091、2,023 Gas；交易总费用
分别为 26,254、26,954、26,942、26,874 Gas。这些只记录用例执行，不纳入
固定输入性能对照或统计样本。carry 条件由输入代数推导，不宣称内部指令覆盖
跟踪或一般化形式验证。

## 当前边界与下一门 B21

本轮共六笔本地 EVM 交易，即一次部署和五次 Fp 调用；没有执行 Fp2。合约保留
原 Fp2 ABI 和函数体，不表示本轮已测量该路径。

raw 明确记录 `optimization_integrated_into_miller=false`、`miller_run=false`、
`full_loop_attempts=0`。最新实际 full 仍是 B18 在 20M 下的 OOG，最新前缀
仍是 B15。B18 的 `halfMod` 语法归属仅占已收费 root 的 6.0557%，不能把本次
隔离窗口 6.6451% 的降幅当作 Miller 的降幅，更不能据此承诺完整循环成功。

下一门仅规划 B21：

1. 在独立 B15-based Miller 合约中精确替换本轮测得的完整 `_mul`，除合约名
   和该函数外，其余源码保持 B15 不变。
2. 只执行原两步前缀正例及一个错误目标，核对全部 768B 输出、正确拒绝、实际
   Gas、内存和 516 次成功 MODEXP。
3. 不重复本轮边界或原语 profile，不自动追加 full64、最终幂或新测试矩阵。

B21 本轮未实施。交易 / block 上限仍为 20M / 30M，没有提高预算、修改
Suite、Manifest、SRS 或论文能力主张。最终幂、GT target binding、exact
SnarkPack EVM 验证和完整结算仍未完成，完整验证 Gas 与完成率保持 null。

## 产物、复现与来源

```bash
npm run --prefix contracts g5c3b20
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

当前 raw 已存在时，运行器会拒绝覆盖；本页归档直接核对保存结果，未重跑 EVM。

- 合约：`contracts/src/Bls12381FpMulQuarterProfile.sol`。
- 运行器：`contracts/scripts/g5c3_fp_mul_quarter.mjs`。
- 实测记录：`experiments/g5_c3_raw/fp_mul_quarter_profile.json`。
- 保留基线：`experiments/g5_c3_raw/field_lowlevel_profile.json`。
- 设计来源：`experiments/g5_c3_raw/miller_route_review.json`。

环境为 Apple M4 / arm64、Node 23.11.0、solc 0.8.30、optimizer 200、无 viaIR、
本地 EthereumJS / Prague，无 RPC。raw 保留执行时 Git 基准
`e80aa2b9fb8770653d7eb23351d27fc89b0d466f` 与 `git_dirty=true`；后续提交不把
此记录改写为 clean-commit 性能样本。wall time 含观察器开销，不作统计结论。

独立只读归档核对了十二项来源/源码/锁文件/Manifest 哈希、五个完整输出、
每笔费用及 MODEXP 成功记录，没有追加 VM 或重型测试。

| 产物或来源 | SHA-256 |
|---|---|
| B20 合约 | `35d6ff2f8634286b20f777a48488cdff747722f15a9ac55ee2c8e2908a6100f9` |
| B20 运行器 | `c435f69399c7cc8801c7e10e049cf6934b3e872b9f14fbcda340577dc2f36335` |
| B20 raw | `0d44c5b76b33816cf37f85aaacd52ff7170afc09cd5e92a987b10c198b5cb52c` |
| B20 runtime | `20cff3583d699e2d453f02e12d8a0cba514a214c11abae541e8a8d9422c58e8f` |
| B6 基线 raw | `4753d634e56d59986817e61846bb6c9d6252ed60477eb225c08cc2d95b95d318` |
| B19 设计记录 | `0f95968de0efdb6f41af221e113b3545f757d48e6a9a284e2b980bc4c113061b` |
| B18 历史 full | `6645a9dbdc8c363944723edc89b09a947e5533b95d3a507076b8086c2fb9ec37` |

目标状态仍为 `BLOCKED_SOFTWARE_MILLER_SUB_LOWLEVEL_EXCEEDS_20M`，本轮 PASS
只针对这五个固定 Fp 用例及同输入窗口收益，不表示完整验证器通过。

## 后续记录：B21 集成前缀已通过

2026-10-27，B21 将本页同一完整 `_mul` 集成到独立 B15-based Miller 版本，
仅执行一次部署与原两步前缀正例、一个错误目标。正例全部 768B 正确，错误
目标以 `INVALID_MILLER_RESULT` 拒绝。正例总 Gas 为 `985,557 -> 951,873`，
减少 33,684（3.41776%）；1,905 words 活动内存和 516 次成功 MODEXP 均不变。
runtime 为 `7,106 -> 7,078B`，部署总 Gas 为 `1,589,758 -> 1,583,679`。

B21 没有运行 full64：`full_loop_attempts=0`、`full_attempt=null`。B18 的
20M OOG 来自不同旧版本，不能写成新 quarter 版本的实测失败。当前目标状态
因此为 `BLOCKED_QUARTER_MILLER_FULL_VARIANT_UNTESTED`，完整验证 Gas 仍为 null。
本页前文保留 B20 当时的隔离测量与计划背景，集成结果见
[B21 前缀报告](G5_C3_MILLER_QUARTER_PREFIX.md)。下一门 B22 只读作停止/继续
路线决策，不自动追加 full 重试、最终幂或原语优化。

B21 raw：`experiments/g5_c3_raw/miller_quarter_prefix_evm.json`，SHA-256
`afab1dbf4d32d5ed1a0066e049512fde22839ce612e02deec7f9398c6474eb01`。
