# Next steps: user's five-step route (2026-10-30)

## Current work: G9 source/evidence route review completed; stop here

[G9路线审查](G9_EXACT_VERIFICATION_ROUTE.md)：多交易精确验证仅作为后续设计候选，
资源可行性未证明，完整实现仍BLOCKED。原软件Miller路线冻结，不重跑20M full、不扫描原语。
本轮未实现合约/编译/EVM/新证明，未提高协议能力标记；来源与文档归档完毕。

若用户继续，必须先把候选路线闭合为具体设计，而非直接启动半个验证器：

1. 规范wire/GT解码与原Fiat–Shamir编码、全部TIPP/MIPP/KZG/聚合等式必须有完整清单。
2. 明确最终幂和安全的方程检查策略；不把固定测试RNG或host目标值当验证结果。
3. 列出每个续算阶段的合约计算/持久状态、终止条件、整体交易数及时间预算；未知预算不准自动扩。
4. 新接收器仅在完整验证后一次消费同一批次，并原子检查状态证明及NF/CT；不得绕过原认证。
5. 只有设计/预算闭合并另行批准后，复用G1-D0 W32 B2做一正一负；它不等于现有B4结算验收。

单交易20M和block30M仅是本地研究配置，不当作实时公链永久上限。当前总费用/交易数
没有实测或可信调度，保持null。若设计无法闭合，保留受阻及论文收窄声明，不虚构效率。

本轮结束，不自动开发/试跑分阶段验证器、wrapper、G10或新的性能实验。

## Previous component: G8-E lifecycle accounting archived; stop here

[G8-E](G8_E_LIFECYCLE_COSTS.md)：复用G8-D一次真实执行的21笔收据，完成分类、
Gas及通信字节守恒、固定来源/赢家状态核对。没有补跑EVM或生成新证明。
成功17笔65,473,608 Gas；部署34,082,689，非部署成功业务31,390,919。
失败测试2,263,025 Gas不进入成功路径，离线证明/私密交付仍未重测，不能填零。

本轮在G8-E停止。不自动开始G9/G10，不扩批、不重新跑G1-E、不改密码学关系或VK。

下一轮应先给出G9精确SnarkPack EVM的有限范围路线决策方案：

1. 阅读被冻结的精确验证失败证据及现有真实接口，确认尚缺哪一段。
2. 在明确Gas/资源预算下比较能够维持原验证语义的候选路线；不能跳过验证或静默换方案。
3. 先确定路线和验收边界，再决定是否编写一次最小可行实验；不重启无目的软件Miller扫描。
4. 无可行路线时如实保留受阻状态，论文只报告已完成的本地逐票备用路线。

G10完整公平比较和最终版本复现后置；当前G8-E分类费用不是完整SnarkPack系统效率结论。

## Previous component: G8-D one-time Winner written; stop here

[G8-D](G8_D_WINNER_ONCE.md)：基于G8-C提交2ce1cae，新接收器继承原批次/冻结逻辑，
固定P3组件，存储重建60输入及原证明真实验证后在同一交易写Winner一次。21笔交易、
一个正例及四个必要原子拒绝，Winner=1/事件一次；新Setup/Prove/CP/签名为0。
披露实测2,073,560 Gas、644 B；不等于全系统性能优势。G8-C/raw/旧源码不覆盖。

下一轮只做G8-E：

1. 固定同一G8-D版本及部署/认证/接收/冻结/承诺/披露清单，明确复用数据与信任根。
2. 从同一次完整本地执行记录成功生命周期成本，部署、拒绝测试、离线证明成本分开。
3. 公共输入、交易data和去重通信字节口径分别列出，不混入G7/G8-C历史组件Gas。
4. 先审计现有21笔是否满足记录需求；仅在缺少必要证据时补运行，不重复G1-E或扩批。

G8-D到此停止，不自动进行G8-E/G9/G10。精确SnarkPack EVM仍未完成，当前批次是逐票
Groth16备用路线。一次披露作用于指定权威receiver实例，不是全局阻止克隆合约。

## Previous component: G8-C certified frozen-storage P3 passed; next only G8-D

[G8-C](G8_C_FROZEN_P3_BINDING.md)：新 Gate V2 只加必要认证读取，原认证算法保持不变；
新 receiver 在自身接收/冻结入口强制认证，不是可绕过 facade。固定原 B4 根、选票/状态 VK，
P3 固定 G4-D Ark/EIP 双编码来源与 EIP SHA-256/Keccak。实际 CT2 和认证承诺生成60项输入，
与 G6/G7 均一致；原512 B证明验证通过。28 签名交易、10 原子拒绝、440 历史归档项未变。
本轮到 G8-C 停止；未写 Winner、未新 Setup/Prove，未触碰 G9。

下一轮唯一任务 G8-D：

