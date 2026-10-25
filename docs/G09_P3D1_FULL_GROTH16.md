# G0.9-P3-D1 fixed-123 complete P3 Groth16

Status: `PASS_P3D1_REAL_GROTH16_FIXED_123_COMPLETE_P3`.

P3-D1 runs real BLS12-381 Groth16 setup, proving and verification over the
exact P3-D0 relation. The relation covers three selected Trustee CP proofs,
three authenticated private-package commitments and openings, fixed
Trustee-1-2-3 threshold reconstruction, bounded private tallies and the public
winner rule.

## Public and private boundary

The 59 public inputs contain the three CP statement prefixes, three package
commitments, three aggregate `C1` points, registered weight `W=10` and winner
`1`. The CP suffixes, package openings, plaintext group points and tally
integers remain private.

The verifier must bind the public statement prefixes, package commitments,
aggregate ciphertexts and registered weight to the accepted election state.
This external state-binding requirement is not replaced by Groth16.

## Real Release measurement

```bash
cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-dkg --bin friendly-p3d1-full-groth16
```

| Measurement | Result |
|---|---:|
| Constraints | 6,205,629 |
| Public inputs | 59 |
| Witness variables | 5,978,859 |
| Witness synthesis | 8,633.805 ms |
| Setup | 64,668.553 ms |
| Prove | 43,604.668 ms |
| Verify | 6.019 ms |
| Wrong-winner verify | 5.870 ms |
| Proof | 192 bytes |
| Compressed proving key | 1,837,594,176 bytes |
| Compressed verifying key | 3,224 bytes |
| Peak RSS | 7,779.359 MiB |

The proof verifies for public winner `1`. Reusing the same proof with only the
public winner changed to `2` fails verification. This is the single necessary
negative check; no broader negative matrix or repeated benchmark campaign was
run. The proving key was measured in memory and was not persisted.

## Claim boundary and next step

This completes a real fixed-123 P3 proof for the four-voter experiment and
records `full_p3=true`. It is not a production trusted-setup ceremony and does
not provide one universal circuit/setup for every possible legal Trustee
subset.

The ballot-validity Groth16/SnarkPack path was completed in the later G1 gates.
After G2 state proofs and G3 process separation, the next P3 extension is G4:
first compare a second legal Trustee subset in R1CS, then publicly bind the
selected subset before attempting another real Groth16 proof. Repeating the
fixed-123 benchmark alone is not necessary.

Artifacts:

- `experiments/g0_9_p3d1_raw/p3d1_full_groth16.json`; and
- `fixtures/jubjub_p3d1_public_layout_v1.json`.

The run records a dirty worktree because it measures the implementation before
commit. Final paper tables require a later clean-commit repetition.
