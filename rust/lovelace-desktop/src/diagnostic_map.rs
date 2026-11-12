//! Maps a run diagnostic's 1-based line and UTF-16 column onto an editor line.
//!
//! The engine reports a position as a **UTF-16 code-unit offset** (see `lovelace_lang::LangError`)
//! and the execution core turns it into a 1-based line and column. The editor buffer, however, is
//! a `Vec<String>` of lines whose columns are counted in `char`s, and it stores its selection in
//! those same columns. This module is the one place that translates among the three spaces
//! (UTF-16 columns, char columns, UTF-8 byte offsets), so the underline, the gutter marker, the
//! caret readout and the selection's byte range are placed by rules the unit tests below pin down
//! instead of by ad-hoc arithmetic in the paint code.
//!
//! Columns are 1-based: column 1 is the first UTF-16 code unit of the line, column 2 the second
//! code unit (the low surrogate of an astral character is column 2 of that character). A column
//! that lands inside a surrogate pair is clamped back to the start of that character, because an
//! underline cannot start in the middle of a code point. A column at or past the end of the line
//! yields an empty range at end-of-line; the painter gives such a range one cell of width so the
//! failure is still visible.

/// A diagnostic position resolved against one editor line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticRange {
    /// 1-based line, also the gutter line number to mark.
    pub line: usize,
    /// 0-based char index where the underline starts on `line`.
    pub start_char: usize,
    /// 0-based exclusive char index where the underline ends (`line`'s char count).
    pub end_char: usize,
    /// Byte offset of `start_char` within the line text (without a trailing `\r`).
    pub start_byte: usize,
    /// Byte offset of `end_char` within the line text (without a trailing `\r`).
    pub end_byte: usize,
}

/// Resolves a 1-based line and 1-based UTF-16 column against `lines`.
///
/// Returns `None` when the line is outside the buffer, so a diagnostic can never leave a marker
/// past the last line. A line that ends with the `\r` of a CRLF buffer is measured without that
/// `\r`; the carriage return is a line terminator, not a source column.
pub fn range_for(lines: &[String], line: usize, column: usize) -> Option<DiagnosticRange> {
    if line == 0 || line > lines.len() {
        return None;
    }
    let text = &lines[line - 1];
    let content = text.strip_suffix('\r').unwrap_or(text);
    let target_units = column.saturating_sub(1);

    let mut consumed_units = 0usize;
    let mut start_char = content.chars().count();
    let mut start_byte = content.len();
    for (char_index, (byte_index, character)) in content.char_indices().enumerate() {
        let next = consumed_units + character.len_utf16();
        if target_units < next {
            start_char = char_index;
            start_byte = byte_index;
            break;
        }
        consumed_units = next;
    }

    Some(DiagnosticRange {
        line,
        start_char,
        end_char: content.chars().count(),
        start_byte,
        end_byte: content.len(),
    })
}

/// Converts a 0-based char column on `lines[row]` into a byte offset in the buffer text.
///
/// The buffer text is the editor's own source: the lines joined with `\n`. A `\r` left by a
/// CRLF split is part of the line in that text (only [`range_for`] removes it, because it is a
/// diagnostic column and not a source position), so a selection that covers the CR counts it.
/// A column past the end of the line clamps to the end of the line; a row outside the buffer
/// returns `None`.
pub fn source_byte_offset(lines: &[String], row: usize, char_col: usize) -> Option<usize> {
    let line = lines.get(row)?;
    let mut offset: usize = lines[..row].iter().map(|line| line.len() + 1).sum();
    for character in line.chars().take(char_col) {
        offset += character.len_utf8();
    }
    Some(offset)
}

/// Converts a 0-based char column on `lines[row]` into a 0-based UTF-16 code-unit column.
///
/// This is the column unit the engine names and the status strip publishes; an astral character
/// is two columns wide. A column past the end of the line clamps to the line's UTF-16 length;
/// a row outside the buffer returns `None`.
pub fn utf16_column(lines: &[String], row: usize, char_col: usize) -> Option<usize> {
    let line = lines.get(row)?;
    Some(line.chars().take(char_col).map(char::len_utf16).sum())
}

