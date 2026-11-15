// J1 only. Bounded compilation/public-fixture checks, never Setup/Prove or J2-J5.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import {execFileSync,spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
const destination=process.argv[2];assert.ok(destination,'usage: node scripts/joint_proof_run_j1.mjs NEW_RUN_DIR');
const out=path.resolve(destination);assert.ok(!fs.existsSync(out),'never overwrite an earlier attempt');
const sha=b=>createHash('sha256').update(b).digest('hex');
const git=(...args)=>execFileSync('git',args,{encoding:'utf8'}).trim();
const configPath='configs/experiment.J1-INNER-VERIFIER-PROBE.json';
const config=JSON.parse(fs.readFileSync(configPath));
assert.equal(git('rev-parse','HEAD'),config.baseline_commit,'review baseline changes before another attempt');
for(const p of config.source_pins)assert.equal(sha(fs.readFileSync(p.path)),p.sha256,p.path);
const before=git('ls-files','-z').split('\0').filter(Boolean).map(p=>({path:p,sha256:sha(fs.readFileSync(p))}));
fs.mkdirSync(path.join(out,'logs'),{recursive:true});
const save=(name,value)=>fs.writeFileSync(path.join(out,name),JSON.stringify(value,null,2)+'\n');
save('historical_files.json',before);
const sources=[configPath,'scripts/joint_proof_run_j1.mjs','tools/j1-inner-verifier-probe/Cargo.toml','tools/j1-inner-verifier-probe/Cargo.lock',
 'crates/vv-friendly-dkg/src/bin/j1_inner_verifier_probe.rs',
 ...['inner_groth16.rs','native_fq_compile.rs','current_outer_fr_rejected.rs'].map(p=>'crates/vv-friendly-crypto/src/joint/'+p)]
 .map(p=>{const b=fs.readFileSync(p),target=path.join(out,'source_snapshot',p);fs.mkdirSync(path.dirname(target),{recursive:true});fs.writeFileSync(target,b,{flag:'wx'});return{path:p,sha256:sha(b)};});
save('source_files.json',sources);save('config.json',config);
const manifest='tools/j1-inner-verifier-probe/Cargo.toml';
// Record the installed, exact-version primary implementation and licenses.
const metaArgs=['+1.99.1','metadata','--format-version','1','--locked','--offline','--manifest-path',manifest];
const metadata=JSON.parse(execFileSync('cargo',metaArgs,{encoding:'utf8',timeout:30000,maxBuffer:16*1024*1024}));
const evidence=[];
for(const [name,relative]of [['ark-groth16','src/constraints.rs'],['ark-r1cs-std','src/pairing/mod.rs'],['ark-r1cs-std','src/pairing/bls12/mod.rs'],['ark-r1cs-std','src/fields/nonnative/mod.rs']]){
 const pkg=metadata.packages.find(p=>p.name===name);assert.equal(pkg.version,'0.4.0');
 const base=path.dirname(pkg.manifest_path),bytes=fs.readFileSync(path.join(base,relative));
 const target=path.join(out,'dependency_evidence',name+'-'+pkg.version,relative);fs.mkdirSync(path.dirname(target),{recursive:true});fs.writeFileSync(target,bytes,{flag:'wx'});
 evidence.push({crate:name,version:pkg.version,source:pkg.source,path:relative,sha256:sha(bytes),archive_path:path.relative(out,target)});
 for(const license of ['LICENSE-MIT','LICENSE-APACHE']){
  const src=path.join(base,license),dst=path.join(out,'dependency_evidence',name+'-'+pkg.version,license);
  if(fs.existsSync(src)&&!fs.existsSync(dst))fs.writeFileSync(dst,fs.readFileSync(src),{flag:'wx'});
 }
}
save('dependency_evidence.json',{command:['cargo',...metaArgs],crates:metadata.packages.map(p=>({name:p.name,version:p.version,source:p.source})),evidence});
const env={...process.env,CARGO_BUILD_JOBS:'2',RAYON_NUM_THREADS:'4',CARGO_TARGET_DIR:path.resolve('target')};
const started=performance.now();let child=null,interrupted=false;
const m={schema:'VV/JOINT/J1-EXECUTION/v1',status:'RUNNING',stage:'J1',started_utc:new Date().toISOString(),
 baseline_commit:git('rev-parse','HEAD'),branch:git('branch','--show-current'),git_status_at_start:git('status','--short'),
 source_version_sha256:sha(Buffer.from(JSON.stringify(sources))),sources,config_sha256:sha(fs.readFileSync(configPath)),
 environment:{hardware:os.cpus()[0].model,memory_bytes:os.totalmem(),platform:os.platform(),arch:os.arch(),node:process.version},
 budget:config.budget,commands:[],running_command:null,
 setup_count:0,proof_generation_count:0,new_dkg_count:0,new_p3_count:0,r1cs_synthesized:false,
 constraint_count:null,recursive_verification_executed:false,recursive_crypto_negative_tests:[],j2_j5_started:false};
