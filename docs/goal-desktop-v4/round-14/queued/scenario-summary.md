# Round 14 transient-state GUI scenario evidence

Generated: 2026-11-12 12:16:16.696 -03:00
Flow: **queued**; attempts: 5 of 5
Driver: tools/gui-harness.ps1 -Action scenario -Flow queued -FastStatus -CaptureStates '' -MaxAttempts 5 -FastKeys:True -WakeRepaint:False -CaptureBurst 1
Product: rust\target-desktop\debug\lovelace-desktop.exe
PID: 22196 (closed at the end of the gesture)
Driver log: docs\goal-desktop-v4\round-14\queued\driver.log

## Window and UIA readiness

Window: hwnd=209390660 rect=1296x839 at 26,26 after 1125 ms (restored_from_minimized=False).
UIA: 43 descendants from FindAll(Descendants) after 67 ms.
Script: `1 + 1` (typed with SendKeys, foreground asserted before every send)

## Sampler

Fine sampler: one cached status element; the loop spins to a real 1 ms cadence. `Start-Sleep -Milliseconds 1` was measured at ~15.5 ms on this host, so the spike would have missed the 7 ms Queued window; the gate uses a Stopwatch spin instead. The read itself costs 0.3-1.2 ms.
Trigger keys are injected with `keybd_event` (non-blocking). `SendKeys.SendWait` was measured blocking for 30-53 ms, which is exactly the window in which Queued/Cancelling begin, so the sampler starts immediately after injection instead.
Pixels: one immediate grab at the matching sample, without waking the renderer.

## Requested states

| state | attempts | captured | attempt | match ms | grab ms | dump ms | PNG | bytes | UIA nodes | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |

## Per-attempt sampling observations

| flow | attempt | first sample | terminal | Queued sampled at | Queued->Running gap | Cancelling sampled at | Cancelling->terminal gap | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| queued | 1 | 63.1 ms, untitled.lv  |  rev 6  |  precision 1000  |  Ln 1, Col 6  |  run Finished | 63.1 ms, untitled.lv  |  rev 6  |  precision 1000  |  Ln 1, Col 6  |  run Finished | - | - | - | - | ctrl+enter_send=5.5ms first_query=0.9ms |
| queued | 2 | 36.5 ms, untitled.lv  |  rev 6  |  precision 1000  |  Ln 1, Col 6  |  run Finished | 36.5 ms, untitled.lv  |  rev 6  |  precision 1000  |  Ln 1, Col 6  |  run Finished | - | - | - | - | ctrl+enter_send=0.5ms first_query=0.4ms |
| queued | 3 | 40.5 ms, untitled.lv  |  rev 6  |  precision 1000  |  Ln 1, Col 6  |  run Finished | 40.5 ms, untitled.lv  |  rev 6  |  precision 1000  |  Ln 1, Col 6  |  run Finished | - | - | - | - | ctrl+enter_send=0.6ms first_query=0.4ms |
| queued | 4 | 65.9 ms, untitled.lv  |  rev 6  |  precision 1000  |  Ln 1, Col 6  |  run Finished | 65.9 ms, untitled.lv  |  rev 6  |  precision 1000  |  Ln 1, Col 6  |  run Finished | - | - | - | - | ctrl+enter_send=0.5ms first_query=0.4ms |
| queued | 5 | 49 ms, untitled.lv  |  rev 6  |  precision 1000  |  Ln 1, Col 6  |  run Finished | 49 ms, untitled.lv  |  rev 6  |  precision 1000  |  Ln 1, Col 6  |  run Finished | - | - | - | - | ctrl+enter_send=0.6ms first_query=0.4ms |

## Submit-triggered pixel grabs

Preallocated bitmap copied immediately after the Ctrl+Enter injection, before the UIA label can catch up.

| attempt | frame | grab ms after submit | bytes | file |
| --- | --- | --- | --- | --- |
| 1 | 1 | 30.8 | 127037 | docs\goal-desktop-v4\round-14\queued\run-queued-submit-attempt1-f1.png |
| 1 | 2 | 46.3 | 130512 | docs\goal-desktop-v4\round-14\queued\run-queued-submit-attempt1-f2.png |
| 1 | 3 | 58.2 | 134921 | docs\goal-desktop-v4\round-14\queued\run-queued-submit-attempt1-f3.png |
| 2 | 1 | 11 | 134978 | docs\goal-desktop-v4\round-14\queued\run-queued-submit-attempt2-f1.png |
| 2 | 2 | 22.1 | 140645 | docs\goal-desktop-v4\round-14\queued\run-queued-submit-attempt2-f2.png |
| 2 | 3 | 34.2 | 140645 | docs\goal-desktop-v4\round-14\queued\run-queued-submit-attempt2-f3.png |
| 3 | 1 | 15.1 | 140687 | docs\goal-desktop-v4\round-14\queued\run-queued-submit-attempt3-f1.png |
| 3 | 2 | 25.6 | 145345 | docs\goal-desktop-v4\round-14\queued\run-queued-submit-attempt3-f2.png |
| 3 | 3 | 38.1 | 141603 | docs\goal-desktop-v4\round-14\queued\run-queued-submit-attempt3-f3.png |
| 4 | 1 | 14 | 140756 | docs\goal-desktop-v4\round-14\queued\run-queued-submit-attempt4-f1.png |
| 4 | 2 | 31.5 | 140743 | docs\goal-desktop-v4\round-14\queued\run-queued-submit-attempt4-f2.png |
| 4 | 3 | 63.6 | 141588 | docs\goal-desktop-v4\round-14\queued\run-queued-submit-attempt4-f3.png |
| 5 | 1 | 17.6 | 140692 | docs\goal-desktop-v4\round-14\queued\run-queued-submit-attempt5-f1.png |
| 5 | 2 | 34 | 145339 | docs\goal-desktop-v4\round-14\queued\run-queued-submit-attempt5-f2.png |
| 5 | 3 | 46.8 | 141563 | docs\goal-desktop-v4\round-14\queued\run-queued-submit-attempt5-f3.png |

## Product trace timings (sub-millisecond source)

| run | Queued (ms) | Cancelling (ms) |
| --- | --- | --- |
| 1 | 8.868 | - |
| 2 | 5.49 | - |
| 3 | 6.541 | - |
| 4 | 5.652 | - |
| 5 | 6.814 | - |

Queued duration: min 5.49 ms, max 8.868 ms across 5 run(s).

Trace: docs\goal-desktop-v4\round-14\queued\run-trace.log

## Logged status transitions

| phase | wall clock | elapsed ms | status |
| --- | --- | --- | --- |
| pre-run | 12:16:08.330 |  | untitled.lv  \|  rev 6  \|  precision 1000  \|  Ln 1, Col 6  \|  run Idle |
| ctrl+enter | 12:16:08.421 | 63.1 | untitled.lv  \|  rev 6  \|  precision 1000  \|  Ln 1, Col 6  \|  run Finished |

## Captured frames

| state | frame | grab ms | bytes | file |
| --- | --- | --- | --- | --- |

## Artifacts

| file | size / node count |
| --- | --- |
| scenario-final.png | 140673 bytes |
| scenario-final-uia.txt | 63 UIA nodes |
| scenario-summary.md | this file |

