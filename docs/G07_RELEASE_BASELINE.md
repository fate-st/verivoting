# G0.7 Release baseline

## Result

G0.7 passed on the frozen commit
`0eec138e0415593ddba31b0601f6428934632c56` on branch
`experiment/g07-g08-friendly-migration`.

The workspace built in the Cargo `release` profile, formatting and strict
Release Clippy passed, 22 required tests passed with no failures, five real P1
Groth16 trials verified, and the four-voter Release E2E returned public winner
`1` after checking all ten legal 3-of-5 trustee subsets.

This round did not begin G0.8, full P3, SnarkPack, state proofs, multiprocess
GJKR, contract work, scale tests, or additional fault campaigns.

## Frozen environment

| Item | Value |
|---|---|
| Date | 2026-10-20 (Asia/Shanghai) |
| Git commit | `0eec138e0415593ddba31b0601f6428934632c56` |
| Branch | `experiment/g07-g08-friendly-migration` |
| OS | macOS 15.7.1 (`24G231`), Darwin 24.6.0, arm64 |
| CPU | Apple M4, 10 logical CPUs |
| Memory | 32 GiB (`34,359,738,368` bytes) |
| Rust | `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, LLVM 23.1.1 |
| Cargo | `cargo 1.99.1 (5f94df478 2026-08-27)` |
| Cargo profile | `release`, repository `opt-level = 1`, build jobs = 2 |
| Manifest canonical digest | `c1b74c703e1f52d3ee8a1ea94c46433cc7991fccf0d64d503209e737dc5f29dd` |
| P1 base seed | `20261020` |

The unqualified `cargo -V` command selected a broken/default stable shim on
this machine. All measured commands therefore selected the installed
`1.99.1-aarch64-apple-darwin` toolchain explicitly.

## Release build issue and resolution

The first Release build did not fail in Rust compilation. It became stuck in
`rust-objcopy --strip-debug` with 0% CPU while Cargo waited for the
`proc-macro2` and `quote` build-script binaries. The system also contained
older orphaned `rust-objcopy` processes in the same state.

The successful run used `CARGO_PROFILE_RELEASE_STRIP=none`. This disables only
the defective post-link debug-symbol stripping step; it retains the Release
profile and its optimized code generation. The clean Release workspace build
then completed in `32.32 s`.

```text
CARGO_BUILD_JOBS=2 \
CARGO_PROFILE_RELEASE_STRIP=none \
RUSTUP_TOOLCHAIN=1.99.1-aarch64-apple-darwin \
cargo build --workspace --release --locked
```

## Required checks

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| Release Clippy, all targets, `-D warnings` | PASS (`8.41 s`) |
| Release workspace tests | PASS: 22 passed, 0 failed |
| Resource-heavy Ristretto test | 1 filtered as required; retained in source |
| P1 Groth16, five trials | PASS: 5/5 verified |
| Changed public winner | Rejected in 5/5 trials |
| Four-voter Release E2E | PASS |
| Internal private tally | `(4,2,4)` |
| Public winner | `1` |
| Legal threshold subsets | all 10 combinations of 3-of-5 |

The E2E runner checks the private tally against `(4,2,4)` before emitting the
winner-only public artifact. The artifact intentionally has
`public_tally_present=false` and `full_p3_generated=false`.

## Five-trial P1 measurements

Every trial used 3,521 constraints, 2 public inputs, a 192-byte proof, a
1,039,440-byte proving key, and a 488-byte verifying key.

| Trial | Seed | Setup ms | Prove ms | Verify ms | Verified | Wrong winner rejected |
|---:|---:|---:|---:|---:|---|---|
| 0 | 20261020 | 61.146 | 36.100 | 0.940 | yes | yes |
| 1 | 20261021 | 44.378 | 23.891 | 0.909 | yes | yes |
| 2 | 20261022 | 46.639 | 23.871 | 0.906 | yes | yes |
| 3 | 20261023 | 47.951 | 24.552 | 0.910 | yes | yes |
| 4 | 20261024 | 46.676 | 24.330 | 0.927 | yes | yes |

| Metric | Setup ms | Prove ms | Verify ms |
|---|---:|---:|---:|
| Mean | 49.358 | 26.549 | 0.918 |
| Median | 46.676 | 24.330 | 0.910 |
| Sample standard deviation | 6.714 | 5.347 | 0.015 |
| Minimum | 44.378 | 23.871 | 0.906 |
| Maximum | 61.146 | 36.100 | 0.940 |

Five trials are an initial Release baseline, not the final statistical sample
for publication figures.

## Four-voter E2E measurement

| Stage | Time ms |
|---|---:|
| DKG | 10.499 |
| Registration close | 0.424 |
| Vote encryption | 0.969 |
| Native CP prove/verify | 2.156 |
| Combine shares and decode | 5.767 |
| P1 setup | 60.413 |
| P1 prove | 30.697 |
| P1 verify | 0.925 |

The public artifact is 1,491 bytes; private threshold packages total 1,551
bytes. These are native-core plus P1 measurements, not full-P3 measurements.

## Raw evidence

- `experiments/g0_7_raw/p1-trial-0.json` through `p1-trial-4.json`;
- `experiments/g0_7_raw/e2e-public.json`;
- `experiments/g0_7_raw/runs.csv`;
- `experiments/g0_7_raw/g07_release_baseline.json`.

The current benchmark writer has a legacy `NO_GIT_REPOSITORY` placeholder in
the `runs.csv` Git column. The authoritative frozen commit is recorded in this
report and the companion JSON; performance rows themselves are unchanged.

## Gate decision

`G0.7_RELEASE_BASELINE = PASS_WITH_STRIP_WORKAROUND`.

G0.8 may proceed in a later task. No Jubjub protocol migration was implemented
in this step.

