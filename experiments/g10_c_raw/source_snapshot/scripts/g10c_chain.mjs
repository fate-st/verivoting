// Continue the SAME functional native run into the unchanged authenticated EVM fallback.
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';import {createHash} from 'node:crypto';
const [arg]=process.argv.slice(2);assert.ok(arg,'usage: node scripts/g10c_chain.mjs EXISTING_NATIVE_RUN_ROOT');
const out=path.resolve(arg),native=path.join(out,'native'),bridge=path.join(out,'chain_bridge');
assert.ok(!fs.existsSync(bridge),'do not overwrite chain evidence');fs.mkdirSync(bridge);
const load=p=>JSON.parse(fs.readFileSync(p)),sha=b=>createHash('sha256').update(b).digest('hex');
const nr=load(path.join(native,'report.json')),publicPath=path.join(native,'winner_only/public.json'),publicFixture=load(publicPath);
assert.equal(nr.status,'PASS_CONNECTED_NATIVE_FIVE_WORKER_NEW_P3_WINNER_ONCE_AND_FULL_TALLY_DIAGNOSTIC');
assert.equal(nr.new_p3_proofs,1);assert.equal(nr.new_p3_proof_from_current_private_witness,true);assert.equal(nr.worker_evidence.unique_process_ids,5);
const m={schema:'VV/G10-C/NATIVE-TO-EVM-EXECUTION/v1',started_utc:new Date().toISOString(),native_report_sha256:sha(fs.readFileSync(path.join(native,'report.json'))),
 native_proof_sha256:publicFixture.proof_sha256,driver_sha256:sha(fs.readFileSync(new URL(import.meta.url))),commands:[],optimization_performed:false,
 exact_snarkpack_evm:false,individual_groth16_evm_fallback:true};
const save=()=>fs.writeFileSync(path.join(bridge,'manifest.json'),JSON.stringify(m,null,2)+'\n');
const env={...process.env,CARGO_BUILD_JOBS:'2',CARGO_PROFILE_RELEASE_STRIP:'none',RAYON_NUM_THREADS:'4'};
function run(name,program,args,timeout=180000){const log=path.join(bridge,name+'.log'),fd=fs.openSync(log,'wx'),t=performance.now();
 const r=spawnSync(program,args,{env,stdio:['ignore',fd,fd],timeout});fs.closeSync(fd);m.commands.push({name,program,args,cwd:process.cwd(),elapsed_ms:performance.now()-t,exit_code:r.status,signal:r.signal,error:r.error?.message??null,log});save();
 process.stdout.write(fs.readFileSync(log));assert.equal(r.status,0,name);}
const write=(p,v)=>fs.writeFileSync(p,JSON.stringify(v,null,2)+'\n',{flag:'wx'});
save();try{
 const bin='g10c_commitment_export';
 run('export-build','cargo',['+1.99.1','build','--release','--locked','--offline','-p','vv-friendly-dkg','--bin',bin],300000);
 run('export-clippy','cargo',['+1.99.1','clippy','--release','--locked','--offline','-p','vv-friendly-dkg','--bin',bin,'--','-D','warnings']);
 run('fmt','cargo',['+1.99.1','fmt','--all','--','--check']);
 const commitments=path.join(bridge,'commitments.json'),commitReport=path.join(bridge,'commitment_export.json');
 run('public-commitment-export',path.resolve('target/release/'+bin),[publicPath,commitments,commitReport]);
 const eip=load(path.join(native,'winner_only/evm.json'));
 const current={...eip,schema:'VV/G10-C/CURRENT-P3-EVM/v1',verifying_key_ark_sha256:publicFixture.verifying_key_sha256,
  proof_ark_sha256:publicFixture.proof_sha256,source_fixture:publicPath,source_fixture_sha256:sha(fs.readFileSync(publicPath))};
 const currentPath=path.join(bridge,'p3_eip2537.json');write(currentPath,current);
 const exportPath=path.join(bridge,'p3_export_binding.json');
 write(exportPath,{status:'PASS_CURRENT_NATIVE_P3_EIP_EXPORT',output_fixture_sha256:sha(fs.readFileSync(currentPath)),
  native_report_sha256:m.native_report_sha256,source_native_eip_sha256:sha(fs.readFileSync(path.join(native,'winner_only/evm.json'))),
  ark_proof_sha256:publicFixture.proof_sha256,scope:'EIP encoding produced by same Rust typed proof/VK after actual native verification; not claiming compressed and EIP hashes equal'});
 const base=load('configs/experiment.G8C-FROZEN-P3-W32.json');
 const pin=p=>({path:p,sha256:sha(fs.readFileSync(p))});
 const config={...base,gate:'G10-C-chain',scope:'same-run fresh P3 + new five-worker commitments in original authenticated individual-Groth16 fallback contracts',
  source_pins:[pin(currentPath),base.source_pins[1],base.source_pins[2],pin(commitments),pin(publicPath)]};
 const configPath=path.join(bridge,'config.json');write(configPath,config);
 run('chain-syntax','node',['--check','contracts/scripts/g10c_winner_chain.mjs']);
 run('evm-connected','node',['contracts/scripts/g10c_winner_chain.mjs',path.join(out,'chain'),configPath,exportPath],660000);
 const chain=load(path.join(out,'chain/winner_once.json'));
 assert.equal(chain.status,'PASS_CURRENT_NATIVE_P3_AND_COMMITMENTS_EVM_WINNER_ONCE');
 assert.equal(chain.ark_proof_sha256,publicFixture.proof_sha256);assert.equal(chain.successful_winner_writes,1);assert.equal(chain.winner_event_count,1);
 assert.equal(chain.snarkpack_evm_verified,false);assert.equal(chain.all_60_inputs_match,true);
 m.chain_report_sha256=sha(fs.readFileSync(path.join(out,'chain/winner_once.json')));m.status='PASS_SAME_NATIVE_PROOF_AND_WORKER_COMMITMENTS_CONNECTED_TO_EVM_FALLBACK';
}catch(e){m.status='FAIL';m.error=String(e);throw e;}finally{m.finished_utc=new Date().toISOString();save();}
console.log(JSON.stringify({status:m.status,proof_sha256:m.native_proof_sha256,exact_snarkpack_evm:false,new_native_proof_reused_in_chain:true}));
