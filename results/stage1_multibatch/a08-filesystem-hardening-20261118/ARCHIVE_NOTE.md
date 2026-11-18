# Archive: A-08 filesystem hardening (Stages 1–2 only)

This append-only archive extends f9fc0ec903329db6bcc4123f8b8b094aec78e0ac; the experimental branches and their HEADs remain unchanged. The upload is made in a standalone local clone with its own .git directory, not in either experimental worktree.

## Included

- Exact current Runner, native C source and macOS arm64 helper binary (fixed binary SHA-256 is preserved).
- Fourteen top-level final test/report artifacts, byte-for-byte; the original SHA256SUMS is kept verbatim.
- Read-only historical-integrity checker, upload inventory and before/after integrity evidence.
- Thirteen real filesystem test results: PASS_A08_FILESYSTEM_HARDENING.

## Explicit exclusions and limits

No research_pk.bin, build-target, target cache, node_modules, private witness, simulated filesystem tree or symbolic link is included. Frozen research PK remains local: 847800624 bytes, SHA-256 448d9d92b9250aa2bcbf59c82036ab1615f2fe323bbb5a39b8f8d7ec9f73da1c. This Git archive does not contain the proving key and is not a standalone complete proof-reproduction package.

The native binary targets macOS arm64. The Runner and archived test harness deliberately retain the locked absolute integration/local evidence paths. No portability changes or rebuilt helper are introduced during upload. The supporting checker is copied under check_readonly_original_workspace.mjs; the original test's absolute reference is preserved. First-build test evidence remains local at /private/tmp/vv-a08-path-guard-test-Q9X1St and is superseded by the final fingerprinted run archived here.

No credentials were detected by the explicit pattern scan; evidence copies are unredacted. Directory names, username, paths and public source/hash metadata remain visible. The pre-upload execution report is historical: its statement that no commit/push ran applies to the filesystem experiment, not to this separately authorized archival upload.

The archival staged diff reports one trailing blank line at the end of the original REPORT.md. It is preserved intentionally to keep the raw evidence and its recorded SHA-256 byte-identical; no source-code whitespace errors were reported. This evidence-format warning is not represented as a clean full archival diff check.

## Gate state

A-06/B-02 and A-07/B-03 remain FROZEN. Gate A-08 is NOT CREATED (three old results preparation files remain untouched), Stage 3 is NOT STARTED, Gate C is NOT AUTHORIZED / NOT STARTED, Gate D is NOT STARTED. No Rust/Solidity compilation, Setup, Prove, Worker or EVM execution was performed by this upload.

Native helper remaining limits: an opened directory FD follows its inode after same-UID rename; SHA-check-to-exec still has a theoretical same-UID replacement window. No universal local filesystem attack resistance is claimed.
