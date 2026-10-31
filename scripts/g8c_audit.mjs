// Exact-byte/ABI/source/provenance audit; no new proving or benchmark execution.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {createRequire} from 'node:module';
const {Interface,Transaction,keccak256,AbiCoder}=createRequire(new URL('../contracts/package.json',import.meta.url))('ethers');
const read=p=>JSON.parse(fs.readFileSync(p)),text=p=>fs.readFileSync(p,'utf8'),hash=b=>createHash('sha256').update(b).digest('hex'),sha=p=>hash(fs.readFileSync(p));
const bytes=h=>Buffer.from(h.replace(/^0x/,''),'hex'),hex=s=>`0x${s}`;
const r=read('experiments/g8c_raw/frozen_p3_binding.json'),config=read('configs/experiment.G8C-FROZEN-P3-W32.json');
const g7=read(config.source_pins[0].path),b=read(config.source_pins[1].path),a=read(config.source_pins[2].path),f=read(config.source_pins[3].path),g6=read(config.source_pins[4].path),art=read(r.contract_artifact);
assert.equal(r.status,'PASS_AUTHENTICATED_INTAKE_AND_FROZEN_STORAGE_P3_BINDING');
for(const p of [...r.source_pins,...r.source_hashes])assert.equal(sha(p.path),p.sha256,p.path);
for(const p of r.source_hashes)assert.equal(sha(`experiments/g8c_raw/provenance/${p.path.replaceAll('/','__')}.executed.txt`),p.sha256);
assert.equal(sha(r.contract_artifact),r.contract_artifact_sha256);assert.equal(sha(r.g7_native_export_report),r.g7_native_export_report_sha256);
assert.equal(hash(bytes(g6.verifying_key_hex)),config.p3_ark_vk_sha256);assert.equal(hash(bytes(g7.vk_eip2537_hex)),config.p3_eip_vk_sha256);assert.equal(keccak256(g7.vk_eip2537_hex),config.p3_eip_vk_keccak256);
assert.equal(hash(bytes(g6.proof_hex)),g7.proof_ark_sha256);assert.equal(hash(bytes(g6.proof_hex)),r.ark_proof_sha256);assert.equal(hash(bytes(g7.proof_neg_a_eip2537_hex)),r.eip_proof_sha256);
assert.equal(bytes(g6.proof_hex).length,192);assert.equal(bytes(g7.proof_neg_a_eip2537_hex).length,512);
assert.equal(bytes(g7.wrong_vk_eip2537_hex).length,bytes(g7.vk_eip2537_hex).length,'negative VK has the same public-input width');
const gateAbi=new Interface(art.contracts.certificate_gate_v2.abi),receiverAbi=new Interface(art.contracts.authenticated_receiver.abi),storeAbi=new Interface(art.contracts.original_commitment_store.abi),p3Abi=new Interface(art.contracts.frozen_p3_binder.abi);
const by=Object.fromEntries(r.transactions.map(t=>[t.label,t]));assert.equal(r.transaction_count,28);assert.equal(r.transactions.length,28);
assert.deepEqual(r.transactions.map(t=>t.nonce),Array.from({length:28},(_,i)=>i));
for(const t of r.transactions){const s=Transaction.from(t.signed_transaction_hex);assert.ok(s.isSigned());assert.equal(s.hash,t.transaction_hash);assert.equal(s.nonce,t.nonce);assert.equal(s.data,t.calldata_hex);assert.equal(s.to?.toLowerCase()??null,t.to);assert.equal(s.chainId,1n);assert.equal(s.gasLimit,20_000_000n);
 assert.equal(bytes(t.calldata_hex).length,t.calldata_bytes);assert.equal(hash(bytes(t.calldata_hex)),t.calldata_sha256);assert.equal(t.gas_refund,0);assert.equal(t.total_gas,Math.max(t.intrinsic_gas+t.execution_gas,t.minimum_gas_limit_including_calldata_floor));assert.ok(t.total_gas<20_000_000);}
