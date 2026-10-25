# G0.9-H2-A exact hash-to-scalar feasibility

Status: `PASS_H2A_COUNTER_0_1_FIRST_VALID_R1CS`.

H2-A constrains the exact Suite v2 hash-to-scalar relation for two real
generalized-CP transcripts: one whose first accepted counter is 0 and one
whose first accepted counter is 1. This is the smallest necessary experiment
after H1. It is not the final verification-key-stable H2 circuit.

## Implemented relation

For every evaluated counter, the circuit constructs the canonical H2S-v2
input from the codec magic, H2S domain, suite identifier, operation domain,
transcript length, transcript bytes and big-endian counter. It then:

1. evaluates SHA-512 in R1CS;
2. takes the first 32 digest bytes and constrains the high-nibble mask;
3. interprets the candidate as a 252-bit little-endian integer;
4. constrains strict comparison with the Jubjub scalar modulus; and
5. requires every earlier candidate to be invalid and the claimed counter's
   candidate to be valid.

The accepted scalar and counter are packed into two public field elements.
The CP group equations are deliberately outside this isolated gate.

## Measured Release result

| Case | Counter | Attempts | Constraints | Public inputs | Witness variables | Witness synthesis | Peak RSS after case |
|---|---:|---:|---:|---:|---:|---:|---:|
| Frozen seed `20261021` | 0 | 1 | 1,013,462 | 2 | 1,005,082 | 876.486 ms | 871.672 MiB |
| Frozen seed `20261053` | 1 | 2 | 2,019,916 | 2 | 2,003,674 | 1,741.488 ms | 1,707.500 MiB |

Both transcripts are 684 bytes. The exact H2S input is 806 bytes and occupies
seven SHA-512 blocks per attempt. Native and R1CS scalars match in both cases.
The necessary negative checks also pass: a changed transcript, a changed
operation domain and a false counter-1 claim for the counter-0 vector are all
rejected.

The measurements are from the real Release execution in
`experiments/g0_9_h2_raw/h2a_hash_to_scalar.json`; they are not simulated.
That run records `git_dirty=true` because it measured the newly implemented
H2-A code before commit. It is suitable for development feasibility, but it
must be repeated from a clean commit before use in a final paper table.

## Reproduce

```bash
env CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 run -j 2 \
  --release --locked -p vv-friendly-crypto \
  --bin friendly-cp-hash-to-scalar-h2a
```

The focused unit test is:

```bash
env CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 test -j 2 \
  --release --locked -p vv-friendly-crypto hash_to_scalar_gadget
```

## Claim boundary and next gate

The present circuit hashes only the attempts through the accepted counter, so
its shape changes with that counter. It does not implement one uniform
128-attempt relation and therefore is not yet suitable as the final fixed
Groth16 verification circuit. It also does not integrate CP equations or run
Groth16 setup, prove or verify. Consequently `full_h2=false` and
`full_p3=false`.

H2-B now demonstrates one verification-key-stable shape for two attempts; see
`docs/G09_H2B_FIXED_SHAPE.md`. It does not implement the Suite v2 128-attempt
bound. H2-C must fix the final manifest-pinned hash-to-field construction
before CP integration. No 128-attempt cost is reported as a measured result.
