# G0.9-R clean Release baseline and suite lock

Status: `PASS_G09R_CLEAN_BASELINE_SUITE_FROZEN`.

G0.9-R reproduces the existing G0.8/H0/H1 path at Git commit
`39cca56d730e97d136f3f998df1ab81eda9d82f4` with a clean worktree. It adds no
new cryptographic relation. Its purpose is to give H2 and later gates one
fixed code, toolchain, manifest and suite baseline.

## Fixed environment

| Item | Value |
|---|---|
| Branch | `experiment/g08-jubjub-native` |
| Git dirty at each measured run | `false` |
| Host | Apple M4, 32 GiB, macOS 15.7.1 |
| Rust | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| Cargo | `cargo 1.99.1 (5f94df478 2026-08-27)` |
| Release profile | `opt-level=1`, `strip=none`, locked dependencies |
| `Cargo.lock` SHA-256 | `aed5c37a333eb2be6c5c37bfcae74d21147c3a19f3d2c0f84e0e92d97f7aa167` |
| Jubjub manifest SHA-256 | `ae9f187bc8b52cf67a8d8766532aa4e313e2f108be78da13e0eaf722d704b3cb` |

## Necessary verification only

- Release workspace tests: 50 passed, 0 failed, 1 filtered.
- The filtered test is
  `real_dalek_equation_satisfies_non_native_r1cs_and_mutation_fails`; it is the
  previously identified high-memory Ristretto non-native test.
- Release workspace Clippy with `-D warnings`: passed.
- Native E2E: registration `4/10`, packages `5/5`, selected Trustees 1--3,
  internal evaluation tally `(4,2,4)`, public winner `1`, no public tally.
- H0: 684 transcript bytes and 16,416 byte-binding constraints.
- H1: 890,920 constraints, 776.481 ms witness synthesis and 817.156 MiB peak
  RSS. The digest matches the native frozen vector.

The measurements are stored in
`experiments/g0_9_r_raw/g09_r_baseline.json`. The clean reruns also replace
the prior dirty-run metadata in the G0.8-E2, H0 and H1 raw artifacts.

## Suite lock for the next gates

G0.9-H2/H3/H4 use only the existing
`VERIVOTING-JUBJUB-PROTOTYPE-G0.8-v2` experiment suite:

- Jubjub from `ark-ed-on-bls12-381 0.4.0` for encryption, DKG and CP;
- Jubjub base field `Fq` as the BLS12-381 Groth16 constraint field;
- Jubjub subgroup scalar field `Fr` for native secrets and CP scalars;
- 32-byte canonical compressed points with subgroup validation;
- 32-byte canonical little-endian scalars accepted only when strictly below
  `Fr`;
- SHA-512 H2S-v2 with domain separation, a 252-bit mask and first-valid
  rejection sampling over counters 0 through 127.

The official Ristretto manifest remains a legacy comparison baseline. Its
group and scalar encodings must not be mixed into the G0.9 circuit path. This
suite lock is an experiment-version decision, not a production-readiness or
full-P3 claim.

## Downstream gate status

H2-A subsequently constrained the exact H2S-v2 wrapper, 252-bit mask, strict
`candidate < Fr` check and first-valid rule for real counter-0/1 vectors. H2-B
demonstrated one fixed two-attempt circuit shape. Neither implements the Suite
v2 rejection-128 bound. H2-C subsequently evaluated, but did not adopt, a
versioned single-hash wide-reduction candidate. H2-D must fix the final
manifest-pinned mapping before CP group equations or Groth16 proof generation
are added.
