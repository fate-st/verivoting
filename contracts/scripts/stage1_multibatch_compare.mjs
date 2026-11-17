// Explicitly gated Stage1 runner. Gate A ONLY compiles; no automatic B/C/D.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {spawn,execFileSync} from 'node:child_process';
import solc from 'solc';
import {ContractFactory,Interface,AbiCoder,keccak256} from 'ethers';
import {Common,Hardfork,Mainnet} from '@ethereumjs/common';
import {createVM,runTx,paramsVM} from '@ethereumjs/vm';
import {createBlock} from '@ethereumjs/block';
import {createLegacyTx,getCalldataFloorGas} from '@ethereumjs/tx';
import {createAccount,createAddressFromPrivateKey,createAddressFromString,hexToBytes,bytesToHex} from '@ethereumjs/util';
export const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
export const configPath='configs/experiment.STAGE1-MULTIBATCH.json';
export const sourceFiles=[configPath,'crates/vv-friendly-dkg/src/bin/stage1_multibatch_materials.rs',
 'contracts/src/stage1/Stage1Receivers.sol','contracts/scripts/stage1_multibatch_compare.mjs','scripts/stage1_multibatch_audit.mjs'];
export const read=p=>fs.readFileSync(path.resolve(root,p));
export const load=p=>JSON.parse(read(p));
export const sha=b=>createHash('sha256').update(b).digest('hex');
export const wire=s=>Buffer.from(s.replace(/^0x/,''),'hex');
export const hex=s=>s.startsWith('0x')?s:'0x'+s;
export const json=v=>JSON.stringify(v,(_,x)=>typeof x==='bigint'?x.toString():x,2)+'\n';
export const writeNew=(p,v)=>{const f=path.resolve(root,p);fs.mkdirSync(path.dirname(f),{recursive:true});fs.writeFileSync(f,json(v),{flag:'wx'});};
export const cfg=load(configPath);
export const git=(...a)=>execFileSync('git',a,{cwd:root,encoding:'utf8',env:{...process.env,GIT_OPTIONAL_LOCKS:'0'}}).trim();
const abi=AbiCoder.defaultAbiCoder();
const metric=(value,status='MEASURED')=>({status,value});
const normalized=v=>JSON.parse(JSON.stringify(v,(_,x)=>typeof x==='bigint'?x.toString():x));
const absolute=p=>path.resolve(root,p);
const scriptPath='contracts/scripts/stage1_multibatch_compare.mjs';
export const EXPECTED_HEAD='3f8813c9d69d0e45631e9acb7bbb247a1119f511';
export const EXPECTED_PARENT='f086879c4dc247e528d62d2253d76d6afd603669';
export const EXPECTED_GRANDPARENT='2444269b52573d9c2f3e551ef7d6b8bb9d0bd008';
export const EXPECTED_BASE='72ccc561d8934b0d761e17764ef9012bfe012626';
export const EXPECTED_GRANDPARENT_TREE='92f7331a7d9b534d17108409342851381811aa52';
export const EXPECTED_PARENT_TREE='3b8b57b375bb0391afe431935057a2711c5608e9';
export const EXPECTED_HEAD_TREE='a21e84e05a2c653218a50ec99769b81766d12b91';
export const EXPECTED_BINARY_SHA256='da5c0f46378af23ac61905e5f4f96d42d174dfc45e3711eec53863901fc1a488';
export const approvedPatches=[scriptPath,'scripts/stage1_multibatch_audit.mjs','docs/STAGE1_MULTIBATCH.md'];
// Fixed historical inventory, not inferred from whichever HEAD happens to be checked out.
export const expectedAdded=[
 'configs/experiment.STAGE1-MULTIBATCH.json',scriptPath,'contracts/src/stage1/Stage1Receivers.sol',
 'crates/vv-friendly-dkg/src/bin/stage1_multibatch_materials.rs','docs/G19_SNARKMUX_EVM_FEASIBILITY.md','docs/STAGE1_MULTIBATCH.md',
 ...['SHA256SUMS','criteria.json','environment.json','evidence/prior_paper_source.json','evidence/prototype_response.json',
 'execution.json','historical_files.json','logs/source-access.stderr.log','logs/source-access.stdout.log','report.md','results.json','source_inventory.json','verifier_inventory.json'].map(p=>'experiments/snarkmux_evm/'+p),
 ...['01','02'].flatMap(id=>[
  ...(id==='01'?['gate_a_failed.json']:['compiled.json','gate_a.json']),
  ...['cargo-build','rust-format','rust-version','syntax-stage1_multibatch_audit.mjs','syntax-stage1_multibatch_compare.mjs']
   .flatMap(n=>['command.json','stderr.log','stdout.log'].map(s=>'logs/'+n+'.'+s)),
  'protected_files.json'].map(p=>`experiments/stage1_multibatch_raw/gate-a-20261117-${id}/`+p)),
 'results/g19/decision.json','results/stage1_multibatch/gate-a-20261117-02/SHA256SUMS',
 'results/stage1_multibatch/gate-a-20261117-02/gate_a_audit.json','scripts/g19_snarkmux_source_admission.mjs','scripts/stage1_multibatch_audit.mjs'
].sort();
// Second fixed inventory: exactly 3 Modified source files and 31 Added A-03 artifacts.
export const expectedA03Changes=[
 ['M','contracts/scripts/stage1_multibatch_compare.mjs'],['M','docs/STAGE1_MULTIBATCH.md'],['M','scripts/stage1_multibatch_audit.mjs'],
 ...['compiled.json','gate_a.json','logs/cargo-build.command.json','logs/cargo-build.stderr.log','logs/cargo-build.stdout.log',
 'logs/preflight-cargo-version.command.json','logs/preflight-cargo-version.stderr.log','logs/preflight-cargo-version.stdout.log',
 'logs/preflight-rust-version.command.json','logs/preflight-rust-version.stderr.log','logs/preflight-rust-version.stdout.log',
 'logs/rust-format.command.json','logs/rust-format.stderr.log','logs/rust-format.stdout.log',
 'logs/rust-version.command.json','logs/rust-version.stderr.log','logs/rust-version.stdout.log',
 'logs/syntax-stage1_multibatch_audit.mjs.command.json','logs/syntax-stage1_multibatch_audit.mjs.stderr.log','logs/syntax-stage1_multibatch_audit.mjs.stdout.log',
 'logs/syntax-stage1_multibatch_compare.mjs.command.json','logs/syntax-stage1_multibatch_compare.mjs.stderr.log','logs/syntax-stage1_multibatch_compare.mjs.stdout.log',
 'preflight_b.json','protected_files.json','static_checks.json','storage_layout.json'].map(p=>['A','experiments/stage1_multibatch_raw/gate-a-20261117-03/'+p]),
 ...['SHA256SUMS','gate_a_audit.json','preflight_b.json','report.md'].map(p=>['A','results/stage1_multibatch/gate-a-20261117-03/'+p])
].sort((a,b)=>a[1]<b[1]?-1:a[1]>b[1]?1:0).map(([status,p])=>status+'\t'+p);
// Third independently checked static inventory; preserve both earlier inventories verbatim.
export const expectedA04Changes=[
 'M\tcontracts/scripts/stage1_multibatch_compare.mjs',
 'M\tdocs/STAGE1_MULTIBATCH.md',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/binary_reproducibility.json',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/compiled.json',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/gate_a.json',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/cargo-build.command.json',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/cargo-build.stderr.log',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/cargo-build.stdout.log',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/preflight-cargo-version.command.json',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/preflight-cargo-version.stderr.log',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/preflight-cargo-version.stdout.log',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/preflight-rust-version.command.json',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/preflight-rust-version.stderr.log',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/preflight-rust-version.stdout.log',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/rust-format.command.json',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/rust-format.stderr.log',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/rust-format.stdout.log',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/rust-version.command.json',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/rust-version.stderr.log',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/rust-version.stdout.log',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/syntax-stage1_multibatch_audit.mjs.command.json',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/syntax-stage1_multibatch_audit.mjs.stderr.log',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/syntax-stage1_multibatch_audit.mjs.stdout.log',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/syntax-stage1_multibatch_compare.mjs.command.json',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/syntax-stage1_multibatch_compare.mjs.stderr.log',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/logs/syntax-stage1_multibatch_compare.mjs.stdout.log',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/preflight_b.json',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/protected_files.json',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/static_checks.json',
 'A\texperiments/stage1_multibatch_raw/gate-a-20261117-04/storage_layout.json',
 'A\tresults/stage1_multibatch/gate-a-20261117-04/SHA256SUMS',
 'A\tresults/stage1_multibatch/gate-a-20261117-04/gate_a_audit.json',
 'A\tresults/stage1_multibatch/gate-a-20261117-04/preflight_b.json',
 'A\tresults/stage1_multibatch/gate-a-20261117-04/report.md',
 'M\tscripts/stage1_multibatch_audit.mjs'
];
const inside=(p,d)=>p.startsWith(d+'/');
export function resultsPathForRun(rawRun){runPath(rawRun);return 'results/stage1_multibatch/'+path.basename(rawRun);}
export function validateConfig({phase,run,aRun,bRun,rawRun}={}){
 assert.ok(['A','B','C','D'].includes(phase),'explicit phase required');
 assert.equal(git('rev-parse','HEAD'),EXPECTED_HEAD);
 assert.equal(git('rev-parse','HEAD^'),EXPECTED_PARENT);
 assert.equal(git('rev-parse','HEAD~2'),EXPECTED_GRANDPARENT);
 assert.equal(git('rev-parse','HEAD~3'),EXPECTED_BASE);
 assert.equal(cfg.base_commit,EXPECTED_BASE);
 assert.equal(git('rev-parse',EXPECTED_GRANDPARENT+'^{tree}'),EXPECTED_GRANDPARENT_TREE);
 assert.equal(git('rev-parse',EXPECTED_PARENT+'^{tree}'),EXPECTED_PARENT_TREE);
 assert.equal(git('rev-parse','HEAD^{tree}'),EXPECTED_HEAD_TREE);
 assert.equal(expectedAdded.length,59);
 const added=git('diff','--name-status',EXPECTED_BASE+'..'+EXPECTED_GRANDPARENT).split('\n');
 assert.deepEqual(added,expectedAdded.map(p=>'A\t'+p),'fixed 59-file all-Added inventory');
 const a03Changes=git('diff','--name-status',EXPECTED_GRANDPARENT+'..'+EXPECTED_PARENT).split('\n');
 assert.equal(expectedA03Changes.length,34);assert.equal(expectedA03Changes.filter(p=>p.startsWith('M\t')).length,3);
 assert.equal(expectedA03Changes.filter(p=>p.startsWith('A\t')).length,31);
 assert.deepEqual(a03Changes,expectedA03Changes,'fixed 34-file 3M+31A inventory');
 const a04Changes=git('diff','--name-status',EXPECTED_PARENT+'..'+EXPECTED_HEAD).split('\n');
 assert.equal(expectedA04Changes.length,35);assert.equal(expectedA04Changes.filter(p=>p.startsWith('M\t')).length,3);
 assert.equal(expectedA04Changes.filter(p=>p.startsWith('A\t')).length,32);
 assert.deepEqual(a04Changes,expectedA04Changes,'fixed 35-file 3M+32A inventory');
 let dirs;
 if(phase==='A'){assert.equal(path.basename(runPath(run)),'gate-a-20261117-06');dirs=[run,resultsPathForRun(run)];}
 else if(phase==='D'){
  assert.match(rawRun??'',/^experiments\/stage1_multibatch_raw\/[A-Za-z0-9_-]+$/);
  assert.equal(run,resultsPathForRun(rawRun));dirs=[rawRun,run];
 }else{
  runPath(run);runPath(aRun);assert.notEqual(run,aRun);dirs=[run,resultsPathForRun(run),aRun,resultsPathForRun(aRun)];
  if(phase==='C'){runPath(bRun);assert.ok(bRun!==run&&bRun!==aRun);dirs.push(bRun,resultsPathForRun(bRun));}
 }
 const allowedDirs=[...dirs,...historyDirs];
 const workspaces={};
 for(const [kind,args] of [['staged',['diff','--cached','--name-only','-z']],['unstaged',['diff','--name-only','-z']],
  ['untracked',['ls-files','--others','--exclude-standard','-z']]]){
  const paths=git(...args).split('\0').filter(Boolean);workspaces[kind]=paths;
  for(const p of paths)assert.ok(approvedPatches.includes(p)||allowedDirs.some(d=>inside(p,d)),`${phase}: unauthorized ${kind} path ${p}`);
 }
 assert.equal(cfg.release_opt_level,1);assert.match(read('Cargo.toml').toString(),/opt-level\s*=\s*1/);
 for(const p of cfg.source_pins)assert.equal(sha(read(p.path)),p.sha256,p.path);
 assert.equal(solc.version().split('+')[0],cfg.solidity.version);
 for(const n of ['vm','common','tx'])assert.equal(load(`contracts/node_modules/@ethereumjs/${n}/package.json`).version,cfg.ethereumjs_version);
 return {phase,head:EXPECTED_HEAD,parent:EXPECTED_PARENT,grandparent:EXPECTED_GRANDPARENT,base:EXPECTED_BASE,
  grandparent_tree:EXPECTED_GRANDPARENT_TREE,parent_tree:EXPECTED_PARENT_TREE,tree:EXPECTED_HEAD_TREE,
  added,a03_changes:a03Changes,a04_changes:a04Changes,workspaces,
  untracked_scope:'git ls-files --others --exclude-standard; historical ignored build caches independently included in recursive snapshots'};
}
export function recursiveSnapshot(p){
 const full=absolute(p);if(!fs.existsSync(full))return {path:p,exists:false,entries:[],file_count:0};
 const entries=[];
 function walk(f,relative){const s=fs.lstatSync(f);const type=s.isSymbolicLink()?'symlink':s.isDirectory()?'directory':s.isFile()?'file':'other';
  assert.notEqual(type,'other','unsupported archive entry '+f);
  const e={path:relative,type};if(type==='file'){e.bytes=s.size;e.sha256=sha(fs.readFileSync(f));}
  if(type==='symlink'){e.target=fs.readlinkSync(f);e.sha256=sha(Buffer.from(e.target));}entries.push(e);
  if(type==='directory')for(const name of fs.readdirSync(f).sort())walk(path.join(f,name),relative==='.'?name:relative+'/'+name);
 }walk(full,'.');
 return {path:p,exists:true,file_count:entries.filter(e=>e.type==='file').length,entry_count:entries.length,
  entries_sha256:sha(Buffer.from(json(entries))),entries};
}
export const historyDirs=['experiments/stage1_multibatch_raw/gate-a-20261117-01','experiments/stage1_multibatch_raw/gate-a-20261117-02',
 'results/stage1_multibatch/gate-a-20261117-02','results/stage1_multibatch/gate-a-20261117-01',
 'experiments/stage1_multibatch_raw/gate-a-20261117-03','results/stage1_multibatch/gate-a-20261117-03',
 'experiments/stage1_multibatch_raw/gate-a-20261117-04','results/stage1_multibatch/gate-a-20261117-04',
 'experiments/stage1_multibatch_raw/gate-a-20261117-05','results/stage1_multibatch/gate-a-20261117-05',
 'experiments/stage1_multibatch_raw/gate-b-20261117-01','results/stage1_multibatch/gate-b-20261117-01'];
