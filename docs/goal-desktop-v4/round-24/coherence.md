# RUN-003 — one coherent snapshot per committed statement

Requirement: *After each committed statement, variables, functions, log output, plots, revision
and diagnostics reflect the same coherent engine snapshot. No pane may show a value from a
different statement than the log's last committed line.*

## 1. The update path before this round

The panes were updated from **separate events**, and the committed snapshot was only assembled at
the end of a run:

| Pane data | Event that moved it (before) | Workbench code |
| --- | --- | --- |
| `committed` counter | `StepCommitted` — only `committed = statement + 1` | `workbench.rs` old `apply_event` arm |
| `[progress]` log line | `StepStarted`, appended **immediately** | old `StepStarted` arm |
| `[stdout]` log line | `Output`, appended **immediately** | old `Output` arm |
| variables / functions / state / plot / result / revision | `WorkspaceSnapshot` published only by `Finished`, `Cancelled`, `Failed` | old terminal arms |
| cancelled/failed snapshot | a second lookup, `ExecutionHost::last_snapshot(id)`, not carried by the event | old `Cancelled`/`Failed` arms |
| workspace pane | `active_snapshot()` | `ui/app.rs::workspace_pane` |
| logs pane | `active_logs()` | `ui/app.rs::logs_pane` |
| plot pane | `document.last_plot` | `ui/app.rs::sync` |
| editor diagnostic | `active_diagnostic()` | `ui/app.rs::editor_pane` |
| revision | `document.revision` | `ui/app.rs` status strip / tab strip |

Consequences that violate RUN-003:

1. `StepCommitted` moved the committed counter while the variables pane still showed the previous
   run's snapshot (or the empty state), and `StepStarted`/`Output` already put a line for
   statement *n+1* into the log. A frame in that window showed a log line from the next statement
   beside variables from the previous one.
2. `Cancelled`/`Failed` published the snapshot through a map lookup separate from the event that
   ended the run; the event and the publish were not the same object.
3. The window read five different workbench accessors at five points in the frame; nothing forced
   them to describe the same commit.

## 2. What changed: one snapshot event, one application branch, one assembled view

### 2.1 The event carries the whole snapshot

`rust/lovelace-desktop/src/execution.rs`:

- `WorkspaceSnapshot` is now documented as the atomic unit of pane state and carries a new
  `total` field (statements the run visited) beside `committed_steps`, so the view can paint
  `committed K/T` from the snapshot itself.
- `DesktopEvent::StepCommitted` gained `snapshot: WorkspaceSnapshot` (`execution.rs:112`,
  emitted at `execution.rs:736`). The worker builds it with the same `snapshot(progress, None)`
  call that will produce a terminal snapshot, after the variable/function/state/plot captures of
  that commit.
- `DesktopEvent::Cancelled` and `DesktopEvent::Failed` also carry their `snapshot`. The workbench
  no longer calls `last_snapshot()` for pane data; the terminal object in the event is the object
  the panes adopt (the host still publishes it, and the execution tests now assert the two are
  identical).

### 2.2 The workbench applies commit + log + counter in one branch

`rust/lovelace-desktop/src/workbench.rs`:

- New per-document `pending_logs` buffer. `StepStarted` and `Output` **buffer** their
  `[progress]`/`[stdout]` lines instead of appending them; `StepCommitted` flushes the buffer and
  publishes the event's snapshot in the same `apply_event` branch
  (`workbench.rs:980`). So a statement's log lines enter the log only when its effects enter the
  workspace snapshot.
- `publish_snapshot` (`workbench.rs:1232`) now adopts **exactly** the snapshot's plot. A new run
  whose committed prefix has no plot clears the previous run's picture instead of leaving it
  beside the new variables.
- A stale (edited-away) run never publishes a `StepCommitted` snapshot; its buffered lines are
  flushed with the existing `Run result discarded` notice at the terminal event, and no pane
  claims its effects.
- Ordering and text are unchanged (`[progress] statement n/N: label`, then the statement's
  `[stdout]`, then later `[result]`/`[timing]`); only the moment of append moved to the commit.
  The LOG-002 event-sequence tests still pass unchanged.

### 2.3 One assembled object for the frame

`workbench.rs:204` adds `DocumentView`, built by `Workbench::view` (`workbench.rs:1317`) from
**one** document state and **one** committed snapshot:

```
revision, has_snapshot, committed_steps, total, result,
variables, functions, state, plot, output,
logs, diagnostic (revision-gated), run_state, elapsed_ms
```

It exposes the coherence predicates the tests use:

- `committed_statement()` — 1-based committed step (the state revision);
- `last_log_statement()` — 1-based statement of the last attributed log line;
- `last_committed_log_statement()` — last attributed line **inside** `committed_steps`.

`log_kind.rs:101` adds `LogLine::statement: Option<usize>` so "which statement does this line
belong to" is model data, not string parsing; progress and stdout lines are tagged by the worker
event index.

