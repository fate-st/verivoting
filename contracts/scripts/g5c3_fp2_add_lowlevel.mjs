// One allocation-only Fp2 addition candidate: original cost input plus ONE equality/zero boundary; no Miller/proving.
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
const baselineName = 'experiments/g5_c3_raw/fp2_add_cost_profile.json';
const baselineBytes = read(baselineName);
const baseline = JSON.parse(baselineBytes);
const millerName = 'experiments/g5_c3_raw/miller_lowlevel_evm.json';
const millerBytes = read(millerName);
const miller = JSON.parse(millerBytes);
const prefixName = 'experiments/g5_c3_raw/miller_fp2_prefix_evm.json';
const prefixBytes = read(prefixName);
const prefix = JSON.parse(prefixBytes);
const sourceBytes = read(baseline.source_fixture);
const source = JSON.parse(sourceBytes);
const fixtureBytes = read(baseline.fixture);
const fixture = JSON.parse(fixtureBytes);
const baselineSource = read(baseline.contract_source).toString('utf8');
const contractName = 'contracts/src/Bls12381Fp2AddLowLevelProfile.sol';
const contractSource = read(contractName).toString('utf8');
const { Fp, Fp2 } = bls12_381.fields;
assert.equal(baseline.gate, 'G5-C3-B10');
assert.equal(prefix.gate, 'G5-C3-B9');
assert.equal(sha256(read(prefix.contract_source)), prefix.contract_source_sha256);
assert.equal(sha256(read('contracts/scripts/g5c3_miller_fp2_prefix.mjs')), prefix.runner_source_sha256);
assert.equal(sha256(millerBytes), prefix.latest_full_attempt_result_sha256);
assert.equal(miller.gate, 'G5-C3-B7');
assert.equal(sha256(read(miller.contract_source)), miller.contract_source_sha256);
assert.equal(sha256(read('contracts/scripts/g5c3_miller_lowlevel.mjs')), miller.runner_source_sha256);
assert.equal(sha256(sourceBytes), baseline.source_fixture_sha256);
assert.equal(sha256(fixtureBytes), baseline.fixture_sha256);
assert.equal(sha256(baselineSource), baseline.contract_source_sha256);
assert.equal(sha256(read('contracts/scripts/g5c3_fp2_add_profile.mjs')), baseline.runner_source_sha256);
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
  '_square', '_sub2', '_mul', '_mul2', '_readFp', '_read2', '_writeFp', '_write2', 'profileFpMul', 'profileFp2Mul', 'profileFp2Add'];
for (const name of unchangedFunctions) assert.equal(body(contractSource, name), body(baselineSource, name));
const expectedSource = baselineSource.replace('contract Bls12381Fp2AddProfile {',
  'contract Bls12381Fp2AddLowLevelProfile {').replace(body(baselineSource, '_add2'), body(contractSource, '_add2'));
assert.equal(contractSource, expectedSource, 'only contract name and _add2 may change');
assert.equal(body(baselineSource, '_add2'), body(read(prefix.contract_source).toString('utf8'), '_add2'));
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
const expected = fp2BE({ c0: mod(q.x.c0 + q.y.c0), c1: mod(q.x.c1 + q.y.c1) });
assert.equal(expected, fp2BE(Fp2.add(q.x, q.y)));
const limbBase = 1n << 256n;
const hostOperandPaths = Object.fromEntries(['c0', 'c1'].map(key => {
  const sum = q.x[key] + q.y[key];
  return [key, { sum_requires_reduction: sum >= Fp.ORDER,
    low_limb_sum_carry: q.x[key] % limbBase + q.y[key] % limbBase >= limbBase,
    reduction_low_borrow: sum >= Fp.ORDER && sum % limbBase < Fp.ORDER % limbBase }];
}));
const fixedPoints = Buffer.from(fixture.points_eip2537_hex.slice(2), 'hex');
assert.equal(bytesToHex(fixedPoints.subarray(128, 256)), fp2BE(q.x));
assert.equal(bytesToHex(fixedPoints.subarray(256, 384)), fp2BE(q.y));

const settings = { optimizer: { enabled: true, runs: 200 }, evmVersion: 'prague',
  outputSelection: { '*': { '*': ['abi', 'evm.bytecode.object', 'evm.deployedBytecode.object'] } } };
assert.equal(solc.version(), baseline.compiler);
assert.deepEqual(settings, baseline.compiler_settings);
const compilation = JSON.parse(solc.compile(JSON.stringify({ language: 'Solidity',
  sources: { 'Bls12381Fp2AddLowLevelProfile.sol': { content: contractSource } }, settings })));
