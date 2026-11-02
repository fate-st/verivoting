# G8-E execution evidence

Only accounting/checking commands ran; no EVM, proving, signing or CP generation.
The source experiment is the archived G8-D execution, not a newly measured run.

The first accounting and deterministic-check commands failed while reading the
committed central summary through Git: Node spawnSync's default output buffer
was smaller than the archived JSON (ENOBUFS). No derived result had been produced.
The exact failed generator is retained in failed-sources/. After increasing the
read buffer to 16 MiB, both commands passed. No old source/raw/receipts were changed.

commands.json contains actual argv, exit codes, attempts, elapsed command time and
logs. Command wall time is NOT a new cryptographic performance measurement.
