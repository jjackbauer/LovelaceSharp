# Round 5 — cutover: the shipped binary is the native workbench

**Status:** green. The default `lovelace-desktop` binary opens the new eframe/egui 0.37
workbench, runs the real `lovelace_lang::Engine` on a worker through the tested
`ExecutionHost`, and exposes the old Win32 EDIT shell only via `--legacy-win32`. The existing
`--selftest` still passes 15/15, and the new `--gui-selftest` passes 7/7. The unit suite is
90/90 (81 pre-existing + 9 new).

Scope touched:

| Path | Change |
|---|---|
| `rust/lovelace-desktop/Cargo.toml` | Windows-only `eframe`/`egui` 0.37 and `resvg` 0.50 dependencies |
| `rust/lovelace-desktop/src/main.rs` | four-way dispatch: workbench (default), `--gui-selftest`, `--selftest`, `--legacy-win32` |
| `rust/lovelace-desktop/src/workbench.rs` | **new** headless document/run model over `ExecutionHost` |
| `rust/lovelace-desktop/src/gui_selftest.rs` | **new** `--gui-selftest` checks on the real engine |
| `rust/lovelace-desktop/src/ui/mod.rs` | **new** workbench window module |
| `rust/lovelace-desktop/src/ui/app.rs` | **new** toolbar, tabs, panes, splitters, status, logs, workspace |
| `rust/lovelace-desktop/src/ui/editor.rs` | **new** single-buffer adaptation of the spike editor |
| `rust/lovelace-desktop/src/ui/syntax.rs` | **new** copy of the measured syntax layouter |
| `rust/lovelace-desktop/src/ui/plot.rs` | **new** engine-SVG plot pane with an empty state |
| `rust/lovelace-desktop/src/engine.rs` | `Evaluator::set_plot_dir` + shared writable `default_plot_dir()` |
| `rust/lovelace-desktop/src/engine/real.rs` | forwards `set_plot_dir` to `lovelace_lang::Engine` |
| `rust/lovelace-desktop/src/app.rs` | pre-cutover documents also get the writable plot directory |
| `rust/lovelace-desktop/src/execution.rs` | keeps the last plot a run produced; two test-only accessors allowed |
| `rust/Cargo.lock` | generated: 451 external packages locked from the local registry cache |
| `docs/goal-desktop-v4/round-5/cutover.md` | this report |

`rust/lovelace-desktop-spike/**`, `rust/lovelace-lang/**`, `rust/lovelace-run/**`,
`rust/Cargo.toml` and every C#/web file were **not** modified by this session.

---

## 1. What moved

The spike's measured UI was copied into the product crate and adapted, not rewritten from
memory:

| Spike module | Product module | Adaptation |
|---|---|---|
| `editor.rs` | `ui/editor.rs` | one `TextEditor` bound to a workbench document instead of the spike's owned sample-doc list; gutter, syntax colours, current-line highlight, caret, virtualised rows and Ln/Col are the spike implementation |
| `syntax.rs` | `ui/syntax.rs` | same palette/layouter, keyword list extended with `assume`/`pi` |
| `plot.rs` | `ui/plot.rs` | loads only the engine's `PlotCapture.svg`; no sample SVG, no malformed-SVG toggle; renders the empty state when no run produced a plot; keeps wheel zoom, drag pan, Fit and resvg raster metrics |
| `app.rs` (layout/panes) | `ui/app.rs` | same pane rectangles, splitters and accessibility pane labels; toolbar, tab strip, workspace, logs and status strip now read `Workbench` instead of sample vectors |
| `sample.rs` | — | **not copied**; its 5 000-line fixture, variables and SVG are not in the shipped path |
| `bench.rs`, `json.rs`, `cpu.rs` | — | not copied; no benchmark or fake data path in the product |

New headless code:

* `workbench.rs` owns two or more documents, one `ExecutionHost`, per-document `DocumentId`,
  source revision, precision, `EventSink`, last `WorkspaceSnapshot`, last `PlotCapture`, logs
  and run handle/phase/outcome. It is the single run path used by both the window and
  `--gui-selftest`.
