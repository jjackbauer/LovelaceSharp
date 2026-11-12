# EDT-006: failed-run diagnostics on the offending source

Requirement: a failed run's diagnostic must be drawn on the offending source in the
desktop editor, not only written to the logs pane. This round adds the editor decoration
(underline + gutter marker), the hover tooltip, click/keyboard navigation, and the
1-based UTF-16 mapping with unit tests. It also feeds EDT-014: the desktop now treats an
engine position as a UTF-16 code-unit offset instead of a byte offset.

## What is drawn

- **Underline.** From the diagnostic's reported column to end-of-line, in
  `DIAGNOSTIC_UNDERLINE` (`#ff7b72`), at the bottom of the character cells. The span is
  computed in `char`s (`diagnostic_map::range_for`) and painted on the editor's monospace
  grid; a column at end-of-line still gets one cell of width so the failure is visible.
- **Gutter marker.** A solid red bar (`DIAGNOSTIC_MARKER`, `#e04b45`) at the left edge of
  the gutter on the reported 1-based line, clear of the right-aligned line number.
- **Hover tooltip.** Hovering the underlined line (text area or gutter marker) shows
  `Diagnostic::render()`, e.g. `line 1, column 8: Cannot divide by zero.` It is a
  pointer-anchored tooltip, so it cannot steal the editor's click or keyboard focus.
- **Click and keyboard navigation.** Clicking the gutter area on the marked line sets
  the caret to the exact reported line and column and focuses the editor. `F8` is a
  window-global shortcut that does the same from any pane; it is listed in `--help`.
- **Pane header.** The editor header also states `Diagnostic Ln <line>, Col <column>:
  <message>` (truncated to 70 chars, full text on hover), so the position and message are
  readable without hovering and appear in the UI Automation tree.

The workbench remains the single source of truth: the pane copies
`Workbench::diagnostic(index)`, and the log line `Diagnostic: <rendered>` is unchanged.

## The mapping rule

1. **Engine position.** `lovelace_lang::LangError::position` is a zero-based **UTF-16
   code-unit** offset into the statement text the engine was handed. The desktop's
   `execution::Diagnostic::for_statement` reads the position from the message, converts
   it to a byte offset with `byte_offset_utf16` (an offset inside a surrogate pair clamps
   to that code point; an offset past the end clamps to the text length), adds the
   statement's absolute byte offset, and then derives the 1-based line and 1-based
   **UTF-16 column** with `statements::line_column`. A message with no position points at
   the statement start. The rendered diagnostic is `line L, column C: message`.
