//! The host-side script source: identity, normalisation, and the map back to the caller's text.
//!
//! This is the additive Rust counterpart of two reference types that the wire never exposes:
//!
//! * Lovelace.Suite.ScriptSource.ToSemicolonStatements - the newline-to-semicolon normaliser every
//!   C# front-end applies before evaluation, with the exact brace/bracket/paren, string and
//!   byte-order-mark rules (including the length-preserving newline rewrite); and
//! * Lovelace.Run.ScriptPositions - the caller-text/engine-text pair the runner uses to translate
//!   an engine offset back into the caller's UTF-16 offset before publishing a diagnostic or timing
//!   position.
//!
//! The runner itself evaluates the caller's text directly (the live tokenizer treats a top-level
//! newline as a separator), so the normalised text and the offset map are host-facing API, not a
//! change to the envelope. SourceIdentity and Diagnostic add the other half the reference keeps on
//! the host: a diagnostic can name the source it belongs to (eval, stdin, or a file path) without
//! adding a field to the four-field wire diagnostic.
//!
//! Everything here is additive: the published JSON shape, exit codes and evaluation behaviour are
//! unchanged. The wire remains the four-field message/position/line/column diagnostic this module
//! serialises.

use lovelace_json::Json;

/// The host surface a script's text was supplied through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceOrigin {
    /// The text came from the command line (--eval).
    Eval,
    /// The text was read from a file (a bare path, --file, or a file read by an embedding host).
    File,
    /// The text was read from standard input (--stdin).
    Stdin,
}

impl SourceOrigin {
    /// The stable label a host can show next to a diagnostic.
    pub fn label(self) -> &'static str {
        match self {
            SourceOrigin::Eval => "eval",
            SourceOrigin::File => "file",
            SourceOrigin::Stdin => "stdin",
        }
    }
}

/// The identity a diagnostic names its source by.
///
/// The reference's ScriptSource carries no name: a C# host knows which file it read. The runner has
/// three source surfaces, so this type records which one produced the text. A file source carries
/// its path; an inline or stdin source has a display name but no path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceIdentity {
    name: String,
    path: Option<String>,
    origin: SourceOrigin,
}

impl SourceIdentity {
    /// The identity of a script supplied inline (the --eval surface).
    pub fn eval() -> Self {
        Self {
            name: "<eval>".to_string(),
            path: None,
            origin: SourceOrigin::Eval,
        }
    }

    /// The identity of a script supplied on standard input.
    pub fn stdin() -> Self {
        Self {
            name: "<stdin>".to_string(),
            path: None,
            origin: SourceOrigin::Stdin,
        }
    }

    /// The identity of a script read from path.
    pub fn file(path: impl Into<String>) -> Self {
        let path = path.into();
        Self {
            name: path.clone(),
            path: Some(path),
            origin: SourceOrigin::File,
        }
    }

    /// A display name for a host surface that is not one of the three runner surfaces.
    pub fn named(name: impl Into<String>, origin: SourceOrigin) -> Self {
        Self {
            name: name.into(),
            path: None,
            origin,
        }
    }

    /// The name a diagnostic shows for this source.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The originating file path, when the source is a file.
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// The host surface the text was supplied through.
    pub fn origin(&self) -> SourceOrigin {
        self.origin
    }
}

/// Normalises a script to the engine's semicolon-separated program text.
///
/// This is the exact port of ScriptSource.ToSemicolonStatements:
///
/// 1. CRLF and lone CR become LF;
/// 2. one leading U+FEFF is stripped;
/// 3. a newline at bracket depth zero is rewritten to a semicolon (or a space after an existing
///    semicolon and at the start), while newlines inside braces/brackets/parens and inside a
///    double-quoted string are preserved.
///
/// The rewrite is length-preserving once line endings are normalised: each newline becomes exactly
/// one character, so an engine offset can be translated back to the caller's text by
/// ScriptPositions. Quoting is the reference's simple rule: a double quote toggles string mode, and
/// a backslash has no special meaning.
pub fn to_semicolon_statements(source: &str) -> String {
    let text = source.replace("\r\n", "\n").replace('\r', "\n");
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);

    let mut out = String::with_capacity(text.len());
    let mut depth = 0usize;
    let mut in_string = false;
    // The last non-whitespace character emitted outside a string, as the reference tracks it.
    let mut last: Option<char> = None;

    for c in text.chars() {
        if in_string {
            out.push(c);
            if c == '"' {
                in_string = false;
                last = Some(c);
            }
            continue;
        }

        if c == '"' {
            in_string = true;
            out.push(c);
            last = Some(c);
            continue;
        }

        match c {
            '{' | '[' | '(' => {
                depth += 1;
                out.push(c);
                last = Some(c);
            }
            '}' | ']' | ')' => {
                depth = depth.saturating_sub(1);
                out.push(c);
                last = Some(c);
            }
            '\n' => {
                if depth == 0 {
                    // A separator after a semicolon (trailing/blank lines) and at the start is a
                    // space.
                    let replacement = if last.is_none() || last == Some(';') {
                        ' '
                    } else {
                        ';'
                    };
                    out.push(replacement);
                    if replacement == ';' {
                        last = Some(';');
                    }
                } else {
                    out.push('\n');
                }
            }
            _ => {
                if !c.is_whitespace() {
                    last = Some(c);
                }
                out.push(c);
            }
        }
    }

    out
}

