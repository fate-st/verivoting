// G8-D actual-byte/ABI/statement/atomicity audit; no setup, proving or benchmark.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {createRequire} from 'node:module';
const {Interface,Transaction,keccak256,AbiCoder,getCreateAddress}=createRequire(new URL('../contracts/package.json',import.meta.url))('ethers');
const read=p=>JSON.parse(fs.readFileSync(p)),text=p=>fs.readFileSync(p,'utf8'),hash=b=>createHash('sha256').update(b).digest('hex'),sha=p=>hash(fs.readFileSync(p));
const bytes=h=>Buffer.from(h.replace(/^0x/,''),'hex'),hex=s=>`0x${s}`;
const r=read('experiments/g8d_raw/winner_once.json'),config=read('configs/experiment.G8C-FROZEN-P3-W32.json');
const g7=read(config.source_pins[0].path),b=read(config.source_pins[1].path),a=read(config.source_pins[2].path),f=read(config.source_pins[3].path),g6=read(config.source_pins[4].path),art=read(r.contract_artifact);
assert.equal(r.status,'PASS_FIXED_FROZEN_P3_ATOMIC_WINNER_WRITE_ONCE');assert.equal(r.gate,'G8-D');
assert.equal(r.baseline_commit,'2ce1caedf11a064c36f8322daf65e69b8522bf76');assert.equal(r.git_commit,r.baseline_commit);assert.ok(r.git_dirty);
for(const p of [...r.source_pins,...r.source_hashes])assert.equal(sha(p.path),p.sha256,p.path);
for(const p of r.source_hashes)assert.equal(sha(`experiments/g8d_raw/provenance/${p.path.replaceAll('/','__')}.executed.txt`),p.sha256);
assert.equal(sha(r.g8c_archive),r.g8c_archive_sha256);assert.equal(r.g8c_archive_sha256,'12a519caf8f6446c79f0ba6867f426621fba1455da566c4a75a3726d7d9cbcec');
assert.equal(sha(r.contract_artifact),r.contract_artifact_sha256);assert.equal(sha(r.g7_native_export_report),r.g7_native_export_report_sha256);
assert.equal(hash(bytes(g6.verifying_key_hex)),r.ark_vk_sha256);assert.equal(hash(bytes(g7.vk_eip2537_hex)),r.eip_vk_sha256);assert.equal(keccak256(g7.vk_eip2537_hex),r.eip_vk_keccak256);
assert.equal(hash(bytes(g6.proof_hex)),r.ark_proof_sha256);assert.equal(hash(bytes(g6.proof_hex)),g7.proof_ark_sha256);assert.equal(hash(bytes(g7.proof_neg_a_eip2537_hex)),r.eip_proof_sha256);
assert.equal(bytes(g6.proof_hex).length,192);assert.equal(bytes(g7.proof_neg_a_eip2537_hex).length,512);
const gateAbi=new Interface(art.contracts.certificate_gate_v2.abi),receiverAbi=new Interface(art.contracts.winner_receiver.abi),storeAbi=new Interface(art.contracts.original_commitment_store.abi),p3Abi=new Interface(art.contracts.frozen_p3_binder.abi);
const by=Object.fromEntries(r.transactions.map(t=>[t.label,t]));assert.equal(r.transaction_count,21);assert.equal(r.transactions.length,21);
assert.deepEqual(r.transactions, text('experiments/g8d_raw/transactions.jsonl').trim().split('\n').map(s=>JSON.parse(s)));
assert.deepEqual(r.transactions.map(t=>t.nonce),Array.from({length:21},(_,i)=>i));
for(const t of r.transactions){const s=Transaction.from(t.signed_transaction_hex);assert.ok(s.isSigned());assert.equal(s.hash,t.transaction_hash);assert.equal(s.nonce,t.nonce);assert.equal(s.data,t.calldata_hex);assert.equal(s.to?.toLowerCase()??null,t.to);assert.equal(s.chainId,1n);assert.equal(s.gasLimit,20_000_000n);
 assert.equal(bytes(t.calldata_hex).length,t.calldata_bytes);assert.equal(hash(bytes(t.calldata_hex)),t.calldata_sha256);assert.equal(t.gas_refund,0);assert.equal(t.total_gas,Math.max(t.intrinsic_gas+t.execution_gas,t.minimum_gas_limit_including_calldata_floor));assert.ok(t.total_gas<20_000_000);}
