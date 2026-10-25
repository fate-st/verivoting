# Implementation and claim boundary

## Implemented experiment path

1. Validate `configs/protocol_manifest.json` and hash its canonical encoding.
2. Run the 5-trustee, threshold-3 DKG phases using real Ristretto group
   operations. The adversarial DKG experiment injects one bad private share and
   one inconsistent Feldman broadcast.
3. Register the four fixed weights, validate Pedersen openings and the exact
   registered-weight sum, then sign the close record.
4. Encrypt the fixed choices with weighted additive ElGamal and fold ballots.
5. Produce and natively verify generalized Chaum–Pedersen decryption-share
   proofs bound to `eid`, final context, pre-policy, disclosure digest, target,
   trustee id, verification key, all ciphertext bases, and all shares.
6. Commit to each private package and sign its public commitment.
7. Combine threshold shares and decode bounded tallies with BSGS. Check all ten
   3-of-5 subsets.
8. Generate and verify a real Groth16 P1 proof for bounds and deterministic
   winner selection. Change the public winner and confirm verification fails.
9. Serialize a winner-only public event without the tally or private packages.

## G0.5 CP-circuit feasibility result

The next required experiment was run once with a real native generalized
Chaum--Pedersen transcript. The native proof verified, and the first circuit
equation `z*G = A0 + d*VK` was satisfied by the corresponding witness.

This is a preflight result, not a Groth16 proof and not P3:

- constraints: `3,501,941`;
- public inputs: `324`;
- peak RSS in the dev build: `8,018.9 MiB`;
- configured proof-run guard: `1,000,000` constraints;
- Groth16 setup/prove/verify: not run because the preflight exceeded the guard.

The dominant component is non-native double-scalar multiplication
(`3,484,879` constraints). The necessary next action is to reduce that cost;
P2/P3-a is not started from this result. See `docs/P3_FEASIBILITY.md` and
`experiments/raw/p3-feasibility.json`.

The first sound optimization used the canonical 253-bit scalar bound and
reduced the result to `3,461,084` constraints. This remains blocked by the same
resource guard; see `docs/P3_OPTIMIZATION_REPORT.md`.

## G0.6 curve-decision result

An isolated Jubjub/BLS12-381 prototype compiled one CP equation (`9,571`
constraints), one ElGamal coordinate (`12,538`), and one homomorphic point
addition (`5,867`). All three produced and verified real 192-byte Groth16
proofs; changed public inputs and invalid relations were rejected. Peak process
RSS was `101.3 MiB` in the development-profile run.

This did not change the manifest or native Ristretto protocol. The challenge is
a canonical public input but SHA-512 is not recomputed in-circuit, and the
ElGamal/state rows are component experiments rather than full ballot or state
proofs. See `docs/P3_CURVE_DECISION.md` and
`experiments/raw/p3_curve_compare.csv`.

## G0.7 Release reproduction

The Release workspace build, strict Clippy and required regression tests now
pass with `CARGO_PROFILE_RELEASE_STRIP=none`, which avoids a host
`rust-objcopy --strip-debug` stall without changing optimized code generation.
The initial five-trial P1 baseline and four-voter Ristretto E2E passed. See
`docs/G07_RELEASE_BASELINE.md`.

The benchmark CSV writer now records the actual Git SHA, dirty state, manifest
suite, CPU, memory, Rust version and build profile for new runs. Historical CSV
files were not rewritten.

## G0.8 native Jubjub progress

The independent `vv-friendly-crypto` crate now freezes Jubjub Suite v2. Its
SHA-512 rejection sampler masks candidates to 252 bits before the strict
canonical scalar check; public keys and ciphertext points cross a canonical
subgroup-validation boundary; cryptographic randomness requires `CryptoRng`;
and BSGS uses checked arithmetic and a documented table limit.

The `vv-friendly-dkg` crate implements a single-process, real-group Pedersen
VSS/GJKR experiment with five pre-policy-authenticated trustees, complaint
correction, Feldman extraction, a three-signature KeyCore certificate and
threshold-3 decryption. Native generalized Chaum--Pedersen now proves each
Trustee's three decryption-share equations. The verifier recomputes the exact
Suite v2 challenge and returns a type that alone is accepted by the verified
threshold-combination path.

All five Trustee proofs verify. Every one of the ten legal 3-of-5 subsets uses
only verified shares and decrypts `(4,2,4)` with winner `1`; the aggregate
secret key is never reconstructed. G0.8-E1 additionally creates five private
CP packages, commits to each package with a fresh Pedersen opening, signs each
public commitment with the corresponding pre-policy authentication key,
verifies all five packages, then deterministically selects Trustees 1--3.
Only the selected verified-share type reaches threshold decryption. The public
event contains winner `1`, contexts, package commitments and signatures, but
no tally, shares, CP proofs or openings.

