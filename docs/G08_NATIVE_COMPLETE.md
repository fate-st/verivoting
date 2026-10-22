# G0.8-E2 complete native Jubjub path

Status: `PASS_G08_NATIVE_SINGLE_PROCESS`.

This gate completes the planned native Jubjub voting path. “Complete native”
does not mean full P3, ballot Groth16, SnarkPack, state proof, multiprocess
deployment or a production voting system.

## Implemented sequence

1. Generate the RA key and freeze its public key, `W_max=10`, registration
   version and winner rule in a registration-policy digest.
2. Include that digest with the Trustee authentication roster in `policy_pre`
   before starting the one-process 5-Trustee GJKR experiment.
3. Commit the four authorized weights `(3,2,1,4)`, enforce unique credential
   identifiers and verify every Pedersen opening.
4. Canonically close count 4 and exact registered weight 10, sign the closing
   record, and verify its election, final context, policy and signature.
5. Bind both close-record and disclosure digests into target v3.
6. Encrypt and fold the four weighted votes; generate generalized CP proofs and
   authenticated private packages for all five Trustees.
7. Verify 5/5 packages, deterministically select Trustees 1--3, combine only
   verified share types, internally check `(4,2,4)`, and publish only winner 1.

## Release result

Command:

```bash
cargo +1.99.1 run --release --locked -p vv-friendly-dkg \
  --bin friendly-native-e2e
```

| Measurement | Result |
|---|---:|
| DKG | 13.850 ms |
| Registration close and verification | 1.888 ms |
| Four-voter encryption and folding | 5.871 ms |
| Five CP proofs plus private packages | 9.011 ms |
| Verify five packages and select threshold | 14.028 ms |
| Verified threshold decryption | 1.486 ms |
| Registered count / weight | 4 / 10 |
| Packages created / verified / selected | 5 / 5 / 3 |
| Complete private-package encoding | 3,020 bytes total |
| Published winner | 1 |

Raw public artifact:
`experiments/g0_8_complete_raw/e2e-public.json`.

The public artifact contains the registration certificate, context digests,
three selected package commitments/signatures and winner. It explicitly omits
the tally, decryption shares, CP proofs and commitment openings.

## Necessary validation only

- Exact close succeeds; duplicate credentials and a bad opening fail.
- Altered close data and election/final-context replay fail.
- Substitution of the registration-close digest invalidates the CP target.
- Strict Release Clippy passes.
- Release workspace regression passes 47 tests; the separately retained
  high-memory Ristretto circuit test is the only filtered test.

No additional denial-of-service, long-duration, multiprocess or scale campaign
was run in this gate.

## Claim boundary and next gate

`full_native_g08=true`, `ballot_groth16_implemented=false`, and
`full_p3=false`. The next single gate is G0.9-H0/H1: canonical CP transcript
input equality followed by one real in-circuit hash feasibility measurement.
