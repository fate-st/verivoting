# gate-a-20261117-05 acceptance (compile/static/preflight only)

| # | Item | Status |
|---|---|---|
| 1 | Four-commit chain and three explicitly named fixed trees | PASS_STATIC |
| 2 | Three fixed inventories: 59A, 3M+31A and 3M+32A | PASS_STATIC |
| 3 | Dynamic paths, phase call sites, A/B read-only inputs, all three patch hashes frozen | PASS_STATIC |
| 4 | Recursive A-01/02/03/04 archives including ignored caches and absent result A-01 | PASS_STATIC |
| 5 | Raw Legacy EVM refund counter; no zero-refund assumption | PASS_STATIC |
| 6 | Per-transaction blocks, BigInt 14 fields and receipt identities | PASS_STATIC / RUNTIME_PENDING_B |
| 7 | Prague 2537/3529/7623 active; 8037 inactive; quotient 5 | PASS_STATIC |
| 8 | Compiler storage slots and offsets 0/8/16/17 | PASS_COMPILE |
| 9 | V5 complete replacement + proof-only replacement, VM/context boundaries, hash gates and RSS guards | PASS_STATIC / RUNTIME_PENDING_B / RUNTIME_PENDING_C |
| 10 | Rust +1.99.1 Release opt-level=1, solc 0.8.30, A-03 binary reproducibility | PASS_COMPILE |
| 11 | Independent Gate B preflight only | READY_FOR_HUMAN_APPROVAL |

Setup=0; Prove=0; VM creation=0; EVM transactions=0. Runtime assertions are wired but have NOT been executed. No commit or push.

B preflight is READY_FOR_HUMAN_APPROVAL, not a B pass. B reads the A raw/results inputs; C reads both A and B raw/results inputs with full recursive before/after checks. RSS guards and B/C admission checks are statically wired, not runtime-tested.
