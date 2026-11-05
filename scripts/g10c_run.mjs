// One function-first integration; no optimization or statistical timing campaign.
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';
import {spawnSync,execFileSync} from 'node:child_process';import {createHash} from 'node:crypto';
const [arg]=process.argv.slice(2);assert.ok(arg,'usage: node scripts/g10c_run.mjs NEW_OUTPUT_DIRECTORY');
const out=path.resolve(arg);assert.ok(!fs.existsSync(out));fs.mkdirSync(out,{recursive:true});fs.mkdirSync(path.join(out,'execution'));
const sha=b=>createHash('sha256').update(b).digest('hex');
const env={...process.env,CARGO_BUILD_JOBS:'2',CARGO_PROFILE_RELEASE_STRIP:'none',RAYON_NUM_THREADS:'4'};
const m={schema:'VV/G10-C/FUNCTIONAL-EXECUTION/v1',started_utc:new Date().toISOString(),source_base_sha:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),source_was_dirty:true,
 driver_sha256:sha(fs.readFileSync(new URL(import.meta.url))),commands:[],optimization_campaign:false,statistical_benchmark:false};
const save=()=>fs.writeFileSync(path.join(out,'execution/manifest.json'),JSON.stringify(m,null,2)+'\n');
function run(name,program,args,timeout=180000){const log='execution/'+name+'.log',fd=fs.openSync(path.join(out,log),'wx');const t=performance.now();
 const r=spawnSync(program,args,{env,stdio:['ignore',fd,fd],timeout});fs.closeSync(fd);m.commands.push({name,program,args,cwd:process.cwd(),environment:{CARGO_BUILD_JOBS:'2',CARGO_PROFILE_RELEASE_STRIP:'none',RAYON_NUM_THREADS:'4'},elapsed_ms:performance.now()-t,exit_code:r.status,signal:r.signal,error:r.error?.message??null,log});save();
 process.stdout.write(fs.readFileSync(path.join(out,log)));assert.equal(r.status,0,name);}
save();try{
 const bin='g10c_complete_native';
 run('old-b1-public-archive','node',['scripts/g10b1_report.mjs','--check']);
 run('old-b2-cost-archive','node',['scripts/g10b2_report.mjs','--check']);
 run('build','cargo',['+1.99.1','build','--release','--locked','--offline','-p','vv-friendly-dkg','--bin',bin],300000);
 run('fmt','cargo',['+1.99.1','fmt','--all','--','--check']);
 run('clippy','cargo',['+1.99.1','clippy','--release','--locked','--offline','-p','vv-friendly-dkg','--bin',bin,'--','-D','warnings']);
 run('tests','cargo',['+1.99.1','test','--release','--locked','--offline','-p','vv-friendly-dkg','--bin',bin],300000);
 const exe=path.resolve('target/release/'+bin);m.binary_sha256=sha(fs.readFileSync(exe));
 run('native-connected',exe,['--output-dir',path.join(out,'native'),'--private-dir',path.join(out,'private_diagnostic')],660000);
 const native=JSON.parse(fs.readFileSync(path.join(out,'native/report.json')));
 assert.equal(native.status,'PASS_CONNECTED_NATIVE_FIVE_WORKER_NEW_P3_WINNER_ONCE_AND_FULL_TALLY_DIAGNOSTIC');assert.equal(native.binary_sha256,m.binary_sha256);
 assert.equal(native.new_p3_proofs,1);assert.equal(native.worker_evidence.unique_process_ids,5);
 for(const p of native.sources)assert.equal(sha(fs.readFileSync(p.path)),p.sha256);
 m.source_version_sha256=native.source_version_sha256;
 run('public-winner-separate-process',exe,['--verify-winner',path.join(out,'native/winner_only/public.json'),'--output',path.join(out,'winner_public_process.json')]);
 run('full-tally-separate-process',exe,['--verify-full',path.join(out,'private_diagnostic/full_tally.json'),'--winner-fixture',path.join(out,'native/winner_only/public.json'),'--output',path.join(out,'full_tally_public_process.json')]);
 m.native_report_sha256=sha(fs.readFileSync(path.join(out,'native/report.json')));m.full_tally_payload_withheld_from_public_archive=true;
 m.status='PASS_CONNECTED_NATIVE_NEW_P3_AND_SEPARATE_PUBLIC_RELEASE_VERIFIERS';
}catch(e){m.status='FAIL';m.error=String(e);throw e;}finally{m.finished_utc=new Date().toISOString();save();}
console.log(JSON.stringify({status:m.status,output:out,new_p3_proofs:1,trustee_workers:5,full_tally_payload_published:false}));
