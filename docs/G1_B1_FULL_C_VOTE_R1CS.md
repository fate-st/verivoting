# G1-B1 complete fixed-suite C_vote R1CS

Status: `PASS_G1B1_FULL_C_VOTE_R1CS_FIXED_DEPTH2_FOUR_REGION`.

G1-B1 is the first experiment that composes all five ballot conditions from
the paper in one Jubjub-native R1CS. It deliberately fixes a small registration
shape so the relation can be validated before running Groth16.

## Fixed suite

- Address and voting nullifier: separately domain-separated HMAC-SHA-256 with
  a private 32-byte voter seed, public election identifier, final context and
  key-set hash.
- Region assignment: SHA-256 of the canonical final-context/address encoding;
  the low two digest bits select one of four regions.
- Registration: four leaves and Merkle depth two, using the same canonical
  SHA-256 entry and node encodings as the native registration close.
- Leaf identifier: the derived address; the private Jubjub weight commitment
  is the other leaf value.
- Ballot core: one-hot choice, `1 <= w <= 10`, commitment opening and three
  Weighted ElGamal coordinates under the public election key.

The public layout follows `(eid, ctx_e, k, RT_i, nf_i, keySetHash_e,
PK_enc,e, ct_i)`. The weight commitment, address, path, weight, opening,
choices and encryption randomness remain private.

## Real Release measurement

| Item | Result |
| --- | ---: |
| Five ballot conditions | 5 |
| Candidates / regions | 3 / 4 |
| Registration leaves / depth | 4 / 2 |
| HMAC PRF evaluations | 2 |
| Region SHA-256 evaluations | 1 |
| Registration SHA-256 evaluations | 3 |
| Weighted ElGamal equations | 6 |
| Constraints | 854,454 |
| Public inputs | 25 |
| Witness variables | 839,832 |
| Witness synthesis | 777.431 ms |
| Peak RSS | 754.859 MiB |

The valid witness satisfies the relation. The one necessary negative test
changes only the public final context while retaining the original address,
nullifier, region, root and private witness; the circuit rejects it.

## Claim boundary and next gate

This result establishes the complete ballot relation only for the fixed
four-leaf/four-region experiment suite. RA signature verification and equality
with the immutable election registry remain external checks. No Groth16 proof
or SnarkPack aggregation was run.

G1-B2 subsequently performs real Groth16 setup/prove/verify over this exact
relation and rejects one changed public input. See
`docs/G1_B2_FULL_C_VOTE_GROTH16.md`.

## Artifacts

- Raw result: `experiments/g1_b1_raw/full_c_vote_r1cs.json`
- Public layout: `fixtures/jubjub_ballot_cvote_b1_public_layout_v1.json`
- Circuit: `crates/vv-friendly-crypto/src/ballot_identity_gadget.rs`
- Runner: `crates/vv-friendly-dkg/src/bin/ballot_cvote_b1.rs`

The measurement was produced from the current local worktree. It is not a
claim that the changes are committed or uploaded.
