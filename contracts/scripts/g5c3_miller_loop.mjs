// Real software Miller-loop component, not final exponentiation or a verifier.
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
import { traceSnarkpackB2 } from './snarkpack_transcript_g5c2.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const read = (name) => fs.readFileSync(path.join(root, name));
const sha256 = (value) => createHash('sha256').update(value).digest('hex');
const metadata = (cmd, args) => execFileSync(cmd, args, { cwd: root, encoding: 'utf8' }).trim();
const sourceName = 'contracts/src/Bls12381MillerLoop.sol';
const publicName = 'fixtures/snarkpack_g5c1_public_v1.json';
const endpointName = 'fixtures/snarkpack_g5c3_tipp_endpoint_v1.json';
const publicBytes = read(publicName);
const source = JSON.parse(publicBytes);
const endpointBytes = read(endpointName);
const endpoint = JSON.parse(endpointBytes);
const c2Bytes = read('fixtures/snarkpack_g5c2_transcript_v1.json');
const c2 = JSON.parse(c2Bytes);
const packages = JSON.parse(read('contracts/package-lock.json'));
const { Fp, Fp2, Fp6, Fp12, Fr } = bls12_381.fields;
assert.equal(source.schema, 'VV/G5-C1/SNARKPACK-PUBLIC-FIXTURE/v1');
assert.equal(source.aggregate_version, 'V2');
assert.equal(source.batch_size, 2);
assert.equal(endpoint.schema, 'VV/G5-C3/TIPP-ENDPOINT/v1');
assert.equal(endpoint.source_fixture_sha256, sha256(publicBytes));
assert.equal(endpoint.c2_trace_fixture_sha256, sha256(c2Bytes));
assert.equal(endpoint.private_witness_emitted, false);
assert.equal(packages.packages['node_modules/@noble/curves'].version, '2.4.0');
const wire = Buffer.from(source.aggregate_proof_hex, 'hex');
assert.equal(wire.length, 5236);
for (const value of [source, endpoint, c2]) assert.equal(value.aggregate_proof_sha256, sha256(wire));
const aBytes = wire.subarray(4468, 4516);
const bBytes = wire.subarray(4516, 4612);
const a = bls12_381.G1.Point.fromBytes(aBytes);
const b = bls12_381.G2.Point.fromBytes(bBytes);
assert.equal(aBytes.toString('hex'), endpoint.final_a_compressed_hex);
assert.equal(bBytes.toString('hex'), endpoint.final_b_compressed_hex);
const p = a.toAffine();
const q = b.toAffine();
const coefficients6 = (value) => [value.c0.c0, value.c0.c1, value.c1.c0,
  value.c1.c1, value.c2.c0, value.c2.c1];
const fpBE = (value, length = 64) => {
  assert.ok(value >= 0n && value < Fp.ORDER);
  return Buffer.from(value.toString(16).padStart(length * 2, '0'), 'hex');
};
const encode12 = (value) => `0x${Buffer.concat([
  ...coefficients6(value.c0), ...coefficients6(value.c1),
].map((value) => fpBE(value))).toString('hex')}`;
const fromLE = (bytes) => BigInt(`0x${Buffer.from(bytes).reverse().toString('hex')}`);
function decodeGT(offset) {
  const compressed = wire.subarray(offset, offset + 288);
  const values = Array.from({ length: 6 }, (_, i) => {
    const value = fromLE(compressed.subarray(i * 48, (i + 1) * 48));
    assert.ok(value < Fp.ORDER);
    return value;
  });
  const torus = Fp6.fromBigSix(values);
  const value = Fp12.mul(Fp12.create({ c0: torus, c1: Fp6.ONE }),
    Fp12.inv(Fp12.create({ c0: torus, c1: Fp6.neg(Fp6.ONE) })));
  assert.ok(Fp12.eql(Fp12.pow(value, Fr.ORDER), Fp12.ONE));
  const roundtrip = Fp6.mul(Fp6.add(value.c0, Fp6.ONE), Fp6.inv(value.c1));
  assert.deepEqual(Buffer.concat(coefficients6(roundtrip)
    .map((x) => fpBE(x, 48).reverse())), compressed);
  return value;
}
// C2 proof/statement challenge binding is independently recomputed, not accepted
// from the supplied target. Precomputed lines below are REFERENCE ONLY: no line,
// G2 state, or accumulator is ever supplied to the Solidity entry point.
const traces = traceSnarkpackB2(source.aggregate_proof_hex, source.external_transcript_hex);
assert.deepEqual(traces, c2.valid);
const cInv = fromLE(Buffer.from(traces.find((row) => row.id === 'gipa-0-round').challenge_le_hex, 'hex'));
const c = Fr.inv(cInv);
const target = Fp12.mul(Fp12.mul(decodeGT(1152), Fp12.pow(decodeGT(3796), c)),
  Fp12.pow(decodeGT(4084), cInv));
