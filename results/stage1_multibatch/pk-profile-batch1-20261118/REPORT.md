# Batch 1 PK Profile 完整执行报告

最终状态：**PASS_PROFILE_ONLY**。Gate C：**NOT AUTHORIZED / NOT STARTED**。本轮已结束，不自动继续。

## 1. 前置、移植与诊断源码核验

原始根目录：`/Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter`。临时根目录：`/tmp/vv-pk-profile`。

HEAD 保持 `3f8813c9d69d0e45631e9acb7bbb247a1119f511`；原始 Rust Blob 为 `547d459670db3f9c15b833a1c24dc28f2cfe1602`。
三个批准补丁与 A-06 引脚允许路径交集，仅按已获确认的严格字节/哈希一致规则移植；移植前后全部等于 A-06 固定值。未覆盖任何历史记录。

- `contracts/scripts/stage1_multibatch_compare.mjs`：`665be8f8800429c01026c680cd362c086ec9bee1c48034566bc259a5a1695c52`
- `scripts/stage1_multibatch_audit.mjs`：`0305e0740a2c1f15d3aa4d1cdb279908ccf3f25984dc2bd5091f1f6e0773e9bc`
- `docs/STAGE1_MULTIBATCH.md`：`f5620e7b30dc6dcfee6714393aee75d3f75d09b161f7c21a694c9bea76195e33`

15 个引脚（9 个配置输入 + 6 个 A-06 文件）在原工作区及隔离 Worktree 全部匹配。Cargo.toml/Cargo.lock 未修改；libc 已为直接依赖；autobins 未禁用。已捕获 141 个未跟踪文件的完整字节副本及逐项哈希。单一移植补丁为 `evidence/approved_patches.diff`；历史未跟踪文件副本保留于 `evidence/untracked_bytes/`，不作为本次 Worker 输入。只给 Worker 原样 request/setup 和只读 PK 符号链接。

独立诊断源码：`crates/vv-friendly-dkg/src/bin/stage1_multibatch_pk_profile.rs`；SHA-256：`6b1ee7d425a037e8355aa2a724817bf21db4b05c557fb3faedb166b4af28fb58`。
`instrumentation.diff` SHA-256：`45f0231a82ee64fa327a75f80dd8c17501dbe6e27fb7621e319428f1e1df6a2f`。差异包括 16 段计时、字节计数、getrusage、独立诊断输出和仅允许 B1 的入口保护；输出 JSON 构造与文件写出分开计时。电路、密码学调用、规范验证、编码、种子、选票顺序不变。

worker 返回 `Result<()>`，不返回 ProvingKey/R1CS 大对象；main 中外层计时在 worker 真正返回后停止，涵盖其局部 Drop。完整报告中的结论来自实际单次运行，不是模拟。

## 2. 编译与外层时间分解

编译命令：`cargo +1.99.1 build --release --locked --offline --bin stage1_multibatch_pk_profile`，CARGO_TARGET_DIR=`/tmp/vv-pk-profile/target`，CARGO_BUILD_JOBS=2，RAYON_NUM_THREADS=4，RUSTUP_TOOLCHAIN=1.99.1，Release opt-level=1。

编译退出码 0；compile_wall_ms = 47691.619709（47.691620 s），独立于运行墙钟。完整逻辑副本产生 5 条未使用项警告（1 个常量、4 个函数）：OLD_STATE, canonical, spawn_worker, ballot_vk, prepare；没有编译错误。

运行命令：`/tmp/vv-pk-profile/target/release/stage1_multibatch_pk_profile prove-worker experiments/stage1_multibatch_raw/pk-profile-batch1 B 1 --approved B`，cwd=`/tmp/vv-pk-profile/worktree`；只启动一次，PID=59257，退出码=0。

outer_wall_semantics=`spawn_to_observed_exit`，nominal_poll_interval_ms=250，polling_overhead_bound_ms=null。日志在计时前打开；计时从 Popen 前到 poll 观察到退出为止。

outer_wall_ns=597139812792；outer_wall_ms=597139.812792；outer_wall_seconds=597.139813。

