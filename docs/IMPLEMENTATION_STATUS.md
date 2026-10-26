# Implementation and claim boundary

Current checkpoint (2026-10-26): B14's isolated low-allocation `_sub2`
comparison passes the saved public input and one modular-wrap boundary.
B12 remains the latest measured Miller prefix, and B7 the latest actual
full-loop attempt on an older variant. Next is B15: exact `_sub2` integration
and the original prefix pair. See the B12-B14 sections for measured scope.

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

## G5-C1 SnarkPack public fixture and runtime prerequisites

The distinct-voter B=2 runner now optionally exports the real aggregate wire
proof, source Groth16 VK, public verifier SRS, two public statements and exact
external statement transcript. Binary proof/VK and public SRS encode/decode
roundtrips pass; verification with the decoded materials accepts the aggregate
and rejects the same proof after changing only the second nullifier.

The aggregate is 5,236 bytes and its external transcript is 1,654 bytes.
Rust/JavaScript external bytes match. Local Prague SHA-256, G2 MSM and pairing
true/false probes pass, but they do not execute an aggregate verifier.
The pinned bellperson implementation requires GT arithmetic and comparison
against non-unit GT targets that the existing EIP-2537 Groth16 route does not
provide. Exact SnarkPack EVM validation remains
`BLOCKED_EXACT_SNARKPACK_V2_EVM_VERIFIER_NOT_IMPLEMENTED`, with verification
gas unset. This is not a claim that an EVM implementation is impossible.

The Manifest and suite are unchanged. G5-C2 subsequently completes internal
transcript conformance using the saved public proof, not another proof run. See
`docs/G5_C1_SNARKPACK_ROUTE.md` and
`experiments/g5_c_raw/snarkpack_route.json`.

## G5-C2 exact official internal transcript conformance

The saved B=2 public aggregate is replayed through a temporary logging-only
copy of the pinned official Transcript implementation; `verify.rs` is unchanged.
Independent JavaScript wire decoding reconstructs 15 GT elements and all six
internal challenges per case. The complete bases, accepted counters and
canonical scalars match 12/12 official calls across the valid case and the
single changed-second-nullifier case. Three rejected candidates occur in the
real fixture, without a synthetic rejection experiment.

Complete native verification accepts the proof and rejects the changed
statement; hcom remains unchanged and the other five challenges change.
Setup/prove are not run. The temporary manifest removes only an inactive
CUDA-supraseal dependency conflict; active CPU features and fixed cryptographic
dependency versions match the original. Installed sources, the project lock
and the source fixture are unchanged.

This is `PASS_G5C2_OFFICIAL_INTERNAL_TRANSCRIPT_RUST_JS_CONFORMANCE`, not
Solidity/EVM aggregate verification. GT arithmetic and pairing-to-target
validation remain blocked, aggregate gas is unset and the Manifest is unchanged.
Next is the minimal G5-C3 exact-GT-route gate. See
`docs/G5_C2_TRANSCRIPT_CONFORMANCE.md` and
`experiments/g5_c2_raw/transcript_conformance.json`.

## G5-C3-A proof-derived Fp12 multiplication component

An experimental Solidity component now performs the complete Fp12 product of
two GT operands decoded from the saved real B=2 proof. Base-field multiplication
uses two real MODEXP squares with the full 381-bit modulus and modular halving;
54 base multiplications cause 108 MODEXP calls. Native blstrs, independent
Noble decoding/product and the Solidity returned value agree.

The valid local Prague signed transaction uses 609,940 total gas and 2,500
calldata bytes; the same operands with only one canonical target coefficient
changed revert using 609,708 gas. Runtime size is 3,423 bytes. These are
component measurements, not aggregate-verifier gas or settlement costs.

This is `PASS_G5C3_FP12_PRODUCT_COMPONENT_ONLY`. The two proof-derived
commitments are not a complete actual GIPA subexpression. GT subgroup/wire
binding in Solidity, scalar powers and pairing-to-nonunit-GT comparison remain
unimplemented. Complete G5-C3 and exact SnarkPack EVM verification stay blocked,
aggregate gas is null, no setup/prove is rerun and the Manifest remains unchanged.
The following G5-C3-B section records that single actual TIPP endpoint route.
See `docs/G5_C3_FP12_PRODUCT.md` and
`experiments/g5_c3_raw/fp12_product_evm.json`.

