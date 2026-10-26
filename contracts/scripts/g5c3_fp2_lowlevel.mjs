// Fp2 allocation-only comparison: two original cost cases + three necessary Fp2 boundaries; no Miller/proving rerun.
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
const millerName = 'experiments/g5_c3_raw/miller_lowlevel_evm.json';
const millerBytes = read(millerName);
const miller = JSON.parse(millerBytes);
const sourceBytes = read(baseline.source_fixture);
const source = JSON.parse(sourceBytes);
const fixtureBytes = read(baseline.fixture);
const fixture = JSON.parse(fixtureBytes);
const baselineSource = read(baseline.contract_source).toString('utf8');
const contractName = 'contracts/src/Bls12381Fp2LowLevelProfile.sol';
const contractSource = read(contractName).toString('utf8');
const { Fp, Fp2 } = bls12_381.fields;
assert.equal(baseline.gate, 'G5-C3-B6');
assert.equal(miller.gate, 'G5-C3-B7');
assert.equal(sha256(read(miller.contract_source)), miller.contract_source_sha256);
assert.equal(sha256(read('contracts/scripts/g5c3_miller_lowlevel.mjs')), miller.runner_source_sha256);
assert.equal(sha256(sourceBytes), baseline.source_fixture_sha256);
assert.equal(sha256(fixtureBytes), baseline.fixture_sha256);
assert.equal(sha256(baselineSource), baseline.contract_source_sha256);
assert.equal(sha256(read('contracts/scripts/g5c3_field_lowlevel.mjs')), baseline.runner_source_sha256);
assert.equal(source.schema, 'VV/G5-C1/SNARKPACK-PUBLIC-FIXTURE/v1');
assert.equal(fixture.schema, 'VV/G5-C3/MILLER-LOOP/v1');
for (const [name, key] of [
  ['Cargo.lock', 'cargo_lock_sha256'], ['contracts/package-lock.json', 'package_lock_sha256'],
  ['configs/protocol_manifest.json', 'official_manifest_sha256'],
  ['configs/protocol_manifest.JUBJUB-PROTOTYPE.json', 'prototype_manifest_sha256'],
]) {
  assert.equal(sha256(read(name)), baseline[key]);
  assert.equal(sha256(read(name)), miller[key]);
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
  '_square', '_add2', '_sub2', '_mul', '_readFp', '_read2', '_writeFp', '_write2', 'profileFpMul', 'profileFp2Mul'];
for (const name of unchangedFunctions) assert.equal(body(contractSource, name), body(baselineSource, name));
const expectedSource = baselineSource.replace('contract Bls12381FieldLowLevelProfile {',
  'contract Bls12381Fp2LowLevelProfile {').replace(body(baselineSource, '_mul2'), body(contractSource, '_mul2'));
assert.equal(contractSource, expectedSource, 'only contract name and _mul2 may change');
const wire = Buffer.from(source.aggregate_proof_hex, 'hex');
assert.equal(wire.length, 5236);
assert.equal(sha256(wire), baseline.aggregate_proof_sha256);
assert.equal(sha256(wire), source.aggregate_proof_sha256);
assert.equal(sha256(wire), miller.aggregate_proof_sha256);
const q = bls12_381.G2.Point.fromBytes(wire.subarray(4516, 4612)).toAffine();
const fpBE = (value) => {
  assert.ok(value >= 0n && value < Fp.ORDER);
  return Buffer.from(value.toString(16).padStart(128, '0'), 'hex');
};
const fp2BE = (value) => bytesToHex(Buffer.concat([fpBE(value.c0), fpBE(value.c1)]));
const fpLeft = bytesToHex(fpBE(q.x.c0));
const fpRight = bytesToHex(fpBE(q.y.c0));
const fpExpected = bytesToHex(fpBE((q.x.c0 * q.y.c0) % Fp.ORDER));
assert.equal(fpExpected, bytesToHex(fpBE(Fp.mul(q.x.c0, q.y.c0))));
const mod = (value) => ((value % Fp.ORDER) + Fp.ORDER) % Fp.ORDER;
const fp2Expected = fp2BE({ c0: mod(q.x.c0 * q.y.c0 - q.x.c1 * q.y.c1),
  c1: mod(q.x.c0 * q.y.c1 + q.x.c1 * q.y.c0) });
