# G0.9-H4-B real Groth16 for one-Trustee CP

Status: `PASS_H4B_REAL_GROTH16_ONE_TRUSTEE_CP`.

H4-B runs real BLS12-381 Groth16 setup, proof generation and verification over
the exact H4-A relation. The circuit recomputes the adopted CP-H2F-v3 challenge
from the canonical 684-byte transcript and checks all four generalized
Chaum--Pedersen group equations for one Trustee.

## Real Release measurement

The command was:

```bash
cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-crypto --bin friendly-cp-groth16-h4b
```

The single necessary measurement produced:

- constraints: `1,207,066`;
- packed public inputs: `23`;
- witness variables: `1,164,214`;
- setup: `14,025.404 ms`;
- proof generation: `8,206.572 ms`;
- verification: `3.017 ms`;
- proof: `192 bytes`;
- compressed proving key: `380,080,848 bytes`;
- compressed verifying key: `1,496 bytes`;
- peak RSS after setup: `2,686.656 MiB`;
- peak RSS after proving: `2,886.219 MiB`.

The valid proof verifies. Replacing one byte in the packed public transcript
causes verification to fail. No additional negative-test matrix or repeated
performance campaign was run at this gate.

The proving key was measured in memory and was not persisted. Deterministic
experiment seeds make this run reproducible; they are not a production trusted
setup ceremony.

## Claim boundary and next gate

This result completes G0.9-H4 for one Trustee. It is not a proof over three
selected Trustees, does not open private-package commitments, does not perform
Lagrange threshold combination, and does not prove tally or winner correctness.
It is therefore not full P3.

The next necessary stage is G0.9-P3-A. First freeze the formal P3 public-input
and private-witness layout; then compose exactly three distinct, selected
Trustee CP relations in one circuit and measure that relation before adding
commitment openings, interpolation or winner logic.

Raw result:
`experiments/g0_9_h4b_raw/h4b_groth16.json`.