for (const error of compilation.errors ?? []) {
  if (error.severity === 'error') throw Error(error.formattedMessage);
  console.warn(error.formattedMessage);
}
const compiled = compilation.contracts['Bls12381Fp2AddLowLevelProfile.sol'].Bls12381Fp2AddLowLevelProfile;
const iface = new Interface(compiled.abi);
const factory = new ContractFactory(compiled.abi, `0x${compiled.evm.bytecode.object}`);
const common = new Common({ chain: Mainnet, hardfork: Hardfork.Prague });
assert.equal(common.hardfork(), baseline.hardfork);
const vm = await createVM({ common });
// Public deterministic local-only key; never touches RPC/user funds.
const senderKey = hexToBytes(`0x${'2a'.repeat(32)}`);
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
console.log('G5-C3-B11: deploy isolated cost instrumentation; no Miller/proving runs');
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
console.log('G5-C3-B11: original public cost input, then ONE necessary sum-equals-p/zero boundary');
const fp2Add = await profile('Fp2 addition', 'profileFp2Add', fp2BE(q.x), fp2BE(q.y), expected, 0);
assert.equal(fp2Add.left_hex, baseline.fp2_add.left_hex);
assert.equal(fp2Add.right_hex, baseline.fp2_add.right_hex);
assert.equal(fp2Add.staticcall_opcode_gas, 0);
assert.equal(fp2Add.expected_hex, baseline.fp2_add.expected_hex);
assert.equal(fp2Add.calldata_sha256, baseline.fp2_add.calldata_sha256);
const boundaryA = { c0: Fp.ORDER - 1n, c1: 0n };
const boundaryB = { c0: 1n, c1: 0n };
const boundaryExpected = fp2BE({ c0: 0n, c1: 0n });
assert.equal(boundaryExpected, fp2BE(Fp2.add(boundaryA, boundaryB)));
assert.equal((boundaryA.c0 + boundaryB.c0) % Fp.ORDER, 0n);
const boundary = { id: 'sum_equals_p_and_zero', comparison_performance_sample: false,
  host_operand_paths: { c0: 'sum exactly p, canonical result zero', c1: 'zero plus zero' },
  ...await profile('sum_equals_p_and_zero', 'profileFp2Add', fp2BE(boundaryA), fp2BE(boundaryB), boundaryExpected, 0) };
assert.equal(boundary.staticcall_opcode_gas, 0);
const previous = baseline.fp2_add;
const comparison = {
  baseline_window_gas: previous.instrumented_arithmetic_window_gas,
  optimized_window_gas: fp2Add.instrumented_arithmetic_window_gas,
  window_gas_reduction_percent: 100 * (1 - fp2Add.instrumented_arithmetic_window_gas / previous.instrumented_arithmetic_window_gas),
  baseline_execution_gas: previous.execution_gas, optimized_execution_gas: fp2Add.execution_gas,
  baseline_total_gas: previous.total_gas, optimized_total_gas: fp2Add.total_gas,
  baseline_allocator_growth_bytes: previous.allocator_growth_in_window_bytes,
  optimized_allocator_growth_bytes: fp2Add.allocator_growth_in_window_bytes,
  allocator_growth_reduction_percent: 100 * (1 - fp2Add.allocator_growth_in_window_bytes / previous.allocator_growth_in_window_bytes),
  baseline_window_memory_gas: previous.window_evm_memory_expansion_gas,
  optimized_window_memory_gas: fp2Add.window_evm_memory_expansion_gas,
  baseline_floor_extra_gas: previous.calldata_floor_extra_gas,
  optimized_floor_extra_gas: fp2Add.calldata_floor_extra_gas,
};
const measuredImprovement = comparison.optimized_window_gas < comparison.baseline_window_gas
  && comparison.optimized_execution_gas < comparison.baseline_execution_gas
  && comparison.optimized_allocator_growth_bytes < comparison.baseline_allocator_growth_bytes;
