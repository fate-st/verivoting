# G1-B0 weighted-ballot cryptographic core

Status: `PASS_G1B0_WEIGHTED_BALLOT_CORE_R1CS`.

G1-B0 freezes and measures the first independently testable part of the
paper's single-ballot relation. It is intentionally narrower than the complete
five-condition `C_vote`.

## Constrained relation

For three candidates, public inputs contain the certified election key `PK`,
the registered-weight commitment `cm`, public bound `W`, and three ciphertext
pairs `(C0_j,C1_j)`. Private witnesses contain weight `w`, its commitment
opening, one-hot bits `v_j`, and encryption randomness `r_j`. The circuit
enforces

```text
1 <= w <= W
sum_j v_j = 1
cm = w G + opening H_registration
C0_j = r_j G
C1_j = r_j PK + v_j w G
```

All public group elements use canonical compressed Jubjub binding.

## Real Release measurement

| Item | Result |
| --- | ---: |
| Candidates | 3 |
| One-hot equations | 1 |
| Weight-range gate | 1 |
| Commitment-opening equation | 1 |
| Weighted ElGamal equations | 6 |
| Constraints | 46,573 |
| Public inputs | 17 |
| Witness variables | 39,940 |
| Witness synthesis | 108.856 ms |
| Peak RSS | 49.156 MiB |

The valid relation is satisfied. The single necessary negative test replaces
the one-hot choice with a two-hot choice and is rejected.

## Claim boundary

This result is not the paper's complete `C_vote`. It does not yet constrain:

- the election/final context;
- the credential-derived PRF address and deterministic region;
- the voting nullifier;
- registration-root membership.

No ballot Groth16 proof or SnarkPack aggregation was run in G1-B0. G1-B1
subsequently freezes the exact PRF/assignment/nullifier/Merkle suite and adds
those relations. See `docs/G1_B1_FULL_C_VOTE_R1CS.md`.

## Artifacts

- Raw result: `experiments/g1_b0_raw/ballot_core_r1cs.json`
- Public layout: `fixtures/jubjub_ballot_core_b0_public_layout_v1.json`
- Circuit: `crates/vv-friendly-crypto/src/ballot_gadget.rs`
- Runner: `crates/vv-friendly-crypto/src/bin/ballot_core_b0.rs`

The measurement was produced from the current local worktree. It is not a
claim that the changes are committed or uploaded.
