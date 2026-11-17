# Stage1 A-06 independent evidence archive

This branch archives the approved A-06 source patches and evidence. It is **not**
the original experiment branch and is not a new execution or Gate B approval.

- Archive branch: `archive/stage1-a06-budget-evidence`.
- Base: A-05 commit `a286ee5840a575fefed20c9cd3331e7ed9726fdf`.
- Original experiment HEAD must remain `3f8813c9d69d0e45631e9acb7bbb247a1119f511`.
- Original workspace: `/Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter`.
- Separate archive worktree: `/Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-stage1-a06-archive`.

## Uploaded scope

1. Unchanged copies of the three approved Stage1 source patches: the runner,
   independent auditor, and `docs/STAGE1_MULTIBATCH.md`.
2. A-06 compilation, static checks, B-02 preflight, independent A audit,
   source hashes, complete diffs, and original checksums.
3. B-01 Setup metadata, VK/key hashes, timing/RSS records, timeout command
   record, and failed-run logs. B-01 did not pass Gate B and has no completed
   batch proof metrics; missing metrics must not be inferred or filled in.
4. An explicit upload inventory, checksums, and read-only original-workspace
   integrity checks under `results/stage1_multibatch/archive-a06/`.

A-05 evidence is inherited unchanged from the base commit, not copied over.
Copied evidence and logs are not redacted. Credential-pattern checks and manual
log review found no real authentication credential or private voter witness in
the upload set. The existing synthetic EVM signing key in the runner is a fixed
local test key, not a production credential. Local source paths and process IDs
are intentionally preserved as experimental provenance.

## Large proving key: retained locally, not uploaded

- Relative path: `experiments/stage1_multibatch_raw/gate-b-20261117-01/materials/research_pk.bin`.
- Size: **847800624 bytes**.
- SHA-256: `448d9d92b9250aa2bcbf59c82036ab1615f2fe323bbb5a39b8f8d7ec9f73da1c`.
- The original local file remains untouched. No Git LFS installation or upload.
- The key is a deterministic nonproduction research proving key, as recorded by
  the original Setup metadata. It is excluded, not represented by a fake file.

All `build-target` directories, unrelated caches/temporary files and private
witnesses are excluded. Historical snapshot JSON files may name/hash excluded
cache entries and the proving key; those references do not mean the underlying
files are present in this branch.

## Evidence and reproducibility boundaries

The copied original A-06 records report `PASS_GATE_A_COMPILE_ONLY`,
`READY_FOR_HUMAN_APPROVAL`, and `PASS_GATE_A_ARTIFACT_AUDIT`. These are records of
the original execution, **not** an independent validation performed by this
archive upload. No Setup, Prove, VM creation or transaction is authorized or
executed by the archival workflow. Gate B-02 and Gate C remain unapproved.

The runner intentionally keeps its original HEAD/source locks. Checking out
this archive branch is not equivalent to the original locked experiment
workspace. No lock is changed merely to make this branch executable.

Because the proving key and build caches are absent, this Git branch alone is
**not a standalone complete proof-reproduction package**. B-01 is an incomplete
failed run; no completed B-01 proof is claimed. A fresh, separately authorized
execution must use the original protocol/setup rules, dependencies and resource
budgets. Do not reuse the B-01 partial materials as B-02 materials.

Original A-01..A-06 and B-01 raw/results trees, including ignored caches and the
retained key, are recursively hashed before copying and checked again afterward.
The original HEAD, branch, index bytes, patch hashes and paper hash must remain
unchanged. `original_integrity.json` records the pre-commit integrity check;
the final operator report separately confirms read-only post-commit/post-push
verification. Only the independent archive worktree may commit or push, and
only the new archive ref may be pushed. No merge or force push is permitted.
