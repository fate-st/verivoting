// G8-E: deterministic accounting derived from the ONE archived G8-D execution.
// No VM, RPC, compiler, setup, proving, signing, or synthetic performance samples.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {createRequire} from 'node:module';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
process.chdir(root);
const {Interface, Transaction, AbiCoder} = createRequire(new URL('../contracts/package.json', import.meta.url))('ethers');
const mode = process.argv[2] ?? '--check';
assert.ok(['--write', '--check'].includes(mode), 'use --write or --check');
const baseline = '4a2cc163cd8b0d805d4a582c6e0d91586fc2627e';
const sourcePath = 'experiments/g8d_raw/winner_once.json';
const jsonPath = 'results/g8_e/lifecycle_costs.json';
const docPath = 'docs/G8_E_LIFECYCLE_COSTS.md';
const read = p => fs.readFileSync(p);
const load = p => JSON.parse(read(p));
const hash = b => createHash('sha256').update(b).digest('hex');
const bytes = h => Buffer.from(h.replace(/^0x/, ''), 'hex');
const pin = p => ({path: p, sha256: hash(read(p))});
const r = load(sourcePath), art = load(r.contract_artifact);
assert.equal(hash(read(sourcePath)), '20b72b4f23a66dcd8ef06e30c62494ad8f989d51107fb3c64876a9b1c1ff3268');
assert.equal(r.status, 'PASS_FIXED_FROZEN_P3_ATOMIC_WINNER_WRITE_ONCE');
assert.equal(hash(read(r.contract_artifact)), r.contract_artifact_sha256);
for (const p of [...r.source_pins, ...r.source_hashes]) assert.equal(hash(read(p.path)), p.sha256, p.path);

// Six central status files intentionally evolve; verify their archived baseline
// versions instead. Every other G8-D manifest entry must still match live bytes.
const maintained = new Set(['README.md', 'docs/IMPLEMENTATION_STATUS.md', 'docs/NEXT_STEPS.md',
  'scripts/summarize.py', 'experiments/aggregate/summary.json', 'experiments/aggregate/summary.md']);
let liveManifestEntries = 0, baselineManifestEntries = 0;
for (const line of read('experiments/g8d_raw/SHA256SUMS').toString().trim().split('\n')) {
  const [, expected, p] = line.match(/^([0-9a-f]{64})\s+(.+)$/) ?? [];
  assert.ok(p);
  const content = maintained.has(p) ? execFileSync('git', ['show', `${baseline}:${p}`], {maxBuffer: 16 * 1024 * 1024}) : read(p);
  assert.equal(hash(content), expected, p);
  if (maintained.has(p)) baselineManifestEntries++; else liveManifestEntries++;
}
assert.equal(liveManifestEntries + baselineManifestEntries, 69);

