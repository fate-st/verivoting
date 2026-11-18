# Stage1 Gate B-02 regression evidence archive

This is an add-only evidence supplement to archive commit
`aed94f4810b5964da65f206b29ba3422d60576d7` on
`archive/stage1-a06-budget-evidence`. It does not alter the original experiment
workspace, source locks, A-06 evidence, B-01 failure evidence, or prior manifests.

## Included

- All 61 permitted B-02 raw/result files: fresh Setup metadata, two fresh B4
  state proofs, worker commands/logs, complete public materials, nine independent
  Prague EVM sessions with signed transaction evidence, execution and independent
  audit results, external worker RSS samples, and original checksum manifests.
- Four archive-only files: this note, an explicit upload inventory,
  original-workspace integrity record, and a separate portable checksum manifest
  under `results/stage1_multibatch/archive-b02/`.

The copied records report `PASS_GATE_B_REGRESSION_ONLY` and
`PASS_GATE_B_EVIDENCE_AUDIT_ONLY`: six positive sessions, three negative/recovery
sessions, and nine independent VM identities. These are preserved records of the
completed original single attempt, not experiments rerun during uploading.
The upload does not grant permission to begin Gate C.

The upload set was reviewed as text/public experimental evidence. No credential
pattern or private voter witness was found, and no copy was redacted. Existing
synthetic EVM test signatures and local path/PID provenance are preserved.

## Intentionally excluded proving key and caches

`experiments/stage1_multibatch_raw/gate-b-20261117-02/materials/research_pk.bin`
is retained untouched in the original local workspace, not uploaded:

- Size: **847800624 bytes**.
- SHA-256: `448d9d92b9250aa2bcbf59c82036ab1615f2fe323bbb5a39b8f8d7ec9f73da1c`.
- Deterministic nonproduction research Setup; no Git LFS installation/upload.

Build caches, temporary control scripts, unrelated files and private witnesses
are not uploaded. The previously excluded B-01 proving key is also left local.

## Checksum scopes and reproducibility boundary

The original B-02 `SHA256SUMS` is preserved verbatim. All **58** entries were
checked locally with the retained proving key. In this archive **57** of those
entries have corresponding uploaded files; the one `research_pk.bin` entry is
intentionally absent. Do not claim that its bytes exist in the Git branch.
Both entries in `EXTERNAL_EVIDENCE_SHA256SUMS` are present and verified.

The separate `archive-b02/SHA256SUMS` covers all uploaded payload and archive
metadata except itself, without requiring the omitted key. It does not replace
or rewrite either original checksum file. Existing A-06 and post-push archive
checksum files remain unchanged.

This branch alone is **not a standalone complete proof-reproduction package**.
The run is fixed-parameter individual-Groth16 EVM fallback regression, not a
formal performance sample, full SnarkPack EVM validation, public-chain deployment,
or production trusted setup. Witness diagnostic timing must not be mechanically
added to the Prove call's internally overlapping synthesis work, and sampled RSS
must not replace the Rust OS high-water mark.

Before and after copying, the original HEAD, index, status, source pins, paper,
and all A-01..A-06/B-01/B-02 raw/results trees are checked unchanged. That is 16
paths (14 existing directories, including retained keys and ignored caches).
The archive integrity JSON records the pre-commit check; final read-only
post-commit/post-push checks are separately reported. Original HEAD remains
`3f8813c9d69d0e45631e9acb7bbb247a1119f511`. Only an independent detached worktree
may commit and push this supplement; no force push, merge or experiment rerun.
