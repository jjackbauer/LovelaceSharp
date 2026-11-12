# Round 14 transient-state GUI scenario evidence

Generated: 2026-11-12 12:15:42.539 -03:00
Flow: **cancel**; attempts: 1 of 4
Driver: tools/gui-harness.ps1 -Action scenario -Flow cancel -FastStatus -CaptureStates 'run Cancelling' -MaxAttempts 4 -FastKeys:True -WakeRepaint:True -CaptureBurst 3
Product: rust\target-desktop\debug\lovelace-desktop.exe
PID: 30364 (closed at the end of the gesture)
Driver log: docs\goal-desktop-v4\round-14\cancel\driver.log

## Window and UIA readiness

Window: hwnd=51055468 rect=1296x839 at 234,234 after 1216 ms (restored_from_minimized=False).
UIA: 43 descendants from FindAll(Descendants) after 82 ms.
Script: `matmul(eye(400), eye(400))` (typed with SendKeys, foreground asserted before every send)

## Sampler

Fine sampler: one cached status element; the loop spins to a real 1 ms cadence. `Start-Sleep -Milliseconds 1` was measured at ~15.5 ms on this host, so the spike would have missed the 7 ms Queued window; the gate uses a Stopwatch spin instead. The read itself costs 0.3-1.2 ms.
Trigger keys are injected with `keybd_event` (non-blocking). `SendKeys.SendWait` was measured blocking for 30-53 ms, which is exactly the window in which Queued/Cancelling begin, so the sampler starts immediately after injection instead.
Pixels: at the matching sample the driver moves the pointer 3 px inside the window (a real input event that requests an egui repaint, with no click and no run-state change), then grabs 3 frame(s) 8 ms apart. The product trace shows whether a frame was begun while the state was current; the PNG set is the pixel evidence for that timing.

## Requested states

| state | attempts | captured | attempt | match ms | grab ms | dump ms | PNG | bytes | UIA nodes | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| run Cancelling | 1 | yes | 1 | 6 | 41.9 | 136.3 | docs\goal-desktop-v4\round-14\cancel\run-cancelling.png | 141916 | 44 | status='untitled.lv  |  rev 27  |  precision 1000  |  Ln 1, Col 27  |  run Cancelling' trigger=escape |

## Per-attempt sampling observations

| flow | attempt | first sample | terminal | Queued sampled at | Queued->Running gap | Cancelling sampled at | Cancelling->terminal gap | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| cancel | 1 | - | 207.2 ms, untitled.lv  |  rev 27  |  precision 1000  |  Ln 1, Col 27  |  run Cancelled | - | - | 6 ms | 201.2 ms | ctrl+enter+4.2ms esc+0.6ms |

## Product trace timings (sub-millisecond source)

| run | Queued (ms) | Cancelling (ms) |
| --- | --- | --- |
| 1 | 7.643 | 28.279 |

Queued duration: min 7.643 ms, max 7.643 ms across 1 run(s).

Trace: docs\goal-desktop-v4\round-14\cancel\run-trace.log

## Logged status transitions

| phase | wall clock | elapsed ms | status |
| --- | --- | --- | --- |
| pre-run | 12:15:41.056 |  | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Idle |
| ctrl+enter | 12:15:41.100 | 21.2 | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Running |
| escape | 12:15:42.000 | 6 | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Cancelling |
| escape | 12:15:42.002 | 207.2 | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Cancelled |

## Captured frames

| state | frame | grab ms | bytes | file |
| --- | --- | --- | --- | --- |
| run Cancelling | 1 | 41.9 | 141916 | docs\goal-desktop-v4\round-14\cancel\run-cancelling.png |
| run Cancelling | 2 | 69.6 | 141916 | docs\goal-desktop-v4\round-14\cancel\run-cancelling-frame2.png |
| run Cancelling | 3 | 94 | 144831 | docs\goal-desktop-v4\round-14\cancel\run-cancelling-frame3.png |

## Artifacts

| file | size / node count |
| --- | --- |
| scenario-running.png | 138256 bytes |
| scenario-final.png | 144831 bytes |
| docs\goal-desktop-v4\round-14\cancel\run-cancelling.png | 141916 bytes |
| docs\goal-desktop-v4\round-14\cancel\run-cancelling-uia.txt | 44 UIA nodes |
| scenario-final-uia.txt | 44 UIA nodes |
| scenario-summary.md | this file |

