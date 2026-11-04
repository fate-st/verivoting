# G10-A：干净 e1e477f 的30轮配对复现

状态：PASS_CLEAN_E1E477F_THREE_PROCESSES_30_PAIRED_ROUNDS。本轮没有修改原Rust入口、原证明池、Manifest或密码学关系。

## 实测

| 路线 | 两批B4完整接收中位数 ms | Q1 / Q3 ms | 每票接收中位数 ms |
| --- | ---: | ---: | ---: |
| SnarkPack + Direct Fold | 9.082339 | 8.672273 / 10.815119 | 1.135292 |
| SnarkPack + State Proof | 13.898156 | 13.209941 / 15.716096 | 1.737270 |

配对窗口State/Direct比值中位数 1.489022，本轮30/30对窗口中State路线更慢。
结论限于当前固定8票/W32/B4和验证者负担，不支持完整系统优势，也不能据此判定状态证明在所有部署中无价值。

## 干净源码及采样单位

- 从完整提交 e1e477f94247dc8ee261400cfb2ec381e0f44fc3 建立独立detached worktree。构建、格式、严格Clippy、6项相关测试及每个运行前后均检查HEAD和空git status。三份raw均git_dirty=false。
- 不修改常量ROUNDS=10：三次顺序独立进程各10轮，共30轮配对/60窗口。每窗口平均8次完整两批接收；共480次计时流程，但只是一组固定8票证明池，不是480场或30场独立选举。
- 保留每进程原有2次/路线预热、交替AB/BA、相同研究种子。全部样本保留，无阈值删除、重跑替换或跨旧dirty样本拼接。报告列出进程分组统计，不按30个IID选举构造显著性结论。
- 使用锁定Rust 1.99.1 Release/offline，构建jobs=2、Rayon=4；复用外部依赖/编译缓存。干净源码不等于冷缓存或全新机器。
- 结果和日志在worktree之外产生；所有运行完成后才导入本分支，不把事后复制结果后的dirty状态倒填到运行时。驱动自身由独立SHA追踪，不冒充e1e477f中的源码。

| 进程 | Direct两批中位数 ms | State两批中位数 ms | 共享进程峰值 MiB |
| --- | ---: | ---: | ---: |
| 1 | 10.573378 | 13.184036 | 8.828 |
| 2 | 9.304289 | 13.540427 | 8.734 |
| 3 | 8.937135 | 15.285221 | 8.563 |

RSS包括共同材料和两路线，不是逐路线内存，更不是Prove内存。独立选票池尚未增加，不能以三个进程冒充三个证明池。

## 正确性和成本边界

三次均到达同一CT2。每次先实际测试两类错误newState，原SnarkPack通过后Direct/Groth16分别拒绝，NF/CT保持不变；共6次执行、仅2种负例。
没有新Setup/Prove/聚合生成或EVM交易；生成时间均为null/REUSED_NOT_RETIMED，不能填零。
状态证明生成负担已作为G10-B必需成本登记。G2-D/G2-E/G6历史Prove/Setup仅作带哈希来源的参考，不加到本轮干净接收时间上冒充完整实测。
未完成多个独立证明池、逐路线RSS、匹配的生成时间或完整系统成本，因此没有完成上传计划的全部性能论证。

## 复现命令

```sh
# 在仓库中建立一个全新、固定提交的worktree；下列均替换为新的绝对路径
git worktree add --detach /ABS/NEW_CLEAN_SOURCE e1e477f94247dc8ee261400cfb2ec381e0f44fc3
node scripts/g10a_clean_reproduce.mjs /ABS/NEW_CLEAN_SOURCE /ABS/EXTERNAL_TARGET_CACHE /ABS/NEW_OUTPUT_DIR
node scripts/g10a_clean30_report.mjs --check
```

驱动拒绝脏源码、错误提交、源码内输出和已有输出目录；每个命令/时间/退出码、运行前后状态及源/二进制哈希保存在 experiments/g10_a_clean30_raw/execution/manifest.json。
新输出目录复现须单独导入/分析，不覆盖本次归档；本报告脚本仅复核本次已归档数据。
原始三份run与全部日志：experiments/g10_a_clean30_raw；统计：results/g10_a_clean30/summary.json。
旧 experiments/g10_a_raw、results/g10_a 和原文档均保持原字节。G9-P1仍BLOCKED，P2未启动。
