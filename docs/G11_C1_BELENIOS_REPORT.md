# G11-C1 — Belenios 3.0 原始实现复现

状态：**PASS_BELENIOS_CLI_REPRODUCTION**。执行日期：2026-11-06。

本轮只运行 Belenios，未启动其他外部方案、G9、VeriVoting 新证明或 EVM 交易。
未修改论文、Solidity、原密码学库或 G1–G10 历史数据。

## 1. 已取得的证据

- 官方 `tests/tool/demo-threshold.sh` 未修改脚本，真实通过：选票、重投、差分审计、门限解密、结果审计。
- 独立适配器完成 N=8/32/128 各 5 次预热、30 次正式测量，共 **105 场不同 UUID 的独立选举**。
- 每场重新生成凭证、门限密钥和随机化选票；合计生成并验证 **5,880 张**加密选票，其中正式样本为 5,040 张。
- 105/105 场最终加权计票均与合成工作负载逐项相同，无失败正式样本。
- N=8 独立 pilot 的两个必要负例均被官方 CLI 拒绝：改变选票证明响应得到 `InvalidBallot`；缺少服务器部分解密得到 `a partial decryption is missing`。均为正常 exit 124，不是超时或进程信号。
- 所有公开选举 `.bel` 档案、逐命令日志、原始 JSON、统计 CSV 和安装来源均已保存。

第一次 pilot 因适配器给逐行解析器提供了多行 JSON 而失败。原日志保留在
`experiments/g11/baselines/belenios/pilot-20261106-01`；修正输入序列化后，
`pilot-20261106-02` 通过。首次失败不作为密码学负例，也不纳入统计。

## 2. 官方来源与版本固定

官方发布页：<https://www.belenios.org/software.html>。
官方 3.0 源码：<https://www.belenios.org/releases/belenios-3.0.tar.gz>。
源代码镜像：<https://github.com/glondu/belenios/tree/3.0>。
本轮使用 tag 3.0 中的 `doc/tool.md`，不是后来 master 的文档。

| 对象 | 固定值 |
| --- | --- |
| Belenios tag / 软件版本 | `3.0`；CLI 输出 `3.0 (3.0)` |
| Commit | `5a1596cefcc0944467ddc4bc9cdb0f24013b0d05` |
| 源码 tar SHA-256 | `6e9fed14ccf849632b84570cbe852bea3cbe30b5c9baf6a4a3cad7e7d16c62f9` |
| Release CLI SHA-256 | `ad43e6fb93d447e516210ba5ae24144868ad290a53b8872e097acaeba130521c` |
| 实验配置 SHA-256 | `3c6fdf660523933cbe888880678b3eb3d6fac36d95757cdd40ddb6b8426f5e5a` |
| 实际运行适配器 SHA-256 | `44b7357fc232414a05736e58b02e2dd481efa2f87bb869539ff092433b115ec2` |
| VeriVoting 基准提交 | `f18833db40895bc8918b806c32ac35ec167cca0d` |
| 分支 | `experiment/g11-comparative-evaluation`，远程 SHA 已只读核对 |

官方 tar 与 Git tag 的 **649 个受跟踪文件**逐字节相同；符号链接按链接文本比较。
本轮 Belenios 受跟踪源码未修改。软件 3.0 与当前 G11 文献版本一致；
选举 JSON 内部 `version=1` 是其协议格式编号，不是软件 1.0。

许可证：AGPL-3.0-or-later；上游源文件含 OpenSSL 链接例外。
`COPYING.belenios.txt`、完整依赖版本和 opam export 已归档。

## 3. 构建条件和必要兼容处理

机器：Apple M4，10 核，32 GiB RAM，macOS 15.7.1，arm64；Node 23.11.0。
OCaml 4.14.2、Dune 3.16.0；官方冻结 opam 仓库提交
`4093222dece7c1c8f93dc2fd8dd7eec15fa80fed`。

工具链与原生依赖均在隔离目录构建，没有安装全局软件包或改写 shell 配置。
本轮使用 opam 2.6.1 二进制，而上游 bootstrap 使用 2.2.1；这一包管理器差异已记录。
GMP 6.3.0、libsodium 1.0.20、pkgconf 2.3.0、zlib 1.3.1 均有来源与哈希。
全部 89 项已安装包见 `installation-20261106/dependencies-final.txt`。