for(const[name,c]of Object.entries(art.contracts)){assert.equal(c.address,r.addresses[name]);assert.equal(hash(bytes(c.runtime_bytecode_hex)),c.runtime_sha256);assert.equal(keccak256(c.runtime_bytecode_hex),c.runtime_keccak256);assert.equal(bytes(c.runtime_bytecode_hex).length,c.runtime_bytecode_bytes);assert.ok(c.runtime_bytecode_bytes<=24576);}
function constructorArgs(name,label,iface){const c=art.contracts[name],t=by[label];assert.equal(t.calldata_hex.slice(0,c.creation_bytecode_hex.length),c.creation_bytecode_hex);return AbiCoder.defaultAbiCoder().decode(iface.fragments.find(x=>x.type==='constructor').inputs,hex(t.calldata_hex.slice(c.creation_bytecode_hex.length)));}
const receiverArgs=constructorArgs('authenticated_receiver','deploy_authenticated_receiver',receiverAbi),storeArgs=constructorArgs('original_commitment_store','deploy_original_commitment_store',storeAbi),p3Args=constructorArgs('frozen_p3_binder','deploy_frozen_p3_binder',p3Abi);
assert.equal(receiverArgs[0].toLowerCase(),r.addresses.certificate_gate_v2);assert.equal(storeArgs[0].toLowerCase(),r.addresses.certificate_gate_v2);assert.equal(storeArgs[1].toLowerCase(),r.addresses.authenticated_receiver);
assert.equal(p3Args[0].toLowerCase(),r.addresses.certificate_gate_v2);assert.equal(p3Args[1].toLowerCase(),r.addresses.authenticated_receiver);assert.equal(p3Args[2].toLowerCase(),r.addresses.original_commitment_store);
assert.equal(p3Args[3],g7.vk_eip2537_hex);assert.equal(keccak256(AbiCoder.defaultAbiCoder().encode([receiverAbi.fragments.find(x=>x.type==='constructor').inputs[1]],[receiverArgs[1]])),config.binding_digest);
assert.deepEqual([...receiverArgs[2]],config.accepted_roots);assert.equal(hash(bytes(receiverArgs[3])),config.ballot_eip_vk_sha256);assert.equal(hash(bytes(receiverArgs[4])),config.state_eip_vk_sha256);
// Source-level check: the actual receiver's batch/freeze bodies are EXACTLY
// the old bodies plus the mandatory in-function certificate check. No facade.
function fn(src,head){const start=src.indexOf(head);assert.ok(start>=0,head);const brace=src.indexOf('{',start);let depth=1,i=brace+1;for(;depth;i++){if(src[i]==='{')depth++;if(src[i]==='}')depth--;}return src.slice(brace,i);}
const oldReceiver=text('contracts/src/VeriVotingSettlement.sol'),newReceiver=text('contracts/src/VeriVotingAuthenticatedSettlement.sol');
for(const head of ['function submitBatch(','function freeze('])assert.equal(fn(newReceiver,head).replace('\n        _requireCertifiedBinding();',''),fn(oldReceiver,head));
for(const head of ['function _checkRecord(','function _batchRoot(','function _stateInputs(','function _pack32(','function _unpack32('])assert.equal(fn(newReceiver,head),fn(oldReceiver,head));
const oldGate=text('contracts/src/auth/VeriVotingCertificateGate.sol'),newGate=text('contracts/src/auth/VeriVotingCertificateGateV2.sol');
assert.equal(fn(newGate,'constructor(').replace('\n        publicCoefficients = coefficients;',''),fn(oldGate,'constructor('));
assert.equal(fn(newGate,'function submitDkgSignature('),fn(oldGate,'function submitDkgSignature('));
let reg=fn(newGate,'function authenticateRegistration(');for(const line of ['certifiedWRegistered = c.wRegistered;','certifiedNRegistered = c.nRegistered;','certifiedWMax = c.wMax;','certifiedCertificateSha256 = sha256(certificate);'])reg=reg.replace(`\n        ${line}`,'');
assert.equal(reg,fn(oldGate,'function authenticateRegistration('));
assert.deepEqual(p3Abi.getFunction('verifyFrozenP3').inputs.map(i=>i.type),['bytes','uint8','uint64']);assert.equal(p3Abi.getFunction('verifyFrozenP3').stateMutability,'view');
assert.deepEqual(p3Abi.getFunction('buildP3Inputs').inputs.map(i=>i.type),['uint8','uint64']);
for(const c of [art.contracts.frozen_p3_binder,art.contracts.authenticated_receiver])assert.ok(!c.abi.some(e=>e.type==='function'&&/winner|disclos/i.test(e.name)),'no Winner/disclosure endpoint this round');
const be=(v,n)=>Buffer.from(BigInt(v).toString(16).padStart(n*2,'0'),'hex'),blob=b=>Buffer.concat([be(b.length,8),b]),str=s=>blob(Buffer.from(s));
const prefix=d=>Buffer.concat([str('VV-CANONICAL-V1'),str(d)]),pack=b=>Array.from({length:Math.ceil(b.length/31)},(_,i)=>hex(Buffer.from(b.subarray(i*31,i*31+31)).reverse().toString('hex').padStart(64,'0')));
function expectedInputs(selector,winner){const table=[[1,2,3],[1,2,4],[1,2,5],[1,3,4],[1,3,5],[1,4,5],[2,3,4],[2,3,5],[2,4,5],[3,4,5]],c=g6.public_context,ct=c.frozen_ciphertexts,out=[];
 for(const id of table[selector]){const row=g6.trustee_roster[id-1];out.push(...pack(Buffer.concat([prefix('VV/JUBJUB/GENERALIZED-CP/TRANSCRIPT/v3'),str('VERIVOTING-JUBJUB-PROTOTYPE-G0.8-v2'),
  ...['election_id_hex','policy_pre_hex','final_context_hex','registration_close_digest_hex','disclosure_digest_hex','cp_target_hex'].map(k=>bytes(c[k])),be(id,4),bytes(row.verification_share_hex),be(3,8),...ct.map(p=>bytes(p.c0_hex)),be(3,8)])),...pack(bytes(row.package_commitment_hex)));}
 out.push(hex(be(selector,32).toString('hex')));for(const p of ct)out.push(...pack(bytes(p.c1_hex)));out.push(hex(be(32,32).toString('hex')),hex(be(winner,32).toString('hex')));return out;}
