// G7: exact archived G6 proof and G4-D VK. Local signed Prague transactions only.
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
const read = p => fs.readFileSync(path.resolve(root, p));
const sha = b => createHash('sha256').update(b).digest('hex');
const load = p => JSON.parse(read(p));
const metadata = (c, a) => execFileSync(c, a, { cwd: root, encoding: 'utf8' }).trim();
const outputDir = process.argv[2] ?? 'experiments/g7_raw';
const resultName = `${outputDir}/g7_p3_prague_evm.json`;
const artifactName = `${outputDir}/contract_artifact.json`;
for (const p of [resultName, artifactName]) assert.equal(fs.existsSync(path.resolve(root, p)), false, `new path required: ${p}`);
const fixtureName = 'fixtures/g6_p3_eip2537_g7_v1.json';
const exportName = 'experiments/g7_raw/public_export.json';
const manifestName = 'configs/protocol_manifest.JUBJUB-G7-P3-EVM-W32.json';
const contractName = 'contracts/src/Bls12381Groth16Verifier.sol';
const config = load(manifestName), fixture = load(fixtureName), exported = load(exportName);
for (const p of config.source_pins) assert.equal(sha(read(p.path)), p.sha256, p.path);
for (const p of exported.source_hashes) assert.equal(sha(read(p.path)), p.sha256, p.path);
assert.equal(exported.status, 'PASS_G6_ARCHIVE_NATIVE_VERIFY_EIP2537_EXPORT');
assert.equal(exported.setup_count, 0); assert.equal(exported.proof_generation_count, 0);
assert.deepEqual(exported.native_checks.map(x => x.accepted), [true, false, false, false, false]);
assert.equal(exported.output_fixture_sha256, sha(read(fixtureName)));
assert.equal(fixture.schema, 'VV/G7/G6-P3-EIP2537/v1');
assert.equal(fixture.relation_id, config.relation_id);
assert.equal(fixture.input_layout, config.input_layout);
assert.equal(fixture.verifying_key_ark_sha256, config.verifying_key_sha256);
assert.equal(fixture.proof_ark_sha256, config.proof_sha256);
assert.equal(fixture.public_input_count, 60);
assert.equal(fixture.setup_run, false); assert.equal(fixture.proof_generation_run, false);
assert.equal(fixture.private_witness_emitted, false);
assert.equal(hexToBytes(fixture.proof_neg_a_eip2537_hex).length, 512);
for (const vk of [fixture.vk_eip2537_hex, fixture.wrong_vk_eip2537_hex]) assert.equal(hexToBytes(vk).length, 8704);
assert.notEqual(fixture.vk_eip2537_hex, fixture.wrong_vk_eip2537_hex);
assert.equal(fixture.wrong_vk_eip2537_hex, load('fixtures/g4c_eip2537_fixture_v2.json').vk_eip2537_hex);
const changed = values => values.flatMap((x, i) => x === fixture.public_inputs_be_hex[i] ? [] : [i]);
assert.deepEqual(changed(fixture.wrong_selector_inputs_be_hex), [51]);
assert.deepEqual(changed(fixture.wrong_winner_inputs_be_hex), [59]);
const changedC1 = changed(fixture.changed_ciphertext_inputs_be_hex);
assert.ok(changedC1.length > 0 && changedC1.every(i => i === 52 || i === 53));
for (const name of ['public_inputs_be_hex', 'wrong_selector_inputs_be_hex', 'wrong_winner_inputs_be_hex', 'changed_ciphertext_inputs_be_hex']) {
  assert.equal(fixture[name].length, 60);
  assert.ok(fixture[name].every(x => /^0x[0-9a-f]{64}$/.test(x)));
}
const historical = load('experiments/g5_d_raw/g5d_p3_v2_evm.json');
const contractSource = read(contractName).toString('utf8');
assert.equal(sha(contractSource), historical.contract_source_sha256);
const settings = {
  optimizer: { enabled: true, runs: 200 }, evmVersion: 'prague',
  outputSelection: { '*': { '*': ['abi', 'evm.bytecode.object', 'evm.deployedBytecode.object'] } },
};
assert.deepEqual(settings, historical.compiler_settings); assert.equal(solc.version(), historical.compiler);
console.log('G7 [1/7] compile unchanged verifier / pinned solc / Prague');
const compilation = JSON.parse(solc.compile(JSON.stringify({ language: 'Solidity',
  sources: { 'Bls12381Groth16Verifier.sol': { content: contractSource } }, settings })));
