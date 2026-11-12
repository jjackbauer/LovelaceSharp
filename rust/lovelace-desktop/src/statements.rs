//! Top-level statement slicing for Lovelace source.
//!
//! The desktop application runs a document one top-level statement at a time so that progress,
//! cancellation and diagnostics can name a step. This module owns only that split: it does not
//! parse expressions, change source text or know about the engine. A statement ends at a
//! semicolon or a newline that is outside strings, parentheses, brackets and braces, or at the
//! end of the buffer. A `#` comment is skipped for separator purposes (a `;` in a comment does
//! not split), and a statement made only of comments is not a statement.
//!
//! The returned [Statement] values keep the source span they came from: the trimmed text plus
//! its absolute byte offset and 1-based start line. Callers reconstruct the original buffer by
//! concatenating the gaps of the source between successive statements with the statement texts.

/// A single top-level statement and the place it came from in the full source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Statement {
    text: String,
    byte_offset: usize,
    start_line: usize,
    index: usize,
}

impl Statement {
    /// The statement text with surrounding whitespace removed.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The absolute byte offset of the first character of [Statement::text] in the full source.
    pub fn byte_offset(&self) -> usize {
        self.byte_offset
    }

    /// The 1-based line the statement starts on in the full source.
    pub fn start_line(&self) -> usize {
        self.start_line
    }

    /// The 0-based position of the statement among the sliced statements.
    pub fn index(&self) -> usize {
        self.index
    }

    /// A short human-readable label, used by progress events and the status bar.
    pub fn label(&self) -> String {
        let first_line = self.text.lines().next().unwrap_or_default().trim();
        let mut label = String::new();
        for (count, ch) in first_line.chars().enumerate() {
            if count == 60 {
                label.push('…');
                break;
            }
            label.push(ch);
        }
        label
    }
}

/// Splits a source buffer into its top-level statements.
///
/// The function is total: every input, including empty, comment-only and unbalanced input,
/// yields a vector and never panics.
pub fn slice(source: &str) -> Vec<Statement> {
    let bytes = source.as_bytes();
    let line_starts = line_starts(source);
    let mut statements = Vec::new();
    let mut segment_start = 0usize;
    let mut depth = 0i32;
    let mut index = 0usize;
    let mut quote: Option<u8> = None;
    let mut in_comment = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if in_comment {
            if byte == b'\n' {
                in_comment = false;
                if depth <= 0 {
                    push_statement(source, segment_start, index, &line_starts, &mut statements);
                    segment_start = index + 1;
                }
            }
            index += 1;
            continue;
        }
        if let Some(active_quote) = quote {
            if byte == b'\\' {
                // An escape hides the next byte from quote/separator handling. Skipping two
                // bytes is safe for UTF-8: every continuation byte is non-ASCII, so it can
                // never be mistaken for a quote or a separator.
                index += 2;
                continue;
            }
            if byte == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }
        match byte {
            b'"' | b'\'' => quote = Some(byte),
            b'#' => in_comment = true,
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth = (depth - 1).max(0),
            b';' | b'\n' if depth <= 0 => {
                push_statement(source, segment_start, index, &line_starts, &mut statements);
                segment_start = index + 1;
            }
            _ => {}
        }
        index += 1;
    }

    push_statement(
        source,
        segment_start,
        source.len(),
        &line_starts,
        &mut statements,
    );
    statements
}

/// Returns the 1-based line and column of a byte offset in `source`.
///
/// Columns count **UTF-16 code units**, the convention the engine publishes
/// (`lovelace_lang::LangError::position` is a UTF-16 code-unit offset) and the editor maps with
/// [`crate::diagnostic_map`]. That makes an astral character two columns wide, exactly as it is
/// two `char` units in the engine. Lines are split on `\n`, so a CRLF pair ends one line and the
/// `\r` is not counted as a column.
pub fn line_column(source: &str, byte_offset: usize) -> (usize, usize) {
    let mut offset = byte_offset.min(source.len());
    while !source.is_char_boundary(offset) {
        offset -= 1;
    }
    let before = &source[..offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map(|tail| tail.encode_utf16().count() + 1)
        .unwrap_or(1);
    (line, column)
}

/// The byte offsets at which each source line starts.
fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    starts.extend(source.match_indices('\n').map(|(index, _)| index + 1));
    starts
}

/// The 1-based line for a byte offset, from the precomputed line starts.
fn line_of(line_starts: &[usize], byte_offset: usize) -> usize {
    line_starts
        .partition_point(|start| *start <= byte_offset)
        .max(1)
}

