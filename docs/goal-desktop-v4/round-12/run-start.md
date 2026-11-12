# RUN-001 / PERF-004: Ctrl+Enter -> `run Running` (round 12)

Requirement (register): *"Ctrl/Cmd+Enter starts a background run and reports running state within
100 ms even when evaluation lasts 30 seconds"* (RUN-001), *"Run button/shortcut acknowledges
within 100 ms"* (PERF-004).

**Verdict: met, measured.** A real-window timestamp trace puts the product path at **8.5 ms from
the Ctrl+Enter frame to the first painted `run Running` frame** (5.3 ms to the end of the egui
frame that draws it). The three final window measurements are **57.2 / 57.5 / 68.4 ms**
(`-FastStatus`), of which 50.9 / 51.6 / 62.5 ms is the harness's own `SendKeys.SendWait` call
before sampling starts. The 105.6 / 111.9 / 151.7 / 120 ms previously recorded are dominated by
the driver, not the desktop: the coarse sampler walks the whole UIA tree after the status change
(43.4-73.3 ms per sample) and only starts sampling after `SendWait` returns (32-64 ms). No
product code on the run path was changed: there is no product component near the budget, and
optimising the 5-12 ms that exists would be the micro-optimisation the task forbids.

## Instrumentation

New module `rust/lovelace-desktop/src/run_trace.rs`. It is off by default (one relaxed atomic
load per mark) and is switched on by `LOVELACE_RUN_TRACE=<file>` (or `stderr`) or, in-memory, by
`--gui-selftest`. Each line is
`<mark> mono_us=<us since first mark> wall_ms=<UTC ms> thread=<name> <detail>`, so one line can be
correlated with the driver's `Get-Date` wall clock without a shared clock.

Marks on the run path (all added without changing behaviour):

| mark | site | thread |
| --- | --- | --- |
| `frame_begin` | `WorkbenchApp::ui_root` entry, while idle | UI |
| `key_event_handled` | Ctrl+Enter found in `ui_root`'s `input.events` | UI |
| `command_run_enter` | `Workbench::command(Run)` | UI |
| `submit_enter` / `submit_exit` | `ExecutionHost::submit` | UI |
| `run_phase_set` | `Workbench::run_active`, after `doc.phase = Running` | UI |
| `worker_start` | `RunWorker::run` | worker |
| `engine_build_enter` / `engine_constructed` | around the evaluator factory | worker |
| `run_started_emitted` | after `DesktopEvent::RunStarted` is sent | worker |
| `run_started_applied` | `Workbench::apply_event` consumes `RunStarted` | UI |
| `status_bar_drawn` | the status strip is built in the key frame | UI |
| `frame_end_after_key` | end of the key frame (running state drawn) | UI |
| `frame_after_paint_begin` | start of the next frame; the previous one was presented | UI |

In trace mode the key frame requests one immediate repaint at its end, so the next frame start is
an upper bound on "the running frame has been painted and presented". Trace mode also writes and
flushes 8-10 short lines to the trace file on the UI thread inside the measured frame (about
1 ms); the 5.3 ms frame figure is therefore an upper bound too. That extra frame and file I/O
exist only while tracing; without the environment variable the window behaves exactly as before.

`--gui-selftest` `keyboard` check now resets the in-memory trace, drives the same
`command_for_shortcut` / `Workbench::command` path and prints the queue-side breakdown, e.g.:

```
run-start trace [command_run_enter +0.000ms | submit_enter +0.006ms | submit_exit +0.049ms |
run_phase_set +0.052ms | worker_start +0.125ms | engine_build_enter +0.134ms |
engine_constructed +0.345ms | run_started_emitted +0.352ms | run_started_applied +1.597ms]
```

(`run_started_applied` is late only because the headless check polls every 1 ms; the visible
state does not depend on that event — `run_phase_set` happens 52 us after the command.)

Driver additions in `tools/gui-harness.ps1` (opt-in, existing behaviour unchanged):

- `-FastStatus`: cache the one status `Text` element once and poll `Current.Name` every 1 ms
  instead of re-walking the whole UIA tree and sleeping 150 ms. Per-sample cost is logged.
- `-TraceFile <path>`: set `LOVELACE_RUN_TRACE` for the launched product.
- every scenario logs the `SendKeys` call duration and every sample's own query duration.

## Breakdown, real window, before the sampler fix

`docs/goal-desktop-v4/round-12/trace-1/` — standard `-Action scenario` (123.9 ms reported),
`LOVELACE_RUN_TRACE` on. Times relative to the frame's `key_event_handled`:

| mark | cumulative |
| --- | --- |
| `key_event_handled` | 0.000 ms |
| `command_run_enter` | +0.020 ms |
| `submit_enter` | +0.055 ms |
| `submit_exit` (submit returns) | +0.158 ms |
| `run_phase_set` (phase is Running) | +0.184 ms |
| `worker_start` (other thread) | +0.366 ms |
| `engine_constructed` (other thread) | +0.788 ms |
| `run_started_emitted` (other thread) | +0.831 ms |
| `status_bar_drawn` (text is `run Running`) | +1.035 ms |
| `panes_drawn` (editor, plot, workspace, logs) | +4.920 ms |
| `frame_end_after_key` (running frame complete) | +4.949 ms |
| `frame_after_paint_begin` (paint + present done) | +11.819 ms |
| `run_started_applied` (next frame's poll) | +11.868 ms |

## Breakdown, real window, final build

`docs/goal-desktop-v4/round-12/after-trace-2/` — `-FastStatus -TraceFile`, 57.7 ms reported:

| mark | cumulative |
| --- | --- |
| `key_event_handled` | 0.000 ms |
| `command_run_enter` | +0.019 ms |
| `submit_enter` | +0.059 ms |
| `submit_exit` (submit returns) | +0.169 ms |
| `run_phase_set` (phase is Running) | +0.202 ms |
| `worker_start` (other thread) | +0.381 ms |
| `engine_constructed` (other thread) | +0.738 ms |
| `run_started_emitted` (other thread) | +0.769 ms |
| `status_bar_drawn` (text is `run Running`) | +1.200 ms |
| `panes_drawn` (editor, plot, workspace, logs) | +5.269 ms |
| `frame_end_after_key` (running frame complete) | +5.298 ms |
| `frame_after_paint_begin` (paint + present done) | +8.451 ms |
| `run_started_applied` (next frame's poll) | +8.485 ms |

Cross-check against the driver's wall clock: `enterWall` 11:32:24.790, `key_event_handled`
11:32:24.795, `frame_end_after_key` 11:32:24.800, `frame_after_paint_begin` 11:32:24.803 —
**13 ms from the start of the keystroke call to the painted running state**, while the driver's
first status sample did not start until 11:32:24.842 (`SendKeys` returned after 51.8 ms).

The frame is 5.3 ms; the dominant in-frame cost is the central panes (4.07 ms: editor, plot,
workspace and log text), then paint/tessellate/present (3.15 ms). Neither is near the 100 ms
budget; the submit itself is 0.11 ms and the engine is constructed on the worker thread, not on
the UI thread.

## Where the reported 105-152 ms went

Every standard-scenario run logs the sample that finally observed `run Running`; its own query
time is 43-73 ms because UIA re-reads the tree after the accessibility update:

| run | reported ms | observing sample's query | SendKeys call |
| --- | --- | --- | --- |
| `standard-1` | 134.3 | 73.3 ms | (not logged in that build; ~50-54 ms from trace) |
| `standard-2` | 105.0 | 43.4 ms | (same) |
| `standard-3` | 128.7 | 58.6 ms | (same) |
| previous rounds | 105.6 / 111.9 / 151.7 / 120 | — | — |

So the two dominant costs of the *measurement* are the blocking `SendKeys.SendWait`
(32-64 ms, which happens before the first sample) and the first post-update full-tree UIA read
(43-73 ms). The product is 8.5-12 ms across both trace runs. The fix is therefore in the sampler,
which the task explicitly allows ("if you need finer resolution, add a finer sampling mode to the
harness, but do not change the product to please the sampler"). `-FastStatus` removes the
full-tree read from the reported number: it polls a cached element every 1 ms (query 1.0-1.2 ms).
The remaining 50-63 ms is `SendKeys` itself; it is reported, not removed.

## Final three measurements

`tools/gui-harness.ps1 -Action scenario -FastStatus` on the final binary, script typed as
`matmul(eye(400), eye(400))` (one long kernel, so the state cannot be confused with a fast
finish), window 1296x839, fine sampler:

| run | Ctrl+Enter -> Running | SendKeys call | observing query | Esc -> Cancelled |
| --- | --- | --- | --- | --- |
| `after-fine-1` | **57.2 ms** | 50.9 ms | 1.2 ms | 47.6 ms |
| `after-fine-2` | **57.5 ms** | 51.6 ms | 1.0 ms | 62.6 ms |
| `after-fine-3` | **68.4 ms** | 62.5 ms | 1.0 ms | 35.6 ms |

All three are under 100 ms. Three standard-sampler runs of the same gesture (kept unchanged for
comparability) report 134.3 / 105.0 / 128.7 ms; the delta is the sampler, not the product.
Artifacts: `after-fine-*/scenario-summary.md`, `scenario-running.png`,
`scenario-running-uia.txt`, `driver.log`; trace: `after-trace-2/run-trace.log`.

## Tests

- `cargo test -p lovelace-desktop`: **108 passed, 0 failed, exit 0** (`cargo-test.log`).
- `lovelace-desktop.exe --gui-selftest`: **checks=10 failed=0, exit 0** (`gui-selftest.log`).
  The run used `APPDATA` under `rust/target-desktop/tmp/selftest/appdata` with the working
  directory outside it: this session denies child processes writes under the real `%APPDATA%`,
  so the two plot checks cannot pass with the default location. That is an environment limit
  already documented for this session, not a product regression; with a writable app-data
  directory the plot and plot-dir checks pass exactly as before.
- The instrumentation is additive: with `LOVELACE_RUN_TRACE` unset the only difference is one
  relaxed atomic load per mark site and no extra frames. `execution`'s timing test still shows
  `submit` under its 100 ms bound (52-63 us in these runs; the round-12 selftest prints 63 us).

## Reproduce

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:TMP="$repo\rust\target-desktop\tmp"; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR="$repo\rust\target-desktop"
& "$repo\.toolchain\cargo\bin\cargo.exe" build -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"

# fast, correct RUN-001 measurement + in-product trace
powershell -File tools/gui-harness.ps1 -Action scenario -FastStatus `
  -TraceFile docs\goal-desktop-v4\round-12\after-trace-2\run-trace.log `
  -Exe rust\target-desktop\debug\lovelace-desktop.exe `
  -OutDir docs\goal-desktop-v4\round-12\after-trace-2 `
  -Log docs\goal-desktop-v4\round-12\after-trace-2\driver.log
```

The trace file holds all marks (including `frame_begin` for idle frames and the headless
threads); filter on the labels in the table above.
