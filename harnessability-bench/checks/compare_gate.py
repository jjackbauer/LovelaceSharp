#!/usr/bin/env python3
"""Compare a gate run against the frozen baseline and evaluate the plan's hypotheses.

Reads two trajectory files, re-verifies every candidate against the hidden probes with the
controller's own verifier, and prints the per-mode delta plus an explicit PASS/FAIL per hypothesis
from API-ENHANCEMENT-PLAN.md. A falsified hypothesis is reported, never smoothed over.
"""
import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
BENCH = HERE.parent
CTRL = BENCH / "lovelace-harnessability-public-branch"
RUNNER = BENCH / "runner" / "Lovelace.Run.exe"
ORDER = ["L0", "L1", "L2", "L3"]
sys.path.insert(0, str(CTRL))
import bench as ctl  # noqa: E402

HYPOTHESES = {
    "H2": "H_10(L1) >= 0.80",
    "H3": "H_10(L2) >= 0.88",
    "H4": "H_10(L3) - H_10(L2) <= 0.10",
    "H5": "L1 false convergences <= 1",
    "H6": "|H_10(L0) - baseline| <= 0.03",
    "H7": "no mode loses a solve, and unknown-name failures fall",
}


def load(path):
    return [json.loads(l) for l in Path(path).read_text(encoding="utf-8").splitlines() if l.strip()]


def solve_steps(rows, oracles, budget):
    """Per (task, mode): the first evaluation that passes every hidden probe."""
    out = {}
    for r in rows:
        step = None
        for i, e in enumerate([x for x in r["events"] if x["type"] == "candidate"], 1):
            ok, _ = ctl.verify_script(str(RUNNER), r["task_id"], e["script"], oracles)
            if ok:
                step = i
                break
        out[(r["task_id"], r["mode"])] = step
    return out


def curve(steps, n, budget):
    return [sum(1 for s in steps if s is not None and s <= k) / n for k in range(1, budget + 1)]


def h_b(steps, budget):
    n = len(steps)
    return sum(curve(steps, n, budget)) / budget if n else 0.0


def unknown_failures(rows):
    total = 0
    for r in rows:
        for e in r["events"]:
            if e["type"] != "observation" or e["observation"].get("ok") is not False:
                continue
            for d in e["observation"].get("diagnostics") or []:
                text = d.get("message") if isinstance(d, dict) else d
                if text and "Unknown function" in text:
                    total += 1
    return total


def false_convergences(rows, oracles):
    bad = 0
    for r in rows:
        if not r["agent_claimed_done"]:
            continue
        ok, _ = ctl.verify_script(str(RUNNER), r["task_id"], r["final_script"] or "", oracles)
        if not ok:
            bad += 1
    return bad


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--baseline", default=str(BENCH / "runs" / "baseline" / "runs-matrix.jsonl"))
    ap.add_argument("--gate", required=True)
    ap.add_argument("--budget", type=int, default=10)
    args = ap.parse_args()

    oracles = json.loads((CTRL / "private" / "oracles.json").read_text(encoding="utf-8"))
    base_rows, gate_rows = load(args.baseline), load(args.gate)
    base_steps, gate_steps = solve_steps(base_rows, oracles, args.budget), solve_steps(gate_rows, oracles, args.budget)
    tasks = []
    for r in gate_rows:
        if r["task_id"] not in tasks:
            tasks.append(r["task_id"])

    print("## Gate comparison (%d runs each)" % len(gate_rows))
    print()
    print("| Mode | baseline solved | gate solved | baseline H_%d | gate H_%d | delta |" % (args.budget, args.budget))
    print("|---|---:|---:|---:|---:|---:|")
    hb = {}
    for mode in ORDER:
        bs = [s for (t, m), s in base_steps.items() if m == mode]
        gs = [s for (t, m), s in gate_steps.items() if m == mode]
        if not bs and not gs:
            continue
        b_h, g_h = h_b(bs, args.budget), h_b(gs, args.budget)
        hb[mode] = (b_h, g_h)
        print("| %s | %.0f%% | %.0f%% | %.3f | %.3f | %+.3f |" % (
            mode, 100 * sum(s is not None for s in bs) / len(bs),
            100 * sum(s is not None for s in gs) / len(gs), b_h, g_h, g_h - b_h))
    print()

    print("Per task (baseline -> gate), first evaluation that passes every hidden probe:")
    print()
    print("| Task | " + " | ".join(ORDER) + " |")
    print("|---|" + "---|" * len(ORDER))
    lost = []
    for t in tasks:
        cells = []
        for mode in ORDER:
            b, g = base_steps.get((t, mode)), gate_steps.get((t, mode))
            cells.append("%s->%s" % ("-" if b is None else b, "-" if g is None else g))
            if b is not None and g is None:
                lost.append((t, mode))
        print("| %s | %s |" % (t, " | ".join(cells)))
    print()

    base_unknown, gate_unknown = unknown_failures(base_rows), unknown_failures(gate_rows)
    base_fc = {m: false_convergences([r for r in base_rows if r["mode"] == m], oracles) for m in ORDER}
    gate_fc = {m: false_convergences([r for r in gate_rows if r["mode"] == m], oracles) for m in ORDER}

    results = []
    l1 = gate_steps.get(("undefined-name", "L1")), gate_steps.get(("matrix-inverse", "L1"))
    ok_h1 = all(s is not None and s <= 3 for s in l1)
    results.append(("H1", "L1 solves undefined-name and matrix-inverse in <= 3 evaluations",
                    ok_h1, "measured %s" % (l1,)))
    results.append(("H2", HYPOTHESES["H2"], hb.get("L1", (0, 0))[1] >= 0.80, "measured %.3f" % hb.get("L1", (0, 0))[1]))
    results.append(("H3", HYPOTHESES["H3"], hb.get("L2", (0, 0))[1] >= 0.88, "measured %.3f" % hb.get("L2", (0, 0))[1]))
    gap = hb.get("L3", (0, 0))[1] - hb.get("L2", (0, 0))[1]
    results.append(("H4", HYPOTHESES["H4"], gap <= 0.10, "measured gap %.3f" % gap))
    results.append(("H5", HYPOTHESES["H5"], gate_fc["L1"] <= 1,
                    "measured %d (baseline %d)" % (gate_fc["L1"], base_fc["L1"])))
    control = abs(hb.get("L0", (0, 0))[1] - hb.get("L0", (0, 0))[0])
    results.append(("H6", HYPOTHESES["H6"], control <= 0.03, "measured drift %.3f" % control))
    results.append(("H7", HYPOTHESES["H7"], not lost and gate_unknown < base_unknown,
                    "lost solves %s; unknown-name failures %d -> %d" % (lost or "none", base_unknown, gate_unknown)))

    print("| # | Hypothesis | Verdict | Evidence |")
    print("|---|---|---|---|")
    for name, intent, ok, evidence in results:
        print("| %s | %s | **%s** | %s |" % (name, intent, "HOLDS" if ok else "FALSIFIED", evidence))
    print()
    print("held %d/%d" % (sum(1 for _, _, ok, _ in results if ok), len(results)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