1. 新增一次性披露结算入口，固定连接本轮 receiver/commitment/P3 组件，不允许 caller 选 verifier/VK。
2. 检查尚未披露，调用本轮真正从冻结存储构造输入的 verifyFrozenP3，成功后同一交易写 Winner 一次。
3. 复用 G6 原证明；错误 Winner 和重复披露必须正常拒绝，失败前后 Winner/计数/冻结状态不变。
4. 不公开 tally、CP opening、usable shares，不将通用 view 验证器的重复调用视为重复 Winner 写入。

通过后停止；G8-E 完整交易闭环/成本，G9 精确 SnarkPack EVM，G10 公平比较与干净源码复现后置。
本次644 B calldata比G7少1920 B，但2,058,601 Gas高于G7独立组件；如实记录，不预设优化收益。

## Previous component: G8-B five-person acceptance (G8-C plan below is now completed)

[补充验收](G8_B_FIVE_TRUSTEES.md) 从本地已提交 `e3b7cfb` 建立
`experiment/g8b-five-trustees`，不从上传评估的旧 `ac021610` 重做 G8-B。
5/5 原承诺签名和 KeyCore 份额通过真实 EVM；认证集合 `{1,2,3}` 不变，承诺顺序 `4,5,3,1,2`，
先由不同集合 `{3,4,5}` 达到门限。新四项必要原子拒绝、411 历史归档项核对通过。
不改原合约/字节码/密码学算法，不生成新 CP/签名/Setup/Prove。本轮到该补充验收停止。

唯一下一步仍是下面列出的 G8-C：接收前强制认证 + 存储重建 60 输入 + 复用 G6/G4-D 证明/VK。
不能把五人承诺认证、或不同集合达到门限，写成已完成五份 CP/P3 或新的门限解密实验。

## Previous component: G8-B first-three acceptance; G8-C plan remains current

[G8-B](G8_B_TRUSTEE_COMMITMENTS.md) 从 `ac021610` 建立独立
`experiment/g8b-trustee-commitments`：五份 G6 公开记录解码及原生验证，三份原承诺签名在 EVM
验证并存储；验证份额由已认证 KeyCore 系数实际执行多项式群等式检查。
同一次 Prague VM 流程真实认证证书、验证两批旧 B4 证明并冻结 CT2，然后绑定 target/承诺。
10 必要原子拒绝、48 Rust 回归、严格 Clippy、376 条历史归档项检查通过；0 新 CP/重签/Setup/Prove。
完整 G8 仍 PARTIAL，当前到 G8-B 验收停止。

下一轮只做 G8-C：

1. 保留旧 receiver/证书组件原字节，新增整合入口，在批次接收前强制相同原证书认证和上下文绑定。
2. 复用本轮已认证 KeyCore、身份/验证份额、冻结 CT2/CP target 和承诺，从真实合约存储重建
   完整 60 项 P3 公开输入；不能允许调用者替换公钥、密文、target 或承诺数组。
3. 逐项检查与 G6 输入兼容，使用 G4-D 原 VK、G6 已归档真实证明完成 P3 EVM 验证；
   CP opening/usable shares 保持私密，在现有 P3 内证明，不公开传入合约。
4. P3-ready 只是记录数门限，不代替 CP/P3 检查或 Winner 授权。一次性 Winner 写入留给 G8-D，
   完整交易闭环和总成本留给 G8-E；不拼接历史 Gas、不声称精确 SnarkPack EVM 已完成。

不增加 B8/B64、新性能图、公链部署或历史证明重跑；G9/G10 按原顺序后置。

## Previous component: G8-A certificate gate (the following G8-B plan is now completed above)

[G8-A](G8_A_ORIGINAL_CERTIFICATE_AUTH.md) 从已提交 `1c19f811` 建立
`experiment/g8-auth-disclosure`，完成原 Jubjub Schnorr/SHA-512 的真实 EVM 证书门控。
三个不同 certifier 达到原 `q_cert=n-t=3`；原 authority 注册签名及关闭/disclosure 摘要通过，
保留独立解密角色 `tau=t+1=3`。12 原子拒绝、13 hash/6 原挑战、48 Rust 回归、严格 Clippy 通过。
0 SNARK Setup/Prove；原 DKG signing 通过确定性重建执行，原注册 certificate 字节匹配 G6。
本轮到独立认证组件停止，原 phase-one receiver 尚未 enforce 门控，完整 G8 仍 PARTIAL。

下一轮只做 G8-B：

1. 原 Trustee package commitment 签名验证/存储，绑定同一已冻结 Election Context/CT2/target。
2. 认证身份与 verification share 必须绑定已认证 KeyCore（由签名认可的公开系数派生/核对），
   不得把调用者提供的新验证份额或新身份公钥直接当认证值。
3. 不重复 Trustee、不足解密门限/错上下文拒绝；不要把建钥 `q_cert` 与解密 `tau` 混成一个角色。
4. CP commitment opening 不公开：在后续 P3 中证明同一已认证承诺的私密打开与 CP。

