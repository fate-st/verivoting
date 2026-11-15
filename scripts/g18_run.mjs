// Bounded G18-1 controller. This fixed Mac cannot pass the required Groth16 gate.
// Run one official smoke for actual backend evidence, then stop; no A/B/C mutation.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync,spawn} from 'node:child_process';
import {performance} from 'node:perf_hooks';
const run=process.argv[2];assert.ok(run,'usage: node scripts/g18_run.mjs NEW_RUN_DIR');
assert.ok(!fs.existsSync(run),'Never overwrite a run');
const read=p=>fs.readFileSync(p),sha=b=>createHash('sha256').update(b).digest('hex');
const pin=p=>({path:p,sha256:sha(read(p)),bytes:read(p).length});
const git=(...a)=>execFileSync('git',a,{encoding:'utf8',maxBuffer:8*1024*1024}).trim();
const configPath='configs/experiment.G18-DECISION.json',config=JSON.parse(read(configPath)),configPin=pin(configPath);
assert.equal(git('rev-parse','HEAD'),config.baseline_commit);assert.equal(git('branch','--show-current'),config.branch);
assert.equal(os.platform(),config.environment.platform);assert.equal(os.arch(),config.environment.arch);
fs.mkdirSync(path.join(run,'logs'),{recursive:true});
const save=(p,j)=>fs.writeFileSync(path.join(run,p),JSON.stringify(j,null,2)+'\n',{flag:'wx'});
const historical=git('ls-files','-z').split('\0').filter(Boolean).map(pin);save('historical_files.json',historical);
for(const e of config.input_pins)assert.equal(sha(read(e.path)),e.sha256,e.path);
const sources=[configPath,'scripts/g18_prepare.mjs','scripts/g18_preflight.mjs','scripts/g18_run.mjs',
 'experiments/g18_prover_admission/Cargo.toml','experiments/g18_prover_admission/src/main.rs'].map(pin);
save('source_files.json',sources);
for(const e of sources){const dest=path.join(run,'source_snapshot',e.path);fs.mkdirSync(path.dirname(dest),{recursive:true});fs.writeFileSync(dest,read(e.path),{flag:'wx'});}
fs.mkdirSync(path.join(run,'upstream'));
for(const e of config.official_smoke.official_files){
 assert.equal(sha(read(e.installed_path)),e.sha256);
 fs.writeFileSync(path.join(run,'upstream',e.package+'__'+e.file.replaceAll('/','__')),read(e.installed_path),{flag:'wx'});
}
const env={...process.env};
for(const key of Object.keys(env))if(/^(BONSAI_|RISC0_)/.test(key))delete env[key];
const cargo=execFileSync('rustup',['which','--toolchain','1.99.1','cargo'],{encoding:'utf8'}).trim();
env.PATH=path.dirname(cargo)+path.delimiter+env.PATH;env.RUSTUP_TOOLCHAIN='1.99.1';
env.RISC0_HOME=path.resolve('target/g17-tools/risc0');
env.RISC0_SERVER_PATH=path.resolve('target/g17-tools/risc0/extensions/v3.1.0-cargo-risczero-aarch64-apple-darwin/r0vm');
env.RISC0_DEV_MODE='0';env.CARGO_BUILD_JOBS='2';env.CARGO_TARGET_DIR=path.resolve('target/g18-admission');
env.CARGO_PROFILE_RELEASE_STRIP='none';env.RUST_LOG='info';
const m={schema:'VV/G18/EXECUTION/v1',status:'RUNNING',run,baseline_commit:git('rev-parse','HEAD'),branch:config.branch,
 started_utc:new Date().toISOString(),config:configPin,sources,commands:[],
 env_overrides:{RISC0_DEV_MODE:'0',RISC0_HOME:env.RISC0_HOME,RISC0_SERVER_PATH:env.RISC0_SERVER_PATH,
  CARGO_TARGET_DIR:env.CARGO_TARGET_DIR,CARGO_BUILD_JOBS:'2',host_strip:'none',remote_prover:false,
  local_backend:'Explicit ExternalProver using same official r0vm; default native hardware selection'},
 run_source_is_new_uncommitted:true,engineering_repair_retests:0,old_tracked_files:historical.length};
