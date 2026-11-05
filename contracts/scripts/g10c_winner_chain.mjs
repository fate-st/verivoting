// G10-C chain continuation: unchanged contracts, this run's new worker commitments/P3.
// Individual-Groth16 EVM fallback remains explicit; no exact SnarkPack claim.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import solc from 'solc';
import {ContractFactory,Interface,keccak256,getCreateAddress} from 'ethers';
import {Common,Hardfork,Mainnet} from '@ethereumjs/common';
import {createBlock} from '@ethereumjs/block';
import {createLegacyTx} from '@ethereumjs/tx';
import {createVM,runTx} from '@ethereumjs/vm';
import {createAccount,createAddressFromPrivateKey,createAddressFromString,hexToBytes,bytesToHex} from '@ethereumjs/util';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const read=p=>fs.readFileSync(path.resolve(root,p)),load=p=>JSON.parse(read(p));
const sha=b=>createHash('sha256').update(b).digest('hex'),hex=s=>`0x${s}`,wire=h=>Buffer.from(h.replace(/^0x/,''),'hex');
const meta=(c,a)=>execFileSync(c,a,{cwd:root,encoding:'utf8'}).trim();
const raw=process.argv[2];assert.ok(raw,'require new chain output directory');const resultPath=`${raw}/winner_once.json`,artifactPath=`${raw}/contract_artifacts.json`;
for(const p of[resultPath,artifactPath])assert.ok(!fs.existsSync(path.resolve(root,p)),`new output required ${p}`);
const configPath=process.argv[3];assert.ok(configPath,'require current-run bridge config');const config=load(configPath);
const frozenConfig=load('configs/experiment.G8C-FROZEN-P3-W32.json');
for(const key of ['p3_ark_vk_sha256','p3_eip_vk_sha256','p3_eip_vk_keccak256','ballot_eip_vk_sha256','state_eip_vk_sha256','binding_digest','accepted_roots'])assert.deepEqual(config[key],frozenConfig[key],key);
for(const p of config.source_pins)assert.equal(sha(read(p.path)),p.sha256,p.path);
const g7=load(config.source_pins[0].path),bf=load(config.source_pins[1].path),a=load(config.source_pins[2].path),f=load(config.source_pins[3].path),g6=load(config.source_pins[4].path);
assert.deepEqual(a.public_context,g6.public_context);assert.deepEqual(f.public_context,g6.public_context);assert.deepEqual(bf.public_context,g6.public_context);
assert.equal(g7.verifying_key_ark_sha256,config.p3_ark_vk_sha256);assert.equal(sha(wire(g6.verifying_key_hex)),config.p3_ark_vk_sha256);
assert.equal(sha(wire(g6.proof_hex)),g7.proof_ark_sha256);assert.equal(g7.proof_ark_sha256,g6.proof_sha256);
assert.equal(sha(read(g7.source_fixture)),g7.source_fixture_sha256);assert.equal(g7.source_fixture,config.source_pins[4].path);
assert.equal(sha(wire(g7.vk_eip2537_hex)),config.p3_eip_vk_sha256);assert.equal(keccak256(g7.vk_eip2537_hex),config.p3_eip_vk_keccak256);
assert.equal(wire(g6.proof_hex).length,192);assert.equal(wire(g7.proof_neg_a_eip2537_hex).length,512);
assert.equal(sha(wire(bf.ballot_vk_eip2537_hex)),config.ballot_eip_vk_sha256);assert.equal(sha(wire(bf.state_vk_eip2537_hex)),config.state_eip_vk_sha256);
// The archived G7 native exporter already canonical-decoded/verified/re-encoded
// this exact proof/VK. Pin its report and hashes; do not claim sha(192B)==sha(512B).
const g7ExportPath=process.argv[4];assert.ok(g7ExportPath,'require current native export provenance');const g7Report=load(g7ExportPath);
assert.equal(g7Report.status,'PASS_CURRENT_NATIVE_P3_EIP_EXPORT');
assert.equal(g7Report.output_fixture_sha256,config.source_pins[0].sha256);
const sourcePaths=['auth/CanonicalVV.sol','auth/Sha512.sol','auth/JubjubSchnorrVerifier.sol','auth/VeriVotingCertificateGate.sol',
 'auth/VeriVotingCertificateGateV2.sol','auth/JubjubVerificationShare.sol','auth/VeriVotingTrusteeCommitments.sol',
 'VeriVotingSettlement.sol','VeriVotingAuthenticatedSettlement.sol','G8CExperimentPins.sol','VeriVotingFrozenP3Verifier.sol',
 'VeriVotingWinnerOnlySettlement.sol','IndividualGroth16BatchVerifier.sol','Bls12381Groth16Verifier.sol','interfaces/IBatchVerifier.sol'].map(p=>`contracts/src/${p}`);