* `gui_selftest.rs` creates a real-engine workbench, edits sources, calls `run_active`,
  polls the typed events and asserts on the published state. It never evaluates inline.

---

## 2. What was wired

**Per-document state.** Two tabs at start (`welcome.lv` with an ordinary editable starter
script, `untitled.lv` empty). Each has its own `DocumentId`, source/revision, precision,
`EventSink`, snapshot, last plot and logs. `Workbench::run_active` submits only the active
document; a run in another tab does not mutate it. `--gui-selftest`'s `two-tabs` check proves
the two ids and independent snapshots.

**Run path.** Run → `Workbench::run_active` → `ExecutionHost::submit`, which slices the source
and spawns `lovelace-run-<doc>-<run>`. The UI thread never calls `eval`; it calls
`Workbench::poll` at the top of each frame and requests a 16 ms repaint only while a run is in
flight. `Start`/`StepCommitted`/`Output`/`SubProgress`/`Finished`/`Cancelled`/`Failed` events
update the document that owns the event.

**Cancel.** The toolbar shows a red **Cancel** button while the active document's phase is
`Running`/`Cancelling`; it calls `ExecutionHost::cancel`. The cancellation token is checked at
every statement boundary. On the terminal `Cancelled` event the worker's last snapshot is
adopted, so committed variables/functions/plot remain visible; the status strip says
`run Cancelled` and the log records the committed statement count.

**Precision.** The toolbar's real `DragValue` control updates the active document's precision
(1…100 000). The value is carried by `ExecutionHost::submit`, and the worker calls the real
engine's `set_precision` before statement 1. `--gui-selftest` proves the effect on rendered
digits: `pi` renders `3.14159265` at 8 and `3.14159265358979323846` at 20.

**Logs.** Per document: run request/start, every `print()` line (raw, no prefix), every
diagnostic (with line/column), the `= result` line and a terminal line
(`Run finished…`, `Run cancelled…`, `Run failed…`). Logs are capped at 500 entries.

**Plot.** The plot pane loads the active document's `PlotCapture.svg` into the measured
resvg scene and shows a UIA `Image` node (`SVG plot canvas, 640 by 400 points, zoom 1.00`).
It shows `No plot yet. Run a script that calls plot(...).` when there is no capture. The last
plot per document is kept when a newer run produces none.

**Empty states.** Workspace, functions, logs and plot each render an explicit empty message
before the first run; no sample variables/log lines/SVG exist in the default UI.

**Plot directory.** The round-5 window run found that the engine's default
`%TEMP%\lovelace-plots\plot.svg` is not writable from the product process in this environment
(`Access is denied`), which made plot statements fail. The workbench now probes a writable
per-user application directory (falling back to a writable temp/workspace directory) and gives
each run engine its own `run-N` directory through the new `Evaluator::set_plot_dir`. The
pre-cutover `--selftest` documents use the same shared helper.

**Plot retention.** The real engine resets its captured plot at the start of every `eval`, and
`ExecutionHost` evaluates one statement at a time. The worker previously overwrote its plot
progress with `None` on the next statement. It now keeps the last `Some(plot)` seen during the
run, so a plot statement followed by arithmetic still reaches the snapshot.

**Legacy shell.** `--legacy-win32` is the only path to `win32::shell::run`; the default path
calls `ui::run`. `--selftest` and its 15 checks were kept unchanged and re-observed green.

---

## 3. A1–A4 commands and observed outputs

