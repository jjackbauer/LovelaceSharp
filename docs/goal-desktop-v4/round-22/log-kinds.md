# LOG-002: every log line has a kind, and every kind has a visible marker

Requirement: *the logs pane must distinguish, at a glance, echo, stdout, result, diagnostic,
timing and progress lines, each with a readable marker/text. Today the log is mostly uniform
`[info]`/`[error]` text with the result line bare.*

This round replaces the workbench's severity-only `LogLevel` with a closed [`LogKind`] carried by
every line, renders a distinct marker and colour per kind in the product window, routes the
per-statement `StepStarted` events the workbench already received into `progress` lines, and
captures the live window showing `stdout`, `result`, `diagnostic` and `timing` markers together.

## Kind → marker → colour table

| Kind | Visible marker | Colour in the window | Produced by |
| --- | --- | --- | --- |
| `echo` | `[echo]` | lavender `#c8b6ff` | a quick-eval input echoed before evaluation. **Unwired**: the product window has no quick-eval input (see below). |
| `stdout` | `[stdout]` | primary text `#d8dee9` | one `Output` event per `print(...)` line from the running document |
| `result` | `[result]` | accent teal `#4ec9b0` | the `Finished` snapshot's result (an expression statement's value) |
| `diagnostic` | `[diagnostic]` | error red `#ff7b72` | a `Failed` event's structured diagnostic; also hard application errors (file open, worker spawn, plot directory) |
| `timing` | `[timing]` | warning gold `#d7ba7d` | the terminal run lines that carry elapsed milliseconds: `Run finished … in N ms`, `Run failed … in N ms`, `Run cancelled … in N ms` |
| `progress` | `[progress]` | progress blue `#7fb0d8` | the existing per-statement `StepStarted` event: `statement i/n: <statement label>` |
| `info` | `[info]` | dim grey-blue `#8b98a9` | readiness, plot-directory, open, queue, start, cancellation request, stale-result and other advisory lines |

The marker is part of the model, not the paint code: `LogLine::render()` returns
`"<marker> <text>"`, the window paints exactly that string, and the unit tests pin it. The log
pane header's hover and the empty-pane state also show the legend
`echo: [echo] · stdout: [stdout] · …` so the vocabulary is discoverable before the first run.

There is no separate warning/error severity in the workbench log any more: an old warning is
`info`, an old error is `diagnostic`. The legacy Win32 shell (`--legacy-win32`) still uses its
own old `LogLevel` vocabulary; it is not the product window and was left untouched.

## Code changes

- **`rust/lovelace-desktop/src/log_kind.rs`** (new)
  - `LogKind` with exactly seven variants and `LogKind::ALL`.
  - `label()` (bare kind name), `marker()` (`[echo]`, `[stdout]`, …), `LogLine::new`, and
    `LogLine::render()`.
  - Three model tests (below) including a known seven-kind sequence.
- **`rust/lovelace-desktop/src/workbench.rs`**
  - `LogLine` now carries `kind: LogKind` (no `level`, no optional kind).
  - `Workbench::log` takes a required `LogKind`; every call site was classified explicitly.
  - `StepStarted` is no longer ignored: it appends
    `[progress] statement i/n: <label>` using the existing event's index and label and the total
    received from `RunStarted`.
  - `Finished` appends `[result] = …` then `[timing] Run finished: … in … ms.`
  - `Failed` appends `[diagnostic] Diagnostic: …` then `[timing] Run failed after … in … ms; …`.
  - `Cancelled` appends `[timing] Run cancelled after … in … ms; …`.
  - `Output` appends `[stdout]` with the engine's bytes unchanged.
  - Hard errors (plot directory, file open, worker spawn) are `diagnostic`; warnings are `info`.
  - `StepCommitted`/`SubProgress` still update committed/total but are not logged separately:
    both describe the same statement `StepStarted` already announced, so logging them would
    duplicate every progress row. Progress is not invented; it is routed from the existing event.
- **`rust/lovelace-desktop/src/ui/app.rs`**
  - Logs pane maps every `LogKind` to its colour and paints `entry.render()`.
  - Added `LOG_ECHO` and `LOG_PROGRESS` colour constants and the header-hover/empty-state legend.
