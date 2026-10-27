# VeriVoting-Batch measured summary

Build profile(s): `dev-unoptimized`

## Current G5 checkpoint

- Latest recorded gate: `G5-C3-B23` / `BLOCKED_QUARTER_SOFTWARE_MILLER_EXCEEDS_20M_FROZEN`
- Latest measured prefix: `G5-C3-B21`; latest actual full-loop attempt: `G5-C3-B23`
- A prefix pass does not establish full-loop conformance or a completed SnarkPack verifier; historical sections below retain their original scope.
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

Historical B17 was derived analysis only, with zero new EVM transactions; it did not remove the full-loop/SnarkPack blocker.

## Gate status

| Gate | Status |
|---|---|
| G0_native_core_and_p1 | PASS |
| G0.5_cp_single_equation | BLOCKED_RESOURCE_GUARD_NO_GROTH16_PROOF |
| G0.6_curve_decision | PARTIAL_PASS_ISOLATED_CURVE_EXPERIMENT_FULL_P3_BLOCKED |
| G0.8_jubjub_native | PASS_G08_NATIVE_SINGLE_PROCESS |
| G0.9_R_clean_baseline | PASS_G09R_CLEAN_BASELINE_SUITE_FROZEN |
| G0.9_H0_cp_transcript | PASS_H0_CANONICAL_TRANSCRIPT_R1CS_BOUND_NO_HASH |
| G0.9_H1_sha512 | PASS_H1_SHA512_R1CS_EXACT_DIGEST |
| G0.9_H2A_hash_to_scalar | PASS_H2A_COUNTER_0_1_FIRST_VALID_R1CS |
| G0.9_H2B_fixed_shape | PASS_H2B_FIXED_TWO_ATTEMPT_R1CS_FEASIBILITY |
| G0.9_H2C_wide_hash_to_field | PASS_H2C_WIDE_SHA512_MOD_FR_CANDIDATE_R1CS |
| G0.9_H2D_adopted_cp_hash_to_field | PASS_H2D_CP_H2F_V3_ADOPTED |
| G0.9_H3_first_cp_equation | PASS_H3_H2F_BOUND_FIRST_CP_EQUATION_R1CS |
| G0.9_H4A_complete_one_trustee_cp | PASS_H4A_COMPLETE_ONE_TRUSTEE_CP_VECTOR_R1CS |
| G0.9_H4B_one_trustee_cp_groth16 | PASS_H4B_REAL_GROTH16_ONE_TRUSTEE_CP |
| G0.9_P3A_three_selected_trustee_cp | PASS_P3A_THREE_SELECTED_TRUSTEE_CP_R1CS |
| G0.9_P3A_three_selected_trustee_cp_groth16 | PASS_P3A_REAL_GROTH16_THREE_SELECTED_TRUSTEE_CP |
| G0.9_P3B0_package_h2f_adoption | PASS_P3B0_PACKAGE_H2F_V1_ADOPTED |
| G0.9_P3B1_three_package_openings_r1cs | PASS_P3B1_THREE_PACKAGE_OPENINGS_R1CS |
| G0.9_P3C0_fixed_123_threshold_combination_r1cs | PASS_P3C0_FIXED_123_THRESHOLD_COMBINATION_R1CS |
| G0.9_P3C1_bounded_tally_binding_r1cs | PASS_P3C1_BOUNDED_TALLY_BINDING_R1CS |
| G0.9_P3D0_public_winner_linkage_r1cs | PASS_P3D0_PUBLIC_WINNER_LINKAGE_R1CS |
| G0.9_P3D1_fixed_123_complete_p3_groth16 | PASS_P3D1_REAL_GROTH16_FIXED_123_COMPLETE_P3 |
| G1_B0_weighted_ballot_core_r1cs | PASS_G1B0_WEIGHTED_BALLOT_CORE_R1CS |
| G1_B1_full_c_vote_r1cs | PASS_G1B1_FULL_C_VOTE_R1CS_FIXED_DEPTH2_FOUR_REGION |
| G1_B2_full_c_vote_groth16 | PASS_G1B2_REAL_GROTH16_FULL_C_VOTE_FIXED_DEPTH2_FOUR_REGION |
| G1_full_c_vote | PASS_G1B2_REAL_GROTH16_FULL_C_VOTE_FIXED_DEPTH2_FOUR_REGION |
| G1_ballot_groth16 | PASS_G1B2_REAL_GROTH16_FULL_C_VOTE_FIXED_DEPTH2_FOUR_REGION |
| G1_C0_snarkpack_compatibility | PASS_G1C0_REAL_SNARKPACK_V2_MIN_BATCH_2_NONPRODUCTION_SRS |
| G1_C1_distinct_voter_snarkpack | PASS_G1C1_REAL_SNARKPACK_V2_DISTINCT_VOTERS_BATCH_2_NONPRODUCTION_SRS |
| G1_snarkpack | PARTIAL_PASS_G1C1_DISTINCT_VOTERS_BATCH_2_NONPRODUCTION_SRS |
| G2_A_state_transition_r1cs | PASS_G2A_FIXED_B2_STATE_TRANSITION_R1CS |
| G2_B_state_transition_groth16 | PASS_G2B_REAL_GROTH16_FIXED_B2_STATE_TRANSITION |
| G2_state_proof | PASS_G2B_REAL_GROTH16_FIXED_B2_STATE_TRANSITION |
| G3_A_five_process_dkg_happy_path | PASS_G3A_FIVE_OS_PROCESS_DKG_HAPPY_PATH |
| G3_B_one_trustee_restart_recovery | PASS_G3B_ONE_TRUSTEE_RESTART_RECOVERY |
| G3_multiprocess | PARTIAL_PASS_G3B_ONE_RESTART_AUTHENTICATED_LOCAL_TRANSPORT |
| G4_A_second_subset_p3_shape | PASS_G4A_SECOND_SUBSET_COMMON_R1CS_COUNT_SHAPE |
| G4_B_public_two_subset_p3_groth16 | PASS_G4B_REAL_GROTH16_TWO_SUBSET_PUBLIC_SELECTOR |
| G4_general_subset_p3 | PARTIAL_PASS_G4B_PUBLIC_BOUND_TWO_SUBSETS_SAME_GROTH16_SETUP |
| G4_full_p3 | PARTIAL_PASS_G4B_TWO_SUBSET_COMPLETE_P3_NOT_ARBITRARY_3_OF_5 |
| G5_A_local_prague_state_verifier | PASS_G5A_LOCAL_PRAGUE_G2B_GROTH16_VERIFIER |
| G5_B_local_prague_two_subset_p3_verifier | PASS_G5B_LOCAL_PRAGUE_TWO_SUBSET_P3_GROTH16_VERIFIER |
| G5_C1_snarkpack_wire_and_runtime_prerequisites | PASS_G5C1_PUBLIC_WIRE_FIXTURE_AND_PRAGUE_PRIMITIVES |
| G5_C2_exact_internal_transcript_conformance | PASS_G5C2_OFFICIAL_INTERNAL_TRANSCRIPT_RUST_JS_CONFORMANCE |
| G5_C3_A_fp12_product_component | PASS_G5C3_FP12_PRODUCT_COMPONENT_ONLY |
| G5_C3_B_actual_tipp_endpoint_route | PARTIAL_PASS_TIPP_ENDPOINT_CONFORMANCE_EVM_TARGET_BLOCKED |
| G5_C3_B2_software_miller_loop_component | PARTIAL_PASS_MILLER_PREFIX_FULL_LOOP_OOG_20M |
| G5_C3_B3_memory_optimized_miller_component | PARTIAL_PASS_MILLER_MEMORY_REDUCED_FULL_LOOP_OOG_20M |
| G5_C3_B4_sparse_optimized_miller_component | PARTIAL_PASS_SPARSE_MILLER_PREFIX_FULL_LOOP_OOG_20M |
| G5_C3_B5_isolated_field_cost_profile | PASS_ISOLATED_FP_FP2_COST_PROFILE_ONLY |
| G5_C3_B6_low_level_field_comparison | PASS_LOW_LEVEL_FP_FIXED_CASES_AND_COST_REDUCTION_ONLY |
| G5_C3_B7_low_level_miller_integration | PARTIAL_PASS_LOW_LEVEL_MILLER_PREFIX_FULL_LOOP_OOG_20M |
| G5_C3_B8_fp2_allocation_comparison | PASS_FP2_FIXED_CASES_AND_COST_REDUCTION_ONLY |
| G5_C3_B9_fp2_miller_prefix_integration | PASS_FP2_MILLER_PREFIX_INTEGRATION_ONLY |
| G5_C3_B10_isolated_fp2_add_cost_profile | PASS_ISOLATED_FP2_ADD_COST_PROFILE_ONLY |
| G5_C3_B11_low_allocation_fp2_add_comparison | PASS_LOW_ALLOCATION_FP2_ADD_FIXED_CASES_ONLY |
| G5_C3_B12_add2_miller_prefix_integration | PASS_ADD2_MILLER_PREFIX_INTEGRATION_ONLY |
| G5_C3_B13_isolated_fp2_sub_cost_profile | PASS_ISOLATED_FP2_SUB_COST_PROFILE_ONLY |
| G5_C3_B14_low_allocation_fp2_sub_comparison | PASS_LOW_ALLOCATION_FP2_SUB_FIXED_CASES_ONLY |
| G5_C3_B15_sub2_miller_prefix_integration | PASS_SUB2_MILLER_PREFIX_INTEGRATION_ONLY |
| G5_C3_B16_latest_software_miller_full_attempt | BLOCKED_SOFTWARE_MILLER_SUB_LOWLEVEL_EXCEEDS_20M |
| G5_C3_B17_derived_miller_budget_review | PASS_DERIVED_MILLER_BUDGET_REVIEW_ONLY |
| G5_C3_B18_failed_full_attempt_cost_attribution | PASS_SAME_PROGRAM_FAILED_ATTEMPT_COST_ATTRIBUTION_ONLY |
| G5_C3_B19_source_and_algebra_route_review | PASS_SOURCE_DERIVED_MUL_QUARTER_ROUTE_REVIEW_ONLY |
| G5_C3_B20_isolated_fp_quarter_comparison | PASS_FP_QUARTER_FIXED_CASES_AND_WINDOW_REDUCTION_ONLY |
| G5_C3_B21_quarter_mul_miller_prefix_integration | PASS_QUARTER_MILLER_PREFIX_INTEGRATION_ONLY |
| G5_C3_B22_read_only_full_conformance_decision | PASS_DERIVED_SINGLE_FULL_CONFORMANCE_DECISION_ONLY |
| G5_C3_B23_quarter_full_conformance_attempt | BLOCKED_QUARTER_SOFTWARE_MILLER_EXCEEDS_20M_FROZEN |
| G5_C3_complete_gt_and_pairing_target_route | PARTIAL_PASS_QUARTER_PREFIX_FULL_MILLER_BLOCKED_FROZEN |
| G5_C_exact_snarkpack_target_verifier | BLOCKED_QUARTER_SOFTWARE_MILLER_EXCEEDS_20M_FROZEN |
| G5_target_chain | PARTIAL_PASS_G5B_LOCAL_STATE_AND_TWO_SUBSET_P3_VERIFIERS_FULL_SETTLEMENT_BLOCKED |

