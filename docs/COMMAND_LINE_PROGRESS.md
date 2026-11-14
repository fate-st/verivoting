# 命令行自动查看实验进度

在项目根目录执行：

```sh
node scripts/experiment_progress.mjs --watch
```

每2秒刷新，自动按开始时间选择最新运行目录，显示各步骤结果、耗时、选票/状态证明数量和最新日志。
结束后仍可等待下一次新运行；按Ctrl+C只退出查看器，不影响实验。

只看一次：`node scripts/experiment_progress.mjs`。

固定查看某次运行：

```sh
node scripts/experiment_progress.mjs --watch --run experiments/stage2_matched_cost_raw/run-20261114-02
```

也可使用`--interval 5`改变刷新间隔，或`--json`一次性输出结构化状态。
目前自动查找范围为experiments/stage2_matched_cost_raw中的阶段二成本实验；其他位置可用--run或--base指定。
仅支持VV/STAGE2/EXECUTION/v1，项目整体进度与后续任务仍以docs/IMPLEMENTATION_STATUS.md及docs/NEXT_STEPS.md为准。

查看器只读取记录，不会编译、生成证明、启动或终止实验，不会修改旧数据或自动提交Git。
清单RUNNING不等于已核实进程存活；日志为空也不等于卡住。步骤进度不是算力百分比或剩余时间估计。
进程步骤耗时包含PK加载等开销，纯Prove/聚合时间另行展示；本轮通过不代表整个项目完成。

必要检查：`node --test scripts/experiment_progress.test.mjs`（仅查看器测试，不运行密码学实验）。

## 本轮 EVM 对比

默认自动从原生成本和 EVM 累计对比两类目录中按 started_utc 选择最新运行，不依赖文件夹复制时间。
无需手动 tail 或启动另一场实验：

```sh
node scripts/experiment_progress.mjs --watch
# 只读核查本次 EVM 档案，不重跑证明或交易
node scripts/stage2_evm_fold_check.mjs experiments/stage2_evm_fold_raw/run-20261114-01
```

EVM 面板显示独立的八个执行步骤，不把原生成本阶段的 Setup/Prove 进度混入本轮。
完成后的两批接收 Gas 只是一个分项；部署/认证/冻结与限制见 [完整报告](STAGE2_EVM_FOLD_COMPARISON.md)。
