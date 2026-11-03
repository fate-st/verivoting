# P0/P1 execution record

Baseline: `a5204043bdcec5a788f79de2ad79225320845e64`.
Working branch: `experiment/p0-sync-g9p1-design`.
These checks ran with uncommitted changes, as recorded in `commands.json`.
They are evidence synchronization and bounded offline design checks, not new
Groth16 proofs, Solidity deployments, EVM measurements or complete SnarkPack verification.

`commands.json` records actual commands, timestamps, exit codes and stdout/stderr logs.
The first summary integration attempt failed because a new variable shadowed the
historical P1 result. The variable was renamed; `summary-1.log` is retained and
`summary-2.log` records the successful rerun. This was not a cryptographic failure.
`final-checks-1.log` is empty: the recorder disables stdin, so the initial inline
Node check received no code. Its exit code is not acceptance evidence. The actual
checks are in the reproducible `scripts/p0p1_check.mjs`, executed in the next log.
The first design-script draft also incorrectly expected a B2 archived negative;
the old B2 field is null. The final script derives one canonical nullifier change
in memory and leaves the original fixture unchanged. That draft error is described
in the design report; no draft output is represented as successful verification.

The LaTeX build uses an isolated temporary output directory. Its log does not imply
visual layout review or a complete standalone paper source bundle. The archived
paper snapshot still requires the original template class and bibliography.

`acceptance.json` records the final limited acceptance checks. `SHA256SUMS` pins
the changed/new repository sources, documents, reports and logs, excluding itself.
The paper snapshot hash also pins the updated original paper at acceptance time.
P0 passes its evidence mapping; P1 remains blocked on exact-check correspondence
and bounded GT continuation/total-resource design. P2 has not started.
