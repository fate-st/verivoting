# Stage1 Batch 1 PK profile: independent evidence archive

This is an archival upload, not a new experiment or an authorization for Gate C.
It extends `archive/stage1-a06-budget-evidence` from
`a546d483723314aaf33efab37145b77eb9f80614`. The original experiment branch and
workspace remain at `3f8813c9d69d0e45631e9acb7bbb247a1119f511` with the same
three approved, uncommitted patches and historical evidence.

## Archived source and evidence

- Independent diagnostic entry:
  `crates/vv-friendly-dkg/src/bin/stage1_multibatch_pk_profile.rs`.
- Source SHA-256:
  `6b1ee7d425a037e8355aa2a724817bf21db4b05c557fb3faedb166b4af28fb58`.
- Evidence root:
  `results/stage1_multibatch/pk-profile-batch1-20261118/`.
- Core artifacts: `batch1_profile.json`, `outer_wall.json`,
  `instrumentation.diff`, `REPORT.md`, `report.json`, `stage_timings.csv`,
  compiler/runtime logs, environment, original pre/post integrity records and
  the canonical recursive snapshot manifests.
- Original capture paths and timestamps are retained, without redaction or
  rewriting original checksums. A source copy and original batch output are
  also retained at the nested paths referenced by the original `SHA256SUMS`.
- `upload_inventory.json`, `upload_integrity.json` and `UPLOAD_SHA256SUMS`
  document this upload separately from the executed diagnostic.

The source diff adds timers, CPU/byte counters, independent diagnostic output
and a single-Batch-1 guard. It does not alter the original material generator,
its nine configuration input pins, six A-06 source pins, circuits, algorithms,
canonical validation, proof seeds, dependencies or serialization. The three
approved patches inherited by this archive already match A-06 exactly.

## Actual single-run result

Status: **PASS_PROFILE_ONLY**. Apple M4, 32 GiB RAM, macOS 15.7.1;
Rust/Cargo 1.99.1, Release `opt-level=1`, Rayon threads 4.

| Metric | Measured value |
|---|---:|
| Isolated compilation | 47.691619709 s |
| Spawn-to-observed-exit wall time | 597.139812792 s |
| Worker total including local Drops | 596.426130416 s |
| PK SHA-256 | 1.575599042 s |
| PK canonical deserialization | 571.967173458 s |
| Deserialization share of Worker | 95.89908025308344% |
| Deserialization user CPU | 560.12052 s |
| Deserialization system CPU | 7.083287 s |
| Bytes consumed before tail check | 847800624 |
| Witness diagnostic | 3.472283959 s |
| Prove | 18.767489084 s |
| Internal unattributed remainder | 0.604637624 s |
| Process peak RSS | 5739 MiB |

The Ark proof is 192 bytes, SHA-256
`ff2b7b683b63a172c317612ab63fc093a4fa0599b7fd1faa767050ce059d45ec`.
The EIP-2537 proof is 512 bytes, SHA-256
`8fad586ada7525d7b9b492d42d6b1d4387f754e31dce18959af2bc5d42333341`.
Both are byte-identical to the archived B-02 Batch 1 proofs. The original
batch JSON schema and all non-timing/non-RSS fields match. Both integer
nanosecond timing identities and all 16 protected-path checks passed
(14 existing directories unchanged; two paths still absent).

This is one diagnostic in an active-cache state, not cold-disk benchmarking
or a statistical performance claim. Deserialization includes streamed reads,
compressed point decoding and unchanged validation; no individual validation
subcomponent is claimed to account for its entire time. Historical B-02
timing includes log opening and per-poll RSS probes, unlike the new timer;
the historical 593.399 s cannot be treated as an exactly matched boundary.
Deterministic research RNG is used only for reproduction, not as a production
randomness recommendation. No PK-reuse optimization was implemented.

## Exclusions and reproducibility limits

Not uploaded:

- `research_pk.bin`, 847800624 bytes, SHA-256
  `448d9d92b9250aa2bcbf59c82036ab1615f2fe323bbb5a39b8f8d7ec9f73da1c`;
- the PK symbolic link, isolated `target/` compilation cache and old
  `build-target/` caches;
- `evidence/untracked_bytes/`, the local full-byte backup of 141 original
  untracked files, including retained large proving keys;
- unrelated files and private witnesses.

The key remains local and untouched; no LFS upload is used. Snapshot files
may name/hash excluded files. These references do not mean that their bytes
are included. This branch alone is **not a standalone complete proof-
reproduction package**. The archived control/report/wrapper scripts preserve
original absolute local paths and are provenance, not portable commands to
run blindly. Existing HEAD locks are intentionally not weakened for this
archive checkout.

From the repository root, check upload contents without executing experiments:

```sh
shasum -a 256 -c results/stage1_multibatch/pk-profile-batch1-20261118/UPLOAD_SHA256SUMS
```

Inside the evidence root, `shasum -a 256 -c SHA256SUMS` checks all original
23 artifact entries, with their original bytes and path layout preserved.

No Setup, Prove, VM, transaction or optimization is run by this upload.
The already completed diagnostic ran one Batch 1 proof and no Batch 2.
**Gate C remains NOT AUTHORIZED / NOT STARTED.** No experiment branch is
pushed or merged, and no existing remote history is rewritten.
