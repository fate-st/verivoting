# G0.9-H4-A complete one-Trustee generalized CP relation

Status: `PASS_H4A_COMPLETE_ONE_TRUSTEE_CP_VECTOR_R1CS`.

H4-A extends H3 with all three coordinate equations while computing the
CP-H2F-v3 challenge only once:

\[
\begin{aligned}
d &= H_q(\mathsf{transcript}),\\
zG &= A_0+dVK,\\
zC_{0,j} &= A_j+dD_j,\qquad j\in\{1,2,3\}.
\end{aligned}
\]

The circuit binds all 11 relevant canonical compressed Jubjub points at their
fixed positions in the 684-byte transcript: `VK`, three ciphertext `C0`
points, three decryption shares, `A0`, and three coordinate commitments. All
four group equations use the same constrained response and challenge.

## Real Release measurement

The command was:

```bash
cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-crypto --bin friendly-cp-vector-h4a
```

The measured result is:

- CP equations: `4`;
- transcript points bound: `11`;
- constraints: `1,207,066`;
- packed public inputs: `23`;
- witness variables: `1,164,214`;
- witness synthesis: `1,529.184 ms`;
- peak RSS: `1,270.656 MiB`;
- CP-H2F input: `801` bytes in `7` SHA-512 blocks.

This adds 38,211 constraints over H3 while completing the native three-coordinate
one-Trustee CP relation.

## Acceptance boundary

The valid native vector satisfies the first equation and all three coordinate
equations. The following necessary mutations are rejected:

1. a changed context byte in the transcript;
2. one altered decryption share together with its matching transcript bytes;
   and
3. one altered coordinate commitment together with its matching transcript
   bytes.

No larger defensive mutation matrix is run.

## Claim boundary

H4-A completes one Trustee's generalized CP relation in R1CS. It does not
verify multiple Trustee packages, threshold selection or interpolation,
private-package openings, tally reconstruction, winner logic, or full P3. No
Groth16 setup/prove/verify is included in H4-A.

H4-B subsequently ran one real Groth16 setup/prove/verify cycle for this exact
relation, including one altered public-transcript rejection. See
`docs/G09_H4B_GROTH16.md`.

Raw result:
`experiments/g0_9_h4a_raw/h4a_complete_trustee_cp.json`.
