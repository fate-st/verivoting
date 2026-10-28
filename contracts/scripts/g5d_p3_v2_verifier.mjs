// G5-D: replay archived tally-v2 proofs, never setup/prove or call a network.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import solc from 'solc';
import { ContractFactory, Interface, keccak256 } from 'ethers';
import { Common, Hardfork, Mainnet } from '@ethereumjs/common';
import { createBlock } from '@ethereumjs/block';
import { createLegacyTx } from '@ethereumjs/tx';
import { createAccount, createAddressFromPrivateKey, hexToBytes, bytesToHex } from '@ethereumjs/util';
import { createVM, runTx } from '@ethereumjs/vm';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const read = name => fs.readFileSync(path.resolve(root, name));
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const load = name => JSON.parse(read(name));
const metadata = (cmd, args) => execFileSync(cmd, args, { cwd: root, encoding: 'utf8' }).trim();
const output = path.resolve(root, process.argv[2] ?? 'experiments/g5_d_raw/g5d_p3_v2_evm.json');
assert.equal(fs.existsSync(output), false, 'choose a new result path; historical results are immutable');

const fixtureName = 'fixtures/g4c_eip2537_fixture_v2.json';
const sourceName = 'experiments/g4_c_raw/partial_turnout_groth16.json';
const exportName = 'experiments/g5_d_raw/public_export.json';
const arkFixtureName = 'fixtures/jubjub_g4c_partial_turnout_v2.json';
const legacyFixtureName = 'fixtures/g4b_eip2537_fixture_v1.json';
const legacyResultName = 'experiments/g5_b_raw/p3_prague_evm.json';
const manifestName = 'configs/protocol_manifest.JUBJUB-P3-WEIGHT-V2.json';
const source = load(sourceName), fixture = load(fixtureName), exported = load(exportName);
const legacy = load(legacyFixtureName), legacyResult = load(legacyResultName);
const manifest = load(manifestName), arkFixture = load(arkFixtureName);
const relationId = 'VV/JUBJUB/P3/BOUNDED-TALLY/v2';
assert.equal(source.status, 'PASS_G4C_REAL_GROTH16_PARTIAL_TURNOUT_TALLY_V2');
assert.equal(source.p3_tally_relation_id, relationId);
assert.equal(fixture.p3_tally_relation_id, relationId);
assert.equal(fixture.relation, 'G4-C tally-v2 fixed-123 full/partial turnout P3');
assert.equal(fixture.curve, 'BLS12-381');
assert.equal(fixture.public_input_count, 60);
assert.equal(fixture.winner_public_input_index, 59);
assert.deepEqual(fixture.selected_trustees, [1, 2, 3]);
assert.equal(fixture.private_witness_emitted, false);
assert.equal(fixture.production_setup, false);
assert.equal(source.verifying_key_sha256, fixture.verifying_key_ark_sha256);
assert.equal(source.verifying_key_sha256, arkFixture.verifying_key_sha256);
assert.equal(source.manifest_sha256, sha256(read(manifestName)));
assert.equal(manifest.p3_tally_relation_id, relationId);
assert.equal(exported.setup_run, false);
assert.equal(exported.proof_generation_run, false);
assert.equal(exported.status, 'PASS_G5D_ARCHIVED_TALLY_V2_PUBLIC_EXPORT_NATIVE_REPLAY');
assert.equal(exported.native_positive_verification_count, 2);
assert.equal(exported.native_negative_verification_count, 1);
assert.equal(exported.output_fixture_sha256, sha256(read(fixtureName)));
assert.equal(exported.source_fixture_sha256, sha256(read(arkFixtureName)));
assert.equal(exported.source_run_sha256, sha256(read(sourceName)));
assert.equal(fixture.source_fixture_sha256, sha256(read(arkFixtureName)));
assert.equal(fixture.source_run_sha256, sha256(read(sourceName)));
assert.equal(fixture.manifest_sha256, sha256(read(manifestName)));
assert.equal(legacyResult.fixture, legacyFixtureName);
assert.equal(legacyResult.fixture_sha256, sha256(read(legacyFixtureName)));
assert.equal(legacyResult.valid_proof_verified, true);
assert.equal(legacy.relation, 'G4-B two-subset complete P3 disclosure');
assert.equal(legacy.public_input_count, 60);
assert.equal(legacyResult.verification_key_keccak256, keccak256(legacy.vk_eip2537_hex));
assert.notEqual(fixture.vk_eip2537_hex, legacy.vk_eip2537_hex);
for (const vk of [fixture.vk_eip2537_hex, legacy.vk_eip2537_hex]) {
  assert.equal(hexToBytes(vk).length, 896 + 61 * 128);
}
assert.equal(fixture.cases.length, 2);
const full = fixture.cases.find(x => x.case_name === 'full_turnout');
const partial = fixture.cases.find(x => x.case_name === 'partial_turnout');
assert.ok(full && partial);
const frModulus = 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001n;
for (const item of [full, partial]) {
  assert.equal(hexToBytes(item.proof_neg_a_eip2537_hex).length, 512);
  assert.equal(item.public_inputs_be_hex.length, 60);
  assert.ok(item.public_inputs_be_hex.every(x => /^0x[0-9a-f]{64}$/i.test(x) && BigInt(x) < frModulus));
  const original = arkFixture.cases.find(x => x.case_name === item.case_name);
  assert.equal(item.proof_ark_sha256, original.proof_sha256);
  const convertedInputs = original.public_inputs_ark_compressed_le_hex.map(x =>
    `0x${Buffer.from(x, 'hex').reverse().toString('hex')}`);
  assert.deepEqual(item.public_inputs_be_hex, convertedInputs);
  assert.equal(BigInt(item.public_inputs_be_hex[59]), 1n);
}
const changedIndices = partial.public_inputs_be_hex.flatMap((x, i) =>
  x !== partial.changed_winner_public_inputs_be_hex[i] ? [i] : []);
