import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { traceSnarkpackB2 } from './snarkpack_transcript_g5c2.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
const command = (program, args, options = {}) => execFileSync(program, args, {
  cwd: root, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024, ...options,
});
const cargo = (args, options) => command('cargo', ['+1.99.1', ...args], options);
const fixtureName = 'fixtures/snarkpack_g5c1_public_v1.json';
const fixtureBytes = fs.readFileSync(path.join(root, fixtureName));
const fixture = JSON.parse(fixtureBytes);
const lockBytes = fs.readFileSync(path.join(root, 'Cargo.lock'));
const metadata = JSON.parse(cargo(['metadata', '--offline', '--locked', '--format-version', '1']));
const dependency = metadata.packages.find((p) => p.name === 'bellperson' && p.version === '0.27.0');
assert.ok(dependency, 'pinned bellperson 0.27.0 dependency not found');
const dependencyDir = path.dirname(dependency.manifest_path);
const relativeTranscript = 'src/groth16/aggregate/transcript.rs';
const relativeVerifier = 'src/groth16/aggregate/verify.rs';
const referenceTranscript = fs.readFileSync(path.join(dependencyDir, relativeTranscript), 'utf8');
const referenceVerifier = fs.readFileSync(path.join(dependencyDir, relativeVerifier));
assert.equal(sha256(referenceTranscript), '4ae0d7da2ecc4a2bcf1eafe0cec2209ce0fde7c27bf377e6c6e782bff87b7ac8');
assert.equal(sha256(referenceVerifier), 'cd6412860def9630b47a58a96c7245dc4a714e0b847dd43780858412eabe578d');

// Preserve the installed dependency. Instrument ONLY a fresh temporary copy;
// the actual verifier, SHA state, bincode and rejection decisions are unchanged.
const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'verivoting-g5c2-'));
const copiedDependency = path.join(scratch, 'bellperson');
fs.cpSync(dependencyDir, copiedDependency, { recursive: true });
// A path dependency resolves inactive optional dependencies too. The upstream
// CUDA-supraseal branch conflicts with blstrs' pinned CPU blst=0.3.10. Remove
// ONLY that inactive dependency in the temporary manifest, never upgrade blst.
const sourceManifest = fs.readFileSync(path.join(dependencyDir, 'Cargo.toml'), 'utf8');
let cpuManifest = sourceManifest.replace('cuda-supraseal = ["supraseal-c2"]', 'cuda-supraseal = []');
const optionalSection = '[dependencies.supraseal-c2]\nversion = "0.1.0"\noptional = true\n';
assert.equal(cpuManifest.split(optionalSection).length, 2);
cpuManifest = cpuManifest.replace(optionalSection, '');
fs.writeFileSync(path.join(copiedDependency, 'Cargo.toml'), cpuManifest);
let tracedSource = referenceTranscript;
function replaceOnce(before, after) {
  assert.equal(tracedSource.split(before).length, 2, 'reference instrumentation anchor changed');
  tracedSource = tracedSource.replace(before, after);
}
replaceOnce('    buffer: Vec<u8>,\n',
  '    buffer: Vec<u8>,\n    trace_tag: String,\n    trace_bytes: Vec<u8>,\n');
replaceOnce('            buffer: Vec::new(),\n',
  '            buffer: Vec::new(),\n            trace_tag: application_tag.to_owned(),\n' +
  '            trace_bytes: [PREFIX.as_bytes(), application_tag.as_bytes()].concat(),\n');
replaceOnce('        self.buffer.clear();\n',
  '        self.trace_bytes.extend_from_slice(&self.buffer);\n        self.buffer.clear();\n');
