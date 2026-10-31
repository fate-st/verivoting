// Provenance/ABI/trace audit only, not another cryptographic benchmark.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
const {Interface,Transaction,AbiCoder,keccak256}=createRequire(new URL('../contracts/package.json',import.meta.url))('ethers');
const read=p=>JSON.parse(fs.readFileSync(p)),hash=b=>createHash('sha256').update(b).digest('hex'),sha=p=>hash(fs.readFileSync(p));
const bytes=h=>Buffer.from(h.replace(/^0x/,''),'hex'),hex=s=>`0x${s}`;
const r=read('experiments/g8b_raw/trustee_commitments.json'),f=read(r.fixture),e=read(r.export_report),g6=read(f.source_g6_fixture),a=read(f.source_g8a_fixture),b=read(r.receiver_fixture);
assert.equal(r.status,'PASS_ORIGINAL_TRUSTEE_COMMITMENTS_AUTHENTICATED_FOR_LIVE_FROZEN_CT2');
assert.equal(sha(r.fixture),r.fixture_sha256);assert.equal(sha(r.export_report),r.export_report_sha256);
for(const p of [...r.source_hashes,...r.source_pins,...e.source_hashes,...e.source_pins])assert.equal(sha(p.path),p.sha256,p.path);
for(const p of r.source_hashes)assert.equal(sha(`experiments/g8b_raw/provenance/${p.path.replaceAll('/','__')}.executed.txt`),p.sha256);
assert.deepEqual(f.public_context,g6.public_context);assert.deepEqual(f.public_context,a.public_context);
assert.equal(f.records.length,5);assert.equal(e.public_signature_count_native_verified,5);assert.equal(e.verification_share_polynomial_checks,5);
assert.equal(r.q_cert,r.n-r.t);assert.equal(r.tau,r.t+1);
const be=(v,n)=>Buffer.from(BigInt(v).toString(16).padStart(n*2,'0'),'hex');
const blob=v=>Buffer.concat([be(v.length,8),v]);
const text=s=>blob(Buffer.from(s)),prefix=s=>Buffer.concat([text('VV-CANONICAL-V1'),text(s)]);
const c=f.public_context,suite='VERIVOTING-JUBJUB-PROTOTYPE-G0.8-v2';
for(const [i,row]of f.records.entries()){
 const old=g6.trustee_roster[i],sig=row.signature;
 assert.equal(row.trustee_id,old.trustee_id);assert.equal(row.authentication_key.encoded_hex,old.authentication_public_key_hex);
 assert.equal(row.verification_share.encoded_hex,old.verification_share_hex);assert.equal(row.commitment.encoded_hex,old.package_commitment_hex);
 const wire=bytes(sig.signature_hex);assert.equal(wire.toString('hex'),old.commitment_signature_hex);assert.equal(wire.length,66);
 assert.equal(wire.subarray(0,32).toString('hex'),sig.nonce.encoded_hex);
 assert.equal(BigInt(hex(Buffer.from(wire.subarray(32,64)).reverse().toString('hex'))).toString(),sig.response_scalar);assert.equal(wire.readUInt16BE(64),sig.counter);
 const original=Buffer.concat([prefix('VV/JUBJUB/PRIVATE-PACKAGE/COMMIT/v5'),text(suite),
  ...['election_id_hex','policy_pre_hex','final_context_hex','registration_close_digest_hex','disclosure_digest_hex','cp_target_hex'].map(k=>bytes(c[k])),
  be(row.trustee_id,4),bytes(row.commitment.encoded_hex)]);
 assert.equal(hash(original),row.original_message_hex,'independent exact v5 preimage');
 assert.equal(r.original_challenge_checks[i].challenge,row.original_challenge_scalar);
 assert.equal(r.original_challenge_checks[i].counter,sig.counter);
}
const art=read(r.contract_artifact);assert.equal(sha(r.contract_artifact),r.contract_artifact_sha256);
const abi=new Interface(art.contracts.trustee_commitments.abi),receiverAbi=new Interface(art.contracts.receiver.abi);
const byLabel=Object.fromEntries(r.transactions.map(t=>[t.label,t]));
assert.equal(r.transactions.length,25);assert.equal(r.transaction_count,25);
assert.deepEqual(r.transactions.map(t=>t.nonce),Array.from({length:25},(_,i)=>i));
for(const t of r.transactions){
 const signed=Transaction.from(t.signed_transaction_hex);assert.ok(signed.isSigned());assert.equal(signed.nonce,t.nonce);assert.equal(signed.hash,t.transaction_hash);
 assert.equal(signed.data,t.calldata_hex);assert.equal(signed.to?.toLowerCase()??null,t.to);assert.equal(signed.chainId,1n);assert.equal(signed.gasLimit,20_000_000n);
 assert.equal(bytes(t.calldata_hex).length,t.calldata_bytes);assert.equal(hash(bytes(t.calldata_hex)),t.calldata_sha256);
 assert.equal(t.total_gas,Math.max(t.intrinsic_gas+t.execution_gas,t.minimum_gas_limit_including_calldata_floor));assert.ok(t.total_gas<20_000_000);
}
assert.equal(new Set(r.transactions.map(t=>Transaction.from(t.signed_transaction_hex).from)).size,1,'same public local research transaction sender');
// The component really points at these same-run original contracts, not fake
// success flags or a different unfrozen fixture injected as a receiver.
const deployment=byLabel.deploy_commitment_component,creation=art.creation_bytecodes.trustee_commitments;
assert.equal(deployment.calldata_hex.slice(0,creation.hex.length),creation.hex);
const ctor=abi.fragments.find(x=>x.type==='constructor');
const args=AbiCoder.defaultAbiCoder().decode(ctor.inputs,hex(deployment.calldata_hex.slice(creation.hex.length)));
assert.equal(args[0].toLowerCase(),art.contracts.certificate_gate.address);assert.equal(args[1].toLowerCase(),art.contracts.receiver.address);
assert.equal(args[2].keyCoreDigest,hex(c.key_core_digest_hex));
for(let i=0;i<5;i++)assert.equal(args[5][i].encoded,hex(a.authentication_keys[i].encoded_hex));
for(let i=0;i<3;i++)assert.equal(args[6][i].encoded,hex(a.public_coefficients[i].encoded_hex));
for(const code of Object.values(art.contracts)){
 assert.equal(hash(bytes(code.runtime_bytecode_hex)),code.runtime_sha256);assert.equal(keccak256(code.runtime_bytecode_hex),code.runtime_keccak256);
 assert.ok(code.runtime_bytecode_bytes>0&&code.runtime_bytecode_bytes<=24576);
}
function checkTrace(msm,pairing,vk,proof,inputs){
 const k=bytes(vk),p=bytes(proof),scalars=[hex('0'.repeat(63)+'1'),...inputs];
 const wire=Buffer.concat(scalars.flatMap((x,i)=>[k.subarray(896+128*i,896+128*(i+1)),bytes(x)]));
 assert.equal(hash(wire),msm.input_sha256);assert.equal(wire.length,msm.input_bytes);
 const pairs=Buffer.concat([p.subarray(0,384),k.subarray(0,384),bytes(msm.return_hex),k.subarray(384,640),p.subarray(384),k.subarray(640,896)]);
 assert.equal(hash(pairs),pairing.input_sha256);assert.ok(msm.success&&pairing.success);assert.equal(BigInt(pairing.return_hex),1n);
}
for(let i=0;i<2;i++){
 const t=byLabel[`accept_batch_${i+1}`];assert.equal(t.receipt_status,1);
 const decoded=receiverAbi.decodeFunctionData('submitBatch',t.calldata_hex)[0],batch=b.batches[i];
 assert.equal(decoded.root,hex(batch.root_hex));assert.equal(Number(decoded.index),i+1);
 const curve=t.precompile_calls.filter(p=>['000c','000f'].includes(p.address.slice(-4)));assert.equal(curve.length,10);
 for(let j=0;j<5;j++)checkTrace(curve[2*j],curve[2*j+1],j<4?b.ballot_vk_eip2537_hex:b.state_vk_eip2537_hex,
  j<4?batch.ballot_proofs_eip2537_hex[j]:batch.state_proof_eip2537_hex,j<4?batch.ballot_inputs_be_hex[j]:batch.state_inputs_be_hex);
}
assert.equal(byLabel.freeze_ct2.receipt_status,1);assert.equal(byLabel.bind_authenticated_frozen_ct2.receipt_status,1);
const final=r.final_checkpoint;assert.equal(final.freeze_seal,hex(g6.freeze_digest_hex));assert.equal(final.cp_target,hex(c.cp_target_hex));
assert.equal(final.receiver_seal,final.freeze_seal);assert.equal(final.receiver_target,final.cp_target);assert.equal(final.receiver_spent,8);assert.equal(final.receiver_next,3);
assert.deepEqual(final.receiver_state,c.frozen_ciphertexts.flatMap(p=>[hex(p.c0_hex),hex(p.c1_hex)]));
assert.ok(final.bound&&final.gate_authorized&&final.receiver_frozen&&final.p3_ready);assert.equal(final.count,3);assert.deepEqual(final.submitted,[true,true,true,false,false]);
assert.equal(r.accepted_commitments.length,3);
for(const [i,accepted]of r.accepted_commitments.entries()){
 const t=r.transactions[accepted.transaction_nonce],row=f.records[i],s=abi.decodeFunctionData('submitCommitment',t.calldata_hex)[0];
 assert.equal(t.receipt_status,1);assert.equal(Number(s.trusteeId),i+1);assert.equal(s.electionId,hex(c.election_id_hex));assert.equal(s.finalContext,hex(c.final_context_hex));assert.equal(s.target,hex(c.cp_target_hex));
 assert.equal(s.verificationShare.encoded,hex(row.verification_share.encoded_hex));assert.equal(s.commitment.encoded,hex(row.commitment.encoded_hex));
 assert.equal(s.signature.nonce.encoded,hex(row.signature.nonce.encoded_hex));assert.equal(s.signature.response.toString(),row.signature.response_scalar);assert.equal(Number(s.signature.counter),row.signature.counter);
 assert.equal(final.commitments[i],s.commitment.encoded);assert.equal(final.verification_shares[i],s.verificationShare.encoded);
 assert.ok(t.precompile_calls.some(p=>p.address.endsWith('0002')&&p.return_hex===hex(row.original_message_hex)));
 const event=abi.parseLog(t.logs.at(-1));assert.equal(event.name,'CommitmentAuthenticated');assert.equal(Number(event.args.trusteeId),i+1);assert.equal(event.args.target,s.target);
 assert.equal(accepted.checkpoint.count,i+1);assert.equal(accepted.checkpoint.p3_ready,i===2);
}
const reasons={unauthenticated_context_cannot_bind:'UNCERTIFIED',unfrozen_receiver_cannot_bind:'NOT_FROZEN',
 forged_package_signature:'BAD_PACKAGE_SIGNATURE',changed_package_commitment:'BAD_PACKAGE_SIGNATURE',changed_cp_target:'CP_TARGET',changed_final_context:'FINAL_CONTEXT',
 changed_election_id:'ELECTION_ID',substituted_valid_verification_share:'VERIFICATION_SHARE',duplicate_trustee_id:'DUPLICATE_TRUSTEE',two_trustees_cannot_enter_p3:'DECRYPTION_QUORUM'};
