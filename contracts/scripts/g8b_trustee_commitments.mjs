// G8-B: same-run original certificate gate + real frozen receiver + public records.
// NOT P3/Winner integration and NOT exact SnarkPack EVM verification.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import solc from 'solc';
import {ContractFactory,Interface,keccak256,AbiCoder} from 'ethers';
import {Common,Hardfork,Mainnet} from '@ethereumjs/common';
import {createBlock} from '@ethereumjs/block';
import {createLegacyTx} from '@ethereumjs/tx';
import {createVM,runTx} from '@ethereumjs/vm';
import {createAccount,createAddressFromPrivateKey,createAddressFromString,hexToBytes,bytesToHex} from '@ethereumjs/util';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const read=p=>fs.readFileSync(path.resolve(root,p)),load=p=>JSON.parse(read(p));
const sha=b=>createHash('sha256').update(b).digest('hex');
const meta=(c,a)=>execFileSync(c,a,{cwd:root,encoding:'utf8'}).trim();
const raw=process.argv[2]??'experiments/g8b_raw',resultPath=`${raw}/trustee_commitments.json`,artifactPath=`${raw}/contract_artifacts.json`;
for(const p of[resultPath,artifactPath])assert.equal(fs.existsSync(path.resolve(root,p)),false,`new output required: ${p}`);
const manifestPath='configs/protocol_manifest.JUBJUB-G8B-TRUSTEE-COMMITMENTS-W32.json';
const fixturePath='fixtures/g8b_trustee_commitments_public_v1.json',exportPath='experiments/g8b_raw/public_export.json';
const config=load(manifestPath),f=load(fixturePath),exported=load(exportPath);
for(const p of [...config.source_pins,...exported.source_pins,...exported.source_hashes])assert.equal(sha(read(p.path)),p.sha256,p.path);
assert.equal(exported.status,'PASS_FIVE_ARCHIVED_G6_PACKAGE_SIGNATURES_AND_KEYCORE_SHARES');
assert.equal(sha(read(fixturePath)),exported.output_fixture_sha256);
assert.equal(f.schema,'VV/G8-B/ORIGINAL-TRUSTEE-COMMITMENTS/PUBLIC/v1');
const certPath=f.source_g8a_fixture,receiverPath='fixtures/g8_receiver_eip2537_v1.json';
const a=load(certPath),batchesFixture=load(receiverPath),c=f.public_context;
assert.deepEqual(c,a.public_context);assert.deepEqual(c,batchesFixture.public_context);
const sourcePaths=['auth/CanonicalVV.sol','auth/Sha512.sol','auth/JubjubSchnorrVerifier.sol','auth/VeriVotingCertificateGate.sol',
 'auth/JubjubVerificationShare.sol','auth/VeriVotingTrusteeCommitments.sol','VeriVotingSettlement.sol',
 'IndividualGroth16BatchVerifier.sol','Bls12381Groth16Verifier.sol','interfaces/IBatchVerifier.sol'].map(p=>`contracts/src/${p}`);