- **`rust/lovelace-desktop/src/gui_selftest.rs`**
  - The print check now looks for `LogKind::Stdout` (same line and text as before).
- **`rust/lovelace-desktop/src/main.rs`**
  - Registers the new `log_kind` module.

No tests were weakened: every existing log assertion still checks the same text/order/count, and
only the field it filters on moved from `level` to `kind` (`Stdout`/`Diagnostic`).

## Reachability in a single crafted run

The capture uses two runs on the same document because a run that fails cannot also publish the
successful `Finished` result line (the failure ends the run before a result snapshot):

1. Run 1 source `6 * 7` → successful `Finished`:
   `[progress] statement 1/1: 6 * 7`, `[result] = 42`,
   `[timing] Run finished: 1 statements committed in 7 ms.`
2. Focus is restored to the editor through UI Automation, and the failing script is appended to
   the same buffer:
   ```
   6 * 7
   print("hi")
   1 / 0
   ```
   Run 2 → `[progress] statement 1/3: 6 * 7`,
   `[progress] statement 2/3: print("hi")`, `[stdout] hi`,
   `[progress] statement 3/3: 1 / 0`,
   `[diagnostic] Diagnostic: line 3, column 1: Cannot divide by zero.`,
   `[timing] Run failed after 2 committed statements in 5 ms; the committed prefix remains in
   the workspace.`

The append after a finished run also produces the expected
`[info] The displayed result is stale: the source changed after it finished.` line. All 17 log
lines in the capture were read back from the live accessibility tree; the required four markers
appear together in the one image:

```
[result] = 42                                   <- run 1
[timing] Run finished: 1 statements committed in 7 ms.
…
[stdout] hi                                     <- run 2
…
[diagnostic] Diagnostic: line 3, column 1: Cannot divide by zero.
[timing] Run failed after 2 committed statements in 5 ms; …
```

Kinds reachable from real events: `stdout`, `result`, `diagnostic`, `timing`, `progress`, and
`info`. Every one of the six appears in the captured UIA log; `echo` does not and is not faked.

### Unwired: `echo`

The product window (egui workbench) has no quick-eval input. The legacy application core in
`app.rs` still defines `Command::QuickEval` for `--selftest` / `--legacy-win32`, but the shipped
window does not bind it to any control, so no code path can produce an echo line today.
`LogKind::Echo` exists, has a marker and colour, and the model tests render it; the capture script
asserts that no `[echo]` line is present. Wiring it later needs one quick-eval input that logs
`LogKind::Echo` before evaluating; no such fake line was inserted for this round.

The workbench *did* already receive per-statement step events (`StepStarted`, `StepCommitted`,
`SubProgress`); the gap was only that the log dropped them. `StepStarted` is now routed to
`[progress]`, so progress is not invented and does not need a new event.

## Unit tests and observed output

Run in the workspace PowerShell 5.1 environment (as prescribed):

```
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:TMP="$repo\rust\target-desktop\tmp"; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR="$repo\rust\target-desktop"
& "$repo\.toolchain\cargo\bin\cargo.exe" test -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"
```

Observed (full log in `cargo-test.log`):

```
test result: ok. 197 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
```

New tests:

| Test | What it pins |
| --- | --- |
| `log_kind::tests::every_kind_has_a_distinct_readable_marker` | all seven kinds, one unique `[label]` marker each |
| `log_kind::tests::a_log_line_renders_its_kind_marker_before_the_text` | `render()` prefixes the marker for every kind, including `echo` |
| `log_kind::tests::a_known_sequence_keeps_every_kind_once_in_order` | a nine-entry known sequence keeps all seven kinds and the exact order |
| `workbench::tests::a_successful_run_logs_the_expected_kind_sequence_in_order` | real `ExecutionHost` events for `print("hi")` + `6 * 7` yield exactly `Info, Info, Progress, Stdout, Progress, Result, Timing`, with `statement 1/2…`, `hi`, `statement 2/2…`, `= 42`, and a `Run finished … ms.` line |
| `workbench::tests::a_failing_run_logs_diagnostic_and_timing_after_progress_and_stdout` | `print("hi")` + `1 / 0` yields exactly `Info, Info, Progress, Stdout, Progress, Diagnostic, Timing`, with the diagnostic text and the timed `Run failed` line |

