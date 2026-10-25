# G1-C0 minimum-batch SnarkPack compatibility

## Result

G1-C0 passes the narrow compatibility gate. Two independently randomized
Groth16 proofs of the exact fixed-suite G1-B1 `C_vote` relation are generated
with `ark-groth16 0.4.0`, converted to the canonical `blstrs 0.7.0` BLS12-381
representation, individually verified by `bellperson 0.27.0`, and aggregated
with bellperson SnarkPack V2. The aggregate verifies, while replacing the first
proof's public final context is rejected.

This is a real SnarkPack code path, not a simulated timing. It is deliberately
the minimum supported batch of two and therefore is not a scale or compression
claim.

## Measured Release run

- Source relation: 854,454 constraints and 25 public inputs per proof.
- Source proofs: two distinct 192-byte proofs, 384 bytes total.
- Second source proof: 4,446.743 ms.
- Arkworks-to-bellperson conversion: 4.760 ms.
- Functional aggregation SRS generation: 4.344 ms.
- SnarkPack V2 aggregation: 4.003 ms.
- Aggregate verification: 3.153 ms.
- Two individual bellperson verifications: 0.980 ms total.
- Aggregate proof: 5,236 bytes.
- Peak RSS: 2,134.141 MiB.
- Valid aggregate: accepted.
- Changed public context and its corresponding transcript: rejected.

The aggregate is larger and slower to verify than two individual proofs at
this minimum batch. That is expected for a logarithmic aggregation protocol
with fixed proof overhead and does not justify a performance claim.

## Trust and claim boundary

The aggregation SRS is produced by bellperson `setup_fake_srs` from a fixed
seed and is not persisted. It supports reproducibility of the functional gate
only; it is not a production ceremony. The source proving key, source proofs
and private witness are also not persisted. Both proofs use the same valid
public statement with independent proof randomness, so G1-C0 establishes
format and verifier compatibility, not a multi-voter batch result.

## Reproduce

```bash
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-ballot-cvote-b1 -- \
  --output experiments/g1_b1_raw/full_c_vote_r1cs.json \
  --layout-output fixtures/jubjub_ballot_cvote_b1_public_layout_v1.json \
  --groth16-output experiments/g1_b2_raw/full_c_vote_groth16.json \
  --snarkpack-output experiments/g1_c0_raw/snarkpack_min_batch_2.json
```

Raw measurement:
`experiments/g1_c0_raw/snarkpack_min_batch_2.json`.

## Next necessary gate

G1-C1 should aggregate the smallest batch of distinct registered-voter ballot
statements, including distinct nullifiers and ciphertexts, using the same
frozen transcript encoding. Do not begin large-batch scaling until that gate
passes.
