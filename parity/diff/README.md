# parity/diff — differential gate (C# oracle vs Rust runner)

A fail-closed differential harness. It generates deterministic input, submits the **same**
script and the **same** extra flags to the C# oracle runner and the Rust runner, and compares
their JSON envelopes structurally.

- Dependency-free: Node 24 built-ins only (`node:child_process`, `node:crypto`, `node:fs`, `node:path`).
- It does not build, patch, or skip either runner. A mismatch is a measurement, not a defect to hide.
- It never edits `rust/` or any C# source.

## Quick start

```
node parity/diff/run.mjs --cases 20 --seed 1
node parity/diff/run.mjs --list
node parity/diff/run.mjs --selftest
node parity/diff/run.mjs --cases 5 --seed 7 --out parity/diff/runs/demo --no-minimize
node parity/diff/run.mjs --regressions
```

## CLI

| Option | Meaning | Default |
|---|---|---|
| `--cases N` | **exactly N generated cases** (asserted); **0 exits 2** | 200 |
| `--seed S` | integer seed for the generator | 1 |
| `--concurrency K` | parallel case workers | 6 |
| `--out DIR` | run directory | `parity/diff/runs/<utc-stamp>-seed<S>` |
| `--filter TEXT` | only cases whose id contains TEXT | — |
| `--no-minimize` | skip minimisation (the full input is still recorded) | off |
| `--regressions` | run only `parity/diff/regressions/*.json` | off |
| `--list` | print the category list and exit 0 | — |
| `--selftest` | prove the harness can fail; exit 0 only if it can | — |
| `--help` | usage | — |

**Exit codes** (run.mjs:22-24):

| Code | Meaning |
|---|---|
| 0 | every executed case matched and both binaries executed |
| 1 | at least one mismatch (generated case or replayed regression) |
| 2 | harness/config failure: missing binary, spawn failure or timeout, `--cases 0`, unreadable/malformed regression, unknown option, empty corpus, failing selftest |

A process-level failure (spawn error or the 30 s per-process timeout) is counted as `error`
and forces exit 2: the gate never reports green when it could not execute both binaries
(DIFF-010).

## What `--cases N` means (exact accounting)

`--cases N` is the size of the **generated corpus** — exactly N cases, not a floor and not a
per-category approximation. The generator apportions N over the weights and then asserts the
total (run.mjs:643-650); a drift is a harness failure (exit 2), never a quietly different run.
With `--filter` the generated ledger is the filtered subset of those N; with `--regressions`
nothing is generated and the stored corpus is replayed instead.

Total executions are **generated + replayed regressions**, and the two are reported separately
everywhere, because DIFF-009 counts comparisons actually performed:

- the console prints an `EXECUTION LEDGER` block and **two** category tables — `generated corpus
  by category` (ending in `GENERATED`) and `regressions replayed by category` (ending in
  `REPLAYED`);
- `manifest.json` `counts`/`categories` describe the generated corpus only, and
  `regressionCounts` + `regressions.counts` describe the replay;
- `monitor.json` has separate `generated` and `regressions` blocks.

A regression whose id, script and args already appear in this run's generated corpus is executed
once, as the generated case; it is counted under `duplicatesSkipped`, not double-run.

## Normalisation — exactly four fields

The **only** normalised fields are:

```
revision, elapsed, elapsedTime, timings
```

Each is replaced, **at any depth**, by the literal string `"<volatile>"`
(compare.mjs:11-19, compare.mjs:34-46). Nothing else is normalised or omitted.

Consequences, by design:

- Key **presence** is still part of the contract. If the oracle emits `elapsedTime` and the
  Rust runner does not, that is a `missing-key` mismatch — the volatile rule replaces the
  value of a present field, it does not excuse an absent one. Widening this would hide a real
  wire difference and is forbidden by the goal.
- Only the four names are volatile. `artifacts`, `functions`, `diagnostics`, `elapsedTime.unit`
  and every other field are compared literally.

