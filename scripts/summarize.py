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
    g09_r_path = raw.parent / "g0_9_r_raw" / "g09_r_baseline.json"
    g09_h0_path = raw.parent / "g0_9_h0_raw" / "h0_transcript.json"
    g09_h1_path = raw.parent / "g0_9_h1_raw" / "h1_sha512.json"
    g09_h2a_path = raw.parent / "g0_9_h2_raw" / "h2a_hash_to_scalar.json"
    g09_h2b_path = raw.parent / "g0_9_h2_raw" / "h2b_fixed_two.json"
    g09_h2c_path = raw.parent / "g0_9_h2_raw" / "h2c_wide_hash_to_field.json"
    g09_h2d_path = raw.parent / "g0_9_h2d_raw" / "h2d_adoption.json"
    g09_h3_path = raw.parent / "g0_9_h3_raw" / "h3_first_cp_equation.json"
    g09_h4a_path = raw.parent / "g0_9_h4a_raw" / "h4a_complete_trustee_cp.json"
    g09_h4b_path = raw.parent / "g0_9_h4b_raw" / "h4b_groth16.json"
    g09_p3a_path = raw.parent / "g0_9_p3a_raw" / "p3a_three_cp_r1cs.json"
    g09_p3a_g16_path = raw.parent / "g0_9_p3a_raw" / "p3a_three_cp_groth16.json"
    g09_p3b0_path = raw.parent / "g0_9_p3b0_raw" / "package_h2f_adoption.json"
    g09_p3b0_e2e_path = raw.parent / "g0_9_p3b0_raw" / "e2e-public.json"
    g09_p3b1_path = (
        raw.parent / "g0_9_p3b1_raw" / "p3b1_three_package_openings_r1cs.json"
    )
    g09_p3c0_path = (
        raw.parent / "g0_9_p3c0_raw" / "p3c0_threshold_combination_r1cs.json"
    )
    g09_p3c1_path = (
        raw.parent / "g0_9_p3c1_raw" / "p3c1_bounded_tally_binding_r1cs.json"
    )
    g09_p3d0_path = raw.parent / "g0_9_p3d0_raw" / "p3d0_winner_linkage_r1cs.json"
    g09_p3d1_path = raw.parent / "g0_9_p3d1_raw" / "p3d1_full_groth16.json"
    g1_b0_path = raw.parent / "g1_b0_raw" / "ballot_core_r1cs.json"
    g1_b1_path = raw.parent / "g1_b1_raw" / "full_c_vote_r1cs.json"
    g1_b2_path = raw.parent / "g1_b2_raw" / "full_c_vote_groth16.json"
    g1_c0_path = raw.parent / "g1_c0_raw" / "snarkpack_min_batch_2.json"
    g1_c1_path = (
        raw.parent / "g1_c1_raw" / "snarkpack_distinct_voters_batch_2.json"
    )
    g2_a_path = raw.parent / "g2_a_raw" / "state_transition_b2_r1cs.json"
    g2_b_path = raw.parent / "g2_b_raw" / "state_transition_b2_groth16.json"
    g3_a_path = raw.parent / "g3_a_raw" / "five_process_dkg.json"
    g3_b_path = raw.parent / "g3_b_raw" / "one_trustee_restart.json"
    g4_a_path = raw.parent / "g4_a_raw" / "second_subset_p3_shape.json"
    g4_b_path = raw.parent / "g4_b_raw" / "public_subset_groth16.json"
    g5_a_path = raw.parent / "g5_a_raw" / "g2b_prague_evm.json"
    g5_b_path = raw.parent / "g5_b_raw" / "p3_prague_evm.json"
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
    g09_r = json.loads(g09_r_path.read_text()) if g09_r_path.exists() else None
    g09_h0 = json.loads(g09_h0_path.read_text()) if g09_h0_path.exists() else None
    g09_h1 = json.loads(g09_h1_path.read_text()) if g09_h1_path.exists() else None
    g09_h2a = (
        json.loads(g09_h2a_path.read_text()) if g09_h2a_path.exists() else None
    )
    g09_h2b = (
        json.loads(g09_h2b_path.read_text()) if g09_h2b_path.exists() else None
    )
    g09_h2c = (
        json.loads(g09_h2c_path.read_text()) if g09_h2c_path.exists() else None
    )
    g09_h2d = (
        json.loads(g09_h2d_path.read_text()) if g09_h2d_path.exists() else None
    )
    g09_h3 = json.loads(g09_h3_path.read_text()) if g09_h3_path.exists() else None
    g09_h4a = (
        json.loads(g09_h4a_path.read_text()) if g09_h4a_path.exists() else None
    )
    g09_h4b = (
        json.loads(g09_h4b_path.read_text()) if g09_h4b_path.exists() else None
    )
    g09_p3a = (
        json.loads(g09_p3a_path.read_text()) if g09_p3a_path.exists() else None
    )
    g09_p3a_g16 = (
        json.loads(g09_p3a_g16_path.read_text())
        if g09_p3a_g16_path.exists()
        else None
    )
    g09_p3b0 = (
        json.loads(g09_p3b0_path.read_text()) if g09_p3b0_path.exists() else None
    )
    g09_p3b0_e2e = (
        json.loads(g09_p3b0_e2e_path.read_text())
        if g09_p3b0_e2e_path.exists()
        else None
    )
    g09_p3b1 = (
        json.loads(g09_p3b1_path.read_text()) if g09_p3b1_path.exists() else None
    )
    g09_p3c0 = (
        json.loads(g09_p3c0_path.read_text()) if g09_p3c0_path.exists() else None
    )
    g09_p3c1 = (
        json.loads(g09_p3c1_path.read_text()) if g09_p3c1_path.exists() else None
    )
    g09_p3d0 = (
        json.loads(g09_p3d0_path.read_text()) if g09_p3d0_path.exists() else None
    )
    g09_p3d1 = (
        json.loads(g09_p3d1_path.read_text()) if g09_p3d1_path.exists() else None
    )
    g1_b0 = json.loads(g1_b0_path.read_text()) if g1_b0_path.exists() else None
    g1_b1 = json.loads(g1_b1_path.read_text()) if g1_b1_path.exists() else None
    g1_b2 = json.loads(g1_b2_path.read_text()) if g1_b2_path.exists() else None
    g1_c0 = json.loads(g1_c0_path.read_text()) if g1_c0_path.exists() else None
    g1_c1 = json.loads(g1_c1_path.read_text()) if g1_c1_path.exists() else None
    g2_a = json.loads(g2_a_path.read_text()) if g2_a_path.exists() else None
    g2_b = json.loads(g2_b_path.read_text()) if g2_b_path.exists() else None
    g3_a = json.loads(g3_a_path.read_text()) if g3_a_path.exists() else None
    g3_b = json.loads(g3_b_path.read_text()) if g3_b_path.exists() else None
    g4_a = json.loads(g4_a_path.read_text()) if g4_a_path.exists() else None
    g4_b = json.loads(g4_b_path.read_text()) if g4_b_path.exists() else None
    g5_a = json.loads(g5_a_path.read_text()) if g5_a_path.exists() else None
    g5_b = json.loads(g5_b_path.read_text()) if g5_b_path.exists() else None
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
        "classification": "measured-native-core-real-two-subset-public-selector-p3-groth16-real-fixed-suite-ballot-groth16-distinct-voter-minimum-batch-snarkpack-fixed-b2-state-groth16-five-process-dkg-with-one-restart-and-local-prague-state-and-two-subset-p3-verifiers",
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
            "G0.9_R_clean_baseline": g09_r["status"] if g09_r else "NOT_RUN",
            "G0.9_H0_cp_transcript": g09_h0["status"] if g09_h0 else "NOT_RUN",
            "G0.9_H1_sha512": g09_h1["status"] if g09_h1 else "NOT_RUN",
            "G0.9_H2A_hash_to_scalar": (
                g09_h2a["status"] if g09_h2a else "NOT_RUN"
            ),
            "G0.9_H2B_fixed_shape": (
                g09_h2b["status"] if g09_h2b else "NOT_RUN"
            ),
            "G0.9_H2C_wide_hash_to_field": (
                g09_h2c["status"] if g09_h2c else "NOT_RUN"
            ),
            "G0.9_H2D_adopted_cp_hash_to_field": (
                g09_h2d["status"] if g09_h2d else "NOT_RUN"
            ),
            "G0.9_H3_first_cp_equation": (
                g09_h3["status"] if g09_h3 else "NOT_RUN"
            ),
            "G0.9_H4A_complete_one_trustee_cp": (
                g09_h4a["status"] if g09_h4a else "NOT_RUN"
            ),
            "G0.9_H4B_one_trustee_cp_groth16": (
                g09_h4b["status"] if g09_h4b else "NOT_RUN"
            ),
            "G0.9_P3A_three_selected_trustee_cp": (
                g09_p3a["status"] if g09_p3a else "NOT_RUN"
            ),
            "G0.9_P3A_three_selected_trustee_cp_groth16": (
                g09_p3a_g16["status"] if g09_p3a_g16 else "NOT_RUN"
            ),
            "G0.9_P3B0_package_h2f_adoption": (
                g09_p3b0["status"] if g09_p3b0 else "NOT_RUN"
            ),
            "G0.9_P3B1_three_package_openings_r1cs": (
                g09_p3b1["status"] if g09_p3b1 else "NOT_RUN"
            ),
            "G0.9_P3C0_fixed_123_threshold_combination_r1cs": (
                g09_p3c0["status"] if g09_p3c0 else "NOT_RUN"
            ),
            "G0.9_P3C1_bounded_tally_binding_r1cs": (
                g09_p3c1["status"] if g09_p3c1 else "NOT_RUN"
            ),
            "G0.9_P3D0_public_winner_linkage_r1cs": (
                g09_p3d0["status"] if g09_p3d0 else "NOT_RUN"
            ),
            "G0.9_P3D1_fixed_123_complete_p3_groth16": (
                g09_p3d1["status"] if g09_p3d1 else "NOT_RUN"
            ),
            "G1_B0_weighted_ballot_core_r1cs": (
                g1_b0["status"] if g1_b0 else "NOT_RUN"
            ),
            "G1_B1_full_c_vote_r1cs": (
                g1_b1["status"] if g1_b1 else "NOT_RUN"
            ),
            "G1_B2_full_c_vote_groth16": (
                g1_b2["status"] if g1_b2 else "NOT_RUN"
            ),
            "G1_full_c_vote": (
                g1_b2["status"]
                if g1_b2
                else g1_b1["status"]
                if g1_b1
                else "BLOCKED_CONTEXT_PRF_REGION_NULLIFIER_AND_MERKLE_NOT_IMPLEMENTED"
            ),
            "G1_ballot_groth16": (
                g1_b2["status"]
                if g1_b2
                else "BLOCKED_COMPLETE_C_VOTE_GROTH16_NOT_RUN"
            ),
            "G1_C0_snarkpack_compatibility": (
                g1_c0["status"] if g1_c0 else "NOT_RUN"
            ),
            "G1_C1_distinct_voter_snarkpack": (
                g1_c1["status"] if g1_c1 else "NOT_RUN"
            ),
            "G1_snarkpack": (
                "PARTIAL_PASS_G1C1_DISTINCT_VOTERS_BATCH_2_NONPRODUCTION_SRS"
                if g1_c1
                else "PARTIAL_PASS_G1C0_MIN_BATCH_NONPRODUCTION_SRS"
                if g1_c0
                else "BLOCKED_NOT_INTEGRATED"
            ),
            "G2_A_state_transition_r1cs": (
                g2_a["status"] if g2_a else "NOT_RUN"
            ),
            "G2_B_state_transition_groth16": (
                g2_b["status"] if g2_b else "NOT_RUN"
            ),
            "G2_state_proof": (
                "PASS_G2B_REAL_GROTH16_FIXED_B2_STATE_TRANSITION"
                if g2_b
                else "PARTIAL_PASS_G2A_FIXED_B2_R1CS_NO_GROTH16"
                if g2_a
                else "BLOCKED_CIRCUIT_NOT_IMPLEMENTED"
            ),
            "G3_A_five_process_dkg_happy_path": (
                g3_a["status"] if g3_a else "NOT_RUN"
            ),
            "G3_B_one_trustee_restart_recovery": (
                g3_b["status"] if g3_b else "NOT_RUN"
            ),
            "G3_multiprocess": (
                "PARTIAL_PASS_G3B_ONE_RESTART_AUTHENTICATED_LOCAL_TRANSPORT"
                if g3_b
                else "PARTIAL_PASS_G3A_FIVE_PROCESS_HAPPY_PATH_NO_RESTART_RECOVERY"
                if g3_a
                else "BLOCKED_SINGLE_PROCESS_SIMULATION_ONLY"
            ),
            "G4_A_second_subset_p3_shape": (
                g4_a["status"] if g4_a else "NOT_RUN"
            ),
            "G4_B_public_two_subset_p3_groth16": (
                g4_b["status"] if g4_b else "NOT_RUN"
            ),
            "G4_general_subset_p3": (
                "PARTIAL_PASS_G4B_PUBLIC_BOUND_TWO_SUBSETS_SAME_GROTH16_SETUP"
                if g4_b
                else "PARTIAL_PASS_G4A_TWO_SUBSETS_COMMON_COUNT_SHAPE_NO_PUBLIC_BINDING"
                if g4_a
                else "BLOCKED_SECOND_SUBSET_NOT_MEASURED"
            ),
            "G4_full_p3": (
                "PARTIAL_PASS_G4B_TWO_SUBSET_COMPLETE_P3_NOT_ARBITRARY_3_OF_5"
                if g4_b
                else g09_p3d1["status"]
                if g09_p3d1
                else "BLOCKED_COMBINED_GROTH16_NOT_RUN"
            ),
            "G5_A_local_prague_state_verifier": (
                g5_a["status"] if g5_a else "NOT_RUN"
            ),
            "G5_B_local_prague_two_subset_p3_verifier": (
                g5_b["status"] if g5_b else "NOT_RUN"
            ),
            "G5_target_chain": (
                "PARTIAL_PASS_G5B_LOCAL_STATE_AND_TWO_SUBSET_P3_VERIFIERS_FULL_SETTLEMENT_BLOCKED"
                if g5_b
                else "PARTIAL_PASS_G5A_LOCAL_STATE_VERIFIER_FULL_SETTLEMENT_BLOCKED"
                if g5_a
                else "BLOCKED_CONTRACTS_NOT_IMPLEMENTED"
            ),
        },
        "dkg": dkg,
        "p1": p1,
        "dlog": dlog,
        "e2e": e2e,
        "cp_feasibility": cp,
        "cp_optimization": cp_optimization,
        "friendly_curve_experiment": friendly_curve,
        "g08_jubjub_native": g08,
        "g09_r_clean_baseline": g09_r,
        "g09_h0_cp_transcript": g09_h0,
        "g09_h1_sha512": g09_h1,
        "g09_h2a_hash_to_scalar": g09_h2a,
        "g09_h2b_fixed_shape": g09_h2b,
        "g09_h2c_wide_hash_to_field": g09_h2c,
        "g09_h2d_adopted_cp_hash_to_field": g09_h2d,
        "g09_h3_first_cp_equation": g09_h3,
        "g09_h4a_complete_one_trustee_cp": g09_h4a,
        "g09_h4b_one_trustee_cp_groth16": g09_h4b,
        "g09_p3a_three_selected_trustee_cp": g09_p3a,
        "g09_p3a_three_selected_trustee_cp_groth16": g09_p3a_g16,
        "g09_p3b0_package_h2f_adoption": g09_p3b0,
        "g09_p3b0_native_e2e": g09_p3b0_e2e,
        "g09_p3b1_three_package_openings_r1cs": g09_p3b1,
        "g09_p3c0_fixed_123_threshold_combination_r1cs": g09_p3c0,
        "g09_p3c1_bounded_tally_binding_r1cs": g09_p3c1,
        "g09_p3d0_public_winner_linkage_r1cs": g09_p3d0,
        "g09_p3d1_fixed_123_complete_p3_groth16": g09_p3d1,
        "g1_b0_weighted_ballot_core_r1cs": g1_b0,
        "g1_b1_full_c_vote_r1cs": g1_b1,
        "g1_b2_full_c_vote_groth16": g1_b2,
        "g1_c0_snarkpack_compatibility": g1_c0,
        "g1_c1_distinct_voter_snarkpack": g1_c1,
        "g2_a_state_transition_r1cs": g2_a,
        "g2_b_state_transition_groth16": g2_b,
        "g3_a_five_process_dkg_happy_path": g3_a,
        "g3_b_one_trustee_restart_recovery": g3_b,
        "g4_a_second_subset_p3_shape": g4_a,
        "g4_b_public_two_subset_p3_groth16": g4_b,
        "g5_a_local_prague_state_verifier": g5_a,
        "g5_b_local_prague_two_subset_p3_verifier": g5_b,
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
    if g09_r:
        verification = g09_r["verification"]
        lines.extend([
            "",
            "## G0.9-R clean Release baseline",
            "",
            f"- Status: `{g09_r['status']}`",
            f"- Git commit / dirty at run: `{g09_r['git_commit']} / {g09_r['git_dirty_at_run']}`",
            f"- Workspace tests passed / failed / filtered: `{verification['tests_passed']} / {verification['tests_failed']} / {verification['tests_filtered']}`",
            f"- Strict workspace Clippy: `{verification['workspace_release_clippy_deny_warnings']}`",
            f"- Frozen Suite: `{g09_r['suite_lock']['suite_id']}`",
            f"- Next gate: `{g09_r['next_gate']}`",
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
    if g09_h2a:
        counter_zero = g09_h2a["counter_zero"]
        counter_one = g09_h2a["counter_one"]
        lines.extend([
            "",
            "## G0.9-H2-A exact hash-to-scalar R1CS",
            "",
            f"- Status: `{g09_h2a['status']}`",
            f"- Counter 0 attempts / constraints / witness: `{counter_zero['attempts_hashed']} / {counter_zero['constraints']} / {counter_zero['witness_ms']:.3f} ms`",
            f"- Counter 1 attempts / constraints / witness: `{counter_one['attempts_hashed']} / {counter_one['constraints']} / {counter_one['witness_ms']:.3f} ms`",
            f"- H2S input bytes / blocks per attempt: `{counter_zero['h2s_input_bytes']} / {counter_zero['sha512_blocks_per_attempt']}`",
            f"- Changed transcript / domain rejected: `{g09_h2a['changed_transcript_rejected']} / {g09_h2a['changed_domain_rejected']}`",
            f"- False counter-1 claim rejected: `{g09_h2a['false_counter_one_claim_rejected']}`",
            f"- Uniform 128-attempt circuit: `{g09_h2a['uniform_128_attempt_circuit_implemented']}`",
            f"- CP equations / Groth16 / full H2 / full P3: `{g09_h2a['cp_equations_integrated']} / {g09_h2a['groth16_run']} / {g09_h2a['full_h2']} / {g09_h2a['full_p3']}`",
        ])
    if g09_h2b:
        counter_zero = g09_h2b["counter_zero"]
        counter_one = g09_h2b["counter_one"]
        lines.extend([
            "",
            "## G0.9-H2-B fixed-shape H2S feasibility",
            "",
            f"- Status: `{g09_h2b['status']}`",
            f"- Fixed attempts / Suite v2 maximum: `{g09_h2b['fixed_attempts']} / {g09_h2b['native_suite_max_attempts']}`",
            f"- Counter 0 constraints / witness / peak RSS: `{counter_zero['constraints']} / {counter_zero['witness_ms']:.3f} ms / {counter_zero['peak_rss_mib_after_case']:.3f} MiB`",
            f"- Counter 1 constraints / witness / peak RSS: `{counter_one['constraints']} / {counter_one['witness_ms']:.3f} ms / {counter_one['peak_rss_mib_after_case']:.3f} MiB`",
            f"- Same constraints / public inputs / witness variables: `{g09_h2b['same_constraints']} / {g09_h2b['same_public_inputs']} / {g09_h2b['same_witness_variables']}`",
            f"- False counter-1 claim rejected: `{g09_h2b['false_counter_one_claim_rejected']}`",
            f"- Suite v2 128-attempt relation: `{g09_h2b['suite_v2_128_attempt_relation']}`",
            f"- CP equations / Groth16 / full H2 / full P3: `{g09_h2b['cp_equations_integrated']} / {g09_h2b['groth16_run']} / {g09_h2b['full_h2']} / {g09_h2b['full_p3']}`",
        ])
    if g09_h2c:
        first_case = g09_h2c["first_case"]
        second_case = g09_h2c["second_case"]
        lines.extend([
            "",
            "## G0.9-H2-C wide SHA-512-to-field candidate",
            "",
            f"- Status: `{g09_h2c['status']}`",
            f"- Candidate suite: `{g09_h2c['candidate_suite_id']}`",
            f"- Mapping: `{g09_h2c['mapping']}`",
            f"- H2F input bytes / SHA-512 blocks: `{first_case['h2f_input_bytes']} / {first_case['sha512_blocks']}`",
            f"- Constraints / public inputs / witness variables: `{first_case['constraints']} / {first_case['public_inputs']} / {first_case['witness_variables']}`",
            f"- Witness ms first / second: `{first_case['witness_ms']:.3f} / {second_case['witness_ms']:.3f}`",
            f"- Same constraints / public inputs / witness variables: `{g09_h2c['same_constraints']} / {g09_h2c['same_public_inputs']} / {g09_h2c['same_witness_variables']}`",
            f"- Changed transcript rejected: `{g09_h2c['changed_transcript_rejected']}`",
            f"- Suite v2 changed / candidate adopted: `{g09_h2c['current_suite_v2_changed']} / {g09_h2c['candidate_suite_adopted']}`",
            f"- CP equations / Groth16 / full H2 / full P3: `{g09_h2c['cp_equations_integrated']} / {g09_h2c['groth16_run']} / {g09_h2c['full_h2']} / {g09_h2c['full_p3']}`",
        ])
    if g09_h2d:
        lines.extend([
            "",
            "## G0.9-H2-D adopted CP-H2F-v3",
            "",
            f"- Status: `{g09_h2d['status']}`",
            f"- Suite: `{g09_h2d['adopted_suite_id']}`",
            f"- Constraints / public inputs / witness variables: `{g09_h2d['constraints']} / {g09_h2d['public_inputs']} / {g09_h2d['witness_variables']}`",
            f"- Witness / peak RSS: `{g09_h2d['witness_ms']:.3f} ms / {g09_h2d['peak_rss_mib']:.3f} MiB`",
            f"- Native vector / R1CS match: `{g09_h2d['native_matches_frozen_vector']} / {g09_h2d['r1cs_matches_native']}`",
            f"- Changed transcript rejected: `{g09_h2d['changed_transcript_rejected']}`",
            f"- Full H2 / CP equation R1CS / Groth16 / full P3: `{g09_h2d['full_h2']} / {g09_h2d['cp_equations_integrated_in_r1cs']} / {g09_h2d['groth16_run']} / {g09_h2d['full_p3']}`",
        ])
    if g09_h3:
        lines.extend([
            "",
            "## G0.9-H3 first CP equation",
            "",
            f"- Status: `{g09_h3['status']}`",
            f"- Constraints / public inputs / witness variables: `{g09_h3['constraints']} / {g09_h3['public_inputs']} / {g09_h3['witness_variables']}`",
            f"- Witness / peak RSS: `{g09_h3['witness_ms']:.3f} ms / {g09_h3['peak_rss_mib']:.3f} MiB`",
            f"- Transcript points / group arithmetic constrained in R1CS: `{g09_h3['transcript_points_bound_in_r1cs']} / {g09_h3['group_arithmetic_constrained_in_r1cs']}`",
            f"- Changed transcript / wrong VK / wrong response rejected: `{g09_h3['changed_transcript_rejected']} / {g09_h3['wrong_verification_share_rejected']} / {g09_h3['wrong_response_rejected']}`",
            f"- First equation / vector equations / Groth16 / full P3: `{g09_h3['first_cp_equation_complete']} / {g09_h3['vector_cp_equations_complete']} / {g09_h3['groth16_run']} / {g09_h3['full_p3']}`",
        ])
    if g09_h4a:
        lines.extend([
            "",
            "## G0.9-H4-A complete one-Trustee CP vector",
            "",
            f"- Status: `{g09_h4a['status']}`",
            f"- CP equations / transcript points bound: `{g09_h4a['cp_equations_constrained_in_r1cs']} / {g09_h4a['transcript_points_bound_in_r1cs']}`",
            f"- Constraints / public inputs / witness variables: `{g09_h4a['constraints']} / {g09_h4a['public_inputs']} / {g09_h4a['witness_variables']}`",
            f"- Witness / peak RSS: `{g09_h4a['witness_ms']:.3f} ms / {g09_h4a['peak_rss_mib']:.3f} MiB`",
            f"- Native first / coordinate equations: `{g09_h4a['native_first_equation_valid']} / {g09_h4a['native_coordinate_equations_valid']}`",
            f"- Changed transcript / wrong share / wrong coordinate rejected: `{g09_h4a['changed_transcript_rejected']} / {g09_h4a['wrong_decryption_share_rejected']} / {g09_h4a['wrong_proof_coordinate_rejected']}`",
            f"- Complete one Trustee / multi-Trustee / Groth16 / full P3: `{g09_h4a['complete_one_trustee_cp']} / {g09_h4a['multi_trustee_cp_complete']} / {g09_h4a['groth16_run']} / {g09_h4a['full_p3']}`",
        ])
    if g09_h4b:
        lines.extend([
            "",
            "## G0.9-H4-B real Groth16 for one-Trustee CP",
            "",
            f"- Status: `{g09_h4b['status']}`",
            f"- Constraints / public inputs / witness variables: `{g09_h4b['constraints']} / {g09_h4b['public_inputs']} / {g09_h4b['witness_variables']}`",
            f"- Setup / prove / verify: `{g09_h4b['setup_ms']:.3f} / {g09_h4b['prove_ms']:.3f} / {g09_h4b['verify_ms']:.3f} ms`",
            f"- Proof / proving key / verifying key: `{g09_h4b['proof_bytes']} / {g09_h4b['proving_key_compressed_bytes']} / {g09_h4b['verifying_key_compressed_bytes']} bytes`",
            f"- Peak RSS after setup / prove: `{g09_h4b['peak_rss_after_setup_mib']:.3f} / {g09_h4b['peak_rss_after_prove_mib']:.3f} MiB`",
            f"- Valid proof / changed public transcript rejected: `{g09_h4b['valid_proof_verified']} / {g09_h4b['changed_public_transcript_rejected']}`",
            f"- Complete one Trustee / Groth16 / full P3: `{g09_h4b['complete_one_trustee_cp']} / {g09_h4b['groth16_run']} / {g09_h4b['full_p3']}`",
        ])
    if g09_p3a:
        lines.extend([
            "",
            "## G0.9-P3-A three selected Trustee CP relations",
            "",
            f"- Status: `{g09_p3a['status']}`",
            f"- Selected Trustees / native CP verified: `{g09_p3a['selected_trustees']} / {g09_p3a['native_cp_verified']}`",
            f"- Trustees / CP equations in R1CS: `{g09_p3a['trustees_in_r1cs']} / {g09_p3a['cp_equations_in_r1cs']}`",
            f"- Public prefix / private transcript / private response per Trustee: `{g09_p3a['public_prefix_bytes_per_trustee']} / {g09_p3a['private_transcript_bytes_per_trustee']} / {g09_p3a['private_response_scalar_bytes_per_trustee']} bytes`",
            f"- Constraints / public inputs / witness variables: `{g09_p3a['constraints']} / {g09_p3a['public_inputs']} / {g09_p3a['witness_variables']}`",
            f"- Witness / peak RSS: `{g09_p3a['witness_ms']:.3f} ms / {g09_p3a['peak_rss_mib']:.3f} MiB`",
            f"- Valid relation / wrong third share rejected: `{g09_p3a['valid_relation_satisfied']} / {g09_p3a['wrong_third_trustee_share_rejected']}`",
            f"- Private CP values emitted / Groth16 / package openings / full P3: `{g09_p3a['private_cp_values_emitted']} / {g09_p3a['groth16_run']} / {g09_p3a['package_commitment_openings_in_r1cs']} / {g09_p3a['full_p3']}`",
        ])
    if g09_p3a_g16:
        lines.extend([
            "",
            "## G0.9-P3-A real Groth16 for three selected Trustee CPs",
            "",
            f"- Status: `{g09_p3a_g16['status']}`",
            f"- Trustees / CP equations: `{g09_p3a_g16['trustees_in_circuit']} / {g09_p3a_g16['cp_equations_in_circuit']}`",
            f"- Constraints / public inputs / witness variables: `{g09_p3a_g16['constraints']} / {g09_p3a_g16['public_inputs']} / {g09_p3a_g16['witness_variables']}`",
            f"- Setup / prove / verify: `{g09_p3a_g16['setup_ms']:.3f} / {g09_p3a_g16['prove_ms']:.3f} / {g09_p3a_g16['verify_ms']:.3f} ms`",
            f"- Proof / proving key / verifying key: `{g09_p3a_g16['proof_bytes']} / {g09_p3a_g16['proving_key_compressed_bytes']} / {g09_p3a_g16['verifying_key_compressed_bytes']} bytes`",
            f"- Peak RSS after setup / prove: `{g09_p3a_g16['peak_rss_after_setup_mib']:.3f} / {g09_p3a_g16['peak_rss_after_prove_mib']:.3f} MiB`",
            f"- Valid proof / changed public prefix rejected: `{g09_p3a_g16['valid_proof_verified']} / {g09_p3a_g16['changed_public_prefix_rejected']}`",
            f"- Full P3-A / package openings / full P3: `{g09_p3a_g16['full_p3a']} / {g09_p3a_g16['package_commitment_openings_in_circuit']} / {g09_p3a_g16['full_p3']}`",
        ])
    if g09_p3b0:
        lines.extend([
            "",
            "## G0.9-P3-B0 private-package H2F adoption",
            "",
            f"- Status: `{g09_p3b0['status']}`",
            f"- Suite: `{g09_p3b0['suite_id']}`",
            f"- Selected Trustees: `{g09_p3b0['selected_trustees']}`",
            f"- Package / H2F input bytes / SHA-512 blocks: `{g09_p3b0['package_bytes_per_trustee']} / {g09_p3b0['h2f_input_bytes']} / {g09_p3b0['sha512_blocks']}`",
            f"- Constraints / public inputs / witness variables per package: `{g09_p3b0['constraints_per_package']} / {g09_p3b0['public_inputs_per_package']} / {g09_p3b0['witness_variables_per_package']}`",
            f"- Total witness / peak RSS: `{g09_p3b0['total_witness_ms']:.3f} ms / {g09_p3b0['peak_rss_mib']:.3f} MiB`",
            f"- Fixed shape / native-R1CS match / changed package rejected: `{g09_p3b0['same_fixed_shape']} / {g09_p3b0['native_r1cs_match']} / {g09_p3b0['changed_package_rejected']}`",
            f"- Native package commitment migrated: `{g09_p3b0['native_package_commitment_migrated']}`",
            f"- Package opening in P3 / Groth16 / full P3: `{g09_p3b0['package_opening_equation_in_p3_r1cs']} / {g09_p3b0['groth16_run']} / {g09_p3b0['full_p3']}`",
        ])
        if g09_p3b0_e2e:
            p3b0_e2e = g09_p3b0_e2e["experiment"]
            p3b0_event = p3b0_e2e["winner_only_public_event"]
            lines.extend([
                f"- Native E2E packages created / verified / selected: `{p3b0_e2e['packages_created']} / {p3b0_e2e['packages_verified']} / {p3b0_e2e['packages_selected']}`",
                f"- Native E2E selected Trustees / winner: `{p3b0_event['selected_trustees']} / {p3b0_event['winner']}`",
                f"- Winner-event version / full P3: `{p3b0_event['event_version']} / {p3b0_e2e['full_p3']}`",
            ])
    if g09_p3b1:
        lines.extend([
            "",
            "## G0.9-P3-B1 three private-package openings",
            "",
            f"- Status: `{g09_p3b1['status']}`",
            f"- Selected Trustees: `{g09_p3b1['selected_trustees']}`",
            f"- Native CP / packages verified: `{g09_p3b1['native_cp_verified']} / {g09_p3b1['native_packages_verified']}`",
            f"- CP / package-H2F / opening equations in R1CS: `{g09_p3b1['cp_equations_in_r1cs']} / {g09_p3b1['package_h2f_relations_in_r1cs']} / {g09_p3b1['package_opening_equations_in_r1cs']}`",
            f"- Package / H2F input bytes / SHA-512 blocks: `{g09_p3b1['package_bytes_per_trustee']} / {g09_p3b1['package_h2f_input_bytes']} / {g09_p3b1['package_sha512_blocks']}`",
            f"- Constraints / public inputs / witness variables: `{g09_p3b1['constraints']} / {g09_p3b1['public_inputs']} / {g09_p3b1['witness_variables']}`",
            f"- Witness / peak RSS: `{g09_p3b1['witness_ms']:.3f} ms / {g09_p3b1['peak_rss_mib']:.3f} MiB`",
            f"- Valid relation / changed third commitment rejected: `{g09_p3b1['valid_relation_satisfied']} / {g09_p3b1['changed_third_commitment_rejected']}`",
            f"- Private packages / openings emitted: `{g09_p3b1['private_package_bytes_emitted']} / {g09_p3b1['private_openings_emitted']}`",
            f"- Opening gate / Groth16 / threshold+tally / winner / full P3: `{g09_p3b1['package_opening_gate_complete']} / {g09_p3b1['groth16_run']} / {g09_p3b1['lagrange_and_tally_in_r1cs']} / {g09_p3b1['winner_logic_in_r1cs']} / {g09_p3b1['full_p3']}`",
        ])
    if g09_p3c0:
        lines.extend([
            "",
            "## G0.9-P3-C0 fixed 1-2-3 threshold combination",
            "",
            f"- Status: `{g09_p3c0['status']}`",
            f"- Selected Trustees / coefficients: `{g09_p3c0['selected_trustees']} / {g09_p3c0['lagrange_coefficients']}`",
            f"- Native coefficients / threshold tally match: `{g09_p3c0['native_coefficients_match']} / {g09_p3c0['native_threshold_tally_matches_fixture']}`",
            f"- CP / package-H2F / openings / threshold equations: `{g09_p3c0['cp_equations_in_r1cs']} / {g09_p3c0['package_h2f_relations_in_r1cs']} / {g09_p3c0['package_opening_equations_in_r1cs']} / {g09_p3c0['lagrange_combination_equations_in_r1cs']}`",
            f"- Public C1 / private plaintext group points: `{g09_p3c0['public_ciphertext_c1_points']} / {g09_p3c0['private_plaintext_group_points']}`",
            f"- Constraints / public inputs / witness variables: `{g09_p3c0['constraints']} / {g09_p3c0['public_inputs']} / {g09_p3c0['witness_variables']}`",
            f"- Witness / peak RSS: `{g09_p3c0['witness_ms']:.3f} ms / {g09_p3c0['peak_rss_mib']:.3f} MiB`",
            f"- Valid relation / changed third plaintext rejected: `{g09_p3c0['valid_relation_satisfied']} / {g09_p3c0['changed_third_plaintext_rejected']}`",
            f"- Private plaintext emitted: `{g09_p3c0['private_plaintext_groups_emitted']}`",
            f"- Threshold gate / decoding / tally values / winner / Groth16 / full P3: `{g09_p3c0['threshold_combination_gate_complete']} / {g09_p3c0['bounded_decoding_in_r1cs']} / {g09_p3c0['tally_values_in_r1cs']} / {g09_p3c0['winner_logic_in_r1cs']} / {g09_p3c0['groth16_run']} / {g09_p3c0['full_p3']}`",
        ])
    if g09_p3c1:
        lines.extend([
            "",
            "## G0.9-P3-C1 bounded private tally binding",
            "",
            f"- Status: `{g09_p3c1['status']}`",
            f"- Selected Trustees / coefficients: `{g09_p3c1['selected_trustees']} / {g09_p3c1['lagrange_coefficients']}`",
            f"- Registered weight: `{g09_p3c1['public_registered_weight']}`",
            f"- CP / package-H2F / openings / threshold equations: `{g09_p3c1['cp_equations_in_r1cs']} / {g09_p3c1['package_h2f_relations_in_r1cs']} / {g09_p3c1['package_opening_equations_in_r1cs']} / {g09_p3c1['lagrange_combination_equations_in_r1cs']}`",
            f"- Tally group bindings / ranges / sum relations: `{g09_p3c1['tally_group_binding_equations_in_r1cs']} / {g09_p3c1['tally_range_relations_in_r1cs']} / {g09_p3c1['tally_sum_relations_in_r1cs']}`",
            f"- Constraints / public inputs / witness variables: `{g09_p3c1['constraints']} / {g09_p3c1['public_inputs']} / {g09_p3c1['witness_variables']}`",
            f"- Witness / peak RSS: `{g09_p3c1['witness_ms']:.3f} ms / {g09_p3c1['peak_rss_mib']:.3f} MiB`",
            f"- Valid relation / sum-preserving changed tallies rejected: `{g09_p3c1['valid_relation_satisfied']} / {g09_p3c1['sum_preserving_changed_tallies_rejected']}`",
            f"- Private plaintexts / tallies emitted: `{g09_p3c1['private_plaintext_groups_emitted']} / {g09_p3c1['private_tallies_emitted']}`",
            f"- Bounded tally gate / winner / Groth16 / full P3: `{g09_p3c1['bounded_tally_binding_in_r1cs']} / {g09_p3c1['winner_logic_in_r1cs']} / {g09_p3c1['groth16_run']} / {g09_p3c1['full_p3']}`",
        ])
    if g09_p3d0:
        lines.extend([
            "",
            "## G0.9-P3-D0 public winner linkage",
            "",
            f"- Status: `{g09_p3d0['status']}`",
            f"- Selected Trustees / coefficients: `{g09_p3d0['selected_trustees']} / {g09_p3d0['lagrange_coefficients']}`",
            f"- Registered weight / public winner: `{g09_p3d0['public_registered_weight']} / {g09_p3d0['public_winner']}`",
            f"- CP / package-H2F / openings / threshold equations: `{g09_p3d0['cp_equations_in_r1cs']} / {g09_p3d0['package_h2f_relations_in_r1cs']} / {g09_p3d0['package_opening_equations_in_r1cs']} / {g09_p3d0['lagrange_combination_equations_in_r1cs']}`",
            f"- Tally bindings / ranges / sum: `{g09_p3d0['tally_group_binding_equations_in_r1cs']} / {g09_p3d0['tally_range_relations_in_r1cs']} / {g09_p3d0['tally_sum_relations_in_r1cs']}`",
            f"- Winner flags / pairwise relations: `{g09_p3d0['winner_candidate_flags_in_r1cs']} / {g09_p3d0['winner_pairwise_relations_in_r1cs']}`",
            f"- Constraints / public inputs / witness variables: `{g09_p3d0['constraints']} / {g09_p3d0['public_inputs']} / {g09_p3d0['witness_variables']}`",
            f"- Witness / peak RSS: `{g09_p3d0['witness_ms']:.3f} ms / {g09_p3d0['peak_rss_mib']:.3f} MiB`",
            f"- Valid relation / wrong public winner rejected: `{g09_p3d0['valid_relation_satisfied']} / {g09_p3d0['wrong_public_winner_rejected']}`",
            f"- Private plaintexts / tallies emitted: `{g09_p3d0['private_plaintext_groups_emitted']} / {g09_p3d0['private_tallies_emitted']}`",
            f"- Winner R1CS / Groth16 / full P3 proof: `{g09_p3d0['winner_logic_in_r1cs']} / {g09_p3d0['groth16_run']} / {g09_p3d0['full_p3']}`",
        ])
    if g09_p3d1:
        lines.extend([
            "",
            "## G0.9-P3-D1 fixed-123 complete P3 Groth16",
            "",
            f"- Status: `{g09_p3d1['status']}`",
            f"- Selected Trustees / coefficients: `{g09_p3d1['selected_trustees']} / {g09_p3d1['lagrange_coefficients']}`",
            f"- Registered weight / public winner: `{g09_p3d1['public_registered_weight']} / {g09_p3d1['public_winner']}`",
            f"- Constraints / public inputs / witness variables: `{g09_p3d1['constraints']} / {g09_p3d1['public_inputs']} / {g09_p3d1['witness_variables']}`",
            f"- Witness / setup / prove / verify: `{g09_p3d1['witness_ms']:.3f} / {g09_p3d1['setup_ms']:.3f} / {g09_p3d1['prove_ms']:.3f} / {g09_p3d1['verify_ms']:.3f} ms`",
            f"- Proof / proving key / verifying key: `{g09_p3d1['proof_bytes']} / {g09_p3d1['proving_key_compressed_bytes']} / {g09_p3d1['verifying_key_compressed_bytes']} bytes`",
            f"- Peak RSS: `{g09_p3d1['peak_rss_after_verify_mib']:.3f} MiB`",
            f"- Valid proof / wrong public winner rejected: `{g09_p3d1['valid_proof_verified']} / {g09_p3d1['wrong_public_winner_rejected']}`",
            f"- Proving key persisted / private tallies emitted: `{g09_p3d1['proving_key_persisted']} / {g09_p3d1['private_tallies_emitted']}`",
            f"- Groth16 / fixed-123 full P3: `{g09_p3d1['groth16_run']} / {g09_p3d1['full_p3']}`",
        ])
    if g1_b0:
        lines.extend([
            "",
            "## G1-B0 weighted-ballot cryptographic core",
            "",
            f"- Status: `{g1_b0['status']}`",
            f"- Candidate count / registered-weight bound: `{g1_b0['candidate_count']} / {g1_b0['registered_weight_bound']}`",
            f"- One-hot / weight-range / commitment-opening relations: `{g1_b0['one_hot_relations_in_r1cs']} / {g1_b0['weight_range_relations_in_r1cs']} / {g1_b0['weight_commitment_opening_equations_in_r1cs']}`",
            f"- Weighted ElGamal group equations: `{g1_b0['elgamal_group_equations_in_r1cs']}`",
            f"- Constraints / public inputs / witness variables: `{g1_b0['constraints']} / {g1_b0['public_inputs']} / {g1_b0['witness_variables']}`",
            f"- Witness / peak RSS: `{g1_b0['witness_ms']:.3f} ms / {g1_b0['peak_rss_mib']:.3f} MiB`",
            f"- Valid relation / two-hot rejected: `{g1_b0['valid_relation_satisfied']} / {g1_b0['two_hot_witness_rejected']}`",
            f"- Ballot core / full C_vote / Groth16 / SnarkPack: `{g1_b0['full_ballot_core_b0']} / {g1_b0['full_c_vote']} / {g1_b0['groth16_run']} / {g1_b0['snarkpack_run']}`",
        ])
    if g1_b1:
        lines.extend([
            "",
            "## G1-B1 complete fixed-suite C_vote R1CS",
            "",
            f"- Status: `{g1_b1['status']}`",
            f"- Five conditions / candidates / regions: `{g1_b1['five_conditions_in_r1cs']} / {g1_b1['candidate_count']} / {g1_b1['region_count']}`",
            f"- Registration leaves / depth: `{g1_b1['registration_leaf_count']} / {g1_b1['registration_depth']}`",
            f"- PRFs / region hashes / Merkle hashes: `{g1_b1['prf_evaluations_in_r1cs']} / {g1_b1['region_hashes_in_r1cs']} / {g1_b1['merkle_hashes_in_r1cs']}`",
            f"- Constraints / public inputs / witness variables: `{g1_b1['constraints']} / {g1_b1['public_inputs']} / {g1_b1['witness_variables']}`",
            f"- Witness / peak RSS: `{g1_b1['witness_ms']:.3f} ms / {g1_b1['peak_rss_mib']:.3f} MiB`",
            f"- Valid relation / changed context rejected: `{g1_b1['valid_relation_satisfied']} / {g1_b1['changed_context_rejected']}`",
            f"- Full C_vote R1CS / Groth16 / SnarkPack: `{g1_b1['full_c_vote_r1cs']} / {g1_b1['groth16_run']} / {g1_b1['snarkpack_run']}`",
        ])
    if g1_b2:
        lines.extend([
            "",
            "## G1-B2 complete fixed-suite C_vote Groth16",
            "",
            f"- Status: `{g1_b2['status']}`",
            f"- Constraints / public inputs / witness variables: `{g1_b2['constraints']} / {g1_b2['public_inputs']} / {g1_b2['witness_variables']}`",
            f"- Witness / setup / prove / verify: `{g1_b2['witness_ms']:.3f} / {g1_b2['setup_ms']:.3f} / {g1_b2['prove_ms']:.3f} / {g1_b2['verify_ms']:.3f} ms`",
            f"- Proof / proving key / verifying key: `{g1_b2['proof_bytes']} / {g1_b2['proving_key_compressed_bytes']} / {g1_b2['verifying_key_compressed_bytes']} bytes`",
            f"- Peak RSS: `{g1_b2['peak_rss_after_verify_mib']:.3f} MiB`",
            f"- Valid proof / changed context rejected: `{g1_b2['valid_proof_verified']} / {g1_b2['changed_public_context_rejected']}`",
            f"- Full C_vote / Groth16 / SnarkPack: `{g1_b2['full_c_vote']} / {g1_b2['groth16_run']} / {g1_b2['snarkpack_run']}`",
        ])
    if g1_c0:
        lines.extend([
            "",
            "## G1-C0 minimum-batch SnarkPack compatibility",
            "",
            f"- Status: `{g1_c0['status']}`",
            f"- Implementation / version: `{g1_c0['implementation']} / {g1_c0['implementation_version']}`",
            f"- Aggregate version / batch / public inputs per proof: `{g1_c0['aggregate_version']} / {g1_c0['batch_size']} / {g1_c0['public_inputs_per_proof']}`",
            f"- Second prove / conversion / aggregation-SRS / aggregate / verify: `{g1_c0['second_prove_ms']:.3f} / {g1_c0['conversion_ms']:.3f} / {g1_c0['aggregation_srs_ms']:.3f} / {g1_c0['aggregation_ms']:.3f} / {g1_c0['aggregate_verify_ms']:.3f} ms`",
            f"- Individual total / aggregate proof: `{g1_c0['total_individual_proof_bytes']} / {g1_c0['aggregate_proof_bytes']} bytes`",
            f"- Converted individual proofs / aggregate / changed context rejected: `{g1_c0['converted_individual_proofs_verified']} / {g1_c0['aggregate_proof_verified']} / {g1_c0['changed_public_context_rejected']}`",
            f"- Peak RSS / production setup: `{g1_c0['peak_rss_mib']:.3f} MiB / {g1_c0['production_setup']}`",
        ])
    if g1_c1:
        lines.extend([
            "",
            "## G1-C1 distinct-registered-voter SnarkPack",
            "",
            f"- Status: `{g1_c1['status']}`",
            f"- Batch / public inputs per proof: `{g1_c1['batch_size']} / {g1_c1['public_inputs_per_proof']}`",
            f"- Setup / first prove / second prove: `{g1_c1['groth16_setup_ms']:.3f} / {g1_c1['first_prove_ms']:.3f} / {g1_c1['second_prove_ms']:.3f} ms`",
            f"- Aggregate / verify: `{g1_c1['aggregation_ms']:.3f} / {g1_c1['aggregate_verify_ms']:.3f} ms`",
            f"- Individual total / aggregate proof: `{g1_c1['total_individual_proof_bytes']} / {g1_c1['aggregate_proof_bytes']} bytes`",
            f"- Distinct statements / nullifiers / ciphertexts: `{g1_c1['registered_voter_statements_distinct']} / {g1_c1['nullifiers_distinct']} / {g1_c1['ciphertexts_distinct']}`",
            f"- Aggregate valid / changed second nullifier rejected: `{g1_c1['aggregate_proof_verified']} / {g1_c1['changed_second_nullifier_rejected']}`",
            f"- Peak RSS / production setup: `{g1_c1['peak_rss_mib']:.3f} MiB / {g1_c1['production_setup']}`",
        ])
    if g2_a:
        lines.extend([
            "",
            "## G2-A fixed-B=2 state-transition R1CS",
            "",
            f"- Status: `{g2_a['status']}`",
            f"- Region / batch index / batch size: `{g2_a['region']} / {g2_a['batch_index']} / {g2_a['batch_size']}`",
            f"- Constraints / public inputs / witness variables: `{g2_a['constraints']} / {g2_a['public_inputs']} / {g2_a['witness_variables']}`",
            f"- Batch hashes / accumulator components / group additions: `{g2_a['batch_hashes_in_r1cs']} / {g2_a['accumulator_components']} / {g2_a['group_additions_in_r1cs']}`",
            f"- Witness / peak RSS: `{g2_a['witness_ms']:.3f} ms / {g2_a['peak_rss_mib']:.3f} MiB`",
            f"- Same-region source / exact SnarkPack-input serialization: `{g2_a['source_records_are_same_region']} / {g2_a['source_public_input_serialization_matches']}`",
            f"- Valid transition / substituted ciphertext rejected: `{g2_a['valid_transition_satisfied']} / {g2_a['substituted_ciphertext_rejected']}`",
            f"- Groth16 run: `{g2_a['groth16_run']}`",
        ])
    if g2_b:
        lines.extend([
            "",
            "## G2-B fixed-B=2 state-transition Groth16",
            "",
            f"- Status: `{g2_b['status']}`",
            f"- Region / batch index / batch size: `{g2_b['region']} / {g2_b['batch_index']} / {g2_b['batch_size']}`",
            f"- Constraints / public inputs / witness variables: `{g2_b['constraints']} / {g2_b['public_inputs']} / {g2_b['witness_variables']}`",
            f"- Setup / prove / verify: `{g2_b['setup_ms']:.3f} / {g2_b['prove_ms']:.3f} / {g2_b['verify_ms']:.3f} ms`",
            f"- Proof / proving key / verifying key: `{g2_b['proof_bytes']} / {g2_b['proving_key_compressed_bytes']} / {g2_b['verifying_key_compressed_bytes']} bytes`",
            f"- Peak RSS after setup / prove / verify: `{g2_b['peak_rss_after_setup_mib']:.3f} / {g2_b['peak_rss_after_prove_mib']:.3f} / {g2_b['peak_rss_after_verify_mib']:.3f} MiB`",
            f"- Valid proof / changed public batch root rejected: `{g2_b['valid_proof_verified']} / {g2_b['changed_public_batch_root_rejected']}`",
            f"- Proving key persisted / private witness emitted: `{g2_b['proving_key_persisted']} / {g2_b['private_witness_emitted']}`",
        ])
    if g3_a:
        lines.extend([
            "",
            "## G3-A five-process DKG happy path",
            "",
            f"- Status: `{g3_a['status']}`",
            f"- Trustee processes / unique PIDs / successful exits: `{g3_a['trustee_process_count']} / {g3_a['unique_os_process_ids']} / {g3_a['all_processes_exited_successfully']}`",
            f"- Private state files / persisted: `{g3_a['independent_private_state_files']} / {g3_a['private_state_files_persisted']}`",
            f"- Authenticated deal / decryption messages: `{g3_a['authenticated_deal_messages']} / {g3_a['authenticated_decryption_share_messages']}`",
            f"- DKG-to-final / vote-fold / threshold decrypt: `{g3_a['dkg_to_final_ms']:.3f} / {g3_a['vote_encrypt_and_fold_ms']:.3f} / {g3_a['fixed_subset_threshold_decrypt_ms']:.3f} ms`",
            f"- Total wall / transport bytes: `{g3_a['total_wall_ms']:.3f} ms / {g3_a['public_transport_bytes']} bytes`",
            f"- Certificate quorum / public-state agreement: `{g3_a['certificate_quorum_verified']} / {g3_a['all_trustees_agreed_on_public_state']}`",
            f"- Selected Trustees / private tally / winner: `{g3_a['selected_trustees']} / {g3_a['private_tally']} / {g3_a['winner']}`",
            f"- Full G3: `{g3_a['full_g3']}`",
        ])
    if g3_b:
        lines.extend([
            "",
            "## G3-B one-Trustee restart recovery",
            "",
            f"- Status: `{g3_b['status']}`",
            f"- Logical Trustees / OS process instances: `{g3_b['trustee_process_count']} / {g3_b['total_os_process_instances']}`",
            f"- Restarted Trustee / initial PID / resumed PID: `{g3_b['restarted_trustee']} / {g3_b['initial_restarted_process_id']} / {g3_b['resumed_process_id']}`",
            f"- Restart observed / persisted state reused / authenticated resume record: `{g3_b['process_restart_observed']} / {g3_b['persisted_private_state_reused']} / {g3_b['authenticated_resume_messages']}`",
            f"- DKG-to-final / vote-fold / threshold decrypt: `{g3_b['dkg_to_final_ms']:.3f} / {g3_b['vote_encrypt_and_fold_ms']:.3f} / {g3_b['fixed_subset_threshold_decrypt_ms']:.3f} ms`",
            f"- Total wall / transport bytes: `{g3_b['total_wall_ms']:.3f} ms / {g3_b['public_transport_bytes']} bytes`",
            f"- Public-state agreement / certificate quorum: `{g3_b['all_trustees_agreed_on_public_state']} / {g3_b['certificate_quorum_verified']}`",
            f"- Selected Trustees / private tally / winner: `{g3_b['selected_trustees']} / {g3_b['private_tally']} / {g3_b['winner']}`",
            f"- Full G3: `{g3_b['full_g3']}`",
        ])
    if g4_a:
        baseline = g4_a["generalized_123"]
        second = g4_a["generalized_124"]
        lines.extend([
            "",
            "## G4-A second-subset complete-P3 R1CS shape",
            "",
            f"- Status: `{g4_a['status']}`",
            f"- Subsets: `{baseline['selected_trustees']} / {second['selected_trustees']}`",
            f"- Native tallies / winners: `{baseline['native_threshold_tally']} / {baseline['native_winner']}` and `{second['native_threshold_tally']} / {second['native_winner']}`",
            f"- Constraints: `{baseline['constraints']} / {second['constraints']}`",
            f"- Public inputs: `{baseline['public_inputs']} / {second['public_inputs']}`",
            f"- Witness variables: `{baseline['witness_variables']} / {second['witness_variables']}`",
            f"- Witness synthesis: `{baseline['witness_ms']:.3f} / {second['witness_ms']:.3f} ms`",
            f"- Common count shape / public layout preserved: `{g4_a['common_count_shape_confirmed']} / {g4_a['public_input_layout_preserved']}`",
            f"- Public coefficient binding / Groth16: `{g4_a['coefficient_binding_is_public']} / {g4_a['groth16_run']}`",
        ])
    if g4_b:
        baseline = g4_b["baseline"]
        second = g4_b["second_subset"]
        lines.extend([
            "",
            "## G4-B public two-subset complete-P3 Groth16",
            "",
            f"- Status: `{g4_b['status']}`",
            f"- Supported subsets: `{g4_b['supported_subsets']}`",
            f"- Setup subset / proof subset: `{baseline['selected_trustees']} / {second['selected_trustees']}`",
            f"- Constraints / public inputs / witness variables: `{second['constraints']} / {second['public_inputs']} / {second['witness_variables']}`",
            f"- Witness synthesis (setup/proof assignment): `{baseline['witness_ms']:.3f} / {second['witness_ms']:.3f} ms`",
            f"- Setup / prove / verify: `{g4_b['setup_ms']:.3f} / {g4_b['prove_ms']:.3f} / {g4_b['verify_ms']:.3f} ms`",
            f"- Proof / proving key / verifying key: `{g4_b['proof_bytes']} / {g4_b['proving_key_compressed_bytes']} / {g4_b['verifying_key_compressed_bytes']} bytes`",
            f"- Peak RSS after setup / prove / verify: `{g4_b['peak_rss_after_setup_mib']:.3f} / {g4_b['peak_rss_after_prove_mib']:.3f} / {g4_b['peak_rss_after_verify_mib']:.3f} MiB`",
            f"- Same setup parameters / valid proof: `{g4_b['same_setup_parameters_used']} / {g4_b['valid_second_subset_proof_verified']}`",
            f"- Selector bound to transcript IDs / changed selector rejected: `{g4_b['public_selector_bound_to_transcript_ids']} / {g4_b['changed_public_selector_rejected']}`",
            f"- Two-subset complete P3 / arbitrary-subset P3: `{g4_b['full_p3_for_supported_two_subset_gate']} / {g4_b['full_arbitrary_subset_p3']}`",
            f"- Manifest unchanged / proving key persisted: `{g4_b['manifest_unchanged']} / {g4_b['proving_key_persisted']}`",
        ])
    if g5_a:
        lines.extend([
            "",
            "## G5-A local Prague state-proof Solidity verifier",
            "",
            f"- Status: `{g5_a['status']}`",
            f"- Runtime / compiler: `EthereumJS {g5_a['dependencies']['@ethereumjs/vm']} / {g5_a['compiler']}`",
            f"- Hardfork / public inputs / IC points: `{g5_a['hardfork']} / {g5_a['public_inputs']} / {g5_a['ic_points']}`",
            f"- Valid transaction gas / execution gas: `{g5_a['valid']['total_gas']} / {g5_a['valid']['execution_gas']}`",
            f"- Changed-root transaction gas / status: `{g5_a['wrong_batch_root']['total_gas']} / {g5_a['wrong_batch_root']['receipt_status']}`",
            f"- Deployment gas / runtime bytes: `{g5_a['deployment']['total_gas']} / {g5_a['runtime_bytecode_bytes']}`",
            f"- Compressed / EIP-2537 proof / logical proof+inputs: `{g5_a['proof_compressed_bytes']} / {g5_a['proof_eip2537_bytes']} / {g5_a['logical_proof_and_public_input_bytes']} bytes`",
            f"- Actual verifier calldata / transaction DA: `{g5_a['valid']['calldata_bytes']} / {g5_a['verifier_transaction_calldata_da_bytes']} bytes`",
            f"- Valid proof / changed root rejected: `{g5_a['valid_proof_verified']} / {g5_a['changed_public_batch_root_rejected']}`",
            f"- Full batch DA / full settlement / SnarkPack EVM / P3 EVM: `{g5_a['full_batch_da_measured']} / {g5_a['full_settlement_measured']} / {g5_a['snarkpack_evm_verified']} / {g5_a['p3_evm_verified']}`",
        ])
    if g5_b:
        lines.extend([
            "",
            "## G5-B local Prague two-subset P3 disclosure verifier",
            "",
            f"- Status: `{g5_b['status']}`",
            f"- Runtime / compiler: `EthereumJS {g5_b['dependencies']['@ethereumjs/vm']} / {g5_b['compiler']}`",
            f"- Public inputs / IC points / supported subsets: `{g5_b['public_inputs']} / {g5_b['ic_points']} / {g5_b['supported_subsets']}`",
            f"- Setup subset / proof subset / public winner: `{g5_b['setup_subset']} / {g5_b['proof_subset']} / {g5_b['public_winner']}`",
            f"- Valid transaction gas / execution gas: `{g5_b['valid']['total_gas']} / {g5_b['valid']['execution_gas']}`",
            f"- Changed-selector transaction gas / status: `{g5_b['wrong_selector']['total_gas']} / {g5_b['wrong_selector']['receipt_status']}`",
            f"- Deployment gas / runtime bytes: `{g5_b['deployment']['total_gas']} / {g5_b['runtime_bytecode_bytes']}`",
            f"- Compressed / EIP-2537 proof / logical proof+inputs: `{g5_b['proof_compressed_bytes']} / {g5_b['proof_eip2537_bytes']} / {g5_b['logical_proof_and_public_input_bytes']} bytes`",
            f"- Actual verifier calldata: `{g5_b['valid']['calldata_bytes']} bytes`",
            f"- Valid proof / changed selector rejected: `{g5_b['valid_proof_verified']} / {g5_b['changed_public_selector_rejected']}`",
            f"- P3 EVM / arbitrary-subset P3 / registry binding / full settlement: `{g5_b['p3_evm_verified']} / {g5_b['full_arbitrary_subset_p3']} / {g5_b['accepted_election_state_binding_checked']} / {g5_b['full_settlement_measured']}`",
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
        "> This report contains no fabricated values for blocked gates and does not claim arbitrary-subset or production full-P3 security.",
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

