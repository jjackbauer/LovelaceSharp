# RUN-009: Queued, Running, Cancelling, Finished, Cancelled, Failed and Stale (round 13)

Requirement: the desktop must expose the seven run states with terminal events and distinct
status-strip labels, prove the full transition sequences for a fast success, a cancelled long run,
a failed run and an edit after a successful run, and report a real-window `run Cancelling`
observation or its measured duration.

**Verdict: met.** One explicit `RunState` machine now owns the shipped window's run state
(`rust/lovelace-desktop/src/run_state.rs`). Every accepted transition is appended to a
per-document event log; the status strip renders `run <label>` from that state. `--gui-selftest`
asserts the four required sequences and counts exactly one terminal run event in each. A
real-window scenario trace observes `run_state_cancelling` and measures it at **25.877 ms**
(run 1) / **21.627 ms** (run 2); the UIA screenshot sampler missed the label because its first
post-Esc sample was taken 34.0 / 34.4 ms after Esc, after the state had already become
`run Cancelled` (details below).

## State machine

States and status-strip labels:

| State | Status strip | Kind | Entered when | Leaves via |
| --- | --- | --- | --- | --- |
| `Idle` | `run Idle` | no run | document starts / file load | submit accepted -> `Queued` |
| `Queued` | `run Queued` | in flight | `ExecutionHost::submit` accepted | `RunStarted` -> `Running`; cancel accepted -> `Cancelling`; terminal event -> `Finished`/`Cancelled`/`Failed` |
| `Running` | `run Running` | in flight | worker delivers `RunStarted` | cancel accepted -> `Cancelling`; terminal event -> `Finished`/`Cancelled`/`Failed` |
| `Cancelling` | `run Cancelling` | in flight | cancel accepted while `Queued` or `Running` | terminal event -> `Cancelled` (or `Finished`/`Failed` if the worker won the race) |
| `Finished` | `run Finished` | terminal result | terminal success event | source edit -> `Stale`; submit -> `Queued` |
| `Cancelled` | `run Cancelled` | terminal result | terminal cancellation event, or an in-flight run whose terminal event was abandoned | submit -> `Queued` |
| `Failed` | `run Failed` | terminal result | terminal failure event | submit -> `Queued` |
| `Stale` | `run Stale` | out-of-date result | an edit advanced the source revision while `Finished` | submit -> `Queued`; further edits are a no-op |

Transition rules enforced by `RunStateMachine`:

| From | Trigger | To | Event-log reason |
| --- | --- | --- | --- |
| `Idle`, `Finished`, `Cancelled`, `Failed`, `Stale` | submit accepted | `Queued` | `submit accepted` |
| `Queued` | worker's `RunStarted` applied | `Running` | `worker reported RunStarted` |
| `Queued`, `Running` | cancellation accepted | `Cancelling` | `cancellation requested` |
| `Queued`, `Running`, `Cancelling` | non-stale `Finished` event | `Finished` | `Finished event applied` |
| `Queued`, `Running`, `Cancelling` | non-stale `Cancelled` event | `Cancelled` | `Cancelled event applied` |
| `Queued`, `Running`, `Cancelling` | non-stale `Failed` event | `Failed` | `Failed event applied` |
| `Finished` | source revision advances | `Stale` | `source revision advanced after a finished result` |
| `Stale` | source revision advances again | `Stale` (no transition) | none; `Finished` is not resurrected |
| `Queued`, `Running` | source revision advances (an edit) | `Cancelling` | `source changed while the run was in flight` |
| any in-flight state | terminal event whose revision is older than the buffer | `Cancelled` | `terminal event not published; the run was abandoned` |
| any non-`Idle` state | document loaded from disk | `Idle` | `document loaded from disk` |

`Finished`, `Cancelled` and `Failed` are terminal *run events*. `Stale` is a terminal *display*
state: it is reached only by an edit after `Finished` and is deliberately not counted as a second
terminal run event.

### Exactly one terminal event per run

`RunStateMachine::finish` refuses unless the state is still in flight and the run id matches the
tracked run. A duplicated `Finished`, a late `Cancelled` for the same run, or a terminal event for
an older run changes nothing and logs no transition. The workbench additionally closes the run
handle on the first accepted terminal event, so a replayed typed event is rejected before it can
reach the machine. Unit tests
`exactly_one_terminal_run_event_is_accepted_per_run`,
`a_duplicate_terminal_event_cannot_produce_a_second_terminal_transition` and
`a_finished_result_goes_stale_once_and_editing_again_cannot_resurrect_it` pin this invariant.

