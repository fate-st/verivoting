// Read-only acceptance: old G11 remains a historical snapshot, not a claim about newly added C1 runs.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import {ROOT, read, sha, schedule, workload, parseTime} from './run.mjs';
import {stats} from './summarize.mjs';

const config = read(path.join(ROOT, 'configs/g11/c1-belenios.json'));
assert.equal(schedule(config, false).length, 105);
assert.equal(new Set(schedule(config, false).map(s => `${s.n}/${s.phase}/${s.trial}`)).size, 105);
assert.deepEqual(stats([1, 2, 3]).mean, 2);
assert.equal(parseTime('0.02 real 0.01 user 0.00 sys\n123 maximum resident set size').peak_rss_bytes, 123);
assert.equal(parseTime('not measured').peak_rss_bytes, null);
const w = workload(8, 'measured', 0, config.workload_seed);
assert.equal(w.expected_tally.reduce((a, x) => a + x, 0), w.voters.reduce((a, v) => a + v.weight, 0));
assert.notDeepEqual(w, workload(8, 'measured', 1, config.workload_seed));

const raw = process.argv[2];
if (raw) {
  const dir = path.resolve(raw), run = read(path.join(dir, 'run.json')), env = read(path.join(dir, 'environment.json'));
  assert.equal(env.provenance.commit, config.source_commit); assert.equal(env.provenance.tracked_source_unchanged, true);
  assert.equal(env.config_sha256, sha(fs.readFileSync(path.join(ROOT, 'configs/g11/c1-belenios.json'))));
  const uuids = new Set();
  for (const p of run.samples) {
    const s = read(path.join(dir, p)), d = path.dirname(path.join(dir, p));
    assert.equal(s.benchmark.status, 'passed'); assert.equal(s.ballot_count, s.n);
    assert.deepEqual(s.actual_tally.map(r => r.map(Number)), [s.expected_tally]);
    assert.equal(s.archive_sha256, sha(fs.readFileSync(path.join(d, 'election.bel'))));
    assert.equal(s.private_material_exported, false); assert.equal(uuids.has(s.uuid), false); uuids.add(s.uuid);
    const commands = fs.readFileSync(path.join(d, 'commands.jsonl'), 'utf8').trim().split('\n').map(JSON.parse);
    assert.equal(commands.length, s.command_count);
    assert.equal(commands.filter(c => c.stage === 'ballot_generation').length, s.n);
    assert.equal(commands.filter(c => c.stage === 'single_ballot_verification').length, s.n);
    assert.equal(commands.filter(c => c.stage === 'threshold_decryption').length, 4);
    for (const c of commands) { assert.equal(c.error, null); assert.equal(c.signal, null); assert.equal(c.expected_rejection ? c.exit_code === 124 : c.exit_code === 0, true); }
    assert.equal(commands.filter(c => c.expected_rejection).length, s.phase === 'pilot' ? 2 : 0);
    if (s.phase === 'pilot') {
      const log = fs.readFileSync(path.join(d, 'stderr.log'), 'utf8');
      assert.match(log, /belenios-tool: error: "InvalidBallot"/);
      assert.match(log, /belenios-tool: a partial decryption is missing/);
      assert.doesNotMatch(log, /Unexpected end of input/);
    }
    assert.equal(s.benchmark.measurements.gas, null); assert.equal(s.benchmark.measurements.proof_generation_ms, null);
  }
  assert.equal(run.completed, run.planned); assert.equal(run.status, 'passed');
  if (run.planned !== 1) {
    assert.equal(run.planned, 105);
    const samples = run.samples.map(p => read(path.join(dir, p)));
    for (const n of config.sizes) for (const [phase, count] of [['warmup', 5], ['measured', 30]])
      assert.equal(samples.filter(s => s.n === n && s.phase === phase).length, count);
  }
  console.log(JSON.stringify({status: 'PASS_G11_C1_RAW_ACCEPTANCE', raw: dir, elections: run.completed, unique_elections: uuids.size}));
} else console.log('PASS_G11_C1_ADAPTER_TESTS (no official protocol execution)');

// Ensure this script can be imported/syntax checked without silently executing a benchmark.
assert.ok(fileURLToPath(import.meta.url).endsWith('check.mjs'));
