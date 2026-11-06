// Categorical source comparison, not an external protocol benchmark.
import fs from 'node:fs'; import path from 'node:path'; import os from 'node:os';
import assert from 'node:assert/strict'; import {pathToFileURL,fileURLToPath} from 'node:url';
import {ROOT,json,sha} from './common.mjs'; import {csv} from './model.mjs';
export const DATA_PATH=path.join(ROOT,'experiments/g11/literature/evidence.json');
export function validateEvidence(data) {
  const fields=Object.keys(data.definitions); const states=['supported','not_supported','conditional','unknown'];
  assert.equal(data.systems.length,6); assert.equal(new Set(data.systems.map(s=>s.id)).size,6);
  const rows=[];
  for(const system of data.systems) {
    assert.equal(system.architecture.stages.length,6);assert.equal(system.architecture.stage_locators.length,6);
    if(system.access==='full_text_relevant_sections_reviewed') {
      assert.match(system.source_sha256,/^[a-f0-9]{64}$/);assert.ok(system.reviewed_pdf_pages.length);
      assert.ok(system.reviewed_pdf_pages.every(p=>Number.isInteger(p)&&p>=1&&p<=system.pdf_pages));
    }
    for(const field of fields) {
      const p=system.properties[field]??{status:'unknown',locator:'Complete original protocol/security sections unavailable',note:system.unknown_reason};
      assert.ok(states.includes(p.status));assert.ok(p.locator?.length&&p.note?.length);
      if(p.status!=='unknown') assert.ok(['full_text_relevant_sections_reviewed','local_source_and_archived_execution'].includes(system.access));
      rows.push({system:system.name,version:system.version,attribute:field,...p,source:system.url,source_sha256:system.source_sha256,evidence_level:system.access});
    }
  }
  return rows;
}
async function artifactCsv(rows,headers,target,previewCols) {
  // Resolve only the Codex bundled dependency, never this repository's node_modules.
  const module=process.env.G11_ARTIFACT_TOOL_MODULE??path.join(os.homedir(),'.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/@oai/artifact-tool/dist/artifact_tool.mjs');
  if(!fs.existsSync(module))throw Error('Set G11_ARTIFACT_TOOL_MODULE to a bundled @oai/artifact-tool module');
  const {Workbook}=await import(pathToFileURL(module).href);
  const workbook=Workbook.create(),sheet=workbook.worksheets.add('Comparison');sheet.showGridLines=false;
  const matrix=[headers,...rows.map(row=>headers.map(key=>row[key]??''))];
  const range=sheet.getRangeByIndexes(0,0,matrix.length,headers.length);range.values=matrix;
  range.format.font={name:'Arial',size:10};range.format.rowHeight=68;range.format.columnWidth=29;range.format.wrapText=true;
  const header=sheet.getRangeByIndexes(0,0,1,headers.length);header.format.fill='#263B54';header.format.font={name:'Arial',size:10,bold:true,color:'#FFFFFF'};
  sheet.freezePanes.freezeRows(1);sheet.freezePanes.freezeColumns(1);workbook.recalculate();
  assert.deepEqual(range.values,matrix);
  const stored=range.values;const objects=stored.slice(1).map(row=>Object.fromEntries(headers.map((h,i)=>[h,row[i]])));
  fs.writeFileSync(target,csv(objects,headers),{flag:'wx'});
  const imported=await Workbook.fromCSV(fs.readFileSync(target,'utf8'),{sheetName:'Readback'});
  assert.deepEqual(imported.worksheets.getItemAt(0).getUsedRange().values,matrix.map(row=>row.map(String)));
  const inspect=await workbook.inspect({kind:'table',range:'Comparison!A1:D4',include:'values',maxChars:900,tableMaxRows:4,tableMaxCols:4});
  const preview=await workbook.render({sheetName:'Comparison',range:`A1:${previewCols}7`,scale:1.5,format:'png'});
  fs.writeFileSync(target.replace('.csv','.preview.png'),new Uint8Array(await preview.arrayBuffer()),{flag:'wx'});
  return {csv_sha256:sha(fs.readFileSync(target)),rows:rows.length,cols:headers.length,roundtrip:'exact',inspection:inspect.ndjson};
}
export async function buildLiterature(out,architectureOut=path.join(out,'architecture')) {
  const data=json(DATA_PATH),rows=validateEvidence(data);fs.mkdirSync(out,{recursive:true});fs.mkdirSync(architectureOut,{recursive:true});
  const write=(p,value)=>fs.writeFileSync(path.join(out,p),typeof value==='string'?value:JSON.stringify(value,null,2)+'\n',{flag:'wx'});
  write('papers.yaml',{yaml_1_2_note:'JSON syntax is valid YAML 1.2',review_date:data.review_date,papers:data.systems.map(({properties,architecture,trust,...paper})=>paper)});
  write('evidence.json',data); // If out is the input source directory, use a separate fresh generated directory.
  const fields=Object.keys(data.definitions);
  const security=data.systems.map(s=>({system:s.name,version:s.version,...Object.fromEntries(fields.map(f=>[f,s.properties[f]?.status??'unknown'])),trust_assumptions:s.trust.assumptions,evidence:rows.filter(r=>r.system===s.name).map(r=>`${r.attribute}: ${r.locator}`).join('; '),implementation:s.implementation}));
  const architecture=data.systems.map(s=>({system:s.name,version:s.version,...Object.fromEntries(s.architecture.stages.map((stage,i)=>[`stage_${i+1}`,stage])),...Object.fromEntries(['execution','private_key_custody','tally_actor','full_tally_visibility','public_check'].map(k=>[k,s.architecture[k]])),evidence:s.architecture.stage_locators.join('; ')}));
  const trust=data.systems.map(s=>({system:s.name,version:s.version,...s.trust,source:s.url}));
  const checks={};
  for(const [name,table,preview]of [['security_matrix',security,'D'],['architecture_matrix',architecture,'D'],['trust_model',trust,'D'],['attribute_evidence',rows,'D']])
    checks[name]=await artifactCsv(table,Object.keys(table[0]),path.join(out,`${name}.csv`),preview);
  const stageNames=['Initialization','Registration','Ballot generation','Validation and accumulation','Final tally','Disclosure'];
  for(const s of data.systems){
    const lines=['flowchart LR',`  %% ${s.name}; ${s.version}`,`  %% Evidence: ${s.url}`];
    for(let i=0;i<6;i++){lines.push(`  S${i+1}["${stageNames[i]}<br/>${s.architecture.stages[i]}"]`);if(i)lines.push(`  S${i} --> S${i+1}`);}
    lines.push(`  %% Execution: ${s.architecture.execution}`,`  %% Keys: ${s.architecture.private_key_custody}`,`  %% Full tally: ${s.architecture.full_tally_visibility}`,`  %% Public checks: ${s.architecture.public_check}`);
    fs.writeFileSync(path.join(architectureOut,`${s.id}.mmd`),lines.join('\n')+'\n',{flag:'wx'});
  }
  const escape=s=>s.replaceAll('&','\\&').replaceAll('_','\\_');
  write('references.bib',data.systems.filter(s=>s.id!=='verivoting').map(s=>`@${s.id==='belenios'?'manual':'misc'}{g11_${s.id},\n  title = {${escape(s.title)}},\n  author = {${s.authors.split('; ').join(' and ')}},\n  year = {${s.year}},\n  note = {${escape(s.version)}; reviewed ${data.review_date}},\n  url = {${s.url}}${s.doi?`,\n  doi = {${s.doi}}`:''}\n}`).join('\n\n')+'\n');
  const summary={status:'PARTIAL_G11_AB_FOUR_EXTERNAL_FULLTEXTS_ONE_ACCESS_GAP',systems:6,external_systems:5,fulltext_external_systems:4,
    source_evidence_sha256:sha(fs.readFileSync(DATA_PATH)),attribute_rows:rows.length,unknown_rows:rows.filter(r=>r.status==='unknown').length,
    pending:['Emami complete original protocol/security text and exact code revision'],
    new_protocol_runs:0,new_setup:0,new_proofs:0,new_evm:0,exact_snarkpack_evm_complete:false,checks};
  write('literature_summary.json',summary);console.log(JSON.stringify({...summary,checks:undefined}));return summary;
}
if(path.resolve(process.argv[1]??'')===fileURLToPath(import.meta.url)){
  const args=process.argv.slice(2);if(args[0]!=='--out'||!args[1])throw Error('Usage: literature.mjs --out /NEW/DIR [--architecture-out /NEW/ARCH/DIR]');
  await buildLiterature(path.resolve(args[1]),args[2]==='--architecture-out'?path.resolve(args[3]):undefined);
}
