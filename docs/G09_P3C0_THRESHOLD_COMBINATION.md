# G0.9-P3-C0 fixed 1-2-3 threshold combination

Status: `PASS_P3C0_FIXED_123_THRESHOLD_COMBINATION_R1CS`.

P3-C0 extends P3-B1 with threshold reconstruction for the deterministically
selected Trustee set `{1,2,3}`. It stops at private plaintext group elements;
it does not decode or publish the tally.

## Constrained relation

For Trustee identifiers `(1,2,3)`, interpolation at zero gives:

```text
(lambda_1, lambda_2, lambda_3) = (3, -3, 1).
```

For each of the three candidate coordinates, the circuit reuses the
CP-bound share variables and constrains:

```text
D_j = 3*D_1,j - 3*D_2,j + D_3,j
M_j = C1_j - D_j
```

The three canonical aggregate `C1` points are public inputs. The three
plaintext group elements `M_j` are private witnesses and are not emitted.
The small fixed coefficients are implemented with point additions and
doublings rather than general non-native scalar multiplication.

## Real Release measurement

```bash
cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-dkg --bin friendly-p3c0-threshold-combination
```

| Measurement | Result |
|---|---:|
| Selected Trustees | 1, 2, 3 |
| Lagrange coefficients | 3, -3, 1 |
| CP equations | 12 |
| Package-H2F relations | 3 |
| Package opening equations | 3 |
| Threshold combination equations | 3 |
| Public aggregate `C1` points | 3 |
| Private plaintext group points | 3 |
| Constraints | 6,201,342 |
| Public inputs | 57 |
| Witness variables | 5,974,593 |
| Witness synthesis | 8,407.870 ms |
| Peak RSS | 5,607.562 MiB |

The native coefficients match `(3,-3,1)`, all three native CP proofs and
authenticated packages verify, and native threshold decryption matches the
fixed four-voter fixture. The R1CS is satisfied. Changing the third private
plaintext point is rejected; no additional negative-test matrix was run.

## Claim boundary and next gate

P3-C0 completes the fixed-subset threshold-combination gate and records
`full_p3c0=true`. It does not reveal a tally and does not prove that each
private group element encodes a bounded integer. Winner correctness and
Groth16 are also absent, so `full_p3=false`.

P3-C1 subsequently added three private integer tally witnesses, enforced their
bounds and total-weight relation, and constrained `M_j=t_j G`; see
`docs/G09_P3C1_BOUNDED_TALLY.md`. Winner comparison and Groth16 remain later
gates.

Artifacts:

- `experiments/g0_9_p3c0_raw/p3c0_threshold_combination_r1cs.json`; and
- `fixtures/jubjub_p3c0_public_layout_v1.json`.

The run records a dirty worktree because it measures the implementation before
commit. Final paper tables require a later clean-commit repetition.
