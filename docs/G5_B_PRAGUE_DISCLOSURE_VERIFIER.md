# G5-B local Prague P3 disclosure-proof verifier

Status: `PASS_G5B_LOCAL_PRAGUE_TWO_SUBSET_P3_GROTH16_VERIFIER`.

G5-B exports the real G4-B complete-P3 proof and verifies it using the same
Solidity source and Prague execution route established by G5-A. The P3
verification key is installed in a new constructor-pinned verifier instance;
it is different from G5-A's state-proof key.

## Relation and necessary cases

The G4-B relation combines three generalized CP relations, package commitment
openings, threshold reconstruction, bounded private tallies and public-winner
selection. Its one public selector supports exactly two ordered subsets:
`false -> {1,2,3}`, `true -> {1,2,4}`. A setup generated from `{1,2,3}` is
used to prove the `{1,2,4}` assignment, with public winner `1`.

The unchanged circuit has 6,231,973 constraints, 60 public inputs and 6,005,106
witness variables. The EIP-2537 export contains a 512-byte `-A || B || C`
proof, an 8,704-byte verification key and 60 canonical public scalars. The key
has 61 IC points, including the implicit constant term. Private CP suffixes,
decryption shares, openings, plaintext group elements, tally values and the
proving key are not serialized. Deterministic experimental seeds do not
provide production privacy or a production trusted setup.

The EVM gate executes only the two necessary verification cases:

1. The native-verified proof and its 60 public inputs succeed.
2. The same proof with only selector input 51 changed from `1` to `0` reverts
   with `INVALID_GROTH16`.

Both transactions execute the real 61-term G1 MSM and four-pair check. The
Solidity source is unchanged from G5-A, including scalar-range, proof-length
and precompile-return checks. The shared runner also rechecks the source
status, input width, constraint count, two-subset scope and selector mutation.

## Real measurements

Apple M4, arm64 macOS; Rust 1.99.1 Release `opt-level=1`; Node 23.11.0;
EthereumJS VM 10.1.3; solc 0.8.30, optimizer 200 runs, `evmVersion=prague`.
These are signed transactions executed by a local in-memory EVM.

| Measurement | Result |
|---|---:|
| Valid-proof receipt | 1 (success) |
| Changed-selector receipt | 0 (revert) |
| Valid transaction total gas | 1,331,473 |
| Valid execution gas | 1,278,293 |
| Changed-selector transaction gas | 1,331,539 |
| Measured G1 MSM gas | 424,560 |
| Measured four-pair check gas | 168,100 |
| Deployment gas | 6,631,812 |
| Deployed runtime | 1,807 bytes |
| Compressed arkworks proof | 192 bytes |
| EIP-2537 proof | 512 bytes |
| Logical proof plus public inputs | 2,432 bytes |
| Actual ABI calldata | 2,564 bytes |

The real source-proof reproduction took 64,115.567 ms setup, 40,229.260 ms
proving and 5.926 ms native verification, with 7,818.922 MiB peak RSS. It is
saved in `g5_b_raw/`; the original G4-B baseline is preserved.

The G5-A regression through the shared runner remains 940,640 gas and 1,828
calldata bytes. Its report is saved separately as
`experiments/g5_b_raw/state_verifier_regression.json`, preserving the original
G5-A baseline as well.

## Claim boundary and next gate

This verifies the cryptographic P3 relation for the implemented two-subset
gate. It does not compare the statement against a frozen election registry,
authenticate commitment signatures on chain, or write a final winner. Full
`AcceptWinnerDisclosure`, complete settlement/DA, SnarkPack EVM verification
and public-chain finality are not measured. The calldata number covers this
verifier transaction only. Gas includes the current storage-based VK reads.
The complete Manifest `target_chain` capability remains blocked.

Next is G5-C: inspect the pinned B=2 SnarkPack V2 verifier's required operations
and exact transcript/encoding against the Prague runtime. Establish a minimal
executable verification route before attempting full settlement or scale
benchmarks. A Groth16 pairing check alone does not validate SnarkPack.

## Reproduce

Generate the public fixture once from the repository root:

```bash
CARGO_BUILD_JOBS=2 cargo +1.99.1 run -j 2 --release --locked \
  -p vv-friendly-dkg --bin friendly-g4b-public-subset-groth16 -- \
  --output experiments/g5_b_raw/source_p3_groth16.json \
  --layout-output experiments/g5_b_raw/source_p3_layout.json \
  --evm-fixture-output fixtures/g4b_eip2537_fixture_v1.json
cd contracts
npm ci --ignore-scripts --no-audit --no-fund
npm run g5b
```

With the public fixture and native report present, `npm run g5b` alone repeats
the EVM gate without another setup/prove. Optional positional output paths
allow independent regression records:

```bash
npm run g5a -- experiments/g5_b_raw/state_verifier_regression.json
```

Artifacts:

- `fixtures/g4b_eip2537_fixture_v1.json`;
- `experiments/g5_b_raw/source_p3_groth16.json`;
- `experiments/g5_b_raw/p3_prague_evm.json`;
- `contracts/scripts/g5b_disclosure_verifier.mjs`;
- `contracts/scripts/prague_groth16_runner.mjs` (shared executor).