## Four-voter end-to-end fixture

- Public winner: `1`
- P1 Groth16 verified: `True`
- Full P3 generated: `False`
- Public tally present: `False`
- Legal 3-of-5 subsets checked: `10`
- Public artifact bytes: `1491`
- Private package bytes: `1551`

## G0.8 complete native Jubjub path

- Status: `PASS_G08_NATIVE_SINGLE_PROCESS`
- Build profile: `release-opt-level-1`
- Registered count / weight: `4 / 10`
- Packages created / verified / selected: `5 / 5 / 3`
- Public winner: `1`
- Public tally present: `False`
- Full native G0.8: `True`
- Full P3: `False`

## G0.9-R clean Release baseline

- Status: `PASS_G09R_CLEAN_BASELINE_SUITE_FROZEN`
- Git commit / dirty at run: `39cca56d730e97d136f3f998df1ab81eda9d82f4 / False`
- Workspace tests passed / failed / filtered: `50 / 0 / 1`
- Strict workspace Clippy: `PASS`
- Frozen Suite: `VERIVOTING-JUBJUB-PROTOTYPE-G0.8-v2`
- Next gate: `G0.9-H2`

## G0.9-H0 canonical CP transcript

- Status: `PASS_H0_CANONICAL_TRANSCRIPT_R1CS_BOUND_NO_HASH`
- Transcript bytes: `684`
- Transcript SHA-256: `b902f98b94fd9a5b04a73782a8ab6e73baa7e1525a20d49ee6d2dec8c8bd1d96`
- R1CS byte-binding constraints: `16416`
- Swapped registration fields rejected: `True`
- Changed registration digest rejected: `True`
- SHA-512 circuit implemented: `False`
- Full P3: `False`

## G0.9-H1 real SHA-512 R1CS

- Status: `PASS_H1_SHA512_R1CS_EXACT_DIGEST`
- Transcript bytes / padded blocks: `684 / 6`
- Constraints: `890920`
- Packed public inputs: `3`
- Witness synthesis: `776.481 ms`
- Peak RSS: `817.156 MiB`
- R1CS digest matches native: `True`
- Changed transcript / digest rejected: `True / True`
- Hash-to-scalar implemented: `False`
- CP equations integrated: `False`
- Groth16 run: `False`
- Full P3: `False`

## G0.9-H2-A exact hash-to-scalar R1CS

- Status: `PASS_H2A_COUNTER_0_1_FIRST_VALID_R1CS`
- Counter 0 attempts / constraints / witness: `1 / 1013462 / 876.486 ms`
- Counter 1 attempts / constraints / witness: `2 / 2019916 / 1741.488 ms`
- H2S input bytes / blocks per attempt: `806 / 7`
- Changed transcript / domain rejected: `True / True`
- False counter-1 claim rejected: `True`
- Uniform 128-attempt circuit: `False`
- CP equations / Groth16 / full H2 / full P3: `False / False / False / False`

## G0.9-H2-B fixed-shape H2S feasibility

- Status: `PASS_H2B_FIXED_TWO_ATTEMPT_R1CS_FEASIBILITY`
- Fixed attempts / Suite v2 maximum: `2 / 128`
- Counter 0 constraints / witness / peak RSS: `2020168 / 1821.592 ms / 1709.594 MiB`
- Counter 1 constraints / witness / peak RSS: `2020168 / 1716.996 ms / 1762.156 MiB`
- Same constraints / public inputs / witness variables: `True / True / True`
- False counter-1 claim rejected: `True`
- Suite v2 128-attempt relation: `False`
- CP equations / Groth16 / full H2 / full P3: `False / False / False / False`

## G0.9-H2-C wide SHA-512-to-field candidate

- Status: `PASS_H2C_WIDE_SHA512_MOD_FR_CANDIDATE_R1CS`
- Candidate suite: `VERIVOTING-JUBJUB-H2F-CANDIDATE-G0.9-v3`
- Mapping: `OS2IP_BE(SHA-512(canonical_input)) mod Jubjub-Fr`
- H2F input bytes / SHA-512 blocks: `808 / 7`
- Constraints / public inputs / witness variables: `1148545 / 2 / 1113725`
- Witness ms first / second: `1228.740 / 1160.321`
- Same constraints / public inputs / witness variables: `True / True / True`
- Changed transcript rejected: `True`
- Suite v2 changed / candidate adopted: `False / False`
- CP equations / Groth16 / full H2 / full P3: `False / False / False / False`

## G0.9-H2-D adopted CP-H2F-v3

