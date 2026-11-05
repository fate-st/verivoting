// The functional entrypoint. No optimization loops or batch-size expansion.
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';import {createHash} from 'node:crypto';
const args=process.argv.slice(2),existing=args[0]==='--finalize-existing';
const out=path.resolve(existing?args[1]??'':args[0]??'');
assert.ok(existing?args.length===2:args.length===1,'usage: node scripts/run_functional_experiment.mjs NEW_RUN_ROOT | --finalize-existing COMPLETED_ROOT');
const sha=b=>createHash('sha256').update(b).digest('hex');
const commands=[];
if(!existing){assert.ok(!fs.existsSync(out),'new output directory required');
 for(const script of ['scripts/g10c_run.mjs','scripts/g10c_chain.mjs']){
  const t=performance.now(),r=spawnSync(process.execPath,[script,out],{stdio:'inherit',timeout:1200000});
  commands.push({program:process.execPath,args:[script,out],exit_code:r.status,elapsed_ms:performance.now()-t});
  assert.equal(r.status,0,script);
 }
}
const load=p=>JSON.parse(fs.readFileSync(path.join(out,p)));
const n=load('native/report.json'),c=load('chain/winner_once.json'),nm=load('execution/manifest.json'),cm=load('chain_bridge/manifest.json');
assert.equal(nm.status,'PASS_CONNECTED_NATIVE_NEW_P3_AND_SEPARATE_PUBLIC_RELEASE_VERIFIERS');
assert.equal(cm.status,'PASS_SAME_NATIVE_PROOF_AND_WORKER_COMMITMENTS_CONNECTED_TO_EVM_FALLBACK');
assert.equal(n.p3_proof_sha256,c.ark_proof_sha256);assert.equal(n.worker_evidence.unique_process_ids,5);
assert.equal(n.new_p3_proofs,1);assert.equal(n.winner_writes,1);assert.equal(c.successful_winner_writes,1);assert.equal(c.winner_event_count,1);
assert.deepEqual(c.public_inputs_be_hex,load('native/winner_only/evm.json').public_inputs_be_hex);
const successful=c.transactions.filter(t=>t.receipt_status===1),deploy=successful.filter(t=>t.label.startsWith('deploy_'));
const sum=(a,k)=>a.reduce((s,t)=>s+t[k],0);
const manifest={schema:'VV/FUNCTIONAL-COMPLETION/v1',status:'PASS_IMPLEMENTED_NATIVE_AND_INDIVIDUAL_GROTH16_EVM_FALLBACK_FLOW',
 recorded_utc:new Date().toISOString(),one_command_driver_invoked_for_proving_stages:!existing,finalized_from_actual_sequential_stage_runs:existing,commands,
 driver_sha256:sha(fs.readFileSync(new URL(import.meta.url))),native_report_sha256:sha(fs.readFileSync(path.join(out,'native/report.json'))),
 chain_report_sha256:sha(fs.readFileSync(path.join(out,'chain/winner_once.json'))),native_proof_matches_evm:true,new_p3_proofs:1,cp_worker_processes:5,
 public_input_fields_matched:60,one_winner_write_in_each_executed_state_machine:true,
 successful_evm_transactions:successful.length,rejected_acceptance_transactions:c.transactions.length-successful.length,
 successful_gas_including_deployment:sum(successful,'total_gas'),deployment_gas:sum(deploy,'total_gas'),successful_non_deployment_gas:sum(successful.filter(t=>!t.label.startsWith('deploy_')),'total_gas'),
 successful_calldata_bytes:sum(successful,'calldata_bytes'),gas_totals_scope:'same new EVM execution receipts; not native proving costs, not a one-block total or exact SnarkPack cost',
 implementation_profile:'native exact SnarkPack + original state proofs, new worker CP/P3; authenticated EVM individual-Groth16 fallback',
 full_target_architecture_complete:false,exact_snarkpack_evm:false,independent_distributed_dkg:false,production_deployment:false,full_privacy_proof:false,
 full_tally_is_counterfactual_diagnostic_not_authorized_winner_only_release:true,full_tally_usable_material_not_in_public_archive:true,
 remaining_required:['exact SnarkPack EVM verification + its receiver integration','independent DKG/private key custody in the same full-flow context','T4/T5 mathematical security composition','final matched full-system comparison and clean-source reproduction']};
fs.writeFileSync(path.join(out,'functional_completion.json'),JSON.stringify(manifest,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({status:manifest.status,new_p3:1,workers:5,evm_transactions:c.transactions.length,successful_gas:manifest.successful_gas_including_deployment,target_architecture_complete:false}));