## G5-C3-B actual TIPP endpoint conformance; exact EVM route still blocked

The same saved B=2 proof now fixes the real relation
`e(final_a,final_b) = ip_ab * z_ab_left^c * z_ab_right^c_inv`.
Rust uses the official wire reader and the saved C2 official challenges;
independent JavaScript reconstructs all six complete challenge traces directly
from the original proof/external statement before computing the endpoint.
The native and JS pairing and folded target agree in all twelve Fp coefficients.
One target-only mutation, multiplication by nonidentity `proof.ip_ab`, is
rejected by both native and JS while remaining in the prime-order GT subgroup.

The exact endpoint points also execute successfully at Prague `0x0f`, consuming
70,300 primitive gas and returning 32-byte false because the pairing is not one.
No GT target is passed to that ABI. This is not an EVM endpoint rejection or
acceptance test, and no EVM wrong-target rejection is claimed.

Status is `PARTIAL_PASS_TIPP_ENDPOINT_CONFORMANCE_EVM_TARGET_BLOCKED`.
Software Miller loop/final exponentiation and exact GT-target comparison are
missing. This is an implementation/API gap, not measured OOG or a proof of
infeasibility. Complete G5-C3, SnarkPack EVM and full settlement remain blocked;
aggregate gas is null, setup/prove are not rerun and the Manifest is unchanged.
The following G5-C3-B2 section records that software component attempt.
See `docs/G5_C3_TIPP_ENDPOINT.md` and
`experiments/g5_c3_raw/tipp_endpoint_route.json`.

## G5-C3-B2 software Miller prefix passes; full loop OOG at 20M budget

A separate Solidity component now computes projective G2 updates, signed NAF
lines and Fp12 Miller arithmetic internally from the saved actual endpoint.
The old product contract remains unchanged. The pinned Noble schedule has
64 steps/69 lines; incomplete prefixes are unconjugated, while the full loop
applies negative-x conjugation. No host precomputed line or intermediate state
is used as the contract's computation.

The two-step prefix exercises doubling, signed -Q addition, a square and three
line products. It matches the full reference output in a signed local Prague
transaction using 4,805,196 total gas, 1,316 calldata bytes and 642 successful
MODEXP calls. A canonical wrong result reverts with INVALID_MILLER_RESULT using
4,804,731 gas. The new runtime is 5,675 bytes.

Exactly one full 64-step attempt consumes the fixed 20,000,000 gas cap and
returns out of gas. Exit-state active memory is 85,860 EVM words; all 1,530
MODEXP calls already executed succeeded. This is a failed-attempt budget spend,
not a measured completed-loop cost. Status is
`PARTIAL_PASS_MILLER_PREFIX_FULL_LOOP_OOG_20M`; full EVM Miller conformance,
final exponentiation, exact GT comparison and SnarkPack verification are not
claimed. No curve/subgroup/proof binding is added by this arithmetic component.

The following G5-C3-B3 section records memory-only optimization against B2,
with the same prefix checks and one full attempt under the same budget.
Setup/prove, Cargo.lock and
the Manifest remain unchanged. See `docs/G5_C3_MILLER_LOOP.md` and
`experiments/g5_c3_raw/miller_loop_evm.json`.

## G5-C3-B3 memory-only Miller optimization; full loop still OOG at 20M

A separate contract deeply allocates persistent Q/R/accumulator/line objects,
copies all live scalar limbs from each operation's scratch result, then rewinds
only the temporary allocator. The point/line/field/tower/codec bodies and NAF
are unchanged from B2. Q and mutable R do not alias. The reset is not annotated
memory-safe and the compiler remains pinned to solc0.8.30/noIR/optimizer200.

On the identical two-step prefix, signed local Prague verification uses
2,371,368 total gas instead of 4,805,196 (50.65% reduction), and active EVM
memory words are 7,123 instead of 35,464 (79.91% reduction). Both variants run
642 successful MODEXPs. The same wrong result reverts using 2,371,186 gas.
This is a one-component, one-input comparison, not an end-to-end speedup.