- Status: `PASS_H2D_CP_H2F_V3_ADOPTED`
- Suite: `VERIVOTING-JUBJUB-CP-H2F-G0.9-v3`
- Constraints / public inputs / witness variables: `1151661 / 2 / 1116817`
- Witness / peak RSS: `1219.986 ms / 1196.922 MiB`
- Native vector / R1CS match: `True / True`
- Changed transcript rejected: `True`
- Full H2 / CP equation R1CS / Groth16 / full P3: `True / False / False / False`

## G0.9-H3 first CP equation

- Status: `PASS_H3_H2F_BOUND_FIRST_CP_EQUATION_R1CS`
- Constraints / public inputs / witness variables: `1168855 / 23 / 1130518`
- Witness / peak RSS: `1228.952 ms / 1283.797 MiB`
- Transcript points / group arithmetic constrained in R1CS: `True / True`
- Changed transcript / wrong VK / wrong response rejected: `True / True / True`
- First equation / vector equations / Groth16 / full P3: `True / False / False / False`

## G0.9-H4-A complete one-Trustee CP vector

- Status: `PASS_H4A_COMPLETE_ONE_TRUSTEE_CP_VECTOR_R1CS`
- CP equations / transcript points bound: `4 / 11`
- Constraints / public inputs / witness variables: `1207066 / 23 / 1164214`
- Witness / peak RSS: `1529.184 ms / 1270.656 MiB`
- Native first / coordinate equations: `True / 3`
- Changed transcript / wrong share / wrong coordinate rejected: `True / True / True`
- Complete one Trustee / multi-Trustee / Groth16 / full P3: `True / False / False / False`

## G0.9-H4-B real Groth16 for one-Trustee CP

- Status: `PASS_H4B_REAL_GROTH16_ONE_TRUSTEE_CP`
- Constraints / public inputs / witness variables: `1207066 / 23 / 1164214`
- Setup / prove / verify: `14025.404 / 8206.572 / 3.017 ms`
- Proof / proving key / verifying key: `192 / 380080848 / 1496 bytes`
- Peak RSS after setup / prove: `2686.656 / 2886.219 MiB`
- Valid proof / changed public transcript rejected: `True / True`
- Complete one Trustee / Groth16 / full P3: `True / True / False`

## G0.9-P3-A three selected Trustee CP relations

- Status: `PASS_P3A_THREE_SELECTED_TRUSTEE_CP_R1CS`
- Selected Trustees / native CP verified: `[1, 2, 3] / 3`
- Trustees / CP equations in R1CS: `3 / 12`
- Public prefix / private transcript / private response per Trustee: `452 / 232 / 32 bytes`
- Constraints / public inputs / witness variables: `3611598 / 45 / 3485994`
- Witness / peak RSS: `4876.174 ms / 3258.906 MiB`
- Valid relation / wrong third share rejected: `True / True`
- Private CP values emitted / Groth16 / package openings / full P3: `False / False / False / False`

## G0.9-P3-A real Groth16 for three selected Trustee CPs

- Status: `PASS_P3A_REAL_GROTH16_THREE_SELECTED_TRUSTEE_CP`
- Trustees / CP equations: `3 / 12`
- Constraints / public inputs / witness variables: `3611598 / 45 / 3485994`
- Setup / prove / verify: `36858.792 / 23581.980 / 5.349 ms`
- Proof / proving key / verifying key: `192 / 1037976624 / 2552 bytes`
- Peak RSS after setup / prove: `5182.062 / 5824.047 MiB`
- Valid proof / changed public prefix rejected: `True / True`
- Full P3-A / package openings / full P3: `True / False / False`

## G0.9-P3-B0 private-package H2F adoption

- Status: `PASS_P3B0_PACKAGE_H2F_V1_ADOPTED`
- Suite: `VERIVOTING-JUBJUB-PACKAGE-H2F-G0.9-v1`
- Selected Trustees: `[1, 2, 3]`
- Package / H2F input bytes / SHA-512 blocks: `602 / 725 / 6`
- Constraints / public inputs / witness variables per package: `1000827 / 2 / 967157`
- Total witness / peak RSS: `3030.297 ms / 1059.766 MiB`
- Fixed shape / native-R1CS match / changed package rejected: `True / True / True`
- Native package commitment migrated: `True`
- Package opening in P3 / Groth16 / full P3: `False / False / False`
- Native E2E packages created / verified / selected: `5 / 5 / 3`
- Native E2E selected Trustees / winner: `[1, 2, 3] / 1`
- Winner-event version / full P3: `VV/JUBJUB/WINNER-ONLY-EVENT/v4 / False`

## G0.9-P3-B1 three private-package openings

- Status: `PASS_P3B1_THREE_PACKAGE_OPENINGS_R1CS`
- Selected Trustees: `[1, 2, 3]`
- Native CP / packages verified: `3 / 3`
- CP / package-H2F / opening equations in R1CS: `12 / 3 / 3`
- Package / H2F input bytes / SHA-512 blocks: `602 / 725 / 6`
- Constraints / public inputs / witness variables: `6191178 / 51 / 5966673`
- Witness / peak RSS: `9234.655 ms / 5661.500 MiB`
- Valid relation / changed third commitment rejected: `True / True`
- Private packages / openings emitted: `False / False`
- Opening gate / Groth16 / threshold+tally / winner / full P3: `True / False / False / False / False`

## G0.9-P3-C0 fixed 1-2-3 threshold combination

- Status: `PASS_P3C0_FIXED_123_THRESHOLD_COMBINATION_R1CS`
- Selected Trustees / coefficients: `[1, 2, 3] / ['3', '-3', '1']`
- Native coefficients / threshold tally match: `True / True`
- CP / package-H2F / openings / threshold equations: `12 / 3 / 3 / 3`
- Public C1 / private plaintext group points: `3 / 3`
- Constraints / public inputs / witness variables: `6201342 / 57 / 5974593`
- Witness / peak RSS: `8407.870 ms / 5607.562 MiB`
- Valid relation / changed third plaintext rejected: `True / True`
- Private plaintext emitted: `False`
- Threshold gate / decoding / tally values / winner / Groth16 / full P3: `True / False / False / False / False / False`

## G0.9-P3-C1 bounded private tally binding

- Status: `PASS_P3C1_BOUNDED_TALLY_BINDING_R1CS`
- Selected Trustees / coefficients: `[1, 2, 3] / ['3', '-3', '1']`
- Registered weight: `10`
- CP / package-H2F / openings / threshold equations: `12 / 3 / 3 / 3`
- Tally group bindings / ranges / sum relations: `3 / 3 / 1`
- Constraints / public inputs / witness variables: `6203703 / 58 / 5976943`
- Witness / peak RSS: `8806.231 ms / 5590.266 MiB`
- Valid relation / sum-preserving changed tallies rejected: `True / True`
- Private plaintexts / tallies emitted: `False / False`
- Bounded tally gate / winner / Groth16 / full P3: `True / False / False / False`

## G0.9-P3-D0 public winner linkage

- Status: `PASS_P3D0_PUBLIC_WINNER_LINKAGE_R1CS`
- Selected Trustees / coefficients: `[1, 2, 3] / ['3', '-3', '1']`
- Registered weight / public winner: `10 / 1`
- CP / package-H2F / openings / threshold equations: `12 / 3 / 3 / 3`
- Tally bindings / ranges / sum: `3 / 3 / 1`
- Winner flags / pairwise relations: `3 / 6`
- Constraints / public inputs / witness variables: `6205629 / 59 / 5978859`
- Witness / peak RSS: `8566.464 ms / 5607.875 MiB`
- Valid relation / wrong public winner rejected: `True / True`
- Private plaintexts / tallies emitted: `False / False`
- Winner R1CS / Groth16 / full P3 proof: `True / False / False`

## G0.9-P3-D1 fixed-123 complete P3 Groth16

