// B18: one observational replay of B16; aggregate costs only, unchanged program and 20M cap.
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
const evidenceName = 'experiments/g5_c3_raw/miller_sub_full_evm.json';
const evidenceBytes = read(evidenceName);
const evidence = JSON.parse(evidenceBytes);
const reviewName = 'experiments/g5_c3_raw/miller_budget_review.json';
const reviewBytes = read(reviewName);
const review = JSON.parse(reviewBytes);
assert.equal(evidence.gate, 'G5-C3-B16');
assert.equal(review.gate, 'G5-C3-B17');
assert.equal(sha256(evidenceBytes), review.evidence_sha256);
assert.equal(sha256(read('contracts/scripts/g5c3_miller_sub_full.mjs')), evidence.runner_source_sha256);
assert.equal(sha256(read('contracts/scripts/g5c3_miller_budget.mjs')), review.analysis_source_sha256);
assert.equal(sha256(baselineBytes), evidence.baseline_sha256);
assert.equal(sha256(contractSource), evidence.contract_source_sha256);
const settings = structuredClone(evidence.compiler_settings);
assert.deepEqual(settings, baseline.compiler_settings);
settings.outputSelection['*']['*'].push('evm.deployedBytecode.sourceMap', 'evm.deployedBytecode.generatedSources');
settings.outputSelection['*'][''] = ['ast'];
assert.equal(solc.version(), evidence.compiler);
const sourceUnit = path.basename(sourceName);
const compilation = JSON.parse(solc.compile(JSON.stringify({ language: 'Solidity',
  sources: { [sourceUnit]: { content: contractSource } }, settings })));
for (const error of compilation.errors ?? []) {
  if (error.severity === 'error') throw Error(error.formattedMessage);
}
const compiled = compilation.contracts[sourceUnit].Bls12381MillerLoopSubLowLevel;
const expectedRuntime = Buffer.from(compiled.evm.deployedBytecode.object, 'hex');
assert.equal(sha256(expectedRuntime), evidence.deployed_runtime_sha256);
assert.equal(expectedRuntime.length, evidence.runtime_bytecode_bytes);
const sourceId = compilation.sources[sourceUnit].id;
const ranges = [];
function visit(node, parent = null) {
  if (!node || typeof node !== 'object') return;
  let owner = parent;
  if (node.nodeType === 'FunctionDefinition' || node.nodeType === 'YulFunctionDefinition') {
    const [start, length, file] = node.src.split(':').map(Number);
    owner = node.nodeType === 'FunctionDefinition' ? node.name || '<constructor>' : parent;
    ranges.push({ start, end: start + length, file, owner,
      leaf: node.nodeType === 'YulFunctionDefinition' ? owner + '::' + node.name : owner });
  }
  for (const value of Object.values(node)) {
    if (Array.isArray(value)) for (const item of value) visit(item, owner);
    else if (value && typeof value === 'object') visit(value, owner);
  }
}
visit(compilation.sources[sourceUnit].ast);
ranges.sort((a, b) => (a.end - a.start) - (b.end - b.start));
const entries = compiled.evm.deployedBytecode.sourceMap.split(';');
const location = [-1, -1, -1, '', 0];
const pcMap = new Map();
let instruction = 0;
for (let pc = 0; pc < expectedRuntime.length; instruction++) {
  const entry = entries[instruction];
  if (entry !== undefined) entry.split(':').forEach((value, field) => {
    if (value !== '') location[field] = field === 3 ? value : Number(value);
  });
  const [start, length, file] = location;
  const valid = entry !== undefined && file === sourceId && start >= 0 && length > 0
    && start + length <= Buffer.byteLength(contractSource);
  const range = valid ? ranges.find(r => r.file === file && r.start <= start && r.end >= start + length) : null;
  const bucket = entry === undefined ? '<no-source-map>'
    : file < 0 ? '<unmapped>' : file !== sourceId ? '<compiler-generated:' + file + '>'
      : range?.owner ?? '<contract-scaffolding>';
  pcMap.set(pc, { bucket, leaf: range?.leaf ?? bucket, source_id: file,
    byte_start: start, byte_length: length, instruction_index: instruction });
  const op = expectedRuntime[pc];
  pc += 1 + (op >= 0x60 && op <= 0x7f ? op - 0x5f : 0);
}
const iface = new Interface(compiled.abi);
const factory = new ContractFactory(compiled.abi, '0x' + compiled.evm.bytecode.object);
const common = new Common({ chain: Mainnet, hardfork: Hardfork.Prague });
const vm = await createVM({ common });
assert.equal(common.hardfork(), evidence.hardfork);
const senderKey = hexToBytes('0x' + '2f'.repeat(32));
const sender = createAddressFromPrivateKey(senderKey);
await vm.stateManager.putAccount(sender, createAccount({ nonce: 0n, balance: 10n ** 24n }));
const block = createBlock({ header: { number: 1n, gasLimit: 30_000_000n,
  baseFeePerGas: 1n, timestamp: 1_750_000_000n } }, { common, skipConsensusFormatValidation: true });
