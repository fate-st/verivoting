# G3-A five-process DKG happy path

## Result

G3-A passes the minimal process-separation gate. Five Trustee identities run as
five concurrent OS processes with five different PIDs. Each process persists
its own authentication secret, VSS polynomials and final secret share in a
separate mode-`0600` file under `target/g3a-work-<coordinator-pid>/private/`.

The processes exchange 25 dealer-to-receiver share records through atomic local
files. Every record is bound to the DKG session and authenticated with the
dealer's Jubjub Schnorr key. Each receiver independently checks the Pedersen and
Feldman relations, derives the same public polynomial, transcript digest,
KeyCore and certified election public key, and signs that KeyCore.

After DKG, Trustees 1, 2 and 3 produce three session- and aggregate-bound signed
decryption-share vectors. The coordinator combines only these public messages;
it does not read the persisted private state files.

## Measured Release run

- Trustee processes / unique PIDs: 5 / 5.
- Successful process exits: 5 / 5.
- Independently persisted private states: 5.
- Authenticated VSS deal messages: 25.
- Authenticated decryption-share messages: 3.
- Qualified Trustees / certificate signers: 5 / 5.
- Complaints / disqualifications / extraction recoveries: 0 / 0 / 0.
- DKG start to five final states: 57.281 ms.
- Vote encryption and homomorphic fold: 5.431 ms.
- Fixed `{1,2,3}` threshold decryption: 27.352 ms.
- Total wall time: 155.093 ms.
- Authenticated local transport artifacts: 39,162 bytes.
- Coordinator / child peak RSS: 1.953 / 26.734 MiB.
- Private tally: `(4,2,4)`.
- Public winner: `1`.
- Interpolated verification key matches the certified public key: yes.

## Claim boundary

This is the honest five-process happy path over authenticated atomic local-file
transport. It is not remote networking, TLS, restart recovery, timeout handling
or a Byzantine-process experiment. Only the fixed threshold subset `{1,2,3}`
is exercised because G3-A validates process separation, not the already tested
ten-subset algebra.

G3-A does not change the cryptographic suite, so it remains bound to the active
`VERIVOTING-JUBJUB-PROTOTYPE-G2B-2.2` Manifest instead of changing its digest
and invalidating the existing G1/G2 artifacts.

## Reproduce

```bash
cargo +1.99.1 run -j 2 --release --locked -p vv-friendly-dkg \
  --bin friendly-g3a-multiprocess
```

Raw result:

- `experiments/g3_a_raw/five_process_dkg.json`

G3-B now terminates and restarts Trustee 3 after its private state is persisted,
and preserves the same transcript, KeyCore and fixed `{1,2,3}` threshold result.
See `docs/G3_B_RESTART_RECOVERY.md`.
