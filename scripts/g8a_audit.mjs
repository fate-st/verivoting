// Consistency audit, not another cryptographic execution or benchmark.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {createRequire} from 'node:module';
const {Interface}=createRequire(new URL('../contracts/package.json',import.meta.url))('ethers');
const read=p=>JSON.parse(fs.readFileSync(p)),hash=b=>createHash('sha256').update(b).digest('hex');
const sha=p=>hash(fs.readFileSync(p)),bytes=h=>Buffer.from(h.replace(/^0x/,''),'hex');
const r=read('experiments/g8a_raw/certificate_auth.json'),f=read(r.fixture),e=read(r.export_report),g6=read(f.source_g6_fixture);
assert.equal(r.status,'PASS_ORIGINAL_JUBJUB_DKG_AND_REGISTRATION_CERTIFICATES_LOCAL_PRAGUE');
assert.equal(sha(r.fixture),r.fixture_sha256);assert.equal(sha(r.export_report),r.export_report_sha256);
for(const p of [...r.source_hashes,...r.source_pins,...e.source_hashes])assert.equal(sha(p.path),p.sha256,p.path);
assert.equal(r.q_cert,r.n-r.t);assert.equal(r.tau,r.t+1);
const signatures=[...f.dkg_certificate_signatures,f.registration_signature];
const Q=6554484396890773809930967563523245729705921265872317281365359162392183254199n;
for(const row of signatures){
 const b=bytes(row.signature_hex);assert.equal(b.length,66);assert.equal(b.subarray(0,32).toString('hex'),row.nonce.encoded_hex);
 assert.equal(BigInt(`0x${Buffer.from(b.subarray(32,64)).reverse().toString('hex')}`).toString(),row.response_scalar);
 assert.equal(b.readUInt16BE(64),row.counter);assert.ok(BigInt(row.response_scalar)<Q);
 for(const h of row.hash_attempts){assert.equal(createHash('sha512').update(bytes(h.input_hex)).digest('hex'),h.sha512_hex);
  const candidate=bytes(h.sha512_hex).subarray(0,32);candidate[31]&=15;
  const n=BigInt(`0x${Buffer.from(candidate).reverse().toString('hex')}`);
  if(h.counter<row.counter)assert.ok(n>=Q);else assert.equal(n.toString(),row.challenge_scalar);
 }
}
for(const [i,p]of f.authentication_keys.entries())assert.equal(p.encoded_hex,g6.trustee_roster[i].authentication_public_key_hex);
for(const [i,p]of f.verification_shares.entries())assert.equal(p.encoded_hex,g6.trustee_roster[i].verification_share_hex);
assert.equal(hash(bytes(f.keycore_bytes_hex)),g6.public_context.key_core_digest_hex);
assert.ok(g6.public_context.registration_certificate_hex.endsWith(f.registration_signature.signature_hex));
assert.equal(f.public_context.registration_certificate_hex,g6.public_context.registration_certificate_hex);
for(const c of r.sha512_checks){assert.equal(c.actual_digest,c.expected_digest);assert.equal(c.actual_digest,`0x${createHash('sha512').update(bytes(c.input_hex)).digest('hex')}`);}
assert.equal(r.sha512_checks.length,13);assert.equal(r.original_challenge_checks.length,6);
assert.ok(r.order_two_point_rejected&&r.identity_point_rejected);
assert.equal(sha(r.contract_artifact),r.contract_artifact_sha256);
const artifacts=read(r.contract_artifact),iface=new Interface(artifacts.contracts.certificate_gate.abi);
assert.equal(r.accepted_dkg_signatures.length,3);
for(const [i,t]of r.accepted_dkg_signatures.entries()){
 assert.equal(t.receipt_status,1);assert.equal(t.checkpoint.count,i+1);assert.equal(t.checkpoint.dkg_certified,i===2);
 const decoded=iface.decodeFunctionData('submitDkgSignature',t.calldata_hex),original=f.dkg_certificate_signatures[i];
 assert.equal(Number(decoded[0]),i+1);assert.equal(decoded[1],`0x${f.public_context.election_id_hex}`);
 assert.equal(decoded[2],`0x${f.public_context.final_context_hex}`);
 assert.equal(decoded[3].nonce.encoded,`0x${original.nonce.encoded_hex}`);
 assert.equal(decoded[3].response.toString(),original.response_scalar);assert.equal(Number(decoded[3].counter),original.counter);
}
const registration=iface.decodeFunctionData('authenticateRegistration',r.registration.calldata_hex);
assert.equal(registration[0].root,`0x${g6.public_context.registration_root_hex}`);
assert.equal(registration[1].nonce.encoded,`0x${f.registration_signature.nonce.encoded_hex}`);
assert.equal(registration[1].response.toString(),f.registration_signature.response_scalar);
assert.equal(r.registration.receipt_status,1);assert.equal(r.authorization.receipt_status,1);
assert.equal(r.final_checkpoint.authorized,true);assert.deepEqual(r.final_checkpoint.signer_mask,[true,true,true,false,false]);
assert.equal(r.final_checkpoint.close_digest,`0x${g6.public_context.registration_close_digest_hex}`);
assert.equal(r.final_checkpoint.disclosure_digest,`0x${g6.public_context.disclosure_digest_hex}`);
assert.equal(r.negative_tests.length,12);for(const t of r.negative_tests){assert.equal(t.receipt_status,0);assert.ok(t.atomic_rejection);assert.equal(t.exception,'revert');}
const forgedDkg=iface.decodeFunctionData('submitDkgSignature',r.negative_tests.find(t=>t.case==='forged_dkg_signature').calldata_hex)[3];
const forgedRegistration=iface.decodeFunctionData('authenticateRegistration',r.negative_tests.find(t=>t.case==='forged_registration_signature').calldata_hex)[1];
assert.ok(forgedDkg.response<Q&&forgedRegistration.response<Q,'forgery negatives use canonical response scalars, not encoding failures');
const txs=[r.deployment,...r.accepted_dkg_signatures,r.registration,r.authorization,...r.negative_tests].sort((a,b)=>a.nonce-b.nonce);
assert.equal(txs.length,18);assert.deepEqual(txs.map(x=>x.nonce),Array.from({length:18},(_,i)=>i));
for(const t of txs){assert.equal(bytes(t.calldata_hex).length,t.calldata_bytes);assert.equal(hash(bytes(t.calldata_hex)),t.calldata_sha256);
 assert.equal(t.total_gas,Math.max(t.intrinsic_gas+t.execution_gas,t.minimum_gas_limit_including_calldata_floor));assert.ok(t.total_gas<20_000_000);}
