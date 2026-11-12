# Final state — desktop-v4 (round cap reached: 30/30)

- **Gate**: FAIL [READINESS]. No release claim is made.
- **P0**: 47/171 PASS, 1 waived (TST-012, user decision DEC-006), 0 FAIL, 123 NOT_TESTED.
- **Journeys**: 6/22 PASS (J-02, J-03, J-10, J-11, J-12, J-15).
- **Dimensions**: 0/19 MET. Scorecard: functional 19/54, visual 24/97, performance 4/20.
- **Stale evidence**: 12 rows were re-bound to current artifacts at HEAD a4167786; 2 mutable
  driver logs were dropped from evidence (LAY-008, PLT-011) - evidence logs must be frozen.
- **Tests**: `cargo test -p lovelace-desktop` 355 passed, 0 failed. `--gui-selftest` 19 checks;
  plot checks need APPDATA outside the cwd in this sandbox.
- **Open defects**: none severity 1; none open at all after DEF-001..004 were fixed.

## What exists (verified)
Native egui workbench: tabs, virtualised syntax-highlighted editor with gutter, diagnostics
decoration, completion from the engine registry, undo/redo, quick eval, typed logs, engine-backed
variables/functions tables, structured results with per-field expand/collapse, symbolic inspector
(pretty/canonical/tree/assumptions/trace/MathIR), resvg plots with zoom/pan/fit and byte-identical
SVG export plus scaled PNG export, native file open/save with BOM+newline fidelity, session restore,
crash-recovery journal. Real engine on workers; ~20 ms kernel cancel; ~1 ms Running; coherent
committed snapshots; AccessKit UIA tree.

## Not done (the honest gap)
123 P0 rows untested: TST 15, RUN 13, EDT 10, A11Y 10, VIS 10, PLF 9, INC 9, SYM 8, and others.
16 journeys unrun. Release gate (TST-016 provenance, clean-tree, release build) not attempted.
Linux/macOS not ported (DEC-006). Human usability waived (DEC-006).

## Next objectives for a continuation
1. INC (incremental cache, 9 rows) - the last large semantic subsystem.
2. RUN remaining (queued/duplicate policy, RUN-010/011/012/013 evidence from the window).
3. VIS light theme + contrast validation; A11Y focus/keyboard/scale rows.
4. Journeys J-01/J-04..J-09/J-13/J-14/J-16..J-22.
5. TST-016 release manifest, clean-tree build, then -Verify.