assert.equal(r.hardfork,'prague');assert.equal(r.compiler_settings.evmVersion,'prague');assert.equal(r.compiler_settings.optimizer.runs,200);assert.equal(r.compiler_settings.viaIR,true);
for(const[name,c]of Object.entries(art.contracts)){assert.equal(c.address,r.addresses[name]);assert.equal(hash(bytes(c.runtime_bytecode_hex)),c.runtime_sha256);assert.equal(keccak256(c.runtime_bytecode_hex),c.runtime_keccak256);assert.equal(bytes(c.runtime_bytecode_hex).length,c.runtime_bytecode_bytes);assert.ok(c.runtime_bytecode_bytes<=24576);}
function constructorArgs(name,label,iface){const c=art.contracts[name],t=by[label];assert.equal(t.calldata_hex.slice(0,c.creation_bytecode_hex.length),c.creation_bytecode_hex);return AbiCoder.defaultAbiCoder().decode(iface.fragments.find(x=>x.type==='constructor').inputs,hex(t.calldata_hex.slice(c.creation_bytecode_hex.length)));}
const receiverArgs=constructorArgs('winner_receiver','deploy_winner_receiver',receiverAbi),storeArgs=constructorArgs('original_commitment_store','deploy_original_commitment_store',storeAbi),p3Args=constructorArgs('frozen_p3_binder','deploy_frozen_p3_binder',p3Abi);
assert.equal(receiverArgs[0].toLowerCase(),r.addresses.certificate_gate_v2);assert.equal(storeArgs[0].toLowerCase(),r.addresses.certificate_gate_v2);assert.equal(storeArgs[1].toLowerCase(),r.addresses.winner_receiver);
assert.equal(p3Args[0].toLowerCase(),r.addresses.certificate_gate_v2);assert.equal(p3Args[1].toLowerCase(),r.addresses.winner_receiver);assert.equal(p3Args[2].toLowerCase(),r.addresses.original_commitment_store);assert.equal(p3Args[3],g7.vk_eip2537_hex);
assert.equal(receiverArgs[5].toLowerCase(),r.addresses.frozen_p3_binder);assert.equal(receiverArgs[5],getCreateAddress({from:Transaction.from(by.deploy_frozen_p3_binder.signed_transaction_hex).from,nonce:3}));
assert.equal(keccak256(AbiCoder.defaultAbiCoder().encode([receiverAbi.fragments.find(x=>x.type==='constructor').inputs[1]],[receiverArgs[1]])),config.binding_digest);
assert.deepEqual([...receiverArgs[2]],config.accepted_roots);assert.equal(hash(bytes(receiverArgs[3])),config.ballot_eip_vk_sha256);assert.equal(hash(bytes(receiverArgs[4])),config.state_eip_vk_sha256);
assert.deepEqual(receiverAbi.getFunction('discloseWinner').inputs.map(i=>i.type),['bytes','uint8','uint64']);assert.deepEqual(receiverAbi.getFunction('bindFrozenP3').inputs,[]);
const source=text('contracts/src/VeriVotingWinnerOnlySettlement.sol');assert.match(source,/is VeriVotingAuthenticatedSettlement/);assert.match(source,/public immutable frozenP3Verifier/);
assert.equal((source.match(/winner = proposedWinner/g)||[]).length,1);assert.ok(source.indexOf('require(!winnerDisclosed')<source.indexOf('frozenP3Verifier.verifyFrozenP3'));
assert.ok(source.indexOf('frozenP3Verifier.verifyFrozenP3')<source.indexOf('winner = proposedWinner'));assert.ok(!source.includes('function submitBatch(')&&!source.includes('function freeze('));
const be=(v,n)=>Buffer.from(BigInt(v).toString(16).padStart(n*2,'0'),'hex'),blob=b=>Buffer.concat([be(b.length,8),b]),str=s=>blob(Buffer.from(s));
const prefix=d=>Buffer.concat([str('VV-CANONICAL-V1'),str(d)]),pack=b=>Array.from({length:Math.ceil(b.length/31)},(_,i)=>hex(Buffer.from(b.subarray(i*31,i*31+31)).reverse().toString('hex').padStart(64,'0')));
function expectedInputs(selector,winner){const table=[[1,2,3],[1,2,4],[1,2,5],[1,3,4],[1,3,5],[1,4,5],[2,3,4],[2,3,5],[2,4,5],[3,4,5]],c=g6.public_context,ct=c.frozen_ciphertexts,out=[];
 for(const id of table[selector]){const row=g6.trustee_roster[id-1];out.push(...pack(Buffer.concat([prefix('VV/JUBJUB/GENERALIZED-CP/TRANSCRIPT/v3'),str('VERIVOTING-JUBJUB-PROTOTYPE-G0.8-v2'),
  ...['election_id_hex','policy_pre_hex','final_context_hex','registration_close_digest_hex','disclosure_digest_hex','cp_target_hex'].map(k=>bytes(c[k])),be(id,4),bytes(row.verification_share_hex),be(3,8),...ct.map(p=>bytes(p.c0_hex)),be(3,8)])),...pack(bytes(row.package_commitment_hex)));}
 out.push(hex(be(selector,32).toString('hex')));for(const p of ct)out.push(...pack(bytes(p.c1_hex)));out.push(hex(be(32,32).toString('hex')),hex(be(winner,32).toString('hex')));return out;}