const sources=Object.fromEntries(sourcePaths.map(p=>[p.replace('contracts/src/',''),{content:read(p).toString('utf8')}]));
const settings={optimizer:{enabled:true,runs:200},viaIR:true,evmVersion:'prague',outputSelection:{'*':{'*':['abi','evm.bytecode.object','evm.deployedBytecode.object']}}};
console.log('G10-C-chain [1] compile additive gate/read interfaces, authenticated receiver and fixed-VK P3 binder');
const compilation=JSON.parse(solc.compile(JSON.stringify({language:'Solidity',sources,settings})));
for(const e of compilation.errors??[]){if(e.severity==='error')throw new Error(e.formattedMessage);console.warn(e.formattedMessage);}
const get=(file,name)=>compilation.contracts[file][name];
const gateCode=get('auth/VeriVotingCertificateGateV2.sol','VeriVotingCertificateGateV2');
const receiverCode=get('VeriVotingWinnerOnlySettlement.sol','VeriVotingWinnerOnlySettlement');
const storeCode=get('auth/VeriVotingTrusteeCommitments.sol','VeriVotingTrusteeCommitments');
const p3Code=get('VeriVotingFrozenP3Verifier.sol','VeriVotingFrozenP3Verifier');
const grothCode=get('Bls12381Groth16Verifier.sol','Bls12381Groth16Verifier');
const adapterCode=get('IndividualGroth16BatchVerifier.sol','IndividualGroth16BatchVerifier');
const mathCode=get('auth/JubjubSchnorrVerifier.sol','JubjubSchnorrVerifier');
const gateAbi=new Interface(gateCode.abi),receiverAbi=new Interface(receiverCode.abi),storeAbi=new Interface(storeCode.abi),p3Abi=new Interface(p3Code.abi),grothAbi=new Interface(grothCode.abi),adapterAbi=new Interface(adapterCode.abi);
const common=new Common({chain:Mainnet,hardfork:Hardfork.Prague}),vm=await createVM({common});assert.ok(common.isActivatedEIP(2537));
// Public LOCAL-ONLY deterministic research sender, no public RPC/deployment.
const senderKey=hexToBytes(`0x${'3c'.repeat(32)}`),sender=createAddressFromPrivateKey(senderKey);
await vm.stateManager.putAccount(sender,createAccount({nonce:0n,balance:10n**25n}));
const block=createBlock({header:{number:1n,gasLimit:BigInt(config.block_gas_limit),baseFeePerGas:1n,timestamp:1_750_000_000n}},{common,skipConsensusFormatValidation:true});
fs.mkdirSync(path.resolve(root,raw),{recursive:true});
const journalPath=path.resolve(root,raw,'transactions.jsonl');fs.writeFileSync(journalPath,'',{flag:'wx'});
let nonce=0n,trace=[];const stack=[],transactions=[];
vm.evm.events.on('beforeMessage',m=>stack.push({address:m.to?.toString(),input_bytes:m.data.length,input_sha256:sha(m.data)}));
vm.evm.events.on('afterMessage',r=>{const m=stack.pop();if(m&&['0002','000c','000f'].includes(m.address?.slice(-4)))trace.push({...m,
 success:!r.execResult.exceptionError,execution_gas:Number(r.execResult.executionGasUsed),return_hex:bytesToHex(r.execResult.returnValue)});});