之后再整合强制证书 gate 的新 receiver、从冻结存储构建 60 项 P3 输入并一次写 Winner，
优先复用 G6/G4-D 同一证明/VK。G9/G10 仍按原顺序后置，不扩批、不重跑旧证明或 Miller。

## Previous component: G8 phase 1 passed; authentication/disclosure follow-up

[G8 receiver/freeze](G8_WINNER_ONLY_SETTLEMENT.md) 从 `9baed479` 新建
`experiment/g8-winner-only-settlement`。实际合约验证八份原始单票证明与两份状态证明，
连续接收两个 B4 后冻结 CT2；同一 G6 CP target 和 seal 一致。13 原子拒绝通过。
这是独立明确的逐票 Groth16 备用/对照路线，不是 SnarkPack、mock 或仅 State Proof。
0 Setup/Prove，不重跑 G1-E/G4-D/G6/G7；本轮到该阶段停止。

下一轮仍在 G8 内，只补：

1. 原 DKG/注册证书及 Trustee Jubjub Schnorr 认证的真实验证；当前锚点只是部署者预置。
   不换成 secp256k1 重签后声称已验证原认证，不把主机检查当作链上检查。
2. 已认证承诺与同一冻结上下文绑定，从合约存储重建全部 60 个 P3 输入并调用 G4-D VK。
3. 只写 Winner 一次；伪造签名、改承诺/冻结目标、重复披露必须拒绝且不写状态。

这些未完成前 G8 始终 PARTIAL。G9 精确 SnarkPack 路线独立待解，不重启冻结的 Miller 局部优化；
G10 公平系统比较和最终干净版本复现后续进行。不要增加规模、重做旧统计或拼接组件 Gas。

## Previous checkpoint: G7 completed

[G7](G7_P3_EVM.md) 使用 G6 同一份已归档 P3 proof、G4-D 原 VK（60 输入），
在未改 Solidity 验证器上执行真实本地 Prague 签名交易。正例成功，Selector/Winner/合法点
密文替换/实际旧 VK 四项配对负例拒绝；部署 6,631,884 Gas、验证 1,331,401 Gas、
ABI calldata 2,564 B。未 Setup/Prove，未重跑 G1-E/G4-D/G6。48 测试/严格 Clippy 通过。

本轮唯一目标已达成，到此停止。下一轮 G8 必须实际实现批次/旧状态/NF 检查、冻结后禁止接收、
对照存储重建输入、Trustee 认证签名及承诺绑定、一次性 Winner 写入；不能把 G7 的公开验证器
视为这些状态检查已经实现。真实多进程 Trustee 私密交付仍未接入 G6。
G9 精确 SnarkPack EVM 路线独立待解，不绕过、不重启已冻结软件 Miller 循环；
G10 完整公平系统比较、最终干净版本复现后续进行。无新规模或重型证明重跑。

## Previous native checkpoint: G6 completed

[G6 report](G6_NATIVE_WINNER_ONLY.md): one actual native run verifies both G2-E
batches, freezes the receiver's CT2, generates five new private CP packages,
privately decodes subset 123, creates one new G4-D P3 proof and writes winner
once. Target substitution, repeated disclosure/freeze and post-freeze receive
reject without writes; a separate-process public-only replay also passes.
Eight ballot proofs/two aggregates/two state proofs are reused, not re-proved.
Full VK and R1CS match G4-D. 48 relevant tests/strict Clippy pass.

The same-VK/proof local EVM follow-up is now complete in G7 above, keeping
old fixed-123 G5-D data historical. Next implement actual contract receiver,
freeze and one-time winner storage checks. Exact SnarkPack EVM still needs
a feasible exact route; do not restart the frozen software-Miller loop or use
mocks. No new batch scales, G1-E reruns or spliced component Gas comparisons.

G6 is serial in-memory and same-process private delivery, not persistent
multi-process service, production privacy, full-chain settlement or a privacy
composition theorem. System comparisons and final clean-source reproduction
remain after the contract work. The earlier Step 3 specification below is
now completed by G6, not still an open native task.

## Historical comparison preparation without new heavy experiments

At the user's request, [Stage 1 comparison](COMPARISON_STAGE1.md) now reuses
G1-E's six immutable statistical figures, adds version-scoped literature
function/security tables, and plots the existing G2-D/E functional samples.
G2-D uses matched component sums, not separately timed transactions; G2-E has
no measured full Direct Fold receiver baseline. No cross-paper timing ranking,
new proofs, rerun G1-E, new batch scale or inferred EVM Gas is included.
That preparation did not itself complete Steps 3/4. G6 has since completed
the native Step 3; the contract work in Step 4 remains unfinished.

## 1. Completed: G2-E consecutive B4 state transitions

Same D5/W32 D2 pool, S0 -> S1 -> S2. Two real aggregate/state verifiers pass;
nonzero continuity and independent flat eight-ballot sum match. Skip/replay/reset
reject in the native in-memory checkpoint. Report: [G2-E](G2_E_MULTIBATCH_STATE.md).

