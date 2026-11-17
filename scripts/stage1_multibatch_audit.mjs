// A/B evidence checks and explicitly approved D derivation. Never executes cryptography or EVM.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import {createRequire} from 'node:module';
import {root,cfg,configPath,load,read,sha,wire,writeNew,checkProtected,validateConfig} from '../contracts/scripts/stage1_multibatch_compare.mjs';
const {Transaction,Interface}=createRequire(path.join(root,'contracts/package.json'))('ethers');
const measured=value=>({status:'MEASURED',value});
const derived=value=>({status:'DERIVED',value});
const missing={status:'NOT_MEASURED',value:null};
const artifactPaths=dir=>fs.readdirSync(path.join(root,dir),{withFileTypes:true}).flatMap(e=>e.isDirectory()?e.name==='build-target'?[]:artifactPaths(dir+'/'+e.name):[dir+'/'+e.name]);
function sources(pins){for(const p of pins)assert.equal(sha(read(p.path)),p.sha256,p.path);}
function checkA(run){
 const a=load(run+'/gate_a.json');assert.equal(a.status,'PASS_GATE_A_COMPILE_ONLY');sources(a.source_pins);
 assert.equal(a.config_sha256,sha(read(configPath)));assert.equal(a.compiled_sha256,sha(read(run+'/compiled.json')));
 assert.equal(a.binary_sha256,sha(read(a.binary)));assert.ok(a.commands.every(c=>c.code===0&&c.reason===null));
 const compiled=load(run+'/compiled.json');sources(compiled.sources);
 assert.equal(compiled.compiler.split('+')[0],'0.8.30');assert.deepEqual(compiled.settings.optimizer,{enabled:true,runs:200});assert.equal(compiled.settings.viaIR,true);
 assert.equal(compiled.settings.evmVersion,'prague');assert.equal(a.setup_count,0);assert.equal(a.proof_generation_count,0);assert.equal(a.evm_transaction_count,0);
 checkProtected(load(run+'/protected_files.json'));
 return {status:'PASS_GATE_A_ARTIFACT_AUDIT',gate:'A',config_sha256:a.config_sha256,
  compiled_contracts:compiled.sizes.filter(c=>c.name.startsWith('Stage1')),commands:a.commands.map(c=>({program:c.program,args:c.args,code:c.code,stdout:c.stdout,stderr:c.stderr})),
  rust_opt_level:1,cryptographic_function_tests:missing,evm_gas:missing,next_gate:'B_REQUIRES_NEW_HUMAN_CONFIRMATION',historical_files_unchanged:true};
}
function checkMaterials(run,gate){
 const m=load(run+'/materials/materials.json');assert.equal(m.phase,gate);assert.equal(m.state_proofs_generated,gate==='B'?2:8);
 assert.equal(m.archived_state_proofs_reused,0);assert.equal(m.all_state_proofs_fresh,true);assert.equal(m.serial_workers,true);
 assert.equal(m.config_sha256,sha(read(configPath)));assert.equal(m.request_sha256,sha(read(run+'/materials/request.json')));
 assert.equal(m.ballots.length,32);assert.equal(new Set(m.ballots.map(b=>b.nullifier)).size,32);
 assert.equal(m.setup.vk_sha256,cfg.state_vk_ark_sha256);assert.ok(m.setup.setup_ms>0);
 const old=load('fixtures/g8_receiver_eip2537_v1.json');
 for(let i=0;i<m.batches.length;i++){
  const b=m.batches[i];assert.equal(b.phase,gate);assert.equal(b.batch_index,i+1);assert.equal(b.root,cfg.ordered_batch_roots[i]);
  assert.equal(b.proof_origin,'NEW_THIS_RUN_SEPARATE_SERIAL_WORKER');
  assert.equal(b.proof_ark_bytes,wire(b.proof_ark_hex).length);assert.equal(b.proof_ark_sha256,sha(wire(b.proof_ark_hex)));
  assert.equal(b.proof_eip_bytes,wire(b.eip.proof_neg_a_eip2537_hex).length);assert.equal(sha(wire(b.eip.vk_eip2537_hex)),cfg.state_vk_eip_sha256);
  assert.ok(b.peak_rss_mib>0&&b.prove_ms>0&&b.witness_ms>0&&b.verify_ms>0);
  if(i)assert.deepEqual(b.old_state,m.batches[i-1].new_state);
  if(i<2){const flatten=v=>v.flatMap(x=>['0x'+x.c0_hex,'0x'+x.c1_hex]);assert.deepEqual(b.old_state,flatten(old.batches[i].old_state));assert.deepEqual(b.new_state,flatten(old.batches[i].new_state));}
 }
 const commandPids=m.commands.map(c=>c.pid);assert.equal(new Set(commandPids).size,commandPids.length);
 for(const c of m.commands){assert.equal(c.exit_code,0);assert.ok(c.wall_ms>0);}
 return m;
}
function transactionCheck(t){
 const signed=Transaction.from(t.signed_transaction_hex);assert.ok(signed.isSigned());assert.equal(signed.hash,t.hash);assert.equal(signed.data,t.calldata_hex);assert.equal(signed.nonce,t.nonce);
 assert.equal(wire(t.calldata_hex).length,t.calldata_bytes);assert.equal(sha(wire(t.calldata_hex)),t.calldata_sha256);assert.equal(t.refund,0);
 assert.equal(t.gas_used,Math.max(t.intrinsic_gas+t.execution_gas,t.calldata_floor));assert.equal(t.receipt_cumulative_gas,t.gas_used);assert.ok(t.gas_used<cfg.transaction_gas_limit);
}
function checkRun(run,gate){
 const top=load(run+`/gate_${gate.toLowerCase()}.json`);sources(top.source_pins);const m=checkMaterials(run,gate);
 const expectedStatus=gate==='B'?'PASS_GATE_B_REGRESSION_ONLY':'PASS_GATE_C_FORMAL_EXECUTION';assert.equal(top.status,expectedStatus);
 const ks=gate==='B'?cfg.regression_k:cfg.formal_k,rows=[];
 for(const k of ks){let final=null;
  for(const route of cfg.routes){
   const r=load(run+`/sessions/positive-k${k}-${route}.json`);assert.equal(r.status,'PASS');assert.equal(r.gate,gate);assert.equal(r.k,k);assert.equal(r.route,route);
   assert.equal(r.kind,gate==='B'?'DEVELOPMENT_REGRESSION_NOT_PERFORMANCE':'FORMAL_POSITIVE_ONLY');assert.equal(r.fresh_vm,true);
   assert.equal(r.materials_sha256,sha(read(run+'/materials/materials.json')));assert.equal(r.config_sha256,sha(read(configPath)));
   assert.equal(r.snarkpack_evm_verified,false);assert.equal(r.negatives.length,0);assert.ok(r.transactions.every(t=>t.status===1));r.transactions.forEach(transactionCheck);
   assert.equal(r.transactions.filter(t=>t.category==='intake').length,route==='legacy'?4*k:k);
   assert.equal(r.final_checkpoint.count,4*k);assert.equal(r.final_checkpoint.next,k+1);assert.equal(r.final_checkpoint.position,0);assert.equal(r.final_checkpoint.frozen,true);
   assert.deepEqual(r.final_checkpoint.state,m.batches[k-1].new_state);assert.deepEqual(r.final_checkpoint.spent,m.ballots.map((_,i)=>i<4*k));
   if(final)assert.deepEqual(r.final_checkpoint,final);else final=r.final_checkpoint;rows.push(r);
  }
 }
 assert.equal(new Set(rows.map(r=>r.vm_identity)).size,rows.length);assert.equal(new Set(rows.map(r=>r.initial_vm_root)).size,1);
 if(gate==='C')assert.equal(rows.length,24);
 if(gate==='B')for(const route of cfg.routes){
  const r=load(run+`/sessions/negative-k2-${route}.json`);assert.equal(r.kind,'NEGATIVE_ONLY');assert.equal(r.status,'PASS');r.transactions.forEach(transactionCheck);
  for(const n of r.negatives){assert.deepEqual(n.before,n.after);const t=r.transactions.find(t=>t.hash===n.transaction_hash);assert.equal(t.status,0);assert.equal(t.exception,'revert');assert.equal(t.revert_reason,n.reason);assert.equal(t.logs.length,0);}
  for(const name of ['illegal_batch_index','replacement_ballot_proof','within_batch_duplicate_nf','stale_old_state','cross_batch_duplicate_nf','receive_after_freeze'])assert.ok(r.negatives.some(n=>n.case===name));
  if(route==='legacy'){assert.deepEqual(r.recovery.map(s=>s.step),[1,2,3,4,5]);assert.ok(r.negatives.some(n=>n.case==='legacy_incomplete_freeze'));assert.ok(r.negatives.some(n=>n.case==='charlie_first_attempt'));
   assert.deepEqual(r.recovery.map(s=>s.checkpoint.count),[2,2,3,4,4]);assert.deepEqual(r.recovery.map(s=>s.checkpoint.next),[1,1,1,2,2]);}
  else assert.ok(r.negatives.some(n=>n.case==='omission_a'));
 }
 checkProtected(load(run+'/protected_files.json'));return {m,rows};
}
function outputPath(run){return 'results/stage1_multibatch/'+path.basename(run);}
function emit(p,v,check){if(check)assert.deepEqual(read(p),Buffer.from(typeof v==='string'?v:JSON.stringify(v,null,2)+'\n'),p);else {
 const f=path.join(root,p);fs.mkdirSync(path.dirname(f),{recursive:true});fs.writeFileSync(f,typeof v==='string'?v:JSON.stringify(v,null,2)+'\n',{flag:'wx'});}}
