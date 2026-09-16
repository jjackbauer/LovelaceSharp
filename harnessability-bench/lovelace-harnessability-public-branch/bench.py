#!/usr/bin/env python3
from __future__ import annotations
import argparse, hashlib, json, statistics, subprocess, sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
TASKS = {t["id"]: t for t in json.loads((HERE / "tasks.json").read_text(encoding="utf-8"))}
GOLDENS_PATH = HERE / "private" / "goldens.json"
PROBES_PATH = HERE / "private" / "probes.json"
ORACLES_PATH = HERE / "private" / "oracles.json"


def run_lovelace(runner: str, script: str, include_functions: bool = False) -> dict[str, Any]:
    argv = [runner, "--stdin", "--json"]
    if not include_functions:
        argv.append("--omit-functions")
    p = subprocess.run(argv, input=script, text=True, capture_output=True)
    try:
        obj = json.loads(p.stdout)
    except Exception as ex:
        raise RuntimeError(
            f"runner emitted invalid JSON (exit {p.returncode}): {p.stdout[:1000]!r} / stderr={p.stderr[:1000]!r}"
        ) from ex
    obj["_processExitCode"] = p.returncode
    return obj


def _canon(v: Any) -> Any:
    if isinstance(v, dict):
        # Keep semantic structure, remove presentation/runtime metadata.
        drop = {
            "display", "pretty", "typed", "elapsed", "elapsedTime", "timings",
            "revision", "plot", "path", "svg", "cancellation", "_processExitCode"
        }
        return {k: _canon(v[k]) for k in sorted(v) if k not in drop}
    if isinstance(v, list):
        return [_canon(x) for x in v]
    return v


def semantic_fingerprint(envelope: dict[str, Any]) -> str:
    """Fingerprint only the final structured value, not implementation details.

    Intermediary variable names, helper functions, display strings, timings and revisions are
    intentionally excluded so equivalent solutions are accepted.
    """
    if not envelope.get("ok"):
        raise ValueError(f"cannot fingerprint failing envelope: {envelope.get('message')}")
    payload = _canon((envelope.get("result") or {}).get("structured"))
    raw = json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(raw.encode("utf-8")).hexdigest()


def project(envelope: dict[str, Any], mode: str) -> dict[str, Any]:
    """Project the same runner envelope into one of four agent-visible feedback surfaces."""
    ok = bool(envelope.get("ok"))
    if mode == "L0":
        if ok:
            return {"ok": True, "result": (envelope.get("result") or {}).get("display")}
        return {"ok": False, "error": "failed"}

    if mode == "L1":
        out: dict[str, Any] = {"ok": ok}
        if ok:
            out["result"] = (envelope.get("result") or {}).get("display")
        else:
            out["error"] = envelope.get("message", "failed")
            out["diagnostics"] = [d.get("message") for d in envelope.get("diagnostics", [])]
        return out

    if mode in {"L2", "L3"}:
        if ok:
            out = {
                "ok": True,
                "result": {
                    "kind": (envelope.get("result") or {}).get("kind"),
                    "structured": (envelope.get("result") or {}).get("structured"),
                },
                "variables": [
                    {
                        "name": v.get("name"),
                        "kind": v.get("kind"),
                        "structured": v.get("structured"),
                    }
                    for v in envelope.get("variables", [])
                ],
            }
        else:
            out = {
                "ok": False,
                "code": envelope.get("code"),
                "category": envelope.get("category"),
                "message": envelope.get("message"),
                "recoverable": envelope.get("recoverable"),
                "diagnostics": [
                    {
                        "message": d.get("message"),
                        "position": d.get("position"),
                        "line": d.get("line"),
                        "column": d.get("column"),
                    }
                    for d in envelope.get("diagnostics", [])
                ],
            }
            if envelope.get("partialVariables") is not None:
                out["partialVariables"] = [
                    {"name": v.get("name"), "kind": v.get("kind"), "structured": v.get("structured")}
                    for v in envelope.get("partialVariables", [])
                ]

        # L3 adds execution telemetry. Semantic registry lookup is deliberately on-demand via
        # `inspect`; dumping the full registry on every evaluation is a context-cost confound.
        if mode == "L3":
            if envelope.get("timings") is not None:
                out["timings"] = envelope.get("timings")
            if envelope.get("cancellation") is not None:
                out["cancellation"] = envelope.get("cancellation")
        return out

    raise ValueError(f"unknown mode {mode!r}")


