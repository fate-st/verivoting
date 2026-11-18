# Stage1 A-08 wiring revision: BLOCKED, archive only

This upload saves the current four source changes and three existing preparation/evidence files from the isolated integration worktree. It does not finish A-08, authorize Gate C, run an experiment, or change either experiment worktree's HEAD.

Parent: `b48197ba6c0fc9ed7df1a3144882327335f0780a`.
Upload destination: `archive/stage1-a06-budget-evidence`.

## Actual state

**BLOCKED_GATE_A08**. The source revision adds A-08 admission, independent authorization and plan-only paths, eight-batch/material and 24-session schema wiring, worker instance-ID handshake, macOS process-start identity checks, process-group cleanup, and durable per-batch diagnostics. It is an unadmitted source revision, not a completed A-08 baseline.

Two attempts to create/build A-08 were rejected by the execution approval reviewer. The reviewer required an explicit approval in the chat body rather than only the attached implementation instructions. The assistant stopped and did not bypass that restriction.

- No A-08 raw baseline directory, `compiled.json`, immutable manifest or admitted binary fingerprint exists.
- A preliminary Rust build passed with compiler-reported 5.73s, but final edits came afterward. This is NOT the final revision's compilation acceptance.
- JavaScript syntax, Rust formatting and diff whitespace checks passed in the preceding implementation turn.
- Dedicated offline synthetic schema checks passed for 8 batches/24 sessions and rejected seven negative cases. No cryptographic or EVM execution is implied.
- The ordinary C entry rejected with `GATE_C_NOT_AUTHORIZED`.
- The formal audit entry rejected the synthetic fixture with `NON_REAL_EXECUTION_EVIDENCE`.
- The full `--plan-only` chain still awaits the completed A-08 baseline.
- `Gate C = NOT AUTHORIZED / NOT STARTED`; `Gate D = NOT STARTED`.
- No authorization file was created. No Setup, proof worker, proof generation, VM, transaction, new Gas, or performance sample was run for this upload.

The prior-turn checks are described honestly in `results/stage1_multibatch/a08-wiring-blocked-upload-20261118/status.json`; not every original stdout/stderr receipt was persisted locally. This archive does not manufacture missing execution logs.

## Preserved history and source pins

Root source files now contain the blocked A-08 wiring revision. They are intentionally different from the A-07 source pins. Use parent `b48197ba6c0fc9ed7df1a3144882327335f0780a` to recover the exact A-07 root sources. All A-07/B-03 archived files and earlier A-06/B-02 artifacts remain unchanged. Neither a source update nor this upload revalidates those historic stages against the new root code.

No Cargo manifest/lockfile, Solidity source, R1CS circuit, fixture, protocol configuration, RNG seed formula or paper was modified by this upload. Canonical original 16-path snapshots, A-07/B-03 four-directory snapshots and both workspace metadata are checked independently.

The new `results/stage1_multibatch/gate-a-20261117-08/` contains only preparation evidence:
`integrity_before.json`, `prebuild_readonly_review.md`, and a clearly labelled synthetic rejection fixture. Their directory name is not evidence of an accepted A-08 baseline.

## Exclusions and limitations

The 847800624-byte PK remains local, with SHA-256:
`448d9d92b9250aa2bcbf59c82036ab1615f2fe323bbb5a39b8f8d7ec9f73da1c`.

PK bodies/symlinks, native executables, build caches and node_modules are excluded. No Git LFS is used. This branch alone is not a complete standalone proof-reproduction package.

Only the independent detached archive worktree may commit/push. Original and integration workspaces retain HEAD:
`3f8813c9d69d0e45631e9acb7bbb247a1119f511`.

This upload does not approve final compilation, A-08 creation, Gate C, Gate D, or any subsequent experiment.

## Read-only archive verification

```sh
shasum -a 256 -c results/stage1_multibatch/a08-wiring-blocked-upload-20261118/SHA256SUMS
```