## 2. Completed now: G4-D complete P3-v2 ten subsets, one circuit/VK

Report: [G4-D](G4_D_TEN_SUBSETS_P3_V2.md). Ten real NEW Groth16 proofs for
123,124,125,134,135,145,234,235,245,345 use one new relation, one fresh setup,
one PK/VK and the SAME accepted W32 CT2. Actual full A/B/C matrix hashes equal
for all ten, not only counts. Selector binds CP transcript IDs and Lagrange
bits IN R1CS; CP/package H2F, opening, tally-v2/slack and winner stay constrained.
Public replay 10/10 and necessary selector/winner negatives pass.
This is not production registry adoption, arbitrary n/t or a privacy theorem;
the G4-D VK had not yet been tested on EVM at that historical checkpoint.
G7 now verifies G6's proof with this VK; old G5-D fixed-123 v2 Gas remains historical.

## 3. Completed by G6: complete native Winner-only state machine

1. Feed the exact two accepted G2-E batches through the real native receiver;
   check both real proof families, context/root/next/old state and spent NFs.
2. Freeze the resulting CT2 ONCE, with the same certified DKG, registration-close
   record, candidate order, weight32 and disclosure policy. Do not splice W10
   material or merely name a static JSON file as a live freeze operation.
3. Generate and privately deliver authenticated CP packages for that frozen
   target; verify CP and openings before selecting three legal Trustees.
4. Bounded decode privately, build the same G4-D relation and generate/verify
   one actual winner proof. If reconstructing parameters, compare exact VK;
   if reusing public material, report reuse and all compatibility checks, not
   a fresh proving time. Do not re-prove all ten subsets just for integration.
5. Accept only inputs equal to frozen storage, write winner once and never
   publish usable shares, openings, private packages or the private tally.
6. Add only the necessary frozen-target substitution and repeated-disclosure
   rejection. Keep existing G2-E order/NF checks instead of a new attack matrix.

Stop after this one native integrated functional acceptance. Record exact
source/parameter/input provenance and actual costs; functional native linkage
does NOT establish live contract settlement or the full privacy composition.

## 4. Then complete actual contract settlement

Contracts must really enforce certified contexts/registry/accepted roots,
next/old-state equality, spent nullifiers, target freezing and one-time winner
writes with the correct G4-D key. Prior component verifiers do not do this.
Exact SnarkPack EVM remains BLOCKED under the historical 20M implementation
budget; select a feasible exact route BEFORE claiming completion. Do not
restart the frozen software-Miller trial loop or replace it with mocks.
No production ceremony or real public-chain deployment is completed.

## 5. Last: paper's fair end-to-end comparisons

Reuse G1-E; no repeat or B64/128 scan. Compare equal-parameter/equal-security:
individual vs aggregate validity; Direct Fold vs state proof; Winner-only vs
Full Tally at the SAME frozen target; actual complete Gas/calldata/nonoverlap
wire material. Setup/proving and online verification are separate. Do not
sum unrelated v1/v2/W10/W32 Gas or infer total settlement from components.

No B8 state circuit, new batch scales, unfocused arithmetic optimization or
new figure campaign while Steps 3/4 still need functional integration.



## Adjusted evaluation rules for subsequent work

- Reuse the existing JSON and summary pipeline. Only add minimal adapters and
  compatibility checks needed by actual measurements; no wholesale framework
  rebuild or mandatory seven-figure target.
- Fair transition comparison is SnarkPack plus DIRECT FOLD versus SnarkPack
  plus STATE PROOF. A variant with no accumulation check is only an explicitly
  weakened security ablation. Native fold timings are not target-chain Gas.
- Accepted-batch consistency and aggregator censorship resistance differ.
  The current paper proves properties within accepted batches, not inclusion
  of every ballot delivered to an aggregator. An independent accepted-intake
  commitment/log is a separate integration feature; do not claim it is already
  enforced by state R1CS or confuse it with the registration root.
- Stale old state, spent nullifiers and registry equality need their actual
  receiving/settlement layer. Test each necessary invariant when that module
  is implemented, rather than rerunning a full attack matrix at every B.
- After functional B=32 acceptance, collect independent repetitions under the
  frozen suite. Separate setup, proving, verification backends and process RSS;
  report raw samples and median. Ten samples are an initial floor, not a strong
  tail estimate: label P95 cautiously or increase repetitions when it matters.
- Count actual nonoverlapping wire payloads; do not add the same ciphertext
  again as both public inputs and DA. Missing EVM/DA/Gas is null, not zero.
- Depth scans, larger Trustee profiles, B=256, alternative Gro16.Agg and public
  testnets are later/optional tasks. Align the paper's promised scope before
  treating omitted experiments as completed. Preserve its two main research
  questions; the plan's four evaluation axes need not replace their numbering.

