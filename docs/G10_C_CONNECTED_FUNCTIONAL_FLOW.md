# G10-C：先完成流程，不优化

状态：PASS_IMPLEMENTED_NATIVE_AND_INDIVIDUAL_GROTH16_EVM_FALLBACK_FLOW。本轮运行的是实际连接的原生流程及**已有逐票Groth16 EVM备用路线**，不是完整精确SnarkPack链上架构。

## 已实际连通的流程

认证DKG/注册重建 → 两个B4的真实SnarkPack/状态证明验证 → CT1/CT2 → 一次冻结 → 五个独立OS进程生成CP/承诺签名 → 私管道交付 → CP/打开/身份核查 → 私密3-of-5解密 → 一份新P3 → 原生Winner一次写入。
随后使用**同一份新P3、同一组新承诺、同一CT2**，在新本地Prague VM中实际部署原合约，认证证书、逐票验证两批、验证状态证明、冻结、认证承诺、从存储重建60输入，再验证新P3并写入Winner=1一次。
不是拼接历史JSON；链上交易、配对调用输入、返回值、公开输入和证明哈希都有本轮独立核对。错误Winner在披露前实际得到pairing=0；正确证明pairing=1；重放不进入pairing。

| 本轮功能验收 | 实际结果 |
| --- | --- |
| 私包进程 | 5个不同PID：83586, 83587, 83588, 83589, 83590 |
| 新私包 / 新P3 | 5 / 1；不是复用G6旧P3 |
| P3 VK | 原G4-D VK不变，0d37a77cfc8a1cdfed612b397149d96816188d164e08a74994dd2abfca6cff29 |
| 新证明 SHA-256 | 226e395d81a1ae45531ae2e012d5ca3439c0a618b956f0603617234ba7a9f833 |
| 公开输入 | 原生新证明与合约存储重建60/60相同 |
| 原生/链上Winner | 均为1，分别只写一次 |
| EVM交易 | 21笔，17成功、4项必要原子拒绝 |
| 新披露交易 | 2,073,548 Gas，644 B calldata |

原生本次PK重建 90.700 s、Prove 45.059 s、总执行 136.017 s、进程累计峰值 8.396 GiB。
这是一次功能验收，不是统计性能结论；未改电路/曲线/合约算法或优化参数，未扩B64+，未重跑G1-E。
本轮EVM成功路径总计66,172,916 Gas（含部署34,082,689），非部署业务32,090,227；失败验收费用另列原始交易，不计成功路径。
该Gas只属于本次逐票备用路线，不是精确SnarkPack费用，也不与原生Setup/Prove相加。

## Full Tally 对照的边界

使用同一CT2/DKG和同一三份有效份额，对照验证公开完整计票与仅公开Winner的材料。
Full Tally分支包含可用解密包/打开，**不能被Winner-only证书授权**，本轮明确将它定义为反事实公开解密诊断，不是给真实协议增加泄露入口。
诊断JSON留在运行目录private_diagnostic/full_tally.json（目录0700/文件0600），未复制到Git归档；归档只含验证成功、错误计票拒绝、大小及hash。Winner公共重放在独立进程完成，不加载私钥/CP打开/计票。
Winner公共fixture 18694 B、Full Tally诊断fixture 7489 B口径不同：Winner还包含缓存VK和60项派生输入，不能直接把两数排名当通信优势。
公开payload扫描未发现CP包/opening/tally/秘密份额字段。研究种子公开，因此这仍不构成运行隐私或Function-Equal定理证明。

## 仍然没有完成的目标功能

1. **精确SnarkPack EVM验证及其接收器整合。** 原生已真实验证SnarkPack，EVM仍走逐票Groth16备用验证，不能悄悄跳过此缺口。
2. **同完整流程的独立DKG/私钥保管。** 五个worker是真进程，但密钥/份额来自集中、确定性的研究bootstrap；并非五个独立Trustee完成本次DKG，不是跨机器生产部署。
3. T4/T5数学安全组合、最终统一比较与干净提交复现。功能通过不替代安全证明。

下一步只围绕上述功能闭环推进，不继续增加小型优化/成本Gate。若精确链上路线预算/语义尚不能闭合，如实保留阻塞，不能把已完成备用路线称为完整目标架构。

## 一条命令运行现有完整功能

```sh
# 新目录必须不存在。一次新Setup/Prove，随后完整本地EVM；日志逐阶段保留。
node scripts/run_functional_experiment.mjs /ABS/NEW_RUN_ROOT
# 以下只检查本次归档，不重跑证明或EVM
node scripts/g10c_chain_audit.mjs
node scripts/g10c_report.mjs --check
node scripts/g10c_check.mjs
```

本次先实际顺序执行g10c_run.mjs和g10c_chain.mjs，再用--finalize-existing核对衔接；没有伪称新总入口已重新跑过证明阶段。
归档experiments/g10_c_raw，汇总results/g10_c/functional_summary.json；绝对临时路径通过archive_map.json映射，原始结果字节不改。
本轮新增代码未修改旧协议Manifest、旧源码/图表/论文；实际源版本5da752ded5f812524f52ff8e5a51d4529d1903d4c06749237f5112f01fba8bab，运行时工作区有未提交修改，不声称最终干净源码复现。
