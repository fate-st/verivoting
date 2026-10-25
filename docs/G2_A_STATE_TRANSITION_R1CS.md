# G2-A fixed-B=2 regional state-transition R1CS

## Result

G2-A passes the minimal state-relation gate. It consumes the exact public ballot
records for registered voters 1 and 2 from the passing G1-C1 artifact. Both
records belong to region 4 and have distinct voting nullifiers and ciphertexts.

The public state statement follows the paper's order:

`(eid, ctx_e, k, b, B, R_batch, keySetHash, PK_enc, CT_old, CT_new)`.

The circuit reconstructs each record's frozen 25-field ballot vector, computes
the two domain-separated batch leaves and inner node, and enforces the ordered
root. It then folds both ballots into all six ciphertext components. Old-state
points are listed before new-state points in the public layout.

## Measured Release run

- Batch size / candidates / region: 2 / 3 / 4.
- Constraints: 1,360,002.
- Public inputs: 37.
- Witness variables: 1,329,560.
- Batch hashes: 3.
- Accumulator components / Jubjub additions: 6 / 12.
- Witness synthesis: 1,145.101 ms.
- Negative-witness synthesis: 1,226.160 ms.
- Peak RSS: 1,196.984 MiB.
- Valid transition: accepted.
- Substituted ciphertext with unchanged public root/state: rejected.
- Groth16: not run.

## Claim boundary

This is the exact fixed-`B=2` R1CS relation, not a state proof or a scale result.
The transaction-level checks for an accepted registration root, unused
nullifiers and equality with the current contract state remain external checks,
matching the paper's settlement algorithm.

## Reproduce

First reproduce G1-C1, then run:

```bash
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-crypto \
  --bin friendly-state-transition-g2a
```

Artifacts:

- `experiments/g2_a_raw/state_transition_b2_r1cs.json`
- `fixtures/jubjub_state_g2a_public_layout_v1.json`

G2-B now proves this unchanged relation with real Groth16 and a single
changed-public-batch-root verification test. See
`docs/G2_B_STATE_TRANSITION_GROTH16.md`.
