# A-08 pre-build call-chain review

Reviewed before creating the A-08 raw baseline. A-07's existing integrationGateA only invokes rustc/cargo and does not execute a materials binary. The new main dispatches A-08 explicitly to gateA08, not legacy gateA/integrationNextGate. gateA08 performs read-only frozen source/PK/request/Setup hash checks, invokes rustc +1.99.1 -Vv and cargo +1.99.1 build --release --locked --offline, copies the compiled native binary only into the NEW A-08 directory, and calls local solc compileSolidity. No Setup/Prove/prepare/prove-range/createVM/runTx call is executed along this path. A-07/B-03 are read-only frozen inputs. Source changes are limited to Runner, Auditor, Rust diagnostics, and docs. Cargo manifests, lockfile, circuits, fixtures and Solidity are not edited.

The new native control/authentication paths are engineering gates, not changes to the R1CS or the proof RNG derivation. Existing loaded-PK canonical validation remains enabled. Missing frozen Setup must block; there is no Setup regeneration fallback in prepare.

The preliminary Rust compile finished successfully (cargo +1.99.1, release, locked, offline, build jobs 2, Rayon 4; compiler output: Finished release profile [optimized] in 5.73s). This preliminary build preceded final formatting/diagnostic edits; the final A-08 build receipt is authoritative.

Process-identity probe: sandboxed /bin/ps execution returned EPERM; the explicitly escalated read-only probe succeeded using libproc microsecond identity, with no signals, Workers, proof generation or VM. The system Python bridge is a new operational dependency; it changes no Cargo dependency. Restricted process-information permissions must yield CLEANUP_INCOMPLETE, never guessed cleanup success.