G0.8-E2 adds a Jubjub RA key and registration policy that are frozen into
`policy_pre` before DKG. Four Pedersen weight commitments are checked for
unique credentials and exact integer sum; the RA signs a close record bound to
the final context. The verified close and disclosure digests enter the tally
target, generalized CP transcript and private-package commitment. The Release
run closes count 4 and weight 10, verifies 5/5 packages, selects Trustees 1--3
and emits winner 1 without publishing the private tally.

This is `PASS_G08_NATIVE_SINGLE_PROCESS`: the planned native Jubjub G0.8 path
is complete. It is not a weighted-ballot Groth16 implementation, full P3,
SnarkPack, state proof, multiprocess deployment or chain result. The official
Ristretto manifest is unchanged. See `docs/G08_NATIVE_COMPLETE.md` and
`experiments/g0_8_complete_raw/e2e-public.json`.

## G0.9-R clean baseline

Commit `39cca56d730e97d136f3f998df1ab81eda9d82f4` was reproduced with a clean
worktree, Rust 1.99.1, locked dependencies and Release `opt-level=1`,
`strip=none`. The necessary workspace test run passed 50 tests and filtered
the one named high-memory Ristretto test; strict workspace Clippy passed.

The G0.8-E2, H0 and H1 results all reproduced successfully with
`git_dirty=false`. For H2/H3/H4, the existing Jubjub Suite v2 is authoritative
and the Ristretto manifest is retained only as a legacy comparison baseline.
See `docs/G09_R_BASELINE.md` and
`experiments/g0_9_r_raw/g09_r_baseline.json`.

## G0.9-H0 CP transcript boundary

The registration-bound generalized-CP transcript v3 now has a deterministic
golden vector. Its exact canonical encoding is 684 bytes and has SHA-256
`b902f98b94fd9a5b04a73782a8ab6e73baa7e1525a20d49ee6d2dec8c8bd1d96`.
The native challenge scalar is also frozen by a regression test.

A minimal real R1CS relation allocates the 684 bytes as witness bytes and
public bytes and enforces exact equality. The valid vector satisfies 16,416
constraints; swapping the registration-close and disclosure digests or
changing the registration-close digest makes the relation unsatisfied. This is
`PASS_H0_CANONICAL_TRANSCRIPT_R1CS_BOUND_NO_HASH`: SHA-512/rejection sampling
is not computed in-circuit, no Groth16 proof is generated, and this is not P3.
See `docs/G09_H0_TRANSCRIPT.md` and
`experiments/g0_9_h0_raw/h0_transcript.json`.

## G0.9-H1 SHA-512 circuit

A real SHA-512 R1CS now hashes the exact frozen 684-byte H0 transcript. The
circuit implements SHA-512 padding and six compression blocks, packs the
64-byte public digest into three field elements, and constrains the computed
digest to that public value.

The clean Release rerun matched the frozen native digest, used 890,920
constraints and 883,152 witness variables, synthesized the witness in 776.481
ms and reached 817.156 MiB peak process RSS. A changed transcript and a changed public digest
were both rejected. This is `PASS_H1_SHA512_R1CS_EXACT_DIGEST`.

H1 hashes the CP transcript directly. It does not yet implement the protocol's
hash-to-scalar wrapper and rejection sampling, integrate the CP group
equations, or run Groth16. It is therefore not full CP verification or P3. See
`docs/G09_H1_SHA512.md` and
`experiments/g0_9_h1_raw/h1_sha512.json`.

## G0.9-H2-A exact hash-to-scalar feasibility

The exact Suite v2 H2S wrapper, SHA-512, 252-bit mask, strict Jubjub scalar
range check and first-valid rule are now constrained for two real frozen CP
transcripts. The counter-0 case uses 1,013,462 constraints; the counter-1 case
hashes two attempts and uses 2,019,916 constraints. Native and circuit outputs
match, while changed transcript/domain values and a false counter-1 claim are
rejected.

This is `PASS_H2A_COUNTER_0_1_FIRST_VALID_R1CS`, not full H2. The circuit shape
depends on the accepted counter and no uniform 128-attempt circuit, CP equation
integration or Groth16 proof has been implemented. See
`docs/G09_H2A_HASH_TO_SCALAR.md` and
`experiments/g0_9_h2_raw/h2a_hash_to_scalar.json`.

## G0.9-H2-B fixed-shape feasibility

One fixed R1CS now always evaluates the counter-0 and counter-1 H2S attempts
and derives the first valid result. Both real frozen vectors use exactly
2,020,168 constraints, 2 public inputs and 2,003,927 witness variables. The
counter-0 and counter-1 paths have identical shape, and a false counter-1 claim
is rejected.

