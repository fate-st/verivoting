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

// One exact field-tower multiplication only: NOT a SnarkPack verifier and NOT
// a pairing-to-GT implementation. Neither native acceptance nor a supplied
// product is used in place of arithmetic inside the deployed Solidity code.
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const fixtureName = 'fixtures/snarkpack_g5c3_fp12_product_v1.json';
const sourceName = 'contracts/src/Bls12381Fp12Product.sol';
const read = (name) => fs.readFileSync(path.join(root, name));
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
const metadata = (cmd, args) => execFileSync(cmd, args, { cwd: root, encoding: 'utf8' }).trim();
const fixtureBytes = read(fixtureName);
const fixture = JSON.parse(fixtureBytes);
const publicFixtureBytes = read(fixture.source_fixture);
const publicFixture = JSON.parse(publicFixtureBytes);
const source = read(sourceName).toString('utf8');
const packages = JSON.parse(read('contracts/package-lock.json'));
const { Fp, Fp6, Fp12, Fr } = bls12_381.fields;
assert.equal(fixture.schema, 'VV/G5-C3/FP12-PRODUCT/v1');
assert.equal(fixture.encoding, 'VV/G5-C3/Fp12-12xFp64BE/v1');
assert.equal(fixture.source_fixture, 'fixtures/snarkpack_g5c1_public_v1.json');
assert.equal(fixture.source_fixture_sha256, sha256(publicFixtureBytes));
assert.equal(fixture.left_operand, 'proof.com_ab.0');
assert.equal(fixture.right_operand, 'proof.com_c.0');
assert.equal(fixture.left_wire_offset, 0);
assert.equal(fixture.right_wire_offset, 576);
assert.equal(fixture.private_witness_emitted, false);
assert.equal(fixture.setup_run, false);
assert.equal(fixture.prove_run, false);
assert.equal(publicFixture.schema, 'VV/G5-C1/SNARKPACK-PUBLIC-FIXTURE/v1');
assert.equal(publicFixture.batch_size, 2);
const proofBytes = Buffer.from(publicFixture.aggregate_proof_hex, 'hex');
assert.equal(proofBytes.length, 5236);
assert.equal(sha256(proofBytes), fixture.aggregate_proof_sha256);
assert.equal(sha256(proofBytes), publicFixture.aggregate_proof_sha256);

const coefficients6 = (value) => [value.c0.c0, value.c0.c1, value.c1.c0,
  value.c1.c1, value.c2.c0, value.c2.c1];
const fpBE = (value) => {
  assert.ok(value >= 0n && value < Fp.ORDER);
  return Buffer.from(value.toString(16).padStart(128, '0'), 'hex');
};
const encode12 = (value) => `0x${Buffer.concat([
  ...coefficients6(value.c0), ...coefficients6(value.c1),
].map(fpBE)).toString('hex')}`;
const fpLE48 = (value) => Buffer.from(value.toString(16).padStart(96, '0'), 'hex').reverse();

// Independent @noble/curves decompression from the ORIGINAL compressed proof,
// rather than trusting the expanded operands provided by the Rust fixture.
function decodeGT(offset) {
  const wire = proofBytes.subarray(offset, offset + 288);
  assert.equal(wire.length, 288);
  const coefficients = Array.from({ length: 6 }, (_, i) => {
    const value = BigInt(`0x${Buffer.from(wire.subarray(i * 48, (i + 1) * 48)).reverse().toString('hex')}`);
    assert.ok(value < Fp.ORDER, 'non-canonical compressed GT coefficient');
    return value;
  });
  const compressed = Fp6.fromBigSix(coefficients);
  const value = Fp12.mul(Fp12.create({ c0: compressed, c1: Fp6.ONE }),
    Fp12.inv(Fp12.create({ c0: compressed, c1: Fp6.neg(Fp6.ONE) })));
  assert.ok(Fp12.eql(Fp12.pow(value, Fr.ORDER), Fp12.ONE), 'GT subgroup check failed');
  const roundtrip = Fp6.mul(Fp6.add(value.c0, Fp6.ONE), Fp6.inv(value.c1));
  assert.deepEqual(Buffer.concat(coefficients6(roundtrip).map(fpLE48)), wire);
  return value;
}
const left = decodeGT(0);
const right = decodeGT(576);
assert.equal(encode12(left), fixture.left_fp12_hex);
assert.equal(encode12(right), fixture.right_fp12_hex);
assert.equal(encode12(Fp12.mul(left, right)), fixture.product_fp12_hex,
  'independent JS product differs from native blstrs product');
