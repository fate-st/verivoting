# G0.9-P3-D0 public winner linkage

Status: `PASS_P3D0_PUBLIC_WINNER_LINKAGE_R1CS`.

P3-D0 extends P3-C1 with one public winner identifier. It reuses the exact
private tally bits already bound to the threshold-decrypted plaintext points;
no second, disconnected tally witness is allocated.

## Constrained relation

For every possible public winner, the circuit enforces the
highest-weight, smallest-identifier-on-tie rule:

```text
earlier candidate IDs must have strictly smaller tallies than the winner
later candidate IDs may tie but must not have larger tallies
winner must be one of candidate IDs 1, 2, 3
```

The fixed private tally is `(4,2,4)`, so the only valid public winner is `1`.
The registered-weight bound `W=10` and winner `1` are public; tally integers
and plaintext group points remain private.

## Real Release measurement

```bash
cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-dkg --bin friendly-p3d0-winner-linkage
```

| Measurement | Result |
|---|---:|
| Selected Trustees | 1, 2, 3 |
| Lagrange coefficients | 3, -3, 1 |
| CP equations | 12 |
| Package-H2F / opening equations | 3 / 3 |
| Threshold combination equations | 3 |
| Tally group bindings / ranges / sum | 3 / 3 / 1 |
| Winner flags / pairwise relations | 3 / 6 |
| Public registered weight / winner | 10 / 1 |
| Constraints | 6,205,629 |
| Public inputs | 59 |
| Witness variables | 5,978,859 |
| Witness synthesis | 8,566.464 ms |
| Peak RSS | 5,607.875 MiB |

The valid relation satisfies the circuit. The single necessary negative case
changes only the public winner from `1` to `2`; the circuit rejects it because
candidate 1 ties candidate 2 and has the smaller identifier. No additional
negative-test matrix was run.

## Claim boundary and next gate

P3-D0 completes the intended CP, authenticated-package, threshold, bounded
tally and winner relation in R1CS and records `full_p3d0=true`. It does not run
Groth16, so it does not claim a complete P3 proof and records `full_p3=false`.

P3-D1 subsequently completed one real Groth16 setup/prove/verify run over this
exact relation, with one altered public-winner verification check; see
`docs/G09_P3D1_FULL_GROTH16.md`.

Artifacts:

- `experiments/g0_9_p3d0_raw/p3d0_winner_linkage_r1cs.json`; and
- `fixtures/jubjub_p3d0_public_layout_v1.json`.

The run records a dirty worktree because it measures the implementation before
commit. Final paper tables require a later clean-commit repetition.
