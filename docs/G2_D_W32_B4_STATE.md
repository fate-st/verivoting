# G2-D：D5/W32 环境的 B=4 状态证明

状态：`PASS_G2D_D5_W32_B4_REAL_STATE_AND_OMISSION_REJECTED`；开始时间 `2026-10-30T07:54:24Z`。
本轮只完成一次第一批次/零旧状态的功能和性能预实验；验收后停止，不做B8或其他模块。

## 1. 基准、配置与真实源码

开发基点是远端核实一致且干净的G1-E提交 `8db390a6fae628b5e451be4fb99f9fe92d69dcd6`。
独立分支 `experiment/g2d-w32-b4-state`。先提交新增入口/配置，再从干净的
`4c6220f66c7fdc53b71966a001e5716eaf9f9f1a` 执行。git_dirty=false是进程开始状态，写出任何结果前再次检查HEAD未变且工作区干净。
所有运行日志先保存在仓库外，结束后复制归档，不伪装结果文件产生后的dirty状态。

新增入口 `crates/vv-friendly-dkg/src/bin/g2d_state_d5w32_b4.rs`，配置
`configs/protocol_manifest.JUBJUB-G2D-STATE-D5-W32-B4.json`。
冻结D5/W32选票Manifest、D0/D1/D2/G1-E/G2-C数据/fixture/原入口和密码学库代码均不变。
Cargo.toml只注册入口并直接引用已有锁定sha2用于矩阵指纹；Cargo.lock仅增加该直接依赖引用，没有版本升级。
不得将当前Cargo.lock哈希声称为历史运行的旧哈希；旧测量仍由其历史提交和记录追溯。

## 2. 合法性聚合与累计关系的连接

1. 读取D2完整32证明池，验证冻结raw/fixture/Manifest和状态实现哈希。规范解码32份证明并检查round-trip和字节不同，只选择indices0..3。
2. 四份源Groth16证明在同一个D2 ballot VK下验证；四个地址/nullifier不同，每行25字段。按candidate_id检查三坐标顺序，记录打包逐字段等于原矩阵。
3. 公钥、eid、final context、keySetHash、region4及registration_root均与冻结D2相同。按D2 seed重建size32研究generic SRS，hash一致，specialize到4，真实重新生成一份B4 SnarkPack V2 aggregate并完整验证；未新生成选票证明/DKG/Groth16 ballot Setup。
4. 将同一矩阵LE字段值规范编码为32字节BE叶向量，计算 Root_v2(eid,ctx,4,1,4,X4)。状态R1CS用相同四条记录和顺序重算有序树、规范点编码和全部六个密文分量累计。
5. 初始old_state为三坐标零密文。提交的new_state先在计时外准备；G1-E candidate-major独立参考不调用原fold/加法封装，检查每个候选人c0/c1精确相等。
6. 单独生成一套状态Setup/PK/VK和一份状态证明，37个公开输入布局保持 `VV/JUBJUB/G2-C/STATE-B4-LAYOUT/v1`。状态VK与选票VK明确不同，未加载/复用W10状态参数。

两证明分别验证，以相同有序矩阵和batch root进行原生连接；不是state R1CS内部递归验证SnarkPack。
通用B4累计电路本身没有独立W32/注册深度字段：D5/W32资格由认证选票上下文、合法性证明及host冻结根检查传递。
这不是新的in-circuit权重范围证明，也不能省掉共同的输入绑定检查。

## 3. R1CS 指纹与唯一遗漏负例

实际对完整A/B/C系数矩阵流式SHA-256，而非根据约束数推断关系：
`361195013588cde48bf43e1c5b0071f9cbe8462675246fe3d6f88dec41c6315e`。
编码domain为VV/G2-D/ORDERED-R1CS-ABC/v1，依次包含instance/witness/constraint计数、矩阵标签/行数、各行宽度，
每项系数canonical LE32和列索引u64 BE；包含常数instance。编码说明和耗时均记录在raw JSON。
正/负例矩阵摘要完全一致，37-input packing与实际instance assignment精确一致。
保留既有state实现文件哈希和layout，但不因count相同而推断旧VK兼容；本轮使用独立新状态Setup，
不依赖历史W10参数的关系等价假设，也未倒填历史矩阵指纹。

唯一负例保持四条记录、顺序、batch root、原aggregate及old_state不变，仅将new_state改为ct1+ct2+ct4。
R1CS不满足（首个不满足约束2743654）；原正确状态proof对修改后语句验证失败。
改变的公开字段恰为零基25..36，即全部new_state字段；前25字段未变。没有尝试给无效witness生成proof，也没有扩展攻击矩阵。

## 4. 真实计时和资源

| 指标 | 实测 |
|---|---:|
| 源选票证明组成 | 4份复用，0份新生成；完整32池仅解码 |
| 新B4聚合生成 | 8.159 ms |
| 共同SnarkPack Verify | 4.951792 ms |
| 共同矩阵/上下文/根绑定检查 | 3.416083 ms |
| 独立Direct Fold及新状态相等检查 | 0.004333 ms |
| 正确R1CS合成/满足性/packing | 4006.886 ms |
| 正确/负例矩阵指纹额外耗时 | 2155.859 / 1998.142 ms |
| 状态约束/公开输入/witness变量 | 2743666 / 37 / 2693602 |
| 一次State Setup | 24941.019 ms |
| 一次State Prove | 15998.832 ms |
| State VK预处理 | 1.049 ms |
| State Verify | 2.088500 ms，通过 |
| 遗漏第三票后的原proof Verify | 2.397042 ms，拒绝 |
| State Proof / VK / public inputs | 192 / 2168 / 1184 B |
| State PK压缩大小 | 847800624 B，只在内存测量，未保存 |
| 源aggregate / 输入矩阵 / transcript | 8212 / 3200 / 3262 B |
| 进程累计峰值RSS | 5911.078 MiB |

