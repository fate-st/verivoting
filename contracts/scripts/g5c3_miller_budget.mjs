// Read-only analysis of saved B16 evidence: no VM, transactions or new benchmark.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import solc from 'solc';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const read = name => fs.readFileSync(path.join(root, name));
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const evidenceName = 'experiments/g5_c3_raw/miller_sub_full_evm.json';
const evidenceBytes = read(evidenceName);
const evidence = JSON.parse(evidenceBytes);
const baselineBytes = read(evidence.baseline);
const baseline = JSON.parse(baselineBytes);
const previousBytes = read(evidence.previous_full_attempt_result);
const previous = JSON.parse(previousBytes);
assert.equal(evidence.gate, 'G5-C3-B16');
assert.equal(baseline.gate, 'G5-C3-B15');
assert.equal(previous.gate, 'G5-C3-B7');
assert.equal(sha256(baselineBytes), evidence.baseline_sha256);
assert.equal(sha256(previousBytes), evidence.previous_full_attempt_result_sha256);
assert.equal(sha256(read('contracts/scripts/g5c3_miller_sub_full.mjs')), evidence.runner_source_sha256);
assert.equal(sha256(read('contracts/scripts/g5c3_miller_sub_prefix.mjs')), baseline.runner_source_sha256);
const sourceBytes = read(evidence.contract_source);
assert.equal(sha256(sourceBytes), evidence.contract_source_sha256);
assert.equal(evidence.contract_source_sha256, baseline.contract_source_sha256);
for (const [name, key] of [
  [evidence.fixture, 'fixture_sha256'], [evidence.source_fixture, 'source_fixture_sha256'],
  [evidence.endpoint_fixture, 'endpoint_fixture_sha256'], ['Cargo.lock', 'cargo_lock_sha256'],
  ['contracts/package-lock.json', 'package_lock_sha256'],
  ['configs/protocol_manifest.json', 'official_manifest_sha256'],
  ['configs/protocol_manifest.JUBJUB-PROTOTYPE.json', 'prototype_manifest_sha256'],
]) assert.equal(sha256(read(name)), evidence[key], name);

const attempt = evidence.full_attempt;
assert.equal(attempt.exception, 'out of gas');
assert.equal(attempt.receipt_status, 0);
assert.equal(attempt.return_hex, '0x');
assert.equal(attempt.gas_refund, 0);
assert.equal(attempt.total_gas, evidence.transaction_gas_limit);
assert.equal(attempt.execution_gas + attempt.intrinsic_gas, attempt.total_gas);
assert.equal(attempt.modexp_failure_count, 0);
assert.equal(attempt.modexp_call_count, attempt.modexp_success_count);
assert.equal(evidence.full_miller_verification_gas, null);
assert.equal(evidence.full_miller_loop_verified_in_evm, false);

// Adding output selection does not alter the program. Prove this by comparing
// the deployed runtime, then use its instruction-indexed deployed source map.
assert.equal(solc.version(), evidence.compiler);
const settings = structuredClone(evidence.compiler_settings);
settings.outputSelection['*']['*'].push('evm.deployedBytecode.sourceMap');
const sourceUnit = path.basename(evidence.contract_source);
const sourceText = sourceBytes.toString('utf8');
const compilation = JSON.parse(solc.compile(JSON.stringify({ language: 'Solidity',
  sources: { [sourceUnit]: { content: sourceText } }, settings })));