const update=()=>fs.writeFileSync(path.join(run,'execution.json'),JSON.stringify(m,null,2)+'\n');update();
async function command(name,program,args,seconds,memoryMiB){
 assert.equal(sha(read(configPath)),configPin.sha256,'Frozen config changed');
 const started=performance.now(),at=new Date().toISOString();
 const stdout=`logs/${name}.stdout.log`,stderr=`logs/${name}.stderr.log`;
 const fd1=fs.openSync(path.join(run,stdout),'wx'),fd2=fs.openSync(path.join(run,stderr),'wx');
 const child=spawn(program,args,{env,detached:true,stdio:['ignore',fd1,fd2]});
 m.running={name,pid:child.pid,started_utc:at,stdout,stderr};update();
 let samples=0,peakSingle=0,peakTree=0,termination=null,spawnError=null;
 const stop=reason=>{if(termination)return;termination=reason;try{process.kill(-child.pid,'SIGTERM')}catch{}
  setTimeout(()=>{try{process.kill(-child.pid,'SIGKILL')}catch{}},1500).unref();};
 const poll=setInterval(()=>{
  if(performance.now()-started>seconds*1000)stop('TIME_BUDGET');
  try{
   const all=execFileSync('/bin/ps',['-axo','pid=,ppid=,rss='],{encoding:'utf8'}).trim().split('\n').map(l=>l.trim().split(/\s+/).map(Number));
   const ids=new Set([child.pid]);let changed=true;
   while(changed){changed=false;for(const [pid,ppid]of all)if(ids.has(ppid)&&!ids.has(pid)){ids.add(pid);changed=true;}}
   const rss=all.filter(r=>ids.has(r[0])).map(r=>r[2]/1024);
   if(rss.length){samples++;peakSingle=Math.max(peakSingle,...rss);peakTree=Math.max(peakTree,rss.reduce((a,b)=>a+b,0));
    if(Math.max(...rss)>memoryMiB||rss.reduce((a,b)=>a+b,0)>memoryMiB)stop('RSS_BUDGET');}
  }catch{}
 },config.budgets.poll_ms);
 const heart=setInterval(()=>console.log(`${name}: ${Math.round((performance.now()-started)/1000)}s, sampled process/tree ${peakSingle.toFixed(1)}/${peakTree.toFixed(1)} MiB`),20000);
 const result=await new Promise(resolve=>{child.once('error',e=>{spawnError=e.message});child.once('close',(code,signal)=>resolve({code,signal}));});
 clearInterval(poll);clearInterval(heart);fs.closeSync(fd1);fs.closeSync(fd2);
 const c={name,program,args,started_utc:at,finished_utc:new Date().toISOString(),...result,
  spawn_error:spawnError,termination,wall_ms:performance.now()-started,budget_seconds:seconds,budget_memory_mib:memoryMiB,
  sampled_max_single_process_rss_mib:samples?peakSingle:null,sampled_max_process_tree_rss_mib:samples?peakTree:null,
  rss_sample_count:samples,rss_scope:'1-second host RSS sampling; not exact peak or guest logical/prover allocation',stdout,stderr};
 m.commands.push(c);m.running=null;update();console.log(JSON.stringify(c));return c;
}
const good=c=>c.code===0&&!c.signal&&!c.termination&&!c.spawn_error;
try{
 const pre=await command('environment-preflight',process.execPath,['scripts/g18_preflight.mjs',run],120,16384);
 if(!good(pre))throw Error('PREFLIGHT_FAILED');
 const environment=JSON.parse(read(path.join(run,'environment_preflight.json')));
 assert.equal(environment.required_groth16_wrapper_available,false,'Unexpected environment: stop rather than change the frozen gate');
 const manifest='experiments/g18_prover_admission/Cargo.toml';
 const lock=await command('generate-isolated-lock',cargo,['generate-lockfile','--manifest-path',manifest,'--offline'],120,16384);
 if(!good(lock))throw Error('LOCK_GENERATION_FAILED');
 const lockPin=pin('experiments/g18_prover_admission/Cargo.lock');save('isolated_lock.json',lockPin);
 fs.copyFileSync(lockPin.path,path.join(run,'source_snapshot/experiments/g18_prover_admission/Cargo.lock'),fs.constants.COPYFILE_EXCL);
 const build=await command('build-official-smoke-host',cargo,['build','--manifest-path',manifest,'--release','--locked','--offline'],600,16384);
 if(!good(build))throw Error('ADMISSION_HOST_BUILD_FAILED');
 const fmt=await command('fmt-admission',cargo,['fmt','--manifest-path',manifest,'--','--check'],120,16384);
 // Formatting is recorded separately; never changes a proof admission result.
 m.fmt_passed=good(fmt);update();
 const host=path.join(env.CARGO_TARGET_DIR,'release/g18-prover-admission');save('host_binary.json',pin(host));
 const loop=config.official_smoke.official_files[0].installed_path;
 const prep=await command('prepare-official-smoke',host,['prepare',run,loop],120,16384);
 if(!good(prep))throw Error('OFFICIAL_SMOKE_PREPARATION_FAILED');
 const smoke=await command('official-real-proof-smoke',host,['smoke',run,loop],1800,24576);
 m.official_smoke_command_passed=good(smoke);
 m.status='STOP_G18_1_REQUIRED_GROTH16_ENVIRONMENT_UNAVAILABLE';
 m.stop_reason='Apple M4 fixed environment cannot provide the required official Groth16 wrapper; one official Composite smoke retained independently';
}catch(e){m.status='STOP_G18_1_ADMISSION_FAILURE';m.error=String(e);}
finally{
 for(const e of historical)assert.equal(sha(read(e.path)),e.sha256,'Old tracked file changed: '+e.path);
 for(const e of sources)assert.equal(sha(read(e.path)),e.sha256,'Frozen execution source changed: '+e.path);
 assert.equal(sha(read(configPath)),configPin.sha256);m.historical_files_unchanged=true;
 m.finished_utc=new Date().toISOString();update();console.log(JSON.stringify({status:m.status,run,
  official_smoke_passed:m.official_smoke_command_passed??null,auto_follow_on:false},null,2));
}
