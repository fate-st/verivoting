// Lightweight archive/receipt consistency audit. This does not execute cryptography.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
const read = p => JSON.parse(fs.readFileSync(p));
const digest = b => createHash('sha256').update(b).digest('hex');
const sha = p => digest(fs.readFileSync(p));
const r = read('experiments/g7_raw/g7_p3_prague_evm.json');
const exported = read(r.export_report), f = read(r.fixture), g6 = read(f.source_fixture);
const d = read('fixtures/jubjub_g4d_ten_subsets_p3_v2_public.json');
assert.equal(r.status, 'PASS_SAME_G6_PROOF_G4D_VK_PRAGUE_1_POSITIVE_4_NEGATIVES');
assert.equal(r.proof_ark_sha256, digest(Buffer.from(g6.proof_hex, 'hex')));
assert.equal(r.verifying_key_ark_sha256, digest(Buffer.from(g6.verifying_key_hex, 'hex')));
assert.equal(g6.verifying_key_hex, d.verifying_key_hex);
assert.deepEqual(f.public_inputs_be_hex, g6.public_inputs_le_hex.map(x => `0x${Buffer.from(x, 'hex').reverse().toString('hex')}`));
assert.equal(f.source_fixture_sha256, sha(f.source_fixture));
assert.equal(r.fixture_sha256, sha(r.fixture)); assert.equal(r.export_report_sha256, sha(r.export_report));
for (const pin of [...r.source_pins, ...r.source_hashes, ...exported.source_hashes]) assert.equal(sha(pin.path), pin.sha256, pin.path);
assert.equal(r.setup_count, 0); assert.equal(r.proof_generation_count, 0);
assert.equal(r.transaction_count, 7); assert.equal(r.hardfork, 'prague');
assert.equal(r.public_input_count, 60); assert.equal(r.valid.receipt_status, 1);
assert.equal(r.valid.calldata_bytes, 2564);
assert.deepEqual(Object.keys(r.negatives), ['wrong_selector', 'wrong_winner', 'changed_ciphertext', 'wrong_vk']);
for (const [name, tx] of Object.entries({valid: r.valid, ...r.negatives})) {
  const accepted = name === 'valid';
  assert.equal(tx.receipt_status, accepted ? 1 : 0);
  assert.equal(tx.revert_reason, accepted ? null : 'INVALID_GROTH16');
  assert.deepEqual(tx.precompile_calls.map(x => x.address.slice(-4)), ['000c', '000f']);
  assert.ok(tx.precompile_calls.every(x => x.success && x.exception === null));
  assert.equal(BigInt(tx.precompile_calls[1].return_hex), accepted ? 1n : 0n);
}
for (const tx of [...Object.values(r.deployments), r.valid, ...Object.values(r.negatives)]) {
  const data = Buffer.from(tx.calldata_hex.slice(2), 'hex');
  assert.equal(data.length, tx.calldata_bytes); assert.equal(digest(data), tx.calldata_sha256);
  assert.equal(tx.total_gas, Math.max(tx.intrinsic_gas + tx.execution_gas, tx.minimum_gas_limit_including_calldata_floor));
  assert.equal(tx.gas_refund, 0); assert.ok(tx.total_gas < tx.transaction_gas_limit);
}
assert.equal(r.valid.calldata_hex, r.negatives.wrong_vk.calldata_hex);
assert.equal(r.contract_artifact_sha256, sha(r.contract_artifact));
const a = read(r.contract_artifact);
assert.equal(digest(Buffer.from(a.deployed_g4d_runtime_hex.slice(2), 'hex')), r.deployments.g4d.runtime_bytecode_sha256);
assert.equal(digest(Buffer.from(a.deployed_old_g4c_runtime_hex.slice(2), 'hex')), r.deployments.old_fixed123.runtime_bytecode_sha256);
assert.equal(r.freeze_storage_binding_checked, false); assert.equal(r.complete_settlement_measured, false);
assert.equal(r.snarkpack_evm_verified, false); assert.equal(r.full_system_gas, null);
// Historical G6 sources remain byte-identical, except Cargo.toml's new binary registration.
const previous = read('experiments/g6_raw/native_winner_only.json');
let historicalSources = 0;
for (const pin of previous.source_hashes) {
  const actual = pin.path === 'crates/vv-friendly-dkg/Cargo.toml'
    ? digest(execFileSync('git', ['show', `35fc537:${pin.path}`])) : sha(pin.path);
  assert.equal(actual, pin.sha256, pin.path); historicalSources++;
}
let archives = 0;
for (const gate of ['g1_e', 'g2_d', 'g2_e', 'g4_d', 'g6']) {
  for (const line of fs.readFileSync(`experiments/${gate}_raw/SHA256SUMS`, 'utf8').trim().split('\n')) {
    const [, hash, p] = line.match(/^([0-9a-f]{64})\s+(.+)$/) ?? [];
    assert.ok(p);
    if (!/^(experiments|fixtures|results)\//.test(p) || p.startsWith('experiments/aggregate/')) continue;
    assert.equal(sha(p), hash, p); archives++;
  }
}
const commands = read('experiments/g7_raw/commands.json').commands;
const latest = Object.fromEntries(commands.map(c => [c.name, c]));
for (const name of ['fmt-check', 'clippy', 'tests', 'state-tests', 'build', 'public-export', 'evm']) assert.equal(latest[name].exit_code, 0, name);
console.log(JSON.stringify({status:'PASS_G7_ARCHIVE_RECEIPT_SOURCE_AUDIT',
  historical_artifacts_unchanged:archives,historical_g6_source_hashes_checked:historicalSources,
  final_command_outcomes_pass:true,earlier_failed_attempts_preserved:commands.filter(c=>c.exit_code!==0).length,
  one_archived_proof_no_setup_or_prove:true,positive_and_four_pairing_negatives:true,
  contract_state_binding_or_snarkpack_claimed:false}));
