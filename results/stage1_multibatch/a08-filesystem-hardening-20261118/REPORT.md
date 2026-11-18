# Gate A-08 文件系统写入隔离加固报告

最终状态：`PASS_A08_FILESYSTEM_HARDENING`。仅完成阶段 1–2；这是文件系统加固验收，不是 Gate A-08、Rust/Solidity 编译或八批实验验收。

## 1. 执行范围与身份

- 实际工作区：`/private/tmp/vv-pk-reuse-integration.PohVG2/worktree`
- 分支：`feat/stage1-pk-reuse-integration`
- HEAD：`3f8813c9d69d0e45631e9acb7bbb247a1119f511`
- 所有 Git 命令显式带 `GIT_OPTIONAL_LOCKS=0`；没有 commit、push、reset 或索引更新。

修改任何源码前已逐项执行 realpath、show-toplevel、branch、HEAD、status 核验，并登记三份准备文件；控制台记录对应状态如下（非修改后状态）：

```text
 M contracts/scripts/stage1_multibatch_compare.mjs
 M crates/vv-friendly-dkg/src/bin/stage1_multibatch_materials.rs
 M docs/STAGE1_MULTIBATCH.md
 M scripts/stage1_multibatch_audit.mjs
?? contracts/node_modules
?? experiments/stage1_multibatch_raw/gate-a-20261117-07/
?? experiments/stage1_multibatch_raw/gate-b-20261117-03/
?? results/stage1_multibatch/gate-a-20261117-07/
?? results/stage1_multibatch/gate-a-20261117-08/
?? results/stage1_multibatch/gate-b-20261117-03/
```

修改后完整状态：

```text
 M contracts/scripts/stage1_multibatch_compare.mjs
 M crates/vv-friendly-dkg/src/bin/stage1_multibatch_materials.rs
 M docs/STAGE1_MULTIBATCH.md
 M scripts/stage1_multibatch_audit.mjs
?? contracts/node_modules
?? contracts/scripts/native/
?? experiments/stage1_multibatch_raw/gate-a-20261117-07/
?? experiments/stage1_multibatch_raw/gate-b-20261117-03/
?? results/stage1_multibatch/gate-a-20261117-07/
?? results/stage1_multibatch/gate-a-20261117-08/
?? results/stage1_multibatch/gate-b-20261117-03/
```

本轮项目内仅改动 Runner，新增 C 源码与其 arm64 本机可执行文件。Rust 源码、Auditor、文档原有修改均为之前已有，三者本轮 SHA-256 均未改变。未准备真实 Cargo 依赖/编译缓存；临时目录仅含模拟夹具与文件系统测试证据。

### 三份真实 A-08 准备文件（前后全字段一致）

| 绝对路径 | 类型 | 字节数 | mode | SHA-256 |
| --- | --- | ---: | --- | --- |
| `/private/tmp/vv-pk-reuse-integration.PohVG2/worktree/results/stage1_multibatch/gate-a-20261117-08/integrity_before.json` | regular file | 37071 | `0644` | `dfab4eea1efecf8272f2f7db96512feb087abc0eac44398855ea8c5048214e44` |
| `/private/tmp/vv-pk-reuse-integration.PohVG2/worktree/results/stage1_multibatch/gate-a-20261117-08/prebuild_readonly_review.md` | regular file | 1763 | `0644` | `a873c9228fb8b38d643b854b74bc45269f579196cadd422d3dc198414b482b98` |
| `/private/tmp/vv-pk-reuse-integration.PohVG2/worktree/results/stage1_multibatch/gate-a-20261117-08/synthetic-reject/gate_c.json` | regular file | 127 | `0644` | `837d06b76baec6057e8f0a0dec5d96d175e87c5ba527361e4363a410ba1db923` |

真实 A-08 raw 目录、manifest 和 Gate C 授权文件均未创建。既有 results/A-08 目录只保留上述三份准备文件；不把它称为已建立 Gate A-08。

## 2. 源码与构建指纹

