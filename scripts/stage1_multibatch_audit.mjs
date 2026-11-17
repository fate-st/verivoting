// A/B evidence checks and explicitly approved D derivation. Never executes cryptography or EVM.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import {createRequire} from 'node:module';
import {root,cfg,configPath,load,read,sha,wire,json,checkProtected,validateConfig,recursiveSnapshot,approvedPatches,
 pragueCommon,pragueStatic,checkStorageLayouts,phaseCallSites,vmMetadataFields,inputProvenance,verifyA,verifyPreflightB,
 resultsPathForRun,sourcePatchSnapshot,readonlyInputSnapshot,checkReadonlyInputs,currentSourcePins,binaryReproducibility} from '../contracts/scripts/stage1_multibatch_compare.mjs';
const {Transaction,Interface}=createRequire(path.join(root,'contracts/package.json'))('ethers');
const {createTxFromRLP,getCalldataFloorGas}=createRequire(path.join(root,'contracts/package.json'))('@ethereumjs/tx');
const measured=value=>({status:'MEASURED',value});
const derived=value=>({status:'DERIVED',value});
const missing={status:'NOT_MEASURED',value:null};
const artifactPaths=dir=>fs.readdirSync(path.join(root,dir),{withFileTypes:true}).flatMap(e=>e.isDirectory()?e.name==='build-target'?[]:artifactPaths(dir+'/'+e.name):[dir+'/'+e.name]);
function sources(pins){for(const p of pins)assert.equal(sha(read(p.path)),p.sha256,p.path);}
function checkA(run){
 const a=verifyA(run);assert.equal(a.status,'PASS_GATE_A_COMPILE_ONLY');sources(a.source_pins);
 assert.equal(a.config_sha256,sha(read(configPath)));assert.equal(a.compiled_sha256,sha(read(run+'/compiled.json')));
 assert.equal(a.binary_sha256,sha(read(a.binary)));assert.ok(a.commands.every(c=>c.code===0&&c.reason===null));
 const compiled=load(run+'/compiled.json');sources(compiled.sources);
 assert.equal(compiled.compiler.split('+')[0],'0.8.30');assert.deepEqual(compiled.settings.optimizer,{enabled:true,runs:200});assert.equal(compiled.settings.viaIR,true);
 assert.equal(compiled.settings.evmVersion,'prague');assert.equal(a.setup_count,0);assert.equal(a.proof_generation_count,0);assert.equal(a.evm_transaction_count,0);
 assert.equal(a.vm_creation_count,0);assert.equal(a.static_checks_sha256,sha(read(run+'/static_checks.json')));
 assert.equal(a.storage_layout_sha256,sha(read(run+'/storage_layout.json')));checkStorageLayouts(compiled.contracts);pragueStatic();phaseCallSites();
 assert.deepEqual(load(run+'/static_checks.json').provenance,inputProvenance());
 const s=read('contracts/scripts/stage1_multibatch_compare.mjs').toString(),self=read('scripts/stage1_multibatch_audit.mjs').toString();
 assert.ok(!/assert\.equal\(entry\.refund,\s*0\)/.test(s));assert.ok(!/assert\.equal\(t\.refund,\s*0\)/.test(self));
 for(const field of vmMetadataFields)assert.ok(s.includes(field+':'),field+' output wiring');
 for(const field of gasFields)assert.ok(s.includes(field+':'),field+' output wiring');
 for(const expression of ['r.execResult.gasRefund??0n','blockNumber+=1n','blockGasUsed:0n','getCalldataFloorGas(signed,sender)',
  'used<=BigInt(cfg.block_gas_limit)','common.param(\'maxRefundQuotient\')'])assert.ok(s.includes(expression),expression);
 const preflight=load(run+'/preflight_b.json');verifyPreflightB(run,preflight.b_run);
 assert.deepEqual(preflight,load(resultsPathForRun(run)+'/preflight_b.json'));assert.ok(preflight.future_directories.every(p=>!fs.existsSync(path.join(root,p.path))));
 assert.ok(preflight.commands.every(c=>c.code===0));
 const callSites=phaseCallSites(),reproducibility=binaryReproducibility(run);
 // Runtime code paths are inspected here, never executed as functional tests.
 for(const e of ['verifyPreflightB(aRun,path.basename(run))','verifyGateB(bRun)','protectedSnapshot({phase:gate,run,aRun,bRun})'])assert.ok(s.includes(e),e);
 for(const e of ['m.setup.worker_peak_rss_mib<=cfg.budgets.child_rss_mib','b.peak_rss_mib<=cfg.budgets.child_rss_mib'])assert.ok(self.includes(e),e);
 checkProtected(load(run+'/protected_files.json'));
 const acceptance=[
  {id:1,item:'G18 / parent / HEAD chain and both fixed trees',status:'PASS_STATIC'},
  {id:2,item:'Two fixed inventories: 59A and 3M+31A',status:'PASS_STATIC'},
  {id:3,item:'Dynamic paths, every phase call site, A/B read-only inputs in C, D source freeze',status:'PASS_STATIC'},
  {id:4,item:'Recursive A-01/02/03 archives including ignored caches and absent result A-01',status:'PASS_STATIC'},
  {id:5,item:'Raw Legacy EVM refund counter; no zero-refund assumption',status:'PASS_STATIC'},
  {id:6,item:'Per-transaction blocks, BigInt 14 fields and receipt identities',status:'PASS_STATIC',runtime_status:'RUNTIME_PENDING_B'},
  {id:7,item:'Prague 2537/3529/7623 active; 8037 inactive; quotient 5',status:'PASS_STATIC'},
  {id:8,item:'Compiler storage slots and offsets 0/8/16/17',status:'PASS_COMPILE'},
  {id:9,item:'VM/context/C-VOTE boundaries, strong preflight hashes, dual C gate, RSS guards',status:'PASS_STATIC',runtime_status:['RUNTIME_PENDING_B','RUNTIME_PENDING_C']},
  {id:10,item:'Rust +1.99.1 Release opt-level=1, solc 0.8.30, A-03 binary reproducibility',status:'PASS_COMPILE'},
  {id:11,item:'Independent Gate B preflight only',status:'READY_FOR_HUMAN_APPROVAL'}];
 return {status:'PASS_GATE_A_ARTIFACT_AUDIT',gate:'A',config_sha256:a.config_sha256,
  run,acceptance,git_version:a.version,phase_call_sites:callSites,binary_reproducibility:reproducibility,
  admission_policy:load(run+'/static_checks.json').admission_policy,prague:pragueStatic(),storage_checks:compiled.storage_checks,vm_metadata_fields:vmMetadataFields,
  compiled_contracts:compiled.sizes.filter(c=>c.name.startsWith('Stage1')),commands:a.commands.map(c=>({program:c.program,args:c.args,code:c.code,stdout:c.stdout,stderr:c.stderr})),
  rust_opt_level:1,cryptographic_function_tests:missing,evm_gas:missing,next_gate:'B_REQUIRES_NEW_HUMAN_CONFIRMATION',historical_files_unchanged:true};
}
function checkMaterials(run,gate){
 const m=load(run+'/materials/materials.json');assert.equal(m.phase,gate);assert.equal(m.state_proofs_generated,gate==='B'?2:8);
 assert.equal(m.archived_state_proofs_reused,0);assert.equal(m.all_state_proofs_fresh,true);assert.equal(m.serial_workers,true);
 assert.equal(m.config_sha256,sha(read(configPath)));assert.equal(m.request_sha256,sha(read(run+'/materials/request.json')));
 assert.equal(m.ballots.length,32);assert.equal(new Set(m.ballots.map(b=>b.nullifier)).size,32);
 assert.equal(m.setup.vk_sha256,cfg.state_vk_ark_sha256);assert.ok(m.setup.setup_ms>0);
 assert.ok(m.setup.worker_peak_rss_mib>0&&m.setup.worker_peak_rss_mib<=cfg.budgets.child_rss_mib,'Setup worker RSS outside budget');
 const old=load('fixtures/g8_receiver_eip2537_v1.json');
 for(let i=0;i<m.batches.length;i++){
  const b=m.batches[i];assert.equal(b.phase,gate);assert.equal(b.batch_index,i+1);assert.equal(b.root,cfg.ordered_batch_roots[i]);
  assert.equal(b.proof_origin,'NEW_THIS_RUN_SEPARATE_SERIAL_WORKER');
  assert.equal(b.proof_ark_bytes,wire(b.proof_ark_hex).length);assert.equal(b.proof_ark_sha256,sha(wire(b.proof_ark_hex)));
  assert.equal(b.proof_eip_bytes,wire(b.eip.proof_neg_a_eip2537_hex).length);assert.equal(sha(wire(b.eip.vk_eip2537_hex)),cfg.state_vk_eip_sha256);
  assert.ok(b.peak_rss_mib>0&&b.prove_ms>0&&b.witness_ms>0&&b.verify_ms>0);
  assert.ok(b.peak_rss_mib>0&&b.peak_rss_mib<=cfg.budgets.child_rss_mib,'Prove worker RSS outside budget');
  if(i)assert.deepEqual(b.old_state,m.batches[i-1].new_state);
  if(i<2){const flatten=v=>v.flatMap(x=>['0x'+x.c0_hex,'0x'+x.c1_hex]);assert.deepEqual(b.old_state,flatten(old.batches[i].old_state));assert.deepEqual(b.new_state,flatten(old.batches[i].new_state));}
 }
 const commandPids=m.commands.map(c=>c.pid);assert.equal(new Set(commandPids).size,commandPids.length);
 for(const c of m.commands){assert.equal(c.exit_code,0);assert.ok(c.wall_ms>0);}
 return m;
}
const gasFields=['block_number','transaction_index_in_block','block_gas_used_before','gas_pre_refund','refund_counter_raw','refund_cap',
 'refund_applied_before_floor','gas_after_refund_before_floor','minimum_gas_limit','eip7623_floor_gas','floor_adjustment','refund_effective_in_receipt','gas_used','receipt_cumulative_gas'];
