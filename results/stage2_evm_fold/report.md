# Stage-two EVM Direct Fold vs State Proof

Status: PASS_PAIRED_EVM_FOLD_COMPARISON. Run: `experiments/stage2_evm_fold_raw/run-20261114-01`.

D5/W32, 32 registered, 8 cast, three candidates, two consecutive B4. Independent fresh Prague VMs; same initial VM root, certified checkpoint, CT1, CT2 and freeze seal.

| Metric | Actual Jubjub Direct Fold | State Proof |
| --- | ---: | ---: |
| Batch 1 receive Gas | 4523055 | 3400469 |
| Batch 2 receive Gas | 4838516 | 3402641 |
| Two-batch receive Gas | 9361571 | 6803110 |
| Receive Gas per vote (8 votes) | 1170196.375 | 850388.75 |
| Two-batch calldata bytes | 12744 | 13896 |
| Receiver deployment Gas (includes child verifiers) | 6721411 | 10856237 |
| Gate + receiver deployment Gas | 14947967 | 19082793 |
| Certificate authentication Gas | 10593712 | 10593712 |
| Freeze Gas | 202588 | 204028 |
| Successful deploy/auth/receive/freeze prefix Gas | 35105838 | 36683643 |
| Local worker peak RSS MiB | 316.21875 | 320.9375 |

State Proof saves 2,558,461 receive Gas (27.329%) but costs 4,134,826 extra deployment Gas. For this two-batch successful prefix, State Proof costs 1,577,805 MORE Gas in total. Extra calldata is 576 bytes per batch (512-byte EIP-2537 state proof plus ABI overhead). No overall system-efficiency conclusion follows.

## Executed commands

`node scripts/stage2_evm_fold_run.mjs experiments/stage2_evm_fold_raw/run-20261114-01`

- `cargo +1.99.1 build --release --locked --offline -p vv-friendly-dkg --bin stage2_point_vectors` — exit 0; log `logs/build-rust-reference.log`.
- `cargo +1.99.1 fmt --all -- --check` — exit 0; log `logs/fmt.log`.
- `cargo +1.99.1 clippy --release --locked --offline -p vv-friendly-dkg --bin stage2_point_vectors -- -D warnings` — exit 0; log `logs/clippy-reference.log`.
- `/Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter/target/release/stage2_point_vectors /Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter/experiments/stage2_evm_fold_raw/run-20261114-01/rust_points.json` — exit 0; log `logs/rust-reference.log`.
- `/opt/homebrew/Cellar/node/23.11.0/bin/node --max-old-space-size=6144 /Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter/contracts/scripts/stage2_evm_fold_compare.mjs /Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter/experiments/stage2_evm_fold_raw/run-20261114-01 prepare` — exit 0; log `logs/compile.log`.
- `/opt/homebrew/Cellar/node/23.11.0/bin/node --max-old-space-size=6144 /Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter/contracts/scripts/stage2_evm_fold_compare.mjs /Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter/experiments/stage2_evm_fold_raw/run-20261114-01 points` — exit 0; log `logs/point-conformance.log`.
- `/opt/homebrew/Cellar/node/23.11.0/bin/node --max-old-space-size=6144 /Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter/contracts/scripts/stage2_evm_fold_compare.mjs /Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter/experiments/stage2_evm_fold_raw/run-20261114-01 direct_fold` — exit 0; log `logs/direct-fold-vm.log`.
- `/opt/homebrew/Cellar/node/23.11.0/bin/node --max-old-space-size=6144 /Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter/contracts/scripts/stage2_evm_fold_compare.mjs /Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter/experiments/stage2_evm_fold_raw/run-20261114-01 state_proof` — exit 0; log `logs/state-proof-vm.log`.

## Correctness and necessary negative checks

Rust/EVM: four canonical roundtrips, four additions, both real ciphertext folds; three invalid canonical/subgroup probes rejected.
Both routes: 18 signed transactions each, 9 successes and 9 expected normal reverts; all failed application checkpoints unchanged.
Wrong new state is tested before first acceptance and before freezing: four successful ballot pairings, then Direct Fold mismatch; State route runs a fifth pairing returning zero.

| Negative | Direct Fold reason | State Proof reason |
| --- | --- | --- |
| uncertified | UNCERTIFIED | UNCERTIFIED |
| wrong_new_state_before_first_acceptance | DIRECT_FOLD_MISMATCH | INVALID_GROTH16 |
| wrong_old_state | OLD_STATE | OLD_STATE |
| wrong_batch_root | ACCEPTED_ROOT | ACCEPTED_ROOT |
| another_voter_proof | INVALID_GROTH16 | INVALID_GROTH16 |
| changed_ciphertext_encoding_in_committed_matrix | BATCH_ROOT | BATCH_ROOT |
| replay_first_batch | NEXT_INDEX | NEXT_INDEX |
| cross_batch_spent_nullifier | SPENT_NULLIFIER | SPENT_NULLIFIER |
| receive_after_freeze | FROZEN | FROZEN |

