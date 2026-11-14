// Exact signed-transaction comparison; no DKG, P3, Groth16 setup or proving.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import solc from 'solc';
import {ContractFactory,Interface,AbiCoder} from 'ethers';
import {Common,Hardfork,Mainnet} from '@ethereumjs/common';
import {createBlock} from '@ethereumjs/block';
import {createLegacyTx} from '@ethereumjs/tx';
import {createVM,runTx} from '@ethereumjs/vm';
import {createAccount,createAddressFromPrivateKey,createAddressFromString,hexToBytes,bytesToHex} from '@ethereumjs/util';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const [directory,phase]=process.argv.slice(2),out=path.resolve(directory??'');
assert.ok(directory&&['prepare','points','direct_fold','state_proof'].includes(phase),'usage: compare NEW_RUN_DIR prepare|points|direct_fold|state_proof');
const read=p=>fs.readFileSync(path.resolve(root,p)),load=p=>JSON.parse(read(p));
const sha=b=>createHash('sha256').update(b).digest('hex'),hex=s=>'0x'+s,wire=h=>Buffer.from(h.replace(/^0x/,''),'hex');
const writeNew=(name,value)=>fs.writeFileSync(path.join(out,name),JSON.stringify(value,null,2)+'\n',{flag:'wx'});
const configPath='configs/experiment.G8C-FROZEN-P3-W32.json',config=load(configPath);
for(const p of config.source_pins)assert.equal(sha(read(p.path)),p.sha256,p.path);
const g7=load(config.source_pins[0].path),bf=load(config.source_pins[1].path),a=load(config.source_pins[2].path),g6=load(config.source_pins[4].path);
assert.deepEqual(a.public_context,g6.public_context);assert.deepEqual(bf.public_context,g6.public_context);
assert.equal(sha(wire(bf.ballot_vk_eip2537_hex)),config.ballot_eip_vk_sha256);
assert.equal(sha(wire(bf.state_vk_eip2537_hex)),config.state_eip_vk_sha256);
assert.equal(bf.batches.length,2);
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

const settings={optimizer:{enabled:true,runs:200},viaIR:true,evmVersion:'prague',outputSelection:{'*':{'*':['abi','evm.bytecode.object','evm.deployedBytecode.object']}}};
const sourcePaths=['auth/CanonicalVV.sol','auth/Sha512.sol','auth/JubjubSchnorrVerifier.sol','auth/VeriVotingCertificateGateV2.sol',
 'interfaces/IBatchVerifier.sol','Bls12381Groth16Verifier.sol','IndividualGroth16BatchVerifier.sol','G8CExperimentPins.sol',
 'VeriVotingAuthenticatedSettlement.sol','JubjubPointOps.sol','JubjubPointOpsHarness.sol','VeriVotingAuthenticatedDirectFoldSettlement.sol'];