- Runner: `/private/tmp/vv-pk-reuse-integration.PohVG2/worktree/contracts/scripts/stage1_multibatch_compare.mjs`; 93682 bytes; mode `644`; SHA-256 `c8ef4870b7c1fce95979c23a420df6041d004e913f8a1c0fcfaf5e931602c3fa`
- C source: `/private/tmp/vv-pk-reuse-integration.PohVG2/worktree/contracts/scripts/native/vv_safe_fs_helper.c`; 10250 bytes; mode `644`; SHA-256 `ee5800375de0bf6336e48eb8840577a950cc08649bbd2941546d5af88e8b15eb`
- C binary: `/private/tmp/vv-pk-reuse-integration.PohVG2/worktree/contracts/scripts/native/vv_safe_fs_helper`; 35072 bytes; mode `755`; SHA-256 `bca16b634c6bf78599ea83d0cd66737536a7190a8bec4ae99d066e2bcee448a5`

架构：`arm64`。编译器实测：

```text
Apple clang version 17.0.0 (clang-1700.4.4.1)
Target: arm64-apple-darwin24.6.0
Thread model: posix
InstalledDir: /Library/Developer/CommandLineTools/usr/bin
```

实际 C 编译命令（两次工程构建，第二次为最终二进制；没有 Rust/Solidity 编译）：

```sh
cc -std=c11 -Wall -Wextra -Werror -O2 contracts/scripts/native/vv_safe_fs_helper.c -o contracts/scripts/native/vv_safe_fs_helper
node --check contracts/scripts/stage1_multibatch_compare.mjs
GIT_OPTIONAL_LOCKS=0 git diff --check
node /private/tmp/vv-a08-path-guard-test-UtjDki/test.mjs /private/tmp/vv-a08-path-guard-test-UtjDki
```

C 编译、Node 语法检查和 diff --check 实际退出码均为 0。初次测试后修正了受控可执行文件归档路径在“锚点已存在/尚不存在”两种情况下的匹配，并补上日志辅助进程启动失败时关闭 stdin/回收的处理；随后重新编译、更新固定二进制哈希，在新的独立目录重新执行全部 13 项。初次证据保留于 `/private/tmp/vv-a08-path-guard-test-Q9X1St`，不冒充最终版本测试。

## 3. P0 门禁与 C dirFD 验收

1. 五项环境变量先只读验证；RUSTUP_HOME 的物理目录明确列为只读，不进入六根白名单。不存在任何自动安装/复制依赖动作。
2. NFC 组件比较、Darwin 保守大小写折叠、realpath 物理归属与逐级 lstat 联用。realpath 不是写权限；`/tmp` symlink 一律不能作为写路径。新建组件只接受安全 ASCII。
3. 原主工作区全树、集成 A-07/B-03 raw/results、共享 node_modules 物理目标、原 Cargo home 与冻结材料目录均禁止写入。缓存根不得落入集成源码树。
4. 六个根两两校验；只允许 VV_INTEGRATION_TARGET_DIR 与 CARGO_TARGET_DIR 完全相同。TMPDIR 必须为独立 /private/tmp 子目录。目标存在则 EEXIST。所有门禁在首次 mkdir/file write 前完成。
5. C `anchor_open()` 从 `/` 开始，每个组件逐级 `openat(O_DIRECTORY|O_NOFOLLOW)`、fstat；最终锚点 fstat 严格核对 Runner 登记的设备和 inode。PROD 再检查 registered-root 来自固定 A-08 根/已登记环境根，target 在该根内，allowed-root 是最近存在前缀，不把 CLI 路径本身当作授权。
6. C intermediate mkdirat 为 0700，再 openat/no-follow/fstat。文件 openat 为 O_CREAT|O_EXCL|O_NOFOLLOW|O_WRONLY,0600；fsync(file) 与 fsync(parent) 均实调。仅受控归档可执行文件在同一文件 FD 上 fchmod(0700)，避免原 copyFile 路径被收口后丢失可执行权限；普通日志/JSON 保持 0600。该补充在模拟文本文件上验证，未启动任何 Worker。
7. 每次同步/流式启动辅助程序均校验 regular/non-symlink/executable/full SHA-256，与 Runner 固定值完全一致。TEST_ONLY 临时根与 pause 参数隔离；PROD 拒绝测试根和任何 pause 参数。

正常新 A-08 文件通过已存在 raw 父目录锚点：dev=`16777234`, inode=`64577647`；native PARENT_OPEN/WRITTEN 回传身份逐项一致，完整记录见 test_results.json。

## 4. Runner 全部受保护输出入口

