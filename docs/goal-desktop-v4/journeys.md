# Mandatory journeys — executable specification

CON-007 and TST-001 require each of the 22 named journeys to have an input, an action sequence, an
expected observable and a reproducible artifact. This file is the executable specification; the
`Status` column says what is required before the journey can be run against the product window. A
journey is never marked green here — the gate does that, with an artifact.

Reference data for the web-side half lives in [`reference/`](reference/) and was captured by
`tools/reference-capture.ps1`. `REF-*` names below point at those captures.

| # | Journey | Input | Actions | Expected observable | Reference | Status |
|---|---|---|---|---|---|---|
| J-01 | First-run | no arguments | launch → open the built-in example → Run | editor, typed result and status correct within two primary actions | — | needs first-run affordance + example document |
| J-02 | Basic script | `a = 2` / `b = a^10` / `b` | type, Run | result `1024`, variables `a`,`b`, log line, cursor position | `REF-basic` | **PASS (round 33)** via `--journey J-02`; window capture [round-33/J-02.png](round-33/J-02.png) + [UIA](round-33/J-02-uia.txt) |
| J-03 | Precision | `setprecision(20)`; `sqrt(2)`; `setprecision(1000)`; `sqrt(2)` | Run, change precision control, Run | full stored digits at each precision, visible precision, cache invalidation | `REF-precision`, `REF-digits` | **PASS (round 33)** via `--journey J-03`: 19 fractional digits at 20, 1000 at 1000, `reused=0` after the change; [log](round-33/journey-run.log) |
| J-04 | Session isolation | tab A `x = 10`, tab B `x = 99`, different precision | switch tabs during a run | no state leak; each tab's workspace matches its engine | — | needs editable second tab + per-tab engines |
| J-05 | Concurrent evaluation | long script in A, short script in B | start A, run B, cancel A | UI responsive, B result correct | — | needs two live documents + concurrent hosts |
| J-06 | Real cancellation | cancellation-aware expensive script | Run, Cancel | underlying work stops, terminal event, no spinner | — | needs a long kernel and the Cancel path |
| J-07 | Incremental reuse | `a = 2`; `b = a + 1`; `c = pi(100)`; edit only the last statement | Run twice | reused count > 0, identical values, reused never presented as recomputed | `REF-incremental` (`reused=3`) | needs the incremental cache (INC) |
| J-08 | Dependency invalidation | change `a`, or redefine `func f` | Run twice | transitive dependents recompute, independent statements reuse | — | needs the incremental cache (INC) |
| J-09 | Side effects | `print` and `plot` twice | Run twice | each run executes its effects, no duplicate lines within a run | `REF-stream`, `REF-plot` | **PASS (round 33)** via `--journey J-09`: each run prints `alpha`/`beta` once and writes a fresh plot; [log](round-33/journey-run.log) |
| J-10 | Clear semantics | `func f(x) = x + 1`; `y = 41`; `f(y)` then Clear Variables | Run, clear | `y` removed, `f` retained | `REF-clear` | **PASS (round 33)** via `--journey J-10`: 2 variables removed, 124 functions kept (`f` still user-defined); [log](round-33/journey-run.log) |
| J-11 | Autocomplete | type `sq` | observe completion list | `sqrt` with signature; user symbols appear after a run | completions catalog (132 items) | needs the completion surface |
| J-12 | Error navigation | multiline script with a Unicode error | Run | exact line/column highlighted, code/category readable, jump to source | `REF-error` | partly: gui-selftest proves the diagnostic text, not navigation |
| J-13 | Structured mathematics | symbolic simplify, matrix inverse, solve, 1000-digit number | Run, inspect | exactness metadata preserved, no re-parse of display strings | `REF-symbolic`, `REF-matrix`, `REF-solve` | needs the structured result renderer |
| J-14 | MathIR | `compile(x^2 + 1, [x])` | Run, inspect IR, copy | byte-preserved IR; editing source does not re-evaluate | `REF-matir` | needs the inspector |
| J-15 | Plot presentation | `plot(1..10, (1..10)^2)` | Run, zoom, pan, fit, export | engine SVG bytes, crisp at HiDPI, SVG/PNG export | `REF-plot` (15 000-byte SVG) | **PASS (round 33)** via `--journey J-15`: 15 000 bytes, census 2/13/11/1, zoom/pan/fit, 800×600 + 1600×1200 rasters, byte-identical SVG export; window capture [round-33/J-15.png](round-33/J-15.png) |
| J-16 | File safety | open a file, edit, external modify, resolve | save/reload | line endings preserved, no silent overwrite | — | needs native file lifecycle (DOC-002/006) |
| J-17 | Crash recovery | edit an unsaved tab, kill the process, relaunch | recover | intact text, tabs and layout restored, nothing auto-runs | — | needs the recovery journal (DOC-005) |
| J-18 | Theme/scale | — | switch light/dark, 100/150/200 % scale, 900×600 | no cropped primary actions or unreadable text | — | needs the light theme + DPI matrix |
| J-19 | Accessibility | — | keyboard-only open, edit, run, inspect, export | screen reader announces status/progress/diagnostics | — | needs keyboard map + UIA announcements |
| J-20 | Stress | 500 KB script, 100k log lines, 1 MB SVG | measure | bounded latency and memory | — | needs the stress fixtures and budgets |
| J-21 | Offline | — | disable the network, run all primary workflows | no listener, no external request | — | needs an offline run of the release build |
| J-22 | Release evidence | clean pinned SHA | build, run the GUI suite, save evidence | manifest with OS/SHA/toolchain/results/screenshot hashes | — | needs `tools/release-manifest.ps1` |

## Running rules

- A journey's result is recorded as a `gui` artifact (screenshot, UIA dump or driver log) bound to the
  commit under test; a headless `--gui-selftest` line alone is not a journey pass.
- The reference column is the web-side expectation captured from the live Studio; where the oracle is
  defective the deviation is recorded in [`deviations.md`](deviations.md) and the contract wins.
- Journeys blocked on an unbuilt feature stay `NOT_TESTED` in the gate; they are never "covered" by a
  neighbouring journey that happens to pass.
