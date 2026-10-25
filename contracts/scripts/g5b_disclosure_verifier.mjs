import assert from 'node:assert/strict';
import { runPragueGroth16Gate } from './prague_groth16_runner.mjs';

await runPragueGroth16Gate({
  gate: 'G5-B',
  status: 'PASS_G5B_LOCAL_PRAGUE_TWO_SUBSET_P3_GROTH16_VERIFIER',
  relation: 'G4-B two-subset complete P3 disclosure',
  fixture: 'fixtures/g4b_eip2537_fixture_v1.json',
  sourceRun: 'experiments/g5_b_raw/source_p3_groth16.json',
  output: 'experiments/g5_b_raw/p3_prague_evm.json',
  publicInputs: 60,
  changedIndex: 51,
  changedInputsKey: 'changed_selector_public_inputs_be_hex',
  negativeResultKey: 'wrong_selector',
  negativeVerifiedField: 'changed_public_selector_rejected',
  validateSource(source, fixture) {
    assert.equal(source.status, 'PASS_G4B_REAL_GROTH16_TWO_SUBSET_PUBLIC_SELECTOR');
    assert.equal(source.valid_second_subset_proof_verified, true);
    assert.equal(source.changed_public_selector_rejected, true);
    assert.equal(source.same_setup_parameters_used, true);
    assert.equal(source.manifest_unchanged, true);
    assert.equal(source.full_arbitrary_subset_p3, false);
    assert.equal(source.second_subset.public_inputs, fixture.public_input_count);
    assert.equal(source.second_subset.constraints, 6231973);
    assert.deepEqual(source.baseline.selected_trustees, [1, 2, 3]);
    assert.deepEqual(source.second_subset.selected_trustees, [1, 2, 4]);
    assert.deepEqual(fixture.supported_subsets, [[1, 2, 3], [1, 2, 4]]);
    assert.equal(fixture.selector_public_input_index, 51);
    assert.equal(BigInt(fixture.public_inputs_be_hex[51]), 1n);
    assert.equal(BigInt(fixture.changed_selector_public_inputs_be_hex[51]), 0n);
  },
  resultFields(fixture, source) {
    return {
      p3_evm_verified: true,
      p3_scope: 'two-subset complete P3 cryptographic relation only',
      supported_subsets: fixture.supported_subsets,
      setup_subset: fixture.setup_subset,
      proof_subset: fixture.proof_subset,
      same_setup_parameters_used: source.same_setup_parameters_used,
      public_winner: Number(BigInt(fixture.public_inputs_be_hex[59])),
      full_arbitrary_subset_p3: false,
      accepted_election_state_binding_checked: false,
    };
  },
  limitations: [
    'G4-B P3 relation supports only ordered subsets {1,2,3} and {1,2,4}',
    'the EVM verifier checks the P3 proof but does not compare its statement to an accepted election registry or frozen target',
    'authenticated commitment signature checks and write-once winner state are not implemented in this gate',
  ],
});
