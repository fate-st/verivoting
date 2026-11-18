import fs from 'node:fs';
import path from 'node:path';
import cp from 'node:child_process';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import {pathToFileURL} from 'node:url';
const T=process.argv[2]??'/private/tmp/vv-a08-path-guard-test-Q9X1St';
const I='/private/tmp/vv-pk-reuse-integration.PohVG2/worktree';
const R=await import(pathToFileURL(I+'/contracts/scripts/stage1_multibatch_compare.mjs'));
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const output=(name,v)=>fs.writeFileSync(T+'/'+name,typeof v==='string'?v:JSON.stringify(v,null,2)+'\n',{flag:'wx'});
const readCheck=()=>JSON.parse(cp.execFileSync(process.execPath,['/private/tmp/vv-a08-blocked-upload.Jvt2Cj/check.mjs'],{encoding:'utf8',maxBuffer:8*1024*1024,env:{...process.env,GIT_OPTIONAL_LOCKS:'0'}}));
const before=readCheck();output('integrity_before.json',before);
const threePaths=['integrity_before.json','prebuild_readonly_review.md','synthetic-reject/gate_c.json'];
function preparations(){return threePaths.map(n=>{const p=I+'/results/stage1_multibatch/gate-a-20261117-08/'+n,s=fs.lstatSync(p);return {path:p,type:s.isFile()?'regular file':s.isSymbolicLink()?'symlink':'other',bytes:s.size,mode:(s.mode&0o7777).toString(8).padStart(4,'0'),sha256:hash(fs.readFileSync(p))};});}
const prepBefore=preparations();output('preparation_files_before.json',prepBefore);
const identity={physical_worktree:fs.realpathSync.native(I),head:before.integration.head,branch:before.integration.branch,status_short_before:before.integration.status,
 pre_edit_runner_sha256:'a80a13415d5db6f9286a93a21094af4ffcd37d18e34874cc15830fba12e2204d',
 pre_edit_status:' M contracts/scripts/stage1_multibatch_compare.mjs\n M crates/vv-friendly-dkg/src/bin/stage1_multibatch_materials.rs\n M docs/STAGE1_MULTIBATCH.md\n M scripts/stage1_multibatch_audit.mjs\n?? contracts/node_modules\n?? experiments/stage1_multibatch_raw/gate-a-20261117-07/\n?? experiments/stage1_multibatch_raw/gate-b-20261117-03/\n?? results/stage1_multibatch/gate-a-20261117-07/\n?? results/stage1_multibatch/gate-a-20261117-08/\n?? results/stage1_multibatch/gate-b-20261117-03/\n'};