## Generator (generator.mjs)

Deterministic from `--seed`: no `Math.random`, no clock, no locale-dependent formatting, no
I/O. Each category owns a `mulberry32` stream seeded by `(seed, category name)`
(generator.mjs:38-72), so a category's cases are a stable prefix independent of `--cases`.

Category quotas come from the documented weights by largest remainder (generator.mjs:177-193);
cases are emitted round-robin so a small `--cases` still covers the categories that receive a
quota. A case is `{ id, category, script, args: [] }` where `args` are extra runner flags.

Case ids are `g-s<seed>-<category>-<template>-<occurrence>` (generator.mjs:240). The id does
**not** depend on `--cases` (a 20-case regression still names the same input in a 5 000-case
run of the same seed) and **does** include the seed (two seeds never collide on one regression
filename).

### Weights

| Category | Weight | Covers |
|---|---|---|
| arithmetic | 18 | natural/integer/real, huge values (`3^114`, `2^200+…`), division, powers, modulo, precedence |
| precision | 10 | `setprecision(8/16/20/50/1000)`, `pi(n)`, `e(n)`, `evalf` |
| arrays | 16 | vector, matrix, reshape/transpose, dot/matmul/det/inv/linsolve, empty and singular shapes |
| symbolic | 18 | `symbol`, diff, integrate, solve, simplify, series, limit, factor, expand, evaluate |
| records | 14 | solve_full, integrate_full, limit_full, inv_full, linsolve_full, optimize_full, compile_full |
| errors | 12 | division by zero, unknown function, wrong arity, parse error, invalid index, domain error |
| syntax | 7 | comment attempts, CRLF, BOM, unicode identifiers/strings, multiline, tabs |
| registry | 5 | `type`, `inspect`, `capabilities`, `assumptions`, and the builtin registry payload |

### Flags: only what the oracle accepts

`args` may contain only `--omit-functions`, `--omit-variables`, `--print-budget <n>` — verified
against the oracle's own `--help`. The generator throws on any other flag
(generator.mjs:30, generator.mjs:209-216). The gate never passes a flag the oracle rejects.

Two facts worth knowing before editing templates:

- The brief's example flag `--precision` **does not exist**. The oracle answers
  `Error: Unknown argument '--precision'.` and exits 2. Precision is exercised through the
  script surface (`setprecision`, `pi`, `e`, `evalf`), never through a flag.
- The script language has **no comment syntax**: the tokenizer has no comment branch
  (`Lovelace.Suite/Tokenizer.cs:22-125`) and `%` is the modulo operator. The `syntax` templates
  therefore assert the observable behaviour of a comment *attempt* (a ParseError) rather than a
  working comment.

Evaluation categories pass `--omit-functions` so the 40 KB builtin registry does not dominate
every payload; the `registry` category deliberately runs without it on some cases so the
registry payload itself is compared. Both runners always receive identical args for a case.

## Comparator (compare.mjs)

Compares `{ exitCode, envelope }` after normalisation, in this precedence:

1. **exit-code** — the two exit codes differ (path `$.exitCode`).
2. **invalid-json** — stdout is not JSON on either side. The record includes the first 200
   characters of the offending stdout; this is a mismatch, never a crash (compare.mjs:148-215).
3. **envelope** — structural comparison of the normalised envelopes.

Structural rules: object key **order is irrelevant** (keys are sorted during normalisation);
an added or removed key is a mismatch; a changed scalar is a mismatch; array length and array
order matter. Scalars compare by JSON value, so `1` and `1.0` are equal.

A **stderr difference is not a mismatch by itself**; only a differing exit code makes it one.
stderr length is recorded for both runners in every case record and mismatch file.

Each mismatch records `firstDifference` (path, kind, expected, actual) plus up to 100 differing
paths in `differences` and the exact `differenceCount`. The first difference is stable and
deterministic: keys are visited in sorted order, arrays in index order.

