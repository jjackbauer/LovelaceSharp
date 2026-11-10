#!/usr/bin/env python3
"""Independent, fail-closed comparison with the C# runner's golden fixtures.

Only fields explicitly declared volatile by Lovelace.Run.Tests/TestSupport.cs
are normalized. This script does not depend on the C# or Rust test frameworks.
"""

import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
RUNNER = ROOT / "rust" / "target" / "release" / (
    "lovelace-run.exe" if sys.platform == "win32" else "lovelace-run"
)
FIXTURES = ROOT / "Lovelace.Run.Tests" / "fixtures"
VOLATILE = {"revision", "elapsed", "elapsedTime", "timings"}


def normalized(node):
    if isinstance(node, dict):
        return {k: "<volatile>" if k in VOLATILE else normalized(v) for k, v in node.items()}
    if isinstance(node, list):
        return [normalized(x) for x in node]
    return node


def differences(want, got, path="$", out=None):
    if out is None:
        out = []
    if isinstance(want, dict) and isinstance(got, dict):
        for key in sorted(want.keys() | got.keys()):
            if key not in want:
                out.append(f"{path}.{key}: unexpected field")
            elif key not in got:
                out.append(f"{path}.{key}: missing field")
            else:
                differences(want[key], got[key], f"{path}.{key}", out)
    elif isinstance(want, list) and isinstance(got, list):
        if len(want) != len(got):
            out.append(f"{path}: expected {len(want)} elements, got {len(got)}")
        for i in range(min(len(want), len(got))):
            differences(want[i], got[i], f"{path}[{i}]", out)
    elif want != got:
        out.append(f"{path}: {str(want)[:100]!r} != {str(got)[:100]!r}")
    return out


def run(args):
    try:
        process = subprocess.run(
            [str(RUNNER), *args], cwd=ROOT, capture_output=True,
            encoding="utf-8", errors="replace", timeout=30,
        )
    except subprocess.TimeoutExpired:
        return None, None, "timeout after 30 seconds", None
    except OSError as exc:
        return None, None, str(exc), None
    try:
        doc, end = json.JSONDecoder().raw_decode(process.stdout.lstrip())
        if process.stdout.lstrip()[end:].strip():
            raise ValueError("additional stdout content after the JSON document")
    except (ValueError, json.JSONDecodeError) as exc:
        doc = None
        parse_error = f"invalid JSON stdout: {exc}; stdout={process.stdout[:250]!r}"
    else:
        parse_error = None
    return process.returncode, doc, process.stderr, parse_error


def check_goldens():
    passed = 0
    paths = sorted(FIXTURES.glob("*.json"))
    for golden in paths:
        name = golden.stem
        script = golden.with_suffix(".ls")
        expected = json.loads(golden.read_text(encoding="utf-8"))
        code, actual, stderr, error = run(["--file", str(script), "--omit-functions"])
        expected_exit = 1 if name == "error_envelope" else 0
        failures = []
        if code != expected_exit:
            failures.append(f"exit code {code}, expected {expected_exit}")
        if error:
            failures.append(error)
        if stderr:
            failures.append(f"stderr not empty: {stderr[:250]!r}")
        if actual is not None:
            failures.extend(differences(normalized(expected), normalized(actual)))
        if not failures:
            passed += 1
            print(f"PASS {name}")
        else:
            print(f"FAIL {name}: {len(failures)} differences")
            for item in failures[:7]:
                print(f"  {item}")
    print(f"GOLDEN SUMMARY: {passed}/{len(paths)} exact matches")
    return passed == len(paths) and len(paths) == 20


def check_cli():
    cases = [
        ("registry listing", ["--functions"], 0, "functions"),
        ("one function", ["--function", "sin"], 0, "function"),
        ("cancellation ledger", ["--eval", "1+1", "--cancel-after", "1000"], 0, "cancellation"),
        ("print budget", ["--eval", "1+1", "--print-budget", "10"], 0, "result"),
        ("plot filename", ["--eval", "1+1", "--plot-file", "audit.svg"], 0, "result"),
        ("missing file is script error", ["--file", "__audit_nonexistent__.ls"], 1, None),
    ]
    passed = 0
    for label, args, expected_exit, key in cases:
        code, doc, stderr, error = run(args)
        okay = code == expected_exit and error is None
        if key is not None:
            okay = okay and isinstance(doc, dict) and key in doc
        if okay:
            passed += 1
            print(f"PASS {label}")
        else:
            print(f"FAIL {label}: exit={code} expected={expected_exit}, "
                  f"key={key}, present={list(doc)[:12] if isinstance(doc, dict) else None}, "
                  f"parse={error}, stderr={str(stderr)[:160]!r}")
    print(f"CLI SUMMARY: {passed}/{len(cases)} checks")
    return passed == len(cases)


if __name__ == "__main__":
    if not RUNNER.is_file():
        print(f"ERROR: release runner does not exist: {RUNNER}")
        sys.exit(2)
    golden_ok = check_goldens()
    cli_ok = check_cli()
    sys.exit(0 if golden_ok and cli_ok else 1)