const receiverAbi = new Interface(art.contracts.winner_receiver.abi);
const storeAbi = new Interface(art.contracts.original_commitment_store.abi);
const p3Abi = new Interface(art.contracts.frozen_p3_binder.abi);
const phaseLabels = {
  deployment: ['deploy_certificate_gate_v2', 'deploy_winner_receiver', 'deploy_original_commitment_store', 'deploy_frozen_p3_binder'],
  initialization: ['bind_fixed_p3_once'],
  certification: ['authenticate_dkg_1', 'authenticate_dkg_2', 'authenticate_dkg_3', 'authenticate_registration'],
  intake: ['accept_batch_1', 'accept_batch_2'],
  freeze: ['freeze_ct2'],
  commitments: ['bind_authenticated_frozen_ct2', 'authenticate_commitment_1', 'authenticate_commitment_2', 'authenticate_commitment_3'],
  disclosure: ['disclose_winner_once'],
  rejected: ['repeat_p3_binding', 'disclose_before_freeze', 'wrong_winner_before_disclosure', 'repeat_winner_disclosure'],
};
const classification = new Map();
for (const [phase, labels] of Object.entries(phaseLabels)) for (const label of labels) {
  assert.ok(!classification.has(label)); classification.set(label, phase);
}
assert.equal(classification.size, r.transaction_count);
assert.deepEqual(r.transactions.map(t => t.nonce), Array.from({length: 21}, (_, i) => i));
assert.equal(new Set(r.transactions.map(t => t.transaction_hash)).size, 21);
const rows = r.transactions.map((t, sourceIndex) => {
  assert.ok(classification.has(t.label), t.label);
  const phase = classification.get(t.label), signed = Transaction.from(t.signed_transaction_hex);
  assert.ok(signed.isSigned()); assert.equal(signed.hash, t.transaction_hash);
  assert.equal(signed.data, t.calldata_hex); assert.equal(signed.nonce, t.nonce);
  assert.equal(signed.to?.toLowerCase() ?? null, t.to);
  assert.equal(bytes(signed.data).length, t.calldata_bytes);
  assert.equal(hash(bytes(signed.data)), t.calldata_sha256);
  assert.equal(t.receipt_status, phase === 'rejected' ? 0 : 1);
  assert.equal(t.gas_refund, 0);
  const floorExtra = t.total_gas - t.intrinsic_gas - t.execution_gas;
  assert.equal(t.total_gas, Math.max(t.intrinsic_gas + t.execution_gas, t.minimum_gas_limit_including_calldata_floor));
  assert.ok(floorExtra >= 0);
  return {source_index: sourceIndex, nonce: t.nonce, label: t.label, phase,
    transaction_hash: t.transaction_hash, receipt_status: t.receipt_status, to: t.to,
    total_gas: t.total_gas, intrinsic_gas: t.intrinsic_gas, execution_gas: t.execution_gas,
    calldata_floor_extra_gas: floorExtra, transaction_data_bytes: t.calldata_bytes,
    creation_input_bytes: t.to === null ? t.calldata_bytes : 0,
    contract_calldata_bytes: t.to === null ? 0 : t.calldata_bytes,
    signed_transaction_bytes: bytes(t.signed_transaction_hex).length, revert_reason: t.revert_reason};
});
const sumKeys = ['total_gas', 'intrinsic_gas', 'execution_gas', 'calldata_floor_extra_gas',
  'transaction_data_bytes', 'creation_input_bytes', 'contract_calldata_bytes', 'signed_transaction_bytes'];
function total(selected) {
  return {transaction_count: selected.length,
    ...Object.fromEntries(sumKeys.map(k => [k, selected.reduce((n, t) => n + t[k], 0)]))};
}
const phases = Object.fromEntries(Object.keys(phaseLabels).map(p => [p, total(rows.filter(t => t.phase === p))]));
const successful = rows.filter(t => t.receipt_status === 1);
const totals = {deployment: phases.deployment,
  non_deployment_success: total(successful.filter(t => t.phase !== 'deployment')),
  successful_including_deployment: total(successful), rejected_tests: phases.rejected, all_acceptance_transactions: total(rows)};
assert.equal(totals.successful_including_deployment.transaction_count, 17);
assert.equal(totals.non_deployment_success.transaction_count, 13);
assert.equal(totals.rejected_tests.transaction_count, 4);
for (const k of sumKeys) {
  assert.equal(totals.deployment[k] + totals.non_deployment_success[k], totals.successful_including_deployment[k]);
  assert.equal(totals.successful_including_deployment[k] + totals.rejected_tests[k], totals.all_acceptance_transactions[k]);
  assert.equal(Object.values(phases).reduce((n, p) => n + p[k], 0), totals.all_acceptance_transactions[k]);
}