Apple Clang 17 下，通用依赖 `base` v0.16.3 的探测会错误保留 x86 的
`-mpopcnt`。补丁只将该 CPU 选项探测限制到 x86，未修改密码学算法。
补丁及 MIT 许可证见 `scripts/g11/c1/`。
还补足了上游实际引用的 `hex` 和 `cohttp-lwt-unix` 依赖。
网络重试、缺失依赖和不成功构建的日志均保留，不计入选举性能。

实际成功的最终构建命令（已设置隔离 include/library/pkg-config 路径）：

```sh
/private/tmp/vv-g11c1-20261106-K03bsm/opam exec \
  --root=/private/tmp/vv-g11c1-20261106-K03bsm/opam-root --switch=4.14.2 -- \
  dune build --profile release -j2 \
  -p belenios-platform,belenios-platform-native,belenios-lib,belenios-tool
```

官方 demo 固定调用 `_run/tool-debug/bin/belenios-tool`。本轮只把该路径链接到
上述 Release CLI；没有修改 demo，也没有将 debug 结果当作 Release 性能。

## 4. 实验配置与比较边界

- 一个问题、三个候选人、必须选择且只选择一个、无空白票；注册整数权重为 1..3。
- 官方 Ed25519 群、原生凭证与非交互式选票证明；未替换为 Jubjub/Groth16。
- 使用官方门限 demo 的组合方式，但扩展为 **3-of-5 Pedersen + 一把必须参与的服务器密钥**。
  选中的门限 Trustee ID 为 2/3/4，服务器 ID 为 1。
  这不是 VeriVoting 的纯 3-of-5 配置；也不表示所有 Belenios 配置都必须如此组合。
- Belenios 将权重关联到注册凭证，并在累计中使用；没有移植 VeriVoting 的加权选票电路。
- Trustee CLI 由本地运行器顺序调用，不是六台独立机器，不测网络、HTTP 服务、私密交付和故障恢复。
- 公告板使用官方 append-only `.bel` 文件和原生命令，不是新写的服务端模拟。
- 工作负载种子只确定合成权重和选择。`BELENIOS_USE_URANDOM=1` 提供新密钥、凭证和选票随机性。
  每轮轮换 N 的执行顺序，所有预热在正式样本之前完成；不是始终从小到大运行。
- `workload.json` 明确公开合成明文选择，便于检查计票。它不是隐私证明实验，更不能用于真实选举。
  私有凭证、Trustee 私钥、原始敏感 stdout 均留在仓库外的 0700 临时目录。

## 5. 时间、通信与内存口径

每条官方命令的单调时钟 wall time、exit code、stdout 摘要、stderr 及
`/usr/bin/time -l` 数据保存在 `commands.jsonl` 和 `stderr.log`。

| 指标 | 本轮实际含义 |
| --- | --- |
| `setup_ms` | 凭证、门限/服务器密钥、参数及初始公告板命令耗时总和 |
| `ballot_generation_ms` | 全部 `generate-ballot`：加密 + 证明生成 + 命令内部有效性校验；含 CLI 启动和档案读取 |
| `verification_ms` | 全部独立 `verify-ballot` 的总和；每票接收前执行，含当前档案加载 |
| `archive_ballot_audit_ms` | 投票完成后的整场 `election verify`；不同于逐票验证总和 |
| `tally_ms` | `compute-encrypted-tally`，含原生命令需要的加载与检查，不是纯群加法微基准 |
| `decryption_ms` | 三份门限解密 + 一份服务器解密 + `compute-result`，不含份额事件追加 |
| `end_to_end_ms` | 从构造工作负载到最终结果审计完成的本地顺序流程，含 setup、接收、各次审计和运行器 I/O；不含下载、构建、最终归档复制 |
| `communication_bytes` | 完整公开 `.bel` 档案字节数，含 tar/event 结构与初始参数；不是实际网络流量或全部私密通信 |
| `proof_bytes` | 选票 proof/signature 子对象的 JSON 编码大小之和；不是可直接传给别的系统的独立证明格式 |
| `ciphertext_bytes` | 各选票 `choices` 密文数组的 JSON 编码大小之和 |
| `memory_peak_mib` | 所有顺序 CLI 子进程 RSS 峰值的最大值，不是多个节点或运行器内存之和 |

纯加密时间、单独证明生成时间不可从未修改的 CLI 公平分离，保持 `null`。
没有 Groth16 式公开输入向量；相应字段保持 `null`。
Gas/calldata/链上交易数不适用，均为 `null`，不是“0 Gas”。
CPU 字段是逐命令低分辨率计时的和，10 ms 舍入可能显著低估短命令，不能用来得出 CPU 效率结论。
证明与密文子对象大小也不能不加说明地当作完整通信量。