assert.equal(encode12(target), endpoint.target_fp12_hex);
const digits = [];
for (let x = 0xd201000000010000n; x > 1n; x >>= 1n) {
  if (!(x & 1n)) digits.unshift(0);
  else if ((x & 3n) === 3n) { digits.unshift(-1); x += 1n; }
  else digits.unshift(1);
}
assert.equal(digits.length, 64);
assert.deepEqual(digits.flatMap((value, i) => value ? [[i, value]] : []),
  [[1, -1], [3, 1], [6, 1], [15, 1], [47, 1]]);
const lines = bls12_381.utils.calcPairingPrecomputes(b);
assert.equal(lines.length, digits.length);
assert.deepEqual(lines.map((row) => row.length), digits.map((d) => d ? 2 : 1));
const prefixSteps = 2; // First signed addition, a square and three line products.
let accumulator = Fp12.ONE;
let prefix;
for (let i = 0; i < lines.length; i++) {
  if (i !== 0) accumulator = Fp12.sqr(accumulator);
  for (const [l0, l1, l4] of lines[i]) {
    const sparse = Fp12.create({
      c0: Fp6.create({ c0: l0, c1: Fp2.mul(l1, p.x), c2: Fp2.ZERO }),
      c1: Fp6.create({ c0: Fp2.ZERO, c1: Fp2.mul(l4, p.y), c2: Fp2.ZERO }),
    });
    accumulator = Fp12.mul(accumulator, sparse);
  }
  if (i + 1 === prefixSteps) prefix = accumulator;
}
const fullMiller = Fp12.conjugate(accumulator); // Negative BLS parameter, full loop only.
assert.ok(Fp12.eql(fullMiller, bls12_381.pairing(a, b, false)));
const finalPairing = Fp12.finalExponentiate(fullMiller);
assert.equal(encode12(finalPairing), endpoint.pairing_fp12_hex);
assert.ok(Fp12.eql(finalPairing, target));
assert.ok(!Fp12.eql(fullMiller, finalPairing), 'Miller output is not yet the final pairing');
const pointInput = bytesToHex(Buffer.concat([fpBE(p.x), fpBE(p.y),
  fpBE(q.x.c0), fpBE(q.x.c1), fpBE(q.y.c0), fpBE(q.y.c1)]));
