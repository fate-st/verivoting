# G10-B1：同一八票集合的四路线原生接收比较

状态：PASS_FOUR_MATCHED_NATIVE_INTAKE_ROUTES。本轮仅实现并测量原生接收，不是完整投票系统/Gas比较。

## 真实测量

全部使用D5/W32、同一DKG上下文、公钥/注册根、同一8张归档选票和25字段输入、两批B4、同一选票VK/状态VK/SnarkPack V2 SRS。
四路线在**同一个新二进制**中重测，不拼接G10-A历史时间。下表单位均为完成8票接收（CT0→CT1→CT2）的毫秒。

| 路线 | 中位数 ms | Q1 / Q3 ms | 每票中位数 ms | 实际写入次数 | 独立进程峰值 MiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| 逐票 Groth16 / 逐票累计 | 3.966229 | 3.410083 / 4.286669 | 0.495779 | 8 | 6.672 |
| B4 提交 / 逐票 Groth16 / Direct Fold | 3.817719 | 3.441740 / 4.212953 | 0.477215 | 2 | 6.609 |
| SnarkPack + Direct Fold | 9.607130 | 8.962177 / 11.338940 | 1.200891 | 2 | 8.000 |
| SnarkPack + State Proof | 14.234349 | 13.405089 / 17.118409 | 1.779294 | 2 | 7.750 |

**当前B4接收中两条逐票验证路线更快，SnarkPack和状态证明没有在本轮显示原生接收优势。**
不能预设Proposed更快；也不能外推到B16/B32、公链或完整系统。逐票写入是内存checkpoint写入，不是8笔EVM交易。

## 验证工作与可比性

- Legacy不是改名的B4 Receiver：真实逐票Groth16验证、逐票六个Jubjub分量累计、逐票NF和cursor更新，共8次原子提交，第4/8票对照CT1/CT2。
- 单票针对原有已接受B4根验证位置和深度2成员路径。原LEAF/NODE v2域、规范25字段编码不变；运行加载和单元测试均与原生产batch_root逐项匹配。路径验证/编码在接收计时内；8条路径512 B。路径构造在发送端预处理，单独计时batch1=0.158541ms、batch2=0.130208ms，不隐藏为免费。
- Transaction-batch真实验证8份原单票证明，根/旧状态/上下文/NF共同检查及Direct Fold后，两次B4原子提交。不是把组件耗时相加。
- 两条SnarkPack路线继续使用原Receiver的共同检查和真实聚合验证；分别直接累计或验证原两份状态证明。每条成功流程确认CT1/CT2和8个鲜活NF。
- 四个必要密码学负例：两条新路线均用原有效record/root和另一选民的原证明，实际Groth16返回false；两条聚合路线用错误newState，SnarkPack成功后Direct/State分别拒绝。失败时CT/NF/cursor保持不变，不使用冻结或缺失认证短路来冒充密码学拒绝。
- 4项相关单元测试通过，覆盖原根路径/位置、逐票原子写入及旧Receiver的连续性/跨批重复NF。不是新增大规模攻击矩阵。

## 采样、版本及边界

一个计时进程，每路线12窗口，每窗口4次完整8票流程；四路线循环轮换顺序，每个位置各出现3次，各预热2次。48窗口/192次流程仍只是一组8票证明池；无慢样本过滤、无IID选举显著性结论。
另4个新进程分别仅运行一路的1次预热/4次流程记录RSS；共同loader会加载所有路线材料，故该RSS含共同加载，不是增量算法内存，也不是Prove内存。
构建Rust1.99.1 Release/locked/offline，jobs=2、Rayon=4。原始日志记录实际命令、退出码、计时和硬件。

基准Git：b253f60e73f9dec41b80b03d339b2442ce9bfe19；执行时工作区存在本轮未提交修改，raw诚实保存git_dirty=true，**不宣称干净已提交源码复现**。
执行源码版本SHA-256：644e75eeb817a74a1218f5b76477e75b38f9116728b7cbf846b7411e4a017353；二进制SHA-256：c19a3943312727cb120105585e489cb3888d5edea9de6b7b2f0116e95ff9cb6f。
全部9项源码/锁文件加驱动原字节快照在experiments/g10_b1_raw/source_snapshot，校验脚本逐项检查，不用基准HEAD冒充新源码版本。

首轮开发试跑保存在experiments/g10_b1_raw/development_attempt1，随后完善实际记录输入绑定、拒绝路径证据和发送端路径成本，才执行最终版本。首轮源码hash不同，故不混入最终统计；不是按速度挑选样本。
本轮没有新Setup/Prove/聚合生成/EVM；相关生成耗时为null，不是0。通信仅分项：单票proof、aggregate、matrix、路径、state proof和CT，不把派生输入重复累加，完整网络封包/calldata未测。
冻结/门限/P3/Winner后缀、Full Tally对比、匹配生成负担、独立证明池和完整系统复现仍未完成。G9-P1仍BLOCKED，P2未启动。旧实验/Manifest/论文和旧图不改。

## 复现与产物

```sh
# 新结果目录必须不存在；锁文件依赖已在本机缓存
node scripts/g10b1_run.mjs /ABS/NEW_OUTPUT_DIRECTORY
# 下列只核查本次归档，不重跑证明/性能
node scripts/g10b1_report.mjs --check
node scripts/g10b1_check.mjs
```

原始数据：experiments/g10_b1_raw/matched.json及memory-1..4.json；执行命令：experiments/g10_b1_raw/execution/manifest.json；统计：results/g10_b1/summary.json。
下一步先核对已有生成成本及缺口，固定必要预算后补匹配成本；不继续扩B64+、不重跑旧规模实验或软件Miller路线。
