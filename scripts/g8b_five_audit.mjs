// Audit the supplemental real execution without creating new crypto results.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
const {Interface,Transaction,keccak256}=createRequire(new URL('../contracts/package.json',import.meta.url))('ethers');
const read=p=>JSON.parse(fs.readFileSync(p)),hash=b=>createHash('sha256').update(b).digest('hex'),sha=p=>hash(fs.readFileSync(p));
const bytes=h=>Buffer.from(h.replace(/^0x/,''),'hex'),hex=s=>`0x${s}`;
const r=read('experiments/g8b_five_raw/all_trustees.json'),old=read('experiments/g8b_raw/trustee_commitments.json');
const f=read('fixtures/g8b_trustee_commitments_public_v1.json'),g6=read(f.source_g6_fixture),art=read(old.contract_artifact);
assert.equal(r.status,'PASS_FIVE_ORIGINAL_COMMITMENTS_CERTIFIER_AND_DECRYPTION_ROSTER_SEPARATED');
for(const p of [...r.source_pins,...r.source_hashes])assert.equal(sha(p.path),p.sha256,p.path);
for(const p of r.source_hashes)assert.equal(sha(`experiments/g8b_five_raw/provenance/${p.path.replaceAll('/','__')}.executed.txt`),p.sha256);
assert.equal(r.transactions.length,23);assert.equal(r.transaction_count,23);assert.equal(r.prefix_transactions_reexecuted,13);
const prefix=r.transactions.slice(0,13);assert.equal(prefix.filter(t=>t.receipt_status===0).length,2);
for(const [i,t]of prefix.entries()){
 assert.equal(t.kind,'reexecuted_exact_prefix');
 for(const k of ['signed_transaction_hex','transaction_hash','calldata_hex','receipt_status','total_gas','intrinsic_gas','execution_gas','return_hex','revert_reason','precompile_calls'])assert.deepEqual(t[k],old.transactions[i][k],`${i} ${k}`);
}
assert.deepEqual(r.transactions.map(t=>t.nonce),Array.from({length:23},(_,i)=>i));
for(const t of r.transactions){
 const signed=Transaction.from(t.signed_transaction_hex);assert.ok(signed.isSigned());assert.equal(signed.nonce,t.nonce);assert.equal(signed.hash,t.transaction_hash);
 assert.equal(signed.data,t.calldata_hex);assert.equal(signed.to?.toLowerCase()??null,t.to);assert.equal(signed.chainId,1n);
 assert.equal(signed.gasLimit,20_000_000n);assert.equal(bytes(t.calldata_hex).length,t.calldata_bytes);assert.equal(hash(bytes(t.calldata_hex)),t.calldata_sha256);
 assert.equal(t.total_gas,Math.max(t.intrinsic_gas+t.execution_gas,t.minimum_gas_limit_including_calldata_floor));assert.ok(t.total_gas<20_000_000);
}
assert.equal(new Set(r.transactions.map(t=>Transaction.from(t.signed_transaction_hex).from)).size,1);
for(const check of r.runtime_checks){const actual=art.contracts[check.name];assert.equal(check.address,actual.address);assert.equal(check.runtime_sha256,hash(bytes(actual.runtime_bytecode_hex)));
 assert.equal(check.runtime_keccak256,keccak256(actual.runtime_bytecode_hex));assert.ok(check.exact_g8b_runtime_match);}
