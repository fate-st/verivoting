// One full64 positive attempt on the unchanged B15 contract, bounded by 20M gas.
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
const fullName = baseline.latest_full_attempt_result;
const fullBytes = read(fullName);
const fullBaseline = JSON.parse(fullBytes);
const primitiveName = 'experiments/g5_c3_raw/fp2_sub_lowlevel_profile.json';
const primitiveBytes = read(primitiveName);
const primitive = JSON.parse(primitiveBytes);
const baselineSource = read(baseline.contract_source).toString('utf8');
const primitiveSource = read(primitive.contract_source).toString('utf8');
const fixtureBytes = read(baseline.fixture);
const fixture = JSON.parse(fixtureBytes);
const publicBytes = read(baseline.source_fixture);
const source = JSON.parse(publicBytes);
const endpointBytes = read(baseline.endpoint_fixture);
const endpoint = JSON.parse(endpointBytes);
const sourceName = 'contracts/src/Bls12381MillerLoopSubLowLevel.sol';
const { Fp, Fp12 } = bls12_381.fields;
const packages = JSON.parse(read('contracts/package-lock.json'));
assert.equal(baseline.gate, 'G5-C3-B15');
assert.equal(primitive.gate, 'G5-C3-B14');
assert.equal(primitive.all_boundaries_verified, true);
assert.equal(primitive.measured_window_execution_allocation_improvement, true);
assert.equal(sha256(primitiveSource), primitive.contract_source_sha256);
assert.equal(sha256(read('contracts/scripts/g5c3_fp2_sub_lowlevel.mjs')), primitive.runner_source_sha256);
assert.equal(sha256(read(primitive.baseline)), primitive.baseline_sha256);
assert.equal(fullBaseline.gate, 'G5-C3-B7');
assert.equal(sha256(fullBytes), baseline.latest_full_attempt_result_sha256);
assert.equal(sha256(read(fullBaseline.contract_source)), fullBaseline.contract_source_sha256);
assert.equal(sha256(read('contracts/scripts/g5c3_miller_lowlevel.mjs')), fullBaseline.runner_source_sha256);
assert.equal(sha256(primitiveBytes), baseline.primitive_baseline_sha256);
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
  assert.equal(sha256(read(name)), baseline[key], `${name} changed since baseline`);
  assert.equal(sha256(read(name)), primitive[key], `${name} changed since B14`);
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
// Recheck the frozen full Miller value without final exponentiation.
const fullReference = bls12_381.pairing(a, b, false);
assert.equal(encode12(fullReference), fixture.full_miller_fp12_hex);
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
// The contract, not merely the formulas, is exactly the saved B15 source.
assert.equal(contractSource, baselineSource);
assert.equal(sha256(contractSource), baseline.contract_source_sha256);
assert.equal(functionBody(contractSource, '_sub2'), functionBody(primitiveSource, '_sub2'));
const settings = { optimizer: { enabled: true, runs: 200 }, evmVersion: 'prague',
  outputSelection: { '*': { '*': ['abi', 'evm.bytecode.object', 'evm.deployedBytecode.object'] } } };
assert.equal(solc.version(), baseline.compiler);
assert.deepEqual(settings, baseline.compiler_settings);
const compilation = JSON.parse(solc.compile(JSON.stringify({ language: 'Solidity',
  sources: { 'Bls12381MillerLoopSubLowLevel.sol': { content: contractSource } }, settings })));
for (const error of compilation.errors ?? []) {
  if (error.severity === 'error') throw Error(error.formattedMessage);
  console.warn(error.formattedMessage);
}
const compiled = compilation.contracts['Bls12381MillerLoopSubLowLevel.sol'].Bls12381MillerLoopSubLowLevel;
const iface = new Interface(compiled.abi);
const factory = new ContractFactory(compiled.abi, `0x${compiled.evm.bytecode.object}`);
const common = new Common({ chain: Mainnet, hardfork: Hardfork.Prague });
const vm = await createVM({ common });
assert.equal(common.hardfork(), baseline.hardfork);
// Public deterministic local-only key. No RPC, real chain or user account.
const senderKey = hexToBytes(`0x${'2f'.repeat(32)}`);
const sender = createAddressFromPrivateKey(senderKey);
await vm.stateManager.putAccount(sender, createAccount({ nonce: 0n, balance: 10n ** 24n }));
const block = createBlock({ header: { number: 1n, gasLimit: 30_000_000n,
  baseFeePerGas: 1n, timestamp: 1_750_000_000n } }, { common, skipConsensusFormatValidation: true });