const report = {
  gate: 'G5-C3-B11', status: measuredImprovement ? 'PASS_LOW_ALLOCATION_FP2_ADD_FIXED_CASES_ONLY' : 'PASS_FP2_ADD_CORRECTNESS_COST_TARGET_NOT_MET',
  target_verifier_status: prefix.target_verifier_status,
  target_verifier_status_source: prefix.target_verifier_status_source,
  recorded_at_utc: new Date().toISOString(), git_commit: metadata('git', ['rev-parse', 'HEAD']),
  git_dirty: metadata('git', ['status', '--porcelain']).length > 0,
  cpu: os.cpus()[0].model, os: os.platform(), arch: os.arch(), node_version: process.version,
  compiler: solc.version(), compiler_settings: settings, hardfork: common.hardfork(),
  execution_scope: 'one deployment, one original public Fp2-addition cost case and ONE equality/zero boundary; no Miller/RPC/proving',
  arithmetic_changed: true, optimization_implemented: true, mathematical_formula_changed: false,
  transaction_gas_limit: 20_000_000, block_gas_limit: 30_000_000,
  baseline: baselineName, baseline_sha256: sha256(baselineBytes),
  baseline_source_sha256: sha256(baselineSource), only_add2_source_changed: true,
  prefix_baseline: prefixName, prefix_baseline_sha256: sha256(prefixBytes),
  latest_full_attempt_gate: miller.gate, latest_full_attempt_result: millerName,
  latest_full_attempt_result_sha256: sha256(millerBytes),
  source_fixture: baseline.source_fixture, source_fixture_sha256: sha256(sourceBytes),
  fixture: baseline.fixture, fixture_sha256: sha256(fixtureBytes), aggregate_proof_sha256: sha256(wire),
  operand_source: 'proof.tmipp.gipa.final_b affine x plus y; same full Fp2 operands as B8 multiplication',
  host_operand_paths: hostOperandPaths,
  contract_source: contractName, contract_source_sha256: sha256(contractSource),
  runner_source_sha256: sha256(fs.readFileSync(fileURLToPath(import.meta.url))),
  arithmetic_source_bodies_unchanged: unchangedFunctions,
  add2_source_sha256: sha256(body(contractSource, '_add2')),
  package_lock_sha256: sha256(read('contracts/package-lock.json')), cargo_lock_sha256: sha256(read('Cargo.lock')),
  official_manifest_sha256: sha256(read('configs/protocol_manifest.json')),
  prototype_manifest_sha256: sha256(read('configs/protocol_manifest.JUBJUB-PROTOTYPE.json')),
  runtime_bytecode_bytes: runtime.length, deployed_runtime_sha256: sha256(runtime), deployed_contract: address.toString(),
  deployment: deployment.measurement, fp2_add: fp2Add, comparison,
  boundaries: [boundary], all_boundaries_verified: true,
  measured_window_execution_allocation_improvement: measuredImprovement,
  window_definition: 'same gasleft window around _add2 after canonical decode and before encoding; includes compiler/call and instrumentation overhead',
  gas_accounting: 'total=max(intrinsic+execution,Prague calldata floor) with zero refund; floor extra is NOT arithmetic-window cost',
  primitive_comparison_only: true, arithmetic_transactions: 2, performance_comparison_samples: 1,
  miller_run: false, full_miller_loop_run: false, full_loop_attempts: 0, full_attempt: null,
  full_miller_loop_verified_in_evm: false, full_miller_verification_gas: null,
  evm_final_exponentiation_implemented: false, pairing_to_gt_evm_verified: false,
  exact_snarkpack_evm_verified: false, complete_g5c3: false, snarkpack_verification_gas: null,
  setup_run: false, prove_run: false, private_witness_emitted: false,
  next_gate: measuredImprovement
    ? 'Integrate this unchanged _add2 into a separate B9-based Miller variant and validate the same prefix pair ONLY before any full-loop decision; no cap increase or final exponentiation'
    : 'Retain B10/B9 and inspect isolated addition costs before integration; no full-loop retry',
  limitations: [
    'One fixed public cost sample plus ONE equality/zero correctness boundary, not statistical timing or an exhaustive arithmetic test',
    'Only _add2 allocation/canonical-add implementation changes; the mathematical formula, other arithmetic and suite remain unchanged',
    'Instrumented window cost is not an inlined Miller constant or proof of the unique bottleneck',
    'Prague calldata-floor extra is transaction pricing, not arithmetic execution or memory cost',
    'Allocator-pointer growth is not EVM peak memory or host RSS',
    'No multiplication/prefix/full-loop/setup/prove/scale rerun; no protocol capability promotion',
  ],
};
const outputPath = path.resolve(root, process.argv[2] ?? 'experiments/g5_c3_raw/fp2_add_lowlevel_profile.json');
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(`Fp2 addition: total=${fp2Add.total_gas}, execution=${fp2Add.execution_gas}, window=${fp2Add.instrumented_arithmetic_window_gas}, allocator=${fp2Add.allocator_growth_in_window_bytes}B, MODEXP=${fp2Add.modexp_call_count}, floor_extra=${fp2Add.calldata_floor_extra_gas}`);
console.log(`Fixed-cost comparison improved=${measuredImprovement}; ONE equality/zero boundary PASS; full Miller/GT/SnarkPack remain unverified.`);
console.log(`wrote ${path.relative(root, outputPath)}`);