for (const error of compilation.errors ?? []) {
  if (error.severity === 'error') throw Error(error.formattedMessage);
}
const contracts = Object.values(compilation.contracts[sourceUnit]);
assert.equal(contracts.length, 1);
const compiled = contracts[0].evm.deployedBytecode;
const runtime = Buffer.from(compiled.object, 'hex');
assert.equal(runtime.length, evidence.runtime_bytecode_bytes);
assert.equal(sha256(runtime), evidence.deployed_runtime_sha256);
const target = attempt.last_root_step;
let instructionIndex = 0;
let found = false;
for (let pc = 0; pc < runtime.length; instructionIndex++) {
  const opcode = runtime[pc];
  if (pc === target.pc) {
    assert.equal(opcode, 0x1c); // SHR; PUSH immediates must not be counted as opcodes.
    assert.equal(target.opcode, 'SHR');
    found = true;
    break;
  }
  pc += 1 + (opcode >= 0x60 && opcode <= 0x7f ? opcode - 0x5f : 0);
}
assert.ok(found, 'last observed PC is not an instruction boundary');
const entries = compiled.sourceMap.split(';');
assert.ok(instructionIndex < entries.length, 'last instruction has no source-map entry');
const location = ['-1', '-1', '-1', '', '0'];
for (let i = 0; i <= instructionIndex; i++) {
  entries[i].split(':').forEach((value, field) => {
    if (value !== '') location[field] = value;
  });
}
const [start, length, fileId] = location.slice(0, 3).map(Number);
assert.equal(fileId, compilation.sources[sourceUnit].id);
assert.ok(start >= 0 && length > 0 && start + length <= sourceBytes.length);
const snippet = sourceBytes.subarray(start, start + length).toString('utf8');
const line = sourceBytes.subarray(0, start).toString('utf8').split('\n').length;
assert.equal(snippet, 'shr(1, hi)');
const modexpGas = attempt.modexp_execution_gas;
const executionGas = attempt.execution_gas;
const unclassifiedGas = executionGas - modexpGas;
const words = attempt.evm_active_memory_words_at_exit;
const memoryExpansionGas = 3 * words + Math.floor(words * words / 512);
const nextGate = 'B18: ONE same-runtime, same-calldata, same-20M attribution run; reconcile charged root-opcode/source bins, MODEXP children and terminal gas burn before proposing any arithmetic change';
const report = {
  schema: 'VV/G5-C3/MILLER-BUDGET-REVIEW/v1',
  gate: 'G5-C3-B17', status: 'PASS_DERIVED_MILLER_BUDGET_REVIEW_ONLY',
  classification: 'derived-read-only-analysis-of-saved-evm-evidence',
  derived_nonbenchmark: true, evm_transactions_executed: 0,
  source_map_recompiled_only: true, new_evm_benchmark_run: false,
  recorded_at_utc: new Date().toISOString(),
  git_commit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
  evidence: evidenceName, evidence_sha256: sha256(evidenceBytes),
  prefix_evidence: evidence.baseline, prefix_evidence_sha256: sha256(baselineBytes),
  previous_full_evidence: evidence.previous_full_attempt_result,
  previous_full_evidence_sha256: sha256(previousBytes),
  analysis_source: path.relative(root, fileURLToPath(import.meta.url)),
  analysis_source_sha256: sha256(fs.readFileSync(fileURLToPath(import.meta.url))),
  contract_source: evidence.contract_source, contract_source_sha256: sha256(sourceBytes),
  compiler: solc.version(), compiler_settings_used: settings,
  runtime_sha256: sha256(runtime), runtime_bytecode_bytes: runtime.length,
  source_map_sha256: sha256(compiled.sourceMap),
  last_observed_root_source: {
    ...target, instruction_index: instructionIndex, source_unit: sourceUnit,
    source_id: fileId, byte_start: start, byte_length: length, line, snippet,
    interpretation: 'terminal trigger only; no cumulative function hotspot attribution',
  },
  derived_costs: {
    observed_execution_gas: executionGas,
    successful_modexp_child_gas: modexpGas,
    successful_modexp_child_percent_of_execution: 100 * modexpGas / executionGas,
    execution_gas_not_attributed_to_successful_modexp_children: unclassifiedGas,
    memory_expansion_gas_at_observed_watermark: memoryExpansionGas,
    memory_expansion_percent_of_observed_execution: 100 * memoryExpansionGas / executionGas,
    costs_are_from_failed_attempt_only: true,
  },
  budget_policy: {
    experiment_transaction_limit: evidence.transaction_gas_limit,
    recorded_block_limit: evidence.block_gas_limit,
    chain_hard_limit_established_by_this_analysis: false,
    automatic_budget_increase: false,
  },
  routing_decision: {
    software_changes: 'HOLD_PENDING_CUMULATIVE_COST_ATTRIBUTION',
    next_gate: 'G5-C3-B18', next_gate_executed: false,
    selected_work: 'one cumulative-cost attribution experiment on the unchanged software route',
    reason: 'saved evidence identifies root gas exhaustion but has no cumulative opcode/source ranking',
    wrapper_work: 'DEFERRED_REQUIRES_NEW_VERIFIER_RELATION_AND_RESOURCE_PLAN',
    automatic_full_retry_for_success: false,
    automatic_arithmetic_optimization: false,
  },
  latest_full_attempt_gate: evidence.gate, latest_prefix_gate: baseline.gate,
  target_verifier_status: evidence.target_verifier_status,
  target_verifier_status_source: evidenceName,
  full_miller_loop_verified_in_evm: false, full_miller_verification_gas: null,
  estimated_full_gas: null, completion_percent: null,
  exact_snarkpack_evm_verified: false, snarkpack_verification_gas: null,
  protocol_capabilities_changed: false, miller_run: false, full_loop_attempts: 0,
  next_gate: nextGate,
  limitations: [
    'This analysis adds no EVM measurement; B16 remains the latest actual full attempt',
    'The terminal source span is not a hotspot; unclassified root costs are not all arithmetic',
    'Memory expansion is only its schedule cost, not all memory-related opcode cost or host RSS',
    'Failed spend and MODEXP count do not estimate completed-loop gas or completion percentage',
    'B18 is planned only; no change to curve, encoding, Suite, proof relation or Manifest',
  ],
};
const output = path.resolve(root, process.argv[2] ?? 'experiments/g5_c3_raw/miller_budget_review.json');
fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ status: report.status, evm_transactions_executed: 0,
  terminal_source: `${sourceUnit}:${line} ${snippet}`,
  modexp_child_percent: report.derived_costs.successful_modexp_child_percent_of_execution,
  memory_expansion_gas: memoryExpansionGas, next_gate: report.routing_decision.next_gate }));
console.log('wrote ' + path.relative(root, output));
