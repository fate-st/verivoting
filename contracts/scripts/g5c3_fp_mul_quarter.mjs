// B20: one isolated Fp quarterMod cost comparison and four planned carry cases; no Miller.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import solc from 'solc';
import { ContractFactory, Interface } from 'ethers';
import { bls12_381 } from '@noble/curves/bls12-381.js';
import { Common, Hardfork, Mainnet } from '@ethereumjs/common';
import { createBlock } from '@ethereumjs/block';
import { createLegacyTx } from '@ethereumjs/tx';
import { createAccount, createAddressFromPrivateKey, hexToBytes, bytesToHex } from '@ethereumjs/util';
import { createVM, runTx } from '@ethereumjs/vm';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const read = (name) => fs.readFileSync(path.join(root, name));
const sha256 = (value) => createHash('sha256').update(value).digest('hex');
const metadata = (cmd, args) => execFileSync(cmd, args, { cwd: root, encoding: 'utf8' }).trim();
const baselineName = 'experiments/g5_c3_raw/field_lowlevel_profile.json';
const baselineBytes = read(baselineName);
const baseline = JSON.parse(baselineBytes);
const planName = 'experiments/g5_c3_raw/miller_route_review.json';
const planBytes = read(planName);
const plan = JSON.parse(planBytes);
const fullName = plan.latest_full_attempt_result;
const fullBytes = read(fullName);
const fullBaseline = JSON.parse(fullBytes);
const contractName = 'contracts/src/Bls12381FpMulQuarterProfile.sol';
const contractSource = read(contractName).toString('utf8');
const baselineSource = read(baseline.contract_source).toString('utf8');
const sourceBytes = read(baseline.source_fixture);
const source = JSON.parse(sourceBytes);
const fixtureBytes = read(baseline.fixture);
const fixture = JSON.parse(fixtureBytes);
const { Fp } = bls12_381.fields;
assert.equal(baseline.gate, 'G5-C3-B6');
assert.equal(plan.gate, 'G5-C3-B19');
assert.equal(fullBaseline.gate, 'G5-C3-B18');
assert.equal(plan.planned_validation.executed, false);
assert.equal(plan.planned_validation.planned_arithmetic_transactions, 5);
assert.equal(sha256(baselineBytes), plan.isolated_baseline_sha256);
assert.equal(sha256(fullBytes), plan.evidence_sha256);
assert.equal(sha256(baselineSource), baseline.contract_source_sha256);
assert.equal(sha256(read('contracts/scripts/g5c3_field_lowlevel.mjs')), baseline.runner_source_sha256);
assert.equal(sha256(read(plan.analysis_source)), plan.analysis_source_sha256);
assert.equal(sha256(read(fullBaseline.runner_source)), fullBaseline.runner_source_sha256);
assert.equal(sha256(read(fullBaseline.contract_source)), fullBaseline.contract_source_sha256);
assert.equal(sha256(sourceBytes), baseline.source_fixture_sha256);
assert.equal(sha256(fixtureBytes), baseline.fixture_sha256);
assert.equal(source.schema, 'VV/G5-C1/SNARKPACK-PUBLIC-FIXTURE/v1');
assert.equal(fixture.schema, 'VV/G5-C3/MILLER-LOOP/v1');
for (const [name, key] of [
  ['Cargo.lock', 'cargo_lock_sha256'], ['contracts/package-lock.json', 'package_lock_sha256'],
  ['configs/protocol_manifest.json', 'official_manifest_sha256'],
  ['configs/protocol_manifest.JUBJUB-PROTOTYPE.json', 'prototype_manifest_sha256'],
]) {
  assert.equal(sha256(read(name)), baseline[key], name);
  assert.equal(sha256(read(name)), plan[key], name);
}
function body(text, name) {
  const start = text.indexOf('    function ' + name + '(');
  assert.ok(start >= 0, name);
  const brace = text.indexOf('{', start);
  let depth = 1;
  for (let i = brace + 1; i < text.length; i++) {
    if (text[i] === '{') depth++;
    if (text[i] === '}') depth--;
    if (!depth) return text.slice(start, i + 1);
  }
  throw Error('unterminated function ' + name);
}
const oldMul = body(baselineSource, '_mul');
const newMul = body(contractSource, '_mul');
const unchangedFunctions = [...baselineSource.matchAll(/^    function (\w+)\(/gm)]
  .map(x => x[1]).filter(x => x !== '_mul');
for (const name of unchangedFunctions) assert.equal(body(contractSource, name), body(baselineSource, name));
assert.equal(contractSource.replace('contract Bls12381FpMulQuarterProfile', 'contract Bls12381FieldLowLevelProfile')
  .replace(newMul, oldMul), baselineSource, 'outside _mul only the contract name may change');
const helperSection = (text, name) => text.slice(text.indexOf('            function ' + name + '('),
  text.indexOf('            function square('));
const restoredMul = newMul.replace(helperSection(newMul, 'quarterMod'), helperSection(oldMul, 'halfMod'))
  .replace('            hi, lo := quarterMod(hi, lo)',
    '            hi, lo := halfMod(hi, lo)\n            hi, lo := halfMod(hi, lo)');
assert.equal(restoredMul, oldMul, 'only the helper and final pair of halvings may change');
const P = Fp.ORDER;
const B = 1n << 256n;
const P_LO = P % B;
assert.equal(P % 4n, 3n);
assert.ok(2n * P_LO < B && B < 3n * P_LO && 3n * P_LO < 2n * B);
assert.equal(BigInt(plan.parameter_certificates.modulus_hex), P);
const wire = Buffer.from(source.aggregate_proof_hex, 'hex');
assert.equal(wire.length, 5236);
assert.equal(sha256(wire), baseline.aggregate_proof_sha256);
assert.equal(sha256(wire), source.aggregate_proof_sha256);
const q = bls12_381.G2.Point.fromBytes(wire.subarray(4516, 4612)).toAffine();
const fpBE = value => {
  assert.ok(value >= 0n && value < P);
  return Buffer.from(value.toString(16).padStart(128, '0'), 'hex');
};
const fpLeft = bytesToHex(fpBE(q.x.c0));
const fpRight = bytesToHex(fpBE(q.y.c0));
const fpExpected = bytesToHex(fpBE(q.x.c0 * q.y.c0 % P));
assert.equal(fpExpected, bytesToHex(fpBE(Fp.mul(q.x.c0, q.y.c0))));
assert.equal(fpLeft, baseline.fp.left_hex);
assert.equal(fpRight, baseline.fp.right_hex);
assert.equal(fpExpected, baseline.fp.expected_hex);
assert.equal(fpLeft, plan.planned_validation.performance_case.left_hex);
assert.equal(fpRight, plan.planned_validation.performance_case.right_hex);
const fixedPoints = Buffer.from(fixture.points_eip2537_hex.slice(2), 'hex');
assert.equal(bytesToHex(fixedPoints.subarray(128, 192)), fpLeft);
assert.equal(bytesToHex(fixedPoints.subarray(256, 320)), fpRight);
const cases = plan.planned_validation.correctness_cases.map(entry => {
  const a = BigInt(entry.input_a_hex), b = BigInt(entry.input_b_hex);
  const t = 4n * a * b % P, k = t & 3n;
  const product = k * P_LO, low = (t % B) + product % B;
  const expected = bytesToHex(fpBE(a * b % P));
  assert.equal(t, BigInt(entry.reduced_square_difference_hex));
  assert.equal(Number(k), entry.correction_k);
  assert.equal(Number(product / B), entry.expected_kp_low_product_carry);
  assert.equal(Number(low / B), entry.expected_sum_low_carry);
  assert.equal(expected, entry.expected_product_hex);
  assert.equal(expected, bytesToHex(fpBE(Fp.mul(a, b))));
  return { ...entry, expected_hex: expected };
});
assert.deepEqual(cases.map(x => x.correction_k), [0, 1, 2, 3]);
const settings = { optimizer: { enabled: true, runs: 200 }, evmVersion: 'prague',
  outputSelection: { '*': { '*': ['abi', 'evm.bytecode.object', 'evm.deployedBytecode.object'] } } };
assert.equal(solc.version(), baseline.compiler);
assert.deepEqual(settings, baseline.compiler_settings);
const compilation = JSON.parse(solc.compile(JSON.stringify({ language: 'Solidity',
  sources: { 'Bls12381FpMulQuarterProfile.sol': { content: contractSource } }, settings })));
for (const error of compilation.errors ?? []) {
  if (error.severity === 'error') throw Error(error.formattedMessage);
  console.warn(error.formattedMessage);
}
const compiled = compilation.contracts['Bls12381FpMulQuarterProfile.sol'].Bls12381FpMulQuarterProfile;
const iface = new Interface(compiled.abi);
const factory = new ContractFactory(compiled.abi, `0x${compiled.evm.bytecode.object}`);
const common = new Common({ chain: Mainnet, hardfork: Hardfork.Prague });
assert.equal(common.hardfork(), baseline.hardfork);
const vm = await createVM({ common });
// Public deterministic local-only key; never touches RPC/user funds.
const senderKey = hexToBytes(`0x${'25'.repeat(32)}`);
const sender = createAddressFromPrivateKey(senderKey);
await vm.stateManager.putAccount(sender, createAccount({ nonce: 0n, balance: 10n ** 24n }));
const block = createBlock({ header: { number: 1n, gasLimit: 30_000_000n,
  baseFeePerGas: 1n, timestamp: 1_750_000_000n } }, { common, skipConsensusFormatValidation: true });
const stack = [];
let trace = [];
let topOpcodeGas = 0;
let staticcallOpcodeGas = 0;
let gasMarkers = [];
let nonce = 0n;
vm.evm.events.on('step', (step) => {
  if (step.depth !== 0) return;
  const fee = Number(step.opcode.dynamicFee ?? BigInt(step.opcode.fee));
  if (step.opcode.name === 'GAS') gasMarkers.push({ pc: step.pc,
    gas_left_before: Number(step.gasLeft), memory_words_before: Number(step.memoryWordCount),
    top_opcode_gas_before: topOpcodeGas, staticcall_gas_before: staticcallOpcodeGas });
  topOpcodeGas += fee;
  if (step.opcode.name === 'STATICCALL') staticcallOpcodeGas += fee;
});
vm.evm.events.on('beforeMessage', (message) => { stack.push(message.to?.toString()); });
vm.evm.events.on('afterMessage', (result) => {
  if (stack.pop() === '0x0000000000000000000000000000000000000005') {
    trace.push({ execution_gas: Number(result.execResult.executionGasUsed),
      success: !result.execResult.exceptionError, return_bytes: result.execResult.returnValue.length });
  }
});
async function transact(data, to) {
  trace = [];
  topOpcodeGas = 0;
  staticcallOpcodeGas = 0;
  gasMarkers = [];
  const tx = createLegacyTx({ nonce: nonce++, gasLimit: 20_000_000n, gasPrice: 10n,
    value: 0n, data: hexToBytes(data), ...(to ? { to } : {}) }, { common }).sign(senderKey);
  const started = performance.now();
  const result = await runTx(vm, { tx, block });
  return { result, measurement: {
    transaction_hash: bytesToHex(tx.hash()), receipt_status: result.receipt.status,
    total_gas: Number(result.totalGasSpent), execution_gas: Number(result.execResult.executionGasUsed),
    intrinsic_gas: Number(tx.getIntrinsicGas()), minimum_gas_limit_including_calldata_floor: Number(tx.getMinimumGasLimit()),
    gas_refund: Number(result.gasRefund),
    calldata_floor_applied: to ? result.totalGasSpent > tx.getIntrinsicGas() + result.execResult.executionGasUsed : null,
    calldata_bytes: tx.data.length, calldata_sha256: sha256(tx.data),
    local_vm_wall_ms: performance.now() - started,
    evm_active_memory_words_at_exit: result.execResult.runState ? Number(result.execResult.runState.memoryWordCount) : null,
    evm_memory_expansion_cost_at_exit: result.execResult.runState ? Number(result.execResult.runState.highestMemCost) : null,
    exception: result.execResult.exceptionError?.error ?? null,
    modexp_call_count: trace.length, modexp_execution_gas: trace.reduce((sum, x) => sum + x.execution_gas, 0),
    modexp_calls: [...trace],
    top_level_opcode_gas: topOpcodeGas, staticcall_opcode_gas: staticcallOpcodeGas,
    gas_markers: [...gasMarkers],
  } };
}
const outputPath = path.resolve(root, process.argv[2] ?? 'experiments/g5_c3_raw/fp_mul_quarter_profile.json');
assert.ok(!fs.existsSync(outputPath), 'B20 output exists; inspect it instead of repeating the experiment');
console.log('G5-C3-B20: one deployment; isolated Fp only, no Miller');
const deployment = await transact((await factory.getDeployTransaction()).data);
assert.equal(deployment.result.receipt.status, 1);
const address = deployment.result.createdAddress;
assert.ok(address);
const runtime = await vm.stateManager.getCode(address);
assert.ok(runtime.length > 0 && runtime.length <= 24576);
async function profile(label, left, right, expected) {
  const transaction = await transact(iface.encodeFunctionData('profileFpMul', [left, right]), address);
  const m = transaction.measurement;
  let computed = null, windowGas = null, freeBefore = null, freeAfter = null, decodeError = null;
  if (m.receipt_status === 1) {
    try {
      const decoded = iface.decodeFunctionResult('profileFpMul', bytesToHex(transaction.result.execResult.returnValue));
      [computed, windowGas, freeBefore, freeAfter] = decoded;
    } catch (error) { decodeError = error.message; }
  }
  const begin = m.gas_markers[0], end = m.gas_markers.at(-1);
  const allocator = freeAfter !== null && freeBefore !== null ? Number(freeAfter - freeBefore) : null;
  const checks = {
    transaction_succeeded: m.receipt_status === 1 && m.exception === null,
    exact_canonical_output: computed === expected && computed !== null && BigInt(computed) < P,
    two_successful_modexps: m.modexp_call_count === 2 && m.modexp_calls.every(x => x.success && x.return_bytes === 64),
    zero_refund: m.gas_refund === 0,
    execution_gas_reconciled: m.top_level_opcode_gas + m.modexp_execution_gas === m.execution_gas,
    total_gas_reconciled: m.total_gas === Math.max(m.intrinsic_gas + m.execution_gas, m.minimum_gas_limit_including_calldata_floor),
    four_window_and_call_gas_markers: m.gas_markers.length === 4,
    window_gas_reconciled: windowGas !== null && begin !== undefined && end !== undefined
      && begin.gas_left_before - end.gas_left_before === Number(windowGas),
    window_within_execution: windowGas !== null && Number(windowGas) > m.modexp_execution_gas && Number(windowGas) <= m.execution_gas,
    allocation_matches_baseline: allocator === baseline.fp.allocator_growth_in_window_bytes,
  };
  const memoryCost = words => 3 * words + Math.floor(words * words / 512);
  return { operation: label, left_hex: left, right_hex: right, expected_hex: expected, computed_hex: computed,
    verified: Object.values(checks).every(Boolean), checks, decode_error: decodeError, ...m,
    instrumented_arithmetic_window_gas: windowGas === null ? null : Number(windowGas),
    window_minus_precompile_execution_gas: windowGas === null ? null : Number(windowGas) - m.modexp_execution_gas,
    window_evm_memory_expansion_gas: begin && end ? memoryCost(end.memory_words_before) - memoryCost(begin.memory_words_before) : null,
    window_memory_words_before: begin?.memory_words_before ?? null, window_memory_words_after: end?.memory_words_before ?? null,
    window_staticcall_opcode_gas: begin && end ? end.staticcall_gas_before - begin.staticcall_gas_before : null,
    execution_gas_outside_window: windowGas === null ? null : m.execution_gas - Number(windowGas),
    free_memory_before_bytes: freeBefore === null ? null : Number(freeBefore),
    free_memory_after_bytes: freeAfter === null ? null : Number(freeAfter), allocator_growth_in_window_bytes: allocator };
}
console.log('G5-C3-B20: ONE saved public performance input');
const fp = { comparison_performance_sample: true,
  ...await profile('Fp multiply, fixed public input', fpLeft, fpRight, fpExpected) };
const boundaries = [];
if (fp.verified) {
  console.log('G5-C3-B20: four planned correction/carry correctness inputs only');
  for (const entry of cases) {
    const actual = await profile(entry.id, entry.input_a_hex, entry.input_b_hex, entry.expected_hex);
    boundaries.push({ id: entry.id, comparison_performance_sample: false,
      internal_difference_hex_from_host_identity: entry.reduced_square_difference_hex,
      host_derived_correction_k: entry.correction_k,
      host_derived_kp_low_product_carry: entry.expected_kp_low_product_carry,
      host_derived_sum_low_carry: entry.expected_sum_low_carry,
      branch_conditions_are_host_derived_not_evm_instruction_coverage: true, evm_executed: true, ...actual });
    if (!actual.verified) break;
  }
}
const allCorrect = fp.verified && boundaries.length === 4 && boundaries.every(x => x.verified);
const measuredImprovement = allCorrect && fp.instrumented_arithmetic_window_gas < baseline.fp.instrumented_arithmetic_window_gas;
const status = !allCorrect ? 'FAIL_FP_QUARTER_CORRECTNESS_OR_ACCOUNTING'
  : measuredImprovement ? 'PASS_FP_QUARTER_FIXED_CASES_AND_WINDOW_REDUCTION_ONLY'
    : 'STOP_FP_QUARTER_CORRECTNESS_PASS_NO_WINDOW_REDUCTION';
const comparison = {
  baseline_window_gas: baseline.fp.instrumented_arithmetic_window_gas,
  candidate_window_gas: fp.instrumented_arithmetic_window_gas,
  window_gas_reduction_percent: fp.instrumented_arithmetic_window_gas === null ? null
    : 100 * (1 - fp.instrumented_arithmetic_window_gas / baseline.fp.instrumented_arithmetic_window_gas),
  baseline_execution_gas: baseline.fp.execution_gas, candidate_execution_gas: fp.execution_gas,
  baseline_total_gas: baseline.fp.total_gas, candidate_total_gas: fp.total_gas,
  baseline_allocator_growth_bytes: baseline.fp.allocator_growth_in_window_bytes,
  candidate_allocator_growth_bytes: fp.allocator_growth_in_window_bytes,
  baseline_window_memory_gas: baseline.fp.window_evm_memory_expansion_gas,
  candidate_window_memory_gas: fp.window_evm_memory_expansion_gas,
  baseline_runtime_bytes: baseline.runtime_bytecode_bytes, candidate_runtime_bytes: runtime.length,
  baseline_deployment_gas: baseline.deployment.total_gas, candidate_deployment_gas: deployment.measurement.total_gas,
  modexp_count_unchanged: fp.modexp_call_count === baseline.fp.modexp_call_count,
  calldata_unchanged: fp.calldata_sha256 === baseline.fp.calldata_sha256,
};
const nextGate = measuredImprovement
  ? 'B21: integrate ONLY the exact measured quarterMod _mul into a separate B15-based Miller variant; run the same prefix positive/wrong-result pair only; no full64 retry'
  : 'Stop this quarterMod candidate and retain B15/B6; read-only route decision, no automatic alternative optimization or Miller retry';
const report = {
  schema: 'VV/G5-C3/FP-MUL-QUARTER-PROFILE/v1', gate: 'G5-C3-B20', status,
  classification: 'one-fixed-input-isolated-fp-multiply-comparison-plus-four-necessary-correctness-cases',
  derived_nonbenchmark: false, recorded_at_utc: new Date().toISOString(),
  git_commit: metadata('git', ['rev-parse', 'HEAD']), git_dirty: metadata('git', ['status', '--porcelain']).length > 0,
  cpu: os.cpus()[0].model, os: os.platform(), arch: os.arch(), node_version: process.version,
  compiler: solc.version(), compiler_settings: settings, hardfork: common.hardfork(),
  execution_scope: 'one deployment, one fixed public Fp comparison and four planned correctness cases; no Fp2/Miller/proving/RPC',
  transaction_gas_limit: 20_000_000, block_gas_limit: 30_000_000,
  baseline: baselineName, baseline_sha256: sha256(baselineBytes), baseline_source_sha256: sha256(baselineSource),
  route_plan: planName, route_plan_sha256: sha256(planBytes),
  source_fixture: baseline.source_fixture, source_fixture_sha256: sha256(sourceBytes),
  fixture: baseline.fixture, fixture_sha256: sha256(fixtureBytes), aggregate_proof_sha256: sha256(wire),
  operand_source: baseline.operand_source, contract_source: contractName, contract_source_sha256: sha256(contractSource),
  runner_source: path.relative(root, fileURLToPath(import.meta.url)),
  runner_source_sha256: sha256(fs.readFileSync(fileURLToPath(import.meta.url))),
  source_changes: 'only _mul local halfMod helper replaced with quarterMod and two calls fused; contract name changed',
  all_other_source_bytes_unchanged: true, arithmetic_source_bodies_unchanged: unchangedFunctions,
  quarter_modulus_specific_carry_certificates_verified: true,
  package_lock_sha256: sha256(read('contracts/package-lock.json')), cargo_lock_sha256: sha256(read('Cargo.lock')),
  official_manifest_sha256: sha256(read('configs/protocol_manifest.json')),
  prototype_manifest_sha256: sha256(read('configs/protocol_manifest.JUBJUB-PROTOTYPE.json')),
  runtime_bytecode_bytes: runtime.length, deployed_runtime_sha256: sha256(runtime), deployed_contract: address.toString(),
  deployment: deployment.measurement, fp, boundaries, comparison,
  correctness_and_accounting_passed: allCorrect, all_boundaries_verified: boundaries.length === 4 && boundaries.every(x => x.verified),
  measured_window_improvement: measuredImprovement, optimization_implemented: true,
  optimization_integrated_into_miller: false, candidate_accepted_for_separate_prefix_gate: measuredImprovement,
  performance_comparison_samples: 1, arithmetic_transactions: 1 + boundaries.length,
  evm_transactions_executed: 2 + boundaries.length,
  expected_modexp_count_per_arithmetic_transaction: 2,
  historical_full_attempt_result: fullName, historical_full_attempt_result_sha256: sha256(fullBytes),
  latest_full_attempt_gate: fullBaseline.gate, latest_full_attempt_result: fullName,
  latest_prefix_gate: plan.latest_prefix_gate,
  target_verifier_status: fullBaseline.target_verifier_status, target_verifier_status_source: fullName,
  primitive_comparison_only: true, miller_run: false, full_loop_attempts: 0,
  full_miller_loop_verified_in_evm: false, full_miller_verification_gas: null, estimated_full_gas: null, completion_percent: null,
  evm_final_exponentiation_implemented: false, pairing_to_gt_evm_verified: false,
  exact_snarkpack_evm_verified: false, complete_g5c3: false, snarkpack_verification_gas: null,
  protocol_capabilities_changed: false, setup_run: false, prove_run: false, private_witness_emitted: false,
  next_gate: nextGate,
  limitations: [
    'One same-input gas comparison only; four correction/carry cases are correctness tests, not extra performance samples',
    'Host references and carry flags generate/describe inputs; actual contract performs both MODEXP squares and the complete constrained-by-EVM arithmetic',
    'Isolated instrumented call-site costs are not per-operation constants or a full64 gas estimate',
    'Only Fp is executed; the retained Fp2 ABI/body is not tested by this gate',
    'No Miller integration, prefix/full retry, final exponentiation, GT target binding or complete SnarkPack EVM verification',
    'Allocator growth, active EVM memory, expansion gas and observer wall time have distinct meanings; wall time is not a statistical result',
  ],
};
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ status, all_correct: allCorrect, measured_window_improvement: measuredImprovement,
  comparison, arithmetic_transactions: report.arithmetic_transactions, next_gate: nextGate }));
console.log('wrote ' + path.relative(root, outputPath));
if (!allCorrect) process.exitCode = 1;