| 分项 | ns | ms | 秒 | 占 outer % |
|---|---:|---:|---:|---:|
| process_config | 9800917 | 9.800917 | 0.009801 | 0.001641 |
| worker_total | 596426130416 | 596426.130416 | 596.426130 | 99.880483 |
| outer_residual | 703881459 | 703.881459 | 0.703881 | 0.117875 |

process_config 从 main 入口至 config 返回，包含参数、路径和配置 9 项引脚校验，不含 OS 创建/动态链接。outer_residual 已用整数纳秒断言非负，含进程创建、动态链接、分发、Profile JSON 写出、退出与轮询误差，不能全部解释为 OS 启动成本。

## 3. Worker 内部 16 阶段与残差

| 阶段 | ns | ms | 秒 | 占 worker % |
|---|---:|---:|---:|---:|
| worker_input | 33847333 | 33.847333 | 0.033847 | 0.005675 |
| request_bind | 54792 | 0.054792 | 0.000055 | 0.000009 |
| witness_diagnostic | 3472283959 | 3472.283959 | 3.472284 | 0.582182 |
| pk_sha256 | 1575599042 | 1575.599042 | 1.575599 | 0.264173 |
| pk_open | 58625 | 0.058625 | 0.000059 | 0.000010 |
| pk_deserialize | 571967173458 | 571967.173458 | 571.967173 | 95.899080 |
| pk_tail_check | 2375 | 0.002375 | 0.000002 | 0.000000 |
| vk_ark_hash | 69667 | 0.069667 | 0.000070 | 0.000012 |
| proof_seed | 250375 | 0.250375 | 0.000250 | 0.000042 |
| prove | 18767489084 | 18767.489084 | 18.767489 | 3.146658 |
| prepare_vk | 1066417 | 1.066417 | 0.001066 | 0.000179 |
| ark_verify | 2073375 | 2.073375 | 0.002073 | 0.000348 |
| proof_serialize | 1541 | 0.001541 | 0.000002 | 0.000000 |
| eip_export | 88791 | 0.088791 | 0.000089 | 0.000015 |
| omission_check | 4250 | 0.004250 | 0.000004 | 0.000001 |
| output_write | 1429708 | 1.429708 | 0.001430 | 0.000240 |
| unattributed_worker | 604637624 | 604.637624 | 0.604638 | 0.101377 |

16 阶段互斥；整数断言 `worker_total_ns = sum(16 stages) + unattributed_worker_ns` 通过。内部残差包括 setup.json 读取、JSON 对象构造、显式 R1CS Drop、返回前其他局部对象 Drop 及少量未埋点记账。

## 4. PK SHA-256

pk_sha256_ms=1575.599042，为独立顺序读文件及完整性哈希检查，不与反序列化合并。

## 5. PK 反序列化墙钟与边界

pk_deserialize_ms=571967.173458，包含流式读取、压缩点解码和原有 Validate 检查。没有使用 Validate::No，也未复用内存 PK。不能从本次总耗时推断其中子群检查的独立占比。

## 6. 反序列化 CPU 辅助指标

user_cpu_ms=560120.52；sys_cpu_ms=7083.287；累计 CPU / 该阶段墙钟=0.991672。
采用 getrusage(RUSAGE_SELF) 区间差值，累计本进程所有线程 CPU 时间，可能大于墙钟；这些指标不加入墙钟求和。

## 7. 实际消费字节数

pk_deserialize_bytes_read=847800624，CountingReader<BufReader<File>> 在 deserialize_compressed 返回后、尾部检查前取值。它记录反序列化器实际消费的逻辑字节，非 OS 物理读盘量。与 PK 847800624 bytes 一致。

## 8. VK Ark 哈希

vk_ark_hash_ms=0.069667。Ark VK SHA-256：`92871cac5c3452ab0a1647939507d7b041dcccf189e3b60ea2b19c1d6e1e1856`；EIP VK SHA-256：`1367728908bb6c62009afb38ad09555d5a8c463bc6945b3ceb83ae763f376896`，均保持既有固定值。