let nonce = 0n;
let trace = [];
let lastRootStep = null;
vm.evm.events.on('step', (step) => {
  if (step.depth === 0) lastRootStep = { pc: step.pc, opcode: step.opcode.name,
    gas_left_before: Number(step.gasLeft), memory_words_before: Number(step.memoryWordCount) };
});
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
  lastRootStep = null;
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
    last_root_step: lastRootStep,
    exception: error?.error ?? null, revert_reason: reason, return_hex: bytesToHex(result.execResult.returnValue),
    modexp_call_count: trace.length, modexp_success_count: trace.filter((x) => x.success).length,
    modexp_failure_count: trace.filter((x) => !x.success).length,
    modexp_execution_gas: trace.reduce((sum, x) => sum + x.execution_gas, 0),
    modexp_trace_sha256: sha256(JSON.stringify(trace)), modexp_failures: trace.filter((x) => !x.success),
  } };
}
const outputPath = path.resolve(root, process.argv[2] ?? 'experiments/g5_c3_raw/miller_sub_full_evm.json');
assert.ok(!fs.existsSync(outputPath), 'full-attempt output already exists; inspect it instead of rerunning');
console.log('G5-C3-B16: deploy the unchanged B15 source; no prefix or negative calls');
const deployment = await transact((await factory.getDeployTransaction()).data);
assert.equal(deployment.result.receipt.status, 1);
const address = deployment.result.createdAddress;
assert.ok(address);
const runtime = await vm.stateManager.getCode(address);
assert.equal(sha256(runtime), baseline.deployed_runtime_sha256);
assert.equal(runtime.length, baseline.runtime_bytecode_bytes);
const fullData = iface.encodeFunctionData('verifyMillerLoopOrRevert',
  [pointInput, fixture.full_miller_fp12_hex]);
assert.equal(sha256(hexToBytes(fullData)), fullBaseline.full_attempt.calldata_sha256);
console.log('G5-C3-B16: ONE full64 positive transaction, fixed 20M cap');
const full = await transact(fullData, address);
const m = full.measurement;
let returned = null;
let decodeError = null;
if (full.result.receipt.status === 1) {
  try { returned = iface.decodeFunctionResult('verifyMillerLoopOrRevert',
    bytesToHex(full.result.execResult.returnValue))[0]; }
  catch (error) { decodeError = error.message; }
}
const fullVerified = full.result.receipt.status === 1 && returned === fixture.full_miller_fp12_hex
  && m.modexp_call_count === 14_316 && m.modexp_failure_count === 0;
const outOfGas = full.result.receipt.status === 0 && m.exception === 'out of gas';
assert.equal(m.gas_refund, 0);
assert.equal(m.total_gas, Math.max(m.intrinsic_gas + m.execution_gas,
  m.minimum_gas_limit_including_calldata_floor));
if (outOfGas) assert.equal(m.total_gas, 20_000_000);
const status = fullVerified ? 'PASS_FULL64_SOFTWARE_MILLER_CONFORMANCE_ONLY'
  : outOfGas ? 'BLOCKED_SOFTWARE_MILLER_SUB_LOWLEVEL_EXCEEDS_20M'
    : 'FAIL_FULL64_SOFTWARE_MILLER_CONFORMANCE';