assert.equal(fp2Expected, fp2BE(Fp2.mul(q.x, q.y)));
const fixedPoints = Buffer.from(fixture.points_eip2537_hex.slice(2), 'hex');
assert.equal(bytesToHex(fixedPoints.subarray(128, 256)), fp2BE(q.x));
assert.equal(bytesToHex(fixedPoints.subarray(256, 384)), fp2BE(q.y));

const settings = { optimizer: { enabled: true, runs: 200 }, evmVersion: 'prague',
  outputSelection: { '*': { '*': ['abi', 'evm.bytecode.object', 'evm.deployedBytecode.object'] } } };
assert.equal(solc.version(), baseline.compiler);
assert.deepEqual(settings, baseline.compiler_settings);
const compilation = JSON.parse(solc.compile(JSON.stringify({ language: 'Solidity',
  sources: { 'Bls12381Fp2LowLevelProfile.sol': { content: contractSource } }, settings })));
for (const error of compilation.errors ?? []) {
  if (error.severity === 'error') throw Error(error.formattedMessage);
  console.warn(error.formattedMessage);
}
const compiled = compilation.contracts['Bls12381Fp2LowLevelProfile.sol'].Bls12381Fp2LowLevelProfile;
const iface = new Interface(compiled.abi);
const factory = new ContractFactory(compiled.abi, `0x${compiled.evm.bytecode.object}`);
const common = new Common({ chain: Mainnet, hardfork: Hardfork.Prague });
assert.equal(common.hardfork(), baseline.hardfork);
const vm = await createVM({ common });
// Public deterministic local-only key; never touches RPC/user funds.
const senderKey = hexToBytes(`0x${'27'.repeat(32)}`);
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
console.log('G5-C3-B8: deploy isolated cost instrumentation; no Miller/proving runs');
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
  assert.equal(computed, expected, `${label} product mismatch`);
  const m = transaction.measurement;
  assert.equal(m.modexp_call_count, calls);
  assert.ok(m.modexp_calls.every((x) => x.success && x.return_bytes === 64));
  assert.equal(m.gas_refund, 0);
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
    allocator_growth_in_window_bytes: Number(freeAfter - freeBefore) };
}
console.log('G5-C3-B8: one Fp multiply and one Fp2 multiply on saved endpoint coordinates');
const fp = await profile('Fp multiply', 'profileFpMul', fpLeft, fpRight, fpExpected, 2);
const fp2 = await profile('Fp2 multiply', 'profileFp2Mul', fp2BE(q.x), fp2BE(q.y), fp2Expected, 6);

