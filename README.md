# VeriVoting-Batch VS Code experiment starter

This repository is an executable research prototype derived from the paper's
V4.5 architecture, with versioned native, ballot, state and disclosure experiments.
Measured fixed-parameter functionality is kept separate from production setup,
full privacy/security composition, live settlement and exact on-chain verification.
Unimplemented work is never reported as measured.

## What is implemented

### Latest project checkpoint: G7 same G6 proof / G4-D VK on local Prague EVM (2026-10-30)

[G7](docs/G7_P3_EVM.md) 复用 G6 同一份真实 P3 证明及 G4-D 原始 VK，60 输入，未改 Solidity
验证器。实际签名交易：两个 VK 部署、一个验证成功及四个配对层拒绝（Selector/Winner/密文/真实旧 VK）。
新 VK 部署 6,631,884 Gas，验证 1,331,401 Gas，ABI calldata 2,564 B；本轮 Setup/Prove 均为零。
格式、严格 Clippy、48 项回归测试及 299 项历史数据/图表清单检查通过；字节码和来源 hash 已归档。
这里只完成验证器组件，不包含合约冻结、Trustee 签名、一次性赢家存储或完整 SnarkPack EVM。
G7 验收后停止；下一轮再做 G8 状态结算，不拼接历史组件 Gas。下列“新 VK 未经 EVM”是历史阶段说明，
其验证器组件缺口现已由 G7 补齐，完整结算缺口仍然存在。

### Previous native checkpoint: G6 native Winner-only integration (2026-10-30)

[G6](docs/G6_NATIVE_WINNER_ONLY.md) 已在同一次实际运行中完成：两个 B4 的真实 SnarkPack/State
验证 → 接收器生成 CT2 → 一次冻结 → 五份新私密 CP 包 → `{1,2,3}` 私密门限解码 →
一份新 G4-D P3 v2 证明 → 重建全部 60 输入验证 → winner=1 仅写入一次。
篡改冻结目标、重复披露、重复冻结与冻结后追加批次均拒绝且不改写状态。
完整 VK/矩阵与 G4-D 相同；Setup 重建 69.022 s、新 Prove 41.178 s、公布检查/准备/验证/写入 6.861 ms。
48 项相关测试、严格 Clippy 及另一进程纯公开重放通过；只复用八份票证明、两个聚合和两个状态证明。
这是单次、同进程私密交付、串行内存的研究功能闭环；不是生产隐私、持久化服务或完整合约结算。
新 VK 的 EVM 与完整 SnarkPack EVM/系统级 Gas 对比仍未完成；下一轮先做同 VK 的本地 EVM 验证。

### Previous P3 checkpoint: G4-D ten-subset P3 v2 on W32 CT2 (2026-10-30)

[G4-D](docs/G4_D_TEN_SUBSETS_P3_V2.md) 在同一个新 P3 v2 电路、同一套 PK/VK 下，对同一
G2-E D5/W32 最终 CT2 完成十种 3-of-5 子集的真实 Groth16 Prove/Verify（10/10）。
不是十个硬编码电路：selector=0..9 在 R1CS 内绑定三个 CP transcript ID 和固定表中的
canonical Lagrange bits，十次完整有序 A/B/C 系数矩阵 SHA-256 完全相同。

DKG/公钥/上下文/32 选民注册根/关闭摘要、前八票密文与 nullifier 重建精确匹配 D2；CT2
匹配 G2-E。五份真实 CP/签名私包先原生验证，十种组合各选三份验证份额并私有解码。
电路保留真实 CP SHA-512 H2F、12 条 CP 等式、三份 Package-H2F/Pedersen 打开、三坐标插值、
tally-v2 的 sum<=W32/slack、群绑定和最小编号 tie-break winner。只输出 winner=1 和公开证明材料。

关系 6236368 constraints / 60 inputs；一次新 Setup 64068.560000 ms；
每份 proof 192 B。VK SHA-256：0d37a77cfc8a1cdfed612b397149d96816188d164e08a74994dd2abfca6cff29。
51 项库/状态回归测试与严格 Clippy 通过，公开材料另进程 10/10 重放通过。错误 selector 和 winner 拒绝。

raw 如实记录基点 9167fa01b64a21d36b80b526eb289f9dc7d971b6、git_dirty=true 和源码 hash；不虚构干净提交。
旧 tally/winner 文件仅改 pub(crate) 可见性，约束体逐字不变；旧配置、密钥、实验和 G1-E 图表不改。
这是固定 n=5/tau=3、一个 CT2 的功能证据，不是生产安全性或性能统计。新 VK 尚未上链，
旧 G5-D 的固定 123 v2 Gas 不能用于本关系；完整 SnarkPack EVM 仍 BLOCKED。

G4-D 当时提出的原生状态机后续已由上方 G6 完成；G4-D 本身只记录十子集组件验收，
不将它追溯改写为完整流程或合约结算的执行证据。

### Previous state checkpoint: G2-E consecutive B4 state transitions (2026-10-30)

[G2-E](docs/G2_E_MULTIBATCH_STATE.md) 实际完成两个连续 B=4 批次：同一 D5/W32 选票池，
S0 → S1 → S2；第二批 old_state 精确等于第一批 new_state 且非零，最终全部六个密文分量与
独立累计八张票一致。原生接收层检查同一选举/公钥/冻结注册根、已固定批次根、next、old_state
及 spent nullifiers，通过两个真实验证器后才更新状态。跳过首批、重放首批和重置第二批旧状态
均拒绝且不写状态。公开 fixture 在另一进程中再次验证两批，零 Setup/Prove。

复用八份选票证明、首批 aggregate 和首批状态证明；只新增第二批 aggregate 和一份状态证明。
重建 Setup 后完整 VK 字节与 G2-D 一致；非零旧状态的完整 R1CS A/B/C 指纹也一致。
重建 Setup 25314.304084 ms，新第二批 Prove 16917.430791 ms；
两批 State Verify 2.566542 / 3.004792 ms；
累计峰值 RSS 5793.875 MiB。49 项测试及严格 Clippy 通过。

测量基点为 61193416f36a4bb4132091a10259ced1282d5978，新增源码当时未提交，raw 如实记录 git_dirty=true 和逐文件
SHA-256，不将基点 HEAD 冒充新增入口的已提交版本。密码学库、电路布局、冻结选票配置及历史
D0/D1/D2/G1-E/G2-C/G2-D 结果未改。

这是两个 B4 状态证明和原生内存检查点，不是 B8 电路、递归证明、独立接纳日志或链上结算。
P3 v2 十种子集、W32 完整 Winner-only 串联和完整链上验证仍未完成。下一步只推进 P3 v2
同一新 VK 下的十子集，不重复已经通过的原生门限十子集，不重启冻结的 Miller 路线。

### Previous state checkpoint: G2-D D5/W32 B4 state proof (2026-10-30)

[G2-D](docs/G2_D_W32_B4_STATE.md) passes from clean source
`4c6220f66c7fdc53b71966a001e5716eaf9f9f1a`. Four exact D2 D5/W32 ballots are reused;
one new B4 SnarkPack aggregate and one separately keyed state proof verify.
The same ordered rows/root bind both verifiers; omitting only ct3 from the
submitted new state rejects in R1CS and under the original state proof.
An independent candidate-major sum matches every ciphertext coordinate.

State constraints/inputs: 2743666/37; complete matrix
digest equal in positive/negative cases. State setup/prove/verify:
24941.019/15998.832/2.089 ms; proof/VK/inputs:
192/2168/1184 B. Shared SnarkPack Verify is
4.952 ms; common matrix/root binding 3.416 ms;
independent direct fold 0.004333 ms. Single-sample baseline/proposed
component sums are 8.372/10.456 ms, NOT whole transactions or
a native efficiency benefit. Setup/Prove are additional. Peak process RSS:
5911.078 MiB; PK 847800624 B measured only, not persisted.

47 tests and strict Clippy pass. Historical artifacts/crypto code/frozen suite
are unchanged. The 37-input generic B4 relation is not a new in-circuit W32
range proof: its D5/W32 source is bound by the ballot context and host checks.
This is first-batch/zero-old-state native linkage, NOT recursive SnarkPack,
multi-batch settlement, censorship resistance, W32 P3 or EVM Gas. STOP after B4.

### Previous fixed-pool checkpoint: G1-E repetitions (2026-10-30)