assert.equal(partial.changed_winner_public_inputs_be_hex.length, 60);
assert.deepEqual(changedIndices, [59]);
assert.equal(BigInt(partial.changed_winner_public_inputs_be_hex[59]), 2n);

const contractName = 'contracts/src/Bls12381Groth16Verifier.sol';
const contractSource = read(contractName).toString('utf8');
assert.equal(sha256(contractSource), legacyResult.contract_source_sha256, 'reuse the exact existing verifier source');
const settings = {
  optimizer: { enabled: true, runs: 200 }, evmVersion: 'prague',
  outputSelection: { '*': { '*': ['abi', 'evm.bytecode.object', 'evm.deployedBytecode.object'] } },
};
assert.deepEqual(settings, legacyResult.compiler_settings);
assert.equal(solc.version(), legacyResult.compiler);
console.log('G5-D [1/6] compile unchanged Groth16 verifier with the pinned Prague settings');
const compilation = JSON.parse(solc.compile(JSON.stringify({
  language: 'Solidity', sources: { 'Bls12381Groth16Verifier.sol': { content: contractSource } },
  settings,
})));
for (const error of compilation.errors ?? []) {
  if (error.severity === 'error') throw new Error(error.formattedMessage);
  console.warn(error.formattedMessage);
}
const compiled = compilation.contracts['Bls12381Groth16Verifier.sol'].Bls12381Groth16Verifier;
const iface = new Interface(compiled.abi);
const factory = new ContractFactory(compiled.abi, `0x${compiled.evm.bytecode.object}`);
const common = new Common({ chain: Mainnet, hardfork: Hardfork.Prague });
assert.equal(common.isActivatedEIP(2537), true);
const vm = await createVM({ common });
// Public deterministic sender used only in this in-memory VM. No RPC/network.
const senderKey = hexToBytes(`0x${'31'.repeat(32)}`);
const sender = createAddressFromPrivateKey(senderKey);
await vm.stateManager.putAccount(sender, createAccount({ nonce: 0n, balance: 10n ** 24n }));
const block = createBlock({ header: {
  number: 1n, gasLimit: 30_000_000n, baseFeePerGas: 1n, timestamp: 1_750_000_000n,
} }, { common, skipConsensusFormatValidation: true });
let nonce = 0n, trace = [];
const stack = [];
vm.evm.events.on('beforeMessage', message => {
  stack.push({ address: message.to?.toString(), input_bytes: message.data.length });
});
vm.evm.events.on('afterMessage', result => {
  const message = stack.pop();
  if (message?.address?.endsWith('000c') || message?.address?.endsWith('000f')) {
    trace.push({ ...message,
      execution_gas: Number(result.execResult.executionGasUsed),
      success: !result.execResult.exceptionError,
      return_bytes: result.execResult.returnValue.length,
      return_hex: bytesToHex(result.execResult.returnValue),
      exception: result.execResult.exceptionError?.error ?? null,
    });
  }
});
async function transact(data, to) {
  trace = [];
  const tx = createLegacyTx({ nonce: nonce++, gasLimit: 20_000_000n, gasPrice: 10n,
    value: 0n, data: hexToBytes(data), ...(to ? { to } : {}),
  }, { common }).sign(senderKey);
  const start = performance.now();
  const result = await runTx(vm, { tx, block });
  const elapsed = performance.now() - start;
  assert.equal(stack.length, 0);
  let reason = null;
  if (result.execResult.exceptionError?.error === 'revert') {
    reason = iface.parseError(bytesToHex(result.execResult.returnValue))?.args[0] ?? null;
  }
  const zero = tx.data.filter(x => x === 0).length;
  const total = Number(result.totalGasSpent), execution = Number(result.execResult.executionGasUsed);
  const intrinsic = Number(tx.getIntrinsicGas()), minimum = Number(tx.getMinimumGasLimit());
  const refund = Number(result.gasRefund);
  assert.equal(refund, 0, 'no refunds expected in this verifier-only experiment');
  assert.equal(total, Math.max(intrinsic + execution, minimum));
  return { result, measurement: {
    transaction_hash: bytesToHex(tx.hash()), to: to?.toString() ?? null,
    receipt_status: result.receipt.status, total_gas: total, execution_gas: execution,
    intrinsic_gas: intrinsic, gas_refund: refund,
    minimum_gas_limit_including_calldata_floor: minimum,
    calldata_floor_supplement_gas: total - (intrinsic + execution),
    transaction_gas_limit: 20_000_000,
    calldata_bytes: tx.data.length, calldata_sha256: sha256(tx.data),
    calldata_zero_bytes: zero, calldata_nonzero_bytes: tx.data.length - zero,
    standard_calldata_gas: zero * 4 + (tx.data.length - zero) * 16,
    local_vm_wall_ms: elapsed,
    exception: result.execResult.exceptionError?.error ?? null,
    revert_reason: reason, return_hex: bytesToHex(result.execResult.returnValue),
    precompile_calls: [...trace], accounting_checked: true,
  } };
}
async function deploy(vk) {
  const request = await factory.getDeployTransaction(vk, 60);
  const tx = await transact(request.data);
  assert.equal(tx.result.receipt.status, 1);
  const address = tx.result.createdAddress;
  assert.ok(address);
  const code = await vm.stateManager.getCode(address);
  assert.ok(code.length > 0 && code.length <= 24576);
  return { address, measurement: tx.measurement, code };
}
function checkedVerification(tx, accepted) {
  assert.equal(tx.result.receipt.status, accepted ? 1 : 0);
  const calls = tx.measurement.precompile_calls;
  assert.equal(calls.length, 2);
  assert.deepEqual(calls.map(x => x.address.slice(-4)), ['000c', '000f']);
  assert.deepEqual(calls.map(x => x.input_bytes), [61 * 160, 4 * 384]);
  assert.deepEqual(calls.map(x => x.return_bytes), [128, 32]);
  assert.ok(calls.every(x => x.success));
  assert.equal(BigInt(calls[1].return_hex), accepted ? 1n : 0n);
  assert.equal(tx.measurement.exception, accepted ? null : 'revert');
  assert.equal(tx.measurement.revert_reason, accepted ? null : 'INVALID_GROTH16');
}
const encode = (proof, inputs) => iface.encodeFunctionData('verifyOrRevert', [proof, inputs]);
console.log('G5-D [2/6] deploy one v2 verifier and one historical-v1-key verifier');
const current = await deploy(fixture.vk_eip2537_hex);
const old = await deploy(legacy.vk_eip2537_hex);
console.log('G5-D [3/6] verify the archived partial-turnout v2 proof');
const partialData = encode(partial.proof_neg_a_eip2537_hex, partial.public_inputs_be_hex);
const validPartial = await transact(partialData, current.address);
checkedVerification(validPartial, true);
console.log('G5-D [4/6] reject the same proof with only winner input 59 changed');
const wrongWinner = await transact(encode(partial.proof_neg_a_eip2537_hex,
  partial.changed_winner_public_inputs_be_hex), current.address);
