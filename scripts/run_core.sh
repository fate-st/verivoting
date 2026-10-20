#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "$0")/.." && pwd)"
cd "$project_dir"

echo "[1/4] validating and testing the implemented workspace"
cargo test --workspace --release

echo "[2/4] running native core, DKG, P1 Groth16, BSGS and E2E fixture"
cargo run --release -p vv-bench -- all --trials 1

echo "[3/4] aggregating machine-readable results"
python3 scripts/summarize.py experiments/raw experiments/aggregate

echo "[4/4] result locations"
echo "  raw:       experiments/raw"
echo "  aggregate: experiments/aggregate/summary.md"