const sources=Object.fromEntries(sourcePaths.map(p=>[p.replace('contracts/src/',''),{content:read(p).toString('utf8')}]));
const settings={optimizer:{enabled:true,runs:200},viaIR:true,evmVersion:'prague',outputSelection:{'*':{'*':['abi','evm.bytecode.object','evm.deployedBytecode.object']}}};
console.log('G8-B [1] compile original certificate/receiver sources and new commitment storage');
const compilation=JSON.parse(solc.compile(JSON.stringify({language:'Solidity',sources,settings})));
for(const e of compilation.errors??[]){if(e.severity==='error')throw new Error(e.formattedMessage);console.warn(e.formattedMessage);}
const code=(file,name)=>compilation.contracts[file][name];
const gateCode=code('auth/VeriVotingCertificateGate.sol','VeriVotingCertificateGate'),receiverCode=code('VeriVotingSettlement.sol','VeriVotingSettlement');
const commitmentCode=code('auth/VeriVotingTrusteeCommitments.sol','VeriVotingTrusteeCommitments');
const mathCode=code('auth/JubjubSchnorrVerifier.sol','JubjubSchnorrVerifier');
const gateAbi=new Interface(gateCode.abi),receiverAbi=new Interface(receiverCode.abi),commitAbi=new Interface(commitmentCode.abi),mathAbi=new Interface(mathCode.abi);
const common=new Common({chain:Mainnet,hardfork:Hardfork.Prague}),vm=await createVM({common});assert.ok(common.isActivatedEIP(2537));
// Explicit public LOCAL-ONLY sender. No RPC or public network is used.
const senderKey=hexToBytes(`0x${'3b'.repeat(32)}`),sender=createAddressFromPrivateKey(senderKey);
await vm.stateManager.putAccount(sender,createAccount({nonce:0n,balance:10n**25n}));
const block=createBlock({header:{number:1n,gasLimit:BigInt(config.block_gas_limit),baseFeePerGas:1n,timestamp:1_750_000_000n}},{common,skipConsensusFormatValidation:true});
let nonce=0n,trace=[];const stack=[];
vm.evm.events.on('beforeMessage',m=>stack.push({address:m.to?.toString(),input_bytes:m.data.length,input_sha256:sha(m.data)}));
vm.evm.events.on('afterMessage',r=>{const m=stack.pop();if(m&&['0002','000c','000f'].includes(m.address?.slice(-4)))trace.push({...m,
 success:!r.execResult.exceptionError,execution_gas:Number(r.execResult.executionGasUsed),return_hex:bytesToHex(r.execResult.returnValue)});});
const transactions=[];
async function tx(label,data,to){
 trace=[];const signed=createLegacyTx({nonce:nonce++,gasLimit:BigInt(config.transaction_gas_limit),gasPrice:10n,value:0n,data:hexToBytes(data),...(to?{to}:{})},{common}).sign(senderKey);
 const start=performance.now(),r=await runTx(vm,{tx:signed,block});
 const execution=Number(r.execResult.executionGasUsed),intrinsic=Number(signed.getIntrinsicGas()),minimum=Number(signed.getMinimumGasLimit());
 assert.equal(stack.length,0);assert.equal(Number(r.gasRefund),0);assert.equal(Number(r.totalGasSpent),Math.max(intrinsic+execution,minimum));
 let reason=null;if(r.execResult.exceptionError?.error==='revert')reason=commitAbi.parseError(bytesToHex(r.execResult.returnValue))?.args[0]??null;
 const measurement={label,nonce:Number(signed.nonce),transaction_hash:bytesToHex(signed.hash()),signed_transaction_hex:bytesToHex(signed.serialize()),
  to:to?.toString()??null,receipt_status:r.receipt.status,total_gas:Number(r.totalGasSpent),execution_gas:execution,intrinsic_gas:intrinsic,gas_refund:Number(r.gasRefund),
  minimum_gas_limit_including_calldata_floor:minimum,calldata_floor_supplement_gas:Number(r.totalGasSpent)-execution-intrinsic,transaction_gas_limit:config.transaction_gas_limit,
  calldata_bytes:signed.data.length,calldata_hex:bytesToHex(signed.data),calldata_sha256:sha(signed.data),return_hex:bytesToHex(r.execResult.returnValue),
  exception:r.execResult.exceptionError?.error??null,revert_reason:reason,precompile_calls:[...trace],local_vm_wall_ms:performance.now()-start,
  logs:r.receipt.logs.map(([address,topics,data])=>({address:bytesToHex(address),topics:topics.map(bytesToHex),data:bytesToHex(data)}))};
 transactions.push(measurement);return {result:r,measurement};
}
async function view(address,abi,name,args=[]){
 const before=(await vm.stateManager.getAccount(sender)).nonce;await vm.evm.journal.checkpoint();
 let r;try{r=await vm.evm.runCall({caller:sender,to:address,data:hexToBytes(abi.encodeFunctionData(name,args)),gasLimit:20_000_000n,isStatic:true});}
 finally{await vm.evm.journal.revert();}
 assert.equal((await vm.stateManager.getAccount(sender)).nonce,before);assert.equal(r.execResult.exceptionError,undefined,`view ${name}`);
 return abi.decodeFunctionResult(name,bytesToHex(r.execResult.returnValue));
}
async function deploy(label,compiled,args){
 const factory=new ContractFactory(compiled.abi,`0x${compiled.evm.bytecode.object}`);
 const r=await tx(label,(await factory.getDeployTransaction(...args)).data);assert.equal(r.measurement.receipt_status,1,`${label}: ${JSON.stringify(r.measurement)}`);
 assert.ok(r.result.createdAddress);return {address:r.result.createdAddress,measurement:r.measurement};
}
const hex=s=>`0x${s}`,point=p=>({x:p.x,y:p.y,encoded:hex(p.encoded_hex)});
const sig=s=>({nonce:point(s.nonce),response:s.response_scalar,counter:s.counter});
const flatten=state=>state.flatMap(r=>[hex(r.c0_hex),hex(r.c1_hex)]);
const expected={electionId:hex(c.election_id_hex),manifestDigest:hex(a.manifest_digest_hex),policyPre:hex(c.policy_pre_hex),prekeyContext:hex(c.prekey_context_hex),
 dkgSession:hex(c.dkg_session_hex),keyCoreDigest:hex(c.key_core_digest_hex),finalContext:hex(c.final_context_hex),publicKey:hex(c.public_key_hex),
 registrationRoot:hex(c.registration_root_hex),closeDigest:hex(c.registration_close_digest_hex),disclosureDigest:hex(c.disclosure_digest_hex)};
