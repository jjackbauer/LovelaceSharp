# EDT-014: Unicode offsets for diagnostics, selections and source ranges

Requirement: *Diagnostic offsets, selections and source ranges correctly convert UTF-16 <->
Rust byte/grapheme positions on emoji, combining marks and newlines.*

Every offset below is pinned by a unit test that compares the full value, not a partial count.
`column` is 1-based UTF-16 code units (the engine's unit), `start_char`/`end_char` are 0-based
`char` indices (the editor's cell unit), and `start_byte`/`end_byte` are UTF-8 byte offsets in
the line text with the trailing CR of a CRLF split removed.

## The rule

1. **Diagnostics** arrive as a 1-based line and 1-based UTF-16 column
   (`execution::Diagnostic::for_statement` + `statements::line_column`). `range_for` walks the
   line with `char_indices`, accumulating `char::len_utf16`, so the start char is the character
   that owns the reported code unit and the start byte is that character's UTF-8 offset. A
   column inside a surrogate pair clamps back to the astral character's start; a column at or
   past end-of-line is an empty range at end-of-line. A trailing `\r` is a line terminator and
   is stripped before any of this.
2. **Selections** are stored by `TextEditor` as `((row, char_col), (row, char_col))`, because
   the editor paints one `char` per cell. `TextEditor::selection_bytes` converts both endpoints
   with `diagnostic_map::source_byte_offset`, which walks the buffer lines (joined with `\n`) and
   accumulates `char::len_utf8`. The result is a code-point-exact byte range into the same text
   `TextEditor::source` returns.
3. **Caret readout** is converted at the display boundary: `TextEditor::caret_column_utf16`
   counts `char::len_utf16` up to the caret, and `TextEditor::line_col_1based` adds the 1-based
   offset. An astral character is therefore two columns wide in the status strip and the
   accessibility label, exactly as the engine and `range_for` count it.

## Tests and exact offsets

| Test | Input | Expected | Observed |
| --- | --- | --- | --- |
| `diagnostic_map::tests::combining_mark_before_the_error_keeps_the_underlined_token_byte_exact` | `e` + U+0301 + ` = 1 / 0`, line 1, column 6; and the live-engine script `s = "e\u{0301}"; 1 / 0` | line 1, chars 5..10, bytes 6..11; live engine: (line 1, column 11, byte 11), chars 10..15, bytes 11..16 | pass |
| `ui::editor::tests::astral_selection_converts_to_exact_bytes_and_the_caret_utf16_column` | `a😀b`, selection `((0,0),(0,3))`, caret 3 | bytes `(0,6)`; `source[0..6] == "a😀b"`; caret UTF-16 4 (0-based) / column 5 (1-based) | failed before fix, pass after |
| `ui::editor::tests::a_combining_mark_selection_covers_the_whole_clusters_bytes` | `e` + U+0301 + ` = 1`, selection `((0,0),(0,2))` | bytes `(0,3)`; `source[0..3] == "e\u{0301}"`; caret UTF-16 2 | pass |
| `diagnostic_map::tests::lf_and_crlf_report_the_same_line_and_column_range_and_exclude_the_cr` | lines `a = 1`, `b = 2`, `1 / 0` with LF and with CRLF; line 3 columns 3 and 6 | column 3: chars 2..5, bytes 2..5; column 6: chars 5..5, bytes 5..5; both identical for LF and CRLF | pass |
| `diagnostic_map::tests::emoji_combining_mark_and_crlf_before_the_error_give_the_exact_tuple` | `s = "😀"\r\nt = "e\u{0301}"\r\n1 / 0\r\n` | line 3, column 1, source byte 23, line chars 0..5, line bytes 0..5 | pass |

### 1. Combining mark before the error column

Input line (10 chars, 11 bytes):

```text
e\u{0301} = 1 / 0
```

`e` is char 0 / byte 0; U+0301 is char 1 / bytes 1..3; the cluster `e`+U+0301 is chars 0..2 and
bytes 0..3. Column 6 is the `1` of `1 / 0`.

```text
DiagnosticRange { line: 1, start_char: 5, end_char: 10, start_byte: 6, end_byte: 11 }
line[6..11]   = "1 / 0"
line[..6]     = "e\u{0301} = "
chars().nth(5) = '1'
```

A mapping that treated the column as a byte index would start at byte 5 (the space), and one
that summed one byte per char would also start at byte 5. The assertion is exact: byte 6, after
the whole grapheme, so the underline covers the intended token and not half a grapheme.

The same test then runs an engine-reported column on one line: `s = "e\u{0301}"; 1 / 0`. The
engine fails at the second statement with `Cannot divide by zero.`; `Diagnostic::for_statement`
resolves (line 1, column 11, byte 11), and the mapped range is chars 10..15 / bytes 11..16,
`source[11..16] == "1 / 0"`. A byte-column mapping would have started at byte 10 (the space).

### 2. Astral character inside a selection

Buffer `a😀b`: 3 `char`s, 6 UTF-8 bytes (`a` 1 + U+1F600 4 + `b` 1), 4 UTF-16 code units.

```text
selection_range()      = Some(((0, 0), (0, 3)))   // editor char columns
selection_bytes()      = (0, 6)                   // source[0..6] == "a😀b"
selected_text()        = Some("a😀b")
caret_column_utf16()   = 4                        // a=1, emoji=2, b=1
line_col_1based()      = (1, 5)                   // 1-based UTF-16 column
```

`selection_bytes` is a new conversion (the editor previously exposed only char positions and
`selected_text`); it passes on the first run. The caret readout is covered by the defect below.

The combining-mark selection (2b) selects the full two-code-point cluster: chars 0..2 map to
bytes 0..3, and the slice is exactly `e` + U+0301. Selection conversion walks characters, so no
byte offset can land inside a UTF-8 code point.

### 3. LF versus CRLF

LF input `a = 1\nb = 2\n1 / 0\n` and CRLF input `a = 1\r\nb = 2\r\n1 / 0\r\n` produce the same
ranges:

```text
line 3, column 3 -> DiagnosticRange { line: 3, start_char: 2, end_char: 5, start_byte: 2, end_byte: 5 }
line 3, column 6 -> DiagnosticRange { line: 3, start_char: 5, end_char: 5, start_byte: 5, end_byte: 5 }
```

The CRLF editor line is stored as `"1 / 0\r"`, but the range's line text stops at byte 5
(`crlf[2][..5] == "1 / 0"`): the CR is not a source column. Column 6 is one past the five-char
line; if the CR were counted, the empty range would start on the CR (char/byte 5..6) instead of
at end-of-line.

### 4. Mixed: emoji, combining mark and CRLF before the error

Input source (CRLF throughout; line 1 is 10 bytes, line 2 is 9 bytes, 2-byte terminators):

```text
s = "😀"
t = "e\u{0301}"
1 / 0
```

The default engine's own evaluation of the script returns `Cannot divide by zero.` (its message
carries no position, so the desktop anchors the failure at the failing statement start, exactly
as the execution core does). `statements::slice` yields three statements; the failing third
statement (`index 2`) is `1 / 0` at byte 23. `Diagnostic::for_statement` resolves:

```text
diagnostic = (line 3, column 1, byte_offset 23)
source[23..28] = "1 / 0"
range_for(lines, 3, 1) = DiagnosticRange { line: 3, start_char: 0, end_char: 5, start_byte: 0, end_byte: 5 }
final tuple = (line 3, column 1, source byte 23, chars 0..5, bytes 0..5)
```

The test uses `engine::default_engine_factory()`: the real engine in the default build, the
deterministic fake under `--no-default-features`. It passes in both.

## Defect fixed

Before the fix, the new astral test failed on the caret readout only:

```text
thread 'ui::editor::tests::astral_selection_converts_to_exact_bytes_and_the_caret_utf16_column'
panicked at lovelace-desktop\src\ui\editor.rs:1600:9:
assertion `left == right` failed: the readout column is 1-based UTF-16, not the 3-char caret column
  left: (1, 4)
 right: (1, 5)
test result: FAILED. 191 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

`TextEditor::line_col_1based` returned `caret_col + 1`, a 1-based **char** column. After `a😀b`
the caret is char 3, so it displayed Column 4 while the engine and `range_for` call that position
UTF-16 column 5. The status strip, the run-log caret header and the editor's accessibility label
all read this method.

Fix: `line_col_1based` now returns `caret_column_utf16() + 1`; the new `caret_column_utf16`
delegates to `diagnostic_map::utf16_column`, which sums `char::len_utf16`. ASCII columns are
unchanged, so no existing test changed. The full before-fix output is in
`unicode-offsets-before-fix.log`.

**No diagnostic-mapping defect was found.** The combining-mark, LF/CRLF and mixed diagnostic
tests passed against the existing `range_for` on their first run; `range_for` already stripped
the CR and walked UTF-16 lengths over `char_indices`. The selection-to-byte conversion is new
surface, not a repair.

## Full suite summary

Command (Windows, from the round brief):

```text
$env:RUSTUP_HOME='C:\Users\ricar\dev\LovelaceSharp\.toolchain\rustup'
$env:CARGO_HOME='C:\Users\ricar\dev\LovelaceSharp\.toolchain\cargo'
$env:TMP='C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop\tmp'; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR='C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop'
cargo test -p lovelace-desktop --manifest-path rust\Cargo.toml
```

Observed (`cargo-test.log`):

```text
test result: ok. 192 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s
```

The count grew from 187 to 192: five new tests (three diagnostic-mapping, two editor selection).
The same five pass on the fake-engine build (`cargo-test-fake-engine.log`):

```text
test result: ok. 186 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.66s
```

The five new tests were run and observed passing in both logs:

```text
test diagnostic_map::tests::combining_mark_before_the_error_keeps_the_underlined_token_byte_exact ... ok
test diagnostic_map::tests::lf_and_crlf_report_the_same_line_and_column_range_and_exclude_the_cr ... ok
test diagnostic_map::tests::emoji_combining_mark_and_crlf_before_the_error_give_the_exact_tuple ... ok
test ui::editor::tests::a_combining_mark_selection_covers_the_whole_clusters_bytes ... ok
test ui::editor::tests::astral_selection_converts_to_exact_bytes_and_the_caret_utf16_column ... ok
```

## Files

- `rust/lovelace-desktop/src/diagnostic_map.rs` - `source_byte_offset`, `utf16_column`,
  `selection_byte_range`; three new tests. `range_for` unchanged.
- `rust/lovelace-desktop/src/ui/editor.rs` - `caret_column_utf16`, `selection_bytes`, the
  UTF-16 `line_col_1based` fix; two new tests.
- `docs/goal-desktop-v4/round-21/` - this document and the three test logs.

No existing test was weakened or changed. This round modified only the two desktop source files
above and this `round-21` directory; no engine crate, `tools/` file or gate file was touched.
