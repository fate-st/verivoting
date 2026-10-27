// B19: saved-evidence/source/algebra review only. No VM import or contract edit.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const read = name => fs.readFileSync(path.join(root, name));
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const evidenceName = 'experiments/g5_c3_raw/miller_cost_attribution.json';
const evidenceBytes = read(evidenceName);
const evidence = JSON.parse(evidenceBytes);
const primitiveName = 'experiments/g5_c3_raw/field_lowlevel_profile.json';
const primitiveBytes = read(primitiveName);
const primitive = JSON.parse(primitiveBytes);
assert.equal(evidence.gate, 'G5-C3-B18');
assert.equal(evidence.reproduction_matches_b16, true);
assert.equal(evidence.reconciliation.passed, true);
assert.equal(evidence.reconciliation.difference_gas, 0);
assert.equal(evidence.routing_decision.candidate_for_read_only_review, null);
assert.equal(sha256(read(evidence.runner_source)), evidence.runner_source_sha256);
assert.equal(sha256(read(evidence.baseline)), evidence.baseline_sha256);
assert.equal(sha256(read(evidence.budget_review)), evidence.budget_review_sha256);
assert.equal(primitive.gate, 'G5-C3-B6');
assert.equal(sha256(read('contracts/scripts/g5c3_field_lowlevel.mjs')), primitive.runner_source_sha256);
assert.equal(sha256(read(primitive.contract_source)), primitive.contract_source_sha256);
const sourceBytes = read(evidence.contract_source);
assert.equal(sha256(sourceBytes), evidence.contract_source_sha256);
for (const [name, key] of [
  [evidence.fixture, 'fixture_sha256'], [evidence.source_fixture, 'source_fixture_sha256'],
  [evidence.endpoint_fixture, 'endpoint_fixture_sha256'], ['Cargo.lock', 'cargo_lock_sha256'],
  ['contracts/package-lock.json', 'package_lock_sha256'],
  ['configs/protocol_manifest.json', 'official_manifest_sha256'],
  ['configs/protocol_manifest.JUBJUB-PROTOTYPE.json', 'prototype_manifest_sha256'],
]) assert.equal(sha256(read(name)), evidence[key], name);

const source = sourceBytes.toString('utf8');
function functionBody(text, name) {
  const start = text.indexOf('    function ' + name + '(');
  assert.ok(start >= 0);
  const brace = text.indexOf('{', start);
  let depth = 1;
  for (let i = brace + 1; i < text.length; i++) {
    if (text[i] === '{') depth++;
    if (text[i] === '}') depth--;
    if (!depth) return { text: text.slice(start, i + 1), start,
      line: text.slice(0, start).split('\n').length };
  }
  throw Error('unterminated function');
}
const mul = functionBody(source, '_mul');
assert.equal(mul.text, functionBody(read(primitive.contract_source).toString('utf8'), '_mul').text);
assert.equal([...mul.text.matchAll(/hi, lo := halfMod\(hi, lo\)/g)].length, 2);
assert.equal([...mul.text.matchAll(/:= square\(input, r, hi, lo\)/g)].length, 2);
const pHi = BigInt(source.match(/P_HI\s*=\s*(0x[0-9a-f]+)/)[1]);
const pLo = BigInt(source.match(/P_LO\s*=\s*(0x[0-9a-f]+)/)[1]);
const B = 1n << 256n;
const P = pHi * B + pLo;
assert.equal(P % 4n, 3n);
assert.ok(2n * pLo < B && B < 3n * pLo && 3n * pLo < 2n * B);
assert.ok(4n * P < B * B);

