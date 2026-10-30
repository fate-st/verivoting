// Artifact/source/provenance audit only. Real cryptographic verification is the Rust run/replay.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
const read=p=>JSON.parse(fs.readFileSync(p,'utf8'));
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const r=read('experiments/g6_raw/native_winner_only.json');
const p=read('fixtures/jubjub_g6_native_winner_only_public_v1.json');
const replay=read('experiments/g6_raw/public_replay.json');
const old=read('experiments/g4_d_raw/ten_subsets_p3_v2.json');
const previous=read('fixtures/jubjub_g4d_ten_subsets_p3_v2_public.json');
const state=read('experiments/g2_e_raw/multibatch_state_b4.json');
assert.equal(r.status,'PASS_LIVE_RECEIVER_FREEZE_PRIVATE_DECRYPT_P3_WINNER_ONCE');
assert.equal(r.setup_rebuild_count,1); assert.equal(r.new_p3_proof_count,1);
assert.equal(r.new_private_packages,5); assert.equal(r.private_packages_verified,5);
assert.deepEqual([r.reused_ballot_proof_count,r.reused_snarkpack_proof_count,r.reused_state_proof_count],[8,2,2]);
assert.equal(r.g4d_relation_and_vk_unchanged,true);
assert.equal(r.verifying_key_sha256,old.verifying_key_sha256);
assert.equal(p.verifying_key_hex,previous.verifying_key_hex);
assert.equal(r.shape.matrix_sha256,old.matrix_sha256);
assert.deepEqual(r.final_checkpoint.state,state.final_state);
assert.deepEqual(replay.final_checkpoint,r.final_checkpoint);
assert.deepEqual(p.final_checkpoint,r.final_checkpoint);
assert.equal(r.frozen_checkpoint.public_winner,null);
assert.equal(r.frozen_checkpoint.winner_write_count,0);
assert.equal(r.final_checkpoint.public_winner,1);
assert.equal(r.final_checkpoint.winner_write_count,1);
assert.equal(r.final_checkpoint.spent_nullifier_count,8);
assert.equal(r.final_checkpoint.next_batch_index,3);
assert.equal(r.batches.length,2);
for(const [i,b] of r.batches.entries()) {
 assert.equal(b.snarkpack_verified,true);assert.equal(b.state_proof_verified,true);
 assert.equal(b.checkpoint.public_winner,null);assert.equal(b.checkpoint.frozen,false);
 assert.equal(b.checkpoint.spent_nullifier_count,(i+1)*4);
}
for(const flag of ['frozen_target_substitution_rejected','repeat_disclosure_rejected','second_freeze_rejected','post_freeze_batch_rejected','rejected_operations_do_not_write'])assert.equal(r.negative_tests[flag],true);
assert.equal(replay.status,'PASS_PUBLIC_RECEIVE_FREEZE_AND_WINNER_ONCE');
assert.equal(replay.setup_count,0);assert.equal(replay.new_p3_proof_count,0);assert.equal(replay.no_private_material_loaded,true);
assert.equal(r.public_fixture_sha256,sha(r.public_fixture));
assert.equal(p.proof_sha256,crypto.createHash('sha256').update(Buffer.from(p.proof_hex,'hex')).digest('hex'));
assert.equal(p.proof_hex.length/2,192);assert.equal(p.public_inputs_le_hex.length,60);
assert(old.cases.every(c=>c.proof_sha256!==p.proof_sha256));
assert(p.trustee_roster.every((x,i)=>x.package_commitment_hex!==previous.trustee_roster[i].package_commitment_hex));
for(const pin of [...r.source_hashes,...r.source_artifact_pins])assert.equal(sha(pin.path),pin.sha256,pin.path);
const forbidden=new Set(['tallies','private_tally','plaintext_groups','decryption_shares','cp_package','private_package','opening','secret_share','secret_key','response','cp_transcript_hex']);
function scan(x,path){if(Array.isArray(x))x.forEach((v,i)=>scan(v,`${path}[${i}]`));else if(x&&typeof x==='object')for(const [k,v]of Object.entries(x)){assert(!forbidden.has(k),`${path}.${k}`);scan(v,`${path}.${k}`);}}
scan(r,'raw');scan(p,'public');scan(replay,'replay');
assert.equal(r.private_tally_or_shares_emitted,false);assert.equal(r.proving_key_persisted,false);
assert.equal(r.gas_total,null);assert.equal(r.calldata_bytes,null);assert.equal(r.evm_run,false);assert.equal(r.full_settlement,false);
const commands=read('experiments/g6_raw/commands.json').commands;
assert.equal(commands.length,7);assert(commands.every(x=>x.exit_code===0&&!x.resource_budget_stop));
let archives=0;
for(const gate of ['g1_e','g2_d','g2_e','g4_d']) {
 for(const line of fs.readFileSync(`experiments/${gate}_raw/SHA256SUMS`,'utf8').trim().split('\n')){
  const match=line.match(/^([0-9a-f]{64})\s+(.+)$/);assert(match);
  const [,h,path]=match;if(!/^(experiments|fixtures|results)\//.test(path))continue;
  assert.equal(sha(path),h,path);archives++;
 }
}
console.log(JSON.stringify({status:'PASS_G6_ARTIFACT_SOURCE_AND_PUBLIC_EXPORT_AUDIT',source_hashes:r.source_hashes.length,input_pins:r.source_artifact_pins.length,historical_artifacts_unchanged:archives,one_new_proof:true,five_new_commitments:true,private_payload_fields_absent:true,all_seven_commands_pass:true}));
