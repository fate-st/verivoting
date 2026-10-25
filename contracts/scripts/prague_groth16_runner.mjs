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
import {
  createAccount, createAddressFromPrivateKey, hexToBytes, bytesToHex,
} from '@ethereumjs/util';
import { createVM, runTx } from '@ethereumjs/vm';

export async function runPragueGroth16Gate(config) {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  const fixturePath = path.join(root, config.fixture);
  const fixtureBytes = fs.readFileSync(fixturePath);
  const fixture = JSON.parse(fixtureBytes);
  const sourceRunPath = config.sourceRun;
  const sourceRunBytes = fs.readFileSync(path.join(root, sourceRunPath));
  const sourceRun = JSON.parse(sourceRunBytes);
  const sourcePath = path.join(root, 'contracts/src/Bls12381Groth16Verifier.sol');
  const source = fs.readFileSync(sourcePath, 'utf8');
  const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
  const metadata = (cmd, args) => execFileSync(cmd, args, { cwd: root, encoding: 'utf8' }).trim();
  const packages = JSON.parse(fs.readFileSync(path.join(root, 'contracts/package-lock.json')));
  const compilerSettings = {
    optimizer: { enabled: true, runs: 200 },
    evmVersion: 'prague',
    outputSelection: { '*': { '*': ['abi', 'evm.bytecode.object', 'evm.deployedBytecode.object'] } },
  };
  const compilation = JSON.parse(solc.compile(JSON.stringify({
    language: 'Solidity', sources: { 'Bls12381Groth16Verifier.sol': { content: source } },
    settings: compilerSettings,
  })));
  for (const error of compilation.errors ?? []) {
    if (error.severity === 'error') throw new Error(error.formattedMessage);
    console.warn(error.formattedMessage);
  }
  const compiled = compilation.contracts['Bls12381Groth16Verifier.sol'].Bls12381Groth16Verifier;
  const iface = new Interface(compiled.abi);
  const factory = new ContractFactory(compiled.abi, `0x${compiled.evm.bytecode.object}`);
  assert.equal(fixture.public_input_count, config.publicInputs);
  assert.equal(fixture.relation, config.relation);
  assert.equal(fixture.private_witness_emitted, false);
  config.validateSource(sourceRun, fixture);
  assert.equal(hexToBytes(fixture.proof_neg_a_eip2537_hex).length, 512);
  assert.equal(hexToBytes(fixture.vk_eip2537_hex).length, 896 + (config.publicInputs + 1) * 128);
  const changedInputs = fixture[config.changedInputsKey];
  assert.equal(changedInputs.length, fixture.public_input_count);
  const changedIndices = fixture.public_inputs_be_hex.flatMap((value, index) =>
    value !== changedInputs[index] ? [index] : []);
  assert.deepEqual(changedIndices, [config.changedIndex]);

  const common = new Common({ chain: Mainnet, hardfork: Hardfork.Prague });
  assert.equal(common.isActivatedEIP(2537), true);
  const vm = await createVM({ common });
  // Public deterministic local-only sender, never connected to an RPC network.
  const senderKey = hexToBytes(`0x${'20'.repeat(32)}`);
  const sender = createAddressFromPrivateKey(senderKey);
  await vm.stateManager.putAccount(sender, createAccount({ nonce: 0n, balance: 10n ** 24n }));
  const block = createBlock({ header: {
    number: 1n, gasLimit: 30_000_000n, baseFeePerGas: 1n, timestamp: 1_750_000_000n,
  } }, { common, skipConsensusFormatValidation: true });
  let nonce = 0n;
  let activePrecompileTrace = [];
  const messageStack = [];
  vm.evm.events.on('beforeMessage', (message) => {
    messageStack.push(message.to?.toString());
  });
  vm.evm.events.on('afterMessage', (result) => {
    const address = messageStack.pop();
    if (address?.endsWith('000c') || address?.endsWith('000f')) {
      activePrecompileTrace.push({
        address,
        execution_gas: Number(result.execResult.executionGasUsed),
        success: !result.execResult.exceptionError,
        return_bytes: result.execResult.returnValue.length,
      });
    }
  });

  async function transact(data, to) {
    activePrecompileTrace = [];
    const tx = createLegacyTx({
      nonce: nonce++, gasLimit: 20_000_000n, gasPrice: 10n, value: 0n,
      data: hexToBytes(data), ...(to ? { to } : {}),
    }, { common }).sign(senderKey);
    const started = performance.now();
    const result = await runTx(vm, { tx, block });
    const elapsedMs = performance.now() - started;
    const zeroBytes = tx.data.filter((value) => value === 0).length;
    return { result, measurement: {
      transaction_hash: bytesToHex(tx.hash()),
      receipt_status: result.receipt.status,
      total_gas: Number(result.totalGasSpent),
      execution_gas: Number(result.execResult.executionGasUsed),
      gas_refund: Number(result.gasRefund),
      calldata_bytes: tx.data.length,
      calldata_zero_bytes: zeroBytes,
      calldata_nonzero_bytes: tx.data.length - zeroBytes,
      standard_calldata_gas: zeroBytes * 4 + (tx.data.length - zeroBytes) * 16,
      local_vm_wall_ms: elapsedMs,
      precompile_calls: [...activePrecompileTrace],
    } };
  }

  console.log(`${config.gate}: compiling Solidity 0.8.30 for Prague and deploying fixed VK`);
  const deployRequest = await factory.getDeployTransaction(
    fixture.vk_eip2537_hex, fixture.public_input_count);
  const deployment = await transact(deployRequest.data);
  assert.equal(deployment.result.receipt.status, 1, 'verifier deployment failed');
  const address = deployment.result.createdAddress;
  assert.ok(address, 'deployment did not return an address');
  const deployedCode = await vm.stateManager.getCode(address);
  assert.ok(deployedCode.length > 0 && deployedCode.length <= 24576);

  console.log(`${config.gate}: executing valid ${config.relation} through real EIP-2537 precompiles`);
  const validData = iface.encodeFunctionData('verifyOrRevert', [
    fixture.proof_neg_a_eip2537_hex, fixture.public_inputs_be_hex,
  ]);
  const valid = await transact(validData, address);
  assert.equal(valid.result.receipt.status, 1, 'valid native Groth16 proof failed in EVM');
  assert.equal(valid.measurement.precompile_calls.length, 2);
  assert.deepEqual(valid.measurement.precompile_calls.map((call) => call.return_bytes), [128, 32]);
  assert.ok(valid.measurement.precompile_calls.every((call) => call.success));

  console.log(`${config.gate}: executing the same proof with only public input ${config.changedIndex} changed`);
  const wrongData = iface.encodeFunctionData('verifyOrRevert', [
    fixture.proof_neg_a_eip2537_hex, changedInputs,
  ]);
  const wrong = await transact(wrongData, address);
  assert.equal(wrong.result.receipt.status, 0, 'changed public input was accepted');
  assert.equal(wrong.measurement.precompile_calls.length, 2);
  assert.equal(wrong.result.execResult.exceptionError?.error, 'revert');
  const decodedError = iface.parseError(bytesToHex(wrong.result.execResult.returnValue));
  assert.equal(decodedError?.args[0], 'INVALID_GROTH16');

  const result = {
    gate: config.gate,
    status: config.status,
    classification: 'real-solidity-bytecode-and-eip2537-execution-in-local-prague-ethereumjs-vm',
    recorded_at_utc: new Date().toISOString(),
    git_commit: metadata('git', ['rev-parse', 'HEAD']),
    git_dirty: metadata('git', ['status', '--porcelain']).length > 0,
    cpu: os.cpus()[0].model,
    os: os.platform(), arch: os.arch(), node_version: process.version,
    compiler: solc.version(), compiler_settings: compilerSettings,
    dependencies: Object.fromEntries(Object.entries(packages.packages)
      .filter(([name]) => name.startsWith('node_modules/@ethereumjs/') || name === 'node_modules/ethers')
      .map(([name, value]) => [name.replace('node_modules/', ''), value.version])),
    hardfork: common.hardfork(), chain_id: Number(common.chainId()),
    execution_scope: 'local in-memory EVM with signed transactions and metered receipts',
    eip2537_active: common.isActivatedEIP(2537),
    curve: fixture.curve, relation: fixture.relation,
    fixture: path.relative(root, fixturePath), fixture_sha256: sha256(fixtureBytes),
    source_groth16_artifact: sourceRunPath, source_groth16_sha256: sha256(sourceRunBytes),
    prototype_manifest_sha256: sha256(fs.readFileSync(path.join(root,
      'configs/protocol_manifest.JUBJUB-PROTOTYPE.json'))),
    contract_source_sha256: sha256(source),
    package_lock_sha256: sha256(fs.readFileSync(path.join(root, 'contracts/package-lock.json'))),
    deployed_runtime_sha256: sha256(deployedCode),
    deployed_contract: address.toString(), runtime_bytecode_bytes: deployedCode.length,
    verification_key_keccak256: keccak256(fixture.vk_eip2537_hex),
    verification_key_eip2537_bytes: hexToBytes(fixture.vk_eip2537_hex).length,
    verification_key_storage: 'constructor-pinned Solidity bytes storage; immutable API, no replacement method',
    public_inputs: fixture.public_input_count, ic_points: fixture.public_input_count + 1,
    proof_compressed_bytes: sourceRun.proof_bytes, proof_eip2537_bytes: 512,
    logical_proof_and_public_input_bytes: 512 + 32 * fixture.public_input_count,
    changed_public_input_indices: changedIndices,
    native_setup_ms: sourceRun.setup_ms, native_prove_ms: sourceRun.prove_ms,
    native_verify_ms: sourceRun.verify_ms,
    deployment: deployment.measurement,
    valid: valid.measurement, [config.negativeResultKey]: wrong.measurement,
    valid_proof_verified: true, [config.negativeVerifiedField]: true,
    verifier_transaction_calldata_da_bytes: valid.measurement.calldata_bytes,
    full_batch_da_measured: false, full_settlement_measured: false,
    snarkpack_evm_verified: false, ...config.resultFields(fixture, sourceRun),
    production_setup: false, private_witness_emitted: false,
    limitations: [
      'local Prague execution, not a deployed public-chain transaction or finality measurement',
      ...config.limitations,
      'VK uses straightforward storage reads; gas is for this implementation and compiler',
      'verifier calldata excludes the ordered public ballot matrix U, nullifier/context checks and state writes',
      'complete target-chain capability remains BLOCKED in the Manifest',
    ],
  };
  fs.mkdirSync(path.join(root, 'contracts/build'), { recursive: true });
  fs.writeFileSync(path.join(root, 'contracts/build/Bls12381Groth16Verifier.json'),
    JSON.stringify(compiled, null, 2) + '\n');
  const output = path.resolve(root, process.argv[2] ?? config.output);
  fs.mkdirSync(path.dirname(output), { recursive: true });
  fs.writeFileSync(output, JSON.stringify(result, null, 2) + '\n');
  console.log(`${config.gate} PASS: valid=${valid.measurement.total_gas} gas; changed-input=${wrong.measurement.total_gas} gas; calldata=${valid.measurement.calldata_bytes} B; runtime=${deployedCode.length} B`);
  console.log(`wrote ${path.relative(root, output)}`);
  return result;
}