let nonce = 0n;
let trace = [];
let lastRootStep = null;
let observeRuntime = false;
let rootGas = 0;
let rootSteps = 0;
let terminal = null;
const opcodeBins = new Map();
const functionBins = new Map();
const leafBins = new Map();
const add = (bins, name, gas) => {
  const item = bins.get(name) ?? { name, charged_gas: 0, executed_steps: 0 };
  item.charged_gas += gas;
  item.executed_steps++;
  bins.set(name, item);
};
vm.evm.events.on('step', step => {
  if (step.depth !== 0) return;
  lastRootStep = { pc: step.pc, opcode: step.opcode.name,
    gas_left_before: Number(step.gasLeft), memory_words_before: Number(step.memoryWordCount) };
  if (!observeRuntime) return;
  const fee = Number(step.opcode.dynamicFee ?? BigInt(step.opcode.fee));
  if (fee > Number(step.gasLeft)) {
    terminal = { ...lastRootStep, requested_opcode_gas: fee, opcode_executed: false,
      ...pcMap.get(step.pc) };
    return;
  }
  rootGas += fee;
  rootSteps++;
  const mapping = pcMap.get(step.pc);
  add(opcodeBins, step.opcode.name, fee);
  add(functionBins, mapping?.bucket ?? '<invalid-pc>', fee);
  add(leafBins, mapping?.leaf ?? '<invalid-pc>', fee);
});
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
  lastRootStep = null;
  const tx = createLegacyTx({ nonce: nonce++, gasLimit: 20_000_000n, gasPrice: 10n,
    value: 0n, data: hexToBytes(data), ...(to ? { to } : {}) }, { common }).sign(senderKey);
  const started = performance.now();
  const result = await runTx(vm, { tx, block });
  return { result, measurement: {
    transaction_hash: bytesToHex(tx.hash()), receipt_status: result.receipt.status,
    total_gas: Number(result.totalGasSpent), execution_gas: Number(result.execResult.executionGasUsed),
    intrinsic_gas: Number(tx.getIntrinsicGas()), minimum_gas_limit_including_calldata_floor: Number(tx.getMinimumGasLimit()),
    gas_refund: Number(result.gasRefund), transaction_gas_limit: 20_000_000,
    calldata_bytes: tx.data.length, calldata_sha256: sha256(tx.data),
    local_vm_wall_ms_with_observer: performance.now() - started,
    evm_active_memory_words_at_exit: result.execResult.runState
      ? Number(result.execResult.runState.memoryWordCount) : null,
    last_root_step: lastRootStep, exception: result.execResult.exceptionError?.error ?? null,
    return_hex: bytesToHex(result.execResult.returnValue),
    modexp_call_count: trace.length, modexp_success_count: trace.filter(x => x.success).length,
    modexp_failure_count: trace.filter(x => !x.success).length,
    modexp_execution_gas: trace.reduce((sum, x) => sum + x.execution_gas, 0),
    modexp_trace_sha256: sha256(JSON.stringify(trace)), modexp_failures: trace.filter(x => !x.success),
  } };
}
const outputPath = path.resolve(root, process.argv[2] ?? 'experiments/g5_c3_raw/miller_cost_attribution.json');
assert.ok(!fs.existsSync(outputPath), 'attribution output exists; inspect rather than rerun');
console.log('G5-C3-B18: one deployment, unchanged B16 runtime and compiler');
const deployment = await transact((await factory.getDeployTransaction()).data);
assert.equal(deployment.result.receipt.status, 1);
const address = deployment.result.createdAddress;
assert.ok(address);
const runtime = await vm.stateManager.getCode(address);
assert.equal(sha256(runtime), evidence.deployed_runtime_sha256);
assert.equal(deployment.measurement.total_gas, evidence.deployment.total_gas);
const fullData = iface.encodeFunctionData('verifyMillerLoopOrRevert', [pointInput, fixture.full_miller_fp12_hex]);
assert.equal(sha256(hexToBytes(fullData)), evidence.full_attempt.calldata_sha256);
observeRuntime = true;
console.log('G5-C3-B18: ONE fixed full64 transaction, same 20M cap; aggregate observer only');
const full = await transact(fullData, address);
observeRuntime = false;
const m = full.measurement;
const mismatches = [];
for (const key of ['transaction_hash', 'receipt_status', 'total_gas', 'execution_gas', 'intrinsic_gas',
  'minimum_gas_limit_including_calldata_floor', 'gas_refund', 'calldata_bytes', 'calldata_sha256',
  'evm_active_memory_words_at_exit', 'exception', 'return_hex', 'modexp_call_count',
  'modexp_success_count', 'modexp_failure_count', 'modexp_execution_gas', 'modexp_trace_sha256']) {
  if (m[key] !== evidence.full_attempt[key]) mismatches.push({ key, expected: evidence.full_attempt[key], actual: m[key] });
}
if (JSON.stringify(m.last_root_step) !== JSON.stringify(evidence.full_attempt.last_root_step)) mismatches.push({ key: 'last_root_step' });
const outOfGas = m.receipt_status === 0 && m.exception === 'out of gas';
const terminalBurn = outOfGas && terminal ? terminal.gas_left_before : null;
const sorted = bins => [...bins.values()].sort((a, b) => b.charged_gas - a.charged_gas)
  .map(x => ({ ...x, percent_of_charged_root: 100 * x.charged_gas / rootGas }));
