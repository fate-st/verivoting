# G1-E：统一 D5/W32 的正式原生性能实验

本轮十项验收均完成：`PASS_150_TIMING_TRIALS_45_MEMORY_PROCESSES`。实际开始时间：`2026-10-30T07:00:30Z`。
分支 `experiment/g1e-fixed-suite-benchmark`；本轮结束后停止，未实施其他 Gate。

## 1. 固定数据源与真实源码版本

开发基点为干净的 `808f1441d1f954214ba5abf1c507b42b031ee306`，本地/远端一致。
包含新入口和分析脚本的实际测量提交为 `6824611b4c77d5ad21d62a14bd4e95cfa19f1ad4`。
进程开始时读取 git_dirty=false；整个测量期间不改源码，结束时再次核对 HEAD、tracked diff 和源码 hash。
新输出自然产生未跟踪文件，不将“输出后目录有新文件”误称为测量源码未提交。

五批次均为 D2 的32证明池有序前缀：C_vote、深度5、32注册单位权重选民、上界32、3候选人、region4，
同一 BLS12-381 VK、SnarkPack V2、同一 size-32 generic research SRS（各B分别 specialize，不声称 verifier SRS字节相同）。
全部证明规范解码/round-trip、去重，并在同一 VK 下验证；公开矩阵精确匹配原 ciphertext records。
研究 SRS 按原 seed重建并核对 hash；没有 Groth16 Setup，也没有新选票 Prove。
D0/D1/D2 源码、JSON、fixture、冻结 Manifest及密码学库不变。Cargo.toml只注册新二进制，
因此这份构建清单不再等于 D2记录的旧hash；新源码提交和新清单hash已记录。

## 2. 必要正确性检查

五个B各一次真实聚合正例及一次末票 nullifier负例：只改末行 field7，保持 aggregate不变，
重建匹配的新 transcript，全部拒绝。新实验的独立 candidate-major参考直接对c0/c1群元素相加，
不调用 fold_state_for_batch 或 Ciphertext::add，与原record-major fold比较，所有候选人一致。
不改写旧D2的同函数自比较历史；独立检查不计入验证时间。

4项新测试覆盖全部支持规模含B32、单坐标变更的精确增量、排列保持同态和、空/非法宽度边界。
空参考和返回旧状态，正式fold仍拒绝空/非支持规模；同态和与有序batch身份不同。
另43项既有库测试全部通过；Release build、全target strict Clippy、格式检查通过。

## 3. 测量设计及统计口径

每B五轮预热，共25批次，保存在warmup.json且排除统计；初始K校准也不入正式样本。
正式每B30轮：150批次JSON、450操作窗口。每轮打乱B顺序和三操作顺序，固定ChaCha20 seed=20261030并保存实际顺序。
预热按150ms目标校准K，正式阶段冻结K。逐条保存K、elapsed及elapsed/K。
450窗口实际为115.769–352.606ms，4个超过目标250ms；全部保留，无离群删除/替换。

逐票与聚合Verify同用Bellperson/blstrs及预处理VK。解码、SRS重建/specialize、功能检查单列/排除在线时间。
聚合计时包括实际生成/返回/释放对象，验证包括调用循环及验证随机数初始化。
计时无RSS采样线程；复用同一证明池，不是30次独立选举或新Groth16 Prove。

| B | Individual median ms | Aggregate Verify median ms | Aggregation median ms | Paired verifier ratio median [95% CI] |
|---|---:|---:|---:|---:|
| 2 | 0.967169 | 3.128145 | 4.360998 | 0.302 [0.300, 0.307] |
| 4 | 1.901342 | 4.091989 | 8.782294 | 0.481 [0.440, 0.504] |
| 8 | 3.944255 | 4.830629 | 16.769415 | 0.816 [0.768, 0.834] |
| 16 | 7.134917 | 5.522694 | 30.857372 | 1.402 [1.376, 1.432] |
| 32 | 15.446558 | 6.480822 | 52.507146 | 2.461 [2.373, 2.543] |


完整CSV/JSON报告mean、median、样本std(ddof=1)、min/max及10,000次percentile Bootstrap中位数95%条件区间。
Bootstrap seed=20261030；比例为各轮individual/aggregate Verify比值的中位数，不等于两个中位数相除。
串行样本可能相关，CI只描述本机固定池的操作样本，不是独立选举总体区间。n=30不支持强P95结论，P95为null/CSV空值，不是零。

本次B16/B32验证者比例区间高于1，B2/B4/B8低于1，说明较大批次在本机有验证者计算收益；
不能断言普适交叉点正好B16。聚合增加总计算：B32聚合＋验证中位数58.772ms，仍高于逐票验证。

## 4. 字节与通信口径

原证明每份192B；B2/4/8/16/32 aggregate为5236/8212/11188/14164/17140B，全部大于原证明总量。
B32 proof＋one matrix：单票31744B、聚合42740B，没有通信压缩收益。矩阵已含密文编码，不能另加相同密文；
transcript大小单列，可由矩阵导出，不作为第二份必传数据。
Bellperson VK wire=3364B（不是Ark压缩格式）；generic SRS wire=18448B，
各B verifier-SRS JSON大小也记录。JSON文本编码不等于通用二进制协议长度；缓存VK/SRS不计入每批载荷。
完整DA/framing/calldata、Gas未测量，保持null。

