// Read-only evidence audit; does not execute EVM, Setup or Prove.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
const sha = b => createHash('sha256').update(b).digest('hex');
export function audit(run) {
  const dir = path.resolve(run), read = name => JSON.parse(fs.readFileSync(path.join(dir, name + '.json')));
  const sums = path.join(dir, 'SHA256SUMS');
  if (fs.existsSync(sums)) for (const line of fs.readFileSync(sums, 'utf8').trimEnd().split('\n')) {
    const match = /^([0-9a-f]{64})  (.+)$/.exec(line); assert.ok(match, 'checksum line');
    const target = path.resolve(dir, match[2]); assert.ok(target.startsWith(dir + path.sep));
    assert.equal(sha(fs.readFileSync(target)), match[1], 'archive checksum ' + match[2]);
  }
  const m = read('execution'), p = read('points'), a = read('direct_fold'), b = read('state_proof');
  assert.equal(m.status, 'PASS_PAIRED_EVM_FOLD_COMPARISON');
  assert.equal(m.commands.length, 8); assert.ok(m.commands.every(c => c.code === 0 && !c.termination));
  for (const key of ['new_setup', 'new_proofs', 'new_p3', 'new_dkg']) assert.equal(m[key], 0);
  assert.equal(m.exact_snarkpack_evm, false);
  assert.equal(p.status, 'PASS_RUST_EVM_POINT_CONFORMANCE');
  assert.equal(p.valid_probes.length, 8); assert.equal(p.invalid_probes.length, 3);
  assert.ok(p.invalid_probes.every(x => x.normal_revert));
  const sum = list => list.reduce((n, x) => n + x, 0);
  const reasons = ['UNCERTIFIED', null, 'OLD_STATE', 'ACCEPTED_ROOT', 'INVALID_GROTH16', 'BATCH_ROOT', 'NEXT_INDEX', 'SPENT_NULLIFIER', 'FROZEN'];
  for (const r of [a, b]) {
    assert.equal(r.status, 'PASS_TWO_B4_ACCEPTANCE_AND_FREEZE');
    assert.equal(r.hardfork, 'prague'); assert.equal(r.tx_gas_limit, 20000000);
    assert.equal(r.block_gas_limit, 30000000); assert.equal(r.snarkpack_evm_verified, false);
    assert.equal(r.new_setup, 0); assert.equal(r.new_proofs, 0); assert.equal(r.new_p3, 0);
    assert.equal(r.transactions.length, 18); assert.equal(r.negatives.length, 9);
    for (const t of r.transactions) {
      const data = Buffer.from(t.calldata_hex.slice(2), 'hex');
      assert.equal(data.length, t.calldata_bytes); assert.equal(sha(data), t.calldata_sha256);
      assert.equal(data.filter(x => x === 0).length, t.calldata_zero_bytes);
      assert.equal(t.calldata_zero_bytes + t.calldata_nonzero_bytes, t.calldata_bytes);
      assert.equal(t.refund, 0);
      assert.equal(t.gas_used, Math.max(t.intrinsic_gas + t.execution_gas, t.calldata_floor));
      assert.equal(t.receipt_cumulative_gas, t.gas_used);
      assert.ok(t.gas_used < t.gas_limit); assert.ok(!String(t.exception).includes('out of gas'));
    }
    r.negatives.forEach((n, i) => {
      assert.equal(n.reason, reasons[i] ?? (r.phase === 'direct_fold' ? 'DIRECT_FOLD_MISMATCH' : 'INVALID_GROTH16'));
      assert.equal(n.application_checkpoint_unchanged, true); assert.deepEqual(n.before, n.after);
      const t = r.transactions.find(x => x.hash === n.tx_hash);
      assert.equal(t.status, 0); assert.equal(t.exception, 'revert'); assert.equal(t.revert_reason, n.reason);
      assert.equal(t.logs.length, 0);
    });
    const bad = r.negatives[1];
    assert.equal(bad.before.next, 1); assert.equal(bad.before.frozen, false); assert.equal(bad.before.spent_count, 0);
    assert.equal(bad.pairings, r.phase === 'direct_fold' ? 4 : 5);
    const pairings = r.transactions.find(x => x.hash === bad.tx_hash).calls.filter(x => x.address.endsWith('000f'));
    assert.ok(pairings.slice(0, 4).every(x => BigInt(x.return_hex) === 1n));
    if (r.phase === 'state_proof') assert.equal(BigInt(pairings[4].return_hex), 0n);
    const accepted = r.transactions.filter(t => t.label.startsWith('accept_batch_'));
    assert.equal(accepted.length, 2);
    for (const t of accepted) {
      const ps = t.calls.filter(x => x.address.endsWith('000f'));
      assert.equal(ps.length, r.phase === 'direct_fold' ? 4 : 5);
      assert.ok(ps.every(x => x.success && BigInt(x.return_hex) === 1n)); assert.equal(t.status, 1);
    }
    assert.equal(sum(accepted.map(t => t.gas_used)), r.accepted_batch_gas);
    assert.equal(sum(accepted.map(t => t.calldata_bytes)), r.accepted_batch_calldata_bytes);
    assert.equal(r.final_checkpoint.spent_count, 8); assert.equal(r.final_checkpoint.next, 3);
    assert.equal(r.final_checkpoint.frozen, true); assert.ok(r.final_checkpoint.spent.every(Boolean));
    assert.deepEqual(r.final_checkpoint, r.frozen_checkpoint);
  }
  assert.equal(a.initial_vm_state_root, b.initial_vm_state_root);
  for (const key of ['certified_checkpoint', 'checkpoints', 'frozen_checkpoint']) assert.deepEqual(a[key], b[key]);
  for (const entry of read('source_files')) {
    assert.equal(sha(fs.readFileSync(path.join(dir, 'source_snapshot', entry.path))), entry.sha256, entry.path);
    assert.equal(sha(fs.readFileSync(entry.path)), entry.sha256, 'current execution source changed: ' + entry.path);
  }
  const rolling = new Set(['docs/NEXT_STEPS.md', 'docs/IMPLEMENTATION_STATUS.md']);
  const history = read('historical_files');
  for (const e of history) if (!rolling.has(e.path)) assert.equal(sha(fs.readFileSync(e.path)), e.sha256, 'old file changed: ' + e.path);
  for (const e of read('source_pins').fixture_pins) assert.equal(sha(fs.readFileSync(e.path)), e.sha256, e.path);
  return {status:'PASS_EVIDENCE_AUDIT', transactions:36, atomic_negative_transactions:18,
    matched_checkpoints:true, immutable_historical_files_checked:history.length - 2,
    source_version_sha256:m.source_version_sha256};
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  assert.ok(process.argv[2], 'usage: node scripts/stage2_evm_fold_check.mjs RUN_DIR');
  console.log(JSON.stringify(audit(process.argv[2]), null, 2));
}