- Status: `PASS_P3D1_REAL_GROTH16_FIXED_123_COMPLETE_P3`
- Selected Trustees / coefficients: `[1, 2, 3] / ['3', '-3', '1']`
- Registered weight / public winner: `10 / 1`
- Constraints / public inputs / witness variables: `6205629 / 59 / 5978859`
- Witness / setup / prove / verify: `8633.805 / 64668.553 / 43604.668 / 6.019 ms`
- Proof / proving key / verifying key: `192 / 1837594176 / 3224 bytes`
- Peak RSS: `7779.359 MiB`
- Valid proof / wrong public winner rejected: `True / True`
- Proving key persisted / private tallies emitted: `False / False`
- Groth16 / fixed-123 full P3: `True / True`

## G1-B0 weighted-ballot cryptographic core

- Status: `PASS_G1B0_WEIGHTED_BALLOT_CORE_R1CS`
- Candidate count / registered-weight bound: `3 / 10`
- One-hot / weight-range / commitment-opening relations: `1 / 1 / 1`
- Weighted ElGamal group equations: `6`
- Constraints / public inputs / witness variables: `46573 / 17 / 39940`
- Witness / peak RSS: `108.856 ms / 49.156 MiB`
- Valid relation / two-hot rejected: `True / True`
- Ballot core / full C_vote / Groth16 / SnarkPack: `True / False / False / False`

## G1-B1 complete fixed-suite C_vote R1CS

- Status: `PASS_G1B1_FULL_C_VOTE_R1CS_FIXED_DEPTH2_FOUR_REGION`
- Five conditions / candidates / regions: `5 / 3 / 4`
- Registration leaves / depth: `4 / 2`
- PRFs / region hashes / Merkle hashes: `2 / 1 / 3`
- Constraints / public inputs / witness variables: `854454 / 25 / 839832`
- Witness / peak RSS: `777.431 ms / 754.859 MiB`
- Valid relation / changed context rejected: `True / True`
- Full C_vote R1CS / Groth16 / SnarkPack: `True / False / False`

## G1-B2 complete fixed-suite C_vote Groth16

- Status: `PASS_G1B2_REAL_GROTH16_FULL_C_VOTE_FIXED_DEPTH2_FOUR_REGION`
- Constraints / public inputs / witness variables: `854454 / 25 / 839832`
- Witness / setup / prove / verify: `777.431 / 8015.162 / 4623.984 / 2.386 ms`
- Proof / proving key / verifying key: `192 / 251898000 / 1592 bytes`
- Peak RSS: `1880.547 MiB`
- Valid proof / changed context rejected: `True / True`
- Full C_vote / Groth16 / SnarkPack: `True / True / False`

## G1-C0 minimum-batch SnarkPack compatibility

- Status: `PASS_G1C0_REAL_SNARKPACK_V2_MIN_BATCH_2_NONPRODUCTION_SRS`
- Implementation / version: `filecoin-project/bellperson / 0.27.0`
- Aggregate version / batch / public inputs per proof: `V2 / 2 / 25`
- Second prove / conversion / aggregation-SRS / aggregate / verify: `4446.743 / 4.760 / 4.344 / 4.003 / 3.153 ms`
- Individual total / aggregate proof: `384 / 5236 bytes`
- Converted individual proofs / aggregate / changed context rejected: `True / True / True`
- Peak RSS / production setup: `2134.141 MiB / False`

## G1-C1 distinct-registered-voter SnarkPack

- Status: `PASS_G1C1_REAL_SNARKPACK_V2_DISTINCT_VOTERS_BATCH_2_NONPRODUCTION_SRS`
- Batch / public inputs per proof: `2 / 25`
- Setup / first prove / second prove: `7292.883 / 4306.160 / 4443.555 ms`
- Aggregate / verify: `3.908 / 2.955 ms`
- Individual total / aggregate proof: `384 / 5236 bytes`
- Distinct statements / nullifiers / ciphertexts: `True / True / True`
- Aggregate valid / changed second nullifier rejected: `True / True`
- Peak RSS / production setup: `2516.266 MiB / False`

## G2-A fixed-B=2 state-transition R1CS

- Status: `PASS_G2A_FIXED_B2_STATE_TRANSITION_R1CS`
- Region / batch index / batch size: `4 / 1 / 2`
- Constraints / public inputs / witness variables: `1360002 / 37 / 1329560`
- Batch hashes / accumulator components / group additions: `3 / 6 / 12`
- Witness / peak RSS: `1145.101 ms / 1196.984 MiB`
- Same-region source / exact SnarkPack-input serialization: `True / True`
- Valid transition / substituted ciphertext rejected: `True / True`
- Groth16 run: `False`

## G2-B fixed-B=2 state-transition Groth16

- Status: `PASS_G2B_REAL_GROTH16_FIXED_B2_STATE_TRANSITION`
- Region / batch index / batch size: `4 / 1 / 2`
- Constraints / public inputs / witness variables: `1360002 / 37 / 1329560`
- Setup / prove / verify: `13254.145 / 8776.350 / 2.660 ms`
- Proof / proving key / verifying key: `192 / 419767248 / 2168 bytes`
- Peak RSS after setup / prove / verify: `2676.859 / 3090.062 / 3090.062 MiB`
- Valid proof / changed public batch root rejected: `True / True`
- Proving key persisted / private witness emitted: `False / False`

## G3-A five-process DKG happy path

- Status: `PASS_G3A_FIVE_OS_PROCESS_DKG_HAPPY_PATH`
- Trustee processes / unique PIDs / successful exits: `5 / 5 / True`
- Private state files / persisted: `5 / True`
- Authenticated deal / decryption messages: `25 / 3`
- DKG-to-final / vote-fold / threshold decrypt: `57.281 / 5.431 / 27.352 ms`
- Total wall / transport bytes: `155.093 ms / 39162 bytes`
- Certificate quorum / public-state agreement: `True / True`
- Selected Trustees / private tally / winner: `[1, 2, 3] / [4, 2, 4] / 1`
- Full G3: `False`

## G3-B one-Trustee restart recovery

- Status: `PASS_G3B_ONE_TRUSTEE_RESTART_RECOVERY`
- Logical Trustees / OS process instances: `5 / 6`
- Restarted Trustee / initial PID / resumed PID: `3 / 48827 / 48830`
- Restart observed / persisted state reused / authenticated resume record: `True / True / 1`
- DKG-to-final / vote-fold / threshold decrypt: `57.763 / 5.360 / 15.317 ms`
- Total wall / transport bytes: `135.047 ms / 39714 bytes`
- Public-state agreement / certificate quorum: `True / True`
- Selected Trustees / private tally / winner: `[1, 2, 3] / [4, 2, 4] / 1`
- Full G3: `False`

## G4-A second-subset complete-P3 R1CS shape

- Status: `PASS_G4A_SECOND_SUBSET_COMMON_R1CS_COUNT_SHAPE`
- Subsets: `[1, 2, 3] / [1, 2, 4]`
- Native tallies / winners: `[4, 2, 4] / 1` and `[4, 2, 4] / 1`
- Constraints: `6236532 / 6236532`
- Public inputs: `59 / 59`
- Witness variables: `6009006 / 6009006`
- Witness synthesis: `8831.783 / 7550.190 ms`
- Common count shape / public layout preserved: `True / True`
- Public coefficient binding / Groth16: `False / False`

## G4-B public two-subset complete-P3 Groth16

- Status: `PASS_G4B_REAL_GROTH16_TWO_SUBSET_PUBLIC_SELECTOR`
- Supported subsets: `[[1, 2, 3], [1, 2, 4]]`
- Setup subset / proof subset: `[1, 2, 3] / [1, 2, 4]`
- Constraints / public inputs / witness variables: `6231973 / 60 / 6005106`
- Witness synthesis (setup/proof assignment): `8415.777 / 7152.295 ms`
- Setup / prove / verify: `61062.475 / 38615.505 / 6.529 ms`
- Proof / proving key / verifying key: `192 / 1843893696 / 3272 bytes`
- Peak RSS after setup / prove / verify: `7604.453 / 8177.266 / 8177.266 MiB`
- Same setup parameters / valid proof: `True / True`
- Selector bound to transcript IDs / changed selector rejected: `True / True`
- Two-subset complete P3 / arbitrary-subset P3: `True / False`
- Manifest unchanged / proving key persisted: `True / False`

