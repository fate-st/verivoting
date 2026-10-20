#!/usr/bin/env python3
"""Build the G0.6 curve comparison from measured JSON artifacts only."""

from __future__ import annotations

import csv
import json
import sys
from pathlib import Path


FIELDS = [
    "baseline_commit",
    "experiment_revision",
    "route",
    "component",
    "implementation",
    "encryption_group",
    "proof_curve",
    "build_profile",
    "constraints",
    "public_inputs",
    "synthesis_witness_ms",
    "setup_ms",
    "prove_ms",
    "verify_ms",
    "proof_bytes",
    "peak_rss_mb",
    "valid_relation_satisfied",
    "valid_proof_verified",
    "changed_public_input_rejected",
    "invalid_relation_rejected",
    "challenge_recomputed_in_circuit",
    "full_p3",
    "status",
]


def load(path: Path) -> dict:
    return json.loads(path.read_text())


def csv_value(value: object) -> object:
    return str(value).lower() if isinstance(value, bool) else value


def ristretto_row(report: dict) -> dict:
    preflight = report["preflight"]
    return {
        "baseline_commit": "92d147001fe84c58f85d7692ff9980b4684cd193",
        "experiment_revision": "92d147001fe84c58f85d7692ff9980b4684cd193",
        "route": "A_Ristretto255_non_native",
        "component": "single_cp_equation",
        "implementation": preflight["implementation"],
        "encryption_group": "Ristretto255/curve25519-dalek-4.1.3",
        "proof_curve": "BLS12-381/ark-groth16-0.4.0",
        "build_profile": report["build_profile"],
        "constraints": preflight["constraints"],
        "public_inputs": preflight["public_inputs"],
        "synthesis_witness_ms": f"{preflight['synthesis_witness_ms']:.6f}",
        "setup_ms": "NOT_RUN",
        "prove_ms": "NOT_RUN",
        "verify_ms": "NOT_RUN",
        "proof_bytes": "NOT_RUN",
        "peak_rss_mb": f"{report['peak_rss_mb']:.6f}",
        "valid_relation_satisfied": preflight["valid_relation_satisfied"],
        "valid_proof_verified": "NOT_RUN",
        "changed_public_input_rejected": "NOT_RUN",
        "invalid_relation_rejected": "NOT_RUN_IN_THIS_ARTIFACT",
        "challenge_recomputed_in_circuit": "false",
        "full_p3": "false",
        "status": report["status"],
    }


def friendly_row(envelope: dict, key: str) -> dict:
    experiment = envelope["experiment"]
    metrics = experiment[key]
    return {
        "baseline_commit": envelope["baseline_commit"],
        "experiment_revision": envelope["experiment_revision"],
        "route": "B_Jubjub_native_field_isolated",
        "component": metrics["component"],
        "implementation": "ark_ed_on_bls12_381_0_4",
        "encryption_group": experiment["experiment_group"],
        "proof_curve": experiment["proof_curve"],
        "build_profile": envelope["build_profile"],
        "constraints": metrics["constraints"],
        "public_inputs": metrics["public_inputs"],
        "synthesis_witness_ms": f"{metrics['synthesis_witness_ms']:.6f}",
        "setup_ms": f"{metrics['setup_ms']:.6f}",
        "prove_ms": f"{metrics['prove_ms']:.6f}",
        "verify_ms": f"{metrics['verify_ms']:.6f}",
        "proof_bytes": metrics["proof_bytes"],
        "peak_rss_mb": f"{envelope['peak_rss_mb']:.6f}",
        "valid_relation_satisfied": metrics["valid_relation_satisfied"],
        "valid_proof_verified": metrics["valid_proof_verified"],
        "changed_public_input_rejected": metrics["changed_public_input_rejected"],
        "invalid_relation_rejected": metrics["invalid_relation_rejected"],
        "challenge_recomputed_in_circuit": (
            experiment["cp_challenge_recomputed_in_circuit"]
            if key == "cp"
            else "NOT_APPLICABLE"
        ),
        "full_p3": experiment["full_p3"],
        "status": envelope["status"],
    }


def main() -> int:
    raw = Path(sys.argv[1] if len(sys.argv) > 1 else "experiments/raw")
    baseline = load(raw / "cp-optimization-baseline_256_bit_joint.json")
    optimized = load(raw / "cp-optimization-canonical_253_bit_joint.json")
    friendly = load(raw / "p3-curve-friendly.json")
    rows = [
        ristretto_row(baseline),
        ristretto_row(optimized),
        friendly_row(friendly, "cp"),
        friendly_row(friendly, "elgamal_coordinate"),
        friendly_row(friendly, "state_point_addition"),
    ]
    output = raw / "p3_curve_compare.csv"
    with output.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=FIELDS)
        writer.writeheader()
        writer.writerows(
            {key: csv_value(value) for key, value in row.items()} for row in rows
        )
    print(f"wrote {output} from {len(rows)} measured rows")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
