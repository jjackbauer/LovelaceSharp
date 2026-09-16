#!/usr/bin/env python3
"""Run the Lovelace harnessability matrix against real DSH headless sessions.

One cell = (task, observation mode, replicate).  Each cell boots a headless DSH
profile patched by bench.patch.yml, which removes every first-party tool and
mounts the benchmark MCP server, so the session sees exactly the observation
surface assigned to that mode.

The driver records, per run:

* the candidate/observation/inspection events the benchmark surface actually
  served (from the MCP server's own log - the ground truth of the surface);
* per-candidate token usage and the exposed tool list (from the DSH session log,
  decoded from its concatenated zstd frames);
* the final program and whether the agent claimed completion.

Output is one JSON object per run (JSONL), matching run.schema.json, ready for
bench.py score.

    python run_matrix.py --tasks arity-dot,matrix-trace --modes L0,L2 --replicates 1
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
BENCH = HERE.parent
REPO = BENCH.parent
CTRL = BENCH / "lovelace-harnessability-public-branch"
RUNS = BENCH / "runs"
DEFAULT_RUNNER = BENCH / "runner" / "Lovelace.Run.exe"
PATCH = HERE / "bench.patch.yml"
MCP_SERVER = HERE / "bench-mcp" / "server.mjs"
DECODER = HERE / "session_events.mjs"
BENCH_PY = CTRL / "bench.py"
FENCE = chr(96) * 3

DSH_CHECKOUT = Path(os.environ.get("DSH_CHECKOUT", r"C:\Users\ricar\deepseek-harness"))
DSH_HOME = Path(os.environ.get("DSH_HOME", str(Path.home() / ".dsh")))
SESSIONS_ROOT = DSH_HOME / "sessions"

SUBMIT_INSTRUCTION = (
    "Use the benchmark's evaluation tool for every candidate program, and call "
    "its submission tool with the complete final program when you are done."
)


def runner_identity(runner):
    """The exact binary and revision a trajectory came from.

    The gate compares two matrices; without this a runtime change and a sampling difference are
    indistinguishable in the evidence.
    """
    digest = hashlib.sha256(Path(runner).read_bytes()).hexdigest()
    revision = subprocess.run(["git", "-C", str(REPO), "rev-parse", "HEAD"],
                              capture_output=True, text=True)
    status = subprocess.run(["git", "-C", str(REPO), "status", "--porcelain"],
                            capture_output=True, text=True)
    return {
        "runner_path": str(runner),
        "runner_sha256": digest,
        "git_revision": (revision.stdout or "").strip() or None,
        "git_dirty": bool((status.stdout or "").strip()),
    }


def session_dirs():
    if not SESSIONS_ROOT.exists():
        return set()
    return {str(p) for p in SESSIONS_ROOT.glob("*/*") if p.is_dir()}


def session_surface(events):
    """The evaluate_candidate scripts and observation payloads a session saw."""
    names = {}
    scripts = []
    observations = []
    for e in events:
        t = e.get("type")
        if t == "tool/call":
            d = e["data"]
            name = d.get("name") or ""
            names[d.get("callId")] = name
            if name.endswith("evaluate_candidate"):
                try:
                    args = json.loads(d.get("arguments") or "{}")
                except ValueError:
                    args = {}
                scripts.append(args.get("script"))
        elif t == "tool/result":
            msg = e["data"].get("message") or {}
            name = names.get((msg.get("source") or {}).get("callId"), "")
            if not name.endswith("evaluate_candidate"):
                continue
            text = None
            for block in msg.get("content") or []:
                for c in block.get("content") or []:
                    if c.get("type") == "text":
                        text = c.get("text")
            try:
                observations.append(json.loads(text))
            except (ValueError, TypeError):
                observations.append(None)
    return scripts, observations


def surface_key(scripts, observations):
    return (tuple(scripts), json.dumps(observations, sort_keys=True, ensure_ascii=False))


def record_key(record):
    scripts = [e.get("script") for e in record["events"] if e.get("type") == "candidate"]
    observations = [e.get("observation") for e in record["events"] if e.get("type") == "observation"]
    return surface_key(scripts, observations)


def script_key(record):
    return tuple(e.get("script") for e in record["events"] if e.get("type") == "candidate")


def observation_prefix_score(record_observations, session_observations):
    n = 0
    for a, b in zip(record_observations, session_observations):
        if a != b:
            break
        n += 1
    return n


def pick_session(candidates, record_observations):
    """Among sessions with identical candidate scripts, take the best prefix match.

    Candidates alone are not enough: two arms (modes) can run the same task at
    the same time and submit identical scripts.  The observation payloads are
    mode-specific, so the longest matching prefix identifies the right session.
    The record may hold a trailing budget-exhausted observation the session log
    never shows, which is why this is a prefix test rather than equality.
    """
    best = None
    best_score = -1
    for log, observations in candidates:
        score = observation_prefix_score(record_observations, observations)
        if score > best_score or (score == best_score and best and log > best):
            best, best_score = log, score
    return best if best_score > 0 else None


def locate_session(record_key_value=None, since=None, record=None):
    if record is None:
        return None
    scripts = script_key(record)
    if not scripts:
        return None
    observations = [e.get("observation") for e in record["events"] if e.get("type") == "observation"]
    matches = []
    for d in SESSIONS_ROOT.glob("*/*"):
        if not d.is_dir():
            continue
        if since is not None and d.stat().st_mtime < since - 60:
            continue
        for log in sorted(d.glob("session.v3.jsonl*")):
            s, o = session_surface(decode_session(log))
            if len(s) >= len(scripts) and tuple(s[:len(scripts)]) == scripts:
                matches.append((str(log), o))
    return pick_session(matches, observations)


def decode_session(path):
    p = subprocess.run(["node", str(DECODER), str(path)], capture_output=True, text=True,
                       encoding="utf-8", errors="replace")
    events = []
    for line in (p.stdout or "").splitlines():
        line = line.strip()
        if line:
            try:
                events.append(json.loads(line))
            except json.JSONDecodeError:
                pass
    return events


def harness_facts(events):
    """Model identity, exposed tool list, token usage per tool call, wall time."""
    model = provider = None
    tools = []
    usage_by_call = []
    last_usage = {}
    calls = 0
    start = end = None
    for e in events:
        t = e.get("type")
        if e.get("time"):
            start = start or e["time"]
            end = e["time"]
        if t == "request/header":
            h = e["data"]["header"]
            model = (h.get("config") or {}).get("model")
            provider = (h.get("config") or {}).get("provider")
            tools = [x.get("name") for x in h.get("tools", [])]
        elif t == "assistant/message":
            last_usage = e["data"].get("usage") or {}
        elif t == "tool/call":
            calls += 1
            data = e["data"]
            name = data.get("name") or (data.get("call") or {}).get("name")
            usage_by_call.append({"name": name, "usage": dict(last_usage)})
    return {
        "model": model,
        "provider": provider,
        "exposed_tools": tools,
        "tool_calls": calls,
        "usage_by_call": usage_by_call,
        "wall_ms": (end - start) if (start and end) else None,
    }


def build_session_index():
    """One pass over the session tree: candidate-script key -> [(log, observations)]."""
    index = {}
    for d in SESSIONS_ROOT.glob("*/*"):
        if not d.is_dir():
            continue
        for log in sorted(d.glob("session.v3.jsonl*")):
            scripts, observations = session_surface(decode_session(log))
            if not scripts:
                continue
            # A session may hold one more call than the record has candidates: the
            # evaluation that was refused because the budget was exhausted is a tool
            # call in the session but never becomes a candidate event.  Indexing
            # every prefix lets a record match the session that starts with it.
            for i in range(1, len(scripts) + 1):
                index.setdefault(tuple(scripts[:i]), []).append((str(log), observations))
    return index


def reattach_tokens(record, facts):
    usages = [c["usage"] for c in facts.get("usage_by_call", [])
              if str(c.get("name", "")).endswith("evaluate_candidate")]
    i = 0
    for e in record["events"]:
        if e.get("type") != "candidate":
            continue
        usage = usages[i] if i < len(usages) else {}
        i += 1
        e["input_tokens"] = int(usage.get("inputTokens") or 0)
        e["output_tokens"] = int(usage.get("outputTokens") or 0)


def repair(path):
    """Re-derive harness facts and token usage for an existing runs file.

    Needed because a first pass attributed session logs by creation time, which
    is ambiguous when several arms run concurrently.
    """
    p = Path(path)
    rows = [json.loads(l) for l in p.read_text(encoding="utf-8").splitlines() if l.strip()]
    index = build_session_index()
    print("session index: %d distinct run surfaces" % len(index))
    fixed = 0
    for r in rows:
        observations = [e.get("observation") for e in r["events"] if e.get("type") == "observation"]
        log = pick_session(index.get(script_key(r), []), observations)
        if not log:
            r["harness"]["session_match"] = "unmatched"
            continue
        facts = harness_facts(decode_session(Path(log)))
        reattach_tokens(r, facts)
        r["harness"].update({
            "provider": facts.get("provider"),
            "exposed_tools": facts.get("exposed_tools"),
            "tool_calls": facts.get("tool_calls"),
            "session_log": log,
            "session_match": "content",
        })
        if facts.get("model"):
            r["model"] = facts["model"]
        (RUNS / r["run_id"] / "run.json").write_text(
            json.dumps(r, indent=2, ensure_ascii=False), encoding="utf-8")
        fixed += 1
    with p.open("w", encoding="utf-8") as fh:
        for r in rows:
            fh.write(json.dumps(r, ensure_ascii=False) + chr(10))
    print("repaired %d/%d runs in %s" % (fixed, len(rows), p))
    return 0


def parse_fenced(text):
    """Last fenced block that looks like a program, for runs that never submit."""
    blocks = []
    inside = False
    buf = []
    for line in text.splitlines():
        if line.strip().startswith(FENCE):
            if inside:
                blocks.append(chr(10).join(buf).strip())
                buf = []
            inside = not inside
            continue
        if inside:
            buf.append(line)
    if inside and buf:
        blocks.append(chr(10).join(buf).strip())
    for b in reversed(blocks):
        if "func " in b:
            return b
    return blocks[-1] if blocks else None


def build_prompt(task_id, budget):
    """The controller's own prompt, plus the submission-tool instruction."""
    p = subprocess.run(
        [sys.executable, str(BENCH_PY), "prompt", "--task", task_id, "--budget", str(budget)],
        capture_output=True, text=True,
    )
    if p.returncode != 0:
        raise SystemExit("bench.py prompt failed: " + p.stderr)
    return p.stdout.strip() + chr(10) + SUBMIT_INSTRUCTION + chr(10)


