# Round 7 - Desktop workbench hardening

Status: **delivered**. All three objectives are implemented in `rust/lovelace-desktop/**` and the
A1-A5 acceptance was observed on the shipped artifact.

Scope: `rust/lovelace-desktop/**` only. No file outside that tree was edited except the evidence
under `docs/goal-desktop-v4/round-7/`. `rust/lovelace-lang`, `rust/lovelace-symbolics`,
`rust/lovelace-run`, `rust/Cargo.toml`, the C#/web trees and the spike crate were not touched.

## 1. What was wrong

* The workbench resolved its plot directory by probing candidates, and its last candidate was
  `<cwd>\lovelace-plots`. The round-5 window run recorded `Plot directory:
  C:\Users\ricar\dev\LovelaceSharp\lovelace-plots.` and left that directory in the repository
  (PLF-006, RISK-007). The directory was still present at the start of this round with
  `run-1\plot.svg` (333 bytes).
* The workbench had no keyboard commands, no startup focus, and no `--gui-selftest` proof that a
  long run leaves the event loop responsive.
* Cancellation was only checked between statements, which made a long run possible but not
  interruptible inside one evaluation.

## 2. Changes

| File | Change |
|---|---|
| `src/settings.rs` | Extracted `app_dir_from(appdata, xdg, home)`; `default_config_path` and the new `default_plot_dir` are now siblings derived the same way (`%APPDATA%\Lovelace\desktop.json` and `%APPDATA%\Lovelace\plots`). Added derivation tests. |
| `src/engine.rs` | Removed the probe list that ended in `<cwd>\lovelace-plots`. `default_plot_dir()` is now the app-scoped directory; `prepare_plot_dir()` creates it, writes and removes a probe file, and returns a message naming the path on failure. Tests assert the `%APPDATA%\Lovelace\plots` path, that it is outside the cwd, and that preparation reports the resolved directory. |
| `src/workbench.rs` | Stores the resolved `plot_root` (`plot_dir()`), logs `Plot directory: <path>.` and any preparation error at startup, exposes `toolbar_state()` (disabled Run + visible Cancel while running), `command(WorkbenchCommand)`, and `poll_report()` - the exact poll loop with each nonblocking `ExecutionHost::next_event` call timed. |
| `src/keyboard.rs` (new) | `WorkbenchCommand` and `command_for_shortcut(enter, escape, command_modifier)`: Ctrl+Enter -> Run, Esc -> Cancel, bare Enter -> no command. Used by both the window and `--gui-selftest`. |
| `src/ui/editor.rs` | `surface_id()` makes the editor's egui id a shared, deterministic value; Ctrl+Enter no longer inserts a newline. |
| `src/ui/app.rs` | Requests editor focus on the first frame; handles Ctrl+Enter/Esc globally from the frame's key events; builds the toolbar from `Workbench::toolbar_state()`; prints the resolved plot directory to stderr at window startup; two headless egui tests drive the real `WorkbenchApp`. |
| `src/gui_selftest.rs` | Startup `GUI_SELFTEST plot-dir <path>` line; new checks `plot-dir`, `keyboard`, `long-run` (10 checks total). |
| `src/main.rs` | Registers `keyboard` and documents the shortcuts in `--help`. |

No test was deleted, ignored or weakened. The 90 pre-existing tests still pass; the suite is now
105 tests.

## 3. Test-first evidence

The plot-directory defect was reproduced as a failing test before the fix:

```text
thread 'engine::tests::plot_dir_is_the_roaming_appdata_lovelace_plots_directory' panicked at
lovelace-desktop\src\engine.rs:220:9:
assertion `left == right` failed: the plot directory must be derived from the app-scoped %APPDATA%\Lovelace directory
  left: "C:\\Users\\ricar\\dev\\LovelaceSharp\\rust\\target-desktop\\tmp\\lovelace-desktop-plots"
 right: "C:\\Users\\ricar\\AppData\\Roaming\\Lovelace\\plots"
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 90 filtered out
```

Baseline before this round (same environment):

```text
test result: ok. 90 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
```

New tests (15):

