#!/usr/bin/env python3
"""Score a matrix JSONL and emit a markdown summary of the ablation.

Runs the controller's own scorer (bench.py score) so the headline metrics are
exactly the published ones, then adds the harness-side facts the trajectory file
carries: wall time, exposed tool surface, evaluation counts, L3 registry usage
and the hidden-probe verdict per run.
"""
import argparse
import json
import statistics
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
BENCH = HERE.parent
CTRL = BENCH / "lovelace-harnessability-public-branch"
RUNNER = BENCH / "runner" / "Lovelace.Run.exe"
ORDER = ["L0", "L1", "L2", "L3"]
sys.path.insert(0, str(CTRL))
import bench  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", default=str(BENCH / "runs" / "runs-matrix.jsonl"))
    ap.add_argument("--budget", type=int, default=10)
    ap.add_argument("--out", default=str(BENCH / "runs" / "score-matrix.json"))
    args = ap.parse_args()

    runs_path = Path(args.runs)
    rows = [json.loads(l) for l in runs_path.read_text(encoding="utf-8").splitlines() if l.strip()]
    oracles = json.loads((CTRL / "private" / "oracles.json").read_text(encoding="utf-8"))

    p = subprocess.run(
        [sys.executable, str(CTRL / "bench.py"), "score", "--runs", str(runs_path),
         "--runner", str(RUNNER), "--budget", str(args.budget)],
        capture_output=True, text=True,
    )
    if p.returncode != 0:
        print(p.stderr, file=sys.stderr)
        raise SystemExit("bench.py score failed")
    score = json.loads(p.stdout)
    Path(args.out).write_text(json.dumps(score, indent=2), encoding="utf-8")

    by_mode = {}
    for r in rows:
        by_mode.setdefault(r["mode"], []).append(r)

    print("## Observation-surface ablation (real DSH sessions, model = %s)" % rows[0]["model"])
    print()
    print("| Mode | n | solved | Success@1 | Success@3 | median evals to green | median tokens to green | false convergence | repeated diagnostics | H_%d |" % args.budget)
    print("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    for mode in ORDER:
        s = score.get(mode)
        if not s:
            continue
        print("| %s | %d | %.0f%% | %.2f | %.2f | %s | %s | %.2f | %.2f | **%.3f** |" % (
            mode, s["n"], 100 * s["success_rate"], s["success_at_1"], s["success_at_3"],
            s["median_evaluations_to_green"], s["median_tokens_to_green"],
            s["false_convergence_rate"], s["repeated_diagnostic_rate"], s["H_B"]))
    print()
    print("| Mode | mean evals | mean wall s | mean tool calls | L3 inspections | exposed tools |")
    print("|---|---:|---:|---:|---:|---|")
    for mode in ORDER:
        rs = by_mode.get(mode)
        if not rs:
            continue
        evals = [len([e for e in r["events"] if e["type"] == "candidate"]) for r in rs]
        walls = [r["harness"]["wall_ms"] / 1000.0 for r in rs if r["harness"].get("wall_ms")]
        calls = [r["harness"].get("tool_calls") or 0 for r in rs]
        insp = [len([e for e in r["events"] if e["type"] == "inspection"]) for r in rs]
        tools = sorted({t for r in rs for t in (r["harness"].get("exposed_tools") or [])})
        print("| %s | %.2f | %.1f | %.2f | %d | %s |" % (
            mode, statistics.mean(evals), statistics.mean(walls) if walls else 0,
            statistics.mean(calls), sum(insp), ", ".join(tools) or "-"))
    print()

    # Per-run hidden-probe verdict, straight from the controller's verifier.
    cells = {}
    for r in rows:
        step = None
        for i, e in enumerate([x for x in r["events"] if x["type"] == "candidate"], 1):
            ok, _ = bench.verify_script(str(RUNNER), r["task_id"], e["script"], oracles)
            if ok:
                step = i
                break
        final_ok, _ = bench.verify_script(str(RUNNER), r["task_id"], r["final_script"] or "", oracles)
        evals = len([x for x in r["events"] if x["type"] == "candidate"])
        cells[(r["task_id"], r["mode"])] = (step, final_ok, evals)

    tasks = []
    for r in rows:
        if r["task_id"] not in tasks:
            tasks.append(r["task_id"])
    print("Per task: first evaluation that passes every hidden probe (E = used all %d and failed)." % args.budget)
    print()
    print("| Task | " + " | ".join(ORDER) + " |")
    print("|---|" + "---|" * len(ORDER))
    for tid in tasks:
        line = []
        for mode in ORDER:
            cell = cells.get((tid, mode))
            line.append("E" if cell is None else (str(cell[0]) if cell[0] else "-"))
        print("| %s | %s |" % (tid, " | ".join(line)))
    print()
    solved = sum(1 for v in cells.values() if v[0])
    false_conv = sum(1 for r in rows
                     if r["agent_claimed_done"]
                     and not bench.verify_script(str(RUNNER), r["task_id"], r["final_script"] or "", oracles)[0])
    print("hidden-probe verification: %d/%d runs solved, %d false convergences (claimed done, final program still fails)"
          % (solved, len(cells), false_conv))
    print("wrote %s" % args.out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
