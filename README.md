# VeriVoting-Batch VS Code experiment starter

This repository is an executable **native-core and P1** experiment derived from
the paper's V4.5 protocol description.  It is intentionally fail-closed: the
runner validates one versioned protocol manifest and does not report unfinished
larger-batch/recursive state proofs, general-subset P3, remote multiprocess
operation, broad crash matrices, or on-chain work as measurements.

## What is implemented

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
| G2-A fixed-B=2 state transition | Canonical ordered batch root and six-component Jubjub accumulator relation | PASS R1CS; 1,360,002 constraints, no Groth16 |
| G2-B fixed-B=2 state proof | Real Groth16 over the unchanged G2-A relation | PASS; 192-byte proof, changed public batch root rejected |
| G3-A process-separated DKG | Five concurrent Trustees, persisted private state and session-bound signed local transport | PASS happy path; fixed `{1,2,3}` threshold result, no restart test |
| G3-B restart recovery | Trustee 3 exits after state persistence and resumes under a new PID | PASS one restart; transcript/KeyCore/tally preserved |
| G4-A second-subset P3 shape | Complete P3 R1CS for `{1,2,3}` and `{1,2,4}` through one fixed-width coefficient path | PASS equal count shape; 6,236,532 constraints and 59 public inputs, no public coefficient binding/Groth16 |
| G4-B public two-subset P3 | Public selector binds `{1,2,3}`/`{1,2,4}` IDs and derives their coefficients in one complete-P3 circuit | PASS real Groth16 with one shared setup; 6,231,973 constraints, 60 public inputs and 192-byte proof; not all ten subsets |
| G5-A local Prague state verifier | Solidity/EIP-2537 verification of the real 37-input G2-B proof | PASS; 940,640 gas and 1,828-byte calldata; wrong root reverts |
| G5-B local Prague P3 verifier | Same Solidity source verifies the real 60-input two-subset G4-B disclosure proof with its own pinned key | PASS; 1,331,473 gas and 2,564-byte calldata; wrong selector reverts |
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

