// Bounded real execution, with outputs outside source; no setup/prove/aggregation/EVM.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawnSync, execFileSync} from 'node:child_process';
const [output] = process.argv.slice(2);
assert.ok(output, 'usage: node scripts/g10b1_run.mjs NEW_OUTPUT_DIRECTORY');
const out=path.resolve(output);
assert.ok(!fs.existsSync(out),'do not overwrite results');
fs.mkdirSync(out,{recursive:true});fs.mkdirSync(path.join(out,'execution'));
const git=(...a)=>execFileSync('git',a,{encoding:'utf8'}).trim();
const sha=b=>createHash('sha256').update(b).digest('hex');
const env={...process.env,CARGO_BUILD_JOBS:'2',CARGO_PROFILE_RELEASE_STRIP:'none',RAYON_NUM_THREADS:'4'};
const manifest={schema:'VV/G10-B1/EXECUTION/v1',started_utc:new Date().toISOString(),source_base_sha:git('rev-parse','HEAD'),source_git_dirty:!!git('status','--porcelain'),driver_sha256:sha(fs.readFileSync(new URL(import.meta.url))),commands:[],runs:[],new_setup:0,new_proofs:0,new_aggregates:0,new_evm:0};
const save=()=>fs.writeFileSync(path.join(out,'execution/manifest.json'),JSON.stringify(manifest,null,2)+'\n');
function run(name,program,args,timeout=150000){
 const log='execution/'+name+'.log',fd=fs.openSync(path.join(out,log),'wx');const t=performance.now();
 const r=spawnSync(program,args,{env,stdio:['ignore',fd,fd],timeout});fs.closeSync(fd);
 manifest.commands.push({name,program,args,started_cwd:process.cwd(),environment:{CARGO_BUILD_JOBS:env.CARGO_BUILD_JOBS,CARGO_PROFILE_RELEASE_STRIP:env.CARGO_PROFILE_RELEASE_STRIP,RAYON_NUM_THREADS:env.RAYON_NUM_THREADS},elapsed_ms:performance.now()-t,exit_code:r.status,signal:r.signal,error:r.error?.message??null,log});save();
 process.stdout.write(fs.readFileSync(path.join(out,log)));assert.equal(r.status,0,name);
}
save();
try{
 const cargo=['+1.99.1'];const bin='friendly-g10b1-matched-native';
 run('build','cargo',[...cargo,'build','--release','--locked','--offline','-p','vv-friendly-dkg','--bin',bin],300000);
 run('fmt','cargo',[...cargo,'fmt','--all','--','--check']);
 run('clippy','cargo',[...cargo,'clippy','--release','--locked','--offline','-p','vv-friendly-dkg','--bin',bin,'--','-D','warnings']);
 run('tests','cargo',[...cargo,'test','--release','--locked','--offline','-p','vv-friendly-dkg','--bin',bin],300000);
 const executable=path.resolve('target/release/'+bin);manifest.binary_sha256=sha(fs.readFileSync(executable));
 const routes=['individual_groth16_per_vote','transaction_batch_individual_groth16','snarkpack_direct_fold','snarkpack_state_proof'];
 for(const [name,route] of [['matched',null],...routes.map((r,i)=>['memory-'+(i+1),r])]){
  const file=name+'.json';run(name,executable,['--output',path.join(out,file),...(route?['--memory-route',route]:[])]);
  const bytes=fs.readFileSync(path.join(out,file)),raw=JSON.parse(bytes);
  assert.equal(raw.binary_sha256,manifest.binary_sha256);assert.equal(raw.source_base_sha,manifest.source_base_sha);
  for(const p of raw.sources)assert.equal(sha(fs.readFileSync(p.path)),p.sha256);
  manifest.source_version_sha256??=raw.source_version_sha256;assert.equal(raw.source_version_sha256,manifest.source_version_sha256);
  manifest.runs.push({file,route,sha256:sha(bytes),status:raw.status});save();
 }
 manifest.status='PASS_G10_B1_FOUR_ROUTE_INTAKE_AND_ISOLATED_PROCESS_RSS';
}catch(e){manifest.status='FAIL';manifest.error=String(e);throw e;}
finally{manifest.finished_utc=new Date().toISOString();save();}
console.log(JSON.stringify({status:manifest.status,output:out,source_base:manifest.source_base_sha,source_version:manifest.source_version_sha256}));
