// Actual TIPP endpoint conformance + EVM ABI check. NOT an EVM GT verifier.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { bls12_381 } from '@noble/curves/bls12-381.js';
import { Common, Hardfork, Mainnet } from '@ethereumjs/common';
import { createVM } from '@ethereumjs/vm';
import { createAddressFromString, bytesToHex } from '@ethereumjs/util';
import { traceSnarkpackB2 } from './snarkpack_transcript_g5c2.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const read = (name) => fs.readFileSync(path.join(root, name));
const sha256 = (value) => createHash('sha256').update(value).digest('hex');
const metadata = (cmd, args) => execFileSync(cmd, args, { cwd: root, encoding: 'utf8' }).trim();
const fixtureName = 'fixtures/snarkpack_g5c3_tipp_endpoint_v1.json';
const fixtureBytes = read(fixtureName);
const fixture = JSON.parse(fixtureBytes);
const publicBytes = read('fixtures/snarkpack_g5c1_public_v1.json');
const source = JSON.parse(publicBytes);
const c2Bytes = read('fixtures/snarkpack_g5c2_transcript_v1.json');
const c2 = JSON.parse(c2Bytes);
const { Fp, Fp6, Fp12, Fr } = bls12_381.fields;
assert.equal(fixture.schema, 'VV/G5-C3/TIPP-ENDPOINT/v1');
assert.equal(fixture.encoding, 'VV/G5-C3/Fp12-12xFp64BE/v1');
assert.equal(fixture.source_fixture, 'fixtures/snarkpack_g5c1_public_v1.json');
assert.equal(fixture.source_fixture_sha256, sha256(publicBytes));
assert.equal(fixture.c2_trace_fixture, 'fixtures/snarkpack_g5c2_transcript_v1.json');
assert.equal(fixture.c2_trace_fixture_sha256, sha256(c2Bytes));
assert.equal(source.schema, 'VV/G5-C1/SNARKPACK-PUBLIC-FIXTURE/v1');
assert.equal(source.aggregate_version, 'V2');
assert.equal(source.batch_size, 2);
assert.equal(c2.schema, 'VV/G5-C2/SNARKPACK-INTERNAL-TRANSCRIPT/v1');
assert.equal(c2.fixture_sha256, sha256(publicBytes));
assert.equal(fixture.setup_run, false);
assert.equal(fixture.prove_run, false);
assert.equal(fixture.private_witness_emitted, false);
const proof = Buffer.from(source.aggregate_proof_hex, 'hex');
assert.equal(proof.length, 5236);
for (const value of [fixture, source, c2]) {
  assert.equal(value.aggregate_proof_sha256, sha256(proof));
}
assert.equal(proof.readUInt32LE(1488), 2);
assert.deepEqual(fixture.wire_offsets, {
  ip_ab: 1152, z_ab_left: 3796, z_ab_right: 4084, final_a: 4468, final_b: 4516,
});
// Recompute the FULL C2 trace from wire + external bytes, not native coefficients
// or a host-supplied challenge. Compare all base bytes/counters/scalars.
const traces = traceSnarkpackB2(source.aggregate_proof_hex, source.external_transcript_hex);
assert.deepEqual(traces, c2.valid);
const fromLE = (bytes) => BigInt(`0x${Buffer.from(bytes).reverse().toString('hex')}`);
const cInv = fromLE(Buffer.from(traces.find((row) => row.id === 'gipa-0-round').challenge_le_hex, 'hex'));
const c = Fr.inv(cInv);
assert.equal(fromLE(Buffer.from(fixture.c_inv_le_hex, 'hex')), cInv);
assert.equal(fromLE(Buffer.from(fixture.c_le_hex, 'hex')), c);
assert.equal(Fr.mul(c, cInv), 1n);
const coefficients6 = (value) => [value.c0.c0, value.c0.c1, value.c1.c0,
  value.c1.c1, value.c2.c0, value.c2.c1];