replaceOnce('        Challenge(r)\n', `        if let Ok(path) = std::env::var("VV_G5C2_TRACE_OUT") {
            let encode = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
            let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path).unwrap();
            std::io::Write::write_fmt(&mut file, format_args!("{}|{}|{}|{}\\n",
                self.trace_tag, encode(&self.trace_bytes), counter_nonce,
                encode(r.to_repr().as_ref()))).unwrap();
        }
        Challenge(r)
`);
fs.writeFileSync(path.join(copiedDependency, relativeTranscript), tracedSource);
assert.deepEqual(fs.readFileSync(path.join(copiedDependency, relativeVerifier)), referenceVerifier);
const manifest = `[package]
name = "verivoting-g5c2-reference"
version = "0.0.0"
edition = "2021"
[workspace]
members = ["."]
exclude = ["bellperson"]
resolver = "2"
[[bin]]
name = "g5c2-reference-replay"
path = ${JSON.stringify(path.join(root, 'crates/vv-friendly-dkg/src/bin/g5c2_native_replay.rs'))}
[dependencies]
bellperson = { path = ${JSON.stringify(copiedDependency)}, version = "=0.27.0", default-features = false, features = ["groth16"] }
blstrs = "=0.7.0"
rand_chacha = "=0.3.1"
serde_json = "=1.0.154"
vv-types = { path = ${JSON.stringify(path.join(root, 'crates/vv-types'))} }
[profile.release]
opt-level = 1
strip = "none"
`;
const manifestPath = path.join(scratch, 'Cargo.toml');
fs.writeFileSync(manifestPath, manifest);
fs.writeFileSync(path.join(scratch, 'Cargo.lock'), lockBytes);
const tracePath = path.join(scratch, 'official-verifier-trace.tsv');
console.log('G5-C2: replaying saved proof with logging-only pinned reference; no setup/prove');
cargo(['run', '-j', '2', '--release', '--manifest-path', manifestPath, '--',
  '--fixture', fixtureName, '--output', 'experiments/g5_c2_raw/native_replay.json'], {
  stdio: 'inherit',
  env: { ...process.env, VV_G5C2_TRACE_OUT: tracePath,
    CARGO_TARGET_DIR: path.join(root, 'target') },
});
const nativeBytes = fs.readFileSync(path.join(root, 'experiments/g5_c2_raw/native_replay.json'));
const native = JSON.parse(nativeBytes);
assert.equal(native.instrumented_reference, true);
assert.equal(native.valid_aggregate_verified, true);
assert.equal(native.changed_second_nullifier_rejected, true);
assert.equal(native.counter_bytes, 8);
assert.equal(native.setup_run, false);
assert.equal(native.prove_run, false);
assert.equal(native.fixture_sha256, sha256(fixtureBytes));
// Inspect only the active normal/build graph. Full cargo metadata would also
// fetch unrelated third-party dev/benchmark dependencies from the path crate.
const referenceTree = cargo(['tree', '--offline', '--locked', '--manifest-path', manifestPath,
  '-e', 'normal,build', '--prefix', 'none', '--format', '{p}|{f}',
  '-p', 'verivoting-g5c2-reference']);
const referencePackages = referenceTree.trim().split('\n').map((line) => {
  const [packageInfo, features] = line.split('|');
  const [, name, version] = packageInfo.match(/^(\S+) v(\S+)/);
  return { name, version, features: features.replace(/ \(\*\)$/, '').split(',').filter(Boolean) };
});
for (const [name, version] of Object.entries({
  bellperson: '0.27.0', blstrs: '0.7.0', blst: '0.3.10', bincode: '1.3.3', sha2: '0.10.9',
})) {
  assert.ok(referencePackages.some((p) => p.name === name && p.version === version),
    `pinned reference dependency missing: ${name} ${version}`);
}
const baselineFeatures = metadata.resolve.nodes.find((n) => n.id === dependency.id).features.toSorted();
const referenceFeatures = referencePackages.find((p) => p.name === 'bellperson').features.toSorted();
assert.deepEqual(referenceFeatures, baselineFeatures, 'CPU verifier enabled features changed');
assert.ok(referenceFeatures.every((f) => !['cuda', 'opencl', 'cuda-supraseal'].includes(f)));
assert.deepEqual(fs.readFileSync(path.join(root, fixtureName)), fixtureBytes,
  'reference replay changed the saved public proof fixture');
