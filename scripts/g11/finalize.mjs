import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';
import {ROOT,BASE,json,sha,seal,runChild,assertHistoricalUnchanged,filesUnder} from './common.mjs';
import {DATA_PATH,validateEvidence} from './literature.mjs';
const runPath=process.argv[2];if(!runPath)throw Error('Usage: finalize.mjs results/g11/run-YYYYMMDD-NN');
const resolved=path.resolve(ROOT,runPath);assert.ok(resolved.startsWith(path.join(ROOT,'results/g11/run-')));
const run=json(path.join(resolved,'runs.json'));assert.ok(run.executions.every(e=>e.status==='passed'));
const literature=json(path.join(resolved,'literature/evidence.json')),data=json(DATA_PATH);assert.deepEqual(literature,data);
const final=path.join(ROOT,'results/g11/final');assert.ok(!fs.existsSync(final));fs.mkdirSync(final);
const tests=await runChild(process.execPath,['scripts/g11/tests.mjs'],{out:final,name:'tests',timeoutMs:30000});assert.equal(tests.status,'passed');
const write=(p,data)=>fs.writeFileSync(p,typeof data==='string'?data:JSON.stringify(data,null,2)+'\n',{flag:'wx'});
const outputs=['papers.yaml','security_matrix.csv','architecture_matrix.csv','trust_model.csv','attribute_evidence.csv','references.bib','literature_summary.json'];
fs.mkdirSync(path.join(ROOT,'results/g11/literature'));
for(const name of outputs){
  const source=path.join(resolved,'literature',name);
  for(const dir of ['experiments/g11/literature','results/g11/literature']){
    const target=path.join(ROOT,dir,name);fs.copyFileSync(source,target,fs.constants.COPYFILE_EXCL);
  }
}
fs.mkdirSync(path.join(ROOT,'experiments/g11/architecture'));
for(const s of data.systems)fs.copyFileSync(path.join(resolved,'literature/architecture',`${s.id}.mmd`),path.join(ROOT,'experiments/g11/architecture',`${s.id}.mmd`),fs.constants.COPYFILE_EXCL);
const count=validateEvidence(data),sources=json(path.join(ROOT,'results/g11/source_access.json'));
const acceptance={status:'PASS_G11_FRAMEWORK_PARTIAL_G11_AB',base_commit:BASE,accepted_run:path.relative(ROOT,resolved),
  source_evidence_sha256:sha(fs.readFileSync(DATA_PATH)),literature_status:'PARTIAL_FOUR_FULLTEXTS_ONE_ACCESS_GAP',
  external_fulltext_sources:sources.sources.length,attribute_rows:count.length,unknown_attribute_rows:count.filter(p=>p.status==='unknown').length,
  model_runner_test:tests,read_only_regressions:run.executions.filter(e=>e.name.startsWith('regression-')),
  pdf_pages_visually_reviewed:[1,2,3,4,5,6],csv_preview_tables_visually_reviewed:4,
  first_attempt_preserved:'results/g11/run-20261106-01',external_protocol_runs:0,new_setup:0,new_proofs:0,new_evm:0,
  full_g11_complete:false,exact_snarkpack_evm_complete:false,
  history:assertHistoricalUnchanged(),limitations:['Emami full protocol/security text unavailable.','No external code was reproduced.','Current G9, independent DKG and T4/T5 gaps unchanged.']};
write(path.join(final,'acceptance.json'),acceptance);
write(path.join(final,'G11_REPRODUCIBILITY.md'),`# G11 本轮复现\n\n基准 ${BASE}；当前开发分支 experiment/g11-comparative-evaluation，运行时包含未提交的G11文件，未声称干净提交复现。\n\n\`node scripts/g11/run.mjs --case all --out /ABS/NEW_RUN_DIR\`\n\n\`node scripts/g11/check.mjs\` 仅校验本轮已封存材料，不重新运行密码学。\n\n运行器需要Node和Python；CSV导出依赖Codex独立运行时的artifact-tool，通过G11_ARTIFACT_TOOL_MODULE指定，不安装进原协议依赖。PDF构建需要reportlab，来源版本核查需要pypdf。\n\n每个运行目录保存environment.json、config.json、runs.json、summary.csv、原始stdout/stderr及SHA256SUMS。框架耗时不是投票协议耗时。只读核查记录的历史Gas/Prove不计为新测量。\n\n旧G8审计包含对整个历史源码树的冻结断言，在后续正常Cargo变更后不适用于当前树。首次失败保留；第二次在原4a2cc163版本临时源码中运行同一未修改审计，所有断言通过。另核对整个当前历史部分与5f170c5一致。\n\n公开缓存论文不随Git发布；source_access.json保存四份PDF的SHA-256和物理页码。重新下载时哈希不一致应停止，不接受网站静默换版。Emami条目等待完整原文，不能补猜结论。\n`);
const sealPaths=[...filesUnder(path.join(ROOT,'scripts/g11')),...filesUnder(path.join(ROOT,'configs/g11')),
  ...filesUnder(path.join(ROOT,'experiments/g11')),...filesUnder(path.join(ROOT,'results/g11')),
  ...fs.readdirSync(path.join(ROOT,'docs')).filter(p=>p.startsWith('G11')).map(p=>path.join(ROOT,'docs',p))].sort();
write(path.join(final,'SHA256SUMS'),sealPaths.map(p=>`${sha(fs.readFileSync(p))}  ${path.relative(final,p)}`).join('\n')+'\n');
console.log(JSON.stringify({status:acceptance.status,files_hashed:sealPaths.length,external_protocol_runs:0}));
