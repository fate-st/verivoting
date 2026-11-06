// Orchestration only: all credentials, DKG, ballots, proofs, tallying and audits use upstream Belenios.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync, spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {BenchmarkResult} from '../model.mjs';

export const ROOT = fileURLToPath(new URL('../../../', import.meta.url));
export const sha = x => createHash('sha256').update(x).digest('hex');
export const read = p => JSON.parse(fs.readFileSync(p, 'utf8'));
export const write = (p, x) => fs.writeFileSync(p, typeof x === 'string' ? x : JSON.stringify(x, null, 2) + '\n', {flag: 'wx'});
const git = (dir, ...args) => execFileSync('git', ['-C', dir, ...args], {encoding: 'utf8'}).trim();
const bytes = x => Buffer.byteLength(JSON.stringify(x));

export function workload(n, phase, trial, seed) {
  const digest = i => Buffer.from(sha(`${seed}/${n}/${phase}/${trial}/${i}`), 'hex');
  const voters = Array.from({length: n}, (_, i) => {
    const h = digest(i);
    return {id: `v${String(i + 1).padStart(4, '0')}`, weight: 1 + h[0] % 3, choice: h[1] % 3};
  });
  const tally = [0, 0, 0];
  for (const v of voters) tally[v.choice] += v.weight;
  return {seed, n, phase, trial, voters, expected_tally: tally};
}

export function schedule(config, pilot) {
  if (pilot) return [{n: 8, phase: 'pilot', trial: 0}];
  const plan = [];
  // Warm-ups are real independent elections, excluded from statistics. Rotate size order each round.
  for (const [phase, count] of [['warmup', config.warmups], ['measured', config.trials]])
    for (let trial = 0; trial < count; trial++)
      for (let j = 0; j < config.sizes.length; j++)
        plan.push({n: config.sizes[(trial + j) % config.sizes.length], phase, trial});
  return plan;
}

export function parseTime(stderr) {
  const rss = stderr.match(/(\d+)\s+maximum resident set size/);
  const cpu = stderr.match(/([\d.]+)\s+real\s+([\d.]+)\s+user\s+([\d.]+)\s+sys/);
  return {peak_rss_bytes: rss ? Number(rss[1]) : null,
    cpu_ms: cpu ? (Number(cpu[2]) + Number(cpu[3])) * 1000 : null};
}

export function sourceProvenance(source, bin, config) {
  assert.equal(git(source, 'rev-parse', 'HEAD'), config.source_commit);
  assert.equal(git(source, 'diff', '--stat', 'HEAD'), '', 'Upstream tracked source must remain unmodified');
  const tracked = execFileSync('git', ['-C', source, 'ls-files', '-z'], {encoding: 'utf8'}).split('\0').filter(Boolean);
  return {url: config.source_url, tag: config.release, commit: config.source_commit, license: config.license,
    git_tree: git(source, 'rev-parse', 'HEAD^{tree}'),
    source_tree_sha256: sha(tracked.sort().map(p => {
      const file = path.join(source, p);
      return `${sha(fs.lstatSync(file).isSymbolicLink() ? fs.readlinkSync(file) : fs.readFileSync(file))}  ${p}`;
    }).join('\n')),
    binary_sha256: sha(fs.readFileSync(bin)), binary_bytes: fs.statSync(bin).size,
    tracked_source_unchanged: true, compile_profile: config.compile_profile,
    files: Object.fromEntries(['COPYING', 'INSTALL.md', 'opam-bootstrap.sh', 'tests/tool/demo-threshold.sh',
      'src/tool/tool_election.ml', 'src/tool/setup.ml', 'src/lib/question/question_h.atd'].map(p => [p, sha(fs.readFileSync(path.join(source, p)))]))};
}

