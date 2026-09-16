#!/usr/bin/env python3
"""Verify the RUN-REPORT.md verifier invariants against the reconstructed corpus.

Each case states whether the hidden-probe verifier should ACCEPT the program:
a hard-coded visible answer must be rejected, an equivalent implementation must
be accepted, and a correct function without the visible sample call must pass.
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

CASES = [
    ("hard-coded factorial visible answer", False, "factorial-off-by-one",
     "func factorial(n) { 720 }" + chr(10) + "factorial(6)"),
    ("hard-coded determinant visible answer", False, "undefined-name",
     "func matrix_det(m) { -2 }" + chr(10) + "matrix_det([[1,2],[3,4]])"),
    ("equivalent determinant implementation", True, "undefined-name",
     "func matrix_det(m) { det(transpose(m)) }" + chr(10) + "matrix_det([[1,2],[3,4]])"),
    ("equivalent exact-third implementation", True, "exact-third",
     "func third_of(n) { n * (1/3) }" + chr(10) + "third_of(1)"),
    ("fibonacci without visible sample call", True, "fib-base-case",
     "func fib(n) { if (n < 2) { n } else { fib(n-1) + fib(n-2) } }"),
    ("golden accepted on a silent-failure task", True, "matrix-trace",
     "func matrix_trace(m) { trace(m) }" + chr(10) + "matrix_trace([[2,1],[5,7]])"),
]


def main():
    oracles = json.loads((CTRL / "private" / "oracles.json").read_text(encoding="utf-8"))
    passed = 0
    for label, expected, tid, script in CASES:
        ok, env = bench.verify_script(RUNNER, tid, script, oracles)
        good = ok == expected
        passed += good
        note = "" if ok else "  [%s]" % env.get("message")
        print("%s  %s: accepted=%s (expected %s)%s" % ("PASS" if good else "FAIL", label, ok, expected, note))
    print()
    print("verifier invariants: %d/%d hold" % (passed, len(CASES)))
    return 0 if passed == len(CASES) else 1


if __name__ == "__main__":
    sys.exit(main())
