# G0.6 release reproducibility record

## Outcome

G0.6 is **partially complete**. The frozen source and manifest were identified,
the development-profile regression suite and strict Clippy check pass, and the
isolated curve-decision experiment is reproducible. The required Release build
did not complete in the available run: it remained at `Compiling proc-macro2
v1.0.107` for more than eight minutes and was terminated with exit code `130`.
Consequently, this document does not claim that Release tests, Release E2E, or
five Release P1 trials passed.

Status: `BLOCKED_RELEASE_BUILD_TIMEOUT`.

## Frozen inputs and environment

| Item | Recorded value |
|---|---|
| Baseline Git commit | `92d147001fe84c58f85d7692ff9980b4684cd193` |
| Experiment date | `2026-10-20` (Asia/Shanghai) |
| OS | macOS `15.7.1` (`24G231`), Darwin `24.6.0`, arm64 |
| CPU | Apple M4, 10 logical CPUs |
| Memory | 32 GiB (`34,359,738,368` bytes) |
| Rust | `rustc 1.99.1 (58cf8f9c3 2026-10-13)`; LLVM `23.1.1` |
| Cargo profile requested | `release`, repository setting `opt-level = 1` |
| Manifest canonical digest | `c1b74c703e1f52d3ee8a1ea94c46433cc7991fccf0d64d503209e737dc5f29dd` |
| Manifest file SHA-256 | `e65d6861b975a65c0f96617b3ab8faada47ce5b93219190ee871a4feefb98503` |
| Native encryption group | `Ristretto255/curve25519-dalek-4.1.3` |

The baseline repository was on branch `main` and clean before the G0.6 changes.
The source commit above identifies that baseline; the G0.6 files in this worktree
are uncommitted experiment changes and do not pretend to be part of that commit.

## Commands and actual results

Required Release command attempted:

```text
cargo test --workspace --release --locked
```

Actual result: `BLOCKED_RELEASE_BUILD_TIMEOUT`; first Release compilation did
not progress beyond `proc-macro2 v1.0.107` during the eight-minute observation
window and was interrupted. The following dependent Release measurements are
therefore `NOT_RUN`, not zero:

- `cargo clippy --workspace --all-targets --release -- -D warnings`;
- `cargo run --release --locked -p vv-bench -- e2e`;
- `cargo run --release --locked -p vv-bench -- p1 --trials 5`.

Checks completed in the development profile after the Release block:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked -- \
  --skip real_dalek_equation_satisfies_non_native_r1cs_and_mutation_fails
cargo run -p vv-bench --locked -- curve-compare \
  --output experiments/raw --seed 20261020
python3 scripts/curve_compare.py experiments/raw
```

The development regression run passed 22 tests with zero failures and one
filtered resource-heavy Ristretto R1CS test. The omitted relation had already
been measured by the frozen G0.5 JSON artifacts; re-synthesizing it would use
about 8 GiB and does not add a new G0.6 decision datum. The strict development
Clippy run passed after toolchain selection was made explicit.

## Binding regression coverage

The retained tests cover the established chain:

```text
policy_pre -> prekey_context -> DKG session_id -> KeyCore
           -> final context -> registration/threshold transcript
```

They reject policy substitution, stale final context, cross-election replay,
stale DKG session data, bad registration openings, invalid CP challenges,
commitments, and responses. No protocol encoding or native curve was changed by
the G0.6 friendly-curve experiment.

## Reproduction boundary

The Release gate must be rerun on a machine or CI runner that can finish the
first optimized dependency build. Until then, Release E2E and five-trial P1
numbers remain absent. Development-profile timings in the curve comparison are
valid measured prototype data, but they are not Release performance claims.

