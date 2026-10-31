// Minimal G8-B supplement. Replay the exact archived signed prefix to execute
// real deployments/certificates/batches/freeze, then test roster != certifier set.
// NO source/bytecode rewriting, direct contract-storage injection or new proving.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {Interface,keccak256} from 'ethers';
import {Common,Hardfork,Mainnet} from '@ethereumjs/common';
import {createBlock} from '@ethereumjs/block';
import {createLegacyTx,createTxFromRLP} from '@ethereumjs/tx';
import {createVM,runTx} from '@ethereumjs/vm';
import {createAccount,createAddressFromPrivateKey,createAddressFromString,hexToBytes,bytesToHex} from '@ethereumjs/util';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const read=p=>fs.readFileSync(path.resolve(root,p)),load=p=>JSON.parse(read(p));
const sha=b=>createHash('sha256').update(b).digest('hex'),hex=s=>`0x${s}`;
const raw=process.argv[2]??'experiments/g8b_five_raw',output=`${raw}/all_trustees.json`;
assert.ok(!fs.existsSync(path.resolve(root,output)),'new output required');
const configPath='configs/experiment.G8B-FIVE-TRUSTEES.json',config=load(configPath);
for(const p of config.source_pins)assert.equal(sha(read(p.path)),p.sha256,p.path);
const original=load(config.source_pins[0].path),art=load(config.source_pins[1].path),fixture=load(config.source_pins[2].path);
for(const p of [...original.source_hashes,...original.source_pins])assert.equal(sha(read(p.path)),p.sha256,p.path);
const store=createAddressFromString(art.contracts.trustee_commitments.address),gate=createAddressFromString(art.contracts.certificate_gate.address),receiver=createAddressFromString(art.contracts.receiver.address);
const abi=new Interface(art.contracts.trustee_commitments.abi),gateAbi=new Interface(art.contracts.certificate_gate.abi),receiverAbi=new Interface(art.contracts.receiver.abi);
const common=new Common({chain:Mainnet,hardfork:Hardfork.Prague}),vm=await createVM({common});
// Same EXPLICITLY PUBLIC local research sender. No RPC/public-chain transaction.
const senderKey=hexToBytes(`0x${'3b'.repeat(32)}`),sender=createAddressFromPrivateKey(senderKey);
await vm.stateManager.putAccount(sender,createAccount({nonce:0n,balance:10n**25n}));
const block=createBlock({header:{number:1n,gasLimit:BigInt(config.block_gas_limit),baseFeePerGas:1n,timestamp:1_750_000_000n}},{common,skipConsensusFormatValidation:true});
let nonce=0n,trace=[];const stack=[],transactions=[];
vm.evm.events.on('beforeMessage',m=>stack.push({address:m.to?.toString(),input_bytes:m.data.length,input_sha256:sha(m.data)}));
vm.evm.events.on('afterMessage',r=>{const m=stack.pop();if(m&&['0002','000c','000f'].includes(m.address?.slice(-4)))trace.push({...m,
 success:!r.execResult.exceptionError,execution_gas:Number(r.execResult.executionGasUsed),return_hex:bytesToHex(r.execResult.returnValue)});});