const update=()=>save('execution.json',m);update();
for(const signal of ['SIGINT','SIGTERM'])process.once(signal,()=>{interrupted=true;child?.kill('SIGTERM');});
async function run(name,args,{expectedCode=0,jsonFile=null}={}){
 assert.ok(!interrupted,'interrupted');
 console.log('\nSTART '+name);const begin=performance.now(),log='logs/'+name+'.log',fd=fs.openSync(path.join(out,log),'wx');
 child=spawn('cargo',args,{env,stdio:['ignore','pipe','pipe']});let peak=0,termination=null,buffer='';
 m.running_command={name,pid:child.pid,started_utc:new Date().toISOString(),log};update();
 for(const stream of [child.stdout,child.stderr])stream.on('data',b=>{fs.writeSync(fd,b);buffer+=b.toString();process.stdout.write(b);});
 const timer=setInterval(()=>{
  if(performance.now()-begin>config.budget.worker_seconds*1000||performance.now()-started>config.budget.total_seconds*1000)termination='WALL_BUDGET';
  try{
   const rows=execFileSync('ps',['-axo','pid=,ppid=,rss='],{encoding:'utf8'}).trim().split('\n').map(l=>l.trim().split(/\s+/).map(Number));
   const ids=new Set([child.pid]);let changed=true;while(changed){changed=false;for(const [pid,ppid]of rows)if(ids.has(ppid)&&!ids.has(pid)){ids.add(pid);changed=true;}}
   const rss=rows.filter(([pid])=>ids.has(pid)).reduce((sum,row)=>sum+row[2],0)/1024;peak=Math.max(peak,rss);
   if(rss>config.budget.worker_rss_mib)termination='RSS_BUDGET';
   if(termination)for(const pid of [...ids].reverse())try{process.kill(pid,'SIGTERM');}catch{}
  }catch{if(termination)child?.kill('SIGTERM');}
 },1000);
 const result=await new Promise(resolve=>{child.once('error',e=>resolve({code:null,error:String(e)}));child.once('close',(code,signal)=>resolve({code,signal}));});
 clearInterval(timer);fs.closeSync(fd);child=null;
 const c={name,program:'cargo',args,expected_exit_code:expectedCode,...result,termination,elapsed_ms:performance.now()-begin,polled_process_tree_peak_rss_mib:peak||null,log};
 m.commands.push(c);m.running_command=null;update();
 assert.equal(termination,null,name+' resource limit');assert.equal(result.code,expectedCode,name+' unexpected compiler/runtime result');
 if(jsonFile){const line=buffer.split('\n').find(l=>l.startsWith('{"status":'));assert.ok(line,'native compile-control JSON');save(jsonFile,JSON.parse(line));}
 return buffer;
}
try{
 await run('fmt-workspace',['+1.99.1','fmt','--all','--','--check']);
 await run('fmt-isolated',['+1.99.1','fmt','--manifest-path',manifest,'--all','--','--check']);
 await run('build-public-preflight',['+1.99.1','build','--release','--locked','--offline','-p','vv-friendly-dkg','--bin','j1_inner_verifier_probe']);
 await run('clippy-public-preflight',['+1.99.1','clippy','--release','--locked','--offline','-p','vv-friendly-dkg','--bin','j1_inner_verifier_probe','--','-D','warnings']);
 await run('public-fixture-preflight',['+1.99.1','run','--release','--locked','--offline','-p','vv-friendly-dkg','--bin','j1_inner_verifier_probe','--',path.join(out,'public_preflight.json')]);
 await run('build-native-fq-control',['+1.99.1','build','--release','--locked','--offline','--manifest-path',manifest,'--bin','j1-native-fq-compile']);
 await run('clippy-native-fq-control',['+1.99.1','clippy','--release','--locked','--offline','--manifest-path',manifest,'--bin','j1-native-fq-compile','--','-D','warnings']);
 await run('native-fq-control',['+1.99.1','run','--release','--locked','--offline','--manifest-path',manifest,'--bin','j1-native-fq-compile'],{jsonFile:'native_fq_control.json'});
 const diagnostics=await run('current-outer-fr-compile',['+1.99.1','build','--release','--locked','--offline','--manifest-path',manifest,'--bin','j1-current-outer-fr-rejected','--message-format=json'],{expectedCode:101});
 const errors=diagnostics.split('\n').filter(l=>l.startsWith('{')).flatMap(l=>{try{const v=JSON.parse(l);return v.reason==='compiler-message'&&v.message.level==='error'?[v.message]:[];}catch{return[];}});
 assert.ok(errors.some(e=>e.code?.code==='E0277'&&e.message.includes('SNARKGadget')),'not the expected concrete trait/domain incompatibility');
 save('outer_fr_compile_diagnostics.json',{status:'EXPECTED_DOMAIN_COMPATIBILITY_REJECTION',errors});
 const native=JSON.parse(fs.readFileSync(path.join(out,'native_fq_control.json'))),input=JSON.parse(fs.readFileSync(path.join(out,'public_preflight.json')));
 assert.equal(native.status,'PASS_NATIVE_FQ_COMPILE_ONLY');assert.equal(native.native_gadget_constraint_field_bits,381);assert.equal(native.current_outer_constraint_field_bits,255);
 assert.equal(input.status,'PASS_PUBLIC_FIXTURE_PREFLIGHT_ONLY');assert.equal(input.in_circuit_verification_executed,false);
 for(const p of before)assert.equal(sha(fs.readFileSync(p.path)),p.sha256,'historical tracked file changed '+p.path);
 for(const p of sources)assert.equal(sha(fs.readFileSync(p.path)),p.sha256,'source changed during attempt '+p.path);
 m.old_tracked_files_unchanged=true;m.native_gadget_available_over_fq=true;m.current_fr_backend_compatible=false;
 m.status='BLOCKED_INNER_RECURSIVE_VERIFIER';m.reason='Locked Groth16VerifierGadget is over inner BLS12-381 Fq, not the current outer/state Fr; no compatible non-native pairing verifier has been implemented.';
 m.recursive_crypto_negative_tests=['valid_original_proof','wrong_nullifier','changed_ciphertext','different_election_context','mismatched_vk'].map(name=>({name,status:'NOT_RUN',reason:'current Fr verifier cannot compile; no circuit satisfaction claim'}));
}catch(e){m.status=interrupted?'INTERRUPTED':'FAIL_OR_RESOURCE_BLOCKED';m.error=String(e);process.exitCode=1;}
finally{m.finished_utc=new Date().toISOString();m.wall_ms=performance.now()-started;update();console.log(JSON.stringify({status:m.status,output:out,reason:m.reason??m.error??null}));}
