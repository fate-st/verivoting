# VeriVoting-Batch measured summary

Build profile(s): `dev-unoptimized`

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
