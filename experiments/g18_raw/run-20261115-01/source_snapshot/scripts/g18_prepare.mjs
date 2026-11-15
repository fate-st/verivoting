// Freeze the user-defined G18 parameters once. No proving, installations or Git writes.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
const sha=b=>createHash('sha256').update(b).digest('hex');
const read=p=>fs.readFileSync(p),load=p=>JSON.parse(read(p));
const pin=p=>({path:p,sha256:sha(read(p)),bytes:read(p).length});
assert.equal(execFileSync('git',['branch','--show-current'],{encoding:'utf8'}).trim(),'experiment/g18-final-architecture-decision');
const baseline=execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim();
assert.equal(baseline,'151ed277d76ed0344c3fb19938319f22a179675c');
assert.ok(!fs.existsSync('configs/experiment.G18-DECISION.json'),'Config already frozen; never rewrite');
const pool=load('fixtures/snarkpack_g1d0_depth5_public_v1.json');
const state=load('fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json');
const evm=load('fixtures/g8_receiver_eip2537_v1.json');
assert.equal(pool.registry_count,32);assert.equal(pool.merkle_depth,5);
assert.equal(evm.batches.length,2);assert.equal(state.batches.length,2);
for(let k=0;k<2;k++){
 assert.deepEqual(state.batches[k].source_ballot_proofs_hex,pool.source_proofs_ark_compressed_hex.slice(k*4,k*4+4));
 assert.deepEqual(state.batches[k].source_public_inputs_le_hex,pool.source_public_inputs_le_hex.slice(k*4,k*4+4));
 for(let i=0;i<4;i++)for(let j=0;j<25;j++)assert.equal(
  Buffer.from(state.batches[k].source_public_inputs_le_hex[i][j],'hex').reverse().toString('hex'),
  evm.batches[k].ballot_inputs_be_hex[i][j].replace(/^0x/,''));
}
assert.deepEqual(evm.expected_final_state,state.final_state);
const registry=path.join(os.homedir(),'.cargo/registry');
const index='index.crates.io-1949cf8c6b5b557f';
const lock=read('experiments/zkvm_abc/Cargo.lock').toString();
const official=[];
for(const [pkg,file]of [['risc0-zkvm-3.1.0','examples/loop.bin'],['risc0-zkvm-3.1.0','examples/loop.s'],
 ['risc0-zkvm-3.1.0','examples/datasheet.rs'],['risc0-zkos-v1compat-2.2.3','elfs/v1compat.elf'],
 ['risc0-groth16-3.0.6','src/prove/docker.rs']]){
 const archive=path.join(registry,'cache',index,pkg+'.crate'),p=path.join(registry,'src',index,pkg,file);
 const digest=sha(read(archive));assert.ok(lock.includes(digest),'Official crate archive hash not in G17 lock');
 const bytes=execFileSync('tar',['-xOf',archive,pkg+'/'+file],{maxBuffer:8*1024*1024});
 assert.deepEqual(read(p),bytes,'Installed official source differs from locked archive');
 official.push({package:pkg,file,installed_path:p,sha256:sha(bytes),bytes:bytes.length,crate_archive_sha256:digest,
  crate_archive_matches_g17_lock:true,installed_file_matches_archive:true});
}
const inputs=['configs/protocol_manifest.JUBJUB-EVAL-D5-W32.json','configs/experiment.G8C-FROZEN-P3-W32.json',
 'fixtures/snarkpack_g1d0_depth5_public_v1.json','fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json',
 'fixtures/g8_receiver_eip2537_v1.json','fixtures/g8a_original_certificates_public_v1.json',
 'fixtures/g8b_trustee_commitments_public_v1.json','experiments/zkvm_abc/Cargo.lock',
 'experiments/zkvm_abc/guest_b_groth16/Cargo.lock','contracts/package-lock.json',
 'contracts/src/JubjubPointOps.sol','contracts/src/VeriVotingAuthenticatedDirectFoldSettlement.sol',
 'contracts/scripts/stage2_evm_fold_compare.mjs','crates/vv-friendly-crypto/src/lib.rs',
 'results/g17_z1_recovery/guest_execution.json',
 'experiments/zkvm_abc_raw/run-20261115-03/artifacts/guest-b.elf',
 'experiments/zkvm_abc_raw/run-20261115-03/artifacts/guest-b.bin'];