The window now assembles `self.view` once per frame in `sync()` (`ui/app.rs:219`) immediately
after `poll()`, and every pane renders that same value (`ui/app.rs:536` clones it for the pane
closures; editor, plot, workspace and logs panes take `&DocumentView`). The Workspace pane paints
the committed state line `rev 1 · committed 4/4` from the same view as the variables.

## 3. Tests

All tests assert on the one assembled `DocumentView` returned by `Workbench::active_view()`; the
success test inspects it *between* individual host events, immediately after each
`StepCommitted`.

| Test | What it proves |
| --- | --- |
| `workbench::tests::every_committed_step_publishes_one_coherent_snapshot` | 4-statement script `a = 1`; `b = a + 1`; `print("p")`; `b`. Drains the real `ExecutionHost` one event at a time; after commit n asserts, on the same view: `committed_steps == n`, `committed_statement() == Some(n)`, source `revision` unchanged, exact `variables` (after 2: `a=1`, `b=2`), `state.variables` identical, functions count, output count, `last_committed_log_statement() == Some(n)`, `last_log_statement() == Some(n)`, and **no** log line from a later statement. After statement 2 the last progress line is exactly `statement 2/4: b = a + 1`. |
| `workbench::tests::a_failure_at_statement_three_keeps_the_committed_prefix_and_names_statement_three` | `a = 1`; `b = a + 1`; `c = 1 / 0`; `b`. The same view after `Failed`: `committed_steps == 2`, variables exactly `a=1,b=2` and **no `c`**, result `2`, source revision unchanged; `last_committed_log_statement() == Some(2)`; the diagnostic (installed in the same branch as the snapshot) has `statement_index == 2` (0-based ⇒ statement 3), line 3 column 1, `Cannot divide by zero.`; log carries the diagnostic and `Run failed after 2 committed statements`. |
| `workbench::tests::cancelling_after_statement_two_keeps_the_same_coherent_prefix` | `a = 1`; `b = a + 1`; `c = 3`; `b`, with a gated engine that holds statement 3 until the live cancel flag is set. Same rule: `run_state == Cancelled`, `committed_steps == 2`, `committed_statement() == Some(2)`, variables exactly `a=1,b=2`, no `c`, revision unchanged, `last_committed_log_statement() == Some(2)`, the log tail names the cancelled attempt at 3, timing says `Run cancelled after 2 statements`, no diagnostic. |
| `workbench::tests::a_new_run_without_a_plot_clears_the_previous_run_plot_in_the_view` | Plot clause: a plot run's view carries its SVG; a following plotless run's view has `plot == None` and the document's plot field agrees, so the picture cannot outlive its variables/revision. |
| `ui::app::tests::a_frame_renders_the_same_assembled_view_the_workbench_published` | Window clause: after a 4-statement run, `app.view` (the object the frame renders) equals `workbench.active_view()`, with `revision == 1`, `committed_steps == 4`, `a=1,b=2`, last committed log statement 4. |

Observed suite lines (full log in `cargo-test.log`):

```
test workbench::tests::a_failure_at_statement_three_keeps_the_committed_prefix_and_names_statement_three ... ok
test workbench::tests::a_new_run_without_a_plot_clears_the_previous_run_plot_in_the_view ... ok
test workbench::tests::cancelling_after_statement_two_keeps_the_same_coherent_prefix ... ok
test workbench::tests::every_committed_step_publishes_one_coherent_snapshot ... ok
test ui::app::tests::a_frame_renders_the_same_assembled_view_the_workbench_published ... ok
```

### Revision wording

`WorkspaceSnapshot.revision` is the **source** revision the run was submitted for (it stays `1`
for all four statements; an edit would advance it and gate the run stale). The per-commit state
revision is `committed_steps` / `committed_statement()`. Both live in the same `DocumentView`, and
the window paints the source revision and the committed prefix from that view
(`rev 1 · committed 4/4`). For the failure at statement 3, the pane state is "revision 1,
committed through statement 2", which is the requested "revision at 2" in state terms; no field
is read from a different event.

## 4. Window proof

The session was **unlocked** for this round (foreground was an ordinary desktop window, not the
lock screen), so this used the normal harness `tools/gui-harness.ps1` for launch, UIA invocation,
UIA dump and capture (read-only; `tools/` was not modified).

