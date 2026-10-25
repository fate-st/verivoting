# G0.9-H2-B fixed-shape bounded-rejection feasibility

Status: `PASS_H2B_FIXED_TWO_ATTEMPT_R1CS_FEASIBILITY`.

H2-B evaluates the smallest fixed-shape construction needed after H2-A. One
R1CS always evaluates counters 0 and 1, derives which candidate is first
valid, and constrains the public counter and scalar to that result. It uses the
unchanged Suite v2 encoding and SHA-512 relation.

This gate tests circuit shape and selection logic only. Two attempts do not
replace the manifest-pinned Suite v2 bound of 128 attempts.

## Implemented relation

The circuit always performs both H2S-v2 attempts. For each attempt it
constrains the canonical wrapper, SHA-512, 252-bit mask and strict comparison
with the Jubjub scalar modulus. It then:

1. requires at least one of the two candidates to be canonical;
2. selects counter 0 whenever its candidate is canonical;
3. otherwise selects counter 1 and its candidate; and
4. constrains the selected counter and scalar to two packed public inputs.

No branch changes the number or layout of constraints.

## Measured Release result

| Input case | Accepted counter | Attempts evaluated | Constraints | Public inputs | Witness variables | Witness synthesis | Peak RSS after case |
|---|---:|---:|---:|---:|---:|---:|---:|
| Frozen seed `20261021` | 0 | 2 | 2,020,168 | 2 | 2,003,927 | 1,821.592 ms | 1,709.594 MiB |
| Frozen seed `20261053` | 1 | 2 | 2,020,168 | 2 | 2,003,927 | 1,716.996 ms | 1,762.156 MiB |

The two paths have identical constraint, public-input and witness-variable
counts. Both native results match their frozen vectors, and a false counter-1
claim for the counter-0 transcript is rejected.

The measurements come from the real Release execution in
`experiments/g0_9_h2_raw/h2b_fixed_two.json`. The run records `git_dirty=true`
because it measures newly added code before commit; it must be repeated from a
clean commit before inclusion in a final paper table.

## Reproduce

```bash
env CARGO_PROFILE_RELEASE_STRIP=none cargo +1.99.1 run -j 2 \
  --release --locked -p vv-friendly-crypto \
  --bin friendly-cp-hash-to-scalar-h2b
```

## Claim boundary and next gate

H2-B proves that the first-valid selection can use one fixed circuit shape for
a fixed attempt count. It does not implement all 128 Suite v2 attempts, CP
equations, Groth16 setup/prove/verify or P3. Therefore
`suite_v2_128_attempt_relation=false`, `full_h2=false` and `full_p3=false`.

H2-C now evaluates one versioned single-hash wide-reduction candidate; see
`docs/G09_H2C_WIDE_HASH_TO_FIELD.md`. H2-D must decide whether to adopt that
candidate or retain rejection-128. CP integration must wait until the final
suite is fixed.