## 6. 正式结果

以下为每个规模 **30 场正式样本的均值**，不含 5 场预热、官方 demo、pilot 或安装。
时间为 ms；`end-to-end` 另给样本标准差（n−1）。CSV 保留均值、中位数、标准差、min/max、p95 和样本数。

| N | Setup | 生成全部选票 | 逐票验证总和 | 同态累计 | 解密/组合 | 完整流程 mean ± SD | CLI peak RSS (MiB) | 公开档案 (bytes) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 8 | 167.55 | 121.76 | 114.48 | 14.86 | 71.08 | 670.52 ± 23.77 | 10.30 | 67,584 |
| 32 | 168.06 | 502.76 | 525.01 | 23.80 | 76.04 | 1,716.67 ± 46.68 | 11.32 | 167,424 |
| 128 | 175.65 | 2,238.97 | 3,181.45 | 61.21 | 92.07 | 7,337.44 ± 185.12 | 13.25 | 567,296 |

完整结果审计的均值分别为 37.62 / 82.85 / 260.25 ms。
这些是官方本地 CLI 应用流程的测量，不能与现有 VeriVoting 进程内微基准直接画排名图。
本轮没有重新运行匹配工作负载下的 VeriVoting，不声称任一方案更快。

## 7. 文件入口与复现

- `experiments/g11/baselines/belenios/installation-20261106/`：源码与依赖来源、官方 demo、原始安装日志。
- `experiments/g11/baselines/belenios/run-20261106-01/`：105 份原始样本、工作负载、公开档案和逐命令日志。
- `results/g11/c1/samples.csv`：全部样本；使用 `phase=measured` 选择正式结果。
- `results/g11/c1/summary.csv`：各 `voters` 下的统计；不可用指标的统计值为空。
- `results/g11/c1/analysis.json`：验收计数和 CSV 回读校验。
- `SHA256SUMS`：各新数据目录分别封存；旧 G11 的 manifest 不修改。

实际执行的正式实验命令（在 VeriVoting 仓库根目录）：

```sh
node scripts/g11/c1/run.mjs --suite \
  --source /private/tmp/vv-g11c1-20261106-K03bsm/belenios-git \
  --bin /private/tmp/vv-g11c1-20261106-K03bsm/belenios-git/_build/install/default/bin/belenios-tool \
  --out experiments/g11/baselines/belenios/run-20261106-01 \
  --private-root /private/tmp/vv-g11c1-20261106-K03bsm/private-suite-01
node scripts/g11/c1/check.mjs experiments/g11/baselines/belenios/run-20261106-01
node scripts/g11/c1/summarize.mjs \
  --raw experiments/g11/baselines/belenios/run-20261106-01 --out results/g11/c1
node scripts/g11/check.mjs
```

重新安装时，使用全新的绝对缓存目录，不要覆盖本次数据：

```sh
bash scripts/g11/c1/install-macos-arm64.sh /private/tmp/vv-belenios-reproduction-new
```

该安装配方已通过 `bash -n`，等价的安装步骤已真实执行；没有为了验收再次从空缓存完整重装一次。
随后使用 `--pilot` 跑一场，或 `--suite` 跑完整 105 场；为 `--out` 和 `--private-root` 指定新的路径。
脚本不覆盖已有样本。清理临时目录前，请注意其中包含复现用的 CLI 二进制和私密实验密钥。

独立审计不需要任何私钥：

```sh
/private/tmp/vv-g11c1-20261106-K03bsm/belenios-git/_build/install/default/bin/belenios-tool \
  election verify --dir experiments/g11/baselines/belenios/run-20261106-01/n128-measured-00
```

统计器默认使用表格运行时回读/预览；只安装 Node 的机器可在新输出目录加
`--plain-csv`，以相同统计公式导出 CSV，而不依赖专用渲染模块。

## 8. 验收边界与下一步

已完成 Belenios 官方原型的凭证、选票/证明、验证、公告板接收、同态累计、门限解密及审计复现。
尚未完成：真实网络服务部署、公平跨系统性能对照、第二外部方案、Winner-only 外部性能复现。
本轮不改变 G9、独立 DKG 部署或 T4/T5 安全证明的既有状态。

下一轮应先统一 VeriVoting 与本基线的工作负载和应用层计时边界，并明确门限参与方与结果披露差异，
再决定第二个可复现基线。不应把这些 CLI 时间直接与旧的进程内证明验证时间比较，也不应预设性能优势。
本轮完成后停止，不自动启动第二方案或改写论文。