[G1-E](docs/G1_E_FIXED_SUITE_BENCHMARK.md) is measured from clean source
`6824611b4c77d5ad21d62a14bd4e95cfa19f1ad4`. B=2/4/8/16/32 use prefixes of one D2
proof pool, the same VK and generic research SRS; zero new ballot proofs/Setup.
Five excluded warmups and 30 formal rounds per B yield 150 batch records and
450 operation windows; 45 fresh B/operation processes measure memory separately.
All five positive/negative cases and independent fold comparisons pass;
43 existing + 4 new tests and strict Clippy pass. Historical data are unchanged.

B16/B32 reduce native verifier time here, but aggregation adds computation and
aggregate proofs are larger. These are repeated windows on ONE fixed pool/machine,
not independent elections. P95 is withheld at n=30. Warmed 1 ms phase RSS is a
sampled lower bound, not the historical Setup/proving process peak. Six figures
have PNG/PDF/SVG versions. CSV/statistics: `results/g1_e`; raw observations:
`experiments/g1_e_raw`. See report for conditional CIs and exact measurement scope.

STOP after G1-E. W32 state/P3, production SRS, SnarkPack EVM and full settlement
remain unfinished; no larger-batch or Miller work is added.

### Previous fixed-suite checkpoint: G1-D2 B=32 (2026-10-30)

[G1-D2](docs/G1_D2_UNIFORM_BATCH32.md) passes in the SAME frozen D5/W32 suite.
All compatibility checks match D1 before its 16 archived proofs are reused;
only voters 17–32 generate 16 new proofs. All 32 verify individually, one real
size-32 V2 aggregate verifies, and changing voter 32's nullifier with a rebuilt
transcript rejects. D0/D1 sources, artifacts and descriptor bytes are unchanged.

Measured from clean source commit `8e72034aa6b97c52ad23adccdff563cc27578001`, with
`git_dirty=false` at process start and a clean unchanged tree until archival.
Setup reconstruction: 10.106 s; ONLY 16 new proves: 106.863 s.
Individual/aggregate Verify: 14.048915 / 6.942209 ms;
aggregation: 57.087584 ms. Source proofs/aggregate/matrix:
6144 / 17140 / 25600 B; direct fold/check: 0.041917 ms;
peak process RSS: 3111.375 MiB. Strict Clippy and all 43 library tests pass.

The fixed-suite functional sizes are now 2/4/8/16/32. These are single samples,
NOT repeated statistics or a stable speedup/crossover conclusion; aggregate
proof size remains larger. This task STOPS after B32. Next is planned repeated
native measurements/automatic figures, not B64/256, W32 state/P3 or Miller work.

### Previous fixed-suite checkpoint: G1-D1 B=16 (2026-10-28)

[G1-D1](docs/G1_D1_UNIFORM_BATCH16.md) passes in the byte-identical D5/W32
suite. Reconstructed DKG/context/root, first eight rows, VK bytes/hash and
research SRS hash match G1-D0 BEFORE eight archived proofs are reused. Only
voters 9–16 generate new proofs. All 16 verify individually, the real size-16
aggregate verifies, and changing voter 16's nullifier rejects.

One setup reconstruction: 11.006 s; eight NEW proves: 56.349 s. Same-run
bellperson individual/aggregate verification: 6.917 / 4.991 ms; aggregation:
28.525 ms. Source proofs/aggregate: 3,072 / 14,164 B; peak process RSS:
2,911.922 MiB. The smaller verification time is one sample, NOT a statistical
speedup/crossover claim; proof size is still larger. All 43 library tests and
strict Clippy pass. D0 code/configuration/raw/fixtures remain unchanged.

G1-D1 stopped at B16; its B32 follow-up has now passed in G1-D2 above.
Repeated statistics, W32 state/P3 and full settlement remain unimplemented/
unmeasured. No software Miller work is included.

### Previous fixed-suite checkpoint: G1-D0 B=2/4/8

[G1-D0](docs/G1_D0_UNIFORM_NATIVE_PILOT.md) runs a fixed depth-5, 32-leaf,
unit-weight registry with W=32, one certified key/context and one new ballot
VK. Eight real source proofs form ordered prefixes B=2/4/8; all aggregates
verify and the sole changed-eighth-nullifier negative rejects.

| B | Individual bellperson Verify (ms) | Aggregate Verify (ms) | Individual proofs / aggregate (B) |
|---|---:|---:|---:|
| 2 | 1.330 | 3.923 | 384 / 5,236 |
| 4 | 1.847 | 3.971 | 768 / 8,212 |
| 8 | 3.949 | 5.210 | 1,536 / 11,188 |

This single pilot shows no size/verifier-time advantage at these points; it
is not repeated statistics or a crossover result. Native direct-fold checks
are also measured (0.003292 / 0.006459 / 0.010583 ms), as a fair baseline
component rather than a complete settlement. Current-suite state/P3 and full
Gas/DA remain unmeasured; historical weight-10 results cannot be spliced in.
Its planned B=16 increment has since passed in G1-D1 above, reusing matching
archived proofs. No software Miller work is included.

### Previous state checkpoint: G2-C (weight 10)

[G2-C](docs/G2_C_FOUR_BALLOT_STATE.md) reuses and reverifies the archived
G1-C2 B=4 aggregate, then proves a newly keyed four-ballot state relation.
The exact verified 25-field rows bind the ordered batch root and all four
ciphertext additions. The valid state proof passes; keeping all four records,
the root and aggregate unchanged while omitting only ct3 from the submitted
new state fails R1CS and rejects the original proof under that new state.

Measured: 2,743,666 constraints / 37 public inputs, setup 23.233 s, prove
15.853 s, verify 2.049 ms, proof 192 B, peak RSS 6,083 MiB. One fresh state
setup, one proof and one negative case were run. This is a zero-old-state
functional case, not statistical evaluation, recursive verification or EVM
settlement. Old B=2 state entry points/artifacts remain unchanged.

Its planned fixed-suite B=2/4/8 pilot has since passed in G1-D0 above,
before extending the unified matrix to B=16/32 and repeated measurements.
No new software Miller work is included.

### Latest native batch checkpoint: G1-C3 (2026-10-28)

[G1-C3](docs/G1_C3_EIGHT_VOTER_SNARKPACK.md) passes native B=8 with eight
different registered voters, one region and authenticated DKG key. A separately
versioned depth-3 ballot relation uses one fresh setup and eight real proofs;
its V2 aggregate verifies and changing only the eighth nullifier rejects.
Each ballot has 928,335 constraints / 25 public inputs. Aggregate size is
11,188 B versus 1,536 B for eight individual proofs; aggregate verification is
5.276 ms versus 4.200 ms for same-run individual bellperson verification.
This single functional run shows no size or verifier-time advantage.

The shared ballot type is depth-parameterized, retaining depth 2 as its old
default; archived B=2/B=4 entry points and artifacts are unchanged. No old key
is reused for depth 3. SRS remains nonproduction. Its planned B=4 state/omission
follow-up has now passed in G2-C above. No new EVM run was made.

### Previous native batch checkpoint: G1-C2

[G1-C2](docs/G1_C2_FOUR_VOTER_SNARKPACK.md) passes B=4 with four distinct
registered voters in one region, one authenticated DKG key, four fresh ballot
proofs and one real SnarkPack V2 aggregate. Changing only the fourth nullifier
rejects. One setup and four proofs were run; B=2 code/results are unchanged.
Aggregate size is 8,212 B versus 768 B for four individual proofs; aggregate
verification is 3.402 ms versus 1.760 ms for individual bellperson verification.
This single functional run does not show compression or speedup. State proofs
were fixed B=2 at that checkpoint; B=4 has since passed in G2-C. Fake SRS stays
nonproduction. Its depth-3 / eight-voter increment passed in G1-C3 above.

### P3 correctness revision (2026-10-28)

The current tally relation is v2: `sum(t_j) <= W_registered`, implemented by
a private range-constrained slack. The former exact-sum relation rejected
legitimate partial turnout. See [the G4-C experiment](docs/G4_C_PARTIAL_TURNOUT.md)
and its separate raw result. A new relation requires a new Groth16 setup/VK;
old P3-C1/D0/D1, G4-A/B and G5-B measurements below are historical **v1**
results, not v2 proving times, subset coverage or EVM gas. The official and old
prototype Manifests remain unchanged; G4-C uses its own experiment descriptor.
The fixed-20M software Miller/SnarkPack route remains frozen.