Observed test lines from the suite log:

```
test log_kind::tests::a_known_sequence_keeps_every_kind_once_in_order ... ok
test log_kind::tests::a_log_line_renders_its_kind_marker_before_the_text ... ok
test log_kind::tests::every_kind_has_a_distinct_readable_marker ... ok
test workbench::tests::a_failing_run_logs_diagnostic_and_timing_after_progress_and_stdout ... ok
test workbench::tests::a_successful_run_logs_the_expected_kind_sequence_in_order ... ok
```

The run-slice tests deliberately start at the run's `Run queued` line, because a read-only
`APPDATA` legitimately adds a startup `[diagnostic]` line; the tests still pin the complete
run-event sequence, and the model test pins the full seven-kind taxonomy.

## Window capture

Artifacts in this directory:

| File | Size | Dimensions | What |
| --- | --- | --- | --- |
| `log-kinds.png` | 1,180,230 bytes | 3888x4500 | full product window, `PrintWindow(PW_RENDERFULLCONTENT)` |
| `log-kinds-logs.png` | 196,448 bytes | 2280x794 | native crop of the logs pane from the same capture |
| `log-kinds-uia.txt` | 4,777 bytes | — | live UIA tree with all 17 log lines in order |
| `work/capture-log.txt` | 4,093 bytes | — | driver log: launch, typing, runs, capture, palette counts |

Method (locked session; `GetForegroundWindow` is the lock screen, so no foreground is taken):

1. Launched `lovelace-desktop.exe <empty .lv>` with `__COMPAT_LAYER=DPIUNAWARE`, `APPDATA` under
   this round's directory and `TMP` under `rust/target-desktop/tmp`; no plot statement is run.
2. Posted `WM_SETFOCUS`, then per character `WM_KEYDOWN` with `VK_PROCESSKEY`, `WM_CHAR`,
   `WM_KEYUP`; newlines with `VK_RETURN`. The editor really typed `6 * 7`.
3. Invoked the `Run` button through UI Automation's `InvokePattern` (no foreground needed).
   After the first run, UI Automation `SetFocus` on the editor's accesskit node restored egui
   focus (the Run invoke had moved it to the button), and the failing script was appended.
4. Invoked `Run` again; the status strip reached `run Failed`.
5. Set the window geometry and forced a repaint (`SetWindowPos`, `RedrawWindow`,
   `WM_SETFOCUS`), then captured with `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT)`.
6. Dumped the live accessibility tree with the unmodified `tools/gui-harness.ps1 -Action uia`
   (a read-only action; `tools/` was not modified). The tree's log nodes prove the exact text and
   order; the script fails if `[echo]` appears or if the four required markers are missing.
7. Located the logs pane by its `#222830` header band and the status strip by its `#1b1b1b` band,
   cropped the detail image, and probed the crop for the exact painted marker colours.

The locked session's compositor reported a 300% scale to the DPI-unaware product during this
capture (the same 96↔288 dpi switching round 20 documented). The capture is therefore the full
3888x4500 physical surface of the 1296x1500 logical window; the driver re-applies the geometry,
forces a repaint and retries until the full-width log header band is present, so a stale 432x500
surface can never be delivered. The detail crop is native pixels from that capture and is the
easy-to-read evidence; the full PNG is the unmodified window capture.

Palette probe over the log crop (exact RGB pixels, from `work/capture-log.txt`):

```
palette info        #8B98A9 pixels=1541
palette stdout      #D8DEE9 pixels=17
palette result      #4EC9B0 pixels=29
palette diagnostic  #FF7B72 pixels=102
palette timing      #D7BA7D pixels=317
palette progress    #7FB0D8 pixels=274
```

All six wired kinds are painted with their documented colours. `#C8B6FF` (echo) is absent, as
expected for the unwired kind — the script also asserts the UIA text contains no `[echo]`.

## Suite summary

`cargo test -p lovelace-desktop`: **197 passed, 0 failed, 0 ignored** in 0.61 s (exit 0), including
the five new LOG-002 tests. `--gui-selftest` was not used as evidence for this round: it exercises
the plot path, which is out of scope here and whose checks are unrelated to LOG-002.
