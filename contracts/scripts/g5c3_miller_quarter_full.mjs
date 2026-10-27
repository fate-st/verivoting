// B23: one unchanged-runtime full64 attempt. No retries or extra test matrix.
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
const read = name => fs.readFileSync(path.join(root, name));
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const metadata = (cmd, args) => execFileSync(cmd, args, { cwd: root, encoding: 'utf8' }).trim();
const outputPath = path.resolve(root, process.argv[2] ?? 'experiments/g5_c3_raw/miller_quarter_full_evm.json');
assert.ok(!fs.existsSync(outputPath), 'B23 output exists; inspect it, do not repeat the attempt');
let phase = 'PRECONDITION';
const report = {
  schema: 'VV/G5-C3/MILLER-QUARTER-FULL/v1', gate: 'G5-C3-B23',
  classification: 'one-fixed-input-full64-conformance-attempt-at-fixed20M', derived_nonbenchmark: false,
  recorded_at_utc: new Date().toISOString(),
  git_commit: metadata('git', ['rev-parse', 'HEAD']), git_dirty: metadata('git', ['status', '--porcelain']).length > 0,
  cpu: os.cpus()[0].model, os: os.platform(), arch: os.arch(), node_version: process.version,
  runner_source: path.relative(root, fileURLToPath(import.meta.url)),
  runner_source_sha256: sha256(fs.readFileSync(fileURLToPath(import.meta.url))),
  execution_scope: 'one deployment and ONE full64 positive only; same B21 runtime, fixed20M transaction/30M block; no RPC',
  transaction_gas_limit: 20_000_000, block_gas_limit: 30_000_000,
  evm_transactions_submitted: 0, evm_transactions_executed: 0, full_call_submitted: false,
  full_execution_result_received: false, full_attempted: false, full_loop_attempts: 0,
  new_full_variant_tested: false,
  deployment: null, full_attempt: null, preconditions_verified: false,
  miller_run: false, miller_prefix_run: false, full_miller_loop_run: false,
  full_miller_loop_completed: false, full_miller_loop_verified_in_evm: false,
  full_miller_verification_gas: null, estimated_full_gas: null, completion_percent: null,
  full_return_matches_reference: false, full_return_payload_bytes: null, full_return_decode_error: null,
  optimization_implemented: false, contract_changed_by_this_run: false, prefix_checks_executed_this_run: false,
  primitive_boundary_checks_executed_this_run: false, root_step_listener_used: false, large_step_trace_saved: false,
  host_precomputed_lines_or_state_used_as_contract_computation: false,
  evm_final_exponentiation_implemented: false, pairing_to_gt_evm_verified: false,
  exact_snarkpack_evm_verified: false, complete_g5c3: false, snarkpack_verification_gas: null,
  protocol_capabilities_changed: false, setup_run: false, prove_run: false, private_witness_emitted: false,
};