for (const e of compilation.errors ?? []) {
  if (e.severity === 'error') throw new Error(e.formattedMessage);
  console.warn(e.formattedMessage);
}
const compiled = compilation.contracts['Bls12381Groth16Verifier.sol'].Bls12381Groth16Verifier;
const iface = new Interface(compiled.abi);
const factory = new ContractFactory(compiled.abi, `0x${compiled.evm.bytecode.object}`);
const common = new Common({ chain: Mainnet, hardfork: Hardfork.Prague });
assert.equal(common.isActivatedEIP(2537), true); assert.equal(common.isActivatedEIP(7623), true);
const vm = await createVM({ common });
// Public, local-only sender key. Never used on an RPC/public network.
const senderKey = hexToBytes(`0x${'37'.repeat(32)}`);
const sender = createAddressFromPrivateKey(senderKey);
await vm.stateManager.putAccount(sender, createAccount({ nonce: 0n, balance: 10n ** 24n }));
const block = createBlock({ header: { number: 1n, gasLimit: BigInt(config.block_gas_limit),
  baseFeePerGas: 1n, timestamp: 1_750_000_000n } }, { common, skipConsensusFormatValidation: true });
let nonce = 0n, trace = [];
const stack = [];
vm.evm.events.on('beforeMessage', message => stack.push({ address: message.to?.toString(), input_bytes: message.data.length }));
vm.evm.events.on('afterMessage', result => {
  const m = stack.pop();
  if (m?.address?.endsWith('000c') || m?.address?.endsWith('000f')) trace.push({ ...m,
    execution_gas: Number(result.execResult.executionGasUsed), success: !result.execResult.exceptionError,
    return_bytes: result.execResult.returnValue.length, return_hex: bytesToHex(result.execResult.returnValue),
    exception: result.execResult.exceptionError?.error ?? null });
});
async function transact(data, to) {
  trace = [];
  const tx = createLegacyTx({ nonce: nonce++, gasLimit: BigInt(config.transaction_gas_limit), gasPrice: 10n,
    value: 0n, data: hexToBytes(data), ...(to ? { to } : {}) }, { common }).sign(senderKey);
  const start = performance.now();
  const result = await runTx(vm, { tx, block });
  const elapsed = performance.now() - start;
  assert.equal(stack.length, 0);
  const zero = tx.data.filter(x => x === 0).length;
  const total = Number(result.totalGasSpent), execution = Number(result.execResult.executionGasUsed);
  const intrinsic = Number(tx.getIntrinsicGas()), minimum = Number(tx.getMinimumGasLimit());
  const refund = Number(result.gasRefund);
  assert.equal(refund, 0); assert.equal(total, Math.max(intrinsic + execution, minimum));
  let reason = null;
  if (result.execResult.exceptionError?.error === 'revert') reason = iface.parseError(bytesToHex(result.execResult.returnValue))?.args[0] ?? null;
  return { result, measurement: {
    transaction_hash: bytesToHex(tx.hash()), to: to?.toString() ?? null, nonce: Number(tx.nonce),
    receipt_status: result.receipt.status, total_gas: total, execution_gas: execution,
    intrinsic_gas: intrinsic, gas_refund: refund, minimum_gas_limit_including_calldata_floor: minimum,
    calldata_floor_supplement_gas: total - intrinsic - execution, transaction_gas_limit: config.transaction_gas_limit,
    calldata_bytes: tx.data.length, calldata_hex: bytesToHex(tx.data), calldata_sha256: sha(tx.data),
    calldata_zero_bytes: zero, calldata_nonzero_bytes: tx.data.length - zero,
    standard_calldata_gas: zero * 4 + (tx.data.length - zero) * 16, local_vm_wall_ms: elapsed,
    exception: result.execResult.exceptionError?.error ?? null, revert_reason: reason,
    return_hex: bytesToHex(result.execResult.returnValue), precompile_calls: [...trace], accounting_checked: true,
  } };
}
async function deploy(vk) {
  const request = await factory.getDeployTransaction(vk, 60);
  const tx = await transact(request.data);
  assert.equal(tx.result.receipt.status, 1);
  const address = tx.result.createdAddress; assert.ok(address);
  const code = await vm.stateManager.getCode(address);
  assert.ok(code.length > 0 && code.length <= 24576);
  return { address, code, measurement: { ...tx.measurement, deployed_contract: address.toString(),
    runtime_bytecode_bytes: code.length, runtime_bytecode_sha256: sha(code), runtime_bytecode_keccak256: keccak256(code) } };
}
function checked(tx, accepted) {
  assert.equal(tx.result.receipt.status, accepted ? 1 : 0);
  const calls = tx.measurement.precompile_calls;
  assert.deepEqual(calls.map(x => x.address.slice(-4)), ['000c', '000f']);
  assert.deepEqual(calls.map(x => x.input_bytes), [9760, 1536]);
  assert.deepEqual(calls.map(x => x.return_bytes), [128, 32]);
  assert.ok(calls.every(x => x.success));
  assert.equal(BigInt(calls[1].return_hex), accepted ? 1n : 0n);
  assert.equal(tx.measurement.exception, accepted ? null : 'revert');
  assert.equal(tx.measurement.revert_reason, accepted ? null : 'INVALID_GROTH16');
}
console.log('G7 [2/7] deploy G4-D VK and actual historical G4-C fixed-123 VK');
const current = await deploy(fixture.vk_eip2537_hex), old = await deploy(fixture.wrong_vk_eip2537_hex);
const encode = inputs => iface.encodeFunctionData('verifyOrRevert', [fixture.proof_neg_a_eip2537_hex, inputs]);
const validData = encode(fixture.public_inputs_be_hex);
console.log('G7 [3/7] exact G6 proof + exact 60 inputs: accept');
const valid = await transact(validData, current.address); checked(valid, true);
const negatives = {};
let step = 4;
for (const [name, inputs, target] of [
  ['wrong_selector', fixture.wrong_selector_inputs_be_hex, current.address],
  ['wrong_winner', fixture.wrong_winner_inputs_be_hex, current.address],
  ['changed_ciphertext', fixture.changed_ciphertext_inputs_be_hex, current.address],
  ['wrong_vk', fixture.public_inputs_be_hex, old.address],
]) {
  console.log(`G7 [${step++}/7] ${name}: reject`);
  const tx = await transact(encode(inputs), target); checked(tx, false); negatives[name] = tx.measurement;
}
assert.equal(nonce, 7n);
assert.equal(valid.measurement.calldata_hex, negatives.wrong_vk.calldata_hex);
assert.equal(valid.measurement.calldata_bytes, 2564);
// Keep both actual deployed runtimes: constructor immutables differ between VKs.
const artifact = {
  compiler: solc.version(), compiler_settings: settings, contract_source: contractName,
  contract_source_sha256: sha(contractSource), abi: compiled.abi,
  creation_bytecode_hex: `0x${compiled.evm.bytecode.object}`,
  creation_bytecode_bytes: compiled.evm.bytecode.object.length / 2,
  creation_bytecode_sha256: sha(hexToBytes(`0x${compiled.evm.bytecode.object}`)),
  compiled_runtime_template_hex: `0x${compiled.evm.deployedBytecode.object}`,
  deployed_g4d_runtime_hex: bytesToHex(current.code), deployed_old_g4c_runtime_hex: bytesToHex(old.code),
};
const packages = load('contracts/package-lock.json');
const sourcePaths = [manifestName, contractName, 'contracts/scripts/g7_p3_verifier.mjs', 'contracts/package.json',
  'contracts/package-lock.json', 'crates/vv-friendly-dkg/src/bin/g7_p3_eip2537_export.rs',
  'crates/vv-friendly-crypto/src/evm_export.rs', 'crates/vv-friendly-crypto/src/p3ten_gadget.rs', 'Cargo.lock'];
