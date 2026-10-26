import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { Common, Hardfork, Mainnet } from '@ethereumjs/common';
import { createVM } from '@ethereumjs/vm';
import { createAddressFromString, hexToBytes, bytesToHex } from '@ethereumjs/util';

// This gate tests prerequisites, NOT the SnarkPack verifier. No native result
// or supplied GT value is used as an EVM acceptance decision.
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const fixtureName = 'fixtures/snarkpack_g5c1_public_v1.json';
const sourceName = 'experiments/g5_c_raw/source_snarkpack_distinct_voters_batch_2.json';
const fixtureBytes = fs.readFileSync(path.join(root, fixtureName));
const fixture = JSON.parse(fixtureBytes);
const sourceBytes = fs.readFileSync(path.join(root, sourceName));
const source = JSON.parse(sourceBytes);
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
const decode = (hex) => hexToBytes(`0x${hex}`);
const metadata = (cmd, args) => execFileSync(cmd, args, { cwd: root, encoding: 'utf8' }).trim();
assert.equal(fixture.schema, 'VV/G5-C1/SNARKPACK-PUBLIC-FIXTURE/v1');
assert.equal(fixture.aggregate_version, 'V2');
assert.equal(fixture.batch_size, 2);
assert.equal(fixture.public_inputs_per_proof, 25);
assert.equal(fixture.verifier_srs.n, 2);
assert.equal(fixture.public_inputs_le_hex.length, 2);
assert.equal(fixture.changed_second_nullifier_public_inputs_le_hex.length, 2);
assert.equal(fixture.binary_proof_and_vk_roundtrip, true);
assert.equal(fixture.public_verifier_srs_roundtrip, true);
assert.equal(fixture.decoded_native_aggregate_verified, true);
assert.equal(fixture.decoded_changed_second_nullifier_rejected, true);
assert.equal(source.aggregate_proof_verified, true);
assert.equal(source.changed_second_nullifier_rejected, true);
assert.equal(source.registered_voter_statements_distinct, true);
assert.equal(fixture.private_witness_emitted, false);
assert.equal(fixture.aggregation_prover_srs_emitted, false);
assert.equal(fixture.groth16_proving_key_emitted, false);
const proofBytes = decode(fixture.aggregate_proof_hex);
assert.equal(proofBytes.length, fixture.aggregate_proof_bytes);
assert.equal(proofBytes.length, source.aggregate_proof_bytes);
assert.equal(sha256(proofBytes), fixture.aggregate_proof_sha256);
const transcript = decode(fixture.external_transcript_hex);
assert.equal(transcript.length, fixture.external_transcript_bytes);
assert.equal(sha256(transcript), fixture.external_transcript_sha256);
const domain = Buffer.from('VV/JUBJUB/G1-C0/SNARKPACK-PUBLIC-INPUTS/v1');
const be32 = (n) => { const b = Buffer.alloc(4); b.writeUInt32BE(n); return b; };
const encode = (rows) => Buffer.concat([domain, be32(rows.length), ...rows.flatMap((row) => {
  assert.equal(row.length, 25);
  return [be32(row.length), ...row.map((value) => {
    assert.match(value, /^[0-9a-f]{64}$/);
    return Buffer.from(value, 'hex');
  })];
})]);
assert.deepEqual(encode(fixture.public_inputs_le_hex), Buffer.from(transcript));
assert.equal(encode(fixture.changed_second_nullifier_public_inputs_le_hex).toString('hex'),
  fixture.changed_external_transcript_hex);
assert.deepEqual(fixture.public_inputs_le_hex[0],
  fixture.changed_second_nullifier_public_inputs_le_hex[0]);
const changedIndices = fixture.public_inputs_le_hex[1].flatMap((value, i) =>
  value === fixture.changed_second_nullifier_public_inputs_le_hex[1][i] ? [] : [i]);
assert.deepEqual(changedIndices, [7], 'negative must change only the second nullifier first packed field');

const common = new Common({ chain: Mainnet, hardfork: Hardfork.Prague });
assert.equal(common.isActivatedEIP(2537), true);
const vm = await createVM({ common });
async function probe(label, address, data, expected) {
  const started = performance.now();
  const result = await vm.evm.runCall({
    to: createAddressFromString(`0x${address.toString(16).padStart(40, '0')}`),
    data, gasLimit: 1_000_000n,
  });
  assert.equal(result.execResult.exceptionError, undefined, `${label} failed`);
  assert.deepEqual(result.execResult.returnValue, expected, `${label} output mismatch`);
  return {
    label, address: `0x${address.toString(16).padStart(2, '0')}`,
    input_bytes: data.length, output_bytes: result.execResult.returnValue.length,
    output_hex: bytesToHex(result.execResult.returnValue),
    execution_gas: Number(result.execResult.executionGasUsed),
    local_wall_ms: performance.now() - started, matched_expected: true,
  };
}
const probes = [];
probes.push(await probe('SHA-256 of exact external SnarkPack statement transcript', 2,
  transcript, decode(fixture.external_transcript_sha256)));

