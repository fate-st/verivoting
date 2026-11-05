// One bounded two-B4 aggregation campaign; no Groth16 setup or proof generation.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawnSync,execFileSync} from 'node:child_process';
const [arg]=process.argv.slice(2);assert.ok(arg,'usage: node scripts/g10b2_run.mjs NEW_OUTPUT_DIR');
const out=path.resolve(arg);assert.ok(!fs.existsSync(out),'refuse overwrite');fs.mkdirSync(out,{recursive:true});fs.mkdirSync(path.join(out,'execution'));
const sha=b=>createHash('sha256').update(b).digest('hex');
const env={...process.env,CARGO_BUILD_JOBS:'2',CARGO_PROFILE_RELEASE_STRIP:'none',RAYON_NUM_THREADS:'4'};
const m={schema:'VV/G10-B2/EXECUTION/v1',started_utc:new Date().toISOString(),source_base_sha:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),source_was_dirty:!!execFileSync('git',['status','--porcelain'],{encoding:'utf8'}).trim(),driver_sha256:sha(fs.readFileSync(new URL(import.meta.url))),commands:[],new_groth16_setups:0,new_groth16_proofs:0,new_evm:0};
const save=()=>fs.writeFileSync(path.join(out,'execution/manifest.json'),JSON.stringify(m,null,2)+'\n');
function run(name,program,args,timeout=120000){const log='execution/'+name+'.log';const fd=fs.openSync(path.join(out,log),'wx');const t=performance.now();
 const r=spawnSync(program,args,{env,stdio:['ignore',fd,fd],timeout});fs.closeSync(fd);
 m.commands.push({name,program,args,cwd:process.cwd(),environment:{CARGO_BUILD_JOBS:'2',CARGO_PROFILE_RELEASE_STRIP:'none',RAYON_NUM_THREADS:'4'},elapsed_ms:performance.now()-t,exit_code:r.status,signal:r.signal,error:r.error?.message??null,log});save();process.stdout.write(fs.readFileSync(path.join(out,log)));assert.equal(r.status,0,name);}
save();try{
 const bin='g10b2_aggregation_cost';
 run('b1-archive-before','node',['scripts/g10b1_report.mjs','--check']);
 run('build','cargo',['+1.99.1','build','--release','--locked','--offline','-p','vv-friendly-dkg','--bin',bin],300000);
 run('fmt','cargo',['+1.99.1','fmt','--all','--','--check']);
 run('clippy','cargo',['+1.99.1','clippy','--release','--locked','--offline','-p','vv-friendly-dkg','--bin',bin,'--','-D','warnings']);
 run('receiver-regression','cargo',['+1.99.1','test','--release','--locked','--offline','-p','vv-friendly-dkg','--bin','friendly-g10b1-matched-native'],300000);
 const binary=path.resolve('target/release/'+bin);m.binary_sha256=sha(fs.readFileSync(binary));
 run('aggregation',binary,['--output',path.join(out,'aggregation_cost.json')]);
 const bytes=fs.readFileSync(path.join(out,'aggregation_cost.json')),raw=JSON.parse(bytes);
 assert.equal(raw.status,'PASS_EXACT_TWO_B4_AGGREGATION_GENERATION');assert.equal(raw.binary_sha256,m.binary_sha256);
 assert.equal(raw.new_aggregate_count_including_warmup,26);assert.equal(raw.new_groth16_setups,0);
 for(const s of raw.sources)assert.equal(sha(fs.readFileSync(s.path)),s.sha256);
 m.source_version_sha256=raw.source_version_sha256;m.raw_sha256=sha(bytes);
 run('b1-archive-after','node',['scripts/g10b1_report.mjs','--check']);
 m.status='PASS_G10_B2_BOUNDED_AGGREGATION_COST';
}catch(e){m.status='FAIL';m.error=String(e);throw e;}finally{m.finished_utc=new Date().toISOString();save();}
console.log(JSON.stringify({status:m.status,output:out,source_version:m.source_version_sha256,new_groth16_proofs:0,new_evm:0}));
