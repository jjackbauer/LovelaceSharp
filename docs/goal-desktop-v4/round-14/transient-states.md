# RUN-009 transient states: real-window captures (round 14)

Requirement: certify the three transient status labels - `run Queued`, `run Cancelling` and
`run Stale` - with real-window artifacts, not just the headless state-machine sequences.

**Verdict: all three states were captured in the shipped window.** Each PNG below is a real
screen capture of the product window. No image was synthesized, re-rendered or edited. The Queued
PNG is a byte-identical copy of the frame file the harness saved at capture time; SHA-256 hashes
are given below so the equality is checkable.

Environment: Windows 11 Pro (build 26100) desktop session, Windows PowerShell 5.1 (`powershell
-NoProfile -ExecutionPolicy Bypass -File ...`), shipped debug binary
`rust/target-desktop/debug/lovelace-desktop.exe` (64,163,840 bytes, 2026-11-12 11:57). The harness
is `tools/gui-harness.ps1`. No product source was changed for this work; no plot was produced.

## Results

| State | Captured | PNG | Bytes | UIA nodes | Trigger -> UIA match | Trigger -> pixel grab | Attempts | Measured state duration |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `run Cancelling` | **yes** | `docs/goal-desktop-v4/round-14/cancel/run-cancelling.png` | 141,916 | 44 | **6.0 ms** | 41.9 ms (frame 1) | **1** (of 4 allowed) | **28.279 ms** (product trace) |
| `run Stale` | **yes** | `docs/goal-desktop-v4/round-14/stale/run-stale.png` | 135,431 | 47 | **27.6 ms** | 67.3 ms | **1** (of 1) | persistent display state by design |
| `run Queued` | **yes** | `docs/goal-desktop-v4/round-14/queued/run-queued.png` | 130,512 | 46 | 24.4 ms (separate control run) | **46.3 ms** (frame 2 of 3) | **1** (of 5 in the final run; 7 earlier misses) | **8.868 ms** final run; 5.490-8.868 ms across 5 runs |

The "UIA match" column is the sampler offset from the triggering action to the first sample whose
status text matched the requested label. "Pixel grab" is the offset at which the screen copy
completed. They differ because the accessibility label and the composited screen do not update in
lockstep (details below).

The UIA node counts come from the state dumps (`run-cancelling-uia.txt`, `run-queued-uia.txt`,
`run-stale-uia.txt`). For Cancelling and Queued the tree walk starts after the grab and takes
90-160 ms, so the status node inside the dump shows the successor state; the dump header records
the status text that triggered the capture.

## Why the transient states needed new measurement machinery

Four measured facts changed the original plan:

1. **`SendKeys.SendWait` blocks for the whole key-handling window.** The `scenario-running`
   gesture measures 30-53 ms per call. The product handles the key and enters
   Queued/Cancelling *during* that call, so the legacy first post-send sample starts too late.
   The harness now injects the Ctrl+Enter and Esc triggers with `keybd_event` (non-blocking) when
   `-FastKeys` is set, and starts sampling immediately afterwards. In the traced Queued run the
   key was handled 0.05 ms after the frame carrying it began, and `run_state_queued` followed
   0.27 ms later.
2. **`Start-Sleep -Milliseconds 1` is not 1 ms on this host.** Measured 15.5 ms average, which
   cannot resolve the ~8 ms Queued window. The fine sampler therefore keeps the cached status
   element and spins the remainder of each 1 ms interval with `Thread.SpinWait`; actual cadence is
   ~1.3-2.0 ms including the 0.3-1.2 ms cached-element read. No full tree walk happens per poll.
3. **The UIA label lags the paint.** In the final Queued run the state lasted 8.868 ms; in the
   preceding control run the cached status element did not report `run Queued` until 24.4 ms
   after submit, and in the final run the first sample at 63 ms already said Finished. A capture
   triggered only by the UIA label grabs the successor frame. Queued is therefore captured by a
   **submit-triggered grab**: three full-window bitmaps are preallocated *before* the key is
   injected, then copied immediately after it.
