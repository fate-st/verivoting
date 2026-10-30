# G4-D：同一 P3 v2 电路 / 同一 VK 的十种 Trustee 子集

状态：`PASS_TEN_P3_V2_SUBSETS_ONE_CIRCUIT_ONE_VK_FROZEN_W32_CT2`。开始：`2026-10-30T09:13:57Z`。
本轮只完成用户指定的第二步，不开发 B8 状态电路、B64/128、Miller、合约或新图表。

## 1. 十种组合和真实冻结目标

使用同一 G2-E CT2（两批各四票后的密文），而不是旧 W10 目标：
`fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json` 的 SHA-256
`42e68fa9bee72fb77e47c292dd3e65386cf4a348192c1ee0a9731cfe2f5eff19`。运行前另一个实际进程重放 G2-E 的两个 aggregate/state proofs，
next=3/spent=8 且最终状态一致；只验证，零 Setup/Prove。证据在 g2e_input_replay.json 和日志。

按原 D0 seed=20261028 重建认证 DKG/RA 注册关闭；pre-policy 在 DKG 前固定。
逐字节匹配 D2 的公钥、eid、pre-policy、prekey context、key core、final context、32 人地址/注册根/关闭摘要。
原生重建前八票密文和 nullifier 并匹配归档，八票累计精确等于 CT2。没有生成选票证明或状态证明。

公开注册权重 32，实际接受八票。五名 Trustee 各创建一次真实 CP 私包、签名承诺并验证；
每种组合使用其中三份已验证份额，实际门限解密和 BSGS 有界解码，私有检查 fixture tally/winner。
私有 tally、CP 份额/响应/nonce、package opening 和 Trustee secret 均不序列化。固定实验 seed 仅用于研究可复现，
不是对知晓 seed 的人提供实际选民隐私；不把函数性用例当作 tally-hiding 数学证明。

## 2. 单个通用电路如何绑定子集

新模块 `crates/vv-friendly-crypto/src/p3ten_gadget.rs`，同一类型 P3TenSubsetCircuit。
选择器为一个公开域元素，顺序对应：123、124、125、134、135、145、234、235、245、345。

1. 电路对 selector 与 0..9 做 equality，约束至少一个为真，非法 selector 不满足。
2. 根据相同 equality flags 选出三个有序 u32 Trustee ID，并强制等于三份实际 CP transcript 的 ID 字节。
3. 同一 flags 从固定十行表逐 bit 选出 canonical Jubjub 标量插值系数；群乘法在 R1CS 内使用这些受约束的 bits。
   常量表覆盖全部十种组合，host 只构造固定表，不按当前 witness 注入自由系数，不替代任何群运算。
4. 复用实际完整 CP（SHA-512 H2F 挑战和全部四条等式/Trustee）与三份 Pedersen package opening。
5. 三个坐标分别插值/解密，约束 tally 的群表示、每项范围和 sum+private_u64_slack=W_registered。
6. 复用未改变的 winner tie-break 约束，只公开 winner，不公开 tally 向量。

小型 R1CS 单元测试实际覆盖全部十行 bits、ID、常数多项式插值，并拒绝 selector=10 及 selector=0/IDs=124。
没有为不同组合编译十个不同关系。

## 3. 新关系、新布局、新参数

- Protocol：`VERIVOTING-JUBJUB-EXPERIMENT-G4D-D5-W32-P3TEN-v1`。
- 计票语义仍为 `VV/JUBJUB/P3/BOUNDED-TALLY/v2`，不改回旧 exact-sum v1。
- 新子集关系：`VV/JUBJUB/P3/BOUNDED-TALLY-v2/TEN-SUBSETS/v1`；布局：`VV/JUBJUB/G4-D/INPUT-LAYOUT/v1`。
- 每个实例 6236368 constraints / 60 inputs / 6009491 witnesses。
- public selector 位于零基 51，W_registered 位于 58，winner 位于 59。
  虽然宽度与旧 v2 都是 60，selector 语义及矩阵已改变，旧 key 不兼容。
