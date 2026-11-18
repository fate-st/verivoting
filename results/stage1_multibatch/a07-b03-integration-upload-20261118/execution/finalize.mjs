import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import cp from 'node:child_process';
import assert from 'node:assert/strict';
const task=path.dirname(new URL(import.meta.url).pathname),w=task+'/worktree';
const a='experiments/stage1_multibatch_raw/gate-a-20261117-07',b='experiments/stage1_multibatch_raw/gate-b-20261117-03',out='results/stage1_multibatch/gate-b-20261117-03';
const load=p=>JSON.parse(fs.readFileSync(p)),sha=x=>crypto.createHash('sha256').update(x).digest('hex');
const before=load(task+'/before.json'),after=load(task+'/after.json'),baseline=load(w+'/'+a+'/gate_a.json'),audit=load(w+'/'+out+'/reuse_audit.json');
assert.equal(after.status,'PASS_ORIGINAL_WORKSPACE_AND_16_PATHS_UNCHANGED');assert.equal(audit.status,'PASS_GATE_A07_AND_REUSE_INTEGRATION');
for(const p of baseline.source_pins)assert.equal(sha(fs.readFileSync(w+'/'+p.path)),p.sha256,p.path);
for(const p of before.manifests)assert.equal(sha(fs.readFileSync(w+'/'+p.path)),p.sha256,p.path);
const summary=load(w+'/'+b+'/materials/prepare_summary.json'),r=summary.range;
const git=(...args)=>cp.execFileSync('git',['--no-optional-locks',...args],{cwd:w,encoding:'utf8'});
const stages={ac1:load(task+'/ac1.json'),ac2:load(task+'/ac2.json'),ac3:load(task+'/ac3.execution.json'),a07:load(task+'/a07.execution.json'),a07_audit:load(task+'/a07-audit.execution.json'),b03:load(task+'/b03.execution.json'),b03_audit:load(task+'/b03-audit.execution.json')};
for(const p of [stages.ac3,stages.a07,stages.a07_audit,stages.b03,stages.b03_audit])assert.equal(p.code,0);
assert.equal(git('diff','--','Cargo.toml','Cargo.lock','crates/vv-friendly-dkg/Cargo.toml'),'');
const report={status:audit.status,finished_at:new Date().toISOString(),worktree:w,branch:git('branch','--show-current').trim(),base_head:git('rev-parse','HEAD').trim(),
 modified_source_files:['crates/vv-friendly-dkg/src/bin/stage1_multibatch_materials.rs','contracts/scripts/stage1_multibatch_compare.mjs','scripts/stage1_multibatch_audit.mjs','docs/STAGE1_MULTIBATCH.md'],
 source_pins:baseline.source_pins,build_fingerprint:baseline.fingerprint,
 system:{sw_vers:cp.execFileSync('sw_vers',{encoding:'utf8'}),uname:cp.execFileSync('uname',['-a'],{encoding:'utf8'})},stages,
 measured:{setup_generation_count:0,setup_validation_ms:summary.setup_reuse.reuse_validation_ms,setup_worker_pid:summary.commands[0].pid,range_worker_pid:r.pid,pk_load_count:r.pk_load_count,
  pk_once_load_total_ms:r.pk_once_load_total_ms,pk_sha256_ms:r.pk_sha256_ms,pk_deserialize_ms:r.pk_deserialize_ms,vk_ark_hash_ms:r.vk_ark_hash_ms,
  worker_total_ms:r.worker_total_ms,worker_peak_rss_mib:r.worker_peak_rss_mib,materials_wall_ms:audit.materials_wall_ms,
  batches:summary.batches.map((x,i)=>({batch_index:x.batch_index,witness_ms:x.witness_ms,prove_ms:x.prove_ms,verify_ms:x.verify_ms,
   batch_pk_deserialize_ms:x.batch_pk_deserialize_ms,instant_rss_mib:r[`batch_${x.batch_index}_instant_rss_mib`],cumulative_peak_rss_mib:r.rss_sequence[i].process_peak_rss_mib,
   ark_bytes:x.proof_ark_bytes,eip_bytes:x.proof_eip_bytes,ark_sha256:x.proof_ark_sha256,eip_sha256:x.proof_eip_sha256,public_inputs:x.eip.public_input_count,worker_pid:x.worker_pid}))},
 original_workspace_integrity:after,limits:{historical_setup_reused:true,historical_seed_request_reused:true,seed_is_research_not_production_randomness:true,
  source_pins_versioned_in_A07_not_overwritten_in_A06:true,instant_rss_not_required_to_decrease:true,eight_batch_memory_bound_proven:false,
  schema_test_operational_fields_are_synthetic:true,vm_creation_count:0,evm_transaction_count:0,evm_gas:null,gate_c:'NOT_AUTHORIZED_NOT_STARTED',gate_d:'NOT_STARTED',git_commit_count:0,git_push_count:0}};
