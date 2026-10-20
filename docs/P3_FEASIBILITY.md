# G0.5: Ristretto CP in BLS12-381 R1CS

## Decision

`BLOCKED_RESOURCE_GUARD_NO_GROTH16_PROOF`

The minimal feasibility experiment used a real native Chaum--Pedersen vector
and constrained the first equation

```text
z*G = A0 + d*VK
```

with non-native Curve25519-base-field arithmetic in a BLS12-381 R1CS. The
native proof verified and the real witness satisfied the circuit. The circuit
contained `3,501,941` constraints and `324` public inputs. Peak RSS was
`8,018.9 MiB` in the dev build.

The experiment's safety guard was `1,000,000` constraints, so Groth16 setup,
proof generation, and verification were deliberately not run. Blank timing
and proof-size cells in `cp_circuit_bench.csv` mean **not run**, not zero.

## Constraint breakdown

| Component | Constraints |
|---|---:|
| Inputs and canonical scalar range | 2,726 |
| Public-point curve membership | 9,548 |
| Non-native double-scalar multiplication | 3,484,879 |
| Ristretto quotient equality | 4,788 |
| Total | 3,501,941 |

## Claim boundary

This result establishes only that one real first CP equation can be expressed
and satisfied in the selected R1CS stack. It does not establish a Groth16 CP
proof, complete generalized CP verification, or P3.

The following required bindings remain unimplemented:

- compressed Ristretto decoding and canonical-encoding binding in-circuit;
- exact SHA-512 transcript hashing and rejection sampling in-circuit;
- `z*CT0 = A1 + d*D` and the generalized coordinate loop;
- threshold-package commitments, Lagrange reconstruction, bounded tally and
  winner selection.

The next necessary experiment is optimization of the non-native scalar
multiplication. P2/P3-a should not begin until the single-equation circuit can
complete setup/prove/verify within the declared resource budget.

## Reproduce the bounded preflight

```bash
cargo run -p vv-bench -- p3-feasibility \
  --manifest configs/protocol_manifest.json \
  --output experiments/raw \
  --seed 20261020 \
  --max-cp-constraints 1000000
```

Artifacts:

- `fixtures/cp_vectors.json`: real native compatibility vector and transcript;
- `experiments/raw/p3-feasibility.json`: machine-readable decision;
- `experiments/raw/cp_circuit_bench.csv`: one bounded preflight row.