/// The caller's script text paired with the engine's normalised text and the offset map between
/// them.
///
/// This is the port of Lovelace.Run.ScriptPositions. Offsets are UTF-16 code units, the unit the
/// engine and the published positions use. The map exists because normalisation is not a bijection
/// for two inputs: a CRLF pair becomes one LF (so every later engine offset is one short per
/// preceding Windows line break), and a leading U+FEFF is dropped (shifting the first character by
/// one). The newline-to-semicolon pass itself is one character in, one character out, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptPositions {
    source: String,
    engine_source: String,
    // source_index[e] is the caller-text UTF-16 offset that engine offset e came from.
    source_index: Vec<usize>,
    source_len: usize,
}

impl ScriptPositions {
    /// Builds the normalised text and the offset map for source once.
    pub fn new(source: &str) -> Self {
        let engine_source = to_semicolon_statements(source);
        let source_units: Vec<u16> = source.encode_utf16().collect();
        let engine_len = engine_source.encode_utf16().count();
        let mut source_index = Vec::with_capacity(engine_len);
        let mut s = usize::from(source_units.first() == Some(&0xFEFF));
        for _ in 0..engine_len {
            if s + 1 < source_units.len()
                && source_units[s] == 0x000D
                && source_units[s + 1] == 0x000A
            {
                // The engine's one character belongs to the LF half of the pair.
                s += 1;
            }
            source_index.push(s);
            s += 1;
        }
        Self {
            source: source.to_string(),
            engine_source,
            source_index,
            source_len: source_units.len(),
        }
    }

    /// The text the caller supplied: the text every published position indexes into.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The text the engine is handed after normalisation.
    pub fn engine_source(&self) -> &str {
        &self.engine_source
    }

    /// Translates an engine offset to the caller's UTF-16 offset.
    ///
    /// Negative offsets clamp to the start; an offset at or past the end of the engine text clamps
    /// to the end of the caller's text (a lexer that ran off the end reports the end), exactly as
    /// the reference's ToSourceOffset does.
    pub fn to_source_offset(&self, engine_offset: isize) -> usize {
        if engine_offset < 0 {
            return 0;
        }
        match self.source_index.get(engine_offset as usize) {
            Some(&offset) => offset,
            None if self.source_index.is_empty() => 0,
            None => self.source_len,
        }
    }

    /// The 1-based line and column of a caller-text offset.
    ///
    /// CRLF, CR and LF each end a line exactly once, so a Windows, classic-Mac and Unix script
    /// report the same place. The scan is over UTF-16 code units, matching the published protocol.
    /// An offset past the end is reported as line 1, column offset + 1 (the reference's
    /// out-of-range rule).
    pub fn line_column(&self, source_offset: usize) -> (usize, usize) {
        let units: Vec<u16> = self.source.encode_utf16().collect();
        if source_offset > units.len() {
            return (1, source_offset + 1);
        }

        let mut line = 1usize;
        let mut line_start = 0usize;
        let mut i = 0usize;
        while i < source_offset {
            if units[i] == 0x000A {
                // The LF half of a CRLF: the break was counted when the CR was read.
                if i > 0 && units[i - 1] == 0x000D {
                    i += 1;
                    continue;
                }
                line += 1;
                line_start = i + 1;
            } else if units[i] == 0x000D {
                line += 1;
                line_start = i + if i + 1 < units.len() && units[i + 1] == 0x000A {
                    2
                } else {
                    1
                };
            }
            i += 1;
        }

        (line, (source_offset.saturating_sub(line_start)) + 1)
    }
}

/// One source-positioned diagnostic plus the identity of the source it belongs to.
///
/// The four wire fields are what to_json publishes; the identity is host metadata the envelope
/// deliberately does not carry, so a host can say "demo.ls:3:1: ..." without changing the protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    message: String,
    position: usize,
    line: usize,
    column: usize,
    source: SourceIdentity,
}

impl Diagnostic {
    /// The diagnostic text.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The zero-based UTF-16 offset into the caller's text.
    pub fn position(&self) -> usize {
        self.position
    }

    /// The 1-based line in the caller's text.
    pub fn line(&self) -> usize {
        self.line
    }