## Historical G5 evidence and frozen-route checkpoint

Current checkpoint (2026-10-27): B23 runs the exact B21 runtime once at20M and
reaches root OOG, with 11,076 successful MODEXPs and no child failures. B23 is
the latest actual full attempt; B21 remains the passing prefix. Completed-loop
gas is still unknown/null. The precommitted fixed-budget software arithmetic/
full-retry route is now frozen. Next is read-only evidence/claim alignment and
selection of one independent native-batch increment, not another verifier run.

The current native core and real P1 Groth16 path pass. Historical gates and
remaining boundaries are listed below. The frozen verifier subroute does not
prevent independent native-batch work; mocks must not replace measurements.

G4-B is complete for exactly two ordered subsets. One public selector binds the
CP transcript Trustee IDs and derives the Lagrange coefficients for `{1,2,3}`
or `{1,2,4}`. A setup generated from the first assignment successfully proves
and verifies the second assignment in the same 6,231,973-constraint circuit.
This is not arbitrary 3-of-5 support; the other eight subsets remain outside
the implemented gate and the Manifest is unchanged.

G5-A is also complete: the 37-input G2-B state proof verifies in a real
Solidity contract executed by local Prague EthereumJS VM. Valid verification
uses 940,640 total gas and 1,828 calldata bytes; changing only batch-root input
7 reverts. Full batch DA, SnarkPack/P3 verification and settlement writes are
not included in that measurement.

G5-B is complete: the real 60-input G4-B P3 proof verifies in the same Solidity
source deployed with its own pinned key. Valid verification uses 1,331,473 gas
and 2,564 calldata bytes; changing only selector input 51 reverts. It remains
limited to the two-subset cryptographic relation, without registry comparisons
or winner-state writes.

G5-C1 is complete: the real distinct-voter B=2 aggregate, source VK, public
verifier SRS and exact external statement transcript are exported. Decoded
native verification passes and a changed second nullifier rejects. The
external Rust/JavaScript bytes match and four real Prague primitive probes
pass. This is not a SnarkPack EVM verifier: GT/Fp12 operations and comparison
against non-unit GT targets are missing from the current contract route.
See `docs/G5_C1_SNARKPACK_ROUTE.md`; exact SnarkPack verification stays BLOCKED
and its gas is not measured.

G5-C2 is complete: six actual official verifier calls per case match independent
JavaScript decoding of the saved aggregate wire proof (12/12 calls). Full base
bytes, accepted counters and canonical challenges match, including three real
rejected attempts. The same proof with only the second nullifier changed is
rejected by the complete native verifier. No setup/prove was rerun. This is
host-language transcript conformance, not SnarkPack EVM verification. See
`docs/G5_C2_TRANSCRIPT_CONFORMANCE.md`.

G5-C3-A is complete as a multiplication COMPONENT only. Two real proof-derived
GT operands produce the same full Fp12 product in native blstrs, independent JS
and Solidity in local Prague EVM. The valid signed transaction uses 609,940
total gas; a canonical wrong target reverts. Each call executes 108 real
MODEXP operations. This is not a full GIPA subexpression or pairing-target
verification, so complete G5-C3 remains unfinished. See
`docs/G5_C3_FP12_PRODUCT.md`.

G5-C3-B has a passing actual-endpoint native/JS conformance component, but not
an exact EVM pairing-to-GT implementation. Both implementations compute the
same twelve Fp coefficients and reject one wrong in-subgroup GT target.
All six C2 challenges are independently reconstructed from the original wire.
The actual endpoint's Prague pairing probe returned false (nonunit), consuming
70,300 primitive gas; that boolean ABI never receives a GT target. The missing
software Miller loop/final exponentiation was an implementation gap at that
gate; the following B2 experiment now records a concrete attempt and budget
failure. See `docs/G5_C3_TIPP_ENDPOINT.md`.

G5-C3-B2 adds a real software Miller-loop contract. The first two steps cover
doubling, the first signed addition, square and three line products; the signed
transaction matches all reference coefficients using 4,805,196 total gas.
A wrong canonical result reverts. One full 64-step transaction hits the fixed
20M cap with an actual out-of-gas exception; it does not verify the full loop.
This is a resource failure for this implementation/budget, not an impossibility
result. See `docs/G5_C3_MILLER_LOOP.md`.

G5-C3-B3 implements memory-only scratch reuse in a separate contract, preserving
the B2 source/runner/fixture/result. Same-prefix gas falls from 4,805,196 to
2,371,368 (50.65%) and active memory words from 35,464 to 7,123 (79.91%); the
642 MODEXP calls and correct/wrong-result decisions are unchanged. One full
attempt still OOGs at the SAME 20M cap. Full-loop correctness and final pairing
remain blocked. See `docs/G5_C3_MILLER_MEMORY.md`.

