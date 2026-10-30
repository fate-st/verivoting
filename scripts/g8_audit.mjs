// Archive/trace consistency audit only; actual crypto executes in Rust and Prague.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
const read=p=>JSON.parse(fs.readFileSync(p));
const hash=b=>createHash('sha256').update(b).digest('hex');
const sha=p=>hash(fs.readFileSync(p));
const bytes=h=>Buffer.from(h.replace(/^0x/,''),'hex');
const r=read('experiments/g8_raw/receiver_freeze.json'),f=read(r.fixture),e=read(r.export_report);
assert.equal(r.status,'PARTIAL_G8_REAL_INDIVIDUAL_BALLOT_STATE_EVM_CT2_FREEZE_PASS');
assert.equal(sha(r.fixture),r.fixture_sha256);assert.equal(sha(r.export_report),r.export_report_sha256);
for(const p of [...r.source_pins,...r.source_hashes,...e.source_hashes])assert.equal(sha(p.path),p.sha256,p.path);
assert.equal(r.hardfork,'prague');assert.equal(r.setup_count,0);assert.equal(r.proof_generation_count,0);
assert.equal(r.reused_ballot_proofs,8);assert.equal(r.reused_state_proofs,2);
function checkEquationTrace(msm,pairing,vk,proof,inputs){
 const k=bytes(vk),p=bytes(proof),scalars=['0x'+'0'.repeat(63)+'1',...inputs];
 const input=Buffer.concat(scalars.flatMap((x,i)=>[k.subarray(896+128*i,896+128*(i+1)),bytes(x)]));
 assert.equal(hash(input),msm.input_sha256);assert.equal(input.length,msm.input_bytes);
 const pairs=Buffer.concat([p.subarray(0,384),k.subarray(0,384),bytes(msm.return_hex),k.subarray(384,640),p.subarray(384),k.subarray(640,896)]);
 assert.equal(hash(pairs),pairing.input_sha256);assert.equal(pairs.length,1536);
 assert.ok(msm.success&&pairing.success);assert.equal(BigInt(pairing.return_hex),1n);
}
assert.equal(r.accepted_batches.length,2);
for(const [i,b]of r.accepted_batches.entries()){
 assert.equal(b.receipt_status,1);assert.equal(b.checkpoint.next_batch_index,i+2);assert.equal(b.checkpoint.spent_count,(i+1)*4);
 assert.equal(b.checkpoint.frozen,false);
 const curve=b.precompile_calls.filter(x=>['000c','000f'].includes(x.address.slice(-4)));
 assert.deepEqual(curve.map(x=>x.address.slice(-4)),Array(5).fill(['000c','000f']).flat());
 for(let j=0;j<5;j++){
  const vk=j<4?f.ballot_vk_eip2537_hex:f.state_vk_eip2537_hex;
  const proof=j<4?f.batches[i].ballot_proofs_eip2537_hex[j]:f.batches[i].state_proof_eip2537_hex;
  const inputs=j<4?f.batches[i].ballot_inputs_be_hex[j]:f.batches[i].state_inputs_be_hex;
  checkEquationTrace(curve[2*j],curve[2*j+1],vk,proof,inputs);
 }
 const hashes=b.precompile_calls.filter(x=>x.address.endsWith('0002'));
 assert.equal(hashes.length,7);assert.equal(hashes.at(-1).return_hex,`0x${f.batches[i].root_hex}`);
 assert.deepEqual(b.checkpoint.state,f.batches[i].new_state.flatMap(x=>[`0x${x.c0_hex}`,`0x${x.c1_hex}`]));
}
assert.deepEqual(r.accepted_batches[0].checkpoint.state,f.batches[1].old_state.flatMap(x=>[`0x${x.c0_hex}`,`0x${x.c1_hex}`]));
assert.equal(r.negative_tests.length,13);
for(const n of r.negative_tests){assert.equal(n.receipt_status,0,n.case);assert.equal(n.atomic_rejection,true,n.case);assert.equal(n.exception,'revert');}
for(const[name,count]of[['wrong_ballot_proof',1],['wrong_state_proof_after_four_valid_ballots',5]]){
 const n=r.negative_tests.find(x=>x.case===name),pairs=n.precompile_calls.filter(x=>x.address.endsWith('000f'));
 assert.equal(pairs.length,count);assert.ok(n.precompile_calls.every(x=>x.success));
 assert.ok(pairs.slice(0,-1).every(x=>BigInt(x.return_hex)===1n));assert.equal(BigInt(pairs.at(-1).return_hex),0n);
}
assert.equal(r.freeze.receipt_status,1);assert.equal(r.final_checkpoint.frozen,true);
assert.equal(r.final_checkpoint.freeze_digest,`0x${f.expected_freeze_digest_hex}`);
assert.equal(r.final_checkpoint.cp_target,`0x${f.public_context.cp_target_hex}`);
assert.equal(r.final_checkpoint.spent_count,8);assert.equal(r.final_checkpoint.next_batch_index,3);
assert.ok(r.final_checkpoint.spent_nullifiers.every(Boolean));
const txs=[r.deployment,...r.accepted_batches,...r.negative_tests,r.freeze].sort((a,b)=>a.nonce-b.nonce);
assert.equal(txs.length,r.transaction_count);assert.deepEqual(txs.map(x=>x.nonce),Array.from({length:17},(_,i)=>i));
for(const t of txs){assert.equal(bytes(t.calldata_hex).length,t.calldata_bytes);assert.equal(hash(bytes(t.calldata_hex)),t.calldata_sha256);
 assert.equal(t.total_gas,Math.max(t.intrinsic_gas+t.execution_gas,t.minimum_gas_limit_including_calldata_floor));assert.ok(t.total_gas<20_000_000);}