export function runElection({bin, config, spec, out, privateRoot}) {
  const label = `n${spec.n}-${spec.phase}-${String(spec.trial).padStart(2, '0')}`;
  const pub = path.join(out, label), priv = path.join(privateRoot, label);
  fs.mkdirSync(pub); fs.mkdirSync(priv, {mode: 0o700});
  const commands = [], start = performance.now();
  const w = workload(spec.n, spec.phase, spec.trial, config.workload_seed);
  write(path.join(pub, 'workload.json'), w);
  let seq = 0;
  const call = (stage, args, {input = '', reject = false} = {}) => {
    const t = performance.now();
    const r = spawnSync('/usr/bin/time', ['-l', bin, ...args], {cwd: priv, input, encoding: 'utf8',
      env: {...process.env, BELENIOS_USE_URANDOM: '1', BELENIOS_DEBUG: ''}, timeout: config.command_timeout_ms,
      maxBuffer: 16 * 1024 * 1024});
    const elapsed = performance.now() - t;
    const stderr = r.stderr ?? '', stdout = r.stdout ?? '';
    const id = String(seq++).padStart(4, '0');
    // Full stdout can contain credential/key material: retain it only inside the private runtime.
    write(path.join(priv, `${id}.stdout`), stdout);
    fs.appendFileSync(path.join(pub, 'stderr.log'), `\n[${id} ${stage}] ${JSON.stringify(args)}\n${stderr}`);
    const record = {id, stage, command: ['/usr/bin/time', '-l', '<pinned-belenios-tool>', ...args],
      elapsed_ms: elapsed, exit_code: r.status, signal: r.signal, error: r.error?.message ?? null,
      expected_rejection: reject, stdout_bytes: Buffer.byteLength(stdout), stdout_sha256: sha(stdout), ...parseTime(stderr)};
    commands.push(record); fs.appendFileSync(path.join(pub, 'commands.jsonl'), JSON.stringify(record) + '\n');
    assert.equal(Boolean(r.error), false, record.error ?? 'CLI execution error');
    if (reject) assert.notEqual(r.status, 0, 'Expected an upstream cryptographic/protocol rejection');
    else assert.equal(r.status, 0, `Belenios failed at ${stage}: ${stderr.slice(-2000)}`);
    return stdout;
  };
  const save = (name, data) => write(path.join(priv, name), data);
  const stageSum = stage => commands.filter(c => c.stage === stage && !c.expected_rejection).reduce((a, c) => a + c.elapsed_ms, 0);
  const add = (type, input) => call('board_acceptance', ['archive', 'add-event', `--type=${type}`], {input});
  const tt = (...args) => ['setup', 'generate-trustee-key-threshold', '--group', config.group, ...args];
  let result;
  try {
    const uuid = call('setup', ['setup', 'generate-token']).trim();
    assert.match(uuid, /^[A-Za-z0-9]+$/);
    save('voters.txt', w.voters.map(v => `${v.id}@example.invalid,${v.id},${v.weight}`).join('\n') + '\n');
    call('credentials', ['setup', 'generate-credentials', '--uuid', uuid, '--group', config.group, '--file', 'voters.txt']);
    const names = fs.readdirSync(priv);
    fs.renameSync(path.join(priv, names.find(n => n.endsWith('.pubcreds'))), path.join(priv, 'public_creds.json'));
    fs.renameSync(path.join(priv, names.find(n => n.endsWith('.privcreds'))), path.join(priv, 'private_creds.json'));
    const credentials = read(path.join(priv, 'private_creds.json'));
    assert.equal(Object.keys(credentials).length, spec.n);
    const ids = [];
    for (let i = 1; i <= config.threshold_trustees; i++)
      ids.push(call('dkg', tt('--threshold-context', `${i}/${config.threshold}/${config.threshold_trustees}`, '--step', '1')).trim());
    save('certs.jsons', ids.map(id => fs.readFileSync(path.join(priv, `${id}.cert`), 'utf8').trim()).join('\n') + '\n');
    call('dkg', tt('--certs', 'certs.jsons', '--step', '2'));
    save('polynomials.jsons', ids.map((id, i) => call('dkg', tt('--threshold-context', `${i + 1}/${config.threshold}/${config.threshold_trustees}`,
      '--certs', 'certs.jsons', '--key', `${id}.key`, '--step', '3')).trim()).join('\n') + '\n');
    call('dkg', tt('--certs', 'certs.jsons', '--step', '4', '--polynomials', 'polynomials.jsons'));
    const voutputs = ids.map(id => call('dkg', tt('--certs', 'certs.jsons', '--key', `${id}.key`, '--step', '5'),
      {input: fs.readFileSync(path.join(priv, `${id}.vinput`), 'utf8')}).trim());
    save('threshold.json', call('dkg', tt('--certs', 'certs.jsons', '--step', '6', '--polynomials', 'polynomials.jsons'), {input: voutputs.join('\n') + '\n'}));
    call('dkg', ['setup', 'generate-trustee-key', '--group', config.group]);
    const server = fs.readdirSync(priv).find(n => n.endsWith('.pubkey')).replace('.pubkey', '');
    save('public_keys.jsons', fs.readFileSync(path.join(priv, `${server}.pubkey`), 'utf8'));
    call('setup', ['setup', 'make-trustees']);
    save('questions.json', {description: 'Synthetic G11-C1 weighted election; no human voters', name: label,
      questions: [{answers: ['A', 'B', 'C'], min: 1, max: 1, question: 'Choose exactly one candidate'}]});
    call('setup', ['setup', 'make-election', '--uuid', uuid, '--group', config.group, '--template', 'questions.json']);
    call('setup', ['archive', 'init']);
    // Match the official demo: subsequent commands consume the archive, not loose setup files.
    for (const name of ['election.json', 'trustees.json', 'public_creds.json'])
      fs.renameSync(path.join(priv, name), path.join(priv, `${name}.archived`));
    const publicSetupDone = performance.now();
    let ciphertextBytes = 0, proofBytes = 0, ballotBytes = 0;
    for (const [i, voter] of w.voters.entries()) {
      assert.equal(typeof credentials[voter.id], 'string', `Missing credential for ${voter.id}`);
      save('credential-' + i, credentials[voter.id] + '\n');
      // Upstream load_from_file parses one JSON value per line, not pretty-printed multiline JSON.
      save('choice-' + i + '.json', JSON.stringify([[0, 1, 2].map(j => Number(j === voter.choice))]) + '\n');
      const ballot = call('ballot_generation', ['election', 'generate-ballot', '--privcred', 'credential-' + i, '--choice', 'choice-' + i + '.json']);
      save('ballot-' + i + '.json', ballot);
      const b = JSON.parse(ballot);
      assert.equal(b.answers.length, 1);
      ciphertextBytes += bytes(b.answers[0].choices);
      proofBytes += bytes({individual_proofs: b.answers[0].individual_proofs, overall_proof: b.answers[0].overall_proof, signature: b.signature});
      ballotBytes += Buffer.byteLength(ballot);
      call('single_ballot_verification', ['election', 'verify-ballot', '--ballot', 'ballot-' + i + '.json']);
      if (spec.phase === 'pilot' && i === 0) {
        const wrong = structuredClone(b);
        const p = wrong.answers[0].individual_proofs[0][0];
        p.response = (BigInt(p.response) + 1n).toString();
        save('invalid-ballot.json', JSON.stringify(wrong) + '\n');
        call('invalid_ballot', ['election', 'verify-ballot', '--ballot', 'invalid-ballot.json'], {reject: true});
      }
      add('Ballot', ballot);
    }
    call('all_ballot_audit', ['election', 'verify']);
    add('EndBallots', '');
    const encrypted = call('tally', ['election', 'compute-encrypted-tally']);
    add('EncryptedTally', encrypted);
    call('encrypted_tally_audit', ['election', 'verify']);
    let partialBytes = 0;
    for (let i = 0; i < config.threshold; i++) {
      const pd = call('threshold_decryption', ['election', 'decrypt-threshold', '--key', `${ids[i]}.key`, '--decryption-key', `${ids[i]}.dkey`, '--trustee-id', String(i + 2)]);
      partialBytes += Buffer.byteLength(pd); add('PartialDecryption', pd);
    }
    if (spec.phase === 'pilot')
      call('missing_mandatory_server', ['election', 'compute-result'], {reject: true});
    const serverPd = call('threshold_decryption', ['election', 'decrypt', '--privkey', `${server}.privkey`, '--trustee-id', '1']);
    partialBytes += Buffer.byteLength(serverPd); add('PartialDecryption', serverPd);
    const final = call('combine_and_decode', ['election', 'compute-result']);
    const tally = JSON.parse(final).result;
    assert.deepEqual(tally.map(row => row.map(Number)), [w.expected_tally], 'Weighted result differs from independent workload tally');
    add('Result', final);
    call('final_audit', ['election', 'verify']);
    const elapsed = performance.now() - start;
    const archive = path.join(priv, `${uuid}.bel`);
    fs.copyFileSync(archive, path.join(pub, 'election.bel'), fs.constants.COPYFILE_EXCL);
    write(path.join(pub, 'result.json'), JSON.parse(final));
    const peak = commands.every(c => c.peak_rss_bytes !== null) ? Math.max(...commands.map(c => c.peak_rss_bytes)) : null;
    const cpu = commands.every(c => c.cpu_ms !== null) ? commands.reduce((a, c) => a + c.cpu_ms, 0) : null;
    result = {
      gate: config.gate, label, uuid, ...spec, candidate_count: config.candidates,
      threshold: {pedersen: `${config.threshold}-of-${config.threshold_trustees}`, mandatory_server: 1, selected_trustee_ids: [2, 3, 4, 1]},
      expected_tally: w.expected_tally, actual_tally: tally, workload_sha256: sha(fs.readFileSync(path.join(pub, 'workload.json'))),
      ballot_count: spec.n, source_commit: config.source_commit,
      benchmark: new BenchmarkResult({system: 'Belenios 3.0 (official CLI, Ed25519)', kind: 'external_execution', status: 'passed',
        evidence: [`${label}/commands.jsonl`, `${label}/stderr.log`, `${label}/election.bel`, `${label}/result.json`],
        measurements: {setup_ms: stageSum('setup') + stageSum('credentials') + stageSum('dkg'),
          ballot_generation_ms: stageSum('ballot_generation'), verification_ms: stageSum('single_ballot_verification'),
          tally_ms: stageSum('tally'), decryption_ms: stageSum('threshold_decryption') + stageSum('combine_and_decode'),
          proof_bytes: proofBytes, ciphertext_bytes: ciphertextBytes, communication_bytes: fs.statSync(archive).size,
          storage_bytes: fs.statSync(archive).size, memory_peak_mib: peak === null ? null : peak / 2 ** 20,
          cpu_ms: cpu, throughput_votes_per_second: spec.n / (elapsed / 1000), end_to_end_ms: elapsed},
        missing_reason: ['proof_generation_ms is inseparable from encryption/internal verification in upstream generate-ballot',
          'aggregation_ms/state_proof_ms/p3_verification_ms/cp_package_bytes are not Belenios operations',
          'public_input_bytes: no Groth16-style public-input vector; archive and ballot wire bytes reported separately',
          'gas/calldata_bytes/transaction_count: not applicable, no blockchain',
          'memory is maximum single sequential CLI child RSS, not multi-host aggregate memory; CPU timer resolution is 10 ms']}),
      stage_ms: Object.fromEntries([...new Set(commands.map(c => c.stage))].map(s => [s, stageSum(s)])),
      end_to_end_excluding_setup_ms: start + elapsed - publicSetupDone,
      bytes: {ballot_wire: ballotBytes, ciphertext_json: ciphertextBytes, proof_and_signature_json: proofBytes,
        partial_decryption_wire: partialBytes, encrypted_tally_wire: Buffer.byteLength(encrypted),
        final_result_wire: Buffer.byteLength(final), public_archive: fs.statSync(archive).size},
      command_count: commands.length, negative_tests: commands.filter(c => c.expected_rejection).map(c => ({stage: c.stage, exit_code: c.exit_code})),
      timing_scope: 'Sequential local CLI lifecycle including startup, archive loading, timed command I/O and audits. Setup separate. No HTTP/network latency. Download/build excluded.',
      communication_scope: 'Public append-only .bel archive size, including tar/framing, setup, ballots, tally, partial decryptions and result; not private delivery or network traffic.',
      archive_sha256: sha(fs.readFileSync(archive)), private_material_exported: false,
    };
  } catch (e) {
    result = {gate: config.gate, label, ...spec, error: e.stack, command_count: commands.length,
      benchmark: new BenchmarkResult({system: 'Belenios 3.0', kind: 'external_execution', status: 'failed', evidence: [`${label}/commands.jsonl`, `${label}/stderr.log`],
        missing_reason: ['Failed election; no complete benchmark measurement. Partial command observations remain in commands.jsonl.']})};
  }
  write(path.join(pub, 'sample.json'), result);
  return result;
}

