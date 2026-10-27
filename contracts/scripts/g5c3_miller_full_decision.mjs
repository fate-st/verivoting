// B22: saved-evidence decision only. No VM, compilation or new gas measurement.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const read = name => fs.readFileSync(path.join(root, name));
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const prefixName = 'experiments/g5_c3_raw/miller_quarter_prefix_evm.json';
const prefixBytes = read(prefixName);
const prefix = JSON.parse(prefixBytes);
const primitiveBytes = read(prefix.primitive_baseline);
const primitive = JSON.parse(primitiveBytes);
const fullName = prefix.latest_full_attempt_result;
const fullBytes = read(fullName);
const full = JSON.parse(fullBytes);
const earlierBytes = read(full.baseline);
const earlier = JSON.parse(earlierBytes);
const oldPrefixBytes = read(prefix.baseline);
const oldPrefix = JSON.parse(oldPrefixBytes);
assert.equal(prefix.gate, 'G5-C3-B21');
assert.equal(primitive.gate, 'G5-C3-B20');
assert.equal(full.gate, 'G5-C3-B18');
assert.equal(earlier.gate, 'G5-C3-B16');
assert.equal(sha256(primitiveBytes), prefix.primitive_baseline_sha256);
assert.equal(sha256(fullBytes), prefix.latest_full_attempt_result_sha256);
assert.equal(sha256(earlierBytes), full.baseline_sha256);
assert.equal(sha256(oldPrefixBytes), prefix.baseline_sha256);
for (const record of [prefix, primitive, full]) {
  assert.equal(sha256(read(record.runner_source)), record.runner_source_sha256);
  assert.equal(sha256(read(record.contract_source)), record.contract_source_sha256);
}
assert.equal(prefix.prefix_conformance_and_accounting_passed, true);
assert.equal(prefix.prefix_gas_reduced, true);
assert.equal(primitive.correctness_and_accounting_passed, true);
assert.equal(primitive.measured_window_improvement, true);
assert.equal(prefix.full_loop_attempts, 0);
assert.equal(prefix.full_attempt, null);
assert.equal(prefix.new_full_variant_tested, false);
assert.equal(full.full_attempt.exception, 'out of gas');
assert.equal(full.full_attempt.total_gas, 20_000_000);
assert.notEqual(prefix.deployed_runtime_sha256, full.deployed_runtime_sha256);
assert.equal(earlier.expected_full_modexp_calls_from_formula, 14_316);
assert.equal(prefix.transaction_gas_limit, full.transaction_gas_limit);
for (const [name, key] of [
  [prefix.fixture, 'fixture_sha256'], [prefix.source_fixture, 'source_fixture_sha256'],
  [prefix.endpoint_fixture, 'endpoint_fixture_sha256'], ['Cargo.lock', 'cargo_lock_sha256'],
  ['contracts/package-lock.json', 'package_lock_sha256'],
  ['configs/protocol_manifest.json', 'official_manifest_sha256'],
  ['configs/protocol_manifest.JUBJUB-PROTOTYPE.json', 'prototype_manifest_sha256'],
]) {
  assert.equal(sha256(read(name)), prefix[key], name);
  assert.equal(sha256(read(name)), full[key], name);
}
const source = read(prefix.contract_source).toString('utf8');
const oldSource = read(oldPrefix.contract_source).toString('utf8');
function body(text, name) {
  const start = text.indexOf('    function ' + name + '(');
  assert.ok(start >= 0);
  let i = text.indexOf('{', start), depth = 1;
  for (let j = i + 1; j < text.length; j++) {
    if (text[j] === '{') depth++;
    if (text[j] === '}') depth--;
    if (!depth) return text.slice(start, j + 1);
  }
  throw Error('unterminated function');
}
assert.equal(body(source, '_mul'), body(read(primitive.contract_source).toString('utf8'), '_mul'));
assert.equal(source.replace('contract Bls12381MillerLoopQuarter', 'contract Bls12381MillerLoopSubLowLevel')
  .replace(body(source, '_mul'), body(oldSource, '_mul')), oldSource);
