// One bounded generation pilot, NOT final statistics or a complete system benchmark.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync,spawn} from 'node:child_process';
const sha=b=>createHash('sha256').update(b).digest('hex');
const git=(...a)=>execFileSync('git',a,{encoding:'utf8'}).trim();
const load=p=>JSON.parse(fs.readFileSync(p));
const save=(p,v)=>fs.writeFileSync(p,JSON.stringify(v,null,2)+'\n',{flag:'wx'});
const checks=[
 ['inventory',['scripts/stage2_inventory.mjs','--check','experiments/stage2_matched_cost_raw/experiment_inventory.json']],
 ['b1',['scripts/g10b1_report.mjs','--check']],
 ['b2',['scripts/g10b2_report.mjs','--check']],
 ['connected-chain',['scripts/g10c_chain_audit.mjs']],
 ['connected-report',['scripts/g10c_report.mjs','--check']],
 ['belenios',['scripts/g11/c1/check.mjs','experiments/g11/baselines/belenios/run-20261106-01']],
 ['g9-boundary',['scripts/g9p1_feasibility.mjs','--check']],
];
const phases=['ballot-setup','ballot-prove','state-setup','state-prove','aggregate-receive'];
function verify(out) {
 const m=load(path.join(out,'execution.json'));
 assert.equal(m.status,'PASS_MATCHED_GENERATION_PILOT');
 for(const f of m.files)assert.equal(sha(fs.readFileSync(path.join(out,f.path))),f.sha256,f.path);
 for(const f of load(path.join(out,'source_files.json')))assert.equal(sha(fs.readFileSync(path.join(out,'source_snapshot',f.path))),f.sha256,f.path);
 const p=Object.fromEntries(phases.map(k=>[k,load(path.join(out,k+'.json'))]));
 assert.equal(p['ballot-setup'].setup_count,1);assert.equal(p['state-setup'].setup_count,1);
 assert.equal(p['ballot-prove'].setup_count,0);assert.equal(p['state-prove'].setup_count,0);
 assert.equal(p['ballot-prove'].proofs.length,8);assert.equal(p['state-prove'].proofs.length,2);
 for(const r of [...p['ballot-prove'].proofs,...p['state-prove'].proofs]) {
  assert.equal(r.verified,true);assert.equal(r.bytes,192);assert.equal(sha(Buffer.from(r.proof_hex,'hex')),r.sha256);
 }
 assert.equal(new Set(p['ballot-prove'].proofs.map(r=>r.sha256)).size,8);
 const r=p['aggregate-receive'];assert.equal(r.aggregate_count,2);assert.equal(r.historical_proofs_used_by_receivers,0);
 assert.equal(r.receivers.routes.length,4);assert.equal(r.receivers.necessary_negatives.length,4);
 assert.ok(r.receivers.routes.every(v=>v.same_ct1_ct2_and_spent8));
 assert.ok(r.receivers.necessary_negatives.every(v=>v.checkpoint_unchanged));
 assert.equal(m.historical_tracked_files_unchanged,true);
 console.log(JSON.stringify({status:'PASS_ARCHIVED_MATCHED_GENERATION_PILOT',ballot_setup:1,state_setup:1,new_ballot_proofs:8,new_state_proofs:2,new_aggregates:2,routes:4,negative_cases:4,new_p3:0,new_evm:0,statistical_claim:false}));
}
if(process.argv[2]==='--check') {verify(path.resolve(process.argv[3]));process.exit(0);}
const [output]=process.argv.slice(2);
assert.ok(output,'usage: node scripts/stage2_matched_cost.mjs NEW_OUTPUT_DIR | --check EXISTING_DIR');
const out=path.resolve(output);assert.ok(!fs.existsSync(out),'never overwrite an existing run');
fs.mkdirSync(out,{recursive:true});fs.mkdirSync(path.join(out,'logs'));
const runtime=fs.mkdtempSync(path.join(os.tmpdir(),'vv-stage2-cost-'));fs.chmodSync(runtime,0o700);
const env={...process.env,CARGO_BUILD_JOBS:'2',CARGO_PROFILE_RELEASE_STRIP:'none',RAYON_NUM_THREADS:'4'};
const tracked=git('ls-files','-z').split('\0').filter(Boolean);
const historical=tracked.filter(p=>fs.existsSync(p)&&fs.statSync(p).isFile()).map(p=>({path:p,sha256:sha(fs.readFileSync(p))}));
save(path.join(out,'historical_files.json'),historical);
const sourcePaths=[...new Set([...tracked.filter(p=>/^(crates\/.*\.(rs|toml)|Cargo\.(toml|lock)|rust-toolchain.*)$/.test(p)),
 'crates/vv-friendly-dkg/src/bin/stage2_matched_cost.rs','crates/vv-friendly-dkg/src/bin/common/stage2_routes.rs','scripts/stage2_matched_cost.mjs','scripts/stage2_inventory.mjs'])];