try {
  const decisionName = 'experiments/g5_c3_raw/miller_full_decision.json';
  const decisionBytes = read(decisionName), decision = JSON.parse(decisionBytes);
  const prefixName = decision.prefix_evidence;
  const prefixBytes = read(prefixName), prefix = JSON.parse(prefixBytes);
  const primitiveBytes = read(decision.primitive_evidence), primitive = JSON.parse(primitiveBytes);
  const previousName = decision.historical_full_evidence;
  const previousBytes = read(previousName), previous = JSON.parse(previousBytes);
  const plan = decision.planned_validation;
  Object.assign(report, {
    decision: decisionName, decision_sha256: sha256(decisionBytes),
    baseline: prefixName, baseline_sha256: sha256(prefixBytes),
    primitive_baseline: decision.primitive_evidence, primitive_baseline_sha256: sha256(primitiveBytes),
    previous_full_attempt_gate: previous.gate, previous_full_attempt_result: previousName,
    previous_full_attempt_result_sha256: sha256(previousBytes),
    latest_prefix_gate: prefix.gate, prefix_evidence_source: prefixName,
    latest_full_attempt_gate: previous.gate, latest_full_attempt_result: previousName,
    latest_full_attempt_result_sha256: sha256(previousBytes),
    target_verifier_status: prefix.target_verifier_status, target_verifier_status_source: prefixName,
    contract_source: prefix.contract_source, contract_source_sha256: prefix.contract_source_sha256,
    compiler: prefix.compiler, compiler_settings: prefix.compiler_settings,
  });
  assert.equal(decision.gate, 'G5-C3-B22');
  assert.equal(plan.gate, 'G5-C3-B23');
  assert.equal(plan.executed, false);
  assert.equal(decision.routing_decision.selected_action, 'ONE_BOUNDED_FULL_CONFORMANCE_ATTEMPT_THEN_STOP_ON_FAILURE');
  assert.equal(prefix.gate, 'G5-C3-B21');
  assert.equal(primitive.gate, 'G5-C3-B20');
  assert.equal(previous.gate, 'G5-C3-B18');
  for (const [bytes, expected] of [
    [prefixBytes, decision.prefix_evidence_sha256], [primitiveBytes, decision.primitive_evidence_sha256],
    [previousBytes, decision.historical_full_evidence_sha256],
    [read(decision.historical_formula_evidence), decision.historical_formula_evidence_sha256],
  ]) assert.equal(sha256(bytes), expected);
  assert.equal(sha256(read(decision.analysis_source)), decision.analysis_source_sha256);
  for (const record of [prefix, primitive, previous]) {
    assert.equal(sha256(read(record.runner_source)), record.runner_source_sha256);
    assert.equal(sha256(read(record.contract_source)), record.contract_source_sha256);
  }
  assert.equal(prefix.prefix_conformance_and_accounting_passed, true);
  assert.equal(primitive.correctness_and_accounting_passed, true);
  assert.equal(primitive.measured_window_improvement, true);
  assert.equal(prefix.new_full_variant_tested, false);
  assert.equal(prefix.full_loop_attempts, 0);
  assert.equal(plan.deployments, 1);
  assert.equal(plan.full_positive_transactions, 1);
  assert.equal(plan.total_evm_transactions, 2);
  assert.equal(plan.transaction_gas_limit, 20_000_000);
  assert.equal(plan.block_gas_limit, 30_000_000);
  assert.equal(plan.expected_complete_modexp_calls_from_unchanged_formula, 14_316);
  assert.equal(plan.expected_return_payload_bytes, 768);
  assert.equal(plan.prefix_reruns, 0);
  assert.equal(plan.primitive_boundary_reruns, 0);
  assert.equal(plan.final_exponentiation, false);
  assert.equal(plan.setup_or_prove, false);
  assert.equal(plan.negative_matrix, false);
  const sourceName = prefix.contract_source;
  const contractSource = read(sourceName).toString('utf8');
  assert.equal(sourceName, plan.source);
  assert.equal(sha256(contractSource), plan.source_sha256);
  assert.equal(sha256(contractSource), decision.contract_source_sha256);
  assert.equal(prefix.deployed_runtime_sha256, plan.runtime_sha256);
  assert.equal(prefix.runtime_bytecode_bytes, 7078);
  for (const [name, key] of [
    [prefix.fixture, 'fixture_sha256'], [prefix.source_fixture, 'source_fixture_sha256'],
    [prefix.endpoint_fixture, 'endpoint_fixture_sha256'], ['Cargo.lock', 'cargo_lock_sha256'],
    ['contracts/package-lock.json', 'package_lock_sha256'],
    ['configs/protocol_manifest.json', 'official_manifest_sha256'],
    ['configs/protocol_manifest.JUBJUB-PROTOTYPE.json', 'prototype_manifest_sha256'],
  ]) {
    assert.equal(sha256(read(name)), prefix[key], name);
    assert.equal(sha256(read(name)), previous[key], name);
    if (Object.hasOwn(decision, key)) assert.equal(sha256(read(name)), decision[key], name);
  }
  const fixture = JSON.parse(read(prefix.fixture));
  const publicFixture = JSON.parse(read(prefix.source_fixture));
  assert.equal(fixture.schema, 'VV/G5-C3/MILLER-LOOP/v1');
  assert.equal(fixture.full_steps, 64);
  assert.equal(fixture.private_witness_emitted, false);
  assert.equal(prefix.fixture, plan.fixture);
  assert.equal(prefix.fixture_sha256, plan.fixture_sha256);
  assert.equal(prefix.source_fixture, plan.source_fixture);
  assert.equal(prefix.source_fixture_sha256, plan.source_fixture_sha256);
  const wire = Buffer.from(publicFixture.aggregate_proof_hex, 'hex');
  assert.equal(wire.length, 5236);
  assert.equal(sha256(wire), prefix.aggregate_proof_sha256);
  assert.equal(sha256(wire), fixture.aggregate_proof_sha256);
  assert.equal(sha256(wire), publicFixture.aggregate_proof_sha256);
  const a = bls12_381.G1.Point.fromBytes(wire.subarray(4468, 4516));
  const b = bls12_381.G2.Point.fromBytes(wire.subarray(4516, 4612));
  const p = a.toAffine(), q = b.toAffine();
  const fpBE = value => {
    assert.ok(value >= 0n && value < bls12_381.fields.Fp.ORDER);
    return Buffer.from(value.toString(16).padStart(128, '0'), 'hex');
  };
  const six = value => [value.c0.c0, value.c0.c1, value.c1.c0, value.c1.c1, value.c2.c0, value.c2.c1];
  const fullReference = bls12_381.pairing(a, b, false); // no final exponentiation
  assert.equal(bytesToHex(Buffer.concat([...six(fullReference.c0), ...six(fullReference.c1)].map(fpBE))),
    fixture.full_miller_fp12_hex);
  const pointInput = bytesToHex(Buffer.concat([fpBE(p.x), fpBE(p.y), fpBE(q.x.c0),
    fpBE(q.x.c1), fpBE(q.y.c0), fpBE(q.y.c1)]));
  assert.equal(pointInput, fixture.points_eip2537_hex);
  const settings = structuredClone(plan.compiler_settings);
  assert.deepEqual(settings, prefix.compiler_settings);
  assert.equal(solc.version(), plan.compiler);
  assert.equal(solc.version(), prefix.compiler);
  const sourceUnit = path.basename(sourceName);
  const compilation = JSON.parse(solc.compile(JSON.stringify({ language: 'Solidity',
    sources: { [sourceUnit]: { content: contractSource } }, settings })));
  for (const error of compilation.errors ?? []) {
    if (error.severity === 'error') throw Error(error.formattedMessage);
  }
  const compiled = compilation.contracts[sourceUnit].Bls12381MillerLoopQuarter;
  const compiledRuntime = Buffer.from(compiled.evm.deployedBytecode.object, 'hex');
  assert.equal(sha256(compiledRuntime), plan.runtime_sha256);
  assert.equal(compiledRuntime.length, prefix.runtime_bytecode_bytes);
  const iface = new Interface(compiled.abi);
  const factory = new ContractFactory(compiled.abi, '0x' + compiled.evm.bytecode.object);
  const fullData = iface.encodeFunctionData('verifyMillerLoopOrRevert', [pointInput, fixture.full_miller_fp12_hex]);
  assert.equal(sha256(hexToBytes(fullData)), plan.full_calldata_sha256);
  assert.equal(sha256(hexToBytes(fullData)), previous.full_attempt.calldata_sha256);
  Object.assign(report, {
    preconditions_verified: true, contract_source_unchanged_since_b21: true,
    compiled_runtime_matches_b21: true, native_full_reference_rechecked_without_final_exponentiation: true,
    expected_full_modexp_calls_from_formula: 14_316, full_steps: 64,
    fixture: prefix.fixture, fixture_sha256: prefix.fixture_sha256,
    source_fixture: prefix.source_fixture, source_fixture_sha256: prefix.source_fixture_sha256,
    endpoint_fixture: prefix.endpoint_fixture, endpoint_fixture_sha256: prefix.endpoint_fixture_sha256,
    aggregate_proof_sha256: sha256(wire),
    package_lock_sha256: prefix.package_lock_sha256, cargo_lock_sha256: prefix.cargo_lock_sha256,
    official_manifest_sha256: prefix.official_manifest_sha256, prototype_manifest_sha256: prefix.prototype_manifest_sha256,
    noble_bls_source_sha256: sha256(read('contracts/node_modules/@noble/curves/abstract/bls.js')),
    noble_tower_source_sha256: sha256(read('contracts/node_modules/@noble/curves/abstract/tower.js')),
    interpreter_source_sha256: sha256(read('contracts/node_modules/@ethereumjs/evm/dist/esm/interpreter.js')),
  });
  const common = new Common({ chain: Mainnet, hardfork: Hardfork.Prague });
  assert.equal(common.hardfork(), prefix.hardfork);
  const vm = await createVM({ common });
  report.hardfork = common.hardfork();
  const senderKey = hexToBytes('0x' + '2f'.repeat(32)); // public local-only key, no RPC
  const sender = createAddressFromPrivateKey(senderKey);
  await vm.stateManager.putAccount(sender, createAccount({ nonce: 0n, balance: 10n ** 24n }));
  const block = createBlock({ header: { number: 1n, gasLimit: 30_000_000n,
    baseFeePerGas: 1n, timestamp: 1_750_000_000n } }, { common, skipConsensusFormatValidation: true });
  let nonce = 0n, trace = [];
  const stack = [];
  vm.evm.events.on('beforeMessage', message => { stack.push(message.to?.toString()); });
  vm.evm.events.on('afterMessage', result => {
    if (stack.pop() === '0x0000000000000000000000000000000000000005') {
      trace.push({ execution_gas: Number(result.execResult.executionGasUsed),
        success: !result.execResult.exceptionError, return_bytes: result.execResult.returnValue.length,
        exception: result.execResult.exceptionError?.error ?? null });
    }
  });
  async function transact(data, to) {
    trace = [];
    const tx = createLegacyTx({ nonce: nonce++, gasLimit: 20_000_000n, gasPrice: 10n,
      value: 0n, data: hexToBytes(data), ...(to ? { to } : {}) }, { common }).sign(senderKey);
    const started = performance.now();
    report.evm_transactions_submitted++;
    const result = await runTx(vm, { tx, block });
    report.evm_transactions_executed++;
    const error = result.execResult.exceptionError;
    let reason = null;
    if (error?.error === 'revert') {
      try { reason = iface.parseError(bytesToHex(result.execResult.returnValue))?.args[0] ?? null; }
      catch { /* retain raw return data */ }
    }
    const state = result.execResult.runState;
    return { result, measurement: {
      transaction_hash: bytesToHex(tx.hash()), receipt_status: result.receipt.status,
      total_gas: Number(result.totalGasSpent), execution_gas: Number(result.execResult.executionGasUsed),
      intrinsic_gas: Number(tx.getIntrinsicGas()), minimum_gas_limit_including_calldata_floor: Number(tx.getMinimumGasLimit()),
      gas_refund: Number(result.gasRefund), transaction_gas_limit: 20_000_000,
      calldata_bytes: tx.data.length, calldata_sha256: sha256(tx.data),
      local_vm_wall_ms_without_root_step_listener: performance.now() - started,
      evm_active_memory_words_at_exit: state ? Number(state.memoryWordCount) : null,
      root_post_execution_state: state ? { program_counter: state.programCounter,
        last_opcode_byte: state.opCode, gas_left_after_execution: Number(state.gasLeft),
        interpretation: 'post-execution VM state, not a pre-charge step event or cumulative source profile' } : null,
      exception: error?.error ?? null, revert_reason: reason, return_hex: bytesToHex(result.execResult.returnValue),
      modexp_call_count: trace.length, modexp_success_count: trace.filter(x => x.success).length,
      modexp_failure_count: trace.filter(x => !x.success).length,
      modexp_return_lengths_all_64: trace.every(x => x.return_bytes === 64),
      modexp_child_gas_all_200: trace.every(x => x.execution_gas === 200),
      modexp_execution_gas: trace.reduce((sum, x) => sum + x.execution_gas, 0),
      modexp_trace_sha256: sha256(JSON.stringify(trace)), modexp_failures: trace.filter(x => !x.success),
      message_stack_balanced: stack.length === 0,
    } };
  }
  phase = 'SETUP';
  console.log('G5-C3-B23: ONE deployment, exact B21 saved runtime and compiler');
  const deployment = await transact((await factory.getDeployTransaction()).data);
  report.deployment = deployment.measurement;
  assert.equal(deployment.result.receipt.status, 1);
  const address = deployment.result.createdAddress;
  assert.ok(address);
  const runtime = await vm.stateManager.getCode(address);
  assert.equal(sha256(runtime), prefix.deployed_runtime_sha256);
  assert.equal(runtime.length, prefix.runtime_bytecode_bytes);
  assert.equal(report.deployment.total_gas, prefix.deployment.total_gas);
  Object.assign(report, { deployed_runtime_matches_b21: true, runtime_bytecode_bytes: runtime.length,
    deployed_runtime_sha256: sha256(runtime), deployed_contract: address.toString() });
  phase = 'FULL';
  console.log('G5-C3-B23: ONE full64 positive, fixed20M; no prefix/boundary/negative rerun');
  report.full_call_submitted = true;
  const full = await transact(fullData, address);
  const m = full.measurement;
  Object.assign(report, { full_execution_result_received: true, full_attempted: true, full_loop_attempts: 1,
    new_full_variant_tested: true,
    full_attempt: m, miller_run: true, full_miller_loop_run: true,
    latest_full_attempt_gate: 'G5-C3-B23', latest_full_attempt_result: path.relative(root, outputPath) });
  delete report.latest_full_attempt_result_sha256; // self-hash cannot be embedded
  let returned = null;
  if (m.receipt_status === 1) {
    try {
      returned = iface.decodeFunctionResult('verifyMillerLoopOrRevert', m.return_hex)[0];
      report.full_return_payload_bytes = (returned.length - 2) / 2;
    } catch (error) { report.full_return_decode_error = error.message; }
  }
  report.full_return_matches_reference = returned === fixture.full_miller_fp12_hex;
  const checks = {
    zero_refund: m.gas_refund === 0,
    transaction_fee_equation: m.total_gas === Math.max(m.intrinsic_gas + m.execution_gas,
      m.minimum_gas_limit_including_calldata_floor),
    no_modexp_failures: m.modexp_failure_count === 0,
    all_observed_modexps_successful: m.modexp_call_count === m.modexp_success_count,
    all_observed_modexp_return_lengths_64: m.modexp_return_lengths_all_64,
    all_observed_modexp_child_gas_200: m.modexp_child_gas_all_200,
    modexp_gas_sum: m.modexp_execution_gas === 200 * m.modexp_call_count,
    message_stack_balanced: m.message_stack_balanced,
  };
  report.common_result_checks = checks;
  const commonPass = Object.values(checks).every(Boolean);
  const pass = commonPass && m.receipt_status === 1 && m.exception === null
    && report.full_return_matches_reference && report.full_return_payload_bytes === 768
    && m.modexp_call_count === 14_316 && m.modexp_success_count === 14_316;
  const oog = commonPass && m.receipt_status === 0 && m.exception === 'out of gas'
    && m.total_gas === 20_000_000 && m.return_hex === '0x';
  report.result_class = pass ? 'PASS' : oog ? 'OOG' : 'OTHER_FAILURE';
  report.status = pass ? 'PASS_FULL64_QUARTER_MILLER_CONFORMANCE_ONLY'
    : oog ? 'BLOCKED_QUARTER_SOFTWARE_MILLER_EXCEEDS_20M_FROZEN'
      : 'FAIL_QUARTER_FULL_MILLER_CONFORMANCE_OR_ACCOUNTING';
  report.target_verifier_status = pass ? 'BLOCKED_FINAL_EXPONENTIATION_AND_GT_TARGET_BINDING_MISSING'
    : oog ? 'BLOCKED_QUARTER_SOFTWARE_MILLER_EXCEEDS_20M_FROZEN'
      : 'BLOCKED_QUARTER_FULL_MILLER_CONFORMANCE_FAILED_FROZEN';
  report.target_verifier_status_source = path.relative(root, outputPath);
  report.full_miller_loop_completed = pass;
  report.full_miller_loop_verified_in_evm = pass;
  report.full_miller_verification_gas = pass ? m.total_gas : null;
  report.failed_attempt_gas_is_capped_spend_not_completed_loop_cost = oog;
  report.software_route_frozen_by_failure = !pass;
  report.next_gate = pass
    ? 'Read-only decision for a minimal final-exponentiation component on the saved full result; no automatic implementation or full retry'
    : 'Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation';
  phase = 'COMPLETE';
} catch (error) {
  report.host_error = { name: error.name, message: error.message, code: error.code ?? null };
  report.failure_phase = phase;
  report.result_class = phase === 'PRECONDITION' ? 'ABORT_PRECONDITION' : phase === 'SETUP' ? 'ABORT_SETUP' : 'OTHER_FAILURE';
  report.status = phase === 'PRECONDITION' ? 'ABORT_B23_PRECONDITION'
    : phase === 'SETUP' ? 'ABORT_B23_SETUP' : 'FAIL_B23_HOST_EXECUTION';
  report.software_route_frozen_by_failure = true;
  report.next_gate = 'Read-only diagnosis of saved abort/failure; no automatic redeploy, arithmetic change or full retry';
}
report.software_route_freeze_scope = 'automatic arithmetic optimization and full retries on this fixed20M software route only; not all project work';
report.full_measurement_is_single_fixed_input_not_statistical = true;
report.limitations = [
  'Success establishes fixed-vector Miller conformance only, never final exponentiation/GT binding/complete SnarkPack',
  'Failed spend and successful MODEXP counts are not completed-loop gas estimates or completion percentages',
  'This run changes no arithmetic/compiler/curve/Suite/Manifest or gas budget and reruns no prefix/boundary matrix',
  'Post-execution VM state is not a pre-charge step trace; wall time has different observation overhead from B18 and is not a statistical comparison',
  'The20M cap is an experiment budget, not a proven target-chain permanent hard limit',
];
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ status: report.status, result_class: report.result_class,
  transactions: report.evm_transactions_executed, full_attempts: report.full_loop_attempts,
  total_gas: report.full_attempt?.total_gas ?? null,
  successful_modexps: report.full_attempt?.modexp_success_count ?? null,
  full_miller_verification_gas: report.full_miller_verification_gas,
  software_route_frozen: report.software_route_frozen_by_failure, next_gate: report.next_gate }));
console.log('wrote ' + path.relative(root, outputPath));
if (!['PASS', 'OOG'].includes(report.result_class)) process.exitCode = 1;
