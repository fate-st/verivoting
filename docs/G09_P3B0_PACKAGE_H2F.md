# G0.9-P3-B0 private-package H2F adoption

Status: `PASS_P3B0_PACKAGE_H2F_V1_ADOPTED`.

P3-B0 makes the private-package message mapping suitable for a fixed-shape
P3 circuit. It is a protocol migration and feasibility measurement, not the
P3-B commitment-opening relation itself.

## Why this gate is necessary

The earlier native package commitment used the Suite-v2 rejection-sampling
`hash_to_scalar` routine with as many as 128 attempts. Treating its result as
an unconstrained P3 witness would not bind the commitment to the private
package, while evaluating all 128 attempts in R1CS would be impractical given
the measured SHA-512 cost. P3-B0 therefore assigns package messages their own
versioned, fixed-shape hash-to-field mapping.

## Adopted mapping

```text
input = CanonicalEncode(
    "VV/JUBJUB/H2F/v3",
    "VERIVOTING-JUBJUB-PACKAGE-H2F-G0.9-v1",
    "PRIVATE-PACKAGE",
    Encode(package_h)
)
digest = SHA-512(input)
m_h = OS2IP_BE(digest) mod Jubjub-Fr
Com_h = m_h*G + rho_h*H
```

The registration-bound package and commitment domains advance to v5. The
winner-only public event advances to v4. Existing signatures, CP proofs and
Ristretto artifacts are not silently reinterpreted.

## Real Release measurement

The necessary runner was:

```bash
cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-dkg --bin friendly-p3b0-package-h2f
```

It created the real registered-context packages for selected Trustees 1, 2
and 3 and checked the same mapping natively and in R1CS.

| Measurement | Result |
|---|---:|
| Private package bytes per Trustee | 602 |
| Canonical H2F input bytes | 725 |
| SHA-512 blocks | 6 |
| Constraints per package | 1,000,827 |
| Public inputs per package | 2 |
| Witness variables per package | 967,157 |
| Total witness synthesis for three packages | 3,030.297 ms |
| Peak RSS | 1,059.766 MiB |

All three instances have the same circuit shape and match the native scalar.
A changed private package is rejected. The public vector contains hashes,
scalars and commitments, but does not disclose the private package bytes.

## Necessary native regression

After migration, the registration-bound native E2E still creates and verifies
5/5 packages, selects Trustees 1--3 and emits public winner `1`. The private
tally remains absent from the winner event. The five private payloads total
3,010 bytes, or 602 bytes per Trustee. The targeted package tests also pass.

## Claim boundary and next gate

P3-B0 only adopts and measures the fixed-shape package-message mapping. It
does not yet place `Com_h = m_h G + rho_h H` inside the combined P3 circuit,
does not run a P3-B Groth16 proof, and does not implement Lagrange
interpolation, secret tally binding or winner correctness. Therefore
`full_p3b=false` and `full_p3=false`.

P3-B1 subsequently derives each selected package encoding from the existing
P3-A witnesses, computes Package-H2F-v1 in circuit, exposes the three
commitments as public inputs, keeps the openings private and constrains the
three Pedersen opening equations. Its first R1CS run is complete; see
`docs/G09_P3B1_PACKAGE_OPENINGS.md`. Interpolation, tally, winner logic and
Groth16 remain later gates.

Artifacts:

- `experiments/g0_9_p3b0_raw/package_h2f_adoption.json`;
- `experiments/g0_9_p3b0_raw/e2e-public.json`; and
- `fixtures/jubjub_package_h2f_v1.json`.

These runs record a dirty worktree because they measure the new implementation
before commit. Final paper tables require a later clean-commit repetition.
