# G4-A second-subset complete-P3 R1CS shape

Status: `PASS_G4A_SECOND_SUBSET_COMMON_R1CS_COUNT_SHAPE`.

G4-A performs the smallest experiment needed before attempting a generalized
Trustee-subset Groth16 setup. One DKG and four-voter election instance is used
to synthesize the complete CP, authenticated-package, threshold-combination,
bounded-tally and public-winner relation for two legal subsets:

- `{1,2,3}` with coefficients `(3,-3,1)`; and
- `{1,2,4}` with coefficients `(8/3,-2,1/3)` in the Jubjub scalar field.

Both native paths verify three CP proofs and three authenticated packages,
recover the private tally `(4,2,4)`, and produce public winner `1`. Both R1CS
instances satisfy the complete relation.

## Release measurement

| Measurement | `{1,2,3}` | `{1,2,4}` |
|---|---:|---:|
| Constraints | 6,236,532 | 6,236,532 |
| Public inputs | 59 | 59 |
| Witness variables | 6,009,006 | 6,009,006 |
| Witness synthesis | 8,831.783 ms | 7,550.190 ms |

Peak process RSS was 5,504.094 MiB. The earlier specialized fixed-123 relation
used 6,205,629 constraints and 5,978,859 witness variables. The increase is the
cost of routing all three coefficients through one fixed-width generic scalar
multiplication path.

## Exact claim boundary

G4-A confirms equal constraint, public-input and witness counts for these two
instances and preserves the prior 59-element public layout. It does **not**
prove that their R1CS matrices are identical: the Lagrange coefficients are
still constrained setup-time parameters, not verifier-selected public inputs.
Consequently, G4-A does not claim proving-key reuse, arbitrary-subset P3, or a
universal setup. No Groth16 operation was run and the cryptographic Manifest
remains limited to the existing fixed-123 proof.

G4-B subsequently completed that two-subset follow-up: it publicly binds the
`{1,2,3}`/`{1,2,4}` choice and reuses one real Groth16 parameter set across
them. It still does not cover the other eight legal 3-of-5 subsets; see
`docs/G4_B_PUBLIC_SUBSET_GROTH16.md`.

## Reproduce

```bash
CARGO_BUILD_JOBS=2 cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-dkg --bin friendly-g4a-second-subset-shape -- \
  --manifest configs/protocol_manifest.JUBJUB-PROTOTYPE.json \
  --baseline experiments/g0_9_p3d1_raw/p3d1_full_groth16.json \
  --output experiments/g4_a_raw/second_subset_p3_shape.json
```

Raw result: `experiments/g4_a_raw/second_subset_p3_shape.json`.