const fixtureName = 'fixtures/snarkpack_g5c3_miller_loop_v1.json';
const fixture = {
  schema: 'VV/G5-C3/MILLER-LOOP/v1', encoding: endpoint.encoding,
  source_fixture: publicName, source_fixture_sha256: sha256(publicBytes),
  endpoint_fixture: endpointName, endpoint_fixture_sha256: sha256(endpointBytes),
  aggregate_proof_sha256: sha256(wire), points_eip2537_hex: pointInput,
  naf_digits: digits, full_steps: digits.length, prefix_steps: prefixSteps,
  prefix_semantics: 'raw accumulator after two loop steps; no final negative-x conjugation',
  prefix_fp12_hex: encode12(prefix), full_miller_fp12_hex: encode12(fullMiller),
  final_pairing_fp12_hex: encode12(finalPairing),
  reference: 'Noble 2.4.0; manual dense sparse-line multiplication matches pairing(false); finalExponentiate matches saved native blstrs target',
  host_lines_or_state_supplied_to_contract: false, setup_run: false, prove_run: false,
  private_witness_emitted: false,
};
const fixtureBytes = Buffer.from(`${JSON.stringify(fixture, null, 2)}\n`);
fs.writeFileSync(path.join(root, fixtureName), fixtureBytes);
const wrongBytes = Buffer.from(hexToBytes(fixture.prefix_fp12_hex));
const first = BigInt(`0x${wrongBytes.subarray(0, 64).toString('hex')}`);
fpBE((first + 1n) % Fp.ORDER).copy(wrongBytes);
const wrongExpected = bytesToHex(wrongBytes);

const contractSource = read(sourceName).toString('utf8');
const settings = { optimizer: { enabled: true, runs: 200 }, evmVersion: 'prague',
  outputSelection: { '*': { '*': ['abi', 'evm.bytecode.object', 'evm.deployedBytecode.object'] } } };
assert.match(solc.version(), /^0\.8\.30\+/);
const compilation = JSON.parse(solc.compile(JSON.stringify({ language: 'Solidity',
  sources: { 'Bls12381MillerLoop.sol': { content: contractSource } }, settings })));
for (const error of compilation.errors ?? []) {
  if (error.severity === 'error') throw Error(error.formattedMessage);
  console.warn(error.formattedMessage);
}
const compiled = compilation.contracts['Bls12381MillerLoop.sol'].Bls12381MillerLoop;
const iface = new Interface(compiled.abi);
const factory = new ContractFactory(compiled.abi, `0x${compiled.evm.bytecode.object}`);
const common = new Common({ chain: Mainnet, hardfork: Hardfork.Prague });
const vm = await createVM({ common });
// Public local-only deterministic key. Never connects to RPC or user accounts.
const senderKey = hexToBytes(`0x${'21'.repeat(32)}`);
const sender = createAddressFromPrivateKey(senderKey);
await vm.stateManager.putAccount(sender, createAccount({ nonce: 0n, balance: 10n ** 24n }));
const block = createBlock({ header: {
  number: 1n, gasLimit: 30_000_000n, baseFeePerGas: 1n, timestamp: 1_750_000_000n,
} }, { common, skipConsensusFormatValidation: true });
const stack = [];
let trace = [];
let nonce = 0n;
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
  let revertReason = null;
  if (error?.error === 'revert') {
    try { revertReason = iface.parseError(bytesToHex(result.execResult.returnValue))?.args[0] ?? null; }
    catch { /* unrecognized revert remains explicit in return_hex */ }
  }
  return { result, measurement: {
    transaction_hash: bytesToHex(tx.hash()), receipt_status: result.receipt.status,
    total_gas: Number(result.totalGasSpent), execution_gas: Number(result.execResult.executionGasUsed),
    transaction_gas_limit: 20_000_000, calldata_bytes: tx.data.length,
    local_vm_wall_ms: performance.now() - started,
    evm_active_memory_words_at_exit: result.execResult.runState
      ? Number(result.execResult.runState.memoryWordCount) : null,
    exception: error?.error ?? null, revert_reason: revertReason,
    return_hex: bytesToHex(result.execResult.returnValue),
    modexp_call_count: trace.length, modexp_success_count: trace.filter((x) => x.success).length,
    modexp_failure_count: trace.filter((x) => !x.success).length,
    modexp_execution_gas: trace.reduce((sum, x) => sum + x.execution_gas, 0),
    modexp_trace_sha256: sha256(JSON.stringify(trace)),
    modexp_failures: trace.filter((x) => !x.success),
  } };
}
console.log('G5-C3-B2: deploy software Miller-loop component; fixed 20M transaction budget');
const deployment = await transact((await factory.getDeployTransaction()).data);
assert.equal(deployment.result.receipt.status, 1);
const address = deployment.result.createdAddress;
assert.ok(address);
const runtime = await vm.stateManager.getCode(address);
assert.ok(runtime.length > 0 && runtime.length <= 24576);
const validData = iface.encodeFunctionData('verifyMillerPrefixOrRevert',
  [pointInput, prefixSteps, fixture.prefix_fp12_hex]);