function transactionCheck(t){
 const signed=Transaction.from(t.signed_transaction_hex);assert.ok(signed.isSigned());assert.equal(signed.hash,t.hash);assert.equal(signed.data,t.calldata_hex);assert.equal(BigInt(signed.nonce),BigInt(t.nonce));
 const decoded=createTxFromRLP(wire(t.signed_transaction_hex),{common:pragueCommon()}),sender=decoded.getSenderAddress();assert.equal(decoded.type,0,'Legacy transaction path');
 assert.equal(wire(t.calldata_hex).length,t.calldata_bytes);assert.equal(sha(wire(t.calldata_hex)),t.calldata_sha256);
 for(const f of [...gasFields,'nonce','intrinsic_gas','execution_gas'])assert.match(t[f],/^(0|[1-9][0-9]*)$/,f+' canonical decimal string');
 const n=Object.fromEntries(gasFields.map(f=>[f,BigInt(t[f])]));
 assert.equal(BigInt(t.intrinsic_gas),decoded.getIntrinsicGas());assert.equal(n.minimum_gas_limit,decoded.getMinimumGasLimit());
 assert.equal(n.eip7623_floor_gas,getCalldataFloorGas(decoded,sender));
 assert.equal(n.transaction_index_in_block,0n);assert.equal(n.block_gas_used_before,0n);
 assert.equal(n.gas_pre_refund,BigInt(t.intrinsic_gas)+BigInt(t.execution_gas));assert.equal(n.refund_cap,n.gas_pre_refund/5n);
 const min=(a,b)=>a<b?a:b,max=(a,b)=>a>b?a:b;
 assert.equal(n.refund_applied_before_floor,min(n.refund_counter_raw,n.refund_cap));
 assert.equal(n.gas_after_refund_before_floor,n.gas_pre_refund-n.refund_applied_before_floor);
 assert.equal(n.floor_adjustment,max(0n,n.eip7623_floor_gas-n.gas_after_refund_before_floor));
 assert.ok(n.gas_used<=BigInt(cfg.block_gas_limit));assert.ok(n.gas_used<=BigInt(cfg.transaction_gas_limit));
 assert.equal(n.gas_used,max(n.gas_pre_refund-n.refund_applied_before_floor,n.eip7623_floor_gas));
 assert.equal(n.refund_effective_in_receipt,max(0n,n.refund_applied_before_floor-n.floor_adjustment));
 assert.equal(n.refund_effective_in_receipt,max(0n,n.gas_pre_refund-n.gas_used));
 assert.equal(n.receipt_cumulative_gas,n.block_gas_used_before+n.gas_used);
}
function crossVmCheck(rows){
 const context='0x'+load('fixtures/jubjub_g6_native_winner_only_public_v1.json').public_context.final_context_hex.replace(/^0x/,'');
 for(const r of rows){for(const f of vmMetadataFields)assert.notEqual(r[f],undefined,f);
  assert.equal(r.chain_id,'1');assert.equal(r.final_context,context);assert.equal(r.deployer_nonce_initial,'0');
  assert.equal(r.gate_deployment_nonce,'0');assert.equal(r.receiver_deployment_nonce,'1');
  assert.equal(BigInt(r.deployer_nonce_final),BigInt(r.deployer_nonce_initial)+BigInt(r.transactions.length));
  assert.equal(r.gate_address,r.addresses.gate);assert.equal(r.receiver_address,r.addresses.receiver);
  for(const [i,t] of r.transactions.entries()){
   assert.equal(BigInt(t.block_number),BigInt(i)+1n);assert.equal(BigInt(t.nonce),BigInt(r.deployer_nonce_initial)+BigInt(i));
   const signed=Transaction.from(t.signed_transaction_hex);assert.equal(signed.chainId,BigInt(r.chain_id));assert.equal(signed.from.toLowerCase(),r.deployer_address.toLowerCase());}
  for(const name of ['gate','receiver']){const t=r.transactions.find(t=>t.label==='deploy_'+name);assert.ok(t);assert.equal(sha(wire(t.calldata_hex)),r[name+'_initcode_sha256']);}
  assert.match(r.receiver_runtime_code_sha256,/^[0-9a-f]{64}$/);
 }
 for(const field of ['chain_id','deployer_address','deployer_nonce_initial','gate_deployment_nonce','receiver_deployment_nonce','gate_address','receiver_address','gate_initcode_sha256','final_context'])
  assert.equal(new Set(rows.map(r=>r[field])).size,1,'isolated VM invariant '+field);
 assert.equal(new Set(rows.map(r=>r.vm_identity)).size,rows.length);
 // Constructor embeds K/schedule in immutables: do not assume identical initcode/runtime across K.
 for(const r of rows)for(const other of rows.filter(o=>o.route===r.route&&o.k===r.k)){
  assert.equal(r.receiver_initcode_sha256,other.receiver_initcode_sha256);assert.equal(r.receiver_runtime_code_sha256,other.receiver_runtime_code_sha256);}
 for(const k of new Set(rows.map(r=>r.k))){const group=rows.filter(r=>r.k===k);if(new Set(group.map(r=>r.route)).size===3)
  assert.equal(new Set(group.map(r=>r.receiver_runtime_code_sha256)).size,3,'three distinct route implementations');}
}
function checkRun(run,gate){
 const request=load(run+'/request.json');assert.equal(request.gate,gate);verifyA(request.a_run);
 const inputsBefore=readonlyInputSnapshot({phase:gate,aRun:request.a_run,bRun:request.b_run});
 const top=load(run+`/gate_${gate.toLowerCase()}.json`);sources(top.source_pins);assert.deepEqual(top.source_pins,currentSourcePins());
 assert.equal(top.config_sha256,sha(read(configPath)));assert.equal(top.run,run);assert.equal(top.a_run,request.a_run);
 const m=checkMaterials(run,gate);
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
 const negatives=[];if(gate==='B')for(const route of cfg.routes){
  const r=load(run+`/sessions/negative-k2-${route}.json`);assert.equal(r.kind,'NEGATIVE_ONLY');assert.equal(r.status,'PASS');r.transactions.forEach(transactionCheck);
  for(const n of r.negatives){assert.deepEqual(n.before,n.after);const t=r.transactions.find(t=>t.hash===n.transaction_hash);assert.equal(t.status,0);assert.equal(t.exception,'revert');assert.equal(t.revert_reason,n.reason);assert.equal(t.logs.length,0);}
  for(const name of ['illegal_batch_index','replacement_ballot_proof','within_batch_duplicate_nf','stale_old_state','cross_batch_duplicate_nf','receive_after_freeze'])assert.ok(r.negatives.some(n=>n.case===name));
  if(route==='legacy'){assert.deepEqual(r.recovery.map(s=>s.step),[1,2,3,4,5]);assert.ok(r.negatives.some(n=>n.case==='legacy_incomplete_freeze'));assert.ok(r.negatives.some(n=>n.case==='charlie_first_attempt'));
   assert.deepEqual(r.recovery.map(s=>s.checkpoint.count),[2,2,3,4,4]);assert.deepEqual(r.recovery.map(s=>s.checkpoint.next),[1,1,1,2,2]);}
  else assert.ok(r.negatives.some(n=>n.case==='omission_a'));
  negatives.push(r);
 }
 crossVmCheck([...rows,...negatives]);assert.equal(rows.length+negatives.length,gate==='B'?9:24);
 checkProtected(load(run+'/protected_files.json'));checkReadonlyInputs(inputsBefore);return {m,rows,negatives};
}
const outputPath=resultsPathForRun;
function emit(p,v,check){if(check)assert.deepEqual(read(p),Buffer.from(typeof v==='string'?v:json(v)),p);else {
 const f=path.join(root,p);fs.mkdirSync(path.dirname(f),{recursive:true});fs.writeFileSync(f,typeof v==='string'?v:json(v),{flag:'wx'});}}
