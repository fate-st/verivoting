// G8 phase 1: REAL individual Groth16 fallback + state proofs + freeze, NOT full G8.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import solc from 'solc';
import {ContractFactory, Interface, keccak256, AbiCoder} from 'ethers';
import {Common, Hardfork, Mainnet} from '@ethereumjs/common';
import {createBlock} from '@ethereumjs/block';
import {createLegacyTx} from '@ethereumjs/tx';
import {createAccount, createAddressFromPrivateKey, hexToBytes, bytesToHex} from '@ethereumjs/util';
import {createVM, runTx} from '@ethereumjs/vm';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const read=p=>fs.readFileSync(path.resolve(root,p));
const load=p=>JSON.parse(read(p));
const sha=b=>createHash('sha256').update(b).digest('hex');
const metadata=(c,a)=>execFileSync(c,a,{cwd:root,encoding:'utf8'}).trim();
const outputDir=process.argv[2]??'experiments/g8_raw';
const resultPath=`${outputDir}/receiver_freeze.json`,artifactPath=`${outputDir}/contract_artifacts.json`;
for(const p of[resultPath,artifactPath])assert.equal(fs.existsSync(path.resolve(root,p)),false,`new path required: ${p}`);
const manifestPath='configs/protocol_manifest.JUBJUB-G8-RECEIVER-FALLBACK-W32.json';
const fixturePath='fixtures/g8_receiver_eip2537_v1.json',exportPath='experiments/g8_raw/public_export.json';
const config=load(manifestPath),fixture=load(fixturePath),exported=load(exportPath);
for(const p of [...config.source_pins,...exported.source_hashes])assert.equal(sha(read(p.path)),p.sha256,p.path);
assert.equal(exported.status,'PASS_EIGHT_BALLOTS_TWO_STATE_PROOFS_NATIVE_REPLAY_AND_EIP2537_EXPORT');
assert.equal(exported.output_fixture_sha256,sha(read(fixturePath)));
assert.equal(fixture.schema,'VV/G8/RECEIVER-FREEZE-EIP2537/v1');
assert.equal(fixture.batch_validity_route,'individual-groth16-not-snarkpack');
assert.equal(fixture.setup_count,0);assert.equal(fixture.proof_generation_count,0);
assert.equal(fixture.batches.length,2);
const sourcePaths=['contracts/src/VeriVotingSettlement.sol','contracts/src/IndividualGroth16BatchVerifier.sol',
 'contracts/src/Bls12381Groth16Verifier.sol','contracts/src/interfaces/IBatchVerifier.sol'];
const sources=Object.fromEntries(sourcePaths.map(p=>[p.replace('contracts/src/',''),{content:read(p).toString('utf8')}]));
const settings={optimizer:{enabled:true,runs:200},viaIR:true,evmVersion:'prague',
 outputSelection:{'*':{'*':['abi','evm.bytecode.object','evm.deployedBytecode.object']}}};
console.log('G8 [1] compile actual receiver and concrete ballot/state verifiers (no mock)');
const compilation=JSON.parse(solc.compile(JSON.stringify({language:'Solidity',sources,settings})));
for(const e of compilation.errors??[]){if(e.severity==='error')throw new Error(e.formattedMessage);console.warn(e.formattedMessage);}
const compiled=compilation.contracts['VeriVotingSettlement.sol'].VeriVotingSettlement;
const verifierCompiled=compilation.contracts['Bls12381Groth16Verifier.sol'].Bls12381Groth16Verifier;
const adapterCompiled=compilation.contracts['IndividualGroth16BatchVerifier.sol'].IndividualGroth16BatchVerifier;
const iface=new Interface(compiled.abi),verifierIface=new Interface(verifierCompiled.abi),adapterIface=new Interface(adapterCompiled.abi);
const factory=new ContractFactory(compiled.abi,`0x${compiled.evm.bytecode.object}`);
const common=new Common({chain:Mainnet,hardfork:Hardfork.Prague});assert.ok(common.isActivatedEIP(2537));
const vm=await createVM({common});
// Public deterministic LOCAL ONLY sender, no RPC or public network.
const senderKey=hexToBytes(`0x${'38'.repeat(32)}`),sender=createAddressFromPrivateKey(senderKey);
await vm.stateManager.putAccount(sender,createAccount({nonce:0n,balance:10n**25n}));
const block=createBlock({header:{number:1n,gasLimit:BigInt(config.block_gas_limit),baseFeePerGas:1n,timestamp:1_750_000_000n}},
 {common,skipConsensusFormatValidation:true});
