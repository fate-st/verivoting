import fs from 'node:fs';
import path from 'node:path';
import cp from 'node:child_process';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
const task=path.dirname(new URL(import.meta.url).pathname);
const original='/Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter';
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
function hash(p){const h=crypto.createHash('sha256'),fd=fs.openSync(p,'r'),b=Buffer.alloc(1048576);try{let n;while((n=fs.readSync(fd,b,0,b.length,null)))h.update(b.subarray(0,n));}finally{fs.closeSync(fd);}return h.digest('hex');}
const git=(...args)=>cp.execFileSync('git',['--no-optional-locks',...args],{cwd:original,encoding:'utf8'});
function state(){
 const a=JSON.parse(fs.readFileSync(original+'/experiments/stage1_multibatch_raw/gate-a-20261117-06/gate_a.json'));
 const c=JSON.parse(fs.readFileSync(original+'/configs/experiment.STAGE1-MULTIBATCH.json'));
 const pins=[...a.source_pins,...c.source_pins];for(const p of pins)assert.equal(hash(original+'/'+p.path),p.sha256,p.path);
 const head=git('rev-parse','HEAD').trim();assert.equal(head,'3f8813c9d69d0e45631e9acb7bbb247a1119f511');
 const protectedPaths=[...['01','02','03','04','05','06'].flatMap(n=>['experiments/stage1_multibatch_raw/','results/stage1_multibatch/'].map(p=>p+'gate-a-20261117-'+n)),...['01','02'].flatMap(n=>['experiments/stage1_multibatch_raw/','results/stage1_multibatch/'].map(p=>p+'gate-b-20261117-'+n))];
 const snapshots=protectedPaths.map(rel=>{
  const dir=original+'/'+rel;if(!fs.existsSync(dir)){assert.ok(['results/stage1_multibatch/gate-a-20261117-01','results/stage1_multibatch/gate-b-20261117-01'].includes(rel));return {path:rel,status:'NOT_PRESENT'};}
  const entries=[];function walk(p,r){assert.ok(!/[\r\n|]/.test(r));const s=fs.lstatSync(p),m=(s.mode&0o7777).toString(8).padStart(4,'0');if(s.isSymbolicLink())entries.push([r,`L | ${r} | ${fs.readlinkSync(p)}`]);else if(s.isDirectory()){entries.push([r,`D | ${r} | ${m}`]);for(const n of fs.readdirSync(p))walk(p+'/'+n,r==='.'?n:r+'/'+n);}else if(s.isFile())entries.push([r,`F | ${r} | ${m} | ${s.size} | ${hash(p)}`]);else throw Error('special file '+p);}
  walk(dir,'.');entries.sort((a,b)=>Buffer.compare(Buffer.from(a[0]),Buffer.from(b[0])));return {path:rel,status:'EXISTS',sha256:sha(entries.map(x=>x[1]).join('\n')+'\n')};
 });
 const index=git('rev-parse','--path-format=absolute','--git-path','index').trim();
 const untracked=git('ls-files','--others','--exclude-standard','-z').split('\0').filter(Boolean).sort().map(p=>({path:p,sha256:hash(original+'/'+p)}));
 return {head,status_short:git('status','--short'),pins,protected_paths:snapshots,index_sha256:hash(index),diff_sha256:sha(git('diff','HEAD','--binary')),untracked,manifests:['Cargo.toml','Cargo.lock','crates/vv-friendly-dkg/Cargo.toml'].map(p=>({path:p,sha256:hash(original+'/'+p)}))};
}
if(process.argv[2]==='before'){const v=state();fs.writeFileSync(task+'/before.json',JSON.stringify(v,null,2)+'\n',{flag:'wx'});console.log('PASS_16_PATHS_15_PINS_AND_ORIGINAL_BASELINE');}
else if(process.argv[2]==='after'){const b=JSON.parse(fs.readFileSync(task+'/before.json'));assert.deepEqual(state(),b);fs.writeFileSync(task+'/after.json',JSON.stringify({status:'PASS_ORIGINAL_WORKSPACE_AND_16_PATHS_UNCHANGED',checked_at:new Date().toISOString(),baseline:b},null,2)+'\n',{flag:'wx'});console.log('PASS_ORIGINAL_WORKSPACE_AND_16_PATHS_UNCHANGED');}
else throw Error('before | after');