function equation(msm,pairing,vk,proof,inputs,expected){const k=bytes(vk),p=bytes(proof),scalars=[hex(be(1,32).toString('hex')),...inputs];const m=Buffer.concat(scalars.flatMap((x,i)=>[k.subarray(896+128*i,896+128*(i+1)),bytes(x)]));assert.equal(hash(m),msm.input_sha256);assert.equal(m.length,msm.input_bytes);
 const pairs=Buffer.concat([p.subarray(0,384),k.subarray(0,384),bytes(msm.return_hex),k.subarray(384,640),p.subarray(384),k.subarray(640,896)]);assert.equal(hash(pairs),pairing.input_sha256);assert.ok(msm.success&&pairing.success);assert.equal(BigInt(pairing.return_hex),BigInt(expected));}

assert.equal(r.public_inputs_be_hex.length,60);assert.deepEqual(r.public_inputs_be_hex,expectedInputs(0,1));assert.deepEqual(r.public_inputs_be_hex,g7.public_inputs_be_hex);
assert.deepEqual(r.public_inputs_be_hex,g6.public_inputs_le_hex.map(h=>hex(Buffer.from(h,'hex').reverse().toString('hex'))));
assert.deepEqual(r.public_input_source_map.map(m=>m.index),Array.from({length:60},(_,i)=>i));
for(const m of r.public_input_source_map){assert.equal(m.value_be_hex,r.public_inputs_be_hex[m.index]);assert.ok(m.sources.length>0);if(m.packed_little_endian_hex){assert.equal(hex(Buffer.from(m.packed_little_endian_hex,'hex').reverse().toString('hex').padStart(64,'0')),m.value_be_hex);assert.equal(m.sources.reduce((n,s)=>n+s.overlap_length,0),m.canonical_bytes_length);}}
for(let i=0;i<2;i++){const t=by[`accept_batch_${i+1}`],batch=b.batches[i],curve=t.precompile_calls.filter(p=>['000c','000f'].includes(p.address.slice(-4)));assert.equal(t.receipt_status,1);assert.equal(curve.length,10);
 for(let j=0;j<5;j++)equation(curve[2*j],curve[2*j+1],j<4?b.ballot_vk_eip2537_hex:b.state_vk_eip2537_hex,j<4?batch.ballot_proofs_eip2537_hex[j]:batch.state_proof_eip2537_hex,j<4?batch.ballot_inputs_be_hex[j]:batch.state_inputs_be_hex,1);}
