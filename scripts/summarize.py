#!/usr/bin/env python3
"""Summarize only measured artifacts; never synthesize blocked-gate values."""

from __future__ import annotations

import csv
import json
import sys
from pathlib import Path


def newest(paths: list[Path]) -> Path | None:
    return max(paths, key=lambda path: path.stat().st_mtime, default=None)


def main() -> int:
    raw = Path(sys.argv[1] if len(sys.argv) > 1 else "experiments/raw")
    aggregate = Path(sys.argv[2] if len(sys.argv) > 2 else "experiments/aggregate")
    aggregate.mkdir(parents=True, exist_ok=True)

    e2e_path = raw / "e2e-public.json"
    dlog_path = raw / "dlog.json"
    p1_path = newest(list(raw.glob("p1-trial-*.json")))
    dkg_path = newest(list(raw.glob("dkg-trial-*.json")))
    cp_path = raw / "p3-feasibility.json"
    required = [e2e_path, dlog_path, p1_path, dkg_path]
    if any(path is None or not path.exists() for path in required):
        print("missing raw artifacts; run `cargo run --release -p vv-bench -- all` first")
        return 2

    e2e = json.loads(e2e_path.read_text())
    dlog = json.loads(dlog_path.read_text())
    p1 = json.loads(p1_path.read_text())
    dkg = json.loads(dkg_path.read_text())
    cp = json.loads(cp_path.read_text()) if cp_path.exists() else None
    csv_rows = []
    runs_path = raw / "runs.csv"
    if runs_path.exists():
        with runs_path.open(newline="") as handle:
            csv_rows = list(csv.DictReader(handle))
    build_profiles = sorted(
        {row.get("compiler_flags", "unknown") for row in csv_rows if row.get("compiler_flags")}
    )

    summary = {
        "classification": "measured-native-core-and-real-p1-not-full-p3",
        "gates": {
            "G0_native_core_and_p1": "PASS",
            "G0.5_cp_single_equation": (
                cp["status"] if cp else "NOT_RUN"
            ),
            "G1_snarkpack": "BLOCKED_NOT_INTEGRATED",
            "G2_state_proof": "BLOCKED_CIRCUIT_NOT_IMPLEMENTED",
            "G3_multiprocess": "BLOCKED_SINGLE_PROCESS_SIMULATION_ONLY",
            "G4_full_p3": "BLOCKED_NON_NATIVE_NESTED_CP_NOT_IMPLEMENTED",
            "G5_target_chain": "BLOCKED_CONTRACTS_NOT_IMPLEMENTED",
        },
        "dkg": dkg,
        "p1": p1,
        "dlog": dlog,
        "e2e": e2e,
        "cp_feasibility": cp,
        "csv_rows_seen": len(csv_rows),
        "build_profiles": build_profiles,
    }
    (aggregate / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")

    event = e2e["public_event"]
    lines = [
        "# VeriVoting-Batch measured summary",
        "",
        f"Build profile(s): `{', '.join(build_profiles) if build_profiles else 'unknown'}`",
        "",
        "## Gate status",
        "",
        "| Gate | Status |",
        "|---|---|",
    ]
    lines.extend(f"| {gate} | {status} |" for gate, status in summary["gates"].items())
    lines.extend([
        "",
        "## Four-voter end-to-end fixture",
        "",
        f"- Public winner: `{event['winner']}`",
        f"- P1 Groth16 verified: `{event['p1_groth16_verified']}`",
        f"- Full P3 generated: `{event['full_p3_generated']}`",
        f"- Public tally present: `{event['public_tally_present']}`",
        f"- Legal 3-of-5 subsets checked: `{e2e['all_legal_subsets_checked']}`",
        f"- Public artifact bytes: `{e2e['public_bytes']}`",
        f"- Private package bytes: `{e2e['private_package_bytes']}`",
        "",
        "## Real P1 Groth16",
        "",
        f"- Constraints: `{p1['constraints']}`",
        f"- Proof bytes: `{p1['proof_bytes']}`",
        f"- Setup: `{p1['setup_ms']:.3f} ms`",
        f"- Prove: `{p1['prove_ms']:.3f} ms`",
        f"- Verify: `{p1['verify_ms']:.3f} ms`",
        f"- Altered public winner rejected: `{p1['wrong_public_winner_rejected']}`",
    ])
    if cp:
        preflight = cp["preflight"]
        lines.extend([
            "",
            "## G0.5 CP feasibility",
            "",
            f"- Status: `{cp['status']}`",
            f"- Real native CP verified: `{cp['native_cp_verified']}`",
            f"- R1CS witness satisfied: `{preflight['valid_relation_satisfied']}`",
            f"- Constraints: `{preflight['constraints']}`",
            f"- Public inputs: `{preflight['public_inputs']}`",
            f"- Peak RSS: `{cp['peak_rss_mb']:.1f} MiB`",
            f"- Groth16 setup/prove/verify run: `{cp['groth16'] is not None}`",
        ])
    lines.extend([
        "",
        "## Bounded discrete log",
        "",
        "| Bound | Decode ms | Decoded |",
        "|---:|---:|---:|",
    ])
    lines.extend(
        f"| {item['bound']} | {item['decode_ms']:.3f} | {item['decoded']} |"
        for item in dlog
    )
    lines.extend([
        "",
        "> This report contains no fabricated values for blocked gates and does not claim full P3 security.",
        "",
    ])
    (aggregate / "summary.md").write_text("\n".join(lines))

    print("Gate status:")
    for gate, status in summary["gates"].items():
        marker = "PASS" if status == "PASS" else "BLOCKED"
        print(f"  [{marker:7}] {gate}: {status}")
    print(f"Build profile(s): {', '.join(build_profiles) if build_profiles else 'unknown'}")
    print(f"Public winner: {event['winner']} | legal subsets: {e2e['all_legal_subsets_checked']}")
    print(f"P1: {p1['constraints']} constraints | {p1['proof_bytes']} B proof | verified={p1['verified']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