## G5-A local Prague state-proof Solidity verifier

- Status: `PASS_G5A_LOCAL_PRAGUE_G2B_GROTH16_VERIFIER`
- Runtime / compiler: `EthereumJS 10.1.3 / 0.8.30+commit.73712a01.Emscripten.clang`
- Hardfork / public inputs / IC points: `prague / 37 / 38`
- Valid transaction gas / execution gas: `940640 / 903280`
- Changed-root transaction gas / status: `940718 / 0`
- Deployment gas / runtime bytes: `4550491 / 1807`
- Compressed / EIP-2537 proof / logical proof+inputs: `192 / 512 / 1696 bytes`
- Actual verifier calldata / transaction DA: `1828 / 1828 bytes`
- Valid proof / changed root rejected: `True / True`
- Full batch DA / full settlement / SnarkPack EVM / P3 EVM: `False / False / False / False`

## G5-B local Prague two-subset P3 disclosure verifier

- Status: `PASS_G5B_LOCAL_PRAGUE_TWO_SUBSET_P3_GROTH16_VERIFIER`
- Runtime / compiler: `EthereumJS 10.1.3 / 0.8.30+commit.73712a01.Emscripten.clang`
- Public inputs / IC points / supported subsets: `60 / 61 / [[1, 2, 3], [1, 2, 4]]`
- Setup subset / proof subset / public winner: `[1, 2, 3] / [1, 2, 4] / 1`
- Valid transaction gas / execution gas: `1331473 / 1278293`
- Changed-selector transaction gas / status: `1331539 / 0`
- Deployment gas / runtime bytes: `6631812 / 1807`
- Compressed / EIP-2537 proof / logical proof+inputs: `192 / 512 / 2432 bytes`
- Actual verifier calldata: `2564 bytes`
- Valid proof / changed selector rejected: `True / True`
- P3 EVM / arbitrary-subset P3 / registry binding / full settlement: `True / False / False / False`

## G5-C1 SnarkPack public wire and Prague prerequisites

- Prerequisite gate: `PASS_G5C1_PUBLIC_WIRE_FIXTURE_AND_PRAGUE_PRIMITIVES`
- Exact target verifier: `BLOCKED_EXACT_SNARKPACK_V2_EVM_VERIFIER_NOT_IMPLEMENTED`
- Batch / public inputs per proof / aggregate bytes: `2 / 25 / 5236`
- Decoded native aggregate / changed second nullifier rejected: `True / True`
- External transcript Rust/JS match: `True`
- Runtime primitive probes: `4` (not aggregate verification)
- Exact SnarkPack EVM / aggregate verification gas: `False / None`
- Blocker: Pinned verifier requires GT/Fp12 operations and pairing-to-non-unit-GT comparison; EIP-2537 exposes only pairing-product-to-one boolean. No exact GT/transcript verifier exists in this repository.
- Next gate: G5-C2 exact internal transcript golden vectors and Rust/JavaScript challenge conformance before any Solidity GT port

## G5-C2 official internal transcript conformance

- Status: `PASS_G5C2_OFFICIAL_INTERNAL_TRANSCRIPT_RUST_JS_CONFORMANCE`
- Actual official calls matched: `12 / 12`
- Independent JS GT wire decodes per case: `15`
- Complete native valid / wrong nullifier rejected: `True / True`
- Full preimages / challenges / rejection counters match: `True / True / True`
- Real rejected attempts observed: `3`
- Setup / prove run: `False / False`
- Exact SnarkPack EVM / aggregate verification gas: `False / None`
- Next gate: G5-C3 one exact GT operation and pairing-to-target route feasibility; reuse this proof, no scale run

## G5-C3-A exact Fp12 product component in local Prague EVM

- Component status: `PASS_G5C3_FP12_PRODUCT_COMPONENT_ONLY`
- Native / independent JS / Solidity product match: `True`
- Valid / wrong-target transaction status: `1 / 0`
- Valid / wrong-target total gas: `609940 / 609708` (one multiplication component only)
- MODEXP calls per transaction / combined precompile execution gas: `108 / 21600`
- Calldata / runtime bytes: `2500 / 3423`
- Complete G5-C3 / pairing-to-GT / exact SnarkPack EVM: `False / False / False`
- Complete SnarkPack verification gas: `None`
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## G5-C3-B actual TIPP endpoint: host conformance, EVM route blocked

- Status: `PARTIAL_PASS_TIPP_ENDPOINT_CONFORMANCE_EVM_TARGET_BLOCKED`
- Native / independent JS endpoint equality: `True / True`
- Native / JS wrong in-subgroup GT target rejected: `True / True` (not EVM rejection)
- Independently recomputed C2 transcript calls: `6`
- Actual-point EVM identity probe: `70300` primitive gas, 32-byte false; not pairing-to-GT verification or transaction gas
- Pairing-to-GT EVM / EVM wrong-target rejection: `False / None`
- Resource conclusion: Implementation/API gap; no software-pairing OOG or impossibility claim has been established
- Blocker: No audited EVM Miller loop plus final exponentiation or validated equivalent relation in repository; the boolean precompile has no arbitrary GT target input/output
- Complete SnarkPack verification gas: `None`
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## G5-C3-B2 software Miller-loop component, fixed budget

- Status: `PARTIAL_PASS_MILLER_PREFIX_FULL_LOOP_OOG_20M`
- Prefix / full steps: `2 / 64`
- Prefix EVM match / wrong prefix rejected: `True / True`
- Prefix valid / wrong-result total gas: `4805196 / 4804731` (prefix component only)
- Prefix MODEXP calls / runtime bytes: `642 / 5675`
- Full-loop verified / exception: `False / out of gas`
- Full attempt gas spent / explicit transaction cap: `20000000 / 20000000` (failed-attempt spend is NOT completed-loop cost)
- Full attempt active memory words at exit: `85860`
- EVM final exponentiation / pairing-to-GT / SnarkPack: `False / False / False`
- Complete SnarkPack verification gas: `None`
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## G5-C3-B3 memory-only Miller optimization, same endpoint/budget

- Status: `PARTIAL_PASS_MILLER_MEMORY_REDUCED_FULL_LOOP_OOG_20M`
- Prefix valid / wrong result total gas: `2371368 / 2371186`
- Same two-step prefix gas: `4805196 -> 2371368` (50.65% reduction, not end-to-end speedup)
- Same prefix EVM active memory words: `35464 -> 7123` (79.91% reduction)
- Prefix MODEXP count unchanged: `642`; wrong result rejected: `True`
- Full-loop verified / exception / capped spend: `False / out of gas / 20000000` (not completed-loop cost)
- Full-attempt active memory words / executed MODEXP calls: `7203 / 5636`
- EVM final exponentiation / pairing-to-GT / full SnarkPack: `False / False / False`
- Complete SnarkPack verification gas: `None`
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## G5-C3-B4 dedicated Fp12 square / sparse-014 line product

- Status: `PARTIAL_PASS_SPARSE_MILLER_PREFIX_FULL_LOOP_OOG_20M`
- Same prefix gas B2 / B3 / B4: `4805196 / 2371368 / 1865022`
- Prefix reduction versus B3: `21.35%` (component-only, not end-to-end)
- Actual prefix MODEXP calls B3 -> B4: `642 -> 516`
- Prefix valid / wrong-result decision: `True / True`
- Specialized host reference matches: `63` squares and `69` line products; not full EVM conformance
- Full loop verified / exception / capped spend: `False / out of gas / 20000000` (not completed-loop cost)
- Full-attempt memory words / MODEXP calls: `5315 / 5556`
- Final exponentiation / pairing-to-GT / SnarkPack EVM: `False / False / False`
- Complete SnarkPack verification gas: `None`
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## G5-C3-B5 isolated Fp / Fp2 cost profile

- Status: `PASS_ISOLATED_FP_FP2_COST_PROFILE_ONLY`; no Miller/proving rerun
- Windows exclude decoding/encoding but include instrumentation and compiled-call overhead; not universal per-operation constants.

