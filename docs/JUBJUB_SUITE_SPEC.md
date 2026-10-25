# VeriVoting Jubjub prototype suite specification

Status: `G0.8-E2 NATIVE_PATH_COMPLETE`, `G0.9-R BASELINE_FROZEN`,
`G0.9-H2-D CP_H2F_V3_ADOPTED`,
`G0.9-H4-B ONE_TRUSTEE_CP_GROTH16_COMPLETE`,
`G0.9-P3-A THREE_SELECTED_CP_GROTH16_COMPLETE`,
`G0.9-P3-B0 PACKAGE_H2F_V1_ADOPTED`,
`G0.9-P3-B1 THREE_PACKAGE_OPENINGS_R1CS_COMPLETE`,
`G0.9-P3-C0 FIXED_123_THRESHOLD_COMBINATION_R1CS_COMPLETE`,
`G0.9-P3-C1 BOUNDED_TALLY_BINDING_R1CS_COMPLETE`,
`G0.9-P3-D0 PUBLIC_WINNER_LINKAGE_R1CS_COMPLETE`,
`G0.9-P3-D1 FIXED_123_COMPLETE_P3_GROTH16_COMPLETE`,
`G1-B0 WEIGHTED_BALLOT_CORE_R1CS_COMPLETE`,
`G1-B1 FIXED_SUITE_FULL_C_VOTE_R1CS_COMPLETE`,
`G1-B2 FIXED_SUITE_FULL_C_VOTE_GROTH16_COMPLETE`.

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
  threshold decryption, a winner-only public event, and the fixed-suite
  five-condition weighted-ballot R1CS, and its real Groth16 proof.
- Not implemented: state circuits, a general-subset production P3 setup,
  SnarkPack, multiprocess deployment and chain verification.

The prototype manifest intentionally fails the official runner's `READY`
requirements. The existing `configs/protocol_manifest.json` remains unchanged.

For G0.9-H2/H3/H4, this Suite v2 specification and
`configs/protocol_manifest.JUBJUB-PROTOTYPE.json` are the authoritative
experiment suite. The official Ristretto manifest is a legacy comparison
baseline and its encodings are not inputs to the G0.9 circuit path. The clean
baseline commit, artifact hashes and toolchain are recorded in
`docs/G09_R_BASELINE.md`.

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

This complete rule is implemented natively. G0.9-H2-A also constrains the exact
wrapper, masking, range check and first-valid rule in R1CS for real accepted
counters 0 and 1. Its circuit shape depends on the accepted counter, so
G0.9-H2-B adds one fixed-shape two-attempt feasibility circuit. The manifest
still requires 128 attempts, so `challenge_recomputed_in_circuit` remains
false for the full protocol until the final bounded-rejection circuit is fixed
and integrated with the CP equations.

G0.9-H2-C separately evaluates
`VERIVOTING-JUBJUB-H2F-CANDIDATE-G0.9-v3`: one SHA-512 digest interpreted as a
512-bit big-endian integer and reduced modulo `Fr`. It is fixed-shape and
measured in R1CS, but it is not part of this Suite v2 specification. Adopting
it would require a new manifest version, regenerated CP/signature vectors and
a corresponding paper/security update.

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

For each Trustee, the current prototype canonically encodes the complete
native CP package, including its election context, verification share,
decryption-share vector, commitment points and response. CP-H2F-v3 no longer
uses a rejection counter. P3-B0 hashes that package to a Jubjub scalar with a
dedicated fixed-shape mapping and commits with a fresh secret opening:

```text
input_h = CanonicalEncode(
    "VV/JUBJUB/H2F/v3",
    "VERIVOTING-JUBJUB-PACKAGE-H2F-G0.9-v1",
    "PRIVATE-PACKAGE",
    Encode(package_h)
)
m_h = OS2IP_BE(SHA-512(input_h)) mod Jubjub-Fr
Com_h = m_h*G + rho_h*H
```

The Trustee signs the public commitment and complete election binding under
the dedicated `PRIVATE-PACKAGE-COMMITMENT-SIGNATURE` domain. A package becomes
selectable only after the verifier checks its election, policy, final context,
target, Trustee verification share, authentication signature, commitment
opening and generalized CP proof. Selection is deterministic by Trustee ID and
requires at least `tau=3` distinct verified packages.