assert.equal(r.negative_tests.length,10);
for(const n of r.negative_tests){const t=r.transactions[n.transaction_nonce];assert.equal(t.receipt_status,0);assert.equal(t.exception,'revert');assert.equal(t.revert_reason,reasons[n.case]);assert.ok(n.atomic_rejection);assert.deepEqual(n.before,n.after);assert.equal(t.logs.length,0);}
const forged=abi.decodeFunctionData('submitCommitment',byLabel.forged_package_signature.calldata_hex)[0];
assert.equal(forged.signature.response,BigInt(f.records[0].signature.response_scalar)+1n);
assert.ok(forged.signature.response<6554484396890773809930967563523245729705921265872317281365359162392183254199n);
assert.equal(abi.decodeFunctionData('submitCommitment',byLabel.substituted_valid_verification_share.calldata_hex)[0].verificationShare.encoded,hex(f.records[1].verification_share.encoded_hex),'negative uses another valid subgroup point, not malformed encoding');
assert.equal(byLabel.three_trustees_ready_for_future_p3.receipt_status,1);
for(const flag of ['signature_scheme_replaced','dkg_reexecuted','receiver_auth_gate_enforced_on_intake','p3_storage_binding_integrated','p3_verified_in_this_flow','winner_written','complete_g8_settlement','snarkpack_evm_verified','private_witness_emitted'])assert.equal(r[flag],false);
for(const field of ['setup_count','proof_generation_count','new_signatures','new_cp_packages']){assert.equal(r[field],0);assert.equal(f[field],0);}
assert.equal(r.full_system_gas,null);assert.equal(r.original_package_signatures_verified_evm,3);assert.equal(r.verification_share_polynomial_checks_evm,3);
let archives=0;
for(const gate of ['g1_e','g2_d','g2_e','g4_d','g6','g7','g8','g8a'])for(const line of fs.readFileSync(`experiments/${gate}_raw/SHA256SUMS`,'utf8').trim().split('\n')){
 const[,h,p]=line.match(/^([0-9a-f]{64})\s+(.+)$/)??[];assert.ok(p);
 if(!/^(experiments|fixtures|results)\//.test(p)||p.startsWith('experiments/aggregate/'))continue;
 assert.equal(sha(p),h,p);archives++;
}
const commands=read('experiments/g8b_raw/commands.json').commands,latest=Object.fromEntries(commands.map(x=>[x.name,x]));
for(const name of ['build','public-export','evm','fmt-check','clippy','tests','state-tests','g8a-history-audit'])assert.equal(latest[name].exit_code,0,name);
const failedAttempts=commands.filter(x=>x.exit_code!==0);
for(const entry of failedAttempts)assert.ok(fs.existsSync(`experiments/g8b_raw/${entry.log}`),'retain unsuccessful attempt logs');
console.log(JSON.stringify({status:'PASS_G8B_ORIGINAL_COMMITMENT_AUTHENTICATION_AND_PROVENANCE_AUDIT',historical_artifact_entries_unchanged:archives,
 actual_signed_transactions:25,original_package_signatures_evm:3,authenticated_polynomial_share_checks_evm:3,native_archived_record_checks:5,
 atomic_negatives:10,failed_command_attempts_retained:failedAttempts.length,receiver_enforces_gate_on_intake:false,full_g8:false}));