const fpBE = (value, length = 64) => {
  assert.ok(value >= 0n && value < Fp.ORDER);
  return Buffer.from(value.toString(16).padStart(length * 2, '0'), 'hex');
};
const encode12 = (value) => `0x${Buffer.concat([
  ...coefficients6(value.c0), ...coefficients6(value.c1),
].map((value) => fpBE(value))).toString('hex')}`;
function decodeGT(offset) {
  const wire = proof.subarray(offset, offset + 288);
  assert.equal(wire.length, 288);
  const coefficients = Array.from({ length: 6 }, (_, i) => {
    const coefficient = fromLE(wire.subarray(i * 48, (i + 1) * 48));
    assert.ok(coefficient < Fp.ORDER);
    return coefficient;
  });
  const torus = Fp6.fromBigSix(coefficients);
  const value = Fp12.mul(Fp12.create({ c0: torus, c1: Fp6.ONE }),
    Fp12.inv(Fp12.create({ c0: torus, c1: Fp6.neg(Fp6.ONE) })));
  assert.ok(Fp12.eql(Fp12.pow(value, Fr.ORDER), Fp12.ONE));
  const recompressed = Fp6.mul(Fp6.add(value.c0, Fp6.ONE), Fp6.inv(value.c1));
  assert.deepEqual(Buffer.concat(coefficients6(recompressed)
    .map((value) => fpBE(value, 48).reverse())), wire);
  return value;
}
const ip = decodeGT(1152);
const target = Fp12.mul(Fp12.mul(ip, Fp12.pow(decodeGT(3796), c)),
  Fp12.pow(decodeGT(4084), cInv));
const aBytes = proof.subarray(4468, 4516);
const bBytes = proof.subarray(4516, 4612);
const a = bls12_381.G1.Point.fromBytes(aBytes);
const b = bls12_381.G2.Point.fromBytes(bBytes);
assert.deepEqual(Buffer.from(a.toBytes(true)), aBytes);
assert.deepEqual(Buffer.from(b.toBytes(true)), bBytes);
assert.equal(aBytes.toString('hex'), fixture.final_a_compressed_hex);
assert.equal(bBytes.toString('hex'), fixture.final_b_compressed_hex);
assert.equal(Buffer.from(a.toBytes(false)).toString('hex'), fixture.final_a_uncompressed_hex);
assert.equal(Buffer.from(b.toBytes(false)).toString('hex'), fixture.final_b_uncompressed_hex);
const started = performance.now();
const paired = bls12_381.pairing(a, b, true);
const independentPairingMs = performance.now() - started;
assert.ok(Fp12.eql(paired, target), 'actual TIPP endpoint equality failed');
assert.equal(encode12(target), fixture.target_fp12_hex);
assert.equal(encode12(paired), fixture.pairing_fp12_hex);
const wrong = Fp12.mul(target, ip);
assert.equal(encode12(wrong), fixture.wrong_target_fp12_hex);
assert.ok(!Fp12.eql(wrong, paired), 'wrong in-subgroup target accepted');
for (const value of [target, wrong]) {
  assert.ok(!Fp12.eql(value, Fp12.ONE));
  assert.ok(Fp12.eql(Fp12.pow(value, Fr.ORDER), Fp12.ONE));
}
assert.equal(fixture.native_endpoint_verified, true);
assert.equal(fixture.native_wrong_gt_target_rejected, true);

// Only the actual endpoint points are sent to the existing precompile. There
// is NO GT target slot in this ABI. Its false result is NOT endpoint rejection.
const p = a.toAffine();
const q = b.toAffine();
const eipInput = Buffer.concat([fpBE(p.x), fpBE(p.y),
  fpBE(q.x.c0), fpBE(q.x.c1), fpBE(q.y.c0), fpBE(q.y.c1)]);
assert.equal(eipInput.length, 384);
const common = new Common({ chain: Mainnet, hardfork: Hardfork.Prague });
assert.equal(common.isActivatedEIP(2537), true);
const vm = await createVM({ common });
const gasBudget = 1_000_000n;
const probeStarted = performance.now();
const probe = await vm.evm.runCall({ to: createAddressFromString(
  '0x000000000000000000000000000000000000000f'), data: eipInput, gasLimit: gasBudget });