function save(p,data){fs.writeFileSync(w+'/'+p,data,{flag:'wx'});}
save(out+'/integration_report.json',JSON.stringify(report,null,2)+'\n');
save(out+'/integrity_before.json',fs.readFileSync(task+'/before.json'));
save(out+'/integrity_after.json',fs.readFileSync(task+'/after.json'));
save(out+'/source_changes.diff',git('diff','--binary','HEAD','--',...report.modified_source_files));
for(const name of ['ac1.json','ac2.json','stage1-build.json','stage1-build.stdout.log','stage1-build.stderr.log','ac3.stdout.log','ac3.stderr.log','ac3.execution.json'])save(out+'/'+name,fs.readFileSync(task+'/'+name));
const f=x=>Number(x).toFixed(6),m=report.measured;
const md=`# Stage1 A-07 / B-03 PK-Reuse integration\n\nStatus: **${report.status}**. No Setup generation, EVM, Gate C, commit or push.\n\n`+
`Worktree: ${w}\n\nBranch: ${report.branch}; base HEAD: ${report.base_head}. Original workspace HEAD/status/index and all 16 protected paths remain unchanged (14 present, 2 absent).\n\n`+
`## Execution and results\n\nStage 1 compiled the changed Rust source in ${f(load(task+'/stage1-build.json').wall_ms/1000)} s. A-07 separately ran cargo build against that new-code cache; it was a cached build check, not a second fresh compile. Mach RSS API compiled and ran.\n\n`+
`Frozen Setup validation worker PID ${m.setup_worker_pid}; one range proof worker PID ${m.range_worker_pid}. Actual Setup generation count = 0. PK loads = ${m.pk_load_count}.\n\n`+
`Once-only PK load ${f(m.pk_once_load_total_ms/1000)} s; deserialize ${f(m.pk_deserialize_ms/1000)} s; range total ${f(m.worker_total_ms/1000)} s; materials command wall ${f(m.materials_wall_ms/1000)} s.\n\n`+
`| Batch | Witness ms | Prove ms | Verify ms | Instant RSS after Drop MiB | Cumulative peak MiB | PK deserialize ms |\n|---|---:|---:|---:|---:|---:|---:|\n`+
 m.batches.map(x=>`| ${x.batch_index} | ${f(x.witness_ms)} | ${f(x.prove_ms)} | ${f(x.verify_ms)} | ${f(x.instant_rss_mib)} | ${f(x.cumulative_peak_rss_mib)} | ${x.batch_pk_deserialize_ms} |`).join('\n')+
`\n\nLifetime ru_maxrss: ${m.worker_peak_rss_mib} MiB. Instant resident_size and cumulative ru_maxrss are distinct; no RSS decrease is required and two batches do not establish an eight-batch bound.\n\n`+
`## Fixed hashes and provenance\n\n`+Object.entries(baseline.fingerprint).filter(([k])=>k.endsWith('sha256')).map(([k,v])=>`- ${k}: \`${v}\``).join('\n')+'\n\n'+m.batches.map(x=>`- Batch ${x.batch_index} Ark (${x.ark_bytes} B): \`${x.ark_sha256}\`; EIP (${x.eip_bytes} B): \`${x.eip_sha256}\`.`).join('\n')+
`\n\nBoth proofs match B-02 bytes exactly; all 37 inputs, roots and states match; Batch 1 new state equals Batch 2 old state. Auditor schema positive and three precise negatives passed (synthetic operational schema fixtures only, not performance samples).\n\n`+
`B-03 deliberately reuses the exact historical request bytes ONLY as a deterministic research seed input. Actual run identity is in integration_request.json and command receipts; old request PID/time are NOT new execution metadata. Frozen config remains byte-identical; new source/binary pins and execution budgets are in A-07 integration_config.json.\n\n`+
`## Stops and limitations\n\nNo new Setup: setup-worker is explicitly a frozen-material validation/copy worker. Only two fresh state proofs were generated via the official Runner → prepare → prove-range path. B-03 is NOT a rerun of the nine-VM B regression. No Gas or eight-batch result exists. Gate C is explicitly blocked pending separate admission/authorization; old A-06/B-02 archives were not re-pinned. PK remains local via a symlink, so this archive alone is not a standalone reproduction package. No commit or push.\n`;
save(out+'/REPORT.md',md);
const files=[];function walk(p){for(const e of fs.readdirSync(w+'/'+p,{withFileTypes:true})){const rel=p+'/'+e.name;if(e.isSymbolicLink()||e.name==='build-target')continue;if(e.isDirectory())walk(rel);else if(e.isFile())files.push(rel);}}
walk(a);walk('results/stage1_multibatch/gate-a-20261117-07');walk(b);walk(out);
save(out+'/SHA256SUMS',files.sort().map(p=>sha(fs.readFileSync(w+'/'+p))+'  '+p).join('\n')+'\n');
console.log(JSON.stringify({status:report.status,report:w+'/'+out+'/REPORT.md',checksum_entries:files.length,metrics:m},null,2));