This is `PASS_H2B_FIXED_TWO_ATTEMPT_R1CS_FEASIBILITY`. It validates the fixed
selection construction but does not implement the Suite v2 bound of 128
attempts and is not full H2, CP verification, Groth16 or P3. See
`docs/G09_H2B_FIXED_SHAPE.md` and
`experiments/g0_9_h2_raw/h2b_fixed_two.json`.

## G0.9-H2-C wide hash-to-field candidate

A versioned candidate now computes one SHA-512 digest over an 808-byte
canonical input and constrains reduction of the complete 512-bit big-endian
digest modulo Jubjub `Fr`. Two real transcript cases both use 1,148,545
constraints, 2 public inputs and 1,113,725 witness variables. Their native and
R1CS outputs match, and a changed transcript is rejected.

This is `PASS_H2C_WIDE_SHA512_MOD_FR_CANDIDATE_R1CS`. It reduces constraints by
43.146% relative to H2-B. H2-C was not itself an adoption result; no CP
equations or Groth16 were run. See `docs/G09_H2C_WIDE_HASH_TO_FIELD.md` and
`experiments/g0_9_h2_raw/h2c_wide_hash_to_field.json`.

## G0.9-H2-D adopted CP-H2F-v3

The Jubjub prototype manifest now pins
`VERIVOTING-JUBJUB-CP-H2F-G0.9-v3`. The final 801-byte canonical input uses one
SHA-512 computation and full 512-bit reduction modulo Jubjub `Fr`. Its fixed
R1CS uses 1,151,661 constraints, 2 public inputs and 1,116,817 witness
variables. Native and R1CS outputs match and a changed transcript is rejected.

Native CP regression verifies 5/5 Trustee proofs and all ten legal threshold
subsets. The registration-bound E2E verifies 5/5 private packages, selects
Trustees 1--3 and emits winner 1 without publishing the tally. Removing the CP
counter reduces five private payloads from 3,020 to 3,010 bytes.

This is `PASS_H2D_CP_H2F_V3_ADOPTED`; standalone H2 is complete for the
prototype. H2-D itself did not connect CP group equations or generate a
Groth16/P3 proof; H3 below subsequently connects the first equation. See
`docs/G09_H2D_CP_H2F_ADOPTION.md`.

## G0.9-H3 first CP equation

The adopted CP-H2F-v3 challenge is now connected to one Trustee's first
generalized CP equation, `zG = A0 + d VK`, in one R1CS. The circuit binds the
canonical compressed `VK` and `A0` bytes at their fixed positions in the
684-byte transcript and constrains all group arithmetic internally.

The Release run uses 1,168,855 constraints, 23 packed public inputs and
1,130,518 witness variables. The valid relation passes; a changed transcript,
wrong verification share and wrong response are rejected. This is
`PASS_H3_H2F_BOUND_FIRST_CP_EQUATION_R1CS`. It is not the coordinate-vector CP,
Groth16 or full P3. See `docs/G09_H3_FIRST_CP_EQUATION.md`.

## G0.9-H4-A complete one-Trustee CP vector

The same in-circuit CP-H2F-v3 challenge now drives the first equation and all
three coordinate equations for one Trustee. The circuit binds 11 canonical
compressed points from the 684-byte transcript and constrains four group
equations without host-language substitution.

The Release run uses 1,207,066 constraints, 23 packed public inputs and
1,164,214 witness variables. The valid vector passes; a changed transcript,
altered decryption share and altered coordinate commitment are rejected. This
is `PASS_H4A_COMPLETE_ONE_TRUSTEE_CP_VECTOR_R1CS`. Groth16, multiple Trustees
and full P3 are not included. See `docs/G09_H4A_COMPLETE_TRUSTEE_CP.md`.

## G0.9-H4-B real one-Trustee CP Groth16

The exact H4-A circuit now completes a real BLS12-381 Groth16 setup, proof and
verification run. It retains 1,207,066 constraints, 23 packed public inputs and
1,164,214 witness variables. Setup took 14,025.404 ms, proving 8,206.572 ms and
verification 3.017 ms. The proof is 192 bytes, the compressed proving key is
380,080,848 bytes, and peak RSS after proving is 2,886.219 MiB.

The valid proof verifies and a changed packed public transcript is rejected.
This is `PASS_H4B_REAL_GROTH16_ONE_TRUSTEE_CP`. It completes the planned H4
one-Trustee gate, not the three-Trustee relation or full P3. See
`docs/G09_H4B_GROTH16.md`.

## G0.9-P3-A three selected Trustee CP R1CS