const j={schema:'VV/G18/FROZEN-DECISION-CONFIG/v1',frozen:true,baseline_commit:baseline,
 baseline_branch:'experiment/g17-zkvm-abc-comparison',branch:'experiment/g18-final-architecture-decision',
 target:{registry_depth:5,registered_voters:32,original_ballots:8,candidates:3,batch_sizes:[4,4],
  admission_batch_sizes:[1,2],functional_batch_sizes:[2,4],aggregation:'Original SnarkPack V2',
  ballot_curve:'BLS12-381',accumulation_curve:'Jubjub',public_inputs_per_ballot:25,
  original_ballot_vk_sha256:pool.groth16_vk_ark_compressed_sha256,ordered_batches:state.batches.map(b=>({
   batch_index:b.batch_index,batch_root:b.batch_root_hex,old_state:b.old_state,new_state:b.new_state,
   aggregate_sha256:b.source_aggregate_sha256})),final_state:state.final_state,
  authenticated_context_sha256:sha(Buffer.from(JSON.stringify(evm.public_context))),
  election_id:evm.public_context.election_id_hex,final_context:evm.public_context.final_context_hex,
  registration_root:evm.public_context.registration_root_hex,public_key:evm.public_context.public_key_hex,
  freeze_digest:evm.expected_freeze_digest_hex,cp_target:evm.public_context.cp_target_hex},
 environment:{platform:os.platform(),arch:os.arch(),cpu:os.cpus()[0].model,
  physical_memory_bytes:os.totalmem(),os_release:os.release(),host_rust:'1.99.1',guest_rust:'1.97.0',
  sdk:'3.1.0',r0vm:'3.1.0',risc0_build:'3.0.7',risc0_groth16:'3.0.6',hardfork:'Prague',
  proving_backend:'Pinned official local macOS-arm64 r0vm; native/default hardware selection, no automatic fallback',
  remote_prover:false,hardware_switch_authorized:false,backend_switch_authorized:false,
  host_build_strip:'none (same known G17 host-link workaround)',cargo_jobs:2},
 budgets:{guest_seconds:600,guest_memory_mib:16384,proof_seconds:1800,proof_memory_mib:24576,
  build_seconds:600,build_memory_mib:16384,setup_or_probe_seconds:120,
  poll_ms:1000,enforcement:'Soft elapsed and maximum sampled single-process + aggregate process-tree RSS limits; not kernel-enforced',
  max_formal_implementation_per_route:1,max_explicit_engineering_repair_retest_per_route:1,
  no_cryptographic_failure_reclassified_as_environment_without_evidence:true},
 official_smoke:{program:'Official SDK datasheet loop.bin + original V1COMPAT_ELF',loop_count:0,
  input_hex:'00000000',receipt_kind:'Composite',expected_journal_hex:'',maximum_attempts:1,
  purpose:'One local proving-backend smoke even if wrapper preflight is unavailable; not a ballot/performance comparison',
  user_elf_sha256:official[0].sha256,kernel_elf_sha256:official[3].sha256,official_files:official},
 admission_order:['freeze','local backend and Groth16 wrapper preflight','one official proof smoke',
  'G17 B1 real receipt only if complete current-environment admission passes','A/B full semantics and function',
  'B2 then two B4 proofs + EVM','new C + fair comparison'],
 gating:{groth16_wrapper_required_for_environment_admission:true,
  stop_before_b1_ballot_proving_if_required_environment_missing:true,
  stop_before_snarkpack_development_if_phase1_fails:true,
  unavailable_subsequent_metrics:null,stop_on_unclosed_crypto_semantics:true},
 decision:{gas_advantage_minimum_fraction:0.15,gas_scope:'Matched full deployment/authentication/two batches/freeze; not verifier-only',
  allowed:['SELECT_A_SNARKPACK_ZKVM','SELECT_B_DIRECT_ZKVM','KEEP_C_AND_REDESIGN','INCONCLUSIVE_RESOURCE_OR_ENVIRONMENT'],
  environment_failure_decision:'INCONCLUSIVE_RESOURCE_OR_ENVIRONMENT',no_unmeasured_architecture_ranking:true},
 official_evm_wrapper:{version:'3.1.0',selector:'0xe71e6620',setup:'2026-10-20.1',
  docker_image_reference:'risczero/risc0-groth16-prover:v2026-10-20.1',
  image_digest_if_not_pulled:null,deployed_verifier_if_not_executed:null},
 official_sources:['https://dev.risczero.com/api/generating-proofs/local-proving',
  'https://github.com/risc0/risc0/releases/tag/v3.1.0'],
 input_pins:inputs.map(pin),historical_c_reference:pin('experiments/stage2_evm_fold_raw/run-20261114-01/direct_fold.json'),
 preservation:{old_tracked_files_immutable:true,old_paper_immutable:true,root_manifests_and_locks_immutable:true,
  no_automatic_commit:true,no_automatic_push:true,no_g19:true,no_automatic_config_rewrite:true}};
assert.equal(j.environment.arch,'arm64');assert.equal(j.environment.platform,'darwin');
fs.writeFileSync('configs/experiment.G18-DECISION.json',JSON.stringify(j,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({status:'FROZEN_G18_CONFIG',...pin('configs/experiment.G18-DECISION.json')},null,2));