源SRS重建/specialize和四次单票验证时间另存raw，不混入在线验证。
RSS包括关系合成、完整矩阵指纹、Setup/Prove，是累计进程峰值，不是独立阶段内存，也不与G1-E暖操作RSS直接比较。
本轮只有一个功能样本，不是正式状态统计或显著性能优势。

## 5. 公平路径比较：共同成本和增量分开

两条路径使用完全相同的四票矩阵、batch root、零旧状态和完整新状态，且共同执行输入/上下文/根绑定检查。
构造提交new_state在计时外，不把累计构造藏进共同检查；独立参考重新累计并对比提交状态作为baseline增量。

| 原生验证路径 | 共同SP | 共同绑定检查 | 增量检查 | 组件和 |
|---|---:|---:|---:|---:|
| SnarkPack + Direct Fold | 4.952 | 3.416 | 0.004333 | 8.372208 ms |
| SnarkPack + State Verify | 同上 | 同上 | 2.088500 | 10.456375 ms |

表中是实际测量组件之和，并非分别实测的完整事务elapsed；没有重复执行共同SP去冒充两套独立样本。
Setup/Prove是额外证明者成本，未隐藏进“验证更快”。本次B4原生状态Verify比Direct Fold贵，因此没有原生验证性能优势。
两层一致性是正确性证据，不自动推出效率优势；以上时间不是Gas，也不代表完整DA/结算成本。

## 6. 实际命令、日志与产物

```bash
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 fmt --all
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 clippy -p vv-friendly-crypto -p vv-friendly-dkg --all-targets --release --locked --offline -- -D warnings
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 test -p vv-friendly-crypto -p vv-friendly-dkg --lib --release --locked --offline
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 test -p vv-friendly-dkg --bin friendly-g2d-state-d5w32-b4 --release --locked --offline
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 build -p vv-friendly-dkg --bin friendly-g2d-state-d5w32-b4 --release --locked --offline
# Commit source as 4c6220f; execute exactly one heavy positive/negative experiment:
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 run --release --locked --offline -p vv-friendly-dkg --bin friendly-g2d-state-d5w32-b4
```

43项库测试+4项复用独立参考测试、严格Clippy及格式检查通过。开发期一次未使用函数lint已在正式采样前修复；无失败的重型样本被删除或覆盖。
日志为experiments/g2_d_raw内clippy/tests/fold-tests/build/release.log；完整环境和二进制hash为environment.json。
初次cargo check --offline仅刷新同版本sha2直接依赖引用，之后所有检查/构建/运行均--locked --offline。

- 原始JSON：`experiments/g2_d_raw/state_transition_d5w32_b4_groth16.json`，SHA256 `f2d965d386b7e902b4467a94e580ca4f5b19233807097aaa6991961f687d96b0`。
- 公开Fixture：`fixtures/jubjub_state_g2d_d5w32_b4_public_v1.json`，SHA256 `254c0f35bd7715f2807ba7981cca7ff52f50bfc5445d5909f4a49420c2f8fe04`。
- 新描述SHA256：`c8568975753d5abfff4898c65e8587b6e0ace25e2165dbba9f66babab947a288`。
- 状态VK（Ark canonical compressed）SHA256：`92871cac5c3452ab0a1647939507d7b041dcccf189e3b60ea2b19c1d6e1e1856`，与D2 ballot Ark VK hash不同。
- batch root：`91f6c687850b799e58e413c6c9c3b67e1acb1f2cb8a03e2b4d88a3729a9a2134`。
- 源B4 aggregate SHA256：`10c4b85a3d063d2aaa0b0d3bce3050f59267652c978636bd3be5e8d06a06ecd6`。

SHA256SUMS清单覆盖本轮归档；源代码、Cargo.lock及配置/基准哈希记录在原始JSON中。
Fixture保存37-field正/负输入、状态proof/VK、四条记录、原四份选票proof/ballot VK、完整B4 aggregate/matrix/transcript/verifier SRS，
可用于独立公开验证。不保存PK、prover SRS、选民秘密、weight opening或Trustee份额。研究seed/fake SRS并非生产可信设置。

复现使用包含本入口且源码一致的干净提交，并指定两个全新输出路径：

```bash
CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_STRIP=none \
cargo +1.99.1 run --release --locked --offline -p vv-friendly-dkg \
  --bin friendly-g2d-state-d5w32-b4 -- \
  --output experiments/g2_d_reproduce/state_transition_d5w32_b4_groth16.json \
  --fixture-output fixtures/jubjub_state_g2d_d5w32_b4_reproduce_v1.json
```

## 7. 验收与停止边界

十项验收均通过：D2四票有效、新B4聚合、有序矩阵/root绑定、状态R1CS/Prove/Verify、遗漏拒绝、独立参考、分离VK、历史文件未改及完整归档。
只有第一批次零旧状态；没有多批次连续性、spent-nullifier/旧状态校验、合约冻结绑定、防重放或完整结算。
根只承诺已选择提交的四条记录，不证明收到的其他票均被接纳；强制纳入/防审查需要独立intake log。
W32 P3、十种Trustee子集、Winner-only/Full-Tally比较和完整SnarkPack EVM仍未完成。
Gas/完整DA/calldata均为null；不迁移旧G5-A W10/B2 Gas。Miller路线保持冻结。
B4至此停止；先审查PK与约5.77GiB（5911MiB）峰值预算，再决定是否单独授权B8，不自动继续。
