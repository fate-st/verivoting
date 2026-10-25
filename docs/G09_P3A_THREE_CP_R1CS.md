# G0.9-P3-A three selected Trustee CP relations

Status: `PASS_P3A_THREE_SELECTED_TRUSTEE_CP_R1CS`.

P3-A composes complete generalized Chaum--Pedersen relations for selected
Trustees 1, 2 and 3 in one R1CS. The fixture is generated from a real
single-process GJKR run, verified registration close and the four-voter
aggregate ciphertext. All three native CP proofs verify before circuit
synthesis.

## Frozen public/private input boundary

Layout `VV/JUBJUB/P3-A/INPUT-LAYOUT/v1` divides each 684-byte CP transcript at
byte 452:

- the first 452 bytes are public and contain the canonical suite/domain,
  election and context digests, frozen target, selected Trustee ID,
  verification share, coordinate count and aggregate ciphertext `C0` vector;
- the remaining 232 transcript bytes are private witness values containing the
  decryption-share vector, `A0` and the three coordinate commitments;
- the 32-byte CP response scalar is also a private witness.

The published layout fixture contains public prefixes and full-transcript
hashes only. It does not emit decryption shares, CP commitments or responses.

## Real Release measurement

The command was:

```bash
cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-dkg --bin friendly-p3a-three-cp
```

The result is:

- selected Trustees: `[1,2,3]`, all distinct;
- native CP proofs verified: `3/3`;
- complete CP equations in R1CS: `12`;
- constraints: `3,611,598`;
- packed public inputs: `45`;
- witness variables: `3,485,994`;
- witness synthesis: `4,876.174 ms`;
- peak RSS: `3,258.906 MiB`.

The valid relation satisfies the circuit. Altering the third Trustee's first
decryption share and its matching private transcript encoding makes the
relation unsatisfied. No larger negative-test matrix was run.

## Claim boundary and next step

This is the three-CP P3-A R1CS relation, not full P3. It does not verify private
package commitment openings, perform Lagrange threshold combination, bind
secret tallies to plaintext points, or prove winner correctness.

The measured resource level was below the experiment machine's 24 GiB process
budget. A subsequent real Groth16 run for this exact relation succeeds and
rejects a changed public prefix. See `docs/G09_P3A_GROTH16.md`. Commitment
openings remain a later P3-B gate.

Artifacts:

- `experiments/g0_9_p3a_raw/p3a_three_cp_r1cs.json`;
- `fixtures/jubjub_p3a_public_layout_v1.json`.