def function_projection(f: dict[str, Any]) -> dict[str, Any]:
    return {
        "name": f.get("name"),
        "parameters": f.get("parameters"),
        "minArity": f.get("minArity"),
        "parameterCount": f.get("parameterCount"),
        "variadic": f.get("variadic"),
        "builtin": f.get("builtin"),
        "plugin": f.get("plugin"),
    }


def load_oracles() -> dict[str, list[str]]:
    if not ORACLES_PATH.exists():
        raise SystemExit("private/oracles.json missing; run `bench.py prepare` first")
    return json.loads(ORACLES_PATH.read_text(encoding="utf-8"))


def load_probes() -> dict[str, list[str]]:
    return json.loads(PROBES_PATH.read_text(encoding="utf-8"))


def build_oracles(runner: str, verbose: bool = True) -> dict[str, list[str]]:
    goldens = json.loads(GOLDENS_PATH.read_text(encoding="utf-8"))
    probes = load_probes()
    out: dict[str, list[str]] = {}
    failures: list[tuple[str, str, Any, Any]] = []

    for tid, script in goldens.items():
        # The visible example must itself execute; otherwise the benchmark task is malformed.
        visible = run_lovelace(runner, script, include_functions=False)
        if not visible.get("ok"):
            failures.append((tid, "visible", visible.get("message"), visible.get("diagnostics")))
            continue

        fps: list[str] = []
        task_probes = probes.get(tid, [])
        if not task_probes:
            fps.append(semantic_fingerprint(visible))
        else:
            for i, probe in enumerate(task_probes):
                env = run_lovelace(runner, script + "\n" + probe, include_functions=False)
                if not env.get("ok"):
                    failures.append((tid, f"probe[{i}]", env.get("message"), env.get("diagnostics")))
                    continue
                fps.append(semantic_fingerprint(env))
        out[tid] = fps
        if verbose:
            short = ", ".join(x[:10] for x in fps)
            print(f"OK  {tid}: {len(fps)} hidden probe(s) [{short}]")

    if failures:
        for f in failures:
            print("FAIL", *f, file=sys.stderr)
        raise SystemExit(f"{len(failures)} golden/probe execution(s) failed; corpus is invalid against this runner")

    ORACLES_PATH.write_text(json.dumps(out, indent=2), encoding="utf-8")
    return out


def prepare(args):
    out = build_oracles(args.runner, verbose=True)
    print(f"\nwrote {ORACLES_PATH} ({len(out)} tasks)")


def observe(args):
    if args.task not in TASKS:
        raise SystemExit(f"unknown task {args.task!r}")
    script = Path(args.candidate).read_text(encoding="utf-8") if args.candidate else TASKS[args.task]["seed"]
    # Evaluation observations never dump the full function registry, even in L3.
    env = run_lovelace(args.runner, script, include_functions=False)
    print(json.dumps(project(env, args.mode), indent=2, ensure_ascii=False))


def inspect_functions(args):
    """L3-only style on-demand semantic registry lookup."""
    env = run_lovelace(args.runner, "0", include_functions=True)
    if not env.get("ok"):
        raise SystemExit(env.get("message", "registry lookup failed"))
    q = args.query.casefold()
    funcs = [function_projection(f) for f in env.get("functions", []) if q in str(f.get("name", "")).casefold()]
    total = len(funcs)
    funcs = funcs[: args.limit]
    print(json.dumps({"query": args.query, "totalMatches": total, "returned": len(funcs), "functions": funcs}, indent=2))


