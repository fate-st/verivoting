# G1-C1 distinct registered-voter SnarkPack

## Result

G1-C1 passes the smallest distinct-voter aggregation gate. Two registered
voters under the same certified four-leaf registration root produce complete
fixed-suite `C_vote` Groth16 proofs. Voter 1 selects candidate 1 and voter 2
selects candidate 2; both are assigned to region 4. The two public statements
contain different voting nullifiers and different Weighted ElGamal ciphertext
vectors.

Both converted source proofs verify individually and their real bellperson
0.27.0 SnarkPack V2 aggregate verifies. Replacing only the second statement's
public voting nullifier causes aggregate verification to fail.

## Measured Release run

- Circuit per ballot: 854,454 constraints and 25 public inputs.
- Groth16 setup: 7,292.883 ms.
- First proof: 4,306.160 ms; second proof: 4,443.555 ms.
- Arkworks-to-bellperson conversion: 4.301 ms.
- Functional aggregation SRS generation: 4.368 ms.
- Aggregation: 3.908 ms; aggregate verification: 2.955 ms.
- Changed-second-nullifier verification: 2.671 ms and rejected.
- Two individual proofs: 384 bytes; aggregate proof: 5,236 bytes.
- Peak RSS: 2,516.266 MiB.

## Claim boundary

This result demonstrates the minimum functional multi-voter batch. It does not
support a scale, compression or production-trust claim: the batch contains two
proofs and uses bellperson's deterministic `setup_fake_srs` helper. Large-batch
measurements are deferred until the regional state relation is implemented.

## Reproduce

```bash
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-ballot-cvote-b1 -- \
  --output experiments/g1_c1_raw/source_full_c_vote_r1cs.json \
  --layout-output fixtures/jubjub_ballot_cvote_c1_public_layout_v1.json \
  --snarkpack-distinct-output experiments/g1_c1_raw/snarkpack_distinct_voters_batch_2.json
```

Raw result: `experiments/g1_c1_raw/snarkpack_distinct_voters_batch_2.json`.

G2-A and G2-B consume the exact two public records embedded in this result. See
`docs/G2_A_STATE_TRANSITION_R1CS.md` and
`docs/G2_B_STATE_TRANSITION_GROTH16.md`.
