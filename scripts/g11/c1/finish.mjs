// Public-only audit and content seal; not an extra performance experiment.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {spawnSync, execFileSync} from 'node:child_process';
import {ROOT, read, write, sha} from './run.mjs';
import {filesUnder, seal, assertHistoricalUnchanged} from '../common.mjs';

const bin = path.resolve(process.argv[2]);
const raw = path.join(ROOT, 'experiments/g11/baselines/belenios');
const runDir = path.join(raw, 'run-20261106-01');
const result = path.join(ROOT, 'results/g11/c1');
const analysis = read(path.join(result, 'analysis.json'));
assert.equal(analysis.status, 'PASS_BELENIOS_CLI_REPRODUCTION');
const config = read(path.join(ROOT, 'configs/g11/c1-belenios.json'));
const env = read(path.join(runDir, 'environment.json'));
assert.equal(env.adapter_sha256, sha(fs.readFileSync(path.join(ROOT, 'scripts/g11/c1/run.mjs'))));
assert.equal(env.config_sha256, sha(fs.readFileSync(path.join(ROOT, 'configs/g11/c1-belenios.json'))));
assert.equal(env.provenance.binary_sha256, sha(fs.readFileSync(bin)));
const historical = assertHistoricalUnchanged();
const audits = [];
for (const n of config.sizes) {
  const dir = path.join(runDir, `n${n}-measured-00`);
  const cmd = ['election', 'verify', '--dir', dir];
  const r = spawnSync(bin, cmd, {encoding: 'utf8', timeout: 30000});
  write(path.join(result, `public-audit-n${n}.stdout.log`), r.stdout ?? '');
  write(path.join(result, `public-audit-n${n}.stderr.log`), r.stderr ?? '');
  assert.equal(r.status, 0); assert.equal(r.signal, null); assert.match(r.stderr, /all checks passed/);
  audits.push({n, command: [bin, ...cmd], exit_code: r.status, signal: r.signal, used_private_material: false});
}
// No credentials, trustee keys, OCaml environment dumps or copied source repository in the public package.
for (const p of filesUnder(raw))
  assert.doesNotMatch(path.basename(p), /\.(key|privkey|privcreds|dkey|env)$|^private_creds/);
const oldCheck = execFileSync(process.execPath, ['scripts/g11/check.mjs'], {cwd: ROOT, encoding: 'utf8'}).trim();
write(path.join(result, 'historical-check.json'), JSON.parse(oldCheck));
for (const dir of [path.join(raw, 'pilot-20261106-02'), runDir])
  execFileSync(process.execPath, ['scripts/g11/c1/check.mjs', dir], {cwd: ROOT});
seal(raw);
const sources = [...filesUnder(path.join(ROOT, 'scripts/g11/c1')), path.join(ROOT, 'configs/g11/c1-belenios.json'), path.join(ROOT, 'docs/G11_C1_BELENIOS_REPORT.md')];
write(path.join(result, 'acceptance.json'), {
  status: 'PASS_G11_C1_BELENIOS_ORIGINAL_CLI', code_base: env.code_sha,
  source_files: Object.fromEntries(sources.map(p => [path.relative(ROOT, p), sha(fs.readFileSync(p))])),
  config_sha256: env.config_sha256, executed_adapter_sha256: env.adapter_sha256,
  official_source_commit: config.source_commit, public_only_audits: audits,
  historical, historical_check: JSON.parse(oldCheck),
  suite_elections: 105, formal_elections: 90, warmup_elections: 15, encrypted_ballots_in_suite: 5880,
  measured_failures: 0, counts: analysis.counts,
  failed_initial_pilot_preserved: true, initial_failure_is_adapter_format_error_not_crypto_rejection: true,
  raw_manifest: {path: path.relative(ROOT, path.join(raw, 'SHA256SUMS')), sha256: sha(fs.readFileSync(path.join(raw, 'SHA256SUMS')))},
  no_new_verivoting_setup_or_prove: true, no_new_evm: true,
  no_external_performance_superiority_claim: true, private_credentials_and_keys_exported: false,
  note: 'Only Belenios C1 is complete; G11 external comparison and production distributed deployment are not complete.',
});
seal(result);
console.log(JSON.stringify({status: 'PASS_G11_C1_SEALED', raw, result, public_audits: audits.length, historical_files_unchanged: true}));