async function tx(label,data,to){
 trace=[];const signed=createLegacyTx({nonce:nonce++,gasLimit:BigInt(config.transaction_gas_limit),gasPrice:10n,value:0n,data:hexToBytes(data),...(to?{to}:{})},{common}).sign(senderKey);
 const start=performance.now(),r=await runTx(vm,{tx:signed,block});
 const intrinsic=Number(signed.getIntrinsicGas()),execution=Number(r.execResult.executionGasUsed),minimum=Number(signed.getMinimumGasLimit());
 assert.equal(stack.length,0);assert.equal(Number(r.gasRefund),0);assert.equal(Number(r.totalGasSpent),Math.max(intrinsic+execution,minimum));
 const measurement={label,nonce:Number(signed.nonce),transaction_hash:bytesToHex(signed.hash()),signed_transaction_hex:bytesToHex(signed.serialize()),
  to:to?.toString()??null,receipt_status:r.receipt.status,total_gas:Number(r.totalGasSpent),intrinsic_gas:intrinsic,execution_gas:execution,gas_refund:Number(r.gasRefund),
  minimum_gas_limit_including_calldata_floor:minimum,transaction_gas_limit:config.transaction_gas_limit,calldata_hex:bytesToHex(signed.data),calldata_bytes:signed.data.length,calldata_sha256:sha(signed.data),
  return_hex:bytesToHex(r.execResult.returnValue),exception:r.execResult.exceptionError?.error??null,
  revert_reason:r.execResult.exceptionError?.error==='revert'?p3Abi.parseError(bytesToHex(r.execResult.returnValue))?.args[0]??null:null,
  precompile_calls:[...trace],local_vm_wall_ms:performance.now()-start,logs:r.receipt.logs.map(([address,topics,data])=>({address:bytesToHex(address),topics:topics.map(bytesToHex),data:bytesToHex(data)}))};
 transactions.push(measurement);fs.appendFileSync(journalPath,JSON.stringify(measurement)+'\n');return {result:r,measurement};
}
async function view(address,iface,name,args=[]){
 const before=(await vm.stateManager.getAccount(sender)).nonce;await vm.evm.journal.checkpoint();
 let r;try{r=await vm.evm.runCall({caller:sender,to:address,data:hexToBytes(iface.encodeFunctionData(name,args)),gasLimit:20_000_000n,isStatic:true});}finally{await vm.evm.journal.revert();}
 assert.equal((await vm.stateManager.getAccount(sender)).nonce,before);assert.equal(r.execResult.exceptionError,undefined,`view ${name}`);return iface.decodeFunctionResult(name,bytesToHex(r.execResult.returnValue));
}
const deployData=async(compiled,args)=>(await new ContractFactory(compiled.abi,`0x${compiled.evm.bytecode.object}`).getDeployTransaction(...args)).data;
async function deploy(label,compiled,args){const r=await tx(label,await deployData(compiled,args));assert.equal(r.measurement.receipt_status,1,`${label}: ${JSON.stringify(r.measurement)}`);assert.ok(r.result.createdAddress);return r.result.createdAddress;}
const c=g6.public_context,point=p=>({x:p.x,y:p.y,encoded:hex(p.encoded_hex)}),sig=s=>({nonce:point(s.nonce),response:s.response_scalar,counter:s.counter});
const binding={electionId:hex(c.election_id_hex),policyPre:hex(c.policy_pre_hex),dkgSession:hex(c.dkg_session_hex),keyCoreDigest:hex(c.key_core_digest_hex),publicKey:hex(c.public_key_hex),
 finalContext:hex(c.final_context_hex),registrationRoot:hex(c.registration_root_hex),registrationCloseDigest:hex(c.registration_close_digest_hex),disclosureDigest:hex(c.disclosure_digest_hex),
 p3VkSha256:hex(g7.verifying_key_ark_sha256),prekeyContext:hex(c.prekey_context_hex),registrationAuthority:hex(c.registration_authority_hex),
 registrationCertificateSha256:hex(sha(wire(c.registration_certificate_hex))),region:4,registeredWeight:32};
const expected={electionId:binding.electionId,manifestDigest:hex(a.manifest_digest_hex),policyPre:binding.policyPre,prekeyContext:binding.prekeyContext,dkgSession:binding.dkgSession,
 keyCoreDigest:binding.keyCoreDigest,finalContext:binding.finalContext,publicKey:binding.publicKey,registrationRoot:binding.registrationRoot,closeDigest:binding.registrationCloseDigest,disclosureDigest:binding.disclosureDigest};
const close={electionId:hex(a.close.election_id_hex),finalContext:hex(a.close.final_context_hex),policyPre:hex(a.close.policy_pre_hex),root:hex(a.close.root_hex),
 nRegistered:a.close.n_registered,wRegistered:a.close.w_registered,wMax:a.close.w_max,version:a.close.version,canonicalRegistryDigest:hex(a.close.canonical_registry_digest_hex)};
