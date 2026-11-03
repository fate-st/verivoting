// G9 route decision only: inspect pinned evidence/source, never compile or run EVM.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
process.chdir(path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..'));
const mode = process.argv[2] ?? '--check'; assert.ok(['--write', '--check'].includes(mode));
const baseline = '4421dd9be2487195db07d91085d941b3aa51a8f6';
const jsonPath = 'results/g9/route_review.json', docPath = 'docs/G9_EXACT_VERIFICATION_ROUTE.md';
const read = p => fs.readFileSync(p), load = p => JSON.parse(read(p));
const hash = b => createHash('sha256').update(b).digest('hex');
const wire = h => Buffer.from(h.replace(/^0x/, ''), 'hex');
const pin = p => ({path: p, sha256: hash(read(p))});
const b23Path = 'experiments/g5_c3_raw/miller_quarter_full_evm.json';
const c2Path = 'experiments/g5_c2_raw/transcript_conformance.json';
const endpointPath = 'experiments/g5_c3_raw/tipp_endpoint_route.json';
const frozenProofPath = 'fixtures/snarkpack_g5c1_public_v1.json';
const w32Path = 'fixtures/snarkpack_g1d0_depth5_public_v1.json';
const b23 = load(b23Path), c2 = load(c2Path), endpoint = load(endpointPath);
const frozenProof = load(frozenProofPath), w32 = load(w32Path);
const g8 = load('fixtures/g8_receiver_eip2537_v1.json');
const g8e = load('results/g8_e/lifecycle_costs.json');
assert.equal(b23.status, 'BLOCKED_QUARTER_SOFTWARE_MILLER_EXCEEDS_20M_FROZEN');
assert.equal(b23.result_class, 'OOG'); assert.equal(b23.full_attempt.exception, 'out of gas');
assert.equal(b23.full_attempt.total_gas, b23.transaction_gas_limit);
assert.equal(b23.full_miller_loop_completed, false); assert.equal(b23.full_miller_verification_gas, null);
assert.equal(b23.software_route_frozen_by_failure, true);
for (const [p, h] of [[b23.contract_source, b23.contract_source_sha256], [b23.fixture, b23.fixture_sha256],
  [frozenProofPath, b23.source_fixture_sha256], [b23.endpoint_fixture, b23.endpoint_fixture_sha256],
  ['contracts/package-lock.json', b23.package_lock_sha256]]) assert.equal(hash(read(p)), h, p);
const historicalCargo = execFileSync('git', ['show', `${b23.git_commit}:Cargo.lock`]);
const currentCargo = read('Cargo.lock');
assert.equal(hash(historicalCargo), b23.cargo_lock_sha256);
assert.equal(hash(currentCargo), hash(execFileSync('git', ['show', `${baseline}:Cargo.lock`])));
function packageIdentities(data) {
  return data.toString().split('\n[[package]]\n').slice(1).map(block =>
    block.split('\n').filter(line => /^(name|version|source|checksum) = /.test(line)).join('\n')).sort();
}
assert.deepEqual(packageIdentities(currentCargo), packageIdentities(historicalCargo), 'external package identities must remain pinned');
const lockLineage = {historical_git_commit: b23.git_commit, historical_cargo_sha256: b23.cargo_lock_sha256,
  current_baseline_commit: baseline, current_cargo_sha256: hash(currentCargo),
  whole_lock_identical: hash(currentCargo) === hash(historicalCargo), package_identity_tables_equal: true,
  interpretation: 'Git diff adds ark-relations and sha2 dependency edges to vv-friendly-dkg only; no external package version/identity change; neither lock is rewritten'};
assert.equal(hash(wire(frozenProof.aggregate_proof_hex)), b23.aggregate_proof_sha256);
assert.equal(c2.fixture_sha256, b23.source_fixture_sha256); assert.equal(c2.matched_call_count, 12);
assert.ok(c2.rust_js_preimages_match && c2.rust_js_challenges_match && c2.rust_js_rejection_counters_match);
assert.equal(c2.exact_snarkpack_evm_verified, false);
assert.ok(endpoint.native_endpoint_verified && endpoint.independent_js_endpoint_verified);
assert.equal(endpoint.pairing_to_gt_evm_verified, false);
assert.equal(g8e.status, 'PASS_DERIVED_G8D_LIFECYCLE_ACCOUNTING_NO_NEW_EVM');

// Preserve all G8-E historical bytes. Only central status files may evolve;
// their saved commit versions are checked instead of rewriting the old manifest.
const maintained = new Set(['README.md', 'docs/IMPLEMENTATION_STATUS.md', 'docs/NEXT_STEPS.md',
  'scripts/summarize.py', 'experiments/aggregate/summary.json', 'experiments/aggregate/summary.md']);
let immutableEntries = 0, committedStatusEntries = 0;
for (const line of read('results/g8_e/SHA256SUMS').toString().trim().split('\n')) {
  const [, h, p] = line.match(/^([0-9a-f]{64})\s+(.+)$/) ?? []; assert.ok(p);
  const data = maintained.has(p) ? execFileSync('git', ['show', `${baseline}:${p}`], {maxBuffer: 16 * 1024 * 1024}) : read(p);
  assert.equal(hash(data), h, p); if (maintained.has(p)) committedStatusEntries++; else immutableEntries++;
}

const registry = path.join(process.env.CARGO_HOME ?? path.join(os.homedir(), '.cargo'), 'registry/src');
const crate = fs.readdirSync(registry).map(p => path.join(registry, p, 'bellperson-0.27.0'))
  .find(p => fs.existsSync(path.join(p, 'src/groth16/aggregate/verify.rs')));
assert.ok(crate, 'use the already locked bellperson 0.27.0 source; do not install/upgrade dependencies');
const official = Object.fromEntries(['verify.rs', 'transcript.rs', 'accumulator.rs'].map(name => {
  const data = fs.readFileSync(path.join(crate, 'src/groth16/aggregate', name));
  return [name, {package_path: `bellperson-0.27.0/src/groth16/aggregate/${name}`, sha256: hash(data), text: data.toString()}];
}));
assert.equal(official['verify.rs'].sha256, c2.reference_verifier_sha256);
assert.equal(official['transcript.rs'].sha256, c2.reference_transcript_sha256);
function at(file, anchor) {
  const s = official[file].text, i = s.indexOf(anchor); assert.ok(i >= 0, anchor);
  return {package_path: official[file].package_path, line: s.slice(0, i).split('\n').length, anchor};
}
const requirements = [
  {id: 'PARSE', check: 'Original V2 proof size/count and canonical point/GT parsing, subgroup and SRS/VK binding',
    evidence: at('verify.rs', 'proof.parsing_check()?;'), evm_status: 'MISSING_COMPLETE_PROOF_PARSER'},
  {id: 'FS', check: 'Original bincode/GT transcript encoding, SHA-256 cumulative rejection counters and all V2 challenges',
    evidence: at('transcript.rs', 'pub fn into_challenge'), evm_status: 'RUST_JS_CONFORMANCE_ONLY'},
  {id: 'GIPA', check: 'All GIPA folding of group/GT/scalar terms and extra-link challenge',
    evidence: at('verify.rs', 'fn gipa_verify_tipp_mipp'), evm_status: 'NOT_IMPLEMENTED'},
  {id: 'KZG_V', check: 'Commitment-key v opening verification', evidence: at('verify.rs', 'fn verify_kzg_v'), evm_status: 'NOT_IMPLEMENTED'},
  {id: 'KZG_W', check: 'Commitment-key w opening and original r-inverse shift', evidence: at('verify.rs', 'fn verify_kzg_w'), evm_status: 'NOT_IMPLEMENTED'},
  {id: 'TIPP', check: 'All z/tab/uab pairing-to-GT equalities, not just the one archived endpoint',
    evidence: at('verify.rs', 'let _check_z ='), evm_status: 'ONE_ENDPOINT_NATIVE_JS_ONLY'},
  {id: 'MIPP', check: 'tc/uc pairings plus final_c scalar/group relation to zc',
    evidence: at('verify.rs', 'let final_z ='), evm_status: 'NOT_IMPLEMENTED'},
  {id: 'AGGREGATE', check: 'Final Groth16 aggregate equation binds every ordered public-input row',
    evidence: at('verify.rs', 'pairing_checks_copy.merge_nonrandom('), evm_status: 'NOT_IMPLEMENTED'},
  {id: 'ACCUMULATOR', check: 'Sound complete equation checking; no fixed native harness RNG on chain',
    evidence: at('accumulator.rs', 'pub fn merge_miller_inputs'), evm_status: 'STRATEGY_UNRESOLVED'},
  {id: 'FINAL_GT', check: 'Complete Miller loops, final exponentiation and exact GT target equality',
    evidence: at('accumulator.rs', 'let left = self.left.final_exponentiation();'), evm_status: 'SOFTWARE_ROUTE_FROZEN_OOG'},
];
const iface = read('contracts/src/interfaces/IBatchVerifier.sol').toString();
const receiver = read('contracts/src/VeriVotingAuthenticatedSettlement.sol').toString();
assert.match(iface, /function verifyBatch\(bytes\[\] calldata proofs, uint256\[\]\[\] calldata inputs\) external view/);
assert.ok(receiver.includes('batchVerifier = new IndividualGroth16BatchVerifier(ballotVk)'));
assert.ok(receiver.indexOf('batchVerifier.verifyBatch(') < receiver.indexOf('spentNullifier[nullifiers[i]] = true'));

const b2 = w32.cases.find(c => c.batch_size === 2); assert.ok(b2);
assert.equal(hash(wire(b2.aggregate_proof_hex)), b2.aggregate_proof_sha256);
assert.equal(w32.groth16_vk_ark_compressed_sha256, g8.ballot_vk_ark_sha256);
assert.equal(hash(wire(w32.groth16_vk_hex)), b2.groth16_vk_sha256);
assert.equal(w32.merkle_depth, 5); assert.equal(w32.registered_weight_bound, 32);
assert.equal(b2.verifier_srs.n, 2); assert.equal(b2.public_inputs_le_hex.length, 2);
const asBE = h => '0x' + Buffer.from(h.replace(/^0x/, ''), 'hex').reverse().toString('hex');
assert.deepEqual(b2.public_inputs_le_hex.map(row => row.map(asBE)), g8.batches[0].ballot_inputs_be_hex.slice(0, 2));
const sourcePaths = [b23Path, c2Path, endpointPath, frozenProofPath, b23.fixture, b23.endpoint_fixture, b23.contract_source,
  w32Path, 'fixtures/g8_receiver_eip2537_v1.json', 'results/g8_e/lifecycle_costs.json', 'results/g8_e/SHA256SUMS',
  'contracts/src/interfaces/IBatchVerifier.sol', 'contracts/src/VeriVotingAuthenticatedSettlement.sol',
  'contracts/src/VeriVotingWinnerOnlySettlement.sol', 'contracts/src/IndividualGroth16BatchVerifier.sol',
  'Cargo.lock', 'contracts/package-lock.json', 'configs/protocol_manifest.JUBJUB-EVAL-D5-W32.json'];
const routes = [
  {id: 'SINGLE_TX_SOFTWARE', decision: 'KEEP_FROZEN',
    rationale: 'The saved full64 positive call failed at the fixed 20M research cap; final exponentiation and complete verification remain absent. No repeat or primitive tuning.'},
  {id: 'STAGED_EXACT', decision: 'PREFERRED_FOR_FURTHER_DESIGN_ONLY', feasibility: 'UNPROVEN',
    rationale: 'Can investigate retaining the original proof equations across contract-computed continuation states; per-step fit, storage and total cost have NOT been measured.',
    drop_in_replacement_for_view_interface: false},
  {id: 'WRAPPER_PROOF', decision: 'DEFER_IMPLEMENTATION', feasibility: 'UNPROVEN',
    rationale: 'Would need a full constrained verifier relation, original transcript/GT operations, sound statement binding, new proof/setup assumptions and independent feasibility work. No existing complete wrapper is claimed.'},
];
const proposedStages = [
  {stage: 'BEGIN', responsibility: 'The authorized receiver binds the original ordered matrix/proof/VK/SRS and current batch/old/new state to a separate session envelope. No spent NF or CT write yet.'},
  {stage: 'ADVANCE', responsibility: 'The EVM computes each transition from stored authenticated state, checks exact next stage and proof binding; caller-provided intermediates are not trusted. No batch acceptance.'},
  {stage: 'FINISH', responsibility: 'Only after ALL listed proof checks succeed mark one session verified. Never infer success from a component, prefix, host flag or uploaded target.'},
  {stage: 'CONSUME', responsibility: 'Only the fixed receiver rechecks current index/old CT/root/NFs and exact batch matrix; verifies the existing state proof and atomically consumes once with CT/NF updates. No caller-switchable backend.'},
];
const review = {schema_version: 1, gate: 'G9_ROUTE_REVIEW', status: 'PASS_ROUTE_REVIEW_ONLY_IMPLEMENTATION_NOT_STARTED',
  artifact_kind: 'source_and_archived_evidence_review_not_new_experiment', reviewed_client_date: '2026-11-02',
  baseline_commit: baseline, generator: pin('scripts/g9_route_review.mjs'), source_pins: sourcePaths.map(pin),
  pinned_reference_sources: Object.values(official).map(({text, ...meta}) => meta), lock_lineage: lockLineage,
  standard_reference: {url: 'https://eips.ethereum.org/EIPS/eip-2537#abi-for-pairing-check', reviewed_client_date: '2026-11-02',
    checked_by_agent_against_primary_page: true, fetched_by_this_script: false,
    semantics: 'G1/G2 pair input; Boolean product-equals-one output; no arbitrary GT target slot or GT result'},
  historical_observations_not_rerun: {full64_result: b23.result_class, capped_failed_spend: b23.full_attempt.total_gas,
    completed_miller_gas: null, completed_snarkpack_gas: null, rust_js_challenges_matched: c2.matched_call_count,
    solidity_transcript_verified: false, software_route_frozen: true},
  routes, decision: {preferred_candidate: 'STAGED_EXACT', selection_scope: 'DESIGN_ONLY_NOT_PROVEN_FEASIBLE',
    full_b2_prototype_ready: false, implement_or_launch_now: false,
    blockers: ['canonical wire/GT and complete Solidity transcript', 'all GT/TIPP/MIPP/KZG/aggregate relations',
      'final exponentiation and sound accumulator policy', 'bounded continuation schedule and total resource cap',
      'new asynchronous receiver protocol and one-time result consumption']},
  complete_verifier_checklist: requirements,
  receiver_integration_plan: {existing_contracts_changed: false, proposed_new_version_required: true,
    current_interface_is_synchronous_view: true, session_envelope_must_not_change_original_fiat_shamir: true,
    session_binding_fields: ['chain and contract addresses', 'suite/version', 'VK and verifier SRS identity', 'proof bytes hash',
      'ordered matrix hash and B', 'election/context/root', 'batch index', 'expected old/new CT state'],
    progression: proposedStages, incomplete_session_cannot_write_spent_nf_or_ct: true,
    overlapping_session_policy: 'initial minimal design permits only one pending next batch; any abort leaves accepted business state unchanged',
    current_b4_receiver_accepts_b2: false},
  material_selection: {historical_gt_component_fixture: frozenProofPath, historical_gt_proof_sha256: frozenProof.aggregate_proof_sha256,
    future_whole_b2_fixture: w32Path, future_whole_b2_case: 2, future_b2_proof_sha256: b2.aggregate_proof_sha256,
    future_b2_shared_research_srs_sha256: w32.shared_aggregation_srs_sha256,
    future_b2_bellperson_vk_sha256: b2.groth16_vk_sha256, future_b2_ark_vk_sha256: w32.groth16_vk_ark_compressed_sha256,
    future_b2_50_fields_match_g8_first_two_rows: true,
    newly_derived_w32_transcript_gt_vectors_needed: true,
    warning: 'Old G5-C1 GT/challenge vectors belong to their old proof; never splice them into the W32 aggregate. Matching W32 rows/VK does not make a B2 session accepted by the fixed B4 receiver.'},
  resource_budget: {current_phase_evm_transactions: 0, current_phase_compilations: 0, current_phase_setup_or_prove: 0,
    future_local_profile: 'Prague research profile, not a statement about live-chain gas limits',
    future_per_transaction_gas_cap: b23.transaction_gas_limit, future_block_context_gas_cap: b23.block_gas_limit,
    future_total_verification_transaction_cap: null, future_total_gas: null,
    launch_requires_explicit_total_transaction_and_execution_time_budget: true, no_automatic_budget_increase: true},
  future_minimal_acceptance: {authorized_this_phase: false, full_relation_cases: 2,
    positive: 'Reuse the archived W32 B2 aggregate and recompute every original check; exact verifier accepts.',
    negative: 'Change only second-row input[7], keep aggregate/VK/SRS unchanged, rebuild external transcript and resulting statement bindings; exact verifier rejects, not merely a stale session-ID/root mismatch.',
    metrics: ['per-tx receipt Gas and summed Gas', 'verification tx count', 'calldata and storage bytes', 'complete relation coverage'],
    component_only_result_must_not_upgrade_exact_verifier: true},
  new_execution_counts: {evm_transactions: 0, compiler_runs: 0, setup: 0, prove: 0, cp_generation: 0, signatures: 0},
  exact_snarkpack_evm_verified: false, target_verifier_status: 'BLOCKED_COMPLETE_EXACT_EVM_IMPLEMENTATION_AND_RESOURCE_FEASIBILITY_UNPROVEN',
  snarkpack_verification_gas: null, software_route_reopened: false, protocol_manifest_changed: false,
  prior_g8e_checks: {immutable_entries: immutableEntries, committed_status_entries: committedStatusEntries},
  stop: 'Stop after this review. No contract implementation, compiler, VM, proof generation or automatic follow-on trial.'};
const json = JSON.stringify(review, null, 2) + '\n';
const checklist = requirements.map(r => `| ${r.id} | ${r.check} | ${r.evm_status} |`).join('\n');
const doc = `# G9：精确 SnarkPack EVM 验证路线决策\n\n` +
  `## 结论：路线审查完成，不是验证器通过\n\n` +
  `日期：2026-11-02（Asia/Shanghai）。G8-E先保存为本地提交 \`${baseline}\`。\n` +
  `本轮只读核对既有源码/失败证据并归档设计，没有新EVM交易、编译、Setup/Prove、CP或签名。\n` +
  `有条件选择**多交易分阶段精确验证**作为后续设计候选；资源可行性未证明，不批准直接启动完整B2原型。\n` +
  `原单交易软件Miller路线继续冻结；wrapper只做候选比较，不开发、不换套件。当前完整精确验证仍BLOCKED，Gas为null。\n\n` +
  `## 已有证据及真正缺口\n\n` +
  `- [B23](../${b23Path})：一次full64在20M预算下OOG，支出20,000,000 Gas；它是失败封顶支出，不是完整Miller完成成本。最终幂和完整聚合验证未完成。\n` +
  `- [C2](../${c2Path})：原生/JS的12次挑战一致，只能证明该跨语言检查；不能说Solidity已经重算挑战。\n` +
  `- [TIPP终点](../${endpointPath})：一条终点等式原生/JS一致，不代表全部TIPP/MIPP/KZG关系完成。\n` +
  `- [EIP-2537官方ABI](https://eips.ethereum.org/EIPS/eip-2537#abi-for-pairing-check)仅接受G1/G2配对并返回乘积是否等于1，无任意GT目标槽或GT返回值。本轮已查阅官方页面，脚本不联网。\n\n` +
  `核对已安装且Cargo.lock锁定的bellperson 0.27.0：verify.rs/transcript.rs与C2原始SHA一致，accumulator.rs也记录独立SHA。\n` +
  `旧实验Cargo.lock与当前整文件不同：Git diff仅在vv-friendly-dkg增加ark-relations/sha2工作区依赖边；外部包身份表一致。分别核对旧Git提交的锁和当前基线锁，不要求二者整文件相等。\n` +
  `以下为原验证关系的完整实现清单，不是把一条Miller等式当整个验证器：\n\n` +
  `| 项目 | 必须保持的检查 | 当前EVM边界 |\n| --- | --- | --- |\n${checklist}\n\n` +
  `所有清单项在JSON中定位到固定包文件/行/anchor；脚本核对源码片段存在，但这不代替形式化等价性证明。\n` +
  `PairingChecks原实现使用随机系数合并检查；不能把功能测试的固定ChaCha20种子照搬链上。\n` +
  `逐条验证全部等式或采用另经论证的合并策略需先说明正确性/健全性，当前保持未解决，不能悄悄替换。\n\n` +
  `## 三条路线的决定\n\n` +
  `| 路线 | 决定 | 原因 |\n| --- | --- | --- |\n` +
  `| 原单交易软件 | 保持冻结 | 已有预算失败；不提高cap、不重复full或扫描算术 |\n` +
  `| 多交易分阶段精确验证 | 仅选为设计候选 | 可能分摊每笔计算，但仍缺GT/最终幂/完整关系及状态存储测量；不预言收益 |\n` +
  `| 额外证明封装 | 暂不实施 | 必须约束完整原验证关系、设置及声明绑定；新增证明假设/电路成本尚无证据 |\n\n` +
  `## 接收器不能直接替换：拟议协议边界\n\n` +
  `现有IBatchVerifier.verifyBatch是同步view；Receiver构造时创建固定B4逐票验证器，并在验证后原子写入NF/CT。\n` +
  `分阶段执行是有状态的异步协议，不能简单把它实现成当前view接口，也不修改旧G8-D合约。未来需要独立新版本。\n\n` +
  `1. BEGIN：由指定Receiver绑定proof、顺序矩阵、B、VK/SRS、选举上下文、batch index、old/new CT，建立session。\n` +
  `2. ADVANCE：合约从已认证存储推进固定阶段，实际计算中间值；只传session和受约束数据，不信任host预计算。\n` +
  `3. FINISH：完整清单全部通过才标记verified，任何组件/前缀通过均不授权收票。\n` +
  `4. CONSUME：唯一Receiver重检当前批次/旧CT/根/NF，对同一矩阵执行原状态证明检查，原子一次消费并更新NF/CT。\n\n` +
  `最小设计仅允许一个待验证next batch，未完成/中止不得改变已接受NF/CT；未实现重入、多并发或故障压力矩阵。\n` +
  `session envelope可包含chain/address等防混用信息，但不能擅自改动原SnarkPack外部/内部Fiat–Shamir域或编码。\n\n` +
  `## 已有B2材料的选择：不重新生成证明\n\n` +
  `旧G5-C1 proof \`${frozenProof.aggregate_proof_sha256}\` 仅作为历史GT组件来源，旧挑战/目标不可用于别的proof。\n` +
  `找到[W32 B2归档](../${w32Path})的batch_size=2 case：\n\n` +
  `- aggregate SHA-256：\`${b2.aggregate_proof_sha256}\`。\n` +
  `- 两行共50个字段转成同一BE表示后，与G8第一批的前两行逐项一致。\n` +
  `- Ark VK摘要与G8 ballot_vk_ark_sha256相同：\`${w32.groth16_vk_ark_compressed_sha256}\`。\n` +
  `- 共享研究SRS：\`${w32.shared_aggregation_srs_sha256}\`；固定该case的n=2 verifier SRS。\n\n` +
  `未来完整B2验证优先复用该材料，但需为这份proof派生/核对自己的挑战/GT向量。当前未重新执行原生完整验证。\n` +
  `B2是独立组件验收，不是现有B4接收器的收票用例；将来接入固定两批B4须用各自真实B4聚合及对应绑定，不能拼接或冒充。\n\n` +
  `## 资源预算与下一次最小验收\n\n` +
  `本轮预算：0 EVM交易、0编译、0证明；只做来源及设计核对。\n` +
  `后续候选保持本地Prague每笔20M、block上下文30M研究配置；不说它们是当前公链永久限制。\n` +
  `总交易数/总Gas/运行时间尚无可信调度依据，继续为null；**必须先确定完整操作/续算清单和总交易及时间上限，才允许原型执行**，不得自动扩预算。\n\n` +
  `只有这些先决条件满足且另行授权后，最小完整验证只做两例：\n\n` +
  `- 正例：原W32 B2 proof、VK、SRS，原声明全部真实验证通过。\n` +
  `- 反例：只改第二行input[7]的nullifier，原proof/VK/SRS不变，重建外部transcript及相应session绑定，让完整验证拒绝；不能用过期session-ID/root不匹配冒充证明拒绝。\n\n` +
  `必须实测每笔和总Gas、交易数、calldata、存储与完整关系覆盖。若只完成GT或其他组件，只能报告组件，不能提升exact标记。\n` +
  `若关键关系/预算不能闭合，则停止并保留BLOCKED，报告已完成的逐票备用路线；不因此自动修改论文或协议。\n\n` +
  `## 复核与产物\n\n` +
  '```sh\nnode scripts/g9_route_review.mjs --write\nnode scripts/g9_route_review.mjs --check\nnode scripts/g8e_accounting.mjs --check\n```\n\n' +
  `脚本不运行VM、不编译、不生成新GT/挑战向量；--check重算并逐字节核对JSON和本报告。G8-E清单${immutableEntries}项现值及${committedStatusEntries}项已提交状态字节通过。\n` +
  `首次脚本将旧Cargo锁哈希直接与当前文件比较而退出；确认上述合法工作区依赖变更后改为分别核对历史/当前锁及包身份。失败日志和原脚本保留，没有修改任何锁或旧raw。\n` +
  `决策JSON：[route_review.json](../${jsonPath})，SHA-256：\`${hash(Buffer.from(json))}\`。\n\n` +
  `到本轮路线审查停止。G9完整精确EVM仍未实现，G10公平系统比较仍后置，禁止以审查PASS冒充验证器PASS。\n`;
for (const [p, content] of [[jsonPath, json], [docPath, doc]]) {
  if (mode === '--check' || fs.existsSync(p)) assert.equal(read(p).toString(), content, `deterministic output mismatch: ${p}`);
  else { fs.mkdirSync(path.dirname(p), {recursive: true}); fs.writeFileSync(p, content, {flag: 'wx'}); }
}
console.log(JSON.stringify({status: review.status, mode, derived_json_sha256: hash(Buffer.from(json)),
  source_files: sourcePaths.length, reference_files: Object.keys(official).length, required_verifier_checks: requirements.length,
  preferred_candidate: review.decision.preferred_candidate, feasibility: 'UNPROVEN', new_evm_transactions: 0,
  exact_snarkpack_evm_verified: false, snarkpack_verification_gas: null, software_route_reopened: false,
  w32_b2_matrix_fields_matched: 50, prototype_ready: false}));
