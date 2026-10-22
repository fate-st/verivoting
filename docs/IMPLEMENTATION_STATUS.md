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

## G0.6 curve-decision result

An isolated Jubjub/BLS12-381 prototype compiled one CP equation (`9,571`
constraints), one ElGamal coordinate (`12,538`), and one homomorphic point
addition (`5,867`). All three produced and verified real 192-byte Groth16
proofs; changed public inputs and invalid relations were rejected. Peak process
RSS was `101.3 MiB` in the development-profile run.

This did not change the manifest or native Ristretto protocol. The challenge is
a canonical public input but SHA-512 is not recomputed in-circuit, and the
ElGamal/state rows are component experiments rather than full ballot or state
proofs. See `docs/P3_CURVE_DECISION.md` and
`experiments/raw/p3_curve_compare.csv`.

## G0.7 Release reproduction

The Release workspace build, strict Clippy and required regression tests now
pass with `CARGO_PROFILE_RELEASE_STRIP=none`, which avoids a host
`rust-objcopy --strip-debug` stall without changing optimized code generation.
The initial five-trial P1 baseline and four-voter Ristretto E2E passed. See
`docs/G07_RELEASE_BASELINE.md`.

The benchmark CSV writer now records the actual Git SHA, dirty state, manifest
suite, CPU, memory, Rust version and build profile for new runs. Historical CSV
files were not rewritten.

## G0.8 native Jubjub progress

The independent `vv-friendly-crypto` crate now freezes Jubjub Suite v2. Its
SHA-512 rejection sampler masks candidates to 252 bits before the strict
canonical scalar check; public keys and ciphertext points cross a canonical
subgroup-validation boundary; cryptographic randomness requires `CryptoRng`;
and BSGS uses checked arithmetic and a documented table limit.

The `vv-friendly-dkg` crate implements a single-process, real-group Pedersen
VSS/GJKR experiment with five pre-policy-authenticated trustees, complaint
correction, Feldman extraction, a three-signature KeyCore certificate and
threshold-3 decryption. Native generalized Chaum--Pedersen now proves each
Trustee's three decryption-share equations. The verifier recomputes the exact
Suite v2 challenge and returns a type that alone is accepted by the verified
threshold-combination path.

All five Trustee proofs verify. Every one of the ten legal 3-of-5 subsets uses
only verified shares and decrypts `(4,2,4)` with winner `1`; the aggregate
secret key is never reconstructed. G0.8-E1 additionally creates five private
CP packages, commits to each package with a fresh Pedersen opening, signs each
public commitment with the corresponding pre-policy authentication key,
verifies all five packages, then deterministically selects Trustees 1--3.
Only the selected verified-share type reaches threshold decryption. The public
event contains winner `1`, contexts, package commitments and signatures, but
no tally, shares, CP proofs or openings.

G0.8-E2 adds a Jubjub RA key and registration policy that are frozen into
`policy_pre` before DKG. Four Pedersen weight commitments are checked for
unique credentials and exact integer sum; the RA signs a close record bound to
the final context. The verified close and disclosure digests enter the tally
target, generalized CP transcript and private-package commitment. The Release
run closes count 4 and weight 10, verifies 5/5 packages, selects Trustees 1--3
and emits winner 1 without publishing the private tally.

This is `PASS_G08_NATIVE_SINGLE_PROCESS`: the planned native Jubjub G0.8 path
is complete. It is not a weighted-ballot Groth16 implementation, full P3,
SnarkPack, state proof, multiprocess deployment or chain result. The official
Ristretto manifest is unchanged. See `docs/G08_NATIVE_COMPLETE.md` and
`experiments/g0_8_complete_raw/e2e-public.json`.

## G0.9-H0 CP transcript boundary

The registration-bound generalized-CP transcript v3 now has a deterministic
golden vector. Its exact canonical encoding is 684 bytes and has SHA-256
`b902f98b94fd9a5b04a73782a8ab6e73baa7e1525a20d49ee6d2dec8c8bd1d96`.
The native challenge scalar is also frozen by a regression test.

A minimal real R1CS relation allocates the 684 bytes as witness bytes and
public bytes and enforces exact equality. The valid vector satisfies 16,416
constraints; swapping the registration-close and disclosure digests or
changing the registration-close digest makes the relation unsatisfied. This is
`PASS_H0_CANONICAL_TRANSCRIPT_R1CS_BOUND_NO_HASH`: SHA-512/rejection sampling
is not computed in-circuit, no Groth16 proof is generated, and this is not P3.
See `docs/G09_H0_TRANSCRIPT.md` and
`experiments/g0_9_h0_raw/h0_transcript.json`.

## G0.9-H1 SHA-512 circuit

A real SHA-512 R1CS now hashes the exact frozen 684-byte H0 transcript. The
circuit implements SHA-512 padding and six compression blocks, packs the
64-byte public digest into three field elements, and constrains the computed
digest to that public value.

The Release run matched the frozen native digest, used 890,920 constraints and
883,152 witness variables, synthesized the witness in 801.618 ms and reached
807.031 MiB peak process RSS. A changed transcript and a changed public digest
were both rejected. This is `PASS_H1_SHA512_R1CS_EXACT_DIGEST`.

H1 hashes the CP transcript directly. It does not yet implement the protocol's
hash-to-scalar wrapper and rejection sampling, integrate the CP group
equations, or run Groth16. It is therefore not full CP verification or P3. See
`docs/G09_H1_SHA512.md` and
`experiments/g0_9_h1_raw/h1_sha512.json`.

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
- A complete friendly-curve proof-system migration. The native G0.8 path is
  complete and isolated SHA-512 hashing is implemented, but hash-to-scalar,
  weighted-ballot Groth16, the full P3 relation, SnarkPack and the state circuit
  have not migrated to Jubjub.
- Five OS processes with authenticated transport and crash/restart persistence.
- Solidity verifier contracts, Foundry tests, gas, calldata and DA costs.

No synthetic measurements are emitted for these items. Their manifest states
start with `BLOCKED_`, and `validate_full_submission()` refuses the manifest.

## Security interpretation

The code is research experiment code, not an audited voting implementation.
Passing tests means the implemented relations and fault cases behaved as
specified for those tests. It is not evidence for universal cryptographic
security, privacy of an operational election, or production readiness.

