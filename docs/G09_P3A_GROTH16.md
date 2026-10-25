# G0.9-P3-A real Groth16 for three selected Trustee CPs

Status: `PASS_P3A_REAL_GROTH16_THREE_SELECTED_TRUSTEE_CP`.

This experiment runs real BLS12-381 Groth16 setup, proof generation and
verification over the exact P3-A R1CS containing the complete CP relations of
selected Trustees 1, 2 and 3. The public/private input layout is unchanged from
`VV/JUBJUB/P3-A/INPUT-LAYOUT/v1`.

## Real Release measurement

The command was:

```bash
cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-dkg --bin friendly-p3a-three-cp
```

The single necessary Groth16 run produced:

- constraints: `3,611,598`;
- packed public inputs: `45`;
- witness variables: `3,485,994`;
- setup: `36,858.792 ms`;
- proof generation: `23,581.980 ms`;
- verification: `5.349 ms`;
- proof: `192 bytes`;
- compressed proving key: `1,037,976,624 bytes`;
- compressed verifying key: `2,552 bytes`;
- peak RSS after setup: `5,182.062 MiB`;
- peak RSS after proving: `5,824.047 MiB`.

The valid proof verifies. Changing one byte in the first Trustee's public
statement prefix causes verification to fail. No additional negative-test
matrix or repeated performance campaign was run.

The proving key was measured in memory and not persisted. Deterministic seeds
are reproducibility inputs, not a production trusted-setup ceremony.

## Claim boundary and next gate

This completes P3-A: three selected complete CP relations have a real Groth16
proof while shares and CP proof values remain private. It is not full P3.
Private-package commitment openings, threshold combination, secret tally
binding and winner correctness are absent.

P3-B0 subsequently adopted the fixed-shape Package-H2F-v1 mapping, P3-B1 added
the three private-package Pedersen opening relations, and P3-C0 added the fixed
Trustee 1--3 threshold combination; see
`docs/G09_P3C0_THRESHOLD_COMBINATION.md`. P3-C1 subsequently added bounded
private tally binding; see `docs/G09_P3C1_BOUNDED_TALLY.md`. Winner logic and
another Groth16 run remain later stages.

Raw result:
`experiments/g0_9_p3a_raw/p3a_three_cp_groth16.json`.
