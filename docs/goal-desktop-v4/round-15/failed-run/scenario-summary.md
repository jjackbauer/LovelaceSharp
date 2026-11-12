# Round 14 transient-state GUI scenario evidence

Generated: 2026-11-12 12:29:43.773 -03:00
Flow: **queued**; attempts: 1 of 3
Driver: tools/gui-harness.ps1 -Action scenario -Flow queued -FastStatus -CaptureStates 'run Failed' -MaxAttempts 3 -FastKeys:True -WakeRepaint:True -CaptureBurst 1 -SubmitGrab:False
Product: C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop\debug\lovelace-desktop.exe
PID: 35816 (closed at the end of the gesture)
Driver log: C:\Users\ricar\dev\LovelaceSharp\docs\goal-desktop-v4\round-15\failed-run\driver.log

## Window and UIA readiness

Window: hwnd=117770862 rect=1296x839 at 26,26 after 1273 ms (restored_from_minimized=False).
UIA: 43 descendants from FindAll(Descendants) after 77 ms.
Script: `a = 1; 1 / 0` (typed with SendKeys, foreground asserted before every send)

## Sampler

Fine sampler: one cached status element; the loop spins to a real 1 ms cadence. `Start-Sleep -Milliseconds 1` was measured at ~15.5 ms on this host, so the spike would have missed the 7 ms Queued window; the gate uses a Stopwatch spin instead. The read itself costs 0.3-1.2 ms.
Trigger keys are injected with `keybd_event` (non-blocking). `SendKeys.SendWait` was measured blocking for 30-53 ms, which is exactly the window in which Queued/Cancelling begin, so the sampler starts immediately after injection instead.
Pixels: at the matching sample the driver moves the pointer 3 px inside the window (a real input event that requests an egui repaint, with no click and no run-state change), then grabs 1 frame(s) 8 ms apart. The product trace shows whether a frame was begun while the state was current; the PNG set is the pixel evidence for that timing.

## Requested states

| state | attempts | captured | attempt | match ms | grab ms | dump ms | PNG | bytes | UIA nodes | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| run Failed | 0 | yes | 1 | 31.6 | 72.2 | 181.7 | C:\Users\ricar\dev\LovelaceSharp\docs\goal-desktop-v4\round-15\failed-run\run-failed.png | 147728 | 50 | status='untitled.lv  \|  rev 13  \|  precision 1000  \|  Ln 1, Col 13  \|  run Failed' trigger=ctrl+enter |

## Per-attempt sampling observations

| flow | attempt | first sample | terminal | Queued sampled at | Queued->Running gap | Cancelling sampled at | Cancelling->terminal gap | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| queued | 1 | 9.6 ms, untitled.lv  \|  rev 13  \|  precision 1000  \|  Ln 1, Col 13  \|  run Idle | 31.6 ms, untitled.lv  \|  rev 13  \|  precision 1000  \|  Ln 1, Col 13  \|  run Failed | 26.6 ms | - | - | - | ctrl+enter_send=4.5ms first_query=0.9ms |


## Logged status transitions

| phase | wall clock | elapsed ms | status |
| --- | --- | --- | --- |
| pre-run | 12:29:42.992 |  | untitled.lv  \|  rev 13  \|  precision 1000  \|  Ln 1, Col 13  \|  run Idle |
| ctrl+enter | 12:29:43.038 | 26.6 | untitled.lv  \|  rev 13  \|  precision 1000  \|  Ln 1, Col 13  \|  run Queued |
| ctrl+enter | 12:29:43.229 | 31.6 | untitled.lv  \|  rev 13  \|  precision 1000  \|  Ln 1, Col 13  \|  run Failed |

## Captured frames

| state | frame | grab ms | bytes | file |
| --- | --- | --- | --- | --- |
| run Failed | 1 | 72.2 | 147728 | C:\Users\ricar\dev\LovelaceSharp\docs\goal-desktop-v4\round-15\failed-run\run-failed.png |

## Artifacts

| file | size / node count |
| --- | --- |
| scenario-final.png | 145025 bytes |
| C:\Users\ricar\dev\LovelaceSharp\docs\goal-desktop-v4\round-15\failed-run\run-failed.png | 147728 bytes |
| C:\Users\ricar\dev\LovelaceSharp\docs\goal-desktop-v4\round-15\failed-run\run-failed-uia.txt | 50 UIA nodes |
| scenario-final-uia.txt | 50 UIA nodes |
| scenario-summary.md | this file |