4. **The product renders on demand, not continuously.** During a run the UI thread can go >100 ms
   without beginning a frame; a state can be entered and left before any frame is drawn. In the
   successful Cancelling run the UIA label was observed at 6.0 ms, but the first screen copy
   (which still showed `run Running` in an earlier run) needed a repaint. The harness now moves
   the pointer 3 px inside the window at the matching sample (`-WakeRepaint`). That is a real
   input event which asks egui to repaint; it does not click, does not change focus and does not
   touch the run state. A 3-frame burst (`-CaptureBurst 3`, 8 ms apart) then gives the compositor
   more than one chance to expose the frame.

Screen frames persist after the state machine has moved on. In the Cancelling run the state
lasted 28.279 ms while the screen still displayed the Cancelling frame at 41.9 ms and 69.6 ms;
the third frame at 94.0 ms shows Cancelled. The PNG is the product's own Cancelling frame, not a
reconstruction: the status strip reads `run Cancelling`, the log pane contains
`[info] Cancellation requested.` and not the terminal line, the toolbar Cancel button is active,
and the variables pane still says `No run has published a workspace yet.`

## Per-state evidence

### `run Cancelling`

- Command: `-Flow cancel -FastStatus -FastKeys -WakeRepaint -CaptureBurst 3 -CaptureStates "run Cancelling" -MaxAttempts 4 -CancelAfterMs 300`
- Script: `matmul(eye(400), eye(400))`, Ctrl+Enter, then Esc 300 ms after `run Running`.
- Capture record: status at trigger `... | run Cancelling`; UIA match **6.0 ms**; frame 1 grab
  **41.9 ms** (141,916 bytes), frame 2 at 69.6 ms (identical bytes), frame 3 at 94.0 ms (144,831
  bytes, shows `run Cancelled`).
- UIA dump: **44 nodes**; header `status_at_trigger='... run Cancelling'`; the live status node
  line inside the dump reads `run Cancelled` because the walk completed at 136.3 ms.
- Product trace (`cancel/run-trace.log`): `run_state_queued` -> `run_state_running` 7.643 ms;
  `run_state_cancelling` -> `run_state_cancelled` **28.279 ms**. Final terminal was
  `run Cancelled` after 741 ms of kernel work.
- Attempt count: **1**. Attempts 2-4 were available but not needed. Two earlier verification runs
  observed the label on the first attempt; the one without `-WakeRepaint` produced only a
  `run Running` frame (the UIA label said Cancelling), which is why wake+burst is now part of the
  command.

### `run Stale`

- Command: `-Flow stale -FastStatus -CaptureStates "run Stale" -Keys "1 + 1" -MaxAttempts 1`
- Gesture: type `1 + 1`, Enter, park caret, Ctrl+Enter, wait for `run Finished` (observed 54.9 ms
  after the send), click the editor to restore keyboard focus, type one character (`x`).
- Capture record: status at trigger `... rev 7 ... | run Stale`; UIA match **27.6 ms**; grab
  **67.3 ms** (135,431 bytes). The 27 ms send call plus the immediate grab make up the offset.
- The PNG shows `run Stale`, rev 7, caret column 7, the Variables pane holding `_ = 2 (Natural)`,
  and the log line `[info] The displayed result is stale: the source changed after it finished.`
- UIA dump: **47 nodes**; the live status node reads `run Stale` (the state persists, so no
  successor caveat).
- Measured duration: not applicable. Stale is the terminal display state after an edit following
  Finished; it lasts until the next edit or submit.
- Attempt count: **1**.

### `run Queued`

- Final command: `-Flow queued -FastStatus -FastKeys -SubmitGrab -Keys "1 + 1" -MaxAttempts 5`
  (no `-CaptureStates`: the pixels are grabbed at the submit action because the UIA label lags).
- Gesture: type `1 + 1`, Enter, park caret, Ctrl+Enter. The flow repeats up to 5 times, clicking
  the editor between attempts.
- Pixel record: the harness preallocated three 1296x839 bitmaps before injecting Ctrl+Enter and
  copied the screen immediately after the injection. In the successful final run:
  frame 1 at **30.8 ms** shows `run Idle`; frame 2 at **46.3 ms** shows **`run Queued`**
  (130,512 bytes); frame 3 at 58.2 ms shows `run Finished`. `run-queued.png` is a byte-identical
  copy of frame 2 (same SHA-256).
- The PNG shows `run Queued` in the status strip, the log line
  `[info] Run queued (revision 6, run 1).` and no "Run started"/"Run finished" lines, the Cancel
  button active, and `No run has published a workspace yet.` in the Variables pane.