export function main(args) {
  const option = name => { const i = args.indexOf(name); assert.ok(i >= 0 && args[i + 1], name); return path.resolve(args[i + 1]); };
  assert.ok(args.includes('--pilot') !== args.includes('--suite'), 'Choose --pilot or --suite');
  const source = option('--source'), bin = option('--bin'), out = option('--out'), privateRoot = option('--private-root');
  assert.ok(!privateRoot.startsWith(path.resolve(ROOT) + path.sep), 'Private runtime must be outside the repository');
  const config = read(path.join(ROOT, 'configs/g11/c1-belenios.json'));
  fs.mkdirSync(out, {recursive: true}); fs.mkdirSync(privateRoot, {recursive: true, mode: 0o700});
  const provenance = sourceProvenance(source, bin, config);
  write(path.join(out, 'environment.json'), {observed_at: new Date().toISOString(), code_sha: git(ROOT, 'rev-parse', 'HEAD'),
    code_status: git(ROOT, 'status', '--porcelain=v1'), branch: git(ROOT, 'branch', '--show-current'), provenance, config,
    config_sha256: sha(fs.readFileSync(path.join(ROOT, 'configs/g11/c1-belenios.json'))),
    adapter_sha256: sha(fs.readFileSync(fileURLToPath(import.meta.url))),
    hardware: {cpu: os.cpus()[0]?.model, cores: os.cpus().length, memory_bytes: os.totalmem(), platform: os.platform(), arch: os.arch(), os_release: os.release()},
    node: process.version, process_timing: '/usr/bin/time -l; JS monotonic wall clock',
    independent_elections: true, crypto_rng: 'OS urandom; no deterministic key/ballot seed'});
  const results = [];
  for (const spec of schedule(config, args.includes('--pilot'))) {
    const sample = runElection({bin, config, spec, out, privateRoot}); results.push(sample);
    console.log(JSON.stringify({label: sample.label, status: sample.benchmark.status,
      ms: sample.benchmark.measurements.end_to_end_ms, tally: sample.actual_tally, error: sample.error}));
    if (sample.benchmark.status !== 'passed') break;
  }
  write(path.join(out, 'run.json'), {status: results.every(r => r.benchmark.status === 'passed') ? 'passed' : 'failed',
    planned: schedule(config, args.includes('--pilot')).length, completed: results.length,
    samples: results.map(r => `${r.label}/sample.json`), external_elections: results.length,
    new_verivoting_setup: 0, new_verivoting_proofs: 0, new_evm_transactions: 0});
  if (results.some(r => r.benchmark.status !== 'passed')) process.exitCode = 1;
}
if (path.resolve(process.argv[1] ?? '') === fileURLToPath(import.meta.url)) main(process.argv.slice(2));
