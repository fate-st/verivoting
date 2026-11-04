// Additive P1 review: source/algebra and finite register layout, NOT an EVM verifier.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {execFileSync} from 'node:child_process';
const require=createRequire(new URL('../contracts/package.json',import.meta.url));
const {bls12_381:bls}=await import(require.resolve('@noble/curves/bls12-381.js'));
const mode=process.argv[2]??'--check';assert.ok(['--write','--check'].includes(mode));
const sha=b=>createHash('sha256').update(b).digest('hex');
const previousPath='results/g9_p1/design_inventory.json',previousBytes=fs.readFileSync(previousPath),prior=JSON.parse(previousBytes);
assert.equal(sha(previousBytes),sha(execFileSync('git',['show','7475f56:'+previousPath],{maxBuffer:1024*1024})));
const registry=path.join(process.env.CARGO_HOME??path.join(os.homedir(),'.cargo'),'registry/src');
const roots=fs.readdirSync(registry).map(d=>path.join(registry,d));
const pkg=name=>roots.map(d=>path.join(d,name)).find(d=>fs.existsSync(d));
const files={verify:path.join(pkg('bellperson-0.27.0'),'src/groth16/aggregate/verify.rs'),prove:path.join(pkg('bellperson-0.27.0'),'src/groth16/aggregate/prove.rs'),accumulator:path.join(pkg('bellperson-0.27.0'),'src/groth16/aggregate/accumulator.rs'),gt:path.join(pkg('blstrs-0.7.0'),'src/gt.rs')};
const sources=Object.fromEntries(Object.entries(files).map(([k,p])=>[k,{path:p,bytes:fs.readFileSync(p)}]));
const anchor=(key,s)=>{const text=sources[key].bytes.toString(),offset=text.indexOf(s);assert.ok(offset>=0,s);return {source:key,line:text.slice(0,offset).split('\n').length,anchor:s};};
const base=prior.static_inventory.pairing_equations.map(x=>({id:x.id,pairs:x.pairs}));
const formulas={
 KZG_V_ALPHA:'e(-g,V_alpha-h*f_v(z))*e(g_alpha-g*z,pi_v_alpha)=1',
 KZG_V_BETA:'e(-g,V_beta-h*f_v(z))*e(g_beta-g*z,pi_v_beta)=1',
 KZG_W_ALPHA:'e(W_alpha-g*f_w(z),-h)*e(pi_w_alpha,h_alpha-h*z)=1',
 KZG_W_BETA:'e(W_beta-g*f_w(z),-h)*e(pi_w_beta,h_beta-h*z)=1',
 TIPP_Z:'e(A,B)=ip_ab*Zab_L^c*Zab_R^a',
 TIPP_T:'e(A,V_alpha)*e(W_alpha,B)=Tab*Tab_L^c*Tab_R^a',
 TIPP_U:'e(A,V_beta)*e(W_beta,B)=Uab*Uab_L^c*Uab_R^a',
 MIPP_T:'e(C,V_alpha)=Tc*Tc_L^c*Tc_R^a',
 MIPP_U:'e(C,V_beta)=Uc*Uc_L^c*Uc_R^a',
 GROTH16_AGGREGATE:'e(alpha*(1+r),beta)*e(IC0*(1+r)+sum_j IC[j+1]*(X0[j]+r*X1[j]),gamma)*e(agg_c,delta)=ip_ab'};