检索已确认 `fs.writeFileSync/appendFileSync/mkdirSync/createWriteStream/renameSync/writeSync/writeFile/appendFile/mkdir/rename/copyFileSync` 调用为零；两个 fs.openSync 仅使用 `r` 读取哈希/冻结 PK 前缀。终端 stdout/stderr 的输出不是 Runner 创建的受保护磁盘文件。所有 JSON 包装、日志流、清理标记和二进制归档都由同一个 safeWriteProtectedFile 调用 C；旧历史分支即使被误调用，其输出也被当前只读黑名单拒绝。

| Runner 行号 | 写入入口及目标表达式 | 安全接口 |
| ---: | --- | --- |
| 136 | `export function safeWriteProtectedFile(target,data,{stream=false,onReady,...options}={}){` | 统一接口定义（sync execFileSync / stream spawn） |
| 155 | `export const writeNew=(p,v)=>safeWriteProtectedFile(outputAbsolute(p),json(v));` | safeWriteProtectedFile → C dirFD |
| 429 | `out=safeWriteProtectedFile(base+'.stdout.log',undefined,{stream:true,env});` | safeWriteProtectedFile → C dirFD |
| 430 | `err=safeWriteProtectedFile(base+'.stderr.log',undefined,{stream:true,env});` | safeWriteProtectedFile → C dirFD |
| 453 | `const marker=path.join(materialsDirectory,'outer_cleanup_requested.json');if(!fs.existsSync(marker))safeWriteProtectedFile(marker,json({reason:'OUTER_WALL_BUDGET',outer_pid:child.pid}));` | safeWriteProtectedFile → C dirFD |
| 474 | `stdout:base+'.stdout.log',stderr:base+'.stderr.log'};writeNew(base+'.command.json',r);` | writeNew → safeWriteProtectedFile → C dirFD |
| 499 | `const before=protectedSnapshot({phase:'A',run});writeNew(run+'/protected_files.json',before);` | writeNew → safeWriteProtectedFile → C dirFD |
| 507 | `const reproducibility=binaryReproducibility(run);writeNew(run+'/binary_reproducibility.json',reproducibility);` | writeNew → safeWriteProtectedFile → C dirFD |
| 512 | `const compiled=compileSolidity();writeNew(run+'/compiled.json',compiled);` | writeNew → safeWriteProtectedFile → C dirFD |
| 513 | `writeNew(run+'/storage_layout.json',{layouts:Object.fromEntries(Object.entries(compiled.contracts['stage1/Stage1Receivers.sol']).map(([n,c])=>[n,c.storageLayout])),checks:compiled.storage_checks});` | writeNew → safeWriteProtectedFile → C dirFD |
| 514 | `writeNew(run+'/static_checks.json',{version,prague:pragueStatic(),storage_checks:compiled.storage_checks,provenance:inputProvenance(),` | writeNew → safeWriteProtectedFile → C dirFD |
| 521 | `writeNew(run+'/gate_a.json',{...base,status:'PASS_GATE_A_COMPILE_ONLY',commands,` | writeNew → safeWriteProtectedFile → C dirFD |
| 527 | `}catch(e){writeNew(run+'/gate_a_failed.json',{...base,status:'BLOCKED',error:String(e),commands});throw e;}` | writeNew → safeWriteProtectedFile → C dirFD |
| 600 | `writeNew(aRun+'/preflight_b.json',report);writeNew(resultsPathForRun(aRun)+'/preflight_b.json',report);` | writeNew → safeWriteProtectedFile → C dirFD |
| 766 | `writeNew(run+`/sessions/${negative?'negative':'positive'}-k${k}-${route}.json`,output);assert.equal(output.status,'PASS',output.error);` | writeNew → safeWriteProtectedFile → C dirFD |
| 777 | `const before=protectedSnapshot({phase:gate,run,aRun,bRun});writeNew(run+'/protected_files.json',before);` | writeNew → safeWriteProtectedFile → C dirFD |
| 779 | `writeNew(run+'/request.json',{gate,identity:`${gate}/${path.basename(run)}/${new Date().toISOString()}`,config_sha256:sha(read(configPath)),a_run:aRun,b_run:bRun??null,approved,formal:gate==='C'});` | writeNew → safeWriteProtectedFile → C dirFD |
| 790 | `checkProtected(before);writeNew(run+`/gate_${gate.toLowerCase()}.json`,{gate,run,a_run:aRun,b_run:bRun??null,status:gate==='B'?'PASS_GATE_B_REGRESSION_ONLY':'PASS_GATE_C_FORMAL_EXECUTION',` | writeNew → safeWriteProtectedFile → C dirFD |
| 843 | `writeNew(run+'/integration_config.json',{schema:'VV/STAGE1/A07/PK-REUSE/v1',...admission,budgets:reuseBudgets,` | writeNew → safeWriteProtectedFile → C dirFD |
| 852 | `if(path.resolve(built)!==absolute(binary))safeWriteProtectedFile(absolute(binary),fs.readFileSync(built),{executable:true});` | safeWriteProtectedFile → C dirFD |
| 859 | `writeNew(run+'/gate_a.json',{schema:'VV/STAGE1/A07/PK-REUSE/v1',status:'PASS_GATE_A07_PK_REUSE_BUILD',run,` | writeNew → safeWriteProtectedFile → C dirFD |
| 870 | `const frozen=integrationFrozen();writeNew(run+'/integration_request.json',{gate,run,a_run:aRun,run_identity:`${path.basename(run)}/${new Date().toISOString()}`,` | writeNew → safeWriteProtectedFile → C dirFD |
| 874 | `writeNew(run+'/compare_summary.json',{schema:'VV/STAGE1/A07/INTEGRATION/v1',status:'PASS_MATERIAL_GENERATION_ONLY_PENDING_AUDIT',` | writeNew → safeWriteProtectedFile → C dirFD |
| 948 | `safeWriteProtectedFile(absolute(binary),fs.readFileSync(path.join(target,'release/stage1_multibatch_materials')),{executable:true});` | safeWriteProtectedFile → C dirFD |
| 949 | `const compiled=compileSolidity();writeNew(A08_RUN+'/compiled.json',compiled);` | writeNew → safeWriteProtectedFile → C dirFD |
| 959 | `checkReadonlyInputs(frozenPaths);writeNew(A08_RUN+'/manifest.json',manifest);verifyA08();` | writeNew → safeWriteProtectedFile → C dirFD |
| 966 | `writeNew(run+'/request.json',{gate:'C',a_run:A08_RUN,identity:'C/'+path.basename(run)+'/'+new Date().toISOString(),evidence_kind:'REAL_EXECUTION',execution_performed:true,` | writeNew → safeWriteProtectedFile → C dirFD |
| 974 | `verifyA08();checkReadonlyInputs(m.frozen_paths);writeNew(run+'/gate_c.json',{status:'PASS_GATE_C_FORMAL_EXECUTION',gate:'C',run,a_run:A08_RUN,evidence_kind:'REAL_EXECUTION',execution_performed:true,` | writeNew → safeWriteProtectedFile → C dirFD |