const hashes=sourcePaths.map(p=>({path:'contracts/src/'+p,sha256:sha(read('contracts/src/'+p))}));
if(phase==='prepare'){
 assert.equal(solc.version().split('+')[0],'0.8.30');
 console.log('P0/P1 compile frozen original State receiver and independent Direct Fold/point modules');
 const sources=Object.fromEntries(sourcePaths.map(p=>[p,{content:read('contracts/src/'+p).toString('utf8')}]));
 const compiled=JSON.parse(solc.compile(JSON.stringify({language:'Solidity',sources,settings})));
 for(const e of compiled.errors??[]){if(e.severity==='error')throw Error(e.formattedMessage);console.warn(e.formattedMessage);}
 // No unlimited code-size or fork overrides. Record original compiler bytecode.
 const all=[];for(const [file,contracts]of Object.entries(compiled.contracts))for(const [name,v]of Object.entries(contracts))all.push({file,name,creation_bytes:v.evm.bytecode.object.length/2,runtime_bytes:v.evm.deployedBytecode.object.length/2});
 writeNew('compiled.json',{compiler:solc.version(),settings,source_hashes:hashes,contracts:compiled.contracts,bytecode_sizes:all});
 writeNew('source_pins.json',{config_path:configPath,config_sha256:sha(read(configPath)),fixture_pins:config.source_pins,contract_sources:hashes,
  ballot_vk_sha256:config.ballot_eip_vk_sha256,state_vk_sha256:config.state_eip_vk_sha256,
  matrix_sha256:sha(Buffer.concat(batches.flatMap(b=>b.ballotInputs.flatMap(r=>r.map(wire))))),
  ballot_proofs_sha256:sha(Buffer.concat(batches.flatMap(b=>b.ballotProofs.map(wire)))),
  state_proofs_sha256:sha(Buffer.concat(batches.map(b=>wire(b.stateProof))))});
 console.log('PASS compilation; no historical source edits, keys or proof generation');process.exit(0);
}
const artifact=JSON.parse(fs.readFileSync(path.join(out,'compiled.json')));
assert.deepEqual(artifact.source_hashes,hashes,'sources changed after compilation');assert.deepEqual(artifact.settings,settings);
const get=(f,n)=>artifact.contracts[f][n];
const gateCode=get('auth/VeriVotingCertificateGateV2.sol','VeriVotingCertificateGateV2');
const direct=phase==='direct_fold';
const receiverCode=get(direct?'VeriVotingAuthenticatedDirectFoldSettlement.sol':'VeriVotingAuthenticatedSettlement.sol',direct?'VeriVotingAuthenticatedDirectFoldSettlement':'VeriVotingAuthenticatedSettlement');
const gateAbi=new Interface(gateCode.abi),receiverAbi=new Interface(receiverCode.abi),abi=AbiCoder.defaultAbiCoder();
const common=new Common({chain:Mainnet,hardfork:Hardfork.Prague});assert.ok(common.isActivatedEIP(2537)&&common.isActivatedEIP(7623));
const vm=await createVM({common});
// Public deterministic local-test sender only. No RPC or public-chain deployment.
const senderKey=hexToBytes('0x'+'3c'.repeat(32)),sender=createAddressFromPrivateKey(senderKey);
await vm.stateManager.putAccount(sender,createAccount({nonce:0n,balance:10n**25n}));
const initialRoot=bytesToHex(await vm.stateManager.getStateRoot());
const block=createBlock({header:{number:1n,gasLimit:BigInt(config.block_gas_limit),baseFeePerGas:1n,timestamp:1_750_000_000n}},{common,skipConsensusFormatValidation:true});
let nonce=0n,trace=[];const stack=[],transactions=[];
const record={schema:'VV/STAGE2/EVM-FOLD-ROUTE/v1',phase,status:'RUNNING',initial_vm_state_root:initialRoot,hardfork:common.hardfork(),
 compiler:artifact.compiler,compiler_settings:settings,tx_gas_limit:config.transaction_gas_limit,block_gas_limit:config.block_gas_limit,
 cpu:os.cpus()[0].model,node:process.version,platform:os.platform(),arch:os.arch(),transactions,negatives:[],checkpoints:[],
 new_setup:0,new_proofs:0,new_p3:0,dkg_reexecuted:false,snarkpack_evm_verified:false,production_setup:false};
