// Explicitly gated Stage1 runner. Gate A ONLY compiles; no automatic B/C/D.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {spawn,execFileSync} from 'node:child_process';
import solc from 'solc';
import {ContractFactory,Interface,AbiCoder,keccak256} from 'ethers';
import {Common,Hardfork,Mainnet} from '@ethereumjs/common';
import {createVM,runTx} from '@ethereumjs/vm';
import {createBlock} from '@ethereumjs/block';
import {createLegacyTx} from '@ethereumjs/tx';
import {createAccount,createAddressFromPrivateKey,createAddressFromString,hexToBytes,bytesToHex} from '@ethereumjs/util';
export const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
export const configPath='configs/experiment.STAGE1-MULTIBATCH.json';
export const sourceFiles=[configPath,'crates/vv-friendly-dkg/src/bin/stage1_multibatch_materials.rs',
 'contracts/src/stage1/Stage1Receivers.sol','contracts/scripts/stage1_multibatch_compare.mjs','scripts/stage1_multibatch_audit.mjs'];
export const read=p=>fs.readFileSync(path.resolve(root,p));
export const load=p=>JSON.parse(read(p));
export const sha=b=>createHash('sha256').update(b).digest('hex');
export const wire=s=>Buffer.from(s.replace(/^0x/,''),'hex');
export const hex=s=>s.startsWith('0x')?s:'0x'+s;
export const writeNew=(p,v)=>{const f=path.resolve(root,p);fs.mkdirSync(path.dirname(f),{recursive:true});fs.writeFileSync(f,JSON.stringify(v,null,2)+'\n',{flag:'wx'});};
export const cfg=load(configPath);
export const git=(...a)=>execFileSync('git',a,{cwd:root,encoding:'utf8',env:{...process.env,GIT_OPTIONAL_LOCKS:'0'}}).trim();
const abi=AbiCoder.defaultAbiCoder();
const metric=(value,status='MEASURED')=>({status,value});
const normalized=v=>JSON.parse(JSON.stringify(v,(_,x)=>typeof x==='bigint'?x.toString():x));
const absolute=p=>path.resolve(root,p);
const scriptPath='contracts/scripts/stage1_multibatch_compare.mjs';
export function validateConfig(){
 assert.equal(git('rev-parse','HEAD'),cfg.base_commit,'use the G18 base; no commits authorized');
 assert.equal(git('diff','--name-only','HEAD'),'','no tracked historical source changes');
 assert.equal(cfg.release_opt_level,1);assert.match(read('Cargo.toml').toString(),/opt-level\s*=\s*1/);
 for(const p of cfg.source_pins)assert.equal(sha(read(p.path)),p.sha256,p.path);
 assert.equal(solc.version().split('+')[0],cfg.solidity.version);
 for(const n of ['vm','common','tx'])assert.equal(load(`contracts/node_modules/@ethereumjs/${n}/package.json`).version,cfg.ethereumjs_version);
}
export function protectedSnapshot(){
 const files=execFileSync('git',['ls-files','--cached','--others','--exclude-standard','-z'],{cwd:root,encoding:'utf8'})
  .split('\0').filter(p=>p&&!sourceFiles.includes(p)&&p!=='docs/STAGE1_MULTIBATCH.md'
   &&!p.startsWith('experiments/stage1_multibatch_raw/')&&!p.startsWith('results/stage1_multibatch/'));
 const paper=path.resolve(root,'../../sn-article-template/sn-article.tex');
 return {files:files.map(p=>({path:p,sha256:sha(read(p))})),paper:{path:paper,sha256:sha(fs.readFileSync(paper))}};
}
export function checkProtected(p){for(const f of p.files)assert.equal(sha(read(f.path)),f.sha256,f.path);assert.equal(sha(fs.readFileSync(p.paper.path)),p.paper.sha256);}
function runPath(p){assert.ok(p&&/^experiments\/stage1_multibatch_raw\/[A-Za-z0-9_-]+$/.test(p),'new isolated run path required');return p;}
function sources(){
 const result={};
 function add(name){if(result[name])return;const content=read('contracts/src/'+name).toString();result[name]={content};
  for(const m of content.matchAll(/import\s+"([^"]+)"\s*;/g))add(path.posix.normalize(path.posix.join(path.posix.dirname(name),m[1])));}
 add('stage1/Stage1Receivers.sol');return result;
}
export function compileSolidity(){
 const src=sources();const settings={...cfg.solidity,outputSelection:{'*':{'*':['abi','evm.bytecode.object','evm.deployedBytecode.object']}}};delete settings.version;
 const start=performance.now(),out=JSON.parse(solc.compile(JSON.stringify({language:'Solidity',sources:src,settings})));
 for(const e of out.errors??[])if(e.severity==='error')throw Error(e.formattedMessage);else console.warn(e.formattedMessage);
 const sizes=Object.entries(out.contracts).flatMap(([file,cs])=>Object.entries(cs).map(([name,c])=>({file,name,creation_bytes:c.evm.bytecode.object.length/2,runtime_bytes:c.evm.deployedBytecode.object.length/2})));
 for(const n of ['Stage1LegacySettlement','Stage1DirectFoldSettlement','Stage1StateProofSettlement']){
  const s=sizes.find(x=>x.name===n);assert.ok(s&&s.creation_bytes>0);assert.ok(s.runtime_bytes<=24576&&s.creation_bytes<=49152,'code size '+n);}
 return {status:'PASS_COMPILE_ONLY',compiler:solc.version(),settings,contracts:out.contracts,sizes,
  compilation_ms:metric(performance.now()-start),sources:Object.entries(src).map(([p,v])=>({path:'contracts/src/'+p,sha256:sha(Buffer.from(v.content))}))};
}
async function command(run,label,program,args,extraEnv={}){
 const env={...process.env,...extraEnv,CARGO_BUILD_JOBS:String(cfg.budgets.build_jobs),RAYON_NUM_THREADS:String(cfg.budgets.rayon_threads)};
 for(const k of Object.keys(env))if(k.startsWith('CARGO_PROFILE_RELEASE_'))delete env[k];
 const base=path.join(run,'logs',label);fs.mkdirSync(path.dirname(absolute(base)),{recursive:true});
 const out=fs.openSync(absolute(base+'.stdout.log'),'wx'),err=fs.openSync(absolute(base+'.stderr.log'),'wx');
 const start=performance.now(),started=new Date().toISOString();let reason=null;
 const child=spawn(program,args,{cwd:root,env,stdio:['ignore','pipe','pipe'],detached:true});
 child.stdout.on('data',b=>{fs.writeSync(out,b);process.stdout.write(b);});child.stderr.on('data',b=>{fs.writeSync(err,b);process.stderr.write(b);});
 const timer=setTimeout(()=>{reason='WALL_BUDGET';try{process.kill(-child.pid,'SIGTERM');}catch{}},cfg.budgets.child_seconds*1000);
 const result=await new Promise(resolve=>{child.once('error',e=>resolve({code:null,error:String(e)}));child.once('close',(code,signal)=>resolve({code,signal}));});
 clearTimeout(timer);fs.closeSync(out);fs.closeSync(err);
 const r={program,args,...result,reason,started,wall_ms:performance.now()-start,
  environment:{CARGO_TARGET_DIR:env.CARGO_TARGET_DIR??null,CARGO_BUILD_JOBS:env.CARGO_BUILD_JOBS,RAYON_NUM_THREADS:env.RAYON_NUM_THREADS},
  stdout:base+'.stdout.log',stderr:base+'.stderr.log'};writeNew(base+'.command.json',r);
 assert.equal(result.code,0,label+' failed; logs preserved');assert.equal(reason,null);return r;
}
function verifyA(run){const a=load(run+'/gate_a.json');assert.equal(a.status,'PASS_GATE_A_COMPILE_ONLY');
 assert.equal(a.config_sha256,sha(read(configPath)));for(const p of a.source_pins)assert.equal(sha(read(p.path)),p.sha256,p.path);return a;}
export async function gateA(run){
 runPath(run);assert.ok(!fs.existsSync(absolute(run)),'never overwrite a run');validateConfig();
 const before=protectedSnapshot();writeNew(run+'/protected_files.json',before);
 const commands=[];const base={gate:'A',scope:'COMPILE_ONLY',run,base_commit:git('rev-parse','HEAD'),branch:git('branch','--show-current'),
  config_sha256:sha(read(configPath)),source_pins:sourceFiles.map(p=>({path:p,sha256:sha(read(p))})),
  environment:{cpu:os.cpus()[0].model,memory_bytes:os.totalmem(),platform:os.platform(),arch:os.arch(),node:process.version},
  setup_count:0,proof_generation_count:0,evm_transaction_count:0,tests:'NOT_RUN_GATE_B_REQUIRES_CONFIRMATION',automatic_commit:false,automatic_push:false};
 try{
  commands.push(await command(run,'rust-version','rustc',['+1.99.1','-Vv']));
  commands.push(await command(run,'cargo-build','cargo',['+1.99.1','build','--release','--locked','--offline','-p','vv-friendly-dkg','--bin','stage1_multibatch_materials'],{CARGO_TARGET_DIR:absolute(run+'/build-target')}));
  commands.push(await command(run,'rust-format','rustfmt',['+1.99.1','--edition','2021','--check',sourceFiles[1]]));
  for(const p of [scriptPath,'scripts/stage1_multibatch_audit.mjs'])commands.push(await command(run,'syntax-'+path.basename(p),'node',['--check',p]));
  console.log('Gate A: compiling the three isolated receivers; no VM is instantiated');
  const compiled=compileSolidity();writeNew(run+'/compiled.json',compiled);checkProtected(before);
  writeNew(run+'/gate_a.json',{...base,status:'PASS_GATE_A_COMPILE_ONLY',commands,
   compiled_sha256:sha(read(run+'/compiled.json')),binary:run+'/build-target/release/stage1_multibatch_materials',
   binary_sha256:sha(read(run+'/build-target/release/stage1_multibatch_materials')),historical_files_unchanged:true,
   next_gate:'B_REQUIRES_NEW_HUMAN_CONFIRMATION'});
 }catch(e){writeNew(run+'/gate_a_failed.json',{...base,status:'BLOCKED',error:String(e),commands});throw e;}
 console.log('PASS Gate A; STOP. B/C/D were not executed.');
}
function bindingData(){
 const a=load('fixtures/g8a_original_certificates_public_v1.json'),g=load('fixtures/jubjub_g6_native_winner_only_public_v1.json'),c=g.public_context;
 const binding={electionId:hex(c.election_id_hex),policyPre:hex(c.policy_pre_hex),dkgSession:hex(c.dkg_session_hex),keyCoreDigest:hex(c.key_core_digest_hex),publicKey:hex(c.public_key_hex),
  finalContext:hex(c.final_context_hex),registrationRoot:hex(c.registration_root_hex),registrationCloseDigest:hex(c.registration_close_digest_hex),disclosureDigest:hex(c.disclosure_digest_hex),
  p3VkSha256:hex(load('fixtures/g8_receiver_eip2537_v1.json').p3_vk_ark_sha256),prekeyContext:hex(c.prekey_context_hex),registrationAuthority:hex(c.registration_authority_hex),
  registrationCertificateSha256:hex(sha(wire(c.registration_certificate_hex))),region:4,registeredWeight:32};
 const expected={electionId:binding.electionId,manifestDigest:hex(a.manifest_digest_hex),policyPre:binding.policyPre,prekeyContext:binding.prekeyContext,dkgSession:binding.dkgSession,
  keyCoreDigest:binding.keyCoreDigest,finalContext:binding.finalContext,publicKey:binding.publicKey,registrationRoot:binding.registrationRoot,closeDigest:binding.registrationCloseDigest,disclosureDigest:binding.disclosureDigest};
 const close={electionId:hex(a.close.election_id_hex),finalContext:hex(a.close.final_context_hex),policyPre:hex(a.close.policy_pre_hex),root:hex(a.close.root_hex),
  nRegistered:a.close.n_registered,wRegistered:a.close.w_registered,wMax:a.close.w_max,version:a.close.version,canonicalRegistryDigest:hex(a.close.canonical_registry_digest_hex)};
 return {a,binding,expected,close};
}
const text=s=>{const b=Buffer.from(s),n=Buffer.alloc(8);n.writeBigUInt64BE(BigInt(b.length));return Buffer.concat([n,b]);};
const u64=n=>{const b=Buffer.alloc(8);b.writeBigUInt64BE(BigInt(n));return b;};
const prefix=s=>Buffer.concat([text('VV-CANONICAL-V1'),text(s)]);
const hash=b=>Buffer.from(sha(b),'hex');
function membership(material,index){
 const batch=Math.floor(index/4),position=index%4,{binding:b}=bindingData();
 const leaves=material.ballots.slice(batch*4,batch*4+4).map((v,i)=>hash(Buffer.concat([prefix('VV/BATCH/LEAF/v2'),wire(b.electionId),wire(b.finalContext),u64(4),u64(batch+1),u64(4),u64(i),u64(25),...v.eip.public_inputs_be_hex.map(wire)])));
 const node=(a,b)=>hash(Buffer.concat([prefix('VV/BATCH/NODE/v2'),a,b]));
 assert.equal('0x'+node(node(leaves[0],leaves[1]),node(leaves[2],leaves[3])).toString('hex'),cfg.ordered_batch_roots[batch]);
 return [leaves[position^1],position<2?node(leaves[2],leaves[3]):node(leaves[0],leaves[1])].map(b=>'0x'+b.toString('hex'));
}
function batchAt(m,index){const b=m.batches[index];return {index:index+1,root:b.root,oldState:b.old_state,newState:b.new_state,
 ballotProofs:m.ballots.slice(index*4,index*4+4).map(x=>x.eip.proof_neg_a_eip2537_hex),ballotInputs:m.ballots.slice(index*4,index*4+4).map(x=>x.eip.public_inputs_be_hex)};}
export async function vmWorker(run,aRun,route,k,negative){
 validateConfig();verifyA(aRun);assert.ok(cfg.routes.includes(route));assert.ok(k>=1&&k<=8);
 const request=load(run+'/request.json'),m=load(run+'/materials/materials.json');assert.equal(m.phase,request.gate);
 assert.equal(m.config_sha256,sha(read(configPath)));assert.equal(m.state_proofs_generated,request.gate==='B'?2:8);
 assert.equal(m.archived_state_proofs_reused,0);assert.equal(m.all_state_proofs_fresh,true);
 if(negative)assert.equal(request.gate,'B','no negatives inside formal performance workers');
 const compiled=load(aRun+'/compiled.json');for(const p of compiled.sources)assert.equal(sha(read(p.path)),p.sha256);
 const get=(f,n)=>compiled.contracts[f][n],names={legacy:'Stage1LegacySettlement',direct_fold:'Stage1DirectFoldSettlement',state_proof:'Stage1StateProofSettlement'};
 const code=get('stage1/Stage1Receivers.sol',names[route]),gateCode=get('auth/VeriVotingCertificateGateV2.sol','VeriVotingCertificateGateV2');
 const ri=new Interface(code.abi),gi=new Interface(gateCode.abi),{a,binding,expected,close}=bindingData();
 const point=p=>({x:p.x,y:p.y,encoded:hex(p.encoded_hex)}),sig=s=>({nonce:point(s.nonce),response:s.response_scalar,counter:s.counter});
 const common=new Common({chain:Mainnet,hardfork:Hardfork.Prague});assert.ok(common.isActivatedEIP(2537)&&common.isActivatedEIP(7623));
 // Every call creates a NEW VM and fresh sender state. No prior K is continued.
 const vm=await createVM({common}),key=hexToBytes('0x'+'3c'.repeat(32)),sender=createAddressFromPrivateKey(key);
 await vm.stateManager.putAccount(sender,createAccount({nonce:0n,balance:10n**25n}));
 const initialRoot=bytesToHex(await vm.stateManager.getStateRoot());
 const block=createBlock({header:{number:1n,gasLimit:BigInt(cfg.block_gas_limit),baseFeePerGas:1n,timestamp:1750000000n}},{common,skipConsensusFormatValidation:true});
 let nonce=0n,gate,receiver,calls=[],stack=[];const txs=[],negatives=[],recovery=[],checkpoints=[];
 vm.evm.events.on('beforeMessage',e=>stack.push({address:e.to?.toString()??null}));
 vm.evm.events.on('afterMessage',e=>{const c=stack.pop();if(c)calls.push({...c,gas:Number(e.execResult.executionGasUsed),success:!e.execResult.exceptionError,return_hex:bytesToHex(e.execResult.returnValue)});});
 function reason(r){const h=bytesToHex(r.execResult.returnValue);if(h.startsWith('0x08c379a0'))return abi.decode(['string'],'0x'+h.slice(10))[0];return null;}
 async function tx(label,category,data,to){
  calls=[];const signed=createLegacyTx({nonce:nonce++,gasLimit:BigInt(cfg.transaction_gas_limit),gasPrice:10n,value:0n,data:hexToBytes(data),...(to?{to}:{})},{common}).sign(key);
  const t=performance.now(),r=await runTx(vm,{tx:signed,block});assert.equal(stack.length,0);
  const entry={label,category,hash:bytesToHex(signed.hash()),nonce:Number(signed.nonce),signed_transaction_hex:bytesToHex(signed.serialize()),
   to:to?.toString()??null,status:r.receipt.status,gas_used:Number(r.totalGasSpent),receipt_cumulative_gas:Number(r.receipt.cumulativeBlockGasUsed),
   intrinsic_gas:Number(signed.getIntrinsicGas()),execution_gas:Number(r.execResult.executionGasUsed),refund:Number(r.gasRefund),calldata_floor:Number(signed.getMinimumGasLimit()),
   calldata_hex:bytesToHex(signed.data),calldata_bytes:signed.data.length,calldata_sha256:sha(signed.data),
   exception:r.execResult.exceptionError?.error??null,revert_reason:r.execResult.exceptionError?reason(r):null,
   calls:[...calls],logs:r.receipt.logs.map(([a,t,d])=>({address:bytesToHex(a),topics:t.map(bytesToHex),data:bytesToHex(d)})),wall_ms:performance.now()-t};
  assert.equal(entry.refund,0);assert.equal(entry.gas_used,Math.max(entry.intrinsic_gas+entry.execution_gas,entry.calldata_floor));
  assert.equal(entry.receipt_cumulative_gas,entry.gas_used);txs.push(entry);console.log(`${route} K${k} ${label}: ${entry.status?'PASS':entry.revert_reason} ${entry.gas_used}`);return {r,entry};
 }
 async function deploy(label,c,args){const data=(await new ContractFactory(c.abi,'0x'+c.evm.bytecode.object).getDeployTransaction(...args)).data;
  const {r,entry}=await tx(label,'deployment',data);assert.equal(entry.status,1);assert.ok(r.createdAddress);return r.createdAddress;}
 async function view(addr,iface,name,args=[]){await vm.evm.journal.checkpoint();let r;try{r=await vm.evm.runCall({caller:sender,to:addr,data:hexToBytes(iface.encodeFunctionData(name,args)),gasLimit:20000000n,isStatic:true});}finally{await vm.evm.journal.revert();}
  assert.equal(r.execResult.exceptionError,undefined,'view '+name);return iface.decodeFunctionResult(name,bytesToHex(r.execResult.returnValue));}
 async function snapshot(){
  const r=await view(receiver,ri,'checkpoint'),g=await view(gate,gi,'checkpoint');const spent=[],signers=[];
  for(const b of m.ballots)spent.push((await view(receiver,ri,'spentNullifier',[b.nullifier]))[0]);
  for(let id=1;id<=5;id++)signers.push((await view(gate,gi,'certifiedSigner',[id]))[0]);
  const weights={};for(const name of ['certifiedWRegistered','certifiedNRegistered','certifiedWMax','certifiedCertificateSha256'])weights[name]=(await view(gate,gi,name))[0];
  return normalized({next:Number(r[0]),count:Number(r[1]),position:Number(r[2]),frozen:r[3],target:r[4],state:[...r[5]],spent,
   gate:[...g],signers,weights,bindingDigest:(await view(receiver,ri,'bindingDigest'))[0],scheduleDigest:(await view(receiver,ri,'scheduleDigest'))[0],plannedBatches:k});
 }
 function data(b){return route==='state_proof'?ri.encodeFunctionData('submitBatch',[b,m.batches[b.index-1].eip.proof_neg_a_eip2537_hex]):ri.encodeFunctionData('submitBatch',[b]);}
 function single(i,old,proof=m.ballots[i].eip.proof_neg_a_eip2537_hex,row=m.ballots[i].eip.public_inputs_be_hex,idx=Math.floor(i/4)+1,pos=i%4){return ri.encodeFunctionData('submitBallot',[idx,pos,old,row,proof,membership(m,i)]);}
 async function reject(label,encoded,expectedReason){const before=await snapshot(),{entry}=await tx(label,'rejected',encoded,receiver),after=await snapshot();
  assert.equal(entry.status,0);assert.equal(entry.exception,'revert');assert.equal(entry.revert_reason,expectedReason);assert.deepEqual(after,before);assert.equal(entry.logs.length,0);
  negatives.push({case:label,reason:expectedReason,before,after,transaction_hash:entry.hash});return entry;}
 async function accept(index){if(route==='legacy'){
   for(let i=index*4;i<index*4+4;i++){const {entry}=await tx('accept_vote_'+(i+1),'intake',single(i,m.ballots[i].old_state),receiver);assert.equal(entry.status,1);
    const s=await snapshot();assert.deepEqual(s.state,m.ballots[i].new_state);assert.equal(s.count,i+1);assert.equal(s.position,(i+1)%4);}
  }else{const {entry}=await tx('accept_batch_'+(index+1),'intake',data(batchAt(m,index)),receiver);assert.equal(entry.status,1);}
  const s=await snapshot();assert.deepEqual(s.state,m.batches[index].new_state);assert.equal(s.next,index+2);assert.equal(s.count,4*(index+1));assert.equal(s.position,0);checkpoints.push(s);}
 const started=new Date().toISOString();let record;
 try{
  gate=await deploy('deploy_gate',gateCode,[expected,a.authentication_keys.map(point),point(a.registration_authority),a.public_coefficients.map(point),hex(a.transcript_digest_hex)]);
  const args=[gate.toString(),binding,cfg.ordered_batch_roots.slice(0,k),m.ballots[0].eip.vk_eip2537_hex];
  if(route==='state_proof')args.push(m.batches[0].eip.vk_eip2537_hex);receiver=await deploy('deploy_receiver',code,args);
  assert.equal((await view(receiver,ri,'certificateGate'))[0].toLowerCase(),gate.toString());
  assert.equal((await view(receiver,ri,'bindingDigest'))[0],cfg.binding_digest);
  if(negative)await reject('uncertified',route==='legacy'?single(0,m.ballots[0].old_state):data(batchAt(m,0)),'UNCERTIFIED');
  for(let i=0;i<3;i++){const {entry}=await tx('authenticate_dkg_'+(i+1),'certification',gi.encodeFunctionData('submitDkgSignature',[i+1,binding.electionId,binding.finalContext,sig(a.dkg_certificate_signatures[i])]),gate);assert.equal(entry.status,1);}
  {const {entry}=await tx('authenticate_registration','certification',gi.encodeFunctionData('authenticateRegistration',[close,sig(a.registration_signature)]),gate);assert.equal(entry.status,1);}
  const certified=await snapshot();assert.equal(certified.gate[6],true);
  if(negative){
   if(route==='legacy'){
    await reject('illegal_batch_index',single(0,m.ballots[0].old_state,undefined,undefined,2),'NEXT_INDEX');
    await reject('replacement_ballot_proof',single(0,m.ballots[0].old_state,m.ballots[1].eip.proof_neg_a_eip2537_hex),'INVALID_GROTH16');
    const firstTwo=[];
    for(let i=0;i<2;i++){const {entry}=await tx(i===0?'recovery_alice':'recovery_bob','recovery',single(i,m.ballots[i].old_state),receiver);assert.equal(entry.status,1);const s=await snapshot();assert.equal(s.count,i+1);assert.equal(s.next,1);assert.deepEqual(s.state,m.ballots[i].new_state);firstTwo.push(entry.hash);}
    recovery.push({step:1,action:'Alice then Bob accepted',transaction_hashes:firstTwo,checkpoint:await snapshot()});
    await reject('legacy_incomplete_freeze',ri.encodeFunctionData('freeze'),'INCOMPLETE');
    await reject('within_batch_duplicate_nf',single(1,m.ballots[2].old_state,undefined,m.ballots[0].eip.public_inputs_be_hex,1,2),'SPENT_NULLIFIER');
    const bad=await reject('charlie_first_attempt',single(2,m.ballots[2].old_state,m.ballots[0].eip.proof_neg_a_eip2537_hex),'INVALID_GROTH16');
    recovery.push({step:2,action:'Charlie rejected with full rollback',transaction_hash:bad.hash,checkpoint:await snapshot()});
    for(let i=2;i<4;i++){const {entry}=await tx(i===2?'recovery_charlie_retry':'recovery_david','recovery',single(i,m.ballots[i].old_state),receiver);assert.equal(entry.status,1);const s=await snapshot();assert.equal(s.count,i+1);assert.deepEqual(s.state,m.ballots[i].new_state);recovery.push({step:i+1,action:i===2?'Charlie retry accepted':'David accepted',transaction_hash:entry.hash,checkpoint:s});
     if(i===3){assert.equal(s.position,0);assert.equal(s.next,2);assert.ok(entry.logs.some(l=>{try{return ri.parseLog(l)?.name==='BatchAccepted';}catch{return false;}}));recovery.push({step:5,action:'Batch complete, emitted by the same David transaction',transaction_hash:entry.hash,checkpoint:s});}}
    await reject('stale_old_state',single(4,m.ballots[0].old_state),'OLD_STATE');
    await reject('cross_batch_duplicate_nf',single(4,m.ballots[4].old_state,undefined,m.ballots[0].eip.public_inputs_be_hex),'SPENT_NULLIFIER');
    checkpoints.push(await snapshot());await accept(1);
   }else{
    let b=batchAt(m,0);b.index=2;await reject('illegal_batch_index',data(b),'NEXT_INDEX');
    b=batchAt(m,0);b.ballotProofs[0]=b.ballotProofs[1];await reject('replacement_ballot_proof',data(b),'INVALID_GROTH16');
    b=batchAt(m,0);b.ballotInputs[1]=b.ballotInputs[0];await reject('within_batch_duplicate_nf',data(b),'DUPLICATE_NULLIFIER');
    b=batchAt(m,0);b.newState=m.batches[0].omission_a_new_state;
    const rejected=await reject('omission_a',data(b),route==='state_proof'?'INVALID_GROTH16':'DIRECT_FOLD_MISMATCH');
    if(route==='state_proof'){const ps=rejected.calls.filter(c=>c.address?.endsWith('000f'));assert.equal(ps.length,5);assert.ok(ps.slice(0,4).every(c=>BigInt(c.return_hex)===1n));assert.equal(BigInt(ps[4].return_hex),0n);}
    await accept(0);b=batchAt(m,1);b.oldState=m.batches[0].old_state;await reject('stale_old_state',data(b),'OLD_STATE');
    b=batchAt(m,1);b.ballotInputs[0]=m.ballots[0].eip.public_inputs_be_hex;await reject('cross_batch_duplicate_nf',data(b),'SPENT_NULLIFIER');await accept(1);
   }
  }else for(let i=0;i<k;i++)await accept(i);
  {const {entry}=await tx('freeze','freeze',ri.encodeFunctionData('freeze'),receiver);assert.equal(entry.status,1);}
  const final=await snapshot();assert.equal(final.frozen,true);assert.equal(final.count,k*4);assert.equal(final.next,k+1);assert.equal(final.position,0);
  assert.deepEqual(final.state,m.batches[k-1].new_state);assert.deepEqual(final.spent,m.ballots.map((_,i)=>i<k*4));
  assert.equal(final.target,'0x'+sha(wire(abi.encode(['string','bytes32','bytes32','uint64','uint64','bytes32[6]'],[cfg.freeze_version,cfg.binding_digest,final.scheduleDigest,k,k*4,final.state]))));
  if(negative)await reject('receive_after_freeze',route==='legacy'?single(0,final.state):data(batchAt(m,0)),'FROZEN');
  record={status:'PASS',final_checkpoint:final,certified_checkpoint:certified};
 }catch(e){record={status:'BLOCKED',error:String(e)};process.exitCode=1;}
 const output={...record,gate:request.gate,kind:negative?'NEGATIVE_ONLY':request.gate==='B'?'DEVELOPMENT_REGRESSION_NOT_PERFORMANCE':'FORMAL_POSITIVE_ONLY',
  k,route,started,finished:new Date().toISOString(),fresh_vm:true,initial_vm_root:initialRoot,vm_identity:`${request.identity}/${negative?'negative':'positive'}/${k}/${route}/${process.pid}`,
  hardfork:common.hardfork(),addresses:{gate:gate?.toString()??null,receiver:receiver?.toString()??null},transactions:txs,negatives,recovery,checkpoints,
  config_sha256:sha(read(configPath)),materials_sha256:sha(read(run+'/materials/materials.json')),compiled_sha256:sha(read(aRun+'/compiled.json')),
  node_peak_rss_mib:metric(process.resourceUsage().maxRSS/1024),measurement_status:'MEASURED',snarkpack_evm_verified:false,public_chain_deployment:false,
  atomicity:route==='legacy'?'per ballot; four non-atomic transactions per completed batch':'one atomic transaction per batch'};
 writeNew(run+`/sessions/${negative?'negative':'positive'}-k${k}-${route}.json`,output);assert.equal(output.status,'PASS',output.error);
}
export async function nextGate(gate,run,aRun,bRun,approved){
 assert.equal(approved,gate,'human approval must explicitly name this gate');assert.ok(['B','C'].includes(gate));runPath(run);validateConfig();const a=verifyA(aRun);
 if(gate==='C'){assert.ok(bRun);const b=load(bRun+'/gate_b.json');assert.equal(b.status,'PASS_GATE_B_REGRESSION_ONLY');assert.equal(b.config_sha256,sha(read(configPath)));}
 assert.ok(!fs.existsSync(absolute(run)),'fresh independent B/C run required');const before=protectedSnapshot();writeNew(run+'/protected_files.json',before);
 writeNew(run+'/request.json',{gate,identity:`${gate}/${path.basename(run)}/${new Date().toISOString()}`,config_sha256:sha(read(configPath)),a_run:aRun,b_run:bRun??null,approved,formal:gate==='C'});
 await command(run,'materials',absolute(a.binary),['prepare',run+'/materials',gate,'--approved',gate]);
 const ks=gate==='B'?cfg.regression_k:cfg.formal_k;
 for(const k of ks)for(const route of cfg.routes)await command(run,`positive-k${k}-${route}`,process.execPath,
  ['--max-old-space-size='+cfg.budgets.node_heap_mib,absolute(scriptPath),'--vm',run,aRun,route,String(k),'positive']);
 if(gate==='B')for(const route of cfg.routes)await command(run,'negative-'+route,process.execPath,
  ['--max-old-space-size='+cfg.budgets.node_heap_mib,absolute(scriptPath),'--vm',run,aRun,route,'2','negative']);
 for(const k of ks){const rows=cfg.routes.map(r=>load(run+`/sessions/positive-k${k}-${r}.json`));for(const r of rows.slice(1))assert.deepEqual(r.final_checkpoint,rows[0].final_checkpoint);}
 checkProtected(before);writeNew(run+`/gate_${gate.toLowerCase()}.json`,{gate,status:gate==='B'?'PASS_GATE_B_REGRESSION_ONLY':'PASS_GATE_C_FORMAL_EXECUTION',
  config_sha256:sha(read(configPath)),source_pins:sourceFiles.map(p=>({path:p,sha256:sha(read(p))})),positive_vm_count:ks.length*3,
  negative_vm_count:gate==='B'?3:0,historical_files_unchanged:true,next_gate:gate==='B'?'C_REQUIRES_NEW_HUMAN_CONFIRMATION':'D_REQUIRES_NEW_HUMAN_CONFIRMATION'});
 console.log('PASS Gate '+gate+'; STOP for human confirmation.');
}
async function main(){
 process.chdir(root);const a=process.argv.slice(2);
 if(a[0]==='--vm'){assert.equal(a.length,6);return vmWorker(a[1],a[2],a[3],Number(a[4]),a[5]==='negative');}
 const flags=Object.fromEntries(Array.from({length:a.length/2},(_,i)=>[a[2*i],a[2*i+1]]));
 assert.ok(a.length%2===0&&flags['--gate']&&flags['--run'],'use --gate A|B|C --run experiments/stage1_multibatch_raw/NEW_ID');
 if(flags['--gate']==='A')return gateA(flags['--run']);
 return nextGate(flags['--gate'],flags['--run'],flags['--a-run'],flags['--b-run'],flags['--approved']);
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url))main().catch(e=>{console.error(e.stack??e);process.exitCode=1;});