The P3-A input layout now keeps each Trustee's public 452-byte CP statement
prefix public while allocating the 232-byte share/proof suffix and response as
private witnesses. A real GJKR, registration-close and four-voter fixture
produces distinct selected Trustees 1--3, and all three native CP proofs verify.

The combined R1CS checks 12 CP group equations with 3,611,598 constraints, 45
packed public inputs and 3,485,994 witness variables. The latest witness
synthesis takes 4,876.174 ms and reaches 3,258.906 MiB peak RSS. The valid relation satisfies
the circuit; changing the third Trustee's share and matching private transcript
is rejected. No private CP values are written to the public fixture.

This is `PASS_P3A_THREE_SELECTED_TRUSTEE_CP_R1CS`. Package commitment
openings, interpolation, tally and winner logic remain separate gates. See
`docs/G09_P3A_THREE_CP_R1CS.md`.

The exact same relation also completes a real Groth16 run. Setup takes
36,858.792 ms, proving 23,581.980 ms and verification 5.349 ms. The proof is
192 bytes, the compressed proving key is 1,037,976,624 bytes, and peak RSS is
5,824.047 MiB. The valid proof verifies and a changed public prefix is
rejected. This is `PASS_P3A_REAL_GROTH16_THREE_SELECTED_TRUSTEE_CP`; it
completes P3-A, not full P3. See `docs/G09_P3A_GROTH16.md`.

## G0.9-P3-B0 fixed-shape private-package H2F

Private-package commitment messages now use the independent
`VERIVOTING-JUBJUB-PACKAGE-H2F-G0.9-v1` mapping: one SHA-512 digest is reduced
modulo Jubjub Fr with a fixed circuit shape. This replaces the package
message's former variable-attempt H2S relation without changing Schnorr
signatures or the CP-H2F-v3 challenge.

For each real selected Trustee package, the circuit hashes 602 package bytes
through a 725-byte canonical input over six SHA-512 blocks. Each relation uses
1,000,827 constraints, 2 public inputs and 967,157 witness variables. All
three native/R1CS outputs match; total witness synthesis is 3,030.297 ms and
peak RSS is 1,059.766 MiB. A changed package is rejected.

The migrated native E2E still creates and verifies 5/5 packages, selects
Trustees 1--3 and emits winner 1 without a public tally. This is
`PASS_P3B0_PACKAGE_H2F_V1_ADOPTED`, not P3-B: the Pedersen opening equations
are not yet part of the combined P3 circuit, and no Groth16 run was made. See
`docs/G09_P3B0_PACKAGE_H2F.md`.

## G0.9-P3-B1 three private-package openings

The P3-B1 circuit reuses each P3-A CP witness to construct the corresponding
602-byte private package, computes Package-H2F-v1 internally, receives the
canonical package commitment as a public input and keeps its Pedersen opening
private. It constrains all 12 CP equations, 3 package-H2F relations and 3
opening equations in one R1CS.

The measured relation uses 6,191,178 constraints, 51 public inputs and
5,966,673 witness variables. The final witness synthesis takes 9,234.655 ms
and reaches 5,661.500 MiB peak RSS. Three native CP proofs and authenticated
packages verify before circuit construction; a changed third public commitment
is rejected. Private package bytes and openings are not emitted.

This is `PASS_P3B1_THREE_PACKAGE_OPENINGS_R1CS`. The package-opening gate is
complete, but Groth16, threshold interpolation, secret tally binding and
winner correctness remain absent. See `docs/G09_P3B1_PACKAGE_OPENINGS.md`.

## G0.9-P3-C0 fixed 1-2-3 threshold combination

For the selected set `{1,2,3}`, the circuit now applies the exact zero-point
Lagrange coefficients `(3,-3,1)` to each of the three already CP-bound share
coordinates. It receives the three canonical aggregate `C1` points publicly
and constrains three private plaintext group elements as `M_j=C1_j-D_j`.

The complete relation uses 6,201,342 constraints, 57 public inputs and
5,974,593 witness variables. Witness synthesis takes 8,407.870 ms and reaches
5,607.562 MiB peak RSS. Native coefficients and threshold decryption match the
fixed four-voter fixture; changing the third private plaintext point is
rejected. No private plaintext point is emitted.

This is `PASS_P3C0_FIXED_123_THRESHOLD_COMBINATION_R1CS`. Fixed-subset group
reconstruction is complete, but bounded tally witnesses, winner correctness
and Groth16 remain absent. See `docs/G09_P3C0_THRESHOLD_COMBINATION.md`.

## G0.9-P3-C1 bounded private tally binding

The P3-C1 circuit reuses the three P3-C0 plaintext variables and binds them to
three private 64-bit integer witnesses with `M_j=t_jG`. Each tally is bounded
by the public registered weight `W=10`, and their sum is constrained to equal
that same bound. Neither the plaintext points nor the tally values are emitted.