// Decode only the SUCCESSFUL transactions. Count payloads as subsets, not extra
// communication added on top of their parent calldata/creation input.
const txBy = Object.fromEntries(r.transactions.map(t => [t.label, t]));
const proofBlobs = [], vkBlobs = [];
function material(list, kind, wire, source) {
  list.push({kind, sha256: hash(bytes(wire)), bytes: bytes(wire).length, source});
}
let callerMatrixBytes = 0;
for (const label of phaseLabels.intake) {
  const t = txBy[label], batch = receiverAbi.decodeFunctionData('submitBatch', t.calldata_hex)[0];
  assert.equal(t.to, r.addresses.winner_receiver);
  assert.equal(batch.ballotProofs.length, 4); assert.equal(batch.ballotInputs.length, 4);
  for (let i = 0; i < 4; i++) {
    assert.equal(batch.ballotInputs[i].length, 25);
    callerMatrixBytes += batch.ballotInputs[i].length * 32;
    material(proofBlobs, 'ballot_groth16', batch.ballotProofs[i], `${label}.ballotProofs[${i}]`);
  }
  material(proofBlobs, 'state_groth16', batch.stateProof, `${label}.stateProof`);
}
const disclosed = receiverAbi.decodeFunctionData('discloseWinner', txBy.disclose_winner_once.calldata_hex);
assert.equal(Number(disclosed[1]), r.selector); assert.equal(Number(disclosed[2]), r.final_checkpoint.winner);
assert.equal(hash(bytes(disclosed[0])), r.eip_proof_sha256);
material(proofBlobs, 'p3_groth16', disclosed[0], 'disclose_winner_once.proof');
function deployedArguments(name, label, iface) {
  const code = art.contracts[name].creation_bytecode_hex, tx = txBy[label];
  assert.ok(tx.calldata_hex.startsWith(code));
  return AbiCoder.defaultAbiCoder().decode(iface.fragments.find(f => f.type === 'constructor').inputs,
    `0x${tx.calldata_hex.slice(code.length)}`);
}
const receiverArgs = deployedArguments('winner_receiver', 'deploy_winner_receiver', receiverAbi);
const storeArgs = deployedArguments('original_commitment_store', 'deploy_original_commitment_store', storeAbi);
const p3Args = deployedArguments('frozen_p3_binder', 'deploy_frozen_p3_binder', p3Abi);
assert.equal(receiverArgs[5].toLowerCase(), r.addresses.frozen_p3_binder);
assert.equal(p3Args[1].toLowerCase(), r.addresses.winner_receiver);
assert.equal(storeArgs[1].toLowerCase(), r.addresses.winner_receiver);
assert.equal(p3Args[2].toLowerCase(), r.addresses.original_commitment_store);
material(vkBlobs, 'ballot_vk_eip2537', receiverArgs[3], 'deploy_winner_receiver.ballotVk');
material(vkBlobs, 'state_vk_eip2537', receiverArgs[4], 'deploy_winner_receiver.stateVk');
material(vkBlobs, 'p3_vk_eip2537', p3Args[3], 'deploy_frozen_p3_binder.eipVk');
assert.equal(vkBlobs[2].sha256, r.eip_vk_sha256);
function uniqueMaterials(list) {
  const unique = new Map();
  for (const m of list) {
    if (!unique.has(m.sha256)) unique.set(m.sha256, {...m, sources: [], kinds: []});
    const entry = unique.get(m.sha256); assert.equal(entry.bytes, m.bytes);
    entry.sources.push(m.source); if (!entry.kinds.includes(m.kind)) entry.kinds.push(m.kind);
  }
  return {occurrences: list.length, occurrence_bytes: list.reduce((n, m) => n + m.bytes, 0),
    unique_blob_count: unique.size, unique_bytes: [...unique.values()].reduce((n, m) => n + m.bytes, 0),
    blobs: [...unique.values()]};
}
assert.equal(proofBlobs.filter(m => m.kind === 'ballot_groth16').length, r.reused_ballot_proofs);
assert.equal(proofBlobs.filter(m => m.kind === 'state_groth16').length, r.reused_state_proofs);
assert.equal(proofBlobs.filter(m => m.kind === 'p3_groth16').length, r.reused_p3_proofs);
const proofs = uniqueMaterials(proofBlobs), keys = uniqueMaterials(vkBlobs);
const bodyRemainder = totals.non_deployment_success.contract_calldata_bytes - proofs.occurrence_bytes - callerMatrixBytes;
assert.ok(bodyRemainder >= 0);

const winnerLogs = r.transactions.flatMap(t => t.logs.filter(l => l.address === r.addresses.winner_receiver)
  .map(l => ({nonce: t.nonce, event: receiverAbi.parseLog(l)}))).filter(x => x.event.name === 'WinnerDisclosed');