G5-C3-B4 adds dedicated general Fp12 square and sparse-014 line multiplication,
preserving B2/B3 and the same field/point/NAF/memory scheme. Prefix gas falls
from 2,371,368 to 1,865,022 (21.35% versus B3); actual MODEXP calls fall from
642 to 516. The positive and wrong-result cases pass, but one full-loop attempt
still OOGs at the SAME20M cap. See `docs/G5_C3_MILLER_SPARSE.md`.

G5-C3-B5 completes exactly one Fp and one Fp2 cost profile on fixed public
endpoint-derived operands. Both full products match BigInt/Noble references.
Instrumented arithmetic windows use4,372/16,468gas, of which actual MODEXP child
execution uses400/1,200gas (2/6calls). Opcode+child gas and transaction accounting
reconcile; these isolated window costs are not Miller inlining constants.
No full loop is rerun. See `docs/G5_C3_FIELD_COST_PROFILE.md`.

G5-C3-B6 completes the isolated low-level primitive comparison. The same two
public products pass, with windows 2,167/9,965 gas versus 4,372/16,468 in B5
(50.43%/39.49% reductions). MODEXP remains 2/6 calls costing 400/1,200 gas; no
arithmetic is replaced by a host result. Four deterministic boundary checks
cover zero, modulus-minus-one square, both odd-halving low carries, and a low
sum carry/difference borrow. All pass. The contract only replaces `_mul` and
reuses its MODEXP input/output; B5 is preserved and no Miller run is made.
See `docs/G5_C3_FIELD_LOWLEVEL.md` and
`experiments/g5_c3_raw/field_lowlevel_profile.json`.

The B6 measurements above remain isolated historical results. G5-C3-B7 now
integrates that exact `_mul` into a new sparse-Miller contract; all 51 other B4
functions and the existing source/results are preserved. The same two-step
positive matches using 1,290,640 gas (30.80% below B4), 2,829 active memory words
(45.94% below B4), and 516 successful MODEXP calls. The canonical wrong target
reverts with `INVALID_MILLER_RESULT`. Exactly one full 64-step attempt at the
SAME 20M cap still fails with root out-of-gas after 7,848 successful MODEXPs and
3,147 active memory words. It does not establish full-loop correctness or a
completed-loop gas cost. Status remains partial, and the target verifier is
`BLOCKED_SOFTWARE_MILLER_LOOP_AFTER_LOW_LEVEL_MUL_EXCEEDS_20M`.
Run `npm run --prefix contracts g5c3b7`; see `docs/G5_C3_MILLER_LOWLEVEL.md` and
`experiments/g5_c3_raw/miller_lowlevel_evm.json`.

G5-C3-B8 completes B7's planned isolated `_mul2` allocation comparison. Only
that function and the profile contract name change relative to B6; its exact
`_mul`, codec and three-product Karatsuba formula are preserved. The same public
Fp2 product matches in full, with its window falling from 9,965 to 8,086 gas
(18.86%), total transaction gas from 38,887 to 37,022, and allocator growth from
2,816 to 1,664 bytes. Six real MODEXP calls are unchanged. The Fp control window
remains 2,167 gas, but its total rises from 27,606 to 27,614; do not claim all
measured costs improve. The three necessary deterministic Fp2 boundaries pass.
No Miller loop is run. The latest full attempt is still B7's OOG at 20M, so the
full verifier remains BLOCKED and aggregate gas remains null.
Run `npm run --prefix contracts g5c3b8`; see `docs/G5_C3_FP2_LOWLEVEL.md` and
`experiments/g5_c3_raw/fp2_lowlevel_profile.json`.

G5-C3-B9 completes that exact `_mul2` integration into a new B7-based Miller
variant, with every other source byte unchanged except the contract name. Only
one deployment and the same prefix positive/wrong-result pair are executed.
The positive matches at 1,132,763 gas versus B7's 1,290,640 (12.23% less), with
2,411 active memory words versus 2,829 (14.78% less). The wrong target correctly
reverts with `INVALID_MILLER_RESULT` at 1,132,581 gas; all 516 MODEXP calls
succeed in each transaction. The new variant has zero full attempts and
`full_attempt: null`. The latest full result is still B7's historical 20M OOG
for the older variant, not a full-loop failure measured on B9. Full-loop
correctness/cost remain unknown for B9, and full Miller/exact SnarkPack remain
BLOCKED with null aggregate-verifier gas.
Run `npm run --prefix contracts g5c3b9`; see
[the B9 report](G5_C3_MILLER_FP2_PREFIX.md) and
[the raw record](../experiments/g5_c3_raw/miller_fp2_prefix_evm.json).