output('identity.json',identity);
const snapshot=p=>{const rows=[];function walk(f,r){const s=fs.lstatSync(f);rows.push({path:r,type:s.isDirectory()?'D':s.isSymbolicLink()?'L':'F',mode:s.mode&0o7777,...(s.isSymbolicLink()?{link:fs.readlinkSync(f)}:s.isFile()?{size:s.size,sha256:hash(fs.readFileSync(f))}:{})});if(s.isDirectory())for(const n of fs.readdirSync(f).sort())walk(f+'/'+n,r+'/'+n);}walk(p,'.');return rows;};
const sim=T+'/sim';fs.mkdirSync(sim);
const old=sim+'/original',integration=sim+'/integration';
const rawParent=integration+'/experiments/stage1_multibatch_raw',resultParent=integration+'/results/stage1_multibatch';
const raw=rawParent+'/gate-a-20261117-08',results=resultParent+'/gate-a-20261117-08';
const protectedPaths=[old,...['gate-a-20261117-07','gate-b-20261117-03'].flatMap(n=>[rawParent+'/'+n,resultParent+'/'+n])];
for(const d of [old+'/gate-a-20261117-06',old+'/gate-b-20261117-02',...protectedPaths.slice(1),results,sim+'/rustup-read-only',sim+'/case/gate-a-20261117-06',sim+'/unicode/\u00e9'])fs.mkdirSync(d,{recursive:true});
for(const d of [old+'/gate-a-20261117-06',old+'/gate-b-20261117-02',...protectedPaths.slice(1)])fs.writeFileSync(d+'/frozen.json','frozen\n',{flag:'wx'});
fs.writeFileSync(results+'/preparation.json','unchanged\n',{flag:'wx'});
const env={VV_INTEGRATION_TARGET_DIR:sim+'/target',CARGO_TARGET_DIR:sim+'/target',CARGO_HOME:sim+'/cargo-home',RUSTUP_HOME:sim+'/rustup-read-only',TMPDIR:sim+'/tmp'};
const testPolicy={mode:'TEST_ONLY',testRoot:T,raw,results,blacklists:[...protectedPaths,sim+'/case/gate-a-20261117-06',sim+'/unicode/\u00e9']};
const opts={env,testPolicy};
const cases=[];const rejectionRows=[];
function reject(label,target,options=opts,expected){
 const b=snapshot(sim);let error;
 try{R.safeWriteProtectedFile(target,'UNSAFE',options);}catch(e){error=e;}
 assert.ok(error,label+' should reject');if(expected)assert.equal(error.code,expected,label);
 assert.deepEqual(snapshot(sim),b,label+' zero-filesystem-write assertion');
 const result={label,input:target,result:'REJECT',code:error.code??null,message:error.message,zero_directory_or_file_changes:true};rejectionRows.push(result);return result;
}
async function test(id,title,body){const sub=await body();cases.push({id,title,status:'PASS',details:sub});console.log('PASS '+id+' '+title);}
await test(1,'new A08 via existing parent inode; duplicate and streaming',async()=>{
 const target=raw+'/nested/new.json',plan=R.protectedWritePlan(target,opts);
 assert.equal(plan.anchor.anchor,rawParent);assert.equal(fs.existsSync(raw),false);
 const beforeStat=fs.statSync(rawParent,{bigint:true}),payload=Buffer.from('new payload\n');
 const log=R.safeWriteProtectedFile(target,payload,opts);assert.deepEqual(fs.readFileSync(target),payload);
 const native=log.trim().split('\n').map(JSON.parse),written=native.find(v=>v.event==='WRITTEN');
 assert.equal(written.anchor_ino,beforeStat.ino.toString());assert.equal(written.anchor_dev,beforeStat.dev.toString());
 assert.equal(fs.statSync(target).mode&0o777,0o600);
 const duplicate=reject('duplicate',target,opts,'EEXIST');
 let nativeDuplicate;try{cp.execFileSync(plan.binary,plan.args,{input:'no overwrite',encoding:'utf8',stdio:['pipe','pipe','pipe'],env:{...process.env,...env}});}catch(e){nativeDuplicate={code:e.status,stderr:e.stderr};}
 assert.ok(nativeDuplicate);assert.match(nativeDuplicate.stderr,/errno=17/);assert.deepEqual(fs.readFileSync(target),payload);
 const streamTarget=raw+'/logs/stream.stdout.log',w=R.safeWriteProtectedFile(streamTarget,undefined,{...opts,stream:true});
 await w.ready;const streamData=Buffer.alloc(262144,0xa5);w.child.stdin.end(streamData);const streamResult=await w.completion;
 assert.deepEqual(fs.readFileSync(streamTarget),streamData);
 const executable=raw+'/build-target/release/stage1_multibatch_materials';
 const executableWrite=R.safeWriteProtectedFile(executable,'NOT_A_REAL_WORKER', {...opts,executable:true});
 assert.equal(fs.statSync(executable).mode&0o777,0o700);
 return {input:target,anchor:plan.anchor,native,duplicate,nativeDuplicate,controlled_executable_copy:{executed:false,mode:'0700',native:executableWrite},stream:{bytes:streamData.length,sha256:hash(streamData),...streamResult}};
});
await test(2,'original A06 read-only',()=>reject('original A06',old+'/gate-a-20261117-06/new.json',opts,'READ_ONLY_PATH'));
await test(3,'original B02 read-only',()=>reject('original B02',old+'/gate-b-20261117-02/new.json',opts,'READ_ONLY_PATH'));
await test(4,'integration A07 read-only',()=>reject('integration A07',rawParent+'/gate-a-20261117-07/new.json',opts,'READ_ONLY_PATH'));
await test(5,'integration B03 read-only',()=>reject('integration B03',resultParent+'/gate-b-20261117-03/new.json',opts,'READ_ONLY_PATH'));
await test(6,'/tmp alias denied for both approved and historical roots',()=>[
 reject('tmp approved',raw.replace('/private/tmp/','/tmp/')+'/alias.json',opts,'SYMLINK_COMPONENT'),
 reject('tmp protected',(old+'/gate-a-20261117-06/new.json').replace('/private/tmp/','/tmp/'),opts,'READ_ONLY_PATH')]);