The complete relation uses 6,203,703 constraints, 58 public inputs and
5,976,943 witness variables. Witness synthesis takes 8,806.231 ms and reaches
5,590.266 MiB peak RSS. The valid `(4,2,4)` witness satisfies the circuit; the
sum-preserving replacement `(5,1,4)` is rejected by the group bindings.

This is `PASS_P3C1_BOUNDED_TALLY_BINDING_R1CS`. The bounded-tally gate is
complete in R1CS. Winner correctness and Groth16 remain absent, so this is not
full P3. See `docs/G09_P3C1_BOUNDED_TALLY.md`.

## G0.9-P3-D0 public winner linkage

The P3-D0 circuit reuses the exact P3-C1 private tally bits and constrains one
public winner identifier using the highest-weight, smallest-identifier-on-tie
rule. For private tally `(4,2,4)`, public winner `1` satisfies the relation and
winner `2` is rejected because candidate 1 has the same tally and the smaller
identifier.

The complete relation uses 6,205,629 constraints, 59 public inputs and
5,978,859 witness variables. Witness synthesis takes 8,566.464 ms and reaches
5,607.875 MiB peak RSS. Neither private tally values nor plaintext group points
are emitted.

This is `PASS_P3D0_PUBLIC_WINNER_LINKAGE_R1CS`. The CP, authenticated-package,
threshold, bounded-tally and winner relation is complete in R1CS. A combined
Groth16 proof has not been generated, so `full_p3=false`. See
`docs/G09_P3D0_WINNER_LINKAGE.md`.

## G0.9-P3-D1 fixed-123 complete P3 Groth16

The exact P3-D0 relation now completes real BLS12-381 Groth16 setup, proof and
verification. The public input packing contains 59 field elements and matches
the circuit layout exactly. The valid proof for winner `1` verifies; changing
only the public winner to `2` causes verification to fail.

Setup takes 64,668.553 ms, proving 43,604.668 ms and verification 6.019 ms. The
proof is 192 bytes, the compressed proving key is 1,837,594,176 bytes, the
compressed verifying key is 3,224 bytes, and peak RSS is 7,779.359 MiB. The
proving key is not persisted.

This is `PASS_P3D1_REAL_GROTH16_FIXED_123_COMPLETE_P3` and records
`full_p3=true` for the fixed Trustee set `{1,2,3}` four-voter experiment. It is
not a production ceremony or a universal setup for every possible legal
Trustee subset. See `docs/G09_P3D1_FULL_GROTH16.md`.

## G1-B0 weighted-ballot cryptographic core

The first ballot gate constrains a private registered weight and commitment
opening, one private three-candidate one-hot choice, and three private
encryption randomness scalars. Public inputs are the certified Jubjub election
key, weight commitment, weight bound and three ciphertext pairs.

The relation uses 46,573 constraints, 17 public inputs and 39,940 witness
variables. Witness synthesis takes 108.856 ms and reaches 49.156 MiB peak RSS.
The valid relation satisfies the circuit; a two-hot witness is rejected.

This is `PASS_G1B0_WEIGHTED_BALLOT_CORE_R1CS`, not the paper's complete
five-condition `C_vote`. Election/context identifiers, credential PRF address,
deterministic region, voting nullifier and registration-root membership remain
absent. Groth16 and SnarkPack have not been run. See
`docs/G1_B0_BALLOT_CORE.md`.

## G1-B1 complete fixed-suite C_vote R1CS

G1-B1 composes the G1-B0 cryptographic core with the remaining four public
bindings required by the paper's five-condition ballot relation. The exact
experiment suite uses a 32-byte voter seed, separately domain-separated
HMAC-SHA-256 address and voting-nullifier derivations, four deterministic
regions, and the native SHA-256 registration leaf/node encoding. The PRF
address is the registered leaf identifier, and a private depth-two path binds
the private weight commitment to the actual four-entry registration-close
root.

The complete relation uses 854,454 constraints, 25 public inputs and 839,832
witness variables. The latest Release witness synthesis takes 777.431 ms and
reaches 754.859 MiB peak RSS. The legal relation satisfies the R1CS, the independently
computed root equals the signed native registration close, and changing only
the public final context is rejected.

This is `PASS_G1B1_FULL_C_VOTE_R1CS_FIXED_DEPTH2_FOUR_REGION`. It is complete
R1CS for this fixed four-leaf/four-region suite, not a variable-depth
production registry and not a Groth16 or SnarkPack result. RA certificate
validation and equality with the immutable election registry remain external
contract checks, as in the paper. See `docs/G1_B1_FULL_C_VOTE_R1CS.md`.

## G1-B2 complete fixed-suite ballot Groth16

