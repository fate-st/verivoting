# PK-Reuse Pilot：两批单次执行完整报告

状态：**PASS_REUSE_PILOT_ONLY**。Gate C：**NOT AUTHORIZED / NOT STARTED**。只运行一次证明进程，顺序生成 Batch 1、Batch 2；完成后停止。没有 Setup、EVM、提交或推送。

## 执行与源码边界

原工作区 `/Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter`，HEAD `3f8813c9d69d0e45631e9acb7bbb247a1119f511`；全部输出在 `/tmp/vv-pk-reuse-pilot`。16 项保护路径的前后快照均从原工作区读取。
9 项配置输入和 6 项 A-06 源码引脚在原工作区/独立 Worktree 均匹配。三个批准补丁只按已授权的哈希一致方式移植；Cargo 清单、锁文件、电路、原材料生成器及原始 Setup/证明种子规则未变。

独立源码：`crates/vv-friendly-dkg/src/bin/stage1_multibatch_pk_reuse_pilot.rs`，SHA-256 `077f358687a6382832d9432fb38004d274e2bf4181bbf44c5df44d9fbe241622`；`reuse_pilot.diff` SHA-256 `c737b6544929d208455274e6a15c8b43b46581079689f481d7a2dc2d7042d43e`。
diff 的功能变化仅限新入口中的一次安全加载、两批只读引用复用、计时/CPU/字节采样、进度标签和错误落盘。反序列化调用点只有一处，仍为完整 `ProvingKey::deserialize_compressed()`；不使用 unchecked/Validate::No、不克隆 PK。

两份 batch JSON 的字段结构未变。为避免虚假元数据，仅将 proof_origin 如实标为单进程只读复用，rss_scope 如实标为累积进程高水位；其余除原有计时/RSS数值外，所有密码学字段逐字节与历史相同。R1CS 与批内对象在各批函数返回时析构，PK 在整个 worker 返回前析构，均计入相应外层计时。

编译命令：`cargo +1.99.1 build --release --locked --offline --bin stage1_multibatch_pk_reuse_pilot`；CARGO_TARGET_DIR=`/tmp/vv-pk-reuse-pilot/target`，CARGO_BUILD_JOBS=2，RAYON_NUM_THREADS=4，Rust/Cargo 1.99.1，Release opt-level=1。编译退出码 0，耗时 46.444753 s，未混入运行墙钟。编译日志保留未使用项警告，没有自动 cargo fix 或修改参数。

运行命令：`/tmp/vv-pk-reuse-pilot/target/release/stage1_multibatch_pk_reuse_pilot prove-batch-range experiments/stage1_multibatch_raw/pk-reuse-pilot-2batch B 1 2 --approved B`；PID=69865，退出码 0。主机 Apple M4 / 32768 MiB / ProductName:		macOS; ProductVersion:		15.7.1; BuildVersion:		24G231。

## 1. 一次性完整 PK 加载

- pk_load_count = **1**。
- pk_once_load_total_ms = **575703.350209**（575.703350 s）。
- batch_1_pk_deserialize_ns = **0**，batch_1_pk_deserialize_ms = **0.0**。
- batch_2_pk_deserialize_ns = **0**，batch_2_pk_deserialize_ms = **0.0**。

每批的零表示批内没有反序列化调用，绝不表示共享加载没有成本；真实加载已单独计入 pk_once_load_total。

| 加载指标 | ms |
|---|---:|
| pk_sha256 | 1581.627416 |
| pk_open | 0.053083 |
| pk_deserialize | 574121.565333 |
| pk_tail_check | 0.002500 |
| vk_ark_hash | 0.040000 |
| load 内部残差 | 0.061877 |

pk_deserialize_user_cpu_ms=563436.369，pk_deserialize_sys_cpu_ms=6976.393；一次加载总区间 user/sys=564953.219/7035.704000000001 ms。CPU 累计全部线程，不与墙钟相加。
pk_deserialize_bytes_read=847800624，在 canonical deserialize 返回后、尾部检查前读取 CountingReader 计数。读取、解码与原有验证均保留；VK/tail 检查均通过。

缓存边界：保护目录快照/输入校验及 Worker 自身 PK SHA-256 均读取了 PK。本轮仍是当前活跃缓存条件；不能宣称冷缓存磁盘性能，也没有独立测量子群检查的耗时占比。

## 2. 两批独立出证与历史对照