await test(7,'actual filesystem case variants',()=>{
 const names=['gate-a-20261117-06','Gate-A-20261117-06','GATE-A-20261117-06'];
 return names.map(n=>{const p=sim+'/case/'+n;let s;try{s=fs.statSync(p,{bigint:true});}catch(e){s={error:e.code};}
  return {path:p,stat:s.ino?{dev:s.dev.toString(),ino:s.ino.toString()}:s,...reject(n,p+'/new.json',opts,'READ_ONLY_PATH')};});
});
await test(8,'NFC/NFD actual inode equivalence and non-ASCII new component',()=>{
 const nfc=sim+'/unicode/\u00e9',nfd=sim+'/unicode/e\u0301';
 const stat=p=>{try{const s=fs.statSync(p,{bigint:true});return {exists:true,dev:s.dev.toString(),ino:s.ino.toString()};}catch(e){return {exists:false,code:e.code};}};
 const a=stat(nfc),b=stat(nfd);return {nfc:{path:nfc,stat:a},nfd:{path:nfd,stat:b},same_inode:a.ino===b.ino&&a.dev===b.dev,
 a:reject('8a NFD protected alias',nfd+'/new.json',opts,'READ_ONLY_PATH'),b:reject('8b nonASCII new dir',raw+'/new-dir-\u00e9/new.json',opts,'NON_ASCII_NEW_COMPONENT')};
});
await test(9,'symlinks to history and unprotected intermediate directories',()=>{
 fs.symlinkSync(old+'/gate-a-20261117-06',raw+'/link-a06');fs.symlinkSync(rawParent+'/gate-b-20261117-03',raw+'/link-b03');
 fs.mkdirSync(sim+'/neutral');fs.symlinkSync(sim+'/neutral',raw+'/middle-link');
 return [reject('A06 symlink',raw+'/link-a06/new.json',opts,'READ_ONLY_PATH'),reject('B03 symlink',raw+'/link-b03/new.json',opts,'READ_ONLY_PATH'),
 reject('intermediate symlink',raw+'/middle-link/new.json',opts,'SYMLINK_COMPONENT')];
});
await test(10,'existing preparation file protected by EEXIST',()=>reject('existing preparation',results+'/preparation.json',opts,'EEXIST'));
await test(11,'environment admission, all 15 nested pairs, PROD isolation and dev/ino',()=>{
 const rows=[reject('RUSTUP target',env.RUSTUP_HOME+'/new.json',opts,'READ_ONLY_PATH')];
 for(const key of Object.keys(env)){
  const missing={...env};delete missing[key];rows.push(reject('missing '+key,raw+'/never-created/missing.json',{...opts,env:missing},'INVALID_ABSOLUTE_PATH'));
  rows.push(reject('relative '+key,raw+'/never-created/relative.json',{...opts,env:{...env,[key]:'relative'}},'INVALID_ABSOLUTE_PATH'));
 }
 rows.push(reject('TMPDIR=/tmp',raw+'/never-created/tmp.json',{...opts,env:{...env,TMPDIR:'/tmp'}}));
 rows.push(reject('TMPDIR=/private/tmp',raw+'/never-created/tmp-root.json',{...opts,env:{...env,TMPDIR:'/private/tmp'}}));
 const names=['A08_RAW','A08_RESULTS','VV_INTEGRATION_TARGET_DIR','CARGO_TARGET_DIR','CARGO_HOME','TMPDIR'];
 const originalRoots=[raw,results,env.VV_INTEGRATION_TARGET_DIR,env.CARGO_TARGET_DIR,env.CARGO_HOME,env.TMPDIR];
 for(let i=0;i<6;i++)for(let j=i+1;j<6;j++){
  const e={...env},p={...testPolicy};const nested=originalRoots[i]+'/nested-guard';
  if(j===1)p.results=nested;else e[names[j]]=nested;
  rows.push(reject('nested pair '+names[i]+'/'+names[j],raw+'/never-created/overlap.json',{env:e,testPolicy:p},'OVERLAPPING_WRITABLE_ROOTS'));
 }
 // Exercise native failures with a real plan obtained through Runner; neither creates any output.
 const plan=R.protectedWritePlan(raw+'/never-created/native.json',opts);
 function nativeReject(label,mutate){const b=snapshot(sim),args=[...plan.args];mutate(args);let e;
  try{cp.execFileSync(plan.binary,args,{input:'bad',encoding:'utf8',stdio:['pipe','pipe','pipe'],env:{...process.env,...env}});}catch(err){e=err;}
  assert.ok(e,label);assert.deepEqual(snapshot(sim),b);const r={label,result:'REJECT',exit_code:e.status,stderr:e.stderr,zero_directory_or_file_changes:true};rows.push(r);return r;}
 nativeReject('PROD test pause',a=>{a[a.indexOf('--mode')+1]='PROD';a.push('--test-pause-before-file-open-ms','1');});
 nativeReject('PROD refuses temporary test root',a=>{a[a.indexOf('--mode')+1]='PROD';});
 nativeReject('wrong expected device',a=>{a[a.indexOf('--expected-dev')+1]='0';});
 nativeReject('wrong expected inode',a=>{a[a.indexOf('--expected-ino')+1]='0';});
 rows.push(reject('Runner wrong mode for test pause',raw+'/never-created/pause.json',{env,testPauseMs:1}));
 return rows;
});
await test(12,'all unsafe-path failures produced no output, logs or command receipts',()=>{
 const banned=raw+'/never-created';assert.equal(fs.existsSync(banned),false);
 const targets=[raw+'/../gate-a-20261117-07/escape.json',raw+'/new..component/new.json',sim+'/outside/new.json'];
 const rows=targets.map((p,i)=>reject('zero-write '+i,p));
 assert.equal(fs.existsSync(banned),false);return {rejections_with_full_snapshot_assertion:rejectionRows.length,rows};
});
await test(13,'same-UID rename after verified directory FD open',async()=>{
 const orig=raw+'/orig_dir',moved=raw+'/moved_dir',trap=sim+'/trap_dir';fs.mkdirSync(orig);fs.mkdirSync(trap);
 const s=fs.statSync(orig,{bigint:true});let opened;
 const w=R.safeWriteProtectedFile(orig+'/race.json',undefined,{...opts,stream:true,testPauseMs:1000,onReady:r=>{
  opened=r;assert.equal(r.parent_ino,s.ino.toString());fs.renameSync(orig,moved);fs.symlinkSync(trap,orig);
 }});
 w.child.stdin.end('original inode payload\n');const done=await w.completion;
 assert.deepEqual(fs.readdirSync(trap),[]);assert.equal(fs.readFileSync(moved+'/race.json','utf8'),'original inode payload\n');
 assert.equal(fs.statSync(moved,{bigint:true}).ino,s.ino);
 return {input:orig+'/race.json',opened,completed:done,moved_ino:fs.statSync(moved,{bigint:true}).ino.toString(),trap_entries:[],
  risk:'An opened dirFD follows the original directory inode after a same-UID rename. It does not prevent movement of that directory.'};
});
const after=readCheck();output('integrity_after.json',after);assert.deepEqual(after.original,before.original);assert.deepEqual(after.integration.frozen_paths,before.integration.frozen_paths);
assert.deepEqual(preparations(),prepBefore);output('preparation_files_after.json',preparations());
const other=['crates/vv-friendly-dkg/src/bin/stage1_multibatch_materials.rs','docs/STAGE1_MULTIBATCH.md','scripts/stage1_multibatch_audit.mjs'];
for(const f of other)assert.equal(after.integration.files.find(v=>v.path===f).sha256,before.integration.files.find(v=>v.path===f).sha256);
const src=fs.readFileSync(I+'/contracts/scripts/stage1_multibatch_compare.mjs','utf8');
assert.equal(/fs\.(?:writeFileSync|appendFileSync|mkdirSync|createWriteStream|renameSync|writeSync|writeFile|appendFile|mkdir|rename|copyFileSync)\s*\(/.test(src),false);
const openCalls=src.split('\n').flatMap((text,i)=>text.includes('fs.openSync(')?[{line:i+1,text}]:[]);assert.ok(openCalls.every(v=>/,'r'\)/.test(v.text)));
const inventory=src.split('\n').flatMap((text,i)=>/writeNew\(|safeWriteProtectedFile\(/.test(text)?[{line:i+1,text}]:[]);
output('write_inventory.json',inventory);
const binary=I+'/'+R.nativeHelperBinary,source=I+'/'+R.nativeHelperSource;
const result={status:'PASS_A08_FILESYSTEM_HARDENING',test_root:T,platform:process.platform,architecture:cp.execFileSync('uname',['-m'],{encoding:'utf8'}).trim(),
 compiler:cp.execFileSync('cc',['--version'],{encoding:'utf8'}),c_source_sha256:hash(fs.readFileSync(source)),c_binary_sha256:hash(fs.readFileSync(binary)),
 runner_sha256:hash(fs.readFileSync(I+'/contracts/scripts/stage1_multibatch_compare.mjs')),helper_pin:R.SAFE_FS_EXPECTED_SHA256,
 tests:cases,write_inventory:inventory,read_only_open_calls:openCalls,
 protected_integrity:{original_matches_historical_baseline:true,original_16_paths:before.original.protected_paths.length,integration_4_paths_unchanged:true,
 existing_three_preparation_files_unchanged:true,other_three_preexisting_modified_sources_unchanged:true,head_unchanged:true,index_unchanged:true},
 not_executed:{rust_compilation:true,solidity_compilation:true,setup:true,prove:true,worker:true,evm:true,stage3_environment_preparation:true,commit:true,push:true},
 gate_states:{A06_B02:'FROZEN',A07_B03:'FROZEN',A08:'NOT_CREATED',C:'NOT_AUTHORIZED_NOT_STARTED',D:'NOT_STARTED'}};
assert.equal(result.c_binary_sha256,result.helper_pin);output('test_results.json',result);
console.log(JSON.stringify({status:result.status,tests:cases.length,test_root:T,c_source_sha256:result.c_source_sha256,c_binary_sha256:result.c_binary_sha256,gate_states:result.gate_states}));
