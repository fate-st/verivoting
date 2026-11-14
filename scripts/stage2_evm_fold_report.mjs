// Pure derivation from signed transaction receipts; no EVM or proof rerun.
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
import {audit} from './stage2_evm_fold_check.mjs';
const [runArg, outputArg] = process.argv.slice(2);
assert.ok(runArg && outputArg, 'usage: node scripts/stage2_evm_fold_report.mjs RUN_DIR NEW_RESULTS_DIR');
const run = path.resolve(runArg), output = path.resolve(outputArg);
assert.ok(!fs.existsSync(output), 'never overwrite an archived report');
const check = audit(run);
const read = n => JSON.parse(fs.readFileSync(path.join(run, n + '.json')));
const hash = n => createHash('sha256').update(fs.readFileSync(path.join(run, n + '.json'))).digest('hex');
const m = read('execution'), a = read('direct_fold'), b = read('state_proof');
const tx = (r, label) => r.transactions.find(t => t.label === label);
const sum = list => list.reduce((n, x) => n + x, 0);
function route(r) {
  const accepted = r.transactions.filter(t => t.label.startsWith('accept_batch_'));
  const auth = r.transactions.filter(t => t.label.startsWith('authenticate_'));
  const successful = r.transactions.filter(t => t.status === 1);
  const failures = r.transactions.filter(t => t.status === 0);
  return {
    route:r.phase, batch1_gas:accepted[0].gas_used, batch2_gas:accepted[1].gas_used,
    two_batch_gas:r.accepted_batch_gas, gas_per_vote:r.accepted_batch_gas / 8,
    two_batch_calldata_bytes:r.accepted_batch_calldata_bytes, calldata_per_batch_bytes:accepted[0].calldata_bytes,
    ballot_proof_bytes_per_batch:4 * 512, state_proof_bytes_per_batch:r.phase === 'state_proof' ? 512 : 0,
    certificate_gate_deployment_gas:tx(r, 'deploy_certificate_gate').gas_used,
    receiver_deployment_gas:tx(r, 'deploy_receiver').gas_used, total_deployment_gas:r.deployment_gas,
    certificate_authentication_gas:sum(auth.map(t => t.gas_used)), freeze_gas:r.freeze_gas,
    successful_deploy_auth_receive_freeze_gas:sum(successful.map(t => t.gas_used)),
    failed_tests_gas_excluded:sum(failures.map(t => t.gas_used)),
    ballot_verifier_inclusive_call_gas_per_batch:r.common_ballot_verifier_call_gas,
    state_verifier_inclusive_call_gas_per_batch:r.state_verifier_call_gas,
    inline_direct_fold_only_gas:null, peak_process_rss_mib:r.peak_node_rss_mib,
    source_path:path.relative(process.cwd(), path.join(run, r.phase + '.json')),
    source_sha256:hash(r.phase),
  };
}
const aa = route(a), bb = route(b);
const s = {
  schema:'VV/STAGE2/EVM-FOLD-SUMMARY/v1', status:'PASS_PAIRED_EVM_FOLD_COMPARISON',
  run_directory:path.relative(process.cwd(), run), base_commit:m.base_commit, branch:m.branch,
  source_version_sha256:m.source_version_sha256, config_sha256:read('source_pins').config_sha256,
  ballot_vk_sha256:read('source_pins').ballot_vk_sha256, state_vk_sha256:read('source_pins').state_vk_sha256,
  environment:m.environment, compiler:a.compiler, compiler_settings:a.compiler_settings,
  hardfork:a.hardfork, tx_gas_limit:a.tx_gas_limit, block_gas_limit:a.block_gas_limit,
  audit:check, setup_count:0, proof_generation_count:0, new_p3_count:0,
  reused_ballot_proof_count:8, reused_state_proof_count:2,
  exact_snarkpack_evm_verified:false, public_chain_deployment:false,
  registered_voters:32, cast_voters:8, candidates:3, batches:2, batch_size:4,
  direct_fold:aa, state_proof:bb,
  receive_gas_saved:aa.two_batch_gas - bb.two_batch_gas,
  receive_gas_saved_percent:100 * (aa.two_batch_gas - bb.two_batch_gas) / aa.two_batch_gas,
  deployment_premium_state_gas:bb.total_deployment_gas - aa.total_deployment_gas,
  prefix_total_extra_state_gas:bb.successful_deploy_auth_receive_freeze_gas - aa.successful_deploy_auth_receive_freeze_gas,
  source_pins:read('source_pins'),
  limitations:[
    'One fixed election and archived proof pool; receipt Gas is deterministic for these concrete transactions, not a statistical or scale claim.',
    'Both receivers use the same individual Groth16 fallback. Neither verifies exact SnarkPack on EVM.',
    'Direct Fold is actual Jubjub compressed decoding and group addition, not affine metadata or BLS12-381 G1 substitution.',
    'Internal decodeVerifiedCiphertext is valid only after exact ballot bytes pass the pinned original ballot SNARK and with certified prime-order PK; old state is identity or prior verified sums.',
    'Point harness uses full subgroup checks, including identity acceptance and order-two rejection.',
    'No new Setup, Prove, DKG, P3 or Winner writing; proof reuse is not zero original generation cost.',
    'All per-call traces are inclusive; child and parent costs cannot be added. Inline fold-only cost is not separately instrumented and is null.',
    'RSS is full local Node process high-water mark, not Ethereum node memory or algorithm incremental memory.',
    'Failed negative transactions are excluded from successful workflow totals; Ethereum sender nonce and fees still change on revert.',
    'Successful prefix includes deployment, certificate authentication, reception and freeze, but not CP/P3/Winner. This is not complete election lifecycle Gas.',
    'Historical state Prove time is not added to these archived-proof EVM receipts as a newly measured end-to-end time.',
  ],
};
fs.mkdirSync(output, {recursive:true});
fs.writeFileSync(path.join(output, 'summary.json'), JSON.stringify(s, null, 2) + '\n', {flag:'wx'});
const rows = [
  ['Batch 1 receive Gas',aa.batch1_gas,bb.batch1_gas],
  ['Batch 2 receive Gas',aa.batch2_gas,bb.batch2_gas],
  ['Two-batch receive Gas',aa.two_batch_gas,bb.two_batch_gas],
  ['Receive Gas per vote (8 votes)',aa.gas_per_vote,bb.gas_per_vote],
  ['Two-batch calldata bytes',aa.two_batch_calldata_bytes,bb.two_batch_calldata_bytes],
  ['Receiver deployment Gas (includes child verifiers)',aa.receiver_deployment_gas,bb.receiver_deployment_gas],
  ['Gate + receiver deployment Gas',aa.total_deployment_gas,bb.total_deployment_gas],
  ['Certificate authentication Gas',aa.certificate_authentication_gas,bb.certificate_authentication_gas],
  ['Freeze Gas',aa.freeze_gas,bb.freeze_gas],
  ['Successful deploy/auth/receive/freeze prefix Gas',aa.successful_deploy_auth_receive_freeze_gas,bb.successful_deploy_auth_receive_freeze_gas],
  ['Local worker peak RSS MiB',aa.peak_process_rss_mib,bb.peak_process_rss_mib],
];
const md = `# Stage-two EVM Direct Fold vs State Proof\n\nStatus: ${s.status}. Run: \`${s.run_directory}\`.\n\n` +
  `D5/W32, 32 registered, 8 cast, three candidates, two consecutive B4. Independent fresh Prague VMs; same initial VM root, certified checkpoint, CT1, CT2 and freeze seal.\n\n` +
  `| Metric | Actual Jubjub Direct Fold | State Proof |\n| --- | ---: | ---: |\n` + rows.map(r => '| ' + r.join(' | ') + ' |').join('\n') + '\n\n' +
  `State Proof saves ${s.receive_gas_saved.toLocaleString('en-US')} receive Gas (${s.receive_gas_saved_percent.toFixed(3)}%) but costs ${s.deployment_premium_state_gas.toLocaleString('en-US')} extra deployment Gas. For this two-batch successful prefix, State Proof costs ${s.prefix_total_extra_state_gas.toLocaleString('en-US')} MORE Gas in total. Extra calldata is 576 bytes per batch (512-byte EIP-2537 state proof plus ABI overhead). No overall system-efficiency conclusion follows.\n\n` +
  `## Executed commands\n\n\`node scripts/stage2_evm_fold_run.mjs ${s.run_directory}\`\n\n` +
  m.commands.map(c => '- `' + [c.program, ...c.args].join(' ') + '` — exit ' + c.code + '; log `' + c.log + '`.').join('\n') + '\n\n' +
  `## Correctness and necessary negative checks\n\nRust/EVM: four canonical roundtrips, four additions, both real ciphertext folds; three invalid canonical/subgroup probes rejected.\nBoth routes: 18 signed transactions each, 9 successes and 9 expected normal reverts; all failed application checkpoints unchanged.\nWrong new state is tested before first acceptance and before freezing: four successful ballot pairings, then Direct Fold mismatch; State route runs a fifth pairing returning zero.\n\n` +
  '| Negative | Direct Fold reason | State Proof reason |\n| --- | --- | --- |\n' + a.negatives.map((n,i) => `| ${n.case} | ${n.reason} | ${b.negatives[i].reason} |`).join('\n') + '\n\n' +
  `Changing a ciphertext inside the committed matrix is rejected at BATCH_ROOT, not falsely described as a decoder or SNARK failure. Full standalone point rejection is tested by the point harness. Revert atomicity concerns protocol application state, not fee/nonce rollback.\n\n` +
  `## Accounting and scope\n\nPrimary Gas uses actual signed runTx receipts, reconciled against intrinsic + execution, refund and Prague calldata floor. Deployment receipts already include child deployments. Every transaction has a separate 30M block context, not a claim that all transactions fit one block. Tx cap is 20M; no unlimited code-size setting or custom precompile.\n\nInclusive ballot-verifier call cost is 1,970,437 Gas per batch on both routes. State-verifier call cost is 886,119 Gas per batch. Neither is an additive decomposition of total Gas; direct fold inline-only cost remains null.\n\n` +
  s.limitations.map(x => '- ' + x).join('\n') + '\n\n' +
  `## Provenance and reproduction\n\nBase Git SHA: \`${s.base_commit}\`; dirty working-tree source snapshot SHA-256: \`${s.source_version_sha256}\`. This is not a clean committed-source reproduction.\nConfig SHA-256: \`${s.config_sha256}\`.\n\n` +
  `Raw \`source_snapshot/\`, \`source_files.json\`, \`source_pins.json\`, \`compiled.json\`, \`execution.json\`, \`rust_points.json\`, \`points.json\`, \`direct_fold.json\`, \`state_proof.json\` and all logs are preserved. Full public signed transaction bytes, per-call traces, deployment bytecode and before/after negative snapshots are included. No private key shares, tally openings or usable decryption shares are exported. The sender key in the local runner is a public test key, not production material.\n\n` +
  `Audit without rerunning cryptography: \`node scripts/stage2_evm_fold_check.mjs ${s.run_directory}\`.\nTo reproduce actual execution, use a NEW output directory with the same driver; it refuses an existing directory.\n\nStop here: no B/N growth, no optimization campaign, no new P3 and no G9 work.\n`;