export function sourcePatchSnapshot(){return approvedPatches.map(p=>({path:p,sha256:sha(read(p))}));}
export function readonlyInputSnapshot({phase,aRun,bRun,rawRun}){
 const dirs=[];
 if(phase==='B'||phase==='C'){dirs.push(runPath(aRun),resultsPathForRun(aRun));
  if(phase==='C')dirs.push(runPath(bRun),resultsPathForRun(bRun));}
 if(phase==='D')dirs.push(runPath(rawRun));
 return dirs.map(p=>{const s=recursiveSnapshot(p);assert.equal(s.exists,true,'read-only input must exist: '+p);return s;});
}
export function checkReadonlyInputs(before){for(const p of before)assert.deepEqual(recursiveSnapshot(p.path),p,'read-only input changed: '+p.path);}
export function protectedSnapshot({phase,run,aRun,bRun,rawRun}){
 const excluded=phase==='D'?[run,rawRun]:[run,resultsPathForRun(run)];
 const files=execFileSync('git',['ls-files','--cached','--others','--exclude-standard','-z'],{cwd:root,encoding:'utf8'})
  .split('\0').filter(p=>p&&!approvedPatches.includes(p)&&!excluded.some(d=>inside(p,d))).sort();
 const paper=path.resolve(root,'../../sn-article-template/sn-article.tex');
 const archives=historyDirs.map(recursiveSnapshot);
 const expectedAbsentHistoryDirs=new Set(['results/stage1_multibatch/gate-a-20261117-01','results/stage1_multibatch/gate-b-20261117-01']);
 for(const a of archives)assert.equal(a.exists,!expectedAbsentHistoryDirs.has(a.path),'history path existence mismatch: '+a.path);
 const scope=Object.fromEntries(Object.entries({phase,run,aRun,bRun,rawRun}).filter(([,v])=>v!==undefined&&v!==null));
 return {scope,files:files.map(p=>({path:p,sha256:sha(read(p))})),archives,readonly_inputs:readonlyInputSnapshot(scope),
  paper:{path:paper,sha256:sha(fs.readFileSync(paper))}};
}
export function checkProtected(p){assert.deepEqual(protectedSnapshot(p.scope),p,'protected file inventory/content including historical ignored caches changed');}
export function pragueCommon(){
 const common=new Common({chain:Mainnet,hardfork:Hardfork.Prague});
 // Same VM parameter table used by VM.constructor; loading parameters does NOT instantiate a VM.
 common.updateParams(paramsVM);
 for(const eip of [2537,3529,7623])assert.equal(common.isActivatedEIP(eip),true,'Prague EIP '+eip);
 assert.equal(common.isActivatedEIP(8037),false);assert.equal(BigInt(common.param('maxRefundQuotient')),5n);return common;
}
export function pragueStatic(){const c=pragueCommon();return {status:'PASS_STATIC',hardfork:c.hardfork(),chain_id:c.chainId().toString(),
  active_eips:[2537,3529,7623],inactive_eips:[8037],max_refund_quotient:c.param('maxRefundQuotient').toString(),vm_created:false};}