for(const flag of ['complete_g8_settlement','trustee_authentication_checked_on_evm','initialization_certificate_checked_on_evm','p3_verified_in_this_flow','winner_written','snarkpack_evm_verified'])assert.equal(r[flag],false);
assert.equal(r.full_system_gas,null);
assert.equal(sha(r.contract_artifact),r.contract_artifact_sha256);
const a=read(r.contract_artifact);
for(const c of Object.values(a.contracts)){assert.equal(hash(bytes(c.runtime_bytecode_hex)),c.runtime_sha256);assert.ok(c.runtime_bytecode_bytes<=24576);}
// Preserve old raw/data/figures. Central aggregate summaries are intentionally regenerated.
let archives=0;
for(const gate of ['g1_e','g2_d','g2_e','g4_d','g6','g7'])for(const line of fs.readFileSync(`experiments/${gate}_raw/SHA256SUMS`,'utf8').trim().split('\n')){
 const[,h,p]=line.match(/^([0-9a-f]{64})\s+(.+)$/)??[];assert.ok(p);
 if(!/^(experiments|fixtures|results)\//.test(p)||p.startsWith('experiments/aggregate/'))continue;
 assert.equal(sha(p),h,p);archives++;
}
const g7=read('experiments/g7_raw/g7_p3_prague_evm.json');
for(const p of g7.source_hashes){const actual=['crates/vv-friendly-dkg/Cargo.toml','contracts/package.json'].includes(p.path)
 ?hash(execFileSync('git',['show',`9baed479:${p.path}`])):sha(p.path);assert.equal(actual,p.sha256,p.path);}
const commands=read('experiments/g8_raw/commands.json').commands,latest=Object.fromEntries(commands.map(x=>[x.name,x]));
for(const name of ['fmt-check','clippy','tests','state-tests','build','public-export','evm'])assert.equal(latest[name].exit_code,0,name);
console.log(JSON.stringify({status:'PASS_G8_PHASE1_REAL_RECEIVER_FREEZE_TRACE_AND_ARCHIVE_AUDIT',historical_artifact_entries_unchanged:archives,
 actual_positive_pairing_equations_bound_to_exact_archived_inputs:10,atomic_rejections:13,real_signed_transactions:17,
 earlier_failed_attempts_preserved:commands.filter(x=>x.exit_code!==0).length,complete_g8:false,snarkpack_evm:false}));