const flatten=ct=>ct.flatMap(p=>[hex(p.c0_hex),hex(p.c1_hex)]);
const batches=bf.batches.map(b=>({index:b.batch_index,root:hex(b.root_hex),oldState:flatten(b.old_state),newState:flatten(b.new_state),ballotProofs:b.ballot_proofs_eip2537_hex,ballotInputs:b.ballot_inputs_be_hex,stateProof:b.state_proof_eip2537_hex}));
console.log('G10-C-chain [2] deploy fixed interconnected contracts; no mutable Gate/Receiver/Store/VK selectors');
const gate=await deploy('deploy_certificate_gate_v2',gateCode,[expected,a.authentication_keys.map(point),point(a.registration_authority),a.public_coefficients.map(point),hex(a.transcript_digest_hex)]);
// gate nonce 0, receiver 1, store 2, P3 binder 3. The receiver's P3 pointer is immutable.
const expectedP3=getCreateAddress({from:sender.toString(),nonce:3});
const receiver=await deploy('deploy_winner_receiver',receiverCode,[gate.toString(),binding,config.accepted_roots,bf.ballot_vk_eip2537_hex,bf.state_vk_eip2537_hex,expectedP3]);
// Old typed ABI is intentionally retained: the new contracts implement the same
// bindingDigest/checkpoint/certificate selectors; old G8-B source stays unchanged.
const store=await deploy('deploy_original_commitment_store',storeCode,[gate.toString(),receiver.toString(),binding,hex(a.manifest_digest_hex),hex(a.transcript_digest_hex),a.authentication_keys.map(point),a.public_coefficients.map(point)]);
const p3Args=[gate.toString(),receiver.toString(),store.toString(),g7.vk_eip2537_hex];
const p3=await deploy('deploy_frozen_p3_binder',p3Code,p3Args);
assert.equal(p3.toString(),expectedP3.toLowerCase());
let t=await tx('bind_fixed_p3_once',receiverAbi.encodeFunctionData('bindFrozenP3'),receiver);assert.equal(t.measurement.receipt_status,1);
for(const[address,iface,name,want]of[[receiver,receiverAbi,'certificateGate',gate],[store,storeAbi,'certificateGate',gate],[store,storeAbi,'receiver',receiver],
 [p3,p3Abi,'certificateGate',gate],[p3,p3Abi,'receiver',receiver],[p3,p3Abi,'commitmentStore',store]])assert.equal((await view(address,iface,name))[0].toLowerCase(),want.toString());
