# Round 14 transient-state GUI scenario evidence

Generated: 2026-11-12 12:11:50.215 -03:00
Flow: **queued**; attempts: 1 of 8
Driver: tools/gui-harness.ps1 -Action scenario -Flow queued -FastStatus -CaptureStates 'run Queued' -MaxAttempts 8 -FastKeys:True -WakeRepaint:True -CaptureBurst 3
Product: rust\target-desktop\debug\lovelace-desktop.exe
PID: 33344 (closed at the end of the gesture)
Driver log: docs\goal-desktop-v4\round-14\queued\driver.log

## Window and UIA readiness

Window: hwnd=127601854 rect=1296x839 at 234,234 after 1092 ms (restored_from_minimized=False).
UIA: 43 descendants from FindAll(Descendants) after 68 ms.
Script: `1 + 1` (typed with SendKeys, foreground asserted before every send)

## Sampler

Fine sampler: one cached status element; the loop spins to a real 1 ms cadence. `Start-Sleep -Milliseconds 1` was measured at ~15.5 ms on this host, so the spike would have missed the 7 ms Queued window; the gate uses a Stopwatch spin instead. The read itself costs 0.3-1.2 ms.
Trigger keys are injected with `keybd_event` (non-blocking). `SendKeys.SendWait` was measured blocking for 30-53 ms, which is exactly the window in which Queued/Cancelling begin, so the sampler starts immediately after injection instead.
Pixels: at the matching sample the driver moves the pointer 3 px inside the window (a real input event that requests an egui repaint, with no click and no run-state change), then grabs 3 frame(s) 8 ms apart. The product trace shows whether a frame was begun while the state was current; the PNG set is the pixel evidence for that timing.

## Requested states

| state | attempts | captured | attempt | match ms | grab ms | dump ms | PNG | bytes | UIA nodes | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| run Queued | 1 | yes | 1 | 24.4 | 63.1 | 161.6 | docs\goal-desktop-v4\round-14\queued\run-queued.png | 134856 | 46 | status='untitled.lv  |  rev 6  |  precision 1000  |  Ln 1, Col 6  |  run Queued' trigger=ctrl+enter |

## Per-attempt sampling observations

| flow | attempt | first sample | terminal | Queued sampled at | Queued->Running gap | Cancelling sampled at | Cancelling->terminal gap | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| queued | 1 | 8.4 ms, untitled.lv  |  rev 6  |  precision 1000  |  Ln 1, Col 6  |  run Idle | 237.2 ms, untitled.lv  |  rev 6  |  precision 1000  |  Ln 1, Col 6  |  run Finished | 24.4 ms | - | - | - | ctrl+enter_send=4.2ms first_query=0.9ms |

## Product trace timings (sub-millisecond source)

| run | Queued (ms) | Cancelling (ms) |
| --- | --- | --- |
| 1 | 8.453 | - |

Queued duration: min 8.453 ms, max 8.453 ms across 1 run(s).

Trace: docs\goal-desktop-v4\round-14\queued\run-trace.log

## Logged status transitions

| phase | wall clock | elapsed ms | status |
| --- | --- | --- | --- |
| pre-run | 12:11:49.440 |  | untitled.lv  \|  rev 6  \|  precision 1000  \|  Ln 1, Col 6  \|  run Idle |
| ctrl+enter | 12:11:49.694 | 24.4 | untitled.lv  \|  rev 6  \|  precision 1000  \|  Ln 1, Col 6  \|  run Queued |
| ctrl+enter | 12:11:49.696 | 237.2 | untitled.lv  \|  rev 6  \|  precision 1000  \|  Ln 1, Col 6  \|  run Finished |

## Captured frames

| state | frame | grab ms | bytes | file |
| --- | --- | --- | --- | --- |
| run Queued | 1 | 63.1 | 134856 | docs\goal-desktop-v4\round-14\queued\run-queued.png |
| run Queued | 2 | 84.4 | 134856 | docs\goal-desktop-v4\round-14\queued\run-queued-frame2.png |
| run Queued | 3 | 102.5 | 134856 | docs\goal-desktop-v4\round-14\queued\run-queued-frame3.png |

## Artifacts

| file | size / node count |
| --- | --- |
| scenario-final.png | 134856 bytes |
| docs\goal-desktop-v4\round-14\queued\run-queued.png | 134856 bytes |
| docs\goal-desktop-v4\round-14\queued\run-queued-uia.txt | 46 UIA nodes |
| scenario-final-uia.txt | 46 UIA nodes |
| scenario-summary.md | this file |