const opcodeRanking = sorted(opcodeBins);
const functionRanking = sorted(functionBins);
const leafRanking = sorted(leafBins);
const sums = [opcodeRanking, functionRanking, leafRanking].map(items => items.reduce((sum, x) => sum + x.charged_gas, 0));
const accounted = rootGas + m.modexp_execution_gas + (terminalBurn ?? 0);
const reconciliation = {
  charged_root_opcode_gas: rootGas, successful_modexp_child_gas: m.modexp_execution_gas,
  terminal_unexecuted_opcode_burn_gas: terminalBurn, accounted_execution_gas: accounted,
  observed_execution_gas: m.execution_gas, difference_gas: m.execution_gas - accounted,
  root_opcode_partition_sum: sums[0], exclusive_function_partition_sum: sums[1], exclusive_leaf_partition_sum: sums[2],
  passed: terminalBurn !== null && accounted === m.execution_gas && sums.every(sum => sum === rootGas)
    && m.gas_refund === 0 && m.modexp_failure_count === 0
    && m.total_gas === Math.max(m.execution_gas + m.intrinsic_gas, m.minimum_gas_limit_including_calldata_floor),
};
const reproduced = mismatches.length === 0;
const pass = reproduced && reconciliation.passed;
const unknownGas = functionRanking.filter(x => x.name.startsWith('<')).reduce((sum, x) => sum + x.charged_gas, 0);
const leading = functionRanking.find(x => !x.name.startsWith('<'));
// A narrow candidate only, not an automatic optimization or full-cost prediction.
const selectedCandidate = pass && leading?.percent_of_charged_root >= 20 && unknownGas / rootGas < 0.1
  ? leading.name : null;
const nextGate = selectedCandidate
  ? 'B19: read-only review of ' + selectedCandidate + ' and compiler-generated overhead; select ONE justified arithmetic/control change before any new EVM test'
  : 'B19: pause arithmetic changes; read-only route decision from this attribution, without a full retry or wrapper implementation';