const result = {
  gate: 'G7', status: 'PASS_SAME_G6_PROOF_G4D_VK_PRAGUE_1_POSITIVE_4_NEGATIVES',
  classification: 'proof-verifier-component-only-not-winner-only-contract-settlement',
  recorded_at_utc: new Date().toISOString(), git_commit: metadata('git', ['rev-parse', 'HEAD']),
  git_dirty: metadata('git', ['status', '--porcelain']).length > 0,
  cpu: os.cpus()[0].model, os: os.platform(), arch: os.arch(), node_version: process.version,
  compiler: solc.version(), compiler_settings: settings,
  dependencies: Object.fromEntries(Object.entries(packages.packages)
    .filter(([n]) => n.startsWith('node_modules/@ethereumjs/') || ['node_modules/ethers', 'node_modules/solc', 'node_modules/@noble/curves'].includes(n))
    .map(([n, v]) => [n.replace('node_modules/', ''), v.version])),
  hardfork: common.hardfork(), eip2537_active: true, eip7623_active: true, chain_id: Number(common.chainId()),
  execution_scope: 'signed legacy transactions, local in-memory Prague VM; no RPC or public-chain deployment',
  relation_id: fixture.relation_id, public_input_count: 60, ic_points: 61, selected_subset: [1, 2, 3],
  proof_ark_sha256: fixture.proof_ark_sha256, verifying_key_ark_sha256: fixture.verifying_key_ark_sha256,
  wrong_vk_ark_sha256: fixture.wrong_vk_ark_sha256,
  vk_eip2537_keccak256: keccak256(fixture.vk_eip2537_hex), vk_eip2537_sha256: sha(hexToBytes(fixture.vk_eip2537_hex)),
  vk_eip2537_bytes: 8704, proof_eip2537_bytes: 512, proof_compressed_bytes: 192,
  public_input_bytes: 1920, logical_proof_and_inputs_bytes: 2432, abi_calldata_bytes: valid.measurement.calldata_bytes,
  mutated_input_indices: { wrong_selector: [51], wrong_winner: [59], changed_ciphertext: changedC1 },
  ciphertext_mutation: fixture.ciphertext_mutation,
  source_fixture: fixture.source_fixture, source_fixture_sha256: fixture.source_fixture_sha256,
  fixture: fixtureName, fixture_sha256: sha(read(fixtureName)),
  export_report: exportName, export_report_sha256: sha(read(exportName)),
  source_pins: config.source_pins, source_hashes: sourcePaths.map(p => ({ path: p, sha256: sha(read(p)) })),
  contract_source_unchanged_from_g5d: true, contract_source_sha256: sha(contractSource),
  deployments: { g4d: current.measurement, old_fixed123: old.measurement },
  valid: valid.measurement, negatives, transaction_count: Number(nonce),
  setup_count: 0, proof_generation_count: 0, archived_g6_proofs_reused: 1,
  freeze_storage_binding_checked: false, trustee_signatures_checked_on_evm: false,
  winner_storage_or_one_time_release_checked: false, snarkpack_evm_verified: false,
  complete_settlement_measured: false, full_system_gas: null, production_setup: false,
  private_witness_emitted: false, software_miller_route_reopened: false,
  peak_node_rss_mib: process.resourceUsage().maxRSS / 1024,
  limitations: [
    'single functional call per case, not independent performance statistics',
    'G7 rejects a changed ciphertext statement but does not compare it to authenticated chain freeze storage',
    'no registry/batch/nullifier state machine, trustee signature checks or one-time winner write in this verifier',
    'same-process trustee delivery and native freeze remain G6 limitations; no distributed production deployment',
    'deterministic research setup; exact SnarkPack EVM and full settlement remain unfinished',
    'Node wall time and process RSS include compilation/VM activity, not chain execution latency',
  ],
};
function writeNew(p, value) {
  fs.mkdirSync(path.dirname(path.resolve(root, p)), { recursive: true });
  fs.writeFileSync(path.resolve(root, p), `${JSON.stringify(value, null, 2)}\n`, { flag: 'wx' });
}
writeNew(artifactName, artifact);
result.contract_artifact = artifactName; result.contract_artifact_sha256 = sha(read(artifactName));
writeNew(resultName, result);
console.log(`G7 PASS: deploy=${current.measurement.total_gas}; verify=${valid.measurement.total_gas}; calldata=${valid.measurement.calldata_bytes} B; four meaningful pairing negatives; setup/prove=0`);