const close={electionId:hex(a.close.election_id_hex),finalContext:hex(a.close.final_context_hex),policyPre:hex(a.close.policy_pre_hex),root:hex(a.close.root_hex),
 nRegistered:a.close.n_registered,wRegistered:a.close.w_registered,wMax:a.close.w_max,version:a.close.version,canonicalRegistryDigest:hex(a.close.canonical_registry_digest_hex)};
const binding={electionId:expected.electionId,policyPre:expected.policyPre,dkgSession:expected.dkgSession,keyCoreDigest:expected.keyCoreDigest,
 publicKey:expected.publicKey,finalContext:expected.finalContext,registrationRoot:expected.registrationRoot,registrationCloseDigest:expected.closeDigest,
 disclosureDigest:expected.disclosureDigest,p3VkSha256:hex(batchesFixture.p3_vk_ark_sha256),prekeyContext:expected.prekeyContext,registrationAuthority:hex(c.registration_authority_hex),
 registrationCertificateSha256:hex(sha(Buffer.from(c.registration_certificate_hex,'hex'))),region:4,registeredWeight:32};
const roots=batchesFixture.batches.map(b=>hex(b.root_hex));
const batches=batchesFixture.batches.map(b=>({index:b.batch_index,root:hex(b.root_hex),oldState:flatten(b.old_state),newState:flatten(b.new_state),
 ballotProofs:b.ballot_proofs_eip2537_hex,ballotInputs:b.ballot_inputs_be_hex,stateProof:b.state_proof_eip2537_hex}));
console.log('G8-B [2] deploy original gate, real individual-ballot/state receiver and commitment component');
const gate=await deploy('deploy_certificate_gate',gateCode,[expected,a.authentication_keys.map(point),point(a.registration_authority),a.public_coefficients.map(point),hex(a.transcript_digest_hex)]);
const receiver=await deploy('deploy_real_receiver',receiverCode,[binding,roots,batchesFixture.ballot_vk_eip2537_hex,batchesFixture.state_vk_eip2537_hex]);
const store=await deploy('deploy_commitment_component',commitmentCode,[gate.address.toString(),receiver.address.toString(),binding,hex(a.manifest_digest_hex),hex(a.transcript_digest_hex),a.authentication_keys.map(point),a.public_coefficients.map(point)]);
assert.equal((await view(receiver.address,receiverAbi,'bindingDigest'))[0],keccak256(AbiCoder.defaultAbiCoder().encode([receiverCode.abi.find(x=>x.type==='constructor').inputs[0]],[binding])));
assert.equal((await view(store.address,commitAbi,'keyCoreDigest'))[0],expected.keyCoreDigest);
assert.equal((await view(store.address,commitAbi,'Q_CERT'))[0],3n);assert.equal((await view(store.address,commitAbi,'TAU'))[0],3n);
const math=createAddressFromString((await view(gate.address,gateAbi,'verifier'))[0]);
const challenges=[];
for(const r of f.records){const actual=await view(math,mathAbi,'signatureChallenge',[hex(r.authentication_key.encoded_hex),hex(r.signature.nonce.encoded_hex),hex(f.original_domain_hex),hex(r.original_message_hex)]);
 assert.deepEqual([actual[0].toString(),Number(actual[1]),actual[2]],[r.original_challenge_scalar,r.signature.counter,true]);
 challenges.push({trustee_id:r.trustee_id,challenge:actual[0].toString(),counter:Number(actual[1]),matched_original:true});}