| Operation | Total tx gas | Execution gas | Arithmetic-window gas | MODEXP gas / calls | Window memory-expansion gas |
|---|---:|---:|---:|---:|---:|
| Fp multiply | 29813 | 6525 | 4372 | 400 / 2 | 211 |
| Fp2 multiply | 45399 | 20447 | 16468 | 1200 / 6 | 899 |

- Full returned products match the fixed public operands' direct BigInt and Noble reference products.
- Root opcode gas plus child MODEXP gas reconciles execution gas; call and memory counters are not mutually exclusive attribution.
- Full Miller run / exact SnarkPack / aggregate gas: `False / False / None`
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## G5-C3-B6 low-level Fp multiplication comparison

- Status: `PASS_LOW_LEVEL_FP_FIXED_CASES_AND_COST_REDUCTION_ONLY`; primitive-only, no Miller/proving rerun
- Same public operands, compiler and measurement windows as B5; only the _mul implementation changes.

| Operation | B5 window gas | B6 window gas | Reduction | B6 total tx gas | MODEXP gas / calls | Window allocator growth B5 -> B6 (B) |
|---|---:|---:|---:|---:|---:|---:|
| Fp multiply | 4372 | 2167 | 50.43% | 27606 | 400 / 2 | 2112 -> 384 |
| Fp2 multiply | 16468 | 9965 | 39.49% | 38887 | 1200 / 6 | 7872 -> 2816 |

- Deterministic arithmetic boundaries verified: `4 / 4`; excluded from the two cost-comparison samples.
- Complete outputs match direct BigInt and Noble; host-derived carry/borrow/odd-halving flags are not internal instruction coverage or an exhaustive arithmetic proof.
- Gas counters reconcile; isolated cost reductions are not measured Miller-loop or end-to-end gains.
- Full Miller run / exact SnarkPack / aggregate gas: `False / False / None`
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## G5-C3-B7 low-level Fp integration into sparse Miller

- Status: `PARTIAL_PASS_LOW_LEVEL_MILLER_PREFIX_FULL_LOOP_OOG_20M`
- Exact B6 _mul replaces B4 _mul; all other source is byte-identical except the contract name.
- Same-prefix total gas B4 -> B7: `1865022 -> 1290640` (30.80% reduction)
- Prefix active memory words B4 -> B7: `5233 -> 2829` (45.94% reduction)
- Prefix valid / wrong-target rejection: `True / True`; MODEXP calls unchanged: `516`
- Exactly one full-loop attempt; verified / root exception / gas spent: `False / out of gas / 20000000`
- Full-attempt active memory words / completed MODEXP calls: `3147 / 7848`; not a completed-step or completion-percentage measurement
- Completed full-Miller gas: `None`; failed capped spend is not full-loop cost
- EVM final exponentiation / pairing-to-GT / exact SnarkPack: `False / False / False`
- Complete SnarkPack verification gas: `None`; no setup/prove or scale runs
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## G5-C3-B8 isolated Fp2 allocation comparison

- Status: `PASS_FP2_FIXED_CASES_AND_COST_REDUCTION_ONLY`; only _mul2 changes, exact B6 _mul retained
- Same public cost operands and windows as B6; no Miller/proving rerun.

| Operation | B6 window gas | B8 window gas | Window reduction | B6 -> B8 total tx gas | B6 -> B8 allocator growth (B) | MODEXP gas / calls |
|---|---:|---:|---:|---:|---:|---:|
| Fp multiply | 2167 | 2167 | 0.00% | 27606 -> 27614 | 384 -> 384 | 400 / 2 |
| Fp2 multiply | 9965 | 8086 | 18.86% | 38887 -> 37022 | 2816 -> 1664 | 1200 / 6 |

- Necessary Fp2 boundaries verified: `3 / 3`; pure-imaginary square, near-modulus square and cross-limb sum/subtract case; not performance samples.
- Complete outputs match BigInt and Noble; host-derived branch flags are not instruction coverage or a formal arithmetic proof.
- Fp control window remains unchanged; its total transaction gas increases slightly outside the window, not an across-the-board cost reduction.
- Latest full-Miller attempt remains B7 at20M OOG; B8 does not promote any Miller/pairing/SnarkPack capability.
- Miller run / exact SnarkPack / aggregate gas: `False / False / None`
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## G5-C3-B9 Fp2 integration: prefix only, no full-loop retry

- Status: `PASS_FP2_MILLER_PREFIX_INTEGRATION_ONLY`; exact B8 _mul2 integrated into B7, other source unchanged except the contract name
- Same-prefix gas B7 -> B9: `1290640 -> 1132763` (12.23% reduction)
- Active memory words B7 -> B9: `2829 -> 2411` (14.78% reduction)
- Valid / wrong-result rejection: `True / True`; MODEXP calls unchanged: `516`
- Full-loop attempts in B9: `0`; new-variant full result: `None` (not tested, not a new OOG)
- Latest actual full attempt: `G5-C3-B7`; inherited target status source: `experiments/g5_c3_raw/miller_lowlevel_evm.json`
- B7 OOG is evidence for the previous variant; prefix savings do not establish this new variant's complete-loop correctness or gas cost.
- Completed full-Miller / SnarkPack gas: `None / None`; final exponentiation and pairing target remain unimplemented
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## G5-C3-B10 unchanged Fp2 addition cost localization

- Status: `PASS_ISOLATED_FP2_ADD_COST_PROFILE_ONLY`; arithmetic changed / optimization implemented: `False / False`
- One deployment and ONE fixed public addition; complete output matches BigInt/Noble. No extra tests or Miller/proving rerun.
- Instrumented arithmetic window / execution gas: `1752 / 5740`; not an inlined Miller constant
- Intrinsic / total / minimum gas including calldata floor: `24952 / 30880 / 30880`
- Calldata floor applied / extra gas: `True / 188`; pricing extra is NOT arithmetic-window cost
- Allocator growth / window memory-expansion gas: `896 B / 91`; not host RSS
- MODEXP calls / gas: `0 / 0`; root opcode gas reconciles execution
- Fixed operands exercise one ordinary component sum and one reduced sum with low-limb carry/borrow; host-derived conditions, not exhaustive instruction coverage.
- No speedup claim: this is a saved baseline. A future lower window need not lower this transaction's floor-limited total.
- At B10, latest prefix / full attempt were B9 / `G5-C3-B7`; aggregate gas `None`
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## G5-C3-B11 low-allocation Fp2 addition: isolated comparison only

- Status: `PASS_LOW_ALLOCATION_FP2_ADD_FIXED_CASES_ONLY`; only _add2 implementation changes, mathematical formula/codec unchanged
- Same-input window gas B10 -> B11: `1752 -> 849` (51.54% reduction)
- Execution gas B10 -> B11: `5740 -> 4852`
- Allocator growth B10 -> B11: `896 -> 384` B (57.14% reduction)
- Total transaction gas B10 -> B11: `30880 -> 30880`; floor extra `188 -> 1076`
- Execution/window savings do NOT lower this floor-limited transaction total; they are not measured Miller or E2E gains.
- One equality/zero boundary verified: `True`; excluded from the one performance comparison sample
- MODEXP calls / actual window memory-expansion gas: `0 / 39`; explicit Yul output size is not the full allocator growth
- Miller run / full-loop attempts / aggregate-verifier gas: `False / 0 / None`; B9/B7 history unchanged
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## G5-C3-B12 low-allocation Fp2 addition integrated into the Miller prefix

- Status: `PASS_ADD2_MILLER_PREFIX_INTEGRATION_ONLY`; exact B11 _add2 integrated, other source bodies unchanged
- Same-prefix total gas B9 -> B12: `1132763 -> 1043835` (7.85% reduction)
- Active memory B9 -> B12: `2411 -> 2079` words (13.77% reduction)
- Successful MODEXP calls / gas: `516 / 103200`; unchanged operation count
- Wrong canonical target rejected: `True`
- Runtime / deployment gas: `6992 B / 1565064`; both slightly higher than B9
- New-variant full attempts / full attempt: `0 / None`; latest actual full attempt remains `G5-C3-B7` on an older variant
- Full Miller / final exponentiation / exact SnarkPack verified: `False / False / False`; aggregate gas `None`
- Prefix savings are measured only for the saved two-step input; full-loop cost is unknown for B12.
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## G5-C3-B13 unchanged Fp2 subtraction: one cost baseline