G4-C has now passed: one fresh setup, two real 192-byte P3 proofs (full/partial
turnout), 6,232,037 constraints and 60 public inputs. Wrong winner and excessive
total weight reject. Only subset `{1,2,3}` was proved under v2; the other subsets
remain unmeasured.

[G5-D](docs/G5_D_P3_V2_EVM.md) now verifies both archived v2 proofs in the
unchanged Solidity verifier on local Prague EVM: partial/full calls consume
1,331,365 / 1,331,377 gas with 2,564-byte calldata each. Changed winner and the
actual old v1 VK both reject. Exactly two deployments and four verification
calls run, with no setup/prove rerun. This is verifier-level acceptance, not
registry/frozen-state binding, settlement writes or a complete SnarkPack path.
Its former native-batch and B=4 state tasks have passed through G1-C3/G2-C;
the next task is the unified native pilot above, not software Miller work.

| Area | Implementation | Reported status |
|---|---|---|
| Canonical encoding and protocol manifest | Domain-separated, length-prefixed, big-endian encoding | READY |
| Registration authority | Pedersen weight commitments, duplicate checks, exact registered-weight close, Schnorr close certificate | READY |
| Threshold setup | Pedersen VSS/GJKR message phases, complaints, corrections, disqualification, Feldman extraction, quorum certificate | READY reference simulation; G3-A five-process happy path passes |
| Voting/tally | Weighted Ristretto ElGamal, homomorphic folding, all ten 3-of-5 combinations, bounded BSGS decoding | READY |
| Threshold-share proof | Generalized Chaum–Pedersen proof over the full ciphertext vector; challenge recomputed from the exact context | READY natively |
| Winner proof P1 | Real arkworks Groth16 proof of score bounds and smallest-ID tie breaking | READY |
| Pre-DKG binding | `policy_pre -> prekey -> DKG session -> KeyCore -> final context` | READY in native experiment |
| G0.5 CP feasibility | 253-bit optimized real CP equation; 3,461,084 constraints | BLOCKED by 1,000,000-constraint proof guard |
| G0.6 curve decision | Isolated Jubjub CP/ElGamal/state relations with real Groth16 | PARTIAL PASS; not a protocol migration or full P3 |
| G0.7 Release baseline | Release build, P1 and four-voter regression | PASS with `strip=none` toolchain workaround |
| G0.8 Jubjub native path | Suite v2, registration close, Weighted ElGamal, DKG, generalized CP, authenticated private packages and winner-only event | PASS in one-process native experiment; no ballot circuit/P3 |
| G0.9-R clean baseline | Commit, Rust toolchain, Release configuration and Jubjub experiment suite | PASS; clean SHA `39cca56`, 50 tests pass, 1 high-memory test filtered |
| G0.9-H0 CP transcript | Registration-bound v3 transcript golden vector and exact R1CS byte binding | PASS; SHA-512 circuit not implemented |
| G0.9-H1 SHA-512 circuit | Real SHA-512 padding/compression over the frozen 684-byte transcript | PASS; 890,920 constraints, no hash-to-scalar/CP integration/Groth16 |
| G0.9-H2-A hash-to-scalar | Exact H2S-v2 wrapper, masking, scalar range and first-valid rule for real counters 0 and 1 | PASS feasibility; variable circuit shape, no CP/Groth16, not full H2 |
| G0.9-H2-B fixed shape | One two-attempt circuit accepts the real counter-0/1 paths with identical shape | PASS feasibility; 2,020,168 constraints, not Suite v2 rejection-128 or full H2 |
| G0.9-H2-C wide H2F candidate | Full 64-byte SHA-512 digest reduced modulo Jubjub Fr in one fixed-shape circuit | PASS candidate feasibility; 1,148,545 constraints, Suite v2 unchanged, not adopted |
| G0.9-H2-D CP-H2F adoption | Prototype manifest pins fixed-shape CP-H2F-v3; native CP and E2E migrated | PASS; 1,151,661 constraints, standalone H2 complete, no CP R1CS/Groth16 |
| G0.9-H3 first CP equation | CP-H2F-v3 challenge and canonical transcript point encodings bound to `zG=A0+dVK` | PASS; 1,168,855 constraints, no coordinate-vector CP/Groth16/P3 |
| G0.9-H4-A one-Trustee CP vector | One CP-H2F-v3 challenge drives the first equation and all three coordinate equations | PASS; 1,207,066 constraints, no Groth16/multi-Trustee P3 |
| G0.9-H4-B one-Trustee CP Groth16 | Real setup, prove and verify over the exact H4-A relation | PASS; 192-byte proof, changed public transcript rejected, not multi-Trustee P3 |
| G0.9-P3-A three selected Trustee CPs | Three complete CP relations with public statement prefixes and private share/proof witnesses | PASS real Groth16; 3,611,598 constraints and 192-byte proof, no package openings/full P3 |
| G0.9-P3-B0 package H2F | Fixed-shape SHA-512-to-field mapping for private-package commitment messages | PASS native/R1CS adoption; 1,000,827 constraints per package, opening equations not yet in P3 |
| G0.9-P3-B1 package openings | Three P3-A CP relations, in-circuit Package-H2F and three Pedersen opening equations | PASS R1CS; 6,191,178 constraints, no Groth16/threshold/winner |
| G0.9-P3-C0 threshold combination | Fixed Trustee 1–3 coefficients `(3,-3,1)` bind three public `C1` points to private plaintext group elements | PASS R1CS; 6,201,342 constraints, no tally decoding/winner/Groth16 |
| G0.9-P3-C1 bounded tally binding | Three private integer tallies are range-bounded, sum to public `W=10`, and satisfy `M_j=t_jG` | PASS R1CS; 6,203,703 constraints, no winner/Groth16 |
| G0.9-P3-D0 winner linkage | Public winner is constrained against the same private tallies using highest-weight, smallest-ID tie breaking | PASS R1CS; 6,205,629 constraints, no combined Groth16 |
| G0.9-P3-D1 fixed-123 complete P3 | Real Groth16 over the completed CP/package/threshold/tally/winner relation | PASS; 192-byte proof, valid verification and wrong winner rejected |
| General-subset/production P3 | One circuit/setup supporting arbitrary selected legal subsets and production ceremony | NOT IMPLEMENTED; G4-B covers only two subsets |
| G1-B0 weighted-ballot core | One-hot choice, registered-weight commitment opening and three Jubjub Weighted ElGamal coordinates | PASS R1CS; 46,573 constraints, not full `C_vote` |
| G1-B1 complete fixed-suite `C_vote` | Context-bound HMAC-SHA-256 address/nullifier, four-region assignment, four-leaf registration membership and G1-B0 core | PASS R1CS; 854,454 constraints, no Groth16 |
| G1-B2 complete fixed-suite ballot proof | Real Groth16 over the exact G1-B1 relation | PASS; 192-byte proof, changed public context rejected |
| G1-C0 ballot aggregation compatibility | Bellperson 0.27.0 SnarkPack V2 over two converted G1-B2 proofs | PASS minimum batch; non-production SRS, no scale claim |
| G1-C1 distinct-voter ballot aggregation | SnarkPack V2 over two different registered-voter statements | PASS minimum batch; distinct nullifiers/ciphertexts, non-production SRS, no scale claim |
| G1-C2 four-voter aggregation | Four different same-region C_vote proofs, depth 2 | PASS native B=4; one run, nonproduction SRS |
| G1-C3 eight-voter aggregation | Eight different same-region C_vote proofs, separate depth-3 layout and new key | PASS native B=8; one run, nonproduction SRS, no efficiency claim |
| G1-D0 same-suite pilot | Fixed depth 5 / W32, same VK and genuine proof-pool prefixes B=2/4/8 | PASS single-run native comparison and direct-fold component; no statistical/settlement claim |
| G1-D2 same-suite B=32 | Strict D1 compatibility, 16 archived plus 16 new proofs, clean committed source | PASS; 32/32 verify, real aggregate and last-nullifier rejection; one functional sample only |
| G1-D1 same-suite B=16 | Strict D0 compatibility, eight archived plus eight new proofs, one real size-16 aggregate | PASS; 16/16 verify and last-nullifier mutation rejects; one functional sample only |
| G2-A fixed-B=2 state transition | Canonical ordered batch root and six-component Jubjub accumulator relation | PASS R1CS; 1,360,002 constraints, no Groth16 |
| G2-B fixed-B=2 state proof | Real Groth16 over the unchanged G2-A relation | PASS; 192-byte proof, changed public batch root rejected |
| G2-C fixed-B=4 state proof | Archived real four-voter aggregate bound to ordered root and complete ciphertext fold | PASS; one 192-byte state proof; omitted ct3 rejects in R1CS and under the original proof; native linkage only |
| G3-A process-separated DKG | Five concurrent Trustees, persisted private state and session-bound signed local transport | PASS happy path; fixed `{1,2,3}` threshold result, no restart test |
| G3-B restart recovery | Trustee 3 exits after state persistence and resumes under a new PID | PASS one restart; transcript/KeyCore/tally preserved |
| G4-A second-subset P3 shape | Complete P3 R1CS for `{1,2,3}` and `{1,2,4}` through one fixed-width coefficient path | PASS equal count shape; 6,236,532 constraints and 59 public inputs, no public coefficient binding/Groth16 |
| G4-B public two-subset P3 | Public selector binds `{1,2,3}`/`{1,2,4}` IDs and derives their coefficients in one complete-P3 circuit | PASS real Groth16 with one shared setup; 6,231,973 constraints, 60 public inputs and 192-byte proof; not all ten subsets |
| G5-A local Prague state verifier | Solidity/EIP-2537 verification of the real 37-input G2-B proof | PASS; 940,640 gas and 1,828-byte calldata; wrong root reverts |
| G5-B local Prague P3 verifier | Same Solidity source verifies the real 60-input two-subset G4-B disclosure proof with its own pinned key | PASS; 1,331,473 gas and 2,564-byte calldata; wrong selector reverts |
| G5-C1 SnarkPack route prerequisites | Public B=2 aggregate/VK/verifier-SRS fixture, native decode/reverify, exact external transcript and Prague primitive probes | PASS prerequisites only; exact GT/internal-transcript EVM verifier remains BLOCKED, no aggregate gas |
| G5-C2 exact internal transcript | Actual official verifier calls compared with independent JS decoding of the same B=2 wire proof | PASS 12/12 calls, real rejection sampling and one changed-nullifier native rejection; no setup/prove or EVM aggregate verification |
| G5-C3-A Fp12 multiplication | Exact Solidity product of two real proof-derived GT operands, cross-checked with native/JS | PASS component only; 609,940 gas, canonical wrong target reverts; pairing-to-GT/full G5-C3 remain BLOCKED |
| G5-C3-B actual TIPP endpoint | Native/independent JS pairing and folded GT target match; wrong in-subgroup target rejects | PARTIAL; actual-point EVM identity probe only, exact GT-target comparison still BLOCKED |
| G5-C3-B2 software Miller loop | All line/state arithmetic in Solidity; real two-step prefix matches and wrong result reverts | PARTIAL; prefix 4,805,196 gas, one full64-step attempt OOG at20M; final pairing/SnarkPack not verified |
| G5-C3-B3 memory-only Miller variant | Same formulas/fixture, bounded scratch reuse; prefix and wrong result checked | PARTIAL; prefix 2,371,368 gas (-50.65%), full64 still OOG at same20M; old baseline preserved |
| G5-C3-B4 dedicated square/014 | General Fp12 square and sparse line multiplication, same field/point/memory scheme | PARTIAL; prefix1,865,022gas (-21.35%vsB3), MODEXP642->516; full64 still OOG20M |
| G5-C3-B5 isolated field cost profile | One Fp and one Fp2 product on saved public endpoint coordinates; all gas accounting reconciles | PASS profile only; windows4,372/16,468gas, MODEXP400/1,200; no Miller rerun/capability promotion |
| G5-C3-B6 low-level Fp primitive | Same two products/windows with reusable MODEXP packing; four carry/halving boundary cases pass | PASS primitive comparison only; windows 2,167/9,965 gas (-50.43%/-39.49% vs B5), MODEXP 2/6 calls unchanged; full verifier remains BLOCKED |
| G5-C3-B7 low-level Miller integration | Exact B6 `_mul` integrated into B4; same two-step prefix and wrong-result cases | PARTIAL; prefix 1,290,640 gas (-30.80% vs B4), 2,829 memory words, 516 successful MODEXPs; one full64 attempt still OOG at the same 20M cap |
| G5-C3-B8 low-level Fp2 primitive | Only `_mul2` changed versus B6; same two public products and three necessary Fp2 boundaries pass | PASS primitive comparison only; Fp2 window 9,965->8,086 gas (-18.86%), allocator growth 2,816->1,664 bytes, six MODEXPs unchanged; no Miller run |
| G5-C3-B9 Fp2 Miller prefix integration | Exact B8 `_mul2` integrated into B7; same two-step prefix positive/wrong-result pair | PASS prefix only; 1,132,763 gas (-12.23% vs B7), 2,411 memory words (-14.78%), 516 successful MODEXPs per case; new variant has no full-loop attempt |
| G5-C3-B10 original Fp2-add cost profile | Unchanged `_add2`; one addition of the same public endpoint operands | PASS cost localization only; window 1,752 gas, allocator growth 896 B, execution 5,740 gas, 0 MODEXPs; no optimization or Miller run |
| G5-C3-B11 low-allocation Fp2-add candidate | Only `_add2` and the contract name change; same public addition and one necessary boundary pass | PASS isolated fixed cases only; window 1,752->849 gas (-51.54%), allocator 896->384 B (-57.14%); transaction total stays 30,880 due to calldata floor; no Miller run |
| G5-C3-B12 Fp2-add Miller prefix integration | Exact B11 `_add2` integrated into B9; same prefix positive/wrong-result pair | PASS prefix only; 1,043,835 gas (-7.85% vs B9), 2,079 memory words (-13.77%), 516 MODEXPs; no full-loop attempt |
| G5-C3-B13 original Fp2-sub cost profile | Unchanged `_sub2`; one subtraction of the same public endpoint operands | PASS cost baseline only; window 1,286 gas, execution 5,296, allocator 640 B, 0 MODEXPs; total 30,880 includes 632 gas calldata-floor supplement |
| G5-C3-B14 low-allocation Fp2-sub candidate | Only `_sub2` changes; same public input and one modular-wrap boundary | PASS isolated fixed cases; window 1,286->765 gas (-40.51%), execution 5,296->4,790, allocator 640->384 B (-40%); total stays 30,880 due to calldata floor |
| G5-C3-B15 Fp2-sub Miller prefix integration | Exact B14 `_sub2` integrated into B12; same prefix pair | PASS prefix only; 985,557 gas (-5.58%), 1,905 memory words (-8.37%), 516 MODEXPs; wrong target rejects |
| G5-C3-B16 latest software Miller full attempt | Unchanged B15 source/runtime; one full64 positive at the same 20M cap | BLOCKED by root OOG; 10,628 MODEXPs succeed, 1,945 memory words; 20M is capped failed spend, completed-loop gas remains null |
| G5-C3-B17 saved-evidence budget review | Recompile source map and derive costs from saved B16 data; 0 EVM transactions | PASS analysis only; MODEXP children 10.65% of failed execution, memory expansion 13,223 gas; full verification remains BLOCKED |
| G5-C3-B18 failed-full-attempt cost attribution | One same-program full64 profiling replay at the unchanged 20M cap | PASS attribution only; B16 OOG exactly reproduced, gas difference 0; full verification remains BLOCKED |
| G5-C3-B19 source/algebra route review | Saved-cost and fixed-modulus review; 0 EVM transactions | PASS analysis only; select one isolated quarterMod candidate, no implementation or measured savings yet |
| G5-C3-B20 isolated quarterMod multiply | One saved Fp performance input and four carry/correction cases; 6 local transactions including deployment | PASS component only; window 2,167->2,023 gas (-6.65%), all five outputs correct; no Miller run |
| G5-C3-B21 quarterMod Miller prefix integration | Exact B20 _mul in a separate B15-based contract; original prefix positive/wrong-target pair only | PASS prefix only; 985,557->951,873 gas (-3.42%), same memory/516 MODEXPs; new full64 variant untested |
| G5-C3-B22 full-conformance decision | Read saved B18/B20/B21 evidence and unchanged full entry; 0 EVM transactions | PASS decision only; plan one same20M full64 attempt, then freeze software optimization/retries on failure; no new gas result |
| G5-C3-B23 quarter full64 budget closure | Exact B21 runtime; one deployment and one fixed20M full positive only | BLOCKED/OOG at20M; 11,076 successful MODEXPs, zero child failures; automatic software optimization/full retries frozen |
| Complete target-chain settlement | SnarkPack verifier, full batch DA and settlement bindings/writes | BLOCKED; G5-A/G5-B measure local cryptographic verifiers |

