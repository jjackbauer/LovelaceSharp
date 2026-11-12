# Round 10 GUI scenario evidence

Generated: 2026-11-12 11:34:02.964 -03:00
Driver: tools/gui-harness.ps1 -Action scenario
Product: C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop\debug\lovelace-desktop.exe
PID: 27752 (closed at the end of the gesture)
Driver log: C:\Users\ricar\dev\LovelaceSharp\docs\goal-desktop-v4\round-12\after-fine-1\driver.log

## Window and UIA readiness

Window: hwnd=3215328 rect=1296x839 at 26,26 after 1221 ms (restored_from_minimized=False).
UIA: 43 descendants from FindAll(Descendants) after 69 ms; the root was re-acquired from hwnd after the window was shown.
Typed with SendKeys (foreground asserted before every send): `matmul(eye(400), eye(400))` + Enter

## Logged state transitions

| phase | wall clock | elapsed ms | status |
| --- | --- | --- | --- |
| pre-run | 11:34:00.919 |  | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Idle |
| ctrl+enter | 11:34:00.980 | 57.2 | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Running |
| escape | 11:34:01.692 | 34.8 | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Cancelling |
| escape | 11:34:01.705 | 47.6 | untitled.lv  \|  rev 27  \|  precision 1000  \|  Ln 1, Col 27  \|  run Cancelled |

Sampler: fine (one cached status element, 1 ms sleep)
Measured Ctrl+Enter -> run Running: **57.2 ms** (11:34:00.922 -> 11:34:00.981); the SendKeys call returned at 50.9 ms and the sample that observed the state took 1.2 ms
Measured Esc -> run Cancelled: **47.6 ms** (11:34:01.657 -> 11:34:01.706); the SendKeys call returned at 33.1 ms and the sample that observed the state took 0.9 ms

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
| scenario-running.png | 138108 bytes (> 20 KB) |
| scenario-final.png | 145038 bytes (> 20 KB) |
| scenario-running-uia.txt | 44 UIA nodes |
| scenario-final-uia.txt | 44 UIA nodes |
| scenario-summary.md | this file |

Initial UIA tree after the window was shown: 43 nodes.

