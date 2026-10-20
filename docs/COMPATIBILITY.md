# Compatibility decisions

## Native group

The native protocol uses Ristretto255 through `curve25519-dalek 4.1.3` for
weighted ElGamal, commitments, signatures, VSS/DKG and Chaum–Pedersen proofs.
Compressed 32-byte Ristretto encodings are used in transcripts.

## Groth16 P1

The P1 circuit uses arkworks `ark-groth16 0.4` over BLS12-381. It is a real
setup/prove/verify path, but only the P1 winner relation is implemented. The
upstream implementation is <https://github.com/arkworks-rs/groth16>.

## SnarkPack

The protocol manifest fixes the paper-level reference to SnarkPack but this
repository does not vendor or invoke an implementation. The SnarkPack paper's
reference implementation used BLS12-381 and a Filecoin/bellperson development
branch; integrating and pinning a maintained compatible implementation remains
a separate G1 task. Source: <https://eprint.iacr.org/2021/529.pdf>.

## Hash to scalar

Transcript challenges use SHA-512 with explicit domains and a bounded rejection
loop. A candidate is masked to 253 bits and accepted only when it is a canonical
Ristretto scalar; the accepted counter is serialized with the proof. The
protocol limit is exactly 128 attempts and failure is returned, never replaced
with modular reduction.