### Observability of transient states

`Queued` and `Cancelling` are set synchronously when `submit` / the cancellation request is
accepted, before the worker event can be applied. They are therefore visible to the next frame and
always present in the per-document transition log. If a frame is too fast to photograph, the log
still carries the transition; the selftest asserts the log, not a screenshot. The real window also
writes one `run_state_*` trace mark per transition when `LOVELACE_RUN_TRACE` is set.

## Observed label sequences (`--gui-selftest`)

The four new checks read `WorkbenchDocument::transitions()` (the event log), render each observed
state with `RunState::label()`, and fail unless the exact sequence and terminal count match. Exact
output from the final run (`docs/goal-desktop-v4/round-13/gui-selftest.log`):

```text
GUI_SELFTEST ok run-state-fast: sequence Idle -> Queued -> Running -> Finished; Queued 1.079 ms; Running 0.016 ms; terminal run events 1
GUI_SELFTEST ok run-state-cancel: sequence Idle -> Queued -> Running -> Cancelling -> Cancelled; Queued 1.036 ms; Running 101.647 ms; Cancelling 32.897 ms (wall 32.913 ms); terminal run events 1
GUI_SELFTEST ok run-state-fail: sequence Idle -> Queued -> Running -> Failed; Queued 1.034 ms; Running 0.009 ms; status "run Failed"; terminal run events 1
GUI_SELFTEST ok run-state-stale: sequence Idle -> Queued -> Running -> Finished -> Stale; Queued 1.306 ms; Running 0.006 ms; Finished displayed 0.005 ms before the edit; second edit kept Stale and added no transition; terminal run events 1
```

Summarised:

| Check | Script / gesture | Observed sequence | Terminal run events |
| --- | --- | --- | --- |
| (a) `run-state-fast` | `a = 2` / `b = a + 10` / `b` on the real engine | `Idle -> Queued -> Running -> Finished` | 1 (`Finished`) |
| (b) `run-state-cancel` | `matmul(eye(400), eye(400))`, Esc inside the kernel | `Idle -> Queued -> Running -> Cancelling -> Cancelled` | 1 (`Cancelled`) |
| (c) `run-state-fail` | `a = 2` / `1 +` (syntax error) | `Idle -> Queued -> Running -> Failed` | 1 (`Failed`) |
| (d) `run-state-stale` | successful run, then edit the buffer twice | `Idle -> Queued -> Running -> Finished -> Stale` | 1 (`Finished`; the second edit adds no transition and `Finished` never reappears) |

Additional status-label assertions in the checks and in the existing suite cover every label:
`run Idle` (`two-tabs`), `run Queued` (fast check immediately after submit, UI frame test),
`run Running` (keyboard / long-run / UI frame test), `run Cancelling` (cancel check immediately
after Esc), `run Finished` (arithmetic / fast check), `run Cancelled` (cancel / keyboard),
`run Failed` (syntax-error / fail check) and `run Stale` (stale check).

### Measured durations of transient states

These are `Instant` timestamps captured when the UI thread accepted each transition; a fast run's
`Running` window is genuinely sub-frame, which is exactly why the log is the assertion surface.

| Check | `Queued` | `Running` | `Cancelling` | Other |
| --- | --- | --- | --- | --- |
| (a) fast success | 1.079 ms | 0.016 ms | not entered | `Running -> Finished` 0.016 ms |
| (b) cancelled long run | 1.036 ms | 101.647 ms (includes the deliberate 100 ms wait before Esc) | **32.897 ms** (wall 32.913 ms) | `Cancelling -> Cancelled` 32.897 ms |
| (c) failed run | 1.034 ms | 0.009 ms | not entered | `Running -> Failed` 0.009 ms |
| (d) stale after success | 1.306 ms | 0.006 ms | not entered | `Finished` displayed 0.005 ms before the edit |

The `Queued` duration is measured from `submit` accepted to the `RunStarted` transition being
applied; it is dominated by the 1 ms `wait_until_idle` poll interval. The `Cancelling` duration is
measured from the cancellation transition to the terminal `Cancelled` transition.

## Real-window `run Cancelling`

Command (same shipped binary, `tools/gui-harness.ps1 -Action scenario`; `-FastStatus` uses the
harness's 1 ms cached UIA sampler, `-TraceFile` enables the product's monotonic run-state trace):