The P1 proof is not a substitute for P3 because the P1 score witness is not yet
bound in-circuit to the committed threshold-decryption packages.  The public E2E
artifact therefore contains the winner but not the private tally or decryption
shares and has `full_p3_generated: false`.

## Reproduce

Prerequisites: a stable Rust toolchain and Python 3. The G5-A/G5-B local
Solidity/EVM gate also uses Node.js 20.19+ and npm; Foundry is not required.

```bash
cargo test --workspace --release
cargo run --release -p vv-bench -- all --trials 1
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

Or use VS Code task **VeriVoting: test and run core** / run:

```bash
./scripts/run_core.sh
```

Useful focused commands:

```bash
cargo run --release -p vv-bench -- manifest
cargo run --release -p vv-bench -- dkg --trials 3
cargo run --release -p vv-bench -- p1 --trials 3
cargo run --release -p vv-bench -- dlog
cargo run --release -p vv-bench -- e2e
cargo run -p vv-bench -- p3-feasibility --max-cp-constraints 1000000
cargo run -p vv-bench -- cp-optimize --cp-implementation canonical_253 --max-cp-constraints 1000000
cargo run -p vv-bench -- curve-compare --output experiments/raw --seed 20261020
cargo run --release -p vv-friendly-crypto --bin friendly-four-voter -- \
  --output experiments/g0_8_suite_v2_raw/friendly-elgamal-four-voter.json \
  --vectors-output fixtures/jubjub_suite_vectors.v2.json