assert.equal((await view(receiver,receiverAbi,'bindingDigest'))[0],config.binding_digest);
const p3Math=createAddressFromString((await view(p3,p3Abi,'verifier'))[0]);
assert.equal((await view(p3Math,grothAbi,'verificationKeyDigest'))[0],config.p3_eip_vk_keccak256);
for(const[name,want]of[['ARK_VK_SHA256',hex(config.p3_ark_vk_sha256)],['EIP_VK_SHA256',hex(config.p3_eip_vk_sha256)],['EIP_VK_KECCAK256',config.p3_eip_vk_keccak256]])assert.equal((await view(p3,p3Abi,name))[0],want);
const nullifiers=batches.flatMap(b=>b.ballotInputs.map(row=>{
 const low=BigInt(row[7]),last=BigInt(row[8]);assert.ok(low<(1n<<248n)&&last<256n);
 return hex(Buffer.concat([Buffer.from(low.toString(16).padStart(64,'0'),'hex').reverse().subarray(0,31),Buffer.from([Number(last)])]).toString('hex'));
}));
assert.equal(new Set(nullifiers).size,8);
async function snapshot(){
 const r=await view(receiver,receiverAbi,'checkpoint'),s=await view(store,storeAbi,'checkpoint'),g=await view(gate,gateAbi,'checkpoint');
 const spent=[],submitted=[],commitments=[],shares=[],certifiers=[];
 for(const nf of nullifiers)spent.push((await view(receiver,receiverAbi,'spentNullifier',[nf]))[0]);
 for(let id=1;id<=5;id++){submitted.push((await view(store,storeAbi,'submitted',[id]))[0]);commitments.push((await view(store,storeAbi,'packageCommitment',[id]))[0]);shares.push((await view(store,storeAbi,'verifiedShare',[id]))[0]);certifiers.push((await view(gate,gateAbi,'certifiedSigner',[id]))[0]);}
 return {receiver_next:Number(r[0]),spent_count:Number(r[1]),frozen:r[2],freeze_seal:r[3],cp_target:r[4],ciphertexts:[...r[5]],spent_nullifiers:spent,
  store_bound:s[0],store_seal:s[1],store_target:s[2],commitment_count:Number(s[3]),p3_ready:s[4],submitted,commitments,verification_shares:shares,
  certificate_count:Number(g[0]),dkg_certified:g[1],registration_certified:g[2],registration_root:g[3],close_digest:g[4],disclosure_digest:g[5],authorized:g[6],certifier_mask:certifiers,
  certified_w_registered:Number((await view(gate,gateAbi,'certifiedWRegistered'))[0]),certified_n_registered:Number((await view(gate,gateAbi,'certifiedNRegistered'))[0]),
  certified_w_max:Number((await view(gate,gateAbi,'certifiedWMax'))[0]),certificate_sha256:(await view(gate,gateAbi,'certifiedCertificateSha256'))[0],
  p3_bound:(await view(receiver,receiverAbi,'p3Bound'))[0],fixed_p3:(await view(receiver,receiverAbi,'frozenP3Verifier'))[0].toLowerCase(),
  winner_disclosed:(await view(receiver,receiverAbi,'winnerDisclosed'))[0],winner:Number((await view(receiver,receiverAbi,'winner'))[0]),winner_selector:Number((await view(receiver,receiverAbi,'winnerSelector'))[0])};
}
const negatives=[],checkpoints=[];
async function reject(label,data,to,reason,expectedPairings=0){
 const before=await snapshot(),r=await tx(label,data,to),after=await snapshot(),t=r.measurement;
 assert.equal(t.receipt_status,0,label);assert.equal(t.exception,'revert',label);assert.equal(t.revert_reason,reason,label);assert.equal(t.logs.length,0);assert.deepEqual(after,before,`${label} atomic`);
 const pairs=t.precompile_calls.filter(p=>p.address.endsWith('000f'));assert.equal(pairs.length,expectedPairings,label);
 if(pairs.length){assert.ok(t.precompile_calls.every(p=>p.success));assert.ok(pairs.slice(0,-1).every(p=>BigInt(p.return_hex)===1n));assert.equal(BigInt(pairs.at(-1).return_hex),0n);}
 negatives.push({case:label,transaction_nonce:t.nonce,revert_reason:reason,pairing_count:pairs.length,before,after,atomic_rejection:true});console.log(`  reject ${label}: ${reason}, pairings=${pairs.length}`);
}
const submit=b=>receiverAbi.encodeFunctionData('submitBatch',[b]);
const disclose=(selector,winner)=>receiverAbi.encodeFunctionData('discloseWinner',[g7.proof_neg_a_eip2537_hex,selector,winner]);
await reject('repeat_p3_binding',receiverAbi.encodeFunctionData('bindFrozenP3'),receiver,'P3_ALREADY_BOUND');
console.log('G10-C-chain [3] original certificate authentication and certified-data getters');
for(let i=0;i<3;i++){const r=await tx(`authenticate_dkg_${i+1}`,gateAbi.encodeFunctionData('submitDkgSignature',[i+1,binding.electionId,binding.finalContext,sig(a.dkg_certificate_signatures[i])]),gate);assert.equal(r.measurement.receipt_status,1);}
t=await tx('authenticate_registration',gateAbi.encodeFunctionData('authenticateRegistration',[close,sig(a.registration_signature)]),gate);assert.equal(t.measurement.receipt_status,1);
const certified=await snapshot();assert.ok(certified.authorized);assert.equal(certified.certified_w_registered,32);assert.equal(certified.certificate_sha256,binding.registrationCertificateSha256);
const expectedRead=(await view(gate,gateAbi,'certifiedBinding'))[0];for(const k of Object.keys(expected))assert.equal(expectedRead[k],expected[k],k);
for(let i=0;i<3;i++)assert.equal((await view(gate,gateAbi,'certifiedCoefficient',[i]))[0].encoded,hex(a.public_coefficients[i].encoded_hex));
for(let i=1;i<=5;i++)assert.equal((await view(gate,gateAbi,'certifiedAuthenticationKey',[i]))[0].encoded,hex(a.authentication_keys[i-1].encoded_hex));
console.log('G10-C-chain [4] execute both real B4 receiver/state-verifier paths unchanged');
for(const[i,b]of batches.entries()){
 const r=await tx(`accept_batch_${i+1}`,submit(b),receiver);assert.equal(r.measurement.receipt_status,1);
 const pairs=r.measurement.precompile_calls.filter(p=>p.address.endsWith('000f'));assert.equal(pairs.length,5);assert.ok(pairs.every(p=>p.success&&BigInt(p.return_hex)===1n));
 const s=await snapshot();assert.deepEqual(s.ciphertexts,b.newState);assert.equal(s.spent_count,(i+1)*4);checkpoints.push({phase:`accepted_batch_${i+1}`,state:s});
}
await reject('disclose_before_freeze',disclose(0,1),receiver,'NOT_FROZEN');
console.log('G10-C-chain [5] freeze actual CT2, preserve original seal, then authenticate original public commitments');
t=await tx('freeze_ct2',receiverAbi.encodeFunctionData('freeze'),receiver);assert.equal(t.measurement.receipt_status,1);
const frozen=await snapshot();assert.equal(frozen.freeze_seal,hex(g6.freeze_digest_hex));assert.equal(frozen.cp_target,hex(c.cp_target_hex));assert.deepEqual(frozen.ciphertexts,flatten(c.frozen_ciphertexts));
t=await tx('bind_authenticated_frozen_ct2',storeAbi.encodeFunctionData('bindFrozen'),store);assert.equal(t.measurement.receipt_status,1);
const submission=row=>({trusteeId:row.trustee_id,electionId:binding.electionId,finalContext:binding.finalContext,target:hex(c.cp_target_hex),
 verificationShare:point(row.verification_share),commitment:point(row.commitment),signature:sig(row.signature)});