Exactly one full attempt still hits the unchanged 20M cap with out of gas;
active memory is 7,203 words and 5,636 MODEXPs have succeeded by then. The
failed run is not full-loop correctness evidence or completed-loop gas.
Status is `PARTIAL_PASS_MILLER_MEMORY_REDUCED_FULL_LOOP_OOG_20M`.
Old B2 artifacts remain unchanged; no new proof/fixture or setup/prove was run.
Full Miller, EVM final exponentiation, GT comparison and SnarkPack stay blocked;
the Manifest is not promoted and aggregate gas is null.

The following G5-C3-B4 section records dedicated square/sparse-line arithmetic
using the same fixtures, two prefix cases and one full attempt at the same cap.
See `docs/G5_C3_MILLER_MEMORY.md` and
`experiments/g5_c3_raw/miller_memory_evm.json`.

## G5-C3-B4 dedicated square/014 arithmetic; full loop still OOG at20M

A separate contract replaces general self-multiplication with general Fp12
squaring (two Fp6 products), and dense line multiplication with sparse014
(13 Fp2 products plus four Fp coordinate-scaling products). This is not GT-only
cyclotomic squaring. Base-field arithmetic, point/NAF logic and the B3 memory
scheme are unchanged. The actual trace's63 squares/69 lines match generic host
reference operations; this does not establish full-loop EVM correctness.

The same two-step EVM positive uses1,865,022 total gas, down21.35% from B3's
2,371,368; MODEXP calls fall from642 to516, all successful. Memory is5,233words
and calldata remains1,316B. Wrong result reverts using1,864,840gas. Runtime is
6,412B. Exactly one full attempt still OOGs at the unchanged20M cap;5,556
MODEXPs succeeded before failure, with5,315 active memory words. That capped
spend is not completed-loop, pairing or aggregate-verifier gas.

Status is `PARTIAL_PASS_SPARSE_MILLER_PREFIX_FULL_LOOP_OOG_20M`; full Miller,
final exponentiation, GT binding and SnarkPack EVM remain blocked, aggregate
gas is null and all B2/B3 sources/results/fixtures and Manifest remain unchanged.
No setup/prove or scale run is performed. The following B5 section records
isolated Fp/Fp2 profiling before another full-loop retry. See
`docs/G5_C3_MILLER_SPARSE.md` and `experiments/g5_c3_raw/miller_sparse_evm.json`.

## G5-C3-B5 isolated Fp/Fp2 cost profile complete; no Miller rerun

A separate instrumented contract copies the15 existing field/codec functions
verbatim. One real Fp product (final_b.x.c0 times y.c0) and one Fp2 product
(final_b.x times y) both match direct BigInt and Noble reference results.
The canonical-decode-to-output-encode arithmetic windows measure4,372/16,468gas;
actual MODEXP execution is400/1,200gas in2/6calls. Transaction totals are
29,813/45,399gas, not the multiplication-window cost. Root opcode+child execution
and intrinsic/transaction gas reconcile, with no calldata floor applied.

Window memory expansion is211/899gas; allocator growth is2,112/7,872bytes.
Gasleft/compiled-call overhead remains in the windows. STATICCALL/memory counters
may overlap, and residual gas is not exclusively attributed to one category.
These are isolated call-site costs, not per-operation constants for full Miller.

Status is `PASS_ISOLATED_FP_FP2_COST_PROFILE_ONLY`. Only the two arithmetic
calls and one deployment run; no full Miller, negative matrix, setup/prove or
scale rerun is added. B2/B3/B4 remain unchanged. Full Miller/final exponentiation/
GT binding/SnarkPack and Manifest capabilities stay blocked; aggregate gas is null.
The following B6 section records the low-level Fp variant and necessary
arithmetic correctness checks before integration, without a full-loop retry.
See `docs/G5_C3_FIELD_COST_PROFILE.md` and
`experiments/g5_c3_raw/field_cost_profile.json`.

## G5-C3-B6 low-level Fp comparison passes; no Miller rerun

A separate contract changes only the Fp multiply implementation and contract
name; both profile windows and all other arithmetic/codec functions are
unchanged. The same 381-bit identity uses two actual MODEXP calls, with explicit
low-limb carry/borrow, canonical add/subtract and two modular halves. Its two
squares reuse one 256-byte input and one 64-byte result slot; there is no memory
watermark reset or trusted host multiplication result.

