// Read-only reconciliation; no workbook export, no source CSV mutation.
import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
const { Workbook } = await import(process.env.ARTIFACT_TOOL_MODULE ?? '@oai/artifact-tool');
const root = fileURLToPath(new URL('../', import.meta.url));
const json = JSON.parse(await fs.readFile(`${root}/results/g1_e/summary.json`, 'utf8'));
const text = await fs.readFile(`${root}/results/g1_e/summary.csv`, 'utf8');
const wb = await Workbook.fromCSV(text, {sheetName: 'G1E'});
console.log((await wb.inspect({kind:'table', tableMaxRows:3, tableMaxCols:4, maxChars:900})).ndjson);
const rows = wb.worksheets.getItemAt(0).getUsedRange().values;
const headers = rows.shift();
assert.equal(rows.length, 5);
for (const row of rows) {
  const r = Object.fromEntries(headers.map((h,i)=>[h,row[i]]));
  const b = json.batches[Number(r.batch_size)];
  assert.equal(Number(r.samples), 30);
  for (const op of ['individual_verify','aggregate_verify','aggregation','verifier_speed_ratio']) {
    assert.equal(Number(r[`${op}_median`]), b.timing[op].median);
    assert.equal(r[`${op}_p95`], '');
  }
  for (const key of ['aggregate_proof_bytes','individual_proof_bytes','matrix_bytes','aggregate_plus_matrix_bytes','individual_plus_matrix_bytes']) {
    assert.equal(Number(r[key]), b.sizes[key]);
  }
}
console.log(JSON.stringify({status:'PASS_READ_ONLY_CSV_JSON_RECONCILIATION', rows:5, compared_numeric_fields:45, empty_p95_cells:20, csv_sha256:createHash('sha256').update(text).digest('hex'), source_modified:false}));
