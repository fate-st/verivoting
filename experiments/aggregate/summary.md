# VeriVoting-Batch measured summary

Build profile(s): `dev-unoptimized`

## Gate status

| Gate | Status |
|---|---|
| G0_native_core_and_p1 | PASS |
| G0.5_cp_single_equation | BLOCKED_RESOURCE_GUARD_NO_GROTH16_PROOF |
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
- Public artifact bytes: `1229`
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

## Bounded discrete log

| Bound | Decode ms | Decoded |
|---:|---:|---:|
| 1000 | 1.190 | 999 |
| 10000 | 3.219 | 9999 |
| 100000 | 10.707 | 99999 |
| 1000000 | 34.664 | 999999 |

> This report contains no fabricated values for blocked gates and does not claim full P3 security.