G5-C3-B10 completes the original `_add2` cost localization without changing
arithmetic. Only one profiling entry point is added; one deployment and one
addition of the same public operands are executed. The complete result matches
the references. The arithmetic window is 1,752 gas, allocator growth 896 B,
window memory-expansion gas 91, and MODEXP count zero. Execution 5,740 plus
intrinsic 24,952 equals 30,692 gas; the actual total is 30,880 because Prague's
calldata floor adds 188 gas outside the window. This is a baseline, not an
optimization result or proof that `_add2` is the only bottleneck.
No Miller is run: the latest prefix remains B9's pass and the latest full
attempt remains B7's historical 20M OOG. B9's full loop remains untested.
Run `npm run --prefix contracts g5c3b10`; see
[the B10 report](G5_C3_FP2_ADD_COST_PROFILE.md) and
[the raw record](../experiments/g5_c3_raw/fp2_add_cost_profile.json).

G5-C3-B11 completes that isolated comparison, changing only the contract name
and `_add2`. The same public addition and the sole added boundary
`((p-1,0),(1,0))` both match complete expected results; the latter is zero.
Window gas falls 1,752->849 (51.54%), execution 5,740->4,852, allocator growth
896->384 B (57.14%), and window memory-expansion gas 91->39; MODEXP count is
zero. Transaction total stays 30,880 because the calldata-floor supplement
rises from 188 to 1,076 gas. Do not call this a total-transaction-gas reduction.
No other arithmetic tests or Miller run are performed. The latest prefix is
still B9's pass and the latest full attempt B7's older-version 20M OOG; B9's
full loop remains untested.
Run `npm run --prefix contracts g5c3b11`; see
[the B11 report](G5_C3_FP2_ADD_LOWLEVEL.md) and
[the raw record](../experiments/g5_c3_raw/fp2_add_lowlevel_profile.json).

G5-C3-B12 completes the planned exact `_add2` integration into a separate
B9-based Miller contract. Only the same prefix pair is run. Positive gas is
1,043,835 versus B9's 1,132,763 (-7.85%); active memory is 2,079 versus 2,411
words (-13.77%). All 516 MODEXPs succeed, and the wrong target rejects.
Runtime/deployment rise slightly (11 B / 2,328 gas). B12 has zero full attempts
and `full_attempt: null`; the inherited blocking status cites B7's older-version
OOG, not a new full-loop result. See [the B12 report](G5_C3_MILLER_ADD_PREFIX.md).

G5-C3-B13 completes the original `_sub2` single-input cost profile. The unchanged
function produces the exact canonical difference on the same public endpoint
operands. Window / execution gas is 1,286 / 5,296, allocator growth 640 B and
window memory expansion 65 gas, with zero MODEXP/STATICCALLs. Total 30,880 gas
contains a 632 gas calldata-floor supplement. This is a baseline only: direct
subtraction has one low-limb borrow and one non-borrow component, while the
modular-wrap branch is not covered. No Miller or extra arithmetic tests run.
See [the B13 report](G5_C3_FP2_SUB_COST_PROFILE.md).

G5-C3-B14 completes the isolated candidate comparison. Only `_sub2` and the
contract name change; all other B13 functions remain byte-identical. The same
public input passes with window 1,286->765 gas (-40.51%), execution
5,296->4,790, allocator 640->384 B (-40%) and window memory expansion 65->39.
Total stays 30,880 gas as the calldata-floor supplement grows 632->1,138.
The sole necessary modular-wrap boundary `(0,0)-(1,2^256)` returns
`(p-1,p-2^256)` and is excluded from the performance sample. Both calls use
zero MODEXP/STATICCALLs. No Miller run occurs. See
[the B14 report](G5_C3_FP2_SUB_LOWLEVEL.md).

G5-C3-B15 completes that prefix integration with 985,557 gas (-5.58% versus
B12), 1,905 active memory words (-8.37%) and 516 successful MODEXPs per case.
The wrong target rejects. This is the latest passing prefix, not full-loop
conformance. See [the B15 report](G5_C3_MILLER_SUB_PREFIX.md).

G5-C3-B16 completes the planned single full64 attempt on that same source and
runtime. It OOGs at 20M: 19,963,388 execution + 36,612 intrinsic gas. All
10,628 MODEXPs succeed; active memory is 1,945 words. No prefix or negative
is rerun. At that gate B16 became the latest actual full result; completed-loop gas stays
null. See [the B16 report](G5_C3_MILLER_SUB_FULL.md).

G5-C3-B17 completes the read-only attribution and route decision. Its script
reproduces B16's runtime/source map and derives MODEXP-child share 10.6475%,
unattributed execution 17,837,788 gas, and memory-expansion schedule cost
13,223 gas (0.06624%). The terminal `halfMod` span is not a hotspot. The saved
record has no cumulative opcode/source ranking, so selecting another primitive
for optimization is unsupported. See [the B17 report](G5_C3_MILLER_BUDGET_REVIEW.md).