let nonce=0n,trace=[];const stack=[];
vm.evm.events.on('beforeMessage',m=>stack.push({address:m.to?.toString(),input_bytes:m.data.length,input_sha256:sha(m.data)}));
vm.evm.events.on('afterMessage',r=>{const m=stack.pop();if(m&&['0002','000c','000f'].includes(m.address?.slice(-4)))trace.push({...m,
 execution_gas:Number(r.execResult.executionGasUsed),success:!r.execResult.exceptionError,
 return_hex:bytesToHex(r.execResult.returnValue),exception:r.execResult.exceptionError?.error??null});});
async function transact(data,to){
 trace=[];const tx=createLegacyTx({nonce:nonce++,gasLimit:BigInt(config.transaction_gas_limit),gasPrice:10n,value:0n,
  data:hexToBytes(data),...(to?{to}:{})},{common}).sign(senderKey);
 const start=performance.now(),result=await runTx(vm,{tx,block});const elapsed=performance.now()-start;
 assert.equal(stack.length,0);const intrinsic=Number(tx.getIntrinsicGas()),execution=Number(result.execResult.executionGasUsed);
 const minimum=Number(tx.getMinimumGasLimit()),total=Number(result.totalGasSpent);
 assert.equal(Number(result.gasRefund),0);assert.equal(total,Math.max(intrinsic+execution,minimum));
 let reason=null;if(result.execResult.exceptionError?.error==='revert')reason=iface.parseError(bytesToHex(result.execResult.returnValue))?.args[0]??null;
 return {result,measurement:{nonce:Number(tx.nonce),transaction_hash:bytesToHex(tx.hash()),to:to?.toString()??null,
 receipt_status:result.receipt.status,total_gas:total,execution_gas:execution,intrinsic_gas:intrinsic,
 minimum_gas_limit_including_calldata_floor:minimum,calldata_floor_supplement_gas:total-intrinsic-execution,gas_refund:Number(result.gasRefund),
 transaction_gas_limit:config.transaction_gas_limit,calldata_bytes:tx.data.length,calldata_sha256:sha(tx.data),calldata_hex:bytesToHex(tx.data),
 local_vm_wall_ms:elapsed,exception:result.execResult.exceptionError?.error??null,revert_reason:reason,precompile_calls:[...trace],logs:result.receipt.logs.map(([a,t,d])=>({address:bytesToHex(a),topics:t.map(bytesToHex),data:bytesToHex(d)}))}};
}
async function view(address,abi,name,args=[]){
 // runCall increments the top-level caller nonce even for isStatic; emulate
 // eth_call by rolling back ALL journal/account/warm-access changes afterwards.
 await vm.evm.journal.checkpoint();
 try {
  const r=await vm.evm.runCall({caller:sender,to:address,data:hexToBytes(abi.encodeFunctionData(name,args)),gasLimit:20_000_000n,isStatic:true});
  assert.equal(r.execResult.exceptionError,undefined,`view ${name}`);
  return abi.decodeFunctionResult(name,bytesToHex(r.execResult.returnValue));
 } finally { await vm.evm.journal.revert(); }
}
const hex=s=>`0x${s}`,flatten=state=>state.flatMap(r=>[hex(r.c0_hex),hex(r.c1_hex)]);
const c=fixture.public_context;
const binding={electionId:hex(c.election_id_hex),policyPre:hex(c.policy_pre_hex),dkgSession:hex(c.dkg_session_hex),
 keyCoreDigest:hex(c.key_core_digest_hex),publicKey:hex(c.public_key_hex),finalContext:hex(c.final_context_hex),
 registrationRoot:hex(c.registration_root_hex),registrationCloseDigest:hex(c.registration_close_digest_hex),disclosureDigest:hex(c.disclosure_digest_hex),
 p3VkSha256:hex(fixture.p3_vk_ark_sha256),prekeyContext:hex(c.prekey_context_hex),registrationAuthority:hex(c.registration_authority_hex),
 registrationCertificateSha256:hex(sha(Buffer.from(c.registration_certificate_hex,'hex'))),region:4,registeredWeight:32};
