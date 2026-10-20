# VeriVoting-Batch measured summary

Build profile(s): `dev-unoptimized`

## Gate status

| Gate | Status |
|---|---|
| G0_native_core_and_p1 | PASS |
| G0.5_cp_single_equation | BLOCKED_RESOURCE_GUARD_NO_GROTH16_PROOF |
| G0.6_curve_decision | PARTIAL_PASS_ISOLATED_CURVE_EXPERIMENT_FULL_P3_BLOCKED |
| G1_snarkpack | BLOCKED_NOT_INTEGRATED |
| G2_state_proof | BLOCKED_CIRCUIT_NOT_IMPLEMENTED |
| G3_multiprocess | BLOCKED_SINGLE_PROCESS_SIMULATION_ONLY |
| G4_full_p3 | BLOCKED_NON_NATIVE_NESTED_CP_NOT_IMPLEMENTED |
| G5_target_chain | BLOCKED_CONTRACTS_NOT_IMPLEMENTED |

## Four-voter end-to-end fixture

- Public winner: `1`
- P1 Groth16 verified: `True`
- Full P3 generated: `False`
- Public tally present: `False`
- Legal 3-of-5 subsets checked: `10`
- Public artifact bytes: `1491`
- Private package bytes: `1551`

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

> This report contains no fabricated values for blocked gates and does not claim full P3 security.
