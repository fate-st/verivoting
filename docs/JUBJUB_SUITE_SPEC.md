# VeriVoting Jubjub prototype suite specification

Status: `G0.8-E2 NATIVE_PATH_COMPLETE`, `G0.9-H1 SHA512_COMPLETE`,
`HASH_TO_SCALAR_AND_PROOF_SYSTEM_MIGRATION_BLOCKED`.

This document defines the isolated suite implemented by
`crates/vv-friendly-crypto` and `crates/vv-friendly-dkg`. It does not modify or
replace the official Ristretto255 manifest.

## Version and claim boundary

- Suite identifier: `VERIVOTING-JUBJUB-PROTOTYPE-G0.8-v2`.
- Prototype manifest:
  `configs/protocol_manifest.JUBJUB-PROTOTYPE.json`.
- Implemented: canonical point/scalar encodings, native hash-to-scalar,
  secondary-generator derivation, Weighted ElGamal, homomorphic folding,
  bounded discrete-log decoding, single-process Pedersen VSS/GJKR, Jubjub
  Schnorr KeyCore certification, registration commitments and authenticated
  close, generalized Chaum--Pedersen, authenticated private packages, verified
  threshold decryption and a winner-only public event.
- Not implemented: weighted-ballot/state circuits, full P3, SnarkPack,
  multiprocess deployment and chain verification.

The prototype manifest intentionally fails the official runner's `READY`
requirements. The existing `configs/protocol_manifest.json` remains unchanged.

## Algebraic parameters

The implementation uses `ark-ed-on-bls12-381` version 0.4.0.

| Parameter | Value |
|---|---|
| Curve model | Jubjub twisted Edwards |
| Base field `Fq` | `52435875175126190479447740508185965837690552500527637822603658699938581184513` |
| Prime subgroup scalar field `Fr` | `6554484396890773809930967563523245729705921265872317281365359162392183254199` |
| Cofactor | `8` |
| R1CS constraint field for BLS12-381 Groth16 | Jubjub base field `Fq` |
| Native encryption/CP scalar field | Jubjub subgroup field `Fr` |

`Fq` and `Fr` are different fields. Native scalar encodings and future circuit
range constraints must use `Fr`; the fact that Jubjub coordinates are native
to the Groth16 constraint field does not make arbitrary `Fq` values valid
Jubjub scalars.

## Canonical encoding

### Points

- Encoding: arkworks canonical compressed Jubjub affine encoding, exactly 32
  bytes.
- Decoding requires: exact length, canonical field encoding, on-curve point and
  membership in the prime-order subgroup.
- Identity is allowed only for explicitly documented internal accumulators and
  zero-message group values. Public keys, Pedersen generators and fresh
  ciphertext `C0` values reject identity.
- Protocol transcripts must use the compressed bytes, never unconstrained host
  coordinates.

### Scalars

- Encoding: exactly 32 little-endian bytes.
- Decoding interprets the bytes as an integer and accepts only values strictly
  below the Jubjub scalar modulus `Fr`; it never reduces a non-canonical input.
- The all-zero scalar is canonical but is rejected where a secret or encryption
  randomness must be nonzero.

The immutable v1 vectors remain at `fixtures/jubjub_suite_vectors.json`.
Suite v2 vectors are generated at `fixtures/jubjub_suite_vectors.v2.json`.

## Canonical context encoding

Variable-length transcript fields use `vv-types::CanonicalEncoder`:

1. `VV-CANONICAL-V1` and the operation domain are length-prefixed;
2. integer lengths are unsigned 64-bit big-endian;
3. counters are unsigned 16-bit big-endian;
4. fixed-size cryptographic values are inserted in their specified canonical
   byte encoding.

Future DKG and CP transcripts must introduce new operation-specific domains and
bind the suite version, election identifier, pre-policy, final context, target,
trustee identifier and ordered vector lengths.

## Hash to scalar

The native prototype implements `VV/JUBJUB/H2S/v2`:

```text
input(counter) = CanonicalEncode(
    suite_id,
    operation_domain,
    transcript,
    counter_u16_be
)
digest = SHA-512(input(counter))
candidate = digest[0..32]
candidate[31] &= 0x0f
candidate = candidate interpreted little-endian
```

