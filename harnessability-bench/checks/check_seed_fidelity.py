#!/usr/bin/env python3
"""Compare the shipped task seeds against validation-report.json.

validation-report.json was produced on the source branch's corpus revision.  The
public branch ships tasks.json as it stands today; where the two disagree the
report is stale, not the runner.  This check makes that difference explicit
instead of silently trusting either artifact.
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
BENCH = HERE.parent
CTRL = BENCH / "lovelace-harnessability-public-branch"
RUNNER = str(BENCH / "runner" / "Lovelace.Run.exe")
sys.path.insert(0, str(CTRL))
import bench  # noqa: E402


def main():
    tasks = json.loads((CTRL / "tasks.json").read_text(encoding="utf-8"))
    report = {r["task"]: r for r in json.loads((CTRL / "validation-report.json").read_text(encoding="utf-8"))}
    same = 0
    diffs = []
    for t in tasks:
        env = bench.run_lovelace(RUNNER, t["seed"], include_functions=False)
        rep = report[t["id"]]
        ok = bool(env.get("ok"))
        res = (env.get("result") or {}).get("display") if ok else None
        err = env.get("message") if not ok else None
        if ok == rep["seed_exec_ok"] and res == rep["seed_result"] and err == rep["error"]:
            same += 1
        else:
            diffs.append((t["id"], rep, ok, res, err))
    print("seeds matching validation-report.json: %d/%d" % (same, len(tasks)))
    for tid, rep, ok, res, err in diffs:
        print()
        print("DIFF %s" % tid)
        def brief(text):
            text = str(text)
            return "%d chars: %s" % (len(text), text if len(text) <= 110 else text[:70] + " ... " + text[-30:])
        print("  report: ok=%s err=%r result=%s" % (rep["seed_exec_ok"], rep["error"], brief(rep["seed_result"])))
        print("  actual: ok=%s err=%r result=%s" % (ok, err, brief(res)))
    print()
    print("Interpretation: a DIFF means the shipped seed differs from the one the")
    print("report was generated from - the corpus, not the runtime, moved.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
