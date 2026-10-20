# VeriVoting Jubjub prototype suite specification

Status: `G0.8-1 COMPLETE`, `G0.8-2 WEIGHTED_ELGAMAL_COMPLETE`,
`FULL_G0.8 BLOCKED`.

This document defines the isolated suite implemented by
`crates/vv-friendly-crypto`. It does not modify or replace the official
Ristretto255 manifest.

## Version and claim boundary

- Suite identifier: `VERIVOTING-JUBJUB-PROTOTYPE-G0.8-v1`.
- Prototype manifest:
  `configs/protocol_manifest.JUBJUB-PROTOTYPE.json`.
- Implemented: canonical point/scalar encodings, native hash-to-scalar,
  secondary-generator derivation, Weighted ElGamal, homomorphic folding and
  bounded discrete-log decoding.
- Not implemented: Jubjub VSS/GJKR, authentication signatures, generalized
  Chaum--Pedersen, package commitments, threshold reconstruction, full G0.8,
  ballot/state circuits, P3 and SnarkPack.

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

Deterministic encoding vectors are generated at
`fixtures/jubjub_suite_vectors.json`.

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

The native prototype implements `VV/JUBJUB/H2S/v1`:

```text
input(counter) = CanonicalEncode(
    suite_id,
    operation_domain,
    transcript,
    counter_u16_be
)
digest = SHA-512(input(counter))
candidate = digest[0..32] interpreted little-endian
```

For counters `0..127`, the first candidate strictly below the Jubjub scalar
modulus is returned. Candidates are not masked and are not reduced modulo the
scalar order. Exhaustion is an error. Operation domains are part of the hash,
so the same bytes under `CP` and `SIGNATURE` do not share a challenge.

This rule is implemented only natively. `challenge_recomputed_in_circuit`
remains false until G0.9 implements and tests the exact transcript and rejection
rule in R1CS.

## Pedersen secondary generator

The prototype derives `H` under domain `VV/JUBJUB/H2G/v1`:

1. hash the suite identifier, label and counter with SHA-512;
2. parse the first 32 bytes with Jubjub's random-byte point map;
3. clear cofactor 8;
4. reject identity and retry, for at most 128 counters.

The implementation does not define `H` as a published scalar multiple of `G`.
The deterministic label for the planned commitment suite is `PEDERSEN-H`.
Actual Pedersen commitments are not yet implemented in the Jubjub prototype.

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
- identity public key and identity fresh `C0`;
- candidate index outside `1..=C`;
- zero or excessive weight;
- decryption with the wrong secret key;
- mutation of one ciphertext component changes the decoded tally; native
  ElGamal is not authenticated, so protocol-level rejection remains dependent
  on the not-yet-implemented CP/package binding.

These checks establish behavior of this native prototype only. They do not
replace a security proof or the missing DKG/CP protocol.

## Next required stage

The next G0.8 step is Jubjub Pedersen VSS/GJKR with `n=5`, `t=2`, `tau=3`
and `q_cert=3`, followed by ten legal 3-of-5 reconstructions. Generalized CP
and the final winner-only E2E remain later G0.8 stages. No G0.9 circuit work
should begin before those native relations are complete.
