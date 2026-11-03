// Evidence synchronization only: never run Setup, Prove or an EVM.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
process.chdir(path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..'));
const mode=process.argv[2]??'--check'; assert.ok(['--write','--check'].includes(mode));
const paperAt=process.argv.indexOf('--paper');
const out='results/p0_paper_sync', snapshot=out+'/sn-article.synced.tex';
assert.ok(mode==='--check'||paperAt>=0,'--write requires --paper PATH');
const paperPath=paperAt>=0?process.argv[paperAt+1]:snapshot;
const sha=b=>createHash('sha256').update(b).digest('hex');
const read=p=>fs.readFileSync(p),load=p=>JSON.parse(read(p));
const baseline='a5204043bdcec5a788f79de2ad79225320845e64';
const sourcePaths=[
 'results/g1_e/summary.json','experiments/g2_e_raw/multibatch_state_b4.json',
 'experiments/g4_d_raw/ten_subsets_p3_v2.json','experiments/g6_raw/native_winner_only.json',
 'experiments/g7_raw/g7_p3_prague_evm.json','experiments/g8c_raw/frozen_p3_binding.json',
 'experiments/g8d_raw/winner_once.json','results/g8_e/lifecycle_costs.json','results/g9/route_review.json'];
const sources=sourcePaths.map(p=>{const b=read(p);assert.equal(sha(b),sha(execFileSync('git',['show',baseline+':'+p],{maxBuffer:16*1024*1024})),p);return {path:p,sha256:sha(b)};});
const [g1,g2,g4,g6,g7,g8c,g8d,g8e,g9]=sourcePaths.map(load);
assert.deepEqual(Object.keys(g1.batches),['2','4','8','16','32']);
assert.equal(g1.timing_trials,150);
assert.ok(g2.second_old_equals_first_new&&g2.final_equals_independent_eight_ballot_fold);
assert.equal(g4.proof_count,10);assert.equal(g4.verify_count,10);assert.ok(g4.one_setup_and_one_vk&&g4.all_ten_matrices_equal);
assert.equal(g6.shape.constraints,6236368);assert.equal(g4.cases[0].shape.constraints,6236368);
assert.equal(g6.verifying_key_sha256,g4.verifying_key_sha256);assert.equal(g7.verifying_key_ark_sha256,g4.verifying_key_sha256);
assert.ok(g8c.all_60_inputs_match&&g8d.winner_write_once_integrated&&g8d.winner_written);
assert.equal(g8d.successful_winner_writes,1);assert.equal(g8d.snarkpack_evm_verified,false);
assert.equal(g9.exact_snarkpack_evm_verified,false);
const paper=read(paperPath),tex=paper.toString();
for(const token of ['a520404','6,236,368','65,473,608','31,390,919','17,236','1,331,401','2,058,601','2,073,560','tab:native-batch-measured']) assert.ok(tex.includes(token),token);
const medians=Object.entries(g1.batches).map(([B,v])=>{
 const row={B:Number(B),n:v.timing.individual_verify.n,individual_ms:v.timing.individual_verify.median,aggregate_ms:v.timing.aggregate_verify.median,aggregation_ms:v.timing.aggregation.median};
 assert.equal(row.n,30);for(const k of ['individual_ms','aggregate_ms','aggregation_ms'])assert.ok(tex.includes(row[k].toFixed(3)));
 return row;
});
const selectTx=(data,gas)=>{const a=data.transactions.filter(t=>t.receipt_status===1&&t.total_gas===gas);assert.equal(a.length,1);return {label:a[0].label,gas:a[0].total_gas,calldata_bytes:a[0].calldata_bytes};};
assert.equal(g7.valid.total_gas,1331401);
const claims=[
 {id:'G1-E',section:'subsec:batch-evaluation',measured:true,scope:'native fixed pool, not independent elections or EVM gas',medians,reproduction_report:'docs/G1_E_FIXED_SUITE_BENCHMARK.md'},
 {id:'G2-E',section:'subsec:batch-evaluation',measured:true,scope:'two consecutive B4 native transitions over eight distinct ballots',reproduction_report:'docs/G2_E_MULTIBATCH_STATE.md'},
 {id:'G4-D',section:'subsec:threshold-evaluation',measured:true,subsets:10,constraints:g6.shape.constraints,public_inputs:60,proof_bytes:192,vk_sha256:g4.verifying_key_sha256,scope:'fixed 5/3 profile; not arbitrary n/t or ten EVM cases',reproduction_report:'docs/G4_D_TEN_SUBSETS_P3_V2.md'},
 {id:'G6',section:'subsec:threshold-evaluation',measured:true,setup_rebuild_ms:g6.setup_rebuild_ms,prove_ms:g6.prove_ms,disclosure_combined_ms:g6.winner_disclosure_check_prepare_verify_write_ms,peak_process_rss_mib:g6.peak_rss_mib,scope:'one native flow, in-process private delivery',reproduction_report:'docs/G6_NATIVE_WINNER_ONLY.md'},
 {id:'G7',section:'tab:prototype-verifiers',measured:true,receipt_gas:g7.valid.total_gas,calldata_bytes:g7.valid.calldata_bytes,reproduction_report:'docs/G7_P3_EVM.md'},
 {id:'G8-C',section:'tab:prototype-verifiers',measured:true,...selectTx(g8c,2058601),scope:'fixed frozen storage binding; individual Groth16 intake',reproduction_report:'docs/G8_C_FROZEN_P3_BINDING.md'},
 {id:'G8-D',section:'tab:prototype-verifiers',measured:true,...selectTx(g8d,2073560),scope:'fixed local fallback Winner write-once',reproduction_report:'docs/G8_D_WINNER_ONCE.md'},
 {id:'G8-E',section:'subsec:threshold-evaluation',measured:false,derived_from_measured_receipts:true,totals:g8e.totals,scope:'same-run receipt accounting, offline costs not retimed',reproduction_report:'docs/G8_E_LIFECYCLE_COSTS.md'},
 {id:'G9',section:'sec6',measured:false,implemented:false,status:'BLOCKED',scope:'route/design only; no exact EVM verification or total Gas'}];
const report={schema:'VV/P0/PAPER-EVIDENCE/v1',status:'PASS_PAPER_EVIDENCE_SYNC',baseline_commit:baseline,paper_sha256:sha(paper),snapshot,sources,claims,new_setup:0,new_prove:0,new_evm_transactions:0,complete_privacy_theorem:false,full_end_to_end_comparison:false};
const put=(p,b)=>{if(mode==='--check'||fs.existsSync(p))assert.deepEqual(read(p),Buffer.from(b),p);else{fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,b,{flag:'wx'});}};
put(snapshot,paper);put(out+'/claim_evidence.json',JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({status:report.status,mode,claims:claims.length,paper_sha256:report.paper_sha256,new_proofs:0,new_evm:0}));