The exact B5 public products match again. Fp/Fp2 arithmetic windows fall from
4,372/16,468 to 2,167/9,965 gas (50.43%/39.49% reductions); signed transaction
totals are 27,606/38,887 gas. Actual MODEXP calls remain 2/6, costing 400/1,200 gas.
Window allocator growth falls from 2,112/7,872 to 384/2,816 bytes, and window memory
expansion from 211/899 to 37/295 gas. These are isolated instrumented call-site
measurements, not inlined Miller constants or peak host-memory measurements.

All four necessary deterministic boundary transactions match the direct
reference: zero; `(p-1)^2`; `a=1, b=(3p+2*2^256-1)/4` (both odd modular halves
carry out of the low limb); and `a=2^256+1, b=2^256-1` (sum carry and difference
borrow). These are correctness checks, not extra performance samples.

Status is `PASS_LOW_LEVEL_FP_FIXED_CASES_AND_COST_REDUCTION_ONLY`. One deployment,
the two fixed cost cases and four boundary cases were executed locally in
Prague EVM with Solc 0.8.30/optimizer 200/no viaIR. No full Miller, setup/prove,
scale experiment or RPC transaction is run; B2-B5 baselines and both Manifests
remain unchanged. Full Miller/final exponentiation/GT binding/SnarkPack remain
BLOCKED and aggregate-verifier gas is null.

The following B7 section records integration of this same `_mul` into a new
sparse-Miller variant and its one fixed-budget full-loop attempt. The primitive
results above remain historical isolated measurements. See
`docs/G5_C3_FIELD_LOWLEVEL.md` and
`experiments/g5_c3_raw/field_lowlevel_profile.json`.

## G5-C3-B7 low-level Miller integration; full loop still OOG at 20M

A separate contract integrates the exact B6 `_mul` into the B4 sparse-Miller
implementation. Apart from the contract name and this function, the source is
unchanged: all 51 other functions, field/codec, sparse formulas, point/NAF logic
and arena lifetime scheme are preserved. Solc 0.8.30, optimizer 200, Prague and
the legacy pipeline remain pinned; deployed runtime is 6,711 bytes.

The same real two-step prefix matches all twelve reference coefficients using
1,290,640 total gas, down 30.80% from B4's 1,865,022. Active memory at exit falls
from 5,233 to 2,829 words (45.94%). All 516 MODEXP calls succeed, unchanged in
count. The canonical wrong result reverts with `INVALID_MILLER_RESULT` using
1,290,458 gas. Exactly one full 64-step transaction is attempted at the unchanged
20M cap and fails with root out-of-gas after 7,848 successful MODEXP calls;
active memory at failure is 3,147 words. The capped spend is not a completed
Miller-loop, pairing or aggregate-verifier cost, and does not establish complete
loop correctness.

Status is `PARTIAL_PASS_LOW_LEVEL_MILLER_PREFIX_FULL_LOOP_OOG_20M`, with target
status `BLOCKED_SOFTWARE_MILLER_LOOP_AFTER_LOW_LEVEL_MUL_EXCEEDS_20M`.
Full Miller, EVM final exponentiation, GT-target binding and exact SnarkPack
verification remain BLOCKED; aggregate-verifier gas is null. B2-B6 baselines,
fixtures, locks, protocol suite and both Manifests remain unchanged. No
setup/prove, scale run, RPC transaction or budget increase is performed.

The following B8 section completes the planned isolated `_mul2` allocation
comparison without retrying the full loop. The B7 result above remains the
latest full-loop attempt; no budget or verifier-capability promotion follows.
Reproduce with `npm run --prefix contracts g5c3b7`; see
`docs/G5_C3_MILLER_LOWLEVEL.md` and
`experiments/g5_c3_raw/miller_lowlevel_evm.json`.

## G5-C3-B8 low-level Fp2 comparison passes; no Miller rerun

A separate instrumented contract changes only `_mul2` and the contract name
relative to B6; `_mul`, all other field/codec functions and profile entry points
remain byte-for-byte unchanged. The same three Karatsuba products use six real
MODEXP calls. Canonical sums use two independent buffers. The exclusive t2
result is reused for c1, and only after both c1 subtractions finish is t0 reused
for c0. Inputs are not overwritten and the free-memory pointer never moves
backwards.