- 一次 fresh deterministic research Setup，十份真实 proof，全部调用同一个不可变 parameters 对象。
- 十次完整 A/B/C 矩阵 SHA-256 全等：`e443da84ae3e636c232de16451bad8dd16f27e6fb6eb8f2097f7adcff7d7e2b0`。
  指纹覆盖计数、完整有序行/系数/列，而非仅比较 count；编码 VV/G4-D/ORDERED-R1CS-ABC/v1，u64 BE 计数和列索引、系数 canonical LE32。
- 唯一 VK SHA-256：`0d37a77cfc8a1cdfed612b397149d96816188d164e08a74994dd2abfca6cff29`，与旧 G4-C v2 和状态 VK 均不同。

新增描述 `configs/protocol_manifest.JUBJUB-G4D-P3-TEN-SUBSETS-W32.json` 仅是隔离研究配置，
不是向既有不可变生产 registry 注册新 verifier。D5/W32 ballot/DKG Manifest、曲线、CP/Package H2F suites、
Merkle/PRF 和 Cargo.lock 未改。lib.rs 只注册新模块；旧 p3c1/p3d 两处仅将 helper 改为 pub(crate)，
原函数体逐字保持相同。不能将“相同宽度”误认为新 VK 已部署或旧 verifier Gas 仍适用。

## 4. 真实 Prove / Verify

一次 Setup：64068.560000 ms；VK 预处理 1.208625 ms。

| Trustee 子集 | selector | Prove ms | Verify ms | Proof B | 结果 |
|---|---:|---:|---:|---:|---|
| 1,2,3 | 0 | 41497.323791 | 5.826958 | 192 | 通过 |
| 1,2,4 | 1 | 42154.707750 | 5.995417 | 192 | 通过 |
| 1,2,5 | 2 | 41163.705791 | 5.692750 | 192 | 通过 |
| 1,3,4 | 3 | 41683.520375 | 5.645750 | 192 | 通过 |
| 1,3,5 | 4 | 41892.315958 | 5.649458 | 192 | 通过 |
| 1,4,5 | 5 | 41783.980333 | 5.715458 | 192 | 通过 |
| 2,3,4 | 6 | 43729.161208 | 5.904708 | 192 | 通过 |
| 2,3,5 | 7 | 42830.291000 | 6.775500 | 192 | 通过 |
| 2,4,5 | 8 | 42316.196917 | 5.630000 | 192 | 通过 |
| 3,4,5 | 9 | 41800.700917 | 5.533125 | 192 | 通过 |

每种组合在 Setup 前实际合成/满足性检查/packing 和全矩阵 hash，额外耗时逐项存在 shapes/cases 中，不混入 Prove。
每项公共输入为 1920 B，VK 3272 B；PK 压缩大小 1844946096 B 只在内存测量，不保存。
五份私包在组合之间复用，不宣称创建了三十份私包。十份 proof 均本轮新生成，没有复用旧 P3 proof。
累计进程峰值 RSS 9486.250 MiB，包含十次关系指纹、Setup 和证明，不是独立阶段 RSS。

这是十个不同子集的单次功能证明，不是十次同分布性能重复，不生成均值优势/置信区间/新论文曲线。
没有把不同 Gate 的 Gas 拼接成结算成本；Gas/calldata 均为 null。

## 5. 必要负例、回归和资源

仅对第一份真实 proof 修改 selector=1（其他字段不变）和 winner=2（其他字段不变），实际 Verify 均拒绝。
不为无效 witness 生成证明，不把全面攻击矩阵复制到每个组合。既有范围/slack/overflow/winner 回归仍通过。
34 项 crypto + 11 项 DKG + 6 项 G2-E 状态回归 = 51 项通过；fmt 和全部相关 target 严格 Clippy 通过。
开发期修正 Rust 类型问题后才正式测量，无失败的重型样本被丢弃或替换。

一次重型运行按串行、两编译 job，外部 watchdog 上限 1800 s / prover process RSS 20000 MiB，
每秒读取进程 RSS，实际命令及采样峰值/终止原因见 commands.json。没有触发上限。
每个成功 proof 立即写独立 subset-XYZ.json，完整 raw 在十项验收通过后才产生。