Changing a ciphertext inside the committed matrix is rejected at BATCH_ROOT, not falsely described as a decoder or SNARK failure. Full standalone point rejection is tested by the point harness. Revert atomicity concerns protocol application state, not fee/nonce rollback.

## Accounting and scope

Primary Gas uses actual signed runTx receipts, reconciled against intrinsic + execution, refund and Prague calldata floor. Deployment receipts already include child deployments. Every transaction has a separate 30M block context, not a claim that all transactions fit one block. Tx cap is 20M; no unlimited code-size setting or custom precompile.

Inclusive ballot-verifier call cost is 1,970,437 Gas per batch on both routes. State-verifier call cost is 886,119 Gas per batch. Neither is an additive decomposition of total Gas; direct fold inline-only cost remains null.

- One fixed election and archived proof pool; receipt Gas is deterministic for these concrete transactions, not a statistical or scale claim.
- Both receivers use the same individual Groth16 fallback. Neither verifies exact SnarkPack on EVM.
- Direct Fold is actual Jubjub compressed decoding and group addition, not affine metadata or BLS12-381 G1 substitution.
- Internal decodeVerifiedCiphertext is valid only after exact ballot bytes pass the pinned original ballot SNARK and with certified prime-order PK; old state is identity or prior verified sums.
- Point harness uses full subgroup checks, including identity acceptance and order-two rejection.
- No new Setup, Prove, DKG, P3 or Winner writing; proof reuse is not zero original generation cost.
- All per-call traces are inclusive; child and parent costs cannot be added. Inline fold-only cost is not separately instrumented and is null.
- RSS is full local Node process high-water mark, not Ethereum node memory or algorithm incremental memory.
- Failed negative transactions are excluded from successful workflow totals; Ethereum sender nonce and fees still change on revert.
- Successful prefix includes deployment, certificate authentication, reception and freeze, but not CP/P3/Winner. This is not complete election lifecycle Gas.
- Historical state Prove time is not added to these archived-proof EVM receipts as a newly measured end-to-end time.

## Provenance and reproduction

Base Git SHA: `0098579fbacd2df0efca9532e8e37dced36ed013`; dirty working-tree source snapshot SHA-256: `e71cbd4e726a831a1258405ffdb5f00347163ab26594d4fb733ff33b9a3aa8f7`. This is not a clean committed-source reproduction.
Config SHA-256: `e48c5ccf4bc9476a0d5b62413d2a0c0ac059c48b4623d77ee19836825d89f9d8`.

Raw `source_snapshot/`, `source_files.json`, `source_pins.json`, `compiled.json`, `execution.json`, `rust_points.json`, `points.json`, `direct_fold.json`, `state_proof.json` and all logs are preserved. Full public signed transaction bytes, per-call traces, deployment bytecode and before/after negative snapshots are included. No private key shares, tally openings or usable decryption shares are exported. The sender key in the local runner is a public test key, not production material.

Audit without rerunning cryptography: `node scripts/stage2_evm_fold_check.mjs experiments/stage2_evm_fold_raw/run-20261114-01`.
To reproduce actual execution, use a NEW output directory with the same driver; it refuses an existing directory.

Stop here: no B/N growth, no optimization campaign, no new P3 and no G9 work.

## Decision and remaining measurement boundary

The same two state transitions work on both receivers. State Proof reduces reception Gas in this fixed fallback backend, but deployment, calldata and off-chain generation are additional costs. Keep it as an explicitly evaluated design option, not a proven mandatory efficiency improvement. This two-batch result does not decide the best path for an exact SnarkPack backend, many-batch amortization or production elections.

The earlier matched native pilot measured 39.119 seconds for two fresh state proofs. That is historical component evidence from a different proof-generation run, NOT a new timing for the archived EIP-2537 proofs used here. No combined E2E total is manufactured. Exact inline Direct Fold-only and pure shared business costs remain null: receipt execution and inclusive verifier calls are recorded, but subtracting those calls leaves a mixed residual, not isolated group-addition Gas.

CSV totals and per-vote calculations were independently recalculated with the spreadsheet runtime, with missing original Prove/inline-only measurements left empty rather than zero. The plain CSV is not an extra workbook or a simulated benchmark.

Rebuild a report from the same raw evidence into a NEW results directory:

```sh
node scripts/stage2_evm_fold_report.mjs experiments/stage2_evm_fold_raw/run-20261114-01 /ABS/NEW_RESULTS_DIR
```

The original archive was finalized using ` node scripts/stage2_evm_fold_archive.mjs experiments/stage2_evm_fold_raw/run-20261114-01 results/stage2_evm_fold `. This one-time finalizer refuses to overwrite derived files or checksums; do not rerun it against a sealed directory. CSV can be reconstructed mechanically from summary fields using its flat export definition. Verify ` SHA256SUMS ` in both the raw and results directories, or run the read-only evidence checker for the raw archive.