assert.equal(fp.left_hex, baseline.fp.left_hex);
assert.equal(fp.right_hex, baseline.fp.right_hex);
assert.equal(fp.expected_hex, baseline.fp.expected_hex);
assert.equal(fp2.left_hex, baseline.fp2.left_hex);
assert.equal(fp2.right_hex, baseline.fp2.right_hex);
assert.equal(fp2.expected_hex, baseline.fp2.expected_hex);
const limbBase = 1n << 256n;
const cases = [
  { id: 'u_squared', a: { c0: 0n, c1: 1n }, b: { c0: 0n, c1: 1n } },
  { id: 'upper_square', a: { c0: Fp.ORDER - 1n, c1: Fp.ORDER - 1n },
    b: { c0: Fp.ORDER - 1n, c1: Fp.ORDER - 1n } },
  { id: 'limb_carry_borrow', a: { c0: Fp.ORDER - 1n, c1: limbBase - 1n },
    b: { c0: limbBase + 1n, c1: Fp.ORDER - 2n } },
];
const boundaries = [];
console.log('G5-C3-B8: three necessary Fp2 sum/subtract/zero-component boundaries, not performance samples');
for (const entry of cases) {
  const sumA = entry.a.c0 + entry.a.c1;
  const sumB = entry.b.c0 + entry.b.c1;
  const t0 = mod(entry.a.c0 * entry.b.c0);
  const t1 = mod(entry.a.c1 * entry.b.c1);
  const t2 = mod(mod(sumA) * mod(sumB));
  const first = mod(t2 - t0);
  const conditions = {
    sum_reduction: sumA >= Fp.ORDER || sumB >= Fp.ORDER,
    sum_low_carry: entry.a.c0 % limbBase + entry.a.c1 % limbBase >= limbBase
      || entry.b.c0 % limbBase + entry.b.c1 % limbBase >= limbBase,
    sum_reduction_low_borrow: sumA >= Fp.ORDER && sumA % limbBase < Fp.ORDER % limbBase
      || sumB >= Fp.ORDER && sumB % limbBase < Fp.ORDER % limbBase,
    c0_negative_difference: t0 < t1,
    c0_nonnegative_low_borrow: t0 >= t1 && t0 % limbBase < t1 % limbBase,
    c1_first_negative_difference: t2 < t0,
    c1_second_negative_difference: first < t1,
    zero_output_component: mod(t0 - t1) === 0n || mod(first - t1) === 0n,
  };
  const expected = fp2BE({ c0: mod(entry.a.c0 * entry.b.c0 - entry.a.c1 * entry.b.c1),
    c1: mod(entry.a.c0 * entry.b.c1 + entry.a.c1 * entry.b.c0) });
  assert.equal(expected, fp2BE(Fp2.mul(entry.a, entry.b)));
  boundaries.push({ id: entry.id, host_branch_conditions: conditions,
    comparison_performance_sample: false,
    ...await profile(entry.id, 'profileFp2Mul', fp2BE(entry.a), fp2BE(entry.b), expected, 6) });
}
for (const condition of Object.keys(boundaries[0].host_branch_conditions)) {
  assert.ok(boundaries.some((entry) => entry.host_branch_conditions[condition]), condition);
}
const compare = (current, previous) => ({
  baseline_window_gas: previous.instrumented_arithmetic_window_gas,
  optimized_window_gas: current.instrumented_arithmetic_window_gas,
  window_gas_reduction_percent: 100 * (1 - current.instrumented_arithmetic_window_gas / previous.instrumented_arithmetic_window_gas),
  baseline_total_gas: previous.total_gas, optimized_total_gas: current.total_gas,
  baseline_allocator_growth_bytes: previous.allocator_growth_in_window_bytes,
  optimized_allocator_growth_bytes: current.allocator_growth_in_window_bytes,
  baseline_window_memory_gas: previous.window_evm_memory_expansion_gas,
  optimized_window_memory_gas: current.window_evm_memory_expansion_gas,
  modexp_count_unchanged: current.modexp_call_count === previous.modexp_call_count,
});
const measuredImprovement = fp2.instrumented_arithmetic_window_gas < baseline.fp2.instrumented_arithmetic_window_gas;

