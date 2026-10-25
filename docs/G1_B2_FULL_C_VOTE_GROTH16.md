# G1-B2 complete fixed-suite ballot Groth16

Status: `PASS_G1B2_REAL_GROTH16_FULL_C_VOTE_FIXED_DEPTH2_FOUR_REGION`.

G1-B2 runs real BLS12-381 Groth16 over the exact five-condition ballot
relation frozen and measured in G1-B1. It does not change the circuit, public
input order, four-leaf registration shape or four-region assignment.

## Real Release measurement

| Item | Result |
| --- | ---: |
| Constraints | 854,454 |
| Public inputs | 25 |
| Witness variables | 839,832 |
| Witness synthesis | 777.431 ms |
| Setup | 8,015.162 ms |
| Prove | 4,623.984 ms |
| Verify | 2.386 ms |
| Changed-context verification | 2.135 ms |
| Proof | 192 bytes |
| Compressed proving key | 251,898,000 bytes |
| Compressed verifying key | 1,592 bytes |
| Peak RSS | 1,880.547 MiB |

The valid proof verifies. The same proof fails when only the public final
context is changed. This is the single necessary negative test for the gate.

## Claim boundary and next gate

The result is a real proof for the fixed four-leaf, depth-two and four-region
experiment suite. Its deterministic setup seed supports reproducibility and is
not a production ceremony. The proving key and private witness are not
persisted.

G1-C0 subsequently pins bellperson 0.27.0 and runs real SnarkPack V2 over the
minimum batch of two converted G1-B2 proofs. See
`docs/G1_C0_SNARKPACK_COMPATIBILITY.md`. This G1-B2 result itself remains a
single-proof measurement.

## Artifacts

- Raw result: `experiments/g1_b2_raw/full_c_vote_groth16.json`
- Public layout: `fixtures/jubjub_ballot_cvote_b1_public_layout_v1.json`
- Circuit: `crates/vv-friendly-crypto/src/ballot_identity_gadget.rs`
- Runner: `crates/vv-friendly-dkg/src/bin/ballot_cvote_b1.rs`

The measurement was produced from the current local worktree. It is not a
claim that the changes are committed or uploaded.
