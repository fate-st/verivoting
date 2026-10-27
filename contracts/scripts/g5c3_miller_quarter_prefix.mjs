// B21: exact B20 quarterMod _mul in a B15-based variant; original prefix pair ONLY.
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
const baselineName = 'experiments/g5_c3_raw/miller_sub_prefix_evm.json';
const baselineBytes = read(baselineName);
const baseline = JSON.parse(baselineBytes);
const primitiveName = 'experiments/g5_c3_raw/fp_mul_quarter_profile.json';
const primitiveBytes = read(primitiveName);
const primitive = JSON.parse(primitiveBytes);
const fullName = primitive.latest_full_attempt_result;
const fullBytes = read(fullName);
const fullBaseline = JSON.parse(fullBytes);
const baselineSource = read(baseline.contract_source).toString('utf8');
const primitiveSource = read(primitive.contract_source).toString('utf8');
const fixtureBytes = read(baseline.fixture);
const fixture = JSON.parse(fixtureBytes);
const publicBytes = read(baseline.source_fixture);
const source = JSON.parse(publicBytes);
const endpointBytes = read(baseline.endpoint_fixture);
const endpoint = JSON.parse(endpointBytes);
const sourceName = 'contracts/src/Bls12381MillerLoopQuarter.sol';
const { Fp, Fp12 } = bls12_381.fields;
const packages = JSON.parse(read('contracts/package-lock.json'));
assert.equal(baseline.gate, 'G5-C3-B15');
assert.equal(primitive.gate, 'G5-C3-B20');
assert.equal(primitive.correctness_and_accounting_passed, true);
assert.equal(primitive.measured_window_improvement, true);
assert.equal(primitive.candidate_accepted_for_separate_prefix_gate, true);
assert.equal(primitive.all_boundaries_verified, true);
assert.equal(sha256(primitiveSource), primitive.contract_source_sha256);
assert.equal(sha256(read(primitive.runner_source)), primitive.runner_source_sha256);
assert.equal(sha256(read(primitive.baseline)), primitive.baseline_sha256);
assert.equal(fullBaseline.gate, 'G5-C3-B18');
assert.equal(sha256(fullBytes), primitive.historical_full_attempt_result_sha256);
assert.equal(sha256(read(fullBaseline.contract_source)), fullBaseline.contract_source_sha256);
assert.equal(sha256(read(fullBaseline.runner_source)), fullBaseline.runner_source_sha256);
assert.equal(sha256(baselineBytes), fullBaseline.prefix_evidence_sha256);
assert.equal(baseline.prefix_correctness_verified_in_evm, true);
assert.equal(baseline.prefix_valid.modexp_call_count, 516);
assert.equal(baseline.fixture_sha256, sha256(fixtureBytes));
assert.equal(baseline.source_fixture_sha256, sha256(publicBytes));
assert.equal(baseline.endpoint_fixture_sha256, sha256(endpointBytes));
assert.equal(baseline.contract_source_sha256, sha256(read(baseline.contract_source)));
assert.equal(baseline.runner_source_sha256, sha256(read('contracts/scripts/g5c3_miller_sub_prefix.mjs')));
for (const [name, key] of [
  ['Cargo.lock', 'cargo_lock_sha256'], ['contracts/package-lock.json', 'package_lock_sha256'],
  ['configs/protocol_manifest.json', 'official_manifest_sha256'],
  ['configs/protocol_manifest.JUBJUB-PROTOTYPE.json', 'prototype_manifest_sha256'],
]) {
  assert.equal(sha256(read(name)), baseline[key], name);
  assert.equal(sha256(read(name)), primitive[key], name);
}
assert.equal(fixture.schema, 'VV/G5-C3/MILLER-LOOP/v1');
assert.equal(fixture.prefix_steps, 2);
assert.equal(fixture.full_steps, 64);
assert.equal(fixture.private_witness_emitted, false);
assert.equal(source.schema, 'VV/G5-C1/SNARKPACK-PUBLIC-FIXTURE/v1');
assert.equal(source.batch_size, 2);
assert.equal(packages.packages['node_modules/@noble/curves'].version, '2.4.0');
const wire = Buffer.from(source.aggregate_proof_hex, 'hex');
assert.equal(wire.length, 5236);
for (const record of [baseline, primitive, fullBaseline, fixture, source, endpoint]) {
  assert.equal(record.aggregate_proof_sha256, sha256(wire));
}
const a = bls12_381.G1.Point.fromBytes(wire.subarray(4468, 4516));
const b = bls12_381.G2.Point.fromBytes(wire.subarray(4516, 4612));
const p = a.toAffine();
const q = b.toAffine();
const fpBE = (value) => {
  assert.ok(value >= 0n && value < Fp.ORDER);
  return Buffer.from(value.toString(16).padStart(128, '0'), 'hex');
};
const six = (value) => [value.c0.c0, value.c0.c1, value.c1.c0,
  value.c1.c1, value.c2.c0, value.c2.c1];