assert.equal(body(source, '_miller'), body(oldSource, '_miller'));
assert.ok(body(source, 'verifyMillerLoopOrRevert').includes('_verify(points, 64, expected)'));
assert.match(body(source, '_miller'), /if \(steps == 64\)[\s\S]*_copy6\(f\.c1, _neg6\(f\.c1\)\)/);
const fixture = JSON.parse(read(prefix.fixture));
assert.equal(fixture.full_steps, 64);
assert.equal(fixture.prefix_steps, prefix.prefix_steps);
assert.equal(Buffer.from(fixture.full_miller_fp12_hex.slice(2), 'hex').length, 768);
const nextGate = 'B23: ONE full64 positive on the exact saved B21 source/runtime/compiler and frozen full fixture at the same20M cap; one deployment, no prefix/boundary rerun; on failure stop software arithmetic and full retries';
const report = {
  schema: 'VV/G5-C3/MILLER-FULL-DECISION/v1', gate: 'G5-C3-B22',
  status: 'PASS_DERIVED_SINGLE_FULL_CONFORMANCE_DECISION_ONLY',
  classification: 'saved-evidence-and-source-review-not-an-evm-benchmark',
  derived_nonbenchmark: true, evm_transactions_executed: 0,
  new_evm_benchmark_run: false, runtime_recompiled: false,
  recorded_at_utc: new Date().toISOString(),
  git_commit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
  git_dirty: execFileSync('git', ['status', '--porcelain'], { cwd: root, encoding: 'utf8' }).trim().length > 0,
  prefix_evidence: prefixName, prefix_evidence_sha256: sha256(prefixBytes),
  primitive_evidence: prefix.primitive_baseline, primitive_evidence_sha256: sha256(primitiveBytes),
  historical_full_evidence: fullName, historical_full_evidence_sha256: sha256(fullBytes),
  historical_formula_evidence: full.baseline, historical_formula_evidence_sha256: sha256(earlierBytes),
  contract_source: prefix.contract_source, contract_source_sha256: prefix.contract_source_sha256,
  deployed_runtime_sha256: prefix.deployed_runtime_sha256, runtime_bytecode_bytes: prefix.runtime_bytecode_bytes,
  analysis_source: path.relative(root, fileURLToPath(import.meta.url)),
  analysis_source_sha256: sha256(fs.readFileSync(fileURLToPath(import.meta.url))),
  package_lock_sha256: prefix.package_lock_sha256, cargo_lock_sha256: prefix.cargo_lock_sha256,
  official_manifest_sha256: prefix.official_manifest_sha256,
  prototype_manifest_sha256: prefix.prototype_manifest_sha256,
  evidence_summary: {
    current_prefix_steps: prefix.prefix_steps, full_steps: fixture.full_steps,
    current_prefix_total_gas: prefix.prefix_valid.total_gas,
    prefix_reduction_percent_observed_only: prefix.comparison.prefix_gas_reduction_percent,
    current_full_attempts: prefix.full_loop_attempts, current_full_variant_tested: false,
    historical_full_is_different_runtime: true, historical_full_exception: full.full_attempt.exception,
    full_entry_exists_unchanged: true, full_only_negative_x_conjugation_not_executed_by_two_step_prefix: true,
  },
  routing_decision: {
    selected_action: 'ONE_BOUNDED_FULL_CONFORMANCE_ATTEMPT_THEN_STOP_ON_FAILURE',
    reason: 'B20 arithmetic and B21 prefix passed; the saved quarter runtime has no full64 evidence, and historical OOG belongs to a different runtime',
    information_value: 'close the current-version full64/budget evidence gap, including late loop/arena use and final negative-x conjugation if execution reaches it',
    paper_scope: 'the current manuscript already defers complete aggregate EVM verification; B23 closes the chosen software-route experiment, not a prerequisite for its restricted claims',
    manuscript_basis: 'sn-article.tex sections on target-verifier compatibility and restricted evaluation; complete comparative batch/settlement measurements remain deferred',
    success_probability_estimated: false, prefix_ratios_used_to_forecast_full_gas: false,
    arithmetic_changes_selected: false, automatic_optimization_search: false,
  },
  planned_validation: {
    gate: 'G5-C3-B23', executed: false, deployments: 1, full_positive_transactions: 1,
    total_evm_transactions: 2, transaction_gas_limit: 20_000_000, block_gas_limit: 30_000_000,
    source: prefix.contract_source, source_sha256: prefix.contract_source_sha256,
    runtime_sha256: prefix.deployed_runtime_sha256,
    compiler: prefix.compiler, compiler_settings: prefix.compiler_settings,
    fixture: prefix.fixture, fixture_sha256: prefix.fixture_sha256,
    source_fixture: prefix.source_fixture, source_fixture_sha256: prefix.source_fixture_sha256,
    method: 'verifyMillerLoopOrRevert(points_eip2537_hex, full_miller_fp12_hex)',
    full_calldata_sha256: full.full_attempt.calldata_sha256,
    expected_complete_modexp_calls_from_unchanged_formula: earlier.expected_full_modexp_calls_from_formula,
    expected_return_payload_bytes: 768, prefix_reruns: 0, primitive_boundary_reruns: 0,
    negative_matrix: false, final_exponentiation: false, setup_or_prove: false,
    instrumentation: 'MODEXP success/failure and gas counters, exact return bytes, gas accounting and active memory; no large opcode/source profiling run',
    success_rule: 'receipt1, exact full canonical 768B fixture, 14316 successful MODEXPs and zero failures, reconciled fees; record measured completed-loop gas only then',
    oog_rule: 'receipt0 with out-of-gas and full20M spend; completed-loop gas remains null; freeze software arithmetic and further full retries',
    other_failure_rule: 'save actual revert/return/decode/accounting evidence; do not mislabel as OOG; freeze software arithmetic/full retries and diagnose read-only',
    success_does_not_complete: 'final exponentiation, exact GT binding or complete SnarkPack verification',
  },
  latest_prefix_gate: prefix.gate, latest_full_attempt_gate: full.gate,
  target_verifier_status: prefix.target_verifier_status, target_verifier_status_source: prefixName,
  contract_changed: false, optimization_implemented: false, protocol_capabilities_changed: false,
  miller_run: false, full_loop_attempts: 0, full_miller_loop_verified_in_evm: false,
  full_miller_verification_gas: null, estimated_full_gas: null, completion_percent: null,
  exact_snarkpack_evm_verified: false, snarkpack_verification_gas: null,
  setup_run: false, prove_run: false, private_witness_emitted: false, next_gate: nextGate,
  limitations: [
    'This gate adds a decision and zero new EVM evidence; B23 is a future plan, not an executed result',
    'Prefix improvement supplies no success guarantee, completed full gas estimate or completion percentage',
    '20M is a chosen experiment budget, not a proven target-chain hard limit; the budget and Suite do not change',
    'A failed attempt closes this software-version budget check without authorizing another optimization/profiling/retry cycle',
  ],
};
const output = path.resolve(root, process.argv[2] ?? 'experiments/g5_c3_raw/miller_full_decision.json');
assert.ok(!fs.existsSync(output), 'B22 decision exists; inspect it instead of overwriting');
fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ status: report.status, evm_transactions_executed: 0,
  decision: report.routing_decision.selected_action, planned_gate_executed: false, next_gate: nextGate }));
console.log('wrote ' + path.relative(root, output));