async function snapshot(){
 const s=await view(store.address,commitAbi,'checkpoint'),masks=[],commitments=[],shares=[];
 for(let id=1;id<=5;id++){masks.push((await view(store.address,commitAbi,'submitted',[id]))[0]);commitments.push((await view(store.address,commitAbi,'packageCommitment',[id]))[0]);shares.push((await view(store.address,commitAbi,'verifiedShare',[id]))[0]);}
 const g=await view(gate.address,gateAbi,'checkpoint'),r=await view(receiver.address,receiverAbi,'checkpoint');
 return {bound:s[0],freeze_seal:s[1],cp_target:s[2],count:Number(s[3]),p3_ready:s[4],submitted:masks,commitments,verification_shares:shares,
  gate_authorized:g[6],receiver_next:Number(r[0]),receiver_spent:Number(r[1]),receiver_frozen:r[2],receiver_seal:r[3],receiver_target:r[4],receiver_state:[...r[5]]};
}
const negatives=[];
async function reject(name,data,reason){
 const before=await snapshot(),r=await tx(name,data,store.address);assert.equal(r.measurement.receipt_status,0,name);assert.equal(r.measurement.exception,'revert',name);
 assert.equal(r.measurement.revert_reason,reason,name);assert.equal(r.measurement.logs.length,0);const after=await snapshot();assert.deepEqual(after,before,`${name} atomic`);
 negatives.push({case:name,transaction_nonce:r.measurement.nonce,atomic_rejection:true,before,after});console.log(`  reject ${name}: ${reason}`);
}
const bind=()=>commitAbi.encodeFunctionData('bindFrozen');
await reject('unauthenticated_context_cannot_bind',bind(),'UNCERTIFIED');
console.log('G8-B [3] real original certificate authentication; no resigned messages');
for(let i=0;i<3;i++){const r=await tx(`authenticate_dkg_${i+1}`,gateAbi.encodeFunctionData('submitDkgSignature',[i+1,expected.electionId,expected.finalContext,sig(a.dkg_certificate_signatures[i])]),gate.address);assert.equal(r.measurement.receipt_status,1);}
const registration=await tx('authenticate_registration',gateAbi.encodeFunctionData('authenticateRegistration',[close,sig(a.registration_signature)]),gate.address);assert.equal(registration.measurement.receipt_status,1);
await reject('unfrozen_receiver_cannot_bind',bind(),'NOT_FROZEN');
console.log('G8-B [4] accept two original B4 batches, freeze live CT2, bind authenticated context');
for(const [i,b]of batches.entries()){
 const r=await tx(`accept_batch_${i+1}`,receiverAbi.encodeFunctionData('submitBatch',[b]),receiver.address);assert.equal(r.measurement.receipt_status,1);
 const pairs=r.measurement.precompile_calls.filter(p=>p.address.endsWith('000f'));assert.equal(pairs.length,5);assert.ok(pairs.every(p=>p.success&&BigInt(p.return_hex)===1n));
}
const freezing=await tx('freeze_ct2',receiverAbi.encodeFunctionData('freeze'),receiver.address);assert.equal(freezing.measurement.receipt_status,1);
const bound=await tx('bind_authenticated_frozen_ct2',bind(),store.address);assert.equal(bound.measurement.receipt_status,1);
const frozen=await snapshot();assert.equal(frozen.freeze_seal,hex(f.freeze_digest_hex));assert.equal(frozen.cp_target,hex(c.cp_target_hex));assert.ok(frozen.gate_authorized&&frozen.receiver_frozen);
assert.deepEqual([...await view(store.address,commitAbi,'frozenCiphertexts')][0].map(String),flatten(c.frozen_ciphertexts));
const submission=r=>({trusteeId:r.trustee_id,electionId:expected.electionId,finalContext:expected.finalContext,target:hex(c.cp_target_hex),
 verificationShare:point(r.verification_share),commitment:point(r.commitment),signature:sig(r.signature)});