assert.deepEqual(fs.readFileSync(path.join(root, 'Cargo.lock')), lockBytes,
  'temporary reference run changed the project lockfile');
assert.equal(sha256(fs.readFileSync(path.join(dependencyDir, relativeTranscript))), sha256(referenceTranscript),
  'installed dependency source changed');

const officialRows = fs.readFileSync(tracePath, 'utf8').trim().split('\n').map((line) => {
  const [tag, base_hex, counter, challenge_le_hex] = line.split('|');
  return { tag, base_hex, counter: Number(counter), challenge_le_hex };
});
assert.equal(officialRows.length, 12, 'expected six reference calls for each of the two cases');
const validJs = traceSnarkpackB2(fixture.aggregate_proof_hex, fixture.external_transcript_hex);
const wrongJs = traceSnarkpackB2(fixture.aggregate_proof_hex, fixture.changed_external_transcript_hex);
const allJs = [...validJs, ...wrongJs];
for (const [i, derived] of allJs.entries()) {
  const official = officialRows[i];
  assert.equal(derived.tag, official.tag, `tag mismatch at call ${i}`);
  assert.equal(derived.base_hex, official.base_hex, `bincode/GT bytes mismatch at call ${i}`);
  assert.equal(derived.attempts.length, official.counter, `rejection counter mismatch at call ${i}`);
  assert.equal(derived.challenge_le_hex, official.challenge_le_hex, `scalar mismatch at call ${i}`);
}
const changedIds = validJs.flatMap((trace, i) =>
  trace.challenge_le_hex === wrongJs[i].challenge_le_hex ? [] : [trace.id]);
assert.deepEqual(changedIds, ['random-r', 'gipa-0-seed', 'gipa-0-round', 'extra-link', 'random-z']);
const rejectedAttempts = allJs.reduce((sum, trace) => sum + trace.attempts.filter((a) => !a.accepted).length, 0);
assert.ok(rejectedAttempts > 0, 'the saved real fixture should exercise rejection sampling');