async function acceptCommitment(id){const row=f.records[id-1],r=await tx(`authenticate_commitment_${id}`,storeAbi.encodeFunctionData('submitCommitment',[submission(row)]),store);assert.equal(r.measurement.receipt_status,1);
 assert.ok(r.measurement.precompile_calls.some(p=>p.address.endsWith('0002')&&p.return_hex===hex(row.original_message_hex)));}
await acceptCommitment(1);await acceptCommitment(2);await acceptCommitment(3);
console.log('G10-C-chain [6] reconstruct 60 exact fields inside contract; compare both original archive encodings');
const actualInputs=[...(await view(p3,p3Abi,'buildP3Inputs',[0,1]))[0]].map(v=>`0x${v.toString(16).padStart(64,'0')}`);
const g6Inputs=g6.public_inputs_le_hex.map(h=>hex(Buffer.from(h,'hex').reverse().toString('hex')));
assert.equal(actualInputs.length,60);assert.deepEqual(actualInputs,g6Inputs);assert.deepEqual(actualInputs,g7.public_inputs_be_hex);
// Independent byte-source map from ACTUAL authenticated/frozen getters, not
// fixture constants substituted for host results in the contract.
const chainBinding=(await view(gate,gateAbi,'certifiedBinding'))[0],state=await snapshot();
const be=(v,n)=>Buffer.from(BigInt(v).toString(16).padStart(2*n,'0'),'hex');
const blob=b=>Buffer.concat([be(b.length,8),b]),text=s=>blob(Buffer.from(s));
const sourcesForInput=[],mapped=[];
function appendPacked(segments,component){
 let start=0;const ranges=segments.map(s=>{const r={source:s.source,offset:start,length:s.bytes.length};start+=s.bytes.length;return r;});
 const combined=Buffer.concat(segments.map(s=>s.bytes));
 for(let pos=0;pos<combined.length;pos+=31){const chunk=combined.subarray(pos,pos+31),value=hex(Buffer.from(chunk).reverse().toString('hex').padStart(64,'0'));
  sourcesForInput.push({index:mapped.length,component,canonical_bytes_offset:pos,canonical_bytes_length:chunk.length,packed_little_endian_hex:chunk.toString('hex'),value_be_hex:value,
   sources:ranges.filter(r=>r.offset<pos+chunk.length&&r.offset+r.length>pos).map(r=>({...r,overlap_start:Math.max(pos,r.offset),overlap_length:Math.min(pos+chunk.length,r.offset+r.length)-Math.max(pos,r.offset)}))});mapped.push(value);}
}
const seg=(source,bytes)=>({source,bytes});
for(const id of[1,2,3]){
 const vshare=(await view(store,storeAbi,'verifiedShare',[id]))[0],commitment=(await view(store,storeAbi,'packageCommitment',[id]))[0];
 appendPacked([seg('constant canonical prefix',text('VV-CANONICAL-V1')),seg('constant original CP domain',text('VV/JUBJUB/GENERALIZED-CP/TRANSCRIPT/v3')),seg('constant original suite',text('VERIVOTING-JUBJUB-PROTOTYPE-G0.8-v2')),
  ...['electionId','policyPre','finalContext'].map(k=>seg(`Gate.certifiedBinding.${k}`,wire(chainBinding[k]))),
  seg('Gate.certifiedCloseDigest',wire(state.close_digest)),seg('Gate.certifiedDisclosureDigest',wire(state.disclosure_digest)),seg('Receiver.cpTarget',wire(state.cp_target)),
  seg('original selector table Trustee ID',be(id,4)),seg(`CommitmentStore.verifiedShare(${id})`,wire(vshare)),seg('original coordinate count',be(3,8)),
  ...[0,2,4].map(j=>seg(`Receiver.frozenCT2[${j}]`,wire(state.ciphertexts[j]))),seg('original decryption-share count',be(3,8))],`trustee_${id}_public_cp_prefix`);
 appendPacked([seg(`CommitmentStore.packageCommitment(${id})`,wire(commitment))],`trustee_${id}_commitment`);
}
function scalar(source,value,component){const h=hex(BigInt(value).toString(16).padStart(64,'0'));sourcesForInput.push({index:mapped.length,component,value_be_hex:h,sources:[{source}]});mapped.push(h);}
scalar('caller proposed selector, original fixed lookup table',0,'selector');
for(let j=0;j<3;j++)appendPacked([seg(`Receiver.frozenCT2[${2*j+1}]`,wire(state.ciphertexts[2*j+1]))],`candidate_${j+1}_C1`);
scalar('Gate.certifiedWRegistered',state.certified_w_registered,'registered_weight');scalar('caller proposed winner, constrained by P3',1,'winner');
assert.deepEqual(mapped,actualInputs);assert.equal(sourcesForInput.length,60);
console.log('G10-C-chain [7] wrong Winner BEFORE finalization, then real verification + atomic Winner write, then replay');
await reject('wrong_winner_before_disclosure',disclose(0,2),receiver,'INVALID_GROTH16',1);
const beforeValid=await snapshot();t=await tx('disclose_winner_once',disclose(0,1),receiver);assert.equal(t.measurement.receipt_status,1);
const validPairings=t.measurement.precompile_calls.filter(p=>p.address.endsWith('000f'));
assert.equal(validPairings.length,1);assert.equal(BigInt(validPairings[0].return_hex),1n);
const afterValid=await snapshot();assert.ok(afterValid.winner_disclosed);assert.equal(afterValid.winner,1);assert.equal(afterValid.winner_selector,0);
assert.deepEqual(afterValid,{...beforeValid,winner_disclosed:true,winner:1,winner_selector:0});
const winnerEvents=t.measurement.logs.filter(l=>l.address===receiver.toString()).map(l=>receiverAbi.parseLog(l)).filter(l=>l?.name==='WinnerDisclosed');
assert.equal(winnerEvents.length,1);assert.equal(winnerEvents[0].args.electionId,binding.electionId);assert.equal(winnerEvents[0].args.freezeSeal,frozen.freeze_seal);
assert.equal(Number(winnerEvents[0].args.winner),1);assert.equal(Number(winnerEvents[0].args.selector),0);
await reject('repeat_winner_disclosure',disclose(0,1),receiver,'ALREADY_DISCLOSED');
const final=await snapshot();assert.deepEqual(final,afterValid);assert.equal(final.commitment_count,3);assert.ok(final.frozen&&final.authorized);assert.deepEqual(final.spent_nullifiers,Array(8).fill(true));
checkpoints.push({phase:'before_valid_disclosure',state:beforeValid},{phase:'after_valid_disclosure',state:afterValid});
const math=createAddressFromString((await view(gate,gateAbi,'verifier'))[0]),adapter=createAddressFromString((await view(receiver,receiverAbi,'batchVerifier'))[0]);
const ballot=createAddressFromString((await view(adapter,adapterAbi,'verifier'))[0]),stateVerifier=createAddressFromString((await view(receiver,receiverAbi,'stateVerifier'))[0]);
const artifacts={compiler:solc.version(),compiler_settings:settings,contracts:{}};
for(const[name,address,compiled]of[['certificate_gate_v2',gate,gateCode],['original_schnorr_verifier',math,mathCode],['winner_receiver',receiver,receiverCode],['original_commitment_store',store,storeCode],
 ['frozen_p3_binder',p3,p3Code],['p3_groth16_verifier',p3Math,grothCode],['batch_adapter',adapter,adapterCode],['ballot_verifier',ballot,grothCode],['state_verifier',stateVerifier,grothCode]]){
 const runtime=await vm.stateManager.getCode(address);assert.ok(runtime.length>0&&runtime.length<=24576);artifacts.contracts[name]={address:address.toString(),abi:compiled.abi,
  creation_bytecode_hex:`0x${compiled.evm.bytecode.object}`,creation_bytecode_bytes:compiled.evm.bytecode.object.length/2,runtime_bytecode_hex:bytesToHex(runtime),runtime_bytecode_bytes:runtime.length,runtime_sha256:sha(runtime),runtime_keccak256:keccak256(runtime)};}