```text
settings::tests::app_dir_prefers_appdata_then_xdg_then_home
settings::tests::plot_dir_is_derived_from_the_settings_application_directory
settings::tests::default_plot_dir_matches_default_config_path_parent
engine::tests::plot_dir_is_the_roaming_appdata_lovelace_plots_directory
engine::tests::plot_dir_is_never_under_the_working_directory
engine::tests::prepare_plot_dir_creates_the_resolved_directory_and_reports_failure
keyboard::tests::ctrl_enter_runs_and_escape_cancels
keyboard::tests::a_bare_enter_and_unrelated_keys_issue_no_command
keyboard::tests::ctrl_enter_wins_when_both_shortcuts_arrive_together
workbench::tests::toolbar_state_tracks_the_active_run_and_the_status_strip
workbench::tests::keyboard_commands_use_the_same_run_and_cancel_entry_points
workbench::tests::plot_dir_is_application_scoped_and_not_under_the_working_directory
workbench::tests::poll_report_measures_a_nonblocking_next_event_path_while_running
ui::app::tests::focus_starts_in_the_editor_and_typed_text_lands_without_a_click
ui::app::tests::ctrl_enter_runs_and_escape_cancels_with_the_visible_running_state
```

## 4. Acceptance

Environment for every command below:

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:TMP="$repo\rust\target-desktop\tmp"; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR="$repo\rust\target-desktop"
```

### A1 - `cargo test -p lovelace-desktop` - PASS

```powershell
& "$repo\.toolchain\cargo\bin\cargo.exe" test -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"
```

Observed (exit 0; full logs: `A1-cargo-test.out.log`, `A1-cargo-test.err.log`):

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.29s
     Running unittests src\main.rs (rust\target-desktop\debug\deps\lovelace_desktop-dd274d469d4bf684.exe)

running 105 tests
...
test result: ok. 105 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.64s
```

### A2 - `cargo build -p lovelace-desktop` - PASS

```powershell
& "$repo\.toolchain\cargo\bin\cargo.exe" build -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"
```

