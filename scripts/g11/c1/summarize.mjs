// Summarize only real complete official CLI elections. No cross-system performance ranking.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import {pathToFileURL, fileURLToPath} from 'node:url';
import {csv} from '../model.mjs';
import {read, write, sha} from './run.mjs';

export function stats(values) {
  assert.ok(values.length > 1 && values.every(Number.isFinite));
  const sorted = [...values].sort((a, b) => a - b), n = sorted.length;
  const mean = values.reduce((a, x) => a + x, 0) / n;
  const quantile = p => {const x = (n - 1) * p, i = Math.floor(x); return sorted[i] + (sorted[Math.min(i + 1, n - 1)] - sorted[i]) * (x - i);};
  return {n, mean, median: quantile(0.5), sample_sd: Math.sqrt(values.reduce((a, x) => a + (x - mean) ** 2, 0) / (n - 1)),
    min: sorted[0], max: sorted[n - 1], p95: quantile(0.95)};
}

async function table(rows, headers, target, sheetName, plainCsv) {
  if (plainCsv) {
    write(target, csv(rows, headers));
    return {rows: rows.length, csv_sha256: sha(fs.readFileSync(target)), renderer: 'plain-csv-no-preview'};
  }
  const modulePath = process.env.G11_ARTIFACT_TOOL_MODULE ?? path.join(os.homedir(), '.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/@oai/artifact-tool/dist/artifact_tool.mjs');
  const {Workbook} = await import(pathToFileURL(modulePath).href);
  const wb = Workbook.create(), sheet = wb.worksheets.add(sheetName);
  const matrix = [headers, ...rows.map(r => headers.map(h => r[h] ?? ''))];
  const range = sheet.getRangeByIndexes(0, 0, matrix.length, headers.length); range.values = matrix;
  range.format.font = {name: 'Arial', size: 10}; range.format.columnWidth = 24;
  range.format.rowHeight = 36; range.format.wrapText = true;
  const header = sheet.getRangeByIndexes(0, 0, 1, headers.length);
  header.format.fill = '#263B54'; header.format.font = {name: 'Arial', size: 10, bold: true, color: '#FFFFFF'};
  sheet.freezePanes.freezeRows(1); wb.recalculate(); assert.deepEqual(range.values, matrix);
  // Serialize the checked artifact table; blanks remain blank, never substituted with zero.
  write(target, csv(range.values.slice(1).map(r => Object.fromEntries(headers.map((h, i) => [h, r[i]]))), headers));
  const imported = await Workbook.fromCSV(fs.readFileSync(target, 'utf8'), {sheetName: 'Readback'});
  assert.deepEqual(imported.worksheets.getItemAt(0).getUsedRange().values, matrix.map(row => row.map(String)));
  const preview = await wb.render({sheetName, range: 'A1:F6', scale: 1.5, format: 'png'});
  fs.writeFileSync(target.replace('.csv', '.preview.png'), new Uint8Array(await preview.arrayBuffer()), {flag: 'wx'});
  return {rows: rows.length, csv_sha256: sha(fs.readFileSync(target)), roundtrip: 'exact',
    inspection: (await wb.inspect({kind: 'table', range: `${sheetName}!A1:F4`, include: 'values', maxChars: 1400, tableMaxRows: 4, tableMaxCols: 6})).ndjson};
}

export async function summarize(raw, out, {plainCsv = false} = {}) {
  const run = read(path.join(raw, 'run.json')), environment = read(path.join(raw, 'environment.json'));
  const samples = run.samples.map(p => read(path.join(raw, p)));
  fs.mkdirSync(out, {recursive: true});
  const rows = samples.map(s => ({label: s.label, n: s.n, phase: s.phase, trial: s.trial, status: s.benchmark.status,
    uuid: s.uuid, ...s.benchmark.measurements,
    ballot_generation_per_vote_ms: s.benchmark.measurements.ballot_generation_ms === null ? null : s.benchmark.measurements.ballot_generation_ms / s.n,
    single_verification_per_vote_ms: s.benchmark.measurements.verification_ms === null ? null : s.benchmark.measurements.verification_ms / s.n,
    archive_ballot_audit_ms: s.stage_ms?.all_ballot_audit ?? null, final_audit_ms: s.stage_ms?.final_audit ?? null,
    credential_generation_ms: s.stage_ms?.credentials ?? null, dkg_ms: s.stage_ms?.dkg ?? null,
    online_excluding_setup_ms: s.end_to_end_excluding_setup_ms ?? null, archive_sha256: s.archive_sha256 ?? null}));
  const keys = Object.keys(rows[0]).filter(k => /(_ms|_bytes|_mib|_second|^gas$|transaction_count)$/.test(k));
  const summary = [];
  for (const n of environment.config.sizes) {
    const measured = rows.filter(r => r.n === n && r.phase === 'measured' && r.status === 'passed');
    for (const metric of keys) {
      const values = measured.map(r => r[metric]).filter(x => x !== null);
      summary.push({n, metric, unit: metric.endsWith('_ms') ? 'ms' : metric.endsWith('_bytes') ? 'bytes' : metric.endsWith('_mib') ? 'MiB' : metric.endsWith('_second') ? 'votes/s' : 'not applicable',
        ...values.length > 1 ? stats(values) : {n_samples: values.length, mean: null, median: null, sample_sd: null, min: null, max: null, p95: null},
        voters: n, samples: values.length, note: values.length ? 'Independent complete elections; warmups excluded; CLI wall time' : 'Not separable or not applicable; see report'});
    }
  }
  // n in stats means observation count: use explicit voters/samples in the exported schema.
  const statsRows = summary.map(({n: unused, n_samples: ignored, ...r}) => r);
  const checks = {
    raw_samples: await table(rows, Object.keys(rows[0]), path.join(out, 'samples.csv'), 'Samples', plainCsv),
    summary: await table(statsRows, Object.keys(statsRows[0]), path.join(out, 'summary.csv'), 'Summary', plainCsv),
  };
  const count = n => ({n, warmups: rows.filter(r => r.n === n && r.phase === 'warmup' && r.status === 'passed').length,
    measured: rows.filter(r => r.n === n && r.phase === 'measured' && r.status === 'passed').length,
    failed: rows.filter(r => r.n === n && r.status !== 'passed').length});
  const counts = environment.config.sizes.map(count);
  const complete = run.completed === run.planned && counts.every(c => c.warmups === environment.config.warmups && c.measured === environment.config.trials && c.failed === 0);
  const result = {gate: 'G11-C1', status: complete ? 'PASS_BELENIOS_CLI_REPRODUCTION' : 'PARTIAL_BELENIOS_CLI_REPRODUCTION',
    raw_root: raw, source_commit: environment.provenance.commit, binary_sha256: environment.provenance.binary_sha256,
    run_sha256: sha(fs.readFileSync(path.join(raw, 'run.json'))), counts, checks,
    no_verivoting_comparative_execution: true, no_claim_of_performance_superiority: true,
    distributed_server_or_network_experiment: false, gas_not_applicable: true};
  write(path.join(out, 'analysis.json'), result); console.log(JSON.stringify(result)); return result;
}
if (path.resolve(process.argv[1] ?? '') === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2); assert.equal(args[0], '--raw'); assert.equal(args[2], '--out');
  await summarize(path.resolve(args[1]), path.resolve(args[3]), {plainCsv: args.includes('--plain-csv')});
}