命令日志在启动业务子进程之前先验证 stdout、stderr、command.json 三个目标，流式日志由持有文件 FD 的 C 进程持续落盘，不在 Node 中直接 open/write。Rust/Cargo 等外部程序自身的文件写入不是这份 Runner 收口的同义词；后续环境准备与构建仍需独立批准，本轮没有运行这些程序。

## 5. 十三项真实文件系统测试

| 项 | 场景 | 预期 | 实测 |
| ---: | --- | --- | --- |
| 1 | new A08 via existing parent inode; duplicate and streaming | 新文件成功，重复 EEXIST | PASS |
| 2 | original A06 read-only | 按规格拒绝；失败前后零变化 | PASS |
| 3 | original B02 read-only | 按规格拒绝；失败前后零变化 | PASS |
| 4 | integration A07 read-only | 按规格拒绝；失败前后零变化 | PASS |
| 5 | integration B03 read-only | 按规格拒绝；失败前后零变化 | PASS |
| 6 | /tmp alias denied for both approved and historical roots | 按规格拒绝；失败前后零变化 | PASS |
| 7 | actual filesystem case variants | 按规格拒绝；失败前后零变化 | PASS |
| 8 | NFC/NFD actual inode equivalence and non-ASCII new component | 按规格拒绝；失败前后零变化 | PASS |
| 9 | symlinks to history and unprotected intermediate directories | 按规格拒绝；失败前后零变化 | PASS |
| 10 | existing preparation file protected by EEXIST | 按规格拒绝；失败前后零变化 | PASS |
| 11 | environment admission, all 15 nested pairs, PROD isolation and dev/ino | 按规格拒绝；失败前后零变化 | PASS |
| 12 | all unsafe-path failures produced no output, logs or command receipts | 按规格拒绝；失败前后零变化 | PASS |
| 13 | same-UID rename after verified directory FD open | 原 inode 写入 moved_dir，trap 为空 | PASS |

