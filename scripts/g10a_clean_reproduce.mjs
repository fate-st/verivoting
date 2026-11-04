// Run the UNMODIFIED e1e477f executable in a clean worktree, exporting all outputs.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawnSync,execFileSync} from 'node:child_process';
const [sourceArg,targetArg,outArg]=process.argv.slice(2);
assert.ok(sourceArg&&targetArg&&outArg,'usage: node g10a_clean_reproduce.mjs CLEAN_WORKTREE TARGET_CACHE NEW_OUTPUT_DIR');
const source=fs.realpathSync(sourceArg),target=path.resolve(targetArg),out=path.resolve(outArg);
assert.ok(!out.startsWith(source+path.sep)&&out!==source,'outputs must stay outside the clean source');
assert.ok(!target.startsWith(source+path.sep)&&target!==source,'build cache must be external too');
assert.ok(!fs.existsSync(out),'output directory must be new');
const git=args=>execFileSync('git',args,{cwd:source,encoding:'utf8'}).trim();
const BASE='e1e477f94247dc8ee261400cfb2ec381e0f44fc3';
const sha=b=>createHash('sha256').update(b).digest('hex');
function checkpoint(){const state={head:git(['rev-parse','HEAD']),status:git(['status','--porcelain'])};assert.equal(state.head,BASE);assert.equal(state.status,'');return state;}
const before=checkpoint();fs.mkdirSync(out);fs.mkdirSync(path.join(out,'execution'));
const env={...process.env,CARGO_TARGET_DIR:target,CARGO_BUILD_JOBS:'2',CARGO_PROFILE_RELEASE_STRIP:'none',RAYON_NUM_THREADS:'4'};
const manifest={schema:'VV/G10-A/CLEAN-REPRODUCTION/v1',source_commit:BASE,source_worktree:source,external_target_cache:target,cache_reused:true,started_utc:new Date().toISOString(),before,driver_sha256:sha(fs.readFileSync(new URL(import.meta.url))),commands:[],runs:[],new_setup:0,new_proofs:0,new_aggregates:0,new_evm:0};
const save=()=>fs.writeFileSync(path.join(out,'execution/manifest.json'),JSON.stringify(manifest,null,2)+'\n');
function run(name,program,args,timeout=150000){
 const pre=checkpoint(),log='execution/'+name+'.log';const fd=fs.openSync(path.join(out,log),'wx');const started=new Date().toISOString(),t=performance.now();
 const r=spawnSync(program,args,{cwd:source,env,stdio:['ignore',fd,fd],timeout});fs.closeSync(fd);
 const post=checkpoint();manifest.commands.push({name,program,args,cwd:source,env:{CARGO_TARGET_DIR:target,CARGO_BUILD_JOBS:'2',CARGO_PROFILE_RELEASE_STRIP:'none',RAYON_NUM_THREADS:'4'},started_utc:started,elapsed_ms:performance.now()-t,exit_code:r.status,signal:r.signal,error:r.error?.message??null,log,source_before:pre,source_after:post});save();
 process.stdout.write(fs.readFileSync(path.join(out,log)));assert.equal(r.status,0,name);
}
save();
run('build','cargo',['+1.99.1','build','--release','--locked','--offline','-p','vv-friendly-dkg','--bin','friendly-g10a-native-fold'],300000);
run('fmt','cargo',['+1.99.1','fmt','--all','--','--check']);
run('clippy','cargo',['+1.99.1','clippy','--release','--locked','--offline','-p','vv-friendly-dkg','--bin','friendly-g10a-native-fold','--','-D','warnings']);
run('tests','cargo',['+1.99.1','test','--release','--locked','--offline','-p','vv-friendly-dkg','--bin','friendly-g10a-native-fold']);
const executable=path.join(target,'release/friendly-g10a-native-fold');manifest.binary_sha256=sha(fs.readFileSync(executable));
for(let i=1;i<=3;i++){
 const file='run-'+i+'.json';run('benchmark-'+i,executable,['--output',path.join(out,file)]);
 const bytes=fs.readFileSync(path.join(out,file)),raw=JSON.parse(bytes);
 assert.equal(raw.status,'PASS_MATCHED_TWO_B4_NATIVE_RECEIVE_PATHS');assert.equal(raw.source_base_sha,BASE);assert.equal(raw.git_dirty,false);assert.equal(raw.binary_sha256,manifest.binary_sha256);
 assert.equal(raw.windows.length,20);assert.equal(raw.method.paired_rounds,10);assert.equal(raw.proof_generation_count,0);assert.equal(raw.evm_executed,false);
 for(const p of raw.sources){assert.equal(sha(fs.readFileSync(path.join(source,p.path))),p.sha256);assert.equal(sha(execFileSync('git',['show',BASE+':'+p.path],{cwd:source})),p.sha256);}
 manifest.runs.push({process:i,path:file,sha256:sha(bytes),source_clean:true,paired_rounds:10,windows:20});save();
}
manifest.after=checkpoint();manifest.finished_utc=new Date().toISOString();manifest.status='PASS_CLEAN_E1E477F_THREE_PROCESSES_30_PAIRED_ROUNDS';manifest.paired_rounds=30;manifest.raw_windows=60;manifest.independent_proof_pools=1;save();
console.log(JSON.stringify({status:manifest.status,source_commit:BASE,source_dirty:false,processes:3,paired_rounds:30,raw_windows:60,independent_proof_pools:1,new_setup:0,new_proofs:0,new_evm:0}));
