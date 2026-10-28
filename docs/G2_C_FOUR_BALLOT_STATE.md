# G2-C：四票状态证明与遗漏第三票实验

状态：`PASS_G2C_REAL_GROTH16_B4_STATE_AND_OMITTED_THIRD_BALLOT_REJECTED`。
2026-10-28 已完成一次真实 Release 功能验收。本轮只做此前计划的这个增量，
没有重做 DKG、单票证明、聚合 SRS 或 EVM 实验。

## 实验回答的问题

四张合法选票的 SnarkPack 聚合通过，是否足以保证四张票都被计入新密文状态？
本次构造同一个合法聚合下的两种状态：

- 正确：`CT_new = CT_old + ct1 + ct2 + ct3 + ct4`，真实状态证明验证通过。
- 遗漏：保留全部四条记录、正确批次根和原聚合，只提交
  `CT_new = CT_old + ct1 + ct2 + ct4`。R1CS 不满足；原合法状态证明在这个新状态
  对应的公开输入下也验证失败。没有尝试为不满足的 witness 生成证明。

这展示的是必要的遗漏功能用例，不是代替健全性证明或完整结算安全分析。

## 实施与绑定

入口：`friendly-g2c-state-batch4`。独立配置：
`configs/protocol_manifest.JUBJUB-G2C-STATE-B4.json`。新布局：
`VV/JUBJUB/G2-C/STATE-B4-LAYOUT/v1`。

1. 读取已归档 G1-C2 公开 fixture，检查固定 SHA-256，解码其中真实 B=4 聚合、
   原单票 VK 和 verifier SRS；使用完整四行矩阵及重建的 transcript 再验证一次。
   此步实测通过，没有重新生成源证明或 SRS。
2. 逐项核对四条记录与刚验证的 `4 × 25` 公开字段矩阵，检查相同的 election、
   final context、region、注册根、公钥及 KeyCore digest。按照字段数值，将
   SnarkPack 的规范 32-byte LE 编码转换为状态叶的规范 32-byte BE 编码，并核对
   `ballot_field_vector_bytes`，不是直接混用两种字节序。
3. 四个叶子绑定各自索引、B=4、batch index=1 和同一上下文；使用既有
   `VV/BATCH/LEAF/v2`、`VV/BATCH/NODE/v2` 编码计算四叶、三内部节点树。
   电路重新执行全部七次 SHA-256，并约束为公开 batch root。
4. 六个累计分量在电路内执行共 24 次 Jubjub 点加法。旧状态为三个零密文，
   这是单批次初始状态验收，不是多批次状态机或 stale-state 实验。
5. 正反例均实际合成并 finalize R1CS，公开输入逐项等于 instance assignment。
   新 B=4 关系执行一套新 Groth16 setup、一份真实证明、一次合法验证；同一遗漏
   案例另做一次原证明验证，没有增加替换、交换、重放等故障矩阵。

原 `StateTransitionCircuit` 保留为具体的 B=2 类型别名；新类型
`StateTransitionCircuitForBatch<4>` 显式选择四票关系。旧 B=2 helper、默认常量、
布局、入口和历史材料保留，不把 B=2 VK 冒充为 B=4。共享状态库源码因参数化而
修改，本次记录新源码哈希，不改写历史测量中的源码哈希。

## 实测数据

| 项目 | 真实结果 |
|---|---:|
| 原 B=4 SnarkPack 聚合重新 Verify | 4.200 ms，通过 |
| B=4 状态约束 / 公开输入 / witness 变量 | 2,743,666 / 37 / 2,693,602 |
| 正例 R1CS 合成、finalize、packing 及满足性检查 | 3,416.734 ms，通过 |
| 遗漏负例同样的 R1CS 检查 | 3,180.690 ms，不满足 |
| 一次全新状态 Setup | 23,232.578 ms |
| 一次真实状态 Prove | 15,853.291 ms |
| VK 预处理 | 1.118 ms |
| 合法状态 Verify | 2.049 ms，通过 |
| 原证明对遗漏状态的 Verify | 2.017 ms，拒绝 |
| 状态证明 / 压缩 VK | 192 / 2,168 B |
| 压缩 PK 大小（只在内存测量，未保存） | 847,800,624 B |
| Setup 后 / Prove 后累计峰值 RSS | 4,683.516 / 6,083.000 MiB |
| 本次进程累计峰值 RSS | 6,083.000 MiB（约 5.94 GiB） |
| 不含编译、测至归档前的流程耗时 | 47,023.338 ms |