const golden = {
  schema: 'VV/G5-C2/SNARKPACK-INTERNAL-TRANSCRIPT/v1',
  fixture: fixtureName, fixture_sha256: sha256(fixtureBytes),
  aggregate_proof_sha256: fixture.aggregate_proof_sha256,
  reference_transcript_sha256: sha256(referenceTranscript), reference_verifier_sha256: sha256(referenceVerifier),
  reference_instrumented_transcript_sha256: sha256(tracedSource),
  original_dependency_manifest_sha256: sha256(sourceManifest),
  temporary_dependency_manifest_sha256: sha256(cpuManifest),
  reference_enabled_features: referenceFeatures,
  counter_bytes: 8, valid: validJs, changed_second_nullifier: wrongJs,
  official_verifier_calls: officialRows,
  scope: 'B=2, 25 inputs per source proof, V2, 64-bit bellperson counter ABI; not full SnarkPack EVM verification',
};
const goldenBytes = Buffer.from(JSON.stringify(golden, null, 2) + '\n');
fs.writeFileSync(path.join(root, 'fixtures/snarkpack_g5c2_transcript_v1.json'), goldenBytes);
const report = {
  gate: 'G5-C2', status: 'PASS_G5C2_OFFICIAL_INTERNAL_TRANSCRIPT_RUST_JS_CONFORMANCE',
  recorded_at_utc: new Date().toISOString(),
  git_commit: command('git', ['rev-parse', 'HEAD']).trim(),
  git_dirty: command('git', ['status', '--porcelain']).trim().length > 0,
  cpu: os.cpus()[0].model, os: os.platform(), arch: os.arch(), node_version: process.version,
  reference_version: 'bellperson 0.27.0 / blstrs 0.7.0',
  javascript_group_library: '@noble/curves 2.4.0; installed locked dependency',
  fixture: fixtureName, fixture_sha256: sha256(fixtureBytes),
  golden_fixture: 'fixtures/snarkpack_g5c2_transcript_v1.json', golden_fixture_sha256: sha256(goldenBytes),
  native_replay_sha256: sha256(nativeBytes), cargo_lock_sha256: sha256(lockBytes),
  reference_transcript_sha256: sha256(referenceTranscript), reference_verifier_sha256: sha256(referenceVerifier),
  reference_instrumented_transcript_sha256: sha256(tracedSource),
  original_dependency_manifest_sha256: sha256(sourceManifest),
  temporary_dependency_manifest_sha256: sha256(cpuManifest),
  reference_enabled_features: referenceFeatures,
  reference_active_dependency_tree_sha256: sha256(referenceTree),
  driver_sha256: sha256(fs.readFileSync(fileURLToPath(import.meta.url))),
  javascript_decoder_sha256: sha256(fs.readFileSync(path.join(root,
    'contracts/scripts/snarkpack_transcript_g5c2.mjs'))),
  native_replay_source_sha256: sha256(fs.readFileSync(path.join(root,
    'crates/vv-friendly-dkg/src/bin/g5c2_native_replay.rs'))),
  temporary_reference_lock_sha256: sha256(fs.readFileSync(path.join(scratch, 'Cargo.lock'))),
  source_fixture_unchanged: true, installed_dependency_unchanged: true, project_lock_unchanged: true,
  batch_size: 2, public_inputs_per_proof: 25, aggregate_proof_bytes: native.aggregate_proof_bytes,
  official_call_count_per_case: 6, matched_call_count: allJs.length,
  gt_elements_independently_decoded_per_case: 15,
  valid_challenges: validJs.map((t) => ({ id: t.id, payload_bytes: t.payload_bytes,
    attempts: t.attempts.length, challenge_le_hex: t.challenge_le_hex })),
  changed_challenge_ids: changedIds, rejected_attempts_observed: rejectedAttempts,
  official_complete_native_valid: native.valid_aggregate_verified,
  official_complete_native_changed_nullifier_rejected: native.changed_second_nullifier_rejected,
  rust_js_preimages_match: true, rust_js_challenges_match: true, rust_js_rejection_counters_match: true,
  setup_run: false, prove_run: false, exact_snarkpack_evm_verified: false,
  snarkpack_verification_gas: null, full_settlement_measured: false,
  target_verifier_status: 'BLOCKED_GT_ARITHMETIC_AND_PAIRING_TO_GT_EVM_ROUTE_NOT_IMPLEMENTED',
  next_gate: 'G5-C3 one exact GT operation and pairing-to-target route feasibility; reuse this proof, no scale run',
  limitations: [
    'Transcript conformance is host-language verification, not a Solidity/EVM implementation or a security proof',
    'Temporary reference copy changes logging only; official verify.rs and installed source remain unchanged',
    'Temporary manifest omits the inactive CUDA-supraseal dependency to preserve CPU blst=0.3.10; enabled verifier features match the original',
    'Only the fixed B=2/64-bit transcript ABI is covered; no arbitrary-batch or platform-independence claim',
    'Native randomized pairing batching still uses deterministic functional-test randomness',
    'No official Manifest capability or protocol suite is promoted',
  ],
};
fs.writeFileSync(path.join(root, 'experiments/g5_c2_raw/transcript_conformance.json'),
  JSON.stringify(report, null, 2) + '\n');
console.log(`G5-C2 PASS: ${allJs.length}/12 official calls match; ${rejectedAttempts} real rejected attempts; native wrong-nullifier rejected`);
console.log('Exact SnarkPack EVM verification remains BLOCKED; no aggregate gas reported.');