def verify_script(
    runner: str, task_id: str, script: str, oracles: dict[str, list[str]]
) -> tuple[bool, dict[str, Any]]:
    """Verify a candidate against hidden functional probes.

    Each probe is appended after the candidate so the candidate's required function definitions
    must survive and generalize beyond the visible example. This prevents a constant visible-answer
    replacement from passing the benchmark.
    """
    probes = load_probes().get(task_id, [])
    expected = oracles[task_id]

    if not probes:
        env = run_lovelace(runner, script, include_functions=False)
        if not env.get("ok"):
            return False, env
        return semantic_fingerprint(env) == expected[0], env

    last: dict[str, Any] = {}
    for probe, expected_fp in zip(probes, expected, strict=True):
        env = run_lovelace(runner, script + "\n" + probe, include_functions=False)
        last = env
        if not env.get("ok") or semantic_fingerprint(env) != expected_fp:
            return False, env
    return True, last


def diag_signature(obs: Any) -> str | None:
    if not isinstance(obs, dict) or obs.get("ok") is True:
        return None
    ds = obs.get("diagnostics")
    if isinstance(ds, list) and ds:
        normalized = []
        for d in ds:
            if isinstance(d, dict):
                normalized.append((d.get("message"), d.get("line"), d.get("column")))
            else:
                normalized.append(str(d))
        return json.dumps(normalized, sort_keys=True)
    return str(obs.get("code") or obs.get("error") or obs.get("message") or "failed")


def percentile_curve(solve_steps: list[int | None], B: int) -> list[float]:
    n = len(solve_steps)
    return [sum(1 for s in solve_steps if s is not None and s <= k) / n for k in range(1, B + 1)] if n else [0.0] * B


def score(args):
    oracles = load_oracles()
    rows = [json.loads(line) for line in Path(args.runs).read_text(encoding="utf-8").splitlines() if line.strip()]
    scored = []

    for r in rows:
        tid = r["task_id"]
        mode = r["mode"]
        events = r.get("events", [])
        candidates = [e for e in events if e.get("type") == "candidate" and isinstance(e.get("script"), str)]
        solve_step = None

        for i, e in enumerate(candidates, 1):
            ok, _ = verify_script(args.runner, tid, e["script"], oracles)
            if ok:
                solve_step = i
                break

        final = r.get("final_script")
        final_ok = False
        if isinstance(final, str):
            final_ok, _ = verify_script(args.runner, tid, final, oracles)
        claimed = bool(r.get("agent_claimed_done"))
        false_convergence = claimed and not final_ok

        obs = [e for e in events if e.get("type") == "observation"]
        sigs = [diag_signature(e.get("observation")) for e in obs]
        sigs = [x for x in sigs if x is not None]
        repeated = sum(1 for a, b in zip(sigs, sigs[1:]) if a == b)
        repeat_rate = repeated / max(1, len(sigs) - 1) if sigs else 0.0

        cum_tokens = 0
        tokens_to_green = None
        cand_seen = 0
        for e in events:
            cum_tokens += int(e.get("input_tokens") or 0) + int(e.get("output_tokens") or 0)
            if e.get("type") == "candidate":
                cand_seen += 1
                if solve_step is not None and cand_seen == solve_step:
                    tokens_to_green = cum_tokens if cum_tokens else None
                    break

        scored.append({
            "mode": mode,
            "task_id": tid,
            "solve_step": solve_step,
            "final_ok": final_ok,
            "false_convergence": false_convergence,
            "repeat_rate": repeat_rate,
            "tokens_to_green": tokens_to_green,
        })

    groups: dict[str, list[dict[str, Any]]] = {}
    for x in scored:
        groups.setdefault(x["mode"], []).append(x)

    B = args.budget
    summary = {}
    for mode, xs in sorted(groups.items()):
        steps = [x["solve_step"] for x in xs]
        curve = percentile_curve(steps, B)
        solved = [s for s in steps if s is not None]
        toks = [x["tokens_to_green"] for x in xs if x["tokens_to_green"] is not None]
        summary[mode] = {
            "n": len(xs),
            "success_rate": sum(s is not None for s in steps) / len(xs),
            "success_at_1": curve[0],
            "success_at_3": curve[min(2, B - 1)],
            "success_at_5": curve[min(4, B - 1)],
            "success_at_10": curve[min(9, B - 1)],
            "median_evaluations_to_green": statistics.median(solved) if solved else None,
            "median_tokens_to_green": statistics.median(toks) if toks else None,
            "repeated_diagnostic_rate": statistics.mean(x["repeat_rate"] for x in xs),
            "false_convergence_rate": sum(x["false_convergence"] for x in xs) / len(xs),
            "H_B": sum(curve) / B,
            "curve": curve,
        }
    print(json.dumps(summary, indent=2))


