# RUN-002 / LOG-003: a quick-eval one-liner in the Logs pane

Requirement: *Run-002 — Logs input executes a one-liner in active document engine, appends
result, and does not change editor source. LOG-003 — Enter executes active session command and
clears input only after accepting request; previous commands are navigable with keyboard.*

This round adds one single-line quick-eval field to the shipped egui workbench, routes it through
the same `ExecutionHost::submit` entry point as the editor's Run, and emits LOG-002's `Echo` kind
from a real submission for the first time. The editor buffer and its revision are never read into
or written by the quick-eval path.

## Behaviour

### The field

The field lives in the Logs pane, immediately above the log list:

```
Quick eval [ quick eval — one-liner, press Enter ] Ctrl+Shift+E Enter runs · Esc leaves the field
[echo] >> 12 - 5
[info] Quick eval queued (revision 1, run 1).
[info] Run started: 1 statements, engine lovelace-lang, precision 1000.
[progress] statement 1/1: 12 - 5
[result] = 7
[timing] Run finished: 1 statements committed in 8 ms.
```

| Contract | Implementation |
| --- | --- |
| Visible label | `Quick eval` (`RichText::strong`) |
| Placeholder | `quick eval — one-liner, press Enter` (`TextEdit::hint_text`) |
| Mouse focus | an ordinary egui `TextEdit`; unit test clicks its center and asserts focus |
| Keyboard focus | `Ctrl+Shift+E`, window-global; dismisses an open completion popup and requests focus on the field |
| Enter | submits the trimmed expression through the workbench |
| Esc | surrenders the field's focus without submitting and without clearing its text |
| Up / Down | restore accepted expressions, newest first; Up stops at the oldest; Down returns toward the newest and then restores the draft that was in the field before navigation began |
| Clear | only after the workbench accepted the request (`QuickEval::accept`); a refused submission keeps the exact typed text |
| Refusal | `[info] Quick eval not submitted: a run is already in progress; the expression was kept.` (or the host-busy form); no `[echo]` is written because no run was submitted |

While the field owns focus, a bare Enter is not interpreted as the global Run shortcut even if the
frame's aggregate modifier state still carries a stale Ctrl bit; the key event's own modifier mask
is used. Ctrl+Enter from the field still means Run. Esc is consumed by the field while it owns
focus, so one Esc leaves the field instead of cancelling a run; with focus elsewhere Esc keeps its
existing Cancel meaning.

### Submission path

`Workbench::quick_eval_active(expression)` and `Workbench::run_active()` both call one private
`begin_run(index, source)`, which performs the single `ExecutionHost::submit(document_id,
source_revision, source, precision)` call and records the accepted run on the document. Quick eval
passes:

- the **active document's** `DocumentId`;
- the active document's **current source revision** (unchanged, not advanced);
- the active document's **current precision** (the run's `Run started:` line proves the value);
- the trimmed expression as the submitted **source**, never the editor buffer.

The document's `source` and `revision` are not assigned anywhere in the quick-eval path. The
window's `sync()` only reloads an editor when the document revision differs from the last painted
revision, so the editor is not touched either.

### What the run publishes

`[echo] >> <expression>` is appended only after `ExecutionHost::submit` returned `Ok`, as
`LogKind::Echo`; `LogLine::render()` produces the exact `[echo] >> …` text the pane paints. The
submission then writes `[info] Quick eval queued (revision N, run M).` and the normal typed run
events append `[info] Run started`, `[progress]`, `[stdout]`, `[result]`, `[diagnostic]` and
`[timing]` exactly as they do for Run.

The quick-eval run is deliberately log-and-status only for the editor-derived panes. The execution
core constructs one fresh engine per run (`RunWorker::execute`); there is no persistent
per-document engine to attach a one-liner to. Submitting only the expression through the same host
path therefore cannot produce a snapshot that describes the document's workspace. The workbench
marks the run (`quick_eval_run`) and withholds its terminal snapshot from the document's
workspace/plot panes and its diagnostic from the editor, because both would name the expression's
text/state as if it were the buffer's. A failed quick eval still logs `[diagnostic]`; it just does
not draw an editor underline. This is also why the window capture's Workspace pane still says
"No run has published a workspace yet." while the quick-eval result is in the log.

