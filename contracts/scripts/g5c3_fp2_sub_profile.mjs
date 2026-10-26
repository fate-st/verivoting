// Cost localization ONLY: one unchanged Fp2 subtraction, using the latest isolated arithmetic baseline.
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
const baselineName = 'experiments/g5_c3_raw/fp2_add_lowlevel_profile.json';
const baselineBytes = read(baselineName);
const baseline = JSON.parse(baselineBytes);
const millerName = 'experiments/g5_c3_raw/miller_lowlevel_evm.json';
const millerBytes = read(millerName);
const miller = JSON.parse(millerBytes);
const prefixName = 'experiments/g5_c3_raw/miller_add_prefix_evm.json';
const prefixBytes = read(prefixName);
const prefix = JSON.parse(prefixBytes);
const sourceBytes = read(baseline.source_fixture);
const source = JSON.parse(sourceBytes);
const fixtureBytes = read(baseline.fixture);
const fixture = JSON.parse(fixtureBytes);
const baselineSource = read(baseline.contract_source).toString('utf8');
const contractName = 'contracts/src/Bls12381Fp2SubProfile.sol';
const contractSource = read(contractName).toString('utf8');
const { Fp, Fp2 } = bls12_381.fields;
assert.equal(baseline.gate, 'G5-C3-B11');
assert.equal(prefix.gate, 'G5-C3-B12');
assert.equal(sha256(baselineBytes), prefix.primitive_baseline_sha256);
assert.equal(sha256(read(prefix.contract_source)), prefix.contract_source_sha256);
assert.equal(sha256(read('contracts/scripts/g5c3_miller_add_prefix.mjs')), prefix.runner_source_sha256);
assert.equal(sha256(millerBytes), prefix.latest_full_attempt_result_sha256);
assert.equal(miller.gate, 'G5-C3-B7');
assert.equal(sha256(read(miller.contract_source)), miller.contract_source_sha256);
assert.equal(sha256(read('contracts/scripts/g5c3_miller_lowlevel.mjs')), miller.runner_source_sha256);
assert.equal(sha256(sourceBytes), baseline.source_fixture_sha256);
assert.equal(sha256(fixtureBytes), baseline.fixture_sha256);
assert.equal(sha256(baselineSource), baseline.contract_source_sha256);
assert.equal(sha256(read('contracts/scripts/g5c3_fp2_add_lowlevel.mjs')), baseline.runner_source_sha256);
assert.equal(source.schema, 'VV/G5-C1/SNARKPACK-PUBLIC-FIXTURE/v1');
assert.equal(fixture.schema, 'VV/G5-C3/MILLER-LOOP/v1');
for (const [name, key] of [
  ['Cargo.lock', 'cargo_lock_sha256'], ['contracts/package-lock.json', 'package_lock_sha256'],
  ['configs/protocol_manifest.json', 'official_manifest_sha256'],
  ['configs/protocol_manifest.JUBJUB-PROTOTYPE.json', 'prototype_manifest_sha256'],
]) {
  assert.equal(sha256(read(name)), baseline[key]);
  assert.equal(sha256(read(name)), miller[key]);
  assert.equal(sha256(read(name)), prefix[key]);
}
function body(text, name) {
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
const unchangedFunctions = ['_ge', '_rawAdd', '_rawSub', '_add', '_sub', '_half',
  '_square', '_add2', '_sub2', '_mul', '_mul2', '_readFp', '_read2', '_writeFp', '_write2', 'profileFpMul', 'profileFp2Mul', 'profileFp2Add'];
for (const name of unchangedFunctions) assert.equal(body(contractSource, name), body(baselineSource, name));
const addedEntry = body(contractSource, 'profileFp2Sub');
const expectedSource = baselineSource.replace('contract Bls12381Fp2AddLowLevelProfile {',
  'contract Bls12381Fp2SubProfile {').replace('    function _ge(', addedEntry + '\n\n    function _ge(');
assert.equal(contractSource, expectedSource, 'only contract name and one new profile entry may change');
assert.equal(body(contractSource, '_sub2'), body(read(prefix.contract_source).toString('utf8'), '_sub2'));
assert.equal(addedEntry, body(baselineSource, 'profileFp2Add').replace('profileFp2Add(', 'profileFp2Sub(').replace('_add2(a, b)', '_sub2(a, b)'));
const wire = Buffer.from(source.aggregate_proof_hex, 'hex');
assert.equal(wire.length, 5236);
assert.equal(sha256(wire), baseline.aggregate_proof_sha256);
assert.equal(sha256(wire), source.aggregate_proof_sha256);
assert.equal(sha256(wire), miller.aggregate_proof_sha256);
assert.equal(sha256(wire), prefix.aggregate_proof_sha256);
const q = bls12_381.G2.Point.fromBytes(wire.subarray(4516, 4612)).toAffine();
const fpBE = (value) => {
  assert.ok(value >= 0n && value < Fp.ORDER);
  return Buffer.from(value.toString(16).padStart(128, '0'), 'hex');
};
const fp2BE = (value) => bytesToHex(Buffer.concat([fpBE(value.c0), fpBE(value.c1)]));
const mod = value => ((value % Fp.ORDER) + Fp.ORDER) % Fp.ORDER;
const expected = fp2BE({ c0: mod(q.x.c0 - q.y.c0), c1: mod(q.x.c1 - q.y.c1) });
assert.equal(expected, fp2BE(Fp2.sub(q.x, q.y)));
const limbBase = 1n << 256n;
const hostOperandPaths = Object.fromEntries(['c0', 'c1'].map(key => {
  const a = q.x[key], b = q.y[key];
  const wraps = a < b;
  const magnitude = wraps ? b - a : a - b;
  return [key, { modular_wrap: wraps,
    direct_subtraction_low_limb_borrow: wraps ? null : a % limbBase < b % limbBase,
    wrap_magnitude_low_limb_borrow: wraps ? b % limbBase < a % limbBase : null,
    wrap_complement_low_limb_borrow: wraps ? Fp.ORDER % limbBase < magnitude % limbBase : null }];
}));
const fixedPoints = Buffer.from(fixture.points_eip2537_hex.slice(2), 'hex');
assert.equal(bytesToHex(fixedPoints.subarray(128, 256)), fp2BE(q.x));
assert.equal(bytesToHex(fixedPoints.subarray(256, 384)), fp2BE(q.y));

const settings = { optimizer: { enabled: true, runs: 200 }, evmVersion: 'prague',
  outputSelection: { '*': { '*': ['abi', 'evm.bytecode.object', 'evm.deployedBytecode.object'] } } };
assert.equal(solc.version(), baseline.compiler);
assert.deepEqual(settings, baseline.compiler_settings);
const compilation = JSON.parse(solc.compile(JSON.stringify({ language: 'Solidity',
  sources: { 'Bls12381Fp2SubProfile.sol': { content: contractSource } }, settings })));
for (const error of compilation.errors ?? []) {
  if (error.severity === 'error') throw Error(error.formattedMessage);
  console.warn(error.formattedMessage);
}
const compiled = compilation.contracts['Bls12381Fp2SubProfile.sol'].Bls12381Fp2SubProfile;
const iface = new Interface(compiled.abi);
const factory = new ContractFactory(compiled.abi, `0x${compiled.evm.bytecode.object}`);
const common = new Common({ chain: Mainnet, hardfork: Hardfork.Prague });
assert.equal(common.hardfork(), baseline.hardfork);
const vm = await createVM({ common });
// Public deterministic local-only key; never touches RPC/user funds.
const senderKey = hexToBytes(`0x${'2c'.repeat(32)}`);
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
console.log('G5-C3-B13: deploy isolated cost instrumentation; no Miller/proving runs');
const deployment = await transact((await factory.getDeployTransaction()).data);
assert.equal(deployment.result.receipt.status, 1);
const address = deployment.result.createdAddress;
assert.ok(address);
const runtime = await vm.stateManager.getCode(address);
assert.ok(runtime.length > 0 && runtime.length <= 24576);
async function profile(label, method, left, right, expected, calls) {
  const transaction = await transact(iface.encodeFunctionData(method, [left, right]), address);
  assert.equal(transaction.result.receipt.status, 1);
  const [computed, windowGas, freeBefore, freeAfter] = iface.decodeFunctionResult(method,
    bytesToHex(transaction.result.execResult.returnValue));
  assert.equal(computed, expected, `${label} result mismatch`);
  const m = transaction.measurement;
  assert.equal(m.modexp_call_count, calls);
  assert.ok(m.modexp_calls.every((x) => x.success && x.return_bytes === 64));
  assert.equal(m.gas_refund, 0);
  assert.equal(m.total_gas, Math.max(m.intrinsic_gas + m.execution_gas,
    m.minimum_gas_limit_including_calldata_floor));
  assert.equal(m.top_level_opcode_gas + m.modexp_execution_gas, m.execution_gas,
    'top-level opcode plus child precompile accounting must reconcile');
  assert.equal(m.gas_markers.length, calls + 2, 'two window GAS markers plus MODEXP gas() calls');
  const begin = m.gas_markers[0];
  const end = m.gas_markers.at(-1);
  assert.equal(begin.gas_left_before - end.gas_left_before, Number(windowGas));
  const memoryCost = (words) => 3 * words + Math.floor(words * words / 512);
  assert.ok(Number(windowGas) > m.modexp_execution_gas && Number(windowGas) <= m.execution_gas);
  assert.ok(freeAfter >= freeBefore);
  return { operation: label, left_hex: left, right_hex: right, expected_hex: expected, computed_hex: computed,
    verified: true, ...m, instrumented_arithmetic_window_gas: Number(windowGas),
    precompile_share_of_window_percent: 100 * m.modexp_execution_gas / Number(windowGas),
    window_minus_precompile_execution_gas: Number(windowGas) - m.modexp_execution_gas,
    window_evm_memory_expansion_gas: memoryCost(end.memory_words_before) - memoryCost(begin.memory_words_before),
    window_memory_words_before: begin.memory_words_before, window_memory_words_after: end.memory_words_before,
    window_staticcall_opcode_gas: end.staticcall_gas_before - begin.staticcall_gas_before,
    execution_gas_outside_window: m.execution_gas - Number(windowGas),
    free_memory_before_bytes: Number(freeBefore), free_memory_after_bytes: Number(freeAfter),
    allocator_growth_in_window_bytes: Number(freeAfter - freeBefore),
    calldata_floor_extra_gas: m.total_gas - m.intrinsic_gas - m.execution_gas };
}
console.log('G5-C3-B13: ONE unchanged Fp2 subtraction on saved public endpoint coordinates');
const fp2Sub = await profile('Fp2 subtraction', 'profileFp2Sub', fp2BE(q.x), fp2BE(q.y), expected, 0);
assert.equal(fp2Sub.left_hex, baseline.fp2_add.left_hex);
assert.equal(fp2Sub.right_hex, baseline.fp2_add.right_hex);
assert.equal(fp2Sub.staticcall_opcode_gas, 0);
const report = {
  gate: 'G5-C3-B13', status: 'PASS_ISOLATED_FP2_SUB_COST_PROFILE_ONLY',
  target_verifier_status: prefix.target_verifier_status,
  target_verifier_status_source: prefix.target_verifier_status_source,
  recorded_at_utc: new Date().toISOString(), git_commit: metadata('git', ['rev-parse', 'HEAD']),
  git_dirty: metadata('git', ['status', '--porcelain']).length > 0,
  cpu: os.cpus()[0].model, os: os.platform(), arch: os.arch(), node_version: process.version,
  compiler: solc.version(), compiler_settings: settings, hardfork: common.hardfork(),
  execution_scope: 'one deployment and ONE original Fp2-subtraction cost transaction ONLY; no other tests/Miller/RPC/proving',
  arithmetic_changed: false, optimization_implemented: false,
  transaction_gas_limit: 20_000_000, block_gas_limit: 30_000_000,
  arithmetic_baseline: baselineName, arithmetic_baseline_sha256: sha256(baselineBytes),
  arithmetic_baseline_source_sha256: sha256(baselineSource),
  prefix_baseline: prefixName, prefix_baseline_sha256: sha256(prefixBytes),
  latest_full_attempt_gate: miller.gate, latest_full_attempt_result: millerName,
  latest_full_attempt_result_sha256: sha256(millerBytes),
  source_fixture: baseline.source_fixture, source_fixture_sha256: sha256(sourceBytes),
  fixture: baseline.fixture, fixture_sha256: sha256(fixtureBytes), aggregate_proof_sha256: sha256(wire),
  operand_source: 'proof.tmipp.gipa.final_b affine x minus y; same full Fp2 operands as B11 addition',
  host_operand_paths: hostOperandPaths,
  contract_source: contractName, contract_source_sha256: sha256(contractSource),
  runner_source_sha256: sha256(fs.readFileSync(fileURLToPath(import.meta.url))),
  arithmetic_source_bodies_unchanged: unchangedFunctions,
  sub2_source_sha256: sha256(body(contractSource, '_sub2')),
  package_lock_sha256: sha256(read('contracts/package-lock.json')), cargo_lock_sha256: sha256(read('Cargo.lock')),
  official_manifest_sha256: sha256(read('configs/protocol_manifest.json')),
  prototype_manifest_sha256: sha256(read('configs/protocol_manifest.JUBJUB-PROTOTYPE.json')),
  runtime_bytecode_bytes: runtime.length, deployed_runtime_sha256: sha256(runtime), deployed_contract: address.toString(),
  deployment: deployment.measurement, fp2_sub: fp2Sub,
  window_definition: 'gasleft difference around unchanged _sub2 after canonical decode and before encoding; includes compiler/call and instrumentation overhead',
  gas_accounting: 'total=max(intrinsic+execution,Prague calldata floor) with zero refund; floor extra is NOT arithmetic-window cost',
  primitive_profile_only: true, arithmetic_transactions: 1,
  miller_run: false, full_miller_loop_run: false, full_loop_attempts: 0, full_attempt: null,
  full_miller_loop_verified_in_evm: false, full_miller_verification_gas: null,
  evm_final_exponentiation_implemented: false, pairing_to_gt_evm_verified: false,
  exact_snarkpack_evm_verified: false, complete_g5c3: false, snarkpack_verification_gas: null,
  setup_run: false, prove_run: false, private_witness_emitted: false,
  next_gate: 'Evaluate ONE isolated low-allocation _sub2 candidate against this saved baseline; compare full canonical output and window/execution/allocation, add only the necessary branch boundary; no full-loop retry or final exponentiation',
  limitations: [
    'One fixed public subtraction cost sample; both components use direct subtraction, with/without low-limb borrow; modular wrap is not covered',
    'Arithmetic is unchanged; this is cost localization, not a new optimization result',
    'Instrumented window cost is not an inlined Miller constant or proof of the unique bottleneck',
    'Prague calldata-floor extra is transaction pricing, not arithmetic execution or memory cost',
    'Allocator-pointer growth is not EVM peak memory or host RSS',
    'No extra arithmetic tests or multiplication/prefix/full-loop/setup/prove/scale rerun; no protocol capability promotion',
  ],
};
const outputPath = path.resolve(root, process.argv[2] ?? 'experiments/g5_c3_raw/fp2_sub_cost_profile.json');
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(`Fp2 subtraction: total=${fp2Sub.total_gas}, execution=${fp2Sub.execution_gas}, window=${fp2Sub.instrumented_arithmetic_window_gas}, allocator=${fp2Sub.allocator_growth_in_window_bytes}B, MODEXP=${fp2Sub.modexp_call_count}, floor_extra=${fp2Sub.calldata_floor_extra_gas}`);
console.log('Unchanged arithmetic cost profile PASS; full Miller/GT/SnarkPack remain unverified.');
console.log(`wrote ${path.relative(root, outputPath)}`);