全部具体输入、期望拒绝码、真实 stderr、零写入目录快照断言与原生回传记录见 `test_results.json`；测试仅使用本目录下模拟 A-06/B-02/A-07/B-03/A-08，未向真实历史或 A-08 写测试文件。

### 大小写及 NFC/NFD 实测

- `/private/tmp/vv-a08-path-guard-test-UtjDki/sim/case/gate-a-20261117-06`：dev=`16777234`, inode=`64577657`, `READ_ONLY_PATH`
- `/private/tmp/vv-a08-path-guard-test-UtjDki/sim/case/Gate-A-20261117-06`：dev=`16777234`, inode=`64577657`, `READ_ONLY_PATH`
- `/private/tmp/vv-a08-path-guard-test-UtjDki/sim/case/GATE-A-20261117-06`：dev=`16777234`, inode=`64577657`, `READ_ONLY_PATH`
- NFC `/private/tmp/vv-a08-path-guard-test-UtjDki/sim/unicode/é` 与 NFD `/private/tmp/vv-a08-path-guard-test-UtjDki/sim/unicode/é`：相同 inode=`64577659`，same_inode=true；8a 拒绝码 `READ_ONLY_PATH`，8b 新非 ASCII 组件拒绝码 `NON_ASCII_NEW_COMPONENT`。

第 11 项包含：5 项环境变量各自缺失/相对路径、RUSTUP 写入、两种宽泛 TMPDIR、全部 15 组父子重叠、PROD 测试参数、PROD 临时根、错误设备和错误 inode；每一子项均比较整个模拟树前后条目/类型/mode/文件 SHA，零新增/改写。

### 目录重命名竞态实证与边界

- C PARENT_OPEN 握手后才执行 rename 和原路径 symlink 替换；parent inode=`64577681`，moved inode=`64577681`，trap entries=`[]`。
- 文件确实位于原 inode 的 moved_dir，未跟随新 symlink 到 trap。这证明 dirFD 固定原对象，而不是阻止 same-UID 移动对象。
- 逐级 O_NOFOLLOW 阻止符号链接路径遍历；st_dev/st_ino 固定锚点身份。但不宣称抵御全部同 UID 并发文件系统攻击。
- Runner SHA-256 校验到 execFileSync/spawn 执行之间仍有理论上的同 UID 可执行文件替换窗口；没有声称消除此风险。
- 预检拒绝的零写入已测试；这不代表写入过程发生 I/O/fsync 故障时具有文件系统事务回滚能力。

## 6. 冻结历史与既有文件完整性

原主工作区 HEAD、index SHA、完整 binary diff、status、141 个未跟踪文件内容和历史清单均与既存 baseline 完全一致；集成 HEAD/index、四份冻结归档、Cargo 清单/锁文件与真实 A-08 三份准备文件亦未变化。全部比较由既有只读 check.mjs 执行，前后完整输出保存在本测试目录，不改历史证据。