No source is evaluated on the UI thread and no busy loop is added: acceptance enters `Queued`
synchronously, the existing per-frame `poll()` applies the typed events, and the status strip moves
to `Finished` when the `Finished` event arrives.

## Code changes

- **`rust/lovelace-desktop/src/quick_eval.rs`** (new)
  - `QuickEval`: text buffer, submitted-expression history (`HISTORY_CAP = 100`), `expression()`
    trimming, `accept()` clearing/recording, `navigate_up()`/`navigate_down()` with draft restore.
  - Four model tests (below).
- **`rust/lovelace-desktop/src/workbench.rs`**
  - `Workbench::quick_eval_active`; private `begin_run` shared with `run_active`.
  - `WorkbenchDocument::quick_eval_run` marks a run submitted from the field.
  - Terminal-event handling uses that marker to skip `publish_snapshot` and the editor diagnostic
    for quick-eval runs; `[echo]`, `[result]`, `[stdout]`, `[diagnostic]` and `[timing]` are still
    emitted from the same events as any run.
  - `set_source` and the failed-run diagnostic behaviour are unchanged; a quick-eval failure cannot
    retarget the editor's decoration.
  - Three workbench tests (below).
- **`rust/lovelace-desktop/src/ui/app.rs`**
  - `quick_eval: QuickEval` and `quick_eval_rect` on `WorkbenchApp`; `quick_eval_id()`.
  - `Ctrl+Shift+E` focus request in `ui_root`, with quick-field Esc suppression of the global
    Cancel command.
  - `quick_eval_row` (label, placeholder, shortcut hint, Enter/Esc/Up/Down, focus lock for
    vertical arrows/Esc) and `submit_quick_eval` (clears only on acceptance).
  - Five UI tests (below), including mouse focus and the refused-text case.
- **`rust/lovelace-desktop/src/main.rs`** — registers `mod quick_eval`.
- **`docs/goal-desktop-v4/round-23/capture-quick-eval.ps1`** — the window proof driver.

No engine crate, `tools/`, or gate file was modified in this round (the unmodified
`tools/gui-harness.ps1` was invoked read-only for the UIA dump, as in earlier rounds).

## Unit tests and observed output

Run in the prescribed workspace PowerShell 5.1 environment:

```
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:TMP="$repo\rust\target-desktop\tmp"; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR="$repo\rust\target-desktop"
& "$repo\.toolchain\cargo\bin\cargo.exe" test -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"
```

Observed (full log in [`cargo-test.log`](cargo-test.log)):

```
test result: ok. 210 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.13s
```

New tests (13; baseline was 197):

| Test | What it pins |
| --- | --- |
| `quick_eval::tests::history_navigation_returns_previous_expressions_newest_first_and_stops_at_the_oldest` | two accepted expressions; Up returns newest then older; Up at the oldest reports no change and keeps the oldest text |
| `quick_eval::tests::an_accepted_expression_is_recorded_trimmed_once_and_clears_the_field` | trimming, exactly one history entry, and the field clears only in `accept` |
| `quick_eval::tests::history_navigation_down_restores_the_newest_then_the_draft` | Down returns toward newest and then restores the pre-navigation draft |
| `quick_eval::tests::blank_text_is_not_an_expression_and_is_never_recorded` | whitespace is not a submission and is not cleared/recorded |
| `workbench::tests::quick_eval_runs_against_the_active_document_without_touching_its_buffer_or_revision` | second tab active; expression `6 * 7` → `Finished`, `= 42`, precision 250 in the run line; both docs' source/revision unchanged; exactly one `[echo] >> 6 * 7`; document snapshot not replaced; inactive document gets no echo |
| `workbench::tests::a_failing_quick_eval_logs_its_diagnostic_without_decorating_the_document` | `1 / 0` → `Failed`, `[diagnostic]` logged, no document snapshot and no editor diagnostic because the position names the expression, not the buffer |
| `workbench::tests::each_quick_eval_submission_emits_exactly_one_echo_line_for_its_own_text` | `6 * 7` then `2 + 3` produce exactly `[">> 6 * 7", ">> 2 + 3"]` echoes and results `["= 42", "= 5"]` |
| `workbench::tests::a_quick_eval_refused_while_busy_keeps_the_expression_and_logs_an_info_advisory` | a run in flight; quick eval returns false, leaves the run in `Queued`, writes no echo, and logs the `[info] … kept` advisory |
| `ui::app::tests::quick_eval_shortcut_focuses_the_field_and_enter_runs_without_editing_the_buffer` | Ctrl+Shift+E focus; typed text does not reach the editor; Enter clears the field, starts the run, leaves source/revision exact, logs one `[echo] >> 6 * 7` and `[result] = 42` |
| `ui::app::tests::quick_eval_history_returns_previous_expressions_newest_first_and_stops_at_the_oldest` | real submits through the field, then Up/Up/Up/Down restore `2 + 3`, `6 * 7`, `6 * 7`, `2 + 3`; editor unchanged |
| `ui::app::tests::the_quick_eval_field_takes_keyboard_focus_from_a_mouse_click` | a pointer click on the painted field gives it keyboard focus |
| `ui::app::tests::escape_leaves_the_quick_eval_field_without_submitting_or_clearing_it` | Esc leaves focus, keeps `6 * 7`, leaves run state `Idle`, writes no echo, and the editor is unchanged |
| `ui::app::tests::a_refused_quick_eval_keeps_the_typed_text_and_logs_info_without_an_echo` | gated in-flight run; Enter in the field keeps `6 * 7`, writes no echo, logs the `[info]` advisory; run is released and completes |