2. **Editor range.** `diagnostic_map::range_for(lines, line, column)` (in
   `src/diagnostic_map.rs`) applies, in order:
   - 1-based `line` must be `1..=lines.len()`, otherwise `None` (no marker past the
     buffer);
   - a trailing `\r` from a CRLF split is removed: the carriage return is a line
     terminator, not a source column;
   - the target is `column - 1` UTF-16 code units; walking the line's chars and adding
     `char.len_utf16()` finds the char that contains that code unit (a column inside a
     surrogate pair clamps back to the astral character's start);
   - `start_char`/`start_byte` are that char's indices; `end_char`/`end_byte` are the end
     of the line, so the underline runs from the reported column to end-of-line;
   - a column past end-of-line clamps to an empty range at end-of-line (painted one cell
     wide; the gutter line is still marked).
   The returned `DiagnosticRange` carries `line` (the gutter line number), char indices
   for painting, and byte indices for assertions.

The execution core now puts the structured `Diagnostic` (message, line, column, byte
offset, statement index/line) in `DesktopEvent::Failed`; the UI never parses the log
text. `WorkbenchDocument` stores the diagnostic together with the source revision it
was produced for. `Workbench::diagnostic(index)` returns it only while that revision is
still the document's current revision, so:

- a failed run publishes the decoration;
- a successful `Finished` run clears it (`diagnostic = None`);
- any edit — including deleting the offending line — advances the revision and hides the
  decoration immediately (the editor also drops it in the same paint pass as the edit);
- `range_for` additionally refuses a line outside the buffer, so no marker can point past
  a shortened buffer.

## Unit tests

All required cases are in `src/diagnostic_map.rs`; the UTF-16 end of the pipeline has
its own tests in `src/execution.rs` and `src/statements.rs`.

| Test | What it pins |
| --- | --- |
| `diagnostic_map::tests::ascii_multiline_maps_the_reported_line_from_column_to_end_of_line` | (a) ASCII multi-line: line 3, column 1 → line 3, chars 0..5, bytes 0..5; gutter line 3 |
| `diagnostic_map::tests::astral_character_before_the_error_counts_as_two_utf16_units` | (b) `"😀x = 1 / 0"`, column 3 → `x` at char 1 / byte 4 (a scalar count would land on the space at char 2); a column inside the surrogate pair clamps to char 0 |
| `diagnostic_map::tests::crlf_input_excludes_the_carriage_return_from_the_range` | (c) `"1 / 0\r"`, line 3, column 3 → chars 2..5, bytes 2..5 (the `\r` is excluded) |
| `diagnostic_map::tests::last_line_maps_exactly_and_missing_lines_have_no_range` | (d) last line `"c = 3 / 0"`, column 5 → chars 4..9, bytes 4..9; line 4 (past the buffer), line 0, and an empty buffer → `None` |
| `diagnostic_map::tests::a_column_past_end_of_line_is_empty_at_end_of_line` | column 99 → empty range at end-of-line |
| `execution::tests::diagnostic_position_is_a_utf16_offset_not_a_byte_offset` | engine position 11 after a 4-byte emoji → byte 13, column 12; inside-pair and past-end clamps |
| `statements::tests::line_column_counts_utf16_code_units_and_crlf_once` | `"ab\n😀x\n"`: emoji start (2,1), `x` (2,3); CRLF line break once |
| `workbench::tests::a_failed_run_yields_a_diagnostic_and_returns_to_idle` | the `Failed` event publishes a structured line-2, column-1 diagnostic at byte 6 with the engine message verbatim |
| `workbench::tests::a_successful_run_clears_the_failure_decoration_and_an_edit_hides_it` | success clears; deleting the offending line hides instead of retargeting |
| `ui::app::tests::failed_run_decorates_the_reported_line_and_f8_jumps_to_it_and_an_edit_clears_it` | the editor receives line 2, chars 0..5 / bytes 0..5; F8 moves the caret to (2,1); an edit clears in the same pass |

Observed pass output (`cargo-test.log`):

```text
test diagnostic_map::tests::a_column_past_end_of_line_is_empty_at_end_of_line ... ok
test diagnostic_map::tests::ascii_multiline_maps_the_reported_line_from_column_to_end_of_line ... ok
test diagnostic_map::tests::astral_character_before_the_error_counts_as_two_utf16_units ... ok
test diagnostic_map::tests::crlf_input_excludes_the_carriage_return_from_the_range ... ok
test diagnostic_map::tests::last_line_maps_exactly_and_missing_lines_have_no_range ... ok
test execution::tests::diagnostic_position_is_a_utf16_offset_not_a_byte_offset ... ok
test statements::tests::line_column_counts_utf16_code_units_and_crlf_once ... ok
test workbench::tests::a_failed_run_yields_a_diagnostic_and_returns_to_idle ... ok
test workbench::tests::a_successful_run_clears_the_failure_decoration_and_an_edit_hides_it ... ok
test ui::app::tests::failed_run_decorates_the_reported_line_and_f8_jumps_to_it_and_an_edit_clears_it ... ok
```

## Window proof

Command (Windows PowerShell 5.1; the harness was not modified):

```text
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools\gui-harness.ps1 -Action scenario ^
  -Flow queued -CaptureStates "run Failed" -Keys "a = 1; 1 / 0" ^
  -OutDir docs\goal-desktop-v4\round-15\failed-run ^
  -Log docs\goal-desktop-v4\round-15\failed-run\driver.log ^
  -Exe rust\target-desktop\debug\lovelace-desktop.exe ^
  -FastStatus -FastKeys -WakeRepaint -MaxAttempts 3
```

Driver result: `SCENARIO_OK FLOW=queued ATTEMPTS=1 CAPTURED=1 REQUESTED=1`; the status
sample matched `run Failed` **31.6 ms** after Ctrl+Enter, the pixel grab completed at
72.2 ms, and the state capture recorded 50 UIA nodes.

**Artifact:** `docs/goal-desktop-v4/round-15/run-failed.png` — **147,728 bytes**,
1296x839 PNG, SHA-256 `068E2CA49378DFEB8AE3B2958E38027C782BCA5088527DF41C32D4FCFAC3AE98`.
The raw harness bundle is in `docs/goal-desktop-v4/round-15/failed-run/`
(`run-failed.png`, `run-failed-uia.txt`, `driver.log`, `scenario-summary.md`,
`scenario-final.png`, `scenario-final-uia.txt`).

The PNG is a real screen capture of the shipped window. It shows the typed script
`a = 1; 1 / 0` on line 1, the red underline from column 8 (`1 / 0`) to end-of-line, the
red gutter bar on line 1, and the hover tooltip `line 1, column 8: Cannot divide by
zero.` (the capture happened while the pointer hovered the underlined line). The
editor header reads `Diagnostic Ln 1, Col 8: Cannot divide by zero.`; the logs pane shows
both error lines; the status strip ends in `run Failed`.

Status and log excerpt (also in `docs/goal-desktop-v4/round-15/status-and-log.txt`,
extracted from `run-failed-uia.txt`):

```text
untitled.lv  |  rev 13  |  precision 1000  |  Ln 1, Col 13  |  run Failed
Script editor, 2 lines, caret line 1 column 13
Diagnostic Ln 1, Col 8: Cannot divide by zero.
[info] Run queued (revision 13, run 1).
[info] Run started: 2 statements, engine lovelace-lang, precision 1000.
[error] Diagnostic: line 1, column 8: Cannot divide by zero.
[error] Run failed after 1 committed statements in 8 ms; the committed prefix remains in the workspace.
```

## Full test summary

Command:

```text
$env:RUSTUP_HOME='C:\Users\ricar\dev\LovelaceSharp\.toolchain\rustup'
$env:CARGO_HOME='C:\Users\ricar\dev\LovelaceSharp\.toolchain\cargo'
$env:TMP='C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop\tmp'; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR='C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop'
cargo test -p lovelace-desktop --manifest-path rust\Cargo.toml
```

Observed tail (full output in `docs/goal-desktop-v4/round-15/cargo-test.log`, 22,210
bytes):

```text
test result: ok. 130 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.60s
```

Build used for the capture: `cargo build -p lovelace-desktop` (dev profile),
`rust/target-desktop/debug/lovelace-desktop.exe`, 64,398,336 bytes.
