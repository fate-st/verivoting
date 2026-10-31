// G8-A: original Jubjub/SHA-512 certificate authentication, standalone gate ONLY.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import solc from 'solc';
import {ContractFactory,Interface,keccak256} from 'ethers';
import {Common,Hardfork,Mainnet} from '@ethereumjs/common';
import {createBlock} from '@ethereumjs/block';
import {createLegacyTx} from '@ethereumjs/tx';
import {createVM,runTx} from '@ethereumjs/vm';
import {createAccount,createAddressFromPrivateKey,createAddressFromString,hexToBytes,bytesToHex} from '@ethereumjs/util';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const read=p=>fs.readFileSync(path.resolve(root,p)),load=p=>JSON.parse(read(p));
const sha=b=>createHash('sha256').update(b).digest('hex');
const meta=(c,a)=>execFileSync(c,a,{cwd:root,encoding:'utf8'}).trim();
const raw=process.argv[2]??'experiments/g8a_raw',resultPath=`${raw}/certificate_auth.json`,artifactPath=`${raw}/contract_artifacts.json`;
for(const p of[resultPath,artifactPath])assert.equal(fs.existsSync(path.resolve(root,p)),false,`new output required ${p}`);
const manifestPath='configs/protocol_manifest.JUBJUB-G8A-CERT-AUTH-W32.json';
const fixturePath='fixtures/g8a_original_certificates_public_v1.json',exportPath='experiments/g8a_raw/public_export.json';
const config=load(manifestPath),f=load(fixturePath),exported=load(exportPath);
for(const p of [...config.source_pins,...exported.source_pins,...exported.source_hashes])assert.equal(sha(read(p.path)),p.sha256,p.path);
assert.equal(exported.status,'PASS_ORIGINAL_DKG_AND_CLOSE_CERTIFICATE_NATIVE_RECONSTRUCTION');
assert.equal(sha(read(fixturePath)),exported.output_fixture_sha256);assert.equal(f.signatures_resigned,false);
assert.equal(f.schema,'VV/G8-A/ORIGINAL-CERTIFICATES/PUBLIC/v1');
assert.deepEqual(f.qual,[1,2,3,4,5]);assert.equal(f.q_cert,f.n-f.t);assert.equal(f.tau,f.t+1);
const names=['CanonicalVV.sol','Sha512.sol','JubjubSchnorrVerifier.sol','VeriVotingCertificateGate.sol'];
const sourcePaths=names.map(n=>`contracts/src/auth/${n}`);
const sources=Object.fromEntries(names.map((n,i)=>[n,{content:read(sourcePaths[i]).toString('utf8')}]));
const settings={optimizer:{enabled:true,runs:200},viaIR:true,evmVersion:'prague',outputSelection:{'*':{'*':['abi','evm.bytecode.object','evm.deployedBytecode.object']}}};
console.log('G8-A [1] compile original-scheme curve/hash implementation and certificate gate');
const compilation=JSON.parse(solc.compile(JSON.stringify({language:'Solidity',sources,settings})));
for(const e of compilation.errors??[]){if(e.severity==='error')throw new Error(e.formattedMessage);console.warn(e.formattedMessage);}
const gateCode=compilation.contracts['VeriVotingCertificateGate.sol'].VeriVotingCertificateGate;
const mathCode=compilation.contracts['JubjubSchnorrVerifier.sol'].JubjubSchnorrVerifier;
const gateAbi=new Interface(gateCode.abi),mathAbi=new Interface(mathCode.abi);
const factory=new ContractFactory(gateCode.abi,`0x${gateCode.evm.bytecode.object}`);
const common=new Common({chain:Mainnet,hardfork:Hardfork.Prague}),vm=await createVM({common});
// Explicitly public, local-only research sender. No RPC/network transaction.
const senderKey=hexToBytes(`0x${'3a'.repeat(32)}`),sender=createAddressFromPrivateKey(senderKey);
await vm.stateManager.putAccount(sender,createAccount({nonce:0n,balance:10n**25n}));
const block=createBlock({header:{number:1n,gasLimit:BigInt(config.block_gas_limit),baseFeePerGas:1n,timestamp:1_750_000_000n}},{common,skipConsensusFormatValidation:true});
let nonce=0n,trace=[];const stack=[];
vm.evm.events.on('beforeMessage',m=>stack.push({address:m.to?.toString(),input_bytes:m.data.length,input_sha256:sha(m.data)}));
vm.evm.events.on('afterMessage',r=>{const m=stack.pop();if(m?.address?.endsWith('0002'))trace.push({...m,success:!r.execResult.exceptionError,execution_gas:Number(r.execResult.executionGasUsed),return_hex:bytesToHex(r.execResult.returnValue)});});
async function tx(data,to){
 trace=[];const signed=createLegacyTx({nonce:nonce++,gasLimit:BigInt(config.transaction_gas_limit),gasPrice:10n,value:0n,data:hexToBytes(data),...(to?{to}:{})},{common}).sign(senderKey);
 const start=performance.now(),r=await runTx(vm,{tx:signed,block});
 const execution=Number(r.execResult.executionGasUsed),intrinsic=Number(signed.getIntrinsicGas()),minimum=Number(signed.getMinimumGasLimit());
 assert.equal(Number(r.gasRefund),0);assert.equal(Number(r.totalGasSpent),Math.max(intrinsic+execution,minimum));
 let reason=null;if(r.execResult.exceptionError?.error==='revert')reason=gateAbi.parseError(bytesToHex(r.execResult.returnValue))?.args[0]??null;
 return {result:r,measurement:{nonce:Number(signed.nonce),transaction_hash:bytesToHex(signed.hash()),receipt_status:r.receipt.status,
  total_gas:Number(r.totalGasSpent),execution_gas:execution,intrinsic_gas:intrinsic,gas_refund:Number(r.gasRefund),minimum_gas_limit_including_calldata_floor:minimum,
  calldata_floor_supplement_gas:Number(r.totalGasSpent)-execution-intrinsic,transaction_gas_limit:config.transaction_gas_limit,
  calldata_bytes:signed.data.length,calldata_hex:bytesToHex(signed.data),calldata_sha256:sha(signed.data),
  return_hex:bytesToHex(r.execResult.returnValue),exception:r.execResult.exceptionError?.error??null,revert_reason:reason,sha256_precompile_calls:[...trace],local_vm_wall_ms:performance.now()-start}};
}
async function view(address,abi,name,args=[]){
 const before=(await vm.stateManager.getAccount(sender)).nonce;
 await vm.evm.journal.checkpoint();
 let r;try{r=await vm.evm.runCall({caller:sender,to:address,data:hexToBytes(abi.encodeFunctionData(name,args)),gasLimit:20_000_000n,isStatic:true});}
 finally{await vm.evm.journal.revert();}
 assert.equal((await vm.stateManager.getAccount(sender)).nonce,before,'view nonce isolation');
 assert.equal(r.execResult.exceptionError,undefined,`view ${name}`);
 return {decoded:abi.decodeFunctionResult(name,bytesToHex(r.execResult.returnValue)),execution_gas:Number(r.execResult.executionGasUsed)};
}
const hex=s=>`0x${s}`,point=p=>({x:p.x,y:p.y,encoded:hex(p.encoded_hex)});
const sig=r=>({nonce:point(r.nonce),response:r.response_scalar,counter:r.counter});
const c=f.public_context;
const expected={electionId:hex(c.election_id_hex),manifestDigest:hex(f.manifest_digest_hex),policyPre:hex(c.policy_pre_hex),prekeyContext:hex(c.prekey_context_hex),
 dkgSession:hex(c.dkg_session_hex),keyCoreDigest:hex(c.key_core_digest_hex),finalContext:hex(c.final_context_hex),publicKey:hex(c.public_key_hex),
 registrationRoot:hex(c.registration_root_hex),closeDigest:hex(c.registration_close_digest_hex),disclosureDigest:hex(c.disclosure_digest_hex)};
