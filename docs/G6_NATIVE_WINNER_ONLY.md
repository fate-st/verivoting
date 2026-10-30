# G6：完整原生 Winner-only 状态机集成

2026-10-30 实际执行通过：`PASS_LIVE_RECEIVER_FREEZE_PRIVATE_DECRYPT_P3_WINNER_ONCE`。
本轮止于原生功能验收，未进行合约开发、额外批次规模或重复性能采样。

## 1. 现在真正连通的流程

```text
重建并认证同一 D5/W32 DKG / 注册关闭记录
    -> G2-E Receiver 实际验证 B4 #1 -> CT1
    -> 同一 Receiver 实际验证 B4 #2 -> CT2
    -> 一次性冻结接收器中的 CT2
    -> 五份新生成的私密 CP/承诺打开包，各自原生验证
    -> 选定 {1,2,3}，私密门限插值与有界解码
    -> 原 G4-D P3 v2 关系，一份新 Groth16 证明
    -> 对照冻结存储重建全部 60 个公开输入并真实验证
    -> winner=1，只写入一次
```

不是把两个历史结果 JSON 合并后宣布成功：运行期间两个 SnarkPack 验证器、两个状态证明验证器、
五份新 CP/签名私包验证和一份新 P3 的 Prove/Verify 都实际执行。
历史 CT2 仅作回归比对；冻结目标取自新建 receiver 顺序接收后的内存状态。

入口：[g6_native_winner_only.rs](../crates/vv-friendly-dkg/src/bin/g6_native_winner_only.rs)。
G6 专用适配器复用原有算法；原 G2-E Receiver、G4-D 关系/runner、密码学库与冻结配置字节未改。
`vv-friendly-dkg/Cargo.toml` 仅注册新入口，没有改变依赖或曲线。

## 2. 每层绑定与写入条件

- **接收：**记录必须是同一 D2 前八票的准确有序公开输入；重新编码并核对矩阵和 transcript。
  Receiver 检查选举/公钥上下文、注册根、固定批次根、next、old-state、重复/spent nullifier，
  两个真实证明家族都通过后才写入状态。两批分别使 spent=4/8、next=2/3。
- **冻结：**仅在两批接收完成后允许。封存 election ID、pre-policy、DKG session/key-core/公钥、
  final context、注册根/关闭摘要、disclosure 摘要、CP target、W32、候选人顺序、两个批次根、
  接收计数、P3 VK 以及固定 `WINNER_ONLY_V1` / tie-break 规则。再次冻结或追加批次均拒绝。
- **私包：**冻结后才生成五份新 generalized-CP 包及 Package-H2F/Pedersen 承诺，原生验证 CP、
  打开和认证签名；状态机还核对五个签名公钥/verification share 与认证 DKG roster 一致。
  私包、打开和解码 tally 只作为同进程私有对象传递，不写入公开文件。
- **P3 与公布：**选用 G4-D 同一通用关系中的 selector=0，即 `{1,2,3}`。winner 由本次私密解码
  与既有规则计算，不由历史 JSON 或硬编码赢家填入见证。公布方法从冻结存储和已认证承诺重建
  全部 60 个输入；准确匹配且真实 Groth16 验证通过，才写 winner。此后同一公布调用拒绝。

冻结 seal 是新增的**原生接收层绑定**，不是给 P3 新增字段或重定义旧 CP transcript。
原 pre-policy、注册/披露摘要仍按原算法生成；G6 固定的 release policy 由该原生状态机和 seal 执行。
这不是新增的密码学安全定理，也不是链上认证与结算。

## 3. 复用与新增，分别记账

| 材料 | 复用 | 本轮新增 |
|---|---:|---:|
| 单票 Groth16 证明 | 8 | 0 |
| B4 SnarkPack V2 | 2 | 0 |
| B4 状态证明 | 2 | 0 |
| CP / 认证私包 | 0 | 5 |
| P3 v2 证明 | 0 | 1 |

旧 PK 未保存，所以本轮用原 G4-D seed 重建一次研究参数。完整 VK **逐字相等**，完整有序
R1CS A/B/C 指纹也相等；没有声称对未保存的历史 PK 做过逐字节比较。
新 G6 package/proof seeds 与 G4-D 不同，五个承诺和新证明 hash 也与历史案例不同。
PK 只在内存中存在，未保存。

## 4. 真实测量（单次功能验收）

环境：Apple M4 / 32 GiB，macOS 15.7.1 aarch64，Rust/Cargo 1.99.1，
Release `opt-level=1`，构建并发 2。不是默认 opt-level=3，也不是性能统计结论。

| 阶段 | 实测 |
|---|---:|
| 第 1 批 Receiver 检查/两个验证/应用 | 5.433209 ms |
| 其中 SnarkPack / State Verify | 3.539875 / 1.852334 ms |
| 第 2 批 Receiver 检查/两个验证/应用 | 5.744875 ms |
| 其中 SnarkPack / State Verify | 3.282250 / 2.434708 ms |
| 冻结 | 0.652167 ms |
| 五私包生成、同进程交付、验证及 roster 安装 | 33.162208 ms |
| 原生插值/有界解码及见证对象装配 | 1.962958 ms |
| R1CS 合成/满足性检查（另列） | 11228.113959 ms |
| 完整矩阵指纹（另列） | 4912.946584 ms |
| P3 Setup 重建 | 69021.922209 ms |
| 一份新 P3 Prove | 41178.248416 ms |
| 公布方法：输入检查、VK prepare、Verify、写入 | 6.861125 ms |
| 总运行时间，不含编译 | 131587.071042 ms |
| 累计进程峰值 RSS，含 Setup/Prove/指纹 | 8602.968750 MiB |
| Proof / P3 公开输入 / VK | 192 / 1920 / 3272 B |
| PK 压缩长度，仅测量未保存 | 1844946096 B |