const wrongData = iface.encodeFunctionData('verifyMillerPrefixOrRevert',
  [pointInput, prefixSteps, wrongExpected]);
const decodedValid = iface.decodeFunctionData('verifyMillerPrefixOrRevert', validData);
const decodedWrong = iface.decodeFunctionData('verifyMillerPrefixOrRevert', wrongData);
assert.equal(decodedValid[0], decodedWrong[0]);
assert.equal(decodedValid[1], decodedWrong[1]);
assert.notEqual(decodedValid[2], decodedWrong[2]);
console.log('G5-C3-B2: execute two-step prefix positive and target-only negative');
const valid = await transact(validData, address);
assert.equal(valid.result.receipt.status, 1, 'correct prefix failed');
const returned = iface.decodeFunctionResult('verifyMillerPrefixOrRevert',
  bytesToHex(valid.result.execResult.returnValue))[0];
assert.equal(returned, fixture.prefix_fp12_hex);
const wrong = await transact(wrongData, address);
assert.equal(wrong.result.receipt.status, 0);
assert.equal(wrong.measurement.exception, 'revert');
assert.equal(wrong.measurement.revert_reason, 'INVALID_MILLER_RESULT');
assert.equal(wrong.measurement.modexp_call_count, valid.measurement.modexp_call_count);
for (const item of [valid, wrong]) assert.equal(item.measurement.modexp_failure_count, 0);
console.log('G5-C3-B2: one full 64-step attempt, same contract and fixed 20M budget');
const full = await transact(iface.encodeFunctionData('verifyMillerLoopOrRevert',
  [pointInput, fixture.full_miller_fp12_hex]), address);
