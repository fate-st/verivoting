# G0.8 Jubjub Suite v2, GJKR and threshold-decryption result

## Result

`G0.8-3 = PASS_NATIVE_DKG_THRESHOLD_NO_CP` on 2026-10-20.

The experiment used real Jubjub group operations in one process with `n=5`,
`t=2`, `tau=3`, and `q_cert=3`. Five Trustee authentication public keys were
fixed in `policy_pre` before DKG. Pedersen VSS/GJKR produced a non-identity
public key, five verification shares and a five-signer KeyCore certificate.

All ten legal 3-of-5 subsets independently interpolated the same public key and
combined partial decryptions for the four weighted ballots. Every subset
recovered private tally `(4,2,4)` and winner `1`. The implementation combines
`x_h*C0` shares with Lagrange coefficients and does not reconstruct the
aggregate secret key.

This is not full G0.8: generalized Chaum--Pedersen proofs and authenticated
private disclosure packages are not implemented. It is not P3 and not a
five-process deployment.

## Suite v2 prerequisite

The DKG run uses `VERIVOTING-JUBJUB-PROTOTYPE-G0.8-v2`. Relative to the v1
component baseline:

- hash-to-scalar clears the high four bits of the 32-byte little-endian
  candidate, then strictly rejects values at or above the Jubjub scalar order;
- the first accepted counter in `0..127` is binding and is tested through an
  actual rejection path;
- public keys and ciphertext points cross canonical encoding, curve and prime
  subgroup checks;
- key generation, encryption, DKG, signatures and nonces require `CryptoRng`;
- the single-key secret is no longer public through `Keypair`;
- BSGS uses checked arithmetic and rejects a table above 1,000,000 entries.

The v1 raw records and `fixtures/jubjub_suite_vectors.json` remain unchanged.
The v2 vectors are stored separately in
`fixtures/jubjub_suite_vectors.v2.json`.

## Release execution

Environment: Apple M4, 32 GiB RAM, macOS arm64, Rust 1.99.1, Cargo Release
profile with repository `opt-level=1` and `CARGO_PROFILE_RELEASE_STRIP=none`.

| Measurement | Result |
|---|---:|
| Qualified dealers | 5/5 |
| Certificate signers | 5 |
| Complaints in normal run | 0 |
| Estimated DKG protocol bytes | 2,080 |
| DKG | 11.587 ms |
| Four-ballot encryption and fold | 5.503 ms |
| All ten threshold decryptions | 15.847 ms |
| Legal subsets passing | 10/10 |
| Private tally | `(4,2,4)` |
| Public winner | `1` |

These timings are one deterministic functional run, not publication-level
statistics. The raw file records HEAD `a80f7223a2dd5376155fff60ba507dc49e3d4f02`
and `git_dirty=true` because the G0.8-3 implementation had not yet been
committed when executed. It does not mislabel the run as a clean build.

## Verification

The necessary Release checks passed:

- `cargo fmt --all -- --check`;
- strict workspace Release Clippy with `-D warnings`;
- 35 workspace Release tests passed, with the existing resource-heavy
  Ristretto test retained but filtered;
- Suite v2 tests for canonical encoding, rejection sampling, subgroup
  boundaries, CSPRNG APIs, ElGamal, BSGS and the four-voter fixture;
- DKG tests for all ten subsets, complaints and correction, Feldman extraction,
  dealer disqualification, certificate quorum, frozen context and invalid
  trustee sets.

## Raw evidence and claim boundary

- `experiments/g0_8_suite_v2_raw/friendly-elgamal-four-voter.json`;
- `fixtures/jubjub_suite_vectors.v2.json`;
- `experiments/g0_8_dkg_raw/g08_dkg_threshold.json`.
- `experiments/g0_8_dkg_raw/g08_validation.json`.

The official `configs/protocol_manifest.json` is unchanged. Only the isolated
Jubjub prototype manifest moved to v2. `full_g08=false` and `full_p3=false`
remain mandatory until generalized CP and the private-package workflow pass.

## Next required stage

The follow-on native generalized Chaum--Pedersen stage is now complete; see
`docs/G08_GENERALIZED_CP.md`. The remaining G0.8 gate is the authenticated
private-package and winner-only event workflow. G0.9, SnarkPack, state proof and
contract work remain out of scope until that gate passes.