fs.mkdirSync(path.join(out,'source_snapshot'),{recursive:true});
const sourceFiles=sourcePaths.map(p=>{const b=fs.readFileSync(p),q=path.join(out,'source_snapshot',p);fs.mkdirSync(path.dirname(q),{recursive:true});fs.writeFileSync(q,b,{flag:'wx'});return {path:p,sha256:sha(b)};});
save(path.join(out,'source_files.json'),sourceFiles);
const m={schema:'VV/STAGE2/EXECUTION/v1',started_utc:new Date().toISOString(),base_commit:git('rev-parse','HEAD'),branch:git('branch','--show-current'),
 git_status_at_start:git('status','--short'),source_version_sha256:sha(Buffer.from(JSON.stringify(sourceFiles))),source_snapshot:'source_snapshot/',
 environment:{hardware:os.cpus()[0].model,memory_bytes:os.totalmem(),platform:os.platform(),arch:os.arch(),node:process.version,
 rustc:execFileSync('rustc',['+1.99.1','-Vv'],{encoding:'utf8'}),cargo:execFileSync('cargo',['+1.99.1','-V'],{encoding:'utf8'}),
 threads:4,build_jobs:2,profile:'release; strip=none; locked; offline',local_pk_load:'SHA-256 pinned freshly generated private cache + frozen VK equality; not external unchecked key loading'},
 budget:{worker_wall_seconds:600,worker_rss_mib:16384,build_wall_seconds:600,pilot_wall_seconds:1800,actual_generated_ballot_proofs:8,actual_generated_state_proofs:2},
 scope:'one reconstructed archived election, fresh randomized proofs; common workstation; sequential isolated workers; not final repeated statistics',
 private_runtime:runtime,commands:[],files:[],status:'RUNNING',exact_snarkpack_evm:false,new_p3:0,new_evm:0};