The exact G1-B1 circuit now completes real BLS12-381 Groth16 setup, proof and
verification. Setup takes 8,015.162 ms, proving 4,623.984 ms and verification
2.386 ms. The proof is 192 bytes, the compressed proving key is 251,898,000
bytes, the compressed verifying key is 1,592 bytes, and peak RSS is 1,880.547
MiB. The proving key and private witness are not persisted.

The valid proof verifies. Reusing it after changing only the public final
context fails verification. This is
`PASS_G1B2_REAL_GROTH16_FULL_C_VOTE_FIXED_DEPTH2_FOUR_REGION`: a real ballot
proof for the fixed four-leaf/four-region suite, not a variable-depth
production setup or, by itself, a SnarkPack result. See
`docs/G1_B2_FULL_C_VOTE_GROTH16.md`.

## G1-C0 minimum-batch SnarkPack compatibility

The official `bellperson 0.27.0` implementation now aggregates two distinct
randomizations of the G1-B2 proof through its real SnarkPack V2 code path.
Canonical arkworks BLS12-381 proof, verifying-key and scalar values are
converted to blstrs and individually reverified before aggregation. Aggregation
takes 4.003 ms and aggregate verification 3.153 ms; changing the first public
final context is rejected. The aggregate proof is 5,236 bytes versus 384 bytes
for the two source proofs, so this minimum-batch gate supports compatibility,
not a compression or scale claim. Peak RSS is 2,134.141 MiB.

The deterministic `setup_fake_srs` helper is used only for reproducible
functional testing and is not a production ceremony. Both source proofs use
the same valid public statement with independent Groth16 randomness. See
`docs/G1_C0_SNARKPACK_COMPATIBILITY.md`.

## G1-C1 distinct registered-voter SnarkPack

The same frozen 25-field `C_vote` public-input layout and the passing G1-C0
conversion path now aggregate two complete ballot proofs for registered voters
1 and 2 in region 4. The voters select candidates 1 and 2, respectively. Their public
statements, voting nullifiers and ciphertext vectors are distinct, while both
statements share the same election identifier, final context and certified
registration root and regional identifier.

Real Release aggregation takes 3.908 ms and aggregate verification 2.955 ms.
The two source proofs occupy 384 bytes and the aggregate proof occupies 5,236
bytes. Peak RSS is 2,516.266 MiB. The valid aggregate verifies; replacing only
the second statement's public voting nullifier is rejected.

This is
`PASS_G1C1_REAL_SNARKPACK_V2_DISTINCT_VOTERS_BATCH_2_NONPRODUCTION_SRS`.
It closes the smallest distinct-voter functional gate, not a scale or
compression claim. Bellperson's deterministic `setup_fake_srs` remains a
non-production reproducibility mechanism. See
`docs/G1_C1_DISTINCT_VOTER_SNARKPACK.md`.

## G2-A fixed-B=2 regional state-transition R1CS

G2-A consumes the exact two same-region public ballot records emitted by the
passing G1-C1 run. Its 37 public inputs follow the paper's order: election,
context, region, batch index, fixed batch size, ordered batch root, key digest,
certified public key, all old-state points, then all new-state points. The
witness contains the two ordered records and the intermediate accumulators.

The circuit recomputes two `VV/BATCH/LEAF/v2` hashes and one separately tagged
inner-node hash over the exact 25-field ballot vectors. It binds both records to
the public election/context/region/key and performs 12 Jubjub additions across
the six ciphertext components. The relation uses 1,360,002 constraints, 37
public inputs and 1,329,560 witness variables. Release witness synthesis takes
1,145.101 ms and peak RSS is 1,196.984 MiB. The valid zero-to-batch transition
satisfies the circuit; replacing one ciphertext while retaining the public
batch root and new state is rejected.

This is `PASS_G2A_FIXED_B2_STATE_TRANSITION_R1CS`. It establishes the smallest
state relation in R1CS, not a state proof: Groth16 has not been run. Contract
checks for accepted registration roots, spent nullifiers and the current old
state remain outside this circuit as specified by the paper. See
`docs/G2_A_STATE_TRANSITION_R1CS.md`.

## G2-B fixed-B=2 regional state-transition Groth16

G2-B runs real Groth16 over the unchanged G2-A relation and its frozen
37-field public layout. Release setup takes 13,254.145 ms, proving 8,776.350 ms
and verification 2.660 ms. The proof is 192 bytes; compressed proving and
verifying keys are 419,767,248 and 2,168 bytes. Peak RSS is 3,090.062 MiB.

