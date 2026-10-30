#!/usr/bin/env python3
"""Validate raw measurements, bootstrap conditional CIs, create six scientific figures."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import sys

import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

BATCHES = [2, 4, 8, 16, 32]
OPS = ["individual_verify", "aggregation", "aggregate_verify"]


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def stats(values, rng):
    values = np.asarray(values, dtype=float)
    assert values.size == 30 and np.all(np.isfinite(values)) and np.all(values > 0)
    boot = np.median(values[rng.integers(0, len(values), (10000, len(values)))], axis=1)
    return dict(n=len(values), mean=float(values.mean()), median=float(np.median(values)),
                std=float(values.std(ddof=1)), min=float(values.min()), max=float(values.max()),
                median_ci95=[float(x) for x in np.quantile(boot, [0.025, 0.975])],
                p95=None, p95_reason="30 operation windows insufficient for a strong tail claim")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--raw", default="experiments/g1_e_raw")
    parser.add_argument("--out", default="results/g1_e")
    args = parser.parse_args()
    raw, out = Path(args.raw), Path(args.out)
    if out.exists():
        raise SystemExit("new analysis directory required; no silent overwrite")
    manifest = json.loads((raw / "manifest.json").read_text())
    assert manifest["git_dirty"] is False and manifest["tracked_source_clean_at_end"]
    for item in manifest["source_hashes"]:
        assert digest(item["path"]) == item["sha256"], item["path"]
    for prefix in ["raw", "fixture", "manifest"]:
        assert digest(manifest["provenance"][prefix]) == manifest["provenance"][prefix + "_sha256"]
    assert manifest["new_groth16_proof_count"] == 0
    cases = {c["batch_size"]: c for c in manifest["cases"]}
    assert sorted(cases) == BATCHES
    assert all(c["positive"] and c["last_nullifier_negative_rejected"] and c["independent_fold_matches"] for c in cases.values())
    warmup = json.loads((raw / "warmup.json").read_text())
    assert len(warmup) == 25
    samples, groups = [], {b: [] for b in BATCHES}
    for batch in BATCHES:
        paths = sorted((raw / f"B{batch}").glob("trial-*.json"))
        assert len(paths) == 30
        for index, path in enumerate(paths, 1):
            trial = json.loads(path.read_text())
            assert trial["round"] == index and trial["batch_size"] == batch
            assert trial["git_commit"] == manifest["git_commit"]
            assert trial["aggregate_proof_sha256"] == cases[batch]["aggregate_proof_sha256"]
            assert sorted(trial["operation_order"]) == sorted(OPS)
            row = dict(batch_size=batch, round=index, batch_order="/".join(map(str, trial["batch_order"])),
                       operation_order="/".join(trial["operation_order"]), source_json=str(path),
                       source_sha256=digest(path))
            for sample in trial["samples"]:
                op = sample["operation"]
                assert sample["success"] and sample["iterations"] > 0
                assert np.isclose(sample["elapsed_ms"] / sample["iterations"], sample["ms_per_operation"], rtol=1e-12)
                row[op + "_ms"] = sample["ms_per_operation"]
                row[op + "_k"] = sample["iterations"]
                row[op + "_window_ms"] = sample["elapsed_ms"]
            row["verifier_speed_ratio"] = row["individual_verify_ms"] / row["aggregate_verify_ms"]
            row["aggregator_plus_verifier_ms"] = row["aggregation_ms"] + row["aggregate_verify_ms"]
            samples.append(row)
            groups[batch].append(row)
    assert len(samples) == 150
    memory = [json.loads(p.read_text()) for p in sorted((raw / "memory").glob("*.json"))]
    assert len(memory) == 45 and len({m["pid"] for m in memory}) == 45
    for m in memory:
        assert m["git_commit"] == manifest["git_commit"] and m["tracked_source_clean"]
        assert m["rss_sample_count"] > 0 and m["phase_peak_is_sampled_lower_bound"]
        assert m["sampled_phase_peak_rss_mib"] >= m["before_rss_mib"]
    rng = np.random.default_rng(20261030)
    summary = {}
    summary_rows = []
    for batch in BATCHES:
        timing = {op: stats([r[op + "_ms"] for r in groups[batch]], rng) for op in OPS}
        timing["verifier_speed_ratio"] = stats([r["verifier_speed_ratio"] for r in groups[batch]], rng)
        timing["aggregator_plus_verifier"] = stats([r["aggregator_plus_verifier_ms"] for r in groups[batch]], rng)
        mem = {}
        for op in OPS:
            subset = [m for m in memory if m["batch_size"] == batch and m["operation"] == op]
            assert sorted(m["repeat"] for m in subset) == [1, 2, 3]
            mem[op] = {key: dict(median=float(np.median([m[key] for m in subset])),
                                min=min(m[key] for m in subset), max=max(m[key] for m in subset))
                       for key in ["pool_loaded_rss_mib", "before_rss_mib", "sampled_phase_peak_rss_mib",
                                   "sampled_phase_delta_mib", "cumulative_process_peak_after_mib"]}
        summary[str(batch)] = dict(timing=timing, memory=mem, sizes=cases[batch])
        flat = dict(batch_size=batch, samples=30)
        for metric, values in timing.items():
            for key in ["mean", "median", "std", "min", "max"]:
                flat[metric + "_" + key] = values[key]
            flat[metric + "_median_ci95_low"], flat[metric + "_median_ci95_high"] = values["median_ci95"]
            flat[metric + "_p95"] = None
        flat.update({key: cases[batch][key] for key in ["individual_proof_bytes", "aggregate_proof_bytes", "matrix_bytes", "transcript_bytes", "individual_plus_matrix_bytes", "aggregate_plus_matrix_bytes"]})
        summary_rows.append(flat)
    out.mkdir(parents=True)
    report = dict(gate="G1-E", status="PASS_150_TIMING_TRIALS_45_MEMORY_PROCESSES", source_git_commit=manifest["git_commit"],
                  analysis=dict(python=sys.version, platform=platform.platform(), numpy=np.__version__, matplotlib=matplotlib.__version__,
                                bootstrap_draws=10000, seed=20261030, ci_scope="conditional percentile-bootstrap median CI; serial operation windows, not independent elections"),
                  measurement_unit="repeated operations on fixed archived proofs, zero new Groth16 proofs", batches=summary,
                  timing_trials=150, operation_windows=450, memory_processes=45, warmup_trials_excluded=25,
                  gas=None, full_da_bytes=None, full_calldata_bytes=None, source_manifest_sha256=digest(raw / "manifest.json"))
    (out / "summary.json").write_text(json.dumps(report, indent=2) + "\n")
    (out / "csv_tables.json").write_text(json.dumps(dict(raw_samples=samples, summary=summary_rows), indent=2) + "\n")
    figures = out / "figures"
    figures.mkdir()
    plt.rcParams.update({"font.family": "DejaVu Sans", "font.size": 10, "axes.spines.top": False,
                         "axes.spines.right": False, "pdf.fonttype": 42, "svg.fonttype": "none"})
    x = np.arange(5)
    def axes(title, ylabel):
        fig, ax = plt.subplots(figsize=(6.8, 4.5), layout="constrained")
        ax.set(title=title, xlabel="Batch size B", ylabel=ylabel)
        ax.set_xticks(x, BATCHES)
        ax.grid(axis="y", alpha=0.2)
        return fig, ax
    def save(fig, name):
        for ext in ["png", "pdf", "svg"]:
            fig.savefig(figures / f"{name}.{ext}", dpi=200)
        plt.close(fig)
    def series(ax, metric, label, divisor=1):
        s = [summary[str(b)]["timing"][metric] for b in BATCHES]
        div = np.array(BATCHES) if divisor == "B" else divisor
        med = np.array([v["median"] for v in s]) / div
        low = np.array([v["median_ci95"][0] for v in s]) / div
        high = np.array([v["median_ci95"][1] for v in s]) / div
        ax.errorbar(x, med, yerr=[np.maximum(0, med-low), np.maximum(0, high-med)], marker="o", capsize=4, label=label)
    fig, ax = axes("Native verification time (median and 95% bootstrap CI)", "Time per batch (ms)")
    series(ax, "individual_verify", "Individual Groth16"); series(ax, "aggregate_verify", "SnarkPack V2"); ax.legend(); save(fig, "fig1_verification")
    fig, ax = axes("Native verification cost per vote", "Time per vote (ms)")
    series(ax, "individual_verify", "Individual Groth16", "B"); series(ax, "aggregate_verify", "SnarkPack V2", "B"); ax.legend(); save(fig, "fig2_per_vote")
    fig, ax = axes("Verifier speed ratio on the fixed proof pool", "Individual / aggregate verification")
    series(ax, "verifier_speed_ratio", "Paired-window ratio"); ax.axhline(1, color="gray", linestyle="--", label="Equal verifier time"); ax.legend(); save(fig, "fig3_speed_ratio")
    fig, axs = plt.subplots(1, 2, figsize=(10, 4.5), layout="constrained")
    for ax, extra, title in zip(axs, [False, True], ["Proof bytes", "Proof + one input matrix"]):
        i = [cases[b]["individual_plus_matrix_bytes" if extra else "individual_proof_bytes"] for b in BATCHES]
        a = [cases[b]["aggregate_plus_matrix_bytes" if extra else "aggregate_proof_bytes"] for b in BATCHES]
        ax.bar(x-.18, i, .36, label="Individual"); ax.bar(x+.18, a, .36, label="Aggregate")
        ax.set(title=title, xlabel="Batch size B", ylabel="Bytes"); ax.set_xticks(x, BATCHES); ax.legend(); ax.grid(axis="y", alpha=.2)
    save(fig, "fig4_proof_communication")
    fig, axs = plt.subplots(1, 2, figsize=(10, 4.5), layout="constrained")
    for op, label in zip(OPS, ["Individual", "Aggregation", "Aggregate verify"]):
        for ax, key in zip(axs, ["sampled_phase_peak_rss_mib", "sampled_phase_delta_mib"]):
            vals = [summary[str(b)]["memory"][op][key] for b in BATCHES]
            med = np.array([v["median"] for v in vals]); lo = np.array([v["min"] for v in vals]); hi = np.array([v["max"] for v in vals])
            ax.errorbar(x, med, yerr=[med-lo, hi-med], marker="o", capsize=3, label=label)
    for ax, title in zip(axs, ["Sampled phase RSS (absolute)", "Sampled RSS above pre-operation baseline"]):
        ax.set(title=title, xlabel="Batch size B", ylabel="MiB"); ax.set_xticks(x, BATCHES); ax.legend(fontsize=8); ax.grid(axis="y", alpha=.2)
    fig.suptitle("3 isolated processes per B/operation; bars = min-max; 1 ms sampling")
    save(fig, "fig5_memory")
    fig, axs = plt.subplots(1, 2, figsize=(10, 4.5), layout="constrained")
    series(axs[0], "aggregation", "Aggregation"); series(axs[0], "aggregator_plus_verifier", "Aggregation + verification")
    series(axs[1], "aggregation", "Aggregation per vote", "B")
    for ax, label in zip(axs, ["Time per batch (ms)", "Time per vote (ms)"]):
        ax.set(xlabel="Batch size B", ylabel=label); ax.set_xticks(x, BATCHES); ax.legend(); ax.grid(axis="y", alpha=.2)
    fig.suptitle("Off-chain aggregation overhead (not EVM Gas)"); save(fig, "fig6_aggregation")
    lines = ["# G1-E measured statistics", "", "30 serial operation windows per B; 5 warmups excluded. 10,000-draw percentile-bootstrap median CIs are conditional on this fixed pool and machine, not independent-election uncertainty. P95 is deliberately unavailable at n=30.", "", "| B | Individual median ms | Aggregate verify median ms | Aggregation median ms | Paired speed ratio (95% CI) |", "|---|---:|---:|---:|---:|"]
    for b in BATCHES:
        t=summary[str(b)]["timing"]; ratio=t["verifier_speed_ratio"]
        lines.append(f"| {b} | {t['individual_verify']['median']:.6f} | {t['aggregate_verify']['median']:.6f} | {t['aggregation']['median']:.6f} | {ratio['median']:.3f} [{ratio['median_ci95'][0]:.3f}, {ratio['median_ci95'][1]:.3f}] |")
    lines += ["", "Memory: each B/operation has 3 fresh processes. Stage peaks are 1 ms sampled lower bounds including baseline, not exact allocator peaks; phase deltas are relative to pre-operation RSS. Cumulative process peaks are separately preserved. Timings exclude RSS sampling.", "", "No new ballot proofs or Groth16 Setup; no current-suite state/P3/settlement/EVM experiment. Aggregate proofs remain larger; proof+matrix excludes cached VK/SRS and full framing. Gas/DA/calldata remain null.", ""]
    (out / "statistics.md").write_text("\n".join(lines))
    print("Validated 150 timing trials, 450 windows and 45 memory processes; wrote statistics and 18 figure files.")


if __name__ == "__main__":
    main()