Environment used verbatim:

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:TMP="$repo\rust\target-desktop\tmp"; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR="$repo\rust\target-desktop"
```

### A1 — `cargo test -p lovelace-desktop` — PASS

```powershell
& "$repo\.toolchain\cargo\bin\cargo.exe" test -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"
```

Observed (final run, no warnings):

```text
running 90 tests
...
test result: ok. 90 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.60s
A1_EXIT=0
```

81 is the pre-existing baseline observed before this session
(`test result: ok. 81 passed; 0 failed; ... finished in 0.60s`). The nine new tests are the
`workbench::tests::*` cases:

```text
starts_with_two_independent_document_tabs
a_finished_run_publishes_result_steps_variables_and_logs
print_output_reaches_the_log_and_the_snapshot
cancellation_keeps_the_committed_prefix
a_failed_run_yields_a_diagnostic_and_returns_to_idle
precision_is_document_scoped_and_clamped
tab_isolation_running_one_document_does_not_touch_the_other
a_plot_from_an_earlier_statement_survives_later_statements
stale_events_from_an_abandoned_run_are_discarded
```

### A2 — `cargo build -p lovelace-desktop` — PASS

```powershell
& "$repo\.toolchain\cargo\bin\cargo.exe" build -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"
```

Observed:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.27s
A2_EXIT=0
```

(PowerShell renders cargo's stderr as a `NativeCommandError` record; the process exit code is
0 and there are no compiler warnings. A clean rebuild took 8.69 s and produced the same exit.)

### A3 — `lovelace-desktop.exe --gui-selftest` — PASS

```powershell
& "$repo\rust\target-desktop\debug\lovelace-desktop.exe" --gui-selftest
```

Observed:

```text
GUI_SELFTEST lovelace-desktop 0.1.0 engine=lovelace-lang
GUI_SELFTEST ok two-tabs: document ids DocumentId(1)/DocumentId(2), precision 1000, independent snapshots
GUI_SELFTEST ok arithmetic: result 12, steps 3, b = 12 (Natural), status [welcome.lv  |  rev 1  |  precision 1000  |  Ln 1, Col 1  |  run Finished]
GUI_SELFTEST ok print: output ["x"], result 2, committed 2
GUI_SELFTEST ok precision: pi at 8 = 3.14159265, at 20 = 3.14159265358979323846 (engine set_precision via submit)
GUI_SELFTEST ok cancel: cancelled after 3 of 50 statements in 1.63 ms (first committed 1)
GUI_SELFTEST ok syntax-error: line 2, column 4: Unexpected token '' at position 3: expected a number, string, identifier, '[', or '('. (outcome Some(Failed), phase Idle)
GUI_SELFTEST ok plot: 334 SVG bytes, title "selftest", 1 polyline with 3 points
GUI_SELFTEST summary checks=7 failed=0
A3_EXIT=0
```

The failing path was also observed: before the writable-plot fix, the same command with the
default process temp printed
`GUI_SELFTEST FAIL plot: the engine produced no plot capture` and exited `EXIT=1`.

### A4 — window liveness and title — PASS

```powershell
$exe="$repo\rust\target-desktop\debug\lovelace-desktop.exe"
$p = Start-Process -FilePath $exe -PassThru
Start-Sleep -Seconds 5
$p.Refresh()
# inspect $p.HasExited, $p.MainWindowTitle, $p.Responding, then:
Stop-Process -Id $p.Id
```

Observed:

```text
A4 alive=True exited=False pid=25872 title=[Lovelace Desktop] responding=True hwnd=33230170
A4 stopped=True
```

One earlier A4 attempt read an empty `MainWindowTitle` at exactly 5 s while the process was
alive and responding; a 0.5 s-sampled run showed the title by 1.45 s, and the repeated final
A4 run above returned `Lovelace Desktop`. This is recorded as an environment-startup
observation, not hidden.

Supplementary window observations (UIA + PrintWindow, same final binary; the orchestrator will
capture its own screenshots/UIA):

* The UIA tree has 57 named nodes after a run (`final-window-run.txt`), including
  `Pane | Editor - welcome.lv`, `Edit | Script editor, 6 lines, caret line 1 column 1`,
  `Pane | Plot - engine SVG`, `Image | SVG plot canvas, 640 by 400 points, zoom 1.00`,
  `Pane | Workspace`, `Pane | Logs` and the status strip.