function csv(header,rows){return [header,...rows].map(r=>r.map(x=>x===null?'':String(x)).join(',')).join('\n')+'\n';}
function deriveD(run,check){
 const {m,rows}=checkRun(run,'C'),out=outputPath(run);const totals=[];const detail=[];
 for(const r of rows){const by=c=>r.transactions.filter(t=>t.category===c).reduce((s,t)=>s+t.gas_used,0);
  const deployment=by('deployment'),certification=by('certification'),intake=by('intake'),freeze=by('freeze');
  const entry={k:r.k,route:r.route,votes:4*r.k,deployment,certification,intake,freeze,deploy_receive:deployment+intake,prefix:deployment+certification+intake+freeze,
   calldata:r.transactions.filter(t=>t.category!=='deployment').reduce((s,t)=>s+t.calldata_bytes,0),status:'DERIVED_FROM_MEASURED_RECEIPTS'};totals.push(entry);
  for(const t of r.transactions)detail.push([r.k,r.route,t.label,t.category,t.hash,t.status,t.gas_used,t.calldata_bytes,'MEASURED']);
 }
 const crossings={};
 for(const basis of ['deploy_receive','prefix']){
  const points=cfg.formal_k.map(k=>{const df=totals.find(x=>x.k===k&&x.route==='direct_fold'),sp=totals.find(x=>x.k===k&&x.route==='state_proof');return {k,saved_gas:df[basis]-sp[basis],status:'DERIVED'};});
  crossings[basis]={points,first_observed_profitable_k:points.find(p=>p.saved_gas>0)?.k??null,scope:'all integers 1..8 measured; no extrapolation beyond 8 and no monotonicity assumption'};
 }
 const old={};for(const route of ['direct_fold','state_proof']){
  const h=load(`experiments/stage2_evm_fold_raw/run-20261114-01/${route}.json`),now=totals.find(t=>t.k===2&&t.route===route);
  const sum=prefix=>h.transactions.filter(t=>t.status===1&&t.label.startsWith(prefix)).reduce((s,t)=>s+t.gas_used,0);
  const prior={deployment:sum('deploy_'),certification:sum('authenticate_'),intake:sum('accept_batch_'),freeze:sum('freeze_')};
  const historical=h.final_checkpoint;const current=rows.find(r=>r.k===2&&r.route===route).final_checkpoint;
  assert.deepEqual(current.state,historical.state);assert.deepEqual(current.spent.slice(0,8),historical.spent);
  old[route]={status:'DERIVED',gas_deltas:Object.fromEntries(Object.keys(prior).map(k=>[k,now[k]-prior[k]])),
   explanation:{deployment:'New shared-base schedule storage and stage1 independent freeze implementation; child verifiers remain included in parent receipts.',
    certification:'Same original Gate, signatures and certified binding; any measured difference must remain explicit.',
    intake:'New common guards and layout/dispatch; same 25-input ballot relation and 37-input state relation. No requirement to match historical Gas.',
    freeze:'Independent stage1 ABI-based version binds K and full prefix schedule; not old G6 CT2 seal.'}};
 }
 emit(out+'/transactions.csv',csv(['k','route','label','category','tx_hash','status','gas_used','calldata_bytes','measurement_status'],detail),check);
 emit(out+'/cumulative_costs.csv',csv(['k','route','votes','deployment','certification','intake','freeze','deploy_receive','full_success_prefix','status'],totals.map(t=>[t.k,t.route,t.votes,t.deployment,t.certification,t.intake,t.freeze,t.deploy_receive,t.prefix,'DERIVED'])),check);
 emit(out+'/per_vote.csv',csv(['k','route','receive_gas_per_vote','deploy_receive_gas_per_vote','prefix_gas_per_vote','status'],totals.map(t=>[t.k,t.route,t.intake/t.votes,t.deploy_receive/t.votes,t.prefix/t.votes,'DERIVED'])),check);
 emit(out+'/offchain.json',{status:'MEASURED',setup:m.setup,batches:m.batches.map(b=>({batch:b.batch_index,witness_ms:measured(b.witness_ms),prove_ms:measured(b.prove_ms),verify_ms:measured(b.verify_ms),
  proof_bytes:measured(b.proof_ark_bytes),eip_bytes:measured(b.proof_eip_bytes),worker_peak_rss_mib:measured(b.peak_rss_mib),witness_scope:b.witness_scope,rss_scope:b.rss_scope})),
  original_ballot_generation_time:missing,snarkpack_evm_gas:missing,warning:'Diagnostic Witness timing overlaps work internally performed by Prove; do not add as exclusive subcomponents.'},check);
 emit(out+'/summary.json',{gate:'D',status:'PASS_AUDITED_DERIVATION',run,formal_clean_vms:24,costs:totals,crossings,historical_stage2_comparison:old,
  fallback_only:true,full_snarkpack_advantage:missing,public_chain_performance:missing,source_config_sha256:sha(read(configPath))},check);
 const report='# Stage1 多批次回退原型结果\n\n24 个独立 Prague VM；仅门槛 C 正式正例参与统计。\n\n'+
  '## 六项结论\n\n1. 三路线最终 CT、完整 Nullifier 集合、认证状态及阶段一冻结结果一致：PASS。\n'+
  '2. Legacy 每票原子、每批非原子；回归五步恢复由独立门槛 B 记录验收，不能说中途失败语义相同。\n'+
  `3. 仅部署＋接收首次观察到 State Proof 净节省的 K：${crossings.deploy_receive.first_observed_profitable_k??'1..8 未观察到'}。\n`+
  `4. 部署＋认证＋接收＋冻结首次观察到净节省的 K：${crossings.prefix.first_observed_profitable_k??'1..8 未观察到'}；未假定单调性。\n`+
  '5. 本次 Setup、逐批 Witness/Prove/Verify、独立工作进程 RSS 和动态字节数见 offchain.json；不是历史39.119s或旧RSS。Witness诊断不与Prove机械相加。\n'+
  '6. 结论仅适用于逐票 Groth16 EVM 回退；不能推广为完整 SnarkPack 链上效率或公链部署结论。\n\n'+
  'K2 与旧Stage2的分项差异及实现原因见 summary.json；旧冻结摘要不要求相同。费用来自本地 signed runTx receipts；内部CREATE/调用轨迹不重复计费。\n';
 emit(out+'/report.md',report,check);return {status:'PASS_GATE_D_AUDIT',formal_vms:24,crossings};
}
function main(){
 process.chdir(root);const a=process.argv.slice(2),check=a.includes('--check');const words=a.filter(x=>x!=='--check');assert.equal(words.length%2,0);
 const f=Object.fromEntries(Array.from({length:words.length/2},(_,i)=>[words[2*i],words[2*i+1]]));
 const gate=f['--gate'],run=f['--run'];assert.ok(['A','B','D'].includes(gate));assert.match(run??'',/^experiments\/stage1_multibatch_raw\/[A-Za-z0-9_-]+$/);validateConfig();
 let result;
 if(gate==='A')result=checkA(run);
 else if(gate==='B'){const v=checkRun(run,'B');result={status:'PASS_GATE_B_EVIDENCE_AUDIT_ONLY',development_vms:v.rows.length,formal_performance_samples:0};}
 else{assert.equal(f['--approved'],'D','Gate D requires explicit approval');result=deriveD(run,check);}
 const out=outputPath(run);emit(out+`/gate_${gate.toLowerCase()}_audit.json`,result,check);
 if(!check){const paths=[...artifactPaths(run),...artifactPaths(out)].filter(p=>!p.endsWith('/SHA256SUMS')).sort();emit(out+'/SHA256SUMS',paths.map(p=>sha(read(p))+'  '+p).join('\n')+'\n',false);}
 console.log(JSON.stringify(result,null,2));
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url))try{main();}catch(e){console.error(e.stack??e);process.exitCode=1;}