| Batch | Witness ms | Prove ms | Ark Verify ms | EIP Export ms | Output ms | 批总计 ms（不含共享加载） | 批内 PK decode ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | 3630.717666 | 18374.457667 | 2.091083 | 0.084416 | 0.973542 | 22628.008458 | 0.0 |
| 2 | 3242.451834 | 18415.718209 | 2.717166 | 0.083875 | 1.068041 | 22254.112084 | 0.0 |

process_config_ms=7.515209；worker_total_ms=620603.574042；unattributed_worker_ms=18.103291。
outer_wall_ms=621501.246625，outer_residual_ms=890.157374。
外层语义 spawn_to_observed_exit，日志打开后 Popen 前计时，名义 RSS/退出轮询间隔 250 ms，polling_overhead_bound_ms=null。全部整数纳秒残差非负，加载层、批次层、Worker 层和外层的求和恒等式通过。

历史两独立 Worker：593.398603 s + 591.718211 s = **1185.116814 s**。
本轮含共享加载的完整外层：**621.501247 s**。
相对历史合计节省 **563.615568 s**，加速比 **1.906862×**（DERIVED，自实测值计算）。这不是同轮独立重复统计；旧计时含日志打开与 RSS 探针，新计时起点在日志打开后，不能声称严格配对统计显著优势或推广到完整系统/Gas。

全部互斥子阶段及残差：

| 层级 | 阶段 | ns | ms |
|---|---|---:|---:|
| load | pk_sha256 | 1581627416 | 1581.627416 |
| load | pk_open | 53083 | 0.053083 |
| load | pk_deserialize | 574121565333 | 574121.565333 |
| load | pk_tail_check | 2500 | 0.002500 |
| load | vk_ark_hash | 40000 | 0.040000 |
| load | pk_once_load_unattributed | 61877 | 0.061877 |
| batch_1 | worker_input | 27851709 | 27.851709 |
| batch_1 | request_bind | 457292 | 0.457292 |
| batch_1 | witness_diagnostic | 3630717666 | 3630.717666 |
| batch_1 | proof_seed | 48375 | 0.048375 |
| batch_1 | prove | 18374457667 | 18374.457667 |
| batch_1 | prepare_vk | 1256000 | 1.256000 |
| batch_1 | ark_verify | 2091083 | 2.091083 |
| batch_1 | proof_serialize | 1292 | 0.001292 |
| batch_1 | eip_export | 84416 | 0.084416 |
| batch_1 | omission_check | 4166 | 0.004166 |
| batch_1 | output_write | 973542 | 0.973542 |
| batch_1 | unattributed | 590065250 | 590.065250 |
| batch_2 | worker_input | 26755333 | 26.755333 |
| batch_2 | request_bind | 230875 | 0.230875 |
| batch_2 | witness_diagnostic | 3242451834 | 3242.451834 |
| batch_2 | proof_seed | 69000 | 0.069000 |
| batch_2 | prove | 18415718209 | 18415.718209 |
| batch_2 | prepare_vk | 1011750 | 1.011750 |
| batch_2 | ark_verify | 2717166 | 2.717166 |
| batch_2 | proof_serialize | 1167 | 0.001167 |
| batch_2 | eip_export | 83875 | 0.083875 |
| batch_2 | omission_check | 4291 | 0.004291 |
| batch_2 | output_write | 1068041 | 1.068041 |
| batch_2 | unattributed | 564000543 | 564.000543 |
| worker | unattributed_worker | 18103291 | 18.103291 |

## 3. RSS、物理内存、Swap 与八批预测

| 项目 | MiB |
|---|---:|
| 主机物理内存 | 32768 |
| 原生 worker_peak_rss（ru_maxrss） | 6173.09375 |
| 外部 250ms 名义采样峰值 | 6173.09375 |
| 历史 B-02 Batch 1 原生基准 | 5513.1875（约5513.188） |
| 本轮原生峰值相对历史差值 | 659.90625 |
| PK 加载后进程累积峰值 | 1568.078125 |
| Batch 1 返回后进程累积峰值 | 5610.890625 |
| Batch 2 返回后进程累积峰值 | 6173.09375 |

外部峰值时间：2026-11-18T06:32:15.056921Z；共 2417 个该 Worker 的有效样本，采样错误 0。外部采样与 ru_maxrss 为不同口径，不互相替代。