assert.equal(probe.execResult.exceptionError, undefined);
assert.deepEqual(probe.execResult.returnValue, new Uint8Array(32));
const output = {
  gate: 'G5-C3-B', status: 'PARTIAL_PASS_TIPP_ENDPOINT_CONFORMANCE_EVM_TARGET_BLOCKED',
  target_verifier_status: 'BLOCKED_EXACT_PAIRING_TO_GT_IMPLEMENTATION_MISSING',
  classification: 'actual-proof-tipp-endpoint-native-js-conformance-and-evm-abi-probe-only',
  recorded_at_utc: new Date().toISOString(), git_commit: metadata('git', ['rev-parse', 'HEAD']),
  git_dirty: metadata('git', ['status', '--porcelain']).length > 0,
  cpu: os.cpus()[0].model, os: os.platform(), arch: os.arch(), node_version: process.version,
  rustc_version: metadata('rustc', ['+1.99.1', '--version']),
  native_build_profile: 'release-opt-level-1', hardfork: common.hardfork(),
  fixture: fixtureName, fixture_sha256: sha256(fixtureBytes),
  source_fixture: fixture.source_fixture, source_fixture_sha256: sha256(publicBytes),
  aggregate_proof_sha256: sha256(proof), batch_size: 2,
  c2_trace_fixture_sha256: sha256(c2Bytes),
  native_generator_source_sha256: sha256(read('crates/vv-friendly-dkg/src/bin/g5c3_tipp_endpoint.rs')),
  runner_source_sha256: sha256(fs.readFileSync(fileURLToPath(import.meta.url))),
  c2_transcript_module_sha256: sha256(read('contracts/scripts/snarkpack_transcript_g5c2.mjs')),
  package_lock_sha256: sha256(read('contracts/package-lock.json')),
  cargo_lock_sha256: sha256(read('Cargo.lock')),
  official_manifest_sha256: sha256(read('configs/protocol_manifest.json')),
  prototype_manifest_sha256: sha256(read('configs/protocol_manifest.JUBJUB-PROTOTYPE.json')),
  official_verifier_sha256: c2.reference_verifier_sha256,
  dependencies: { bellperson: '0.27.0', blstrs: '0.7.0', noble_curves: '2.4.0', ethereumjs_vm: '10.1.3' },
  relation: fixture.relation, c_le_hex: fixture.c_le_hex, c_inv_le_hex: fixture.c_inv_le_hex,
  independently_recomputed_transcript_calls: traces.length,
  native_endpoint_verified: true, independent_js_endpoint_verified: true,
  native_js_target_and_pairing_match: true, native_wrong_gt_target_rejected: true,
  independent_js_wrong_gt_target_rejected: true, wrong_target_mutation: fixture.wrong_target_mutation,
  target_is_nonunit_gt: true, wrong_target_is_nonunit_gt: true,
  independent_js_pairing_wall_ms: independentPairingMs,
  evm_identity_probe: {
    address: '0x0f', execution_scope: 'local runCall primitive probe; NOT signed transaction or endpoint verifier',
    input_hex: bytesToHex(eipInput), input_bytes: eipInput.length, input_sha256: sha256(eipInput),
    output_hex: bytesToHex(probe.execResult.returnValue), output_bytes: probe.execResult.returnValue.length,
    execution_gas: Number(probe.execResult.executionGasUsed), gas_budget: Number(gasBudget),
    local_vm_wall_ms: performance.now() - probeStarted,
    success: true, checks_pairing_product_equals_one: true, gt_target_supplied: false,
    meaning: 'False means this actual pairing is not one; it does not compare the correct or wrong GT target',
  },
  resource_conclusion: 'Implementation/API gap; no software-pairing OOG or impossibility claim has been established',
  pairing_to_gt_evm_verified: false, evm_wrong_gt_target_rejected: null,
  complete_g5c3: false, exact_snarkpack_evm_verified: false, snarkpack_verification_gas: null,
  full_software_pairing_attempted: false, setup_run: false, prove_run: false, private_witness_emitted: false,
  blocker: 'No audited EVM Miller loop plus final exponentiation or validated equivalent relation in repository; the boolean precompile has no arbitrary GT target input/output',
  next_gate: 'Implement and cross-check a software Miller-loop component on this fixed endpoint before final exponentiation; no scale or new proving runs',
  references: ['https://eips.ethereum.org/EIPS/eip-2537#abi-for-pairing-check'],
  limitations: [
    'One endpoint is not full GIPA/TIPP/MIPP/KZG or SnarkPack verification',
    'Wrong-target rejection is native/JS only, not EVM rejection',
    'Primitive gas is not transaction gas, pairing-to-GT verifier gas or aggregate gas',
    'EVM cannot trust the native or JS GT result as a substitute for constrained arithmetic',
    'No GT preimage inversion, wrapper proof, curve change or Manifest promotion is made',
  ],
};
const outputPath = path.resolve(root, process.argv[2] ?? 'experiments/g5_c3_raw/tipp_endpoint_route.json');
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, `${JSON.stringify(output, null, 2)}\n`);
console.log('G5-C3-B partial PASS: actual TIPP equality matches native/JS; wrong GT target rejected there.');
console.log(`EVM actual-endpoint probe: ${output.evm_identity_probe.execution_gas} primitive gas, returns false (nonunit), NOT GT-target comparison.`);
console.log('Exact EVM pairing-to-GT remains BLOCKED by implementation, not a measured resource failure.');
console.log(`wrote ${path.relative(root, outputPath)}`);