assert.equal(winnerLogs.length, 1); assert.equal(winnerLogs[0].nonce, txBy.disclose_winner_once.nonce);
assert.equal(winnerLogs[0].event.args.electionId, r.initial_binding.electionId);
assert.equal(winnerLogs[0].event.args.freezeSeal, r.final_checkpoint.freeze_seal);
assert.equal(Number(winnerLogs[0].event.args.winner), 1);
assert.equal(r.successful_winner_writes, 1); assert.equal(r.final_checkpoint.winner, 1);
assert.ok(r.final_checkpoint.winner_disclosed && r.all_60_inputs_match && r.g6_input_match && r.g7_input_match);
assert.deepEqual(r.after_valid_disclosure, {...r.before_valid_disclosure, winner_disclosed: true, winner: 1, winner_selector: 0});
assert.deepEqual(r.final_checkpoint, r.after_valid_disclosure);
assert.equal(r.final_checkpoint.freeze_seal, r.frozen_checkpoint.freeze_seal);
assert.equal(r.final_checkpoint.cp_target, r.frozen_checkpoint.cp_target);
for (const n of r.negative_tests) {
  assert.equal(txBy[n.case].receipt_status, 0); assert.ok(n.atomic_rejection); assert.deepEqual(n.before, n.after);
}

const cost = {schema_version: 1, gate: 'G8-E', status: 'PASS_DERIVED_G8D_LIFECYCLE_ACCOUNTING_NO_NEW_EVM',
  artifact_kind: 'derived_accounting_not_new_raw_experiment',
  source: {...pin(sourcePath), recorded_at_utc: r.recorded_at_utc, execution_git_commit: r.git_commit,
    execution_git_dirty: r.git_dirty, execution_branch: r.branch, archived_baseline_commit: baseline},
  generator: pin('scripts/g8e_accounting.mjs'),
  supporting_sources: [pin(r.contract_artifact), pin('experiments/g8d_raw/transactions.jsonl'),
    pin('experiments/g8d_raw/SHA256SUMS'), pin('configs/experiment.G8C-FROZEN-P3-W32.json'), ...r.source_pins],
  provenance_checks: {g8d_manifest_live_entries: liveManifestEntries, g8d_manifest_baseline_status_entries: baselineManifestEntries,
    source_code_and_fixture_hashes_matched: r.source_hashes.length + r.source_pins.length},
  scope: {route: 'local Prague individual Groth16 ballot verification + state proofs + authenticated frozen P3 + Winner once',
    registered_voters: r.final_checkpoint.certified_n_registered, registered_weight: r.initial_binding.registeredWeight,
    accepted_ballots: r.final_checkpoint.spent_count, batches: phaseLabels.intake.length, batch_size: 4,
    selected_trustees: r.selected_trustees,
    accounting_basis: 'successful receipt subset of one interleaved acceptance execution, NOT a new success-only replay',
    environment_from_source: {cpu: r.cpu, platform: r.platform, arch: r.arch, node: r.node,
      compiler: r.compiler, compiler_settings: r.compiler_settings, hardfork: r.hardfork}},
  phases, totals, transactions: rows,
  communication: {semantics: 'creation input/calldata and signed serialization are alternative nested views, never added together',
    successful_business_calldata_partition: {proof_payload_bytes: proofs.occurrence_bytes,
      caller_ballot_public_input_matrix_bytes: callerMatrixBytes, other_fields_selectors_and_abi_bytes: bodyRemainder,
      total_bytes: totals.non_deployment_success.contract_calldata_bytes},
    public_binary_materials: {deduplication: 'exact decoded binary SHA-256, not filenames or JSON sizes', proofs, verification_keys: keys,
      semantics: 'subset already present in transaction inputs, NOT extra wire bytes or full off-chain protocol communication'},
    internally_reconstructed_public_inputs: {state: {statements: 2, fields_per_statement: 37, field_bytes: 32, additional_calldata_bytes: 0},
      p3: {statements: 1, fields_per_statement: r.public_input_count, field_bytes: 32, additional_calldata_bytes: 0}},
    private_trustee_transport: {status: 'NOT_MEASURED', bytes: null},
    peer_to_peer_framing_and_retransmission: {status: 'NOT_MEASURED', bytes: null}},
  offline_costs: {status: 'REUSED_NOT_RETIMED',
    reused_proofs: {ballot: r.reused_ballot_proofs, state: r.reused_state_proofs, p3: r.reused_p3_proofs},
    setup_time_ms: null, prove_time_ms: null, dkg_time_ms: null, private_cp_generation_time_ms: null,
    original_setup_or_prover_cost_is_zero: false},
  binding: {addresses: r.addresses, election_id: r.initial_binding.electionId, key_core_digest: r.initial_binding.keyCoreDigest,
    final_context: r.initial_binding.finalContext, freeze_seal: r.final_checkpoint.freeze_seal, cp_target: r.final_checkpoint.cp_target,
    ark_vk_sha256: r.ark_vk_sha256, eip_vk_sha256: r.eip_vk_sha256, ark_proof_sha256: r.ark_proof_sha256,
    eip_proof_sha256: r.eip_proof_sha256, all_60_inputs_match: r.all_60_inputs_match, winner: r.final_checkpoint.winner,
    successful_winner_writes: r.successful_winner_writes, winner_event_count: winnerLogs.length},
  new_execution_counts: {evm_transactions: 0, setup: 0, prove: 0, cp_generation: 0, signatures: 0},
  complete_snarkpack_chain_settlement: false, statistical_efficiency_claim: false, public_chain_deployment: false,
  complete_snarkpack_settlement_gas: null, full_end_to_end_costs: null,
  limitations: ['sums of individual receipts are not one transaction/block and not a statistical performance sample',
    'failed test transactions excluded from successful-path cost; failed packaging attempt not counted as another experiment sample',
    'deployment includes internal CREATE/verifier costs already; do not add precompile/child-contract gas again',
    'no ETH price, latency, full-tally baseline, production setup, private delivery, or exact SnarkPack EVM cost inferred',
    'new baseline commit preserves the old dirty execution provenance instead of relabeling it as a clean run']};
