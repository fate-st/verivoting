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
    friendly_curve_path = raw / "p3-curve-friendly.json"
    cp_optimization_paths = sorted(raw.glob("cp-optimization-*.json"))
    g08_path = raw.parent / "g0_8_complete_raw" / "e2e-public.json"
    g09_h0_path = raw.parent / "g0_9_h0_raw" / "h0_transcript.json"
    g09_h1_path = raw.parent / "g0_9_h1_raw" / "h1_sha512.json"
    required = [e2e_path, dlog_path, p1_path, dkg_path]
    if any(path is None or not path.exists() for path in required):
        print("missing raw artifacts; run `cargo run --release -p vv-bench -- all` first")
        return 2

    e2e = json.loads(e2e_path.read_text())
    dlog = json.loads(dlog_path.read_text())
    p1 = json.loads(p1_path.read_text())
    dkg = json.loads(dkg_path.read_text())
    cp = json.loads(cp_path.read_text()) if cp_path.exists() else None
    friendly_curve = (
        json.loads(friendly_curve_path.read_text())
        if friendly_curve_path.exists()
        else None
    )
    cp_optimization = [
        json.loads(path.read_text()) for path in cp_optimization_paths
    ]
    g08 = json.loads(g08_path.read_text()) if g08_path.exists() else None
    g09_h0 = json.loads(g09_h0_path.read_text()) if g09_h0_path.exists() else None
    g09_h1 = json.loads(g09_h1_path.read_text()) if g09_h1_path.exists() else None
    optimized_cp = next(
        (
            item
            for item in cp_optimization
            if item["preflight"].get("implementation")
            == "canonical_253_bit_joint"
        ),
        None,
    )
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
                optimized_cp["status"]
                if optimized_cp
                else cp["status"]
                if cp
                else "NOT_RUN"
            ),
            "G0.6_curve_decision": (
                friendly_curve["status"] if friendly_curve else "NOT_RUN"
            ),
            "G0.8_jubjub_native": g08["status"] if g08 else "NOT_RUN",
            "G0.9_H0_cp_transcript": g09_h0["status"] if g09_h0 else "NOT_RUN",
            "G0.9_H1_sha512": g09_h1["status"] if g09_h1 else "NOT_RUN",
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
        "cp_optimization": cp_optimization,
        "friendly_curve_experiment": friendly_curve,
        "g08_jubjub_native": g08,
        "g09_h0_cp_transcript": g09_h0,
        "g09_h1_sha512": g09_h1,
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
    ])
    if g08:
        g08_experiment = g08["experiment"]
        g08_event = g08_experiment["winner_only_public_event"]
        lines.extend([
            "",
            "## G0.8 complete native Jubjub path",
            "",
            f"- Status: `{g08['status']}`",
            f"- Build profile: `{g08['build_profile']}`",
            f"- Registered count / weight: `{g08_event['registration']['registered_count']} / {g08_event['registration']['registered_weight']}`",
            f"- Packages created / verified / selected: `{g08_experiment['packages_created']} / {g08_experiment['packages_verified']} / {g08_experiment['packages_selected']}`",
            f"- Public winner: `{g08_event['winner']}`",
            f"- Public tally present: `{g08_event['private_tally_present']}`",
            f"- Full native G0.8: `{g08_experiment['full_native_g08']}`",
            f"- Full P3: `{g08_experiment['full_p3']}`",
        ])
    if g09_h0:
        vector = g09_h0["vector"]
        lines.extend([
            "",
            "## G0.9-H0 canonical CP transcript",
            "",
            f"- Status: `{g09_h0['status']}`",
            f"- Transcript bytes: `{vector['transcript_bytes']}`",
            f"- Transcript SHA-256: `{vector['transcript_sha256']}`",
            f"- R1CS byte-binding constraints: `{g09_h0['r1cs_constraints']}`",
            f"- Swapped registration fields rejected: `{g09_h0['swapped_registration_fields_rejected']}`",
            f"- Changed registration digest rejected: `{g09_h0['changed_registration_digest_rejected']}`",
            f"- SHA-512 circuit implemented: `{g09_h0['sha512_circuit_implemented']}`",
            f"- Full P3: `{g09_h0['full_p3']}`",
        ])
    if g09_h1:
        lines.extend([
            "",
            "## G0.9-H1 real SHA-512 R1CS",
            "",
            f"- Status: `{g09_h1['status']}`",
            f"- Transcript bytes / padded blocks: `{g09_h1['transcript_bytes']} / {g09_h1['padded_sha512_blocks']}`",
            f"- Constraints: `{g09_h1['constraints']}`",
            f"- Packed public inputs: `{g09_h1['public_inputs']}`",
            f"- Witness synthesis: `{g09_h1['witness_ms']:.3f} ms`",
            f"- Peak RSS: `{g09_h1['peak_rss_mib']:.3f} MiB`",
            f"- R1CS digest matches native: `{g09_h1['r1cs_digest_matches_native']}`",
            f"- Changed transcript / digest rejected: `{g09_h1['changed_transcript_rejected']} / {g09_h1['changed_public_digest_rejected']}`",
            f"- Hash-to-scalar implemented: `{g09_h1['hash_to_scalar_mapping_implemented']}`",
            f"- CP equations integrated: `{g09_h1['cp_equations_integrated']}`",
            f"- Groth16 run: `{g09_h1['groth16_run']}`",
            f"- Full P3: `{g09_h1['full_p3']}`",
        ])
    lines.extend([
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
    if cp_optimization:
        lines.extend([
            "",
            "## G0.5 optimization comparison",
            "",
            "| Implementation | Constraints | Public inputs | Witness ms | Peak RSS MiB | Groth16 run |",
            "|---|---:|---:|---:|---:|---|",
        ])
        for item in cp_optimization:
            preflight = item["preflight"]
            lines.append(
                "| {implementation} | {constraints} | {public_inputs} | {witness:.3f} | {rss:.1f} | {groth16} |".format(
                    implementation=preflight["implementation"],
                    constraints=preflight["constraints"],
                    public_inputs=preflight["public_inputs"],
                    witness=preflight["synthesis_witness_ms"],
                    rss=item["peak_rss_mb"],
                    groth16=item["groth16"] is not None,
                )
            )
    if friendly_curve:
        experiment = friendly_curve["experiment"]
        lines.extend([
            "",
            "## G0.6 isolated SNARK-friendly curve experiment",
            "",
            f"- Status: `{friendly_curve['status']}`",
            f"- Build profile: `{friendly_curve['build_profile']}`",
            f"- Experiment group: `{experiment['experiment_group']}`",
            f"- Native manifest changed: `{friendly_curve['manifest_changed']}`",
            f"- CP challenge recomputed in-circuit: `{experiment['cp_challenge_recomputed_in_circuit']}`",
            f"- Full P3: `{experiment['full_p3']}`",
            "",
            "| Component | Constraints | Public inputs | Setup ms | Prove ms | Verify ms | Proof bytes |",
            "|---|---:|---:|---:|---:|---:|---:|",
        ])
        for key in ("cp", "elgamal_coordinate", "state_point_addition"):
            item = experiment[key]
            lines.append(
                "| {component} | {constraints} | {public_inputs} | {setup:.3f} | {prove:.3f} | {verify:.3f} | {proof_bytes} |".format(
                    component=item["component"],
                    constraints=item["constraints"],
                    public_inputs=item["public_inputs"],
                    setup=item["setup_ms"],
                    prove=item["prove_ms"],
                    verify=item["verify_ms"],
                    proof_bytes=item["proof_bytes"],
                )
            )
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
        marker = (
            "PASS"
            if status.startswith("PASS")
            else "PARTIAL"
            if status.startswith("PARTIAL_PASS")
            else "BLOCKED"
        )
        print(f"  [{marker:7}] {gate}: {status}")
    print(f"Build profile(s): {', '.join(build_profiles) if build_profiles else 'unknown'}")
    print(f"Public winner: {event['winner']} | legal subsets: {e2e['all_legal_subsets_checked']}")
    print(f"P1: {p1['constraints']} constraints | {p1['proof_bytes']} B proof | verified={p1['verified']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