G5-C3-B18 completes one such deployment and one unchanged full64 transaction.
The failure endpoint and 10,628 successful MODEXPs exactly match B16. Charged
root 17,837,786 + MODEXP child 2,125,600 + exceptional-halt remainder 2 equals
19,963,388 execution gas with zero difference. The final SHR was not executed.
Only aggregate counters are saved. `_mul` owns 45.14% of charged root gas;
77.58% of root charges map to functions, while 22.42% stays in explicit
unmapped/generated/scaffolding buckets. This exceeds the preselected 10%
unassigned-gas candidate filter, so no automatic arithmetic edit is selected.
See [the B18 report](G5_C3_MILLER_COST_ATTRIBUTION.md).

G5-C3-B19 completes that source/algebra decision with zero EVM transactions.
`_mul`'s 8,051,757 gas still exceeds `_add2` plus all unassigned gas
(6,154,522). The 10% automatic filter is a heuristic, not a reason to forbid
this manual single-candidate review. The exact B6/B15 `_mul` body has two
modular halvings that can be expressed as `(t + (t & 3)*p)/4` for the pinned
p mod 4 = 3. Both small-product and subsequent addition carries must be kept.
The historical halfMod bucket is only 6.0557% of root gas; no actual savings
or completed full64 budget is inferred. See
[the B19 report](G5_C3_MILLER_ROUTE_REVIEW.md).

G5-C3-B20 executes exactly the planned one deployment, original Fp performance
input and four correction/carry cases. All five complete outputs pass. On the
same performance input, window 2,167 -> 2,023 gas (-6.6451%), execution
4,318 -> 4,174 and total 27,606 -> 27,462; two MODEXPs, 384 B allocation and
37 gas window memory expansion are unchanged. No Fp2 or Miller ran. See
[the B20 report](G5_C3_FP_MUL_QUARTER.md).

G5-C3-B21 then integrates only that exact `_mul` into a separate B15-based
contract, with all 51 other function bodies unchanged. One deployment and the
original two prefix calls pass. Positive total gas 985,557 -> 951,873
(-33,684 / -3.4178%); the wrong target reverts at 951,691 gas. Both retain
1,905 memory words, 516 successful MODEXPs / 103,200 child gas and the original
trace. Runtime 7,106 -> 7,078 B, deployment 1,589,758 -> 1,583,679 gas. No
full64, extra boundary or primitive profiling is executed. See
[the B21 report](G5_C3_MILLER_QUARTER_PREFIX.md).

G5-C3-B22 completes that decision with zero EVM transactions. The new runtime
still lacks full64 evidence, including later arena use and terminal negative-x
conjugation. One bounded full call has independent information value because
B18's historical failure cannot answer for this runtime. This is not a forecast
from the 3.4178% prefix saving, nor a required experiment for the manuscript's
already restricted claims. See [the B22 report](G5_C3_MILLER_FULL_DECISION.md).

G5-C3-B23 executes exactly that closure: one successful deployment and one
full64 positive on the exact B21 runtime. Receipt0/root OOG spends 20M
(19,963,388 execution + 36,612 intrinsic), refund0 and empty return. All 11,076
observed MODEXPs succeed (2,215,200 child gas), with 1,945 active memory words.
No full result is returned; completed-loop gas/estimate/completion remain null.
This new-runtime budget failure is now the latest full evidence, replacing the
old "full untested" state. See [the B23 report](G5_C3_MILLER_QUARTER_FULL.md).

The B22 failure stop is binding: no further automatic arithmetic optimization,
full retry, increased cap, new profiler, final exponentiation or wrapper
implementation on this fixed20M software route. Do not convert MODEXP counts or
failed capped spend into completion percentages or full-cost estimates. Other
paper experiments remain in scope without claiming complete target verification.

### Superseded project-priority note: one native-batch plan (read-only)

The current G4-C/G5-D acceptance and B=4 priority above supersede this task selection,
but not the software-route freeze or the claim boundaries below.

1. Keep the B23 budget failure separate from the existing native SnarkPack,
   ballot and fixed-B=2 state-proof results. Check these against the manuscript's
   already restricted claims; do not promote the target-chain Manifest.
2. Inspect the current native batch interface, reusable ballot circuit/key and
   experimental SRS limits. Select at most one batch-size increment beyond the
   current two-distinct-voter case, with exact inputs/artifacts and minimal
   acceptance conditions. Do not assume larger-batch compatibility is measured.
3. This next task itself runs zero EVM/proving experiments and changes no
   cryptography. It only selects one necessary future native experiment; no
   scale matrix, production ceremony, new verifier architecture or purchase is
   authorized automatically. The fixed-budget software-route freeze remains.

The 20M is the fixed experiment budget, not an established chain hard limit.
No cap, curve, Suite or Manifest change was made in B18--B23. Full Miller conformance,
final exponentiation, exact GT target binding and the remaining SnarkPack
equations still precede a complete verifier; aggregate gas stays null.

Subsequent gates cover SnarkPack's target-verifier compatibility and complete
settlement bindings/data publication. The full `target_chain` Manifest entry
remains blocked until those gates are implemented.

Do not promote a `BLOCKED_*` capability in the manifest until its positive and
negative tests, raw outputs and reproduction command are checked in.
