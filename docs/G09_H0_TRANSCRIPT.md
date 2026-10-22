# G0.9-H0 canonical CP transcript boundary

Status: `PASS_H0_CANONICAL_TRANSCRIPT_R1CS_BOUND_NO_HASH`.

H0 freezes the exact bytes that a future CP hash circuit must consume. It does
not implement SHA-512, hash-to-scalar rejection sampling, Groth16 or P3.

## Frozen statement

The vector uses the registration-bound generalized CP transcript v3 with three
ciphertext coordinates. Its canonical order is:

1. canonical codec magic and transcript domain, each length-prefixed;
2. length-prefixed suite identifier;
3. election ID, pre-policy and final context;
4. registration-close digest and disclosure digest;
5. target, Trustee ID and verification share;
6. ordered `C0` count/vector and decryption-share count/vector;
7. `A0` and ordered coordinate-commitment count/vector.

All fixed cryptographic fields use 32-byte canonical encodings. Counts are
unsigned 64-bit big-endian and the Trustee ID is unsigned 32-bit big-endian.

Golden vector:

- seed: `20261021`;
- transcript length: 684 bytes;
- SHA-256: `b902f98b94fd9a5b04a73782a8ab6e73baa7e1525a20d49ee6d2dec8c8bd1d96`;
- SHA-512: `8534949b00278a5d89b755ad9855def34cd69b695c86c67aa15079e2c8d842aa93b00855a9a854401d40bc79c2faa885980045a361d89eb10228515830b18d5f`;
- challenge scalar, little-endian: `552c8f3d6c4bcdebf1042342cccd93d7a589c1d27f257434d2d33051b0799d02`;
- rejection counter: 0.

The complete bytes and component values are in
`fixtures/jubjub_cp_transcript_v3.json`.

## Real R1CS boundary

The H0 relation allocates each native transcript byte as a witness `UInt8` and
the expected canonical byte as a public `UInt8`, then enforces equality.

| Measurement | Result |
|---|---:|
| Transcript bytes | 684 |
| Constraints | 16,416 |
| Public input bits | 5,472 |
| Witness variables | 5,472 |
| Valid vector satisfied | true |
| Registration/disclosure field swap rejected | true |
| Changed registration digest rejected | true |

This deliberately verbose public-byte relation is only an input-boundary test.
It must not be reused as the final P3 public-input layout without a separate
packing design and audit.

## Reproduce

```bash
cargo +1.99.1 run --release --locked -p vv-friendly-crypto \
  --bin friendly-cp-transcript-h0
```

Outputs:

- `fixtures/jubjub_cp_transcript_v3.json`;
- `experiments/g0_9_h0_raw/h0_transcript.json`.

## Next single gate

G0.9-H1 is complete: one real SHA-512 circuit hashes this exact frozen input
and matches the native digest. See `docs/G09_H1_SHA512.md`.

The next single gate is G0.9-H2: implement only the protocol's exact
hash-to-scalar wrapper and bounded rejection rule around the H1 digest path.
It must first be checked as an isolated R1CS relation and must not yet be called
complete CP verification or P3.
