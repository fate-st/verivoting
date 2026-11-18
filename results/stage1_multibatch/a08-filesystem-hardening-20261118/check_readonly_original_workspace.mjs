import fs from 'node:fs';
import cp from 'node:child_process';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
const I='/private/tmp/vv-pk-reuse-integration.PohVG2/worktree';
const O='/Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter';
const load=p=>JSON.parse(fs.readFileSync(p));
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const git=(root,...args)=>cp.execFileSync('git',['--no-optional-locks',...args],{cwd:root,encoding:'utf8',env:{...process.env,GIT_OPTIONAL_LOCKS:'0'}});
function hash(p){const h=crypto.createHash('sha256'),fd=fs.openSync(p,'r'),b=Buffer.alloc(1048576);try{let n;while((n=fs.readSync(fd,b,0,b.length,null)))h.update(b.subarray(0,n));}finally{fs.closeSync(fd);}return h.digest('hex');}
function snapshot(root,rel){const p=root+'/'+rel;if(!fs.existsSync(p))return {path:rel,status:'NOT_PRESENT'};const entries=[];
function walk(p,r){const s=fs.lstatSync(p),m=(s.mode&0o7777).toString(8).padStart(4,'0');if(s.isSymbolicLink())entries.push([r,'L | '+r+' | '+fs.readlinkSync(p)]);else if(s.isDirectory()){entries.push([r,'D | '+r+' | '+m]);for(const n of fs.readdirSync(p))walk(p+'/'+n,r==='.'?n:r+'/'+n);}else if(s.isFile())entries.push([r,'F | '+r+' | '+m+' | '+s.size+' | '+hash(p)]);else throw Error('special file');}
walk(p,'.');entries.sort((a,b)=>Buffer.compare(Buffer.from(a[0]),Buffer.from(b[0])));return {path:rel,status:'EXISTS',sha256:sha(entries.map(x=>x[1]).join('\n')+'\n')};}
const baseline=load(I+'/results/stage1_multibatch/gate-a-20261117-08/integrity_before.json');
function originalState(){const b=baseline.original;return {head:git(O,'rev-parse','HEAD').trim(),status_short:git(O,'status','--short'),
pins:b.pins.map(p=>({path:p.path,sha256:hash(O+'/'+p.path)})),protected_paths:b.protected_paths.map(p=>snapshot(O,p.path)),
index_sha256:hash(git(O,'rev-parse','--path-format=absolute','--git-path','index').trim()),diff_sha256:sha(git(O,'diff','HEAD','--binary')),
untracked:git(O,'ls-files','--others','--exclude-standard','-z').split('\0').filter(Boolean).sort().map(p=>({path:p,sha256:hash(O+'/'+p)})),
manifests:b.manifests.map(p=>({path:p.path,sha256:hash(O+'/'+p.path)}))};}
const sources=['contracts/scripts/stage1_multibatch_compare.mjs','crates/vv-friendly-dkg/src/bin/stage1_multibatch_materials.rs','docs/STAGE1_MULTIBATCH.md','scripts/stage1_multibatch_audit.mjs'];
const added=['results/stage1_multibatch/gate-a-20261117-08/integrity_before.json','results/stage1_multibatch/gate-a-20261117-08/prebuild_readonly_review.md','results/stage1_multibatch/gate-a-20261117-08/synthetic-reject/gate_c.json'];
const original=originalState();assert.deepEqual(original,baseline.original);
const frozen=baseline.integration_frozen.map(p=>snapshot(I,p.path));assert.deepEqual(frozen,baseline.integration_frozen);
assert.equal(git(I,'rev-parse','HEAD').trim(),baseline.integration_head);assert.equal(git(I,'branch','--show-current').trim(),'feat/stage1-pk-reuse-integration');
assert.equal(hash(git(I,'rev-parse','--path-format=absolute','--git-path','index').trim()),baseline.integration_index_sha256);
assert.deepEqual(git(I,'diff','--name-only').trim().split('\n').sort(),sources);
for(const p of baseline.manifests)assert.equal(hash(I+'/'+p.path),p.sha256);
assert.equal(fs.existsSync(I+'/experiments/stage1_multibatch_raw/gate-a-20261117-08'),false);
assert.equal(fs.existsSync(I+'/experiments/stage1_multibatch_raw/gate-c-control/gate_c_authorization.json'),false);
const files=[...sources,...added].map(p=>{const f=I+'/'+p,s=fs.lstatSync(f);assert.ok(s.isFile()&&s.size<2*1024*1024);const data=fs.readFileSync(f),t=data.toString('utf8');assert.ok(!t.includes('\0'));
for(const pattern of [/-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/,/\bgh[pousr]_[A-Za-z0-9_]{30,}\b/,/\bgithub_pat_[A-Za-z0-9_]{30,}\b/,/\bAKIA[0-9A-Z]{16}\b/])assert.ok(!pattern.test(t),'credential-like text: '+p);
return {path:p,bytes:s.size,sha256:sha(data)};});
console.log(JSON.stringify({original,integration:{head:baseline.integration_head,branch:git(I,'branch','--show-current').trim(),status:git(I,'status','--short'),index_sha256:baseline.integration_index_sha256,diff_sha256:sha(git(I,'diff','HEAD','--binary')),frozen_paths:frozen,files},setup_generation_count:0,proof_generation_count:0,vm_creation_count:0,evm_transaction_count:0}));