const encode12 = (value) => bytesToHex(Buffer.concat([
  ...six(value.c0), ...six(value.c1),
].map(fpBE)));
const pointInput = bytesToHex(Buffer.concat([fpBE(p.x), fpBE(p.y),
  fpBE(q.x.c0), fpBE(q.x.c1), fpBE(q.y.c0), fpBE(q.y.c1)]));
assert.equal(pointInput, fixture.points_eip2537_hex);
// The fixed existing fixture is independently rechecked, not regenerated.
// Noble conjugates even a truncated loop: undo it for the raw two-step prefix.
const lines = bls12_381.utils.calcPairingPrecomputes(b);
const prefix = Fp12.conjugate(bls12_381.millerLoopBatch([[lines.slice(0, 2), p.x, p.y]], false));
assert.equal(encode12(prefix), fixture.prefix_fp12_hex);
const wrongBytes = Buffer.from(hexToBytes(fixture.prefix_fp12_hex));
const first = BigInt(`0x${wrongBytes.subarray(0, 64).toString('hex')}`);
fpBE((first + 1n) % Fp.ORDER).copy(wrongBytes);
const wrongExpected = bytesToHex(wrongBytes);
const contractSource = read(sourceName).toString('utf8');
function functionBody(text, name) {
  const start = text.indexOf(`    function ${name}(`);
  assert.ok(start >= 0, name);
  const brace = text.indexOf('{', start);
  let depth = 1;
  for (let i = brace + 1; i < text.length; i++) {
    if (text[i] === '{') depth++;
    if (text[i] === '}') depth--;
    if (!depth) return text.slice(start, i + 1);
  }
  throw Error(`unterminated function ${name}`);
}
const expectedSource = baselineSource.replace('contract Bls12381MillerLoopSubLowLevel {',
  'contract Bls12381MillerLoopQuarter {').replace(functionBody(baselineSource, '_mul'),
    functionBody(primitiveSource, '_mul'));
