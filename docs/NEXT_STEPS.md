# Next gates

Current checkpoint (2026-10-26): B14's low-allocation Fp2-subtraction comparison
passes the original input and one necessary modular-wrap boundary. The next
experiment is B15 only: integrate the exact `_sub2` into a separate B12-based
Miller variant and measure the original prefix pair. B12 has no full-loop
result; the latest actual full attempt remains B7's older-version 20M OOG.

The current native core and real P1 Groth16 path pass. The remaining gates must
be implemented in this order without substituting mocks for measurements:

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

1. G5-C3-B15: preserve B12/B14 and copy the exact B14 `_sub2` into a separate
   B12-based Miller variant. All other source except the contract name must
   remain unchanged. Run only the same two-step prefix positive/wrong-result
   pair on the original fixture, and record its actual integrated gas, memory
   and MODEXP count. The isolated 40.51% saving is not a Miller saving.
2. Use the integrated result to decide the next step. B15 does not automatically
   retry the full loop. Preserve the 20M cap, compiler, suite, curve, encoding
   and Manifest; do not rerun isolated boundaries or broaden the test matrix.
3. Full-loop EVM conformance, final exponentiation and exact GT target binding
   still precede a full pairing/SnarkPack verifier. Keep aggregate-verifier gas
   null until it is actually measured. No scale or proving run is needed for
   the B15 prefix comparison.

Subsequent gates cover SnarkPack's target-verifier compatibility and complete
settlement bindings/data publication. The full `target_chain` Manifest entry
remains blocked until those gates are implemented.

Do not promote a `BLOCKED_*` capability in the manifest until its positive and
negative tests, raw outputs and reproduction command are checked in.