/// Records the trimmed span `source[start..end]` when it contains a statement.
fn push_statement(
    source: &str,
    start: usize,
    end: usize,
    line_starts: &[usize],
    statements: &mut Vec<Statement>,
) {
    let raw = &source[start..end];
    let leading_trimmed = raw.trim_start();
    let text = leading_trimmed.trim_end();
    if text.is_empty() || text.starts_with('#') {
        return;
    }
    let byte_offset = start + (raw.len() - leading_trimmed.len());
    statements.push(Statement {
        text: text.to_string(),
        byte_offset,
        start_line: line_of(line_starts, byte_offset),
        index: statements.len(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rebuilds the buffer from the statements and the untouched gaps around them.
    fn reconstruct(source: &str, statements: &[Statement]) -> String {
        let mut rebuilt = String::new();
        let mut cursor = 0usize;
        for statement in statements {
            assert!(
                statement.byte_offset() >= cursor,
                "statement {:?} starts before the previous statement ended",
                statement
            );
            rebuilt.push_str(&source[cursor..statement.byte_offset()]);
            rebuilt.push_str(statement.text());
            cursor = statement.byte_offset() + statement.text().len();
        }
        rebuilt.push_str(&source[cursor..]);
        rebuilt
    }

    #[test]
    fn t1_slicer_yields_three_statements_that_reconstruct_the_input() {
        let source = "a = 1\nb = \"x;y\nz\"\nc = (1 +\n2) # note\n";
        let statements = slice(source);
        assert_eq!(statements.len(), 3, "statements: {statements:?}");
        assert_eq!(statements[0].text(), "a = 1");
        assert_eq!(statements[1].text(), "b = \"x;y\nz\"");
        assert_eq!(statements[2].text(), "c = (1 +\n2) # note");
        assert_eq!(
            statements
                .iter()
                .map(Statement::byte_offset)
                .collect::<Vec<_>>(),
            vec![0, 6, 18]
        );
        assert_eq!(
            statements
                .iter()
                .map(Statement::start_line)
                .collect::<Vec<_>>(),
            vec![1, 2, 4]
        );
        assert_eq!(
            statements.iter().map(Statement::index).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(reconstruct(source, &statements), source);
    }

    #[test]
    fn t1_a_brace_block_and_a_semicolon_string_are_each_one_statement() {
        let block_source = "block {\n  inner = 1; inner2 = 2\n}\nafter = 3\n";
        let block_statements = slice(block_source);
        assert_eq!(block_statements.len(), 2, "{block_statements:?}");
        assert_eq!(
            block_statements[0].text(),
            "block {\n  inner = 1; inner2 = 2\n}"
        );
        assert_eq!(block_statements[1].text(), "after = 3");
        assert_eq!(reconstruct(block_source, &block_statements), block_source);

        let string_source = "s = \"a;b\"\nt = 1\n";
        let string_statements = slice(string_source);
        assert_eq!(string_statements.len(), 2, "{string_statements:?}");
        assert_eq!(string_statements[0].text(), "s = \"a;b\"");
        assert_eq!(string_statements[1].text(), "t = 1");
        assert_eq!(
            reconstruct(string_source, &string_statements),
            string_source
        );
    }

    #[test]
    fn t1_crlf_source_slices_without_carriage_returns_in_statement_text() {
        let source = "a = 1\r\nb = \"x;y\"\r\nc = 3\r\n";
        let statements = slice(source);
        assert_eq!(statements.len(), 3, "{statements:?}");
        assert_eq!(
            statements.iter().map(Statement::text).collect::<Vec<_>>(),
            vec!["a = 1", "b = \"x;y\"", "c = 3"]
        );
        for statement in &statements {
            assert!(
                !statement.text().contains('\r'),
                "CR leaked into {:?}",
                statement.text()
            );
        }
        assert_eq!(reconstruct(source, &statements), source);
    }

    #[test]
    fn t1_empty_and_comment_only_sources_have_no_statements() {
        for source in [
            "",
            "   ",
            "\n\n\t\n",
            "# only a comment\n",
            "   # indented\n\t# tabbed\n",
            "  \r\n# comment with CRLF\r\n",
        ] {
            let statements = slice(source);
            assert_eq!(
                statements.len(),
                0,
                "source {source:?} produced {statements:?}"
            );
        }

        let with_comment_between = "a = 1\n# note\nb = 2\n";
        let statements = slice(with_comment_between);
        assert_eq!(statements.len(), 2, "{statements:?}");
        assert_eq!(statements[0].text(), "a = 1");
        assert_eq!(statements[1].text(), "b = 2");
        assert_eq!(
            reconstruct(with_comment_between, &statements),
            with_comment_between
        );
    }

    #[test]
    fn t1_statement_labels_name_the_first_line() {
        let statements = slice("alpha = 1\nbeta = 2\n");
        assert_eq!(statements[0].label(), "alpha = 1");
        assert_eq!(statements[1].label(), "beta = 2");
    }

    #[test]
    fn line_column_counts_utf16_code_units_and_crlf_once() {
        // "ab\n<emoji>x\n": the emoji starts line 2 at byte 3, and 'x' starts at byte 7.
        let source = "ab\n\u{1f600}x\n";
        assert_eq!(line_column(source, 3), (2, 1));
        assert_eq!(
            line_column(source, 7),
            (2, 3),
            "the emoji is two UTF-16 units"
        );
        assert_eq!(line_column(source, source.len()), (3, 1));

        // A CRLF pair ends exactly one line; the column restarts after the LF.
        let crlf = "ab\r\ncd";
        assert_eq!(line_column(crlf, 3), (1, 4), "the LF is column 4 of line 1");
        assert_eq!(line_column(crlf, 4), (2, 1));
        assert_eq!(line_column(crlf, 5), (2, 2));
    }
}