遗漏案例的两次合成具有相同约束数与 witness 宽度；首个不满足约束索引为
`2743654`。公开输入只改变新状态的 fields `25..36`（零基索引）；前 25 个字段、
四条源记录、正确批次根和源聚合都未改变。对应根为
`50cf0792b88806800ecb8377856d07a57f285e47dd29917a3ab9c1f704a5e24c`。

Setup、Prove、VK 预处理和 Verify 分别计时。R1CS 时间不是纯 witness 赋值时间；
RSS 是进程累计峰值，不是某阶段的额外内存。本次为一次功能运行，不报告中位数、
P95 或性能优势，也不把历史 B=2 与本次 B=4 拼成正式统计图。

## 材料与复现

- 原始结果：`experiments/g2_c_raw/state_transition_b4_groth16.json`，SHA-256
  `4988e259db57918f6bb978b17057855d21370708881659168ce397f36adfd654`。
- 新公开验证材料：`fixtures/jubjub_state_g2c_b4_public_v1.json`，SHA-256
  `c7446db0bd964f5537e26f68797494bc20a1347c369d1071546c64d2575ba199`。
- 复用源 fixture：`fixtures/snarkpack_g1c2_batch4_public_v1.json`，SHA-256
  `84e3c7748dffa8eeece45ddfdadfe64fcb4c563a3e549a788b0321744f87d024`，原字节未改。
- Apple M4 / macOS aarch64、Rust 1.99.1、Release opt-level=1、strip=none、
  构建并发 2。基点提交 `69e9d5a235d6a333fc36bba95bb756cf94ba782c`，实际
  `git_dirty=true`；实测源码和配置哈希见原始结果，不宣称干净提交复现。

```bash
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none \
cargo +1.99.1 run --release --locked --offline \
  -p vv-friendly-dkg --bin friendly-g2c-state-batch4 -- \
  --output experiments/g2_c_reproduce/state_transition_b4_groth16.json \
  --fixture-output fixtures/jubjub_state_g2c_b4_reproduce_v1.json
```

输出须为新路径。新 fixture 保存状态证明、VK、正反例公开输入、源记录、密文
状态和源聚合引用，不保存 PK、投票 secret、随机数、权重 opening 或 Trustee 私有
份额。确定性研究 setup 和源 fake SRS 不是生产可信设置或实际隐私保证。

两个受影响 crate 的全 target 严格 Release Clippy、格式检查通过，43 项库测试
通过（32 + 11）。只新增一个四叶根及累计和正例单元测试；遗漏负例在本次真实
runner 中执行，不建立额外的重型测试矩阵。

## 结论与边界

本轮补齐了 B=4 下“合法性聚合有效但错误累计被状态关系拒绝”的功能证据。
SnarkPack 与状态证明通过完全相同的有序矩阵和批次根在原生 runner 中衔接，
并非在状态电路内递归验证 SnarkPack。已接受注册根、spent-nullifier、旧状态
新鲜性仍是外部检查；没有合约结算、状态写入或完整 Proposed Gas 测量。
P3 v2 仍限既有已测子集；软件 Miller 固定预算路线保持冻结。

下一项按原方案转向统一参数的原生性能对照：先固定 depth-5 / 32 叶、足够的注册
权重上界和同一 VK，做 B=2/4/8 最小同环境功能采样，再逐步扩展 16/32 和正式
重复统计。不能继续将不同树深度的旧结果当作公平性能矩阵。详见 `NEXT_STEPS.md`。