def run_id_for(mode, task_id, replicate):
    return "dsh-%s-%s-%s-%03d" % ("deepseek-flash", mode, task_id, replicate)


def run_cell(task_id, mode, replicate, budget, timeout, runner, profile):
    model_label = "deepseek-flash"
    run_id = run_id_for(mode, task_id, replicate)
    run_dir = RUNS / run_id
    run_dir.mkdir(parents=True, exist_ok=True)
    events_path = run_dir / "surface.jsonl"
    if events_path.exists():
        events_path.unlink()
    stdout_path = run_dir / "stdout.txt"
    stderr_path = run_dir / "stderr.txt"

    prompt = build_prompt(task_id, budget)
    node_exe = (subprocess.run(["where", "node"], capture_output=True, text=True).stdout.splitlines() or ["node"])[0].strip()
    env = dict(os.environ)
    env.update({
        "BENCH_NODE": node_exe,
        "BENCH_MCP_SERVER": str(MCP_SERVER),
        "BENCH_MCP_CWD": str(MCP_SERVER.parent),
        "BENCH_RUNNER": str(runner),
        "BENCH_TASKS": str(CTRL / "tasks.json"),
        "BENCH_TASK": task_id,
        "BENCH_MODE": mode,
        "BENCH_BUDGET": str(budget),
        "BENCH_RUN_ID": run_id,
        "BENCH_EVENTS": str(events_path),
        "BENCH_PERSONA_SUFFIX": (
            "You are repairing one Lovelace Script program. You may evaluate candidate programs only "
            "through the provided benchmark tool; at most %d evaluations are allowed. "
            "The evaluator also runs hidden functional probes, so preserve the requested function "
            "contract instead of hard-coding the visible example. Call the submission tool with the "
            "complete final program when you are confident it is correct." % budget
        ),
        "DSH_PERMISSION_MODE": "danger-full-access",
    })

    cmd = [
        node_exe, "--import", "tsx/esm", str(DSH_CHECKOUT / "apps" / "cli" / "src" / "bin.ts"),
        "--profile", profile, "--patch", str(PATCH), prompt,
    ]
    started = time.time()
    timed_out = False
    with open(stdout_path, "w", encoding="utf-8") as out, open(stderr_path, "w", encoding="utf-8") as err:
        try:
            proc = subprocess.run(cmd, cwd=str(DSH_CHECKOUT), env=env, stdout=out, stderr=err, timeout=timeout)
            code = proc.returncode
        except subprocess.TimeoutExpired:
            timed_out = True
            code = None
    elapsed = time.time() - started

    surface = []
    if events_path.exists():
        for line in events_path.read_text(encoding="utf-8").splitlines():
            if line.strip():
                surface.append(json.loads(line))

    session_log = locate_session(since=started, record={"events": surface})
    facts = {"model": None, "provider": None, "exposed_tools": [], "tool_calls": 0,
             "usage_by_call": [], "wall_ms": None}
    if session_log:
        facts = harness_facts(decode_session(Path(session_log)))

    candidates = [e for e in surface if e.get("type") == "candidate"]
    final_script = None
    claimed = False
    for e in surface:
        if e.get("type") == "completion":
            final_script = e.get("script")
            claimed = True
    stdout_text = stdout_path.read_text(encoding="utf-8", errors="replace")
    if final_script is None:
        fenced = parse_fenced(stdout_text)
        if fenced:
            final_script = fenced
            claimed = True
    if not final_script and candidates:
        final_script = candidates[-1].get("script")
    if not final_script:
        final_script = ""

    eval_usages = [c["usage"] for c in facts.get("usage_by_call", [])
                   if str(c.get("name", "")).endswith("evaluate_candidate")]

    events = []
    eval_index = 0
    for e in surface:
        if e.get("type") == "candidate":
            usage = eval_usages[eval_index] if eval_index < len(eval_usages) else {}
            eval_index += 1
            events.append({
                "type": "candidate",
                "script": e.get("script"),
                "input_tokens": int(usage.get("inputTokens") or 0),
                "output_tokens": int(usage.get("outputTokens") or 0),
            })
        elif e.get("type") == "observation":
            events.append({"type": "observation", "observation": e.get("observation")})
        elif e.get("type") == "inspection":
            events.append({"type": "inspection", "inspection": e.get("inspection")})

    record = {
        "run_id": run_id,
        "task_id": task_id,
        "mode": mode,
        "model": facts.get("model") or model_label,
        "replicate": replicate,
        "events": events,
        "final_script": final_script,
        "agent_claimed_done": claimed,
        "harness": {
            **runner_identity(runner),
            "profile": profile,
            "patch": str(PATCH),
            "provider": facts.get("provider"),
            "exposed_tools": facts.get("exposed_tools"),
            "tool_calls": facts.get("tool_calls"),
            "evaluations": len(candidates),
            "budget": budget,
            "wall_ms": round(elapsed * 1000),
            "session_log": session_log,
            "exit_code": code,
            "timed_out": timed_out,
            "stdout": str(stdout_path),
            "stderr": str(stderr_path),
        },
    }
    (run_dir / "run.json").write_text(json.dumps(record, indent=2, ensure_ascii=False), encoding="utf-8")
    return record


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tasks", default="all", help="comma-separated task ids, or 'all'")
    ap.add_argument("--modes", default="L2", help="comma-separated observation modes")
    ap.add_argument("--replicates", type=int, default=1)
    ap.add_argument("--budget", type=int, default=10)
    ap.add_argument("--timeout", type=int, default=1200, help="per-run wall-clock timeout (s)")
    ap.add_argument("--runner", default=str(DEFAULT_RUNNER))
    ap.add_argument("--profile", default="headless")
    ap.add_argument("--out", default=str(RUNS / "runs.jsonl"))
    ap.add_argument("--resume", action="store_true",
                    help="keep an existing --out and only run cells it does not already contain")
    ap.add_argument("--repair", action="store_true",
                    help="do not run anything: re-derive harness facts for an existing --out")
    args = ap.parse_args()

    if args.repair:
        return repair(args.out)

    tasks = json.loads((CTRL / "tasks.json").read_text(encoding="utf-8"))
    ids = [t["id"] for t in tasks] if args.tasks == "all" else [t.strip() for t in args.tasks.split(",") if t.strip()]
    modes = [m.strip() for m in args.modes.split(",") if m.strip()]

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    recorded = set()
    if args.resume and out_path.exists():
        for line in out_path.read_text(encoding="utf-8").splitlines():
            if line.strip():
                recorded.add(json.loads(line)["run_id"])
        print("resuming: %d run(s) already recorded" % len(recorded))
    elif out_path.exists():
        out_path.unlink()

    total = len(ids) * len(modes) * args.replicates
    done = 0
    for rep in range(1, args.replicates + 1):
        for mode in modes:
            for tid in ids:
                done += 1
                if run_id_for(mode, tid, rep) in recorded:
                    print("[%d/%d] %s %s rep%d -- already recorded, skipped" % (done, total, tid, mode, rep), flush=True)
                    continue
                print("[%d/%d] %s %s rep%d" % (done, total, tid, mode, rep), flush=True)
                rec = run_cell(tid, mode, rep, args.budget, args.timeout, Path(args.runner), args.profile)
                with open(out_path, "a", encoding="utf-8") as fh:
                    fh.write(json.dumps(rec, ensure_ascii=False) + chr(10))
                print("    evals=%d claimed=%s wall=%sms tools=%s"
                      % (len([e for e in rec["events"] if e["type"] == "candidate"]),
                         rec["agent_claimed_done"], rec["harness"]["wall_ms"],
                         rec["harness"]["exposed_tools"]), flush=True)
    print("wrote " + str(out_path))
    return 0


if __name__ == "__main__":
    sys.exit(main())