const report = {
  gate: 'G5-C3-B8', status: measuredImprovement ? 'PASS_FP2_FIXED_CASES_AND_COST_REDUCTION_ONLY' : 'PASS_FP2_CORRECTNESS_NO_WINDOW_REDUCTION',
  target_verifier_status: miller.target_verifier_status,
  recorded_at_utc: new Date().toISOString(), git_commit: metadata('git', ['rev-parse', 'HEAD']),
  git_dirty: metadata('git', ['status', '--porcelain']).length > 0,
  cpu: os.cpus()[0].model, os: os.platform(), arch: os.arch(), node_version: process.version,
  compiler: solc.version(), compiler_settings: settings, hardfork: common.hardfork(),
  execution_scope: 'one deployment, two original fixed-cost and three Fp2-boundary local Prague transactions; no Miller/RPC',
  optimization_scope: 'only _mul2 sum/subtraction allocation; exact B6 _mul, three-product Karatsuba, field/codec and profile entry points unchanged',
  transaction_gas_limit: 20_000_000, block_gas_limit: 30_000_000,
  baseline: baselineName, baseline_sha256: sha256(baselineBytes), baseline_source_sha256: sha256(baselineSource),
  miller_baseline: millerName, miller_baseline_sha256: sha256(millerBytes),
  only_mul2_source_changed: true, fp_control_source_unchanged: true,
  source_fixture: baseline.source_fixture, source_fixture_sha256: sha256(sourceBytes),
  fixture: baseline.fixture, fixture_sha256: sha256(fixtureBytes), aggregate_proof_sha256: sha256(wire),
  operand_source: 'proof.tmipp.gipa.final_b affine x/y; Fp uses x.c0/y.c0; Fp2 uses full x/y',
  contract_source: contractName, contract_source_sha256: sha256(contractSource),
  runner_source_sha256: sha256(fs.readFileSync(fileURLToPath(import.meta.url))),
  arithmetic_source_bodies_unchanged: unchangedFunctions,
  package_lock_sha256: sha256(read('contracts/package-lock.json')), cargo_lock_sha256: sha256(read('Cargo.lock')),
  official_manifest_sha256: sha256(read('configs/protocol_manifest.json')),
  prototype_manifest_sha256: sha256(read('configs/protocol_manifest.JUBJUB-PROTOTYPE.json')),
  runtime_bytecode_bytes: runtime.length, deployed_runtime_sha256: sha256(runtime), deployed_contract: address.toString(),
  deployment: deployment.measurement, fp, fp2, boundaries,
  all_boundaries_verified: true, measured_fp2_window_improvement: measuredImprovement,
  comparison: { fp: compare(fp, baseline.fp), fp2: compare(fp2, baseline.fp2) },
  window_definition: 'gasleft difference around _mul/_mul2 after canonical decode and before output encoding; includes compiler/call and measurement overhead',
  residual_definition: 'window gas minus actual MODEXP execution gas; not a disjoint attribution to Solidity, memory or call costs',
  opcode_accounting: 'root opcode dynamicFee already includes base/dynamic charges; add child MODEXP execution once; STATICCALL gas may overlap memory expansion and is not added twice',
  primitive_comparison_only: true, miller_run: false, full_miller_loop_verified_in_evm: false,
  evm_final_exponentiation_implemented: false, pairing_to_gt_evm_verified: false,
  exact_snarkpack_evm_verified: false, complete_g5c3: false, snarkpack_verification_gas: null,
  setup_run: false, prove_run: false, private_witness_emitted: false,
  next_gate: measuredImprovement ? 'Integrate this unchanged _mul2 into a new low-level-Miller variant and check the same prefix pair first; assess measured gain before any full-loop retry, keep20M cap and no final exponentiation' : 'Retain B6/B7 and investigate the isolated Fp2 allocation path before integration; no full-loop retry',
  limitations: [
    'Isolated instrumented call-site cost is not an intrinsic per-operation constant for inlined Miller code',
    'Only aggregate opcode/call/memory counters are collected; no exclusive source-level attribution or exhaustive algebra validation',
    'One public Fp control pair, one public Fp2 cost pair and three Fp2 sum/subtract boundaries; no statistical timing or E2E claim',
    'Allocator pointer growth is not peak EVM memory or host RSS; exit memory words/cost are separately recorded',
    'Residual includes STATICCALL/copy/arithmetic/allocation/instrumentation and cannot be assigned entirely to any one category',
    'No full Miller loop, pairing target or SnarkPack verification is executed by this profiling gate',
  ],
};
const outputPath = path.resolve(root, process.argv[2] ?? 'experiments/g5_c3_raw/fp2_lowlevel_profile.json');
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`);
for (const item of [fp, fp2]) console.log(`${item.operation}: total=${item.total_gas}, window=${item.instrumented_arithmetic_window_gas}, MODEXP=${item.modexp_execution_gas} gas / ${item.modexp_call_count} calls`);
console.log('Fp2 isolated correctness PASS; full Miller/GT/SnarkPack capabilities remain BLOCKED.');
console.log(`wrote ${path.relative(root, outputPath)}`);


