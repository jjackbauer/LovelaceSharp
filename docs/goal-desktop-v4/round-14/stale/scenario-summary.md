# Round 14 transient-state GUI scenario evidence

Generated: 2026-11-12 12:11:34.083 -03:00
Flow: **stale**; attempts: 1 of 1
Driver: tools/gui-harness.ps1 -Action scenario -Flow stale -FastStatus -CaptureStates 'run Stale' -MaxAttempts 1 -FastKeys:False -WakeRepaint:False -CaptureBurst 1
Product: rust\target-desktop\debug\lovelace-desktop.exe
PID: 28264 (closed at the end of the gesture)
Driver log: docs\goal-desktop-v4\round-14\stale\driver.log

## Window and UIA readiness

Window: hwnd=71241298 rect=1296x839 at 130,130 after 1110 ms (restored_from_minimized=False).
UIA: 43 descendants from FindAll(Descendants) after 69 ms.
Script: `1 + 1` (typed with SendKeys, foreground asserted before every send)

## Sampler

Fine sampler: one cached status element; the loop spins to a real 1 ms cadence. `Start-Sleep -Milliseconds 1` was measured at ~15.5 ms on this host, so the spike would have missed the 7 ms Queued window; the gate uses a Stopwatch spin instead. The read itself costs 0.3-1.2 ms.
Trigger keys use the legacy blocking `SendKeys.SendWait`; transient states that begin during the send can be missed.
Pixels: one immediate grab at the matching sample, without waking the renderer.

## Requested states

| state | attempts | captured | attempt | match ms | grab ms | dump ms | PNG | bytes | UIA nodes | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| run Stale | 1 | yes | 1 | 27.6 | 67.3 | 128.1 | docs\goal-desktop-v4\round-14\stale\run-stale.png | 135431 | 47 | status='untitled.lv  |  rev 7  |  precision 1000  |  Ln 1, Col 7  |  run Stale' trigger=stale-edit |

## Per-attempt sampling observations

| flow | attempt | first sample | terminal | Queued sampled at | Queued->Running gap | Cancelling sampled at | Cancelling->terminal gap | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| stale | 1 | 54.9 ms, untitled.lv  |  rev 6  |  precision 1000  |  Ln 1, Col 6  |  run Finished | 27.6 ms, run Stale | - | - | - | - | finished_at=54.9ms edit_send=27ms edit_char='x' |

## Product trace timings (sub-millisecond source)

| run | Queued (ms) | Cancelling (ms) |
| --- | --- | --- |
| 1 | 7.665 | - |

Queued duration: min 7.665 ms, max 7.665 ms across 1 run(s).

Trace: docs\goal-desktop-v4\round-14\stale\run-trace.log

## Logged status transitions

| phase | wall clock | elapsed ms | status |
| --- | --- | --- | --- |
| pre-run | 12:11:32.803 |  | untitled.lv  \|  rev 6  \|  precision 1000  \|  Ln 1, Col 6  \|  run Idle |
| ctrl+enter | 12:11:32.879 | 54.9 | untitled.lv  \|  rev 6  \|  precision 1000  \|  Ln 1, Col 6  \|  run Finished |
| stale-edit | 12:11:33.546 | 27.6 | untitled.lv  \|  rev 7  \|  precision 1000  \|  Ln 1, Col 7  \|  run Stale |

## Captured frames

| state | frame | grab ms | bytes | file |
| --- | --- | --- | --- | --- |
| run Stale | 1 | 67.3 | 135431 | docs\goal-desktop-v4\round-14\stale\run-stale.png |

## Artifacts

| file | size / node count |
| --- | --- |
| scenario-final.png | 135431 bytes |
| docs\goal-desktop-v4\round-14\stale\run-stale.png | 135431 bytes |
| docs\goal-desktop-v4\round-14\stale\run-stale-uia.txt | 47 UIA nodes |
| scenario-final-uia.txt | 47 UIA nodes |
| scenario-summary.md | this file |