const json = JSON.stringify(cost, null, 2) + '\n';
const fmt = n => n.toLocaleString('en-US');
const phaseNames = {deployment: '部署', initialization: '初始化绑定', certification: 'DKG/注册证书认证',
  intake: '两批B4接收', freeze: '冻结CT2', commitments: '冻结目标绑定及三份承诺认证', disclosure: '赢家验证并写入', rejected: '拒绝测试（不计成功路径）'};
const table = Object.entries(phases).map(([p, t]) => `| ${phaseNames[p]} | ${t.transaction_count} | ${fmt(t.total_gas)} | ${fmt(t.transaction_data_bytes)} | ${fmt(t.signed_transaction_bytes)} |`).join('\n');
const report = `# G8-E：已有 G8-D 完整本地流程的成本归档\n\n` +
  `## 结论\n\nG8-E PASS，属于已有真实记录的派生统计，不是新EVM实验。无需补运行。\n` +
  `G8-D已先保存为本地提交 \`${baseline}\`；旧结果仍保留实际执行的 git_dirty=true、旧Git SHA及源码快照。\n` +
  `本次不改合约、电路、配置或旧raw，不重新Setup/Prove、签名或生成CP包。\n\n` +
  `唯一计费来源：[G8-D winner_once.json](../${sourcePath})。SHA-256：\`${cost.source.sha256}\`。\n` +
  `该来源的第二次完整验收含21笔：17成功、4正常拒绝；首次归档失败仍留在G8-D，未当成第二个性能样本。\n\n` +
  `## 同版本、同一次执行的分类结果\n\n` +
  `| 阶段 | 笔数 | Gas | transaction data B | 签名交易序列化 B |\n| --- | ---: | ---: | ---: | ---: |\n${table}\n\n` +
  `- 部署：${fmt(totals.deployment.total_gas)} Gas，4笔；含内部CREATE及子验证器，不重复加费用。\n` +
  `- 非部署成功业务：${fmt(totals.non_deployment_success.total_gas)} Gas，13笔，包含初始化/认证/承诺，不仅是投票和披露。\n` +
  `- 成功路径含部署：${fmt(totals.successful_including_deployment.total_gas)} Gas，17笔。\n` +
  `- 失败测试：${fmt(totals.rejected_tests.total_gas)} Gas，4笔，单列不加进成功成本。\n\n` +
  `这里是含负例的同一次验收中成功收据的分类汇总，不是另跑的一条无负例路径；也不是一笔交易或一个区块。\n` +
  `Gas采用实际receipt总Gas，已考虑calldata floor，不能只把intrinsic与execution相加，或再叠加配对轨迹Gas。\n` +
  `环境沿用来源：${r.cpu}、${r.node}、solc ${r.compiler}、Prague、optimizer200/viaIR。\n\n` +
  `## 通信量口径：嵌套项不能重复相加\n\n` +
  `成功部署创建输入共${fmt(totals.deployment.creation_input_bytes)} B；成功业务calldata共${fmt(totals.non_deployment_success.contract_calldata_bytes)} B。\n` +
  `成功路径transaction data共${fmt(totals.successful_including_deployment.transaction_data_bytes)} B，签名交易序列化共${fmt(totals.successful_including_deployment.signed_transaction_bytes)} B。\n` +
  `后者已经包含前者，不能相加；实际重复发送的payload按每笔计数，不作网络字节去重。\n\n` +
  `业务calldata的非重叠拆分：证明payload ${fmt(proofs.occurrence_bytes)} B + 单票公开输入矩阵 ${fmt(callerMatrixBytes)} B + 其他参数/selector/ABI ${fmt(bodyRemainder)} B = ${fmt(totals.non_deployment_success.contract_calldata_bytes)} B。\n` +
  `内部重建的2×37项状态输入和60项P3输入不是额外传入数组，不再各加一遍字段字节。\n\n` +
  `从成功交易ABI提取并按二进制SHA-256去重：${proofs.unique_blob_count}份证明共${fmt(proofs.unique_bytes)} B；${keys.unique_blob_count}份VK共${fmt(keys.unique_bytes)} B。\n` +
  `这些是已包含在交易输入内的公开二进制材料子集，不是另一笔额外通信，也不是完整离线协议通信。\n` +
  `Trustee私密通信、P2P封装/重传未测量，JSON以null/NOT_MEASURED记录，不填0；不把JSON文件大小叫最小协议通信。\n\n` +
  `## 离线密码学成本与状态绑定\n\n` +
  `复用8份单票、2份状态、1份P3证明；本轮Setup/Prove/新EVM交易/CP/签名执行数均为0。\n` +
  `原证明生成和DKG/私密CP成本并非0，而是REUSED_NOT_RETIMED，时间字段保留null。\n` +
  `沿用同一Receiver/Gate/Store、认证KeyCore、原G4-D VK、CT2 freeze seal及CP target；60项输入与G6/G7匹配。\n` +
  `原P3成功后Winner=1，写入及事件均一次；4个失败的业务状态原子不变。私有tally/opening/usable shares不输出。\n\n` +
  `## 必要检查与复现\n\n` +
  `脚本核对21笔签名/数据长度/收据，一一分类；逐笔、分组、总量守恒；成功与失败分离；ABI解码证明/矩阵/VK；赢家事件和冻结状态绑定。\n` +
  `G8-D清单${liveManifestEntries}项核对当前原字节，${baselineManifestEntries}个中央状态文件核对已提交G8-D版本，避免更新状态时篡改旧清单。\n\n` +
  '```sh\nnode scripts/g8e_accounting.mjs --write\nnode scripts/g8e_accounting.mjs --check\nnode scripts/g8d_audit.mjs\n```\n\n' +
  `--write仅新建或接受字节完全相同的派生产物；--check只读重算并逐字节核对JSON/本报告，不重新运行EVM。\n` +
  `派生JSON：[lifecycle_costs.json](../${jsonPath})，SHA-256：\`${hash(Buffer.from(json))}\`。\n\n` +
  `## 停止边界\n\n到G8-E停止。以上是固定D5/W32、两批B4的本地逐票Groth16备用路线成本，不是完整SnarkPack链上结算、完整系统离线+链上总成本或统计效率优势。\n` +
  `G9精确SnarkPack EVM尚未完成；G10公平基线/最终系统比较后置。本轮没有公链部署或新增图表/规模实验。\n`;
for (const [p, content] of [[jsonPath, json], [docPath, report]]) {
  if (mode === '--check' || fs.existsSync(p)) assert.equal(read(p).toString(), content, `deterministic output mismatch: ${p}`);
  else { fs.mkdirSync(path.dirname(p), {recursive: true}); fs.writeFileSync(p, content, {flag: 'wx'}); }
}
console.log(JSON.stringify({status: cost.status, mode, source_sha256: cost.source.sha256,
  derived_json_sha256: hash(Buffer.from(json)), classified_transactions: rows.length,
  successful_transactions: successful.length, rejected_transactions: phases.rejected.transaction_count,
  deployment_gas: totals.deployment.total_gas, non_deployment_success_gas: totals.non_deployment_success.total_gas,
  successful_including_deployment_gas: totals.successful_including_deployment.total_gas,
  rejected_test_gas: phases.rejected.total_gas, new_evm_transactions: 0, setup: 0, prove: 0}));