async function execute(signed,label,kind){
 assert.equal(signed.nonce,nonce++);assert.equal(signed.getSenderAddress().toString(),sender.toString());
 trace=[];const start=performance.now(),r=await runTx(vm,{tx:signed,block});
 const intrinsic=Number(signed.getIntrinsicGas()),execution=Number(r.execResult.executionGasUsed),minimum=Number(signed.getMinimumGasLimit());
 assert.equal(stack.length,0);assert.equal(Number(r.gasRefund),0);assert.equal(Number(r.totalGasSpent),Math.max(intrinsic+execution,minimum));
 const measurement={label,kind,nonce:Number(signed.nonce),transaction_hash:bytesToHex(signed.hash()),signed_transaction_hex:bytesToHex(signed.serialize()),
  to:signed.to?.toString()??null,receipt_status:r.receipt.status,total_gas:Number(r.totalGasSpent),intrinsic_gas:intrinsic,execution_gas:execution,
  minimum_gas_limit_including_calldata_floor:minimum,gas_refund:Number(r.gasRefund),transaction_gas_limit:Number(signed.gasLimit),
  calldata_hex:bytesToHex(signed.data),calldata_bytes:signed.data.length,calldata_sha256:sha(signed.data),return_hex:bytesToHex(r.execResult.returnValue),
  exception:r.execResult.exceptionError?.error??null,revert_reason:r.execResult.exceptionError?.error==='revert'?abi.parseError(bytesToHex(r.execResult.returnValue))?.args[0]??null:null,
  precompile_calls:[...trace],local_vm_wall_ms:performance.now()-start,
  logs:r.receipt.logs.map(([address,topics,data])=>({address:bytesToHex(address),topics:topics.map(bytesToHex),data:bytesToHex(data)}))};
 transactions.push(measurement);return measurement;
}
async function tx(label,data){
 const signed=createLegacyTx({nonce,gasLimit:BigInt(config.transaction_gas_limit),gasPrice:10n,value:0n,to:store,data:hexToBytes(data)},{common}).sign(senderKey);
 return execute(signed,label,'supplement');
}
async function view(address,iface,name,args=[]){
 const before=(await vm.stateManager.getAccount(sender)).nonce;await vm.evm.journal.checkpoint();
 let r;try{r=await vm.evm.runCall({caller:sender,to:address,data:hexToBytes(iface.encodeFunctionData(name,args)),gasLimit:20_000_000n,isStatic:true});}
 finally{await vm.evm.journal.revert();}
 assert.equal((await vm.stateManager.getAccount(sender)).nonce,before);assert.equal(r.execResult.exceptionError,undefined,`view ${name}`);
 return iface.decodeFunctionResult(name,bytesToHex(r.execResult.returnValue));
}
console.log('G8-B five [1] execute exact archived signed prefix in a fresh Prague VM, not copy storage');
const prefix=original.transactions.slice(0,config.prefix_transactions_reexecuted);
for(const old of prefix){
 const actual=await execute(createTxFromRLP(hexToBytes(old.signed_transaction_hex),{common}),old.label,'reexecuted_exact_prefix');
 for(const k of ['transaction_hash','signed_transaction_hex','receipt_status','total_gas','execution_gas','intrinsic_gas','calldata_hex','return_hex','revert_reason'])assert.deepEqual(actual[k],old[k],`replay ${old.nonce} ${k}`);
 assert.deepEqual(actual.precompile_calls,old.precompile_calls,`actual EVM curve/hash trace ${old.nonce}`);
}
const runtimes=[];
for(const[name,old]of Object.entries(art.contracts)){
 const bytes=await vm.stateManager.getCode(createAddressFromString(old.address));assert.equal(bytesToHex(bytes),old.runtime_bytecode_hex);
 runtimes.push({name,address:old.address,runtime_sha256:sha(bytes),runtime_keccak256:keccak256(bytes),bytes:bytes.length,exact_g8b_runtime_match:true});
}
async function snapshot(){
 const s=await view(store,abi,'checkpoint'),g=await view(gate,gateAbi,'checkpoint'),r=await view(receiver,receiverAbi,'checkpoint');
 const certifiers=[],submitted=[],commitments=[],shares=[];
 for(let id=1;id<=5;id++){
  certifiers.push((await view(gate,gateAbi,'certifiedSigner',[id]))[0]);submitted.push((await view(store,abi,'submitted',[id]))[0]);
  commitments.push((await view(store,abi,'packageCommitment',[id]))[0]);shares.push((await view(store,abi,'verifiedShare',[id]))[0]);
 }
 return {bound:s[0],freeze_seal:s[1],cp_target:s[2],count:Number(s[3]),p3_ready:s[4],certifier_count:Number(g[0]),gate_authorized:g[6],certifier_mask:certifiers,
  submitted,commitments,verification_shares:shares,receiver_frozen:r[2],receiver_spent:Number(r[1]),receiver_seal:r[3],receiver_target:r[4],receiver_state:[...r[5]]};
}
const initial=await snapshot();assert.equal(initial.count,0);assert.ok(initial.bound&&initial.gate_authorized&&initial.receiver_frozen);
assert.deepEqual(initial.certifier_mask,[true,true,true,false,false]);assert.equal(initial.freeze_seal,hex(fixture.freeze_digest_hex));
assert.equal(initial.cp_target,hex(fixture.public_context.cp_target_hex));
const point=p=>({x:p.x,y:p.y,encoded:hex(p.encoded_hex)});
const submission=row=>({trusteeId:row.trustee_id,electionId:hex(fixture.public_context.election_id_hex),finalContext:hex(fixture.public_context.final_context_hex),
 target:hex(fixture.public_context.cp_target_hex),verificationShare:point(row.verification_share),commitment:point(row.commitment),
 signature:{nonce:point(row.signature.nonce),response:row.signature.response_scalar,counter:row.signature.counter}});
