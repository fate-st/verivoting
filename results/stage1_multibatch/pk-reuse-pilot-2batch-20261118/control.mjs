import fs from 'node:fs';
import path from 'node:path';
import cp from 'node:child_process';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
const root='/tmp/vv-pk-reuse-pilot';
const original='/Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter';
const rust='crates/vv-friendly-dkg/src/bin/stage1_multibatch_materials.rs';
const pilot='crates/vv-friendly-dkg/src/bin/stage1_multibatch_pk_reuse_pilot.rs';
const material='experiments/stage1_multibatch_raw/gate-b-20261117-02/materials';
const newMaterial='experiments/stage1_multibatch_raw/pk-reuse-pilot-2batch';
const approved=['contracts/scripts/stage1_multibatch_compare.mjs','scripts/stage1_multibatch_audit.mjs','docs/STAGE1_MULTIBATCH.md'];
const protectedPaths=[...['01','02','03','04','05'].flatMap(n=>[`experiments/stage1_multibatch_raw/gate-a-20261117-${n}`,`results/stage1_multibatch/gate-a-20261117-${n}`]),'experiments/stage1_multibatch_raw/gate-b-20261117-01','results/stage1_multibatch/gate-b-20261117-01','experiments/stage1_multibatch_raw/gate-a-20261117-06','results/stage1_multibatch/gate-a-20261117-06','experiments/stage1_multibatch_raw/gate-b-20261117-02','results/stage1_multibatch/gate-b-20261117-02'];
const absent=new Set(['results/stage1_multibatch/gate-a-20261117-01','results/stage1_multibatch/gate-b-20261117-01']);
const load=p=>JSON.parse(fs.readFileSync(p,'utf8'));
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const git=(...a)=>cp.execFileSync('git',['--no-optional-locks',...a],{cwd:original,env:{...process.env,GIT_OPTIONAL_LOCKS:'0'}});
function hash(p){const h=crypto.createHash('sha256'),fd=fs.openSync(p,'r'),b=Buffer.alloc(1048576);try{let n;while((n=fs.readSync(fd,b,0,b.length,null)))h.update(b.subarray(0,n));}finally{fs.closeSync(fd);}return h.digest('hex');}
function write(p,v){fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,v,{flag:'wx'});}
function save(p,v){write(p,JSON.stringify(v,null,2)+'\n');}
function state(){
 const a=load(path.join(original,'experiments/stage1_multibatch_raw/gate-a-20261117-06/gate_a.json'));
 const cfg=load(path.join(original,'configs/experiment.STAGE1-MULTIBATCH.json'));
 const pins=[...cfg.source_pins,...a.source_pins];for(const p of pins)assert.equal(hash(path.join(original,p.path)),p.sha256,p.path);
 const head=git('rev-parse','HEAD').toString();assert.equal(head.trim(),'3f8813c9d69d0e45631e9acb7bbb247a1119f511');
 const blob=git('rev-parse','HEAD:'+rust).toString().trim();assert.equal(blob,'547d459670db3f9c15b833a1c24dc28f2cfe1602');
 const changed=git('diff','HEAD','--name-only','-z').toString().split('\0').filter(Boolean).sort();assert.deepEqual(changed,[...approved].sort());
 const index=git('rev-parse','--path-format=absolute','--git-path','index').toString().trim();
 const untracked=git('ls-files','--others','--exclude-standard','-z').toString().split('\0').filter(Boolean).sort().map(p=>({path:p,bytes:fs.statSync(path.join(original,p)).size,sha256:hash(path.join(original,p))}));
 const oldProfile='/tmp/vv-pk-profile';
 const oldPins=fs.readFileSync(oldProfile+'/SHA256SUMS','utf8').trimEnd().split('\n').map(l=>({path:l.slice(66),sha256:l.slice(0,64)}));
 for(const p of oldPins)assert.equal(hash(oldProfile+'/'+p.path),p.sha256,p.path);
 return {head,status_short:git('status','--short').toString(),index_sha256:hash(index),diff_head_sha256:sha(git('diff','HEAD','--binary')),cached_diff_sha256:sha(git('diff','--cached','--binary')),pins,untracked,original_rust_blob:blob,
   manifests:['Cargo.toml','Cargo.lock','crates/vv-friendly-dkg/Cargo.toml'].map(p=>({path:p,sha256:hash(path.join(original,p))})),
   inputs:['research_pk.bin','request.json','setup.json'].map(p=>({path:material+'/'+p,bytes:fs.statSync(path.join(original,material,p)).size,sha256:hash(path.join(original,material,p))})),
   previous_profile_checksums:oldPins,previous_profile_manifest_sha256:hash(oldProfile+'/SHA256SUMS'),paper_sha256:hash('/Users/lijian/Documents/02学习/论文/latex模版/sn-article-template/sn-article.tex')};
}
function snapshot(rel,label){
 const dir=path.join(original,rel);if(!fs.existsSync(dir)){assert.ok(absent.has(rel));return {path:rel,status:'NOT_PRESENT'};}assert.ok(!absent.has(rel));
 const entries=[];const check=s=>{assert.ok(!/[\r\n|]/.test(s));return s;};
 function walk(p,r){const st=fs.lstatSync(p),mode=(st.mode&0o7777).toString(8).padStart(4,'0');check(r);
  if(st.isSymbolicLink())entries.push([r,`L | ${r} | ${check(fs.readlinkSync(p))}`]);
  else if(st.isDirectory()){entries.push([r,`D | ${r} | ${mode}`]);for(const n of fs.readdirSync(p))walk(path.join(p,n),r==='.'?n:r+'/'+n);}
  else if(st.isFile())entries.push([r,`F | ${r} | ${mode} | ${st.size} | ${hash(p)}`]);
  else throw Error('special filesystem entry: '+p);
 }walk(dir,'.');entries.sort((a,b)=>Buffer.compare(Buffer.from(a[0]),Buffer.from(b[0])));
 const text=entries.map(x=>x[1]).join('\n')+'\n';const manifest=`evidence/${label}/${rel.replaceAll('/','__')}.snapshot.txt`;write(root+'/'+manifest,text);
 return {path:rel,status:'EXISTS',entry_count:entries.length,manifest,sha256:sha(text)};
}
function inputPair(){return ['research_pk.bin','request.json','setup.json'].map(name=>{
 const old=path.join(original,material,name),fresh=path.join(root,'worktree',newMaterial,name);
 const historical=hash(old),current=hash(fresh);assert.equal(current,historical,name);
 return {name,bytes:fs.statSync(old).size,historical_sha256:historical,pilot_sha256:current,equal:true};
});}
if(process.argv[2]==='before'){
 const initial=state();assert.equal(initial.inputs[0].bytes,847800624);assert.equal(initial.inputs[0].sha256,'448d9d92b9250aa2bcbf59c82036ab1615f2fe323bbb5a39b8f8d7ec9f73da1c');
 const pkg=fs.readFileSync(path.join(original,'crates/vv-friendly-dkg/Cargo.toml'),'utf8');assert.match(pkg,/^libc\.workspace\s*=\s*true$/m);assert.ok(!/^autobins\s*=\s*false$/m.test(pkg));
 write(root+'/evidence/approved_patches.diff',git('diff','HEAD','--binary','--',...approved));write(root+'/evidence/status.before.txt',initial.status_short);
 save(root+'/before.json',{...initial,original_root:original,captured_at:new Date().toISOString(),snapshot_algorithm:'UTF-8 byte sort, root dot and empty dirs included, 4-digit permission octal D/F/L LF-terminated records; symlinks not followed; ignored caches included',protected_paths:protectedPaths.map(p=>snapshot(p,'before'))});
 console.log('PASS_15_PINS_AND_16_PATH_BASELINE');
}else if(process.argv[2]==='prepare-inputs'){
 const b=load(root+'/before.json');const w=root+'/worktree';for(const p of [...b.pins,...b.manifests])assert.equal(hash(w+'/'+p.path),p.sha256,p.path);
 const dir=w+'/'+newMaterial;fs.mkdirSync(dir,{recursive:true});for(const n of ['request.json','setup.json'])fs.copyFileSync(path.join(original,material,n),dir+'/'+n,fs.constants.COPYFILE_EXCL);
 fs.symlinkSync(path.join(original,material,'research_pk.bin'),dir+'/research_pk.bin');
 save(root+'/input_integrity_before.json',{status:'PASS_BYTE_IDENTICAL_INPUTS',pairs:inputPair(),request_sha256_equal:true});
 console.log('PASS_INPUTS_AND_PIN_IDENTICAL_TRANSPLANT');
}else if(process.argv[2]==='after'){
 const b=load(root+'/before.json'),s=state();for(const key of Object.keys(s))assert.deepEqual(s[key],b[key],key);
 const snapshots=protectedPaths.map(p=>snapshot(p,'after'));assert.deepEqual(snapshots.map(({manifest,...x})=>x),b.protected_paths.map(({manifest,...x})=>x));
 const inputs=inputPair();assert.deepEqual(inputs,load(root+'/input_integrity_before.json').pairs);
 for(const p of [...b.pins,...b.manifests])assert.equal(hash(root+'/worktree/'+p.path),p.sha256,p.path);
 const wt=root+'/worktree';const changed=cp.execFileSync('git',['--no-optional-locks','diff','HEAD','--name-only','-z'],{cwd:wt}).toString().split('\0').filter(Boolean).sort();assert.deepEqual(changed,[...approved].sort());
 const untracked=cp.execFileSync('git',['--no-optional-locks','ls-files','--others','--exclude-standard','-z'],{cwd:wt}).toString().split('\0').filter(Boolean).sort();
 const mandatory=[pilot,...['request.json','setup.json','research_pk.bin'].map(n=>newMaterial+'/'+n)];
 const allowed=new Set([...mandatory,...['batch_1.json','batch_2.json'].map(n=>newMaterial+'/'+n)]);
 for(const p of mandatory)assert.ok(untracked.includes(p),p);
 for(const p of untracked)assert.ok(allowed.has(p),'unexpected isolated file: '+p);
 if(load(root+'/outer_wall_rss.json').returncode===0)assert.equal(untracked.length,6);
 save(root+'/integrity_after.json',{status:'PASS_ORIGINAL_AND_INPUTS_UNCHANGED',checked_at:new Date().toISOString(),original_head:s.head.trim(),head_and_status_exact_match:true,index_unchanged:true,paper_unchanged:true,previous_profile_unchanged:true,all_untracked_hashes_unchanged:true,source_pins_unchanged:true,isolated_untracked_paths:untracked,request_sha256_equal:true,inputs,protected_paths:snapshots,gate_c:'NOT_AUTHORIZED_NOT_STARTED'});
 console.log('PASS_ORIGINAL_AND_INPUTS_UNCHANGED');
}else throw Error('use before | prepare-inputs | after');