## 5. 内存：明确阶段边界

5批次×3操作×3独立进程=45个不同PID。记录pool加载后RSS、操作前/后、采样阶段峰值/增量、样本数及独立累计high-water mark。
阶段开始在参数准备和一次K校准之后，是warmed-operation RSS。准备包含源证明验证/SRS specialize/参考检查及预先构建的aggregate，
不是cold allocation profile。Mach resident size每1ms采样，可能漏过短峰，是采样下界；包含观察线程开销。
图中误差棒为3进程min/max，不是95%CI。真实零增量不当成缺失，缺失不补零。
B32各阶段采样峰值中位数约18.031/19.609/19.703MiB。
不能与D2含Setup/Prove的3111MiB直接相比后宣称内存优化收益。

## 6. 产物、环境与哈希

六组科学图均有PNG/PDF/SVG：验证、每票成本、配对验证者比率、证明/通信、RSS/增量、聚合及摊销成本。
全部来自G1-E真实JSON，未拼接D0/D1/D2单次计时；六PDF已渲染检查，无裁剪/重叠。
raw_samples.csv为150行批次记录（含K/窗口/顺序/源JSON hash），summary.csv为五规模完整统计。
CSV使用有类型的表格数据，缺失P95不补0；按要求没有额外XLSX。

- manifest SHA256：`f7d657f7329e32bd75910912c9bacb5891574a38c6ac98bb807ab74516bc805d`。
- summary JSON：`e413aab87605db515973a1acb936e36da7f0a253892935cf5c3d92d4ba24bd34`。
- raw CSV：`548e17b96de2c537aba8353781359b314ed7442d91e2121ef8ad27f8a3158f53`。
- summary CSV：`5c71e2d6dfc5f04c9defb16bca126dfcf9e04ec3e4191b6670e0a6aad9c14fb0`。
- 原始/结果全清单：experiments/g1_e_raw/SHA256SUMS；实际硬件、OS、工具链、二进制hash及库版本见environment.json。

Apple M4/32GiB，Release opt-level=1、strip=none、构建并发2；NumPy 2.3.5、Matplotlib 3.11.2。
Rayon未强制单线程。绘图库在仓库外临时虚拟环境，不改密码学库。环境采集使用显式cargo +1.99.1，
因为本机不带+版本的cargo不可用；所有正式构建/测试命令均已显式指定工具链。

## 7. 实际命令与复现

```bash
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 fmt --all
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 clippy -p vv-friendly-crypto -p vv-friendly-dkg --all-targets --release --locked --offline -- -D warnings
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 test -p vv-friendly-crypto -p vv-friendly-dkg --lib --release --locked --offline
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 test -p vv-friendly-dkg --bin friendly-g1e-fixed-suite-benchmark --release --locked --offline
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 build -p vv-friendly-dkg --bin friendly-g1e-fixed-suite-benchmark --release --locked --offline
# Commit source as 6824611, then:
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none /private/tmp/verivoting-g1e.JNP178/venv/bin/python scripts/g1e_run.py
MPLCONFIGDIR=/private/tmp/verivoting-g1e.JNP178/mpl /private/tmp/verivoting-g1e.JNP178/venv/bin/python scripts/g1e_analyze.py
ARTIFACT_TOOL_MODULE=/Users/lijian/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/@oai/artifact-tool/dist/artifact_tool.mjs node scripts/g1e_csv.mjs
```

复现须安装匹配绘图库到自己的venv，从干净且hash匹配的源码运行；g1e_run.py --raw NEW_RAW、
g1e_analyze.py --raw NEW_RAW --out NEW_RESULTS，CSV builder位置参数NEW_RESULTS。默认拒绝覆盖旧结果。
所有build/Clippy/测试/timing/memory campaign日志在raw目录。采样前修复过两项lint告警，没有失败的正式样本被隐去。

## 8. 十项验收及论文表述

G1E-01固定套件、02五规模正例、03五规模负例、04每规模30轮、05字节分离、06独立内存进程、
07六组三格式图、08 JSON/CSV/SHA、09独立参考、10干净提交/Release/测试：全部通过，解释范围如上。

Experimental Evaluation可用英文：

We evaluated native verification over ordered prefixes of one archived 32-ballot proof pool under a fixed depth-five, weight-bound-32 configuration. The circuit, verification key, election context and generic research SRS were held constant. For each batch size in {2,4,8,16,32}, we excluded five warm-up rounds and collected thirty interleaved operation-level timing samples. On this machine, aggregate verification reduced verifier computation at the two largest tested batch sizes, while aggregation increased total computation and aggregate proofs remained larger than the individual proofs. These measurements characterize repeated operations on a fixed proof pool, not independently generated elections, production setup security, or on-chain settlement costs.

仍未完成：W32状态证明、P3 v2十子集、正式隐私证明、Full-Tally披露比较、生产SRS、完整冻结状态绑定/结算及SnarkPack EVM。
本轮停止，下一项只计划G2-D W32 B4，未自动实施。Miller路线继续冻结。