const submit=s=>abi.encodeFunctionData('submitCommitment',[s]),negatives=[];
async function reject(label,data,reason){
 const before=await snapshot(),t=await tx(label,data),after=await snapshot();
 assert.equal(t.receipt_status,0,label);assert.equal(t.exception,'revert');assert.equal(t.revert_reason,reason);assert.equal(t.logs.length,0);assert.deepEqual(after,before,`${label} atomic`);
 negatives.push({case:label,transaction_nonce:t.nonce,before,after,atomic_rejection:true});console.log(`  reject ${label}: ${reason}`);
}
console.log('G8-B five [2] authenticate non-certifiers first; certify {1,2,3} != decrypt-roster set');
const trustee4=fixture.records[3];let bad=submission(trustee4);bad.signature.response=(BigInt(bad.signature.response)+1n).toString();
await reject('forged_trustee4_signature',submit(bad),'BAD_PACKAGE_SIGNATURE');
bad=submission(trustee4);bad.target=hex((BigInt(bad.target)^1n).toString(16).padStart(64,'0'));await reject('changed_trustee4_cp_target',submit(bad),'CP_TARGET');
const accepted=[];let subsetCheckpoint;
for(const[idIndex,id]of config.commitment_submission_order.entries()){
 const row=fixture.records[id-1],t=await tx(`authenticate_commitment_${id}`,submit(submission(row)));assert.equal(t.receipt_status,1);
 const s=await snapshot();assert.equal(s.count,idIndex+1);assert.equal(s.p3_ready,idIndex>=2);assert.equal(s.certifier_count,3);assert.deepEqual(s.certifier_mask,initial.certifier_mask);
 assert.equal(s.commitments[id-1],hex(row.commitment.encoded_hex));assert.equal(s.verification_shares[id-1],hex(row.verification_share.encoded_hex));
 assert.ok(t.precompile_calls.some(p=>p.address.endsWith('0002')&&p.return_hex===hex(row.original_message_hex)));
 accepted.push({trustee_id:id,is_dkg_certifier:s.certifier_mask[id-1],transaction_nonce:t.nonce,checkpoint:s});
 console.log(`  Trustee ${id}: ${t.total_gas} Gas / ${t.calldata_bytes} B, DKG certifier=${s.certifier_mask[id-1]}`);
 if(idIndex===0)await reject('duplicate_trustee4',submit(submission(row)),'DUPLICATE_TRUSTEE');
 if(idIndex===1)await reject('only_noncertifiers_4_5_not_ready',abi.encodeFunctionData('requireP3Ready'),'DECRYPTION_QUORUM');
 if(idIndex===2){subsetCheckpoint=s;assert.deepEqual(s.submitted,[false,false,true,true,true]);
  const ready=await tx('commitment_set_345_ready_for_future_p3',abi.encodeFunctionData('requireP3Ready'));assert.equal(ready.receipt_status,1);}
}
const final=await snapshot();assert.equal(final.count,5);assert.ok(final.p3_ready);assert.deepEqual(final.submitted,[true,true,true,true,true]);
const sourcePaths=['contracts/scripts/g8b_all_trustees.mjs','scripts/g8b_five_record.mjs',configPath,...original.source_hashes.map(p=>p.path)];
const sourceHashes=[...new Set(sourcePaths)].map(p=>({path:p,sha256:sha(read(p))}));
for(const p of sourceHashes){const out=path.resolve(root,raw,'provenance',`${p.path.replaceAll('/','__')}.executed.txt`);fs.mkdirSync(path.dirname(out),{recursive:true});fs.writeFileSync(out,read(p.path),{flag:'wx'});}
const result={gate:'G8-B-five-trustees',status:'PASS_FIVE_ORIGINAL_COMMITMENTS_CERTIFIER_AND_DECRYPTION_ROSTER_SEPARATED',
 recorded_at_utc:new Date().toISOString(),git_commit:execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),
 git_dirty:execFileSync('git',['status','--porcelain'],{cwd:root,encoding:'utf8'}).trim().length>0,branch:execFileSync('git',['branch','--show-current'],{cwd:root,encoding:'utf8'}).trim(),
 hardfork:common.hardfork(),node:process.version,cpu:os.cpus()[0].model,platform:os.platform(),arch:os.arch(),peak_node_rss_mib:process.resourceUsage().maxRSS/1024,
 execution_scope:'actual signed transactions in a fresh local Prague VM; exact historical bytecode/prefix replay, no RPC/public deployment',
 config:configPath,source_pins:config.source_pins,source_hashes:sourceHashes,runtime_checks:runtimes,transactions,transaction_count:transactions.length,
 prefix_transactions_reexecuted:prefix.length,prefix_negative_transactions:prefix.filter(t=>t.receipt_status===0).length,
 original_signature_bytes_preserved:true,original_package_signatures_verified_evm:5,verification_share_polynomial_checks_evm:5,
 dkg_certifier_ids:config.dkg_certifier_ids,commitment_submission_order:config.commitment_submission_order,first_ready_commitment_set:config.first_ready_commitment_set,
 q_cert:3,tau:3,initial_checkpoint:initial,first_ready_checkpoint:subsetCheckpoint,final_checkpoint:final,accepted_commitments:accepted,negative_tests:negatives,
 contract_sources_changed:false,bytecode_changed:false,new_signatures:0,new_cp_packages:0,dkg_reexecuted:false,setup_count:0,proof_generation_count:0,
 private_witness_emitted:false,p3_verified_in_this_flow:false,winner_written:false,receiver_auth_gate_enforced_on_intake:false,
 complete_g8_settlement:false,snarkpack_evm_verified:false,full_system_gas:null,
 limitations:['component-only acceptance of five authenticated public commitments, NOT five CP-verified private decryption shares',
 'the different set {3,4,5} reaches commitment readiness; no new P3 proof or private decryption is claimed for it',
 'old receiver still does not enforce certificate gate before intake; P3 storage reconstruction and Winner write-once remain G8-C/D',
 'original authorized deployment/key trust roots, fixed research profile/setup, no exact SnarkPack EVM or full-system comparison',
 'single functional run; gas depends on submission order/storage warmness and is not independent statistical evidence']};
fs.mkdirSync(path.dirname(path.resolve(root,output)),{recursive:true});fs.writeFileSync(path.resolve(root,output),`${JSON.stringify(result,null,2)}\n`,{flag:'wx'});
console.log(`G8-B five PASS: 5/5 original package signatures; readiness {3,4,5} independent of certifiers {1,2,3}; ${negatives.length} new atomic rejections; ${transactions.length} signed txs; full G8 PARTIAL`);