const submit=s=>commitAbi.encodeFunctionData('submitCommitment',[s]),flip=h=>hex((BigInt(h)^1n).toString(16).padStart(64,'0'));
console.log('G8-B [5] original package signatures, KeyCore share check and minimal necessary negatives');
let bad=submission(f.records[0]);bad.signature.response=(BigInt(bad.signature.response)+1n).toString();await reject('forged_package_signature',submit(bad),'BAD_PACKAGE_SIGNATURE');
bad=submission(f.records[0]);bad.commitment=point(f.records[1].commitment);await reject('changed_package_commitment',submit(bad),'BAD_PACKAGE_SIGNATURE');
bad=submission(f.records[0]);bad.target=flip(bad.target);await reject('changed_cp_target',submit(bad),'CP_TARGET');
bad=submission(f.records[0]);bad.finalContext=flip(bad.finalContext);await reject('changed_final_context',submit(bad),'FINAL_CONTEXT');
bad=submission(f.records[0]);bad.electionId=flip(bad.electionId);await reject('changed_election_id',submit(bad),'ELECTION_ID');
bad=submission(f.records[0]);bad.verificationShare=point(f.records[1].verification_share);await reject('substituted_valid_verification_share',submit(bad),'VERIFICATION_SHARE');
const accepted=[];
for(let i=0;i<3;i++){
 const row=f.records[i],r=await tx(`authenticate_commitment_${i+1}`,submit(submission(row)),store.address);assert.equal(r.measurement.receipt_status,1);
 const s=await snapshot();assert.equal(s.count,i+1);assert.equal(s.p3_ready,i===2);assert.equal(s.commitments[i],hex(row.commitment.encoded_hex));assert.equal(s.verification_shares[i],hex(row.verification_share.encoded_hex));
 assert.ok(r.measurement.precompile_calls.some(p=>p.address.endsWith('0002')&&p.return_hex===hex(row.original_message_hex)),'original native commitment_message exact match in EVM trace');
 accepted.push({trustee_id:i+1,transaction_nonce:r.measurement.nonce,checkpoint:s});console.log(`  Trustee ${i+1}: ${r.measurement.total_gas} Gas / ${r.measurement.calldata_bytes} B`);
 if(i===0)await reject('duplicate_trustee_id',submit(submission(row)),'DUPLICATE_TRUSTEE');
 if(i===1)await reject('two_trustees_cannot_enter_p3',commitAbi.encodeFunctionData('requireP3Ready'),'DECRYPTION_QUORUM');
}
const ready=await tx('three_trustees_ready_for_future_p3',commitAbi.encodeFunctionData('requireP3Ready'),store.address);assert.equal(ready.measurement.receipt_status,1);
const final=await snapshot();assert.equal(final.count,3);assert.equal(final.p3_ready,true);assert.deepEqual(final.submitted,[true,true,true,false,false]);
const deployed=[['certificate_gate',gate.address,gateCode],['original_schnorr_verifier',math,mathCode],['receiver',receiver.address,receiverCode],['trustee_commitments',store.address,commitmentCode]];
const artifacts={compiler:solc.version(),compiler_settings:settings,contracts:{},creation_bytecodes:{}};
for(const[name,address,compiled]of deployed){const runtime=await vm.stateManager.getCode(address);assert.ok(runtime.length>0&&runtime.length<=24576);artifacts.contracts[name]={address:address.toString(),abi:compiled.abi,
 runtime_bytecode_hex:bytesToHex(runtime),runtime_bytecode_bytes:runtime.length,runtime_sha256:sha(runtime),runtime_keccak256:keccak256(runtime)};
 if(name!=='original_schnorr_verifier')artifacts.creation_bytecodes[name]={hex:`0x${compiled.evm.bytecode.object}`,bytes:compiled.evm.bytecode.object.length/2};}
