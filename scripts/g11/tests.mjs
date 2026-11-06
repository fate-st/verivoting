import assert from 'node:assert/strict'; import fs from 'node:fs'; import os from 'node:os'; import path from 'node:path';
import {ElectionConfig,WorkloadConfig,BenchmarkResult,METRICS,csv} from './model.mjs';
import {json,ROOT,runChild,seal,sha} from './common.mjs';
import {validateEvidence,DATA_PATH} from './literature.mjs';
const cfg=json(path.join(ROOT,'configs/g11/framework.json')); new ElectionConfig(cfg.election);new WorkloadConfig(cfg.workload);
assert.throws(()=>new ElectionConfig({...cfg.election,weights:[0]}));
assert.throws(()=>new WorkloadConfig({...cfg.workload,trials:-1}));
assert.throws(()=>new BenchmarkResult({system:'not run',kind:'literature',status:'passed',evidence:[],measurements:{gas:0},missing_reason:'Not run'}));
const result=new BenchmarkResult({system:'not run',kind:'external_execution',status:'not_executed',evidence:[],missing_reason:'No adapter'});
assert.ok(METRICS.every(k=>result.measurements[k]===null)); assert.equal(csv([{a:'a,b',b:null}],['a','b']),'a,b\n"a,b",\n');
const evidence=json(DATA_PATH);assert.equal(validateEvidence(evidence).length,66);
const fabricated=structuredClone(evidence);fabricated.systems.find(s=>s.id==='emami').properties.winner_only={status:'supported',locator:'Abstract',note:'Not enough evidence'};
assert.throws(()=>validateEvidence(fabricated));
const out=fs.mkdtempSync(path.join(os.tmpdir(),'vv-g11-runner-test-'));
const good=await runChild(process.execPath,['-e','console.log("actual successful harness test")'],{out,name:'success',timeoutMs:1000});
const bad=await runChild(process.execPath,['-e','process.exit(3)'],{out,name:'failure',timeoutMs:1000});
const timeout=await runChild(process.execPath,['-e','setTimeout(()=>{},10000)'],{out,name:'timeout',timeoutMs:100});
assert.equal(good.status,'passed');assert.equal(bad.status,'failed');assert.equal(bad.exit_code,3);assert.equal(timeout.status,'timeout');
seal(out);for(const line of fs.readFileSync(path.join(out,'SHA256SUMS'),'utf8').trim().split('\n')){
 const [h,p]=line.split('  ');assert.equal(sha(fs.readFileSync(path.join(out,p))),h);}
console.log(JSON.stringify({status:'PASS_G11_MODEL_AND_RUNNER_TESTS',actual_child_statuses:[good.status,bad.status,timeout.status],logs:out,external_protocol_runs:0}));
