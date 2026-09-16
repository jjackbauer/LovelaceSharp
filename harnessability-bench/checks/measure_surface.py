#!/usr/bin/env python3
"""Instrument the observation surface: what the runtime told the agent, and what it did not.

Reads recorded trajectories and reports the defects the API enhancement plan
targets: unknown-name failures, taxonomy precision, source-span coverage and
partial-result availability.  Run it before and after a runtime change to see the
surface move.
"""
import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
BENCH = HERE.parent
UNKNOWN = re.compile(r"Unknown function '([^']+)'")


def tally(rows):
    out = {"failures": 0, "message": 0, "category": 0, "span": 0, "partial": 0, "unknown": 0}
    for r in rows:
        for e in r["events"]:
            if e["type"] != "observation" or e["observation"].get("ok") is not False:
                continue
            obs = e["observation"]
            out["failures"] += 1
            out["message"] += bool(obs.get("message"))
            out["category"] += bool(obs.get("category"))
            diags = obs.get("diagnostics") or []
            out["span"] += any(isinstance(d, dict) and d.get("position") is not None for d in diags)
            out["partial"] += bool(obs.get("partialVariables"))
            for d in diags:
                text = d.get("message") if isinstance(d, dict) else d
                if UNKNOWN.search(text or ""):
                    out["unknown"] += 1
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", default=str(BENCH / "runs" / "runs-matrix.jsonl"))
    ap.add_argument("--label", default="baseline")
    args = ap.parse_args()

    rows = [json.loads(l) for l in Path(args.runs).read_text(encoding="utf-8").splitlines() if l.strip()]
    print("== observation surface by mode: %s ==" % args.label)
    print()
    print("| Mode | failures | message | category | source span | partial vars | unknown-name failures |")
    print("|---|---:|---:|---:|---:|---:|---:|")
    for mode in ("L0", "L1", "L2", "L3"):
        subset = [r for r in rows if r["mode"] == mode]
        stats = tally(subset)
        n = stats["failures"]
        pct = (lambda v: "%.0f%%" % (100 * v / n) if n else "-")
        print("| %s | %d | %s | %s | %s | %s | %d |" % (
            mode, n, pct(stats["message"]), pct(stats["category"]), pct(stats["span"]),
            pct(stats["partial"]), stats["unknown"]))
    print()
    failures = 0
    with_span = 0
    with_message = 0
    with_category = 0
    with_partial = 0
    categories = Counter()
    unknown_names = Counter()
    for r in rows:
        for e in r["events"]:
            if e["type"] != "observation":
                continue
            obs = e["observation"]
            if obs.get("ok") is not False:
                continue
            failures += 1
            if obs.get("message"):
                with_message += 1
            if obs.get("category"):
                with_category += 1
                categories[obs["category"]] += 1
            diags = obs.get("diagnostics") or []
            # L1 projects diagnostics as bare message strings, L2/L3 as objects
            if any(isinstance(d, dict) and d.get("position") is not None for d in diags):
                with_span += 1
            if obs.get("partialVariables"):
                with_partial += 1
            for d in diags:
                text = d.get("message") if isinstance(d, dict) else d
                m = UNKNOWN.search(text or "")
                if m:
                    unknown_names[m.group(1)] += 1

    print("== observation surface (all modes): %s ==" % args.label)
    print("failure observations: %d" % failures)
    if failures:
        print("  carry a human message      : %d (%.0f%%)" % (with_message, 100 * with_message / failures))
        print("  carry a category           : %d (%.0f%%)" % (with_category, 100 * with_category / failures))
        print("  carry a source span        : %d (%.0f%%)" % (with_span, 100 * with_span / failures))
        print("  carry partial variables    : %d (%.0f%%)" % (with_partial, 100 * with_partial / failures))
    print("  categories: %s" % dict(categories.most_common()))
    print("  unknown-function failures  : %d" % sum(unknown_names.values()))
    print("  names the agent tried     : %s" % dict(unknown_names.most_common(20)))
    print("  unknown name -> resolved? : %s" % (
        "none suggested by the runtime" if unknown_names else "n/a"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