for(const[label,winner,expected]of[['wrong_winner_before_disclosure',2,0],['disclose_winner_once',1,1]]){
 const t=by[label],args=receiverAbi.decodeFunctionData('discloseWinner',t.calldata_hex),curve=t.precompile_calls.filter(p=>['000c','000f'].includes(p.address.slice(-4)));
 assert.equal(args[0],g7.proof_neg_a_eip2537_hex);assert.equal(Number(args[1]),0);assert.equal(Number(args[2]),winner);assert.equal(curve.length,2);
 equation(curve[0],curve[1],g7.vk_eip2537_hex,g7.proof_neg_a_eip2537_hex,expectedInputs(0,winner),expected);}
assert.ok(by.wrong_winner_before_disclosure.nonce<by.disclose_winner_once.nonce);assert.ok(by.disclose_before_freeze.nonce<by.freeze_ct2.nonce);
const reasons={repeat_p3_binding:'P3_ALREADY_BOUND',disclose_before_freeze:'NOT_FROZEN',wrong_winner_before_disclosure:'INVALID_GROTH16',repeat_winner_disclosure:'ALREADY_DISCLOSED'};
assert.equal(r.negative_tests.length,4);for(const n of r.negative_tests){const t=r.transactions[n.transaction_nonce];assert.equal(t.receipt_status,0);assert.equal(t.exception,'revert');assert.equal(t.revert_reason,reasons[n.case]);assert.equal(t.logs.length,0);assert.ok(n.atomic_rejection);assert.deepEqual(n.before,n.after);assert.equal(n.pairing_count,n.case==='wrong_winner_before_disclosure'?1:0);}
assert.equal(r.negative_tests.find(n=>n.case==='wrong_winner_before_disclosure').before.winner_disclosed,false);
assert.equal(r.negative_tests.find(n=>n.case==='wrong_winner_before_disclosure').before.winner,0);
const valid=by.disclose_winner_once;assert.equal(valid.receipt_status,1);assert.equal(valid.to,r.addresses.winner_receiver);assert.equal(valid.calldata_bytes,644);
assert.deepEqual(r.after_valid_disclosure,{...r.before_valid_disclosure,winner_disclosed:true,winner:1,winner_selector:0});assert.deepEqual(r.final_checkpoint,r.after_valid_disclosure);
const allEvents=r.transactions.flatMap(t=>t.logs.filter(l=>l.address===r.addresses.winner_receiver).map(l=>({nonce:t.nonce,event:receiverAbi.parseLog(l)})));
const winners=allEvents.filter(e=>e.event.name==='WinnerDisclosed'),binds=allEvents.filter(e=>e.event.name==='P3Bound');assert.equal(winners.length,1);assert.equal(binds.length,1);
assert.equal(winners[0].nonce,valid.nonce);assert.equal(winners[0].event.args.electionId,r.initial_binding.electionId);assert.equal(winners[0].event.args.freezeSeal,r.final_checkpoint.freeze_seal);assert.equal(Number(winners[0].event.args.winner),1);assert.equal(Number(winners[0].event.args.selector),0);
assert.equal(r.successful_winner_writes,1);assert.equal(r.winner_event_count,1);assert.equal(binds[0].event.args.verifier.toLowerCase(),r.addresses.frozen_p3_binder);
for(let id=1;id<=3;id++){const t=by[`authenticate_commitment_${id}`],s=storeAbi.decodeFunctionData('submitCommitment',t.calldata_hex)[0],row=f.records[id-1];assert.equal(t.receipt_status,1);assert.equal(s.commitment.encoded,hex(row.commitment.encoded_hex));assert.equal(s.verificationShare.encoded,hex(row.verification_share.encoded_hex));assert.equal(s.signature.nonce.encoded,hex(row.signature.nonce.encoded_hex));assert.equal(s.signature.response.toString(),row.signature.response_scalar);assert.equal(Number(s.signature.counter),row.signature.counter);}
const final=r.final_checkpoint;assert.ok(final.authorized&&final.frozen&&final.store_bound&&final.p3_bound&&final.winner_disclosed);assert.equal(final.winner,1);assert.equal(final.freeze_seal,hex(g6.freeze_digest_hex));assert.equal(final.cp_target,hex(g6.public_context.cp_target_hex));assert.equal(final.store_seal,final.freeze_seal);assert.equal(final.store_target,final.cp_target);
assert.equal(final.commitment_count,3);assert.deepEqual(final.submitted,[true,true,true,false,false]);assert.deepEqual(final.spent_nullifiers,Array(8).fill(true));assert.equal(final.fixed_p3,r.addresses.frozen_p3_binder);
for(const flag of ['receiver_auth_gate_enforced_on_intake','p3_storage_binding_integrated','p3_verified_in_this_flow','all_60_inputs_match','winner_written','winner_write_once_integrated'])assert.equal(r[flag],true);
for(const flag of ['complete_g8_settlement','snarkpack_evm_verified','caller_public_input_array_accepted','caller_verifier_address_accepted','private_witness_emitted','production_setup'])assert.equal(r[flag],false);
for(const k of ['setup_count','proof_generation_count','new_cp_packages','new_signatures'])assert.equal(r[k],0);assert.equal(r.full_system_gas,null);
const allowed=new Set(['README.md','docs/IMPLEMENTATION_STATUS.md','docs/NEXT_STEPS.md','scripts/summarize.py','experiments/aggregate/summary.json','experiments/aggregate/summary.md']);
let historical=0;for(const line of execFileSync('git',['ls-tree','-r',r.baseline_commit],{encoding:'utf8'}).trim().split('\n')){
 const m=line.match(/^\d+ blob ([0-9a-f]+)\t(.+)$/);assert.ok(m);const[,object,p]=m;if(allowed.has(p))continue;
 assert.ok(fs.existsSync(p),p);assert.equal(execFileSync('git',['hash-object','--',p],{encoding:'utf8'}).trim(),object,p);historical++;
}
let archives=0;for(const gate of ['g1_e','g2_d','g2_e','g4_d','g6','g7','g8','g8a','g8b','g8b_five','g8c'])for(const line of text(`experiments/${gate}_raw/SHA256SUMS`).trim().split('\n')){const[,h,p]=line.match(/^([0-9a-f]{64})\s+(.+)$/)??[];assert.ok(p);if(!/^(experiments|fixtures|results)\//.test(p)||p.startsWith('experiments/aggregate/'))continue;assert.equal(sha(p),h,p);archives++;}
// The failed packaging attempt is retained, not silently substituted for the valid report.
const prior=text('experiments/g8d_raw/transactions-attempt-1.jsonl').trim().split('\n').map(s=>JSON.parse(s));assert.equal(prior.length,21);
for(let i=0;i<21;i++)for(const key of ['transaction_hash','signed_transaction_hex','total_gas','receipt_status','return_hex'])assert.equal(prior[i][key],r.transactions[i][key]);
assert.ok(fs.existsSync('experiments/g8d_raw/attempt-sources/attempt-1/contracts__scripts__g8d_winner_once.mjs.txt'));
const cmds=read('experiments/g8d_raw/commands.json').commands;assert.equal(cmds.findLast(c=>c.name==='evm').exit_code,0);
for(const c of cmds){assert.ok(fs.existsSync(`experiments/g8d_raw/${c.log}`));}
console.log(JSON.stringify({status:'PASS_G8D_ACTUAL_PAIRINGS_AND_ATOMIC_WINNER_ONCE_AUDIT',signed_transactions:21,public_inputs_matched:60,
 functional_acceptance_items:5,necessary_atomic_rejections:4,actual_winner:1,successful_winner_writes:1,winner_events:1,
 wrong_winner_pairing:0,valid_winner_pairing:1,replay_pairing_calls:0,unchanged_baseline_files:historical,unchanged_archive_entries:archives,
 disclosure_gas:valid.total_gas,disclosure_calldata_bytes:valid.calldata_bytes,new_setup:0,new_proof:0,exact_snarkpack_evm:false}));