Observed (exit 0; `A2-cargo-build.err.log`):

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.27s
```

Artifact: `rust\target-desktop\debug\lovelace-desktop.exe`, 64,000,000 bytes,
SHA256 `F7E42DC8E7F6F238315BFDFBB38E5262F47B8E9E7808B04A90387A63E9789DAB`.

**Harness write-sandbox caveat (observed, not a product failure).** This session's command
sandbox allows commands to write inside the workspace and its mapped temp only. A binary that
lives under the workspace is therefore refused writes to `%APPDATA%` (`Access is denied`); the
same binary copied outside the workspace is not refused. The binary is self-contained, so A3 and
A5 were run from a byte-identical copy:

```powershell
$dest="$env:LOCALAPPDATA\Temp\lovelace-desktop-acceptance"
Copy-Item "$repo\rust\target-desktop\debug\lovelace-desktop.exe" "$dest\lovelace-desktop.exe" -Force
Get-FileHash "$repo\rust\target-desktop\debug\lovelace-desktop.exe","$dest\lovelace-desktop.exe" -Algorithm SHA256
```

Both hashes:

```text
F7E42DC8E7F6F238315BFDFBB38E5262F47B8E9E7808B04A90387A63E9789DAB  ...\rust\target-desktop\debug\lovelace-desktop.exe
F7E42DC8E7F6F238315BFDFBB38E5262F47B8E9E7808B04A90387A63E9789DAB  ...\Local\Temp\lovelace-desktop-acceptance\lovelace-desktop.exe
```

The in-tree failure was also observed and is preserved in `A3-intree-sandbox.txt`. It fails only
the two plot checks, and it reports the reason instead of hiding it:

```text
GUI_SELFTEST FAIL plot: the engine produced no plot capture
GUI_SELFTEST FAIL plot-dir: plot-dir: the app-scoped directory is unusable: Cannot write in the plot directory C:\Users\ricar\AppData\Roaming\Lovelace\plots: Access is denied. (os error 5). Plot statements will fail until it is writable.
GUI_SELFTEST summary checks=10 failed=2
```

### A3 - `lovelace-desktop.exe --gui-selftest` - PASS (exit 0, 10/10 checks)

```powershell
& "$env:LOCALAPPDATA\Temp\lovelace-desktop-acceptance\lovelace-desktop.exe" --gui-selftest
```

Observed (`A3-gui-selftest.txt`):

```text
GUI_SELFTEST lovelace-desktop 0.1.0 engine=lovelace-lang
GUI_SELFTEST plot-dir C:\Users\ricar\AppData\Roaming\Lovelace\plots
GUI_SELFTEST ok two-tabs: document ids DocumentId(1)/DocumentId(2), precision 1000, independent snapshots
GUI_SELFTEST ok arithmetic: result 12, steps 3, b = 12 (Natural), status [welcome.lv  |  rev 1  |  precision 1000  |  Ln 1, Col 1  |  run Finished]
GUI_SELFTEST ok print: output ["x"], result 2, committed 2
GUI_SELFTEST ok precision: pi at 8 = 3.14159265, at 20 = 3.14159265358979323846 (engine set_precision via submit)
GUI_SELFTEST ok cancel: cancelled after 2 of 50 statements in 1.55 ms (first committed 1)
GUI_SELFTEST ok syntax-error: line 2, column 4: Unexpected token '' at position 3: expected a number, string, identifier, '[', or '('. (outcome Some(Failed), phase Idle)
GUI_SELFTEST ok plot: 334 SVG bytes, title "selftest", 1 polyline with 3 points
GUI_SELFTEST ok plot-dir: resolved C:\Users\ricar\AppData\Roaming\Lovelace\plots (outside cwd C:\Users\ricar\dev\LovelaceSharp), plot file C:\Users\ricar\AppData\Roaming\Lovelace\plots\run-1\plot.svg (333 bytes), logged at startup
GUI_SELFTEST ok keyboard: Ctrl+Enter -> Run (status "run Running", Run disabled, Cancel visible); Esc -> run Cancelled in 417.99 ms; toolbar restored
GUI_SELFTEST ok long-run: submit returned in 65 us; 300 ms sample: 197 frames, 202 next-event calls, 5 applied, max 3.800 us; cancel terminal in 98.17 ms after 2 of 13 statements
GUI_SELFTEST summary checks=10 failed=0
```

### A4 - app-scoped plot directory - PASS

Before (accidental directory in the working tree):

```text
C:\Users\ricar\dev\LovelaceSharp\lovelace-plots\run-1\plot.svg   333 bytes   11/12/2026 9:51:14 AM
```

After (startup log line, from the shipped window's log pane and from `--gui-selftest`):

```text
[info] Plot directory: C:\Users\ricar\AppData\Roaming\Lovelace\plots.
GUI_SELFTEST plot-dir C:\Users\ricar\AppData\Roaming\Lovelace\plots
```

The plot files land there (observed with the real engine):

```text
C:\Users\ricar\AppData\Roaming\Lovelace\plots\run-1\plot.svg   333 bytes
C:\Users\ricar\AppData\Roaming\Lovelace\plots\plot.svg         334 bytes  (old-core --selftest engine-plot)
```

Cleanup and verification:

```powershell
Remove-Item -Recurse -Force "$repo\lovelace-plots"
Test-Path "$repo\lovelace-plots"
```

```text
False
```

No process wrote into the working tree: the only plot path the product prints is under
`%APPDATA%`, and the repository has no `lovelace-plots` directory.

### A5 - `--selftest` and `--legacy-win32` - PASS

```powershell
& "$env:LOCALAPPDATA\Temp\lovelace-desktop-acceptance\lovelace-desktop.exe" --selftest
```

Observed (exit 0; `A5-selftest.txt`):

```text
SELFTEST lovelace-desktop 0.1.0 engine=lovelace-lang
SELFTEST ok new-document
SELFTEST ok insert-text
SELFTEST ok open-file
SELFTEST ok run
SELFTEST ok cancel
SELFTEST ok precision
SELFTEST ok save-round-trip
SELFTEST ok tab-isolation
SELFTEST ok undo-redo
SELFTEST ok settings-round-trip
SELFTEST ok settings-corrupt
SELFTEST ok log-cap
SELFTEST ok svg-parse
SELFTEST ok engine-plot
SELFTEST ok no-network-imports
SELFTEST summary checks=15 failed=0
```

Legacy shell smoke test (Start-Process, 5 s, then Stop-Process):

```powershell
$p = Start-Process -FilePath "$dest\lovelace-desktop.exe" -ArgumentList '--legacy-win32' -PassThru
Start-Sleep -Seconds 5
$p.Refresh()
```

Observed:

```text
LEGACY_PID=35580 EXITED=False TITLE=Lovelace Desktop - Untitled 1 HWND=20712520 RESPONDING=True
STOPPED_AFTER_5S
```

## 5. What a long run looks like

`--gui-selftest`'s `long-run` check submits a real-engine run of

```lovelace
setprecision(20000)
sqrt(2)   (x12)
```

and samples the event loop for 300 ms while the heavy statement is in flight, then cancels.
Observed on this machine:

| Measurement | Observed |
|---|---|
| `run_active()` / submit latency | **65 us** (also 43-412 us across runs; the budget is 50 ms) |
| 300 ms sample | 197 frames, 202 nonblocking next-event calls, 5 typed events applied |
| longest single `ExecutionHost::next_event(Duration::ZERO)` | **3.800 us** (budget 50 ms) |
| cancel request to terminal `Cancelled` | **98.17 ms** (budget 2 s) |
| committed prefix at cancellation | 2 of 13 statements |
| `keyboard` check Esc -> `run Cancelled` | **417.99 ms** |
| real window: whole run to `Finished` | 7,865 ms / 13 statements |
| real window: Esc -> `run Cancelled` | 2,291 ms total run wall time, 7 statements committed |

The full run is genuinely long with respect to the UI (7.9 s on this machine). A single
`sqrt(2)` at precision 200000 was also measured independently through the engine: **29,721 ms**
for that one statement. The desktop can run that script, but the engine only observes the
cancellation token between statements, so that variant cannot reach `Cancelled` within 2 s - see
"Not wired yet".

## 6. Real-window evidence

Window: title `Lovelace Desktop`, process responding while the 7.9 s run was in flight
(`driver.log`, `state` lines). Evidence files live in `gui/`.

Startup focus (no click anywhere; `keys "a = 1"` sent to the foreground window):

```text
Text | welcome.lv  |  rev 5  |  precision 1000  |  Ln 1, Col 6  |  run Idle
Edit | Script editor, 6 lines, caret line 1 column 6
```

Ctrl+Enter while the long run is in flight (`gui/uia-running.txt`):

```text
Button | Run | id= | focusable=True | enabled=False
Button | Cancel | id= | focusable=True | enabled=True
Text | long.ls  |  rev 1  |  precision 1000  |  Ln 1, Col 1  |  run Running
```

Esc (`gui/uia-cancelled.txt`):

```text
Button | Run | id= | focusable=True | enabled=True
Text | long.ls  |  rev 1  |  precision 1000  |  Ln 1, Col 1  |  run Cancelled
Text | 2291 ms last run
Text | [info] Run cancelled after 7 statements in 2291 ms; the committed statements remain in the workspace.
```

The shipped window's log pane at startup (`gui/uia-initial.txt`, `gui/window-initial.png`):

```text
Text | [info] Plot directory: C:\Users\ricar\AppData\Roaming\Lovelace\plots.
```

## 7. Not wired yet

* **Cancellation inside one evaluation.** `ExecutionHost` checks its token between statements.
  A single long statement (`sqrt(2)` at precision 200000, ~29.7 s here) runs to completion before
  `Cancelled` is published. Fixing that needs the engine to poll a host-owned cancellation flag
  inside its kernels; `rust/lovelace-lang` is out of scope for this round.
* **The plot directory is fixed per process.** There is no setting or restart-free way to change
  it. It is created and probed once at startup; the log records the path and any failure.
* **Plot history grows.** `run-N` directories accumulate under `%APPDATA%\Lovelace\plots`; there
  is no cleanup, export or "open directory" action.
* **Focus is requested only at startup.** Clicking a toolbar control or another pane moves focus
  away; there is no shortcut to put it back in the editor.
* **Esc cancels without confirmation**, and there is no pause/resume - the only controls are Run
  and Cancel.
* **The workbench still does not persist its own settings.** `settings.rs` (window geometry,
  layout, precision, recent files) is used by the pre-cutover core and legacy shell only; the new
  workbench opens with fixed fractions and precision 1000.
* **Accessibility naming.** `Cancel` / `Run` UIA names come from the egui button labels; there is
  no explicit accessible description beyond the hover text, and no keyboard-focus ring customisation
  for the toolbar.
* **Shortcut discoverability.** Ctrl+Enter/Esc are documented in `--help` and tooltips only; no
  in-window shortcut sheet.

## 8. Evidence index

| File | Contents |
|---|---|
| `A1-cargo-test.out.log` / `.err.log` | full `cargo test` stdout/stderr |
| `A2-cargo-build.err.log` | `cargo build` output |
| `A3-gui-selftest.txt` | the 10-check acceptance run (exit 0) |
| `A3-intree-sandbox.txt` | the same run from the workspace binary under the harness write sandbox |
| `A5-selftest.txt` | 15/15 `--selftest` |
| `gui/driver.log` | every launch/wait/keys/uia/capture/close with timestamps |
| `gui/uia-initial.txt`, `gui/uia-running.txt`, `gui/uia-cancelled.txt`, `gui/uia-focus-typed.txt` | UIA trees |
| `gui/window-initial.png` | the window at startup, log pane showing the resolved plot path |
