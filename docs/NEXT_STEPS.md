# Next gates

## Current checkpoint (2026-10-28): G1-C2 native B=4 accepted

Four different voters, one region and certified DKG key, four fresh C_vote
proofs and one real SnarkPack V2 aggregate pass. Only the fourth nullifier is
changed in the necessary negative case and complete aggregate verification
rejects. One setup / four proofs / one aggregate positive / one aggregate
negative were executed. Old B=2 code/results and the depth-2 ballot circuit
are unchanged. See `docs/G1_C2_FOUR_VOTER_SNARKPACK.md` and
`experiments/g1_c2_raw/snarkpack_distinct_voters_batch_4.json`.

Aggregate size 8,212 B exceeds 768 B for four proofs; aggregate verification
3.402 ms exceeds 1.760 ms for same-run individual bellperson verification.
This functional pass is not an efficiency claim. State proof remains B=2.
The previous G5-D tally-v2 EVM acceptance remains valid and was not rerun.

## Next single experiment: B=8 with a versioned depth-3 registration tree

This is planned, not executed. Continue the uploaded plan's native batching
workflow without waiting for SnarkPack EVM.

1. Add a separately versioned depth-3 ballot relation / eight-leaf registry,
   preserving all old depth-2/B=2/B=4 code paths, keys and data. Do not simply
   change the global depth constant or use a depth-2 VK for a depth-3 witness.
2. Keep three candidates and four regions, with one authenticated DKG key and
   pre-policy fixed before DKG. Generate eight DIFFERENT registered voters in
   one region using existing PRF/region algorithms, not copied proofs or region
   overrides. Freeze a valid per-election weight bound for all eight entries.
3. One fresh setup shared by eight fresh source proofs; verify individually,
   then create/verify a real size-8 V2 aggregate using explicitly experimental
   SRS. One negative changes only the last voter nullifier in the whole matrix.
4. Archive a new configuration and raw directory with exact circuit/key/source
   hashes, per-proof/aggregate timings, sizes and peak RSS. Separate setup from
   proving and verification; do not infer crossover points from B=2/B=4 samples.
5. Stop after one B=8 functional acceptance. B=4 state proof plus the omitted-
   ballot negative is the following separate task; B=16/32 and formal repeated
   performance measurements come later. No EVM, P3 subset expansion or new
   arithmetic profiler is included in the B=8 task.

All-ten-subset v2 P3, B=4/8 state-circuit families with omission tests, fair
performance baselines and full settlement remain separate later deliverables.

The historical B23 fixed-budget software-route freeze below remains binding.

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