const roots=fixture.batches.map(b=>hex(b.root_hex));
const batches=fixture.batches.map(b=>({index:b.batch_index,root:hex(b.root_hex),oldState:flatten(b.old_state),newState:flatten(b.new_state),
 ballotProofs:b.ballot_proofs_eip2537_hex,ballotInputs:b.ballot_inputs_be_hex,stateProof:b.state_proof_eip2537_hex}));
for(const b of batches){assert.equal(b.ballotInputs.length,4);assert.ok(b.ballotInputs.every(x=>x.length===25));}
console.log('G8 [2] deploy receiver; constructor really deploys its own verifiers / immutable anchors');
const deployment=await transact((await factory.getDeployTransaction(binding,roots,fixture.ballot_vk_eip2537_hex,fixture.state_vk_eip2537_hex)).data);
assert.equal(deployment.result.receipt.status,1);
const settlement=deployment.result.createdAddress;assert.ok(settlement);
const {createAddressFromString}=await import('@ethereumjs/util');
const batchVerifier=createAddressFromString((await view(settlement,iface,'batchVerifier'))[0]);
const stateVerifier=createAddressFromString((await view(settlement,iface,'stateVerifier'))[0]);
const ballotVerifier=createAddressFromString((await view(batchVerifier,adapterIface,'verifier'))[0]);
assert.equal((await view(ballotVerifier,verifierIface,'publicInputCount'))[0],25n);
assert.equal((await view(stateVerifier,verifierIface,'publicInputCount'))[0],37n);
assert.equal((await view(batchVerifier,adapterIface,'verificationKeyDigest'))[0],keccak256(fixture.ballot_vk_eip2537_hex));
assert.equal((await view(stateVerifier,verifierIface,'verificationKeyDigest'))[0],keccak256(fixture.state_vk_eip2537_hex));
const bindingParam=compiled.abi.find(x=>x.type==='constructor').inputs[0];
const bindingDigest=keccak256(AbiCoder.defaultAbiCoder().encode([bindingParam],[binding]));
assert.equal((await view(settlement,iface,'bindingDigest'))[0],bindingDigest);
const rawState=load('fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json');
const nullifiers=rawState.batches.flatMap(b=>b.source_public_records.map(r=>hex(r.nullifier_hex)));
async function snapshot(){
 const s=await view(settlement,iface,'checkpoint');
 const spent=[];for(const nf of nullifiers)spent.push((await view(settlement,iface,'spentNullifier',[nf]))[0]);
 return {next_batch_index:Number(s[0]),spent_count:Number(s[1]),frozen:s[2],freeze_digest:s[3],cp_target:s[4],state:[...s[5]],spent_nullifiers:spent};
}
const cases=[],initial=await snapshot();assert.deepEqual(initial.state,batches[0].oldState);
const submit=b=>iface.encodeFunctionData('submitBatch',[b]);
const freeze=()=>iface.encodeFunctionData('freeze');
const clone=structuredClone;
const pairings=t=>t.precompile_calls.filter(x=>x.address.endsWith('000f'));
async function reject(name,data,reason,expectedPairings=0){
 const before=await snapshot(),tx=await transact(data,settlement);
 assert.equal(tx.measurement.receipt_status,0,name);assert.equal(tx.measurement.revert_reason,reason,name);
 assert.equal(tx.measurement.exception,'revert');assert.equal(tx.measurement.logs.length,0);
 assert.deepEqual(await snapshot(),before,`${name}: no state/NF writes`);
 const p=pairings(tx.measurement);assert.equal(p.length,expectedPairings,name);
 if(p.length){assert.ok(tx.measurement.precompile_calls.every(x=>x.success));assert.equal(BigInt(p.at(-1).return_hex),0n);}
 cases.push({case:name,expected:'reject',atomic_rejection:true,...tx.measurement});
 console.log(`  reject ${name}: ${reason}, ${tx.measurement.total_gas} Gas`);
}
console.log('G8 [3] necessary preflight/root/nullifier/proof negatives, no writes on rejection');
await reject('premature_freeze',freeze(),'INCOMPLETE');
await reject('skip_first_batch',submit(batches[1]),'NEXT_INDEX');
let bad=clone(batches[0]);bad.oldState[0]=batches[0].newState[0];await reject('changed_old_state',submit(bad),'OLD_STATE');
bad=clone(batches[0]);bad.ballotInputs[0][5]=`0x${(BigInt(bad.ballotInputs[0][5])+1n).toString(16).padStart(64,'0')}`;
await reject('wrong_registration_root',submit(bad),'REGISTRATION_ROOT');
bad=clone(batches[0]);bad.root=`0x${(BigInt(bad.root)^1n).toString(16).padStart(64,'0')}`;await reject('changed_schedule_root',submit(bad),'ACCEPTED_ROOT');
bad=clone(batches[0]);[bad.ballotInputs[0],bad.ballotInputs[1]]=[bad.ballotInputs[1],bad.ballotInputs[0]];
[bad.ballotProofs[0],bad.ballotProofs[1]]=[bad.ballotProofs[1],bad.ballotProofs[0]];await reject('reordered_valid_ballots',submit(bad),'BATCH_ROOT');
bad=clone(batches[0]);bad.ballotInputs[1]=clone(bad.ballotInputs[0]);bad.ballotProofs[1]=bad.ballotProofs[0];await reject('duplicate_within_batch',submit(bad),'DUPLICATE_NULLIFIER');
bad=clone(batches[0]);bad.ballotProofs[0]=bad.ballotProofs[1];await reject('wrong_ballot_proof',submit(bad),'INVALID_GROTH16',1);
bad=clone(batches[0]);bad.stateProof=batches[1].stateProof;await reject('wrong_state_proof_after_four_valid_ballots',submit(bad),'INVALID_GROTH16',5);
console.log('G8 [4] accept both real consecutive batches; verify actual ten proofs on EVM');
const accepted=[];
for(const [i,b] of batches.entries()){
 const tx=await transact(submit(b),settlement);assert.equal(tx.measurement.receipt_status,1);
 assert.equal(pairings(tx.measurement).length,5);assert.ok(pairings(tx.measurement).every(p=>BigInt(p.return_hex)===1n));
 assert.ok(tx.measurement.precompile_calls.every(p=>p.success));
 const s=await snapshot();assert.equal(s.next_batch_index,i+2);assert.equal(s.spent_count,(i+1)*4);
 assert.deepEqual(s.state,b.newState);assert.deepEqual(s.spent_nullifiers,nullifiers.map((_,j)=>j<(i+1)*4));
 accepted.push({batch_index:i+1,checkpoint:s,...tx.measurement});
 console.log(`  batch ${i+1} accepted: ${tx.measurement.total_gas} Gas / ${tx.measurement.calldata_bytes} B`);
 if(i===0){
  await reject('replayed_first_batch',submit(b),'NEXT_INDEX');
  const repeated=clone(batches[1]);repeated.ballotInputs[0]=clone(b.ballotInputs[0]);repeated.ballotProofs[0]=b.ballotProofs[0];
  await reject('spent_nullifier_across_batches',submit(repeated),'SPENT_NULLIFIER');
 }
}
console.log('G8 [5] freeze CT2 from storage; require exact ORIGINAL G6 CP target and seal');
const freezing=await transact(freeze(),settlement);assert.equal(freezing.measurement.receipt_status,1);
const final=await snapshot();assert.equal(final.frozen,true);
assert.deepEqual(final.state,flatten(fixture.expected_final_state));
assert.equal(final.cp_target,hex(c.cp_target_hex));assert.equal(final.freeze_digest,hex(fixture.expected_freeze_digest_hex));
await reject('repeat_freeze',freeze(),'FROZEN');await reject('receive_after_freeze',submit(batches[1]),'FROZEN');
assert.deepEqual(await snapshot(),final);
const contracts={};
for(const[name,address,abi]of[['settlement',settlement,compiled.abi],['batch_adapter',batchVerifier,adapterCompiled.abi],
 ['ballot_verifier',ballotVerifier,verifierCompiled.abi],['state_verifier',stateVerifier,verifierCompiled.abi]]){
 const code=await vm.stateManager.getCode(address);assert.ok(code.length>0&&code.length<=24576);
 contracts[name]={address:address.toString(),abi,runtime_bytecode_hex:bytesToHex(code),runtime_bytecode_bytes:code.length,runtime_sha256:sha(code),runtime_keccak256:keccak256(code)};
}
const artifacts={compiler:solc.version(),compiler_settings:settings,source_hashes:sourcePaths.map(p=>({path:p,sha256:sha(read(p))})),
 creation_bytecode_hex:`0x${compiled.evm.bytecode.object}`,creation_bytecode_bytes:compiled.evm.bytecode.object.length/2,contracts};