The same public Fp2 product matches direct BigInt and Noble references. Its
arithmetic window falls from 9,965 to 8,086 gas (18.86%), transaction total from
38,887 to 37,022 gas, allocator growth from 2,816 to 1,664 bytes, and window
memory-expansion cost from 295 to 171 gas. Six MODEXP calls still cost 1,200 gas.
The unchanged Fp control window remains 2,167 gas with two MODEXP calls and
384-byte allocator growth; its transaction total rises by 8 gas, from 27,606 to
27,614. These are isolated call-site measurements, not universal operation
constants or evidence that every transaction is faster.

All three necessary deterministic Fp2 boundaries pass: u squared; the square of
`(p-1)+(p-1)u`; and the low-limb carry/borrow pair. They check complete returned
products and are correctness checks rather than extra performance samples.
Status is `PASS_FP2_FIXED_CASES_AND_COST_REDUCTION_ONLY`. B8 executes one local
deployment, the two original cost cases and those three boundaries only. No
Miller loop, setup/prove, scale or RPC run is performed. B2-B7 baselines,
fixtures, locks, protocol suite and both Manifests remain unchanged.

Full Miller, EVM final exponentiation, GT-target binding and exact SnarkPack
verification remain BLOCKED, with null aggregate-verifier gas. The latest full
attempt is still B7's root OOG at the unchanged 20M cap. The following B9 section
completes the planned exact `_mul2` integration and prefix pair only, without a
full-loop retry, cap increase or final exponentiation.
Reproduce with `npm run --prefix contracts g5c3b8`; see
`docs/G5_C3_FP2_LOWLEVEL.md` and
`experiments/g5_c3_raw/fp2_lowlevel_profile.json`.

## G5-C3-B9 exact Fp2 integration passes the prefix pair; no full attempt

`Bls12381MillerLoopFp2LowLevel` changes only the contract name and copies the
exact B8 `_mul2` into B7. The rest of the source remains byte-for-byte unchanged,
including `_mul`, point formulas, NAF schedule, sparse arithmetic, encoding and
arena boundaries. The same fixture, compiler settings, curve, locks and both
Manifests are preserved.

The same real two-step prefix matches all returned coefficients in the local
Prague EVM at 1,132,763 gas, versus B7's 1,290,640 (12.23% less). Active memory
at exit falls from 2,829 to 2,411 words (14.78% less). The canonical wrong target
reverts with `INVALID_MILLER_RESULT` at 1,132,581 gas. Both transactions complete
all 516 MODEXP calls successfully; the arithmetic operation count is unchanged.
These are fixed-input prefix integration measurements, not completed-loop costs
or statistical election-performance results.

Status is `PASS_FP2_MILLER_PREFIX_INTEGRATION_ONLY`. This gate performs one
deployment and exactly those two prefix transactions. `full_loop_attempts` is
zero and `full_attempt` is null. The latest full attempt remains B7's historical
root OOG at the unchanged 20M cap for the older variant. B9's new variant has
not run all 64 steps, so it is neither a full-loop pass nor a measured 20M OOG.
The inherited target-verifier BLOCKED status cites the B7 raw record; full
Miller, EVM final exponentiation, GT-target binding and exact SnarkPack remain
unfinished, with null aggregate-verifier gas. No setup/prove, scale, RPC or
protocol-capability promotion is performed.

The following B10 section completes the planned original Fp2 `_add2` cost
localization on the same public operands. It does not implement another
optimization or assume addition is the only bottleneck; no full-loop retry,
cap increase or final exponentiation belongs in that profiling gate.
Reproduce with `npm run --prefix contracts g5c3b9`; see
[the B9 report](G5_C3_MILLER_FP2_PREFIX.md) and
[the raw record](../experiments/g5_c3_raw/miller_fp2_prefix_evm.json).

## G5-C3-B10 original Fp2-add cost localization; arithmetic unchanged

The new profiling entry point wraps the unchanged `_add2` after canonical
decoding and before output encoding. All original field/codec bodies remain
unchanged. One deployment and one fixed public Fp2 addition are executed;
no optimization or extra arithmetic test is implemented. The operands are the
same proof-derived affine x and y used in B8, and the full result matches direct
BigInt and Noble references.