const report = {
  gate: 'G5-C3-B16', status,
  target_verifier_status: fullVerified ? 'BLOCKED_FINAL_EXPONENTIATION_AND_GT_TARGET_BINDING_MISSING'
    : outOfGas ? 'BLOCKED_SOFTWARE_MILLER_SUB_LOWLEVEL_EXCEEDS_20M'
      : 'BLOCKED_FULL_MILLER_CONFORMANCE_FAILED',
  target_verifier_status_source: path.relative(root, outputPath),
  recorded_at_utc: new Date().toISOString(), git_commit: metadata('git', ['rev-parse', 'HEAD']),
  git_dirty: metadata('git', ['status', '--porcelain']).length > 0,
  cpu: os.cpus()[0].model, os: os.platform(), arch: os.arch(), node_version: process.version,
  compiler: solc.version(), compiler_settings: settings, hardfork: common.hardfork(),
  execution_scope: 'one deployment and ONE full64 positive transaction only; local Prague, fixed20M transaction/30M block limits',
  optimization_implemented: false, contract_source_unchanged_since_b15: true,
  baseline: baselineName, baseline_sha256: sha256(baselineBytes),
  primitive_baseline: primitiveName, primitive_baseline_sha256: sha256(primitiveBytes),
  previous_full_attempt_gate: fullBaseline.gate, previous_full_attempt_result: fullName,
  previous_full_attempt_result_sha256: sha256(fullBytes),
  latest_full_attempt_gate: 'G5-C3-B16', latest_full_attempt_result: path.relative(root, outputPath),
  latest_prefix_gate: baseline.gate, prefix_evidence_source: baselineName,
  fixture: baseline.fixture, fixture_sha256: sha256(fixtureBytes),
  source_fixture: baseline.source_fixture, source_fixture_sha256: sha256(publicBytes),
  endpoint_fixture: baseline.endpoint_fixture, endpoint_fixture_sha256: sha256(endpointBytes),
  aggregate_proof_sha256: sha256(wire), contract_source: sourceName, contract_source_sha256: sha256(contractSource),
  runner_source_sha256: sha256(fs.readFileSync(fileURLToPath(import.meta.url))),
  noble_bls_source_sha256: sha256(read('contracts/node_modules/@noble/curves/abstract/bls.js')),
  noble_tower_source_sha256: sha256(read('contracts/node_modules/@noble/curves/abstract/tower.js')),
  package_lock_sha256: sha256(read('contracts/package-lock.json')), cargo_lock_sha256: sha256(read('Cargo.lock')),
  official_manifest_sha256: sha256(read('configs/protocol_manifest.json')),
  prototype_manifest_sha256: sha256(read('configs/protocol_manifest.JUBJUB-PROTOTYPE.json')),
  runtime_bytecode_bytes: runtime.length, deployed_runtime_sha256: sha256(runtime),
  deployed_contract: address.toString(), transaction_gas_limit: 20_000_000, block_gas_limit: 30_000_000,
  full_steps: 64, expected_full_modexp_calls_from_formula: 14_316,
  deployment: deployment.measurement, full_attempt: m, full_attempted: true,
  miller_run: true, miller_prefix_run: false, full_miller_loop_run: true,
  full_loop_attempts: 1, full_miller_loop_completed: fullVerified,
  full_miller_loop_verified_in_evm: fullVerified,
  full_return_matches_reference: returned === fixture.full_miller_fp12_hex,
  full_return_decode_error: decodeError,
  full_miller_verification_gas: fullVerified ? m.total_gas : null,
  failed_attempt_gas_is_capped_spend_not_completed_loop_cost: !fullVerified,
  prefix_correctness_verified_in_evm: baseline.prefix_correctness_verified_in_evm,
  wrong_prefix_target_rejected_in_evm: baseline.wrong_prefix_target_rejected_in_evm,
  prefix_checks_executed_this_run: false,
  host_precomputed_lines_or_state_used_as_contract_computation: false,
  evm_final_exponentiation_implemented: false, pairing_to_gt_evm_verified: false,
  exact_snarkpack_evm_verified: false, complete_g5c3: false, snarkpack_verification_gas: null,
  setup_run: false, prove_run: false, private_witness_emitted: false,
  next_gate: fullVerified
    ? 'Plan one final-exponentiation component on the saved full Miller output; full pairing and SnarkPack binding remain separate'
    : 'Read-only attribution of this saved full-attempt failure and a software-route/budget decision; no automatic full-loop retry or new primitive profiling',
  limitations: [
    'One fixed public full-loop attempt, not an end-to-end/statistical voting benchmark',
    'Success means only Miller-loop conformance; no EVM final exponentiation, GT target binding or complete SnarkPack verifier',
    'A failed capped spend is not a completed-loop gas measurement; MODEXP counts are not a completion percentage',
    'Source/runtime are identical to B15; no arithmetic, compiler, curve, codec, Manifest or budget change',
    'No prefix or boundary rerun, negative matrix, setup/prove, scale experiment or RPC transaction',
  ],
};
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ status, full_verified: fullVerified, capped_or_actual_total_gas: m.total_gas,
  full_miller_verification_gas: report.full_miller_verification_gas, modexp_successes: m.modexp_success_count,
  memory_words: m.evm_active_memory_words_at_exit, exception: m.exception }));
console.log('wrote ' + path.relative(root, outputPath));
if (!fullVerified && !outOfGas) process.exitCode = 1;