const fullVerified = full.result.receipt.status === 1;
if (fullVerified) {
  assert.equal(iface.decodeFunctionResult('verifyMillerLoopOrRevert',
    bytesToHex(full.result.execResult.returnValue))[0], fixture.full_miller_fp12_hex);
} else {
  // Logic mismatch is a failed experiment, never disguised as a resource block.
  assert.equal(full.measurement.exception, 'out of gas', 'full loop failed for a non-OOG reason');
}
const report = {
  gate: 'G5-C3-B2', status: fullVerified ? 'PASS_SOFTWARE_MILLER_LOOP_COMPONENT_ONLY'
    : 'PARTIAL_PASS_MILLER_PREFIX_FULL_LOOP_OOG_20M',
  target_verifier_status: fullVerified ? 'BLOCKED_FINAL_EXPONENTIATION_AND_GT_TARGET_BINDING_MISSING'
    : 'BLOCKED_SOFTWARE_MILLER_LOOP_EXCEEDS_20M_EXPERIMENT_BUDGET',
  recorded_at_utc: new Date().toISOString(), git_commit: metadata('git', ['rev-parse', 'HEAD']),
  git_dirty: metadata('git', ['status', '--porcelain']).length > 0,
  cpu: os.cpus()[0].model, os: os.platform(), arch: os.arch(), node_version: process.version,
  compiler: solc.version(), compiler_settings: settings, hardfork: common.hardfork(),
  execution_scope: 'signed local in-memory Prague transactions, no RPC; 20M tx/30M block gas',
  fixture: fixtureName, fixture_sha256: sha256(fixtureBytes),
  source_fixture: publicName, source_fixture_sha256: sha256(publicBytes),
  endpoint_fixture: endpointName, endpoint_fixture_sha256: sha256(endpointBytes),
  aggregate_proof_sha256: sha256(wire), contract_source: sourceName,
  contract_source_sha256: sha256(contractSource),
  runner_source_sha256: sha256(fs.readFileSync(fileURLToPath(import.meta.url))),
  c2_transcript_module_sha256: sha256(read('contracts/scripts/snarkpack_transcript_g5c2.mjs')),
  c2_trace_fixture_sha256: sha256(c2Bytes),
  noble_bls_source_sha256: sha256(read('contracts/node_modules/@noble/curves/abstract/bls.js')),
  noble_tower_source_sha256: sha256(read('contracts/node_modules/@noble/curves/abstract/tower.js')),
  noble_curve_source_sha256: sha256(read('contracts/node_modules/@noble/curves/bls12-381.js')),
  package_lock_sha256: sha256(read('contracts/package-lock.json')),
  cargo_lock_sha256: sha256(read('Cargo.lock')),
  official_manifest_sha256: sha256(read('configs/protocol_manifest.json')),
  prototype_manifest_sha256: sha256(read('configs/protocol_manifest.JUBJUB-PROTOTYPE.json')),
  runtime_bytecode_bytes: runtime.length, deployed_runtime_sha256: sha256(runtime),
  deployed_contract: address.toString(), transaction_gas_limit: 20_000_000, block_gas_limit: 30_000_000,
  full_steps: 64, prefix_steps: prefixSteps, prefix_g2_doublings: 2, prefix_signed_g2_additions: 1,
  prefix_fp12_squares: 1, prefix_line_products: 3,
  prefix_correctness_verified_in_evm: true, wrong_prefix_target_rejected_in_evm: true,
  full_miller_loop_verified_in_evm: fullVerified,
  host_full_miller_matches_noble_pairing_without_final_exponent: true,
  host_final_exponent_matches_native_tipp_target: true,
  host_lines_or_accumulator_supplied_to_contract: false,
  evm_final_exponentiation_implemented: false, pairing_to_gt_evm_verified: false,
  exact_snarkpack_evm_verified: false, complete_g5c3: false, snarkpack_verification_gas: null,
  deployment: deployment.measurement, prefix_valid: valid.measurement,
  prefix_wrong_target: wrong.measurement, full_attempt: full.measurement,
  setup_run: false, prove_run: false, private_witness_emitted: false,
  next_gate: fullVerified ? 'Implement final-exponentiation component on this saved Miller output; no scale runs'
    : 'Optimize software Miller memory/arithmetic before reattempting full loop at the same 20M budget; no final-exp or scale run yet',
  limitations: [
    'Two-step prefix covers only the first signed addition, not all 64 steps',
    'Miller output is not a GT subgroup element or final pairing result',
    'Canonical coordinate decoding is not complete on-curve/subgroup validation in Solidity',
    'Saved lines are reference only; contract recomputes all lines from point coordinates',
    'No full SnarkPack transcript, target folding, KZG or randomized batch equations are verified in EVM',
    'A single implementation exceeding this explicit budget does not establish EVM impossibility',
    'Component gas is not full pairing, aggregate verification or settlement gas',
  ],
};
const outputPath = path.resolve(root, process.argv[2] ?? 'experiments/g5_c3_raw/miller_loop_evm.json');
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(`Miller prefix PASS: ${valid.measurement.total_gas} gas; target-only negative rejected.`);
console.log(`Full loop: verified=${fullVerified}, status=${full.measurement.receipt_status}, gas=${full.measurement.total_gas}, exception=${full.measurement.exception}`);
console.log('Final exponentiation / exact GT target / SnarkPack EVM remain BLOCKED.');
console.log(`wrote ${path.relative(root, outputPath)}`);