const update=()=>fs.writeFileSync(path.join(out,'execution.json'),JSON.stringify(m,null,2)+'\n');
const campaignStart=performance.now();
async function run(name,program,args,seconds=600,monitor=false) {
 const log='logs/'+name+'.log',fd=fs.openSync(path.join(out,log),'wx'),start=performance.now();
 console.log('START '+name);
 let peakKiB=0,termination=null,settled=false;
 const child=spawn(program,args,{env,stdio:['ignore',fd,fd]});
 const timer=setInterval(()=>{
  if(settled)return;
  if((performance.now()-start)/1000>seconds || (performance.now()-campaignStart)/1000>1800)termination='WALL_BUDGET';
  if(monitor)try {const rss=Number(execFileSync('ps',['-o','rss=','-p',String(child.pid)],{encoding:'utf8',stdio:['ignore','pipe','ignore']}).trim());peakKiB=Math.max(peakKiB,rss);if(rss>16384*1024)termination='RSS_BUDGET';}catch{}
  if(termination)child.kill('SIGTERM');
 },1000);
 const result=await new Promise(resolve=>{child.on('error',e=>resolve({code:null,signal:null,error:String(e)}));child.on('close',(code,signal)=>resolve({code,signal}));});
 settled=true;clearInterval(timer);fs.closeSync(fd);
 m.commands.push({name,program,args,elapsed_ms:performance.now()-start,...result,log,termination,polled_process_peak_rss_mib:monitor?peakKiB/1024:null});update();
 process.stdout.write(fs.readFileSync(path.join(out,log),'utf8'));
 assert.equal(result.code,0,name+': '+JSON.stringify(result)+' '+(termination??''));
}
function inventory(){
 const families=['g1_e','g2_e','g4_d','g6','g7','g8','g8a','g8b','g8c','g8d','g8e','g10_b1','g10_b2','g10_c','g11'];
 const entries=families.map(id=>{
  const files=tracked.filter(p=>p.startsWith('experiments/'+id+'_raw/')||p.startsWith('results/'+id+'/')||(id==='g11'&&p.startsWith('experiments/g11/')));
  const key=files.filter(p=>/\/(?:[^/]*(?:summary|manifest|environment|commands|execution|native_winner|ten_subsets|matched|aggregation_cost)[^/]*)\.json$/.test(p)&&!p.includes('source_snapshot/'));
  return {experiment:id,tracked_files:files.length,evidence:key.map(p=>{const v=load(p);return {path:p,sha256:sha(fs.readFileSync(p)),status:v.status??null,source_commit:v.source_base_sha??v.git_sha??v.git_head??v.base_commit??null,metadata_keys:Object.keys(v)};}),
   caveat:'Historical metadata remains authoritative; missing per-file commit is null, not retroactively relabeled as current HEAD'};
 });
 const binary='/private/tmp/vv-g11c1-20261106-K03bsm/belenios-git/_build/install/default/bin/belenios-tool';
 save(path.join(out,'experiment_inventory.json'),{base_commit:m.base_commit,branch:m.branch,environment:m.environment,entries,check_commands:checks,
  baseline_boundaries:{G10_B1:'four receiver routes, archived proofs, no generation',G10_B2:'historical generation ledger plus fresh aggregation; not a matched total',G10_C:'implemented native and individual-Groth16 EVM fallback; not exact SnarkPack EVM',G11_C1:'105 official Belenios 3.0 elections; different weights/timing model, not a ranking'},
  belenios_runtime:{archived_binary_path:binary,currently_executable:(()=>{try{fs.accessSync(binary,fs.constants.X_OK);return true;}catch{return false;}})(),historical_results_invalidated:false,rebuild_attempted:false},
  gaps:['durable global registration ledger and RegisterVoter service','partially populated registration tree','general multi-region operation','independent DKG/bootstrap within the integrated flow','exact SnarkPack EVM','T4/T5 mathematical security obligations'],
  historic_check_scope:'Use pinned report/archive checks above; old g10b1/g10b2/g10c_check.mjs gate the old development allowlist and intentionally are not current-HEAD integration checks. Original scripts and hashes preserved.'});
}
update();
try {
 inventory();
 for(const [name,args] of checks)await run('audit-'+name,'node',args,120);
 const cargo=['+1.99.1'],bin='stage2_matched_cost';
 await run('build','cargo',[...cargo,'build','--release','--locked','--offline','-p','vv-friendly-dkg','--bin',bin]);
 await run('fmt','cargo',[...cargo,'fmt','--all','--','--check'],120);
 await run('clippy','cargo',[...cargo,'clippy','--release','--locked','--offline','-p','vv-friendly-dkg','--bin',bin,'--','-D','warnings']);
 await run('tests','cargo',[...cargo,'test','--release','--locked','--offline','-p','vv-friendly-dkg','--bin',bin]);
 const exe=path.resolve('target/release/'+bin);m.binary_sha256=sha(fs.readFileSync(exe));update();
 for(const phase of phases)await run(phase,exe,[phase,out,runtime],600,true);
 for(const f of historical)assert.equal(sha(fs.readFileSync(f.path)),f.sha256,'historical tracked file changed: '+f.path);
 for(const f of sourceFiles)assert.equal(sha(fs.readFileSync(f.path)),f.sha256,'source changed mid-run: '+f.path);
 m.historical_tracked_files_unchanged=true;
 const ballot=load(path.join(out,'ballot-prove.json')),state=load(path.join(out,'state-prove.json')),agg=load(path.join(out,'aggregate-receive.json'));
 const sum=(a,key)=>a.reduce((s,v)=>s+v[key],0);
 const shared=sum(ballot.proofs,'prove_ms'),st=sum(state.proofs,'prove_ms'),ag=sum(agg.aggregates,'aggregate_ms');
 save(path.join(out,'cost_summary.json'),{status:'PASS_SINGLE_MATCHED_PILOT',source_version_sha256:m.source_version_sha256,
  ballot_proving_ms:shared,state_proving_ms:st,aggregation_ms:ag,
  ballot_setup_ms:load(path.join(out,'ballot-setup.json')).setup_ms,state_setup_ms:load(path.join(out,'state-setup.json')).setup_ms,
  routes:agg.receivers.routes.map(r=>({...r,ballot_proving_ms:shared,aggregation_ms:r.route.startsWith('snarkpack')?ag:0,state_proving_ms:r.route==='snarkpack_state_proof'?st:0,
   sequential_measured_component_sum_ms:shared+r.two_batch_flow_ms+(r.route.startsWith('snarkpack')?ag:0)+(r.route==='snarkpack_state_proof'?st:0)})),
  total_scope:'Component sums from this execution, NOT separately timed per-route end-to-end latency; excludes setup, witness reconstruction, key I/O, sender conversion/serialization/path preparation, certificate authentication, freeze/decryption/P3/EVM. Role CPU-time attribution is separate.',
  prover_rss:phases.filter(p=>p.endsWith('prove')).map(p=>({phase:p,peak_rss_mib:load(path.join(out,p+'.json')).process_peak_rss_mib,scope:'isolated worker; zero setup; key load and shared archive loader included'})),
  pilot_wall_ms:performance.now()-campaignStart,independent_new_elections:0,proof_pools:1,statistical_claim:false,gas:null,calldata:null,exact_snarkpack_evm:false});
 for(const name of fs.readdirSync(out).filter(n=>n.endsWith('.json')&&n!=='execution.json'))m.files.push({path:name,sha256:sha(fs.readFileSync(path.join(out,name)))});
 m.status='PASS_MATCHED_GENERATION_PILOT';
}catch(e){m.status='FAIL_OR_RESOURCE_BLOCKED';m.error=String(e);process.exitCode=1;}
finally{
 m.finished_utc=new Date().toISOString();m.pilot_wall_ms=performance.now()-campaignStart;update();
 console.log(JSON.stringify({status:m.status,output:out,private_runtime:runtime,error:m.error??null}));
}
if(m.status==='PASS_MATCHED_GENERATION_PILOT')verify(out);