- Status: `PASS_ISOLATED_FP2_SUB_COST_PROFILE_ONLY`; arithmetic changed / optimization implemented: `False / False`
- Arithmetic transactions / complete canonical result verified: `1 / True`
- Arithmetic window / execution / intrinsic gas: `1286 / 5296 / 24952`
- Total / calldata-floor supplement: `30880 / 632` gas; supplement is outside the arithmetic window
- Allocator growth / window memory-expansion gas / exit active memory: `640 B / 65 / 79 words`
- MODEXP calls / STATICCALL opcode gas: `0 / 0`
- Fixed input uses direct subtraction in both components, with a low-limb borrow in c0 only; modular-wrap branch is not covered.
- B13 is a profiling baseline. Candidate comparisons are recorded separately below; this window is not an inlined Miller cost.
- Latest prefix / actual full attempt: `experiments/g5_c3_raw/miller_add_prefix_evm.json / G5-C3-B7`
- Miller run / full-loop attempts / aggregate gas: `False / 0 / None`
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## G5-C3-B14 low-allocation Fp2 subtraction: isolated comparison

- Status: `PASS_LOW_ALLOCATION_FP2_SUB_FIXED_CASES_ONLY`; only _sub2 implementation changes, all other function bodies unchanged
- Same-input window gas B13 -> B14: `1286 -> 765` (40.51% reduction)
- Execution gas B13 -> B14: `5296 -> 4790`
- Allocator growth B13 -> B14: `640 -> 384` B (40.00% reduction)
- Window memory-expansion gas B13 -> B14: `65 -> 39`
- Total transaction gas B13 -> B14: `30880 -> 30880`; calldata-floor supplement `632 -> 1138`
- Execution/window savings do not lower the floor-limited transaction total and do not establish Miller or E2E savings.
- One modular-wrap boundary verified: `True`; `(0,0) - (1,2^256) = (p-1,p-2^256)`, excluded from performance comparison
- Arithmetic transactions / performance samples / MODEXP calls per input: `2 / 1 / 0`
- Miller run / full-loop attempts / aggregate gas: `False / 0 / None`; at B14, latest prefix was B12 and actual full attempt B7
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## G5-C3-B15 low-allocation Fp2 subtraction integrated into the Miller prefix

- Status: `PASS_SUB2_MILLER_PREFIX_INTEGRATION_ONLY`; exact B14 _sub2 integrated, all other B12 source bodies unchanged
- Same-prefix total gas B12 -> B15: `1043835 -> 985557` (5.58% reduction)
- Active memory B12 -> B15: `2079 -> 1905` words (8.37% reduction)
- Successful MODEXP calls / gas: `516 / 103200`; count and trace unchanged
- Complete saved prefix / wrong target rejected: `True / True`
- Runtime / deployment gas: `7106 B / 1589758`; increases versus B12 are 114 B / 24,694 gas
- At B15, full attempts / full attempt were `0 / None`; its historical full source was `G5-C3-B7`
- This is a prefix-only result. The subsequent full-only B16 record determines the current full-loop status.
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## G5-C3-B16 unchanged B15 contract: one full64 attempt

- Status: `BLOCKED_SOFTWARE_MILLER_SUB_LOWLEVEL_EXCEEDS_20M`; same source/runtime, compiler, fixture and 20M transaction cap
- Full attempts / prefix checks executed this run: `1 / False`
- Actual receipt / exception / capped-or-actual total gas: `0 / out of gas / 20000000`
- Execution / intrinsic gas: `19963388 / 36612`; zero refund
- Successful / failed MODEXP calls: `10628 / 0`; child execution gas `2125600`
- Active memory at exit: `1945` words; last observed root step `{'pc': 5374, 'opcode': 'SHR', 'gas_left_before': 2, 'memory_words_before': 1945}`
- Full-loop conformance / completed-loop gas: `False / None`
- Failed capped spend is not a completed-loop cost; call counts do not measure completion percentage.
- At B16 the latest actual full attempt was B16; previous full attempt was `G5-C3-B7`. B15 prefix evidence is retained.
- EVM final exponentiation / exact SnarkPack verified: `False / False`; aggregate gas `None`
- Next gate: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## G5-C3-B17 saved-evidence budget review: derived analysis, no EVM run

- Status: `PASS_DERIVED_MILLER_BUDGET_REVIEW_ONLY`; new EVM transactions: `0`
- Source-map compilation reproduces the saved runtime: `7106` B / `69912b7444e21d7f8f562795063895da8243f1be400bf3333b2b925b0db58e92`
- Terminal source: PC `5374`, `Bls12381MillerLoopSubLowLevel.sol:355`, `shr(1, hi)`; terminal trigger only, not a cumulative hotspot
- Saved successful MODEXP child gas / observed execution: `2125600 / 19963388` (10.6475%)
- Execution gas not attributed to those children: `17837788`; not all field arithmetic
- Memory expansion at the saved watermark: `13223` gas (0.06624%); this is a subset, not an additional cost category
- Decision: hold arithmetic changes until one cumulative opcode/source attribution run supplies cost rankings; defer a wrapper requiring a new verifier relation.
- At B17, latest actual full / prefix were `G5-C3-B16 / G5-C3-B15`; full and aggregate gas remained null
- Next gate: B18: ONE same-runtime, same-calldata, same-20M attribution run; reconcile charged root-opcode/source bins, MODEXP children and terminal gas burn before proposing any arithmetic change (planned, not executed in B17)

## G5-C3-B18 same-program failed-full-attempt cost attribution

- Status: `PASS_SAME_PROGRAM_FAILED_ATTEMPT_COST_ATTRIBUTION_ONLY`; B16 reproduction: `True`
- Exactly `2` transactions: one deployment and one unchanged full64 at 20M; no prefix/negative rerun
- Charged root opcodes + successful MODEXP children + terminal burn: `17837786 + 2125600 + 2 = 19963388`; difference `0`
- Root function mapping coverage: `77.58%`; unmapped/generated/scaffolding remain explicit
- Exclusive root function ranking (not inclusive caller cost): `_mul` 8051757 gas (45.14%), `<unmapped>` 3993601 gas (22.39%), `_add2` 2155414 gas (12.08%), `_mul2` 1502129 gas (8.42%), `_sub2` 939062 gas (5.26%)
- Opcode/function/leaf rankings are alternative partitions of the same root gas, not additive costs.
- At B18 full attempt / prefix: `G5-C3-B18 / G5-C3-B15`; full-loop and exact SnarkPack remained BLOCKED with null completed-loop/aggregate gas
- Next gate: B19: pause arithmetic changes; read-only route decision from this attribution, without a full retry or wrapper implementation

## G5-C3-B19 saved-evidence/source/algebra route decision: no EVM run

- Status: `PASS_SOURCE_DERIVED_MUL_QUARTER_ROUTE_REVIEW_ONLY`; EVM transactions / arithmetic changes: `0 / False`
- Source-owned rank bound: _mul `8051757` > next mapped + all unassigned `6154522`; margin `1897235`
- At B19 one candidate was selected: fuse the two modular halvings in _mul into quarter(t) = (t + (t & 3)*p)/4 for this fixed p mod 4 = 3; B19 itself measured no implementation or gas improvement.
- Original halfMod source bucket is only `6.0557%` of charged root gas; no claim that this solves the full20M budget.
- At B19, B20 was planned, not executed: `1` deployment, `5` isolated Fp multiplies (one performance input plus four correction/carry cases). No Miller, Fp2 or extra matrix.
- If exact results or isolated-window improvement fail, stop the candidate; prefix integration and any full retry require separate gates.
- At B19 latest actual full / prefix remained `G5-C3-B18 / G5-C3-B15`; target status was `BLOCKED_SOFTWARE_MILLER_SUB_LOWLEVEL_EXCEEDS_20M` with null full/aggregate gas.
- Next gate: B20: one isolated Fp _mul comparison replacing only the two halfMod calls with the fixed-p quarterMod; one saved operand performance case plus four correction/carry correctness cases; no Miller run