The measured arithmetic window is 1,752 gas, allocator-pointer growth is 896 B,
and memory-expansion cost within that window is 91 gas. There are zero MODEXP
calls. Window gas includes compiler/call and instrumentation overhead; it is
not an inlined Miller constant or evidence that this is the unique bottleneck.
Allocator growth is not peak EVM memory or host RSS.

Execution gas is 5,740 and intrinsic gas is 24,952, totaling 30,692. The actual
transaction total is 30,880 because the Prague calldata floor adds 188 gas.
This pricing addition is outside the arithmetic window, not arithmetic or
memory-expansion cost. Status is `PASS_ISOLATED_FP2_ADD_COST_PROFILE_ONLY`.

No multiplication, Miller prefix/full loop, setup/prove, scale or RPC run is
performed. The latest prefix remains B9's pass; the latest full attempt remains
B7's historical 20M OOG for the older variant. B9's new variant has not had a
full-loop attempt. Full Miller and exact SnarkPack remain BLOCKED and aggregate
gas remains null; locks, suite and both Manifests are unchanged.

The following B11 section completes the planned isolated low-allocation
`_add2` comparison on the same public input plus one necessary correctness
boundary. Arithmetic-window gas, execution gas and allocator growth are the
comparison measures; Prague's floor can hide an execution saving in transaction
total. No Miller integration, full-loop retry, cap increase or final
exponentiation is performed in that comparison.
Reproduce with `npm run --prefix contracts g5c3b10`; see
[the B10 report](G5_C3_FP2_ADD_COST_PROFILE.md) and
[the raw record](../experiments/g5_c3_raw/fp2_add_cost_profile.json).

## G5-C3-B11 isolated low-allocation Fp2-add candidate passes fixed cases

The new profiling variant changes only the contract name and `_add2` relative
to B10; the entry points, all other arithmetic and codecs remain unchanged.
Only the original public addition cost case and one necessary boundary are
executed. The boundary `((p-1,0),(1,0))` tests equality at the modulus and zero
addition, returning the complete canonical zero result. Both full results
match the references. There are no other arithmetic tests in this gate.

For the original cost case, the arithmetic window falls from 1,752 to 849 gas
(51.54%), execution gas from 5,740 to 4,852, allocator-pointer growth from
896 to 384 B (57.14%), and window memory-expansion gas from 91 to 39. MODEXP
count remains zero. Transaction total stays 30,880 gas because the Prague
calldata-floor supplement increases from 188 to 1,076 gas. Thus this gate
demonstrates a measured window/execution/allocation reduction, not a lower
total transaction gas or an inlined Miller cost.

Status is `PASS_LOW_ALLOCATION_FP2_ADD_FIXED_CASES_ONLY`. B11 performs one
deployment and two arithmetic calls only; it does not integrate into or run
Miller. The latest prefix remains B9's pass, while the latest full attempt
remains B7's historical OOG at 20M for the older variant. B9's full loop remains
untested. Full Miller and exact SnarkPack remain BLOCKED, aggregate gas remains
null, and locks, suite and both Manifests are unchanged. No setup/prove, scale,
RPC or final-exponentiation experiment is performed.

The following B12 section completes the planned `_add2` integration and prefix
pair. B11 itself remains an isolated arithmetic comparison.
Reproduce with `npm run --prefix contracts g5c3b11`; see
[the B11 report](G5_C3_FP2_ADD_LOWLEVEL.md) and
[the raw record](../experiments/g5_c3_raw/fp2_add_lowlevel_profile.json).

## G5-C3-B12 low-allocation Fp2-add Miller prefix integration

The new Miller variant copies the exact B11 `_add2` into B9. Apart from that
function and the contract name, the complete source is unchanged, including
the other 51 functions, point formulas, NAF, field/codec and memory-lifetime
scheme. The same two-step positive matches all returned coefficients at
1,043,835 gas versus B9's 1,132,763 (7.85% less). Active memory falls from
2,411 to 2,079 words (13.77% less). The wrong canonical target rejects with
`INVALID_MILLER_RESULT` at 1,043,653 gas. Each executes 516 successful MODEXPs.