function csv(header,rows){return [header,...rows].map(r=>r.map(x=>x===null?'':String(x)).join(',')).join('\n')+'\n';}
// Exact rational values, not floating-point rounded pseudo-measurements.
const perVote=(gas,votes)=>({numerator:gas.toString(),denominator:BigInt(votes).toString(),exact_fraction:gas.toString()+'/'+BigInt(votes).toString()});
function deriveD(rawRun,out,check){
 const {m,rows}=checkRun(rawRun,'C');const totals=[];const detail=[];
 for(const r of rows){const by=c=>r.transactions.filter(t=>t.category===c).reduce((s,t)=>s+BigInt(t.gas_used),0n);
  const deployment=by('deployment'),certification=by('certification'),intake=by('intake'),freeze=by('freeze');
  const entry={k:r.k,route:r.route,votes:4*r.k,deployment,certification,intake,freeze,deploy_receive:deployment+intake,prefix:deployment+certification+intake+freeze,
   calldata:r.transactions.filter(t=>t.category!=='deployment').reduce((s,t)=>s+t.calldata_bytes,0),status:'DERIVED_FROM_MEASURED_RECEIPTS'};totals.push(entry);
  for(const t of r.transactions)detail.push([r.k,r.route,t.label,t.category,t.hash,t.status,t.gas_used,t.calldata_bytes,'MEASURED']);
 }
 const crossings={};
 for(const basis of ['deploy_receive','prefix']){
  const points=cfg.formal_k.map(k=>{const df=totals.find(x=>x.k===k&&x.route==='direct_fold'),sp=totals.find(x=>x.k===k&&x.route==='state_proof');return {k,saved_gas:df[basis]-sp[basis],status:'DERIVED'};});
  crossings[basis]={points,first_observed_profitable_k:points.find(p=>p.saved_gas>0n)?.k??null,scope:'all integers 1..8 measured; no extrapolation beyond 8 and no monotonicity assumption'};
 }
 const old={};for(const route of ['direct_fold','state_proof']){
  const h=load(`experiments/stage2_evm_fold_raw/run-20261114-01/${route}.json`),now=totals.find(t=>t.k===2&&t.route===route);
  const sum=prefix=>h.transactions.filter(t=>t.status===1&&t.label.startsWith(prefix)).reduce((s,t)=>s+BigInt(t.gas_used),0n);
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
 emit(out+'/per_vote.csv',csv(['k','route','receive_gas_per_vote_exact','deploy_receive_gas_per_vote_exact','prefix_gas_per_vote_exact','status'],totals.map(t=>[t.k,t.route,perVote(t.intake,t.votes).exact_fraction,perVote(t.deploy_receive,t.votes).exact_fraction,perVote(t.prefix,t.votes).exact_fraction,'DERIVED'])),check);
 emit(out+'/offchain.json',{status:'MEASURED',setup:m.setup,batches:m.batches.map(b=>({batch:b.batch_index,witness_ms:measured(b.witness_ms),prove_ms:measured(b.prove_ms),verify_ms:measured(b.verify_ms),
  proof_bytes:measured(b.proof_ark_bytes),eip_bytes:measured(b.proof_eip_bytes),worker_peak_rss_mib:measured(b.peak_rss_mib),witness_scope:b.witness_scope,rss_scope:b.rss_scope})),
  original_ballot_generation_time:missing,snarkpack_evm_gas:missing,warning:'Diagnostic Witness timing overlaps work internally performed by Prove; do not add as exclusive subcomponents.'},check);
 emit(out+'/summary.json',{gate:'D',status:'PASS_AUDITED_DERIVATION',run:out,raw_run:rawRun,formal_clean_vms:24,costs:totals,crossings,historical_stage2_comparison:old,
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
 const sourceBefore=sourcePatchSnapshot();
 const gate=f['--gate'],phase=gate,run=f['--run'],rawRun=f['--raw-run'];assert.ok(['A','B','D'].includes(gate));
 if(gate!=='D')assert.match(run??'',/^experiments\/stage1_multibatch_raw\/[A-Za-z0-9_-]+$/);
 const request=gate==='B'?load(run+'/request.json'):gate==='D'?load(rawRun+'/request.json'):null;
 const aRun=request?.a_run;
 validateConfig({phase,run,aRun,rawRun});
 const inputsBefore=gate==='B'?readonlyInputSnapshot({phase,aRun}):gate==='D'?
  [...readonlyInputSnapshot({phase,rawRun}),...readonlyInputSnapshot({phase:'C',aRun,bRun:request.b_run})]:[];
 const rawBefore=gate==='D'?recursiveSnapshot(rawRun):null;
 try{
  let result;
  if(gate==='A')result=checkA(run);
  else if(gate==='B'){
   const v=checkRun(run,'B');assert.equal(v.rows.length,6);assert.equal(v.negatives.length,3);
   assert.equal(new Set([...v.rows,...v.negatives].map(r=>r.vm_identity)).size,9);
   result={status:'PASS_GATE_B_EVIDENCE_AUDIT_ONLY',positive_vm_count:6,negative_vm_sessions:3,independent_vm_count:9,
    negative_scope:'3 independent VM sessions, each containing multiple attack scenarios',gas_receipt_audit:'PASS',
    config_sha256:sha(read(configPath)),source_pins:currentSourcePins(),formal_performance_samples:0};
  }
  else{assert.equal(f['--approved'],'D','Gate D requires explicit approval');assert.equal(rawBefore.exists,true);result=deriveD(rawRun,run,check);}
  checkReadonlyInputs(inputsBefore);assert.deepEqual(sourcePatchSnapshot(),sourceBefore,'no source modification during audit');
  const out=gate==='D'?run:outputPath(run);emit(out+`/gate_${gate.toLowerCase()}_audit.json`,result,check);
  if(gate==='A'){
   const table=result.acceptance.map(r=>`| ${r.id} | ${r.item} | ${r.status}${r.runtime_status?' / '+[].concat(r.runtime_status).join(' / '):''} |`).join('\n');
   emit(out+'/report.md','# '+path.basename(run)+' acceptance (compile/static/preflight only)\n\n| # | Item | Status |\n|---|---|---|\n'+table+
    '\n\nSetup=0; Prove=0; VM creation=0; EVM transactions=0. Runtime assertions are wired but have NOT been executed. No commit or push.\n\n'+
    'B preflight is READY_FOR_HUMAN_APPROVAL, not a B pass. B reads the A raw/results inputs; C reads both A and B raw/results inputs with full recursive before/after checks. RSS guards and B/C admission checks are statically wired, not runtime-tested.\n',check);
  }
  if(!check){const input=gate==='D'?rawRun:run;const paths=[...artifactPaths(input),...artifactPaths(out)].filter(p=>!p.endsWith('/SHA256SUMS')).sort();emit(out+'/SHA256SUMS',paths.map(p=>sha(read(p))+'  '+p).join('\n')+'\n',false);}
  console.log(json(result));
 }finally{
  assert.deepEqual(sourcePatchSnapshot(),sourceBefore,'audits must not mutate approved source patches');
  checkReadonlyInputs(inputsBefore);
  if(gate==='D')assert.deepEqual(recursiveSnapshot(rawRun),rawBefore,'Gate C rawRun changed during read-only Gate D audit');
 }
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url))try{main();}catch(e){console.error(e.stack??e);process.exitCode=1;}