| 外部观测阶段 | 样本数 | 首次 RSS MiB | 末次 RSS MiB | 采样峰值 MiB |
|---|---:|---:|---:|---:|
| spawn | 3 | 0.03125 | 0.03125 | 0.03125 |
| pk_load | 2240 | 1.875 | 784.1875 | 1567.421875 |
| batch_1 | 87 | 1196.4375 | 3497.625 | 5610.890625 |
| batch_2 | 86 | 2754.6875 | 4297.59375 | 6173.09375 |
| worker_cleanup | 1 | 2530.734375 | 2530.734375 | 2530.734375 |

ru_maxrss 为进程生命周期累积高水位，单独上升不能证明内存泄漏，外部阶段采样也不是精确边界 RSS。仅两个批次不能证明八批常驻内存有界。
主机 Swap 前：`total = 8192.00M  used = 7017.69M  free = 1174.31M  (encrypted)`；后：`total = 8192.00M  used = 7017.69M  free = 1174.31M  (encrypted)`。
系统 Swap used 变化 0 MiB；全机 Swapout 计数变化对应 0 MiB。这些是主机全局指标，运行前已有 Swap 使用，不能把变化独占归因于 Pilot，也不能据此声称进程完全无 Swap。

八批模型（全部 **DERIVED / NOT_EXECUTED**，不是验收数据或严格上界）：

- 按已测一次加载 + 8×两批平均批耗时 + 已测固定残差，预计 756.147608 s；改用两批较慢者，预计 757.643194 s。
- 若内存驻留平台不再上升，参考已测峰值 6173.09375 MiB；若仅按两个高水位点的增长 562.203125 MiB/批线性外推，八批场景为 9546.3125 MiB。二者均非承诺/上界。
- 物理内存减已测峰值为 26594.90625 MiB；减线性场景为 23221.6875 MiB。这只是名义差值，未扣除系统/其他应用，**不是可用内存保证**。不以任意 16 GiB 阈值单独判定成败。
- 原配置每 child 名义预算为 600 s；本次独立 Pilot 未调用旧 spawn_worker。若后续把整个范围证明放入同一 child，应重新审查其计时边界和预算策略，不能自动修改预算或直接放行 Gate C。

## 4. 输入与密码学字节等价

historical_request_sha256=`e5cf3a56500341f4a91da8dd7f0f6d3eed97431bc7d01eb265372edc5552f238`；pilot_request_sha256=`e5cf3a56500341f4a91da8dd7f0f6d3eed97431bc7d01eb265372edc5552f238`；request_sha256_equal=**true**。运行前后 request、setup、PK 的历史/Pilot 哈希均一致。

| Batch | Ark 新旧相同 SHA-256（192 B） | EIP-2537 新旧相同 SHA-256（512 B） |
|---|---|---|
| 1 | ff2b7b683b63a172c317612ab63fc093a4fa0599b7fd1faa767050ce059d45ec | 8fad586ada7525d7b9b492d42d6b1d4387f754e31dce18959af2bc5d42333341 |
| 2 | ca115876f8b48d4382d446b08a760e477ca7004da73cc2e7213191efdbfb09d7 | bb36782045dd4ada2fd63eac11fe76d1e47f792001a812a16cd82470b5449701 |

Batch 1 根：`0x91f6c687850b799e58e413c6c9c3b67e1acb1f2cb8a03e2b4d88a3729a9a2134`；状态/公开输入规范 JSON 摘要：`6011679dec079faf9892e4af45b0266616b78a216b3997323508219a224d11ec`（新旧相同；该摘要仅为审计，不是新增协议承诺）；公开输入 37 项。

Batch 2 根：`0x4e3a7649aa9d7455509a0b052d85c6745b38cb1f2416045c08ffd25094837fcf`；状态/公开输入规范 JSON 摘要：`8e689c614233c68f1f779e17f747aba584e62370adf5ac47e3bf79e70baefc0f`（新旧相同；该摘要仅为审计，不是新增协议承诺）；公开输入 37 项。

两个批次的根、old_state/new_state 全部密文分量、公开输入、Ark/EIP VK 和导出字节均逐项一致；Batch 1 new_state 严格等于 Batch 2 old_state。完整字段在 report.json。
两次证明均执行真实 Ark 验证；没有 mock、没有替换验证标志、没有 EVM。每批 RNG 仍为 SHA256(request 原始字节 || batch_index 的 u64 大端字节) 初始化独立 ChaCha20Rng；仅用于研究复现、非生产随机数方案。

## 5. 原工作区完整性与停止