const close={electionId:hex(f.close.election_id_hex),finalContext:hex(f.close.final_context_hex),policyPre:hex(f.close.policy_pre_hex),root:hex(f.close.root_hex),
 nRegistered:f.close.n_registered,wRegistered:f.close.w_registered,wMax:f.close.w_max,version:f.close.version,canonicalRegistryDigest:hex(f.close.canonical_registry_digest_hex)};
console.log('G8-A [2] deploy; validate original subgroup/encoding and reconstruct policy / KeyCore');
const deployment=await tx((await factory.getDeployTransaction(expected,f.authentication_keys.map(point),point(f.registration_authority),f.public_coefficients.map(point),hex(f.transcript_digest_hex))).data);
if(deployment.measurement.receipt_status!==1)console.log(JSON.stringify(deployment.measurement));
assert.equal(deployment.measurement.receipt_status,1,'certificate gate deployment');
const gate=deployment.result.createdAddress;assert.ok(gate);
const math=createAddressFromString((await view(gate,gateAbi,'verifier')).decoded[0]);
assert.equal((await view(gate,gateAbi,'keyCoreDigest')).decoded[0],expected.keyCoreDigest);
assert.equal((await view(gate,gateAbi,'finalContext')).decoded[0],expected.finalContext);
assert.equal((await view(gate,gateAbi,'Q_CERT')).decoded[0],3n);assert.equal((await view(gate,gateAbi,'TAU')).decoded[0],3n);
console.log('G8-A [3] actual EVM SHA-512 padding vectors and all original signature challenge bytes');
const shaChecks=[];
for(const [name,data]of [['empty',Buffer.alloc(0)],['abc',Buffer.from('abc')],... [111,112,128,257].map(n=>[`length_${n}`,Buffer.from(Array.from({length:n},(_,i)=>i%256))])]){
 const r=await view(math,mathAbi,'hashSha512',[hex(data.toString('hex'))]),expected=hex(createHash('sha512').update(data).digest('hex'));
 assert.equal(r.decoded[0],expected,name);shaChecks.push({case:name,input_hex:data.toString('hex'),expected_digest:expected,actual_digest:r.decoded[0],view_execution_gas:r.execution_gas});
}
const challengeChecks=[];
for(const row of [...f.dkg_certificate_signatures,f.registration_signature]){
 for(const h of row.hash_attempts){const r=await view(math,mathAbi,'hashSha512',[hex(h.input_hex)]);assert.equal(r.decoded[0],hex(h.sha512_hex));shaChecks.push({case:`original_signature_counter_${h.counter}`,input_hex:h.input_hex,expected_digest:hex(h.sha512_hex),actual_digest:r.decoded[0],view_execution_gas:r.execution_gas});}
 const r=await view(math,mathAbi,'signatureChallenge',[hex(row.public_key.encoded_hex),hex(row.nonce.encoded_hex),hex(row.domain_hex),hex(row.message_hex)]);
 assert.deepEqual([r.decoded[0].toString(),Number(r.decoded[1]),r.decoded[2]],[row.challenge_scalar,row.counter,true]);
 challengeChecks.push({domain_hex:row.domain_hex,message_hex:row.message_hex,challenge_scalar:r.decoded[0].toString(),counter:Number(r.decoded[1]),view_execution_gas:r.execution_gas});
}
const P=BigInt((await view(math,mathAbi,'P')).decoded[0]);
const le32=v=>hex(Buffer.from(v.toString(16).padStart(64,'0'),'hex').reverse().toString('hex'));
assert.equal((await view(math,mathAbi,'validatePoint',[{x:0,y:(P-1n).toString(),encoded:le32(P-1n)},false])).decoded[0],false,'order-2 subgroup point rejected');
assert.equal((await view(math,mathAbi,'validatePoint',[{x:0,y:1,encoded:le32(1n)},false])).decoded[0],false,'identity rejected');
async function snapshot(){
 const s=(await view(gate,gateAbi,'checkpoint')).decoded,mask=[];
 for(let i=1;i<=5;i++)mask.push((await view(gate,gateAbi,'certifiedSigner',[i])).decoded[0]);
 return {count:Number(s[0]),dkg_certified:s[1],registration_certified:s[2],root:s[3],close_digest:s[4],disclosure_digest:s[5],authorized:s[6],signer_mask:mask};
}
const dkgRows=f.dkg_certificate_signatures,regSig=sig(f.registration_signature),negatives=[],accepted=[];
const dkgData=(id,eid,context,s)=>gateAbi.encodeFunctionData('submitDkgSignature',[id,eid,context,s]);
const regData=(cl,s)=>gateAbi.encodeFunctionData('authenticateRegistration',[cl,s]);
async function reject(name,data,reason){
 const before=await snapshot(),r=await tx(data,gate);assert.equal(r.measurement.receipt_status,0,name);assert.equal(r.measurement.revert_reason,reason,name);
 assert.equal(r.measurement.exception,'revert');assert.deepEqual(await snapshot(),before,`${name} atomic`);
 negatives.push({case:name,atomic_rejection:true,...r.measurement});console.log(`  reject ${name}: ${reason}`);
}
const flip=h=>hex((BigInt(h)^1n).toString(16).padStart(64,'0'));
console.log('G8-A [4] original DKG signatures / distinct certification quorum, necessary rejects');
let bad=sig(dkgRows[0]);bad.response=(BigInt(bad.response)+1n).toString();await reject('forged_dkg_signature',dkgData(1,expected.electionId,expected.finalContext,bad),'BAD_DKG_SIGNATURE');
await reject('wrong_election_id',dkgData(1,flip(expected.electionId),expected.finalContext,sig(dkgRows[0])),'ELECTION_ID');
await reject('wrong_dkg_context',dkgData(1,expected.electionId,flip(expected.finalContext),sig(dkgRows[0])),'DKG_CONTEXT');
bad=sig(dkgRows[0]);bad.counter++;await reject('wrong_first_valid_counter',dkgData(1,expected.electionId,expected.finalContext,bad),'BAD_DKG_SIGNATURE');
for(let i=0;i<3;i++){
 const r=await tx(dkgData(i+1,expected.electionId,expected.finalContext,sig(dkgRows[i])),gate);assert.equal(r.measurement.receipt_status,1);
 const s=await snapshot();assert.equal(s.count,i+1);assert.equal(s.dkg_certified,i===2);assert.equal(s.authorized,false);
 accepted.push({role:'DKG_CERTIFICATION',trustee_id:i+1,checkpoint:s,...r.measurement});
 console.log(`  DKG certifier ${i+1}: ${r.measurement.total_gas} Gas`);
 if(i===0)await reject('duplicate_certifier',dkgData(1,expected.electionId,expected.finalContext,sig(dkgRows[0])),'DUPLICATE_CERTIFIER');
 if(i===1)await reject('insufficient_key_certification_quorum',regData(close,regSig),'DKG_QUORUM');
}
await reject('registration_not_yet_authenticated',gateAbi.encodeFunctionData('requireAuthorized'),'UNCERTIFIED');
console.log('G8-A [5] authenticate original registration certificate and exact original close/disclosure digests');
await reject('registration_wrong_election',regData({...close,electionId:flip(close.electionId)},regSig),'REGISTRATION_CONTEXT');
await reject('registration_wrong_context',regData({...close,finalContext:flip(close.finalContext)},regSig),'REGISTRATION_CONTEXT');
await reject('wrong_registration_close_digest',regData({...close,canonicalRegistryDigest:flip(close.canonicalRegistryDigest)},regSig),'REGISTRATION_CLOSE_DIGEST');
bad=structuredClone(regSig);bad.response=(BigInt(bad.response)+1n).toString();await reject('forged_registration_signature',regData(close,bad),'BAD_REGISTRATION_SIGNATURE');
const registration=await tx(regData(close,regSig),gate);assert.equal(registration.measurement.receipt_status,1);
const final=await snapshot();assert.equal(final.authorized,true);assert.equal(final.root,expected.registrationRoot);assert.equal(final.close_digest,expected.closeDigest);assert.equal(final.disclosure_digest,expected.disclosureDigest);
await reject('repeat_registration_certificate',regData(close,regSig),'REGISTRATION_ALREADY_CERTIFIED');
const authorized=await tx(gateAbi.encodeFunctionData('requireAuthorized'),gate);assert.equal(authorized.measurement.receipt_status,1);
assert.deepEqual(await snapshot(),final);
const contracts={};for(const[name,address,abi]of[['certificate_gate',gate,gateCode.abi],['original_schnorr_verifier',math,mathCode.abi]]){
 const code=await vm.stateManager.getCode(address);assert.ok(code.length>0&&code.length<=24576);contracts[name]={address:address.toString(),abi,runtime_bytecode_hex:bytesToHex(code),runtime_bytecode_bytes:code.length,runtime_sha256:sha(code),runtime_keccak256:keccak256(code)};}
