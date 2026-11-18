# Stage1 A-07 / B-03: independent PK-Reuse integration archive

This commit uploads already-completed implementation and evidence. It does not
run a new experiment, authorize Gate C, or modify the original experiment or
integration worktrees. Its parent is archive commit
`2cee60b88785ddf4fb25f2ffd72f55292cf56dec`.

## Scope and locations

Four root files are updated byte-for-byte from the independent integration
worktree, matching the new A-07 source pins:

- `crates/vv-friendly-dkg/src/bin/stage1_multibatch_materials.rs`
- `contracts/scripts/stage1_multibatch_compare.mjs`
- `scripts/stage1_multibatch_audit.mjs`
- `docs/STAGE1_MULTIBATCH.md`

New evidence is under:

- `experiments/stage1_multibatch_raw/gate-a-20261117-07/`
- `experiments/stage1_multibatch_raw/gate-b-20261117-03/`
- `results/stage1_multibatch/gate-a-20261117-07/`
- `results/stage1_multibatch/gate-b-20261117-03/`
- `results/stage1_multibatch/a07-b03-integration-upload-20261118/`

The first four directories preserve 44 regular artifact files exactly,
including the original 43-entry B-03 `SHA256SUMS`. The last directory adds
upload inventory/integrity evidence and verbatim execution wrapper scripts,
intents, stdout/stderr and receipts from the isolated experiment directory.
No historical A-01..06/B-01..02 result or prior Pilot/profile artifact is
overwritten. Root source updates are the NEW A-07 implementation; they are
not a rewrite or revalidation of A-06. Use the earlier immutable commit to
inspect the old root implementation.

## Actual result and boundaries

Result: `PASS_GATE_A07_AND_REUSE_INTEGRATION`, scoped to A-07 and two-batch
materials-only integration through the official Runner, `prepare` and
`prove-range` interfaces. The independent summary audit passed.

- Setup generation: **0**. PID 89744 was a frozen-Setup validation/copy worker,
  not a new Setup computation. Its wall time was 1.786933083 s.
- One range proof worker: PID 89791; PK load count **1**.
- One full PK load: **577.432461791 s**; canonical deserialize:
  **575.854906500 s**.
- Range worker total: **623.622879458 s**.
- Materials command wall: **626.464793084 s**, including preparation and
  the validation worker. These scopes must not be conflated.
- Batch 1/2 Prove: **18.983009959 / 18.852035708 s**.
- Both batch bodies: PK deserialize **0 ms**; the shared load is separately
  charged, not free or omitted.
- Instant RSS after each batch's objects were dropped:
  **2770.703125 / 2363.234375 MiB**.
- Process lifetime peak RSS: **6171.671875 MiB**; cumulative peaks at batch
  boundaries: **5006.609375 / 6171.671875 MiB**.

Instant `resident_size` and `ru_maxrss` are separate measurements. No memory
decrease is required by the audit; two batches do not establish bounded
eight-batch residency. No statistical, complete-system or Gas advantage is
claimed. Offline schema tests use explicitly labelled synthetic operational
fields with real archived proof bytes; they are not performance samples.

The A-07 integration path explicitly rejects Gate C. Eight-batch execution,
24-VM formal testing, and EVM/Gas integration are NOT admitted by this result.
B-03 is not a rerun of the nine-VM B-02 regression. No EVM or Gate D operation
was run. Gate C remains `NOT AUTHORIZED / NOT STARTED`.

## Provenance and build fingerprint

Frozen protocol config, Cargo manifests/lockfile, circuits, Solidity and old
fixtures remain byte-identical. The new source pins and budgets belong to
A-07 `integration_config.json` and `gate_a.json`, not A-06.

B-03 deliberately reuses the original request bytes as an experimental RNG
seed input to check byte equivalence. That file's historical PID/time and
run identity are not the new execution metadata. Actual B-03 identity is
recorded separately in `integration_request.json` and command receipts.
The deterministic RNG remains research-only, not production randomness.

Both 192-byte Ark proofs, 512-byte EIP-2537 proofs, all 37 public inputs,
batch roots and ciphertext components match the historical reference. Batch
1 new state equals Batch 2 old state. Native proof verification was actually
performed during the archived run; this upload does not rerun it.

Full fingerprints are in A-07 `gate_a.json` and B-03
`integration_report.json`: Rust/Cargo 1.99.1, target
`aarch64-apple-darwin`, Release `opt-level=1`, two build jobs and four Rayon
threads. Initial changed-source build wall was 19.148065292 s; A-07's build
command was a separate cached check, not a second fresh compile.

- PK SHA-256:
  `448d9d92b9250aa2bcbf59c82036ab1615f2fe323bbb5a39b8f8d7ec9f73da1c`
- Request SHA-256:
  `e5cf3a56500341f4a91da8dd7f0f6d3eed97431bc7d01eb265372edc5552f238`
- Ark VK SHA-256:
  `92871cac5c3452ab0a1647939507d7b041dcccf189e3b60ea2b19c1d6e1e1856`
- EIP VK SHA-256:
  `1367728908bb6c62009afb38ad09555d5a8c463bc6945b3ceb83ae763f376896`
- Cargo.lock SHA-256:
  `b37e02e7d43bb2cf5bcaabce29986b19796a8f47af0f0f57a2f9c14c43904b6e`
- Native executable SHA-256:
  `b50cf66406a661a698cc8b1cfaed9feb1891b9bf2de59b75a51cbd65bb9c58f1`

## Exclusions and verification

Not uploaded: the 847800624-byte proving key, its run symlink, the native
executable/build-target directory, the integration target cache, or the
node_modules dependency symlink. The key and executable remain local; only
their hashes and provenance are archived. No Git LFS is needed. This branch
alone is **not a complete standalone proof-reproduction package**.

The code retains intentional original-HEAD/branch admission checks and the
archived scripts retain original local absolute paths. Do not run them
blindly from this archive commit or edit old pins to make them pass.

From the repository root, without compiling or running experiments:

```sh
shasum -a 256 -c results/stage1_multibatch/gate-b-20261117-03/SHA256SUMS
shasum -a 256 -c results/stage1_multibatch/a07-b03-integration-upload-20261118/UPLOAD_SHA256SUMS
```

The upload integrity check separately confirms unchanged original and
integration HEAD/index/status/source/evidence. The original experiment HEAD
remains `3f8813c9d69d0e45631e9acb7bbb247a1119f511`, and the integration
worktree remains uncommitted on `feat/stage1-pk-reuse-integration` at that
same HEAD. Only the detached archive-upload worktree may commit/push, only
to `archive/stage1-a06-budget-evidence`, with no force push or merge.
