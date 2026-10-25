# G5-A local Prague state-proof verifier

Status: `PASS_G5A_LOCAL_PRAGUE_G2B_GROTH16_VERIFIER`.

G5-A verifies the real G2-B fixed-`B=2`, three-candidate state-transition proof
in compiled Solidity bytecode. The execution uses EthereumJS VM 10.1.3 with
`Hardfork.Prague`, signed transactions and transaction receipts. This gives a
local EVM execution measurement; no public network or finality is measured.

## Implemented relation and encoding

The unchanged G2-B R1CS has 1,360,002 constraints and 37 public inputs. A fresh
reproduction uses the original deterministic setup/proof seeds. It exports
only the verification key, proof and public statements. The proving key and
private witness are not saved.

`Bls12381Groth16Verifier` pins one verification key in constructor-initialized
Solidity storage, publishes its digest and fixes the public-input width. Its
verification API cannot supply or replace the key. It requires each input to
be strictly below BLS12-381 Fr and checks exact precompile return lengths.

The export follows [EIP-2537](https://eips.ethereum.org/EIPS/eip-2537): Fp
coordinates occupy 64-byte big-endian slots, Fp2 uses `c0 || c1`, and proof
bytes are `encode(-A) || encode(B) || encode(C)`. These are 512 bytes, distinct
from the 192-byte compressed arkworks proof. The verifier computes
`IC[0] + sum(input[i] * IC[i+1])` with 38 terms at precompile `0x0c`, followed
by the four-pair test at `0x0f`:

```
e(-A,B) * e(alpha,beta) * e(vk_x,gamma) * e(C,delta) = 1
```

The successful real pairing checks the nontrivial G2 component order across
arkworks and the EVM. Prague includes EIP-2537; see the
[Pectra meta EIP](https://eips.ethereum.org/EIPS/eip-7600).

## Measured execution

Hardware: Apple M4, arm64 macOS; Node 23.11.0. Compiler: solc 0.8.30,
optimizer enabled with 200 runs, `evmVersion=prague`. Dependency versions and
source/fixture/runtime hashes are in the raw artifact and npm lockfile.

| Measurement | Result |
|---|---:|
| Valid-proof transaction status | 1 (success) |
| Changed-root transaction status | 0 (revert: `INVALID_GROTH16`) |
| Valid transaction total gas | 940,640 |
| Valid verifier execution gas | 903,280 |
| Changed-root transaction total gas | 940,718 |
| G1 MSM measured gas | 280,440 |
| Four-pair check measured gas | 168,100 |
| Constructor deployment gas | 4,550,491 |
| Deployed runtime bytes | 1,807 |
| Constructor-pinned VK bytes | 5,760 |
| Uncompressed proof + 37 public scalars | 1,696 bytes |
| Actual ABI verifier calldata | 1,828 bytes |

The negative transaction reuses the exact same proof and modifies only
public-input index 7, the first packed field of the batch root. Both
transactions execute the real MSM and pairing precompiles. The recorded
precompile gas comes from EVM execution events, not a cost formula.

This run regenerated the source proof in 12,938.809 ms setup, 7,537.491 ms
proving and 2.055 ms native verification. Those measurements are saved under
`g5_a_raw/` and do not overwrite the original G2-A/G2-B baseline.

## Scope and next gate

The 1,828 bytes measure only this verifier's transaction data. They exclude the
ordered public ballot matrix `U`, root/registry/nullifier checks and ciphertext
state writes. Full batch DA and complete settlement costs remain unmeasured.
The storage-based VK reads are part of this implementation's gas; this is not
an optimized verifier comparison. SnarkPack and P3 have not been verified in
the EVM, so the Manifest's complete `target_chain` capability remains blocked.

G5-B subsequently completed the 60-input G4-B disclosure-proof verification
using this same Solidity source; see `docs/G5_B_PRAGUE_DISCLOSURE_VERIFIER.md`.
Full settlement and SnarkPack verification remain separate gates.

## Reproduce

From the repository root, generate the real public fixture once:

```bash
CARGO_BUILD_JOBS=2 cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-crypto --bin friendly-state-transition-g2a -- \
  --output experiments/g5_a_raw/source_state_r1cs.json \
  --layout-output experiments/g5_a_raw/source_state_layout.json \
  --groth16-output experiments/g5_a_raw/source_state_groth16.json \
  --evm-fixture-output fixtures/g2b_eip2537_fixture_v1.json
cd contracts
npm ci --ignore-scripts --no-audit --no-fund
npm run g5a
```

An existing public fixture and its source Groth16 report suffice for repeating
only `npm run g5a`; a new setup/prove is unnecessary for a verifier-only rerun.
The script uses an in-memory execution state and makes no RPC or wallet calls.

Outputs:

- `fixtures/g2b_eip2537_fixture_v1.json` (public proof, VK and both statements);
- `experiments/g5_a_raw/source_state_groth16.json` (native proof measurements);
- `experiments/g5_a_raw/g2b_prague_evm.json` (EVM receipts and gas measurements);
- `contracts/build/Bls12381Groth16Verifier.json` (ignored compiler artifact).
