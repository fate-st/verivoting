import assert from 'node:assert/strict';
import { runPragueGroth16Gate } from './prague_groth16_runner.mjs';

await runPragueGroth16Gate({
  gate: 'G5-A',
  status: 'PASS_G5A_LOCAL_PRAGUE_G2B_GROTH16_VERIFIER',
  relation: 'G2-B fixed-B2 three-candidate state transition',
  fixture: 'fixtures/g2b_eip2537_fixture_v1.json',
  sourceRun: 'experiments/g5_a_raw/source_state_groth16.json',
  output: 'experiments/g5_a_raw/g2b_prague_evm.json',
  publicInputs: 37,
  changedIndex: 7,
  changedInputsKey: 'changed_batch_root_public_inputs_be_hex',
  negativeResultKey: 'wrong_batch_root',
  negativeVerifiedField: 'changed_public_batch_root_rejected',
  validateSource(source, fixture) {
    assert.equal(source.valid_proof_verified, true);
    assert.equal(source.changed_public_batch_root_rejected, true);
    assert.equal(source.public_inputs, fixture.public_input_count);
    assert.equal(source.constraints, 1360002);
  },
  resultFields() { return { p3_evm_verified: false }; },
  limitations: ['G2-B fixed-B2 state verifier only; no SnarkPack or P3 verifier'],
});