// Reuse public EIP-2537 points from G5-A for runtime ABI probes, not as a
// conversion of the compressed SnarkPack proof or as TIPP/MIPP validation.
const pointFixtureName = 'fixtures/g2b_eip2537_fixture_v1.json';
const pointFixtureBytes = fs.readFileSync(path.join(root, pointFixtureName));
const pointFixture = JSON.parse(pointFixtureBytes);
const vk = hexToBytes(pointFixture.vk_eip2537_hex);
const g1 = vk.slice(0, 128);
const g2 = vk.slice(128, 384);
const one = new Uint8Array(32); one[31] = 1;
probes.push(await probe('G2 MSM one public point times one', 14,
  Uint8Array.from([...g2, ...one]), g2));
const modulus = BigInt('0x1a0111ea397fe69a4b1ba7b6434bacd7' +
  '64774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab');
const y = BigInt(bytesToHex(g1.slice(64)));
const negY = ((modulus - y) % modulus).toString(16).padStart(128, '0');
const negG1 = Uint8Array.from([...g1.slice(0, 64), ...decode(negY)]);
probes.push(await probe('pairing product equals one', 15,
  Uint8Array.from([...g1, ...g2, ...negG1, ...g2]), one));
probes.push(await probe('single non-unit pairing returns false, not a GT element', 15,
  Uint8Array.from([...g1, ...g2]), new Uint8Array(32)));

const result = {
  gate: 'G5-C1', status: 'PASS_G5C1_PUBLIC_WIRE_FIXTURE_AND_PRAGUE_PRIMITIVES',
  target_verifier_status: 'BLOCKED_EXACT_SNARKPACK_V2_EVM_VERIFIER_NOT_IMPLEMENTED',
  classification: 'native-wire-roundtrip-and-local-evm-prerequisites-only',
  recorded_at_utc: new Date().toISOString(),
  git_commit: metadata('git', ['rev-parse', 'HEAD']),
  git_dirty: metadata('git', ['status', '--porcelain']).length > 0,
  cpu: os.cpus()[0].model, os: os.platform(), arch: os.arch(), node_version: process.version,
  hardfork: common.hardfork(), eip2537_active: common.isActivatedEIP(2537),
  fixture: fixtureName, fixture_sha256: sha256(fixtureBytes),
  source_run: sourceName, source_run_sha256: sha256(sourceBytes),
  point_probe_fixture: pointFixtureName, point_probe_fixture_sha256: sha256(pointFixtureBytes),
  package_lock_sha256: sha256(fs.readFileSync(path.join(root, 'contracts/package-lock.json'))),
  cargo_lock_sha256: sha256(fs.readFileSync(path.join(root, 'Cargo.lock'))),
  prototype_manifest_sha256: sha256(fs.readFileSync(path.join(root,
    'configs/protocol_manifest.JUBJUB-PROTOTYPE.json'))),
  batch_size: 2, public_inputs_per_proof: 25,
  aggregate_proof_bytes: proofBytes.length, external_transcript_bytes: transcript.length,
  decoded_native_aggregate_verified: true, decoded_changed_second_nullifier_rejected: true,
  external_transcript_rust_js_match: true, changed_second_statement_field_indices: changedIndices,
  primitive_probes: probes,
  exact_snarkpack_evm_verified: false, evm_changed_statement_rejection: null,
  snarkpack_verification_gas: null, full_settlement_measured: false, production_setup: false,
  blocker: 'Pinned verifier requires GT/Fp12 operations and pairing-to-non-unit-GT comparison; EIP-2537 exposes only pairing-product-to-one boolean. No exact GT/transcript verifier exists in this repository.',
  transcript_boundary: 'Only external statements are cross-language matched here. Internal bincode/GT/challenge transcript remains unported.',
  next_gate: 'G5-C2 exact internal transcript golden vectors and Rust/JavaScript challenge conformance before any Solidity GT port',
  limitations: [
    'Primitive runCall gas is not signed-transaction gas, SnarkPack verifier gas or settlement gas',
    'No full SnarkPack proof or changed-statement decision is evaluated by the EVM',
    'Native randomized pairing batching uses deterministic experimental RNG, not production contract randomness',
    'No claim that a compatible EVM verifier is impossible; a separately validated GT implementation or equivalent relation is required',
    'No suite, official Manifest capability, aggregation SRS ceremony or cryptographic security claim is changed',
  ],
  references: ['https://eips.ethereum.org/EIPS/eip-2537',
    'https://docs.rs/crate/bellperson/0.27.0/source/src/groth16/aggregate/verify.rs',
    'https://docs.rs/crate/bellperson/0.27.0/source/src/groth16/aggregate/transcript.rs'],
};
const output = path.resolve(root, process.argv[2] ?? 'experiments/g5_c_raw/snarkpack_route.json');
fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, JSON.stringify(result, null, 2) + '\n');
console.log(`G5-C1 PASS: ${proofBytes.length}-byte real native aggregate, ${transcript.length}-byte external transcript, ${probes.length} runtime probes`);
console.log('Full SnarkPack EVM verification remains BLOCKED; no aggregate verification gas reported.');
console.log(`wrote ${path.relative(root, output)}`);
