#!/usr/bin/env python3
"""Check a trajectory file against the package's run.schema.json.

A dependency-free subset validator: required fields, declared types, the mode
enum and the event-type enum are exactly what bench.py's scorer relies on.
"""
import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
BENCH = HERE.parent
SCHEMA = BENCH / "lovelace-harnessability-public-branch" / "run.schema.json"
TYPES = {"string": str, "integer": int, "boolean": bool, "object": dict, "array": list}


def check_value(value, spec, path, problems):
    if "enum" in spec and value not in spec["enum"]:
        problems.append("%s: %r not in %s" % (path, value, spec["enum"]))
    kind = spec.get("type")
    if kind and kind in TYPES and not isinstance(value, TYPES[kind]):
        problems.append("%s: expected %s, got %s" % (path, kind, type(value).__name__))
    if kind == "integer" and isinstance(value, bool):
        problems.append("%s: boolean is not an integer" % path)
    if "minimum" in spec and isinstance(value, (int, float)) and value < spec["minimum"]:
        problems.append("%s: %r below minimum %r" % (path, value, spec["minimum"]))
    if kind == "object" and "properties" in spec:
        for name, sub in spec["properties"].items():
            if name in value:
                check_value(value[name], sub, "%s.%s" % (path, name), problems)
    if kind == "array" and "items" in spec:
        for i, item in enumerate(value):
            check_value(item, spec["items"], "%s[%d]" % (path, i), problems)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", default=str(BENCH / "runs" / "runs-matrix.jsonl"))
    args = ap.parse_args()
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    rows = [json.loads(l) for l in Path(args.runs).read_text(encoding="utf-8").splitlines() if l.strip()]
    problems = []
    for i, row in enumerate(rows):
        prefix = "run[%d] %s" % (i, row.get("run_id", "?"))
        for name in schema["required"]:
            if name not in row:
                problems.append("%s: missing required field %r" % (prefix, name))
        check_value(row, schema, prefix, problems)
    for p in problems[:20]:
        print("FAIL", p)
    print("%d trajectory object(s) checked against run.schema.json: %s"
          % (len(rows), "all conform" if not problems else "%d problem(s)" % len(problems)))
    return 0 if not problems else 1


if __name__ == "__main__":
    sys.exit(main())