原 HEAD、完整 git status、index、跟踪差量、141 个未跟踪文件内容哈希、论文和上一轮 PK Profile 产物均未变。独立 Worktree 中原始材料生成器 Blob 仍为 `547d459670db3f9c15b833a1c24dc28f2cfe1602`，15 项引脚与 Cargo 清单/锁文件均未变。

| 保护路径 | 状态 | 前后 SHA-256 | 核验 |
|---|---|---|---|
| `experiments/stage1_multibatch_raw/gate-a-20261117-01` | EXISTS | 9fca95613604bc85299ed2691723cc3d0639618ccec87d3ba1f2599cfc589257 | MATCH |
| `results/stage1_multibatch/gate-a-20261117-01` | NOT_PRESENT | — | STILL_ABSENT |
| `experiments/stage1_multibatch_raw/gate-a-20261117-02` | EXISTS | 26db2668ea46a605821f67e6154b2b4eeb5775aa22ad7906e778594ee14fcdc6 | MATCH |
| `results/stage1_multibatch/gate-a-20261117-02` | EXISTS | c748f58d4b417e644c75bae7fe6f47aca6c688bf37c5eb3074d313108783a54f | MATCH |
| `experiments/stage1_multibatch_raw/gate-a-20261117-03` | EXISTS | 5d12a6236ee84e2bcc96072cd086b9d424ece88165f0a9ed74cbc9e9f017e413 | MATCH |
| `results/stage1_multibatch/gate-a-20261117-03` | EXISTS | ab110b069bb33d62989526ab2ddba62da7622573028319c103cc7656d153c2c3 | MATCH |
| `experiments/stage1_multibatch_raw/gate-a-20261117-04` | EXISTS | 663a2d09bf152b3ff04ab6b6a7f91d4baacb5aa0af74633022922e4c8bae9f90 | MATCH |
| `results/stage1_multibatch/gate-a-20261117-04` | EXISTS | c398fb112dcbbb891ecb9d9f35a6616235c2eaa700232289c3872a87f9497bea | MATCH |
| `experiments/stage1_multibatch_raw/gate-a-20261117-05` | EXISTS | a73d794c95d1d4c31d9f3a06b1c9f46b210f81c1d0ba5e39d0926bc743b6a72c | MATCH |
| `results/stage1_multibatch/gate-a-20261117-05` | EXISTS | c6eb5a7ed997f24566ecf8d00d019008d3cf233d492591ec056cf4014928bdce | MATCH |
| `experiments/stage1_multibatch_raw/gate-b-20261117-01` | EXISTS | 99e00d934b2d23e8d3c61c7abe6bbeca541ee5efdbd36e62192431c45526c85e | MATCH |
| `results/stage1_multibatch/gate-b-20261117-01` | NOT_PRESENT | — | STILL_ABSENT |
| `experiments/stage1_multibatch_raw/gate-a-20261117-06` | EXISTS | 6ab6fdca6b239b59d792770bb523060ec48b7ad8d48ba9f0ac7da77cd4f3b8b0 | MATCH |
| `results/stage1_multibatch/gate-a-20261117-06` | EXISTS | 1910bc87adfaa3722c7996bc85d04b0f647e7b88db0c46c25e1d545228c19f97 | MATCH |
| `experiments/stage1_multibatch_raw/gate-b-20261117-02` | EXISTS | dced9d6123a7664b00583db73a8a36a288b58cdb2054d2fe9f6348b59268f10a | MATCH |
| `results/stage1_multibatch/gate-b-20261117-02` | EXISTS | 60d4ed7290422ac9740e18eb1edf999caf813c568e457898aaf4bfa016120dc8 | MATCH |

原始 status 前后逐字一致：

```text
 M contracts/scripts/stage1_multibatch_compare.mjs
 M docs/STAGE1_MULTIBATCH.md
 M scripts/stage1_multibatch_audit.mjs
?? experiments/stage1_multibatch_raw/gate-a-20261117-05/
?? experiments/stage1_multibatch_raw/gate-a-20261117-06/
?? experiments/stage1_multibatch_raw/gate-b-20261117-01/
?? experiments/stage1_multibatch_raw/gate-b-20261117-02/
?? results/stage1_multibatch/gate-a-20261117-05/
?? results/stage1_multibatch/gate-a-20261117-06/
?? results/stage1_multibatch/gate-b-20261117-02/
```

只执行一轮 Pilot，没有自动重试。没有 Setup、8 批扩展、Gate C、EVM、commit 或 push。**Gate C 仍为 NOT AUTHORIZED / NOT STARTED。**
