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
    g5_c_path = raw.parent / "g5_c_raw" / "snarkpack_route.json"
    g5_c2_path = raw.parent / "g5_c2_raw" / "transcript_conformance.json"
    g5_c3_path = raw.parent / "g5_c3_raw" / "fp12_product_evm.json"
    g5_c3b_path = raw.parent / "g5_c3_raw" / "tipp_endpoint_route.json"
    g5_c3m_path = raw.parent / "g5_c3_raw" / "miller_loop_evm.json"
    g5_c3mem_path = raw.parent / "g5_c3_raw" / "miller_memory_evm.json"
    g5_c3s_path = raw.parent / "g5_c3_raw" / "miller_sparse_evm.json"
    g5_c3profile_path = raw.parent / "g5_c3_raw" / "field_cost_profile.json"
    g5_c3low_path = raw.parent / "g5_c3_raw" / "field_lowlevel_profile.json"
    g5_c3ml_path = raw.parent / "g5_c3_raw" / "miller_lowlevel_evm.json"
    g5_c3fp2_path = raw.parent / "g5_c3_raw" / "fp2_lowlevel_profile.json"
    g5_c3fp2prefix_path = raw.parent / "g5_c3_raw" / "miller_fp2_prefix_evm.json"
    g5_c3add_path = raw.parent / "g5_c3_raw" / "fp2_add_cost_profile.json"
    g5_c3addlow_path = raw.parent / "g5_c3_raw" / "fp2_add_lowlevel_profile.json"
    g5_c3addprefix_path = raw.parent / "g5_c3_raw" / "miller_add_prefix_evm.json"
    g5_c3sub_path = raw.parent / "g5_c3_raw" / "fp2_sub_cost_profile.json"
    g5_c3sublow_path = raw.parent / "g5_c3_raw" / "fp2_sub_lowlevel_profile.json"
    g5_c3subprefix_path = raw.parent / "g5_c3_raw" / "miller_sub_prefix_evm.json"
    g5_c3subfull_path = raw.parent / "g5_c3_raw" / "miller_sub_full_evm.json"
    g5_c3budget_path = raw.parent / "g5_c3_raw" / "miller_budget_review.json"
    g5_c3attribution_path = raw.parent / "g5_c3_raw" / "miller_cost_attribution.json"
    g5_c3route_path = raw.parent / "g5_c3_raw" / "miller_route_review.json"
    g5_c3quarter_path = raw.parent / "g5_c3_raw" / "fp_mul_quarter_profile.json"
    g5_c3quarterprefix_path = raw.parent / "g5_c3_raw" / "miller_quarter_prefix_evm.json"
    g5_c3fulldecision_path = raw.parent / "g5_c3_raw" / "miller_full_decision.json"
    g5_c3quarterfull_path = raw.parent / "g5_c3_raw" / "miller_quarter_full_evm.json"
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
    g5_c = json.loads(g5_c_path.read_text()) if g5_c_path.exists() else None
    g5_c2 = json.loads(g5_c2_path.read_text()) if g5_c2_path.exists() else None
    g5_c3 = json.loads(g5_c3_path.read_text()) if g5_c3_path.exists() else None
    g5_c3b = json.loads(g5_c3b_path.read_text()) if g5_c3b_path.exists() else None
    g5_c3m = json.loads(g5_c3m_path.read_text()) if g5_c3m_path.exists() else None
    g5_c3mem = json.loads(g5_c3mem_path.read_text()) if g5_c3mem_path.exists() else None
    g5_c3s = json.loads(g5_c3s_path.read_text()) if g5_c3s_path.exists() else None
    g5_c3ml = json.loads(g5_c3ml_path.read_text()) if g5_c3ml_path.exists() else None
    g5_c3subfull = json.loads(g5_c3subfull_path.read_text()) if g5_c3subfull_path.exists() else None
    g5_c3attribution = json.loads(g5_c3attribution_path.read_text()) if g5_c3attribution_path.exists() else None
    g5_c3quarterfull = json.loads(g5_c3quarterfull_path.read_text()) if g5_c3quarterfull_path.exists() else None
    # An actual B23 result supersedes B18; an abort must not pretend a full ran.
    g5_miller_latest = (g5_c3quarterfull if g5_c3quarterfull and g5_c3quarterfull.get("full_attempted") else None) or g5_c3attribution or g5_c3subfull or g5_c3ml or g5_c3s or g5_c3mem or g5_c3m
    g5_c3profile = json.loads(g5_c3profile_path.read_text()) if g5_c3profile_path.exists() else None
    g5_c3low = json.loads(g5_c3low_path.read_text()) if g5_c3low_path.exists() else None
    g5_c3fp2 = json.loads(g5_c3fp2_path.read_text()) if g5_c3fp2_path.exists() else None
    g5_c3fp2prefix = json.loads(g5_c3fp2prefix_path.read_text()) if g5_c3fp2prefix_path.exists() else None
    g5_c3add = json.loads(g5_c3add_path.read_text()) if g5_c3add_path.exists() else None
    g5_c3addlow = json.loads(g5_c3addlow_path.read_text()) if g5_c3addlow_path.exists() else None
    g5_c3addprefix = json.loads(g5_c3addprefix_path.read_text()) if g5_c3addprefix_path.exists() else None
    g5_c3sub = json.loads(g5_c3sub_path.read_text()) if g5_c3sub_path.exists() else None
    g5_c3sublow = json.loads(g5_c3sublow_path.read_text()) if g5_c3sublow_path.exists() else None
    g5_c3subprefix = json.loads(g5_c3subprefix_path.read_text()) if g5_c3subprefix_path.exists() else None
    g5_c3budget = json.loads(g5_c3budget_path.read_text()) if g5_c3budget_path.exists() else None
    g5_c3route = json.loads(g5_c3route_path.read_text()) if g5_c3route_path.exists() else None
    g5_c3quarter = json.loads(g5_c3quarter_path.read_text()) if g5_c3quarter_path.exists() else None
    g5_c3quarterprefix = json.loads(g5_c3quarterprefix_path.read_text()) if g5_c3quarterprefix_path.exists() else None
    g5_c3fulldecision = json.loads(g5_c3fulldecision_path.read_text()) if g5_c3fulldecision_path.exists() else None
    g5_prefix_latest = (g5_c3quarterprefix if g5_c3quarterprefix and g5_c3quarterprefix["prefix_conformance_and_accounting_passed"] else None) or g5_c3subprefix or g5_c3addprefix or g5_c3fp2prefix or g5_c3ml or g5_c3s or g5_c3mem or g5_c3m
    g5_target_latest = (g5_c3quarterfull if g5_c3quarterfull and g5_c3quarterfull.get("target_verifier_status") else None) or g5_c3quarterprefix or g5_miller_latest
    g5_next_work = g5_c3quarterfull or g5_c3fulldecision or g5_c3quarterprefix or g5_c3quarter or g5_c3route or g5_c3attribution or g5_c3budget or g5_c3subfull or g5_c3subprefix or g5_c3sublow or g5_c3sub or g5_c3addprefix or g5_c3addlow or g5_c3add or g5_c3fp2prefix or g5_c3fp2 or g5_c3ml or g5_c3low or g5_c3profile or g5_miller_latest
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
            "G5_C1_snarkpack_wire_and_runtime_prerequisites": (
                g5_c["status"] if g5_c else "NOT_RUN"
            ),
            "G5_C2_exact_internal_transcript_conformance": (
                g5_c2["status"] if g5_c2 else "NOT_RUN"
            ),
            "G5_C3_A_fp12_product_component": (
                g5_c3["status"] if g5_c3 else "NOT_RUN"
            ),
            "G5_C3_B_actual_tipp_endpoint_route": (
                g5_c3b["status"] if g5_c3b else "NOT_RUN"
            ),
            "G5_C3_B2_software_miller_loop_component": (
                g5_c3m["status"] if g5_c3m else "NOT_RUN"
            ),
            "G5_C3_B3_memory_optimized_miller_component": (
                g5_c3mem["status"] if g5_c3mem else "NOT_RUN"
            ),
            "G5_C3_B4_sparse_optimized_miller_component": (
                g5_c3s["status"] if g5_c3s else "NOT_RUN"
            ),
            "G5_C3_B5_isolated_field_cost_profile": (
                g5_c3profile["status"] if g5_c3profile else "NOT_RUN"
            ),
            "G5_C3_B6_low_level_field_comparison": (
                g5_c3low["status"] if g5_c3low else "NOT_RUN"
            ),
            "G5_C3_B7_low_level_miller_integration": (
                g5_c3ml["status"] if g5_c3ml else "NOT_RUN"
            ),
            "G5_C3_B8_fp2_allocation_comparison": (
                g5_c3fp2["status"] if g5_c3fp2 else "NOT_RUN"
            ),
            "G5_C3_B9_fp2_miller_prefix_integration": (
                g5_c3fp2prefix["status"] if g5_c3fp2prefix else "NOT_RUN"
            ),
            "G5_C3_B10_isolated_fp2_add_cost_profile": (
                g5_c3add["status"] if g5_c3add else "NOT_RUN"
            ),
            "G5_C3_B11_low_allocation_fp2_add_comparison": (
                g5_c3addlow["status"] if g5_c3addlow else "NOT_RUN"
            ),
            "G5_C3_B12_add2_miller_prefix_integration": (
                g5_c3addprefix["status"] if g5_c3addprefix else "NOT_RUN"
            ),
            "G5_C3_B13_isolated_fp2_sub_cost_profile": (
                g5_c3sub["status"] if g5_c3sub else "NOT_RUN"
            ),
            "G5_C3_B14_low_allocation_fp2_sub_comparison": (
                g5_c3sublow["status"] if g5_c3sublow else "NOT_RUN"
            ),
            "G5_C3_B15_sub2_miller_prefix_integration": (
                g5_c3subprefix["status"] if g5_c3subprefix else "NOT_RUN"
            ),
            "G5_C3_B16_latest_software_miller_full_attempt": (
                g5_c3subfull["status"] if g5_c3subfull else "NOT_RUN"
            ),
            "G5_C3_B17_derived_miller_budget_review": (
                g5_c3budget["status"] if g5_c3budget else "NOT_RUN"
            ),
            "G5_C3_B18_failed_full_attempt_cost_attribution": (
                g5_c3attribution["status"] if g5_c3attribution else "NOT_RUN"
            ),
            "G5_C3_B19_source_and_algebra_route_review": (
                g5_c3route["status"] if g5_c3route else "NOT_RUN"
            ),
            "G5_C3_B20_isolated_fp_quarter_comparison": (
                g5_c3quarter["status"] if g5_c3quarter else "NOT_RUN"
            ),
            "G5_C3_B21_quarter_mul_miller_prefix_integration": (
                g5_c3quarterprefix["status"] if g5_c3quarterprefix else "NOT_RUN"
            ),
            "G5_C3_B22_read_only_full_conformance_decision": (
                g5_c3fulldecision["status"] if g5_c3fulldecision else "NOT_RUN"
            ),
            "G5_C3_B23_quarter_full_conformance_attempt": (
                g5_c3quarterfull["status"] if g5_c3quarterfull else "NOT_RUN"
            ),
            "G5_C3_complete_gt_and_pairing_target_route": (
                "PARTIAL_PASS_FULL_QUARTER_MILLER_FINAL_EXPONENTIATION_PENDING"
                if g5_c3quarterfull and g5_c3quarterfull["full_miller_loop_verified_in_evm"]
                else "PARTIAL_PASS_QUARTER_PREFIX_FULL_MILLER_BLOCKED_FROZEN"
                if g5_c3quarterfull and g5_c3quarterfull.get("full_attempted")
                else "PARTIAL_PASS_QUARTER_PREFIX_FULL_VARIANT_UNTESTED"
                if g5_c3quarterprefix and g5_c3quarterprefix["prefix_conformance_and_accounting_passed"]
                else "PARTIAL_PASS_FULL_MILLER_FINAL_EXPONENTIATION_PENDING"
                if g5_c3subfull and g5_c3subfull["full_miller_loop_verified_in_evm"]
                else "PARTIAL_PASS_SUB2_PREFIX_FULL_MILLER_BLOCKED" if g5_c3subfull
                else "PARTIAL_PASS_SUB2_PREFIX_NEW_FULL_VARIANT_UNTESTED" if g5_c3subprefix
                else "PARTIAL_PASS_ADD2_PREFIX_NEW_FULL_VARIANT_UNTESTED" if g5_c3addprefix
                else "PARTIAL_PASS_FP2_PREFIX_NEW_FULL_VARIANT_UNTESTED" if g5_c3fp2prefix
                else "PARTIAL_PASS_MILLER_LOOP_FINAL_EXPONENTIATION_PENDING"
                if g5_miller_latest and g5_miller_latest["full_miller_loop_verified_in_evm"]
                else "PARTIAL_PASS_LOW_LEVEL_PREFIX_FULL_MILLER_BLOCKED" if g5_c3ml
                else "PARTIAL_PASS_SPARSE_PREFIX_FULL_MILLER_BLOCKED" if g5_c3s
                else "PARTIAL_PASS_MEMORY_OPTIMIZED_PREFIX_FULL_MILLER_BLOCKED" if g5_c3mem
                else "PARTIAL_PASS_SOFTWARE_MILLER_PREFIX_FULL_LOOP_BLOCKED" if g5_c3m
                else "PARTIAL_PASS_TIPP_ENDPOINT_HOST_CONFORMANCE_EVM_TARGET_BLOCKED" if g5_c3b
                else "PARTIAL_PASS_FP12_PRODUCT_PAIRING_TO_GT_BLOCKED" if g5_c3 else "NOT_RUN"
            ),
            "G5_C_exact_snarkpack_target_verifier": (
                g5_target_latest["target_verifier_status"] if g5_target_latest
                else g5_c3b["target_verifier_status"] if g5_c3b
                else g5_c3["target_verifier_status"] if g5_c3
                else g5_c2["target_verifier_status"] if g5_c2
                else g5_c["target_verifier_status"] if g5_c else "NOT_RUN"
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
        "g5_c1_snarkpack_route_prerequisites": g5_c,
        "g5_c2_internal_transcript_conformance": g5_c2,
        "g5_c3_fp12_product_component": g5_c3,
        "g5_c3b_actual_tipp_endpoint_route": g5_c3b,
        "g5_c3b2_software_miller_loop": g5_c3m,
        "g5_c3b3_memory_optimized_miller": g5_c3mem,
        "g5_c3b4_sparse_optimized_miller": g5_c3s,
        "g5_c3b5_isolated_field_cost_profile": g5_c3profile,
        "g5_c3b6_low_level_field_comparison": g5_c3low,
        "g5_c3b7_low_level_miller_integration": g5_c3ml,
        "g5_c3b8_fp2_allocation_comparison": g5_c3fp2,
        "g5_c3b9_fp2_miller_prefix_integration": g5_c3fp2prefix,
        "g5_c3b10_isolated_fp2_add_cost_profile": g5_c3add,
        "g5_c3b11_low_allocation_fp2_add_comparison": g5_c3addlow,
        "g5_c3b12_add2_miller_prefix_integration": g5_c3addprefix,
        "g5_c3b13_isolated_fp2_sub_cost_profile": g5_c3sub,
        "g5_c3b14_low_allocation_fp2_sub_comparison": g5_c3sublow,
        "g5_c3b15_sub2_miller_prefix_integration": g5_c3subprefix,
        "g5_c3b16_latest_software_miller_full_attempt": g5_c3subfull,
        "g5_c3b17_derived_miller_budget_review": g5_c3budget,
        "g5_c3b18_failed_full_attempt_cost_attribution": g5_c3attribution,
        "g5_c3b19_source_and_algebra_route_review": g5_c3route,
        "g5_c3b20_isolated_fp_quarter_comparison": g5_c3quarter,
        "g5_c3b21_quarter_mul_miller_prefix_integration": g5_c3quarterprefix,
        "g5_c3b22_read_only_full_conformance_decision": g5_c3fulldecision,
        "g5_c3b23_quarter_full_conformance_attempt": g5_c3quarterfull,
        "g5_current_progress": {
            "latest_completed_gate": g5_next_work["gate"] if g5_next_work else None,
            "latest_gate_is_derived_analysis": bool(g5_next_work and g5_next_work.get("derived_nonbenchmark", False)),
            "latest_prefix_gate": g5_prefix_latest["gate"] if g5_prefix_latest else None,
            "latest_full_attempt_gate": g5_miller_latest["gate"] if g5_miller_latest else None,
            "fixed_budget_software_route_frozen": bool(g5_c3quarterfull and g5_c3quarterfull.get("software_route_frozen_by_failure")),
            "next_gate": g5_next_work["next_gate"] if g5_next_work else None,
        },
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
    ]
    if g5_next_work:
        lines.extend([
            "## Current G5 checkpoint",
            "",
            f"- Latest recorded gate: `{g5_next_work['gate']}` / `{g5_next_work['status']}`",
            f"- Latest measured prefix: `{g5_prefix_latest['gate'] if g5_prefix_latest else 'NOT_RUN'}`; latest actual full-loop attempt: `{g5_miller_latest['gate'] if g5_miller_latest else 'NOT_RUN'}`",
            "- A prefix pass does not establish full-loop conformance or a completed SnarkPack verifier; historical sections below retain their original scope.",
            f"- Next gate: {g5_next_work['next_gate']}",
            "",
        ])
    if g5_c3budget:
        lines.extend([
            "Historical B17 was derived analysis only, with zero new EVM transactions; it did not remove the full-loop/SnarkPack blocker.",
            "",
        ])
    lines.extend([
        "## Gate status",
        "",
        "| Gate | Status |",
        "|---|---|",
    ])
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
    if g5_c:
        lines.extend([
            "",
            "## G5-C1 SnarkPack public wire and Prague prerequisites",
            "",
            f"- Prerequisite gate: `{g5_c['status']}`",
            f"- Exact target verifier: `{g5_c['target_verifier_status']}`",
            f"- Batch / public inputs per proof / aggregate bytes: `{g5_c['batch_size']} / {g5_c['public_inputs_per_proof']} / {g5_c['aggregate_proof_bytes']}`",
            f"- Decoded native aggregate / changed second nullifier rejected: `{g5_c['decoded_native_aggregate_verified']} / {g5_c['decoded_changed_second_nullifier_rejected']}`",
            f"- External transcript Rust/JS match: `{g5_c['external_transcript_rust_js_match']}`",
            f"- Runtime primitive probes: `{len(g5_c['primitive_probes'])}` (not aggregate verification)",
            f"- Exact SnarkPack EVM / aggregate verification gas: `{g5_c['exact_snarkpack_evm_verified']} / {g5_c['snarkpack_verification_gas']}`",
            f"- Blocker: {g5_c['blocker']}",
            f"- Next gate: {g5_c['next_gate']}",
        ])
    if g5_c2:
        lines.extend([
            "",
            "## G5-C2 official internal transcript conformance",
            "",
            f"- Status: `{g5_c2['status']}`",
            f"- Actual official calls matched: `{g5_c2['matched_call_count']} / 12`",
            f"- Independent JS GT wire decodes per case: `{g5_c2['gt_elements_independently_decoded_per_case']}`",
            f"- Complete native valid / wrong nullifier rejected: `{g5_c2['official_complete_native_valid']} / {g5_c2['official_complete_native_changed_nullifier_rejected']}`",
            f"- Full preimages / challenges / rejection counters match: `{g5_c2['rust_js_preimages_match']} / {g5_c2['rust_js_challenges_match']} / {g5_c2['rust_js_rejection_counters_match']}`",
            f"- Real rejected attempts observed: `{g5_c2['rejected_attempts_observed']}`",
            f"- Setup / prove run: `{g5_c2['setup_run']} / {g5_c2['prove_run']}`",
            f"- Exact SnarkPack EVM / aggregate verification gas: `{g5_c2['exact_snarkpack_evm_verified']} / {g5_c2['snarkpack_verification_gas']}`",
            f"- Next gate: {g5_c2['next_gate']}",
        ])
    if g5_c3:
        lines.extend([
            "",
            "## G5-C3-A exact Fp12 product component in local Prague EVM",
            "",
            f"- Component status: `{g5_c3['status']}`",
            f"- Native / independent JS / Solidity product match: `{g5_c3['native_js_solidity_product_match']}`",
            f"- Valid / wrong-target transaction status: `{g5_c3['valid']['receipt_status']} / {g5_c3['wrong_target']['receipt_status']}`",
            f"- Valid / wrong-target total gas: `{g5_c3['valid']['total_gas']} / {g5_c3['wrong_target']['total_gas']}` (one multiplication component only)",
            f"- MODEXP calls per transaction / combined precompile execution gas: `{g5_c3['valid']['modexp_call_count']} / {g5_c3['valid']['modexp_execution_gas']}`",
            f"- Calldata / runtime bytes: `{g5_c3['valid']['calldata_bytes']} / {g5_c3['runtime_bytecode_bytes']}`",
            f"- Complete G5-C3 / pairing-to-GT / exact SnarkPack EVM: `{g5_c3['complete_g5c3']} / {g5_c3['pairing_to_gt_verified']} / {g5_c3['exact_snarkpack_evm_verified']}`",
            f"- Complete SnarkPack verification gas: `{g5_c3['snarkpack_verification_gas']}`",
            f"- Next gate: {(g5_next_work or g5_c3b or g5_c3)['next_gate']}",
        ])
    if g5_c3b:
        lines.extend([
            "",
            "## G5-C3-B actual TIPP endpoint: host conformance, EVM route blocked",
            "",
            f"- Status: `{g5_c3b['status']}`",
            f"- Native / independent JS endpoint equality: `{g5_c3b['native_endpoint_verified']} / {g5_c3b['independent_js_endpoint_verified']}`",
            f"- Native / JS wrong in-subgroup GT target rejected: `{g5_c3b['native_wrong_gt_target_rejected']} / {g5_c3b['independent_js_wrong_gt_target_rejected']}` (not EVM rejection)",
            f"- Independently recomputed C2 transcript calls: `{g5_c3b['independently_recomputed_transcript_calls']}`",
            f"- Actual-point EVM identity probe: `{g5_c3b['evm_identity_probe']['execution_gas']}` primitive gas, 32-byte false; not pairing-to-GT verification or transaction gas",
            f"- Pairing-to-GT EVM / EVM wrong-target rejection: `{g5_c3b['pairing_to_gt_evm_verified']} / {g5_c3b['evm_wrong_gt_target_rejected']}`",
            f"- Resource conclusion: {g5_c3b['resource_conclusion']}",
            f"- Blocker: {g5_c3b['blocker']}",
            f"- Complete SnarkPack verification gas: `{g5_c3b['snarkpack_verification_gas']}`",
            f"- Next gate: {(g5_next_work or g5_c3b)['next_gate']}",
        ])
    if g5_c3m:
        lines.extend([
            "",
            "## G5-C3-B2 software Miller-loop component, fixed budget",
            "",
            f"- Status: `{g5_c3m['status']}`",
            f"- Prefix / full steps: `{g5_c3m['prefix_steps']} / {g5_c3m['full_steps']}`",
            f"- Prefix EVM match / wrong prefix rejected: `{g5_c3m['prefix_correctness_verified_in_evm']} / {g5_c3m['wrong_prefix_target_rejected_in_evm']}`",
            f"- Prefix valid / wrong-result total gas: `{g5_c3m['prefix_valid']['total_gas']} / {g5_c3m['prefix_wrong_target']['total_gas']}` (prefix component only)",
            f"- Prefix MODEXP calls / runtime bytes: `{g5_c3m['prefix_valid']['modexp_call_count']} / {g5_c3m['runtime_bytecode_bytes']}`",
            f"- Full-loop verified / exception: `{g5_c3m['full_miller_loop_verified_in_evm']} / {g5_c3m['full_attempt']['exception']}`",
            f"- Full attempt gas spent / explicit transaction cap: `{g5_c3m['full_attempt']['total_gas']} / {g5_c3m['transaction_gas_limit']}` (failed-attempt spend is NOT completed-loop cost)",
            f"- Full attempt active memory words at exit: `{g5_c3m['full_attempt']['evm_active_memory_words_at_exit']}`",
            f"- EVM final exponentiation / pairing-to-GT / SnarkPack: `{g5_c3m['evm_final_exponentiation_implemented']} / {g5_c3m['pairing_to_gt_evm_verified']} / {g5_c3m['exact_snarkpack_evm_verified']}`",
            f"- Complete SnarkPack verification gas: `{g5_c3m['snarkpack_verification_gas']}`",
            f"- Next gate: {g5_next_work['next_gate']}",
        ])
    if g5_c3mem:
        comparison = g5_c3mem["comparison"]
        lines.extend([
            "",
            "## G5-C3-B3 memory-only Miller optimization, same endpoint/budget",
            "",
            f"- Status: `{g5_c3mem['status']}`",
            f"- Prefix valid / wrong result total gas: `{g5_c3mem['prefix_valid']['total_gas']} / {g5_c3mem['prefix_wrong_target']['total_gas']}`",
            f"- Same two-step prefix gas: `{comparison['baseline_prefix_gas']} -> {comparison['optimized_prefix_gas']}` ({comparison['prefix_gas_reduction_percent']:.2f}% reduction, not end-to-end speedup)",
            f"- Same prefix EVM active memory words: `{comparison['baseline_prefix_memory_words']} -> {comparison['optimized_prefix_memory_words']}` ({comparison['prefix_memory_reduction_percent']:.2f}% reduction)",
            f"- Prefix MODEXP count unchanged: `{g5_c3mem['prefix_valid']['modexp_call_count']}`; wrong result rejected: `{g5_c3mem['wrong_prefix_target_rejected_in_evm']}`",
            f"- Full-loop verified / exception / capped spend: `{g5_c3mem['full_miller_loop_verified_in_evm']} / {g5_c3mem['full_attempt']['exception']} / {g5_c3mem['full_attempt']['total_gas']}` (not completed-loop cost)",
            f"- Full-attempt active memory words / executed MODEXP calls: `{g5_c3mem['full_attempt']['evm_active_memory_words_at_exit']} / {g5_c3mem['full_attempt']['modexp_call_count']}`",
            f"- EVM final exponentiation / pairing-to-GT / full SnarkPack: `{g5_c3mem['evm_final_exponentiation_implemented']} / {g5_c3mem['pairing_to_gt_evm_verified']} / {g5_c3mem['exact_snarkpack_evm_verified']}`",
            f"- Complete SnarkPack verification gas: `{g5_c3mem['snarkpack_verification_gas']}`",
            f"- Next gate: {g5_next_work['next_gate']}",
        ])
    if g5_c3s:
        sparse_comparison = g5_c3s["comparison"]
        lines.extend([
            "",
            "## G5-C3-B4 dedicated Fp12 square / sparse-014 line product",
            "",
            f"- Status: `{g5_c3s['status']}`",
            f"- Same prefix gas B2 / B3 / B4: `{sparse_comparison['unoptimized_b2_prefix_gas']} / {sparse_comparison['baseline_prefix_gas']} / {sparse_comparison['optimized_prefix_gas']}`",
            f"- Prefix reduction versus B3: `{sparse_comparison['prefix_gas_reduction_percent']:.2f}%` (component-only, not end-to-end)",
            f"- Actual prefix MODEXP calls B3 -> B4: `{sparse_comparison['baseline_prefix_modexp_calls']} -> {sparse_comparison['optimized_prefix_modexp_calls']}`",
            f"- Prefix valid / wrong-result decision: `{g5_c3s['prefix_correctness_verified_in_evm']} / {g5_c3s['wrong_prefix_target_rejected_in_evm']}`",
            f"- Specialized host reference matches: `{g5_c3s['specialized_reference_squares_matched']}` squares and `{g5_c3s['specialized_reference_lines_matched']}` line products; not full EVM conformance",
            f"- Full loop verified / exception / capped spend: `{g5_c3s['full_miller_loop_verified_in_evm']} / {g5_c3s['full_attempt']['exception']} / {g5_c3s['full_attempt']['total_gas']}` (not completed-loop cost)",
            f"- Full-attempt memory words / MODEXP calls: `{g5_c3s['full_attempt']['evm_active_memory_words_at_exit']} / {g5_c3s['full_attempt']['modexp_call_count']}`",
            f"- Final exponentiation / pairing-to-GT / SnarkPack EVM: `{g5_c3s['evm_final_exponentiation_implemented']} / {g5_c3s['pairing_to_gt_evm_verified']} / {g5_c3s['exact_snarkpack_evm_verified']}`",
            f"- Complete SnarkPack verification gas: `{g5_c3s['snarkpack_verification_gas']}`",
            f"- Next gate: {g5_next_work['next_gate']}",
        ])
    if g5_c3profile:
        lines.extend([
            "",
            "## G5-C3-B5 isolated Fp / Fp2 cost profile",
            "",
            f"- Status: `{g5_c3profile['status']}`; no Miller/proving rerun",
            "- Windows exclude decoding/encoding but include instrumentation and compiled-call overhead; not universal per-operation constants.",
            "",
            "| Operation | Total tx gas | Execution gas | Arithmetic-window gas | MODEXP gas / calls | Window memory-expansion gas |",
            "|---|---:|---:|---:|---:|---:|",
        ])
        for field in (g5_c3profile["fp"], g5_c3profile["fp2"]):
            lines.append(
                f"| {field['operation']} | {field['total_gas']} | {field['execution_gas']} | {field['instrumented_arithmetic_window_gas']} | {field['modexp_execution_gas']} / {field['modexp_call_count']} | {field['window_evm_memory_expansion_gas']} |"
            )
        lines.extend([
            "",
            "- Full returned products match the fixed public operands' direct BigInt and Noble reference products.",
            "- Root opcode gas plus child MODEXP gas reconciles execution gas; call and memory counters are not mutually exclusive attribution.",
            f"- Full Miller run / exact SnarkPack / aggregate gas: `{g5_c3profile['miller_run']} / {g5_c3profile['exact_snarkpack_evm_verified']} / {g5_c3profile['snarkpack_verification_gas']}`",
            f"- Next gate: {g5_next_work['next_gate']}",
        ])
    if g5_c3low:
        lines.extend([
            "",
            "## G5-C3-B6 low-level Fp multiplication comparison",
            "",
            f"- Status: `{g5_c3low['status']}`; primitive-only, no Miller/proving rerun",
            "- Same public operands, compiler and measurement windows as B5; only the _mul implementation changes.",
            "",
            "| Operation | B5 window gas | B6 window gas | Reduction | B6 total tx gas | MODEXP gas / calls | Window allocator growth B5 -> B6 (B) |",
            "|---|---:|---:|---:|---:|---:|---:|",
        ])
        for key in ("fp", "fp2"):
            field = g5_c3low[key]
            comparison = g5_c3low["comparison"][key]
            lines.append(
                f"| {field['operation']} | {comparison['baseline_window_gas']} | {comparison['optimized_window_gas']} | {comparison['window_gas_reduction_percent']:.2f}% | {field['total_gas']} | {field['modexp_execution_gas']} / {field['modexp_call_count']} | {comparison['baseline_allocator_growth_bytes']} -> {comparison['optimized_allocator_growth_bytes']} |"
            )
        boundaries = g5_c3low["boundaries"]
        lines.extend([
            "",
            f"- Deterministic arithmetic boundaries verified: `{sum(item['verified'] for item in boundaries)} / {len(boundaries)}`; excluded from the two cost-comparison samples.",
            "- Complete outputs match direct BigInt and Noble; host-derived carry/borrow/odd-halving flags are not internal instruction coverage or an exhaustive arithmetic proof.",
            "- Gas counters reconcile; isolated cost reductions are not measured Miller-loop or end-to-end gains.",
            f"- Full Miller run / exact SnarkPack / aggregate gas: `{g5_c3low['miller_run']} / {g5_c3low['exact_snarkpack_evm_verified']} / {g5_c3low['snarkpack_verification_gas']}`",
            f"- Next gate: {g5_next_work['next_gate']}",
        ])
    if g5_c3ml:
        comparison = g5_c3ml["comparison"]
        full = g5_c3ml["full_attempt"]
        lines.extend([
            "",
            "## G5-C3-B7 low-level Fp integration into sparse Miller",
            "",
            f"- Status: `{g5_c3ml['status']}`",
            "- Exact B6 _mul replaces B4 _mul; all other source is byte-identical except the contract name.",
            f"- Same-prefix total gas B4 -> B7: `{comparison['baseline_prefix_gas']} -> {comparison['optimized_prefix_gas']}` ({comparison['prefix_gas_reduction_percent']:.2f}% reduction)",
            f"- Prefix active memory words B4 -> B7: `{comparison['baseline_prefix_memory_words']} -> {comparison['optimized_prefix_memory_words']}` ({comparison['prefix_memory_reduction_percent']:.2f}% reduction)",
            f"- Prefix valid / wrong-target rejection: `{g5_c3ml['prefix_correctness_verified_in_evm']} / {g5_c3ml['wrong_prefix_target_rejected_in_evm']}`; MODEXP calls unchanged: `{g5_c3ml['prefix_valid']['modexp_call_count']}`",
            f"- Exactly one full-loop attempt; verified / root exception / gas spent: `{g5_c3ml['full_miller_loop_verified_in_evm']} / {full['exception']} / {full['total_gas']}`",
            f"- Full-attempt active memory words / completed MODEXP calls: `{full['evm_active_memory_words_at_exit']} / {full['modexp_success_count']}`; not a completed-step or completion-percentage measurement",
            f"- Completed full-Miller gas: `{g5_c3ml['full_miller_verification_gas']}`; failed capped spend is not full-loop cost",
            f"- EVM final exponentiation / pairing-to-GT / exact SnarkPack: `{g5_c3ml['evm_final_exponentiation_implemented']} / {g5_c3ml['pairing_to_gt_evm_verified']} / {g5_c3ml['exact_snarkpack_evm_verified']}`",
            f"- Complete SnarkPack verification gas: `{g5_c3ml['snarkpack_verification_gas']}`; no setup/prove or scale runs",
            f"- Next gate: {g5_next_work['next_gate']}",
        ])
    if g5_c3fp2:
        lines.extend([
            "",
            "## G5-C3-B8 isolated Fp2 allocation comparison",
            "",
            f"- Status: `{g5_c3fp2['status']}`; only _mul2 changes, exact B6 _mul retained",
            "- Same public cost operands and windows as B6; no Miller/proving rerun.",
            "",
            "| Operation | B6 window gas | B8 window gas | Window reduction | B6 -> B8 total tx gas | B6 -> B8 allocator growth (B) | MODEXP gas / calls |",
            "|---|---:|---:|---:|---:|---:|---:|",
        ])
        for key in ("fp", "fp2"):
            field = g5_c3fp2[key]
            comparison = g5_c3fp2["comparison"][key]
            lines.append(
                f"| {field['operation']} | {comparison['baseline_window_gas']} | {comparison['optimized_window_gas']} | {comparison['window_gas_reduction_percent']:.2f}% | {comparison['baseline_total_gas']} -> {comparison['optimized_total_gas']} | {comparison['baseline_allocator_growth_bytes']} -> {comparison['optimized_allocator_growth_bytes']} | {field['modexp_execution_gas']} / {field['modexp_call_count']} |"
            )
        boundaries = g5_c3fp2["boundaries"]
        lines.extend([
            "",
            f"- Necessary Fp2 boundaries verified: `{sum(item['verified'] for item in boundaries)} / {len(boundaries)}`; pure-imaginary square, near-modulus square and cross-limb sum/subtract case; not performance samples.",
            "- Complete outputs match BigInt and Noble; host-derived branch flags are not instruction coverage or a formal arithmetic proof.",
            "- Fp control window remains unchanged; its total transaction gas increases slightly outside the window, not an across-the-board cost reduction.",
            "- Latest full-Miller attempt remains B7 at20M OOG; B8 does not promote any Miller/pairing/SnarkPack capability.",
            f"- Miller run / exact SnarkPack / aggregate gas: `{g5_c3fp2['miller_run']} / {g5_c3fp2['exact_snarkpack_evm_verified']} / {g5_c3fp2['snarkpack_verification_gas']}`",
            f"- Next gate: {g5_next_work['next_gate']}",
        ])
    if g5_c3fp2prefix:
        comparison = g5_c3fp2prefix["comparison"]
        lines.extend([
            "",
            "## G5-C3-B9 Fp2 integration: prefix only, no full-loop retry",
            "",
            f"- Status: `{g5_c3fp2prefix['status']}`; exact B8 _mul2 integrated into B7, other source unchanged except the contract name",
            f"- Same-prefix gas B7 -> B9: `{comparison['baseline_prefix_gas']} -> {comparison['optimized_prefix_gas']}` ({comparison['prefix_gas_reduction_percent']:.2f}% reduction)",
            f"- Active memory words B7 -> B9: `{comparison['baseline_prefix_memory_words']} -> {comparison['optimized_prefix_memory_words']}` ({comparison['prefix_memory_reduction_percent']:.2f}% reduction)",
            f"- Valid / wrong-result rejection: `{g5_c3fp2prefix['prefix_correctness_verified_in_evm']} / {g5_c3fp2prefix['wrong_prefix_target_rejected_in_evm']}`; MODEXP calls unchanged: `{g5_c3fp2prefix['prefix_valid']['modexp_call_count']}`",
            f"- Full-loop attempts in B9: `{g5_c3fp2prefix['full_loop_attempts']}`; new-variant full result: `{g5_c3fp2prefix['full_attempt']}` (not tested, not a new OOG)",
            f"- Latest actual full attempt: `{g5_c3fp2prefix['latest_full_attempt_gate']}`; inherited target status source: `{g5_c3fp2prefix['target_verifier_status_source']}`",
            "- B7 OOG is evidence for the previous variant; prefix savings do not establish this new variant's complete-loop correctness or gas cost.",
            f"- Completed full-Miller / SnarkPack gas: `{g5_c3fp2prefix['full_miller_verification_gas']} / {g5_c3fp2prefix['snarkpack_verification_gas']}`; final exponentiation and pairing target remain unimplemented",
            f"- Next gate: {g5_next_work['next_gate']}",
        ])
    if g5_c3add:
        addition = g5_c3add["fp2_add"]
        lines.extend([
            "",
            "## G5-C3-B10 unchanged Fp2 addition cost localization",
            "",
            f"- Status: `{g5_c3add['status']}`; arithmetic changed / optimization implemented: `{g5_c3add['arithmetic_changed']} / {g5_c3add['optimization_implemented']}`",
            "- One deployment and ONE fixed public addition; complete output matches BigInt/Noble. No extra tests or Miller/proving rerun.",
            f"- Instrumented arithmetic window / execution gas: `{addition['instrumented_arithmetic_window_gas']} / {addition['execution_gas']}`; not an inlined Miller constant",
            f"- Intrinsic / total / minimum gas including calldata floor: `{addition['intrinsic_gas']} / {addition['total_gas']} / {addition['minimum_gas_limit_including_calldata_floor']}`",
            f"- Calldata floor applied / extra gas: `{addition['calldata_floor_applied']} / {addition['calldata_floor_extra_gas']}`; pricing extra is NOT arithmetic-window cost",
            f"- Allocator growth / window memory-expansion gas: `{addition['allocator_growth_in_window_bytes']} B / {addition['window_evm_memory_expansion_gas']}`; not host RSS",
            f"- MODEXP calls / gas: `{addition['modexp_call_count']} / {addition['modexp_execution_gas']}`; root opcode gas reconciles execution",
            "- Fixed operands exercise one ordinary component sum and one reduced sum with low-limb carry/borrow; host-derived conditions, not exhaustive instruction coverage.",
            "- No speedup claim: this is a saved baseline. A future lower window need not lower this transaction's floor-limited total.",
            f"- At B10, latest prefix / full attempt were B9 / `{g5_c3add['latest_full_attempt_gate']}`; aggregate gas `{g5_c3add['snarkpack_verification_gas']}`",
            f"- Next gate: {g5_next_work['next_gate']}",
        ])
    if g5_c3addlow:
        comparison = g5_c3addlow["comparison"]
        addition = g5_c3addlow["fp2_add"]
        lines.extend([
            "",
            "## G5-C3-B11 low-allocation Fp2 addition: isolated comparison only",
            "",
            f"- Status: `{g5_c3addlow['status']}`; only _add2 implementation changes, mathematical formula/codec unchanged",
            f"- Same-input window gas B10 -> B11: `{comparison['baseline_window_gas']} -> {comparison['optimized_window_gas']}` ({comparison['window_gas_reduction_percent']:.2f}% reduction)",
            f"- Execution gas B10 -> B11: `{comparison['baseline_execution_gas']} -> {comparison['optimized_execution_gas']}`",
            f"- Allocator growth B10 -> B11: `{comparison['baseline_allocator_growth_bytes']} -> {comparison['optimized_allocator_growth_bytes']}` B ({comparison['allocator_growth_reduction_percent']:.2f}% reduction)",
            f"- Total transaction gas B10 -> B11: `{comparison['baseline_total_gas']} -> {comparison['optimized_total_gas']}`; floor extra `{comparison['baseline_floor_extra_gas']} -> {comparison['optimized_floor_extra_gas']}`",
            "- Execution/window savings do NOT lower this floor-limited transaction total; they are not measured Miller or E2E gains.",
            f"- One equality/zero boundary verified: `{g5_c3addlow['all_boundaries_verified']}`; excluded from the one performance comparison sample",
            f"- MODEXP calls / actual window memory-expansion gas: `{addition['modexp_call_count']} / {addition['window_evm_memory_expansion_gas']}`; explicit Yul output size is not the full allocator growth",
            f"- Miller run / full-loop attempts / aggregate-verifier gas: `{g5_c3addlow['miller_run']} / {g5_c3addlow['full_loop_attempts']} / {g5_c3addlow['snarkpack_verification_gas']}`; B9/B7 history unchanged",
            f"- Next gate: {g5_next_work['next_gate']}",
        ])
    if g5_c3addprefix:
        comparison = g5_c3addprefix["comparison"]
        prefix = g5_c3addprefix["prefix_valid"]
        lines.extend([
            "",
            "## G5-C3-B12 low-allocation Fp2 addition integrated into the Miller prefix",
            "",
            f"- Status: `{g5_c3addprefix['status']}`; exact B11 _add2 integrated, other source bodies unchanged",
            f"- Same-prefix total gas B9 -> B12: `{comparison['baseline_prefix_gas']} -> {comparison['optimized_prefix_gas']}` ({comparison['prefix_gas_reduction_percent']:.2f}% reduction)",
            f"- Active memory B9 -> B12: `{comparison['baseline_prefix_memory_words']} -> {comparison['optimized_prefix_memory_words']}` words ({comparison['prefix_memory_reduction_percent']:.2f}% reduction)",
            f"- Successful MODEXP calls / gas: `{prefix['modexp_success_count']} / {prefix['modexp_execution_gas']}`; unchanged operation count",
            f"- Wrong canonical target rejected: `{g5_c3addprefix['wrong_prefix_target_rejected_in_evm']}`",
            f"- Runtime / deployment gas: `{g5_c3addprefix['runtime_bytecode_bytes']} B / {g5_c3addprefix['deployment']['total_gas']}`; both slightly higher than B9",
            f"- New-variant full attempts / full attempt: `{g5_c3addprefix['full_loop_attempts']} / {g5_c3addprefix['full_attempt']}`; latest actual full attempt remains `{g5_c3addprefix['latest_full_attempt_gate']}` on an older variant",
            f"- Full Miller / final exponentiation / exact SnarkPack verified: `{g5_c3addprefix['full_miller_loop_verified_in_evm']} / {g5_c3addprefix['evm_final_exponentiation_implemented']} / {g5_c3addprefix['exact_snarkpack_evm_verified']}`; aggregate gas `{g5_c3addprefix['snarkpack_verification_gas']}`",
            "- Prefix savings are measured only for the saved two-step input; full-loop cost is unknown for B12.",
            f"- Next gate: {g5_next_work['next_gate']}",
        ])
    if g5_c3sub:
        subtraction = g5_c3sub["fp2_sub"]
        lines.extend([
            "",
            "## G5-C3-B13 unchanged Fp2 subtraction: one cost baseline",
            "",
            f"- Status: `{g5_c3sub['status']}`; arithmetic changed / optimization implemented: `{g5_c3sub['arithmetic_changed']} / {g5_c3sub['optimization_implemented']}`",
            f"- Arithmetic transactions / complete canonical result verified: `{g5_c3sub['arithmetic_transactions']} / {subtraction['verified']}`",
            f"- Arithmetic window / execution / intrinsic gas: `{subtraction['instrumented_arithmetic_window_gas']} / {subtraction['execution_gas']} / {subtraction['intrinsic_gas']}`",
            f"- Total / calldata-floor supplement: `{subtraction['total_gas']} / {subtraction['calldata_floor_extra_gas']}` gas; supplement is outside the arithmetic window",
            f"- Allocator growth / window memory-expansion gas / exit active memory: `{subtraction['allocator_growth_in_window_bytes']} B / {subtraction['window_evm_memory_expansion_gas']} / {subtraction['evm_active_memory_words_at_exit']} words`",
            f"- MODEXP calls / STATICCALL opcode gas: `{subtraction['modexp_call_count']} / {subtraction['staticcall_opcode_gas']}`",
            "- Fixed input uses direct subtraction in both components, with a low-limb borrow in c0 only; modular-wrap branch is not covered.",
            "- B13 is a profiling baseline. Candidate comparisons are recorded separately below; this window is not an inlined Miller cost.",
            f"- Latest prefix / actual full attempt: `{g5_c3sub['prefix_baseline']} / {g5_c3sub['latest_full_attempt_gate']}`",
            f"- Miller run / full-loop attempts / aggregate gas: `{g5_c3sub['miller_run']} / {g5_c3sub['full_loop_attempts']} / {g5_c3sub['snarkpack_verification_gas']}`",
            f"- Next gate: {g5_next_work['next_gate']}",
        ])
    if g5_c3sublow:
        comparison = g5_c3sublow["comparison"]
        subtraction = g5_c3sublow["fp2_sub"]
        lines.extend([
            "",
            "## G5-C3-B14 low-allocation Fp2 subtraction: isolated comparison",
            "",
            f"- Status: `{g5_c3sublow['status']}`; only _sub2 implementation changes, all other function bodies unchanged",
            f"- Same-input window gas B13 -> B14: `{comparison['baseline_window_gas']} -> {comparison['optimized_window_gas']}` ({comparison['window_gas_reduction_percent']:.2f}% reduction)",
            f"- Execution gas B13 -> B14: `{comparison['baseline_execution_gas']} -> {comparison['optimized_execution_gas']}`",
            f"- Allocator growth B13 -> B14: `{comparison['baseline_allocator_growth_bytes']} -> {comparison['optimized_allocator_growth_bytes']}` B ({comparison['allocator_growth_reduction_percent']:.2f}% reduction)",
            f"- Window memory-expansion gas B13 -> B14: `{comparison['baseline_window_memory_gas']} -> {comparison['optimized_window_memory_gas']}`",
            f"- Total transaction gas B13 -> B14: `{comparison['baseline_total_gas']} -> {comparison['optimized_total_gas']}`; calldata-floor supplement `{comparison['baseline_floor_extra_gas']} -> {comparison['optimized_floor_extra_gas']}`",
            "- Execution/window savings do not lower the floor-limited transaction total and do not establish Miller or E2E savings.",
            f"- One modular-wrap boundary verified: `{g5_c3sublow['all_boundaries_verified']}`; `(0,0) - (1,2^256) = (p-1,p-2^256)`, excluded from performance comparison",
            f"- Arithmetic transactions / performance samples / MODEXP calls per input: `{g5_c3sublow['arithmetic_transactions']} / {g5_c3sublow['performance_comparison_samples']} / {subtraction['modexp_call_count']}`",
            f"- Miller run / full-loop attempts / aggregate gas: `{g5_c3sublow['miller_run']} / {g5_c3sublow['full_loop_attempts']} / {g5_c3sublow['snarkpack_verification_gas']}`; at B14, latest prefix was B12 and actual full attempt B7",
            f"- Next gate: {g5_next_work['next_gate']}",
        ])
    if g5_c3subprefix:
        comparison = g5_c3subprefix["comparison"]
        prefix = g5_c3subprefix["prefix_valid"]
        lines.extend([
            "",
            "## G5-C3-B15 low-allocation Fp2 subtraction integrated into the Miller prefix",
            "",
            f"- Status: `{g5_c3subprefix['status']}`; exact B14 _sub2 integrated, all other B12 source bodies unchanged",
            f"- Same-prefix total gas B12 -> B15: `{comparison['baseline_prefix_gas']} -> {comparison['optimized_prefix_gas']}` ({comparison['prefix_gas_reduction_percent']:.2f}% reduction)",
            f"- Active memory B12 -> B15: `{comparison['baseline_prefix_memory_words']} -> {comparison['optimized_prefix_memory_words']}` words ({comparison['prefix_memory_reduction_percent']:.2f}% reduction)",
            f"- Successful MODEXP calls / gas: `{prefix['modexp_success_count']} / {prefix['modexp_execution_gas']}`; count and trace unchanged",
            f"- Complete saved prefix / wrong target rejected: `{g5_c3subprefix['prefix_correctness_verified_in_evm']} / {g5_c3subprefix['wrong_prefix_target_rejected_in_evm']}`",
            f"- Runtime / deployment gas: `{g5_c3subprefix['runtime_bytecode_bytes']} B / {g5_c3subprefix['deployment']['total_gas']}`; increases versus B12 are 114 B / 24,694 gas",
            f"- At B15, full attempts / full attempt were `{g5_c3subprefix['full_loop_attempts']} / {g5_c3subprefix['full_attempt']}`; its historical full source was `{g5_c3subprefix['latest_full_attempt_gate']}`",
            "- This is a prefix-only result. The subsequent full-only B16 record determines the current full-loop status.",
            f"- Next gate: {g5_next_work['next_gate']}",
        ])
    if g5_c3subfull:
        attempt = g5_c3subfull["full_attempt"]
        lines.extend([
            "",
            "## G5-C3-B16 unchanged B15 contract: one full64 attempt",
            "",
            f"- Status: `{g5_c3subfull['status']}`; same source/runtime, compiler, fixture and 20M transaction cap",
            f"- Full attempts / prefix checks executed this run: `{g5_c3subfull['full_loop_attempts']} / {g5_c3subfull['prefix_checks_executed_this_run']}`",
            f"- Actual receipt / exception / capped-or-actual total gas: `{attempt['receipt_status']} / {attempt['exception']} / {attempt['total_gas']}`",
            f"- Execution / intrinsic gas: `{attempt['execution_gas']} / {attempt['intrinsic_gas']}`; zero refund",
            f"- Successful / failed MODEXP calls: `{attempt['modexp_success_count']} / {attempt['modexp_failure_count']}`; child execution gas `{attempt['modexp_execution_gas']}`",
            f"- Active memory at exit: `{attempt['evm_active_memory_words_at_exit']}` words; last observed root step `{attempt['last_root_step']}`",
            f"- Full-loop conformance / completed-loop gas: `{g5_c3subfull['full_miller_loop_verified_in_evm']} / {g5_c3subfull['full_miller_verification_gas']}`",
            "- Failed capped spend is not a completed-loop cost; call counts do not measure completion percentage.",
            f"- At B16 the latest actual full attempt was B16; previous full attempt was `{g5_c3subfull['previous_full_attempt_gate']}`. B15 prefix evidence is retained.",
            f"- EVM final exponentiation / exact SnarkPack verified: `{g5_c3subfull['evm_final_exponentiation_implemented']} / {g5_c3subfull['exact_snarkpack_evm_verified']}`; aggregate gas `{g5_c3subfull['snarkpack_verification_gas']}`",
            f"- Next gate: {g5_next_work['next_gate']}",
        ])
    if g5_c3budget:
        costs = g5_c3budget["derived_costs"]
        terminal = g5_c3budget["last_observed_root_source"]
        lines.extend([
            "",
            "## G5-C3-B17 saved-evidence budget review: derived analysis, no EVM run",
            "",
            f"- Status: `{g5_c3budget['status']}`; new EVM transactions: `{g5_c3budget['evm_transactions_executed']}`",
            f"- Source-map compilation reproduces the saved runtime: `{g5_c3budget['runtime_bytecode_bytes']}` B / `{g5_c3budget['runtime_sha256']}`",
            f"- Terminal source: PC `{terminal['pc']}`, `{terminal['source_unit']}:{terminal['line']}`, `{terminal['snippet']}`; terminal trigger only, not a cumulative hotspot",
            f"- Saved successful MODEXP child gas / observed execution: `{costs['successful_modexp_child_gas']} / {costs['observed_execution_gas']}` ({costs['successful_modexp_child_percent_of_execution']:.4f}%)",
            f"- Execution gas not attributed to those children: `{costs['execution_gas_not_attributed_to_successful_modexp_children']}`; not all field arithmetic",
            f"- Memory expansion at the saved watermark: `{costs['memory_expansion_gas_at_observed_watermark']}` gas ({costs['memory_expansion_percent_of_observed_execution']:.5f}%); this is a subset, not an additional cost category",
            "- Decision: hold arithmetic changes until one cumulative opcode/source attribution run supplies cost rankings; defer a wrapper requiring a new verifier relation.",
            f"- At B17, latest actual full / prefix were `{g5_c3budget['latest_full_attempt_gate']} / {g5_c3budget['latest_prefix_gate']}`; full and aggregate gas remained null",
            f"- Next gate: {g5_c3budget['next_gate']} (planned, not executed in B17)",
        ])
    if g5_c3attribution:
        attribution = g5_c3attribution
        fees = attribution["reconciliation"]
        lines.extend([
            "",
            "## G5-C3-B18 same-program failed-full-attempt cost attribution",
            "",
            f"- Status: `{attribution['status']}`; B16 reproduction: `{attribution['reproduction_matches_b16']}`",
            f"- Exactly `{attribution['evm_transactions_executed']}` transactions: one deployment and one unchanged full64 at 20M; no prefix/negative rerun",
            f"- Charged root opcodes + successful MODEXP children + terminal burn: `{fees['charged_root_opcode_gas']} + {fees['successful_modexp_child_gas']} + {fees['terminal_unexecuted_opcode_burn_gas']} = {fees['accounted_execution_gas']}`; difference `{fees['difference_gas']}`",
            f"- Root function mapping coverage: `{attribution['source_mapping']['mapped_function_percent_of_charged_root']:.2f}%`; unmapped/generated/scaffolding remain explicit",
            "- Exclusive root function ranking (not inclusive caller cost): " + ", ".join(f"`{item['name']}` {item['charged_gas']} gas ({item['percent_of_charged_root']:.2f}%)" for item in attribution["exclusive_solidity_function_ranking"][:5]),
            "- Opcode/function/leaf rankings are alternative partitions of the same root gas, not additive costs.",
            f"- At B18 full attempt / prefix: `{attribution['gate']} / {attribution['latest_prefix_gate']}`; full-loop and exact SnarkPack remained BLOCKED with null completed-loop/aggregate gas",
            f"- Next gate: {attribution['next_gate']}",
        ])
    if g5_c3route:
        route = g5_c3route
        bound = route["observed_rank_bound"]
        planned = route["planned_validation"]
        lines.extend([
            "",
            "## G5-C3-B19 saved-evidence/source/algebra route decision: no EVM run",
            "",
            f"- Status: `{route['status']}`; EVM transactions / arithmetic changes: `{route['evm_transactions_executed']} / {route['contract_changed']}`",
            f"- Source-owned rank bound: _mul `{bound['mul_source_owned_gas']}` > next mapped + all unassigned `{bound['largest_other_bucket_if_all_unassigned_added']}`; margin `{bound['mul_margin_over_that_bound']}`",
            "- At B19 one candidate was selected: fuse the two modular halvings in _mul into quarter(t) = (t + (t & 3)*p)/4 for this fixed p mod 4 = 3; B19 itself measured no implementation or gas improvement.",
            f"- Original halfMod source bucket is only `{bound['original_halfMod_percent_of_charged_root']:.4f}%` of charged root gas; no claim that this solves the full20M budget.",
            f"- At B19, B20 was planned, not executed: `{planned['planned_deployments']}` deployment, `{planned['planned_arithmetic_transactions']}` isolated Fp multiplies (one performance input plus four correction/carry cases). No Miller, Fp2 or extra matrix.",
            "- If exact results or isolated-window improvement fail, stop the candidate; prefix integration and any full retry require separate gates.",
            f"- At B19 latest actual full / prefix remained `{route['latest_full_attempt_gate']} / {route['latest_prefix_gate']}`; target status was `{route['target_verifier_status']}` with null full/aggregate gas.",
            f"- Next gate: {route['next_gate']}",
        ])
    if g5_c3quarter:
        quarter = g5_c3quarter
        comparison = quarter["comparison"]
        quarter_reduction = (
            f"{comparison['window_gas_reduction_percent']:.2f}%"
            if comparison["window_gas_reduction_percent"] is not None else "unmeasured"
        )
        lines.extend([
            "",
            "## G5-C3-B20 isolated fixed-p quarterMod multiplication comparison",
            "",
            f"- Status: `{quarter['status']}`; correctness/accounting: `{quarter['correctness_and_accounting_passed']}`; only _mul helper/calls and contract name differ from B6",
            f"- Actual EVM transactions / arithmetic calls / performance samples: `{quarter['evm_transactions_executed']} / {quarter['arithmetic_transactions']} / {quarter['performance_comparison_samples']}`; four correction/carry cases excluded from performance sample",
            f"- Same-input window B6 -> B20: `{comparison['baseline_window_gas']} -> {comparison['candidate_window_gas']}`; change `{quarter_reduction}` reduction",
            f"- Same-input execution / total gas B6 -> B20: `{comparison['baseline_execution_gas']} -> {comparison['candidate_execution_gas']}` / `{comparison['baseline_total_gas']} -> {comparison['candidate_total_gas']}`",
            f"- Allocator growth B6 -> B20: `{comparison['baseline_allocator_growth_bytes']} -> {comparison['candidate_allocator_growth_bytes']}` B; MODEXP count / calldata unchanged: `{comparison['modexp_count_unchanged']} / {comparison['calldata_unchanged']}`",
            f"- Runtime B6 -> B20: `{comparison['baseline_runtime_bytes']} -> {comparison['candidate_runtime_bytes']}` B; deployment gas `{comparison['baseline_deployment_gas']} -> {comparison['candidate_deployment_gas']}`",
            f"- Four boundaries verified / candidate accepted for a separate prefix gate: `{quarter['all_boundaries_verified']} / {quarter['candidate_accepted_for_separate_prefix_gate']}`",
            f"- No Fp2 or Miller executed at B20. At that gate full / passing prefix remained `{quarter['latest_full_attempt_gate']} / {quarter['latest_prefix_gate']}`; full/aggregate gas were null and target status BLOCKED.",
            f"- Next gate: {quarter['next_gate']}",
        ])
    if g5_c3quarterprefix:
        integrated = g5_c3quarterprefix
        comparison = integrated["comparison"]
        lines.extend([
            "",
            "## G5-C3-B21 exact quarterMod _mul integrated into the original Miller prefix",
            "",
            f"- Status: `{integrated['status']}`; all other B15 source bytes unchanged except contract name",
            f"- Exactly `{integrated['evm_transactions_executed']}` transactions: one deployment and the original prefix positive/wrong-target pair; full attempts `{integrated['full_loop_attempts']}`",
            f"- Positive total gas B15 -> B21: `{comparison['baseline_prefix_gas']} -> {comparison['optimized_prefix_gas']}` ({comparison['prefix_gas_reduction_percent']:.2f}% reduction)",
            f"- Active memory B15 -> B21: `{comparison['baseline_prefix_memory_words']} -> {comparison['optimized_prefix_memory_words']}` words; MODEXP calls `{comparison['baseline_prefix_modexp_calls']} -> {comparison['optimized_prefix_modexp_calls']}`",
            f"- Exact prefix / wrong target rejection / accounting: `{integrated['prefix_correctness_verified_in_evm']} / {integrated['wrong_prefix_target_rejected_in_evm']} / {integrated['prefix_conformance_and_accounting_passed']}`",
            f"- Runtime bytes B15 -> B21: `{comparison['baseline_runtime_bytes']} -> {comparison['optimized_runtime_bytes']}`; deployment gas `{comparison['baseline_deployment_gas']} -> {comparison['optimized_deployment_gas']}`",
            f"- At B21 target status was `{integrated['target_verifier_status']}`; latest actual full was historical `{integrated['latest_full_attempt_gate']}` on a DIFFERENT variant, not a measured B21 full failure",
            "- No full64, boundary rerun, final exponentiation or capability promotion; full/aggregate gas and completion percentage remain null.",
            f"- Next gate: {integrated['next_gate']}",
        ])
    if g5_c3fulldecision:
        decision = g5_c3fulldecision
        planned = decision["planned_validation"]
        lines.extend([
            "",
            "## G5-C3-B22 read-only full-conformance decision: zero EVM transactions",
            "",
            f"- Status: `{decision['status']}`; EVM transactions / source changes: `{decision['evm_transactions_executed']} / {decision['contract_changed']}`",
            "- Decision: one bounded full64 conformance attempt on the saved quarter runtime, then stop software arithmetic/full retries on failure; not a prediction that 20M will suffice.",
            "- Reason: B21 covers two steps, not the complete loop/arena use or terminal negative-x conjugation; B18 OOG is a different runtime.",
            f"- At B22, B23 was planned, NOT executed: `{planned['deployments']}` deployment + `{planned['full_positive_transactions']}` full positive; transaction/block cap `{planned['transaction_gas_limit']} / {planned['block_gas_limit']}`",
            f"- Planned success: exact `{planned['expected_return_payload_bytes']}` B full output and `{planned['expected_complete_modexp_calls_from_unchanged_formula']}` successful MODEXPs, zero failures and reconciled fees; only then report completed-loop gas.",
            "- OOG/other failure: save the actual outcome, retain null completed-loop gas and stop; do not relabel other failures as OOG or automatically increase the cap.",
            "- No prefix/boundary rerun, negative matrix, final exponentiation, proof generation or new profiling; full Miller/complete SnarkPack remain unverified.",
            f"- At B22 measured prefix / actual full remained `{decision['latest_prefix_gate']} / {decision['latest_full_attempt_gate']}`; target status was `{decision['target_verifier_status']}`",
            f"- Next gate: {decision['next_gate']}",
        ])
    if g5_c3quarterfull:
        full = g5_c3quarterfull
        attempt = full["full_attempt"]
        lines.extend([
            "",
            "## G5-C3-B23 same-runtime quarter full64 conformance attempt",
            "",
            f"- Status / classification: `{full['status']} / {full['result_class']}`",
            f"- Actual EVM transactions / full attempts: `{full['evm_transactions_executed']} / {full['full_loop_attempts']}`; no prefix or boundary rerun",
            f"- Complete full-loop verified / measured completed-loop gas: `{full['full_miller_loop_verified_in_evm']} / {full['full_miller_verification_gas']}`",
            f"- Fixed-budget software automatic optimization/retries frozen: `{full['software_route_frozen_by_failure']}`; this does not freeze unrelated project work",
        ])
        if attempt:
            lines.extend([
                f"- Receipt / actual exception / actual total gas: `{attempt['receipt_status']} / {attempt['exception']} / {attempt['total_gas']}`",
                f"- Execution + intrinsic = total: `{attempt['execution_gas']} + {attempt['intrinsic_gas']} = {attempt['total_gas']}`; refund `{attempt['gas_refund']}`",
                f"- Successful / failed MODEXP calls and child gas: `{attempt['modexp_success_count']} / {attempt['modexp_failure_count']} / {attempt['modexp_execution_gas']}`",
                f"- Active EVM memory at exit: `{attempt['evm_active_memory_words_at_exit']}` words; post-execution state `{attempt['root_post_execution_state']}`",
                "- Failed capped spend is not completed-loop gas; MODEXP counts are not a completion percentage. No source/step profile or wall-time speedup comparison is claimed.",
            ])
        else:
            lines.append(f"- Abort/host failure evidence: `{full.get('failure_phase')} / {full.get('host_error')}`; no completed full result")
        lines.extend([
            f"- Target status: `{full.get('target_verifier_status', 'UNAVAILABLE_ON_ABORT')}`; final exponentiation/GT binding/exact SnarkPack remain unverified",
            f"- Next step: {full['next_gate']}",
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