## G5-C3-B20 isolated fixed-p quarterMod multiplication comparison

- Status: `PASS_FP_QUARTER_FIXED_CASES_AND_WINDOW_REDUCTION_ONLY`; correctness/accounting: `True`; only _mul helper/calls and contract name differ from B6
- Actual EVM transactions / arithmetic calls / performance samples: `6 / 5 / 1`; four correction/carry cases excluded from performance sample
- Same-input window B6 -> B20: `2167 -> 2023`; change `6.65%` reduction
- Same-input execution / total gas B6 -> B20: `4318 -> 4174` / `27606 -> 27462`
- Allocator growth B6 -> B20: `384 -> 384` B; MODEXP count / calldata unchanged: `True / True`
- Runtime B6 -> B20: `2519 -> 2491` B; deployment gas `597983 -> 591914`
- Four boundaries verified / candidate accepted for a separate prefix gate: `True / True`
- No Fp2 or Miller executed at B20. At that gate full / passing prefix remained `G5-C3-B18 / G5-C3-B15`; full/aggregate gas were null and target status BLOCKED.
- Next gate: B21: integrate ONLY the exact measured quarterMod _mul into a separate B15-based Miller variant; run the same prefix positive/wrong-result pair only; no full64 retry

## G5-C3-B21 exact quarterMod _mul integrated into the original Miller prefix

- Status: `PASS_QUARTER_MILLER_PREFIX_INTEGRATION_ONLY`; all other B15 source bytes unchanged except contract name
- Exactly `3` transactions: one deployment and the original prefix positive/wrong-target pair; full attempts `0`
- Positive total gas B15 -> B21: `985557 -> 951873` (3.42% reduction)
- Active memory B15 -> B21: `1905 -> 1905` words; MODEXP calls `516 -> 516`
- Exact prefix / wrong target rejection / accounting: `True / True / True`
- Runtime bytes B15 -> B21: `7106 -> 7078`; deployment gas `1589758 -> 1583679`
- At B21 target status was `BLOCKED_QUARTER_MILLER_FULL_VARIANT_UNTESTED`; latest actual full was historical `G5-C3-B18` on a DIFFERENT variant, not a measured B21 full failure
- No full64, boundary rerun, final exponentiation or capability promotion; full/aggregate gas and completion percentage remain null.
- Next gate: B22: read-only stop/continue decision for the saved quarter-prefix variant; no automatic full64 retry, final exponentiation or new primitive optimization

## G5-C3-B22 read-only full-conformance decision: zero EVM transactions

- Status: `PASS_DERIVED_SINGLE_FULL_CONFORMANCE_DECISION_ONLY`; EVM transactions / source changes: `0 / False`
- Decision: one bounded full64 conformance attempt on the saved quarter runtime, then stop software arithmetic/full retries on failure; not a prediction that 20M will suffice.
- Reason: B21 covers two steps, not the complete loop/arena use or terminal negative-x conjugation; B18 OOG is a different runtime.
- At B22, B23 was planned, NOT executed: `1` deployment + `1` full positive; transaction/block cap `20000000 / 30000000`
- Planned success: exact `768` B full output and `14316` successful MODEXPs, zero failures and reconciled fees; only then report completed-loop gas.
- OOG/other failure: save the actual outcome, retain null completed-loop gas and stop; do not relabel other failures as OOG or automatically increase the cap.
- No prefix/boundary rerun, negative matrix, final exponentiation, proof generation or new profiling; full Miller/complete SnarkPack remain unverified.
- At B22 measured prefix / actual full remained `G5-C3-B21 / G5-C3-B18`; target status was `BLOCKED_QUARTER_MILLER_FULL_VARIANT_UNTESTED`
- Next gate: B23: ONE full64 positive on the exact saved B21 source/runtime/compiler and frozen full fixture at the same20M cap; one deployment, no prefix/boundary rerun; on failure stop software arithmetic and full retries

## G5-C3-B23 same-runtime quarter full64 conformance attempt

- Status / classification: `BLOCKED_QUARTER_SOFTWARE_MILLER_EXCEEDS_20M_FROZEN / OOG`
- Actual EVM transactions / full attempts: `2 / 1`; no prefix or boundary rerun
- Complete full-loop verified / measured completed-loop gas: `False / None`
- Fixed-budget software automatic optimization/retries frozen: `True`; this does not freeze unrelated project work
- Receipt / actual exception / actual total gas: `0 / out of gas / 20000000`
- Execution + intrinsic = total: `19963388 + 36612 = 20000000`; refund `0`
- Successful / failed MODEXP calls and child gas: `11076 / 0 / 2215200`
- Active EVM memory at exit: `1945` words; post-execution state `{'program_counter': 6119, 'last_opcode_byte': 87, 'gas_left_after_execution': 0, 'interpretation': 'post-execution VM state, not a pre-charge step event or cumulative source profile'}`
- Failed capped spend is not completed-loop gas; MODEXP counts are not a completion percentage. No source/step profile or wall-time speedup comparison is claimed.
- Target status: `BLOCKED_QUARTER_SOFTWARE_MILLER_EXCEEDS_20M_FROZEN`; final exponentiation/GT binding/exact SnarkPack remain unverified
- Next step: Fixed20M software route frozen: read-only evidence/claim alignment and prioritization of other unfinished paper experiments; no arithmetic change, full retry, cap increase or wrapper implementation

## Real P1 Groth16

- Constraints: `3521`
- Proof bytes: `192`
- Setup: `1018.377 ms`
- Prove: `279.053 ms`
- Verify: `28.299 ms`
- Altered public winner rejected: `True`

## G0.5 CP feasibility

- Status: `BLOCKED_RESOURCE_GUARD_NO_GROTH16_PROOF`
- Real native CP verified: `True`
- R1CS witness satisfied: `True`
- Constraints: `3501941`
- Public inputs: `324`
- Peak RSS: `8018.9 MiB`
- Groth16 setup/prove/verify run: `False`

## G0.5 optimization comparison

| Implementation | Constraints | Public inputs | Witness ms | Peak RSS MiB | Groth16 run |
|---|---:|---:|---:|---:|---|
| baseline_256_bit_joint | 3501941 | 324 | 174531.438 | 8179.4 | False |
| canonical_253_bit_joint | 3461084 | 324 | 169906.703 | 8240.4 | False |

## G0.6 isolated SNARK-friendly curve experiment

- Status: `PARTIAL_PASS_ISOLATED_CURVE_EXPERIMENT_FULL_P3_BLOCKED`
- Build profile: `dev-unoptimized`
- Experiment group: `Jubjub/ark-ed-on-bls12-381-0.4.0`
- Native manifest changed: `False`
- CP challenge recomputed in-circuit: `False`
- Full P3: `False`

| Component | Constraints | Public inputs | Setup ms | Prove ms | Verify ms | Proof bytes |
|---|---:|---:|---:|---:|---:|---:|
| single_cp_equation | 9571 | 5 | 2893.210 | 2931.179 | 34.082 | 192 |
| one_elgamal_coordinate | 12538 | 6 | 3409.576 | 3535.100 | 39.645 | 192 |
| one_homomorphic_point_addition | 5867 | 6 | 1959.694 | 2260.702 | 39.460 | 192 |

## Bounded discrete log

| Bound | Decode ms | Decoded |
|---:|---:|---:|
| 1000 | 1.190 | 999 |
| 10000 | 3.219 | 9999 |
| 100000 | 10.707 | 99999 |
| 1000000 | 34.664 | 999999 |

> This report contains no fabricated values for blocked gates and does not claim arbitrary-subset or production full-P3 security.
