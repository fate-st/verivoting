# G0.9-P3-B1 three private-package opening relations

Status: `PASS_P3B1_THREE_PACKAGE_OPENINGS_R1CS`.

P3-B1 extends the exact P3-A relation with the three selected private-package
commitments. It reuses the same CP witnesses rather than accepting independent
package bytes.

## Constrained relation

For each selected Trustee, the circuit:

1. verifies the complete four-equation CP-H2F-v3 relation from P3-A;
2. constructs the 602-byte canonical private package from that CP witness;
3. computes Package-H2F-v1 over the constructed package;
4. receives the canonical compressed package commitment as a public input;
5. keeps the Pedersen opening private; and
6. enforces `Com_h = m_h G + rho_h H`.

The three commitment signatures are verified by the native package verifier.
Signature verification is not redundantly embedded in this R1CS.

## Real Release measurement

The only measurement command was:

```bash
cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-dkg --bin friendly-p3b1-package-openings
```

| Measurement | Result |
|---|---:|
| Selected Trustees | 1, 2, 3 |
| Native CP proofs verified | 3 |
| Native authenticated packages verified | 3 |
| CP equations in R1CS | 12 |
| Package-H2F relations in R1CS | 3 |
| Pedersen opening equations in R1CS | 3 |
| Package bytes per Trustee | 602 |
| Package H2F input bytes | 725 |
| SHA-512 blocks per package | 6 |
| Constraints | 6,191,178 |
| Public inputs | 51 |
| Witness variables | 5,966,673 |
| Witness synthesis | 9,234.655 ms |
| Peak RSS | 5,661.500 MiB |

The valid relation is satisfied. Changing the third public package commitment
without changing the corresponding private package and opening is rejected.
No negative-test matrix was run.

The public fixture contains CP-prefix hashes, package hashes, commitments and
their external signatures. It contains neither private package bytes nor
commitment openings.

## Claim boundary and next gate

P3-B1 completes the package-opening R1CS gate and therefore records
`full_p3b=true`. It does not run Groth16 and is not full P3. Lagrange
interpolation, threshold reconstruction, secret tally binding and winner
correctness remain absent, so `full_p3=false`.

P3-C0 subsequently constrains the fixed 3-of-5 Lagrange combination of the
three already-bound decryption-share vectors and stops at private plaintext
group elements; see `docs/G09_P3C0_THRESHOLD_COMBINATION.md`. Bounded tally
binding was then completed in P3-C1; see
`docs/G09_P3C1_BOUNDED_TALLY.md`. Winner logic and Groth16 remain later gates.

Artifacts:

- `experiments/g0_9_p3b1_raw/p3b1_three_package_openings_r1cs.json`; and
- `fixtures/jubjub_p3b1_public_layout_v1.json`.

The run records a dirty worktree because it measures the implementation before
commit. Final paper tables require a later clean-commit repetition.