The valid proof verifies. Reusing it after changing only the public ordered
batch root fails verification in 2.434 ms. The proving key is not persisted and
no private witness is emitted. This is
`PASS_G2B_REAL_GROTH16_FIXED_B2_STATE_TRANSITION`: it completes the fixed-`B=2`
state-proof gate, not a larger-batch benchmark or production setup ceremony.
See `docs/G2_B_STATE_TRANSITION_GROTH16.md`.

## G3-A five-process DKG happy path

G3-A starts five concurrent Trustee OS processes with five distinct PIDs. Each
Trustee owns a separate mode-`0600` private state file and exchanges
session-bound, Jubjub-Schnorr-authenticated messages through atomic local-file
transport. The run verifies 25 dealer-to-receiver Pedersen/Feldman share
messages, obtains five matching final public states and verifies all five
KeyCore certificate signatures.

Release DKG-to-final time is 57.281 ms. Four weighted votes are encrypted and
folded in 5.431 ms. Trustees `{1,2,3}` then emit three authenticated decryption
vectors; combination and decoding take 27.352 ms and recover `(4,2,4)` with
winner `1`. Total wall time is 155.093 ms and the local transport artifacts
occupy 39,162 bytes.

This is `PASS_G3A_FIVE_OS_PROCESS_DKG_HAPPY_PATH`. It establishes process
separation, state persistence and authenticated honest-path transport. It does
not establish restart recovery, timeout handling, remote networking or
Byzantine-process behavior. See `docs/G3_A_MULTIPROCESS_DKG.md`.

## G3-B one-Trustee restart recovery

G3-B stops Trustee 3 immediately after its mode-`0600` private state is
persisted, then starts a replacement process under a new PID. The replacement
loads the original authentication secret and VSS polynomials and signs a
session-bound recovery record containing the checkpoint SHA-256 digest and both
PIDs. The run therefore contains five logical Trustees and six OS process
instances.

All six processes exit successfully. The recovered Trustee finishes the same
25 authenticated VSS messages and participates in the `{1,2,3}` decryption.
DKG-to-final time is 57.763 ms, threshold combination takes 15.317 ms and total
wall time is 135.047 ms. The resulting transcript, KeyCore, certified public
key, tally `(4,2,4)` and winner `1` match G3-A.

This is `PASS_G3B_ONE_TRUSTEE_RESTART_RECOVERY`. It covers one orderly
post-checkpoint restart over local transport, not mid-write crashes, repeated
faults, timeout behavior or remote deployment. See
`docs/G3_B_RESTART_RECOVERY.md`.

## G4-A second-subset complete-P3 R1CS shape

G4-A keeps the prior fixed-123 Groth16 circuit unchanged and adds a separate
fixed-width coefficient path for the feasibility comparison. The complete P3
relation is synthesized for `{1,2,3}` with `(3,-3,1)` and for `{1,2,4}` with
`(8/3,-2,1/3)`. Both native threshold paths recover `(4,2,4)` with winner `1`,
and both R1CS instances are satisfied.

Each instance has 6,236,532 constraints, 59 public inputs and 6,009,006 witness
variables. Release witness synthesis takes 8,831.783 ms for `{1,2,3}` and
7,550.190 ms for `{1,2,4}`; peak process RSS is 5,504.094 MiB.

This is `PASS_G4A_SECOND_SUBSET_COMMON_R1CS_COUNT_SHAPE`. It establishes a
common measured count shape for exactly two legal subsets. It does not show
identical R1CS matrices or proving-key reuse because the coefficients remain
setup-time circuit parameters. Groth16 was not run and the Manifest remains
limited to the existing fixed-123 proof. See
`docs/G4_A_SECOND_SUBSET_P3_SHAPE.md`.

## G4-B public two-subset complete-P3 Groth16

G4-B adds one public Boolean selector to a single complete-P3 circuit. The
selector is bound to the Trustee identifiers already present in the public CP
transcript prefixes: `false` selects ordered subset `{1,2,3}` and `true`
selects `{1,2,4}`. The circuit uses the same selector to derive the exact
canonical Lagrange-coefficient bits, so those coefficients are no longer
setup-time parameters for this two-subset gate.

Both assignments have 6,231,973 constraints, 60 public inputs and 6,005,106
witness variables. A real Groth16 setup generated from `{1,2,3}` was reused to
prove `{1,2,4}`. Setup took 61,062.475 ms, proving 38,615.505 ms and
verification 6.529 ms; the proof is 192 bytes and peak RSS is 8,177.266 MiB.
Changing only public selector input 51 rejects the valid proof, and a selector
that disagrees with the public transcript Trustee IDs makes the R1CS relation
unsatisfied.

This is `PASS_G4B_REAL_GROTH16_TWO_SUBSET_PUBLIC_SELECTOR`. It establishes
parameter reuse for exactly these two selector values, not arbitrary 3-of-5
selection or all ten legal subsets. The deterministic setup is not a production
ceremony, the proving key is not persisted, and the Manifest remains limited to
the prior fixed-123 capability. See `docs/G4_B_PUBLIC_SUBSET_GROTH16.md`.

