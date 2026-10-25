# G0.9-H2-C wide SHA-512-to-field candidate

Status: `PASS_H2C_WIDE_SHA512_MOD_FR_CANDIDATE_R1CS`.

H2-C evaluates one versioned fixed-shape alternative to Suite v2
rejection-128. The candidate hashes the canonical input once, interprets the
complete 64-byte SHA-512 digest as a big-endian integer, and reduces that
integer modulo the Jubjub scalar field order.

The candidate suite identifier is
`VERIVOTING-JUBJUB-H2F-CANDIDATE-G0.9-v3`, under domain
`VV/JUBJUB/H2F/v3`. It is isolated experimental code. The current Suite v2,
native CP, signatures and manifests are unchanged.

## Candidate mapping

```text
input = CanonicalEncode(
    candidate_suite_id,
    operation_domain,
    transcript
)
digest = SHA-512(input)
d = OS2IP_BE(digest) mod Jubjub-Fr
```

For a uniform 512-bit digest, the statistical distance introduced by reducing
modulo a field smaller than `2^253` is conservatively bounded by less than
`2^-259`. This is an analytical bound under the uniform-digest model, not an
experimental security measurement.

## Implemented R1CS relation

The circuit:

1. constructs the exact versioned canonical wrapper;
2. evaluates one real SHA-512 computation;
3. binds each digest byte into a Jubjub-`Fr` non-native field value;
4. performs big-endian Horner accumulation with radix 256, reducing in `Fr`;
   and
5. constrains the canonical little-endian scalar encoding to two packed public
   field elements.

The field operation is therefore bound to all 512 digest bits; no
host-language reduction is substituted for circuit arithmetic.

## Measured Release result

| Source transcript | Constraints | Public inputs | Witness variables | Witness synthesis | Peak RSS after case |
|---|---:|---:|---:|---:|---:|
| seed `20261021` | 1,148,545 | 2 | 1,113,725 | 1,228.740 ms | 1,158.812 MiB |
| seed `20261053` | 1,148,545 | 2 | 1,113,725 | 1,160.321 ms | 1,194.219 MiB |

Both cases use an 808-byte H2F input and seven SHA-512 blocks. Their
constraint, public-input and witness-variable counts are identical. Native and
R1CS results match, and changing the transcript while retaining the original
public scalar is rejected.

The candidate uses 43.146% fewer constraints than the H2-B fixed two-attempt
relation (1,148,545 versus 2,020,168). This comparison is between measured
R1CS results; it is not a comparison with an unexecuted 128-attempt circuit.

Raw results are in
`experiments/g0_9_h2_raw/h2c_wide_hash_to_field.json`. The run records
`git_dirty=true` because it measures newly added code before commit and must be
repeated from a clean commit before use in a final paper table.

## Reproduce

```bash
env CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 run -j 2 \
  --release --locked -p vv-friendly-crypto \
  --bin friendly-cp-hash-to-field-h2c
```

## Claim boundary and next gate

H2-C established implementation feasibility only and remains a historical
candidate measurement. H2-D subsequently adopts the same mapping under final
suite ID `VERIVOTING-JUBJUB-CP-H2F-G0.9-v3`; see
`docs/G09_H2D_CP_H2F_ADOPTION.md`. H2-C itself did not migrate CP or run
Groth16.
