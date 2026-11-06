import fs from 'node:fs'; import path from 'node:path';
import {ROOT, environment, assertHistoricalUnchanged, runChild, seal, json} from './common.mjs';
import {ElectionConfig, WorkloadConfig, BenchmarkResult, METRICS, csv} from './model.mjs';

const args = process.argv.slice(2); const option = key => args[args.indexOf(key) + 1];
if (args.length !== 4 || !args.includes('--case') || !args.includes('--out'))
  throw Error('Usage: node scripts/g11/run.mjs --case inventory|literature|regression|all --out /ABS/NEW_DIR');
const selected = option('--case'); if (!['inventory','literature','regression','all'].includes(selected)) throw Error('Case not implemented');
const out = path.resolve(option('--out')); const relative = path.relative(ROOT, out);
if (!path.isAbsolute(option('--out')) || relative === '' || (!relative.startsWith('..') && !relative.startsWith('results/g11/')))
  throw Error('Use a fresh results/g11 subdirectory or an absolute directory outside the repository');
if (fs.existsSync(out)) throw Error('Output directory already exists; no overwrite allowed');
const config = json(path.join(ROOT,'configs/g11/framework.json'));
new ElectionConfig(config.election); new WorkloadConfig(config.workload);
const history = assertHistoricalUnchanged(); fs.mkdirSync(out, {recursive:true});
fs.writeFileSync(path.join(out,'config.json'), JSON.stringify(config,null,2)+'\n');
fs.writeFileSync(path.join(out,'environment.json'), JSON.stringify(environment(config),null,2)+'\n');
const results = [], executions = [];
for (const name of selected === 'all' ? ['inventory','literature','regression'] : [selected]) {
  if(name==='regression') {
    for(const [label,file,argv] of [
      ['g1-g2-g4','python3',['scripts/comparison_stage1.py','--check-only']],
      ['g8',process.execPath,['scripts/g11/regression_g8.mjs']],
      ['g10-native',process.execPath,['scripts/g10c_report.mjs','--check']],
      ['g10-chain',process.execPath,['scripts/g10c_chain_audit.mjs']],
    ]) {
      const e=await runChild(file,argv,{out,name:`regression-${label}`,timeoutMs:config.timeout_ms});
      executions.push({name:`regression-${label}`,...e});
      results.push(new BenchmarkResult({system:`G11-regression-${label}`,kind:'framework',status:e.status,
        evidence:[`regression-${label}.stdout.log`,`regression-${label}.stderr.log`],missing_reason:'Read-only archived evidence regression, not a protocol benchmark.'}));
    }
    continue;
  }
  const execution = name === 'inventory'
    ? await runChild(process.execPath,['-e','console.log(JSON.stringify({operation:"environment_inventory",crypto_executed:false}))'],{out,name,timeoutMs:config.timeout_ms})
    : await runChild(process.execPath,['scripts/g11/literature.mjs','--out',path.join(out,'literature')],{out,name,timeoutMs:config.timeout_ms});
  executions.push({name,...execution});
  results.push(new BenchmarkResult({system: name === 'inventory' ? 'G11-framework' : 'G11-AB-literature',
    kind: name === 'inventory' ? 'framework' : 'literature', status:execution.status,
    evidence:[`${name}.stdout.log`,`${name}.stderr.log`], measurements:{},
    missing_reason:'No voting protocol or external benchmark was executed. Framework durations are recorded separately.'}));
}
fs.writeFileSync(path.join(out,'runs.json'), JSON.stringify({schema_version:'g11.v1',results,executions,historical_files:history},null,2)+'\n');
fs.writeFileSync(path.join(out,'summary.csv'),csv(results.map(r=>({system:r.system,kind:r.kind,status:r.status,...r.measurements,missing_reason:r.missing_reason})),['system','kind','status',...METRICS,'missing_reason']));
assertHistoricalUnchanged(); seal(out);
console.log(JSON.stringify({status:executions.every(e=>e.status==='passed')?'PASS_G11_FRAMEWORK':'FAILED_G11_FRAMEWORK',out,case_count:results.length,protocol_measurements:0,external_protocol_runs:0}));
if(executions.some(e=>e.status!=='passed')) process.exitCode=1;
