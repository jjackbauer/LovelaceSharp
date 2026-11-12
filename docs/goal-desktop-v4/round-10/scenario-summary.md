# Round 10 GUI scenario evidence

Generated: 2026-11-12 10:58:35.543 -03:00
Driver: tools/gui-harness.ps1 -Action scenario
Product: C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop\debug\lovelace-desktop.exe
PID: 30792 (closed at the end of the gesture)
Driver log: C:\Users\ricar\dev\LovelaceSharp\docs\goal-desktop-v4\round-10\scenario-driver.log

## Window and UIA readiness

Window: hwnd=7671472 rect=1296x839 at 260,260 after 1274 ms (restored_from_minimized=False).
UIA: 43 descendants from FindAll(Descendants) after 75 ms; the root was re-acquired from hwnd after the window was shown.
Typed with SendKeys (foreground asserted before every send): `matmul(eye(400), eye(400))` + Enter

## Logged state transitions

| phase | wall clock | elapsed ms | status |
| --- | --- | --- | --- |
| pre-run | 10:58:33.310 |  | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Idle |
| ctrl+enter | 10:58:33.438 | 120 | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Running |
| escape | 10:58:34.239 | 72.5 | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Cancelled |

Measured Ctrl+Enter -> run Running: **120 ms** (10:58:33.313 -> 10:58:33.441)
Measured Esc -> run Cancelled: **72.5 ms** (10:58:34.166 -> 10:58:34.240)

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
| scenario-running.png | 130850 bytes (> 20 KB) |
| scenario-final.png | 137937 bytes (> 20 KB) |
| scenario-running-uia.txt | 44 UIA nodes |
| scenario-final-uia.txt | 44 UIA nodes |
| scenario-summary.md | this file |

Initial UIA tree after the window was shown: 43 nodes.

