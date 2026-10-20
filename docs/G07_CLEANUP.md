# G0.7 branch cleanup and metadata audit

## Result

`G0.7_CLEANUP = PASS` on 2026-10-20.

The G0.7 branch commit `b4632e227760040df5cb53aed28427291934a50f`
was one commit ahead of `main` commit
`0eec138e0415593ddba31b0601f6428934632c56`. That commit added nine G0.7
report/raw-data files (361 lines) and changed no Rust source. The new G0.8 work
continues on the independent branch `experiment/g08-jubjub-native`.

No automatic merge into `main` was performed.

## Metadata repair

`vv-bench` no longer writes the fixed value `NO_GIT_REPOSITORY` for new CSV
rows. Metadata is captured once before the output directory is created:

- actual `git rev-parse HEAD`, or `UNKNOWN` if Git cannot be queried;
- dirty state from `git status --porcelain`, or `UNKNOWN` on failure;
- manifest digest and encryption/disclosure suite;
- operating system, architecture, CPU, logical threads and RAM;
- Rust version and effective build profile.

Capturing before output creation prevents the benchmark's own newly generated
files from changing `git_dirty` during a multi-trial run. Existing historical
CSV files were not edited.

## Executed metadata smoke run

One P1 Release trial was run solely to validate the new record format; the five
G0.7 performance trials were not repeated.

| Field | Recorded value |
|---|---|
| Git SHA | `b4632e227760040df5cb53aed28427291934a50f` |
| Dirty | `true` (expected for the uncommitted metadata/G0.8 worktree) |
| Suite | `Ristretto255/curve25519-dalek-4.1.3` + BLS12-381 P1 |
| CPU / RAM | Apple M4 / 32.000 GiB |
| Rust | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| Profile | `release-opt-level-1` |
| Constraints / proof | 3,521 / 192 bytes |
| Setup / prove / verify | 61.019 / 32.973 / 0.954 ms |
| Valid proof | yes |

Raw record: `experiments/g0_7_cleanup_raw/runs.csv`.

This single timing row is a metadata smoke test, not a replacement for the
five-trial baseline in `experiments/g0_7_raw/`.

## Regression result

The necessary Release validation used the documented
`CARGO_PROFILE_RELEASE_STRIP=none` workaround:

- strict Release Clippy: pass;
- workspace tests: 30 passed, 0 failed;
- resource-heavy Ristretto non-native test: 1 filtered and retained in source;
- original Ristretto tests: no regression;
- new Jubjub suite/ElGamal tests: 8 passed.

## Remaining boundary

G0.7 is stable. G0.8 currently stops after the independent Suite and native
Weighted ElGamal/BSGS fixture. Jubjub DKG and generalized CP are the next
required work; they are not reported as complete here.