const hashedPaths=[...sourcePaths,manifestPath,'contracts/scripts/g8b_trustee_commitments.mjs','scripts/g8b_record.mjs','contracts/package-lock.json',
 'crates/vv-friendly-dkg/src/bin/g8b_trustee_export.rs','Cargo.lock'];
const sourceHashes=hashedPaths.map(p=>({path:p,sha256:sha(read(p))}));
// Immutable executed bytes, including unchanged imported contract sources.
for(const p of hashedPaths){const out=path.resolve(root,raw,'provenance',`${p.replaceAll('/','__')}.executed.txt`);fs.mkdirSync(path.dirname(out),{recursive:true});fs.writeFileSync(out,read(p),{flag:'wx'});}
const result={gate:'G8-B',status:'PASS_ORIGINAL_TRUSTEE_COMMITMENTS_AUTHENTICATED_FOR_LIVE_FROZEN_CT2',scope:'G8-B component only; not P3/winner/full settlement',
 recorded_at_utc:new Date().toISOString(),git_commit:meta('git',['rev-parse','HEAD']),git_dirty:meta('git',['status','--porcelain']).length>0,branch:meta('git',['branch','--show-current']),
 cpu:os.cpus()[0].model,os:os.platform(),arch:os.arch(),node:process.version,compiler:solc.version(),compiler_settings:settings,hardfork:common.hardfork(),
 fixture:fixturePath,fixture_sha256:sha(read(fixturePath)),export_report:exportPath,export_report_sha256:sha(read(exportPath)),
 certificate_fixture:certPath,receiver_fixture:receiverPath,source_pins:config.source_pins,source_hashes:sourceHashes,
 n:5,t:2,q_cert:3,tau:3,transactions,transaction_count:transactions.length,original_challenge_checks:challenges,
 accepted_commitments:accepted,negative_tests:negatives,frozen_checkpoint:frozen,final_checkpoint:final,
 original_package_signatures_verified_evm:3,verification_share_polynomial_checks_evm:3,archived_g6_public_records_reused:5,
 original_signature_bytes_preserved:true,signature_scheme_replaced:false,new_signatures:0,new_cp_packages:0,dkg_reexecuted:false,
 setup_count:0,proof_generation_count:0,reused_ballot_proofs:8,reused_state_proofs:2,private_witness_emitted:false,production_setup:false,
 receiver_auth_gate_enforced_on_intake:false,commitment_component_enforces_authorized_frozen_context:true,
 p3_storage_binding_integrated:false,p3_verified_in_this_flow:false,winner_written:false,complete_g8_settlement:false,snarkpack_evm_verified:false,full_system_gas:null,
 peak_node_rss_mib:process.resourceUsage().maxRSS/1024,execution_scope:'signed transactions in one local Prague VM, no RPC/public-chain deployment',
 limitations:['authorized gate/receiver deployment addresses and original bootstrap keys remain external trust roots',
 'original receiver intake does not enforce certificates; G8-C must integrate the gate before claiming full authenticated intake',
 'three authenticated PUBLIC records only; readiness is not CP/P3 verification, private decryption or permission to write Winner',
 'no openings/usable decryption shares/tally in calldata; private validity still requires later P3',
 'same fixed all-five QUAL / D5-W32 research profile; no production ceremony, no exact SnarkPack EVM',
 'one functional local execution, not independent performance statistics or a full-system cost comparison']};
function writeNew(p,v){fs.mkdirSync(path.dirname(path.resolve(root,p)),{recursive:true});fs.writeFileSync(path.resolve(root,p),`${JSON.stringify(v,null,2)}\n`,{flag:'wx'});}
writeNew(artifactPath,artifacts);result.contract_artifact=artifactPath;result.contract_artifact_sha256=sha(read(artifactPath));writeNew(resultPath,result);
console.log(`G8-B PASS: 3 original authenticated commitments; ${negatives.length} atomic negatives; ${transactions.length} actual signed transactions; full G8 PARTIAL`);