The current registration-bound package and commitment transcript domains are
v5, and the winner-only event is v4. The P3-B0 native E2E creates and verifies
5/5 packages, selects Trustees 1--3 and internally recovers `(4,2,4)`. Its
public JSON emits winner `1`, context digests, and the three selected
commitments and signatures. It does not emit the tally, decryption shares, CP
proofs or commitment openings. The five complete private encodings total
3,010 bytes (602 bytes per Trustee); this is a native serialization
measurement, not a P3 proof size.

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

G0.9-H2-A constrains the exact Suite v2 wrapper, 252-bit mask, strict scalar
range and first-valid rule for real accepted counters 0 and 1. H2-B
demonstrates a single fixed shape for both paths using two attempts. H2-C
evaluates a versioned wide-reduction candidate, and H2-D adopts it for CP as
`VERIVOTING-JUBJUB-CP-H2F-G0.9-v3`.

G0.9-H3 connects that in-circuit challenge to one Trustee's first equation,
`zG=A0+dVK`, and binds the canonical `VK` and `A0` encodings in the transcript.
H4-A adds all three coordinate equations, binds all 11 transcript points and
completes the one-Trustee CP vector in R1CS. H4-B runs real Groth16 over that
exact relation: setup, proof and verification succeed, and an altered public
transcript is rejected. G0.9-P3-A freezes the public/private input layout and
composes three distinct selected Trustee CP relations. The measured relation
uses 3,611,598 constraints and keeps the shares and proofs private. Real
Groth16 setup, proof and verification succeed, and an altered public prefix is
rejected. P3-B0 then adopts and measures the fixed-shape Package-H2F-v1
mapping for the three selected private packages. P3-B1 constructs those
packages from the CP witnesses and constrains all three Pedersen opening
equations in a 6,191,178-constraint R1CS. P3-C0 applies the fixed Trustee
1--3 coefficients `(3,-3,1)` and binds three private plaintext group elements
in a 6,201,342-constraint R1CS. P3-C1 binds those points to three private
bounded integer tallies, enforces their sum equals public `W=10`, and uses
6,203,703 constraints. P3-D0 reuses those tally bits to enforce public winner
`1` with highest-weight, smallest-identifier tie breaking. The complete R1CS
uses 6,205,629 constraints. P3-D1 runs real Groth16 over that exact fixed-123
relation: setup and proof succeed, the 192-byte proof verifies, and a changed
public winner is rejected.

G1-B0 starts the ballot path with one public registered-weight commitment,
its private weight and opening, one private three-candidate one-hot choice,
and three public Weighted ElGamal ciphertext pairs. The native Jubjub R1CS
uses 46,573 constraints and rejects a two-hot choice. This is only the ballot
cryptographic core: election/context binding, the credential-derived PRF
address and deterministic region, the voting nullifier, and registration-root
membership are not yet constrained.

G1-B1 completes the five-condition ballot relation for one explicitly fixed
experiment suite. A 32-byte voter seed drives separately domain-separated
HMAC-SHA-256 address and nullifier outputs bound to `eid`, the final context
and key-set hash. SHA-256 assigns one of four regions. The derived address and
private weight commitment form the exact native registration leaf, and a
depth-two path binds it to the certified four-leaf closing root. These gates
are composed with the G1-B0 one-hot, weight-opening and three-coordinate
Weighted ElGamal equations. The relation uses 854,454 constraints. Its scope
is fixed to four leaves and four regions; Groth16 and SnarkPack are absent.

G1-B2 runs real BLS12-381 Groth16 setup, proof and verification over that exact
relation. Setup takes 8,015.162 ms, proving 4,623.984 ms and verification
2.386 ms. The proof is 192 bytes; the compressed proving and verifying keys
are 251,898,000 and 1,592 bytes. Peak RSS is 1,880.547 MiB. The valid proof
verifies and changing only the public final context causes verification to
fail. The deterministic setup seed is a reproducibility input, not a
production ceremony.

G1-C0 then converts two independently randomized G1-B2 proofs and their
verification key from arkworks to canonical blstrs BLS12-381 representations,
reverifies them with bellperson 0.27.0, and runs real SnarkPack V2 aggregation.
The minimum batch of two aggregates in 4.003 ms and verifies in 3.153 ms; a
changed public context is rejected. The 5,236-byte aggregate is larger than
the two 192-byte proofs, so this is a compatibility result only. Its
deterministic `setup_fake_srs` is not a production aggregation ceremony.