cargo run --release -p vv-friendly-dkg --bin friendly-dkg-threshold -- \
  --output experiments/g0_8_dkg_raw/g08_dkg_threshold.json
cargo run --release -p vv-friendly-dkg --bin friendly-generalized-cp -- \
  --output experiments/g0_8_cp_raw/g08_generalized_cp.json
cargo run --release -p vv-friendly-dkg --bin friendly-private-packages -- \
  --output experiments/g0_8_packages_raw/e2e-public.json
cargo run --release -p vv-friendly-dkg --bin friendly-native-e2e -- \
  --output experiments/g0_8_complete_raw/e2e-public.json
cargo run --release -p vv-friendly-crypto --bin friendly-cp-transcript-h0 -- \
  --output experiments/g0_9_h0_raw/h0_transcript.json \
  --vector-output fixtures/jubjub_cp_transcript_v3.json
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-crypto \
  --bin friendly-cp-transcript-h1
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-crypto \
  --bin friendly-cp-hash-to-scalar-h2a
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-crypto \
  --bin friendly-cp-hash-to-scalar-h2b
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-crypto \
  --bin friendly-cp-hash-to-field-h2c
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-crypto \
  --bin friendly-cp-h2d-adoption
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-crypto \
  --bin friendly-cp-first-equation-h3
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-crypto \
  --bin friendly-cp-vector-h4a
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-crypto \
  --bin friendly-cp-groth16-h4b
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-p3a-three-cp
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-p3b0-package-h2f
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-p3b1-package-openings
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-p3c0-threshold-combination
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-p3c1-tally-binding
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-p3d0-winner-linkage
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-p3d1-full-groth16
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-crypto \
  --bin friendly-ballot-core-b0
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-ballot-cvote-b1
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-ballot-cvote-b1 -- \
  --groth16-output experiments/g1_b2_raw/full_c_vote_groth16.json \
  --snarkpack-output experiments/g1_c0_raw/snarkpack_min_batch_2.json
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-ballot-cvote-b1 -- \
  --output experiments/g1_c1_raw/source_full_c_vote_r1cs.json \
  --layout-output fixtures/jubjub_ballot_cvote_c1_public_layout_v1.json \
  --snarkpack-distinct-output experiments/g1_c1_raw/snarkpack_distinct_voters_batch_2.json
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-crypto \
  --bin friendly-state-transition-g2a
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-generalized-cp
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-g4a-second-subset-shape
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-g4b-public-subset-groth16
python3 scripts/curve_compare.py experiments/raw
```

G5-A/G5-B with their checked-in public fixtures and native reports:

```bash
cd contracts
npm ci --ignore-scripts --no-audit --no-fund
npm run g5a
npm run g5b
```

See [the G5-A report](docs/G5_A_PRAGUE_STATE_VERIFIER.md) and
[the G5-B report](docs/G5_B_PRAGUE_DISCLOSURE_VERIFIER.md) for the one-time
Rust fixture-generation commands and the exact gas scope.

The previous prerequisite gate is documented in
[G5-C1](docs/G5_C1_SNARKPACK_ROUTE.md). With its saved public
fixture, `cd contracts` and `npm run g5c1` replay only the lightweight
prerequisites; they do not run a complete SnarkPack EVM verifier.

[G5-C2 and the next G5-C3 plan](docs/G5_C2_TRANSCRIPT_CONFORMANCE.md) document
the passing internal-transcript gate. `npm run g5c2` replays the saved proof
through a temporary instrumented CPU reference and compares all actual
challenges with independent JavaScript decoding; it never sets up or proves.

[G5-C3-A and the remaining C3-B plan](docs/G5_C3_FP12_PRODUCT.md) document the
passing multiplication component. `npm run --prefix contracts g5c3` runs only
one valid product and one wrong-target rejection. Its gas is not aggregate gas.

[G5-C3-B](docs/G5_C3_TIPP_ENDPOINT.md) fixes one actual TIPP endpoint from that
same proof. `npm run --prefix contracts g5c3b` rechecks native/independent-JS
conformance and one actual-point EVM ABI probe. Wrong GT target rejection is
native/JS only; software pairing-to-GT remains unimplemented.

[G5-C3-B2](docs/G5_C3_MILLER_LOOP.md) adds a real software Miller component.
`npm run --prefix contracts g5c3b2` checks the two-step prefix positive/negative
and attempts the full loop once at the fixed 20M budget. The latter currently
fails OOG; optimization comes before final exponentiation or scale runs.

[G5-C3-B3](docs/G5_C3_MILLER_MEMORY.md) preserves B2 and adds memory-only reuse.
`npm run --prefix contracts g5c3b3` reuses the saved fixture and compares the
same cases at the same cap. Prefix memory/gas improve, but full Miller still
fails OOG; dedicated square/sparse-line arithmetic is next.

[G5-C3-B4](docs/G5_C3_MILLER_SPARSE.md) implements that dedicated arithmetic.
`npm run --prefix contracts g5c3b4` runs the same cases and one full attempt at
the same cap. Prefix gas improves, but full Miller still OOGs. Next is a small
Fp/Fp2 cost profile, not another full-loop retry or a higher cap.

[G5-C3-B5](docs/G5_C3_FIELD_COST_PROFILE.md) completes that isolated profile.
`npm run --prefix contracts g5c3b5` deploys the instrumentation and runs just
one Fp and one Fp2 product. This baseline is retained for the low-level primitive
comparison below, before any new full-Miller attempt.

[G5-C3-B6](docs/G5_C3_FIELD_LOWLEVEL.md) completes that primitive comparison.
`npm run --prefix contracts g5c3b6` runs the same two public cost cases and four
deterministic arithmetic boundaries in a separate contract. Both products and
all four boundaries pass; isolated arithmetic windows fall to 2,167/9,965 gas.
No Miller loop is rerun at this historical gate; the following B7 records its
integration and fixed-budget full-loop attempt.
Final exponentiation, GT-target comparison and exact SnarkPack stay blocked.

[G5-C3-B7](docs/G5_C3_MILLER_LOWLEVEL.md) integrates the exact B6 multiply into a
new sparse-Miller variant, preserving all other B4 functions and old baselines.
`npm run --prefix contracts g5c3b7` checks the same prefix positive/wrong-result
cases, then makes exactly one full attempt at the unchanged 20M cap. Prefix gas
falls to 1,290,640 (-30.80% vs B4), memory to 2,829 words (-45.94%), and all 516
MODEXP calls succeed. The wrong result reverts with `INVALID_MILLER_RESULT`.
The full attempt still OOGs after 7,848 successful MODEXPs with 3,147 memory
words; that capped spend is not completed-loop or aggregate-verifier gas.
The following B8 completes the planned isolated `_mul2` allocation comparison;
the historical full-loop result above remains the latest full attempt.
Full Miller and exact SnarkPack remain BLOCKED; aggregate gas remains null.

[G5-C3-B8](docs/G5_C3_FP2_LOWLEVEL.md) changes only `_mul2` and the contract name
relative to B6, retaining three Karatsuba products and six real MODEXP calls.
`npm run --prefix contracts g5c3b8` checks the same two public cost cases and
three deterministic Fp2 arithmetic boundaries. All products pass. The Fp2
arithmetic window falls from 9,965 to 8,086 gas (-18.86%), transaction total from
38,887 to 37,022 gas, and window allocator growth from 2,816 to 1,664 bytes.
The unchanged Fp control window stays at 2,167 gas; its transaction total rises
by 8 gas (27,606->27,614), so not every measured total improves.
No Miller loop is run in B8, and the latest full attempt remains B7's 20M OOG.
The following B9 completes the planned integration and prefix pair only.

[G5-C3-B9](docs/G5_C3_MILLER_FP2_PREFIX.md) copies exactly B8's `_mul2` into a
new B7-based Miller variant; all other source, inputs and settings are unchanged.
`npm run --prefix contracts g5c3b9` performs one local deployment and the same
two-step prefix positive/wrong-result pair. The positive matches at 1,132,763
gas (-12.23% vs B7's 1,290,640) and 2,411 memory words (-14.78% vs 2,829).
The wrong target reverts with `INVALID_MILLER_RESULT` at 1,132,581 gas; all 516
MODEXP calls succeed in each case. See the [raw record](experiments/g5_c3_raw/miller_fp2_prefix_evm.json).
There are zero full-loop attempts and `full_attempt` is null. The latest full
attempt remains B7's older variant failing at 20M; B9's new variant has not
been tested through all 64 steps and is not shown to exceed that budget.
Full Miller and exact SnarkPack remain BLOCKED, with null aggregate-verifier
gas. The following B10 completes the planned original `_add2` cost localization;
no full retry, cap increase or final exponentiation is performed.

[G5-C3-B10](docs/G5_C3_FP2_ADD_COST_PROFILE.md) adds only an instrumented entry
point around the unchanged `_add2`. `npm run --prefix contracts g5c3b10` runs
one deployment and one addition of the same fixed public operands. The complete
result matches the references. Its arithmetic window is 1,752 gas, allocator
growth 896 B, and window memory-expansion cost 91 gas, with zero MODEXP calls.
Execution costs 5,740 gas and intrinsic gas is 24,952 (sum 30,692); Prague's
calldata floor raises the actual transaction total to 30,880, adding 188 gas
outside the arithmetic window. See the [raw record](experiments/g5_c3_raw/fp2_add_cost_profile.json).
This is a baseline, not an optimization or evidence of a unique bottleneck.
The latest prefix remains B9's pass and the latest full attempt remains B7's
older-version 20M OOG; B9's full loop is still untested.
The following B11 completes the planned isolated candidate comparison; no
Miller integration/retry, cap increase or final exponentiation is performed.

[G5-C3-B11](docs/G5_C3_FP2_ADD_LOWLEVEL.md) changes only `_add2` and the contract
name relative to B10. `npm run --prefix contracts g5c3b11` checks the same
public addition and one necessary boundary, `((p-1,0),(1,0))`, whose full result
is zero. Both pass. Window gas falls 1,752->849 (-51.54%), execution gas
5,740->4,852, allocator growth 896->384 B (-57.14%), and window memory-expansion
gas 91->39; MODEXP count remains zero. Transaction total does NOT fall: it stays
30,880 as the calldata-floor supplement rises from 188 to 1,076 gas. See the
[raw record](experiments/g5_c3_raw/fp2_add_lowlevel_profile.json).
This is an isolated fixed-input comparison, with no Miller run in B11.
The following B12 completes the planned prefix integration.

[G5-C3-B12](docs/G5_C3_MILLER_ADD_PREFIX.md) integrates the exact B11 `_add2`
into a separate B9-based contract, with the other 51 functions unchanged.
`npm run --prefix contracts g5c3b12` checks the same two-step positive and wrong
target. Positive gas falls 1,132,763->1,043,835 (-7.85%), active memory
2,411->2,079 words (-13.77%), and all 516 MODEXPs still succeed. The wrong
target reverts at 1,043,653 gas. Runtime rises by 11 B and deployment gas by
2,328. There are zero full attempts: the latest actual full run remains B7's
older-version 20M OOG, while B12's complete-loop cost/correctness remain unknown.
See the [raw record](experiments/g5_c3_raw/miller_add_prefix_evm.json).

[G5-C3-B13](docs/G5_C3_FP2_SUB_COST_PROFILE.md) adds only one measurement entry
around the unchanged `_sub2`. `npm run --prefix contracts g5c3b13` performs one
deployment and one subtraction on the same public endpoint x/y. The complete
128 B output matches BigInt and Noble. Its window costs 1,286 gas, execution
5,296, allocator growth 640 B and window memory expansion 65 gas; MODEXP count
is zero. Total 30,880 gas includes a 632 gas calldata-floor supplement outside
the window. Both components use direct subtraction, with a low-limb borrow
only in c0; this sample does not cover modular wrap. This is a baseline, not an
optimization. See the [raw record](experiments/g5_c3_raw/fp2_sub_cost_profile.json).

[G5-C3-B14](docs/G5_C3_FP2_SUB_LOWLEVEL.md) changes only `_sub2` and the
contract name relative to B13. `npm run --prefix contracts g5c3b14` executes
one deployment, the same public subtraction and one necessary boundary:
`(0,0) - (1,2^256) = (p-1,p-2^256)`. Both complete results match BigInt and
Noble. Window gas falls 1,286->765 (-40.51%), execution 5,296->4,790, allocator
growth 640->384 B (-40%), and window memory expansion 65->39 gas; MODEXP
count remains zero. Total transaction gas stays 30,880 as the calldata-floor
supplement rises 632->1,138. See the
[raw record](experiments/g5_c3_raw/fp2_sub_lowlevel_profile.json).

[G5-C3-B15](docs/G5_C3_MILLER_SUB_PREFIX.md) completes that exact `_sub2`
integration and original prefix pair. Gas falls 1,043,835->985,557 (-5.58%)
and active memory 2,079->1,905 words (-8.37%). Both transactions complete all
516 MODEXPs, and the wrong target rejects. Runtime rises 114 B and deployment
24,694 gas. Its historical result contains zero full attempts.

[G5-C3-B16](docs/G5_C3_MILLER_SUB_FULL.md) runs the unchanged B15 contract
once through its full64 entry point, with the original fixture and 20M cap.
`npm run --prefix contracts g5c3b16` performs one deployment and one positive
full-loop attempt only. The actual attempt OOGs: execution 19,963,388 plus
intrinsic 36,612 equals the 20M capped spend. All 10,628 observed MODEXPs
succeed; active memory at exit is 1,945 words. The last root step is pc5374
`SHR` with 2 gas left. The completed-loop gas remains null. See the
[raw record](experiments/g5_c3_raw/miller_sub_full_evm.json).

[G5-C3-B17](docs/G5_C3_MILLER_BUDGET_REVIEW.md) makes that saved-evidence
review reproducible with `npm run --prefix contracts g5c3b17`. It executes zero
EVM transactions. The same-runtime source map locates the terminal instruction,
while derived costs separate 2,125,600 MODEXP-child gas from 17,837,788 gas not
yet attributed to functions. Memory expansion at 1,945 words costs 13,223 gas.
Neither the terminal function nor that memory watermark establishes a hotspot.
See the [derived report](experiments/g5_c3_raw/miller_budget_review.json).

[G5-C3-B18](docs/G5_C3_MILLER_COST_ATTRIBUTION.md) completes that one-run
attribution. It exactly reproduces B16's OOG, 10,628 successful MODEXPs and
terminal PC5374. Charged root opcodes 17,837,786 + MODEXP children 2,125,600 +
exceptional-halt remainder 2 = 19,963,388 execution gas, with zero difference.
`_mul` owns 8,051,757 gas (45.14% of charged root gas); 3,999,108 gas (22.42%)
remains in explicit unmapped/generated/scaffolding buckets. Function attribution
coverage is 77.58%; rankings are exclusive source ownership, not inclusive
caller costs. See the [raw record](experiments/g5_c3_raw/miller_cost_attribution.json).

[G5-C3-B19](docs/G5_C3_MILLER_ROUTE_REVIEW.md) completes the read-only route
review with zero EVM transactions. Even assigning all unassigned costs to the
next mapped function gives 6,154,522 gas, below `_mul`'s 8,051,757. That permits
one source-derived candidate despite the earlier automatic 10% mapping filter:
combine two modular halves into `(t + (t & 3)*p)/4` for the exact fixed modulus.
The low-product carry and subsequent addition carry must both be preserved.
At B19 no Solidity arithmetic or budget was changed, and candidate gas was unmeasured.

[G5-C3-B20](docs/G5_C3_FP_MUL_QUARTER.md) implements that candidate in an
independent profile contract. All five complete Fp products pass. On the
original performance input the window falls 2,167->2,023 gas (-6.65%), execution
4,318->4,174 and total 27,606->27,462. Allocation remains 384 B and MODEXP calls
remain two. The four carry/correction cases are not performance samples.

[G5-C3-B21](docs/G5_C3_MILLER_QUARTER_PREFIX.md) integrates that exact `_mul`
into a new B15-based Miller variant; all 51 other function bodies and remaining
source bytes are unchanged except the contract name. One deployment and the
original prefix pair pass. Positive total gas falls 985,557->951,873 (-3.42%);
active memory stays 1,905 words and each call has 516 successful MODEXPs.
The wrong target reverts with `INVALID_MILLER_RESULT` at 951,691 gas.

[G5-C3-B22](docs/G5_C3_MILLER_FULL_DECISION.md) completes a zero-EVM decision:
one bounded full64 attempt on the saved quarter runtime can close its remaining
current-version evidence gap. The old full failure does not test this runtime;
two prefix steps do not exercise the full loop or terminal conjugation.
This is not a forecast from the 3.42% prefix saving, nor a prerequisite for the
manuscript's existing restricted claims. On OOG or other failure, save evidence
and freeze this fixed-budget software route's automatic arithmetic/full retries.

[G5-C3-B23](docs/G5_C3_MILLER_QUARTER_FULL.md) executes that exact limited
plan. Deployment succeeds at 1,583,679 gas with the same B21 runtime. The one
full64 attempt hits root OOG: 19,963,388 execution + 36,612 intrinsic = 20M,
zero refund and empty return. All 11,076 observed MODEXPs succeed (2,215,200
child gas); active memory is 1,945 words. No prefix, boundary or full retry ran.

Current checkpoint (2026-10-27): B23 is the latest actual full attempt; B21
remains the passing prefix. The new quarter version is now tested and budget-
blocked, not untested. Status is
`BLOCKED_QUARTER_SOFTWARE_MILLER_EXCEEDS_20M_FROZEN`. In accordance with B22,
automatic arithmetic optimization and full retries on this fixed20M software
route are frozen; the rest of the project is not frozen. Failed capped spend
does not measure completed-loop gas or completion percentage.

That B23 next-task note is historical. The P3 partial-turnout correction and
its v2 EVM acceptance have now passed in G4-C/G5-D. The next project task is
native B=8 distinct-voter aggregation after G1-C2's B=4 pass, independent of
the frozen verifier route.
There is no automatic cap increase, new primitive
search, final-exponentiation experiment or wrapper implementation. A different
verifier architecture requires a new scoped decision and resource plan.
Full Miller, final exponentiation, GT binding and exact SnarkPack remain
unfinished; aggregate-verifier gas is null.

Raw results are written to `experiments/raw/`; the summary script writes
`experiments/aggregate/summary.{json,md}`. Runs append to `runs.csv`, so remove
or archive that file before a fresh paper measurement campaign.

## Fixed four-voter fixture

Weights `(3,2,1,4)` and choices `(A,B,A,C)` give the private tally `(4,2,4)`.
The public winner is candidate `1`, because the protocol selects the smallest
identifier on a tie. Every one of the ten legal 3-of-5 trustee subsets is
checked against this result.

## Gate interpretation

- G0 native manifest/core and P1 can be reproduced here.
- G0.5 reached a real single-equation R1CS preflight. The first optimization
  reduced 3,501,941 constraints to 3,461,084, still above the 1,000,000 limit,
  so Groth16 was not run.
- G0.6 measured an isolated native-field alternative: the CP equation used
  9,571 constraints and generated a real proof. It does not recompute the CP
  transcript hash and does not replace the official Ristretto manifest.
- G0.7 reproduced the native core and P1 in Release mode. G0.8 now has the
  isolated Jubjub Suite v2, registration commitments and authenticated close,
  single-process Pedersen VSS/GJKR, native generalized CP, authenticated
  private packages and winner-only publication. G0.8-E2 binds the registration
  policy before DKG and the verified close record into the tally target. This
  completes the planned native G0.8 path, not the ballot circuit or full P3.
- G0.9-H0 freezes the exact 684-byte registration-bound CP transcript and
  checks a real R1CS byte-equality boundary. The temporary H0 relation uses
  16,416 constraints; it does not compute SHA-512 and is not P3.
- G0.9-H1 recomputes the frozen transcript's SHA-512 digest in a real R1CS
  using 890,920 constraints. The circuit matches the native digest and rejects
  a changed transcript or public digest. It does not implement hash-to-scalar
  rejection sampling, CP equations, Groth16 or P3.
- G0.9-R reproduces G0.8/H0/H1 from clean commit `39cca56` using Rust 1.99.1,
  locked dependencies and Release `opt-level=1`, `strip=none`. It freezes the
  existing Jubjub suite for H2/H3/H4 and keeps Ristretto as a legacy baseline.
- G0.9-H2-A constrains the exact H2S-v2 wrapper, 252-bit mask, strict scalar
  range and first-valid rule for real counter-0 and counter-1 transcripts. The
  measured relations use 1,013,462 and 2,019,916 constraints, respectively.
  Circuit shape still depends on the accepted counter; this is not full H2,
  CP integration, a Groth16 run or P3.
- G0.9-H2-B always evaluates two attempts. The real counter-0 and counter-1
  paths both use 2,020,168 constraints and the same public/witness layout. This
  validates fixed-shape selection but does not replace the Suite v2 bound of
  128 attempts, so full H2 remains incomplete.
- G0.9-H2-C evaluates a versioned single-SHA-512 candidate that reduces all
  512 digest bits modulo Jubjub Fr. It uses 1,148,545 constraints, 43.146% less
  than H2-B, and rejects a changed transcript. H2-C itself was a candidate
  result; H2-D subsequently adopts the construction under a final suite ID.
- G0.9-H2-D pins `VERIVOTING-JUBJUB-CP-H2F-G0.9-v3` in the prototype manifest.
  Its final fixed circuit uses 1,151,661 constraints. Native CP verifies 5/5
  proofs and all ten threshold subsets; the registration-bound E2E still emits
  winner 1 without a public tally. This completes standalone H2.
- G0.9-H3 connects H2-D to `zG=A0+dVK`, binds the transcript's canonical `VK`
  and `A0` encodings in R1CS, and rejects a changed transcript, wrong key and
  wrong response. It uses 1,168,855 constraints and is not the coordinate
  vector, Groth16 or P3.
- G0.9-H4-A reuses the same challenge for all three coordinate equations,
  binds 11 transcript points and completes one Trustee's CP vector in
  1,207,066 constraints.
- G0.9-H4-B runs real Groth16 setup, prove and verify over that exact relation.
  Setup took 14,025.404 ms, proving 8,206.572 ms and verification 3.017 ms;
  the proof is 192 bytes and peak RSS is 2,886.219 MiB. A changed public
  transcript is rejected. This remains one Trustee and is not P3.
- G0.9-P3-A freezes `VV/JUBJUB/P3-A/INPUT-LAYOUT/v1` and combines three
  distinct selected Trustee CPs in one R1CS. Public statement prefixes remain
  public, while shares, CP commitments and responses remain witness-only. The
  relation uses 3,611,598 constraints, 45 public inputs and 3,485,994 witness
  variables; the latest witness synthesis took 4,876.174 ms with 3,258.906 MiB
  peak RSS. Real Groth16 setup took 36,858.792 ms, proving 23,581.980 ms and
  verification 5.349 ms; the proof is 192 bytes and peak RSS is 5,824.047 MiB.
  A changed public prefix is rejected. Package openings, threshold combination
  and winner logic are not part of P3-A.
- G0.9-P3-B0 replaces the private-package message's variable-attempt H2S with
  `VERIVOTING-JUBJUB-PACKAGE-H2F-G0.9-v1`, a fixed-shape SHA-512-to-field
  mapping. Each 602-byte selected package produces a 725-byte H2F input over
  six SHA-512 blocks and uses 1,000,827 constraints; all three native/R1CS
  values match and a changed package is rejected. The migrated native E2E
  still verifies 5/5 packages, selects Trustees 1--3 and emits winner 1. This
  gate does not yet constrain the Pedersen opening equations inside P3.
- G0.9-P3-B1 constructs each 602-byte private package directly from the P3-A
  witness, recomputes three Package-H2F-v1 values and constrains three public
  Pedersen commitments with private openings. Together with the 12 CP
  equations, the relation uses 6,191,178 constraints, 51 public inputs and
  5,966,673 witness variables; the final witness synthesis took 9,234.655 ms
  with 5,661.500 MiB peak RSS. All three native packages and signatures verify,
  and a changed third commitment is rejected. No Groth16, threshold
  interpolation, secret tally or winner relation is claimed.
- G0.9-P3-C0 applies the fixed Trustee 1--3 Lagrange coefficients `(3,-3,1)`
  to the already-bound decryption shares. Three canonical aggregate `C1`
  points are public and three resulting plaintext group elements remain
  private. The complete relation uses 6,201,342 constraints, 57 public inputs
  and 5,974,593 witness variables; witness synthesis took 8,407.870 ms with
  5,607.562 MiB peak RSS. The native threshold result matches the fixed
  four-voter fixture, and a changed third private plaintext point is rejected.
  Bounded tally binding, winner logic and Groth16 are not part of P3-C0.
- G0.9-P3-C1 adds three private 64-bit tally witnesses, proves
  `M_j=t_jG`, constrains `0<=t_j<=W`, and enforces `sum(t_j)=W` for public
  `W=10`. The relation uses 6,203,703 constraints, 58 public inputs and
  5,976,943 witness variables; witness synthesis took 8,806.231 ms with
  5,590.266 MiB peak RSS. A sum-preserving replacement `(5,1,4)` is rejected,
  and neither plaintext points nor tallies are emitted. Winner logic and
  Groth16 are not part of P3-C1.
- G0.9-P3-D0 reuses those exact tally bits to constrain public winner `1`
  under the highest-weight, smallest-identifier-on-tie rule. The complete
  relation uses 6,205,629 constraints, 59 public inputs and 5,978,859 witness
  variables; witness synthesis took 8,566.464 ms with 5,607.875 MiB peak RSS.
  Wrong public winner `2` is rejected. The complete relation is now available
  in R1CS, but a combined Groth16 proof has not been run.
- G0.9-P3-D1 runs real Groth16 over that exact fixed-123 relation. Setup took
  64,668.553 ms, proving 43,604.668 ms and verification 6.019 ms. The proof is
  192 bytes, the compressed proving key is 1,837,594,176 bytes, the verifying
  key is 3,224 bytes and peak RSS is 7,779.359 MiB. The valid proof verifies
  and changing only the public winner from `1` to `2` is rejected. The proving
  key is measured in memory and not persisted.
- G1-B0 constrains one registered-weight commitment opening, a three-candidate
  one-hot choice and six Weighted ElGamal group equations. It uses 46,573
  constraints, 17 public inputs and 39,940 witness variables; witness
  synthesis took 108.856 ms with 49.156 MiB peak RSS. The valid relation passes
  and a two-hot witness is rejected. Election/context binding, PRF address and
  region, nullifier, registration-root membership, Groth16 and SnarkPack are
  deliberately absent, so this is not the paper's complete `C_vote`.
- G1-B1 composes those equations with two HMAC-SHA-256 PRFs, deterministic
  four-region assignment and the exact SHA-256 registration leaf/node encoding
  for a four-leaf depth-two tree. The complete fixed-suite five-condition
  relation uses 854,454 constraints, 25 public inputs and 839,832 witness
  variables; witness synthesis took 777.431 ms with 754.859 MiB peak RSS. The
  actual registration-close root matches and a changed public final context is
  rejected. This is complete R1CS for the fixed experiment suite, not a ballot
  Groth16 proof or a variable-depth production registry.
- G1-B2 runs real Groth16 over that exact relation. Setup took 8,015.162 ms,
  proving 4,623.984 ms and verification 2.386 ms. The proof is 192 bytes, the
  compressed proving key is 251,898,000 bytes, the verifying key is 1,592
  bytes and peak RSS is 1,880.547 MiB. The valid proof verifies and changing
  only the public final context is rejected. The proving key and private
  witness are not persisted.
- G1-C0 runs bellperson 0.27.0 SnarkPack V2 over two independently randomized
  G1-B2 proofs after canonical arkworks-to-blstrs conversion. Aggregation took
  4.003 ms and verification 3.153 ms; changing the first public context was
  rejected. The 5,236-byte aggregate exceeds the two 192-byte source proofs,
  so this minimum-batch result is compatibility evidence, not an efficiency
  claim. The deterministic aggregation SRS is not production setup.
- G1-C1 reuses the frozen 25-field public-input layout and aggregates complete
  ballot proofs for registered voters 1 and 2 in region 4. Their public
  statements, nullifiers and ciphertexts are distinct while election context
  and registration root agree. Aggregation took 3.908 ms and verification
  2.955 ms; replacing the second public nullifier was rejected. This remains a
  minimum-batch functional result with a non-production aggregation SRS, not a
  scale or compression claim.
- G2-A consumes those exact two public records, recomputes their ordered batch
  root and constrains the six old-to-new ciphertext components. The relation
  uses 1,360,002 constraints, 37 public inputs and 1,329,560 witness variables;
  witness synthesis took 1,145.101 ms with 1,196.984 MiB peak RSS. A substituted
  ciphertext is rejected. This is R1CS evidence only; state Groth16 is G2-B.
- G2-B runs real Groth16 over that exact relation. Setup took 13,254.145 ms,
  proving 8,776.350 ms and verification 2.660 ms. The proof is 192 bytes, the
  compressed proving key is 419,767,248 bytes, the verifying key is 2,168 bytes
  and peak RSS is 3,090.062 MiB. Reusing the valid proof after changing only
  the public batch root is rejected. The proving key and private witness are
  not persisted.
- G3-A runs five concurrent Trustee processes with five persisted mode-`0600`
  private states. The processes exchange 25 session-bound signed VSS messages,
  agree on one transcript, KeyCore and public key, and Trustees 1--3 issue three
  signed decryption-share vectors. DKG-to-final took 57.281 ms and fixed-subset
  threshold decryption 27.352 ms; the recovered tally is `(4,2,4)` and winner
  `1`. This is an authenticated local-file happy path, not restart recovery or
  remote networking.
- G3-B stops Trustee 3 after its private checkpoint and resumes it under a new
  PID. The resumed process authenticates the checkpoint hash, preserves the
  G3-A transcript, KeyCore and public key, and again recovers `(4,2,4)` with
  winner `1`. This covers one orderly local restart, not a crash matrix or
  remote deployment.
- G4-A confirms an equal complete-P3 count shape for `{1,2,3}` and `{1,2,4}`.
  G4-B then binds those two choices with one public selector and uses one real
  Groth16 setup generated from `{1,2,3}` to prove `{1,2,4}`. The relation has
  6,231,973 constraints, 60 public inputs and 6,005,106 witness variables;
  setup took 61,062.475 ms, proving 38,615.505 ms and verification 6.529 ms.
  The proof is 192 bytes, peak RSS is 8,177.266 MiB, and changing only the
  selector public input is rejected.
- G5-A verifies the real G2-B proof in Solidity on local Prague EthereumJS VM.
  Its constructor-pinned BLS12-381 VK has 38 IC points. The valid transaction
  uses 940,640 gas and 1,828 calldata bytes; changing only batch-root public
  input 7 reverts.
- G5-B uses the same Solidity source with its own pinned P3 key and 60 public
  inputs. The real `{1,2,4}` disclosure proof succeeds at 1,331,473 gas and
  2,564 calldata bytes; changing only selector input 51 reverts. Full batch DA,
  SnarkPack EVM verification, registry bindings and settlement writes remain
  unimplemented. Arbitrary all-ten-subset P3 also remains outside the current
  experimental scope.
- A blocked gate is a development result, not a cryptographic benchmark value.
- These experiments test functional behavior and cost. They do not establish
  Function-Equal privacy or the paper's joint extraction theorem.

See [docs/IMPLEMENTATION_STATUS.md](docs/IMPLEMENTATION_STATUS.md) for the
claim boundary and [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md) for dependency
choices.