“见证对象装配”不等于完整电路 witness 生成；R1CS 检查和 Prove 开销另列。
公布时间不是单独的 pairing Verify，也不与 G1-E warmed-operation RSS 或不同版本数据混合排名。
30 分钟 / 20000 MiB 资源预算未触发；外部命令实耗与采样监控见 `commands.json`。

## 5. 必要验收结果

- 两批均经过真实 SnarkPack V2 和 Groth16 State Verify，接收成功。
- 篡改第一坐标冻结密文的 c1，并据此重建 CP target / 公开输入：`Target` 拒绝，存储不变。
  这是接收层绑定测试，不是生成了错误见证的有效证明。
- 同一真实 P3 验证成功，最终 `winner=1`、`winner_write_count=1`。
- 重复披露：`Disclosed` 拒绝，结果与写入次数不变。
- 再次冻结、冻结后接收批次：`Frozen` 拒绝，未进入证明验证或改写 CT2。
- 45 项相关库测试 + 3 项接收/冻结测试通过；严格全 target Clippy、fmt check、Release build 通过。
  单元测试的 true/false 回调用于 bookkeeping；本轮真实运行和公开重放才是密码学执行证据。
- 另一进程仅载入公开参数、证明和固定公共信任锚，重新接收两批、冻结并验证新 P3 后写入 winner。
  零 Setup/Prove，未加载 Trustee 秘密或私包；不是仅校验结果 JSON 的布尔字段。
- 只读审计核对 19 份运行源码、25 项输入 pin、278 项历史数据/公开材料，确认未改；
  公共导出不存在 tally、可用解密份额、CP response、私包打开或私钥字段。

## 6. 原始产物与复现

- [完整执行 JSON](../experiments/g6_raw/native_winner_only.json)
- [公开 fixture](../fixtures/jubjub_g6_native_winner_only_public_v1.json)
- [另一进程公开重放](../experiments/g6_raw/public_replay.json)
- [实际命令与退出码](../experiments/g6_raw/commands.json)、[环境](../experiments/g6_raw/environment.json)
- [SHA256SUMS](../experiments/g6_raw/SHA256SUMS)、[审计脚本](../scripts/g6_audit.mjs)

运行基点：`9d8e73a2004473ed65571e08b46e14045f3679d6`；新增入口在执行时尚未提交，
raw 如实记录 `git_dirty=true` 和逐文件 hash，不能把基点当成包含新增代码的干净版本。

```bash
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 fmt --all -- --check
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 clippy -p vv-friendly-crypto -p vv-friendly-dkg --all-targets --release --locked --offline -- -D warnings
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 test -p vv-friendly-crypto -p vv-friendly-dkg --lib --release --locked --offline
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 test -p vv-friendly-dkg --bin friendly-g6-native-winner-only --release --locked --offline
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 build -p vv-friendly-dkg --bin friendly-g6-native-winner-only --release --locked --offline
RUSTUP_TOOLCHAIN=1.99.1 target/release/friendly-g6-native-winner-only
RUSTUP_TOOLCHAIN=1.99.1 target/release/friendly-g6-native-winner-only --verify-fixture fixtures/jubjub_g6_native_winner_only_public_v1.json --output experiments/g6_raw/public_replay.json
node scripts/g6_audit.mjs
```

原运行拒绝覆盖已有输出。重新运行应通过 `--output NEW_PATH --fixture-output NEW_PATH` 选择新路径；
公开重放的输出也应选择新路径。默认产物已归档，不需要再次重型执行才能阅读本次结果。

VK SHA-256：`0d37a77cfc8a1cdfed612b397149d96816188d164e08a74994dd2abfca6cff29`。
R1CS SHA-256：`e443da84ae3e636c232de16451bad8dd16f27e6fb6eb8f2097f7adcff7d7e2b0`。
冻结 seal：`05a2ce08ef1edc92398fb7990b5896387aa2efcef5c7d885c5fab676b7e338c0`。
公开 fixture SHA-256：`ca58fb6cd23258e1edbbdcd96ad7a5d5f93ca20b7af0ab7f8b330f7c02cbef78`。

## 7. 仍未完成及下一步

G6 完成的是**串行内存原型的一次真实功能闭环**。私密交付是同进程对象，不是五个联网 Trustee；
没有持久化/并发服务、独立接纳日志、抗删票审查保证或真实公链部署。
研究型确定性 Setup 和旧 Fake SRS 不提供生产可信设置；已公开实验种子使此 fixture 不具备真实选举隐私。
测试不能替代 Groth16–CP 联合健全性及全文的隐私组合证明。

下一轮先验收**同一个 G4-D VK** 的真实本地 EVM 验证，再接入接收/冻结/一次公布的合约状态。
完整 SnarkPack EVM 精确路线仍需解决；不重启已冻结的软件 Miller 试错循环，不沿用旧固定 123 VK 的 Gas。
完整链上结算、Full Tally/Winner-only 公平对比和最终干净源码统一复现仍未完成。本轮到此停止。
