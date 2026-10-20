# VeriVoting-Batch VS Code experiment starter

This repository is an executable **native-core and P1** experiment derived from
the paper's V4.5 protocol description.  It is intentionally fail-closed: the
runner validates one versioned protocol manifest and does not report unfinished
SnarkPack, state-circuit, full-P3, multiprocess, or on-chain work as measurements.

## What is implemented

| Area | Implementation | Reported status |
|---|---|---|
| Canonical encoding and protocol manifest | Domain-separated, length-prefixed, big-endian encoding | READY |
| Registration authority | Pedersen weight commitments, duplicate checks, exact registered-weight close, Schnorr close certificate | READY |
| Threshold setup | Pedersen VSS/GJKR message phases, complaints, corrections, disqualification, Feldman extraction, quorum certificate | READY in one-process protocol simulation |
| Voting/tally | Weighted Ristretto ElGamal, homomorphic folding, all ten 3-of-5 combinations, bounded BSGS decoding | READY |
| Threshold-share proof | Generalized Chaum–Pedersen proof over the full ciphertext vector; challenge recomputed from the exact context | READY natively |
| Winner proof P1 | Real arkworks Groth16 proof of score bounds and smallest-ID tie breaking | READY |
| Pre-DKG binding | `policy_pre -> prekey -> DKG session -> KeyCore -> final context` | READY in native experiment |
| G0.5 CP feasibility | 253-bit optimized real CP equation; 3,461,084 constraints | BLOCKED by 1,000,000-constraint proof guard |
| G0.6 curve decision | Isolated Jubjub CP/ElGamal/state relations with real Groth16 | PARTIAL PASS; not a protocol migration or full P3 |
| Full P3 | Nested non-native CP verification and package binding inside Groth16 | BLOCKED |
| Ballot aggregation | Ballot circuit and SnarkPack integration | BLOCKED |
| State proof / chain | State R1CS, verifier contract, gas and DA measurements | BLOCKED |

The P1 proof is not a substitute for P3 because the P1 score witness is not yet
bound in-circuit to the committed threshold-decryption packages.  The public E2E
artifact therefore contains the winner but not the private tally or decryption
shares and has `full_p3_generated: false`.

## Reproduce

Prerequisites: a stable Rust toolchain and Python 3. No Foundry installation is
needed for the implemented native gates.

```bash
cargo test --workspace --release
cargo run --release -p vv-bench -- all --trials 1
python3 scripts/summarize.py experiments/raw experiments/aggregate
```

Or use VS Code task **VeriVoting: test and run core** / run:

```bash
./scripts/run_core.sh
```

Useful focused commands:

```bash
cargo run --release -p vv-bench -- manifest
cargo run --release -p vv-bench -- dkg --trials 3
cargo run --release -p vv-bench -- p1 --trials 3
cargo run --release -p vv-bench -- dlog
cargo run --release -p vv-bench -- e2e
cargo run -p vv-bench -- p3-feasibility --max-cp-constraints 1000000
cargo run -p vv-bench -- cp-optimize --cp-implementation canonical_253 --max-cp-constraints 1000000
cargo run -p vv-bench -- curve-compare --output experiments/raw --seed 20261020
python3 scripts/curve_compare.py experiments/raw
```

Raw results are written to `experiments/raw/`; the summary script writes
`experiments/aggregate/summary.{json,md}`. Runs append to `runs.csv`, so remove
or archive that file before a fresh paper measurement campaign.

## Fixed four-voter fixture

Weights `(3,2,1,4)` and choices `(A,B,A,C)` give the private tally `(4,2,4)`.
The public winner is candidate `1`, because the protocol selects the smallest
identifier on a tie. Every one of the ten legal 3-of-5 trustee subsets is
checked against this result.

## Gate interpretation

- G0 native manifest/core and P1 can be reproduced here.
- G0.5 reached a real single-equation R1CS preflight. The first optimization
  reduced 3,501,941 constraints to 3,461,084, still above the 1,000,000 limit,
  so Groth16 was not run.
- G0.6 measured an isolated native-field alternative: the CP equation used
  9,571 constraints and generated a real proof. It does not recompute the CP
  transcript hash and does not replace the official Ristretto manifest.
- G1 SnarkPack, G2 state proof, G3 real multiprocess networking, G4 full P3,
  and G5 target-chain verification remain blocked.
- A blocked gate is a development result, not a cryptographic benchmark value.
- These experiments test functional behavior and cost. They do not establish
  Function-Equal privacy or the paper's joint extraction theorem.

See [docs/IMPLEMENTATION_STATUS.md](docs/IMPLEMENTATION_STATUS.md) for the
claim boundary and [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md) for dependency
choices.

