# Round 10 GUI scenario evidence

Generated: 2026-11-12 11:34:18.744 -03:00
Driver: tools/gui-harness.ps1 -Action scenario
Product: C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop\debug\lovelace-desktop.exe
PID: 10132 (closed at the end of the gesture)
Driver log: C:\Users\ricar\dev\LovelaceSharp\docs\goal-desktop-v4\round-12\after-fine-3\driver.log

## Window and UIA readiness

Window: hwnd=26808146 rect=1296x839 at 234,234 after 1104 ms (restored_from_minimized=False).
UIA: 43 descendants from FindAll(Descendants) after 64 ms; the root was re-acquired from hwnd after the window was shown.
Typed with SendKeys (foreground asserted before every send): `matmul(eye(400), eye(400))` + Enter

## Logged state transitions

| phase | wall clock | elapsed ms | status |
| --- | --- | --- | --- |
| pre-run | 11:34:16.645 |  | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Idle |
| ctrl+enter | 11:34:16.718 | 68.4 | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Running |
| escape | 11:34:17.462 | 35.6 | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Cancelled |

Sampler: fine (one cached status element, 1 ms sleep)
Measured Ctrl+Enter -> run Running: **68.4 ms** (11:34:16.649 -> 11:34:16.720); the SendKeys call returned at 62.5 ms and the sample that observed the state took 1 ms
Measured Esc -> run Cancelled: **35.6 ms** (11:34:17.426 -> 11:34:17.463); the SendKeys call returned at 33.9 ms and the sample that observed the state took 0.3 ms

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
| scenario-running.png | 132990 bytes (> 20 KB) |
| scenario-final.png | 139864 bytes (> 20 KB) |
| scenario-running-uia.txt | 44 UIA nodes |
| scenario-final-uia.txt | 44 UIA nodes |
| scenario-summary.md | this file |

Initial UIA tree after the window was shown: 43 nodes.