Status is `PASS_ADD2_MILLER_PREFIX_INTEGRATION_ONLY`. The runtime is 6,992 B
and deployment costs 1,565,064 gas, respectively 11 B and 2,328 gas more than
B9. These tradeoffs are preserved rather than claiming every cost improves.
There are zero full-loop attempts and `full_attempt` is null. The latest actual
full attempt remains B7's historical 20M OOG for its older implementation.
Neither full-loop correctness nor full-loop gas is measured for B12.

Reproduce with `npm run --prefix contracts g5c3b12`; see
[the B12 report](G5_C3_MILLER_ADD_PREFIX.md) and
[the raw record](../experiments/g5_c3_raw/miller_add_prefix_evm.json).

## G5-C3-B13 unchanged Fp2-subtraction cost baseline

A new profile contract preserves all 18 B11 functions and adds only the
contract-name change and one `profileFp2Sub` entry point. Its `_sub2` is also
byte-identical to B12. One deployment and one subtraction of the same public
endpoint x/y are executed; all 128 B of output match independent BigInt modular
subtraction and Noble. c0 takes direct subtraction with a low-limb borrow;
c1 takes direct subtraction without one. The modular-wrap branch is outside
this fixed cost sample.

The arithmetic window costs 1,286 gas, execution 5,296, allocator-pointer growth
640 B and window memory expansion 65 gas. There are zero MODEXP/STATICCALLs.
With zero refund, intrinsic 24,952 plus execution 5,296 is 30,248; Prague's
calldata floor produces total 30,880, including a 632 gas pricing supplement.
The supplement is outside the window. Allocator growth is not host RSS or EVM
peak memory. The cost window includes instrumentation and compiler overhead.

Status is `PASS_ISOLATED_FP2_SUB_COST_PROFILE_ONLY`. This is the saved baseline
for B14's low-allocation `_sub2` comparison described below. B13 implements no
arithmetic optimization,
Miller run, extra test matrix or proof generation. The latest prefix remains
B12 and the latest actual full attempt remains B7. Full Miller, final
exponentiation, GT binding and exact SnarkPack remain unfinished, aggregate
gas is null, and protocol capabilities are unchanged.

Reproduce with `npm run --prefix contracts g5c3b13`; see
[the B13 report](G5_C3_FP2_SUB_COST_PROFILE.md) and
[the raw record](../experiments/g5_c3_raw/fp2_sub_cost_profile.json).

## G5-C3-B14 isolated low-allocation Fp2-subtraction comparison

The candidate changes only the contract name and `_sub2` relative to B13;
all other 18 functions, including the timed entry point, remain byte-identical.
Its fresh result buffers compute canonical subtraction directly from two
limbs and correct a negative difference with one addition of p. Inputs are
read-only. One deployment, the original public input and one modular-wrap
boundary are executed. Both complete 128 B outputs match BigInt and Noble.

For the original input, the arithmetic window falls 1,286->765 gas (40.51%),
execution 5,296->4,790, allocator growth 640->384 B (40%), and window memory
expansion 65->39 gas. Total remains 30,880 because the calldata-floor supplement
increases 632->1,138 gas. Thus measured execution/allocation improve while this
transaction's total gas does not. The window remains an instrumented profile
measurement, not an inlined Miller cost.

The sole boundary is `(0,0) - (1,2^256) = (p-1,p-2^256)`. It checks modular
wrap with low-limb borrow/correction carry in c0 and without either in c1.
Its total is 27,574 gas and window 877; it is excluded from the performance
comparison sample. Both calls use zero MODEXP/STATICCALLs.

Status is `PASS_LOW_ALLOCATION_FP2_SUB_FIXED_CASES_ONLY`. No Miller run or
full-loop attempt occurs; latest prefix/full records remain B12/B7 respectively.
Full Miller, final exponentiation, GT binding and exact SnarkPack remain
unfinished with null aggregate gas. Next is B15: copy this exact `_sub2` into
a separate B12-based Miller variant and first check the original prefix pair.
No full-loop retry, budget increase or proof/scale run is scheduled.

Reproduce with `npm run --prefix contracts g5c3b14`; see
[the B14 report](G5_C3_FP2_SUB_LOWLEVEL.md) and
[the raw record](../experiments/g5_c3_raw/fp2_sub_lowlevel_profile.json).

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

