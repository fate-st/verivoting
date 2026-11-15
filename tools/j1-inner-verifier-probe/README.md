# J1 isolated compile probes

This package is deliberately NOT a member of the protocol workspace. Its own
Cargo.lock pins arkworks 0.4 and enables ark-groth16/r1cs without changing the
original project's Cargo.toml, Cargo.lock, libraries or protocol Suite.

The shared source lives in crates/vv-friendly-crypto/src/joint but is compiled
only by the explicit targets in this isolated manifest. It is not exported by
vv-friendly-crypto and is not an implemented Fr recursive verifier.

Positive compile control: j1-native-fq-compile typechecks a full Fq constraint
body (constant VK, checked proof allocation, public inputs, pairing verification,
and enforce_true). It DOES NOT call that body or synthesize any R1CS.

Expected compile rejection: j1-current-outer-fr-rejected asks the exact same
verifier to implement SNARKGadget<Fr,Fr,Groth16<Bls12_381>>. Locked rustc 1.99.1
reports E0271 (FqConfig vs FrConfig). This is a domain compatibility test, not a
cryptographic mutation test. Do not run all-targets on this package and expect
success; the original workspace's all-targets does not include it.

From the repo root, use the bounded orchestrator with a NEW directory:

```sh
node scripts/joint_proof_run_j1.mjs experiments/joint_proof_raw/NEW_RUN_NAME
# Read-only verification of the archived final attempt:
node scripts/joint_proof_check.mjs experiments/joint_proof_raw/run-20261114-03
```

No Setup/Prove, no circuit satisfaction test, no J2-J5, no voter secrets.
See docs/JOINT_PROOF_FEASIBILITY.md and docs/JOINT_PROOF_COMPARISON.md.
