# G0.8-E1 authenticated private-package experiment

Status: `PASS_NATIVE_PRIVATE_PACKAGES_NO_REGISTRATION_CLOSE`.

This gate is the minimal native experiment after generalized CP. It does not
claim full G0.8 or P3.

## Executed flow

1. Generate five Jubjub authentication keys before deriving `policy_pre`.
2. Run the single-process 5-Trustee, threshold-3 Pedersen VSS/GJKR flow.
3. Encrypt and fold the fixed four-voter fixture.
4. Generate one generalized CP decryption-share package per Trustee.
5. Canonically commit to each complete private package with a fresh Pedersen
   opening and sign the public commitment under a package-specific domain.
6. Verify election/context binding, signature, opening and CP for all five
   packages.
7. Deterministically select the first three distinct verified packages.
8. Feed only their `VerifiedDecryptionShare` values to threshold decryption,
   check the private tally internally, and publish only winner `1` with the
   selected public commitments and signatures.

## Release result

Command:

```bash
cargo run --release --locked -p vv-friendly-dkg \
  --bin friendly-private-packages
```

The run on Apple M4 with Rust 1.99.1 produced:

| Measurement | Result |
|---|---:|
| DKG | 12.479 ms |
| Four-voter encryption and folding | 5.685 ms |
| Five CP proofs plus private commitments/signatures | 9.666 ms |
| Verify five packages and select threshold | 15.691 ms |
| Verified threshold decryption | 1.567 ms |
| Packages created / verified / selected | 5 / 5 / 3 |
| Complete private-package encoding | 2,700 bytes total |
| Published winner | 1 |

Raw public artifact:
`experiments/g0_8_packages_raw/e2e-public.json`.

The public artifact has explicit false markers for private tally, decryption
shares, CP proofs and commitment openings. It records only public contexts,
three selected commitments/signatures and the winner.

## Necessary negative checks

The focused package tests cover changed opening, changed signature, changed
election context, insufficient verified packages and duplicate Trustee IDs.
The generalized CP module already covers incorrect proof equations, challenge,
response and target binding, so those tests are not duplicated here.

## Claim boundary and next step

`registration_close_implemented=false`, `full_g08=false` and `full_p3=false`.
The registered-weight bound is still a fixture constant in the Jubjub path.
The next step is therefore G0.8-E2: migrate only the registration commitment
and authenticated close certificate needed to bind that bound to this same
election and pre-policy. Multi-process transport and P3 circuits are later
gates, not prerequisites for E2.