```text
powershell -NoProfile -ExecutionPolicy Bypass -File tools/gui-harness.ps1 -Action scenario -FastStatus ^
  -TraceFile docs/goal-desktop-v4/round-13/scenario/run-trace.log ^
  -Exe rust/target-desktop/debug/lovelace-desktop.exe ^
  -OutDir docs/goal-desktop-v4/round-13/scenario ^
  -Log docs/goal-desktop-v4/round-13/scenario/driver.log
```

Two runs were captured: `scenario/` and `scenario-2/`. Both drove
`matmul(eye(400), eye(400))` + Enter, waited for `run Running`, then sent Esc inside the kernel.
The UIA transition tables in `scenario-summary.md` show `run Idle -> run Running -> run Cancelled`:
neither capture sampled the `run Cancelling` frame because the first post-Esc UIA sample happens
only after `SendKeys.SendWait` returns.

The product-side real-window trace did capture `Cancelling` in both runs:

| Run | `run_state_queued` | `run_state_running` | `run_state_cancelling` | `run_state_cancelled` | queued for | `Cancelling` lasted | first post-Esc UIA sample |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `scenario/` | mono 3,604,027 us | 3,611,390 us | 4,339,474 us | 4,365,351 us | 7.363 ms | **25.877 ms** | 34.0 ms -> already `Cancelled` |
| `scenario-2/` | 3,625,310 us | 3,632,243 us | 4,356,846 us | 4,378,473 us | 6.933 ms | **21.627 ms** | 34.4 ms -> already `Cancelled` |

Artifacts:

- `scenario/run-trace.log`, `scenario-2/run-trace.log` - real-window product trace with the four
  `run_state_*` marks above (monotonic microseconds and UTC milliseconds per line).
- `scenario/scenario-running.png` - real window showing `run Running` during the kernel.
- `scenario/scenario-final.png` - real window after the terminal event, showing `run Cancelled`.
- `scenario/scenario-summary.md`, `scenario/driver.log` - UIA transitions and timing.
- `scenario/scenario-running-uia.txt`, `scenario/scenario-final-uia.txt` - UIA trees.

**Explicit limitation:** there is no `run Cancelling` screenshot. The state was too short-lived for
this harness's post-`SendKeys` sampler: measured at 25.877 ms and 21.627 ms in the product trace,
while the first UIA sample after Esc ran 34.0 ms and 34.4 ms after the key event. The real-window
product trace is the captured artifact for `run Cancelling`; the duration is reported above rather
than silently skipped.

## Test summary

Environment: Windows PowerShell 5.1; `RUSTUP_HOME`/`CARGO_HOME`/`CARGO_TARGET_DIR` as in the task,
`TMP`/`TEMP` under `rust\target-desktop\tmp`, and `APPDATA` redirected to a writable workspace
directory for the plot checks.

```text
cargo test -p lovelace-desktop --manifest-path rust/Cargo.toml
test result: ok. 121 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.60s
```

Full log: `docs/goal-desktop-v4/round-13/cargo-test.log`.

```text
lovelace-desktop.exe --gui-selftest
GUI_SELFTEST summary checks=14 failed=0
```

Four checks were added to the ten existing ones; all ten existing checks still run and pass. Full
log: `docs/goal-desktop-v4/round-13/gui-selftest.log`.

The new checks do not plot anything; the plot-producing checks were left as they were, and the
`plot-dir` check was run with `APPDATA` redirected as required.

### Code map

- `rust/lovelace-desktop/src/run_state.rs` - `RunState`, `RunStateMachine`,
  `RunStateTransition`, the transition log, the label mapping and the machine-level tests.
- `rust/lovelace-desktop/src/workbench.rs` - `submit` enters `Queued`, `RunStarted` enters
  `Running`, cancellation enters `Cancelling`, terminal events enter `Finished`/`Cancelled`/
  `Failed`, edits after `Finished` enter `Stale`; `status_text` renders `run <label>`.
- `rust/lovelace-desktop/src/gui_selftest.rs` - the four sequence checks and the `run Idle`
  assertion.
- `rust/lovelace-desktop/src/ui/app.rs` - the window frame test now observes `Queued` in the
  Ctrl+Enter frame, `Running` after the worker reports start, `Cancelling` immediately after Esc
  and `Cancelled` after the terminal event.
- `rust/lovelace-desktop/src/main.rs` - registers the module.