const artifacts={compiler:solc.version(),compiler_settings:settings,contracts,creation_bytecode_hex:`0x${gateCode.evm.bytecode.object}`,creation_bytecode_bytes:gateCode.evm.bytecode.object.length/2};
const result={gate:'G8-A',status:'PASS_ORIGINAL_JUBJUB_DKG_AND_REGISTRATION_CERTIFICATES_LOCAL_PRAGUE',scope:'standalone certificate gate; receiver/P3/winner not integrated',
 recorded_at_utc:new Date().toISOString(),git_commit:meta('git',['rev-parse','HEAD']),git_dirty:meta('git',['status','--porcelain']).length>0,branch:meta('git',['branch','--show-current']),
 cpu:os.cpus()[0].model,os:os.platform(),arch:os.arch(),node:process.version,compiler:solc.version(),compiler_settings:settings,hardfork:common.hardfork(),dependency_lock_sha256:sha(read('contracts/package-lock.json')),
 fixture:fixturePath,fixture_sha256:sha(read(fixturePath)),export_report:exportPath,export_report_sha256:sha(read(exportPath)),source_pins:[...config.source_pins,...exported.source_pins],
 source_hashes:[...sourcePaths,manifestPath,'contracts/scripts/g8a_certificate_auth.mjs','contracts/package.json','contracts/package-lock.json','crates/vv-friendly-dkg/Cargo.toml','crates/vv-friendly-dkg/src/bin/g8a_certificate_export.rs','Cargo.lock'].map(p=>({path:p,sha256:sha(read(p))})),
 n:5,t:2,q_cert:3,tau:3,distinct_dkg_signatures_verified_evm:3,original_registration_signatures_verified_evm:1,
 deployment:deployment.measurement,sha512_checks:shaChecks,original_challenge_checks:challengeChecks,order_two_point_rejected:true,identity_point_rejected:true,
 accepted_dkg_signatures:accepted,registration:registration.measurement,authorization:authorized.measurement,negative_tests:negatives,final_checkpoint:final,
 transaction_count:Number(nonce),keycore_matches_original_g6:true,close_and_disclosure_match_original_g6:true,signature_scheme_replaced:false,signatures_resigned:false,
 groth16_setup_count:0,groth16_proof_generation_count:0,private_witness_emitted:false,production_setup:false,peak_node_rss_mib:process.resourceUsage().maxRSS/1024,
 execution_scope:'signed independent transactions in local Prague VM; no RPC/public-chain deployment',
 receiver_auth_gate_integrated:false,trustee_commitment_storage_integrated:false,p3_storage_binding_integrated:false,winner_written:false,complete_g8_settlement:false,snarkpack_evm_verified:false,full_system_gas:null,
 limitations:['bootstrap trustee identity keys/registration authority remain externally authorized trust roots',
 'fixed n5/t2, all-five QUAL and D5/W32 certificate profile, not a generic arbitrary-committee contract',
 'standalone authentication state only; old phase-one receiver does NOT yet enforce this gate',
 'Trustee package signatures/commitments, private openings inside P3 and winner write-once still require integration',
 'research deterministic DKG/keys/setup, no production or privacy-theorem claim',
 'single functional transaction per case; hash/challenge view calls are NOT transaction gas samples or statistics']};
function writeNew(p,v){fs.mkdirSync(path.dirname(path.resolve(root,p)),{recursive:true});fs.writeFileSync(path.resolve(root,p),`${JSON.stringify(v,null,2)}\n`,{flag:'wx'});}
writeNew(artifactPath,artifacts);result.contract_artifact=artifactPath;result.contract_artifact_sha256=sha(read(artifactPath));writeNew(resultPath,result);
console.log(`G8-A PASS: deploy ${result.deployment.total_gas}; DKG ${accepted.map(x=>x.total_gas).join('/')}; original close ${registration.measurement.total_gas}; ${negatives.length} atomic rejections; FULL G8 REMAINS PARTIAL`);
