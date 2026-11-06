import assert from 'node:assert/strict';

export const METRICS = [
  'setup_ms', 'ballot_generation_ms', 'proof_generation_ms', 'aggregation_ms',
  'verification_ms', 'state_proof_ms', 'tally_ms', 'decryption_ms', 'p3_verification_ms',
  'proof_bytes', 'public_input_bytes', 'ciphertext_bytes', 'communication_bytes', 'storage_bytes',
  'cp_package_bytes', 'memory_peak_mib', 'cpu_ms', 'throughput_votes_per_second',
  'end_to_end_ms', 'gas', 'calldata_bytes', 'transaction_count',
];
export class ElectionConfig {
  constructor(value) {
    assert.ok(value && typeof value.election_id === 'string' && value.election_id.length);
    for (const key of ['voter_count', 'candidate_count', 'batch_size'])
      assert.ok(Number.isSafeInteger(value[key]) && value[key] > 0, key);
    assert.ok(value.batch_size <= value.voter_count);
    assert.equal(value.weights?.length, value.voter_count);
    assert.ok(value.weights.every(x => Number.isSafeInteger(x) && x > 0));
    assert.ok(Number.isSafeInteger(value.weights.reduce((a, b) => a + b, 0)));
    assert.equal(typeof value.rule, 'string');
    assert.ok(value.security_parameters?.suite);
    Object.assign(this, structuredClone(value));
  }
}
export class WorkloadConfig {
  constructor(value) {
    assert.equal(typeof value.seed, 'string'); assert.ok(value.seed.length > 0);
    assert.ok(['uniform', 'moderate', 'skewed', 'explicit'].includes(value.distribution));
    for (const key of ['warmups', 'trials']) assert.ok(Number.isSafeInteger(value[key]) && value[key] >= 0);
    assert.equal(typeof value.independent_elections, 'boolean');
    Object.assign(this, structuredClone(value));
  }
}
export class BenchmarkResult {
  constructor({system, kind, status, evidence, measurements = {}, missing_reason}) {
    assert.ok(['framework', 'literature', 'native_execution', 'external_execution'].includes(kind));
    assert.ok(['passed', 'failed', 'timeout', 'not_executed', 'partial_reproduction'].includes(status));
    assert.ok(typeof system === 'string' && Array.isArray(evidence));
    for (const [key, value] of Object.entries(measurements)) {
      assert.ok(METRICS.includes(key), key);
      assert.ok(value === null || (Number.isFinite(value) && value >= 0), key);
    }
    if (['framework', 'literature'].includes(kind) || ['not_executed', 'failed', 'timeout'].includes(status))
      assert.ok(Object.values(measurements).every(x => x === null), 'No invented protocol metrics');
    const metrics = Object.fromEntries(METRICS.map(key => [key, measurements[key] ?? null]));
    if (Object.values(metrics).some(x => x === null)) assert.ok(missing_reason?.length);
    Object.assign(this, {schema_version: 'g11.v1', system, kind, status, evidence, measurements: metrics, missing_reason});
  }
}
export function csv(rows, headers) {
  const field = x => { const s = x == null ? '' : typeof x === 'object' ? JSON.stringify(x) : String(x);
    return /[",\n\r]/.test(s) ? `"${s.replaceAll('"', '""')}"` : s; };
  return [headers, ...rows.map(row => headers.map(key => row[key]))].map(row => row.map(field).join(',')).join('\n') + '\n';
}
