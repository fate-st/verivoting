# G0.9-H3 first CP equation bound to CP-H2F-v3

Status: `PASS_H3_H2F_BOUND_FIRST_CP_EQUATION_R1CS`.

H3 connects the adopted CP-H2F-v3 output to one Trustee's first generalized
Chaum--Pedersen equation:

\[
d=H_q(\mathsf{transcript}),\qquad zG=A_0+dVK.
\]

The circuit hashes the frozen 684-byte registration-bound transcript, reduces
the complete SHA-512 digest modulo Jubjub `Fr`, and uses the resulting bits in
the variable-base multiplication. The Jubjub group operations are constrained
inside R1CS. No host-language group result replaces the relation.

The transcript is a packed public input in this feasibility gate. The circuit
also constrains the arkworks canonical compressed encodings of `VK` and `A0`
at their fixed transcript offsets, so the hashed point bytes cannot be changed
independently from the points used by the equation.

## Real Release measurement

The command was:

```bash
cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-crypto --bin friendly-cp-first-equation-h3
```

The measured result is:

- constraints: `1,168,855`;
- packed public inputs: `23`;
- witness variables: `1,130,518`;
- witness synthesis: `1,228.952 ms`;
- peak RSS: `1,283.797 MiB`;
- transcript: `684` bytes;
- CP-H2F input: `801` bytes in `7` SHA-512 blocks.

The exact timings and peak RSS are machine-specific. The raw artifact records
the Git SHA, dirty state, Rust version, build profile, OS and CPU.

## Acceptance boundary

The valid native vector satisfies both the native equation and R1CS. The
following necessary mutations are rejected:

1. a changed context byte in the transcript;
2. a different verification share; and
3. a response incremented by one.

No additional defensive mutation matrix is run at H3.

## Claim boundary

H3 completes only the first CP equation for one Trustee. H4-A has subsequently
added the three coordinate equations under the same in-circuit challenge; see
`docs/G09_H4A_COMPLETE_TRUSTEE_CP.md`. Multiple Trustees, private-package
openings, threshold interpolation, winner logic, Groth16 and full P3 remain
outside the H3 result.

Raw result:
`experiments/g0_9_h3_raw/h3_first_cp_equation.json`.
