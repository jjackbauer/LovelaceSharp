#!/usr/bin/env python3
"""Reproduce the RUN-REPORT synthetic scorer integration test.

Builds 20 tasks x 4 modes = 80 two-step trajectories (the broken seed first, the
golden second) with the real projection surface, then checks the scorer reports
every run green at step 2 and H_B = 0.9 for a budget of 10 - exactly the result
the branch's RUN-REPORT.md claims.
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
BENCH = HERE.parent
CTRL = BENCH / "lovelace-harnessability-public-branch"
RUNNER = str(BENCH / "runner" / "Lovelace.Run.exe")
OUT = BENCH / "runs" / "synthetic.jsonl"
MODES = ("L0", "L1", "L2", "L3")
sys.path.insert(0, str(CTRL))
import bench  # noqa: E402


def main():
    tasks = json.loads((CTRL / "tasks.json").read_text(encoding="utf-8"))
    goldens = json.loads((CTRL / "private" / "goldens.json").read_text(encoding="utf-8"))
    rows = []
    for t in tasks:
        for mode in MODES:
            events = []
            for script in (t["seed"], goldens[t["id"]]):
                env = bench.run_lovelace(RUNNER, script, include_functions=False)
                events.append({"type": "candidate", "script": script, "input_tokens": 100, "output_tokens": 20})
                events.append({"type": "observation", "observation": bench.project(env, mode)})
            rows.append({
                "run_id": "synthetic-%s-%s" % (mode, t["id"]),
                "task_id": t["id"],
                "mode": mode,
                "model": "synthetic",
                "replicate": 1,
                "events": events,
                "final_script": goldens[t["id"]],
                "agent_claimed_done": True,
            })
    OUT.parent.mkdir(parents=True, exist_ok=True)
    with OUT.open("w", encoding="utf-8") as fh:
        for r in rows:
            fh.write(json.dumps(r, ensure_ascii=False) + chr(10))
    print("wrote %d synthetic trajectories to %s" % (len(rows), OUT))
    print("now run: bench.py score --runs %s --runner %s --budget 10" % (OUT, RUNNER))
    print("expected: every run green at step 2, H_B = 0.9 in every mode")
    return 0


if __name__ == "__main__":
    sys.exit(main())