function runPath(p){assert.ok(p&&/^experiments\/stage1_multibatch_raw\/[A-Za-z0-9_-]+$/.test(p),'new isolated run path required');return p;}
function sources(){
 const result={};
 function add(name){if(result[name])return;const content=read('contracts/src/'+name).toString();result[name]={content};
  for(const m of content.matchAll(/import\s+"([^"]+)"\s*;/g))add(path.posix.normalize(path.posix.join(path.posix.dirname(name),m[1])));}
 add('stage1/Stage1Receivers.sol');return result;
}
export function compileSolidity(){
 const src=sources();const settings={...cfg.solidity,outputSelection:{'*':{'*':['abi','evm.bytecode.object','evm.deployedBytecode.object','storageLayout']}}};delete settings.version;
 const start=performance.now(),out=JSON.parse(solc.compile(JSON.stringify({language:'Solidity',sources:src,settings})));
 for(const e of out.errors??[])if(e.severity==='error')throw Error(e.formattedMessage);else console.warn(e.formattedMessage);
 const sizes=Object.entries(out.contracts).flatMap(([file,cs])=>Object.entries(cs).map(([name,c])=>({file,name,creation_bytes:c.evm.bytecode.object.length/2,runtime_bytes:c.evm.deployedBytecode.object.length/2})));
 for(const n of ['Stage1LegacySettlement','Stage1DirectFoldSettlement','Stage1StateProofSettlement']){
  const s=sizes.find(x=>x.name===n);assert.ok(s&&s.creation_bytes>0);assert.ok(s.runtime_bytes<=24576&&s.creation_bytes<=49152,'code size '+n);}
 const storage_checks=checkStorageLayouts(out.contracts);
 return {status:'PASS_COMPILE_ONLY',compiler:solc.version(),settings,contracts:out.contracts,sizes,
  storage_checks,compilation_ms:metric(performance.now()-start),sources:Object.entries(src).map(([p,v])=>({path:'contracts/src/'+p,sha256:sha(Buffer.from(v.content))}))};
}
export function checkStorageLayouts(contracts){
 return ['Stage1ReceiverBase','Stage1LegacySettlement','Stage1DirectFoldSettlement','Stage1StateProofSettlement'].map(name=>{
  const l=contracts['stage1/Stage1Receivers.sol'][name].storageLayout;
  const fields=['nextBatchIndex','spentCount','positionInBatch','frozen','frozenTarget'].map(n=>{const s=l.storage.find(x=>x.label===n);assert.ok(s,n);return {...s,bytes:l.types[s.type].numberOfBytes};});
  const slot=BigInt(fields[0].slot);
  for(const [i,offset,bytes] of [[0,0,'8'],[1,8,'8'],[2,16,'1'],[3,17,'1']]){
   assert.equal(BigInt(fields[i].slot),slot);assert.equal(fields[i].offset,offset);assert.equal(fields[i].bytes,bytes);}
  assert.equal(BigInt(fields[4].slot),slot+1n);assert.equal(fields[4].offset,0);assert.equal(fields[4].bytes,'32');
  return {contract:name,status:'PASS_COMPILE',packed_slot:slot.toString(),fields};
 });
}
async function command(run,label,program,args,extraEnv={},wallBudgetSeconds=cfg.budgets.child_seconds){
 const env={...process.env,...extraEnv,CARGO_BUILD_JOBS:String(cfg.budgets.build_jobs),RAYON_NUM_THREADS:String(cfg.budgets.rayon_threads)};
 for(const k of Object.keys(env))if(k.startsWith('CARGO_PROFILE_RELEASE_'))delete env[k];
 const base=path.join(run,'logs',label);fs.mkdirSync(path.dirname(absolute(base)),{recursive:true});
 const out=fs.openSync(absolute(base+'.stdout.log'),'wx'),err=fs.openSync(absolute(base+'.stderr.log'),'wx');
 const start=performance.now(),started=new Date().toISOString();let reason=null;
 const child=spawn(program,args,{cwd:root,env,stdio:['ignore','pipe','pipe'],detached:true});
 child.stdout.on('data',b=>{fs.writeSync(out,b);process.stdout.write(b);});child.stderr.on('data',b=>{fs.writeSync(err,b);process.stderr.write(b);});
 const timer=setTimeout(()=>{reason='WALL_BUDGET';try{process.kill(-child.pid,'SIGTERM');}catch{}},wallBudgetSeconds * 1000);
 const result=await new Promise(resolve=>{child.once('error',e=>resolve({code:null,error:String(e)}));child.once('close',(code,signal)=>resolve({code,signal}));});
 clearTimeout(timer);fs.closeSync(out);fs.closeSync(err);
 const r={program,args,...result,reason,started,wall_ms:performance.now()-start,wall_budget_seconds: wallBudgetSeconds,
  environment:{CARGO_TARGET_DIR:env.CARGO_TARGET_DIR??null,CARGO_BUILD_JOBS:env.CARGO_BUILD_JOBS,RAYON_NUM_THREADS:env.RAYON_NUM_THREADS},
  stdout:base+'.stdout.log',stderr:base+'.stderr.log'};writeNew(base+'.command.json',r);
 assert.equal(result.code,0,label+' failed; logs preserved');assert.equal(reason,null);return r;
}
export function currentSourcePins(){
 const pinned=[...new Set([...sourceFiles,...approvedPatches])];
 return pinned.map(p=>({path:p,sha256:sha(read(p))}));
}
export function binaryReproducibility(run){
 const binary=runPath(run)+'/build-target/release/stage1_multibatch_materials',exists=fs.existsSync(absolute(binary));
 const historical=load('experiments/stage1_multibatch_raw/gate-a-20261117-03/gate_a.json').binary_sha256;
 assert.equal(historical,EXPECTED_BINARY_SHA256,'immutable A-03 binary baseline');
 const actual=exists?sha(read(binary)):null;
 return {status:exists&&actual===historical?'PASS_COMPILE':'BLOCKED',binary,exists,binary_sha256:actual,
  historical_a03_binary_sha256:historical,hashes_match:actual===historical,automatic_baseline_change:false};
}
export function verifyA(run){runPath(run);const a=load(run+'/gate_a.json');assert.equal(a.status,'PASS_GATE_A_COMPILE_ONLY');
 assert.equal(a.run,run);assert.equal(a.config_sha256,sha(read(configPath)));assert.deepEqual(a.source_pins,currentSourcePins());
 assert.equal(a.binary,run+'/build-target/release/stage1_multibatch_materials');
 assert.equal(a.compiled_sha256,sha(read(run+'/compiled.json')));assert.equal(a.binary_sha256,sha(read(a.binary)));
 const reproducibility=binaryReproducibility(run);assert.equal(reproducibility.status,'PASS_COMPILE');
 assert.deepEqual(load(run+'/binary_reproducibility.json'),reproducibility);
 assert.equal(a.binary_reproducibility_sha256,sha(read(run+'/binary_reproducibility.json')));
 assert.ok(a.commands.every(c=>c.code===0&&c.reason===null));checkStorageLayouts(load(run+'/compiled.json').contracts);return a;}
export async function gateA(run){
 runPath(run);assert.ok(!fs.existsSync(absolute(run)),'never overwrite a run');const version=validateConfig({phase:'A',run});
 const before=protectedSnapshot({phase:'A',run});writeNew(run+'/protected_files.json',before);
 const commands=[];const base={gate:'A',scope:'COMPILE_ONLY',run,base_commit:git('rev-parse','HEAD'),branch:git('branch','--show-current'),
  config_sha256:sha(read(configPath)),source_pins:currentSourcePins(),
  environment:{cpu:os.cpus()[0].model,memory_bytes:os.totalmem(),platform:os.platform(),arch:os.arch(),node:process.version},
  setup_count:0,proof_generation_count:0,evm_transaction_count:0,tests:'NOT_RUN_GATE_B_REQUIRES_CONFIRMATION',automatic_commit:false,automatic_push:false};
 try{
  commands.push(await command(run,'rust-version','rustc',['+1.99.1','-Vv']));
  commands.push(await command(run,'cargo-build','cargo',['+1.99.1','build','--release','--locked','--offline','-p','vv-friendly-dkg','--bin','stage1_multibatch_materials'],{CARGO_TARGET_DIR:absolute(run+'/build-target')}));
  const reproducibility=binaryReproducibility(run);writeNew(run+'/binary_reproducibility.json',reproducibility);
  assert.equal(reproducibility.status,'PASS_COMPILE','binary differs from A-03; STOP, never alter historical baseline');
  commands.push(await command(run,'rust-format','rustfmt',['+1.99.1','--edition','2021','--check',sourceFiles[1]]));
  for(const p of [scriptPath,'scripts/stage1_multibatch_audit.mjs'])commands.push(await command(run,'syntax-'+path.basename(p),'node',['--check',p]));
  console.log('Gate A: compiling the three isolated receivers; no VM is instantiated');
  const compiled=compileSolidity();writeNew(run+'/compiled.json',compiled);
  writeNew(run+'/storage_layout.json',{layouts:Object.fromEntries(Object.entries(compiled.contracts['stage1/Stage1Receivers.sol']).map(([n,c])=>[n,c.storageLayout])),checks:compiled.storage_checks});
  writeNew(run+'/static_checks.json',{version,prague:pragueStatic(),storage_checks:compiled.storage_checks,provenance:inputProvenance(),
   phase_call_sites:phaseCallSites(),vm_metadata_fields:vmMetadataFields,binary_reproducibility:reproducibility,
   admission_policy:{status:'PASS_STATIC',preflight_hash_fields:['a_run','b_run','config_sha256','compiled_sha256','binary_sha256','source_pins'],
    c_requires_execution_and_audit:true,c_readonly_inputs:['aRun','resultsPathForRun(aRun)','bRun','resultsPathForRun(bRun)'],
    rss_fields:['setup.worker_peak_rss_mib','batches[].peak_rss_mib'],child_rss_mib:cfg.budgets.child_rss_mib,runtime_status:'RUNTIME_PENDING_B_C'},
   runtime_checks:'NOT_EXECUTED_PENDING_B_C'});checkProtected(before);
  assert.deepEqual(currentSourcePins(),base.source_pins,'approved source patches must stay frozen during Gate A');
  writeNew(run+'/gate_a.json',{...base,status:'PASS_GATE_A_COMPILE_ONLY',commands,
   vm_creation_count:0,version,static_checks_sha256:sha(read(run+'/static_checks.json')),storage_layout_sha256:sha(read(run+'/storage_layout.json')),
   binary_reproducibility_sha256:sha(read(run+'/binary_reproducibility.json')),
   compiled_sha256:sha(read(run+'/compiled.json')),binary:run+'/build-target/release/stage1_multibatch_materials',
   binary_sha256:sha(read(run+'/build-target/release/stage1_multibatch_materials')),historical_files_unchanged:true,
   next_gate:'B_REQUIRES_NEW_HUMAN_CONFIRMATION'});
 }catch(e){writeNew(run+'/gate_a_failed.json',{...base,status:'BLOCKED',error:String(e),commands});throw e;}
 console.log('PASS Gate A; STOP. B/C/D were not executed.');
}
export const vmMetadataFields=['route','chain_id','deployer_address','deployer_nonce_initial','gate_deployment_nonce','receiver_deployment_nonce',
 'deployer_nonce_final','gate_address','receiver_address','gate_initcode_sha256','receiver_initcode_sha256','receiver_runtime_code_sha256','final_context'];
export function inputProvenance(){
 const poolPath='fixtures/snarkpack_g1d2_depth5_batch32_public_v1.json',contextPath='fixtures/jubjub_g6_native_winner_only_public_v1.json';
 const p=load(poolPath),context=load(contextPath).public_context.final_context_hex;
 assert.equal(p.source_proof_count,32);assert.equal(p.reused_source_proof_count,16);assert.equal(p.new_source_proof_count,16);
 assert.equal(p.private_witness_emitted,false);assert.equal(p.proof_origins.filter(x=>x==='reused-G1-D1-pool').length,16);
 assert.equal(p.proof_origins.filter(x=>x==='new-G1-D2').length,16);assert.ok(p.source_public_inputs_le_hex.every(row=>row.length===25));
 assert.ok(p.source_public_records.every(row=>row.final_context_hex===context));
 return {status:'PASS_STATIC',pool:poolPath,pool_sha256:sha(read(poolPath)),source_proof_count:32,reused_from_g1d1:16,new_in_g1d2:16,
  private_witness_emitted:false,context_fixture:contextPath,final_context:hex(context),state_input_count:37,
  instance_boundary:'context digest bound; chain ID and Receiver address are NOT separate state circuit public inputs',
  ballot_prove_timing:'NOT_MEASURED; reuse archived proofs only',weight_scope:'fixed 32 voters of weight 1; total 32; per-voter upper bound 32, not arbitrary nonuniform coverage'};
}
export function phaseCallSites(){
 const source=read(scriptPath).toString(),audit=read('scripts/stage1_multibatch_audit.mjs').toString();
 // Check actual entry-point bodies, not this function's own list of expected strings.
 const body=(s,name)=>{const start=s.indexOf('function '+name+'(');assert.ok(start>=0,name);const end=s.indexOf('\nexport ',start+1);return s.slice(start,end<0?s.length:end).replace(/\s+/g,'');};
 const patterns=[['gateA',"validateConfig({phase:'A',run})"],['nextGate',"validateConfig({phase:'B',run,aRun})"],
  ['nextGate',"validateConfig({phase:'C',run,aRun,bRun})"],['vmWorker','validateConfig({phase:request.gate,run,aRun:request.a_run,bRun:request.b_run})'],
  ['preflightGateB',"validateConfig({phase:'A',run:aRun})"]];
 for(const [name,p] of patterns)assert.ok(body(source,name).includes(p),'phase call site '+name);
 const main=body(audit,'main');for(const p of ['validateConfig({phase,run,aRun,rawRun})','sourcePatchSnapshot()',
  'checkReadonlyInputs(inputsBefore)','sourceBefore','sourcePatchSnapshot(),sourceBefore'])assert.ok(main.includes(p),'audit source freeze/read-only wiring '+p);
 const next=body(source,'nextGate');for(const p of ['verifyPreflightB(aRun,path.basename(run))','verifyGateB(bRun)',
  'protectedSnapshot({phase:gate,run,aRun,bRun})','checkReadonlyInputs(before.readonly_inputs)'])assert.ok(next.includes(p),'admission wiring '+p);
 return {status:'PASS_STATIC',runner:patterns.map(([name,expression])=>({name,expression})),
  auditor:'A/B/D explicit; B A-inputs read-only, D raw/source hash freeze; C inputs protected by execution snapshots and audit checks',
  runtime_status:'RUNTIME_PENDING_B_C_D'};
}
export function verifyPreflightB(aRun,bRun){
 assert.match(bRun??'',/^[A-Za-z0-9_-]+$/);const a=verifyA(aRun),p=load(aRun+'/preflight_b.json');
 assert.equal(sha(read(aRun+'/preflight_b.json')),sha(read(resultsPathForRun(aRun)+'/preflight_b.json')),'raw/results preflight bytes');
 assert.equal(p.status,'READY_FOR_HUMAN_APPROVAL');assert.equal(p.scope,'STATIC_PREFLIGHT_ONLY');
 assert.equal(p.a_run,aRun);assert.equal(p.b_run,bRun);assert.equal(p.config_sha256,sha(read(configPath)));
 assert.equal(p.compiled_sha256,a.compiled_sha256);assert.equal(p.binary_sha256,a.binary_sha256);
 assert.deepEqual(p.source_pins,a.source_pins);assert.deepEqual(p.source_pins,currentSourcePins());
 assert.deepEqual(p.resource_budgets,cfg.budgets);assert.equal(p.positive_tasks.length,6);assert.equal(p.negative_tasks.length,3);
 for(const f of ['setup_count','proof_generation_count','vm_creation_count','evm_transaction_count'])assert.equal(p[f],0);
 return p;
}
export function verifyGateB(bRun){
 runPath(bRun);const bExecution=load(bRun+'/gate_b.json'),bAudit=load(resultsPathForRun(bRun)+'/gate_b_audit.json');
 assert.equal(bExecution.status,'PASS_GATE_B_REGRESSION_ONLY');assert.equal(bAudit.status,'PASS_GATE_B_EVIDENCE_AUDIT_ONLY');
 for(const r of [bExecution,bAudit]){assert.equal(r.config_sha256,sha(read(configPath)));assert.deepEqual(r.source_pins,currentSourcePins());}
 assert.equal(bExecution.positive_vm_count,6);assert.equal(bExecution.negative_vm_count,3);
 assert.equal(bAudit.positive_vm_count,6);assert.equal(bAudit.negative_vm_sessions,3);assert.equal(bAudit.independent_vm_count,9);assert.equal(bAudit.gas_receipt_audit,'PASS');
 return {execution:bExecution,audit:bAudit};
}
export async function preflightGateB(aRun,bRun){
 runPath(aRun);assert.match(bRun??'',/^[A-Za-z0-9_-]+$/,'B run must be a basename, not a path');
 validateConfig({phase:'A',run:aRun});const a=verifyA(aRun),before=protectedSnapshot({phase:'A',run:aRun});
 const raw='experiments/stage1_multibatch_raw/'+bRun,result=resultsPathForRun(raw);
 for(const p of [raw,result])assert.equal(fs.existsSync(absolute(p)),false,'future B directory must not exist');
 assert.equal(cfg.rust_toolchain,'1.99.1');assert.equal(cfg.release_opt_level,1);
 assert.deepEqual(cfg.routes,['legacy','direct_fold','state_proof']);assert.deepEqual(cfg.regression_k,[1,2]);
 assert.deepEqual(cfg.budgets,{build_jobs:2,rayon_threads:4,child_seconds:600,child_rss_mib:16384,node_heap_mib:6144});
 assert.equal(cfg.transaction_gas_limit,20000000);assert.equal(cfg.block_gas_limit,30000000);
 for(const g of ['A','B','C','D'])assert.ok(cfg.gates[g]);assert.match(cfg.proof_policy,/No archived state proof reuse/);
 const commands=[];commands.push(await command(aRun,'preflight-rust-version','rustc',['+1.99.1','-Vv']));
 commands.push(await command(aRun,'preflight-cargo-version','cargo',['+1.99.1','-V']));
 const positive=cfg.regression_k.flatMap(k=>cfg.routes.map(route=>({k,route,kind:'DEVELOPMENT_REGRESSION_NOT_PERFORMANCE'})));
 const negative=cfg.routes.map(route=>({k:2,route,kind:'NEGATIVE_ONLY'}));assert.equal(positive.length,6);assert.equal(negative.length,3);
 checkProtected(before);for(const p of [raw,result])assert.equal(fs.existsSync(absolute(p)),false);
 const report={status:'READY_FOR_HUMAN_APPROVAL',scope:'STATIC_PREFLIGHT_ONLY',a_run:aRun,b_run:bRun,
  config_sha256:sha(read(configPath)),compiled_sha256:a.compiled_sha256,binary_sha256:a.binary_sha256,source_pins:a.source_pins,
  positive_tasks:positive,negative_tasks:negative,resource_budgets:cfg.budgets,stopping_rules:cfg.gates,prague:pragueStatic(),commands,
  future_directories:[raw,result].map(path=>({path,exists:false})),setup_count:0,proof_generation_count:0,vm_creation_count:0,evm_transaction_count:0,
  runtime_status:'RUNTIME_PENDING_B',workspace_handoff:'B reads aRun raw/results without mutation; C reads both aRun and bRun raw/results without mutation, including ignored build caches.',
  next_action:'STOP; separate human approval required before B'};
 writeNew(aRun+'/preflight_b.json',report);writeNew(resultsPathForRun(aRun)+'/preflight_b.json',report);
 verifyPreflightB(aRun,bRun);console.log(json(report));return report;
}
function bindingData(){
 const a=load('fixtures/g8a_original_certificates_public_v1.json'),g=load('fixtures/jubjub_g6_native_winner_only_public_v1.json'),c=g.public_context;
 const binding={electionId:hex(c.election_id_hex),policyPre:hex(c.policy_pre_hex),dkgSession:hex(c.dkg_session_hex),keyCoreDigest:hex(c.key_core_digest_hex),publicKey:hex(c.public_key_hex),
  finalContext:hex(c.final_context_hex),registrationRoot:hex(c.registration_root_hex),registrationCloseDigest:hex(c.registration_close_digest_hex),disclosureDigest:hex(c.disclosure_digest_hex),
  p3VkSha256:hex(load('fixtures/g8_receiver_eip2537_v1.json').p3_vk_ark_sha256),prekeyContext:hex(c.prekey_context_hex),registrationAuthority:hex(c.registration_authority_hex),
  registrationCertificateSha256:hex(sha(wire(c.registration_certificate_hex))),region:4,registeredWeight:32};
 const expected={electionId:binding.electionId,manifestDigest:hex(a.manifest_digest_hex),policyPre:binding.policyPre,prekeyContext:binding.prekeyContext,dkgSession:binding.dkgSession,
  keyCoreDigest:binding.keyCoreDigest,finalContext:binding.finalContext,publicKey:binding.publicKey,registrationRoot:binding.registrationRoot,closeDigest:binding.registrationCloseDigest,disclosureDigest:binding.disclosureDigest};
 const close={electionId:hex(a.close.election_id_hex),finalContext:hex(a.close.final_context_hex),policyPre:hex(a.close.policy_pre_hex),root:hex(a.close.root_hex),
  nRegistered:a.close.n_registered,wRegistered:a.close.w_registered,wMax:a.close.w_max,version:a.close.version,canonicalRegistryDigest:hex(a.close.canonical_registry_digest_hex)};
 return {a,binding,expected,close};
}
const text=s=>{const b=Buffer.from(s),n=Buffer.alloc(8);n.writeBigUInt64BE(BigInt(b.length));return Buffer.concat([n,b]);};
const u64=n=>{const b=Buffer.alloc(8);b.writeBigUInt64BE(BigInt(n));return b;};
const prefix=s=>Buffer.concat([text('VV-CANONICAL-V1'),text(s)]);
const hash=b=>Buffer.from(sha(b),'hex');
function membership(material,index){
 const batch=Math.floor(index/4),position=index%4,{binding:b}=bindingData();
 const leaves=material.ballots.slice(batch*4,batch*4+4).map((v,i)=>hash(Buffer.concat([prefix('VV/BATCH/LEAF/v2'),wire(b.electionId),wire(b.finalContext),u64(4),u64(batch+1),u64(4),u64(i),u64(25),...v.eip.public_inputs_be_hex.map(wire)])));
 const node=(a,b)=>hash(Buffer.concat([prefix('VV/BATCH/NODE/v2'),a,b]));
 assert.equal('0x'+node(node(leaves[0],leaves[1]),node(leaves[2],leaves[3])).toString('hex'),cfg.ordered_batch_roots[batch]);
 return [leaves[position^1],position<2?node(leaves[2],leaves[3]):node(leaves[0],leaves[1])].map(b=>'0x'+b.toString('hex'));
}
function batchAt(m,index){const b=m.batches[index];return {index:index+1,root:b.root,oldState:b.old_state,newState:b.new_state,
 ballotProofs:m.ballots.slice(index*4,index*4+4).map(x=>x.eip.proof_neg_a_eip2537_hex),ballotInputs:m.ballots.slice(index*4,index*4+4).map(x=>x.eip.public_inputs_be_hex)};}
export async function vmWorker(run,aRun,route,k,negative){
 const request=load(run+'/request.json');assert.ok(['B','C'].includes(request.gate));
 validateConfig({phase:request.gate,run,aRun:request.a_run,bRun:request.b_run});assert.equal(aRun,request.a_run);
 const inputsBefore=readonlyInputSnapshot({phase:request.gate,aRun:request.a_run,bRun:request.b_run});
 verifyA(aRun);assert.ok(cfg.routes.includes(route));assert.ok(k>=1&&k<=8);
 const m=load(run+'/materials/materials.json');assert.equal(m.phase,request.gate);
 assert.equal(m.config_sha256,sha(read(configPath)));assert.equal(m.state_proofs_generated,request.gate==='B'?2:8);
 assert.equal(m.archived_state_proofs_reused,0);assert.equal(m.all_state_proofs_fresh,true);
 if(negative)assert.equal(request.gate,'B','no negatives inside formal performance workers');
 const compiled=load(aRun+'/compiled.json');for(const p of compiled.sources)assert.equal(sha(read(p.path)),p.sha256);
 const get=(f,n)=>compiled.contracts[f][n],names={legacy:'Stage1LegacySettlement',direct_fold:'Stage1DirectFoldSettlement',state_proof:'Stage1StateProofSettlement'};
 const code=get('stage1/Stage1Receivers.sol',names[route]),gateCode=get('auth/VeriVotingCertificateGateV2.sol','VeriVotingCertificateGateV2');
 const ri=new Interface(code.abi),gi=new Interface(gateCode.abi),{a,binding,expected,close}=bindingData();
 const point=p=>({x:p.x,y:p.y,encoded:hex(p.encoded_hex)}),sig=s=>({nonce:point(s.nonce),response:s.response_scalar,counter:s.counter});
 const common=pragueCommon();
 // Every call creates a NEW VM and fresh sender state. No prior K is continued.
 const vm=await createVM({common}),key=hexToBytes('0x'+'3c'.repeat(32)),sender=createAddressFromPrivateKey(key);
 await vm.stateManager.putAccount(sender,createAccount({nonce:0n,balance:10n**25n}));
 const initialRoot=bytesToHex(await vm.stateManager.getStateRoot());
 const deployerNonceInitial=(await vm.stateManager.getAccount(sender)).nonce;
 let nonce=deployerNonceInitial,blockNumber=1n,gate,receiver,calls=[],stack=[];const txs=[],negatives=[],recovery=[],checkpoints=[],deployments={};
 vm.evm.events.on('beforeMessage',e=>stack.push({address:e.to?.toString()??null}));
 vm.evm.events.on('afterMessage',e=>{const c=stack.pop();if(c)calls.push({...c,gas:e.execResult.executionGasUsed.toString(),success:!e.execResult.exceptionError,return_hex:bytesToHex(e.execResult.returnValue)});});
 function reason(r){const h=bytesToHex(r.execResult.returnValue);if(h.startsWith('0x08c379a0'))return abi.decode(['string'],'0x'+h.slice(10))[0];return null;}
 async function tx(label,category,data,to){
  calls=[];const signed=createLegacyTx({nonce:nonce++,gasLimit:BigInt(cfg.transaction_gas_limit),gasPrice:10n,value:0n,data:hexToBytes(data),...(to?{to}:{})},{common}).sign(key);
  const block=createBlock({header:{number:blockNumber,gasLimit:BigInt(cfg.block_gas_limit),baseFeePerGas:1n,timestamp:1750000000n+blockNumber}},
   {common,skipConsensusFormatValidation:true});blockNumber+=1n;
  const t=performance.now(),r=await runTx(vm,{tx:signed,block,blockGasUsed:0n});assert.equal(stack.length,0);
  const intrinsic=BigInt(signed.getIntrinsicGas()),execution=BigInt(r.execResult.executionGasUsed),pre=intrinsic+execution;
  // Current transactions are Legacy: no EIP-7702 authorization refund component.
  const rawRefund=BigInt(r.execResult.gasRefund??0n),cap=pre/5n,applied=rawRefund<cap?rawRefund:cap;
  const after=pre-applied,floor=BigInt(getCalldataFloorGas(signed,sender)),used=BigInt(r.totalGasSpent);
  const adjustment=floor>after?floor-after:0n,effective=pre>used?pre-used:0n;
  assert.ok(used<=BigInt(cfg.block_gas_limit));assert.equal(used,after>floor?after:floor);
  assert.equal(effective,applied>adjustment?applied-adjustment:0n);assert.equal(BigInt(r.receipt.cumulativeBlockGasUsed),0n+used);
  const entry={label,category,hash:bytesToHex(signed.hash()),nonce:signed.nonce.toString(),signed_transaction_hex:bytesToHex(signed.serialize()),
   to:to?.toString()??null,status:r.receipt.status,
   block_number:block.header.number.toString(),transaction_index_in_block:'0',block_gas_used_before:'0',
   gas_pre_refund:pre.toString(),refund_counter_raw:rawRefund.toString(),refund_cap:cap.toString(),refund_applied_before_floor:applied.toString(),
   gas_after_refund_before_floor:after.toString(),minimum_gas_limit:signed.getMinimumGasLimit().toString(),eip7623_floor_gas:floor.toString(),
   floor_adjustment:adjustment.toString(),refund_effective_in_receipt:effective.toString(),gas_used:used.toString(),
   receipt_cumulative_gas:r.receipt.cumulativeBlockGasUsed.toString(),intrinsic_gas:intrinsic.toString(),execution_gas:execution.toString(),
   refund_counter_scope:'execResult.gasRefund; current Legacy transaction path, not adjusted results.gasRefund',
   calldata_hex:bytesToHex(signed.data),calldata_bytes:signed.data.length,calldata_sha256:sha(signed.data),
   exception:r.execResult.exceptionError?.error??null,revert_reason:r.execResult.exceptionError?reason(r):null,
   calls:[...calls],logs:r.receipt.logs.map(([a,t,d])=>({address:bytesToHex(a),topics:t.map(bytesToHex),data:bytesToHex(d)})),wall_ms:performance.now()-t};
  txs.push(entry);console.log(`${route} K${k} ${label}: ${entry.status?'PASS':entry.revert_reason} ${entry.gas_used}`);return {r,entry};
 }
 async function deploy(label,c,args){const data=(await new ContractFactory(c.abi,'0x'+c.evm.bytecode.object).getDeployTransaction(...args)).data;
  const {r,entry}=await tx(label,'deployment',data);assert.equal(entry.status,1);assert.ok(r.createdAddress);
  deployments[label]={nonce:entry.nonce,initcode_sha256:sha(wire(data)),runtime_code_sha256:sha(await vm.stateManager.getCode(r.createdAddress))};return r.createdAddress;}
 async function view(addr,iface,name,args=[]){await vm.evm.journal.checkpoint();let r;try{r=await vm.evm.runCall({caller:sender,to:addr,data:hexToBytes(iface.encodeFunctionData(name,args)),gasLimit:20000000n,isStatic:true});}finally{await vm.evm.journal.revert();}
  assert.equal(r.execResult.exceptionError,undefined,'view '+name);return iface.decodeFunctionResult(name,bytesToHex(r.execResult.returnValue));}
 async function snapshot(){
  const r=await view(receiver,ri,'checkpoint'),g=await view(gate,gi,'checkpoint');const spent=[],signers=[];
  for(const b of m.ballots)spent.push((await view(receiver,ri,'spentNullifier',[b.nullifier]))[0]);
  for(let id=1;id<=5;id++)signers.push((await view(gate,gi,'certifiedSigner',[id]))[0]);
  const weights={};for(const name of ['certifiedWRegistered','certifiedNRegistered','certifiedWMax','certifiedCertificateSha256'])weights[name]=(await view(gate,gi,name))[0];
  return normalized({next:Number(r[0]),count:Number(r[1]),position:Number(r[2]),frozen:r[3],target:r[4],state:[...r[5]],spent,
   gate:[...g],signers,weights,bindingDigest:(await view(receiver,ri,'bindingDigest'))[0],scheduleDigest:(await view(receiver,ri,'scheduleDigest'))[0],plannedBatches:k});
 }
 function data(b){return route==='state_proof'?ri.encodeFunctionData('submitBatch',[b,m.batches[b.index-1].eip.proof_neg_a_eip2537_hex]):ri.encodeFunctionData('submitBatch',[b]);}
 function single(i,old,proof=m.ballots[i].eip.proof_neg_a_eip2537_hex,row=m.ballots[i].eip.public_inputs_be_hex,idx=Math.floor(i/4)+1,pos=i%4){return ri.encodeFunctionData('submitBallot',[idx,pos,old,row,proof,membership(m,i)]);}
 async function reject(label,encoded,expectedReason){const before=await snapshot(),{entry}=await tx(label,'rejected',encoded,receiver),after=await snapshot();
  assert.equal(entry.status,0);assert.equal(entry.exception,'revert');assert.equal(entry.revert_reason,expectedReason);assert.deepEqual(after,before);assert.equal(entry.logs.length,0);
  negatives.push({case:label,reason:expectedReason,before,after,transaction_hash:entry.hash});return entry;}
 async function accept(index){if(route==='legacy'){
   for(let i=index*4;i<index*4+4;i++){const {entry}=await tx('accept_vote_'+(i+1),'intake',single(i,m.ballots[i].old_state),receiver);assert.equal(entry.status,1);
    const s=await snapshot();assert.deepEqual(s.state,m.ballots[i].new_state);assert.equal(s.count,i+1);assert.equal(s.position,(i+1)%4);}
  }else{const {entry}=await tx('accept_batch_'+(index+1),'intake',data(batchAt(m,index)),receiver);assert.equal(entry.status,1);}
  const s=await snapshot();assert.deepEqual(s.state,m.batches[index].new_state);assert.equal(s.next,index+2);assert.equal(s.count,4*(index+1));assert.equal(s.position,0);checkpoints.push(s);}
 const started=new Date().toISOString();let record;
 try{
  gate=await deploy('deploy_gate',gateCode,[expected,a.authentication_keys.map(point),point(a.registration_authority),a.public_coefficients.map(point),hex(a.transcript_digest_hex)]);
  const args=[gate.toString(),binding,cfg.ordered_batch_roots.slice(0,k),m.ballots[0].eip.vk_eip2537_hex];
  if(route==='state_proof')args.push(m.batches[0].eip.vk_eip2537_hex);receiver=await deploy('deploy_receiver',code,args);
  assert.equal((await view(receiver,ri,'certificateGate'))[0].toLowerCase(),gate.toString());
  assert.equal((await view(receiver,ri,'bindingDigest'))[0],cfg.binding_digest);
  if(negative)await reject('uncertified',route==='legacy'?single(0,m.ballots[0].old_state):data(batchAt(m,0)),'UNCERTIFIED');
  for(let i=0;i<3;i++){const {entry}=await tx('authenticate_dkg_'+(i+1),'certification',gi.encodeFunctionData('submitDkgSignature',[i+1,binding.electionId,binding.finalContext,sig(a.dkg_certificate_signatures[i])]),gate);assert.equal(entry.status,1);}
  {const {entry}=await tx('authenticate_registration','certification',gi.encodeFunctionData('authenticateRegistration',[close,sig(a.registration_signature)]),gate);assert.equal(entry.status,1);}
  const certified=await snapshot();assert.equal(certified.gate[6],true);
  if(negative){
   if(route==='legacy'){
    await reject('illegal_batch_index',single(0,m.ballots[0].old_state,undefined,undefined,2),'NEXT_INDEX');
    await reject('replacement_ballot_proof',single(0,m.ballots[0].old_state,m.ballots[1].eip.proof_neg_a_eip2537_hex),'INVALID_GROTH16');
    // Keep V1's position and membership path, replace the complete ballot by distinct, valid V5.
    await reject('complete_valid_ballot_replacement',single(0,m.ballots[0].old_state,
     m.ballots[4].eip.proof_neg_a_eip2537_hex,m.ballots[4].eip.public_inputs_be_hex),'BATCH_MEMBERSHIP');
    const firstTwo=[];
    for(let i=0;i<2;i++){const {entry}=await tx(i===0?'recovery_alice':'recovery_bob','recovery',single(i,m.ballots[i].old_state),receiver);assert.equal(entry.status,1);const s=await snapshot();assert.equal(s.count,i+1);assert.equal(s.next,1);assert.deepEqual(s.state,m.ballots[i].new_state);firstTwo.push(entry.hash);}
    recovery.push({step:1,action:'Alice then Bob accepted',transaction_hashes:firstTwo,checkpoint:await snapshot()});
    await reject('legacy_incomplete_freeze',ri.encodeFunctionData('freeze'),'INCOMPLETE');
    await reject('within_batch_duplicate_nf',single(1,m.ballots[2].old_state,undefined,m.ballots[0].eip.public_inputs_be_hex,1,2),'SPENT_NULLIFIER');
    const bad=await reject('charlie_first_attempt',single(2,m.ballots[2].old_state,m.ballots[0].eip.proof_neg_a_eip2537_hex),'INVALID_GROTH16');
    recovery.push({step:2,action:'Charlie rejected with full rollback',transaction_hash:bad.hash,checkpoint:await snapshot()});
    for(let i=2;i<4;i++){const {entry}=await tx(i===2?'recovery_charlie_retry':'recovery_david','recovery',single(i,m.ballots[i].old_state),receiver);assert.equal(entry.status,1);const s=await snapshot();assert.equal(s.count,i+1);assert.deepEqual(s.state,m.ballots[i].new_state);recovery.push({step:i+1,action:i===2?'Charlie retry accepted':'David accepted',transaction_hash:entry.hash,checkpoint:s});
     if(i===3){assert.equal(s.position,0);assert.equal(s.next,2);assert.ok(entry.logs.some(l=>{try{return ri.parseLog(l)?.name==='BatchAccepted';}catch{return false;}}));recovery.push({step:5,action:'Batch complete, emitted by the same David transaction',transaction_hash:entry.hash,checkpoint:s});}}
    await reject('stale_old_state',single(4,m.ballots[0].old_state),'OLD_STATE');
    await reject('cross_batch_duplicate_nf',single(4,m.ballots[4].old_state,undefined,m.ballots[0].eip.public_inputs_be_hex),'SPENT_NULLIFIER');
    checkpoints.push(await snapshot());await accept(1);
   }else{
    let b=batchAt(m,0);b.index=2;await reject('illegal_batch_index',data(b),'NEXT_INDEX');
    b=batchAt(m,0);b.ballotProofs[0]=b.ballotProofs[1];await reject('replacement_ballot_proof',data(b),'INVALID_GROTH16');
    b=batchAt(m,0);b.ballotProofs[0]=m.ballots[4].eip.proof_neg_a_eip2537_hex;b.ballotInputs[0]=m.ballots[4].eip.public_inputs_be_hex;
    await reject('complete_valid_ballot_replacement',data(b),'BATCH_ROOT');
    b=batchAt(m,0);b.ballotInputs[1]=b.ballotInputs[0];await reject('within_batch_duplicate_nf',data(b),'DUPLICATE_NULLIFIER');
    b=batchAt(m,0);b.newState=m.batches[0].omission_a_new_state;
    const rejected=await reject('omission_a',data(b),route==='state_proof'?'INVALID_GROTH16':'DIRECT_FOLD_MISMATCH');
    if(route==='state_proof'){const ps=rejected.calls.filter(c=>c.address?.endsWith('000f'));assert.equal(ps.length,5);assert.ok(ps.slice(0,4).every(c=>BigInt(c.return_hex)===1n));assert.equal(BigInt(ps[4].return_hex),0n);}
    await accept(0);b=batchAt(m,1);b.oldState=m.batches[0].old_state;await reject('stale_old_state',data(b),'OLD_STATE');
    b=batchAt(m,1);b.ballotInputs[0]=m.ballots[0].eip.public_inputs_be_hex;await reject('cross_batch_duplicate_nf',data(b),'SPENT_NULLIFIER');await accept(1);
   }
  }else for(let i=0;i<k;i++)await accept(i);
  {const {entry}=await tx('freeze','freeze',ri.encodeFunctionData('freeze'),receiver);assert.equal(entry.status,1);}
  const final=await snapshot();assert.equal(final.frozen,true);assert.equal(final.count,k*4);assert.equal(final.next,k+1);assert.equal(final.position,0);
  assert.deepEqual(final.state,m.batches[k-1].new_state);assert.deepEqual(final.spent,m.ballots.map((_,i)=>i<k*4));
  assert.equal(final.target,'0x'+sha(wire(abi.encode(['string','bytes32','bytes32','uint64','uint64','bytes32[6]'],[cfg.freeze_version,cfg.binding_digest,final.scheduleDigest,k,k*4,final.state]))));
  if(negative)await reject('receive_after_freeze',route==='legacy'?single(0,final.state):data(batchAt(m,0)),'FROZEN');
  record={status:'PASS',final_checkpoint:final,certified_checkpoint:certified};
 }catch(e){record={status:'BLOCKED',error:String(e)};process.exitCode=1;}
 const output={...record,gate:request.gate,kind:negative?'NEGATIVE_ONLY':request.gate==='B'?'DEVELOPMENT_REGRESSION_NOT_PERFORMANCE':'FORMAL_POSITIVE_ONLY',
  k,route:route,started,finished:new Date().toISOString(),fresh_vm:true,initial_vm_root:initialRoot,vm_identity:`${request.identity}/${negative?'negative':'positive'}/${k}/${route}/${process.pid}`,
  hardfork:common.hardfork(),addresses:{gate:gate?.toString()??null,receiver:receiver?.toString()??null},transactions:txs,negatives,recovery,checkpoints,
  chain_id:common.chainId().toString(),deployer_address:sender.toString(),deployer_nonce_initial:deployerNonceInitial.toString(),
  gate_deployment_nonce:deployments.deploy_gate?.nonce??null,receiver_deployment_nonce:deployments.deploy_receiver?.nonce??null,
  deployer_nonce_final:(await vm.stateManager.getAccount(sender)).nonce.toString(),gate_address:gate?.toString()??null,receiver_address:receiver?.toString()??null,
  gate_initcode_sha256:deployments.deploy_gate?.initcode_sha256??null,receiver_initcode_sha256:deployments.deploy_receiver?.initcode_sha256??null,
  receiver_runtime_code_sha256:receiver?sha(await vm.stateManager.getCode(receiver)):null,final_context:binding.finalContext,
  config_sha256:sha(read(configPath)),materials_sha256:sha(read(run+'/materials/materials.json')),compiled_sha256:sha(read(aRun+'/compiled.json')),
  node_peak_rss_mib:metric(process.resourceUsage().maxRSS/1024),measurement_status:'MEASURED',snarkpack_evm_verified:false,public_chain_deployment:false,
  atomicity:route==='legacy'?'per ballot; four non-atomic transactions per completed batch':'one atomic transaction per batch'};
 checkReadonlyInputs(inputsBefore);
 writeNew(run+`/sessions/${negative?'negative':'positive'}-k${k}-${route}.json`,output);assert.equal(output.status,'PASS',output.error);
}
export async function nextGate(gate,run,aRun,bRun,approved){
 assert.equal(approved,gate,'human approval must explicitly name this gate');assert.ok(['B','C'].includes(gate));runPath(run);
 if(gate==='B')validateConfig({phase:'B',run,aRun});else validateConfig({phase:'C',run,aRun,bRun});
 const a=verifyA(aRun);
 if(gate==='B')verifyPreflightB(aRun,path.basename(run));
 if(gate==='C'){const b=verifyGateB(bRun);assert.equal(b.execution.a_run,aRun);assert.equal(b.execution.run,bRun);}
 for(const p of [run,resultsPathForRun(run)])assert.ok(!fs.existsSync(absolute(p)),'fresh independent B/C raw and result directories required');
 const before=protectedSnapshot({phase:gate,run,aRun,bRun});writeNew(run+'/protected_files.json',before);
 try{
  writeNew(run+'/request.json',{gate,identity:`${gate}/${path.basename(run)}/${new Date().toISOString()}`,config_sha256:sha(read(configPath)),a_run:aRun,b_run:bRun??null,approved,formal:gate==='C'});
  const proofCount=gate==='B'?2:8;
  const materialsWallBudget=(1 + proofCount) * cfg.budgets.child_seconds + 120;
  await command(run,'materials',absolute(a.binary),['prepare',run+'/materials',gate,'--approved',gate],{},materialsWallBudget);
  const ks=gate==='B'?cfg.regression_k:cfg.formal_k;
  for(const k of ks)for(const route of cfg.routes)await command(run,`positive-k${k}-${route}`,process.execPath,
   ['--max-old-space-size='+cfg.budgets.node_heap_mib,absolute(scriptPath),'--vm',run,aRun,route,String(k),'positive']);
  if(gate==='B')for(const route of cfg.routes)await command(run,'negative-'+route,process.execPath,
   ['--max-old-space-size='+cfg.budgets.node_heap_mib,absolute(scriptPath),'--vm',run,aRun,route,'2','negative']);
  for(const k of ks){const rows=cfg.routes.map(r=>load(run+`/sessions/positive-k${k}-${r}.json`));for(const r of rows.slice(1))assert.deepEqual(r.final_checkpoint,rows[0].final_checkpoint);}
  assert.deepEqual(currentSourcePins(),a.source_pins,'all approved patches including docs must still match Gate A');
  checkProtected(before);writeNew(run+`/gate_${gate.toLowerCase()}.json`,{gate,run,a_run:aRun,b_run:bRun??null,status:gate==='B'?'PASS_GATE_B_REGRESSION_ONLY':'PASS_GATE_C_FORMAL_EXECUTION',
   config_sha256:sha(read(configPath)),source_pins:currentSourcePins(),positive_vm_count:ks.length*3,
   negative_vm_count:gate==='B'?3:0,historical_files_unchanged:true,next_gate:gate==='B'?'C_REQUIRES_NEW_HUMAN_CONFIRMATION':'D_REQUIRES_NEW_HUMAN_CONFIRMATION'});
 }finally{checkReadonlyInputs(before.readonly_inputs);assert.deepEqual(currentSourcePins(),a.source_pins,'source pins changed during B/C');}
 console.log('PASS Gate '+gate+'; STOP for human confirmation.');
}
async function main(){
 process.chdir(root);const a=process.argv.slice(2);
 if(a[0]==='--vm'){assert.equal(a.length,6);return vmWorker(a[1],a[2],a[3],Number(a[4]),a[5]==='negative');}
 const flags=Object.fromEntries(Array.from({length:a.length/2},(_,i)=>[a[2*i],a[2*i+1]]));
 if(flags['--preflight']){assert.equal(a.length%2,0);assert.equal(flags['--preflight'],'B');return preflightGateB(flags['--a-run'],flags['--b-run']);}
 assert.ok(a.length%2===0&&flags['--gate']&&flags['--run'],'use --gate A|B|C --run experiments/stage1_multibatch_raw/NEW_ID');
 if(flags['--gate']==='A')return gateA(flags['--run']);
 return nextGate(flags['--gate'],flags['--run'],flags['--a-run'],flags['--b-run'],flags['--approved']);
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url))main().catch(e=>{console.error(e.stack??e);process.exitCode=1;});