const q=bls.fields.Fr.ORDER,p=bls.fields.Fp.ORDER,mod=x=>(x%q+q)%q;
const pow=(x,n,m)=>{let a=1n;for(;n;n>>=1n,x=x*x%m)if(n&1n)a=a*x%m;return a;};
const challenge=id=>BigInt('0x'+Buffer.from(prior.actual_javascript_execution.positive_trace.find(t=>t.id===id).challenge_le_hex,'hex').reverse().toString('hex'));
const r=challenge('random-r'),a=challenge('gipa-0-round'),c=pow(a,q-2n,q),z=challenge('random-z');
assert.equal(mod(a*c),1n);assert.ok(r>1n&&r<q&&a>1n&&z>1n);
const {Fp12}=bls.fields;
const generator=bls.pairing(bls.G1.Point.BASE,bls.G2.Point.BASE);
assert.ok(!Fp12.eql(generator,Fp12.ONE));
const h=Fp12.pow(generator,2n);
const sentinelMerge=(left,right)=>Fp12.eql(left,generator)?right:Fp12.eql(right,generator)?left:Fp12.mul(left,right);
assert.ok(Fp12.eql(sentinelMerge(generator,h),h));
assert.ok(!Fp12.eql(sentinelMerge(generator,h),Fp12.mul(generator,h)));
const E=(p**12n-1n)/q;assert.equal(E*q,p**12n-1n);
const bitLength=x=>x.toString(2).length;
const counts={GT_subgroup_q_powers:{count:15,bits:bitLength(q)},GIPA_GT_powers:{count:10,bits:bitLength(q)},independent_final_exponentiations:{count:6,bits:bitLength(E)}};
for(const x of Object.values(counts))x.binary_square_multiply_upper_operations=x.count*2*x.bits;
const slots={original_wire:Math.ceil(5236/32),decoded_GT:15*12*2,decoded_points:(9*128+5*256)/32,matrix:50,scratch:512,metadata_envelope:128};
const storageSlots=Object.values(slots).reduce((a,b)=>a+b,0);
const report={schema:'VV/G9-P1/FEASIBILITY-REVIEW/v2',status:'BLOCKED_NO_COMPLETE_GAS_AND_EQUIVALENCE_BOUND',base_sha:'7475f564dc45b923f30ea25142882f703d821b10',prior:{path:previousPath,sha256:sha(previousBytes)},
 sources:Object.fromEntries(Object.entries(sources).map(([k,v])=>[k,{package_path:path.relative(registry,v.path),sha256:sha(v.bytes)}])),
 ten_equations:base.map(e=>({...e,formula:formulas[e.id]})),
 B2_specialization:{notation:'a=c_inv from gipa-0-round; c=a^-1; r=random-r; z=random-z; additive G1/G2 and multiplicative GT',
   fv:'1+a*z',fw:'z^2*(1+c*r^-1*z)',final_r:'1+a*r',final_zc:'agg_c+c*Zc_L+a*Zc_R',mipp_nonpairing:'C*(1+a*r)==agg_c+c*Zc_L+a*Zc_R',aggregate_r_sum:'1+r',
   derived_scalar_values:{r:r.toString(),a:a.toString(),c:c.toString(),z:z.toString(),fv:mod(1n+a*z).toString(),fw:mod(z*z*mod(1n+c*pow(r,q-2n,q)*z)).toString(),final_r:mod(1n+a*r).toString()},
   anchors:[anchor('prove','fn polynomial_evaluation_product_form_from_transcript'),anchor('verify','let b = z == final_res.zc;'),anchor('verify','let final_r = polynomial_evaluation_product_form_from_transcript')]},
 sentinel_review:{source_generator_is_not_group_identity:[anchor('gt','fn identity() -> Self'),anchor('gt','fn generator() -> Self')],accumulator:[anchor('accumulator','let one = E::Gt::generator();'),anchor('accumulator','if out != &<E as Engine>::Gt::generator()')],
   standard_JS_group_probe_executed:true,generator_neq_identity:true,sentinel_merge_differs_from_literal_GT_product:true,
   complete_native_or_attack_test:false,conclusion:'KZG identity checks require explicit tags; never globally rewrite literal GT generator as identity. Direct conjunction is a relation-level candidate, not bit-for-bit equivalence with randomized accumulator acceptance.'},
 finite_layout_proposal:{Fp:'two u256 limbs encoding one canonical 381-bit coefficient; compare <p; arithmetic modulo p; never raw Montgomery limbs',Fp2:'c0+c1*u, u^2=-1',Fp6:'c0+c1*v+c2*v^2, v^3=u+1',Fp12:'c0+c1*w, w^2=v',
   checkpoint:'program version, session, phase, PC, coefficient/bit/pair cursors, canonical registers, completed-check bitmap; only contract-computed advancement',
   pow:'fixed exponent length; accumulator initially 1; square/multiply micro-PC, no trusted host result',
   inverse_witness:'optional checked nonzero inverse witness, enforce x*y=1; no host-trusted inverses',
   Miller:'persist per-pair G2 line state, product accumulator and signed-x bit cursor; initial bit, negative-x conjugation, line embedding and tower order must match source vectors; unimplemented',
   final_exponent:'generic (p^12-1)/q square-and-multiply is a finite reference schedule, not optimized/native-cross-checked EVM code',
   slot_regions:slots,maximum_allocated_slots_proposed:storageSlots,maximum_allocated_bytes_proposed:storageSlots*32,storage_measured:false,compiled_layout_audited:false},
 fixed_exponent_subprogram_counts:{...counts,known_subprogram_operation_upper_subtotal:Object.values(counts).reduce((n,x)=>n+x.binary_square_multiply_upper_operations,0),
   subtotal_excludes:'Miller, wire decompression, checks/recompression, GT merges, scalar/G1/G2 arithmetic, FS, storage and transactions',gas_per_operation:null,complete_verifier_operation_upper_bound:null},
 resources:{per_transaction_policy_cap:20000000,maximum_actual_transactions:null,maximum_total_gas:null,maximum_execution_time_ms:null,maximum_storage_bytes_measured:null,
   total_gas_formula:'BEGIN + sum(ADVANCE_i) + FINISH + CONSUME, including canonical parsing/FS/arithmetic/checkpoints/calldata; not merely pairing precompiles',
   FS_rejection:'exact rejection loop is data-dependent; a resource timeout is ABORT/INCOMPLETE, not INVALID_PROOF. A finite attempt cap must not be disguised as full-domain equivalence',
   time_bound:'a local watchdog is not guaranteed public-chain transaction inclusion time',p2_allowed:false},
 gate_checks:{all_ten_equations_documented:true,B2_scalar_specialization_source_mapped:true,standard_GT_identity_probe_pass:true,finite_storage_layout_proposed:true,
   GT_stepper_implemented:false,complete_original_domain_equivalence_proven:false,all_phase_gas_bounds_proven:false,total_resource_budget_closed:false},
 new_evm_transactions:0,new_setup:0,new_proofs:0,full_snarkpack_verified:false};
assert.equal(report.ten_equations.length,10);assert.equal(report.ten_equations.reduce((a,b)=>a+b.pairs,0),18);
const output='results/g9_p1_v2/feasibility_review.json',encoded=JSON.stringify(report,null,2)+'\n';
if(mode==='--check'||fs.existsSync(output))assert.equal(fs.readFileSync(output,'utf8'),encoded);else{fs.mkdirSync(path.dirname(output),{recursive:true});fs.writeFileSync(output,encoded,{flag:'wx'});}
console.log(JSON.stringify({status:report.status,mode,ten_equations:10,logical_pairing_terms:18,proposed_slots:storageSlots,finite_pow_operation_subtotal:report.fixed_exponent_subprogram_counts.known_subprogram_operation_upper_subtotal,full_gas:null,P2_allowed:false}));