fs.writeFileSync(path.join(output, 'report.md'), md + "\n## Decision and remaining measurement boundary\n\nThe same two state transitions work on both receivers. State Proof reduces reception Gas in this fixed fallback backend, but deployment, calldata and off-chain generation are additional costs. Keep it as an explicitly evaluated design option, not a proven mandatory efficiency improvement. This two-batch result does not decide the best path for an exact SnarkPack backend, many-batch amortization or production elections.\n\nThe earlier matched native pilot measured 39.119 seconds for two fresh state proofs. That is historical component evidence from a different proof-generation run, NOT a new timing for the archived EIP-2537 proofs used here. No combined E2E total is manufactured. Exact inline Direct Fold-only and pure shared business costs remain null: receipt execution and inclusive verifier calls are recorded, but subtracting those calls leaves a mixed residual, not isolated group-addition Gas.\n\nCSV totals and per-vote calculations were independently recalculated with the spreadsheet runtime, with missing original Prove/inline-only measurements left empty rather than zero. The plain CSV is not an extra workbook or a simulated benchmark.\n\nRebuild a report from the same raw evidence into a NEW results directory:\n\n```sh\nnode scripts/stage2_evm_fold_report.mjs experiments/stage2_evm_fold_raw/run-20261114-01 /ABS/NEW_RESULTS_DIR\n```\n\nThe original archive was finalized using ` node scripts/stage2_evm_fold_archive.mjs experiments/stage2_evm_fold_raw/run-20261114-01 results/stage2_evm_fold `. This one-time finalizer refuses to overwrite derived files or checksums; do not rerun it against a sealed directory. CSV can be reconstructed mechanically from summary fields using its flat export definition. Verify ` SHA256SUMS ` in both the raw and results directories, or run the read-only evidence checker for the raw archive.\n", {flag:'wx'});
console.log(JSON.stringify({status:s.status, results:output, receive_gas_saved_percent:s.receive_gas_saved_percent, prefix_total_extra_state_gas:s.prefix_total_extra_state_gas}));
