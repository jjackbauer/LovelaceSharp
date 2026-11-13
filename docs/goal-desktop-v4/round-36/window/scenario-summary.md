# Round 14 transient-state GUI scenario evidence

Generated: 2026-11-13 06:35:55.578 -03:00
Flow: **stale**; attempts: 1 of 1
Driver: tools/gui-harness.ps1 -Action scenario -Flow stale -FastStatus -CaptureStates '' -MaxAttempts 1 -FastKeys:False -WakeRepaint:False -CaptureBurst 1 -SubmitGrab:False
Product: C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop\debug\lovelace-desktop.exe
PID: 37880 (closed at the end of the gesture)
Driver log: C:\Users\ricar\dev\LovelaceSharp\docs\goal-desktop-v4\round-36\window\driver.log

## Window and UIA readiness

Window: hwnd=988162 rect=1296x839 at 156,156 after 1559 ms (restored_from_minimized=False).
UIA: 76 descendants from FindAll(Descendants) after 113 ms.
Script: `a = 1` (typed with SendKeys, foreground asserted before every send)

## Sampler

Fine sampler: one cached status element; the loop spins to a real 1 ms cadence. `Start-Sleep -Milliseconds 1` was measured at ~15.5 ms on this host, so the spike would have missed the 7 ms Queued window; the gate uses a Stopwatch spin instead. The read itself costs 0.3-1.2 ms.
Trigger keys use the legacy blocking `SendKeys.SendWait`; transient states that begin during the send can be missed.
Pixels: one immediate grab at the matching sample, without waking the renderer.

## Requested states

| state | attempts | captured | attempt | match ms | grab ms | dump ms | PNG | bytes | UIA nodes | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |

## Per-attempt sampling observations

| flow | attempt | first sample | terminal | Queued sampled at | Queued->Running gap | Cancelling sampled at | Cancelling->terminal gap | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| stale | 1 | 67.3 ms, untitled.lv  \|  rev 5  \|  precision 1000  \|  Ln 1, Col 6  \|  run Queued | 154.6 ms, run Stale | 67.3 ms | - | - | - | finished_at=212.8ms edit_send=29.1ms edit_char='x' |


## Logged status transitions

| phase | wall clock | elapsed ms | status |
| --- | --- | --- | --- |
| pre-run | 06:35:47.893 |  | untitled.lv  \|  rev 5  \|  precision 1000  \|  Ln 1, Col 6  \|  run Idle |
| ctrl+enter | 06:35:47.992 | 67.3 | untitled.lv  \|  rev 5  \|  precision 1000  \|  Ln 1, Col 6  \|  run Queued |
| ctrl+enter | 06:35:48.123 | 212.8 | untitled.lv  \|  rev 5  \|  precision 1000  \|  Ln 1, Col 6  \|  run Finished  \|  1/1 · reused 0 |
| stale-edit | 06:35:49.409 | 154.6 | untitled.lv  \|  rev 6  \|  precision 1000  \|  Ln 1, Col 7  \|  run Stale  \|  1/1 · reused 0 |

## Captured frames

| state | frame | grab ms | bytes | file |
| --- | --- | --- | --- | --- |

## Artifacts

| file | size / node count |
| --- | --- |
| scenario-final.png | 171782 bytes |
| scenario-final-uia.txt | 579 UIA nodes |
| scenario-summary.md | this file |

