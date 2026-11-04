# G10-A：同一 W32 两批 B4 的原生 Direct Fold 消融

状态：PASS_MATCHED_TWO_B4_NATIVE_RECEIVE_PATHS。基准 7475f564dc45b923f30ea25142882f703d821b10，本轮执行时 dirty=true，真实源码和二进制哈希保留在 raw。

## 实测结果

| 路线 | 两批完整接收中位数 ms | Q1/Q3 ms | 每票中位数 ms | 接收吞吐量票/s |
| --- | ---: | ---: | ---: | ---: |
| SnarkPack + Direct Fold | 9.206768 | 9.021786 / 11.155187 | 1.150846 | 868.929668 |
| SnarkPack + State Proof | 15.559338 | 13.584855 / 16.373762 | 1.944917 | 514.317870 |

配对窗口 State/Direct 比值中位数 1.506481。这是同一固定证明池的描述性结果，不是完整投票系统加速比或统计显著性结论。
本轮状态证明路线更慢，不能声称原生验证效率优势。首轮两路径均较慢，全部样本保留，不过滤或替换。

## 同环境和计时边界

- 两条路线均实际执行相同 G2-E Receiver：初始化、批次序号/旧CT/上下文/注册根/接受根/Nullifier检查；从真实记录重建矩阵和原transcript；真实SnarkPack验证；原子写checkpoint；最终CT2及8个NF检查。
- 仅增量不同：candidate-major独立Direct Fold重做六分量累计并比较new CT，或从当前提交对象重建37项输入并验证原Groth16状态证明。旧Receiver、折叠实现和所有密码学关系未改。
- 每路线2次预热、10个交替AB/BA配对轮；每窗口8次完整两批流程，共20个原始窗口。不把160次流程误报成160场独立选举。两路径对应调用使用相同研究RNG种子，但并行accumulator任务调度不保证随机系数分配逐等式相同；它不是生产随机性策略。
- 总时间直接包围完整接收流程，不用历史组件时间相加；packing/SP/增量分项只用于解释，不回填total。离线排除文件解码和VK预处理。
- Apple M4，34359738368 B RAM；Rust 1.99.1 Release/locked/offline；RAYON_NUM_THREADS=4。进程累计峰值 9.172 MiB，包括共同加载材料和两路线，不能当逐路线内存比较。

## 必要正确性验收

两路径均到达同一G2-E CT2。仅修改第一批new CT的一个分量、保留原aggregate/状态proof/记录/根；在正式接受前提交。
两次SnarkPack均实际通过，之后Direct Fold或Groth16状态关系拒绝；next=1、spent=0和旧CT均不变。
这不是冻结后的FROZEN拒绝，也不是session ID不匹配。没有增加攻击/故障矩阵。

## 成本和复用口径

复用两份aggregate、两份state proof、同一ballot VK/state VK及B4 verifier SRS；新Setup/Prove/聚合生成均为0次。
生成时间未重测，以null和REUSED_NOT_RETIMED标记，不表示生成成本为零。
每B4公共矩阵3200 B、old+new CT分量384 B；State Proof额外192 B。37项state statement由已绑定数据派生，不能再与矩阵/CT重复相加当通信总量。
这些仅是二进制分项，未定义完整网络封包/DA/calldata，故Gas和calldata保持null。没有EVM、Full Tally/Winner-only或五方案系统比较。

## 复现

```sh
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 build --release --locked --offline -p vv-friendly-dkg --bin friendly-g10a-native-fold
RAYON_NUM_THREADS=4 target/release/friendly-g10a-native-fold --output experiments/g10_a_reproduce/NEW_RUN.json
node scripts/g10a_report.mjs --check
```

源码/原始窗口：experiments/g10_a_raw/native_direct_fold_b4.json；摘要：results/g10_a/summary.json；真实命令/失败构建日志：results/g10_a/execution。
现有报告脚本核对已归档run；新路径复现实验须单独分析，不能覆盖原始测量。
本轮初次构建只因JSON宏默认递归限制失败，修正为512后编译、严格Clippy及6项相关测试通过；失败日志保留。

## 停止边界

只完成原生B4 Direct Fold与State Proof公平增量比较。不扩B8状态或B64+，不重跑G1-E，不从旧Gas拼出系统优势。
G9-P1仍需闭合精确关系和有界EVM资源；G9-P2/P3不启动。
