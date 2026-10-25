# G2-B fixed-B=2 regional state-transition Groth16

## Result

G2-B passes the real Groth16 gate over the unchanged G2-A relation. The proof
binds the frozen 37-field public statement, including the canonical ordered
batch root, certified key information and all six old/new ciphertext
components. Its witness is the same pair of complete, same-region public ballot
records authenticated by the passing G1-C1 SnarkPack experiment.

## Measured Release run

- Batch size / candidates / region: 2 / 3 / 4.
- Constraints: 1,360,002.
- Public inputs: 37.
- Witness variables: 1,329,560.
- Setup: 13,254.145 ms.
- Prove: 8,776.350 ms.
- Verify: 2.660 ms.
- Changed-public-batch-root verification: 2.434 ms and rejected.
- Proof: 192 bytes.
- Compressed proving key: 419,767,248 bytes.
- Compressed verifying key: 2,168 bytes.
- Peak RSS after setup / prove / verify: 2,676.859 / 3,090.062 / 3,090.062 MiB.
- Proving key persisted: no.
- Private witness emitted: no.

## Claim boundary

This is a real proof for the fixed `B=2`, three-candidate experimental suite,
not a larger-batch performance result or a production trusted-setup ceremony.
The deterministic setup and proof seeds make this experiment reproducible.
Accepted-root, spent-nullifier and stale-state checks remain settlement-layer
checks as specified by the paper.

## Reproduce

First reproduce G1-C1 under the active Manifest, then run:

```bash
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-crypto \
  --bin friendly-state-transition-g2a -- \
  --groth16-output experiments/g2_b_raw/state_transition_b2_groth16.json
```

Artifacts:

- `experiments/g2_b_raw/state_transition_b2_groth16.json`
- `experiments/g2_a_raw/state_transition_b2_r1cs.json`
- `fixtures/jubjub_state_g2a_public_layout_v1.json`

The next necessary gate is G3-A: split only the existing five-Trustee native
DKG flow into independently persisted processes before adding restart or fault
matrices.