Observed test lines from the suite log:

```
test quick_eval::tests::an_accepted_expression_is_recorded_trimmed_once_and_clears_the_field ... ok
test quick_eval::tests::blank_text_is_not_an_expression_and_is_never_recorded ... ok
test quick_eval::tests::history_navigation_down_restores_the_newest_then_the_draft ... ok
test quick_eval::tests::history_navigation_returns_previous_expressions_newest_first_and_stops_at_the_oldest ... ok
test workbench::tests::a_quick_eval_refused_while_busy_keeps_the_expression_and_logs_an_info_advisory ... ok
test workbench::tests::a_failing_quick_eval_logs_its_diagnostic_without_decorating_the_document ... ok
test workbench::tests::each_quick_eval_submission_emits_exactly_one_echo_line_for_its_own_text ... ok
test workbench::tests::quick_eval_runs_against_the_active_document_without_touching_its_buffer_or_revision ... ok
test ui::app::tests::the_quick_eval_field_takes_keyboard_focus_from_a_mouse_click ... ok
test ui::app::tests::escape_leaves_the_quick_eval_field_without_submitting_or_clearing_it ... ok
test ui::app::tests::quick_eval_shortcut_focuses_the_field_and_enter_runs_without_editing_the_buffer ... ok
test ui::app::tests::quick_eval_history_returns_previous_expressions_newest_first_and_stops_at_the_oldest ... ok
test ui::app::tests::a_refused_quick_eval_keeps_the_typed_text_and_logs_info_without_an_echo ... ok
```

No existing test was weakened or removed: the suite went from 197 to 210 with the same baseline
tests still passing.

## Window capture

| File | Size | Dimensions | What |
| --- | --- | --- | --- |
| [`quick-eval.png`](quick-eval.png) | 1,146,641 bytes | 3888x4500 | full product window, `PrintWindow(PW_RENDERFULLCONTENT)` |
| [`quick-eval-logs.png`](quick-eval-logs.png) | 170,116 bytes | 2280x828 | native crop: Logs pane header + quick-eval row + all 9 log lines + status strip |
| [`quick-eval-uia.txt`](quick-eval-uia.txt) | 4,018 bytes | — | live UIA tree with the field and all 9 log lines in order |
| [`work/capture-log.txt`](work/capture-log.txt) | 3,244 bytes | — | driver log: foreground check, launch, typing value, focus, status, capture, palette |

### Method (posted input, no SendKeys)

The script uses the round-20/22 no-foreground method:

1. Launched `lovelace-desktop.exe <empty .lv>` with `__COMPAT_LAYER=DPIUNAWARE`, `APPDATA` under
   this round's directory and `TMP` under `rust/target-desktop/tmp`; no plot statement was run.
2. `SetWindowPos` to the full 3888x4500 surface at 96 dpi, `ShowWindow(SW_RESTORE)`,
   `WM_SETFOCUS`, `RedrawWindow`; the capture retries until the full-width `#222830` Logs header
   band is present, so a stale small surface cannot be delivered.
