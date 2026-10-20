# G0.6 P3 curve decision

## Decision

The present Ristretto255-in-BLS12-381 circuit route is not economical enough to
start full P3 or the batch state circuit. The isolated SNARK-friendly route is
the only route in this experiment that demonstrated real Groth16 setup, proof,
and verification within the current resource budget.

This is a direction decision, **not a protocol migration**. The official
manifest, DKG, ElGamal, commitments, native Chaum--Pedersen implementation and
test vectors remain Ristretto255. The next necessary implementation step is a
separate end-to-end migration prototype; the manifest must change only if that
prototype re-instantiates and tests every affected primitive.

## Measured comparison

All rows use seed `20261020` and are development-profile measurements. Blank
proof data for Ristretto means `NOT_RUN` because the one-million-constraint
resource guard was exceeded.

| Route / component | Constraints | Public inputs | Witness ms | Setup ms | Prove ms | Verify ms | Proof | Peak RSS |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Ristretto baseline CP, 256-bit joint | 3,501,941 | 324 | 174,531.438 | NOT_RUN | NOT_RUN | NOT_RUN | NOT_RUN | 8,179.4 MiB |
| Ristretto CP, canonical 253-bit joint | 3,461,084 | 324 | 169,906.703 | NOT_RUN | NOT_RUN | NOT_RUN | NOT_RUN | 8,240.4 MiB |
| Jubjub native-field CP equation | 9,571 | 5 | 244.123 | 2,893.210 | 2,931.179 | 34.082 | 192 B | 101.3 MiB |
| Jubjub ElGamal coordinate | 12,538 | 6 | 362.381 | 3,409.576 | 3,535.100 | 39.645 | 192 B | 101.3 MiB |
| Jubjub homomorphic point addition | 5,867 | 6 | 178.566 | 1,959.694 | 2,260.702 | 39.460 | 192 B | 101.3 MiB |

For the single CP equation, the friendly-field prototype uses 99.72% fewer
constraints than the optimized Ristretto circuit (about 361.6 times fewer).
This ratio is strong evidence about field compatibility, but it is not a full
protocol speedup: the two routes do not yet share identical serialization and
Fiat--Shamir subcircuits, and neither recomputes SHA-512 in-circuit.

The complete machine-readable comparison is
`experiments/raw/p3_curve_compare.csv`; the friendly-route source result is
`experiments/raw/p3-curve-friendly.json`.

## What the friendly experiment actually proves

The isolated Jubjub/BLS12-381 experiment uses constrained group operations; it
does not insert a host-language group result as an unconstrained witness.

- CP relation: `zG = A + dVK`. `A` and `VK` are public, on-curve,
  non-identity, prime-order points; `d` is one public field input with a
  canonical scalar bit decomposition; `z` is a canonical private scalar.
- ElGamal coordinate: public `(PK,C0,C1)` and private canonical `(r,m)` satisfy
  `C0=rG` and `C1=rPK+mG`.
- State step: public points satisfy `old + ballot = new`.

Each of the three circuits generated and verified a real Groth16 proof. A
changed public input failed proof verification, and a separately synthesized
invalid relation was unsatisfied. CP-specific tests also rejected a wrong
commitment, challenge, and response.

The CP challenge is not a free witness: it is a canonical public input bound by
Groth16. It is nevertheless **not recomputed from the Fiat--Shamir transcript
inside the circuit**, so this remains one CP equation rather than complete CP
verification or P3.

## Answers to the four decision questions

1. **Is the current Ristretto optimization large enough?** No. Reducing the
   scalar loop from 256 to 253 bits saved 40,857 constraints, only 1.17%, and
   left the circuit at 3.46 million constraints and roughly 8 GiB peak RSS.
2. **Does a SNARK-friendly curve materially reduce group-equation cost?** Yes
   for the tested relations. The CP equation fell to 9,571 constraints and a
   real proof was generated in about 2.93 seconds in the unoptimized build.
3. **What is the full implementation cost?** Retaining Ristretto requires a
   qualitatively better non-native arithmetic/representation design plus exact
   compressed encoding and SHA-512/rejection sampling. Migrating requires a new
   protocol version and coordinated replacement of GJKR/VSS, ElGamal,
   verification shares, CP, commitments, ballot and state circuits,
   serialization, hash-to-scalar rules, test vectors, and the security proof.
4. **Which route can support formal P3 and batch state proofs?** The measured
   native-field route is the practical candidate. Route A is still possible in
   principle, but its current implementation cannot support those circuits
   within the declared resource budget. Route B must pass an end-to-end
   migration prototype before it becomes the official protocol.

## Security and comparability limits

Jubjub here is used through `ark-ed-on-bls12-381` with explicit on-curve,
prime-order and canonical-scalar constraints. Its base field is the BLS12-381
scalar field, which removes the dominant non-native arithmetic seen in Route A.
The experiment does not claim that swapping curves preserves the paper's
security proof automatically.

The ElGamal row is one coordinate relation, not weighted ballot validity. The
state row is one group addition, not `Pi_state`. There is no in-circuit
Fiat--Shamir hash, generalized CP vector loop, package-opening/signature binding,
SnarkPack aggregation, recursive state linkage, Solidity verifier, or gas
measurement. Therefore `full_p3=false` in every new artifact.

## Next necessary experiment

Create a separate migration prototype that implements the same versioned
encoding for friendly-curve DKG, ElGamal, CP and commitments, then compile one
complete CP transcript relation including the selected in-circuit hash. Do not
start full P3 or change the production manifest until that prototype's positive,
mutation, and replay tests pass.