| 范围 | 路径 | 前后 SHA-256 / 存在状态 |
| --- | --- | --- |
| 原主工作区 | `experiments/stage1_multibatch_raw/gate-a-20261117-01` | `9fca95613604bc85299ed2691723cc3d0639618ccec87d3ba1f2599cfc589257`（一致） |
| 原主工作区 | `results/stage1_multibatch/gate-a-20261117-01` | `NOT_PRESENT`（一致） |
| 原主工作区 | `experiments/stage1_multibatch_raw/gate-a-20261117-02` | `26db2668ea46a605821f67e6154b2b4eeb5775aa22ad7906e778594ee14fcdc6`（一致） |
| 原主工作区 | `results/stage1_multibatch/gate-a-20261117-02` | `c748f58d4b417e644c75bae7fe6f47aca6c688bf37c5eb3074d313108783a54f`（一致） |
| 原主工作区 | `experiments/stage1_multibatch_raw/gate-a-20261117-03` | `5d12a6236ee84e2bcc96072cd086b9d424ece88165f0a9ed74cbc9e9f017e413`（一致） |
| 原主工作区 | `results/stage1_multibatch/gate-a-20261117-03` | `ab110b069bb33d62989526ab2ddba62da7622573028319c103cc7656d153c2c3`（一致） |
| 原主工作区 | `experiments/stage1_multibatch_raw/gate-a-20261117-04` | `663a2d09bf152b3ff04ab6b6a7f91d4baacb5aa0af74633022922e4c8bae9f90`（一致） |
| 原主工作区 | `results/stage1_multibatch/gate-a-20261117-04` | `c398fb112dcbbb891ecb9d9f35a6616235c2eaa700232289c3872a87f9497bea`（一致） |
| 原主工作区 | `experiments/stage1_multibatch_raw/gate-a-20261117-05` | `a73d794c95d1d4c31d9f3a06b1c9f46b210f81c1d0ba5e39d0926bc743b6a72c`（一致） |
| 原主工作区 | `results/stage1_multibatch/gate-a-20261117-05` | `c6eb5a7ed997f24566ecf8d00d019008d3cf233d492591ec056cf4014928bdce`（一致） |
| 原主工作区 | `experiments/stage1_multibatch_raw/gate-a-20261117-06` | `6ab6fdca6b239b59d792770bb523060ec48b7ad8d48ba9f0ac7da77cd4f3b8b0`（一致） |
| 原主工作区 | `results/stage1_multibatch/gate-a-20261117-06` | `1910bc87adfaa3722c7996bc85d04b0f647e7b88db0c46c25e1d545228c19f97`（一致） |
| 原主工作区 | `experiments/stage1_multibatch_raw/gate-b-20261117-01` | `99e00d934b2d23e8d3c61c7abe6bbeca541ee5efdbd36e62192431c45526c85e`（一致） |
| 原主工作区 | `results/stage1_multibatch/gate-b-20261117-01` | `NOT_PRESENT`（一致） |
| 原主工作区 | `experiments/stage1_multibatch_raw/gate-b-20261117-02` | `dced9d6123a7664b00583db73a8a36a288b58cdb2054d2fe9f6348b59268f10a`（一致） |
| 原主工作区 | `results/stage1_multibatch/gate-b-20261117-02` | `60d4ed7290422ac9740e18eb1edf999caf813c568e457898aaf4bfa016120dc8`（一致） |
| 集成工作区 | `experiments/stage1_multibatch_raw/gate-a-20261117-07` | `49497a86b79a854b7b7f563e7337a7418205ff13f71dddecc0c38af02d6c89d7`（一致） |
| 集成工作区 | `experiments/stage1_multibatch_raw/gate-b-20261117-03` | `a93213f5e1a755e5caa99413468d5bab7699985081799f6981762233dd08c38a`（一致） |
| 集成工作区 | `results/stage1_multibatch/gate-a-20261117-07` | `a884ae0a874389b80dfa1202a01d926d2556899577000e22492cf04a88608051`（一致） |
| 集成工作区 | `results/stage1_multibatch/gate-b-20261117-03` | `26d452dde9869f829f02e502b1a505d444c2111d300dc2622dffceae14eab4d8`（一致） |

最终完整性指标：

```json
{
  "original_matches_historical_baseline": true,
  "original_16_paths": 16,
  "integration_4_paths_unchanged": true,
  "existing_three_preparation_files_unchanged": true,
  "other_three_preexisting_modified_sources_unchanged": true,
  "head_unchanged": true,
  "index_unchanged": true
}
```

## 7. 停机状态

```json
{
  "A06_B02": "FROZEN",
  "A07_B03": "FROZEN",
  "A08": "NOT_CREATED",
  "C": "NOT_AUTHORIZED_NOT_STARTED",
  "D": "NOT_STARTED"
}
```

没有 Rust/Solidity 编译、Setup、PK/VK 生成、证明、Worker、EVM、依赖复制、Gate C/D、commit 或 push。保持停止，等待阶段 3 单独授权。本报告不授权下一阶段。

证据：test_results.json、write_inventory.json、identity.json、preparation_files_before/after.json、integrity_before/after.json、final_checks.json、this_turn_runner.diff（相对于本轮前已归档源码），runner_git_diff.patch（相对于 HEAD，包含以前已有修改）。全部文件属于临时测试归档，未写真实 A-08。

