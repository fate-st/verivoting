// Bounded phase-by-phase execution. All outputs are new; previous evidence is immutable.
import fs from 'node:fs';import path from 'node:path';import os from 'node:os';
import assert from 'node:assert/strict';import {execFileSync,spawn} from 'node:child_process';import {createHash} from 'node:crypto';
const output=process.argv[2];assert.ok(output,'usage: node scripts/stage2_evm_fold_run.mjs NEW_OUTPUT_DIR');
const out=path.resolve(output);assert.ok(!fs.existsSync(out),'never overwrite an earlier run');
const sha=b=>createHash('sha256').update(b).digest('hex');
const git=(...a)=>execFileSync('git',a,{encoding:'utf8'}).trim();
const before=git('ls-files','-z').split('\0').filter(Boolean).map(p=>({path:p,sha256:sha(fs.readFileSync(p))}));
fs.mkdirSync(path.join(out,'logs'),{recursive:true});
const save=(p,v)=>fs.writeFileSync(path.join(out,p),JSON.stringify(v,null,2)+'\n');save('historical_files.json',before);
const sourcePaths=[...new Set([...git('ls-files','contracts/src','contracts/package.json','contracts/package-lock.json','Cargo.toml','Cargo.lock').split('\n'),
 'contracts/src/JubjubPointOps.sol','contracts/src/JubjubPointOpsHarness.sol','contracts/src/VeriVotingAuthenticatedDirectFoldSettlement.sol',
 'contracts/scripts/stage2_evm_fold_compare.mjs','scripts/stage2_evm_fold_run.mjs','crates/vv-friendly-dkg/src/bin/stage2_point_vectors.rs',
 'configs/experiment.G8C-FROZEN-P3-W32.json','crates/vv-friendly-dkg/Cargo.toml','crates/vv-friendly-crypto/src/lib.rs'])].filter(Boolean);
const sources=sourcePaths.map(p=>{const b=fs.readFileSync(p),target=path.join(out,'source_snapshot',p);fs.mkdirSync(path.dirname(target),{recursive:true});fs.writeFileSync(target,b,{flag:'wx'});return {path:p,sha256:sha(b)};});save('source_files.json',sources);
const env={...process.env,CARGO_BUILD_JOBS:'2',CARGO_PROFILE_RELEASE_STRIP:'none',RAYON_NUM_THREADS:'4'};
const plan=['build-rust-reference','fmt','clippy-reference','rust-reference','compile','point-conformance','direct-fold-vm','state-proof-vm'];
const m={schema:'VV/STAGE2/EVM-FOLD-EXECUTION/v1',status:'RUNNING',started_utc:new Date().toISOString(),base_commit:git('rev-parse','HEAD'),branch:git('branch','--show-current'),
 git_status_at_start:git('status','--short'),source_version_sha256:sha(Buffer.from(JSON.stringify(sources))),sources,plan,commands:[],
 environment:{hardware:os.cpus()[0].model,memory_bytes:os.totalmem(),node:process.version,platform:os.platform(),arch:os.arch()},
 budget:{worker_seconds:600,total_seconds:1800,worker_rss_mib:16384,tx_gas:20000000,block_gas:30000000},new_setup:0,new_proofs:0,new_p3:0,new_dkg:0,exact_snarkpack_evm:false};
const update=()=>save('execution.json',m);update();
const start=performance.now();let child=null,interrupted=false;
for(const signal of ['SIGINT','SIGTERM'])process.once(signal,()=>{interrupted=true;child?.kill('SIGTERM');});
async function run(name,program,args){
 if(interrupted)throw Error('interrupted before phase '+name);
 console.log('\nSTART '+name);const t=performance.now(),log='logs/'+name+'.log',fd=fs.openSync(path.join(out,log),'wx');
 child=spawn(program,args,{env,stdio:['ignore','pipe','pipe']});let peak=0,termination=null;
 m.running_command={name,pid:child.pid,started_utc:new Date().toISOString(),log};update();
 for(const stream of [child.stdout,child.stderr])stream.on('data',b=>{fs.writeSync(fd,b);process.stdout.write(b);});
 const timer=setInterval(()=>{
  if((performance.now()-t)>600000||(performance.now()-start)>1800000)termination='WALL_BUDGET';
  try{const rss=Number(execFileSync('ps',['-o','rss=','-p',String(child.pid)],{encoding:'utf8',stdio:['ignore','pipe','ignore']}).trim())/1024;peak=Math.max(peak,rss);if(rss>16384)termination='RSS_BUDGET';}catch{}
  if(termination)child.kill('SIGTERM');
 },1000);
 const result=await new Promise(resolve=>{child.once('error',e=>resolve({code:null,error:String(e)}));child.once('close',(code,signal)=>resolve({code,signal}));});
 clearInterval(timer);fs.closeSync(fd);child=null;
 m.commands.push({name,program,args,log,elapsed_ms:performance.now()-t,...result,termination,polled_process_peak_rss_mib:peak||null});m.running_command=null;update();
 assert.equal(result.code,0,name+': '+JSON.stringify(result)+' '+(termination??''));
}
try{
 await run(plan[0],'cargo',['+1.99.1','build','--release','--locked','--offline','-p','vv-friendly-dkg','--bin','stage2_point_vectors']);
 await run(plan[1],'cargo',['+1.99.1','fmt','--all','--','--check']);
 await run(plan[2],'cargo',['+1.99.1','clippy','--release','--locked','--offline','-p','vv-friendly-dkg','--bin','stage2_point_vectors','--','-D','warnings']);
 await run(plan[3],path.resolve('target/release/stage2_point_vectors'),[path.join(out,'rust_points.json')]);
 const script=path.resolve('contracts/scripts/stage2_evm_fold_compare.mjs');
 for(const [name,phase]of [[plan[4],'prepare'],[plan[5],'points'],[plan[6],'direct_fold'],[plan[7],'state_proof']])await run(name,process.execPath,['--max-old-space-size=6144',script,out,phase]);
 for(const p of before)assert.equal(sha(fs.readFileSync(p.path)),p.sha256,'old tracked artifact changed '+p.path);
 for(const p of sources)assert.equal(sha(fs.readFileSync(p.path)),p.sha256,'execution source changed '+p.path);
 const a=JSON.parse(fs.readFileSync(path.join(out,'direct_fold.json'))),b=JSON.parse(fs.readFileSync(path.join(out,'state_proof.json')));
 assert.equal(a.status,'PASS_TWO_B4_ACCEPTANCE_AND_FREEZE');assert.equal(b.status,a.status);
 assert.equal(a.initial_vm_state_root,b.initial_vm_state_root);assert.deepEqual(a.certified_checkpoint,b.certified_checkpoint);assert.deepEqual(a.checkpoints,b.checkpoints);assert.deepEqual(a.frozen_checkpoint,b.frozen_checkpoint);
 m.identical_initial_vm_and_application_checkpoints=true;m.old_tracked_files_unchanged=true;m.status='PASS_PAIRED_EVM_FOLD_COMPARISON';
}catch(e){m.status=interrupted?'INTERRUPTED':'FAIL_OR_RESOURCE_BLOCKED';m.error=String(e);process.exitCode=1;}
finally{m.finished_utc=new Date().toISOString();m.wall_ms=performance.now()-start;update();console.log(JSON.stringify({status:m.status,output:out,error:m.error??null}));}