* Default run of the starter document:
  `welcome.lv | rev 0 | precision 1000 | Ln 1, Col 1 | run Finished`;
  variables `_ = 12 (Natural)`, `a = 2 (Natural)`, `b = 12 (Natural)`; logs
  `Run started: 4 statements, engine lovelace-lang, precision 1000.`, `= 12`,
  `Run finished: 4 statements committed in 9 ms.`
* Cancel on a 30 000-statement document: after 300 ms the status was
  `run Running` and `Cancel present while running: True`; after invoking Cancel the status was
  `run Cancelled`, three variables remained visible, and the log read
  `Run cancelled after 4122 statements in 347 ms; the committed statements remain in the workspace.`
* The explicit fallback still opens: `--legacy-win32` produced the distinct title
  `Lovelace Desktop - Untitled 1` while alive, then was stopped.

### Preserved `--selftest` — PASS

Observed after the plot-directory fix, with the default process temp (the stricter case):

```text
SELFTEST ok new-document
...
SELFTEST ok engine-plot
SELFTEST ok no-network-imports
SELFTEST summary checks=15 failed=0
SELFTEST_EXIT=0
```

---

## 4. `--gui-selftest` check table

| Check | Asserts | Observed detail |
|---|---|---|
| `two-tabs` | two tabs, distinct `ExecutionHost` ids, empty panes, run isolation | ids 1/2, precision 1000, independent snapshots |
| `arithmetic` | `a = 2\nb = a + 10\nb\n` ends result `12`, committed 3, variables contain `b`; run start/result/finish logged; status has revision, precision, cursor, run state | result 12, steps 3, `b = 12 (Natural)`, status `rev 1 / precision 1000 / Ln 1, Col 1 / run Finished` |
| `print` | `print("x")\n1+1\n` yields print output and result `2`; the print line and terminal result line are in the log | output `["x"]`, result 2, committed 2 |
| `precision` | the control changes what the engine renders: `pi` at 8 and 20 digits | `3.14159265` vs `3.14159265358979323846` |
| `cancel` | 50 statements run on the real engine; wait for one commit, cancel, terminal `Cancelled` within 2 s, committed prefix retained, status says `Cancelled` | cancelled after 3/50 in 1.63 ms; snapshot committed steps match; `v0` visible |
| `syntax-error` | malformed statement produces terminal `Failed` with a positioned diagnostic, returns to idle, log records diagnostic and failure | `line 2, column 4: Unexpected token '' at position 3: …`, phase `Idle` |
| `plot` | the engine's own SVG is captured and parses as the expected polyline | 334 SVG bytes, title `selftest`, 1 polyline with 3 points |

The helper `run_to_idle` calls `Workbench::run_active` and `Workbench::wait_until_idle`, the
same methods the window calls; it does not evaluate source or construct engine output itself.

---

## 5. Test-first / failure evidence

* Baseline before the cutover: `running 81 tests` … `test result: ok. 81 passed`.
* The plot-retention test was added before its fix and observed red:
  ```text
  test workbench::tests::a_plot_from_an_earlier_statement_survives_later_statements ... FAILED
  thread ... panicked at lovelace-desktop\src\workbench.rs:
  the plot produced by statement 1 must survive statements 2 and 3
  test result: FAILED. 0 passed; 1 failed; ...
  ```
  The `ExecutionHost` worker now keeps the last `Some(plot)` and the test is green.
* The default-temp plot failure was observed as
  `GUI_SELFTEST FAIL plot: the engine produced no plot capture`, exit 1; the writable
  `set_plot_dir` path was added and `--gui-selftest` plus `--selftest` (with default temp) now
  pass.
* No existing test was deleted, ignored, filtered or loosened. The only edits to
  `execution.rs` are the plot-retention change and `#[allow(dead_code)]` on two test-only
  accessors; all 81 original tests pass.

---

## 6. Limits / not wired yet

Named per UI feature, so none of these is mistaken for delivered behaviour:

