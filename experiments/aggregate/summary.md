# VeriVoting-Batch measured summary

Build profile(s): `dev-unoptimized`

## Gate status

| Gate | Status |
|---|---|
| G0_native_core_and_p1 | PASS |
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
- Setup: `1040.726 ms`
- Prove: `291.942 ms`
- Verify: `28.508 ms`
- Altered public winner rejected: `True`

## Bounded discrete log

| Bound | Decode ms | Decoded |
|---:|---:|---:|
| 1000 | 1.166 | 999 |
| 10000 | 3.144 | 9999 |
| 100000 | 9.680 | 99999 |
| 1000000 | 32.170 | 999999 |

> This report contains no fabricated values for blocked gates and does not claim full P3 security.
