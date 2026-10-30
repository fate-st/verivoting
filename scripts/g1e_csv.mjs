// CSV-only research outputs; no unrequested XLSX workbook.
import fs from 'node:fs/promises';
const { Workbook } = await import(process.env.ARTIFACT_TOOL_MODULE ?? '@oai/artifact-tool');
const out = process.argv[2] ?? 'results/g1_e';
const tables = JSON.parse(await fs.readFile(`${out}/csv_tables.json`, 'utf8'));
for (const name of ['raw_samples', 'summary']) {
  const rows = tables[name];
  const headers = Object.keys(rows[0]);
  const matrix = [headers, ...rows.map(row => headers.map(h => row[h] ?? null))];
  const workbook = Workbook.create();
  const sheet = workbook.worksheets.add(name);
  sheet.getRangeByIndexes(0, 0, matrix.length, headers.length).values = matrix;
  workbook.recalculate();
  const values = sheet.getRangeByIndexes(0, 0, matrix.length, headers.length).values;
  if (values.length !== matrix.length || values[1][0] !== rows[0][headers[0]]) throw Error('CSV typed table mismatch');
  console.log((await workbook.inspect({kind:'table', range:`${name}!A1:D4`, tableMaxRows:4, tableMaxCols:4, maxChars:1200})).ndjson);
  // CSV serialization is explicit: no formatting, no missing-to-zero conversion.
  const quote = value => {const s=value == null ? '' : String(value); return /[",\n\r]/.test(s) ? `"${s.replaceAll('"','""')}"` : s;};
  await fs.writeFile(`${out}/${name}.csv`, values.map(row => row.map(quote).join(',')).join('\n')+'\n', {flag:'wx'});
}