- UIA dump: **46 nodes** from the separate UIA-triggered control attempt; its header records
  `status_at_trigger='... run Queued'`, while the walked status node shows `run Finished` because
  the walk happened after the state ended.
- Measured duration: the product trace gives 8.868 ms (final run) and 5.490, 6.541, 5.652 and
  6.814 ms in the other four runs - **5.49-8.87 ms in the real window**, not the ~1 ms headless
  figure. The headless number measures the state machine alone; in the window the worker's
  `RunStarted` is applied on the UI thread's next poll/frame, which is where the extra ~5-8 ms
  comes from.
- Attempt count: **1** in the final run (frame 2 of 3). Earlier attempts that missed:
  one UIA-triggered attempt (label matched at 24.4 ms, but its late grab at 63.1 ms showed
  `run Finished`; archived under `queued/uia-trigger/` as a negative control) and six
  single-grab submit attempts (each grab showed Idle or Finished). The 3-frame burst captured the
  state on its first attempt once it was used.

## Harness changes (`tools/gui-harness.ps1` only)

New parameters:

| Parameter | Purpose |
| --- | --- |
| `-CaptureStates "run Cancelling,run Queued"` | Requested status labels (comma-separated array). The first sample whose status text matches saves `<slug>.png`, `<slug>-uia.txt`, and logs the trigger offset; sampling continues. |
| `-Flow cancel\|stale\|queued` | Gesture to drive. `cancel` keeps the legacy behaviour when no new options are used. |
| `-MaxAttempts N` | Capture attempts per flow (re-submits / re-runs). |
| `-FastKeys` | Inject Ctrl+Enter and Esc with `keybd_event` instead of blocking `SendKeys.SendWait`. |
| `-CancelAfterMs 300` | Delay between observing Running and pressing Esc. |
| `-StaleChar x` | Character typed after Finished to reach Stale. |
| `-WakeRepaint` | At the matching sample, move the pointer 3 px inside the window to request an egui repaint before grabbing. No click, no focus change. |
| `-CaptureBurst N` | Number of full-window frames grabbed at the matching sample (8 ms apart) and saved as `<slug>.png`, `<slug>-frame2.png`, ... |
| `-SubmitGrab` | Queued only: preallocate three bitmaps before Ctrl+Enter and copy the screen three times immediately after injection, before the UIA label can catch up. |

Behaviour added:

- Capture-on-state in both sampling loops, including PNG (no focus call, no sleep, pixels first),
  UIA walk and a driver-log line with match/grab/dump/total offsets and the attempt number.
- Fine sampler: cached status element plus a `Stopwatch` spin gate that holds a real ~1 ms
  interval (`Start-Sleep -Milliseconds 1` measured 15.5 ms here).
- `keybd_event` trigger path (0.5-5.5 ms per send instead of 30-53 ms).
- Flow-specific attempt loops, per-attempt observations (first sample, terminal, Queued/Cancelling
  sampled offsets and gaps) and a driver summary including requested-state attempts, captures,
  frame offsets and product-trace durations.
- Product-trace parsing (`Get-RunTraceTimings`) so the summary reports the sub-millisecond state
  durations from `run_state_*`.
- Tab-selection verification/retry: the first tab click can be consumed as the activation click
  after a launch; the setup now re-reads the tab rect and retries up to four times.
- `Dump-UiaTree` accepts extra header lines so state dumps record the trigger status.

Default `scenario` behaviour with no new options is unchanged (legacy Running/Cancelled gesture,
Running PNG + final PNG, caret checks, same summary). Only the tab retry is shared. Regression
check after the changes: the default scenario returned `SCENARIO_OK` (Ctrl+Enter -> Running
125.4 ms, Esc -> Cancelled 68.3 ms, 44-node running/final dumps); its artifacts are in
`legacy-smoke/`.

## Reproduction

The product opens `LOVELACE_RUN_TRACE` in append mode. Delete the per-flow `run-trace.log` before
rerunning a command, or the summary will also list the previous process's runs.

