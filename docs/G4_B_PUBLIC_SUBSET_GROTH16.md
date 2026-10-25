# G4-B public two-subset complete-P3 Groth16

Status: `PASS_G4B_REAL_GROTH16_TWO_SUBSET_PUBLIC_SELECTOR`.

G4-B replaces G4-A's setup-time coefficient choice with one public Boolean
selector in a single complete-P3 circuit. The selector supports exactly two
ordered Trustee subsets:

- `false -> {1,2,3}` with Lagrange coefficients `(3,-3,1)`; and
- `true -> {1,2,4}` with coefficients `(8/3,-2,1/3)` in the Jubjub scalar
  field.

Each Trustee identifier is already encoded in the public 452-byte CP transcript
prefix. The circuit constrains those identifiers to the selected ordered subset
and conditionally selects the corresponding canonical scalar bits. Thus the
coefficients are derived from the public selector inside the relation rather
than supplied as unconstrained host-language parameters. The selector is public
input index 51 (zero based) in the frozen 60-field G4-B layout.

## Real Release run

One BLS12-381 Groth16 setup was generated from the `{1,2,3}` assignment. The
same parameters were then used to prove and verify the complete `{1,2,4}` P3
assignment.

| Measurement | `{1,2,3}` setup assignment | `{1,2,4}` proof assignment |
|---|---:|---:|
| Constraints | 6,231,973 | 6,231,973 |
| Public inputs | 60 | 60 |
| Witness variables | 6,005,106 | 6,005,106 |
| R1CS preflight | satisfied | satisfied |
| Witness synthesis | 8,415.777 ms | 7,152.295 ms |

Groth16 setup took 61,062.475 ms, proving took 38,615.505 ms and verification
took 6.529 ms. The proof is 192 bytes, the compressed proving key measured in
memory is 1,843,893,696 bytes, the compressed verifying key is 3,272 bytes and
peak process RSS was 8,177.266 MiB.

Two checks cover the public binding needed by this gate:

1. `{1,2,4}` transcript identifiers paired with selector `false` make the R1CS
   relation unsatisfied.
2. The valid `{1,2,4}` proof fails verification after changing only public
   selector input 51 from `true` to `false`.

The raw artifact does not emit plaintext group elements or private tallies. The
proving key is measured in memory and is not persisted.

## Exact claim boundary

This result establishes one common Groth16 circuit and one reusable parameter
set for the two selector values `{1,2,3}` and `{1,2,4}`. It does **not** support
the remaining eight legal 3-of-5 subsets and is not an arbitrary-subset or
production-ceremony result. The deterministic experiment setup is not a
production trusted setup. The cryptographic Manifest therefore remains at the
prior fixed-123 capability.

## Reproduce

```bash
CARGO_BUILD_JOBS=2 cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-dkg --bin friendly-g4b-public-subset-groth16 -- \
  --manifest configs/protocol_manifest.JUBJUB-PROTOTYPE.json \
  --output experiments/g4_b_raw/public_subset_groth16.json \
  --layout-output fixtures/jubjub_g4b_public_layout_v1.json
```

Raw result: `experiments/g4_b_raw/public_subset_groth16.json`.

Public layout: `fixtures/jubjub_g4b_public_layout_v1.json`.
