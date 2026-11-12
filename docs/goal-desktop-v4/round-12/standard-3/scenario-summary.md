# Round 10 GUI scenario evidence

Generated: 2026-11-12 11:28:18.658 -03:00
Driver: tools/gui-harness.ps1 -Action scenario
Product: C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop\debug\lovelace-desktop.exe
PID: 24012 (closed at the end of the gesture)
Driver log: C:\Users\ricar\dev\LovelaceSharp\docs\goal-desktop-v4\round-12\standard-3\driver.log

## Window and UIA readiness

Window: hwnd=36441862 rect=1296x839 at 52,52 after 1625 ms (restored_from_minimized=False).
UIA: 43 descendants from FindAll(Descendants) after 104 ms; the root was re-acquired from hwnd after the window was shown.
Typed with SendKeys (foreground asserted before every send): `matmul(eye(400), eye(400))` + Enter

## Logged state transitions

| phase | wall clock | elapsed ms | status |
| --- | --- | --- | --- |
| pre-run | 11:28:16.360 |  | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Idle |
| ctrl+enter | 11:28:16.494 | 128.7 | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Running |
| escape | 11:28:17.301 | 59.3 | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Cancelled |

Sampler: standard (full UIA tree walk, 150 ms sleep)
Measured Ctrl+Enter -> run Running: **128.7 ms** (11:28:16.364 -> 11:28:16.498); the sample that observed it took 58.6 ms
Measured Esc -> run Cancelled: **59.3 ms** (11:28:17.241 -> 11:28:17.305); the sample that observed it took 24.1 ms

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
| scenario-running.png | 132374 bytes (> 20 KB) |
| scenario-final.png | 139408 bytes (> 20 KB) |
| scenario-running-uia.txt | 44 UIA nodes |
| scenario-final-uia.txt | 44 UIA nodes |
| scenario-summary.md | this file |

Initial UIA tree after the window was shown: 43 nodes.