3. Found the field in the live accessibility tree as `Edit | Quick eval` (label from
   `labelled_by`) and gave it focus through UI Automation **`SetFocus`**; it reported
   `IsKeyboardFocusable=True`.
4. Posted real key messages to the window: for every character a `WM_KEYDOWN` with the character's
   VK, then `WM_CHAR`, then `WM_KEYUP`; for Enter `WM_KEYDOWN`/`WM_KEYUP` with `VK_RETURN`.
   Messages are posted while the UI thread is briefly suspended so the order is exact. The
   field's `ValuePattern` read back `12 - 5` before Enter, proving the text really landed.
   Note: a `VK_PROCESSKEY` keydown (the sequence the round-22 editor accepted) did not deliver the
   posted `WM_CHAR` into the egui `TextEdit`; the real character VK did. The established
   posted-input path is otherwise unchanged.
5. Waited for the status strip to read `run Finished` (the run is asynchronous), then read the
   field's `ValuePattern` again: `''`. The editor element still read
   `Script editor, 1 lines, caret line 1 column 1`; the status still read `rev 1`. The buffer's
   text and revision were therefore unchanged in the live window as well as in the unit tests.
6. Dumped the live accessibility tree with the unmodified `tools/gui-harness.ps1 -Action uia`
   (read-only; no foreground needed) and copied it here.
7. Captured `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT)`. The script probes the captured log
   area for the exact painted marker colours, so the image is verified to contain the lines.

### Locked-session note

The script takes the foreground only by reading `GetForegroundWindow`; it never calls
`SetForegroundWindow` and never uses `SendKeys`/`SendInput`. In the final run the foreground check
recorded the product window itself (`locked=False`), so that run was not over a lock screen. Earlier
probes in this same session recorded a different foreground window. The posted-input path is the
same in both cases and does not depend on foreground ownership.

### Observed live state

```
initial status='quick-eval.lv  |  rev 1  |  precision 1000  |  Ln 1, Col 1  |  run Idle'
initial editor='Script editor, 1 lines, caret line 1 column 1'  quick_eval=''
quick-eval focus set name='Quick eval' focusable=True
typed quick_eval='12 - 5'
after Enter status='quick-eval.lv  |  rev 1  |  precision 1000  |  Ln 1, Col 1  |  run Finished'
after Enter editor='Script editor, 1 lines, caret line 1 column 1'  quick_eval=''
```

The UIA log, in chronological order (all nine lines):

```
[info] Lovelace Desktop ready. Engine: lovelace-lang. Two documents open.
[info] Plot directory: C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop\tmp\lovelace-desktop\plots.
[info] Opened C:\Users\ricar\dev\LovelaceSharp\docs\goal-desktop-v4\round-23\work\quick-eval.lv.
[echo] >> 12 - 5
[info] Quick eval queued (revision 1, run 1).
[info] Run started: 1 statements, engine lovelace-lang, precision 1000.
[progress] statement 1/1: 12 - 5
[result] = 7
[timing] Run finished: 1 statements committed in 8 ms.
```

The UIA tree also shows the field's visible label (`Text | Quick eval`), the `Edit | Quick eval`
node, `Ctrl+Shift+E`, and `Enter runs · Esc leaves the field`. The status strip in the capture reads
`quick-eval.lv | rev 1 | precision 1000 | Ln 1, Col 1 | run Finished` and `8 ms last run`. The
editor pane still shows `1 lines / 0 bytes`, and the Workspace pane still says
"No run has published a workspace yet.", which is the documented log-only quick-eval decision.

Palette probe over the log crop (exact RGB pixels, from `work/capture-log.txt`):

```
palette echo       #C8B6FF pixels=41
palette result     #4EC9B0 pixels=20
palette info       #8B98A9 pixels=1101
palette timing     #D7BA7D pixels=116
palette progress   #7FB0D8 pixels=72
```

The lavender `#C8B6FF` is `LogKind::Echo`; its 41 pixels are the first window evidence that echo is
now produced by a product path rather than only rendered in a unit test.

## Suite summary

`cargo test -p lovelace-desktop`: **210 passed, 0 failed, 0 ignored** in 1.13 s (exit 0), including
the 13 new RUN-002/LOG-003 tests and the unchanged 197-test baseline. `--gui-selftest` was not used
as evidence for this round because the window capture above exercises the shipped window and the
same real engine through the same `Workbench`; no plot statement was executed.
