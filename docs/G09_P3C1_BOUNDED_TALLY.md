# G0.9-P3-C1 bounded private tally binding

Status: `PASS_P3C1_BOUNDED_TALLY_BINDING_R1CS`.

P3-C1 extends the fixed Trustee 1--3 P3-C0 relation with three private integer
tally witnesses. It directly proves the bounded discrete-log relation in the
circuit; it does not run BSGS inside the circuit and does not publish a tally.

## Constrained relation

For each candidate coordinate `j`, the circuit reuses the P3-C0 plaintext
point and enforces:

```text
M_j = t_j G
0 <= t_j <= W
sum_j(t_j) = W
```

Each `t_j` and each `M_j` is private. The registered-weight bound `W` is one
public field input and is bit-bound to the same 64-bit representation used by
the comparisons. The verifier must bind this public value to the accepted
registration close; P3-C1 does not independently verify that external state.

## Real Release measurement

```bash
cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-dkg --bin friendly-p3c1-tally-binding
```

| Measurement | Result |
|---|---:|
| Selected Trustees | 1, 2, 3 |
| Lagrange coefficients | 3, -3, 1 |
| CP equations | 12 |
| Package-H2F / opening equations | 3 / 3 |
| Threshold combination equations | 3 |
| Tally group bindings / ranges / sum | 3 / 3 / 1 |
| Public registered weight | 10 |
| Constraints | 6,203,703 |
| Public inputs | 58 |
| Witness variables | 5,976,943 |
| Witness synthesis | 8,806.231 ms |
| Peak RSS | 5,590.266 MiB |

The valid private tally is `(4,2,4)`. A single necessary negative case changes
it to `(5,1,4)`: all values remain within the bound and the sum remains 10, but
the existing plaintext group points no longer match, so the circuit rejects
the relation. No larger negative-test matrix was run.

## Claim boundary and next gate

P3-C1 completes the bounded private-tally gate and records `full_p3c1=true`.
It does not constrain the public winner and does not run Groth16, so
`full_p3=false`.

P3-D0 subsequently exposed only a public winner identifier and enforced the
highest-weight, smallest-identifier-on-tie rule against the same private tally
variables; see `docs/G09_P3D0_WINNER_LINKAGE.md`. The next necessary gate is a
single real Groth16 run over the completed relation.

Artifacts:

- `experiments/g0_9_p3c1_raw/p3c1_bounded_tally_binding_r1cs.json`; and
- `fixtures/jubjub_p3c1_public_layout_v1.json`.

The run records a dirty worktree because it measures the implementation before
commit. Final paper tables require a later clean-commit repetition.