    /// The 1-based column in the caller's text.
    pub fn column(&self) -> usize {
        self.column
    }

    /// The source this diagnostic belongs to.
    pub fn source(&self) -> &SourceIdentity {
        &self.source
    }

    /// Serialises the four-field wire diagnostic (message, position, line, column). The source
    /// identity is intentionally not a field: the envelope shape is unchanged.
    pub fn to_json(&self) -> Json {
        debug_assert!(
            !self.source.name().is_empty(),
            "a diagnostic must name its source"
        );
        Json::obj(vec![
            ("message", Json::Str(self.message.clone())),
            ("position", Json::Num(self.position.to_string())),
            ("line", Json::Num(self.line.to_string())),
            ("column", Json::Num(self.column.to_string())),
        ])
    }
}

/// A script as the host received it: its identity, the caller's text, and the normalised text with
/// the map back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptSource {
    identity: SourceIdentity,
    text: String,
    positions: ScriptPositions,
}

impl ScriptSource {
    /// Builds a source for text under identity, normalising it once.
    pub fn new(identity: SourceIdentity, text: impl Into<String>) -> Self {
        let text = text.into();
        let positions = ScriptPositions::new(&text);
        Self {
            identity,
            text,
            positions,
        }
    }

    /// The source's identity.
    pub fn identity(&self) -> &SourceIdentity {
        &self.identity
    }

    /// The caller's text exactly as received.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The normalised text the engine is handed, and the map back to the caller's text.
    pub fn positions(&self) -> &ScriptPositions {
        &self.positions
    }

    /// The engine's semicolon-separated text.
    pub fn normalized(&self) -> &str {
        self.positions.engine_source()
    }

    /// A diagnostic at a CALLER-TEXT offset.
    pub fn diagnostic(&self, message: impl Into<String>, source_offset: usize) -> Diagnostic {
        let (line, column) = self.positions.line_column(source_offset);
        Diagnostic {
            message: message.into(),
            position: source_offset,
            line,
            column,
            source: self.identity.clone(),
        }
    }