```text
powershell -NoProfile -ExecutionPolicy Bypass -File tools\gui-harness.ps1 -Action scenario -Flow cancel -FastStatus -FastKeys -WakeRepaint -CaptureBurst 3 -CaptureStates "run Cancelling" -MaxAttempts 4 -CancelAfterMs 300 -TraceFile docs\goal-desktop-v4\round-14\cancel\run-trace.log -Exe rust\target-desktop\debug\lovelace-desktop.exe -OutDir docs\goal-desktop-v4\round-14\cancel -Log docs\goal-desktop-v4\round-14\cancel\driver.log

powershell -NoProfile -ExecutionPolicy Bypass -File tools\gui-harness.ps1 -Action scenario -Flow stale -FastStatus -CaptureStates "run Stale" -Keys "1 + 1" -MaxAttempts 1 -TraceFile docs\goal-desktop-v4\round-14\stale\run-trace.log -Exe rust\target-desktop\debug\lovelace-desktop.exe -OutDir docs\goal-desktop-v4\round-14\stale -Log docs\goal-desktop-v4\round-14\stale\driver.log

powershell -NoProfile -ExecutionPolicy Bypass -File tools\gui-harness.ps1 -Action scenario -Flow queued -FastStatus -FastKeys -SubmitGrab -Keys "1 + 1" -MaxAttempts 5 -TraceFile docs\goal-desktop-v4\round-14\queued\run-trace.log -Exe rust\target-desktop\debug\lovelace-desktop.exe -OutDir docs\goal-desktop-v4\round-14\queued -Log docs\goal-desktop-v4\round-14\queued\driver.log
```

## Artifact inventory

| File | Bytes / nodes | SHA-256 (PNG only) |
| --- | --- | --- |
| `cancel/run-cancelling.png` | 141,916 | `da04bec04448f01611cd6a7dbdd759950af01beb6d521a3e05be080c71c3a81d` |
| `cancel/run-cancelling-frame2.png` | 141,916 | (same image as frame 1) |
| `cancel/run-cancelling-frame3.png` | 144,831 | `8d5fe52c8e3487482dfd69b33d288b0aa83b937bce31a4cf80de735252affaa4` (Cancelled successor) |
| `cancel/run-cancelling-uia.txt` | 44 nodes | - |
| `cancel/run-trace.log` | product trace | - |
| `stale/run-stale.png` | 135,431 | `a31c44414136c4aa691d7ed78b5aaf80c53ab204c341ee01480660d4d70062d3` |
| `stale/run-stale-uia.txt` | 47 nodes | - |
| `queued/run-queued.png` | 130,512 | `1fa92cad84eac91efe5112f0aa315949995cfff295045d5843a30e461e101504` |
| `queued/run-queued-submit-attempt1-f2.png` | 130,512 | same as `run-queued.png` (the copy source) |
| `queued/uia-trigger/run-queued-uia.txt` | 46 nodes | - |
| `queued/run-trace.log` | product trace (5 runs) | - |

Per-flow driver logs, UIA dumps and `scenario-summary.md` sit beside each PNG. The archived
`queued/uia-trigger/` directory keeps the UIA-triggered attempt that produced only a post-state
frame (`run-queued.png` showing `run Finished`) as an explicit negative control; it is not used as
Queued evidence.

## Limitations and notes

- **The pixel grab can complete after the state ends.** This is the product's asynchronous paint:
  the screen retained the Queued/Cancelling frame while the state machine had already advanced.
  The state's own frame is what the PNG shows; the product trace gives the true state interval.
  No frame was painted on demand for the report, and no image was assembled from parts.
- **UIA dumps are post-trigger.** A full 44-47 node raw walk takes 90-160 ms, longer than either
  transient state. The header records the trigger status; the status node inside the dump may show
  the successor. The node count is unaffected.
- **Stale has no duration.** It is entered by an edit after Finished and persists until the next
  edit or submit.
- **Cancelling duration varies.** The final run measured 28.279 ms; earlier real-window runs
  measured 25.9, 21.6 and 25.7 ms. All are consistent with the 21-36 ms observed range.
- **Queued is not ~1 ms in the window.** The real-window trace measures 5.49-8.87 ms because the
  worker's RunStarted is applied on the UI thread's poll/frame, not synchronously at submit. The
  ~1.1 ms headless number is the state-machine-only interval.
- The `run-queued.png` / `run-queued-submit-attempt1-f2.png` pair is a file copy, not two
  captures; the SHA-256 is recorded so the equality is checkable.
