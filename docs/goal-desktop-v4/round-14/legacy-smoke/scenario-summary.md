# Round 10 GUI scenario evidence

Generated: 2026-11-12 12:18:22.266 -03:00
Driver: tools/gui-harness.ps1 -Action scenario
Product: rust\target-desktop\debug\lovelace-desktop.exe
PID: 34300 (closed at the end of the gesture)
Driver log: docs\goal-desktop-v4\round-14\legacy-smoke\driver.log

## Window and UIA readiness

Window: hwnd=3936302 rect=1296x839 at 130,130 after 1110 ms (restored_from_minimized=False).
UIA: 43 descendants from FindAll(Descendants) after 69 ms; the root was re-acquired from hwnd after the window was shown.
Typed with SendKeys (foreground asserted before every send): `matmul(eye(400), eye(400))` + Enter

## Logged state transitions

| phase | wall clock | elapsed ms | status |
| --- | --- | --- | --- |
| pre-run | 12:18:20.091 |  | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Idle |
| ctrl+enter | 12:18:20.221 | 125.4 | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Running |
| escape | 12:18:20.975 | 68.3 | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Cancelled |

Sampler: standard (full UIA tree walk, 150 ms sleep)
Measured Ctrl+Enter -> run Running: **125.4 ms** (12:18:20.094 -> 12:18:20.222); the SendKeys call returned at 51.2 ms and the sample that observed the state took 68.9 ms
Measured Esc -> run Cancelled: **68.3 ms** (12:18:20.907 -> 12:18:20.976); the SendKeys call returned at 33.1 ms and the sample that observed the state took 33.9 ms

## Caret accessible names

- before the run: `Script editor, 2 lines, caret line 1 column 27`
- after the run: `Script editor, 2 lines, caret line 1 column 27`
- after the refocus click: `Script editor, 2 lines, caret line 1 column 27`
- after Home: `Script editor, 2 lines, caret line 1 column 1`
- after Shift+End: `Script editor, 2 lines, caret line 1 column 27`

Note: the run leaves egui without keyboard focus in this build, so the driver clicks the editor once (the logged refocus click) before pressing Home and Shift+End. The names above show the restored focus still tracks the caret: column 27 -> column 1 -> column 27.

## Artifacts

| file | size / node count |
| --- | --- |
| scenario-running.png | 134065 bytes (> 20 KB) |
| scenario-final.png | 140985 bytes (> 20 KB) |
| scenario-running-uia.txt | 44 UIA nodes |
| scenario-final-uia.txt | 44 UIA nodes |
| scenario-summary.md | this file |

Initial UIA tree after the window was shown: 43 nodes.

