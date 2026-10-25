# G3-B one-Trustee restart recovery

## Result

G3-B passes the single necessary restart-recovery gate. Five logical Trustees
start the same authenticated local-file DKG used by G3-A. Trustee 3 writes its
mode-`0600` authentication and VSS state, then its initial process exits before
sending dealer shares. A sixth OS process resumes Trustee 3 from that exact
state file.

The resumed process signs a session-bound recovery record containing the
persisted state's SHA-256 digest, the initial PID and the new PID. It then uses
the recovered authentication key and dealer polynomials to finish the 25-message
VSS exchange, agree on the same public transcript and KeyCore, sign the KeyCore,
and contribute to the fixed `{1,2,3}` threshold decryption.

## Measured Release run

- Logical Trustees: 5.
- OS process instances: 6.
- Restarted Trustee: 3.
- Initial / resumed PID: 48827 / 48830.
- Authenticated recovery records: 1.
- Persisted-state digest matched: yes.
- Successful process exits: 6 / 6.
- DKG start to five final states: 57.763 ms.
- Vote encryption and homomorphic fold: 5.360 ms.
- Fixed `{1,2,3}` threshold decryption: 15.317 ms.
- Total wall time: 135.047 ms.
- Authenticated local transport artifacts: 39,714 bytes.
- Coordinator / child peak RSS: 1.938 / 26.859 MiB.
- Private tally: `(4,2,4)`.
- Public winner: `1`.

The resulting transcript digest, KeyCore digest and certified public key are
identical to the G3-A happy-path artifacts produced from the same seed and
Manifest.

## Claim boundary

This is one orderly post-checkpoint restart. It is not a mid-write crash,
repeated-crash, timeout, disk-corruption, Byzantine-process or remote-network
experiment. Those cases are not required for the current paper's minimal local
process-separation evidence.

Like G3-A, this deployment experiment remains bound to the active
`VERIVOTING-JUBJUB-PROTOTYPE-G2B-2.2` cryptographic Manifest without changing
its digest.

## Reproduce

```bash
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-g3a-multiprocess -- --restart-trustee 3
```

Raw result:

- `experiments/g3_b_raw/one_trustee_restart.json`

The next necessary gate is G4-A: test the existing complete P3 relation with
one additional legal Trustee subset before deciding whether a universal
selected-subset circuit is justified.