assert.equal(r.public_inputs_be_hex.length,60);assert.deepEqual(r.public_inputs_be_hex,expectedInputs(0,1));assert.deepEqual(r.public_inputs_be_hex,g7.public_inputs_be_hex);assert.deepEqual(r.public_inputs_be_hex,g6.public_inputs_le_hex.map(h=>hex(Buffer.from(h,'hex').reverse().toString('hex'))));
assert.deepEqual(r.public_input_source_map.map(m=>m.index),Array.from({length:60},(_,i)=>i));
for(const m of r.public_input_source_map){assert.equal(m.value_be_hex,r.public_inputs_be_hex[m.index]);assert.ok(m.sources.length>0);if(m.packed_little_endian_hex){assert.equal(hex(Buffer.from(m.packed_little_endian_hex,'hex').reverse().toString('hex').padStart(64,'0')),m.value_be_hex);assert.equal(m.sources.reduce((n,s)=>n+s.overlap_length,0),m.canonical_bytes_length);}}
assert.equal(r.public_input_source_map[51].component,'selector');assert.equal(r.public_input_source_map[58].component,'registered_weight');assert.equal(r.public_input_source_map[59].component,'winner');
function equation(msm,pairing,vk,proof,inputs,expected){const k=bytes(vk),p=bytes(proof),scalars=[hex(be(1,32).toString('hex')),...inputs];const m=Buffer.concat(scalars.flatMap((x,i)=>[k.subarray(896+128*i,896+128*(i+1)),bytes(x)]));assert.equal(hash(m),msm.input_sha256);assert.equal(m.length,msm.input_bytes);
 const pairs=Buffer.concat([p.subarray(0,384),k.subarray(0,384),bytes(msm.return_hex),k.subarray(384,640),p.subarray(384),k.subarray(640,896)]);assert.equal(hash(pairs),pairing.input_sha256);assert.ok(msm.success&&pairing.success);assert.equal(BigInt(pairing.return_hex),BigInt(expected));}
for(let i=0;i<2;i++){const t=by[`accept_batch_${i+1}`],batch=b.batches[i],curve=t.precompile_calls.filter(p=>['000c','000f'].includes(p.address.slice(-4)));assert.equal(t.receipt_status,1);assert.equal(curve.length,10);
 for(let j=0;j<5;j++)equation(curve[2*j],curve[2*j+1],j<4?b.ballot_vk_eip2537_hex:b.state_vk_eip2537_hex,j<4?batch.ballot_proofs_eip2537_hex[j]:batch.state_proof_eip2537_hex,j<4?batch.ballot_inputs_be_hex[j]:batch.state_inputs_be_hex,1);}