for (const value of [fixture.left_fp12_hex, fixture.right_fp12_hex, fixture.product_fp12_hex]) {
  assert.match(value, /^0x[0-9a-f]{1536}$/);
  for (let offset = 0; offset < 768; offset += 64) {
    assert.ok(BigInt(`0x${value.slice(2 + offset * 2, 2 + (offset + 64) * 2)}`) < Fp.ORDER);
  }
}
const wrongBytes = Buffer.from(hexToBytes(fixture.product_fp12_hex));
const firstCoefficient = BigInt(`0x${wrongBytes.subarray(0, 64).toString('hex')}`);
fpBE((firstCoefficient + 1n) % Fp.ORDER).copy(wrongBytes, 0);
const wrongTarget = `0x${wrongBytes.toString('hex')}`;
assert.notEqual(wrongTarget, fixture.product_fp12_hex);
assert.equal(wrongTarget.slice(130), fixture.product_fp12_hex.slice(130));

const compilerSettings = {
  optimizer: { enabled: true, runs: 200 }, evmVersion: 'prague',
  outputSelection: { '*': { '*': ['abi', 'evm.bytecode.object', 'evm.deployedBytecode.object'] } },
};
assert.match(solc.version(), /^0\.8\.30\+/);
assert.equal(packages.packages['node_modules/@noble/curves'].version, '2.4.0');
const compilation = JSON.parse(solc.compile(JSON.stringify({
  language: 'Solidity', sources: { 'Bls12381Fp12Product.sol': { content: source } },
  settings: compilerSettings,
})));
for (const error of compilation.errors ?? []) {
  if (error.severity === 'error') throw new Error(error.formattedMessage);
  console.warn(error.formattedMessage);
}
const compiled = compilation.contracts['Bls12381Fp12Product.sol'].Bls12381Fp12Product;
const iface = new Interface(compiled.abi);
const factory = new ContractFactory(compiled.abi, `0x${compiled.evm.bytecode.object}`);
const common = new Common({ chain: Mainnet, hardfork: Hardfork.Prague });
const vm = await createVM({ common });
// Public deterministic LOCAL-ONLY key. This script never connects to RPC.
const senderKey = hexToBytes(`0x${'20'.repeat(32)}`);
const sender = createAddressFromPrivateKey(senderKey);
await vm.stateManager.putAccount(sender, createAccount({ nonce: 0n, balance: 10n ** 24n }));
const block = createBlock({ header: {
  number: 1n, gasLimit: 30_000_000n, baseFeePerGas: 1n, timestamp: 1_750_000_000n,
} }, { common, skipConsensusFormatValidation: true });
const messageStack = [];
let trace = [];
let nonce = 0n;
vm.evm.events.on('beforeMessage', (message) => {
  messageStack.push(message.to?.toString());
});
vm.evm.events.on('afterMessage', (result) => {
  const address = messageStack.pop();
  if (address === '0x0000000000000000000000000000000000000005') {
    trace.push({ address, execution_gas: Number(result.execResult.executionGasUsed),
      success: !result.execResult.exceptionError, return_bytes: result.execResult.returnValue.length });
  }
});
async function transact(data, to) {
  trace = [];
  const tx = createLegacyTx({ nonce: nonce++, gasLimit: 20_000_000n, gasPrice: 10n,
    value: 0n, data: hexToBytes(data), ...(to ? { to } : {}) }, { common }).sign(senderKey);
  const started = performance.now();
  const result = await runTx(vm, { tx, block });
  const zeroBytes = tx.data.filter((value) => value === 0).length;
  return { result, measurement: {
    transaction_hash: bytesToHex(tx.hash()), receipt_status: result.receipt.status,
    total_gas: Number(result.totalGasSpent), execution_gas: Number(result.execResult.executionGasUsed),
    gas_refund: Number(result.gasRefund), calldata_bytes: tx.data.length,
    calldata_zero_bytes: zeroBytes, calldata_nonzero_bytes: tx.data.length - zeroBytes,
    standard_calldata_gas: zeroBytes * 4 + (tx.data.length - zeroBytes) * 16,
    local_vm_wall_ms: performance.now() - started, modexp_calls: [...trace],
    modexp_call_count: trace.length,
    modexp_execution_gas: trace.reduce((sum, call) => sum + call.execution_gas, 0),
  } };
}
console.log('G5-C3: deploy exact Fp12 multiplication component in local Prague VM');
const deployment = await transact((await factory.getDeployTransaction()).data);
assert.equal(deployment.result.receipt.status, 1, 'deployment failed');
const address = deployment.result.createdAddress;
assert.ok(address);
const runtime = await vm.stateManager.getCode(address);
assert.ok(runtime.length > 0 && runtime.length <= 24576);
assert.equal(deployment.measurement.modexp_call_count, 0);
const args = [fixture.left_fp12_hex, fixture.right_fp12_hex, fixture.product_fp12_hex];
const validData = iface.encodeFunctionData('verifyProductOrRevert', args);
const wrongData = iface.encodeFunctionData('verifyProductOrRevert', [args[0], args[1], wrongTarget]);
const validDecoded = iface.decodeFunctionData('verifyProductOrRevert', validData);
const wrongDecoded = iface.decodeFunctionData('verifyProductOrRevert', wrongData);
assert.equal(validDecoded[0], wrongDecoded[0]);
assert.equal(validDecoded[1], wrongDecoded[1]);
assert.equal(wrongDecoded[2], wrongTarget);
// Assert actual ABI calldata differs only inside the first coefficient of the
// third dynamic bytes argument; offsets, lengths, and both operands unchanged.
const validCalldata = Buffer.from(hexToBytes(validData));
const wrongCalldata = Buffer.from(hexToBytes(wrongData));
assert.equal(validCalldata.length, wrongCalldata.length);
const targetStart = 4 + Number(BigInt(bytesToHex(validCalldata.subarray(68, 100)))) + 32;
const changedCalldataOffsets = [];
for (let i = 0; i < validCalldata.length; i++) {
  if (validCalldata[i] !== wrongCalldata[i]) {
    assert.ok(i >= targetStart && i < targetStart + 64, 'mutation escaped target coefficient');
    changedCalldataOffsets.push(i);
  }
}
assert.ok(changedCalldataOffsets.length > 0);
console.log('G5-C3: execute one valid product and the same operands with one wrong target');
const valid = await transact(validData, address);
assert.equal(valid.result.receipt.status, 1, 'valid product rejected');
const returnedProduct = iface.decodeFunctionResult('verifyProductOrRevert',
  bytesToHex(valid.result.execResult.returnValue))[0];
