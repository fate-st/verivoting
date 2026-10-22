# G0.8 native generalized Chaum--Pedersen result

## Result

`G0.8-4 = PASS_NATIVE_GENERALIZED_CP_NO_PACKAGES` on 2026-10-20.

Five Trustees each generated one native Jubjub generalized
Chaum--Pedersen proof covering the three aggregate ciphertext coordinates. The
native verifier recomputed the Suite v2 Fiat--Shamir challenge from the exact
canonical transcript. All five proofs verified, after which all ten legal
3-of-5 subsets used only verified decryption shares and recovered `(4,2,4)`
with winner `1`.

The aggregate secret key was not reconstructed. This result is native protocol
verification, not an R1CS/Groth16 CP proof and not full P3.

## Bound statement

The target commits to the full ordered `(C0,C1)` aggregate ciphertext vector.
Each Trustee transcript binds:

```text
suite_id, election_id, policy_pre, final_context, target,
trustee_id, verification_share, ordered C0 vector,
ordered decryption-share vector, A0, ordered Aj vector
```

For every Trustee `h` and coordinate `j`, verification checks:

```text
z*G = A0 + d*VK_h
z*C0[j] = A[j] + d*D[h,j]
```

The rejection counter is part of the proof and must equal the counter obtained
by the verifier's first-valid Suite v2 challenge computation. In the recorded
run, Trustee 5 used counter `1`, so the real experiment exercised a rejection
before accepting the next canonical scalar.

## Release measurement

Environment: Apple M4, 32 GiB RAM, macOS arm64, Rust 1.99.1, Cargo Release
profile with repository `opt-level=1` and `CARGO_PROFILE_RELEASE_STRIP=none`.

| Measurement | Result |
|---|---:|
| DKG | 11.910 ms |
| Four-ballot encryption and fold | 6.027 ms |
| Generate all five CP proofs | 6.569 ms |
| Verify all five CP proofs | 10.772 ms |
| Ten verified-share threshold decryptions | 17.207 ms |
| CP proofs accepted | 5/5 |
| Legal threshold subsets accepted | 10/10 |
| Proof size per Trustee, C=3 | 162 bytes |
| Share vector per Trustee, C=3 | 96 bytes |
| Private tally | `(4,2,4)` |
| Public winner | `1` |

This is one deterministic functional run, not a publication-level timing
sample. The raw record truthfully reports HEAD `a80f722` and `git_dirty=true`
because the new implementation was not committed when executed.

## Necessary negative tests

Release tests reject:

- altered decryption share;
- altered coordinate commitment;
- altered response or rejection counter;
- mismatched prover secret and verification share;
- stale final context or wrong Trustee identifier;
- reordered ciphertext coordinates, even if a new target is supplied;
- shortened share/proof vectors;
- unverified point vectors and valid packages from another election at the
  verified threshold-combination boundary.

The complete workspace result is 40 passed, 0 failed, with the existing
resource-heavy Ristretto test retained but filtered. Strict Release Clippy also
passes with `-D warnings`.

## Evidence and boundary

- `crates/vv-friendly-crypto/src/cp.rs`;
- `crates/vv-friendly-dkg/src/bin/generalized_cp.rs`;
- `experiments/g0_8_cp_raw/g08_generalized_cp.json`;
- `experiments/g0_8_cp_raw/g08_validation.json`.

The official Ristretto manifest is unchanged. The isolated Jubjub prototype
manifest now states `PARTIAL_GENERALIZED_CP_NO_PRIVATE_PACKAGES`.
`full_g08=false` and `full_p3=false` remain correct.

## Next required stage

Implement the minimal authenticated private-package workflow and final native
winner-only event:

1. commit to each verified `(D_h,CP_h)` package;
2. sign the commitment under the pre-policy Trustee identity;
3. choose at least three packages that are present and CP-verified;
4. open only those packages to the private combiner;
5. combine, decode and publish only the winner plus public commitments and
   signatures.

No circuit, SnarkPack, state-proof, multiprocess or contract experiment is
required before this final native G0.8 gate.