// Wrong newState runs BEFORE any accepted batch or freeze. Four authentic ballot
// equations pass, then the changed state statement reaches a real pairing 0.
const wrongState=by.changed_first_new_state_before_freeze,stateArgs=receiverAbi.decodeFunctionData('submitBatch',wrongState.calldata_hex)[0],badStateInputs=[...b.batches[0].state_inputs_be_hex];
assert.ok(wrongState.nonce<by.accept_batch_1.nonce&&wrongState.nonce<by.freeze_ct2.nonce);assert.equal(stateArgs.newState[1],hex(b.batches[1].new_state[0].c1_hex));
badStateInputs.splice(27,2,...pack(bytes(stateArgs.newState[1])));
const badCurve=wrongState.precompile_calls.filter(p=>['000c','000f'].includes(p.address.slice(-4)));assert.equal(badCurve.length,10);
for(let j=0;j<5;j++)equation(badCurve[2*j],badCurve[2*j+1],j<4?b.ballot_vk_eip2537_hex:b.state_vk_eip2537_hex,j<4?b.batches[0].ballot_proofs_eip2537_hex[j]:b.batches[0].state_proof_eip2537_hex,j<4?b.batches[0].ballot_inputs_be_hex[j]:badStateInputs,j<4?1:0);
for(const[label,selector,winner,expected]of[['verify_frozen_p3_original',0,1,1],['wrong_winner_pairing',0,2,0],['selector1_after_all_five_pairing',1,1,0]]){
 const t=by[label],args=p3Abi.decodeFunctionData('verifyFrozenP3',t.calldata_hex),curve=t.precompile_calls.filter(p=>['000c','000f'].includes(p.address.slice(-4)));
 assert.equal(args[0],g7.proof_neg_a_eip2537_hex);assert.equal(Number(args[1]),selector);assert.equal(Number(args[2]),winner);assert.equal(curve.length,2);
 equation(curve[0],curve[1],g7.vk_eip2537_hex,g7.proof_neg_a_eip2537_hex,expectedInputs(selector,winner),expected);}
