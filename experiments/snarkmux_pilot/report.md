# SnarkMux Pilot

**BLOCKED_SOURCE**. Stopped at stage 1.

Base: `1ade01f12bc95ab00a514a7986a984816fe1741b`; branch: `experiment/snarkmux-pilot`.

## Actual checks

First original W32 B4; 32 registered voters, three candidates, four Groth16 proofs and 25 unchanged ordered inputs each under one VK. **4/4 verified**, BLS12-381. A fifth archived ballot is a valid replacement control only.

Modified second-ballot nullifier input, swapped input and a fifth proof under original third-ballot inputs are rejected by actual Groth16 verification. These are NOT SnarkMux aggregation negatives.

## Source gate

[Paper](https://eprint.iacr.org/2023/1946), section 6.2, names [this prototype](https://anonymous.4open.science/r/SnarkMux). Actual access: HTTP 401; effective URL: `https://anonymous.4open.science/api/repo/SnarkMux/file/`. No source commit, executable importer or complete verifier was obtained. No prototype build, SnarkMux aggregation or EVM deployment was attempted. This is an access/admission blocker, not proof that no implementation exists.

The inspected PDF reports BN254/Grumpkin recursion with Spartan final compression. Current ballots use BLS12-381. No lossless cross-curve conversion is claimed, and code-level portability remains untested. Official PDF access returned 403 during research; the mirror PDF and hash are retained without claiming official-byte equality.

## Measurements

| Metric | Original individual Groth16 | SnarkMux |
|---|---:|---:|
| Valid proofs | 4/4 | not run |
| Native verify, ms | 3.696459 | N/A |
| VK prepare, ms (separate) | 10.152958 | N/A |
| Compressed proof bytes | 192, 192, 192, 192 | N/A |
| Aggregate time / bytes | N/A | N/A |
| VPrep / online verification | N/A | N/A |
| New EVM verification / deployment Gas | not measured | not measured |

One native sample is admission evidence, not a statistical comparison. Old Stage 2 Gas is not a newly matched baseline. Unexecuted numerical fields are null in results.json.

## Decision

Do not replace SnarkPack. Do not build a hash-only aggregator, ordinary-Groth16 substitute or placeholder EVM verifier. Resume only after obtaining author-linked or author-confirmed equivalent source, pinning its commit and auditing complete BLS12-381 import/verification semantics. No new ballots, DKG, P3 or EVM transactions. No automatic commit or push.

## Reproduction

`node scripts/snarkmux_pilot.mjs --paper /ABS/snarkmux.pdf --output /ABS/NEW_OUTPUT`

The adapter is isolated at experiments/snarkmux_pilot/adapter. It verifies but never creates proofs. No original source, circuit, frozen config or contract was edited. Existing outputs are never overwritten.
