# Implementation and claim boundary

## Implemented experiment path

1. Validate `configs/protocol_manifest.json` and hash its canonical encoding.
2. Run the 5-trustee, threshold-3 DKG phases using real Ristretto group
   operations. The adversarial DKG experiment injects one bad private share and
   one inconsistent Feldman broadcast.
3. Register the four fixed weights, validate Pedersen openings and the exact
   registered-weight sum, then sign the close record.
4. Encrypt the fixed choices with weighted additive ElGamal and fold ballots.
5. Produce and natively verify generalized Chaum–Pedersen decryption-share
   proofs bound to `eid`, final context, pre-policy, disclosure digest, target,
   trustee id, verification key, all ciphertext bases, and all shares.
6. Commit to each private package and sign its public commitment.
7. Combine threshold shares and decode bounded tallies with BSGS. Check all ten
   3-of-5 subsets.
8. Generate and verify a real Groth16 P1 proof for bounds and deterministic
   winner selection. Change the public winner and confirm verification fails.
9. Serialize a winner-only public event without the tally or private packages.

## G0.5 CP-circuit feasibility result

The next required experiment was run once with a real native generalized
Chaum--Pedersen transcript. The native proof verified, and the first circuit
equation `z*G = A0 + d*VK` was satisfied by the corresponding witness.

This is a preflight result, not a Groth16 proof and not P3:

- constraints: `3,501,941`;
- public inputs: `324`;
- peak RSS in the dev build: `8,018.9 MiB`;
- configured proof-run guard: `1,000,000` constraints;
- Groth16 setup/prove/verify: not run because the preflight exceeded the guard.

The dominant component is non-native double-scalar multiplication
(`3,484,879` constraints). The necessary next action is to reduce that cost;
P2/P3-a is not started from this result. See `docs/P3_FEASIBILITY.md` and
`experiments/raw/p3-feasibility.json`.

The first sound optimization used the canonical 253-bit scalar bound and
reduced the result to `3,461,084` constraints. This remains blocked by the same
resource guard; see `docs/P3_OPTIMIZATION_REPORT.md`.

## Pre-DKG binding order

The native experiment now freezes `policy_pre` before DKG, derives the pre-key
context and unique DKG session from it, includes those values in the DKG
transcript and KeyCore, and derives the final context after certification.
Protocol version `0.3-PREPOLICY-V2` and registration version `2` identify the
new encoding. Policy replacement, stale-context, cross-election replay and
stale-session tests pass.

## Deliberately unimplemented

- Ballot-validity Groth16 and actual SnarkPack aggregation.
- Regional state R1CS and recursive/linked state proof.
- Full P3. G0.5 implements only a resource-guarded R1CS preflight for the first
  CP equation; compressed Ristretto binding, in-circuit transcript hashing,
  the vector equation, private-package openings and signatures remain absent.
- Five OS processes with authenticated transport and crash/restart persistence.
- Solidity verifier contracts, Foundry tests, gas, calldata and DA costs.

No synthetic measurements are emitted for these items. Their manifest states
start with `BLOCKED_`, and `validate_full_submission()` refuses the manifest.

## Security interpretation

The code is research experiment code, not an audited voting implementation.
Passing tests means the implemented relations and fault cases behaved as
specified for those tests. It is not evidence for universal cryptographic
security, privacy of an operational election, or production readiness.

