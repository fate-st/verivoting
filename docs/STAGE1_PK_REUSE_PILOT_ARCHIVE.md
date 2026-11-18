# Stage1 PK-Reuse Pilot: two-batch independent evidence archive

This is an upload of the completed, single-execution Pilot, not a new run or
an authorization for Gate C. It extends `archive/stage1-a06-budget-evidence`
from `e5398f8b8a431681138271881592caadf96233c2`, inheriting the earlier A-06,
B-02 and single-Batch-1 PK-profile evidence without changing those files.

## Source and evidence paths

- New standalone source:
  `crates/vv-friendly-dkg/src/bin/stage1_multibatch_pk_reuse_pilot.rs`.
- Source SHA-256:
  `077f358687a6382832d9432fb38004d274e2bf4181bbf44c5df44d9fbe241622`.
- Evidence root:
  `results/stage1_multibatch/pk-reuse-pilot-2batch-20261118/`.
- Core artifacts: `reuse_pilot_profile.json`, `outer_wall_rss.json`,
  `reuse_pilot.diff`, `REPORT.md`, `report.json`, both generated batch JSONs,
  raw RSS samples, compiler/runtime logs, environment, input checks and
  canonical pre/post snapshots for the 16 protected paths.
- The original 26-entry `SHA256SUMS` and its nested source/batch-output paths
  are preserved byte-for-byte. `upload_inventory.json`,
  `upload_integrity.json` and `UPLOAD_SHA256SUMS` describe this upload
  separately from the actual Pilot execution.

All existing configuration pins (9 inputs) and A-06 source pins (6 files),
the original material generator, Cargo manifests/lockfile, circuits, curves,
serialization, setup parameters and per-batch proof seed derivation remain
unchanged. The Pilot source uses one full canonical
`ProvingKey::deserialize_compressed()`, including its original validation,
tail check and VK hash check. Each batch borrows the same immutable key;
there is no unchecked deserialization or PK clone.

The batch JSON field structure is unchanged. `proof_origin` and `rss_scope`
truthfully describe single-process reuse and cumulative process high-water
marks; they are not falsely labelled as independent per-batch workers.

## Actual result: PASS_REUSE_PILOT_ONLY

Apple M4, 32 GiB RAM, macOS 15.7.1, Rust/Cargo 1.99.1, Release `opt-level=1`,
Rayon threads 4. One proving process executed Batch 1 then Batch 2 exactly
once; no Setup was run.

| Metric | Actual value |
|---|---:|
| PK load count | 1 |
| Once-only complete PK load | 575.703350209 s |
| Canonical deserialization within that load | 574.121565333 s |
| Batch 1 PK deserialization | 0.0 ms |
| Batch 2 PK deserialization | 0.0 ms |
| Batch 1 total excluding shared load | 22.628008458 s |
| Batch 2 total excluding shared load | 22.254112084 s |
| Complete outer wall time including shared load | 621.501246625 s |
| Process peak RSS, native ru_maxrss | 6173.09375 MiB |
| External sampled peak RSS | 6173.09375 MiB |
| Peak after Batch 1, cumulative | 5610.890625 MiB |
| Peak after Batch 2, cumulative | 6173.09375 MiB |

The two per-batch zero values mean no decode call occurs inside either batch
body; they do not omit or zero the separately charged shared load cost.
The actual deserialize consumed all 847800624 bytes before its tail check.
Historical and Pilot `request.json` SHA-256 both equal
`e5cf3a56500341f4a91da8dd7f0f6d3eed97431bc7d01eb265372edc5552f238`.

Both 192-byte Ark and 512-byte EIP-2537 proofs are byte-identical to the B-02
independent-worker outputs. The Ark hashes are:

- Batch 1: `ff2b7b683b63a172c317612ab63fc093a4fa0599b7fd1faa767050ce059d45ec`.
- Batch 2: `ca115876f8b48d4382d446b08a760e477ca7004da73cc2e7213191efdbfb09d7`.

The roots, all ciphertext state components, public inputs and VK/export bytes
also match. Batch 1's final state equals Batch 2's initial state. Both proofs
were genuinely verified locally. Integer nanosecond timing identities and
original-workspace integrity checks all passed.

## Interpretation and limits

Historical two-worker wall time was 1185.116814208 s; the single Pilot was
621.501246625 s. Their derived difference is 563.615567583 s, about 1.906862x.
This is a single cross-run diagnostic comparison, not independent repeated
statistics or a complete-system/Gas advantage. Timing boundaries and polling
overhead are documented in the report. The cache was active, not controlled
cold-disk storage. Full canonical validation was retained; no particular
validation subcomponent is claimed to account for its entire cost.

The second batch increased the cumulative native peak by 562.203125 MiB;
two observations do not establish bounded residency for eight batches.
System Swap used and Swapout counters did not increase during this run, but
these are host-global measurements and do not prove absence of process
swapping. A two-point eight-batch time model gives about 756-758 s and a
linear high-water memory scenario about 9546 MiB, both **DERIVED / NOT
EXECUTED**, not resource guarantees. Physical memory minus process RSS is
not available system memory. No arbitrary 16 GiB threshold was used as the
sole failure criterion.

The original child budget is still 600 s. This standalone Pilot did not call
the original `spawn_worker`; integrating a range worker requires a separate
review of timing boundaries and budget policy. No budget was changed here,
and this result does not authorize Gate C.

Deterministic ChaCha20Rng is solely an experimental byte-reproduction setting,
not a production randomness recommendation. No privacy theorem, production
trusted setup, eight-batch test or EVM performance result is claimed.

## Excluded local materials and archive verification

Not uploaded: the 847800624-byte `research_pk.bin`, its symlink, `target/`,
any old `build-target/` caches, unrelated backups or private witnesses.
PK SHA-256 is
`448d9d92b9250aa2bcbf59c82036ab1615f2fe323bbb5a39b8f8d7ec9f73da1c`.
It remains untouched locally; no LFS upload is used. Snapshot manifests may
name/hash excluded files, but their bytes are not present in this archive.
Thus the branch alone is **not a complete standalone proof-reproduction
package**. Original local absolute paths in the archived control/wrapper
scripts are provenance; do not run them blindly as portable commands.

From the repository root, without executing any experiment:

```sh
shasum -a 256 -c results/stage1_multibatch/pk-reuse-pilot-2batch-20261118/UPLOAD_SHA256SUMS
```

Inside the evidence root, `shasum -a 256 -c SHA256SUMS` checks the original
26 diagnostic artifact entries. No historical file/checksum is rewritten.

The original experiment workspace keeps HEAD
`3f8813c9d69d0e45631e9acb7bbb247a1119f511` and exactly the same Git status,
source pins, proof key, paper and protected evidence. Only an independent
archive worktree may commit/push, only to the archive branch, without merge
or force push. The upload performs no Setup, Prove, VM or transaction.
**Gate C remains NOT AUTHORIZED / NOT STARTED.**