const partial=()=>fs.writeFileSync(path.join(out,phase+'.partial.json'),JSON.stringify(record,null,2)+'\n');
vm.evm.events.on('beforeMessage',m=>stack.push({address:m.to?.toString()??null,depth:stack.length,input_bytes:m.data.length,input_sha256:sha(m.data)}));
vm.evm.events.on('afterMessage',r=>{const m=stack.pop();if(m)trace.push({...m,success:!r.execResult.exceptionError,execution_gas:Number(r.execResult.executionGasUsed),return_hex:bytesToHex(r.execResult.returnValue)});});
function revertReason(r){const h=bytesToHex(r.execResult.returnValue);try {if(h.startsWith('0x08c379a0'))return abi.decode(['string'],'0x'+h.slice(10))[0];if(h.startsWith('0x4e487b71'))return 'PANIC:'+abi.decode(['uint256'],'0x'+h.slice(10))[0].toString();}catch{}return null;}
async function tx(label,data,to){
 trace=[];
 const signed=createLegacyTx({nonce:nonce++,gasLimit:BigInt(config.transaction_gas_limit),gasPrice:10n,value:0n,data:hexToBytes(data),...(to?{to}:{})},{common}).sign(senderKey);
 const t=performance.now(),r=await runTx(vm,{tx:signed,block});assert.equal(stack.length,0);
 const intrinsic=Number(signed.getIntrinsicGas()),execution=Number(r.execResult.executionGasUsed),floor=Number(signed.getMinimumGasLimit());
 // This concrete flow never clears storage/refunds; record and verify that fact.
 assert.equal(Number(r.gasRefund),0);assert.equal(Number(r.totalGasSpent),Math.max(intrinsic+execution,floor));
 const bytes=Buffer.from(signed.data),zeros=bytes.reduce((n,v)=>n+Number(v===0),0);
 const m={label,nonce:Number(signed.nonce),hash:bytesToHex(signed.hash()),signed_transaction_hex:bytesToHex(signed.serialize()),
  to:to?.toString()??null,status:r.receipt.status,gas_used:Number(r.totalGasSpent),receipt_cumulative_gas:Number(r.receipt.cumulativeBlockGasUsed),
  intrinsic_gas:intrinsic,execution_gas:execution,refund:Number(r.gasRefund),calldata_floor:floor,gas_limit:config.transaction_gas_limit,
  calldata_bytes:bytes.length,calldata_zero_bytes:zeros,calldata_nonzero_bytes:bytes.length-zeros,calldata_hex:bytesToHex(signed.data),
  calldata_sha256:sha(bytes),return_hex:bytesToHex(r.execResult.returnValue),exception:r.execResult.exceptionError?.error??null,revert_reason:revertReason(r),
  calls:[...trace],logs:r.receipt.logs.map(([address,topics,data])=>({address:bytesToHex(address),topics:topics.map(bytesToHex),data:bytesToHex(data)})),local_vm_wall_ms:performance.now()-t};
 assert.equal(m.receipt_cumulative_gas,m.gas_used,'each tx measured independently in the same local block context, not summed into one block');
 transactions.push(m);partial();console.log(phase+' '+label+': '+(m.status?'PASS':'REJECT '+(m.revert_reason??m.exception))+' gas='+m.gas_used);
 return {r,m};
}
async function view(address,iface,name,args=[]){
 await vm.evm.journal.checkpoint();let r;
 try{r=await vm.evm.runCall({caller:sender,to:address,data:hexToBytes(iface.encodeFunctionData(name,args)),gasLimit:20_000_000n,isStatic:true});}
 finally{await vm.evm.journal.revert();}
 return {r,value:r.execResult.exceptionError?null:iface.decodeFunctionResult(name,bytesToHex(r.execResult.returnValue))};
}
async function checkedView(...args){const v=await view(...args);assert.equal(v.r.execResult.exceptionError,undefined,'view '+args[2]);return v.value;}
async function deploy(label,code,args){const data=(await new ContractFactory(code.abi,'0x'+code.evm.bytecode.object).getDeployTransaction(...args)).data;const {r,m}=await tx(label,data);assert.equal(m.status,1,label+' '+m.exception);assert.ok(r.createdAddress);return r.createdAddress;}
function pairs(m){return m.calls.filter(c=>c.address?.endsWith('000f'));}
function requirePairs(m,n,last=1n){const ps=pairs(m);assert.equal(ps.length,n,m.label);assert.ok(ps.every(c=>c.success));if(n){assert.ok(ps.slice(0,-1).every(c=>BigInt(c.return_hex)===1n));assert.equal(BigInt(ps.at(-1).return_hex),last);}}
try {
 if(phase==='points'){
  const code=get('JubjubPointOpsHarness.sol','JubjubPointOpsHarness'),iface=new Interface(code.abi);
  const h=await deploy('deploy_point_test_harness',code,[]);record.harness=h.toString();
  const ref=JSON.parse(fs.readFileSync(path.join(out,'rust_points.json')));assert.equal(ref.status,'PASS_RUST_PUBLIC_REFERENCE');
  record.reference_sha256=sha(fs.readFileSync(path.join(out,'rust_points.json')));record.valid_probes=[];record.invalid_probes=[];
  for(const p of ref.roundtrips){const v=await checkedView(h,iface,'roundTrip',[hex(p)]);assert.equal(v[0],hex(p));record.valid_probes.push({kind:'roundtrip',input:p,output:v[0],matches_rust:true});}
  for(const c of ref.additions){const v=await checkedView(h,iface,'add',[hex(c.a),hex(c.b)]);assert.equal(v[0],hex(c.sum));record.valid_probes.push({kind:c.label,output:v[0],matches_rust:true});}
  const P=52435875175126190479447740508185965837690552500527637822603658699938581184513n;
  const le=n=>'0x'+Buffer.from(n.toString(16).padStart(64,'0'),'hex').reverse().toString('hex');
  for(const [label,p,reason]of [['y_not_in_field',le(P),'JUBJUB_ENCODING'],['identity_wrong_sign',le((1n<<255n)|1n),'JUBJUB_ENCODING'],['order_two_not_prime_subgroup',le(P-1n),'JUBJUB_SUBGROUP']]){
   const v=await view(h,iface,'roundTrip',[p]);assert.equal(v.r.execResult.exceptionError?.error,'revert');assert.equal(revertReason(v.r),reason);
   record.invalid_probes.push({label,reason,normal_revert:true});
  }
  record.rust_folds=ref.folds;record.status='PASS_RUST_EVM_POINT_CONFORMANCE';partial();
 } else {
  const points=JSON.parse(fs.readFileSync(path.join(out,'points.json')));assert.equal(points.status,'PASS_RUST_EVM_POINT_CONFORMANCE','P1 admission required');
  const gate=await deploy('deploy_certificate_gate',gateCode,[expected,a.authentication_keys.map(point),point(a.registration_authority),a.public_coefficients.map(point),hex(a.transcript_digest_hex)]);
  const receiver=await deploy('deploy_receiver',receiverCode,direct?[gate.toString(),binding,config.accepted_roots,bf.ballot_vk_eip2537_hex]:[gate.toString(),binding,config.accepted_roots,bf.ballot_vk_eip2537_hex,bf.state_vk_eip2537_hex]);
  record.addresses={gate:gate.toString(),receiver:receiver.toString()};
  assert.equal((await checkedView(receiver,receiverAbi,'certificateGate'))[0].toLowerCase(),gate.toString());
  assert.equal((await checkedView(receiver,receiverAbi,'bindingDigest'))[0],config.binding_digest);
  record.addresses.ballot_verifier=(await checkedView(receiver,receiverAbi,'batchVerifier'))[0].toLowerCase();
  if(!direct)record.addresses.state_verifier=(await checkedView(receiver,receiverAbi,'stateVerifier'))[0].toLowerCase();
  const nullifiers=batches.flatMap(b=>b.ballotInputs.map(row=>{const lo=BigInt(row[7]),last=BigInt(row[8]);return '0x'+Buffer.concat([Buffer.from(lo.toString(16).padStart(64,'0'),'hex').reverse().subarray(0,31),Buffer.from([Number(last)])]).toString('hex');}));
  assert.equal(new Set(nullifiers).size,8);
  async function checkpoint(){
   const s=await checkedView(receiver,receiverAbi,'checkpoint'),g=await checkedView(gate,gateAbi,'checkpoint');
   const spent=[];for(const nf of nullifiers)spent.push((await checkedView(receiver,receiverAbi,'spentNullifier',[nf]))[0]);
   return {next:Number(s[0]),spent_count:Number(s[1]),frozen:s[2],target:s[3],cp_target:s[4],state:[...s[5]],spent,
    gate:{certificate_count:Number(g[0]),dkg:g[1],registration:g[2],root:g[3],close:g[4],disclosure:g[5],authorized:g[6]}};
  }
  function data(b){if(!direct)return receiverAbi.encodeFunctionData('submitBatch',[b]);const {stateProof,...without}=b;return receiverAbi.encodeFunctionData('submitBatch',[without]);}
  async function reject(label,b,reason,n=0,last=1n){
   const before=await checkpoint(),{m}=await tx(label,data(b),receiver),after=await checkpoint();
   assert.equal(m.status,0,label);assert.equal(m.exception,'revert',label);assert.equal(m.revert_reason,reason,label);
   requirePairs(m,n,last);assert.equal(m.logs.length,0);assert.deepEqual(after,before,label+' application atomicity');
   record.negatives.push({case:label,reason,pairings:n,application_checkpoint_unchanged:true,before,after,tx_hash:m.hash});partial();
  }
  await reject('uncertified',batches[0],'UNCERTIFIED');
  for(let i=0;i<3;i++){const {m}=await tx('authenticate_dkg_'+(i+1),gateAbi.encodeFunctionData('submitDkgSignature',[i+1,binding.electionId,binding.finalContext,sig(a.dkg_certificate_signatures[i])]),gate);assert.equal(m.status,1);}
  {const {m}=await tx('authenticate_registration',gateAbi.encodeFunctionData('authenticateRegistration',[close,sig(a.registration_signature)]),gate);assert.equal(m.status,1);}
  const certified=await checkpoint();assert.ok(certified.gate.authorized);record.certified_checkpoint=certified;
  let bad=structuredClone(batches[0]);bad.newState[1]=batches[1].newState[1];assert.notEqual(bad.newState[1],batches[0].newState[1]);
  await reject('wrong_new_state_before_first_acceptance',bad,direct?'DIRECT_FOLD_MISMATCH':'INVALID_GROTH16',direct?4:5,direct?1n:0n);
  bad=structuredClone(batches[0]);bad.oldState[0]=batches[0].newState[0];await reject('wrong_old_state',bad,'OLD_STATE');
  bad=structuredClone(batches[0]);bad.root='0x'+'00'.repeat(32);await reject('wrong_batch_root',bad,'ACCEPTED_ROOT');
  bad=structuredClone(batches[0]);bad.ballotProofs[0]=bad.ballotProofs[1];await reject('another_voter_proof',bad,'INVALID_GROTH16',1,0n);
  bad=structuredClone(batches[0]);bad.ballotInputs[0][13]='0x'+(BigInt(bad.ballotInputs[0][13])^1n).toString(16).padStart(64,'0');await reject('changed_ciphertext_encoding_in_committed_matrix',bad,'BATCH_ROOT');
  for(let i=0;i<2;i++){
   const {m}=await tx('accept_batch_'+(i+1),data(batches[i]),receiver);assert.equal(m.status,1);requirePairs(m,direct?4:5);
   const s=await checkpoint();assert.deepEqual(s.state,batches[i].newState);assert.equal(s.spent_count,(i+1)*4);
   assert.deepEqual(s.state,points.rust_folds[i].state_hex.map(hex));record.checkpoints.push(s);partial();
   if(i===0){await reject('replay_first_batch',batches[0],'NEXT_INDEX');bad=structuredClone(batches[1]);bad.ballotInputs[0][7]=batches[0].ballotInputs[0][7];bad.ballotInputs[0][8]=batches[0].ballotInputs[0][8];await reject('cross_batch_spent_nullifier',bad,'SPENT_NULLIFIER');}
  }
  {const {m}=await tx('freeze_ct2',receiverAbi.encodeFunctionData('freeze'),receiver);assert.equal(m.status,1);}
  const frozen=await checkpoint();assert.equal(frozen.target,bf.expected_freeze_digest_hex.startsWith('0x')?bf.expected_freeze_digest_hex:hex(bf.expected_freeze_digest_hex));
  assert.equal(frozen.cp_target,hex(c.cp_target_hex));assert.ok(frozen.frozen);assert.deepEqual(frozen.spent,Array(8).fill(true));record.frozen_checkpoint=frozen;
  await reject('receive_after_freeze',batches[1],'FROZEN');
  record.final_checkpoint=await checkpoint();assert.deepEqual(record.final_checkpoint,frozen);
  const accepted=transactions.filter(t=>t.label.startsWith('accept_batch_'));
  record.accepted_batch_gas=accepted.reduce((s,t)=>s+t.gas_used,0);record.accepted_batch_calldata_bytes=accepted.reduce((s,t)=>s+t.calldata_bytes,0);
  record.deployment_gas=transactions.filter(t=>t.label.startsWith('deploy_')).reduce((s,t)=>s+t.gas_used,0);
  record.freeze_gas=transactions.find(t=>t.label==='freeze_ct2').gas_used;
  record.gas_component_notes='Call trace costs are inclusive and nested: never sum parents with children. Inline Direct Fold cost not separately instrumented; null, not zero. Primary comparison is full signed receipt gas.';
  record.direct_fold_only_gas=null;
  record.state_verifier_call_gas=direct?null:accepted.map(t=>t.calls.find(c=>c.address===record.addresses.state_verifier)?.execution_gas??null);
  record.common_ballot_verifier_call_gas=accepted.map(t=>t.calls.find(c=>c.address===record.addresses.ballot_verifier)?.execution_gas??null);
  record.stage7_encoding_negative_scope='End-to-end ciphertext edit rejects at BATCH_ROOT; separate point module checks canonical encoding and subgroup rejection.';
  record.status='PASS_TWO_B4_ACCEPTANCE_AND_FREEZE';
 }
}catch(e){record.status='FAIL_OR_RESOURCE_BLOCKED';record.error=String(e);process.exitCode=1;}
finally{
 record.finished_utc=new Date().toISOString();record.peak_node_rss_mib=process.resourceUsage().maxRSS/1024;
 partial();writeNew(phase+'.json',record);console.log(phase+' '+record.status+(record.error?' '+record.error:''));
}