def validate(args):
    """Execute the complete corpus/protocol validation against a real Lovelace.Run."""
    oracle_map = build_oracles(args.runner, verbose=False)
    accidental = []
    projection_count = 0
    seed_exec_ok = 0
    seed_exec_error = 0
    projection_bytes = {m: [] for m in ("L0", "L1", "L2", "L3")}

    for tid, task in TASKS.items():
        base = run_lovelace(args.runner, task["seed"], include_functions=False)
        if base.get("ok"):
            seed_exec_ok += 1
        else:
            seed_exec_error += 1
        solved, _ = verify_script(args.runner, tid, task["seed"], oracle_map)
        if solved:
            accidental.append(tid)

        for mode in ("L0", "L1", "L2", "L3"):
            obs = project(base, mode)
            encoded = json.dumps(obs, separators=(",", ":"), ensure_ascii=False)
            projection_bytes[mode].append(len(encoded.encode("utf-8")))
            projection_count += 1

    # Smoke the on-demand semantic registry independently of evaluation observations.
    registry = run_lovelace(args.runner, "0", include_functions=True)
    registry_ok = bool(registry.get("ok")) and any(f.get("name") == "det" for f in registry.get("functions", []))

    summary = {
        "ok": not accidental and registry_ok,
        "tasks": len(TASKS),
        "goldens_verified": len(oracle_map),
        "hidden_probes": sum(len(v) for v in load_probes().values()),
        "accidental_seed_matches": accidental,
        "seed_execution": {"ok": seed_exec_ok, "error": seed_exec_error},
        "observation_projections": projection_count,
        "l3_registry_smoke": registry_ok,
        "mean_observation_bytes": {m: round(sum(v) / len(v), 1) for m, v in projection_bytes.items()},
        "max_observation_bytes": {m: max(v) for m, v in projection_bytes.items()},
    }
    print(json.dumps(summary, indent=2))
    if not summary["ok"]:
        raise SystemExit(1)


def make_prompt(args):
    t = TASKS[args.task]
    s = f"""You are repairing a Lovelace Script program.

Task:
{t['prompt']}

Initial script:
```lovelace
{t['seed']}
```

You may evaluate candidate scripts using the benchmark's Lovelace execution tool.
You have at most {args.budget} evaluations.
The evaluator uses additional hidden functional probes, so preserve the requested function contract rather than hard-coding the visible example.
Return the complete final Lovelace script and only claim completion when it is correct.
"""
    print(s)


def main():
    p = argparse.ArgumentParser()
    sp = p.add_subparsers(dest="cmd", required=True)

    q = sp.add_parser("prepare")
    q.add_argument("--runner", required=True)
    q.set_defaults(fn=prepare)

    q = sp.add_parser("observe")
    q.add_argument("--runner", required=True)
    q.add_argument("--mode", choices=["L0", "L1", "L2", "L3"], required=True)
    q.add_argument("--task", required=True)
    q.add_argument("--candidate")
    q.set_defaults(fn=observe)

    q = sp.add_parser("inspect")
    q.add_argument("--runner", required=True)
    q.add_argument("--query", required=True)
    q.add_argument("--limit", type=int, default=20)
    q.set_defaults(fn=inspect_functions)

    q = sp.add_parser("score")
    q.add_argument("--runner", required=True)
    q.add_argument("--runs", required=True)
    q.add_argument("--budget", type=int, default=10)
    q.set_defaults(fn=score)

    q = sp.add_parser("validate")
    q.add_argument("--runner", required=True)
    q.set_defaults(fn=validate)

    q = sp.add_parser("prompt")
    q.add_argument("--task", required=True)
    q.add_argument("--budget", type=int, default=10)
    q.set_defaults(fn=make_prompt)

    args = p.parse_args()
    args.fn(args)


if __name__ == "__main__":
    main()
