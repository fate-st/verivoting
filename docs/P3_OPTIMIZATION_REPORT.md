# G0.5 CP optimization report

## Scope

This round performed only the two experiments needed by the current paper
claim boundary:

1. repair the pre-DKG policy binding order; and
2. compare the existing joint CP scalar multiplication with one sound,
   minimal optimization.

It did not implement SHA-512 in R1CS, the second CP equation, full P3,
SnarkPack, or stress/fault campaigns beyond the binding checks named below.

## Pre-policy binding fix

The executable dependency chain is now:

```text
policy_pre
  -> prekey_context
  -> DKG session_id
  -> DKG transcript and KeyCore digest
  -> final context
  -> registration close and threshold transcript
```

`policy_pre` is computed from the election identifier, weight bound, rule,
registration version, manifest digest, and RA public key before `run_dkg` is
called. The final context is computed only after DKG from the frozen pre-key
context and certified KeyCore digest. The DKG transcript and KeyCore encode the
pre-policy, pre-key context, and derived session identifier.

The protocol manifest was advanced to
`VERIVOTING-BATCH-V4.5-NATIVE-CORE-0.3-PREPOLICY-V2`, and the registration
format version is now `2`. Existing curves and proof systems were not changed.

Necessary rejection tests cover policy replacement, a stale final context,
cross-election replay, a stale DKG session, and a KeyCore policy mutation.

## Executed CP comparison

Both rows used the same real native CP relation, seed `20261020`, dev build,
and a `1,000,000`-constraint proof-run guard.

| Metric | Baseline 256-bit joint | Canonical 253-bit joint | Change |
|---|---:|---:|---:|
| Total constraints | 3,501,941 | 3,461,084 | -40,857 (-1.17%) |
| Public inputs | 324 | 324 | 0 |
| Double-scalar constraints | 3,484,879 | 3,444,022 | -40,857 |
| Synthesis + witness check | 174,531.438 ms | 169,906.703 ms | -2.65% |
| Peak RSS | 8,179.359 MiB | 8,240.438 MiB | no measured improvement |
| Relation satisfied | yes | yes | unchanged |
| Groth16 setup/prove/verify | not run | not run | guard exceeded |

The optimization is sound because canonical Ristretto scalars are below
`2^253`; the existing scalar-range constraints force bits 253--255 to zero.
The optimized circuit omits only those three provably-zero joint-multiplication
iterations. It does not substitute a host-language group result.

## Primitive constraint profile

| Isolated operation | Constraints |
|---|---:|
| Non-native field multiplication plus output check | 707 |
| Extended Edwards point addition plus output check | 5,539 |
| Ristretto quotient equality profile | 4,800 |

The actual circuit's quotient-equality phase is 4,788 constraints; the isolated
profile uses witness allocations and explicit output checks, so it is reported
separately rather than treated as an exact decomposition identity.

The baseline already uses a one-bit Shamir-style joint multiplication for
`zG-dVK`. A split fixed-base path was not reported as an optimization because,
with the present complete extended-coordinate gadget, it introduces a second
scalar-multiplication accumulation and does not provide an evidenced reduction.
No unexecuted estimate is included in the CSV.

## Decision

Status remains `BLOCKED_RESOURCE_GUARD_NO_GROTH16_PROOF`.

The 253-bit change is correct but too small to justify setup/prove/verify: the
optimized circuit is still 3.46 times the declared constraint budget and uses
about 8 GiB during preflight. The next necessary optimization must change the
non-native group representation or scalar-multiplication gadget materially;
full P3 work should not proceed from this result.

Raw evidence:

- `experiments/raw/cp_optimization.csv`
- `experiments/raw/cp-optimization-baseline_256_bit_joint.json`
- `experiments/raw/cp-optimization-canonical_253_bit_joint.json`
- `fixtures/cp_vectors.json`

Blank Groth16 timing and proof-size cells mean **not run**, not zero.