## 6. 实际命令和产物

环境：CARGO_BUILD_JOBS=2、CARGO_PROFILE_RELEASE_STRIP=none；rustc 1.99.1 (58cf8f9c3 2026-10-13)；Release opt-level=1。
完整硬件/二进制 hash 见 environment.json；开始时 HEAD=`9167fa01b64a21d36b80b526eb289f9dc7d971b6`，新增源码未提交，
git_dirty=true，15 项源码 hash 和 8 项输入 pin 如实记录，运行前后不变，不冒充 clean committed source。

```bash
cargo +1.99.1 fmt --all -- --check
cargo +1.99.1 clippy -p vv-friendly-crypto -p vv-friendly-dkg --all-targets --release --locked --offline -- -D warnings
cargo +1.99.1 test -p vv-friendly-crypto -p vv-friendly-dkg --lib --release --locked --offline
cargo +1.99.1 test -p vv-friendly-dkg --bin friendly-g2e-multibatch-state --release --locked --offline
cargo +1.99.1 build -p vv-friendly-dkg --bin friendly-g4d-ten-subsets --release --locked --offline
# 构建后实际直接执行此二进制，watchdog 监控的就是证明进程（未另跑 cargo run 测量）
./target/release/friendly-g4d-ten-subsets
# 另进程只读公开材料，10/10 验证；零 Setup/Prove。output 必须是新路径
./target/release/friendly-g4d-ten-subsets \
  --verify-fixture fixtures/jubjub_g4d_ten_subsets_p3_v2_public.json \
  --output experiments/g4_d_replay/public_replay.json
```

完整复现可指定 `--output NEW_DIR/ten_subsets.json --fixture-output NEW_FIXTURE.json`；所有现有结果拒绝覆盖。
公开重放证明/语句与共同 VK 一致，不声称独立完成生产 registry 信任根或合约存储认证。

- Raw：`experiments/g4_d_raw/ten_subsets_p3_v2.json`，SHA-256 `292bf1d35dab36d44cbd3020d205ae211f8e33a2a479f5eaed4802f6f4c0e7d5`。
- Public fixture：`fixtures/jubjub_g4d_ten_subsets_p3_v2_public.json`，SHA-256 `6fa754055a578efbc92e08b79a9f85078ab929745a008bfb9c0decff693b357b`。
- 新配置 SHA-256：`9f1611de7c5adde363d71e055042da52a9004133925a38e138ef2f8a35072755`。
- 各 subset-XYZ.json、r1cs_shapes.json、public_replay.json、环境/命令/测试/运行日志和 SHA256SUMS 均归档。
- fixture 只含 VK、十份公开 proof/输入、认证公钥/承诺/签名、注册关闭公开证书及密文上下文，不含秘密 tally/shares/openings/PK。

独立 Node 审计重建 CP target、注册关闭/披露摘要及十行全部 60 个公开字段；Rust 公开重放另行实际验证 10/10 proof。
原 G2-E/G2-D/G1-E SHA 清单均保持通过；旧实验结果、图表和旧配置没有重写。

## 7. 验收边界与下一项

G4-D 仅证明固定五名 Trustee 中任意三名的十种规范组合在同一 CT2 下均可满足同一完整 P3 编译关系并用同一 VK 验证。
不推广到任意 n/t、不同曲线、任意目标性能或已完成的 joint knowledge / Function-Equal privacy 数学论证。
真实 roster/registration/common-target 认证仍由本轮 host 比较实现，而非新合约或额外 R1CS registry 验证。

下一唯一任务是用户第三步：把已接受的 G2-E 当前状态在原生状态机中只冻结一次，生成/交付私包、生成并验证 P3，
只写入一次 winner，必要负例仅针对冻结目标替换和重复披露。不能直接把本轮读取 CT2 静态文件称作实时完整流程。
随后才是完整智能合约结算及最终公平性能比较。SnarkPack EVM 的既定 20M 软件路线保持冻结；
不迁移旧 v1/固定 123 v2 的 Gas 为本 G4-D 新 key 的测量，不宣称生产 trusted setup 或真实公链部署。