const report = {
  schema: 'VV/G5-C3/MILLER-COST-ATTRIBUTION/v1', gate: 'G5-C3-B18',
  status: pass ? 'PASS_SAME_PROGRAM_FAILED_ATTEMPT_COST_ATTRIBUTION_ONLY' : 'FAIL_ATTRIBUTION_REPRODUCTION_OR_RECONCILIATION',
  classification: 'one-fixed-input-failed-full-loop-cost-attribution', derived_nonbenchmark: false,
  recorded_at_utc: new Date().toISOString(), git_commit: metadata('git', ['rev-parse', 'HEAD']),
  git_dirty: metadata('git', ['status', '--porcelain']).length > 0,
  cpu: os.cpus()[0].model, os: os.platform(), arch: os.arch(), node_version: process.version,
  compiler: solc.version(), compiler_settings: settings, hardfork: common.hardfork(),
  baseline: evidenceName, baseline_sha256: sha256(evidenceBytes),
  budget_review: reviewName, budget_review_sha256: sha256(reviewBytes),
  prefix_evidence_source: baselineName, prefix_evidence_sha256: sha256(baselineBytes),
  contract_source: sourceName, contract_source_sha256: sha256(contractSource),
  runner_source: path.relative(root, fileURLToPath(import.meta.url)),
  runner_source_sha256: sha256(fs.readFileSync(fileURLToPath(import.meta.url))),
  fixture: baseline.fixture, fixture_sha256: sha256(fixtureBytes),
  source_fixture: baseline.source_fixture, source_fixture_sha256: sha256(publicBytes),
  endpoint_fixture: baseline.endpoint_fixture, endpoint_fixture_sha256: sha256(endpointBytes),
  aggregate_proof_sha256: sha256(wire),
  package_lock_sha256: sha256(read('contracts/package-lock.json')), cargo_lock_sha256: sha256(read('Cargo.lock')),
  official_manifest_sha256: sha256(read('configs/protocol_manifest.json')),
  prototype_manifest_sha256: sha256(read('configs/protocol_manifest.JUBJUB-PROTOTYPE.json')),
  observer_implementation_source_sha256: sha256(read('contracts/node_modules/@ethereumjs/evm/dist/esm/interpreter.js')),
  runtime_bytecode_bytes: runtime.length, deployed_runtime_sha256: sha256(runtime),
  source_map_sha256: sha256(compiled.evm.deployedBytecode.sourceMap),
  source_mapping: { source_unit: sourceUnit, original_source_id: sourceId,
    generated_source_ids: compiled.evm.deployedBytecode.generatedSources.map(x => x.id),
    method: 'exclusive smallest containing AST function span per opcode source map; no dynamic/inclusive call-stack inference',
    mapped_function_gas: rootGas - unknownGas, explicit_unmapped_generated_scaffolding_gas: unknownGas,
    mapped_function_percent_of_charged_root: 100 * (rootGas - unknownGas) / rootGas },
  transaction_gas_limit: 20_000_000, block_gas_limit: 30_000_000,
  chain_hard_limit_established: false, budget_changed: false, contract_changed: false,
  deployed_contract: address.toString(), deployment: deployment.measurement, full_attempt: m,
  reproduction_matches_b16: reproduced, reproduction_mismatches: mismatches,
  reconciliation, terminal_unexecuted_step: terminal, root_executed_steps: rootSteps,
  root_opcode_ranking: opcodeRanking, exclusive_solidity_function_ranking: functionRanking,
  exclusive_source_leaf_ranking: leafRanking, per_step_trace_saved: false,
  attribution_partition_warning: 'opcode/function/leaf rankings are alternative partitions of SAME root gas; never add them together; MODEXP child gas is separate',
  routing_decision: { leading_mapped_function: leading?.name ?? null,
    candidate_for_read_only_review: selectedCandidate, implementation_authorized_or_executed_by_this_gate: false,
    software_changes: pass ? 'HOLD_UNTIL_ONE_FOCUSED_SOURCE_REVIEW' : 'HOLD_ATTRIBUTION_FAILED',
    wrapper_work: 'DEFERRED_REQUIRES_NEW_VERIFIER_RELATION_AND_RESOURCE_PLAN' },
  latest_full_attempt_gate: 'G5-C3-B18', latest_full_attempt_result: path.relative(root, outputPath),
  previous_full_attempt_gate: evidence.gate, latest_prefix_gate: baseline.gate,
  target_verifier_status: outOfGas ? evidence.target_verifier_status : 'BLOCKED_ATTRIBUTION_REPLAY_CHANGED_OUTCOME',
  target_verifier_status_source: path.relative(root, outputPath),
  full_steps: 64, full_attempted: true, miller_run: true, full_miller_loop_run: true,
  full_loop_attempts: 1, evm_transactions_executed: 2, optimization_implemented: false,
  full_miller_loop_completed: false, full_miller_loop_verified_in_evm: false,
  full_miller_verification_gas: null, estimated_full_gas: null, completion_percent: null,
  failed_attempt_gas_is_capped_spend_not_completed_loop_cost: outOfGas && m.total_gas === 20_000_000,
  prefix_correctness_verified_in_evm: baseline.prefix_correctness_verified_in_evm,
  wrong_prefix_target_rejected_in_evm: baseline.wrong_prefix_target_rejected_in_evm,
  prefix_checks_executed_this_run: false, evm_final_exponentiation_implemented: false,
  pairing_to_gt_evm_verified: false, exact_snarkpack_evm_verified: false,
  snarkpack_verification_gas: null, complete_g5c3: false, protocol_capabilities_changed: false,
  setup_run: false, prove_run: false, private_witness_emitted: false, next_gate: nextGate,
  limitations: [
    'Attribution PASS is not Miller/full SnarkPack correctness; same 20M root OOG remains blocked',
    'Costs rank only the executed prefix of this failed full64 attempt, not all 64 steps or total completion cost',
    'Source-map ownership is exclusive syntactic ownership, not dynamic caller cost; compiler-generated costs remain explicit',
    'Observer wall time is instrumentation overhead, not an uninstrumented performance result or statistical sample',
    'No budget/compiler/curve/Suite/Manifest change, extra negative matrix, final exponentiation, setup/prove, scale or RPC experiment',
  ],
};
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ status: report.status, reproduced, reconciliation,
  top_functions: functionRanking.slice(0, 8), top_opcodes: opcodeRanking.slice(0, 8), next_gate: nextGate }));
console.log('wrote ' + path.relative(root, outputPath));
if (!pass) process.exitCode = 1;