const abi=new Interface(art.contracts.trustee_commitments.abi),gateAbi=new Interface(art.contracts.certificate_gate.abi);
const certified=prefix.filter(t=>t.label.startsWith('authenticate_dkg_')).map(t=>Number(gateAbi.decodeFunctionData('submitDkgSignature',t.calldata_hex)[0]));
assert.deepEqual(certified,[1,2,3]);assert.deepEqual(r.dkg_certifier_ids,certified);
assert.equal(r.q_cert,3);assert.equal(r.tau,3);assert.equal(r.initial_checkpoint.count,0);
assert.deepEqual(r.commitment_submission_order,[4,5,3,1,2]);assert.deepEqual(r.accepted_commitments.map(t=>t.trustee_id),[4,5,3,1,2]);
for(const [i,accepted]of r.accepted_commitments.entries()){
 const t=r.transactions[accepted.transaction_nonce],id=accepted.trustee_id,row=f.records[id-1],s=abi.decodeFunctionData('submitCommitment',t.calldata_hex)[0];
 assert.equal(t.receipt_status,1);assert.equal(t.to,art.contracts.trustee_commitments.address);assert.equal(Number(s.trusteeId),id);
 assert.equal(s.electionId,hex(f.public_context.election_id_hex));assert.equal(s.finalContext,hex(f.public_context.final_context_hex));assert.equal(s.target,hex(f.public_context.cp_target_hex));
 assert.equal(s.verificationShare.encoded,hex(row.verification_share.encoded_hex));assert.equal(s.commitment.encoded,hex(row.commitment.encoded_hex));
 assert.equal(s.signature.nonce.encoded,hex(row.signature.nonce.encoded_hex));assert.equal(s.signature.response.toString(),row.signature.response_scalar);assert.equal(Number(s.signature.counter),row.signature.counter);
 const signature=Buffer.concat([bytes(s.signature.nonce.encoded),Buffer.from(BigInt(s.signature.response).toString(16).padStart(64,'0'),'hex').reverse(),Buffer.from(Number(s.signature.counter).toString(16).padStart(4,'0'),'hex')]);
 assert.equal(signature.toString('hex'),g6.trustee_roster[id-1].commitment_signature_hex,'exact archived authentication signature, no re-signing');
 assert.ok(t.precompile_calls.some(p=>p.address.endsWith('0002')&&p.return_hex===hex(row.original_message_hex)));
 const event=abi.parseLog(t.logs.at(-1));assert.equal(event.name,'CommitmentAuthenticated');assert.equal(Number(event.args.trusteeId),id);assert.equal(event.args.target,s.target);
 const state=accepted.checkpoint;assert.equal(state.count,i+1);assert.equal(state.p3_ready,i>=2);assert.equal(state.certifier_count,3);
 assert.deepEqual(state.certifier_mask,[true,true,true,false,false]);assert.equal(state.submitted[id-1],true);
 assert.equal(accepted.is_dkg_certifier,id<=3);assert.equal(state.commitments[id-1],s.commitment.encoded);assert.equal(state.verification_shares[id-1],s.verificationShare.encoded);
 assert.equal(state.freeze_seal,hex(g6.freeze_digest_hex));assert.equal(state.cp_target,hex(f.public_context.cp_target_hex));
}
const subset=r.first_ready_checkpoint;assert.deepEqual(subset.submitted,[false,false,true,true,true]);assert.equal(subset.count,3);assert.ok(subset.p3_ready);
assert.deepEqual(r.first_ready_commitment_set,[3,4,5]);assert.deepEqual(subset.certifier_mask,[true,true,true,false,false]);
const ready=r.transactions.find(t=>t.label==='commitment_set_345_ready_for_future_p3');assert.equal(ready.receipt_status,1);
const final=r.final_checkpoint;assert.equal(final.count,5);assert.deepEqual(final.submitted,[true,true,true,true,true]);assert.deepEqual(final.certifier_mask,subset.certifier_mask);
assert.equal(final.freeze_seal,hex(g6.freeze_digest_hex));assert.equal(final.receiver_seal,final.freeze_seal);assert.equal(final.receiver_target,final.cp_target);
assert.deepEqual(final.receiver_state,f.public_context.frozen_ciphertexts.flatMap(p=>[hex(p.c0_hex),hex(p.c1_hex)]));
const reasons={forged_trustee4_signature:'BAD_PACKAGE_SIGNATURE',changed_trustee4_cp_target:'CP_TARGET',duplicate_trustee4:'DUPLICATE_TRUSTEE',only_noncertifiers_4_5_not_ready:'DECRYPTION_QUORUM'};
assert.equal(r.negative_tests.length,4);
for(const n of r.negative_tests){const t=r.transactions[n.transaction_nonce];assert.equal(t.receipt_status,0);assert.equal(t.exception,'revert');assert.equal(t.revert_reason,reasons[n.case]);assert.equal(t.logs.length,0);assert.ok(n.atomic_rejection);assert.deepEqual(n.before,n.after);}
const two=r.negative_tests.find(t=>t.case==='only_noncertifiers_4_5_not_ready').before;
assert.deepEqual(two.submitted,[false,false,false,true,true]);assert.equal(two.count,2);assert.equal(two.certifier_count,3);assert.equal(two.p3_ready,false);
assert.equal(r.original_package_signatures_verified_evm,5);assert.equal(r.verification_share_polynomial_checks_evm,5);
for(const k of ['new_signatures','new_cp_packages','setup_count','proof_generation_count'])assert.equal(r[k],0);
for(const k of ['contract_sources_changed','bytecode_changed','dkg_reexecuted','private_witness_emitted','p3_verified_in_this_flow','winner_written','receiver_auth_gate_enforced_on_intake','complete_g8_settlement','snarkpack_evm_verified'])assert.equal(r[k],false);
assert.equal(r.full_system_gas,null);
let archiveEntries=0;
for(const gate of ['g1_e','g2_d','g2_e','g4_d','g6','g7','g8','g8a','g8b'])for(const line of fs.readFileSync(`experiments/${gate}_raw/SHA256SUMS`,'utf8').trim().split('\n')){
 const[,h,p]=line.match(/^([0-9a-f]{64})\s+(.+)$/)??[];assert.ok(p);
 if(!/^(experiments|fixtures|results)\//.test(p)||p.startsWith('experiments/aggregate/'))continue;
 assert.equal(sha(p),h,p);archiveEntries++;
}
const latest=Object.fromEntries(read('experiments/g8b_five_raw/commands.json').commands.map(t=>[t.name,t]));
for(const name of ['evm','baseline-audit'])assert.equal(latest[name].exit_code,0,name);
console.log(JSON.stringify({status:'PASS_G8B_FIVE_TRUSTEES_EXACT_ARCHIVE_AND_DISTINCT_ROLES_AUDIT',signed_transactions:23,
 original_package_signatures_evm:5,unchanged_exact_prefix_transactions:13,dkg_certifiers:[1,2,3],first_ready_commitment_set:[3,4,5],
 new_atomic_negatives:4,historical_archive_entries_unchanged:archiveEntries,contract_sources_changed:false,full_g8:false}));
