// Deterministic post-processing only: no EVM, key generation or proof execution.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {audit} from './stage2_evm_fold_check.mjs';
const [runArg, resultsArg] = process.argv.slice(2);
assert.ok(runArg && resultsArg, 'usage: node scripts/stage2_evm_fold_archive.mjs RUN_DIR RESULTS_DIR');
const run = path.resolve(runArg), results = path.resolve(resultsArg);
const sha = b => createHash('sha256').update(b).digest('hex');
const read = p => JSON.parse(fs.readFileSync(p));
const writeNew = (p,v) => fs.writeFileSync(p, JSON.stringify(v,null,2)+'\n', {flag:'wx'});
const checked = audit(run);
const a = read(path.join(run,'direct_fold.json')), b = read(path.join(run,'state_proof.json'));
const s = read(path.join(results,'summary.json'));
writeNew(path.join(run,'negative_tests.json'), {
  status:'PASS_ALL_18_ATOMIC_REJECTIONS', derived:true,
  sources:['direct_fold','state_proof'].map(n=>({path:n+'.json',sha256:sha(fs.readFileSync(path.join(run,n+'.json')))})),
  direct_fold:a.negatives,state_proof:b.negatives,
  scope:'Application checkpoint rollback only; Ethereum nonce and fee consumption are not reverted. Encoding mutation hits batch-root check; independent point probes are in points.json.',
});
writeNew(path.join(run,'gas_breakdown.json'), {
  status:'MEASURED_RECEIPTS_WITH_NON_ADDITIVE_CALL_TRACES', derived:true,
  notes:'Inclusive callee cost already contains nested precompiles. Residual is NOT a pure Direct Fold/common-business cost; inline Direct Fold and pure shared business remain unmeasured null.',
  routes:[a,b].map(r=>({route:r.phase,shared_business_only_gas:null,inline_direct_fold_only_gas:null,
    accepted:r.transactions.filter(t=>t.label.startsWith('accept_batch_')).map((t,i)=>({
      label:t.label,tx_hash:t.hash,gas_used:t.gas_used,intrinsic_gas:t.intrinsic_gas,
      execution_gas:t.execution_gas,refund:t.refund,calldata_floor:t.calldata_floor,
      calldata_floor_uplift:t.gas_used-(t.intrinsic_gas+t.execution_gas),
      calldata_bytes:t.calldata_bytes,zero_bytes:t.calldata_zero_bytes,nonzero_bytes:t.calldata_nonzero_bytes,
      ballot_verifier_inclusive_call_gas:r.common_ballot_verifier_call_gas[i],
      state_verifier_inclusive_call_gas:r.state_verifier_call_gas?.[i]??null,
      other_execution_residual_gas:t.execution_gas-r.common_ballot_verifier_call_gas[i]-(r.state_verifier_call_gas?.[i]??0),
    })),
    deployments:r.transactions.filter(t=>t.label.startsWith('deploy_')).map(({label,hash,gas_used,calldata_bytes})=>({label,tx_hash:hash,gas_used,calldata_bytes})),
  })),
});
// Same deterministic flat export as the separately recalculated artifact-tool CSV.
const headers=['route','batch1_gas','batch2_gas','receive_gas','deployment_gas','authentication_gas','freeze_gas','prefix_gas','receive_calldata_bytes','receive_gas_per_vote','worker_peak_rss_mib','new_setup_count','new_proof_count','original_prove_ms_unmeasured','inline_fold_gas_unmeasured','source_path','source_sha256','backend'];
const rows=[s.direct_fold,s.state_proof].map(r=>[r.route,r.batch1_gas,r.batch2_gas,r.two_batch_gas,r.total_deployment_gas,r.certificate_authentication_gas,r.freeze_gas,r.successful_deploy_auth_receive_freeze_gas,r.two_batch_calldata_bytes,r.gas_per_vote,r.peak_process_rss_mib,0,0,null,null,r.source_path,r.source_sha256,'individual_groth16_fallback']);
const cell=v=>v===null?'':/[",\r\n]/.test(String(v))?'"'+String(v).replaceAll('"','""')+'"':String(v);
const csv=[headers,...rows].map(r=>r.map(cell).join(',')).join('\n')+'\n';
const csvPath=path.join(results,'comparison.csv');
if(fs.existsSync(csvPath))assert.equal(fs.readFileSync(csvPath,'utf8'),csv,'CSV must exactly match receipt-derived summary');
else fs.writeFileSync(csvPath,csv,{flag:'wx'});
writeNew(path.join(results,'archive_validation.json'),{
  ...checked, csv_matches_summary:true, engine_checked_csv:fs.existsSync(path.join(results,'csv_validation.json')),
  processing_sources:['scripts/stage2_evm_fold_check.mjs','scripts/stage2_evm_fold_report.mjs','scripts/stage2_evm_fold_archive.mjs'].map(p=>({path:p,sha256:sha(fs.readFileSync(p))})),
  report_doc_sha256:sha(fs.readFileSync('docs/STAGE2_EVM_FOLD_COMPARISON.md')),
});
function files(dir) {
  return fs.readdirSync(dir,{withFileTypes:true}).flatMap(e=>e.isDirectory()?files(path.join(dir,e.name)):[path.join(dir,e.name)]);
}
for(const dir of [run,results]) {
  const checksum=path.join(dir,'SHA256SUMS'); assert.ok(!fs.existsSync(checksum),'archive already sealed');
  const paths=files(dir).sort();
  fs.writeFileSync(checksum,paths.map(p=>sha(fs.readFileSync(p))+'  '+path.relative(dir,p)).join('\n')+'\n',{flag:'wx'});
  console.log('SEALED '+dir+': '+paths.length+' files');
}
console.log('PASS CSV equality, explicit unmeasured components, receipt-only derived files and SHA-256 archives');