1. **File operations are absent in the new window.** No New/Open/Save/Save As/Close, no file
   dialogs, no recent files and no dirty-close guard. A path argument opens a file in tab one;
   otherwise the two startup tabs are the only tabs. The old shell still has these behind
   `--legacy-win32`.
2. **Undo/redo, selection, copy/cut and find/replace are absent in the custom editor.**
   Typing, paste, caret movement, Home/End, Backspace/Delete, Enter and Tab work; there is no
   selection model, clipboard copy, multi-cursor, auto-indent or bracket matching.
3. **IME composition is not verified.** The custom editor consumes `Event::Text`/`Event::Paste`
   but does not implement the IME preedit path; no composition test was run.
4. **Two fixed tabs.** No `+`, close or reorder; `Workbench` exposes no add/close command to the
   window. The tab strip shows `*` for a dirty buffer but the OS caption is static.
5. **No menu bar or keyboard shortcuts.** Run, Cancel (while running) and precision are toolbar
   controls only; Ctrl+Enter/Ctrl+N/Ctrl+S/Esc from the old shell are not bound in the new
   window.
6. **No persistent engine per document.** `ExecutionHost` constructs one engine per run (the
   tested core's design). Variables, functions and engine settings from one run are not
   visible to a later run in the same tab; only the last published snapshot remains in the
   panes. Consequently the precision control takes effect at the start of the next run rather
   than on a long-lived engine. `--gui-selftest` proves that path reaches the real engine.
7. **Cancel is cooperative at top-level statement boundaries.** A single long-running statement
   cannot be interrupted; the 50-statement cancel check and the window observation cancel
   between statements.
8. **Workspace is read-only.** It shows snapshot variables (typed display) and user functions
   with a name filter; there is no clear-variables command, value editor, function-body viewer
   or type inspector. Clearing variables exists only in the old shell.
9. **Logs are capped at 500 lines** with no clear button, filter, timestamps or clickable
   diagnostic-to-editor jump.
10. **Plot has no export or history.** Zoom/pan/Fit and the inline SVG parse error are wired;
    saving the plot, opening its path, a plot list and per-run cleanup are absent. Plot files
    accumulate under the probed directory as `run-N/plot.svg`.
11. **No settings persistence in the workbench.** Window geometry, pane fractions and precision
    reset on launch; the old `settings.rs` is not wired into the new window.
12. **No visual/accessibility gate automation.** A4 observes liveness and the title only. The
    UIA tree, caret label, plot `Image` node and screenshots above were captured manually; no
    automated focus traversal, keyboard-only journey or pixel regression test was added.
13. **`--gui-selftest` is headless.** It drives the same `Workbench` methods as the window but
    does not paint frames, exercise resvg rasterisation or publish AccessKit; the window-level
    plot and Cancel checks above are manual UIA observations.
14. **External crates now ship on Windows.** `eframe`/`egui` 0.37 and `resvg` 0.50 are
    resolved from the local registry cache and pinned in `rust/Cargo.lock` (451 packages).
    `rust/README.md` still claims the workspace builds with no external crates; updating that
    document is out of this round's scope.
15. **The Win32 shell is still compiled into the binary.** It is reachable only through
    `--legacy-win32`, but its modules and tests remain until parity allows deletion.

---

## 7. Artifacts

Code:

* `rust/lovelace-desktop/` — the shipped workbench (Cargo.toml, `src/main.rs`,
  `src/workbench.rs`, `src/gui_selftest.rs`, `src/ui/**`, engine/execution wiring).
* `rust/Cargo.lock` — generated dependency lock for the Windows target.

Report:

* `docs/goal-desktop-v4/round-5/cutover.md` — this file.

Observed evidence files (generated under `rust/target-desktop/`, not part of the source tree):
`final-a1.txt`, `final-a2.txt`, `final-a3.txt`, `final-a4.txt`,
`final-selftest-default-temp.txt`, `final-window-run.txt`, `final-window-cancel.txt`,
`ui-tree-after-run.txt`, `window-after-run-a4.png`, `window-running-cancel.png`,
`window-after-cancel.png`.
