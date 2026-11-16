// Admission-only pilot. No simplified SnarkMux and no placeholder verifier.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync,spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
process.chdir(root);
const args=process.argv.slice(2);
const opt=k=>{const i=args.indexOf(k);return i<0?null:args[i+1];};
const out=path.resolve(opt('--output')??'experiments/snarkmux_pilot'),paper=opt('--paper');
assert.ok(paper&&fs.existsSync(paper),'provide --paper PATH to the downloaded SnarkMux PDF');
assert.ok(!fs.existsSync(path.join(out,'execution.json')),'never overwrite a previous run');
const sha=b=>createHash('sha256').update(b).digest('hex');
const git=(...a)=>execFileSync('git',a,{encoding:'utf8'}).trim();
const before=git('ls-files','-z').split('\0').filter(Boolean).map(p=>({path:p,sha256:sha(fs.readFileSync(p))}));
fs.mkdirSync(path.join(out,'evidence'),{recursive:true});fs.mkdirSync(path.join(out,'logs'),{recursive:true});
const save=(p,v)=>fs.writeFileSync(path.join(out,p),Buffer.isBuffer(v)||typeof v==='string'?v:JSON.stringify(v,null,2)+'\n',{flag:'wx'});
const execution={started_utc:new Date().toISOString(),git_commit:git('rev-parse','HEAD'),branch:git('branch','--show-current'),initial_git_status:git('status','--short'),commands:[]};
const env={...process.env,CARGO_BUILD_JOBS:'2',RAYON_NUM_THREADS:'4'};
function command(label,program,argv,timeout=60000){
    const t=performance.now(),r=spawnSync(program,argv,{env,encoding:'utf8',timeout,maxBuffer:8*1024*1024});
    save('logs/'+label+'.stdout.log',r.stdout??'');save('logs/'+label+'.stderr.log',r.stderr??'');
    execution.commands.push({label,program,args:argv,exit_code:r.status,signal:r.signal,error:r.error?.message??null,elapsed_ms:performance.now()-t});return r;
}
let stage='input';
try{
    const pins=[['fixtures/snarkpack_g1d2_depth5_batch32_public_v1.json','6b9a1ca042e3ab1d637fb3b554029cc7a36a5b867430b2707539d1ea61928596'],['fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json','42e68fa9bee72fb77e47c292dd3e65386cf4a348192c1ee0a9731cfe2f5eff19'],['configs/protocol_manifest.JUBJUB-EVAL-D5-W32.json','00c3869c4b10a6e48fa5972b2a98767966db2b2fac0de3692eb8cbbc13cafe5e']];
    for(const[p,h]of pins)assert.equal(sha(fs.readFileSync(p)),h,p);
    const pool=JSON.parse(fs.readFileSync(pins[0][0])),f=JSON.parse(fs.readFileSync(pins[1][0])),b=f.batches[0];
    assert.equal(pool.registry_count,32);assert.equal(pool.merkle_depth,5);assert.equal(b.source_ballot_proofs_hex.length,4);
    assert.deepEqual(b.source_ballot_proofs_hex,pool.source_proofs_ark_compressed_hex.slice(0,4));
    assert.deepEqual(b.source_public_inputs_le_hex,pool.source_public_inputs_le_hex.slice(0,4));
    assert.deepEqual(b.source_public_records,pool.source_public_records.slice(0,4));
    assert.equal(f.source_ballot_vk_hex,pool.groth16_vk_hex);
    assert.equal(sha(Buffer.from(pool.groth16_vk_hex,'hex')),pool.groth16_vk_sha256);
    assert.ok(b.source_public_inputs_le_hex.every(r=>r.length===25));
    const bundle={vk_hex:pool.groth16_vk_hex,proofs_hex:pool.source_proofs_ark_compressed_hex.slice(0,5),public_inputs_le_hex:pool.source_public_inputs_le_hex.slice(0,5)};
    save('input_bundle.json',bundle);
    save('input_manifest.json',{schema:'VV/SNARKMUX-PILOT/INPUT/v1',git_commit:execution.git_commit,sources:pins.map(([p,h])=>({path:p,sha256:h})),registered_voters:32,ballots:4,control_ballots:1,candidates:3,public_inputs_per_ballot:25,
        vk_encoding:pool.groth16_vk_encoding,vk_sha256:pool.groth16_vk_sha256,vk_ark_compressed_sha256:pool.groth16_vk_ark_compressed_sha256,ballot_layout:pool.ballot_layout,
        ordered_public_records:b.source_public_records,ordered_public_inputs_le_hex:b.source_public_inputs_le_hex,
        proofs:bundle.proofs_hex.slice(0,4).map((v,i)=>({index:i,sha256:sha(Buffer.from(v,'hex')),bytes:Buffer.from(v,'hex').length})),
        batch_index:b.batch_index,batch_root_hex:b.batch_root_hex,old_state:b.old_state,new_state:b.new_state,
        control_scope:'fifth archived ballot is only a native replacement control, not aggregated',new_ballot_proofs:0});
    const manifest=path.join(root,'experiments/snarkmux_pilot/adapter/Cargo.toml');
    let r;
    if(!fs.existsSync(path.join(path.dirname(manifest),'Cargo.lock'))){r=command('adapter-lock','cargo',['+1.99.1','generate-lockfile','--offline','--manifest-path',manifest]);assert.equal(r.status,0,'isolated lock');}
    r=command('adapter-build','cargo',['+1.99.1','build','--release','--locked','--offline','--manifest-path',manifest,'--target-dir',path.join(root,'target')],300000);assert.equal(r.status,0,'adapter build');
    execution.rustc=execFileSync('rustc',['+1.99.1','-Vv'],{encoding:'utf8'}).trim();
    execution.cargo=execFileSync('cargo',['+1.99.1','-V'],{encoding:'utf8'}).trim();
    execution.binary_sha256=sha(fs.readFileSync(path.join(root,'target/release/snarkmux-input-check')));
    execution.source_files=['scripts/snarkmux_pilot.mjs','experiments/snarkmux_pilot/adapter/Cargo.toml','experiments/snarkmux_pilot/adapter/Cargo.lock','experiments/snarkmux_pilot/adapter/src/main.rs'].map(p=>({path:p,sha256:sha(fs.readFileSync(p))}));
    r=command('original-b4',path.join(root,'target/release/snarkmux-input-check'),[path.join(out,'input_bundle.json')],30000);assert.equal(r.status,0,'original B4 verification');
    const baseline=JSON.parse(r.stdout);assert.equal(baseline.original_valid_count,4);save('input_verification.json',baseline);
    stage='paper';const pdf=fs.readFileSync(paper);assert.equal(pdf.subarray(0,5).toString(),'%PDF-');save('evidence/snarkmux.pdf',pdf);
    const swift='import PDFKit; import Foundation; let d=PDFDocument(url:URL(fileURLWithPath:CommandLine.arguments[1]))!; print(d.page(at:0)!.string!); print("\\nPAGE 26\\n"); print(d.page(at:25)!.string!)';
    r=command('paper-extract','swift',['-module-cache-path','/private/tmp/vv-snarkmux-swift-cache','-e',swift,path.join(out,'evidence/snarkmux.pdf')],120000);assert.equal(r.status,0,'paper extraction');
    const url='https://anonymous.4open.science/r/SnarkMux';assert.ok(r.stdout.includes(url));assert.match(r.stdout,/BN254\/Grumpkin/);assert.match(r.stdout,/Spartan/);
    const paperSource={url:'https://eprint.iacr.org/2023/1946',download_url:'https://eprint.iacr.asia/2023/1946.pdf',file:'evidence/snarkmux.pdf',sha256:sha(pdf),section:'6.2 / printed page 26',prototype_url:url,
        reported_target_curve:'BN254',reported_recursion_cycle:'BN254/Grumpkin',reported_final_snark:'Spartan',official_mirror_byte_equality_confirmed:false,
        research_download_observation:{official_http:403,mirror_http:200},note:'mirror supplied to this run; text checked with PDFKit; no old SnarkFold implementation is assumed equivalent'};
    save('evidence/paper_source.json',paperSource);
    stage='source';const response=path.join(out,'evidence/prototype_response.txt'),headers=path.join(out,'evidence/prototype_headers.txt');
    r=command('prototype-access','curl',['--silent','--show-error','--location','--max-time','35','--dump-header',headers,'--output',response,'--write-out','%{http_code}\n%{url_effective}\n',url],45000);
    const[http,effective]=r.stdout.trim().split('\n');
    const audit={prototype_url:url,http_status:/^\d{3}$/.test(http)?Number(http):null,effective_url:effective??null,curl_exit_code:r.status,
        source_obtained:false,source_commit:null,prototype_build_attempted:false};save('evidence/source_access.json',audit);
    // A website response alone never admits unaudited code or a pretend importer.
    const reason=[401,403,404].includes(audit.http_status)?'Author-linked prototype source not publicly retrievable in this run':'No audited author-linked source/build/import path obtained; manual source admission required';
    save('compatibility.json',{schema:'VV/SNARKMUX-PILOT/COMPATIBILITY/v1',status:'BLOCKED_SOURCE',reason,
        original_inputs:{valid:4,total:4,curve:baseline.pairing_curve,public_inputs:25,one_vk:true},prototype:audit,paper_profile:paperSource,
        original_proofs_accepted_by_snarkmux:null,aggregate_gate_passed:false,
        curve_warning:'Paper evaluation uses BN254; original proofs use BLS12-381. Code-level portability is untested. Encoding cannot change field or curve semantics.'});
    const result={schema:'VV/SNARKMUX-PILOT/RESULTS/v1',status:'BLOCKED_SOURCE',stop_stage:1,reason,
        baseline:{valid:4,total:4,curve:baseline.pairing_curve,individual_verify_ms:baseline.four_individual_verify_ms,vk_prepare_ms:baseline.vk_prepare_ms,proof_bytes:baseline.checks.map(x=>x.proof_bytes),samples:1,
            binding_controls:baseline.native_binding_controls,controls_are_not_snarkmux_tests:true,evm_gas:null,deployment_gas:null},
        snarkmux:{aggregate_ms:null,aggregate_package_bytes:null,vprep_ms:null,online_verify_ms:null,peak_rss_mib:null,valid_aggregate_verified:null,modified_input_rejected:null,substituted_batch_rejected:null,evm_verified:null,evm_gas:null,deployment_gas:null,calldata_bytes:null},
        new_ballot_proofs:0,new_setups:0,new_dkg:0,new_p3:0,new_evm_transactions:0,stage2:'NOT_RUN_STAGE1_BLOCKED',stage3:'NOT_RUN_STAGE1_BLOCKED',replace_snarkpack:false,no_claim_of_mathematical_impossibility:true};
    save('results.json',result);
    save('report.md',`# SnarkMux Pilot\n\n**BLOCKED_SOURCE**. Stopped at stage 1.\n\nBase: \`${execution.git_commit}\`; branch: \`${execution.branch}\`.\n\n## Actual checks\n\nFirst original W32 B4; 32 registered voters, three candidates, four Groth16 proofs and 25 unchanged ordered inputs each under one VK. **4/4 verified**, BLS12-381. A fifth archived ballot is a valid replacement control only.\n\nModified second-ballot nullifier input, swapped input and a fifth proof under original third-ballot inputs are rejected by actual Groth16 verification. These are NOT SnarkMux aggregation negatives.\n\n## Source gate\n\n[Paper](${paperSource.url}), section 6.2, names [this prototype](${url}). Actual access: HTTP ${audit.http_status}; effective URL: \`${audit.effective_url}\`. No source commit, executable importer or complete verifier was obtained. No prototype build, SnarkMux aggregation or EVM deployment was attempted. This is an access/admission blocker, not proof that no implementation exists.\n\nThe inspected PDF reports BN254/Grumpkin recursion with Spartan final compression. Current ballots use BLS12-381. No lossless cross-curve conversion is claimed, and code-level portability remains untested. Official PDF access returned 403 during research; the mirror PDF and hash are retained without claiming official-byte equality.\n\n## Measurements\n\n| Metric | Original individual Groth16 | SnarkMux |\n|---|---:|---:|\n| Valid proofs | 4/4 | not run |\n| Native verify, ms | ${baseline.four_individual_verify_ms} | N/A |\n| VK prepare, ms (separate) | ${baseline.vk_prepare_ms} | N/A |\n| Compressed proof bytes | ${baseline.checks.map(x=>x.proof_bytes).join(', ')} | N/A |\n| Aggregate time / bytes | N/A | N/A |\n| VPrep / online verification | N/A | N/A |\n| New EVM verification / deployment Gas | not measured | not measured |\n\nOne native sample is admission evidence, not a statistical comparison. Old Stage 2 Gas is not a newly matched baseline. Unexecuted numerical fields are null in results.json.\n\n## Decision\n\nDo not replace SnarkPack. Do not build a hash-only aggregator, ordinary-Groth16 substitute or placeholder EVM verifier. Resume only after obtaining author-linked or author-confirmed equivalent source, pinning its commit and auditing complete BLS12-381 import/verification semantics. No new ballots, DKG, P3 or EVM transactions. No automatic commit or push.\n\n## Reproduction\n\n\`node scripts/snarkmux_pilot.mjs --paper /ABS/snarkmux.pdf --output /ABS/NEW_OUTPUT\`\n\nThe adapter is isolated at experiments/snarkmux_pilot/adapter. It verifies but never creates proofs. No original source, circuit, frozen config or contract was edited. Existing outputs are never overwritten.\n`);
    execution.status=result.status;
}catch(e){execution.status='FAIL_ADMISSION_EXECUTION';execution.failed_stage=stage;execution.error=String(e);process.exitCode=1;}
finally{
    const changed=before.filter(x=>!fs.existsSync(x.path)||sha(fs.readFileSync(x.path))!==x.sha256);
    execution.historical_tracked_files_checked=before.length;execution.historical_tracked_files_unchanged=changed.length===0;execution.changed_historical_files=changed;
    execution.finished_utc=new Date().toISOString();execution.environment={cpu:os.cpus()[0].model,cpus:os.cpus().length,ram_bytes:os.totalmem(),platform:os.platform(),arch:os.arch(),kernel:os.release(),node:process.version};
    save('historical_files.json',before);save('execution.json',execution);if(changed.length)process.exitCode=1;
    const files=[];function walk(d){for(const e of fs.readdirSync(d,{withFileTypes:true})){const p=path.join(d,e.name);if(e.isDirectory())walk(p);else if(e.name!=='SHA256SUMS')files.push(p);}}
    walk(out);save('SHA256SUMS',files.sort().map(p=>sha(fs.readFileSync(p))+'  '+path.relative(out,p)).join('\n')+'\n');
    console.log(JSON.stringify({status:execution.status,output:out,old_files_unchanged:changed.length===0,error:execution.error??null}));
}