for(const a of Object.values(artifacts.contracts)){assert.equal(hash(bytes(a.runtime_bytecode_hex)),a.runtime_sha256);assert.ok(a.runtime_bytecode_bytes<=24576);}
for(const flag of ['signature_scheme_replaced','receiver_auth_gate_integrated','p3_storage_binding_integrated','winner_written','complete_g8_settlement','snarkpack_evm_verified'])assert.equal(r[flag],false);
assert.equal(r.groth16_setup_count,0);assert.equal(r.groth16_proof_generation_count,0);assert.equal(r.full_system_gas,null);
const clarification=read('experiments/g8a_raw/provenance_clarification.json');assert.ok(clarification.dkg_signing_reexecuted_with_original_rng);
let archives=0;
for(const gate of ['g1_e','g2_d','g2_e','g4_d','g6','g7','g8'])for(const line of fs.readFileSync(`experiments/${gate}_raw/SHA256SUMS`,'utf8').trim().split('\n')){
 const[,h,p]=line.match(/^([0-9a-f]{64})\s+(.+)$/)??[];assert.ok(p);
 if(!/^(experiments|fixtures|results)\//.test(p)||p.startsWith('experiments/aggregate/'))continue;
 assert.equal(sha(p),h,p);archives++;
}
const previous=read('experiments/g8_raw/receiver_freeze.json');
for(const p of previous.source_hashes){const actual=['crates/vv-friendly-dkg/Cargo.toml','contracts/package.json'].includes(p.path)?hash(execFileSync('git',['show',`1c19f811:${p.path}`])):sha(p.path);assert.equal(actual,p.sha256,p.path);}
const commands=read('experiments/g8a_raw/commands.json').commands,latest=Object.fromEntries(commands.map(x=>[x.name,x]));
for(const name of ['fmt-check','clippy','tests','state-tests','build','public-export','evm'])assert.equal(latest[name].exit_code,0,name);
console.log(JSON.stringify({status:'PASS_G8A_ORIGINAL_CERTIFICATE_AUTHENTICATION_AND_PROVENANCE_AUDIT',historical_artifact_entries_unchanged:archives,
 actual_signed_transactions:18,original_dkg_signatures_evm:3,original_registration_signatures_evm:1,sha512_checks:13,challenge_checks:6,atomic_negatives:12,
 dkg_public_material_source:'original deterministic reconstruction, not a previously archived G6 five-signature blob',receiver_integration:false,full_g8:false}));
