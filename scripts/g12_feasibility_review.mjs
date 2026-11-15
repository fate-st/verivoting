// G12 is a source/archive review, NOT an aggregate verifier or a benchmark.
// --check never writes; --write creates only the new G12 report and artifacts.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';

process.chdir(path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..'));
const mode = process.argv[2] ?? '--check';
assert.ok(['--write', '--check'].includes(mode), 'usage: node scripts/g12_feasibility_review.mjs [--write|--check]');
assert.equal(process.argv.length <= 3, true);
const BASE = 'fff01791f51dc87e695523c53ed8e6f7e88baee8';
const STAGE2 = '1ade01f12bc95ab00a514a7986a984816fe1741b';
const STATUS = 'BLOCKED_G12_FEASIBILITY';
const read = p => fs.readFileSync(p);
const load = p => JSON.parse(read(p));
const sha = b => createHash('sha256').update(b).digest('hex');
const encode = j => JSON.stringify(j, null, 2) + '\n';
const wire = h => { assert.match(h, /^(?:0x)?(?:[a-fA-F0-9]{2})+$/); return Buffer.from(h.replace(/^0x/, ''), 'hex'); };
const git = args => execFileSync('git', args, {maxBuffer: 64 * 1024 * 1024});
const pin = p => ({path: p, sha256: sha(read(p))});

// Check the complete original tracked tree, not only the few displayed JSONs.
// Rolling indexes are intentionally updated separately; historical snapshots stay in Git.
// Do not require HEAD==BASE: this checker must still work after a future commit.
const rolling = new Set(['docs/NEXT_STEPS.md', 'docs/IMPLEMENTATION_STATUS.md']);
const historicalPath = 'experiments/joint_proof_raw/run-20261114-03/historical_files.json';
const previousWorkingHashes = new Map(load(historicalPath).map(e => [e.path, e.sha256]));
const tree = git(['ls-tree', '-r', '-z', BASE]).toString().split('\0').filter(Boolean);
let protectedFiles = 0, protectedBytes = 0;
const preexistingLineEndings = [];
for (const item of tree) {
  const m = /^(\d+) blob ([a-f0-9]{40})\t([\s\S]+)$/.exec(item);
  assert.ok(m, 'only ordinary Git blobs are expected in this research baseline');
  const [, fileMode, object, p] = m;
  assert.ok(['100644', '100755'].includes(fileMode), 'unexpected file mode: ' + p);
  if (rolling.has(p)) continue;
  const b = read(p);
  const actual = createHash('sha1').update(`blob ${b.length}\0`).update(b).digest('hex');
  if (actual !== object) {
    // core.autocrlf=input already normalized this one historical CSV in Git.
    // Preserve the original working bytes, and require BOTH prior SHA-256 and
    // normalized Git blob to match; do not normalize or rewrite the old CSV.
    const normalized = Buffer.from(b.toString().replaceAll('\r\n', '\n'));
    const normalizedBlob = createHash('sha1').update(`blob ${normalized.length}\0`).update(normalized).digest('hex');
    assert.equal(normalizedBlob, object, 'old tracked bytes changed: ' + p);
    assert.equal(sha(b), previousWorkingHashes.get(p), 'preexisting worktree bytes changed: ' + p);
    preexistingLineEndings.push({path: p, working_sha256: sha(b), normalized_git_sha256: sha(normalized),
      prior_working_sha_source: historicalPath, rule: 'Prior byte hash AND normalized baseline Git blob both match; file not rewritten.'});
  }
  protectedFiles++; protectedBytes += b.length;
}

const paths = {
  g9: 'results/g9/route_review.json', p1: 'results/g9_p1/design_inventory.json',
  p1v2: 'results/g9_p1_v2/feasibility_review.json',
  oog: 'experiments/g5_c3_raw/miller_quarter_full_evm.json',
  fs: 'experiments/g5_c2_raw/transcript_conformance.json',
  endpoint: 'experiments/g5_c3_raw/tipp_endpoint_route.json',
  b2: 'fixtures/snarkpack_g1d0_depth5_public_v1.json',
  g8: 'fixtures/g8_receiver_eip2537_v1.json',
  j1: 'experiments/joint_proof_raw/run-20261114-03/execution.json',
};
const prior = load(paths.p1v2), oog = load(paths.oog), transcript = load(paths.fs);
const endpoint = load(paths.endpoint), w32 = load(paths.b2), g8 = load(paths.g8), j1 = load(paths.j1);
assert.equal(prior.status, 'BLOCKED_NO_COMPLETE_GAS_AND_EQUIVALENCE_BOUND');
assert.equal(oog.result_class, 'OOG');
assert.equal(oog.full_attempt.exception, 'out of gas');
assert.equal(oog.full_attempt.total_gas, 20_000_000);
assert.equal(oog.full_miller_loop_completed, false);
assert.equal(oog.full_miller_verification_gas, null);
assert.equal(oog.exact_snarkpack_evm_verified, false);
assert.equal(transcript.matched_call_count, 12);
assert.ok(transcript.rust_js_preimages_match && transcript.rust_js_challenges_match && transcript.rust_js_rejection_counters_match);
assert.ok(endpoint.native_endpoint_verified && endpoint.independent_js_endpoint_verified);
assert.equal(endpoint.pairing_to_gt_evm_verified, false);
assert.equal(j1.status, 'BLOCKED_INNER_RECURSIVE_VERIFIER');
assert.equal(j1.native_gadget_available_over_fq, true);
assert.equal(j1.current_fr_backend_compatible, false);
assert.equal(j1.r1cs_synthesized, false);
assert.equal(j1.setup_count, 0); assert.equal(j1.proof_generation_count, 0);

const contractPaths = ['contracts/src/interfaces/IBatchVerifier.sol',
  'contracts/src/IndividualGroth16BatchVerifier.sol', 'contracts/src/VeriVotingAuthenticatedSettlement.sol',
  'contracts/src/VeriVotingWinnerOnlySettlement.sol'];
const stablePaths = ['Cargo.toml', 'Cargo.lock', 'contracts/package-lock.json',
  'configs/protocol_manifest.json', 'configs/protocol_manifest.JUBJUB-EVAL-D5-W32.json', ...contractPaths];
for (const p of stablePaths) assert.equal(sha(read(p)), sha(git(['show', `${STAGE2}:${p}`])), 'Stage2 reference changed: ' + p);
assert.match(read(contractPaths[0]).toString(), /function verifyBatch\(bytes\[\] calldata proofs, uint256\[\]\[\] calldata inputs\) external view/);
const receiver = read(contractPaths[2]).toString();
assert.ok(receiver.includes('batchVerifier = new IndividualGroth16BatchVerifier(ballotVk)'));
assert.ok(receiver.indexOf('batchVerifier.verifyBatch(') < receiver.indexOf('spentNullifier[nullifiers[i]] = true'));

const b2 = w32.cases.find(c => c.batch_size === 2);
assert.ok(b2); assert.equal(b2.verifier_srs.n, 2);
assert.deepEqual(b2.public_inputs_le_hex.map(row => row.length), [25, 25]);
assert.equal(wire(b2.aggregate_proof_hex).length, 5236);
assert.equal(sha(wire(b2.aggregate_proof_hex)), b2.aggregate_proof_sha256);
assert.equal(sha(wire(w32.groth16_vk_hex)), b2.groth16_vk_sha256);
assert.equal(w32.groth16_vk_ark_compressed_sha256, g8.ballot_vk_ark_sha256);
assert.equal(w32.merkle_depth, 5); assert.equal(w32.registered_weight_bound, 32);
assert.equal(w32.production_setup, false);
assert.equal(b2.negative, null, 'the original D0 B2 case has no archived negative verdict');
const be = h => '0x' + Buffer.from(wire(h)).reverse().toString('hex');
assert.deepEqual(b2.public_inputs_le_hex.map(row => row.map(be)), g8.batches[0].ballot_inputs_be_hex.slice(0, 2));

// Already installed locked sources only. No download, build, dependency upgrade or cryptographic replay.
const registry = path.join(process.env.CARGO_HOME ?? path.join(os.homedir(), '.cargo'), 'registry/src');
const roots = fs.readdirSync(registry).sort().map(d => path.join(registry, d));
const pkg = name => {
  const dir = roots.map(d => path.join(d, name)).find(p => fs.existsSync(p));
  assert.ok(dir, 'locked source unavailable: ' + name); return dir;
};
const specs = {
  verify: ['bellperson-0.27.0', 'src/groth16/aggregate/verify.rs'],
  proof: ['bellperson-0.27.0', 'src/groth16/aggregate/proof.rs'],
  transcript: ['bellperson-0.27.0', 'src/groth16/aggregate/transcript.rs'],
  accumulator: ['bellperson-0.27.0', 'src/groth16/aggregate/accumulator.rs'],
  prove: ['bellperson-0.27.0', 'src/groth16/aggregate/prove.rs'],
  srs: ['bellperson-0.27.0', 'src/groth16/aggregate/srs.rs'],
  gt: ['blstrs-0.7.0', 'src/gt.rs'],
  serde: ['blstrs-0.7.0', 'src/serde_impl.rs'],
};
const sources = Object.fromEntries(Object.entries(specs).map(([key, [name, rel]]) => {
  const b = read(path.join(pkg(name), rel));
  return [key, {package_path: `${name}/${rel}`, sha256: sha(b), text: b.toString()}];
}));
assert.equal(sources.verify.sha256, transcript.reference_verifier_sha256);
assert.equal(sources.transcript.sha256, transcript.reference_transcript_sha256);
for (const [key, p] of Object.entries(prior.sources)) assert.equal(sources[key].sha256, p.sha256);
const at = (key, anchor) => {
  const s = sources[key], i = s.text.indexOf(anchor); assert.ok(i >= 0, key + ': ' + anchor);
  return {source: s.package_path, line: s.text.slice(0, i).split('\n').length, anchor};
};
const lock = read('Cargo.lock').toString();
const lockedPackages = ['bellperson', 'blstrs', 'blst', 'bincode', 'sha2', 'rayon'].flatMap(name =>
  lock.split('\n[[package]]\n').filter(b => b.startsWith(`name = "${name}"\n`)).map(b =>
    Object.fromEntries([...b.matchAll(/^(name|version|source|checksum) = "([^"]+)"/gm)].map(m => [m[1], m[2]]))));
assert.ok(lockedPackages.some(p => p.name === 'bellperson' && p.version === '0.27.0'));
assert.ok(lockedPackages.some(p => p.name === 'blstrs' && p.version === '0.7.0'));

const items = [
  {id: 'PARSE', name: '解析、规范编码、子群及参数绑定', native: 'IMPLEMENTED', evm: 'PARTIAL',
    required: '原 proof read + parsing_check；B/向量长度/每行维度；G1/G2/GT 规范解码与子群；完整消费字节；固定 VK 和含 n/h_alpha_d 的完整 SRS。',
    gap: '已有 JS B2 解码和组件实验，不是完整 Solidity 入口。GT torus 288B/点压缩与 EIP 字段格式不同；不能仅检查 SRS PartialEq。',
    evidence: [at('proof', 'pub fn parsing_check'), at('proof', 'fn read(mut source: impl Read)'), at('gt', 'fn read_compressed'), at('srs', 'pub struct VerifierSRS'), at('srs', 'impl<E: Engine> PartialEq for VerifierSRS')]},
  {id: 'FS', name: '完整 Fiat–Shamir', native: 'IMPLEMENTED', evm: 'PARTIAL',
    required: '原 bincode、SHA-256、所有 V2 挑战及完整顺序矩阵 external transcript；保留 snarkpack-v1 前缀。',
    gap: '历史 Rust/JS 12 次匹配，不等于 Solidity 实现。usize nonce 宽度固定为该64位证据域；拒绝计数累积哈希，非简单 hash mod q。',
    evidence: [at('transcript', 'pub fn write<S: Serialize>'), at('transcript', 'pub fn into_challenge'), at('verify', '.write(&transcript_include)')]},
  {id: 'GIPA', name: '全部 GIPA 折叠', native: 'IMPLEMENTED', evm: 'NOT_IMPLEMENTED',
    required: '逐轮挑战/inverse、所有 GT 和 G1 折叠、final_r 与 V2 extra-link；B2 仍不可省略。',
    gap: '标量公式和向量核对不能替代电路/合约执行完整折叠。',
    evidence: [at('verify', 'fn gipa_verify_tipp_mipp'), at('verify', 'Transcript::<E>::new("gipa-extra-link")'), at('verify', 'let final_r = polynomial_evaluation_product_form_from_transcript')]},
  {id: 'KZG', name: 'KZG-V/W 四项', native: 'IMPLEMENTED', evm: 'NOT_IMPLEMENTED',
    required: 'V alpha/beta、W alpha/beta；f_w 使用 r^-1 shift 与 z^n；固定所有 SRS 系数。',
    gap: 'identity pairing ABI 可用，但完整挑战/解析/子群/点运算与四项验证未集成。',
    evidence: [at('verify', 'pub fn verify_kzg_v'), at('verify', 'pub fn verify_kzg_w')]},
  {id: 'TIPP', name: 'TIPP 三项目标等式', native: 'IMPLEMENTED', evm: 'PARTIAL',
    required: '最终 zab/tab/uab 三个任意 GT 目标，不只是 pairing product == 1。',
    gap: '只有一项终点的 native/JS 对照；不能用上传的 GT 或布尔值作为验证结果。',
    evidence: [at('verify', 'let _check_z ='), at('verify', 'let _check_ab0 ='), at('verify', 'let _check_ab1 =')]},
  {id: 'MIPP', name: 'MIPP 两项与独立 G1 终点', native: 'IMPLEMENTED', evm: 'NOT_IMPLEMENTED',
    required: 'tc/uc 两项目标等式，并检查 C*final_r==zc；后者不是额外 pairing。',
    gap: '必须保持任何 G1 终点失败会 invalidate 整个验证，不能只数10条配对式。',
    evidence: [at('verify', 'let _check_t ='), at('verify', 'let _check_u ='), at('verify', 'let b = z == final_res.zc;')]},
  {id: 'AGGREGATE', name: '最终聚合与全部公开输入', native: 'IMPLEMENTED', evm: 'NOT_IMPLEMENTED',
    required: '每行维度与 IC 长度一致、行数=B；所有 X_i,j 按 r^i 进入最终式，同时进入应用 external transcript。',
    gap: '只匹配 batch root/session 或复用旧挑战不够；原生调用者负责 external transcript 的完整声明绑定。',
    evidence: [at('verify', 'if (pub_input.len() + 1) != pvk.ic.len()'), at('verify', 'if public_inputs.len() != proof.tmipp.gipa.nproofs as usize'), at('verify', 'pairing_checks_copy.merge_nonrandom(')]},
  {id: 'GT', name: 'GT、Miller 与最终幂', native: 'IMPLEMENTED', evm: 'PARTIAL',
    required: 'Fp/Fp2/Fp6/Fp12 规范运算；GT 解压/子群/乘幂；完整 Miller、最终幂和任意目标比较。',
    gap: '已有部分有限域/软件 Miller 组件；full64 在20M预算失败，未完成最终幂或完整验证；预编译不返回 GT。',
    evidence: [at('gt', 'pub fn uncompress'), at('accumulator', 'let left = self.left.final_exponentiation();')]},
  {id: 'ACCUMULATOR', name: '随机累积与替代语义', native: 'IMPLEMENTED', evm: 'EQUIVALENCE_UNPROVEN',
    required: '明确数学关系与 native 随机系数累积的接受语义、误差和有效输入域；不能使用已知固定随机种子替代论证。',
    gap: '原代码用 Gt::generator() 作特殊 neutral sentinel，实际 identity 不同。直接逐式检查需 typed target、完备性/健全性论证，不等于任意坏证明/种子的逐比特接受等价。',
    evidence: [at('accumulator', 'if out != &<E as Engine>::Gt::generator()'), at('accumulator', 'let one = E::Gt::generator();'), at('accumulator', 'fn derive_non_zero'), at('gt', 'fn identity() -> Self'), at('gt', 'fn generator() -> Self')]},
];
assert.equal(items.length, 9);
const equationAnchors = {
  KZG_V_ALPHA: 'pub fn verify_kzg_v', KZG_V_BETA: 'pub fn verify_kzg_v',
  KZG_W_ALPHA: 'pub fn verify_kzg_w', KZG_W_BETA: 'pub fn verify_kzg_w',
  TIPP_Z: 'let _check_z =', TIPP_T: 'let _check_ab0 =', TIPP_U: 'let _check_ab1 =',
  MIPP_T: 'let _check_t =', MIPP_U: 'let _check_u =', GROTH16_AGGREGATE: 'pairing_checks_copy.merge_nonrandom(',
};
assert.deepEqual(prior.ten_equations.map(e => e.id), Object.keys(equationAnchors));
const equations = prior.ten_equations.map(e => ({...e, evidence: at('verify', equationAnchors[e.id]),
  target_kind: e.id.startsWith('KZG') ? 'IDENTITY_TARGET' : 'ACTUAL_GT_TARGET',
  classification: 'SOURCE_DERIVED_B2_RELATION_NOT_NEW_EVM_EXECUTION'}));
assert.equal(equations.reduce((n, e) => n + e.pairs, 0), 18);
const historical = {
  software_miller: {source: paths.oog, status: oog.status, receipt: oog.full_attempt.transaction_hash,
    failed_capped_spend_gas: oog.full_attempt.total_gas, completed_miller_gas: null,
    completed_snarkpack_gas: null, classification: 'HISTORICAL_MEASURED_FAILED_SPEND_NOT_COMPLETION_COST'},
  transcript: {source: paths.fs, matched_calls: 12, cases: 2, solidity_fs_implemented: false},
  endpoint: {source: paths.endpoint, native_js_one_endpoint_match: true, evm_arbitrary_gt_verified: false},
  p1: {source: paths.p1v2, status: prior.status, finite_design_is_not_measured_feasibility: true},
  j1: {source: paths.j1, status: j1.status, native_fq_bits: 381, current_fr_bits: 255,
    compile_rejection: 'E0271', r1cs_synthesized: false,
    implication: 'Only direct reuse of the locked native gadget in current Fr is blocked; no general zkVM impossibility result.'},
};
const inventory = {
  schema: 'VV/G12/VERIFICATION-INVENTORY/v1', gate: 'G12', status: STATUS,
  artifact_kind: 'SOURCE_AND_ARCHIVED_EVIDENCE_REVIEW_NOT_NEW_CRYPTOGRAPHIC_EXPERIMENT',
  reviewed_date: '2026-11-15', timezone: 'Asia/Shanghai', baseline_commit: BASE,
  baseline_branch: 'experiment/j1-joint-proof-feasibility', stage2_readonly_reference: STAGE2,
  intended_branch: 'experiment/g12-snarkpack-evm-feasibility',
  generator: pin('scripts/g12_feasibility_review.mjs'),
  preservation: {baseline_tracked_file_count: tree.length, checked_immutable_files: protectedFiles,
    checked_immutable_bytes: protectedBytes, baseline_tree_listing_sha256: sha(Buffer.from(tree.join('\0') + '\0')),
    rolling_indexes_excluded: [...rolling], stage2_parameter_contract_lock_checks: stablePaths.length,
    preexisting_line_ending_differences: preexistingLineEndings,
    core_protocol_changed: false, old_results_changed: false, paper_changed: false},
  source_pins: [...new Set([...Object.values(paths), historicalPath, ...stablePaths,
    'docs/G9_EXACT_VERIFICATION_ROUTE.md', 'docs/G9_P1_STAGED_EXACT_DESIGN.md', 'docs/G9_P1_FEASIBILITY_V2.md',
    'docs/STAGE2_EVM_FOLD_COMPARISON.md', 'docs/JOINT_PROOF_FEASIBILITY.md',
    'scripts/g9_route_review.mjs', 'scripts/g9p1_design.mjs', 'scripts/g9p1_feasibility.mjs',
    'contracts/scripts/snarkpack_transcript_g5c2.mjs'])].map(pin),
  locked_packages: lockedPackages,
  reference_sources: Object.values(sources).map(({text, ...metadata}) => metadata),
  standards: [{url: 'https://eips.ethereum.org/EIPS/eip-2537', reviewed_date: '2026-11-15',
    observation: 'Boolean product-pairing identity ABI, no arbitrary GT target/result; MSM/pairing check subgroups, additions do not; canonical 64B BE Fp.'},
  {url: 'https://eips.ethereum.org/EIPS/eip-2565', reviewed_date: '2026-11-15',
    observation: 'Integer modular exponentiation primitive, not an Fp12/Miller/final-exponent verifier.'}],
  historical_observations_not_rerun: historical,
  items, ten_pairing_equations: equations, logical_pairing_terms: 18,
  mandatory_nonpairing_endpoint: {formula: prior.B2_specialization.mipp_nonpairing,
    evidence: at('verify', 'let b = z == final_res.zc;'), evm_status: 'NOT_IMPLEMENTED'},
  b2_specialization: Object.fromEntries(['notation', 'fv', 'fw', 'final_r', 'final_zc', 'aggregate_r_sum'].map(k => [k, prior.B2_specialization[k]])),
  b2_material: {source: paths.b2, batch_size: 2, public_input_rows: 2, fields_per_row: 25,
    aggregate_proof_sha256: b2.aggregate_proof_sha256, aggregate_proof_bytes: 5236,
    bellperson_vk_sha256: b2.groth16_vk_sha256, arkworks_vk_sha256: w32.groth16_vk_ark_compressed_sha256,
    shared_research_srs_sha256: w32.shared_aggregation_srs_sha256,
    verifier_srs_canonical_json_sha256: sha(Buffer.from(encode(b2.verifier_srs))),
    verifier_srs_hash_encoding: 'UTF-8 JSON.stringify(case.verifier_srs,null,2) plus LF; includes n and h_alpha_d; not a standard wire SRS hash',
    full_srs_fields: Object.keys(b2.verifier_srs).sort(), matched_g8_fields: 50,
    original_d0_b2_negative: 'NOT_ARCHIVED', new_full_crypto_replay: false,
    research_fake_srs: true, production_setup: false,
    rule: 'Reuse only this W32 proof with its own VK/SRS/transcript. Do not splice old G5-C1 targets or challenges.'},
  execution_counts_this_review: {evm_transactions: 0, solidity_compilations: 0, rust_compilations: 0,
    setup: 0, prove: 0, native_or_js_full_crypto_replays: 0, group_probes: 0},
  exact_snarkpack_evm_verified: false,
};

const operations = [
  ['PARSE_FS', '规范 scalar/point/GT 解码与子群、完整读取、原 V2 FS 拒绝采样和矩阵绑定'],
  ['GROUP_SCALAR', 'Fr 逆/幂，G1/G2 MSM；KZG 四式与 MIPP G1 终点'],
  ['GT', 'Fp2/Fp6/Fp12 乘法/逆/乘幂、15个 GT 解压与 subgroup、10个 GIPA GT 幂'],
  ['MILLER_FINAL_EXP', '六项任意 GT 目标关系的 Miller/最终幂，或另经证明的完整等价策略'],
  ['SESSION', 'BEGIN/ADVANCE/FINISH/CONSUME 的存储、校验、calldata、错误/中止和原子业务提交'],
];
const budget = {
  schema: 'VV/G12/RESOURCE-BUDGET/v1', gate: 'G12', status: STATUS,
  classification: 'HISTORICAL_EVIDENCE_AND_PARTIAL_FORMULAS_NOT_A_NEW_BENCHMARK',
  local_policy: {profile: 'Existing local Prague research profile; not a universal live-chain limit',
    per_transaction_gas_cap: oog.transaction_gas_limit, block_context_gas_cap: oog.block_gas_limit,
    maximum_total_gas_policy: null, maximum_total_transactions_policy: null, maximum_local_time_ms_policy: null,
    caps_automatically_increased: false},
  known_quantities: [
    {name: 'Historical full64 failed spend', value: oog.full_attempt.total_gas, unit: 'Gas',
      kind: 'HISTORICAL_MEASURED_FAILED_SPEND', source: paths.oog,
      excludes: 'Not a completed Miller or SnarkPack cost; not extrapolated to the full verifier.'},
    {name: 'Four two-term KZG pairing calls only', value: 4 * (37700 + 32600 * 2), unit: 'Gas',
      kind: 'STANDARD_FORMULA_PARTIAL', formula: '4*(37700+32600*2)',
      source: 'https://eips.ethereum.org/EIPS/eip-2537#pairing-check-operation',
      excludes: 'MSM, parser, FS, GT work, transaction/call/calldata/storage costs and all other checks.'},
    {name: 'Archived compressed B2 proof plus scalar matrix', value: 5236 + 2 * 25 * 32, unit: 'bytes',
      kind: 'DERIVED_MATERIAL_LENGTH', source: paths.b2,
      excludes: 'No ABI/session/VK/SRS/envelope overhead; not full transaction calldata.'},
    {name: 'Expanded GT field representation proposal', value: 15 * 12 * 64, unit: 'bytes',
      kind: 'DESIGN_PROPOSAL', excludes: 'Not measured storage and not necessarily all simultaneously live.'},
    {name: 'G9 partial fixed-exponent operation subtotal',
      value: prior.fixed_exponent_subprogram_counts.known_subprogram_operation_upper_subtotal, unit: 'square_or_multiply',
      kind: 'PARTIAL_REFERENCE_ALGORITHM_UPPER_SUBTOTAL', source: paths.p1v2,
      components: prior.fixed_exponent_subprogram_counts,
      excludes: 'Not a lower bound, not a complete verifier upper bound, not Gas; many operations and scheduling are absent.'},
    {name: 'G9 proposed finite register/storage allocation', value: prior.finite_layout_proposal.maximum_allocated_bytes_proposed,
      unit: 'bytes', kind: 'DESIGN_PROPOSAL', proposed_slots: prior.finite_layout_proposal.maximum_allocated_slots_proposed,
      source: paths.p1v2, excludes: 'Not measured, compiled or proven maximum live storage; no EVM admission inference.'},
  ],
  phase_bounds: operations.map(([id, required]) => ({id, required, implementation_complete: false,
    maximum_gas_per_step: null, maximum_step_transactions: null, maximum_storage_bytes: null,
    maximum_execution_time_ms: null, kind: 'UNKNOWN', reason: 'No complete audited EVM implementation/schedule.'})),
  complete_verifier_bounds: {maximum_gas_per_transaction_proven: null, maximum_total_gas: null,
    maximum_transactions: null, maximum_storage_bytes: null, maximum_execution_time_ms: null,
    full_operation_count_upper_bound: null, complete_b2_positive_gas_measured: null,
    complete_b2_negative_gas_measured: null},
  total_cost_definition: 'BEGIN + sum(ADVANCE_i) + FINISH + CONSUME including parsing, FS, arithmetic, checkpoints, calldata and storage.',
  continuation_design: {
    implementation_status: 'DESIGN_ONLY',
    tower: {Fp: 'canonical <p, two u256 limbs; not Montgomery internals', Fp2: 'u^2=-1', Fp6: 'v^3=u+1', Fp12: 'w^2=v'},
    checkpoint: ['session/version', 'phase and micro-PC', 'coefficient/bit/pair cursors',
      'canonical arithmetic registers', 'G2 line state and signed-x Miller cursor', 'completed-check bitmap'],
    transition_rule: 'Only contract-computed or algebraically checked transitions from the authenticated previous checkpoint; never trust uploaded line/GT/host-success values.',
    gt_targets: 'Separate IdentityTarget tag from ActualGT(value). Never globally replace literal GT generator by multiplicative identity.',
    pow: 'Fixed exponent bit schedule; checked inverses; exact tower/line embedding and final-exponent convention require source/vector validation.',
    session_binding: ['chain ID', 'verifier and unique receiver addresses', 'suite/program version', 'full VK and verifier SRS hashes',
      'proof hash', 'ordered matrix and external-transcript hash', 'B', 'eid and final context',
      'registration/accepted batch roots', 'region and batch index', 'old CT and proposed new CT'],
    stages: [
      {name: 'BEGIN', rule: 'Bind full statement; at most one pending next batch; no accepted CT/NF writes.'},
      {name: 'ADVANCE', rule: 'Enforce session, next micro-PC and authenticated registers; resume all required operations.'},
      {name: 'FINISH', rule: 'All nine categories, ten pairing relations and G1 endpoint complete before VERIFIED.'},
      {name: 'CONSUME', rule: 'Fixed receiver only; recheck current old CT/index/root/fresh NFs, exact matrix and state proof; consume once and update CT/NFs atomically.'},
      {name: 'ABORT', rule: 'Incomplete/expired/out-of-budget session is not accepted; preserve accepted business state, never reinterpret as cryptographic INVALID or VERIFIED.'},
    ],
    synchronous_view_drop_in: false, old_g8_contracts_changed: false,
    original_fiat_shamir_changed: false,
    equivalence_obligation: 'Inductively relate every checkpoint/terminal result to the chosen supported-domain verifier relation; explicitly account for randomized batching/sentinel semantics and completeness.',
    rejection_sampling: 'Original data-dependent cumulative-hash rejection loop has no established practical worst-case cap. A resource watchdog gives ABORT/INCOMPLETE, not proof invalidity or unconditional full-domain completion.',
    public_chain_time: 'Local wall time does not bound transaction inclusion/finality time.',
  },
  may_start_full_b2_evm: false,
};

const route = {
  schema: 'VV/G12/ROUTE-DECISION/v1', gate: 'G12', status: STATUS,
  routes: [
    {id: 'A', name: '原 SnarkPack 多交易精确 EVM', decision: 'BLOCKED_DO_NOT_START_G13',
      semantic_status: 'All relations mapped; supported-domain equivalence/randomized accumulator policy unproven.',
      evm_status: 'A reviewable continuation skeleton exists, not a complete implemented/resource-audited verifier.',
      required_work: ['complete canonical parser and FS', 'GT/Miller/final exponent stepper',
        'all equations and MIPP endpoint', 'typed GT targets and sound acceptance semantics',
        'bounded storage/Gas/transactions schedule', 'independent asynchronous receiver protocol'],
      total_gas: null, transactions: null, storage_bytes: null,
      feasibility: 'UNKNOWN_NOT_PROVEN_IMPOSSIBLE', optimization_campaign_started: false},
    {id: 'B', name: '完整 SnarkPack 验证的递归/zkVM 包装', decision: 'PREFERRED_NEXT_SEPARATE_FEASIBILITY_REVIEW_ONLY',
      semantic_status: 'Must actually verify the entire original statement inside an attested/constrained computation; no host Boolean shortcut.',
      evm_status: 'Outer verifier/curve/VK and guest portability are not selected or demonstrated.',
      required_work: ['audit guest portability of locked bellperson/blstrs/BLST FFI/Rayon and entropy; no assumption native crates run unchanged',
        'preserve canonical encoding, complete ordered inputs, transcript, all GT/GIPA/KZG/TIPP/MIPP/aggregate checks',
        'specify a sound deterministic verifier relation or justified randomness; known-seed native randomized batching is not automatically sound inside a prover-controlled guest',
        'bind program/image ID, version, full VK/SRS, proof/matrix hashes and election/batch/state statement in the public journal and contract',
        'admit new backend/assumptions/setup and manifests explicitly; measure guest prove/resources and outer EVM verify'],
      j1_boundary: 'Locked native Fq gadget cannot be used directly in current Fr; no general impossibility of zkVM or other recursion follows.',
      prototype_compiled_this_round: false, new_backend_installed: false, benchmark_evidence: null,
      proof_time_ms: null, peak_rss_mib: null, outer_evm_gas: null,
      feasibility: 'UNKNOWN_NO_SPEED_OR_COMPATIBILITY_CLAIM'},
    {id: 'C', name: '重新设计统一批次证明', decision: 'BACKUP_REQUIRES_NEW_PROTOCOL_SCOPE',
      semantic_status: 'A different proof structure must explicitly retain ballot legality plus exact accepted batch/state binding.',
      required_work: ['either verify existing original proofs in-circuit, or explicitly redesign voter/prover data flow and suite',
        'do not assume aggregator possesses private voter witnesses or replace proof checks with host results',
        'new circuit/layout/VK/setup/manifest and security proof; preserve old protocol as comparison baseline'],
      current_compatible_inner_verifier: false, constraints: null, prove_time_ms: null,
      evm_gas: null, feasibility: 'UNKNOWN_NOT_A_DROP_IN_OPTIMIZATION'},
  ],
  gate_checks: {all_nine_categories_mapped: true, all_ten_equations_and_g1_endpoint_mapped: true,
    full_supported_domain_equivalence_closed: false, complete_gt_stepper_implemented: false,
    all_phase_resource_bounds_closed: false, explicit_total_resource_admission_closed: false,
    full_b2_evm_positive_and_cryptographic_negative_passed: false},
  final_answers: [
    {question: '真正技术障碍是什么？', answer: 'Identity-only pairing ABI cannot evaluate arbitrary GT targets; full original parser/FS/GT/semantic preservation and total staged resource schedule are missing, not just a Solidity function.'},
    {question: 'G9 已解决哪些问题？', answer: 'Pinned wire/transcript conformance and one endpoint; identified ten equations plus G1 endpoint, B2 source/formulas, sentinel issue and continuation skeleton; documented failed full64 spend. Not a complete verifier.'},
    {question: '是否有明确可审计的多交易实现路线？', answer: 'Only a source-mapped design skeleton. Complete transition implementation, supported-domain acceptance semantics and terminal equivalence remain unproven.'},
    {question: '哪些资源上界未知？', answer: 'Complete parse/FS/GT/Miller/final-exponent and checkpoint costs; full per-step/total Gas, maximum transactions/live storage/runtime and practical rejection-sampling admission remain null.'},
    {question: '能否进入 B2 完整 EVM？', answer: 'No. G12 is BLOCKED; no G13, no partial-check acceptance and no cap increase.'},
    {question: '替代方向是什么？', answer: 'Prioritize a separately authorized B guest-portability/semantics review; C is a redesign fallback. Neither is implemented or known cheaper. Preserve current real individual-proof fallback and disclose its non-SnarkPack boundary.'},
  ],
  next_minimal_b2_acceptance_if_gate_later_passes: {authorized_now: false,
    positive: 'Reuse the archived W32 B2 proof with its complete original statement and run all checks.',
    negative: 'Change only row 1 input[7] (second voter nullifier), keep original proof/VK/SRS, rebuild transcript and session, require complete cryptographic rejection; not merely session mismatch.',
    measurements: ['all per-transaction receipts and sum Gas', 'transaction count', 'calldata', 'maximum live storage', 'full completion time and relation coverage']},
  stop: {g13_started: false, contracts_changed: false, setup_or_prove_started: false,
    paper_changed: false, auto_commit: false, auto_push: false,
    instruction: 'Stop after G12. A report-integrity PASS is never a feasibility or verifier PASS.'},
};

const invPath = 'results/g12/verification_inventory.json', budgetPath = 'results/g12/resource_budget.json';
const routePath = 'results/g12/route_decision.json', docPath = 'docs/G12_SNARKPACK_EVM_FEASIBILITY.md';
const invBytes = encode(inventory), budgetBytes = encode(budget);
route.inventory_sha256 = sha(Buffer.from(invBytes)); route.resource_budget_sha256 = sha(Buffer.from(budgetBytes));
const routeBytes = encode(route);
const table = items.map(i => `| ${i.id} | ${i.name} | ${i.native} | ${i.evm} |`).join('\n');
const eqTable = equations.map(e => `| ${e.id} | ${e.pairs} | \`${e.formula}\` | ${e.evidence.line} |`).join('\n');
const resourceTable = budget.known_quantities.map(e => `| ${e.name} | ${e.value.toLocaleString('en-US')} ${e.unit} | ${e.kind} |`).join('\n');
const doc = `# G12：SnarkPack EVM 完整验证可行性审查

## 结论：${STATUS}

日期：2026-11-15（Asia/Shanghai）。本轮是**源码与既有证据审查**，不是新性能实验或完整验证器。
基于最新 J1 提交 \`${BASE}\` 新建 \`experiment/g12-snarkpack-evm-feasibility\`；Stage2 \`${STAGE2}\` 仅作只读参照，不退回旧版本、不丢弃 J1。
**不批准进入 G13/B2 完整 EVM，不自动实施包装或联合证明。** 本轮到此停止，不提交、不推送。

本轮新增 EVM 交易、Solidity/Rust 编译、Setup、Prove、完整密码学重放、群运算探针均为0。
核对基线 ${protectedFiles} 个不可变已跟踪文件的 Git blob 与当前字节；旧代码、参数、fixtures、结果与论文不改。
只有 NEXT_STEPS 和 IMPLEMENTATION_STATUS 两个滚动索引更新，历史版本仍可从基线提交取得。
已有 p3_curve_compare.csv 的工作树 CRLF 与 Git LF 不同（core.autocrlf=input），初次原始 blob 检查因此停止且未生成报告。
随后确认其当前 SHA-256 与 J1 历史字节清单一致、仅换行归一化后与基线 Git blob 一致；脚本同时检查这两项并记录例外，**不改写 CSV**。

## 1. 为什么不是“再写一个 Solidity 函数”

原 SnarkPack V2 不只有最终 Groth16 配对式，还包括规范解析、原挑战、GIPA、KZG、TIPP/MIPP、独立 G1 终点及任意 GT 目标。
[EIP-2537](https://eips.ethereum.org/EIPS/eip-2537#abi-for-pairing-check)只返回配对乘积是否为单位元，不返回 GT 或接受任意 GT 目标。
MSM/配对带子群检查，加法不带；不能用加法预编译代替验证输入域。
[EIP-2565](https://eips.ethereum.org/EIPS/eip-2565)是整数模幂，不是 Fp12 或完整 Miller/最终幂验证器。
因此存在原语接口不匹配，但这**不是原生精确验证在所有资源预算下不可能的证明**。

## 2. G9 证据到底完成了什么

- G5-C2：原生/JS 两个用例共12次挑战、编码及拒绝计数匹配；不是 Solidity FS 完成。
- G5-C3：一条 TIPP 终点 native/JS 对照；不是全部 TIPP/MIPP/KZG 验证。
- full64 软件 Miller：原交易 \`${oog.full_attempt.transaction_hash}\` 在20,000,000 Gas预算 OOG。
  该数字是**失败封顶支出**，不是完成一次 Miller 或 SnarkPack 的成本；最终幂未完成，本轮不重跑或提高预算。
- G9-P1/v2：已列出10条配对等式、独立 MIPP G1 终点、B2 标量公式、有限寄存器布局提案及 sentinel 风险；状态仍为 \`${prior.status}\`。
- Stage2 两路接收器均使用真实逐票 Groth16 回退，不是精确 SnarkPack EVM；不把其 Gas 当成聚合验证成本。
- J1：原生 Fq(381-bit) gadget 编译控制通过，直接要求当前 Fr(255-bit) 后端时 E0271；未构建 R1CS。
  这仅排除**当前锁定后端直接复用**，不排除其他递归后端或 zkVM。没有新递归性能数据。

来源路径、各 SHA-256、锁定包版本及逐条源码行/anchor 见 [verification_inventory.json](../${invPath})。
上游参考为本机 Cargo.lock 锁定的 bellperson 0.27.0 / blstrs 0.7.0；verify/transcript SHA 与旧 C2 相符，不下载或修改依赖。

## 3. 完整验证清单

下表 native 表示上游存在该实现，不是本轮重新执行或证明了实现安全。EVM 的 PARTIAL 仅表示已有组件/对照证据。

| 项目 | 检查 | 原生源码 | EVM 状态 |
| --- | --- | --- | --- |
${table}

### 解析与 FS 的不可省略边界

B2 压缩证明为15个 GT、9个 G1、5个 G2及一个 u32，共5236 B；2行×25个 Fr 输入另占1600 B。
需保持字段规范性、曲线与子群、全部字节消费/规范 roundtrip、B和长度一致，固定完整 VK/SRS。
VerifierSRS 的 PartialEq 未覆盖 n/h_alpha_d，不能靠对象相等代替完整参数绑定；本报告另存包含所有字段的规范 JSON 摘要，其编码不冒称标准 SRS wire。
原生 read/parsing_check 与应用加载/绑定分工存在，指出 EVM 缺口不代表声称原生 parser 有新漏洞。

GT 压缩 wire 为6×48 B LE；FS bincode 序列化用完整12个系数；EIP 字段为64 B BE，三者不得混用。
torus 解压、非零分母、子群及原可编码域要逐项保持，不能发明 identity 特例编码而宣称兼容。
V2 仍使用 \`snarkpack-v1\` 前缀。此64位证据域 nonce 是8 B BE，从1开始，将历次拒绝计数累计到同一 SHA 状态；shave 后按 Fr LE 表示解析，拒绝0/1及非规范值，不能改成取模。
六次 B2 挑战为 hcom、random-r、gipa-0 seed/round、extra-link、random-z；外部 transcript 必须绑定全部非固定顺序输入。
session 的额外绑定独立于原 FS，不能悄悄改变原挑战域。

### B2 的10条配对式与额外终点

记 a=c_inv、c=a^-1，r 为 random-r，z 为 random-z；G1/G2用加法，GT用乘法；A/B/C是 GIPA 最终点。
下表是**来源映射及关系层候选**，不是已经证明等价的 Solidity 实现；行号均对应锁定 verify.rs。

| 关系 | 逻辑配对项 | B2 公式 | 源码行 |
| --- | --- | --- | --- |
${eqTable}

共18个逻辑配对项；另必须检查 \`C*(1+a*r)==agg_c+c*Zc_L+a*Zc_R\`（verify.rs:${at('verify', 'let b = z == final_res.zc;').line}），不能漏掉或算成第11条配对式。
f_v(z)=1+a*z，f_w(z)=z²*(1+c*r^-1*z)，final_r=1+a*r。完整公开矩阵同时参与原应用 transcript 与最终 IC 线性组合。

### 随机累积不能无论证地确定化

原 PairingChecks 用随机非零 Fr 系数合并多项，最后执行一次最终幂；测试 harness 的已知固定种子不具备自动的对抗性健全性。
实现中 \`Gt::generator()\` 还承担特殊 neutral sentinel，而 blstrs 实际乘法单位元是 \`Gt::identity()\`，二者不同。
必须区分 IdentityTarget 与 ActualGT(value)，不能全局把真实 generator 值改写为1。
“逐条检查所有数学等式”是可讨论的替代关系，但不等于对所有坏证明、随机种子、sentinel 角落逐比特接受等价；需明确有效域、完备性及健全性/误差。
本轮不报告新攻击，不修订上游，也不将源码 anchor 检查当成等价性证明。

## 4. 资源：能给出的数字及不能给出的数字

| 数量 | 值 | 性质 |
| --- | --- | --- |
${resourceTable}

四项 KZG 的411,600 Gas仅是标准配对调用公式，不含其他计算/调用/交易，也不是新 receipt。
64,518 是旧提案里固定幂子程序的朴素 square/multiply **局部上界小计**，既非完整上界，也非下界，不能换算成完整 Gas。
41,280 B/1,290 slots 是未编译布局提案，不是证明过的最大活跃存储。
每笔20M、block上下文30M仍只是既有本地 Prague 研究策略，不声称当前公链永久限制。

完整解析/FS、GT解压和幂、Miller、最终幂、检查点以及交易开销均未闭合。
**完整每步/总 Gas、最大交易数、最大存储、完成时间、完整操作数上界全部为 null。** 尚未选择并准入总交易/总时间/总Gas上限。
拒绝采样是数据相关循环，没有本轮可证明的实际最坏执行上限；运营性超时只能 ABORT/INCOMPLETE，不能说 INVALID_PROOF 或宣称完整输入域无条件完成。
本机 watchdog 不等于公共链收录或最终性时间界。
完整分类、排除项和各阶段未知值见 [resource_budget.json](../${budgetPath})。

## 5. 多交易路线：已有可审查骨架，尚无完整实现

旧 IBatchVerifier 是同步 view，G8 接收器构造时固定创建逐票回退验证器；不能原地址直接换成异步分阶段证明。
未来必须另建独立版本，原 G8 合约与历史结果不动。

1. BEGIN：唯一 Receiver 绑定 chain/address、suite/version、完整 VK/SRS、proof/matrix/external-transcript、B、eid/context、注册/已接受批次根、区域/index、old/new CT。不写已接受 CT/NF。
2. ADVANCE：从存储的认证状态按 micro-PC 推进 Fp→Fp2→Fp6→Fp12、GT解压/乘幂及 Miller。保存系数/bit/pair游标、G2线状态和有符号x循环位置；调用者提供的结果不能直接信任。
3. FINISH：九类检查、10条配对式和独立 G1 终点全部通过才能 VERIFIED；部分通过不是接收授权。
4. CONSUME：唯一 Receiver 重查当前旧CT、index、root和新鲜 NF，对同一矩阵验证状态证明，一次性原子消费并更新 NF/CT。
5. ABORT：未完成或预算耗尽不改变已接受业务状态，也不得升级为 VERIFIED。

Fp使用规范<p的两u256表示，Fp2:u²=-1，Fp6:v³=u+1，Fp12:w²=v。检查点还需版本、phase、PC、规范寄存器和完成位图；逆元可由见证提供但必须检查乘积为1。
Miller线嵌入、负x共轭、塔域顺序、最终幂约定、每阶段检查点到原关系的归纳对应仍未实现/验证。
有一个有限结构提案不代表已具备逐步 Gas 证明、完整状态恢复实现或最大资源上界，因此路线A仍不准入。

## 6. A/B/C 三路线决策

| 路线 | 本轮决定 | 保留语义与成本瓶颈 |
| --- | --- | --- |
| A 原多交易精确验证 | BLOCKED，不进入 G13 | 保持完整原关系，但缺 GT stepper、随机/typed-target语义、异步接收及全部资源界；不是数学上已证不可能 |
| B 递归/zkVM 包装 | 下一轮**独立可行性审查候选** | 需 guest可移植性、完整内部验证、program/journal/状态绑定、新假设/VK/设置；生成与外层Gas均未知 |
| C 新统一批次证明 | 备用，需新的协议范围 | 新关系/数据流/电路/VK/Manifest与安全论证，不能当无成本优化 |

B 不得假设 bellperson 的 Rayon、blstrs/BLST FFI 和熵源能直接在 guest运行；本轮没有安装后端、guest编译或测量。
即使 guest执行成功，也不能让恶意证明者选择随机数或采用公开固定种子重放随机批检后，未经论证就声称原健全性成立。
需验证完整确定性关系，或另行论证随机化来源与安全误差，并把完整矩阵、VK/SRS、证明、选举/批次/状态与程序身份绑定到公开 journal 和外层合约。
现有 J1 Fq/Fr 不匹配不能仅改类型别名解决，也不能据此宣布 zkVM 不可能；新后端/曲线/VK/可信设置必须单独准入，不沿用旧 Manifest。

C 的聚合器只有选票公开材料，不能假设持有选民私有 witness。必须真正验证旧证明，或明确重设投票者到证明者的数据流/套件；host-language 成功值不是约束。
没有证据表明B或C一定更快、更便宜或已经可兼容。保留现有真实逐票回退能力及非 SnarkPack 声明边界。

## 7. 六项验收回答与后续边界

1. 真正障碍：任意GT目标与预编译接口不匹配，完整 parser/FS/全部关系及接受语义、资源调度未闭合。
2. G9已完成：跨语言编码/挑战、一个终点、10式+G1终点映射、B2公式/材料、有限设计、真实失败证据；未完成验证器。
3. 多交易路线：有可审查骨架，没有完整已实现且证明终态等价的执行路线。
4. 未知界：完整各阶段/总Gas、交易数、存储、运行时间及实际拒绝采样预算；均保持 null。
5. B2完整EVM：**不允许**；G12为 BLOCKED，不把产物自检 PASS 当作可行性 PASS。
6. 替代方向：先另行授权B的可移植性/语义审查，C为备选，不继续盲写原生 Miller 或自动扩大预算。

未来只有语义与资源门槛闭合后才考虑原 W32 B2 正例和单字段密码学反例。反例保持 proof/VK/SRS，仅改第二票 input[7]并重建对应 transcript/session，必须由完整验证拒绝，不能仅以旧session不匹配冒充。
本轮没有执行该反例，原 D0 B2 case 也没有归档 negative 结果；旧 G5-C1 的另一份证明/挑战不可拼接到 W32。
继续公开记录研究 Fake SRS 和确定性实验设置，不宣称生产可信设置、完整链上批量结算或公链部署。

## 8. 产物与复核

\`node scripts/g12_feasibility_review.mjs --write\` 首次生成本轮独立产物；已存在但不匹配时拒绝覆盖。
\`node scripts/g12_feasibility_review.mjs --check\` 只读核对源码、历史字节、报告和摘要，未来提交后也可运行，不要求HEAD仍等于基线。

- [verification_inventory.json](../${invPath})：SHA-256 \`${sha(Buffer.from(invBytes))}\`。
- [resource_budget.json](../${budgetPath})：SHA-256 \`${sha(Buffer.from(budgetBytes))}\`。
- [route_decision.json](../${routePath})：SHA-256 \`${sha(Buffer.from(routeBytes))}\`。
- [SHA256SUMS](../results/g12/SHA256SUMS)：包括上述JSON、本报告和生成脚本；文件名相对仓库根目录。

复核成功只表示当前审查产物与证据一致。**最终可行性状态仍是 ${STATUS}，完整 SnarkPack EVM=false。**
`;
const outputs = new Map([[invPath, invBytes], [budgetPath, budgetBytes], [routePath, routeBytes], [docPath, doc]]);
const sums = [...outputs].map(([p, content]) => `${sha(Buffer.from(content))}  ${p}`);
sums.push(`${sha(read('scripts/g12_feasibility_review.mjs'))}  scripts/g12_feasibility_review.mjs`);
outputs.set('results/g12/SHA256SUMS', sums.join('\n') + '\n');
// Validate all pre-existing outputs before writing any new artifact.
for (const [p, content] of outputs) if (mode === '--check' || fs.existsSync(p)) {
  assert.equal(read(p).toString(), content, 'G12 deterministic artifact mismatch: ' + p);
}
if (mode === '--write') for (const [p, content] of outputs) if (!fs.existsSync(p)) {
  fs.mkdirSync(path.dirname(p), {recursive: true}); fs.writeFileSync(p, content, {flag: 'wx'});
}
console.log(encode({audit: 'PASS_REVIEW_ARTIFACT_INTEGRITY_ONLY', feasibility_status: STATUS, mode,
  historical_files_unchanged: protectedFiles, verifier_categories: items.length, pairing_relations: equations.length,
  logical_pairing_terms: 18, separate_mipp_g1_endpoint: true, full_b2_evm_allowed: false,
  new_evm_transactions: 0, new_setup: 0, new_proofs: 0, exact_snarkpack_evm_verified: false,
  route_decision_sha256: sha(Buffer.from(routeBytes))}));