const sourceHashes=[...sourcePaths,manifestPath,'contracts/scripts/g8_settlement.mjs','contracts/package.json','contracts/package-lock.json',
 'crates/vv-friendly-dkg/src/bin/g8_receiver_export.rs','crates/vv-friendly-dkg/Cargo.toml','crates/vv-friendly-crypto/src/state_gadget.rs','Cargo.lock']
 .map(p=>({path:p,sha256:sha(read(p))}));
const result={gate:'G8',phase:'receiver-and-freeze-only',status:'PARTIAL_G8_REAL_INDIVIDUAL_BALLOT_STATE_EVM_CT2_FREEZE_PASS',
 recorded_at_utc:new Date().toISOString(),git_commit:metadata('git',['rev-parse','HEAD']),git_dirty:metadata('git',['status','--porcelain']).length>0,
 branch:metadata('git',['branch','--show-current']),cpu:os.cpus()[0].model,os:os.platform(),arch:os.arch(),node:process.version,
 hardfork:common.hardfork(),compiler:solc.version(),compiler_settings:settings,dependency_lock_sha256:sha(read('contracts/package-lock.json')),
 fixture:fixturePath,fixture_sha256:sha(read(fixturePath)),export_report:exportPath,export_report_sha256:sha(read(exportPath)),
 source_pins:config.source_pins,source_hashes:sourceHashes,initial_binding:binding,binding_digest:bindingDigest,accepted_roots:roots,
 ballot_vk_ark_sha256:fixture.ballot_vk_ark_sha256,state_vk_ark_sha256:fixture.state_vk_ark_sha256,p3_vk_ark_sha256:fixture.p3_vk_ark_sha256,
 batch_validity_route:'real individual Groth16, four 25-input statements per B4; NOT SnarkPack',
 batch_validity_checked_on_evm:true,state_proofs_checked_on_evm:true,verifier_addresses_caller_injectable:false,
 setup_count:0,proof_generation_count:0,reused_ballot_proofs:8,reused_state_proofs:2,
 deployment:deployment.measurement,initial_checkpoint:initial,accepted_batches:accepted,negative_tests:cases,
 freeze:freezing.measurement,final_checkpoint:final,exact_g6_freeze_seal_matched:true,exact_g6_cp_target_matched:true,
 transaction_count:Number(nonce),peak_node_rss_mib:process.resourceUsage().maxRSS/1024,
 execution_scope:'signed independent transactions in local in-memory Prague VM; no RPC/public-chain deployment',
 initialization_certificate_checked_on_evm:false,trustee_authentication_checked_on_evm:false,p3_verified_in_this_flow:false,
 winner_written:false,complete_g8_settlement:false,snarkpack_evm_verified:false,full_system_gas:null,production_setup:false,
 private_witness_emitted:false,software_miller_route_reopened:false,
 remaining_tests:{forged_trustee_signature:'NOT_IMPLEMENTED',changed_commitment:'NOT_IMPLEMENTED',winner_proof:'NOT_IMPLEMENTED',repeat_winner:'NOT_IMPLEMENTED'},
 limitations:['phase 1 receiver/freeze only, not complete Winner-only contract settlement',
 'initial DKG/registry/certificate/schedule anchors externally provisioned by deployer, NOT authenticated certificates verified on-chain',
 'all four original ballot proofs are verified individually; no aggregation efficiency or SnarkPack EVM completion claim',
 'local fixed two-B4 schedule is not a censorship-resistant accepted-intake log',
 'research deterministic Groth16 setup is not a production trusted ceremony; no private witness output',
 'one functional execution, not independent performance statistics; do not splice G7/old W10 gas into a full-system total',
 'Trustee private delivery still same-process in G6; original Jubjub Schnorr authentication still needs contract integration']};
function writeNew(p,v){fs.mkdirSync(path.dirname(path.resolve(root,p)),{recursive:true});fs.writeFileSync(path.resolve(root,p),`${JSON.stringify(v,null,2)}\n`,{flag:'wx'});}
writeNew(artifactPath,artifacts);result.contract_artifact=artifactPath;result.contract_artifact_sha256=sha(read(artifactPath));writeNew(resultPath,result);
console.log(`G8 phase 1 PASS (G8 remains PARTIAL): deployment ${result.deployment.total_gas}; batches ${accepted.map(b=>b.total_gas).join('/')}; freeze ${result.freeze.total_gas}; ${cases.length} atomic rejections; NO setup/prove; G6 seal matches`);
