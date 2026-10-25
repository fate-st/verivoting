# G0.9-H2-D CP-H2F-v3 adoption

Status: `PASS_H2D_CP_H2F_V3_ADOPTED`.

H2-D adopts the wide SHA-512-to-field construction for the Jubjub prototype's
generalized Chaum--Pedersen challenge. The final suite identifier is
`VERIVOTING-JUBJUB-CP-H2F-G0.9-v3`; the prototype manifest version is
`VERIVOTING-JUBJUB-PROTOTYPE-G0.9-H2D-0.5`.

The official Ristretto manifest is unchanged. Schnorr signatures, private
package commitments and other non-CP hash users retain their existing
domain-separated H2S-v2 mappings.

That statement records the H2-D boundary. P3-B0 subsequently migrates only
private-package commitment messages to the independent fixed-shape
`VERIVOTING-JUBJUB-PACKAGE-H2F-G0.9-v1` mapping; see
`docs/G09_P3B0_PACKAGE_H2F.md`. Schnorr signatures remain unchanged.

## Adopted mapping

```text
input = CanonicalEncode(
    "VV/JUBJUB/H2F/v3",
    "VERIVOTING-JUBJUB-CP-H2F-G0.9-v3",
    operation_domain,
    transcript
)
digest = SHA-512(input)
d = OS2IP_BE(digest) mod Jubjub-Fr
```

The generalized CP proof no longer serializes a rejection counter. Registered
private-package domains advance to `PRIVATE-PACKAGE/v4` and
`PRIVATE-PACKAGE/COMMIT/v4`.

## Fixed-shape R1CS result

| Measurement | Result |
|---|---:|
| CP transcript bytes | 684 |
| Canonical H2F input bytes | 801 |
| SHA-512 blocks | 7 |
| Constraints | 1,151,661 |
| Public inputs | 2 |
| Witness variables | 1,116,817 |
| Witness synthesis | 1,219.986 ms |
| Peak RSS | 1,196.922 MiB |

The native scalar matches the frozen vector, the R1CS output matches the
native scalar, the prototype manifest pins the same suite, and a changed
transcript is rejected. The final frozen scalar is:

```text
356d07f1fc3927a7c0ae3e598c61c58126e30a68575721d721938a721e622c09
```

## Necessary native regressions

- Native generalized CP: 5/5 Trustee proofs verified.
- All ten legal 3-of-5 subsets recovered `(4,2,4)` and winner `1`.
- Five-proof generation: 6.076 ms.
- Five-proof verification: 10.219 ms.
- Registration-bound E2E: 5/5 packages verified, Trustees 1--3 selected,
  winner `1`, no public tally or decryption shares.
- Five private package payloads: 3,010 bytes, or 602 bytes each.
- Targeted tests: six CP tests and three private-package tests passed.

Raw results:

- `experiments/g0_9_h2d_raw/h2d_adoption.json`;
- `experiments/g0_9_h2d_raw/h2d_generalized_cp.json`;
- `experiments/g0_9_h2d_raw/h2d_e2e-public.json`; and
- `experiments/g0_9_h2d_raw/h2d_transcript_binding.json`.

These runs record `git_dirty=true` because they measure the new implementation
before commit. They must be repeated from a clean commit before final paper
tables are generated.

## Claim boundary and next gate

At the H2-D boundary, the standalone hash-to-field gate is complete for the
prototype, so `full_h2=true`; CP group equations and Groth16 were not part of
that result, and `full_p3=false`.

The paper still describes rejection sampling and must be updated to the
manifest-pinned wide-reduction suite before submission. H3 has subsequently
bound this output to one Trustee's first CP group equation; see
`docs/G09_H3_FIRST_CP_EQUATION.md`. The next necessary gate is the remaining
coordinate equations for that same Trustee, not three-Trustee P3 expansion.