const rootGas = evidence.reconciliation.charged_root_opcode_gas;
const unknown = evidence.source_mapping.explicit_unmapped_generated_scaffolding_gas;
const mapped = evidence.exclusive_solidity_function_ranking.filter(x => !x.name.startsWith('<'));
assert.equal(mapped[0].name, '_mul');
const rivalMaximum = mapped[1].charged_gas + unknown;
assert.ok(mapped[0].charged_gas > rivalMaximum);
const half = evidence.exclusive_source_leaf_ranking.find(x => x.name === '_mul::halfMod');
assert.ok(half);
const fpHex = x => '0x' + x.toString(16).padStart(128, '0');
const plannedCases = [
  ['zero_k0', 0n], ['near_p_k1', P - 2n], ['near_p_k2_sum_carry', P - 1n],
  ['k3_both_product_and_sum_carry', B - 1n],
].map(([id, t]) => {
  const k = t & 3n;
  const product = k * pLo;
  const sum = (t % B) + (product % B);
  const q = (t + k * P) / 4n;
  assert.equal((t + k * P) % 4n, 0n);
  assert.ok(t >= 0n && t < P && q >= 0n && q < P);
  assert.equal(4n * q % P, t);
  assert.equal(product / B, k === 3n ? 1n : 0n);
  return { id, reduced_square_difference_hex: fpHex(t), correction_k: Number(k),
    input_a_hex: fpHex(1n), input_b_hex: fpHex(q), expected_product_hex: fpHex(q),
    expected_kp_low_product_carry: Number(product / B), expected_sum_low_carry: Number(sum / B),
    evm_executed: false, comparison_performance_sample: false };
});
assert.deepEqual(plannedCases.map(x => x.correction_k), [0, 1, 2, 3]);
assert.equal(plannedCases[3].expected_kp_low_product_carry, 1);
assert.equal(plannedCases[3].expected_sum_low_carry, 1);
const nextGate = 'B20: one isolated Fp _mul comparison replacing only the two halfMod calls with the fixed-p quarterMod; one saved operand performance case plus four correction/carry correctness cases; no Miller run';
const report = {
  schema: 'VV/G5-C3/MILLER-ROUTE-REVIEW/v1', gate: 'G5-C3-B19',
  status: 'PASS_SOURCE_DERIVED_MUL_QUARTER_ROUTE_REVIEW_ONLY',
  classification: 'saved-evidence-source-and-algebra-review-not-an-evm-benchmark',
  derived_nonbenchmark: true, new_evm_benchmark_run: false, evm_transactions_executed: 0,
  recorded_at_utc: new Date().toISOString(),
  git_commit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
  git_dirty: execFileSync('git', ['status', '--porcelain'], { cwd: root, encoding: 'utf8' }).trim().length > 0,
  evidence: evidenceName, evidence_sha256: sha256(evidenceBytes),
  isolated_baseline: primitiveName, isolated_baseline_sha256: sha256(primitiveBytes),
  isolated_baseline_contract: primitive.contract_source,
  isolated_baseline_contract_sha256: primitive.contract_source_sha256,
  contract_source: evidence.contract_source, contract_source_sha256: sha256(sourceBytes),
  analysis_source: path.relative(root, fileURLToPath(import.meta.url)),
  analysis_source_sha256: sha256(fs.readFileSync(fileURLToPath(import.meta.url))),
  mul_source_line: mul.line, baseline_and_miller_mul_bodies_identical: true,
  package_lock_sha256: evidence.package_lock_sha256, cargo_lock_sha256: evidence.cargo_lock_sha256,
  official_manifest_sha256: evidence.official_manifest_sha256,
  prototype_manifest_sha256: evidence.prototype_manifest_sha256,
  observed_rank_bound: {
    charged_root_gas: rootGas, mul_source_owned_gas: mapped[0].charged_gas,
    next_mapped_function: mapped[1].name, next_mapped_gas: mapped[1].charged_gas,
    explicit_unassigned_gas: unknown, largest_other_bucket_if_all_unassigned_added: rivalMaximum,
    mul_margin_over_that_bound: mapped[0].charged_gas - rivalMaximum,
    original_halfMod_source_owned_gas: half.charged_gas,
    original_halfMod_percent_of_charged_root: 100 * half.charged_gas / rootGas,
    scope: 'exclusive source-owned costs on the failed B18 path only; not inclusive caller costs or removable-gas guarantees',
  },
  parameter_certificates: {
    modulus_hex: fpHex(P), modulus_mod_4: Number(P % 4n),
    twice_low_limb_less_than_radix: 2n * pLo < B,
    three_times_low_limb_between_one_and_two_radices: B < 3n * pLo && 3n * pLo < 2n * B,
    four_moduli_fit_two_limbs: 4n * P < B * B,
  },
  routing_decision: {
    software_changes: 'ONE_ISOLATED_CANDIDATE_SELECTED_NOT_IMPLEMENTED',
    selected_candidate: 'FUSE_TWO_MODULAR_HALVES_IN_MUL_INTO_ONE_FIXED_P_QUARTER',
    previous_10_percent_filter_is_automatic_candidate_heuristic_not_safety_requirement: true,
    reason: 'mapped _mul remains largest even if all unassigned gas is assigned to the next function; the two halvings admit a source-derived fixed-p identity',
    formula: 'k = t & 3; quarter(t) = (t + k*p)/4, for canonical 0 <= t < p and p mod 4 = 3',
    low_product_carry: 'floor(k*P_LO/2^256) = (k == 3) for this exact pinned modulus only',
    expected_modexp_calls_per_mul: 2, expected_allocation_change: false,
    measured_candidate_gas: null, estimated_full_gas: null,
    wrapper_work: 'DEFERRED_REQUIRES_NEW_VERIFIER_RELATION_AND_RESOURCE_PLAN',
  },
  planned_validation: {
    gate: 'G5-C3-B20', executed: false, isolated_operation: 'Fp _mul only',
    planned_deployments: 1, planned_arithmetic_transactions: 5,
    planned_performance_samples: 1, correctness_cases: plannedCases,
    performance_case: { baseline: primitiveName, operation: 'fp',
      left_hex: primitive.fp.left_hex, right_hex: primitive.fp.right_hex,
      expected_product_hex: primitive.fp.expected_hex,
      historical_instrumented_window_gas: primitive.fp.instrumented_arithmetic_window_gas,
      historical_total_gas: primitive.fp.total_gas,
      historical_allocator_growth_bytes: primitive.fp.allocator_growth_in_window_bytes },
    compiler_and_budget: 'same B6 solc/optimizer/Prague settings; 20M transaction/30M block; no viaIR or protocol changes',
    acceptance: 'exact canonical results in all five calls, two successful MODEXPs per multiply, unchanged allocation, reconciled gas, and strictly lower same-input isolated window than B6',
    stopping_rule: 'if correctness or isolated window improvement fails, retain baseline and stop; no automatic prefix/full retry',
    next_integration_is_separate_gate: true,
  },
  latest_full_attempt_gate: evidence.gate, latest_full_attempt_result: evidenceName,
  latest_prefix_gate: evidence.latest_prefix_gate,
  target_verifier_status: evidence.target_verifier_status, target_verifier_status_source: evidenceName,
  contract_changed: false, optimization_implemented: false, protocol_capabilities_changed: false,
  miller_run: false, full_loop_attempts: 0, full_miller_loop_verified_in_evm: false,
  full_miller_verification_gas: null, estimated_full_gas: null, completion_percent: null,
  exact_snarkpack_evm_verified: false, snarkpack_verification_gas: null,
  setup_run: false, prove_run: false, private_witness_emitted: false, next_gate: nextGate,
  limitations: [
    'This gate records source/algebra analysis only; no new Solidity execution or measured gas improvement',
    'Planned-case hex values are mathematically derived test inputs/expectations, not simulated EVM results',
    'The pinned low-limb carry identity is not generic across moduli or curves',
    'The observed halfMod source bucket is only 6.06% of charged root gas; this candidate is not a full-verifier budget solution',
    'B18 remains the latest actual full attempt; full Miller/final exponentiation/GT binding/exact SnarkPack remain blocked',
  ],
};
const output = path.resolve(root, process.argv[2] ?? 'experiments/g5_c3_raw/miller_route_review.json');
assert.ok(!fs.existsSync(output), 'review output exists; inspect the saved analysis rather than overwrite it');
fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ status: report.status, evm_transactions_executed: 0,
  candidate: report.routing_decision.selected_candidate, observed_rank_bound: report.observed_rank_bound,
  planned_gate: report.planned_validation.gate, next_gate: nextGate }));
console.log('wrote ' + path.relative(root, output));
