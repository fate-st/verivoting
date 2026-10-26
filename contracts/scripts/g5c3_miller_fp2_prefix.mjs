// Integrate only the audited B8 _mul2 into B7; prefix positive/wrong-target pair ONLY, no full-loop retry.
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
const baselineName = 'experiments/g5_c3_raw/miller_lowlevel_evm.json';
const baselineBytes = read(baselineName);
const baseline = JSON.parse(baselineBytes);
const b2Bytes = read(baseline.unoptimized_baseline);
const primitiveName = 'experiments/g5_c3_raw/fp2_lowlevel_profile.json';
const primitiveBytes = read(primitiveName);
const primitive = JSON.parse(primitiveBytes);
const baselineSource = read(baseline.contract_source).toString('utf8');
const primitiveSource = read(primitive.contract_source).toString('utf8');
const b2 = JSON.parse(b2Bytes);
const fixtureBytes = read(baseline.fixture);
const fixture = JSON.parse(fixtureBytes);
const publicBytes = read(baseline.source_fixture);
const source = JSON.parse(publicBytes);
const endpointBytes = read(baseline.endpoint_fixture);
const endpoint = JSON.parse(endpointBytes);
const sourceName = 'contracts/src/Bls12381MillerLoopFp2LowLevel.sol';
const { Fp, Fp12 } = bls12_381.fields;
const packages = JSON.parse(read('contracts/package-lock.json'));
assert.equal(baseline.gate, 'G5-C3-B7');
assert.equal(primitive.gate, 'G5-C3-B8');
assert.equal(primitive.all_boundaries_verified, true);
assert.equal(primitive.measured_fp2_window_improvement, true);
assert.equal(sha256(primitiveSource), primitive.contract_source_sha256);
assert.equal(sha256(read('contracts/scripts/g5c3_fp2_lowlevel.mjs')), primitive.runner_source_sha256);
assert.equal(sha256(read(primitive.baseline)), primitive.baseline_sha256);
assert.equal(b2.gate, 'G5-C3-B2');
assert.equal(sha256(b2Bytes), baseline.unoptimized_baseline_sha256);
assert.equal(sha256(read(b2.contract_source)), b2.contract_source_sha256);
assert.equal(sha256(read('contracts/scripts/g5c3_miller_loop.mjs')), b2.runner_source_sha256);
assert.equal(baseline.prefix_correctness_verified_in_evm, true);
assert.equal(baseline.prefix_valid.modexp_call_count, 516);
assert.equal(baseline.fixture_sha256, sha256(fixtureBytes));
assert.equal(baseline.source_fixture_sha256, sha256(publicBytes));
assert.equal(baseline.endpoint_fixture_sha256, sha256(endpointBytes));
assert.equal(baseline.contract_source_sha256, sha256(read(baseline.contract_source)));
assert.equal(baseline.runner_source_sha256, sha256(read('contracts/scripts/g5c3_miller_lowlevel.mjs')));
for (const [name, key] of [
  ['Cargo.lock', 'cargo_lock_sha256'], ['contracts/package-lock.json', 'package_lock_sha256'],
  ['configs/protocol_manifest.json', 'official_manifest_sha256'],
  ['configs/protocol_manifest.JUBJUB-PROTOTYPE.json', 'prototype_manifest_sha256'],
]) {
  assert.equal(sha256(read(name)), baseline[key], `${name} changed since baseline`);
  assert.equal(sha256(read(name)), primitive[key], `${name} changed since B8`);
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
for (const record of [baseline, primitive, fixture, source, endpoint]) {
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
// Whole-source equality pins all other functions, structs, constants and comments.
const expectedSource = baselineSource.replace('contract Bls12381MillerLoopLowLevel {',
  'contract Bls12381MillerLoopFp2LowLevel {').replace(functionBody(baselineSource, '_mul2'),
  functionBody(primitiveSource, '_mul2'));
assert.equal(contractSource, expectedSource, 'only contract name and the exact B8 _mul2 may change');
assert.equal(functionBody(contractSource, '_mul2'), functionBody(primitiveSource, '_mul2'));
assert.equal(functionBody(contractSource, '_mul'), functionBody(baselineSource, '_mul'));
const settings = { optimizer: { enabled: true, runs: 200 }, evmVersion: 'prague',
  outputSelection: { '*': { '*': ['abi', 'evm.bytecode.object', 'evm.deployedBytecode.object'] } } };
assert.equal(solc.version(), baseline.compiler);
assert.deepEqual(settings, baseline.compiler_settings);
const compilation = JSON.parse(solc.compile(JSON.stringify({ language: 'Solidity',
  sources: { 'Bls12381MillerLoopFp2LowLevel.sol': { content: contractSource } }, settings })));
for (const error of compilation.errors ?? []) {
  if (error.severity === 'error') throw Error(error.formattedMessage);
  console.warn(error.formattedMessage);
}
const compiled = compilation.contracts['Bls12381MillerLoopFp2LowLevel.sol'].Bls12381MillerLoopFp2LowLevel;
const iface = new Interface(compiled.abi);
const factory = new ContractFactory(compiled.abi, `0x${compiled.evm.bytecode.object}`);
const common = new Common({ chain: Mainnet, hardfork: Hardfork.Prague });
const vm = await createVM({ common });
assert.equal(common.hardfork(), baseline.hardfork);
// Public deterministic local-only key. No RPC, real chain or user account.
const senderKey = hexToBytes(`0x${'28'.repeat(32)}`);
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
console.log('G5-C3-B9: deploy exact B8 _mul2 integration; B7/B8 baseline and fixtures unchanged');
const deployment = await transact((await factory.getDeployTransaction()).data);
assert.equal(deployment.result.receipt.status, 1);
const address = deployment.result.createdAddress;
assert.ok(address);
const runtime = await vm.stateManager.getCode(address);
assert.ok(runtime.length > 0 && runtime.length <= 24576);
const validData = iface.encodeFunctionData('verifyMillerPrefixOrRevert',
  [pointInput, 2, fixture.prefix_fp12_hex]);
const wrongData = iface.encodeFunctionData('verifyMillerPrefixOrRevert', [pointInput, 2, wrongExpected]);
const vd = iface.decodeFunctionData('verifyMillerPrefixOrRevert', validData);
const wd = iface.decodeFunctionData('verifyMillerPrefixOrRevert', wrongData);
assert.equal(vd[0], wd[0]); assert.equal(vd[1], wd[1]); assert.notEqual(vd[2], wd[2]);
assert.equal(sha256(hexToBytes(validData)), baseline.prefix_valid.calldata_sha256);
assert.equal(sha256(hexToBytes(wrongData)), baseline.prefix_wrong_target.calldata_sha256);
console.log('G5-C3-B9: same two-step positive/negative, no new test matrix');
const valid = await transact(validData, address);
assert.equal(valid.result.receipt.status, 1, 'correct prefix failed');
assert.equal(iface.decodeFunctionResult('verifyMillerPrefixOrRevert',
  bytesToHex(valid.result.execResult.returnValue))[0], fixture.prefix_fp12_hex);
const wrong = await transact(wrongData, address);
assert.equal(wrong.result.receipt.status, 0);
assert.equal(wrong.measurement.exception, 'revert');
assert.equal(wrong.measurement.revert_reason, 'INVALID_MILLER_RESULT');
for (const item of [valid, wrong]) {
  assert.equal(item.measurement.modexp_call_count, 516,
    'dedicated square/014 operation count differs from the reviewed formula');
  assert.equal(item.measurement.modexp_failure_count, 0);
  assert.equal(item.measurement.gas_refund, 0);
  assert.equal(item.measurement.total_gas, Math.max(item.measurement.intrinsic_gas
    + item.measurement.execution_gas, item.measurement.minimum_gas_limit_including_calldata_floor));
}
assert.equal(valid.measurement.calldata_bytes, baseline.prefix_valid.calldata_bytes);
const memoryReduced = valid.measurement.evm_active_memory_words_at_exit < baseline.prefix_valid.evm_active_memory_words_at_exit;
const gasReduced = valid.measurement.total_gas < baseline.prefix_valid.total_gas;
const report = {
  gate: 'G5-C3-B9', status: gasReduced ? 'PASS_FP2_MILLER_PREFIX_INTEGRATION_ONLY'
    : 'PASS_FP2_MILLER_PREFIX_NO_GAS_REDUCTION',
  target_verifier_status: baseline.target_verifier_status,
  target_verifier_status_source: baselineName,
  recorded_at_utc: new Date().toISOString(), git_commit: metadata('git', ['rev-parse', 'HEAD']),
  git_dirty: metadata('git', ['status', '--porcelain']).length > 0,
  cpu: os.cpus()[0].model, os: os.platform(), arch: os.arch(), node_version: process.version,
  compiler: solc.version(), compiler_settings: settings, hardfork: common.hardfork(),
  execution_scope: 'one deployment and the same two prefix transactions ONLY; no full-loop retry, RPC or proving',
  optimization_scope: 'only exact B8 _mul2 integrated into B7; all other source unchanged including _mul, point/NAF, sparse algebra and arena',
  baseline: baselineName, baseline_sha256: sha256(baselineBytes),
  baseline_contract_source_sha256: baseline.contract_source_sha256,
  primitive_baseline: primitiveName, primitive_baseline_sha256: sha256(primitiveBytes),
  primitive_contract_source: primitive.contract_source, primitive_contract_source_sha256: sha256(primitiveSource),
  exact_b8_mul2_integrated: true, all_other_contract_source_unchanged: true,
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
  prefix_steps: 2, prefix_correctness_verified_in_evm: true,
  wrong_prefix_target_rejected_in_evm: true, expected_prefix_modexp_calls: 516,
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
  },
  deployment: deployment.measurement, prefix_valid: valid.measurement, prefix_wrong_target: wrong.measurement,
  miller_run: true, miller_prefix_run: true, full_miller_loop_run: false, full_loop_attempts: 0, full_attempt: null,
  latest_full_attempt_gate: baseline.gate, latest_full_attempt_result: baselineName,
  latest_full_attempt_result_sha256: sha256(baselineBytes),
  full_miller_loop_verified_in_evm: false, full_miller_verification_gas: null,
  host_precomputed_lines_or_state_used_as_contract_computation: false,
  evm_final_exponentiation_implemented: false, pairing_to_gt_evm_verified: false,
  exact_snarkpack_evm_verified: false, complete_g5c3: false, snarkpack_verification_gas: null,
  setup_run: false, prove_run: false, private_witness_emitted: false,
  next_gate: gasReduced
    ? 'Measure one isolated Fp2 addition allocation path on the same public operands before further optimization or full-loop retry; keep20M cap, no final exponentiation or scale runs'
    : 'Retain B7/B8, inspect the isolated-to-integrated cost difference before any new implementation or full-loop retry',
  limitations: [
    'Prefix-only integration on one fixed public input; not full-loop correctness or a statistical E2E performance result',
    'This new variant has no full64 EVM attempt; B7 is a historical failure for the previous variant at20M',
    'The measured prefix saving does not establish a completed-loop gas cost or justify an automatic full-loop retry',
    'Memory reuse is pinned to Solidity0.8.30 legacy pipeline/optimizer200; not a portable arbitrary-compiler library',
    'No curve/subgroup/identity policy or full SnarkPack transcript/target/KZG binding in the contract',
    'No protocol/curve/Manifest capability promotion; no setup/prove or scale experiments',
  ],
};
const outputPath = path.resolve(root, process.argv[2] ?? 'experiments/g5_c3_raw/miller_fp2_prefix_evm.json');
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(`Prefix PASS: gas ${baseline.prefix_valid.total_gas} -> ${valid.measurement.total_gas}; memory words ${baseline.prefix_valid.evm_active_memory_words_at_exit} -> ${valid.measurement.evm_active_memory_words_at_exit}`);
console.log('No full-loop transaction was built or executed; latest full attempt remains B7 at20M OOG.');
console.log('Final exponentiation / exact GT target / complete SnarkPack EVM remain BLOCKED.');
console.log(`wrote ${path.relative(root, outputPath)}`);