## Minimiser (minimize.mjs)

On a mismatch the script is reduced by delta debugging: **lines first**, then
**whitespace-separated tokens**. A reduction is kept only while a re-run reproduces **both**
the same mismatch category **and** the same first differing JSON path. Every candidate costs
one evaluation (one oracle run + one Rust run); the budget is **40 evaluations per case**
(minimize.mjs:9). The minimisation result is stored in the mismatch record as
`{ original, minimized, category, firstDifference, exitCodes, evaluations, changed }`.

## Artifacts (inside --out)

| File | Contents |
|---|---|
| `manifest.json` | `schemaVersion`, `startedAtUtc`, `finishedAtUtc`, `seed`, `cases`, `executed`, `concurrency`, `oracle {command, sha256}`, `rust {command, sha256}`, `normalization`, `counts`, `categories`, `harnessExit`, `corpusSha256`, `regressions` |
| `monitor.json` | per-category table: cases/match/mismatch/error, mismatch rate, stderr bytes, top first differences |
| `mismatches/<caseId>.json` | case, both exits, stdout/stderr byte counts and previews, classification, `differences`, minimisation |
| `cases.jsonl` | one line per executed case (status, exits, stderr bytes, first-difference path) |
| `corpus.json` | the exact generated corpus, for byte-level reproducibility checks |

`counts` covers the **generated** corpus; `regressionCounts` covers the replay; `executed` reports
generated / regressions / total. A replayed regression is a real execution, so its mismatch count
still drives the exit code — it is simply never confused with `--cases`.

## Regressions (regressions/)

Every generated mismatch is promoted to `regressions/<caseId>.json`:

```json
{
  "id": "...",
  "category": "...",
  "script": "...",
  "args": [],
  "observed": { "oracleExit": 0, "rustExit": 0, "firstDifference": { "path": "...", "kind": "...", "expected": "...", "actual": "..." } },
  "addedAtUtc": "..."
}
```

They are replayed automatically at the start of **every** run (DIFF-008). A regression that no
longer mismatches is reported under `regressions.resolved` and is **never deleted**: losing the
input would lose the case that once failed. A regression whose case is already in this run's
generated corpus (same id, script and args) is not executed twice — the generated copy covers it.
Existing regression files are never rewritten, so `addedAtUtc` stays truthful.

## Selftest

`--selftest` proves the gate is falsifiable in both directions. It runs six probes as real
child invocations of this CLI, each of which spawns the real binaries:

- **control-oracle** and **control-rust** — a runner compared with itself must MATCH and exit 0.
  If the comparator were "always mismatch", these fail.
- **flip-ok**, **drop-key**, **change-scalar**, **reorder-array** — the same envelope with one
  injected corruption must be reported as a mismatch at the expected path and exit 1. If the
  comparator were "always match", these fail.

The selftest exits 0 only if both controls matched and all four corruptions were detected with
exit code 1; otherwise it exits 2. It is independent of whether the two runners currently agree,
so it stays meaningful while the gate is red.

## Known state (snapshot 2026-11-10T14:52Z)

With oracle sha256 `f292249f…` and rust sha256 `88f008da…`, seed 1, 20 generated cases:
**9 match / 11 mismatch / 0 error**, exit 1. The 5 replayed seed-7 regressions add 3 match /
2 mismatch. First differences span seven paths (`$.result.structured.fields[0].value.kind`,
`$.category`, `$.result.display`, `$.result.structured.denominator`, `$.code`, `$.functions`),
so the gate discriminates rather than reporting one universal envelope gap.

**The runners are a moving target.** During this session the Rust binary changed three times and
the oracle DLL was rebuilt (with no C# source change). Always read the sha256 a run prints before
comparing two runs; the selftest is independent of that drift. Details and observed output:
`docs/goal-parity-contract/round-1/diff-harness.md`.
