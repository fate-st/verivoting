# Stage1 A-07 / B-03 PK-Reuse integration

Status: **PASS_GATE_A07_AND_REUSE_INTEGRATION**. No Setup generation, EVM, Gate C, commit or push.

Worktree: /private/tmp/vv-pk-reuse-integration.PohVG2/worktree

Branch: feat/stage1-pk-reuse-integration; base HEAD: 3f8813c9d69d0e45631e9acb7bbb247a1119f511. Original workspace HEAD/status/index and all 16 protected paths remain unchanged (14 present, 2 absent).

## Execution and results

Stage 1 compiled the changed Rust source in 19.148065 s. A-07 separately ran cargo build against that new-code cache; it was a cached build check, not a second fresh compile. Mach RSS API compiled and ran.

Frozen Setup validation worker PID 89744; one range proof worker PID 89791. Actual Setup generation count = 0. PK loads = 1.

Once-only PK load 577.432462 s; deserialize 575.854906 s; range total 623.622879 s; materials command wall 626.464793 s.

| Batch | Witness ms | Prove ms | Verify ms | Instant RSS after Drop MiB | Cumulative peak MiB | PK deserialize ms |
|---|---:|---:|---:|---:|---:|---:|
| 1 | 3832.006667 | 18983.009959 | 2.095708 | 2770.703125 | 5006.609375 | 0 |
| 2 | 3316.019500 | 18852.035708 | 2.854000 | 2363.234375 | 6171.671875 | 0 |

Lifetime ru_maxrss: 6171.671875 MiB. Instant resident_size and cumulative ru_maxrss are distinct; no RSS decrease is required and two batches do not establish an eight-batch bound.

## Fixed hashes and provenance

- pk_sha256: `448d9d92b9250aa2bcbf59c82036ab1615f2fe323bbb5a39b8f8d7ec9f73da1c`
- request_sha256: `e5cf3a56500341f4a91da8dd7f0f6d3eed97431bc7d01eb265372edc5552f238`
- vk_ark_sha256: `92871cac5c3452ab0a1647939507d7b041dcccf189e3b60ea2b19c1d6e1e1856`
- vk_eip_sha256: `1367728908bb6c62009afb38ad09555d5a8c463bc6945b3ceb83ae763f376896`
- setup_sha256: `e6ec81ab5944f064cbb4c93bc9dbcbaacf63b1958fa56d39892263c39f88bc8d`
- cargo_lock_sha256: `b37e02e7d43bb2cf5bcaabce29986b19796a8f47af0f0f57a2f9c14c43904b6e`
- binary_sha256: `b50cf66406a661a698cc8b1cfaed9feb1891b9bf2de59b75a51cbd65bb9c58f1`

- Batch 1 Ark (192 B): `ff2b7b683b63a172c317612ab63fc093a4fa0599b7fd1faa767050ce059d45ec`; EIP (512 B): `8fad586ada7525d7b9b492d42d6b1d4387f754e31dce18959af2bc5d42333341`.
- Batch 2 Ark (192 B): `ca115876f8b48d4382d446b08a760e477ca7004da73cc2e7213191efdbfb09d7`; EIP (512 B): `bb36782045dd4ada2fd63eac11fe76d1e47f792001a812a16cd82470b5449701`.

Both proofs match B-02 bytes exactly; all 37 inputs, roots and states match; Batch 1 new state equals Batch 2 old state. Auditor schema positive and three precise negatives passed (synthetic operational schema fixtures only, not performance samples).

B-03 deliberately reuses the exact historical request bytes ONLY as a deterministic research seed input. Actual run identity is in integration_request.json and command receipts; old request PID/time are NOT new execution metadata. Frozen config remains byte-identical; new source/binary pins and execution budgets are in A-07 integration_config.json.

## Stops and limitations

No new Setup: setup-worker is explicitly a frozen-material validation/copy worker. Only two fresh state proofs were generated via the official Runner → prepare → prove-range path. B-03 is NOT a rerun of the nine-VM B regression. No Gas or eight-batch result exists. Gate C is explicitly blocked pending separate admission/authorization; old A-06/B-02 archives were not re-pinned. PK remains local via a symlink, so this archive alone is not a standalone reproduction package. No commit or push.