const sourceHashes=[...sourcePaths,configPath,'contracts/scripts/g10c_winner_chain.mjs','scripts/g10c_chain.mjs','contracts/package-lock.json'].map(p=>({path:p,sha256:sha(read(p))}));
for(const p of sourceHashes){const out=path.resolve(root,raw,'provenance',`${p.path.replaceAll('/','__')}.executed.txt`);fs.mkdirSync(path.dirname(out),{recursive:true});fs.writeFileSync(out,read(p.path),{flag:'wx'});}
function writeNew(p,v){fs.mkdirSync(path.dirname(path.resolve(root,p)),{recursive:true});fs.writeFileSync(path.resolve(root,p),`${JSON.stringify(v,null,2)}\n`,{flag:'wx'});}
writeNew(artifactPath,artifacts);
const result={gate:'G10-C-chain',status:'PASS_CURRENT_NATIVE_P3_AND_COMMITMENTS_EVM_WINNER_ONCE',recorded_at_utc:new Date().toISOString(),
 git_commit:meta('git',['rev-parse','HEAD']),git_dirty:meta('git',['status','--porcelain']).length>0,branch:meta('git',['branch','--show-current']),cpu:os.cpus()[0].model,
 platform:os.platform(),arch:os.arch(),node:process.version,hardfork:common.hardfork(),compiler:solc.version(),compiler_settings:settings,peak_node_rss_mib:process.resourceUsage().maxRSS/1024,
 execution_scope:'real signed transactions sharing local Prague VM state, each under 20M tx / 30M block context; not one summed block or public deployment',
 source_pins:config.source_pins,source_hashes:sourceHashes,g7_native_export_report:g7ExportPath,g7_native_export_report_sha256:sha(read(g7ExportPath)),
 addresses:Object.fromEntries(Object.entries(artifacts.contracts).map(([k,v])=>[k,v.address])),contract_artifact:artifactPath,contract_artifact_sha256:sha(read(artifactPath)),
 ark_vk_sha256:config.p3_ark_vk_sha256,eip_vk_sha256:config.p3_eip_vk_sha256,eip_vk_keccak256:config.p3_eip_vk_keccak256,
 ark_proof_sha256:sha(wire(g6.proof_hex)),eip_proof_sha256:sha(wire(g7.proof_neg_a_eip2537_hex)),compressed_proof_bytes:192,eip_proof_bytes:512,
 selector:0,selected_trustees:[1,2,3],winner:1,public_input_count:60,public_inputs_be_hex:actualInputs,public_input_source_map:sourcesForInput,
 g6_input_match:true,g7_input_match:true,all_60_inputs_match:true,independent_source_map_matches:true,initial_binding:binding,accepted_roots:config.accepted_roots,
 baseline_commit:'2ce1caedf11a064c36f8322daf65e69b8522bf76',g8c_archive:'experiments/g8c_raw/frozen_p3_binding.json',g8c_archive_sha256:sha(read('experiments/g8c_raw/frozen_p3_binding.json')),
 initialization_policy:'deployer confirms immutable precomputed P3 CREATE address once, before start; selected receiver address is the authoritative election endpoint',
 successful_winner_writes:1,winner_event_count:winnerEvents.length,before_valid_disclosure:beforeValid,after_valid_disclosure:afterValid,
 transactions,transaction_count:transactions.length,negative_tests:negatives,checkpoints,frozen_checkpoint:frozen,final_checkpoint:final,
 initialization_certificate_checked_on_evm:true,receiver_auth_gate_enforced_on_intake:true,trustee_authentication_checked_on_evm:true,original_package_signatures_verified_evm:3,
 p3_storage_binding_integrated:true,p3_verified_in_this_flow:true,caller_public_input_array_accepted:false,caller_verifier_address_accepted:false,
 reused_ballot_proofs:8,reused_state_proofs:2,reused_p3_proofs:1,setup_count:0,proof_generation_count:0,new_cp_packages:0,new_signatures:0,dkg_reexecuted:false,
 private_witness_emitted:false,winner_written:true,winner_write_once_integrated:true,complete_g8_settlement:false,snarkpack_evm_verified:false,full_system_gas:null,production_setup:false,
 limitations:['G10-C-chain only: fixed local individual-Groth16 fallback + state proof + authentic certificates/commitments + P3 + one-time Winner write; exact SnarkPack EVM unresolved',
 'write-once is enforced on this authoritative receiver/election instance, not globally across arbitrarily cloned contracts',
 'fixed D5/W32, n5/t2, original selector0/123 proof; no new ten-subset EVM test or statistical comparison',
 'original bootstrap identity keys/authority and deployment initializer/immutable address selection remain trust roots',
 'EVM stage consumes the same newly generated proof/commitments from this run native stage; no private witness is supplied to EVM',
 'research setup/SRS, local Prague VM, no public deployment or exact SnarkPack; receipt totals are functional-run accounting, not performance superiority']};
writeNew(resultPath,result);
console.log(`G10-C-chain PASS: 60/60 original inputs; authenticated live receiver/freeze/commitments/P3; ${negatives.length} exact-reason atomic rejects; ${transactions.length} signed transactions; Winner=1 stored exactly once`);
