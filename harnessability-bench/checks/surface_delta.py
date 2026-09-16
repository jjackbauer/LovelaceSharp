#!/usr/bin/env python3
"""Measure the observation surface a runner actually produces, against the published baseline.

The benchmark contract is that L0-L3 are FOUR FROZEN PROJECTIONS of one runner envelope
(bench.py:project). A change to the wire is therefore "harnessability work" only if it moves
what an agent can SEE: a field the projection drops is invisible to every mode, and a field it
keeps changes the context cost of every observation.

This checker re-measures both halves against the CURRENT runner and states the deltas:

  1. per-mode observation bytes for the 20 shipped seeds, against validation-report.json
     (the published baseline, recorded before the change);
  2. the cost of the L3 on-demand registry question, answered two ways - the inspect path the
     benchmark shipped (one full evaluation observation) and the --functions query the runner
     now answers without evaluating anything.

Run it after any runner change that touches the envelope:

    python checks/surface_delta.py --runner runner/Lovelace.Run.exe
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
BENCH = HERE.parent
PUBLIC = BENCH / "lovelace-harnessability-public-branch"
sys.path.insert(0, str(PUBLIC))

import bench  # noqa: E402  (the shipped projection is the thing under measurement)

BASELINE_PATH = PUBLIC / "validation-report.json"
RUNNER_KEY = "Lovelace.Run.exe"


def encoded_len(obj) -> int:
    return len(json.dumps(obj, separators=(",", ":"), ensure_ascii=False).encode("utf-8"))


def probe_registry(runner: str, query: str) -> dict:
    """The dedicated registry query, in the runner's own envelope."""
    p = subprocess.run([runner, "--functions", query], text=True, capture_output=True)
    if p.returncode != 0:
        return {"error": f"exit {p.returncode}", "stderr": p.stderr[:300],
                "bytes": len(p.stdout.encode("utf-8"))}
    env = json.loads(p.stdout)
    env["bytes"] = len(p.stdout.encode("utf-8"))
    return env


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--runner", default=str(BENCH / "runner" / RUNNER_KEY))
    ap.add_argument("--query", default="det")
    ap.add_argument("--json-out", default="")
    ap.add_argument("--report-out", default="",
                    help="write a fresh per-task validation report in the published report's shape")
    args = ap.parse_args()

    if not Path(args.runner).exists():
        print(f"runner not found: {args.runner}", file=sys.stderr)
        return 2

    baseline = {}
    if BASELINE_PATH.exists():
        for row in json.loads(BASELINE_PATH.read_text(encoding="utf-8")):
            baseline[row["task"]] = row

    rows = []
    totals = {m: 0 for m in ("L0", "L1", "L2", "L3")}
    base_totals = {m: 0 for m in ("L0", "L1", "L2", "L3")}
    for tid, task in bench.TASKS.items():
        env = bench.run_lovelace(args.runner, task["seed"], include_functions=False)
        row = {"task": tid, "class": task.get("class"), "seed_ok": bool(env.get("ok")), "bytes": {}}
        for mode in ("L0", "L1", "L2", "L3"):
            n = encoded_len(bench.project(env, mode))
            row["bytes"][mode] = n
            totals[mode] += n
            if tid in baseline:
                base_totals[mode] += baseline[tid][f"{mode}_bytes"]
        row["error"] = env.get("message") if not env.get("ok") else None
        row["result"] = (env.get("result") or {}).get("display") if env.get("ok") else None
        rows.append(row)

    if args.report_out:
        # The same shape as validation-report.json, so a fresh measurement can be diffed
        # against the published one field by field.
        report = [
            {
                "task": r["task"],
                "class": r["class"],
                "seed_exec_ok": r["seed_ok"],
                "seed_result": r["result"],
                "error": r["error"],
                **{f"{m}_bytes": r["bytes"][m] for m in ("L0", "L1", "L2", "L3")},
            }
            for r in rows
        ]
        Path(args.report_out).write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")
        print(f"wrote {args.report_out}")

    print("== observation bytes per seed: published baseline -> current runner ==")
    print()
    print("| task | " + " | ".join(f"{m} base -> now" for m in ("L0", "L1", "L2", "L3")) + " |")
    print("|---|" + "---|" * 4)
    for row in rows:
        cells = []
        for mode in ("L0", "L1", "L2", "L3"):
            was = baseline.get(row["task"], {}).get(f"{mode}_bytes")
            now = row["bytes"][mode]
            cells.append("-" if was is None else (f"{was} -> {now}" if was != now else f"{was}"))
        print(f"| {row['task']} | " + " | ".join(cells) + " |")
    print()
    print("| mode | baseline total | current total | delta | baseline mean | current mean | baseline max | current max |")
    print("|---|---:|---:|---:|---:|---:|---:|---:|")
    for mode in ("L0", "L1", "L2", "L3"):
        d = totals[mode] - base_totals[mode]
        now_vals = [r["bytes"][mode] for r in rows]
        was_vals = [baseline[r["task"]][f"{mode}_bytes"] for r in rows if r["task"] in baseline]
        was_mean = round(sum(was_vals) / len(was_vals), 1) if was_vals else 0
        print(f"| {mode} | {base_totals[mode]} | {totals[mode]} | {'+' if d > 0 else ''}{d} | "
              f"{was_mean} | {round(sum(now_vals) / len(now_vals), 1)} | {max(was_vals) if was_vals else 0} | {max(now_vals)} |")

    # --- the L3 registry question, answered two ways ----------------------------------------
    full = bench.run_lovelace(args.runner, "0", include_functions=True)
    q = args.query.casefold()
    funcs = [bench.function_projection(f) for f in full.get("functions", [])
             if q in str(f.get("name", "")).casefold()]
    inspect_bytes = encoded_len({"query": args.query, "totalMatches": len(funcs),
                                 "returned": len(funcs), "functions": funcs})
    eval_envelope_bytes = encoded_len(full)
    query = probe_registry(args.runner, args.query)
    query_bytes = query.get("bytes", 0)

    print()
    print("== the L3 on-demand registry question ==")
    print()
    print("| path | bytes | what it costs |")
    print("|---|---:|---|")
    print(f"| the whole evaluation envelope (what inspect reads the registry out of) | {eval_envelope_bytes} | one full evaluation |")
    print(f"| the projected answer inspect prints | {inspect_bytes} | one full evaluation + a projection |")
    print(f"| --functions {args.query} | {query_bytes} | no evaluation at all |")
    if query_bytes and eval_envelope_bytes:
        print()
        print(f"The query is **{eval_envelope_bytes / query_bytes:.0f}x** smaller than the envelope the shipped path evaluates to answer the same question.")

    if args.json_out:
        Path(args.json_out).write_text(json.dumps({
            "runner": str(args.runner),
            "per_task": rows,
            "totals": totals,
            "baseline_totals": base_totals,
            "registry": {
                "evaluation_envelope_bytes": eval_envelope_bytes,
                "inspect_projection_bytes": inspect_bytes,
                "functions_query_bytes": query_bytes,
                "functions_query_total": query.get("total"),
            },
        }, indent=2), encoding="utf-8")
        print(f"\nwrote {args.json_out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())