/// Converts a selection's 0-based `(row, char column)` endpoints into byte offsets.
///
/// The editor stores a selection in char columns, its cell unit; the engine and diagnostics use
/// UTF-16 code units and the buffer itself is UTF-8. Both endpoints are converted by walking
/// character boundaries, so a byte offset can never land inside a UTF-8 code point. The returned
/// range slices exactly the characters the editor's selection covers in the joined buffer text
/// (the same text the editor's `source` method returns). Returns `None` when either row is
/// outside the buffer or the endpoints are not in reading order.
pub fn selection_byte_range(
    lines: &[String],
    start: (usize, usize),
    end: (usize, usize),
) -> Option<(usize, usize)> {
    if start > end {
        return None;
    }
    Some((
        source_byte_offset(lines, start.0, start.1)?,
        source_byte_offset(lines, end.0, end.1)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(source: &str) -> Vec<String> {
        source.split('\n').map(|line| line.to_string()).collect()
    }

    /// (a) A plain ASCII multi-line script: line 3 column 1 underlines the whole failing line.
    #[test]
    fn ascii_multiline_maps_the_reported_line_from_column_to_end_of_line() {
        let source = "alpha = 1\nbeta = 2\n1 / 0\ngamma = 3\n";
        let range = range_for(&lines(source), 3, 1).expect("line 3 exists");
        assert_eq!(
            range,
            DiagnosticRange {
                line: 3,
                start_char: 0,
                end_char: 5,
                start_byte: 0,
                end_byte: 5,
            }
        );
        // The gutter line number is the diagnostic's line, not a buffer row.
        assert_eq!(range.line, 3);
    }

    /// (b) An astral character before the error column: the emoji is two UTF-16 units, so
    /// column 3 is the `x` after it (char index 1, byte 4), not the space at char 2.
    #[test]
    fn astral_character_before_the_error_counts_as_two_utf16_units() {
        let source = "n = 1\n\u{1f600}x = 1 / 0\n2\n";
        let range = range_for(&lines(source), 2, 3).expect("line 2 exists");
        assert_eq!(
            range,
            DiagnosticRange {
                line: 2,
                start_char: 1,
                end_char: 10,
                start_byte: 4,
                end_byte: 13,
            },
            "an astral character is two UTF-16 code units: column 3 starts at 'x'"
        );
        // A column inside the surrogate pair clamps to the emoji itself rather than splitting it.
        let inside = range_for(&lines("\u{1f600}x"), 1, 2).expect("line 1 exists");
        assert_eq!((inside.start_char, inside.start_byte), (0, 0));
    }

    /// (c) CRLF input: the `\r` is a line terminator and is not part of the underlined span.
    #[test]
    fn crlf_input_excludes_the_carriage_return_from_the_range() {
        let source = "a = 1\r\nb = 2\r\n1 / 0\r\n";
        let all = lines(source);
        assert_eq!(
            all[2], "1 / 0\r",
            "the editor keeps the CR in the split line"
        );
        let range = range_for(&all, 3, 3).expect("line 3 exists");
        assert_eq!(
            range,
            DiagnosticRange {
                line: 3,
                start_char: 2,
                end_char: 5,
                start_byte: 2,
                end_byte: 5,
            },
            "the underline starts at '/' and stops before the CR"
        );
    }

    /// (d) An error on the last line, plus the out-of-buffer cases that must not draw a marker.
    #[test]
    fn last_line_maps_exactly_and_missing_lines_have_no_range() {
        let source = "a = 1\nb = 2\nc = 3 / 0";
        let range = range_for(&lines(source), 3, 5).expect("line 3 exists");
        assert_eq!(
            range,
            DiagnosticRange {
                line: 3,
                start_char: 4,
                end_char: 9,
                start_byte: 4,
                end_byte: 9,
            },
            "column 5 is the '3' of 'c = 3 / 0' on the last line"
        );

        let all = lines(source);
        assert_eq!(range_for(&all, 4, 1), None, "line past the buffer");
        assert_eq!(range_for(&all, 0, 1), None, "line zero is not 1-based");
        assert_eq!(range_for(&[], 1, 1), None, "an empty buffer has no line");
    }

    /// (e) A combining mark is one UTF-16 code unit but two UTF-8 bytes. Column 6 is the `1` of
    /// the failing `1 / 0`; the range must start at byte 6, after the whole `e` + U+0301 grapheme
    /// (bytes 0..3), and cover exactly the failing token, not a shifted or half-grapheme span.
    #[test]
    fn combining_mark_before_the_error_keeps_the_underlined_token_byte_exact() {
        let line = "e\u{0301} = 1 / 0";
        let range = range_for(&lines(line), 1, 6).expect("line 1 exists");
        assert_eq!(
            range,
            DiagnosticRange {
                line: 1,
                start_char: 5,
                end_char: 10,
                start_byte: 6,
                end_byte: 11,
            },
            "column 6 is the '1'; U+0301 adds one extra byte before it"
        );
        assert_eq!(
            &line[range.start_byte..range.end_byte],
            "1 / 0",
            "the underline covers the failing token, not a byte-shifted slice"
        );
        assert_eq!(
            &line[..range.start_byte],
            "e\u{0301} = ",
            "the whole two-code-point grapheme is before the range"
        );
        assert_eq!(line.chars().nth(range.start_char), Some('1'));

        // The same bytes appear when the engine names the column itself: the second statement of
        // `s = "e\u{0301}"; 1 / 0` starts at byte 11 and 1-based UTF-16 column 11.
        let source = "s = \"e\u{0301}\"; 1 / 0";
        let mut engine = crate::engine::default_engine_factory()();
        let message = engine
            .eval(source)
            .expect_err("the combining-mark line fails at `1 / 0`");
        assert_eq!(message, "Cannot divide by zero.");
        let all = crate::statements::slice(source);
        assert_eq!(all.len(), 2, "{all:?}");
        assert_eq!(all[1].text(), "1 / 0");
        assert_eq!(all[1].byte_offset(), 11);
        let diagnostic = crate::execution::Diagnostic::for_statement(&all[1], source, message);
        assert_eq!(
            (diagnostic.line, diagnostic.column, diagnostic.byte_offset),
            (1, 11, 11),
            "the engine's column after the combining mark is UTF-16 column 11"
        );
        let reported =
            range_for(&lines(source), diagnostic.line, diagnostic.column).expect("line 1 exists");
        assert_eq!(
            (reported.start_char, reported.start_byte),
            (10, 11),
            "byte 11 is the '1', not the space at byte 10"
        );
        assert_eq!(
            &source[diagnostic.byte_offset..diagnostic.byte_offset + 5],
            "1 / 0"
        );
    }

    /// (f) The `\r` of a CRLF buffer is a terminator, not a source column: LF and CRLF report the
    /// same 1-based line and the same range, including one column past the five-char line, where
    /// a CR counted as a column would start the empty range on the CR.
    #[test]
    fn lf_and_crlf_report_the_same_line_and_column_range_and_exclude_the_cr() {
        let lf = lines("a = 1\nb = 2\n1 / 0\n");
        let crlf = lines("a = 1\r\nb = 2\r\n1 / 0\r\n");
        assert_eq!(lf[2], "1 / 0");
        assert_eq!(
            crlf[2], "1 / 0\r",
            "the editor keeps the CR in the split line"
        );

        let from_lf = range_for(&lf, 3, 3).expect("line 3 exists");
        let from_crlf = range_for(&crlf, 3, 3).expect("line 3 exists");
        assert_eq!(
            from_lf, from_crlf,
            "the same error column must report the same range on either newline"
        );
        assert_eq!(
            from_lf,
            DiagnosticRange {
                line: 3,
                start_char: 2,
                end_char: 5,
                start_byte: 2,
                end_byte: 5,
            }
        );
        assert_eq!(&crlf[2][from_crlf.start_byte..from_crlf.end_byte], "/ 0");

        // Column 6 is one past the line's five characters: both buffers clamp to end-of-line.
        let past_lf = range_for(&lf, 3, 6).expect("line 3 exists");
        let past_crlf = range_for(&crlf, 3, 6).expect("line 3 exists");
        assert_eq!(past_lf, past_crlf);
        assert_eq!(
            (past_crlf.start_char, past_crlf.start_byte),
            (5, 5),
            "the CR is not counted as the line's sixth character"
        );
        assert_eq!(
            &crlf[2][..past_crlf.start_byte],
            "1 / 0",
            "the line text used by the range excludes the CR"
        );
    }

    /// (g) An emoji line, a combining-mark line and CRLF terminators all precede the failing
    /// line. The engine's own failure message resolves to (line 3, column 1), byte 23 in the
    /// source, and line 3's range 0..5 chars / 0..5 bytes.
    #[test]
    fn emoji_combining_mark_and_crlf_before_the_error_give_the_exact_tuple() {
        let source = "s = \"\u{1f600}\"\r\nt = \"e\u{0301}\"\r\n1 / 0\r\n";
        let mut engine = crate::engine::default_engine_factory()();
        let message = engine
            .eval(source)
            .expect_err("the mixed script fails at `1 / 0`");
        assert_eq!(message, "Cannot divide by zero.");

        let all = crate::statements::slice(source);
        assert_eq!(all.len(), 3, "{all:?}");
        assert_eq!(all[2].text(), "1 / 0");
        assert_eq!(
            all[2].byte_offset(),
            23,
            "line 3 starts after 12 + 11 bytes of CRLF lines"
        );

        let diagnostic = crate::execution::Diagnostic::for_statement(&all[2], source, message);
        assert_eq!(
            (diagnostic.line, diagnostic.column, diagnostic.byte_offset),
            (3, 1, 23),
            "the diagnostic names the exact line, UTF-16 column and source byte"
        );
        assert_eq!(
            &source[diagnostic.byte_offset..diagnostic.byte_offset + 5],
            "1 / 0"
        );

        let range =
            range_for(&lines(source), diagnostic.line, diagnostic.column).expect("line 3 exists");
        assert_eq!(
            (
                range.line,
                range.start_char,
                range.end_char,
                range.start_byte,
                range.end_byte,
            ),
            (3, 0, 5, 0, 5),
            "the final tuple is line 3, column 1, line bytes 0..5"
        );
        assert_eq!(lines(source)[2], "1 / 0\r");
    }

    /// A column past the end of the line yields an empty range at end-of-line (the painter gives
    /// it one cell of width), and the line is still marked.
    #[test]
    fn a_column_past_end_of_line_is_empty_at_end_of_line() {
        let range = range_for(&lines("abc"), 1, 99).expect("line 1 exists");
        assert_eq!(
            range,
            DiagnosticRange {
                line: 1,
                start_char: 3,
                end_char: 3,
                start_byte: 3,
                end_byte: 3,
            }
        );
    }
}
