#!/usr/bin/env python3
"""Read-only reuse of frozen experiments; create two preliminary state figures.

No Rust benchmark, setup, proof, verifier, network request or source CSV write.
G1-E's statistical figures remain byte-for-byte unchanged.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import platform
import subprocess

ROOT = Path(__file__).resolve().parents[1]
PINS = {
    "results/g1_e/summary.json": "e413aab87605db515973a1acb936e36da7f0a253892935cf5c3d92d4ba24bd34",
    "results/g1_e/summary.csv": "5c71e2d6dfc5f04c9defb16bca126dfcf9e04ec3e4191b6670e0a6aad9c14fb0",
    "results/g1_e/raw_samples.csv": "548e17b96de2c537aba8353781359b314ed7442d91e2121ef8ad27f8a3158f53",
    "experiments/g2_d_raw/state_transition_d5w32_b4_groth16.json": "f2d965d386b7e902b4467a94e580ca4f5b19233807097aaa6991961f687d96b0",
    "experiments/g2_e_raw/multibatch_state_b4.json": "01582f079b95d317c728cca2a62c29939a03fd8425e05edd564c721ba9acda51",
}
FIGURES = ["fig1_verification", "fig2_per_vote", "fig3_speed_ratio",
           "fig4_proof_communication", "fig5_memory", "fig6_aggregation"]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def close(a, b):
    require(math.isclose(a, b, rel_tol=1e-12, abs_tol=1e-9), f"Mismatch: {a} != {b}")


def archived_artifact_audit():
    """Check immutable data, not later-edited Cargo/source/status documents."""
    counts = {}
    for gate in ["g1_e", "g2_d", "g2_e", "g4_d"]:
        count = 0
        for line in (ROOT / f"experiments/{gate}_raw/SHA256SUMS").read_text().splitlines():
            expected, name = line.split(maxsplit=1)
            name = name.lstrip("*")
            if not name.startswith(("experiments/", "fixtures/", "results/")):
                continue
            target = (ROOT / name).resolve()
            require(target.is_relative_to(ROOT), "Unsafe archive path")
            require(sha(target) == expected, f"Archive changed: {name}")
            count += 1
        require(count > 0, f"Empty archive: {gate}")
        counts[gate] = count
    return counts


def load_data():
    for name, expected in PINS.items():
        require(sha(ROOT / name) == expected, f"Frozen input changed: {name}")
    g1 = json.loads((ROOT / "results/g1_e/summary.json").read_text())
    d = json.loads((ROOT / "experiments/g2_d_raw/state_transition_d5w32_b4_groth16.json").read_text())
    e = json.loads((ROOT / "experiments/g2_e_raw/multibatch_state_b4.json").read_text())
    require(g1["timing_trials"] == 150 and g1["operation_windows"] == 450, "Wrong G1-E campaign")
    require(sorted(map(int, g1["batches"])) == [2, 4, 8, 16, 32], "Wrong G1-E scales")
    require(d["source_suite"] == e["source_suite"] == "D5/W32/G1-D2", "Mixed suites")
    require(d["frozen_ballot_manifest_sha256"] == e["frozen_ballot_manifest_sha256"], "Mixed manifests")
    require(e["state_vk_matches_g2d_byte_for_byte"], "State VK mismatch")
    require(d["shape"]["matrix_sha256"] == e["nonzero_old_state_shape"]["matrix_sha256"], "State relation mismatch")
    require(e["second_old_equals_first_new"] and e["second_old_state_is_nonzero"], "Missing continuity")
    require(e["final_equals_independent_eight_ballot_fold"] and e["all_both_verifiers_pass"], "Missing acceptance")
    require(e["total_distinct_ballots"] == e["spent_nullifier_count"] == 8, "Wrong ballot count")
    common = d["source_aggregate_verify_ms"] + d["common_binding_checks_ms"]
    direct = common + d["direct_fold_check_ms"]
    state = common + d["verify_ms"]
    close(direct, d["comparison"]["baseline_shared_plus_binding_plus_direct_fold_ms"])
    close(state, d["comparison"]["proposed_shared_plus_binding_plus_state_verify_ms"])
    for batch in e["batches"]:
        require(batch["verified_and_applied"], "Unaccepted transition")
        require(batch["receiver_check_verify_apply_elapsed_ms"] >= batch["snarkpack_verify_ms"] + batch["state_verify_ms"], "Inconsistent receiver timing")
    return g1, d, e, {"snarkpack_ms": d["source_aggregate_verify_ms"],
        "common_binding_ms": d["common_binding_checks_ms"],
        "direct_fold_check_ms": d["direct_fold_check_ms"], "state_verify_ms": d["verify_ms"],
        "direct_component_sum_ms": direct, "state_component_sum_ms": state,
        "separately_timed_end_to_end": False}


def make_figures(out, d, e, components):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    matplotlib.rcParams.update({"font.family": "DejaVu Sans", "font.size": 10,
        "axes.spines.top": False, "axes.spines.right": False,
        "pdf.fonttype": 42, "svg.fonttype": "none", "savefig.dpi": 180})
    blue, gray, orange, green = "#356A9A", "#A8B3BF", "#C97832", "#258475"

    def save(fig, name):
        for ext in ["pdf", "svg", "png"]:
            kw = {"metadata": {"CreationDate": None, "ModDate": None}} if ext == "pdf" else {}
            fig.savefig(out / f"{name}.{ext}", **kw)
        plt.close(fig)

    fig, (ax, detail) = plt.subplots(1, 2, figsize=(11, 5.4), gridspec_kw={"width_ratios": [1.45, 1]})
    fig.subplots_adjust(left=.08, right=.96, top=.80, bottom=.24, wspace=.40)
    fig.suptitle("G2-D: matched B=4 native verification components", fontsize=14, y=.96)
    fig.text(.5, .88, "D5/W32 - same ballots, roots and old/new state - one functional sample", ha="center", fontsize=10)
    common = components["snarkpack_ms"] + components["common_binding_ms"]
    ax.bar([0, 1], [components["snarkpack_ms"]] * 2, color=blue, width=.56, label="Shared SnarkPack Verify")
    ax.bar([0, 1], [components["common_binding_ms"]] * 2,
           bottom=components["snarkpack_ms"], color=gray, width=.56, label="Shared matrix/root binding")
    ax.bar(0, components["direct_fold_check_ms"], bottom=common, width=.56, color=green, label="Direct Fold check")
    ax.bar(1, components["state_verify_ms"], bottom=common, width=.56, color=orange, label="State proof Verify")
    totals = [components["direct_component_sum_ms"], components["state_component_sum_ms"]]
    for x, y in enumerate(totals):
        ax.text(x, y + .22, f"{y:.3f} ms", ha="center", fontsize=11)
    ax.set(xticks=[0, 1], xticklabels=["SnarkPack +\nDirect Fold", "SnarkPack +\nState Proof"],
           ylabel="Sum of measured components (ms)", ylim=(0, max(totals) * 1.18))
    ax.set_axisbelow(True)
    ax.grid(axis="y", alpha=.18)
    ax.legend(loc="upper left", fontsize=8, frameon=False)
    detail.scatter([0, 1], [d["direct_fold_check_ms"], d["verify_ms"]], color=[green, orange], s=65, zorder=3)
    detail.set(yscale="log", xticks=[0, 1], xticklabels=["Direct Fold\ncheck", "State proof\nVerify"],
               ylabel="Incremental check (ms, log scale)", ylim=(.001, 10), xlim=(-.5, 1.5))
    for x, y in enumerate([d["direct_fold_check_ms"], d["verify_ms"]]):
        detail.annotate(f"{y:.6f}", (x, y), xytext=(0, 12), textcoords="offset points", ha="center")
    detail.grid(axis="y", which="both", alpha=.18)
    fig.text(.08, .10, f"State Setup: {d['setup_ms']/1000:.3f} s; Prove: {d['prove_ms']/1000:.3f} s (separate, not in verification bars).", fontsize=9)
    fig.text(.08, .055, "Component sums, not separately timed end-to-end transactions. No error bars or statistical speed claim.", fontsize=9)
    save(fig, "g2d_matched_verification")

    batches = e["batches"]
    fig, (ax, detail) = plt.subplots(1, 2, figsize=(11, 5.4), gridspec_kw={"width_ratios": [1.1, 1.3]})
    fig.subplots_adjust(left=.08, right=.96, top=.80, bottom=.24, wspace=.42)
    fig.suptitle("G2-E: two consecutive B=4 state transitions", fontsize=14, y=.96)
    fig.text(.5, .88, "S0 -> S1 -> S2; eight distinct ballots; same state VK; functional samples only", ha="center", fontsize=10)
    elapsed = [b["receiver_check_verify_apply_elapsed_ms"] for b in batches]
    ax.bar([0, 1], elapsed, color=blue, width=.55)
    for x, y in enumerate(elapsed):
        ax.text(x, y + .25, f"{y:.3f} ms", ha="center")
    ax.set(xticks=[0, 1], xticklabels=["Batch 1\nS0 -> S1", "Batch 2\nS1 -> S2"],
           ylabel="Actual state-proof receiver elapsed (ms)", ylim=(0, max(elapsed) * 1.18))
    ax.grid(axis="y", alpha=.18)
    ax.set_axisbelow(True)
    for values, color, label, offset in [
        ([b["independent_direct_fold_check_ms"] for b in batches], green, "Independent Direct Fold check", -.07),
        ([b["state_verify_ms"] for b in batches], orange, "State proof Verify", .07)]:
        xs = [x + offset for x in [0, 1]]
        detail.scatter(xs, values, color=color, s=55, label=label, zorder=3)
        for x, y in zip(xs, values):
            detail.annotate(f"{y:.6f}", (x, y), xytext=(0, 10), textcoords="offset points", ha="center", fontsize=9)
    detail.set(yscale="log", xticks=[0, 1], xticklabels=["Batch 1", "Batch 2"],
        ylabel="Isolated check (ms, log scale)", ylim=(.001, 30), xlim=(-.5, 1.5))
    detail.legend(loc="upper left", frameon=False, fontsize=8)
    detail.grid(axis="y", which="both", alpha=.18)
    fig.text(.08, .10, "Left: check + both real verifiers + apply. Right: isolated checks, NOT full alternative receiver paths.", fontsize=9)
    fig.text(.08, .055, "No matched full Direct Fold receiver measurement exists; do not infer it by subtracting or adding timings.", fontsize=9)
    save(fig, "g2e_consecutive_transitions")
    return matplotlib.__version__


def report(g1, d, e, c):
    lines = ["# 第一阶段：已有实测数据对比", "",
        "这是对归档数据的只读分析；没有新执行 Setup、Prove、重型验证或性能采样。", "",
        "## G1-E：复用原六组统计图", "",
        "M4 / D5 / W32，同一证明池；每规模 30 个串行操作样本，非 30 次独立选举。",
        "比例是逐轮配对比值的中位数，不是两组中位数相除。区间为原条件 bootstrap 区间。", "",
        "| B | 逐票 Verify ms | 聚合 Verify ms | 聚合生成 ms | 配对验证比 [95% CI] | 原证明 bytes | 聚合证明 bytes |",
        "|---|---:|---:|---:|---|---:|---:|"]
    for b in [2, 4, 8, 16, 32]:
        x = g1["batches"][str(b)]
        t, sizes = x["timing"], x["sizes"]
        r = t["verifier_speed_ratio"]
        lo, hi = r["median_ci95"]
        lines.append(f"| {b} | {t['individual_verify']['median']:.6f} | {t['aggregate_verify']['median']:.6f} | {t['aggregation']['median']:.6f} | {r['median']:.3f} [{lo:.3f}, {hi:.3f}] | {sizes['individual_proof_bytes']} | {sizes['aggregate_proof_bytes']} |")
    b32 = g1["batches"]["32"]
    lines += ["", "B16/B32 在本机固定池有验证者收益；不代表总计算、通信或链上收益。",
        f"B32 聚合+验证中位数 {b32['timing']['aggregator_plus_verifier']['median']:.3f} ms；原证明+一份公开矩阵 {b32['sizes']['individual_plus_matrix_bytes']:,} B，聚合+同一矩阵 {b32['sizes']['aggregate_plus_matrix_bytes']:,} B。",
        "矩阵已经包含密文；可导出的 transcript 不再算成必传的第二份数据。",
        "内存图是预热后操作阶段的 1 ms 采样峰值（3 进程 min/max），不可与 G2 的含 Setup/Prove 累计 RSS 排名。", ""]
    for name in FIGURES:
        lines.append(f"- [原统计图：{name}](../g1_e/figures/{name}.pdf)（PDF；同目录有 PNG/SVG，原字节保留）")
    lines += ["", "## G2-D：同一批 B4 的匹配组件口径", "",
        "![G2-D](g2d_matched_verification.png)", "",
        "| 组件 | ms |", "|---|---:|",
        f"| 两路相同的 SnarkPack Verify | {c['snarkpack_ms']:.6f} |",
        f"| 两路相同的矩阵/根绑定 | {c['common_binding_ms']:.6f} |",
        f"| Direct Fold 增量检查 | {c['direct_fold_check_ms']:.6f} |",
        f"| State Proof Verify | {c['state_verify_ms']:.6f} |",
        f"| Direct Fold 路线组件和 | {c['direct_component_sum_ms']:.6f} |",
        f"| State Proof 路线组件和 | {c['state_component_sum_ms']:.6f} |", "",
        "组件和不是单独计时的端到端事务；这是一个功能样本，没有置信区间。Direct Fold 是有效的正确性基线。",
        f"State Setup {d['setup_ms']/1000:.6f} s / Prove {d['prove_ms']/1000:.6f} s，单独记账。",
        f"State proof {d['proof_bytes']} B / public inputs {d['state_public_input_bytes']} B；不能把两者之和当完整通信量。", "",
        "## G2-E：连续状态转换，不伪造完整替代基线", "",
        "![G2-E](g2e_consecutive_transitions.png)", "",
        "| 批次 | SP Verify ms | State Verify ms | 实际接收检查/验证/应用 ms | 独立直接累计检查 ms |",
        "|---|---:|---:|---:|---:|"]
    for b in e["batches"]:
        lines.append(f"| {b['batch_index']} | {b['snarkpack_verify_ms']:.6f} | {b['state_verify_ms']:.6f} | {b['receiver_check_verify_apply_elapsed_ms']:.6f} | {b['independent_direct_fold_check_ms']:.6f} |")
    lines += ["", "后两列不是两条完整路线的公平计时，不能相除作为系统加速比。没有额外测量共同接收开销，故不补算 Direct Fold 完整接收时间。",
        "两批各四票，第二批旧状态非零；最终密文等于独立累计八票。不是 B8 状态电路。",
        f"重建 Setup {e['setup_rebuild_ms']/1000:.6f} s；只新增第二批 State Prove {e['new_second_prove_ms']/1000:.6f} s。",
        "复用首批状态证明和八份选票证明；不能声称本轮新生成所有证明。", "",
        "## 暂不生成的对比", "",
        "- Full Tally vs Winner-only：等待同一冻结目标的完整原生状态机。",
        "- 完整链上 Gas、calldata、端到端系统图：等待正确新 VK 的真实结算路线。缺失为 null，不是 0。",
        "- 跨论文性能排名、额外规模、重复 G1-E、底层 Miller 优化：本轮不做。", "",
        "G4-D 已有同一新电路/VK 的十种 Trustee 子集真实证明，不重做；下一实现仍是完整原生 Winner-only 连接。", ""]
    return "\n".join(lines)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--out", type=Path, default=Path("results/comparison_stage1"))
    p.add_argument("--check-only", action="store_true")
    args = p.parse_args()
    counts = archived_artifact_audit()
    g1, d, e, c = load_data()
    if args.check_only:
        print(json.dumps({"status": "PASS_FROZEN_INPUTS_AND_SCOPE", "archive_artifacts": counts, "g2d_components": c}, indent=2))
        return
    out = args.out.resolve()
    require(not out.exists(), f"Refuse to overwrite archived outputs: {out}")
    out.mkdir(parents=True)
    version = make_figures(out, d, e, c)
    inputs = dict(PINS)
    for name in FIGURES:
        for ext in ["pdf", "png", "svg"]:
            path = f"results/g1_e/figures/{name}.{ext}"
            inputs[path] = sha(ROOT / path)
    for path in ["experiments/g4_d_raw/ten_subsets_p3_v2.json", "scripts/comparison_stage1.py",
                 "scripts/comparison_stage1_csv_check.mjs", "docs/COMPARISON_STAGE1.md",
                 "docs/LITERATURE_COMPARISON.md", "docs/tables/comparison_stage1.tex",
                 "docs/tables/comparison_stage1.bib"]:
        inputs[path] = sha(ROOT / path)
    data = {"kind": "read-only archive analysis, not a new experimental gate", "analysis_base_git_sha": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "analysis_source_dirty": bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True)),
        "input_sha256": inputs, "archived_data_verified": counts,
        "python": platform.python_version(), "matplotlib": version,
        "g1e_original_statistics_reused": True, "g1e_measurement_git_sha": g1["source_git_commit"],
        "g2d_measurement_git_sha": d["git_commit"], "g2d": c,
        "g2e_measurement_git_sha": e["git_commit"], "g2e_measurement_dirty": e["git_dirty"],
        "g2e_batches": [{k: b[k] for k in ["batch_index", "snarkpack_verify_ms", "state_verify_ms", "receiver_check_verify_apply_elapsed_ms", "independent_direct_fold_check_ms"]} for b in e["batches"]],
        "new_benchmark_runs": 0, "new_setup_runs": 0, "new_proofs": 0,
        "whole_direct_fold_receiver_ms": None, "complete_system_gas": None,
        "scope": ["G1-E original fixed-pool serial operation statistics; no resampling or altered conclusions",
                  "G2-D matched component sums, not end-to-end transaction measurements",
                  "G2-E two functional transition samples; isolated checks are not full alternative routes",
                  "No cross-paper performance ranking, no production setup or privacy theorem claim"]}
    (out / "comparison_data.json").write_text(json.dumps(data, indent=2) + "\n")
    (out / "summary.md").write_text(report(g1, d, e, c))
    files = sorted(x for x in out.iterdir() if x.is_file())
    (out / "SHA256SUMS").write_text("".join(f"{sha(x)}  {x.name}\n" for x in files))
    print(json.dumps({"status": "PASS_COMPARISON_STAGE1", "outputs": str(out), "archive_artifacts": counts, "new_experimental_runs": 0}, indent=2))


if __name__ == "__main__":
    main()
