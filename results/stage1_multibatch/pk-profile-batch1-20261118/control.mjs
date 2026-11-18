import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import cp from 'node:child_process';
import assert from 'node:assert/strict';

const root = '/tmp/vv-pk-profile';
const original = '/Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter';
const targetHead = '3f8813c9d69d0e45631e9acb7bbb247a1119f511';
const rust = 'crates/vv-friendly-dkg/src/bin/stage1_multibatch_materials.rs';
const approved = ['contracts/scripts/stage1_multibatch_compare.mjs','scripts/stage1_multibatch_audit.mjs','docs/STAGE1_MULTIBATCH.md'];
const materials = 'experiments/stage1_multibatch_raw/gate-b-20261117-02/materials';
const protectedPaths = [
  ...['01','02','03','04','05'].flatMap(n => [`experiments/stage1_multibatch_raw/gate-a-20261117-${n}`,`results/stage1_multibatch/gate-a-20261117-${n}`]),
  'experiments/stage1_multibatch_raw/gate-b-20261117-01','results/stage1_multibatch/gate-b-20261117-01',
  'experiments/stage1_multibatch_raw/gate-a-20261117-06','results/stage1_multibatch/gate-a-20261117-06',
  'experiments/stage1_multibatch_raw/gate-b-20261117-02','results/stage1_multibatch/gate-b-20261117-02',
];
const absent = new Set(['results/stage1_multibatch/gate-a-20261117-01','results/stage1_multibatch/gate-b-20261117-01']);
const git = (...args) => cp.execFileSync('git',['--no-optional-locks',...args],{cwd:original,env:{...process.env,GIT_OPTIONAL_LOCKS:'0'}});
const load = p => JSON.parse(fs.readFileSync(p,'utf8'));
const bytesHash = b => crypto.createHash('sha256').update(b).digest('hex');
function hashFile(p) {
  const h=crypto.createHash('sha256'), fd=fs.openSync(p,'r'), b=Buffer.alloc(1024*1024);
  try {let n;while((n=fs.readSync(fd,b,0,b.length,null)))h.update(b.subarray(0,n));}finally{fs.closeSync(fd);}
  return h.digest('hex');
}
function save(p,v){fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,JSON.stringify(v,null,2)+'\n',{flag:'wx'});}
function write(p,b){fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,b,{flag:'wx'});}
function clean(s){assert.ok(!/[\r\n|]/.test(s),'ambiguous canonical record');return s;}
function snapshot(rel,label){
  const dir=path.join(original,rel);
  if(!fs.existsSync(dir)){assert.ok(absent.has(rel),`unexpected absent: ${rel}`);return {path:rel,status:'NOT_PRESENT'};}
  assert.ok(!absent.has(rel),`unexpected existing: ${rel}`);
  const entries=[];
  function visit(p,r){
    const st=fs.lstatSync(p);const mode=(st.mode&0o7777).toString(8).padStart(4,'0');clean(r);
    if(st.isSymbolicLink())entries.push({relative_path:r,type:'L',target:clean(fs.readlinkSync(p))});
    else if(st.isDirectory()){
      entries.push({relative_path:r,type:'D',mode});
      for(const name of fs.readdirSync(p))visit(path.join(p,name),r==='.'?name:r+'/'+name);
    }else if(st.isFile())entries.push({relative_path:r,type:'F',mode,byte_size:st.size,sha256:hashFile(p)});
    else throw Error(`special filesystem entry prohibited: ${p}`);
  }
  visit(dir,'.');entries.sort((a,b)=>Buffer.compare(Buffer.from(a.relative_path),Buffer.from(b.relative_path)));
  const text=entries.map(e=>e.type==='D'?`D | ${e.relative_path} | ${e.mode}`:e.type==='L'?`L | ${e.relative_path} | ${e.target}`:`F | ${e.relative_path} | ${e.mode} | ${e.byte_size} | ${e.sha256}`).join('\n')+'\n';
  const manifest=`evidence/${label}/${rel.replaceAll('/','__')}.snapshot.txt`;
  write(path.join(root,manifest),text);
  return {path:rel,status:'EXISTS',entry_count:entries.length,manifest,sha256:bytesHash(text)};
}
function state(){
  const cfg=load(path.join(original,'configs/experiment.STAGE1-MULTIBATCH.json'));
  const a=load(path.join(original,'experiments/stage1_multibatch_raw/gate-a-20261117-06/gate_a.json'));
  const pins=[...cfg.source_pins,...a.source_pins];
  const checkedPins=pins.map(p=>{const actual=hashFile(path.join(original,p.path));assert.equal(actual,p.sha256,p.path);return {...p,actual};});
  const head=git('rev-parse','HEAD').toString();assert.equal(head.trim(),targetHead);
  const blob=git('rev-parse',`HEAD:${rust}`).toString().trim();assert.equal(blob,'547d459670db3f9c15b833a1c24dc28f2cfe1602');
  const changed=git('diff','HEAD','--name-only','-z').toString().split('\0').filter(Boolean);
  assert.deepEqual([...changed].sort(),[...approved].sort());
  const index=git('rev-parse','--path-format=absolute','--git-path','index').toString().trim();
  const untracked=git('ls-files','--others','--exclude-standard','-z').toString().split('\0').filter(Boolean).sort((a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b)));
  return {head,status_short:git('status','--short').toString(),index_sha256:hashFile(index),
    diff_head_sha256:bytesHash(git('diff','HEAD','--binary')),cached_diff_sha256:bytesHash(git('diff','--cached','--binary')),
    checked_pins:checkedPins,original_rust_blob:blob,
    untracked:untracked.map(p=>({path:p,byte_size:fs.statSync(path.join(original,p)).size,sha256:hashFile(path.join(original,p))})),
    inputs:['research_pk.bin','request.json','setup.json'].map(p=>({path:materials+'/'+p,byte_size:fs.statSync(path.join(original,materials,p)).size,sha256:hashFile(path.join(original,materials,p))})),
    approved_patches:approved.map(p=>({path:p,sha256:hashFile(path.join(original,p))})),
  };
}
if(process.argv[2]==='before'){
  const initial=state();
  assert.equal(initial.inputs[0].byte_size,847800624);assert.equal(initial.inputs[0].sha256,'448d9d92b9250aa2bcbf59c82036ab1615f2fe323bbb5a39b8f8d7ec9f73da1c');
  const pkg=fs.readFileSync(path.join(original,'crates/vv-friendly-dkg/Cargo.toml'),'utf8');
  assert.match(pkg,/^libc\.workspace\s*=\s*true$/m);assert.ok(!/^autobins\s*=\s*false$/m.test(pkg));
  write(path.join(root,'evidence/approved_patches.diff'),git('diff','HEAD','--binary','--',...approved));
  write(path.join(root,'evidence/cached_diff_for_record_only.diff'),git('diff','--cached','--binary'));
  write(path.join(root,'evidence/status.before.txt'),initial.status_short);
  for(const f of initial.untracked){
    const dest=path.join(root,'evidence/untracked_bytes',f.path);fs.mkdirSync(path.dirname(dest),{recursive:true});
    fs.copyFileSync(path.join(original,f.path),dest,fs.constants.COPYFILE_EXCL);assert.equal(hashFile(dest),f.sha256);
  }
  const snapshots=protectedPaths.map(p=>snapshot(p,'before'));
  save(path.join(root,'before.json'),{original_root:original,captured_at:new Date().toISOString(),snapshot_algorithm:'UTF-8 byte-sort, root dot included, 4-digit permission octal, LF terminated D/F/L records, symlinks not followed; all entries including ignored caches',...initial,protected_paths:snapshots});
  console.log(JSON.stringify({status:'PASS_ORIGINAL_BASELINE_CAPTURED',head:initial.head.trim(),pins:initial.checked_pins.length,untracked_files:initial.untracked.length,protected_paths:snapshots.length,existing:snapshots.filter(p=>p.status==='EXISTS').length}));
}else if(process.argv[2]==='after'){
  const before=load(path.join(root,'before.json'));const current=state();
  for(const k of Object.keys(current))assert.deepEqual(current[k],before[k],`original changed: ${k}`);
  const snapshots=protectedPaths.map(p=>snapshot(p,'after'));
  assert.deepEqual(snapshots.map(({manifest,...r})=>r),before.protected_paths.map(({manifest,...r})=>r));
  const worktree=path.join(root,'worktree');
  const isolatedPins=before.checked_pins.map(p=>{const actual=hashFile(path.join(worktree,p.path));assert.equal(actual,p.sha256,p.path);return {...p,isolated_actual:actual};});
  for(const p of ['Cargo.toml','Cargo.lock','crates/vv-friendly-dkg/Cargo.toml',rust])assert.equal(hashFile(path.join(original,p)),hashFile(path.join(worktree,p)),p);
  const changed=cp.execFileSync('git',['--no-optional-locks','diff','HEAD','--name-only','-z'],{cwd:worktree}).toString().split('\0').filter(Boolean);
  assert.deepEqual(changed.sort(),[...approved].sort());
  const isolatedUntracked=cp.execFileSync('git',['--no-optional-locks','ls-files','--others','--exclude-standard','-z'],{cwd:worktree}).toString().split('\0').filter(Boolean).sort();
  assert.deepEqual(isolatedUntracked,[
    'crates/vv-friendly-dkg/src/bin/stage1_multibatch_pk_profile.rs',
    ...['batch_1.json','request.json','research_pk.bin','setup.json'].map(p=>'experiments/stage1_multibatch_raw/pk-profile-batch1/'+p),
  ].sort());
  save(path.join(root,'integrity_after.json'),{status:'PASS_ORIGINAL_AND_PINNED_INPUTS_UNCHANGED',checked_at:new Date().toISOString(),head_and_status_exact_match:true,index_unchanged:true,tracked_and_untracked_bytes_match:true,isolated_untracked_paths:isolatedUntracked,protected_paths:snapshots,inputs:current.inputs,source_pins:isolatedPins,gate_c:'NOT_AUTHORIZED_NOT_STARTED'});
  console.log('PASS_ORIGINAL_AND_PINNED_INPUTS_UNCHANGED');
}else if(process.argv[2]==='verify-worktree'){
  const before=load(path.join(root,'before.json'));const w=path.join(root,'worktree');
  for(const p of before.checked_pins)assert.equal(hashFile(path.join(w,p.path)),p.sha256,p.path);
  for(const p of ['Cargo.toml','Cargo.lock','crates/vv-friendly-dkg/Cargo.toml',rust])assert.equal(hashFile(path.join(w,p)),hashFile(path.join(original,p)),p);
  const dest=path.join(w,'experiments/stage1_multibatch_raw/pk-profile-batch1');fs.mkdirSync(dest,{recursive:true});
  for(const name of ['request.json','setup.json']){fs.copyFileSync(path.join(original,materials,name),path.join(dest,name),fs.constants.COPYFILE_EXCL);assert.equal(hashFile(path.join(dest,name)),hashFile(path.join(original,materials,name)));}
  fs.symlinkSync(path.join(original,materials,'research_pk.bin'),path.join(dest,'research_pk.bin'));
  save(path.join(root,'transplant.json'),{status:'PASS_PIN_IDENTICAL_TRANSPLANT',approved:before.approved_patches,input_paths:before.inputs,untracked_bytes_captured:before.untracked.length,untracked_exclusion:'Historical evidence copies retained in evidence/untracked_bytes; worker reads only frozen tracked sources plus unchanged request/setup and PK link. No prepare/Runner/A/B audit execution.'});
  console.log('PASS_PIN_IDENTICAL_TRANSPLANT');
}else throw Error('use before | verify-worktree | after');
