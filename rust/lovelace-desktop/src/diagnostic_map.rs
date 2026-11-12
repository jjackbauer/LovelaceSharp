//! Maps a run diagnostic's 1-based line and UTF-16 column onto an editor line.
//!
//! The engine reports a position as a **UTF-16 code-unit offset** (see `lovelace_lang::LangError`)
//! and the execution core turns it into a 1-based line and column. The editor buffer, however, is
//! a `Vec<String>` of lines and its columns are counted in `char`s. This module is the one place
//! that translates between the two, so the underline and the gutter marker are placed by a rule
//! the unit tests below pin down instead of by ad-hoc arithmetic in the paint code.
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
