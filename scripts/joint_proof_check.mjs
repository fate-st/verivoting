// Read-only J1 evidence checker. Does not compile or construct/prove a circuit.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
const sha=b=>createHash('sha256').update(b).digest('hex');
export function check(run){
 const dir=path.resolve(run),read=name=>JSON.parse(fs.readFileSync(path.join(dir,name+'.json')));
 const m=read('execution'),native=read('native_fq_control'),fixture=read('public_preflight'),d=read('outer_fr_compile_diagnostics');
 assert.equal(m.status,'BLOCKED_INNER_RECURSIVE_VERIFIER');assert.equal(m.commands.length,9);
 assert.ok(m.commands.slice(0,-1).every(c=>c.code===0&&!c.termination));
 const last=m.commands.at(-1);assert.equal(last.name,'current-outer-fr-compile');assert.equal(last.code,101);assert.equal(last.expected_exit_code,101);assert.equal(last.termination,null);
 assert.ok(d.errors.some(e=>e.code?.code==='E0271'&&e.message.includes('FqConfig')&&e.message.includes('FrConfig')));
 assert.equal(native.status,'PASS_NATIVE_FQ_COMPILE_ONLY');assert.equal(native.native_gadget_constraint_field_bits,381);assert.equal(native.current_outer_constraint_field_bits,255);
 assert.notEqual(native.native_fq_modulus,native.current_outer_fr_modulus);assert.equal(native.r1cs_synthesized,false);
 assert.equal(fixture.status,'PASS_PUBLIC_FIXTURE_PREFLIGHT_ONLY');assert.equal(fixture.native_fixture_preflight_verified,true);
 assert.equal(fixture.negative_a_export_matches,true);assert.equal(fixture.public_input_count,25);assert.equal(fixture.ark_proof_bytes,192);assert.equal(fixture.eip2537_proof_bytes,512);
 assert.equal(fixture.in_circuit_verification_executed,false);assert.equal(fixture.host_bool_allocated_as_witness,false);assert.equal(fixture.private_voter_witness_read,false);
 for(const key of ['setup_count','proof_generation_count','new_dkg_count','new_p3_count'])assert.equal(m[key],0);
 assert.equal(m.constraint_count,null);assert.equal(m.recursive_verification_executed,false);assert.equal(m.j2_j5_started,false);
 assert.equal(m.recursive_crypto_negative_tests.length,5);assert.ok(m.recursive_crypto_negative_tests.every(t=>t.status==='NOT_RUN'));
 assert.equal(m.old_tracked_files_unchanged,true);
 const sources=read('source_files');
 assert.equal(sha(Buffer.from(JSON.stringify(sources))),m.source_version_sha256);
 for(const e of sources){assert.equal(sha(fs.readFileSync(path.join(dir,'source_snapshot',e.path))),e.sha256);assert.equal(sha(fs.readFileSync(e.path)),e.sha256,'current source '+e.path);}
 for(const e of read('config').source_pins)assert.equal(sha(fs.readFileSync(e.path)),e.sha256,e.path);
 for(const e of read('dependency_evidence').evidence)assert.equal(sha(fs.readFileSync(path.join(dir,e.archive_path))),e.sha256);
 const rolling=new Set(['docs/NEXT_STEPS.md','docs/IMPLEMENTATION_STATUS.md']);
 let oldCount=0;for(const e of read('historical_files'))if(!rolling.has(e.path)){assert.equal(sha(fs.readFileSync(e.path)),e.sha256,'old artifact '+e.path);oldCount++;}
 const sums=path.join(dir,'SHA256SUMS');
 if(fs.existsSync(sums))for(const line of fs.readFileSync(sums,'utf8').trimEnd().split('\n')){const v=/^([a-f0-9]{64})  (.+)$/.exec(line);assert.ok(v);const file=path.resolve(dir,v[2]);assert.ok(file.startsWith(dir+path.sep));assert.equal(sha(fs.readFileSync(file)),v[1],v[2]);}
 return {audit:'PASS_HONEST_J1_BLOCKED_EVIDENCE',j1_status:m.status,constraint_count:null,
  recursive_verification_executed:false,original_fixture_checked:true,old_artifacts_checked:oldCount,source_version_sha256:m.source_version_sha256};
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 assert.ok(process.argv[2],'usage: node scripts/joint_proof_check.mjs J1_RUN_DIR');console.log(JSON.stringify(check(process.argv[2]),null,2));
}