| File | Size (bytes) | Dimensions | What |
| --- | --- | --- | --- |
| `coherence-printwindow.png` | **212,890** | 1500x1320 | **The RUN-003 proof**: full product window, `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT)`, after a 4-statement run. Opening it shows the Workspace pane `rev 1 · committed 4/4` with `a = 1 (Natural)` and `b = 2 (Natural)`, and the log tail `[progress] statement 4/4: b`, `[result] = 2`, `[timing] Run finished: 4 statements committed in 8 ms.` Painters are visible in the status strip too: `rev 1`, `run Finished`. |
| `coherence.png` | 271,011 | 1500x1320 | Supporting frame from the harness `-Action capture` (CopyFromScreen) of the same window and run. |
| `coherence-uia.txt` | 4,800 | — | Live UIA tree (66 nodes) with all 12 log lines in order and the workspace state line. |
| `work/driver.log` | 3,196 | — | launch / wait / click / status / UIA / capture / PrintWindow / close timings. |
| `work/pixel-probe.txt` | 331 | — | exact-colour counts in the captured PNG. |
| `work/coherence.lv` | 33 | — | the 4-statement script (`a = 1`, `b = a + 1`, `print("p")`, `b`). |
| `capture-coherence.ps1` | 9,744 | — | the driver, reproducible. |

### Method

1. Created `work/coherence.lv` with the four statements (no plot, though `APPDATA` was redirected
   to `round-24/appdata` as prescribed).
2. Launched `lovelace-desktop.exe work/coherence.lv` through the harness
   (`-Action launch`, inherited `APPDATA`/`TMP`).
3. Restored/sized the window to 1500x1320 with `SetWindowPos` (the one action the harness does not
   expose; the harness `scenario` action resizes internally for the same reason) so the logs pane
   shows the whole run.
4. Invoked the **Run** button through UI Automation `InvokePattern` (`-Action click -Name Run`).
   No typing, no SendKeys, no foreground keystrokes.
5. Polled the live status node until `run Finished`; the observed value was
   `coherence.lv  |  rev 1  |  precision 1000  |  Ln 1, Col 1  |  run Finished`.
6. Dumped the live accessibility tree (`-Action uia`) and copied it to `coherence-uia.txt`; the
   script fails unless the tree contains `[progress] statement 4/4: b`, `[result] = 2`,
   `[timing] Run finished: 4 statements committed`, `rev 1 · committed 4/4` and `run Finished`,
   in that log order, plus `a = 1 (Natural)` and `b = 2 (Natural)`.
7. Took the harness capture (`coherence.png`), then forced a repaint and took
   `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT)` into `coherence-printwindow.png`; both images
   were then opened and inspected. The PrintWindow call returned `ok=True` at 1500x1320.
8. Probe of the PrintWindow PNG (exact RGB counts, from `work/pixel-probe.txt`):
   `logs_progress_pixels=276` (`#7FB0D8`), `logs_timing_pixels=117` (`#D7BA7D`),
   `logs_result_accent_pixels=21` (`#4EC9B0`), `workspace_accent_pixels=21` (variable names and
   the `rev 1 · committed 4/4` line). The log header band was located at y=1065 and the status
   strip at y=1288, so the probe proves the log crop, not an empty region.
9. Closed the window through the harness (graceful).

The UIA log, in order (12 lines; startup lines included):

```
[info] Lovelace Desktop ready. Engine: lovelace-lang. Two documents open.
[info] Plot directory: C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop\tmp\lovelace-desktop\plots.
[info] Opened C:\Users\ricar\dev\LovelaceSharp\docs\goal-desktop-v4\round-24\work\coherence.lv.
[info] Run queued (revision 1, run 1).
[info] Run started: 4 statements, engine lovelace-lang, precision 1000.
[progress] statement 1/4: a = 1
[progress] statement 2/4: b = a + 1
[progress] statement 3/4: print("p")
[stdout] p
[progress] statement 4/4: b
[result] = 2
[timing] Run finished: 4 statements committed in 8 ms.
```

The Workspace pane source line in the same tree is `rev 1 · committed 4/4`; the captured image
shows it immediately above the three variables (`_`, `a`, `b`), so the log's last committed
statement (4), the committed count (4/4) and the variable values (`a = 1`, `b = 2`) are one
visible state.

## 5. Suite summary

```
test result: ok. 215 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.12s
```

Run with the prescribed PowerShell 5.1 environment:

```
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:TMP="$repo\rust\target-desktop\tmp"; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR="$repo\rust\target-desktop"
& "$repo\.toolchain\cargo\bin\cargo.exe" test -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"
```

Baseline was 210; this round adds 5 (four workbench RUN-003 tests and one window-view test) and
no existing test was weakened or removed. The full log is `cargo-test.log`.

## 6. Scope

Changed only `rust/lovelace-desktop/**` and `docs/goal-desktop-v4/round-24/**`:

- `rust/lovelace-desktop/src/execution.rs`
- `rust/lovelace-desktop/src/log_kind.rs`
- `rust/lovelace-desktop/src/workbench.rs`
- `rust/lovelace-desktop/src/ui/app.rs`
- `docs/goal-desktop-v4/round-24/**`

No engine crate, `tools/`, or gate file was modified. The harness was invoked read-only for
launch/UIA/capture; its existing worktree modification predates this round (file mtime
12:19 vs this round's 15:03).