For counters `0..127`, the first 252-bit candidate strictly below the Jubjub
scalar modulus is returned. Candidates are never reduced modulo the scalar
order. The per-attempt acceptance probability is approximately `0.90569`; the
128-attempt exhaustion probability is approximately `5.54e-132` under the
uniform-digest model. Exhaustion remains an explicit error. Operation domains
are part of the hash, so the same bytes under `CP` and `SIGNATURE` do not share
a challenge. The counter is included in signatures and must exactly match the
first accepted candidate.

This complete rule is implemented only natively. G0.9-H1 now recomputes the
SHA-512 digest in R1CS, but `challenge_recomputed_in_circuit` remains false
until the wrapper, masking and bounded rejection rule are also constrained.

## Pedersen secondary generator

The prototype derives `H` under domain `VV/JUBJUB/H2G/v1`:

1. hash the suite identifier, label and counter with SHA-512;
2. parse the first 32 bytes with Jubjub's random-byte point map;
3. clear cofactor 8;
4. reject identity and retry, for at most 128 counters.

The implementation does not define `H` as a published scalar multiple of `G`.
The deterministic label is `PEDERSEN-H`. G0.8-3 uses the resulting point for
Pedersen polynomial commitments; the implementation never publishes a known
discrete-log relation between `H` and `G`.

## Native Weighted ElGamal

For nonzero secret `sk`, public key `PK=sk*G`, nonzero fresh randomness `r`
and integer message `m`:

```text
C0 = r*G
C1 = r*PK + m*G
```

A weighted vote encrypts `weight` in the chosen candidate coordinate and zero
in every other coordinate, with independent nonzero randomness per coordinate.
Weights must satisfy `1 <= weight <= W_max`, and the choice must be in the
canonical candidate order. Ciphertexts fold component-wise by group addition.

Native decryption computes `M=C1-sk*C0`; BSGS recovers the unique integer in
`[0,W_reg]`. The current four-voter fixture uses `W_reg=10` and obtains private
tally `(4,2,4)`, followed by smallest-identifier tie breaking to winner `1`.

## Implemented negative checks

- non-canonical scalar equal to the modulus;
- malformed point encoding and disallowed identity;
- hash domain substitution;
- identity or torsion public key, invalid ciphertext subgroup point, and
  identity fresh `C0`;
- candidate index outside `1..=C`;
- zero or excessive weight;
- excessive BSGS table bounds and checked candidate-index arithmetic;
- decryption with the wrong secret key;
- mutation of one ciphertext component changes the decoded tally; native
  ElGamal is not authenticated, so protocol-level rejection remains dependent
  on the not-yet-implemented CP/package binding.

For G0.8-3, tests also reject duplicate, unsorted and sub-threshold trustee
sets, insufficient certificate signatures, altered election/session binding,
and invalid complaint responses. Corrected private-share complaints and
Feldman extraction recovery are exercised explicitly.

For G0.8-E1, tests reject a changed commitment opening, package signature or
election context. Selection rejects too few verified packages and duplicate
Trustee identifiers. These are the necessary package-boundary checks; the
experiment does not add synthetic network or denial-of-service scenarios.

For G0.8-E2, tests reject duplicate registration credentials, an incorrect
weight-commitment opening, an altered close record, a cross-election or stale
final-context close, and substitution of the registration-close digest in a
CP statement. These checks exercise the new binding without adding unrelated
fault campaigns.

These checks establish behavior of this native prototype only. They do not
replace a security proof or the missing proof-system circuits.

## Native Pedersen VSS/GJKR and threshold decryption

The G0.8-3 configuration is `n=5`, `t=2`, `tau=3`, `q_cert=3`. Trustee
authentication public keys and the registration-policy digest are fixed before
DKG and included in `policy_pre`. The chain is acyclic:

```text
authentication keys + registration policy + parameters -> policy_pre -> prekey_context
-> dkg_session_id -> transcript/KeyCore certificate
```

Each dealer distributes degree-2 value and blinding polynomials and publishes
Pedersen coefficient commitments. Complaints either receive a valid correction
or disqualify the dealer. Feldman records are checked against delivered shares;
an inconsistent broadcast is extracted from `t+1` valid evaluations and is
recorded as a recovery. The aggregate public key and all five verification
shares are certified by distinct Jubjub Schnorr signatures.

For each ordered three-trustee subset, the implementation computes Lagrange
coefficients at zero and combines decryption shares directly:

```text
D[h,j] = x_h * C0[j]
Y[j] = C1[j] - sum_h lambda[h] * D[h,j]
```

The aggregate secret is not reconstructed. All ten legal subsets recover
`(4,2,4)` and winner `1` in the Release experiment.

## Registration close and target binding

G0.8-E2 fixes the RA public key, `W_max=10`, registration version and winner
rule in a registration-policy digest before DKG. The RA verifies four unique
credential entries and their Jubjub Pedersen openings, canonically orders them,
computes the exact integer total and signs a close record containing the final
context, policy, root, count, total, limit and canonical-registry digest.

Successful verification returns a typed close value. Its record and disclosure
digests are then included in target v3:

```text
target = H(eid, policy_pre, final_context,
           registration_close_digest, disclosure_digest,
           ordered aggregate ciphertexts)
```

The four-voter Release run authenticates count 4 and registered weight 10.
Registration exactness is an RA-honesty assumption, matching the paper; the
signature authenticates the assertion and is not a sum proof.

## Native generalized Chaum--Pedersen

For Trustee `h` and each aggregate ciphertext coordinate `j`, the native proof
uses:

```text
D[h,j] = x_h * C0[j]
A0 = eta * G
A[j] = eta * C0[j]
z = eta + d*x_h
```

The verifier recomputes `d` with Suite v2 hash-to-scalar. The E2 transcript-v3
tuple is `suite, eid, policy_pre, final_context, registration-close digest,
disclosure digest, target, h, VK_h, C0-vector, D-vector, A0, A-vector`. The
target separately commits to those registration bindings and the full ordered
`(C0,C1)` ciphertext vector. Verification enforces:

```text
z*G = A0 + d*VK_h
z*C0[j] = A[j] + d*D[h,j]  for every j
```

Successful verification returns `VerifiedDecryptionShare`, whose fields are
private to the CP module. The threshold combiner accepts this verified type,
not an arbitrary point vector. For three candidates, each proof is 162 bytes
and its decryption-share vector is 96 bytes. All five proofs and all ten legal
threshold subsets passed the Release experiment.

## Authenticated private packages and winner-only event

For each Trustee, G0.8-E1 canonically encodes the complete native CP package,
including its election context, verification share, decryption-share vector,
commitment points, response and rejection counter. It hashes that encoding to
a Jubjub scalar and commits with a fresh secret opening:

```text
Com_h = H2S("PRIVATE-PACKAGE", Encode(package_h))*G + rho_h*H
```

The Trustee signs the public commitment and complete election binding under
the dedicated `PRIVATE-PACKAGE-COMMITMENT-SIGNATURE` domain. A package becomes
selectable only after the verifier checks its election, policy, final context,
target, Trustee verification share, authentication signature, commitment
opening and generalized CP proof. Selection is deterministic by Trustee ID and
requires at least `tau=3` distinct verified packages.

The E1 Release run created and verified 5/5 packages, selected Trustees 1--3 and
internally recovered `(4,2,4)`. Its public JSON emits winner `1`, context
digests, and the three selected commitments and signatures. It does not emit
the tally, decryption shares, CP proofs or commitment openings. The complete
private encodings total 2,700 bytes (540 bytes per Trustee); this is a native
serialization measurement, not a P3 proof size.

E2 upgrades package and commitment transcripts to v3 by adding the two
registration-close digests. The five complete private encodings total 3,020
bytes (604 bytes per Trustee). The public event still contains only the close
record, contexts, selected commitments/signatures and winner.

## Next required stage

G0.9-H0 freezes the exact v3 CP transcript field order and 684-byte golden
encoding. A real R1CS byte-equality boundary accepts that vector and rejects a
changed registration digest or exchanged registration/disclosure fields. Its
16,416 constraints and 5,472 public input bits are a temporary H0 boundary,
not the final packed P3 public-input layout.

G0.9-H1 implements the real SHA-512 padding/compression path for the frozen
byte string. It matches the native digest with 890,920 constraints; changed
transcript and digest inputs are rejected. This isolated gate does not perform
the protocol hash-to-scalar mapping, integrate CP equations or run Groth16.

The next gate is only G0.9-H2: constrain the exact Suite v2 hash-to-scalar
wrapper, 252-bit masking and bounded rejection rule, and compare its result
with the frozen native challenge vector. H2 must not yet be called complete CP
verification or P3.