assert.equal(returnedProduct, fixture.product_fp12_hex, 'Solidity arithmetic differs from native product');
const wrong = await transact(wrongData, address);
assert.equal(wrong.result.receipt.status, 0, 'wrong target accepted');
assert.equal(wrong.result.execResult.exceptionError?.error, 'revert');
assert.equal(iface.parseError(bytesToHex(wrong.result.execResult.returnValue))?.args[0],
  'INVALID_FP12_PRODUCT');
for (const transaction of [valid, wrong]) {
  assert.equal(transaction.measurement.modexp_call_count, 108);
  assert.ok(transaction.measurement.modexp_calls.every((call) => call.success && call.return_bytes === 64));
}
const result = {
  gate: 'G5-C3', status: 'PASS_G5C3_FP12_PRODUCT_COMPONENT_ONLY',
  classification: 'one-proof-derived-fp12-product-real-solidity-signed-local-prague-transactions',
  recorded_at_utc: new Date().toISOString(), git_commit: metadata('git', ['rev-parse', 'HEAD']),
  git_dirty: metadata('git', ['status', '--porcelain']).length > 0,
  cpu: os.cpus()[0].model, os: os.platform(), arch: os.arch(), node_version: process.version,
  compiler: solc.version(), compiler_settings: compilerSettings,
  dependencies: Object.fromEntries(Object.entries(packages.packages)
    .filter(([name]) => name.startsWith('node_modules/@ethereumjs/') ||
      ['node_modules/ethers', 'node_modules/@noble/curves', 'node_modules/solc'].includes(name))
    .map(([name, value]) => [name.replace('node_modules/', ''), value.version])),
  hardfork: common.hardfork(), chain_id: Number(common.chainId()),
  transaction_gas_limit: 20_000_000, block_gas_limit: 30_000_000,
  execution_scope: 'local in-memory EVM with signed transactions and metered receipts; no RPC',
  fixture: fixtureName, fixture_sha256: sha256(fixtureBytes),
  source_fixture: fixture.source_fixture, source_fixture_sha256: sha256(publicFixtureBytes),
  aggregate_proof_sha256: sha256(proofBytes),
  contract_source: sourceName, contract_source_sha256: sha256(source),
  runner_source_sha256: sha256(fs.readFileSync(fileURLToPath(import.meta.url))),
  native_generator_source_sha256: sha256(read('crates/vv-friendly-dkg/src/bin/g5c3_fp12_fixture.rs')),
  package_lock_sha256: sha256(read('contracts/package-lock.json')),
  cargo_lock_sha256: sha256(read('Cargo.lock')),
  prototype_manifest_sha256: sha256(read('configs/protocol_manifest.JUBJUB-PROTOTYPE.json')),
  official_manifest_sha256: sha256(read('configs/protocol_manifest.json')),
  deployed_runtime_sha256: sha256(runtime), deployed_contract: address.toString(),
  runtime_bytecode_bytes: runtime.length, encoding: fixture.encoding, tower: fixture.tower,
  left_operand: fixture.left_operand, right_operand: fixture.right_operand,
  left_wire_offset: 0, right_wire_offset: 576, compressed_gt_bytes: 288, expanded_fp12_bytes: 768,
  independent_js_gt_decompression_and_product_match: true,
  native_js_solidity_product_match: true, wrong_canonical_target_rejected: true,
  wrong_target_mutation: 'first Fp coefficient incremented by one modulo p; both operands unchanged',
  changed_calldata_byte_offsets: changedCalldataOffsets,
  deployment: deployment.measurement, valid: valid.measurement, wrong_target: wrong.measurement,
  valid_returned_product_sha256: sha256(hexToBytes(returnedProduct)),
  pairing_to_gt_verified: false, exact_snarkpack_evm_verified: false, complete_g5c3: false,
  target_verifier_status: 'BLOCKED_PAIRING_TO_NONUNIT_GT_AND_FULL_VERIFIER_NOT_IMPLEMENTED',
  snarkpack_verification_gas: null, full_settlement_measured: false,
  setup_run: false, prove_run: false, production_setup: false, private_witness_emitted: false,
  limitations: [
    'Component transaction gas is NOT SnarkPack verification, aggregation or settlement gas',
    'This multiplication is not a complete GIPA/TIPP/MIPP/KZG verifier subexpression',
    'Solidity component performs Fp12 arithmetic, not GT subgroup checks or proof transcript binding',
    'Pairing-to-non-unit-GT comparison remains unimplemented; EIP-2537 returns a product-to-one boolean',
    'One saved B=2 proof-derived example and one target mutation, not exhaustive arithmetic validation',
    'Local Prague execution is not public-chain deployment, data availability or finality evidence',
    'No suite, official Manifest capability or security claim is promoted',
  ],
  next_gate: 'G5-C3-B exact pairing-to-nonunit-GT route; reuse saved proof, no scale/proving run',
};
const output = path.resolve(root, process.argv[2] ?? 'experiments/g5_c3_raw/fp12_product_evm.json');
fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, `${JSON.stringify(result, null, 2)}\n`);
console.log(`G5-C3 component PASS: valid=${valid.measurement.total_gas} gas; wrong-target=${wrong.measurement.total_gas} gas; MODEXP=108 per call`);
console.log('Complete G5-C3 and exact SnarkPack EVM verification remain BLOCKED; no aggregation gas reported.');
console.log(`wrote ${path.relative(root, output)}`);