checkedVerification(wrongWinner, false);
console.log('G5-D [5/6] reject the unchanged partial proof/inputs under the actual old v1 VK');
const wrongKey = await transact(partialData, old.address);
checkedVerification(wrongKey, false);
assert.equal(validPartial.measurement.calldata_sha256, wrongKey.measurement.calldata_sha256);
console.log('G5-D [6/6] verify the archived full-turnout v2 proof with the same v2 VK');
const validFull = await transact(encode(full.proof_neg_a_eip2537_hex, full.public_inputs_be_hex), current.address);
checkedVerification(validFull, true);
assert.equal(Number(nonce), 6);

const packages = load('contracts/package-lock.json');
const result = {
  gate: 'G5-D', status: 'PASS_G5D_LOCAL_PRAGUE_P3_TALLY_V2_FULL_AND_PARTIAL_TURNOUT',
  classification: 'archived-real-g4c-proofs-replayed-in-unchanged-solidity-verifier-on-local-prague-evm',
  recorded_at_utc: new Date().toISOString(), git_commit: metadata('git', ['rev-parse', 'HEAD']),
  git_dirty: metadata('git', ['status', '--porcelain']).length > 0,
  cpu: os.cpus()[0].model, os: os.platform(), arch: os.arch(), node_version: process.version,
  compiler: solc.version(), compiler_settings: settings,
  dependencies: Object.fromEntries(Object.entries(packages.packages)
    .filter(([name]) => name.startsWith('node_modules/@ethereumjs/') || name === 'node_modules/ethers')
    .map(([name, value]) => [name.replace('node_modules/', ''), value.version])),
  hardfork: common.hardfork(), eip2537_active: common.isActivatedEIP(2537), chain_id: Number(common.chainId()),
  execution_scope: 'local in-memory EVM with signed transactions and metered receipts; no RPC/public-chain deployment',
  curve: fixture.curve, relation: fixture.relation, p3_tally_relation_id: relationId,
  selected_trustees: [1, 2, 3], public_inputs: 60, ic_points: 61, changed_winner_input_indices: changedIndices,
  source_groth16_artifact: sourceName, source_groth16_sha256: sha256(read(sourceName)),
  source_ark_fixture: arkFixtureName, source_ark_fixture_sha256: sha256(read(arkFixtureName)),
  fixture: fixtureName, fixture_sha256: sha256(read(fixtureName)),
  export_report: exportName, export_report_sha256: sha256(read(exportName)),
  legacy_fixture: legacyFixtureName, legacy_fixture_sha256: sha256(read(legacyFixtureName)),
  legacy_result: legacyResultName, legacy_result_sha256: sha256(read(legacyResultName)),
  experiment_manifest: manifestName, experiment_manifest_sha256: sha256(read(manifestName)),
  official_manifest_sha256: sha256(read('configs/protocol_manifest.json')),
  prototype_manifest_sha256: sha256(read('configs/protocol_manifest.JUBJUB-PROTOTYPE.json')),
  contract_source: contractName, contract_source_sha256: sha256(contractSource),
  contract_source_unchanged_from_g5b: true,
  runner_source: 'contracts/scripts/g5d_p3_v2_verifier.mjs',
  runner_source_sha256: sha256(read('contracts/scripts/g5d_p3_v2_verifier.mjs')),
  package_lock_sha256: sha256(read('contracts/package-lock.json')),
  verification_key_ark_sha256: fixture.verifying_key_ark_sha256,
  verification_key_eip2537_keccak256: keccak256(fixture.vk_eip2537_hex),
  legacy_verification_key_eip2537_keccak256: keccak256(legacy.vk_eip2537_hex),
  verification_key_eip2537_bytes: hexToBytes(fixture.vk_eip2537_hex).length,
  verification_key_storage: 'separate constructor-pinned VK for each deployment; no replacement method',
  proof_compressed_bytes: 192, proof_eip2537_bytes: 512,
  logical_proof_and_public_input_bytes: 512 + 32 * 60,
  deployments: {
    tally_v2: { ...current.measurement, deployed_contract: current.address.toString(),
      runtime_bytecode_bytes: current.code.length, deployed_runtime_sha256: sha256(current.code) },
    legacy_tally_v1: { ...old.measurement, deployed_contract: old.address.toString(),
      runtime_bytecode_bytes: old.code.length, deployed_runtime_sha256: sha256(old.code) },
  },
  valid_partial: validPartial.measurement, wrong_winner: wrongWinner.measurement,
  wrong_legacy_vk: wrongKey.measurement, valid_full: validFull.measurement,
  valid_partial_proof_verified: true, valid_full_proof_verified: true,
  changed_public_winner_rejected: true, legacy_v1_vk_rejected_v2_proof: true,
  legacy_vk_negative_same_proof_and_inputs: true,
  deployment_count: 2, verification_transactions: 4, evm_transactions: 6,
  setup_run: false, proof_generation_run: false, heavy_groth16_rerun: false,
  p3_v2_evm_verified: true, all_ten_subsets_tested: false,
  registry_and_frozen_state_binding_checked: false, full_settlement_measured: false,
  snarkpack_evm_verified: false, full_batch_da_measured: false,
  software_miller_route_reopened: false, production_setup: false, private_witness_emitted: false,
  next_gate: 'B=4 distinct-registered-voter native SnarkPack aggregation; no SnarkPack EVM prerequisite',
  limitations: [
    'fixed Trustee subset 123 only; v2 other-subset coverage is not established',
    'verification of a P3 proof is not comparison against accepted registry/frozen target, commitment authentication or write-once winner settlement',
    'local signed EVM transactions are not a public-chain deployment or finality measurement',
    'verifier calldata is not full batch DA; no complete Proposed gas or SnarkPack gas is reported',
    'the old-v1-key negative checks these actual archived keys/proofs, not universal VK noninterchangeability',
    'new and legacy deployments may have distinct runtime hashes because the VK digest is immutable',
    'public deterministic research setup/sender seeds are nonproduction',
  ],
};
fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, JSON.stringify(result, null, 2) + '\n', { flag: 'wx' });
console.log(`G5-D PASS: partial=${result.valid_partial.total_gas} gas; full=${result.valid_full.total_gas} gas; winner/VK negatives rejected; zero setup/prove`);
console.log(`wrote ${path.relative(root, output)}`);