assert.equal(contractSource, expectedSource, 'only contract name and exact measured B20 _mul may change');
assert.equal(functionBody(contractSource, '_mul'), functionBody(primitiveSource, '_mul'));
const unchangedFunctions = [...baselineSource.matchAll(/^    function (\w+)\(/gm)]
  .map(x => x[1]).filter(x => x !== '_mul');
for (const name of unchangedFunctions) assert.equal(functionBody(contractSource, name), functionBody(baselineSource, name));
const settings = { optimizer: { enabled: true, runs: 200 }, evmVersion: 'prague',
  outputSelection: { '*': { '*': ['abi', 'evm.bytecode.object', 'evm.deployedBytecode.object'] } } };
assert.equal(solc.version(), baseline.compiler);
assert.deepEqual(settings, baseline.compiler_settings);
assert.deepEqual(settings, primitive.compiler_settings);
const compilation = JSON.parse(solc.compile(JSON.stringify({ language: 'Solidity',
  sources: { 'Bls12381MillerLoopQuarter.sol': { content: contractSource } }, settings })));
for (const error of compilation.errors ?? []) {
  if (error.severity === 'error') throw Error(error.formattedMessage);
  console.warn(error.formattedMessage);
}
const compiled = compilation.contracts['Bls12381MillerLoopQuarter.sol'].Bls12381MillerLoopQuarter;
const iface = new Interface(compiled.abi);
const factory = new ContractFactory(compiled.abi, `0x${compiled.evm.bytecode.object}`);
const common = new Common({ chain: Mainnet, hardfork: Hardfork.Prague });
const vm = await createVM({ common });
assert.equal(common.hardfork(), baseline.hardfork);
// Public deterministic local-only key. No RPC, real chain or user account.
const senderKey = hexToBytes(`0x${'2e'.repeat(32)}`);
const sender = createAddressFromPrivateKey(senderKey);
await vm.stateManager.putAccount(sender, createAccount({ nonce: 0n, balance: 10n ** 24n }));
const block = createBlock({ header: { number: 1n, gasLimit: 30_000_000n,
  baseFeePerGas: 1n, timestamp: 1_750_000_000n } }, { common, skipConsensusFormatValidation: true });
let nonce = 0n;
let trace = [];
const stack = [];
vm.evm.events.on('beforeMessage', (message) => { stack.push(message.to?.toString()); });
vm.evm.events.on('afterMessage', (result) => {
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
  const result = await runTx(vm, { tx, block });
  const error = result.execResult.exceptionError;
  let reason = null;
  if (error?.error === 'revert') {
    try { reason = iface.parseError(bytesToHex(result.execResult.returnValue))?.args[0] ?? null; }
    catch { /* return data remains in the report */ }
  }
  return { result, measurement: {
    transaction_hash: bytesToHex(tx.hash()), receipt_status: result.receipt.status,
    total_gas: Number(result.totalGasSpent), execution_gas: Number(result.execResult.executionGasUsed),
    intrinsic_gas: Number(tx.getIntrinsicGas()), minimum_gas_limit_including_calldata_floor: Number(tx.getMinimumGasLimit()),
    gas_refund: Number(result.gasRefund),
    transaction_gas_limit: 20_000_000, calldata_bytes: tx.data.length, calldata_sha256: sha256(tx.data),
    local_vm_wall_ms: performance.now() - started,
    evm_active_memory_words_at_exit: result.execResult.runState
      ? Number(result.execResult.runState.memoryWordCount) : null,
    exception: error?.error ?? null, revert_reason: reason, return_hex: bytesToHex(result.execResult.returnValue),
    modexp_call_count: trace.length, modexp_success_count: trace.filter((x) => x.success).length,
    modexp_failure_count: trace.filter((x) => !x.success).length,
    modexp_execution_gas: trace.reduce((sum, x) => sum + x.execution_gas, 0),
    modexp_trace_sha256: sha256(JSON.stringify(trace)), modexp_failures: trace.filter((x) => !x.success),
  } };
}
const outputPath = path.resolve(root, process.argv[2] ?? 'experiments/g5_c3_raw/miller_quarter_prefix_evm.json');
assert.ok(!fs.existsSync(outputPath), 'B21 output exists; inspect it instead of rerunning');
console.log('G5-C3-B21: one deployment, exact B20 _mul in an otherwise unchanged B15 variant');
const deployment = await transact((await factory.getDeployTransaction()).data);
assert.equal(deployment.result.receipt.status, 1);
const address = deployment.result.createdAddress;
assert.ok(address);
const runtime = await vm.stateManager.getCode(address);
assert.ok(runtime.length > 0 && runtime.length <= 24576);
const validData = iface.encodeFunctionData('verifyMillerPrefixOrRevert', [pointInput, 2, fixture.prefix_fp12_hex]);
const wrongData = iface.encodeFunctionData('verifyMillerPrefixOrRevert', [pointInput, 2, wrongExpected]);
const vd = iface.decodeFunctionData('verifyMillerPrefixOrRevert', validData);
const wd = iface.decodeFunctionData('verifyMillerPrefixOrRevert', wrongData);
assert.equal(vd[0], wd[0]); assert.equal(vd[1], wd[1]); assert.notEqual(vd[2], wd[2]);
assert.equal(sha256(hexToBytes(validData)), baseline.prefix_valid.calldata_sha256);
assert.equal(sha256(hexToBytes(wrongData)), baseline.prefix_wrong_target.calldata_sha256);
console.log('G5-C3-B21: original prefix positive, then the original wrong-target call only');
const valid = await transact(validData, address);
let returned = null, decodeError = null;
if (valid.measurement.receipt_status === 1) {
  try { returned = iface.decodeFunctionResult('verifyMillerPrefixOrRevert', valid.measurement.return_hex)[0]; }
  catch (error) { decodeError = error.message; }
}
const returnCorrect = returned === fixture.prefix_fp12_hex;
const wrong = returnCorrect ? await transact(wrongData, address) : null;
const wrongRejected = wrong?.measurement.receipt_status === 0 && wrong?.measurement.exception === 'revert'
  && wrong?.measurement.revert_reason === 'INVALID_MILLER_RESULT';
const checks = [valid, ...(wrong ? [wrong] : [])].map((item, index) => {
  const m = item.measurement;
  const previous = index === 0 ? baseline.prefix_valid : baseline.prefix_wrong_target;
  return { case: index === 0 ? 'valid' : 'wrong_target',
    expected_modexp_count: m.modexp_call_count === 516,
    all_modexps_succeeded: m.modexp_success_count === 516 && m.modexp_failure_count === 0,
    modexp_gas_unchanged: m.modexp_execution_gas === 103_200,
    modexp_trace_matches_baseline: m.modexp_trace_sha256 === previous.modexp_trace_sha256,
    calldata_matches_baseline: m.calldata_sha256 === previous.calldata_sha256,
    zero_refund: m.gas_refund === 0,
    gas_accounting_reconciled: m.total_gas === Math.max(m.intrinsic_gas + m.execution_gas,
      m.minimum_gas_limit_including_calldata_floor) };
});
const accountingPass = checks.length === 2 && checks.every(x => Object.entries(x)
  .filter(([key]) => key !== 'case').every(([, value]) => value));
const prefixPass = returnCorrect && wrongRejected && accountingPass;
const memoryReduced = valid.measurement.evm_active_memory_words_at_exit < baseline.prefix_valid.evm_active_memory_words_at_exit;
const gasReduced = prefixPass && valid.measurement.total_gas < baseline.prefix_valid.total_gas;
const status = !prefixPass ? 'FAIL_QUARTER_MILLER_PREFIX_CONFORMANCE_OR_ACCOUNTING'
  : gasReduced ? 'PASS_QUARTER_MILLER_PREFIX_INTEGRATION_ONLY'
    : 'STOP_QUARTER_MILLER_PREFIX_CORRECT_NO_GAS_REDUCTION';
const nextGate = gasReduced
  ? 'B22: read-only stop/continue decision for the saved quarter-prefix variant; no automatic full64 retry, final exponentiation or new primitive optimization'
  : 'Retain B15 and stop quarter integration; no automatic full64 retry or new primitive optimization';
const report = {
  schema: 'VV/G5-C3/MILLER-QUARTER-PREFIX/v1', gate: 'G5-C3-B21', status,
  classification: 'same-fixed-prefix-quarter-mul-integration-positive-and-one-wrong-target-only',
  derived_nonbenchmark: false,
  target_verifier_status: prefixPass ? 'BLOCKED_QUARTER_MILLER_FULL_VARIANT_UNTESTED' : 'BLOCKED_QUARTER_PREFIX_CONFORMANCE_FAILED',
  target_verifier_status_source: path.relative(root, outputPath),
  historical_full_variant_status: fullBaseline.target_verifier_status,
  recorded_at_utc: new Date().toISOString(), git_commit: metadata('git', ['rev-parse', 'HEAD']),
  git_dirty: metadata('git', ['status', '--porcelain']).length > 0,
  cpu: os.cpus()[0].model, os: os.platform(), arch: os.arch(), node_version: process.version,
  compiler: solc.version(), compiler_settings: settings, hardfork: common.hardfork(),
  execution_scope: 'one deployment and original two-step prefix positive/wrong-target pair only; no full64, primitive boundary or proving rerun',
  optimization_scope: 'only exact B20 _mul integrated into B15; all other source bytes unchanged except contract name',
  baseline: baselineName, baseline_sha256: sha256(baselineBytes),
  baseline_contract_source_sha256: baseline.contract_source_sha256,
  primitive_baseline: primitiveName, primitive_baseline_sha256: sha256(primitiveBytes),
  primitive_contract_source: primitive.contract_source, primitive_contract_source_sha256: sha256(primitiveSource),
  exact_b20_mul_integrated: true, all_other_contract_source_unchanged: true,
  source_bodies_unchanged: unchangedFunctions,
  fixture: baseline.fixture, fixture_sha256: sha256(fixtureBytes),
  source_fixture: baseline.source_fixture, source_fixture_sha256: sha256(publicBytes),
  endpoint_fixture: baseline.endpoint_fixture, endpoint_fixture_sha256: sha256(endpointBytes),
  aggregate_proof_sha256: sha256(wire), contract_source: sourceName, contract_source_sha256: sha256(contractSource),
  runner_source: path.relative(root, fileURLToPath(import.meta.url)),
  runner_source_sha256: sha256(fs.readFileSync(fileURLToPath(import.meta.url))),
  noble_bls_source_sha256: sha256(read('contracts/node_modules/@noble/curves/abstract/bls.js')),
  noble_tower_source_sha256: sha256(read('contracts/node_modules/@noble/curves/abstract/tower.js')),
  package_lock_sha256: sha256(read('contracts/package-lock.json')), cargo_lock_sha256: sha256(read('Cargo.lock')),
  official_manifest_sha256: sha256(read('configs/protocol_manifest.json')),
  prototype_manifest_sha256: sha256(read('configs/protocol_manifest.JUBJUB-PROTOTYPE.json')),
  runtime_bytecode_bytes: runtime.length, deployed_runtime_sha256: sha256(runtime),
  deployed_contract: address.toString(), transaction_gas_limit: 20_000_000, block_gas_limit: 30_000_000,
  prefix_steps: 2, prefix_correctness_verified_in_evm: returnCorrect,
  wrong_prefix_target_rejected_in_evm: wrongRejected,
  prefix_conformance_and_accounting_passed: prefixPass, per_case_accounting_checks: checks,
  prefix_return_decode_error: decodeError, expected_prefix_modexp_calls: 516,
  prefix_modexp_calls_unchanged: valid.measurement.modexp_call_count === baseline.prefix_valid.modexp_call_count,
  prefix_memory_reduced: memoryReduced, prefix_gas_reduced: gasReduced,
  comparison: {
    baseline_prefix_gas: baseline.prefix_valid.total_gas, optimized_prefix_gas: valid.measurement.total_gas,
    prefix_gas_saved: baseline.prefix_valid.total_gas - valid.measurement.total_gas,
    prefix_gas_reduction_percent: 100 * (1 - valid.measurement.total_gas / baseline.prefix_valid.total_gas),
    baseline_prefix_memory_words: baseline.prefix_valid.evm_active_memory_words_at_exit,
    optimized_prefix_memory_words: valid.measurement.evm_active_memory_words_at_exit,
    prefix_memory_reduction_percent: 100 * (1 - valid.measurement.evm_active_memory_words_at_exit / baseline.prefix_valid.evm_active_memory_words_at_exit),
    baseline_prefix_modexp_calls: baseline.prefix_valid.modexp_call_count,
    optimized_prefix_modexp_calls: valid.measurement.modexp_call_count,
    baseline_runtime_bytes: baseline.runtime_bytecode_bytes, optimized_runtime_bytes: runtime.length,
    baseline_deployment_gas: baseline.deployment.total_gas, optimized_deployment_gas: deployment.measurement.total_gas,
  },
  deployment: deployment.measurement, prefix_valid: valid.measurement, prefix_wrong_target: wrong?.measurement ?? null,
  evm_transactions_executed: 2 + (wrong ? 1 : 0), prefix_transactions_executed: 1 + (wrong ? 1 : 0),
  miller_run: true, miller_prefix_run: true, full_miller_loop_run: false, full_loop_attempts: 0, full_attempt: null,
  latest_prefix_gate: prefixPass ? 'G5-C3-B21' : baseline.gate,
  latest_full_attempt_gate: fullBaseline.gate, latest_full_attempt_result: fullName,
  latest_full_attempt_result_sha256: sha256(fullBytes),
  latest_full_attempt_is_historical_different_variant: true, new_full_variant_tested: false,
  full_miller_loop_verified_in_evm: false, full_miller_verification_gas: null, estimated_full_gas: null, completion_percent: null,
  host_precomputed_lines_or_state_used_as_contract_computation: false,
  evm_final_exponentiation_implemented: false, pairing_to_gt_evm_verified: false,
  exact_snarkpack_evm_verified: false, complete_g5c3: false, snarkpack_verification_gas: null,
  protocol_capabilities_changed: false, setup_run: false, prove_run: false, private_witness_emitted: false,
  next_gate: nextGate,
  limitations: [
    'Prefix-only integration on one fixed public input, not full64 conformance or a statistical/E2E performance result',
    'This quarter variant has zero full64 attempts; B18 is a historical 20M OOG on the different B15-derived runtime',
    'Prefix gas saving does not estimate completed full64 gas or justify an automatic full retry',
    'Manual memory arena remains pinned to solc0.8.30 legacy optimizer200; no generic portability claim',
    'No final exponentiation, exact GT target binding, complete SnarkPack transcript/KZG verifier, Suite/Manifest change or proof rerun',
  ],
};
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ status, prefix_pass: prefixPass, comparison: report.comparison,
  transactions: report.evm_transactions_executed, full_loop_attempts: 0, next_gate: nextGate }));
console.log('wrote ' + path.relative(root, outputPath));
if (!prefixPass) process.exitCode = 1;