## 9. Witness 与 Prove

witness_diagnostic_ms=3472.283959；prove_ms=18767.489084，独立报告，不将二者合并为 Prove。不包含 prepare 阶段的 32 份原选票证明验证；本轮没有调用 prepare/Setup。

## 10. Ark Verify 与 EIP 导出

ark_verify_ms=2.073375；eip_export_ms=0.088791。仅执行本地 Ark 验证和 EIP-2537 导出检查，没有执行任何 EVM 验证/部署/交易；Gas=NOT_MEASURED。

## 11. 历史对照、归因与 Page Cache

历史 B-02 Batch 1 外层墙钟 593398.6029579999 ms（约 593.399 s）；本次 597139.812792 ms；差值 3741.2098340000957 ms。

历史 spawn_worker 计时开始于打开日志和 current_exe/spawn 之前，每次轮询还包含 ps RSS 检查；新计时开始于日志打开后、Popen 前，轮询不调用 ps。二者均在观察到退出后结束，名义轮询间隔 250 ms，但边界/轮询开销不完全相同，不能作为严格同边界速度提升或单一编译参数因果证据。

本次最大阶段为 **pk_deserialize，571.967173 s，占 worker 95.899080%**；未归因残差为 0.604638 s。

执行前 16 项快照及输入哈希读取了 B-02 大型 PK，Worker 又先执行 PK SHA-256 顺序读取。因此本轮是**当前活跃缓存状态下的测量，不是冷缓存磁盘性能**。CPU/墙钟数据可以辅助判别计算与等待，但没有进一步区分解码和群检查；不得将整个阶段全部归因于其中单一算法。

这是一次诊断，不是统计优势结论，也不把新阶段值倒填为历史 593 秒的分项实测。是否采用单次安全加载、多批只读 PK 复用，须下一轮单独授权；本轮未优化，也不推导链上 Gas 收益。

## 12. 证明字节级核验

- Ark：192 B，SHA-256=`ff2b7b683b63a172c317612ab63fc093a4fa0599b7fd1faa767050ce059d45ec`，与历史原证明逐字节相同。
- EIP-2537：512 B，SHA-256=`8fad586ada7525d7b9b492d42d6b1d4387f754e31dce18959af2bc5d42333341`，与历史导出逐字节相同。
- 批次根：`0x91f6c687850b799e58e413c6c9c3b67e1acb1f2cb8a03e2b4d88a3729a9a2134`；公开输入数量 37。
- batch_1.json 完整递归字段结构与原版相同；除 witness_ms, prove_ms, verify_ms, prepare_vk_ms, peak_rss_mib 五个测量字段外，其余内容全部一致。没有添加诊断字段到 batch_1.json。

确定性 ChaCha20Rng 仅为重现既有 Batch 1 证明字节、验证埋点零扰动；这是研究复现设置，不是生产安全随机数方案。

## 13. 原工作区与输入最终完整性

HEAD/status/index、全部已跟踪差量及 141 个未跟踪文件内容哈希前后一致。完整前后记录见 before.json / integrity_after.json；所有快照在原始主工作区根目录读取，包含根目录、空目录、ignored 缓存、文件权限/大小/hash、符号链接目标，没有忽略空目录。

| 只读输入 | Bytes | 前后相同 SHA-256 |
|---|---:|---|
| research_pk.bin | 847800624 | 448d9d92b9250aa2bcbf59c82036ab1615f2fe323bbb5a39b8f8d7ec9f73da1c |
| request.json | 318 | e5cf3a56500341f4a91da8dd7f0f6d3eed97431bc7d01eb265372edc5552f238 |
| setup.json | 344 | e6ec81ab5944f064cbb4c93bc9dbcbaacf63b1958fa56d39892263c39f88bc8d |

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

原始 git status（前后逐字相同）：

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

运行数量：Setup=0，Batch 1 Prove Worker=1，Batch 2=0，EVM=0，commit=0，push=0。Gate C **NOT AUTHORIZED / NOT STARTED**。完成后停止。
