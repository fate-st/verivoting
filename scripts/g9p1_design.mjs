// P1: source-backed design inventory + two actual JS transcript executions.
// NOT a complete SnarkPack verifier, EVM experiment or feasibility proof.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {traceSnarkpackB2} from '../contracts/scripts/snarkpack_transcript_g5c2.mjs';
process.chdir(path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..'));
const mode=process.argv[2]??'--check';assert.ok(['--write','--check'].includes(mode));
const read=p=>fs.readFileSync(p),load=p=>JSON.parse(read(p));
const sha=b=>createHash('sha256').update(b).digest('hex');
const prior=load('results/g9/route_review.json');
const fixturePath='fixtures/snarkpack_g1d0_depth5_public_v1.json',fixture=load(fixturePath);
const b2=fixture.cases.find(c=>c.batch_size===2);assert.ok(b2);
const proof=Buffer.from(b2.aggregate_proof_hex,'hex');assert.equal(sha(proof),b2.aggregate_proof_sha256);
assert.equal(proof.length,5236);assert.equal(b2.public_inputs_le_hex.length,2);assert.equal(b2.public_inputs_le_hex[0].length,25);
// D0 archived a negative only for its largest pilot batch; B2.negative is null.
// Derive a fresh canonical one-field mutation without changing the source file.
const changedRows=structuredClone(b2.public_inputs_le_hex);
const scalarModulus=0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001n;
const field=BigInt('0x'+Buffer.from(changedRows[1][7],'hex').reverse().toString('hex'));
assert.ok(field+1n<scalarModulus);
changedRows[1][7]=Buffer.from((field+1n).toString(16).padStart(64,'0'),'hex').reverse().toString('hex');
const negative={public_inputs_le_hex:changedRows};
const changes=[];b2.public_inputs_le_hex.forEach((row,i)=>row.forEach((v,j)=>{if(v!==negative.public_inputs_le_hex[i][j])changes.push([i,j]);}));
assert.deepEqual(changes,[[1,7]]);
// Rebuild external bytes, do not trust the archived transcript string.
const transcript=rows=>{const n=Buffer.alloc(4);n.writeUInt32BE(rows.length);return Buffer.concat([Buffer.from('VV/JUBJUB/G1-C0/SNARKPACK-PUBLIC-INPUTS/v1'),n,...rows.flatMap(row=>{const w=Buffer.alloc(4);w.writeUInt32BE(row.length);return [w,...row.map(h=>Buffer.from(h,'hex'))];})]);};
assert.equal(transcript(b2.public_inputs_le_hex).toString('hex'),b2.external_transcript_hex);
negative.external_transcript_hex=transcript(negative.public_inputs_le_hex).toString('hex');
const positive=traceSnarkpackB2(b2.aggregate_proof_hex,b2.external_transcript_hex);
const changed=traceSnarkpackB2(b2.aggregate_proof_hex,negative.external_transcript_hex);
assert.equal(positive.length,6);assert.equal(changed.length,6);
const changedChallenges=positive.filter((v,i)=>v.challenge_le_hex!==changed[i].challenge_le_hex).map(v=>v.id);
assert.deepEqual(changedChallenges,['random-r','gipa-0-seed','gipa-0-round','extra-link','random-z']);
const registry=path.join(process.env.CARGO_HOME??path.join(os.homedir(),'.cargo'),'registry/src');
const packageDir=fs.readdirSync(registry).map(d=>path.join(registry,d,'bellperson-0.27.0')).find(d=>fs.existsSync(path.join(d,'src/groth16/aggregate/verify.rs')));assert.ok(packageDir);
const reference={};
for(const name of ['verify.rs','proof.rs','transcript.rs','accumulator.rs']){const b=read(path.join(packageDir,'src/groth16/aggregate',name));reference[name]={sha256:sha(b),text:b.toString()};}
function anchor(file,text){const source=reference[file].text;const i=source.indexOf(text);assert.ok(i>=0,text);return {file:'bellperson-0.27.0/src/groth16/aggregate/'+file,line:source.slice(0,i).split('\n').length,anchor:text};}
// Explicit elementary equations; do not inherit a deterministic native RNG.
const equations=[
 {id:'KZG_V_ALPHA',pairs:2,target:'identity',source:anchor('verify.rs','let _check1 = kzg_check_v')},
 {id:'KZG_V_BETA',pairs:2,target:'identity',source:anchor('verify.rs','let _check2 = kzg_check_v')},
 {id:'KZG_W_ALPHA',pairs:2,target:'identity',source:anchor('verify.rs','let _check1 = kzg_check_w')},
 {id:'KZG_W_BETA',pairs:2,target:'identity',source:anchor('verify.rs','let _check2 = kzg_check_w')},
 {id:'TIPP_Z',pairs:1,target:'final_zab',source:anchor('verify.rs','let _check_z =')},
 {id:'TIPP_T',pairs:2,target:'final_tab',source:anchor('verify.rs','let _check_ab0 =')},
 {id:'TIPP_U',pairs:2,target:'final_uab',source:anchor('verify.rs','let _check_ab1 =')},
 {id:'MIPP_T',pairs:1,target:'final_tc',source:anchor('verify.rs','let _check_t =')},
 {id:'MIPP_U',pairs:1,target:'final_uc',source:anchor('verify.rs','let _check_u =')},
 {id:'GROTH16_AGGREGATE',pairs:3,target:'ip_ab',source:anchor('verify.rs','pairing_checks_copy.merge_nonrandom(')}];
const scalarCheck=anchor('verify.rs','let b = z == final_res.zc;');
// Source uses Gt::generator as a SPECIAL neutral sentinel in the accumulator.
// It must not be interpreted as a literal e(G1.generator,G2.generator) RHS.
const sentinel=anchor('accumulator.rs','fn add_if_not_one_gt');
assert.ok(reference['accumulator.rs'].text.includes('let one = E::Gt::generator();'));
const steps=[
 {id:'PARSE',operation:'bind exact 5236-byte wire; validate nine G1/five G2 compressed points and fifteen compressed GT values, subgroup and canonical roundtrips',evm:'checked expansion witnesses + MSM subgroup checks; GT torus relation and q-power checks; no trusted host result',gas_upper_bound:null},
 {id:'FS',operation:'original V2 six SHA-256 challenges; original bincode GT serialization and 64-bit cumulative rejection counters',evm:'byte-identical transcript; bounded continuation of rejection loop, never replace by reduction',gas_upper_bound:null},
 {id:'GIPA',operation:'one B2 round: ten GT scalar exponentiations, two G1 terms and five folded GT targets',evm:'persist computed exponentiation registers/bit cursors; every limb canonical; host intermediate values not accepted as outputs',gas_upper_bound:null},
 {id:'KZG_V_W',operation:'four equations, two pairing terms each; original r-inverse shift and product-form polynomials',evm:'four independent EIP-2537 identity pairing checks after checked MSM construction',precompile_pairing_only_formula_gas:4*(37700+32600*2),gas_upper_bound:null},
 {id:'TIPP_MIPP_AGGREGATE',operation:'six arbitrary-GT target equations with ten total pairing terms; separate MIPP scalar/group equality',evm:'software multi-Miller products + final exponentiation + exact GT comparison; native pairing-check returns no GT target',gas_upper_bound:null},
 {id:'FINAL',operation:'require all ten elementary pairing equations plus MIPP equality; no probabilistic accumulator RNG on chain',evm:'candidate is direct conjunction; equivalence/soundness argument including native sentinel behavior required before adoption',gas_upper_bound:null}];
const report={schema:'VV/G9-P1/DESIGN/v1',status:'BLOCKED_P1_RESOURCE_AND_EQUIVALENCE_GATES',baseline_commit:'a5204043bdcec5a788f79de2ad79225320845e64',
 source_fixture:{path:fixturePath,sha256:sha(read(fixturePath)),aggregate_sha256:sha(proof),B:2,public_input_fields:50,wire_bytes:proof.length,matrix_bytes:1600,external_transcript_bytes:transcript(b2.public_inputs_le_hex).length},
 reference_sources:Object.fromEntries(Object.entries(reference).map(([k,v])=>[k,{sha256:v.sha256}])),
 unchanged_prior_review_sha256:sha(read('results/g9/route_review.json')),
 actual_javascript_execution:{cases:2,positive_trace:positive,changed_nullifier_trace:changed,changed_indices:changes,changed_challenges:changedChallenges,full_native_verifier_executed:false,full_snarkpack_relation_verified:false,evm_executed:false},
 static_inventory:{gipa_rounds:1,wire_gt_elements:15,wire_g1_points:9,wire_g2_points:5,gt_expansion_bytes:15*12*64,pairing_equations:equations,logical_pairing_terms:equations.reduce((a,e)=>a+e.pairs,0),mipp_nonpairing_equality:scalarCheck,accumulator_neutral_sentinel:sentinel},
 stages:steps,
 resource_budget:{per_transaction_gas_cap:20000000,block_context_gas_cap:30000000,source:'inherited local research caps, not public-chain limits',total_transaction_cap:null,total_gas_upper_bound:null,total_runtime_cap_ms:null,gas_measured:null,known_pairing_precompile_subtotal_formula_only:411600,formula_source:'https://eips.ethereum.org/EIPS/eip-2537#pairing-check-operation',formula_excludes:'all calldata, storage, MSM, transcript, GT parsing/folding/Miller/final-exponentiation and transaction overhead',p2_launch_authorized:false},
 receiver:{scope:'new independent receiver, not old G8-D',active_sessions_per_election:1,phases:['BEGIN','ADVANCE','FINISH','CONSUME','ABORT'],accepted_nf_ct_unchanged_until_consume:true,consume_requires:'all checks, same envelope/root/old CT/context, original state proof and fresh nullifiers, one-time atomic consumption'},
 blockers:['GT parser subgroup checks and ten GIPA GT powers lack bounded EVM implementations','six arbitrary-GT equations and final exponentiation have no bounded continuation schedule','direct-conjunction handling must preserve intended equations and native accumulator sentinel semantics','total transaction/Gas/time caps cannot be filled from a 20M OOG receipt'],
 new_execution_counts:{javascript_transcript_cases:2,evm_transactions:0,compiler_runs:0,setup:0,prove:0},exact_snarkpack_evm_verified:false,complete_privacy_proof:false};
assert.equal(prior.exact_snarkpack_evm_verified,false);
assert.equal(report.static_inventory.logical_pairing_terms,18);
const output='results/g9_p1/design_inventory.json',bytes=JSON.stringify(report,null,2)+'\n';
if(mode==='--check'||fs.existsSync(output))assert.equal(read(output).toString(),bytes,output);else{fs.mkdirSync(path.dirname(output),{recursive:true});fs.writeFileSync(output,bytes,{flag:'wx'});}
console.log(JSON.stringify({status:report.status,mode,actual_js_cases:2,internal_challenges:12,original_equations:10,logical_pairing_terms:18,total_gas:null,p2_started:false,exact_evm:false}));