    /// A diagnostic at an ENGINE-TEXT offset, translated back to the caller's text first.
    ///
    /// This is the reference's PublishDiagnostic path: the engine indexes the normalised text, and
    /// the host publishes a place in the text the caller supplied.
    pub fn diagnostic_at_engine_offset(
        &self,
        message: impl Into<String>,
        engine_offset: isize,
    ) -> Diagnostic {
        let source_offset = self.positions.to_source_offset(engine_offset);
        self.diagnostic(message, source_offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------
    // ScriptSourceTests.ToSemicolonStatements_* (the seven census rows)
    // -----------------------------------------------------------------

    #[test]
    fn to_semicolon_statements_given_newline_separated_statements_joins_with_semicolons() {
        assert_eq!(
            to_semicolon_statements("a = 1\nb = 2\nc = 3"),
            "a = 1;b = 2;c = 3"
        );
    }

    #[test]
    fn to_semicolon_statements_given_crlf_normalizes_line_endings() {
        assert_eq!(to_semicolon_statements("a = 1\r\nb = 2"), "a = 1;b = 2");
    }

    #[test]
    fn to_semicolon_statements_given_block_preserves_newlines_inside_braces() {
        assert_eq!(
            to_semicolon_statements("func f(x) {\n  return x\n}"),
            "func f(x) {\n  return x\n}"
        );
    }

    #[test]
    fn to_semicolon_statements_given_newline_after_block_emits_semicolon() {
        assert_eq!(
            to_semicolon_statements("func f(x) {\n  return x\n}\ng()"),
            "func f(x) {\n  return x\n};g()"
        );
    }

    #[test]
    fn to_semicolon_statements_given_string_with_newline_preserves_it() {
        assert_eq!(to_semicolon_statements("s = \"a\nb\""), "s = \"a\nb\"");
    }

    #[test]
    fn to_semicolon_statements_given_trailing_semicolon_suppresses_blank_separator() {
        assert_eq!(to_semicolon_statements("a = 1;\nb = 2"), "a = 1; b = 2");
    }

    #[test]
    fn to_semicolon_statements_given_leading_bom_strips_it() {
        assert_eq!(
            to_semicolon_statements("\u{feff}a = 1\nb = 2"),
            "a = 1;b = 2"
        );
    }

    // -----------------------------------------------------------------
    // Additive surface: source identity, offset map, diagnostics
    // -----------------------------------------------------------------

    #[test]
    fn source_identity_distinguishes_eval_stdin_and_file() {
        let eval = SourceIdentity::eval();
        assert_eq!(eval.name(), "<eval>");
        assert_eq!(eval.path(), None);
        assert_eq!(eval.origin(), SourceOrigin::Eval);

        let stdin = SourceIdentity::stdin();
        assert_eq!(stdin.name(), "<stdin>");
        assert_eq!(stdin.path(), None);
        assert_eq!(stdin.origin(), SourceOrigin::Stdin);

        let file = SourceIdentity::file("scripts/demo.ls");
        assert_eq!(file.name(), "scripts/demo.ls");
        assert_eq!(file.path(), Some("scripts/demo.ls"));
        assert_eq!(file.origin(), SourceOrigin::File);
        assert_eq!(SourceOrigin::File.label(), "file");
    }

    #[test]
    fn script_source_reports_the_caller_text_and_its_normalised_form() {
        let source = ScriptSource::new(SourceIdentity::stdin(), "a = 1\r\nb = 2");
        assert_eq!(source.text(), "a = 1\r\nb = 2");
        assert_eq!(source.normalized(), "a = 1;b = 2");
        assert_eq!(source.positions().engine_source(), "a = 1;b = 2");
        assert_eq!(source.identity().origin(), SourceOrigin::Stdin);
    }

    #[test]
    fn positions_map_crlf_engine_offsets_back_to_the_callers_text() {
        // Caller: a = 1 CR LF b = 2; the engine's semicolon is the LF half of the pair.
        let positions = ScriptPositions::new("a = 1\r\nb = 2");
        assert_eq!(positions.engine_source(), "a = 1;b = 2");
        assert_eq!(positions.to_source_offset(4), 4);
        assert_eq!(positions.to_source_offset(5), 6);
        assert_eq!(positions.to_source_offset(6), 7);
        assert_eq!(positions.to_source_offset(-1), 0);
        assert_eq!(positions.to_source_offset(99), 12);
        assert_eq!(positions.line_column(7), (2, 1));
    }

    #[test]
    fn positions_map_a_leading_bom_back_to_the_callers_text() {
        let positions = ScriptPositions::new("\u{feff}a = 1\nb = 2");
        assert_eq!(positions.engine_source(), "a = 1;b = 2");
        assert_eq!(positions.to_source_offset(0), 1);
        assert_eq!(positions.to_source_offset(1), 2);
        assert_eq!(positions.to_source_offset(5), 6);
        assert_eq!(positions.to_source_offset(6), 7);
        assert_eq!(positions.line_column(1), (1, 2));
    }

    #[test]
    fn positions_are_utf16_code_unit_offsets() {
        // The closing quote is the 7th UTF-16 unit on line 1 (the emoji is two units, one char).
        let positions = ScriptPositions::new("a = \"\u{1f600}\"\nb = 2");
        assert_eq!(positions.line_column(7), (1, 8));
        assert_eq!(positions.line_column(9), (2, 1));
        // The engine text is 14 UTF-16 units; char-based indexing would lose one.
        assert_eq!(positions.engine_source().encode_utf16().count(), 14);
        assert_eq!(positions.to_source_offset(13), 13);
    }

    #[test]
    fn diagnostic_names_its_source_and_keeps_the_four_field_wire_shape() {
        let source = ScriptSource::new(SourceIdentity::file("demo.ls"), "a = 1\nb = 2");
        let diagnostic = source.diagnostic("boom", 6);
        assert_eq!(diagnostic.source().name(), "demo.ls");
        assert_eq!(diagnostic.source().origin(), SourceOrigin::File);
        assert_eq!(diagnostic.position(), 6);
        assert_eq!((diagnostic.line(), diagnostic.column()), (2, 1));

        let json = diagnostic.to_json();
        assert_eq!(json.get("message").and_then(Json::as_str), Some("boom"));
        assert_eq!(json.get("position"), Some(&Json::Num("6".to_string())));
        assert_eq!(json.get("line"), Some(&Json::Num("2".to_string())));
        assert_eq!(json.get("column"), Some(&Json::Num("1".to_string())));
    }

    #[test]
    fn diagnostic_at_engine_offset_translates_before_it_names_the_line() {
        // Engine offset 6 is 'b' in the normalised text; in the caller's CRLF text that is index 7.
        let source = ScriptSource::new(SourceIdentity::file("demo.ls"), "a = 1\r\nb = 2");
        let diagnostic = source.diagnostic_at_engine_offset("boom", 6);
        assert_eq!(diagnostic.position(), 7);
        assert_eq!((diagnostic.line(), diagnostic.column()), (2, 1));
        assert_eq!(diagnostic.source().path(), Some("demo.ls"));
    }

    #[test]
    fn empty_source_has_no_offset_a_script_can_name() {
        let source = ScriptSource::new(SourceIdentity::eval(), "");
        assert_eq!(source.normalized(), "");
        assert_eq!(source.positions().to_source_offset(0), 0);
        assert_eq!(source.positions().line_column(0), (1, 1));
    }
}