## G5-A local Prague state-proof verifier

G5-A exports a real G2-B proof, fixed verification key and 37 public inputs in
EIP-2537 format, then compiles Solidity 0.8.30 for Prague and executes signed
transactions in EthereumJS VM 10.1.3. The constructor pins the key; verification
uses a real 38-term G1 MSM and four-pair BLS12-381 check. The valid proof
succeeds and the same proof with only batch-root input 7 changed reverts.

The valid transaction uses 940,640 total gas, including 903,280 execution gas;
the wrong-root transaction uses 940,718 gas. Measured MSM/pairing costs are
280,440/168,100 gas. Deployment uses 4,550,491 gas with 1,807 runtime bytes.
The EIP-2537 proof is 512 bytes and actual verifier calldata is 1,828 bytes.

This is `PASS_G5A_LOCAL_PRAGUE_G2B_GROTH16_VERIFIER`, a local state-verifier
execution result. It does not measure public-chain finality, SnarkPack or P3
verification, registry/nullifier/state writes, or full batch DA. The complete
target-chain Manifest capability stays blocked. See
`docs/G5_A_PRAGUE_STATE_VERIFIER.md`.

## G5-B local Prague two-subset P3 verifier

G5-B exports one real G4-B P3 proof, its verification key and 60 public inputs
using the existing EIP-2537 encoding. The unchanged Solidity verifier is
deployed with the P3 key and executes a 61-term G1 MSM and four-pair check.
The valid `{1,2,4}` proof succeeds; changing only public selector input 51
from `1` to `0` makes the transaction revert with `INVALID_GROTH16`.

The valid transaction uses 1,331,473 total gas (1,278,293 execution gas), and
the wrong-selector transaction uses 1,331,539 gas. MSM/pairing use measured
424,560/168,100 gas. Actual calldata is 2,564 bytes, including the 512-byte
EIP proof and 60 public scalars. Deployment uses 6,631,812 gas; runtime is
1,807 bytes. G5-A's shared-runner regression retains its original gas result.

This is `PASS_G5B_LOCAL_PRAGUE_TWO_SUBSET_P3_GROTH16_VERIFIER`. It covers
exactly the `{1,2,3}`/`{1,2,4}` P3 cryptographic relation in local Prague EVM.
Registry/frozen-target comparisons, commitment-signature checks, winner writes,
SnarkPack verification, full DA and public-chain finality remain outside this
gate. The complete target-chain Manifest capability remains blocked. See
`docs/G5_B_PRAGUE_DISCLOSURE_VERIFIER.md`.

## Pre-DKG binding order

The native experiment now freezes `policy_pre` before DKG, derives the pre-key
context and unique DKG session from it, includes those values in the DKG
transcript and KeyCore, and derives the final context after certification.
Protocol version `0.3-PREPOLICY-V2` and registration version `2` identify the
new encoding. Policy replacement, stale-context, cross-election replay and
stale-session tests pass.

## Deliberately unimplemented

- Scale SnarkPack experiments and a production aggregation ceremony. G1-C1
  covers only the minimum batch of two distinct registered voters.
- Recursive/linked state proofs and larger-batch measurements. G2-B covers the
  fixed-`B=2` relation only.
- A general-subset production P3. G4-B publicly binds `{1,2,3}` and `{1,2,4}`
  in one circuit and reuses one Groth16 parameter set across them, but the other
  eight legal 3-of-5 subsets and a production ceremony remain unimplemented.
- A complete friendly-curve proof-system migration. The native G0.8 path and
  fixed-123 complete P3 and fixed-suite weighted-ballot relations now have real
  Groth16 proofs, a minimum-batch SnarkPack run and a real fixed-B=2 Jubjub
  state proof, but the general production suite is not complete.
- Mid-write crash recovery, remote transport and Byzantine-process tests. G3-B
  covers one orderly post-checkpoint restart over authenticated local files.
- Complete target-chain settlement, SnarkPack EVM verification, full batch DA
  and public-chain confirmation costs. G5-A/G5-B cover local Prague execution
  of the G2-B state-proof and two-subset P3 verifiers; accepted-election-state
  comparisons and settlement writes remain unimplemented.

No synthetic measurements are emitted for these items. Their manifest states
start with `BLOCKED_`, and `validate_full_submission()` refuses the manifest.

## Security interpretation

The code is research experiment code, not an audited voting implementation.
Passing tests means the implemented relations and fault cases behaved as
specified for those tests. It is not evidence for universal cryptographic
security, privacy of an operational election, or production readiness.