const reasons={wrong_same_width_p3_vk:'PINNED_P3_VK',intake_before_certificate_authentication:'UNCERTIFIED',changed_first_new_state_before_freeze:'INVALID_GROTH16',p3_before_freeze:'NOT_FROZEN',receive_after_freeze:'FROZEN',selected_trustee3_missing:'MISSING_COMMITMENT',duplicate_trustee3:'DUPLICATE_TRUSTEE',wrong_winner_pairing:'INVALID_GROTH16',selector1_missing_trustee4:'MISSING_COMMITMENT',selector1_after_all_five_pairing:'INVALID_GROTH16'};
assert.equal(r.negative_tests.length,10);for(const n of r.negative_tests){const t=r.transactions[n.transaction_nonce];assert.equal(t.receipt_status,0);assert.equal(t.exception,'revert');assert.equal(t.revert_reason,reasons[n.case]);assert.equal(t.logs.length,0);assert.ok(n.atomic_rejection);assert.deepEqual(n.before,n.after);}
const beforeWrongState=r.negative_tests.find(n=>n.case==='changed_first_new_state_before_freeze').before;assert.equal(beforeWrongState.frozen,false);assert.equal(beforeWrongState.spent_count,0);assert.ok(beforeWrongState.spent_nullifiers.every(v=>!v));
assert.equal(r.negative_tests.find(n=>n.case==='selector1_missing_trustee4').pairing_count,0);assert.equal(r.negative_tests.find(n=>n.case==='selector1_after_all_five_pairing').pairing_count,1);
for(let id=1;id<=5;id++){const t=by[`authenticate_commitment_${id}`],s=storeAbi.decodeFunctionData('submitCommitment',t.calldata_hex)[0],row=f.records[id-1];assert.equal(t.receipt_status,1);assert.equal(s.commitment.encoded,hex(row.commitment.encoded_hex));assert.equal(s.verificationShare.encoded,hex(row.verification_share.encoded_hex));assert.equal(s.signature.nonce.encoded,hex(row.signature.nonce.encoded_hex));assert.equal(s.signature.response.toString(),row.signature.response_scalar);assert.equal(Number(s.signature.counter),row.signature.counter);}
const final=r.final_checkpoint;assert.ok(final.authorized&&final.frozen&&final.store_bound);assert.equal(final.freeze_seal,hex(g6.freeze_digest_hex));assert.equal(final.cp_target,hex(g6.public_context.cp_target_hex));assert.equal(final.store_seal,final.freeze_seal);assert.equal(final.store_target,final.cp_target);
assert.equal(final.certificate_sha256,hex(hash(bytes(g6.public_context.registration_certificate_hex))));assert.equal(final.certified_w_registered,32);assert.equal(final.certified_n_registered,32);assert.equal(final.certified_w_max,32);assert.equal(final.commitment_count,5);assert.deepEqual(final.spent_nullifiers,Array(8).fill(true));assert.deepEqual(final.submitted,Array(5).fill(true));
assert.equal(by.verify_frozen_p3_original.calldata_bytes,644);
for(const flag of ['receiver_auth_gate_enforced_on_intake','p3_storage_binding_integrated','p3_verified_in_this_flow','all_60_inputs_match'])assert.equal(r[flag],true);
for(const flag of ['winner_written','complete_g8_settlement','snarkpack_evm_verified','caller_public_input_array_accepted','caller_verifier_address_accepted','private_witness_emitted','production_setup'])assert.equal(r[flag],false);
for(const k of ['setup_count','proof_generation_count','new_cp_packages','new_signatures'])assert.equal(r[k],0);assert.equal(r.full_system_gas,null);
// Every byte tracked at the development base remains unchanged EXCEPT these
// explicitly maintained status/summary files. New files do not overwrite history.
let archives=0;
for(const gate of ['g1_e','g2_d','g2_e','g4_d','g6','g7','g8','g8a','g8b','g8b_five'])for(const line of text(`experiments/${gate}_raw/SHA256SUMS`).trim().split('\n')){const[,h,p]=line.match(/^([0-9a-f]{64})\s+(.+)$/)??[];assert.ok(p);if(!/^(experiments|fixtures|results)\//.test(p)||p.startsWith('experiments/aggregate/'))continue;assert.equal(sha(p),h,p);archives++;}
const commands=read('experiments/g8c_raw/commands.json').commands,latest=Object.fromEntries(commands.map(c=>[c.name,c]));assert.equal(latest.evm.exit_code,0);assert.equal(latest['baseline-audit'].exit_code,0);
const failed=commands.filter(c=>c.exit_code!==0);for(const c of failed)assert.ok(fs.existsSync(`experiments/g8c_raw/${c.log}`));
assert.ok(fs.existsSync('experiments/g8c_raw/provenance_failed/attempt1__g8c_frozen_p3.mjs.executed.txt'));
const baseFiles=new Set(execFileSync('git',['ls-tree','-r','--name-only',config.base_commit],{encoding:'utf8'}).trim().split('\n'));
const allowed=new Set(['README.md','docs/IMPLEMENTATION_STATUS.md','docs/NEXT_STEPS.md','scripts/summarize.py','experiments/aggregate/summary.json','experiments/aggregate/summary.md']);
for(const p of execFileSync('git',['diff','--name-only',config.base_commit,'--'],{encoding:'utf8'}).trim().split('\n').filter(Boolean))if(baseFiles.has(p))assert.ok(allowed.has(p),`unexpected historical tracked change: ${p}`);
console.log(JSON.stringify({status:'PASS_G8C_CERTIFIED_FROZEN_SOURCE_AND_ACTUAL_PAIRING_AUDIT',signed_transactions:28,public_inputs_matched:60,
 wrong_state_pre_freeze_pairings:5,missing_selector_pairings:0,wrong_selector_with_all_commitments_pairings:1,necessary_atomic_negatives:10,
 unchanged_historical_archive_entries:archives,failed_command_attempts_retained:failed.length,valid_p3_gas:by.verify_frozen_p3_original.total_gas,valid_p3_calldata_bytes:644,winner_written:false,full_g8:false}));
