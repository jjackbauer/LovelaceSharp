//! A Lovelace grammar highlighter that produces an `egui::text::LayoutJob`.
//!
//! The classification rules are the engine parser's token rules
//! ([`lovelace_lang::syntax::tokenize`]): ASCII identifiers, integers and decimals, repeating
//! decimal literals, plain and `$"..."` interpolated strings, and the parser's punctuator set.
//! The highlighter adds only the context the parser cannot know from one line:
//!
//! - keywords come from [`crate::language::KEYWORDS`], the desktop's single keyword table;
//! - builtins come from an engine snapshot ([`crate::engine::EngineState`]), captured by the
//!   editor from the same live state completion uses and never transcribed into a second list;
//! - a function declaration's name is the identifier immediately after `func`;
//! - symbol names are discovered from `symbol("name")` calls in the same document, and every
//!   matching identifier in an expression is painted as a symbolic token.
//!
//! There is no comment syntax in the Lovelace grammar: the engine tokenizer rejects `#` with
//! `Unexpected character '#'`. The old highlighter painted `#` to end-of-line as a comment;
//! that class was wrong and is gone.
//!
//! Strings and interpolations are the only multi-token constructs a line can hold. An
//! interpolated string's literal text keeps the string colour, its `{` / `}` delimiters get the
//! interpolation colour, and the expression between them is classified by the same rules (so a
//! keyword or number inside `$"{...}"` is painted as a keyword or number).

use std::collections::BTreeSet;
use std::ops::Range;

use egui::{Color32, FontId, TextFormat, text::LayoutJob};

use crate::engine::EngineState;
use crate::language::KEYWORDS;

/// The token classes the highlighter can paint.
///
/// The class is the unit the unit tests assert; the palette decides the colour, so a future
/// theme change cannot change a classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenClass {
    /// Ordinary code, whitespace and characters the grammar does not recognise.
    Plain,
    /// An integer or decimal numeric literal, e.g. `12` or `3.25`.
    Number,
    /// A repeating decimal literal, the grammar's exact-rational literal form: `0.(3)`, `0.1(6)`.
    ///
    /// `1/3` is **not** a rational literal in this grammar: it tokenizes as `Number` `1`,
    /// `Operator` `/`, `Number` `3`, and the engine folds it to an exact rational at evaluation.
    Rational,
    /// A plain string literal, including its quotes and escape sequences.
    String,
    /// The `{` / `}` delimiters around an expression inside a `$"..."` literal.
    Interpolation,
    /// A statement keyword from [`crate::language::KEYWORDS`].
    Keyword,
    /// A punctuator that applies to operands: `+ - * / % ^ ! = == != < <= > >= ..`.
    Operator,
    /// The declared name in a `func name(...)` definition.
    Function,
    /// A builtin name from the engine registry snapshot.
    Builtin,
    /// A structural punctuator: `( ) [ ] { } , ; : .`.
    Delimiter,
    /// An identifier in the document's `symbol("name")` set.
    Symbol,
}

impl TokenClass {
    /// The class's colour in `palette`.
    pub fn color(self, palette: &Palette) -> Color32 {
        match self {
            TokenClass::Plain => palette.plain,
            TokenClass::Number => palette.number,
            TokenClass::Rational => palette.rational,
            TokenClass::String => palette.string,
            TokenClass::Interpolation => palette.interpolation,
            TokenClass::Keyword => palette.keyword,
            TokenClass::Operator => palette.operator,
            TokenClass::Function => palette.function,
            TokenClass::Builtin => palette.builtin,
            TokenClass::Delimiter => palette.delimiter,
            TokenClass::Symbol => palette.symbol,
        }
    }
}

/// Colours for one syntax theme.
///
/// Every class has its own colour. The closest pair is `keyword`/`symbol` (mauve vs lavender);
/// the next is `plain`/`symbol` (both light). `function`'s teal is the window's accent colour,
/// so the capture's palette probe excludes the pane header and border. The window capture in
/// `docs/goal-desktop-v4/round-20/` states these limits rather than claiming a wider visual
/// separation.
#[derive(Clone, Copy)]
pub struct Palette {
    /// Ordinary code and punctuation.
    pub plain: Color32,
    /// Integer and decimal literals.
    pub number: Color32,
    /// Repeating decimal (exact-rational) literals.
    pub rational: Color32,
    /// String literals.
    pub string: Color32,
    /// Interpolation braces inside an interpolated string.
    pub interpolation: Color32,
    /// Language keywords.
    pub keyword: Color32,
    /// Operators.
    pub operator: Color32,
    /// Declared function names.
    pub function: Color32,
    /// Engine builtin names.
    pub builtin: Color32,
    /// Brackets and structural punctuation.
    pub delimiter: Color32,
    /// Symbolic identifiers.
    pub symbol: Color32,
}

/// The dark editor palette.
pub const DARK: Palette = Palette {
    plain: Color32::from_rgb(0xd4, 0xd9, 0xe0),
    number: Color32::from_rgb(0xb5, 0xce, 0x8b),
    rational: Color32::from_rgb(0xe5, 0xc0, 0x7b),
    string: Color32::from_rgb(0xce, 0x91, 0x78),
    interpolation: Color32::from_rgb(0xf0, 0xa8, 0xd0),
    keyword: Color32::from_rgb(0xc5, 0x86, 0xc0),
    operator: Color32::from_rgb(0x56, 0xb6, 0xc2),
    function: Color32::from_rgb(0x4e, 0xc9, 0xb0),
    builtin: Color32::from_rgb(0x4f, 0xc1, 0xff),
    delimiter: Color32::from_rgb(0x8b, 0x98, 0xa9),
    symbol: Color32::from_rgb(0xc8, 0xb6, 0xff),
};

/// The context a line needs beyond its own text.
///
/// The editor refreshes one of these from the live completion catalogue and the document's
/// current source, so the highlighter never owns a keyword, builtin or symbol list that can
/// drift from the engine and the buffer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SyntaxContext {
    builtins: BTreeSet<String>,
    symbols: BTreeSet<String>,
}

impl SyntaxContext {
    /// An empty context: every identifier that is not a keyword is plain.
    pub fn new() -> SyntaxContext {
        SyntaxContext::default()
    }

    /// Builds the context from an engine snapshot: every `is_builtin` function name, plus the
    /// symbols the source declares with `symbol("name")`.
    ///
    /// The window builds its per-pass context from the completion catalogue it already has; this
    /// constructor is the direct, testable form of the same rule against an [`EngineState`].
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn from_engine_state(state: &EngineState, source: &str) -> SyntaxContext {
        let mut context = SyntaxContext::new();
        context.set_builtins(
            state
                .functions
                .iter()
                .filter(|function| function.is_builtin)
                .map(|function| function.name.as_str()),
        );
        context.set_symbols(discover_symbols(source));
        context
    }

    /// Replaces the builtin set with names from an engine-backed source.
    pub fn set_builtins<'a>(&mut self, names: impl IntoIterator<Item = &'a str>) {
        self.builtins = names.into_iter().map(str::to_string).collect();
    }

    /// Replaces the symbol set discovered from the document.
    pub fn set_symbols(&mut self, symbols: BTreeSet<String>) {
        self.symbols = symbols;
    }

    /// True when `word` is an engine builtin in this context.
    pub fn is_builtin(&self, word: &str) -> bool {
        self.builtins.contains(word)
    }

    /// True when `word` is one of the document's symbolic names.
    pub fn is_symbol(&self, word: &str) -> bool {
        self.symbols.contains(word)
    }
}

/// One classified byte range of a line.
///
/// Ranges are byte offsets into the original line. [`classify`] tiles the whole line: every
/// byte belongs to exactly one span, and adjacent spans have different classes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    /// The byte range this class covers.
    pub range: Range<usize>,
    /// The class of the covered text.
    pub class: TokenClass,
}

/// The symbol names a document declares with `symbol("name")` (or `symbol("name", domain)`).
///
/// This is a lexical discovery rule over the source, not a list: the engine's `symbol` builtin
/// takes the name as its first text argument, so the names the document asks for are exactly the
/// ones visible in that call. A `symbol` call inside a string is text and is not discovered.
pub fn discover_symbols(source: &str) -> BTreeSet<String> {
    let bytes = source.as_bytes();
    let mut names = BTreeSet::new();
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'"' {
            index = scan_string(source, index, bytes.len());
            continue;
        }
        if byte == b'$' && index + 1 < bytes.len() && bytes[index + 1] == b'"' {
            // Skip the literal; a symbol declaration inside an interpolation expression is not
            // part of this document's top-level symbol vocabulary.
            index = scan_string(source, index + 1, bytes.len());
            continue;
        }
        if is_ident_start(byte) {
            let start = index;
            index += 1;
            while index < bytes.len() && is_ident_continue(bytes[index]) {
                index += 1;
            }
            if &source[start..index] == "symbol" {
                if let Some(name) = symbol_call_name(source, index) {
                    names.insert(name);
                }
            }
            continue;
        }
        index += char_len_at(source, index);
    }
    names
}

/// Reads the first text argument of a `symbol(` call starting at `index` (just after the name).
fn symbol_call_name(source: &str, mut index: usize) -> Option<String> {
    let bytes = source.as_bytes();
    while index < bytes.len() && (bytes[index] == b' ' || bytes[index] == b'\t') {
        index += 1;
    }
    if bytes.get(index) != Some(&b'(') {
        return None;
    }
    index += 1;
    while index < bytes.len() && (bytes[index] == b' ' || bytes[index] == b'\t') {
        index += 1;
    }
    if bytes.get(index) != Some(&b'"') {
        return None;
    }
    let start = index + 1;
    index = start;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if index + 1 < bytes.len() => index += 2,
            b'"' => return Some(source[start..index].to_string()),
            _ => index += char_len_at(source, index),
        }
    }
    None
}

/// Classifies one source line into spans that tile the line.
pub fn classify(line: &str, context: &SyntaxContext) -> Vec<Span> {
    let mut spans = Vec::new();
    classify_into(line, 0, line.len(), context, &mut spans);
    spans
}

/// Builds the coloured layout job for one source line.
pub fn job(line: &str, font: FontId, palette: &Palette, context: &SyntaxContext) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY; // a code editor row never wraps
    for span in classify(line, context) {
        let text = &line[span.range];
        if text.is_empty() {
            continue;
        }
        job.append(
            text,
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: span.class.color(palette),
                ..Default::default()
            },
        );
    }
    job
}

/// Appends a span, merging it into the previous one when both cover adjacent bytes with the same
/// class. Merging keeps the layout job sections (and the test vectors) free of adjacent
/// same-class fragments.
fn push_span(spans: &mut Vec<Span>, range: Range<usize>, class: TokenClass) {
    if range.start >= range.end {
        return;
    }
    if let Some(last) = spans.last_mut() {
        if last.class == class && last.range.end == range.start {
            last.range.end = range.end;
            return;
        }
    }
    spans.push(Span { range, class });
}

/// Classifies `source[from..to]` and appends absolute spans.
///
/// `expect_function_name` is local to this segment: the identifier immediately after a `func`
/// keyword is the declared name. Whitespace does not clear it, so `func  f` still names `f`;
/// any other token does.
fn classify_into(
    source: &str,
    from: usize,
    to: usize,
    context: &SyntaxContext,
    spans: &mut Vec<Span>,
) {
    let bytes = source.as_bytes();
    let mut index = from;
    let mut expect_function_name = false;
    while index < to {
        let start = index;
        let byte = bytes[index];

        if byte == b'$' && index + 1 < to && bytes[index + 1] == b'"' {
            index = scan_interpolated(source, index, to, context, spans);
            expect_function_name = false;
            continue;
        }

        if byte == b'"' {
            index = scan_string(source, index, to);
            push_span(spans, start..index, TokenClass::String);
            expect_function_name = false;
            continue;
        }

        if byte.is_ascii_digit()
            || (byte == b'.' && index + 1 < to && bytes[index + 1].is_ascii_digit())
        {
            let (end, class) = scan_number(source, index, to);
            index = end;
            push_span(spans, start..index, class);
            expect_function_name = false;
            continue;
        }

        if is_ident_start(byte) {
            index += 1;
            while index < to && is_ident_continue(bytes[index]) {
                index += 1;
            }
            let word = &source[start..index];
            let class = if expect_function_name {
                TokenClass::Function
            } else if KEYWORDS.contains(&word) {
                TokenClass::Keyword
            } else if context.is_builtin(word) {
                TokenClass::Builtin
            } else if context.is_symbol(word) {
                TokenClass::Symbol
            } else {
                TokenClass::Plain
            };
            expect_function_name = class == TokenClass::Keyword && word == "func";
            push_span(spans, start..index, class);
            continue;
        }

        if let Some((end, class)) = scan_operator(source, index, to) {
            index = end;
            push_span(spans, start..index, class);
            expect_function_name = false;
            continue;
        }

        if is_delimiter(byte) {
            index += 1;
            push_span(spans, start..index, TokenClass::Delimiter);
            expect_function_name = false;
            continue;
        }

        // A plain run: whitespace, non-ASCII text outside a string, or a character the grammar
        // does not recognise. It stops at the next construct that starts a token.
        index += char_len_at(source, index);
        while index < to && !starts_token(source, index, to) {
            index += char_len_at(source, index);
        }
        if !source[start..index].chars().all(char::is_whitespace) {
            expect_function_name = false;
        }
        push_span(spans, start..index, TokenClass::Plain);
    }
}

/// Scans a number beginning at `start`, following the parser tokeniser's number rule.
///
/// The grammar has three numeric forms: integer, decimal (including a leading-dot decimal), and
/// a repeating decimal `0.(3)` / `0.1(6)` / `0(3)`, which is the only exact-rational literal.
fn scan_number(source: &str, start: usize, to: usize) -> (usize, TokenClass) {
    let bytes = source.as_bytes();
    let mut index = start;
    let mut rational = false;

    if bytes[index] == b'.' {
        // A leading-dot decimal: `.5`. The caller only enters here when a digit follows.
        index += 1;
        while index < to && bytes[index].is_ascii_digit() {
            index += 1;
        }
    } else {
        while index < to && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index < to && bytes[index] == b'.' {
            if index + 1 < to && bytes[index + 1] == b'(' {
                // `0.(3)`: the parenthesised period is part of the literal only when it closes.
                let end = repeating_end(source, index + 2, to);
                if let Some(end) = end {
                    index = end;
                    rational = true;
                }
            } else if index + 1 < to && bytes[index + 1].is_ascii_digit() {
                index += 1;
                while index < to && bytes[index].is_ascii_digit() {
                    index += 1;
                }
            }
        }
    }

    if !rational
        && index < to
        && bytes[index] == b'('
        && index + 1 < to
        && bytes[index + 1].is_ascii_digit()
    {
        // The postfix repeating form `0(3)` and `.5(3)`.
        if let Some(end) = repeating_end(source, index + 1, to) {
            index = end;
            rational = true;
        }
    }

    (
        index,
        if rational {
            TokenClass::Rational
        } else {
            TokenClass::Number
        },
    )
}

/// Consumes digits up to `)` and returns the index after it, or `None` when the suffix does not
/// close with a non-empty digit run.
fn repeating_end(source: &str, mut index: usize, to: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let digits_start = index;
    while index < to && bytes[index].is_ascii_digit() {
        index += 1;
    }
    if index > digits_start && index < to && bytes[index] == b')' {
        Some(index + 1)
    } else {
        None
    }
}

/// Scans a plain string beginning at the opening quote and returns the index after its close (or
/// `to` when unterminated).
fn scan_string(source: &str, quote: usize, to: usize) -> usize {
    let bytes = source.as_bytes();
    let mut index = quote + 1;
    while index < to {
        match bytes[index] {
            b'\\' if index + 1 < to => index += 2,
            b'"' => return index + 1,
            _ => index += char_len_at(source, index),
        }
    }
    to
}

/// Scans a `$"..."` literal beginning at `start`, appending string, interpolation and expression
/// spans, and returns the index after its close (or `to` when unterminated).
fn scan_interpolated(
    source: &str,
    start: usize,
    to: usize,
    context: &SyntaxContext,
    spans: &mut Vec<Span>,
) -> usize {
    let bytes = source.as_bytes();
    let mut index = start + 2; // skip `$"`
    let mut segment_start = start;
    while index < to {
        let byte = bytes[index];
        if byte == b'{' {
            if index + 1 < to && bytes[index + 1] == b'{' {
                index += 2; // `{{` is literal text
                continue;
            }
            push_span(spans, segment_start..index, TokenClass::String);
            push_span(spans, index..index + 1, TokenClass::Interpolation);
            let expression_start = index + 1;
            let mut depth = 1usize;
            let mut close = expression_start;
            while close < to {
                match bytes[close] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                close += char_len_at(source, close);
            }
            if close < to {
                classify_into(source, expression_start, close, context, spans);
                push_span(spans, close..close + 1, TokenClass::Interpolation);
                index = close + 1;
                segment_start = index;
            } else {
                // Unterminated interpolation: the rest is still string text; the parser reports
                // the real error, and the highlighter does not invent tokens past it.
                push_span(spans, expression_start..to, TokenClass::String);
                return to;
            }
            continue;
        }
        if byte == b'}' && index + 1 < to && bytes[index + 1] == b'}' {
            index += 2; // `}}` is literal text
            continue;
        }
        if byte == b'"' {
            index += 1;
            push_span(spans, segment_start..index, TokenClass::String);
            return index;
        }
        if byte == b'\\' && index + 1 < to {
            index += 2;
            continue;
        }
        index += char_len_at(source, index);
    }
    push_span(spans, segment_start..to, TokenClass::String);
    to
}

/// Recognises the parser's two- and one-character operators.
fn scan_operator(source: &str, index: usize, to: usize) -> Option<(usize, TokenClass)> {
    let bytes = source.as_bytes();
    if index + 1 < to && bytes[index + 1] < 0x80 {
        let pair = (bytes[index], bytes[index + 1]);
        if matches!(
            pair,
            (b'=', b'=') | (b'!', b'=') | (b'<', b'=') | (b'>', b'=') | (b'.', b'.')
        ) {
            return Some((index + 2, TokenClass::Operator));
        }
    }
    if matches!(
        bytes[index],
        b'+' | b'-' | b'*' | b'/' | b'%' | b'^' | b'!' | b'=' | b'<' | b'>'
    ) {
        return Some((index + 1, TokenClass::Operator));
    }
    None
}

/// True for the parser's structural punctuators.
fn is_delimiter(byte: u8) -> bool {
    matches!(
        byte,
        b'(' | b')' | b'[' | b']' | b'{' | b'}' | b',' | b';' | b':' | b'.'
    )
}

/// True when a token construct starts at `index`, used to end a plain run.
fn starts_token(source: &str, index: usize, to: usize) -> bool {
    let bytes = source.as_bytes();
    let byte = bytes[index];
    if byte == b'"' {
        return true;
    }
    if byte == b'$' && index + 1 < to && bytes[index + 1] == b'"' {
        return true;
    }
    if byte.is_ascii_digit() {
        return true;
    }
    if byte == b'.' && index + 1 < to && bytes[index + 1].is_ascii_digit() {
        return true;
    }
    if is_ident_start(byte) {
        return true;
    }
    if scan_operator(source, index, to).is_some() {
        return true;
    }
    is_delimiter(byte)
}

fn char_len_at(source: &str, index: usize) -> usize {
    source[index..]
        .chars()
        .next()
        .map(|character| character.len_utf8())
        .unwrap_or(1)
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Range;

    use crate::engine::{EngineFunction, EngineState};

    /// A context whose builtin names come from a synthetic engine snapshot; `source` supplies the
    /// `symbol("...")` declarations exactly as a document would.
    fn context(builtins: &[&str], source: &str) -> SyntaxContext {
        let state = EngineState {
            variables: Vec::new(),
            functions: builtins
                .iter()
                .map(|name| EngineFunction {
                    name: (*name).to_string(),
                    parameters: vec!["x".to_string()],
                    is_builtin: true,
                })
                .collect(),
        };
        SyntaxContext::from_engine_state(&state, source)
    }

    fn significant(line: &str, context: &SyntaxContext) -> Vec<(Range<usize>, TokenClass)> {
        classify(line, context)
            .into_iter()
            .filter(|span| span.class != TokenClass::Plain)
            .map(|span| (span.range, span.class))
            .collect()
    }

    /// Asserts the line is tiled from byte 0 to its length with no gaps and no overlaps, then
    /// checks the exact (range, class) pairs of every non-plain span.
    fn assert_exact(line: &str, context: &SyntaxContext, expected: &[(Range<usize>, TokenClass)]) {
        let spans = classify(line, context);
        assert_eq!(
            spans.first().map(|span| span.range.start),
            Some(0),
            "spans must start at byte 0: {line:?}"
        );
        assert_eq!(
            spans.last().map(|span| span.range.end),
            Some(line.len()),
            "spans must end at the line length: {line:?}"
        );
        for pair in spans.windows(2) {
            assert_eq!(
                pair[0].range.end, pair[1].range.start,
                "spans must tile the line without gaps: {line:?}"
            );
        }
        assert_eq!(significant(line, context), expected, "line {line:?}");
    }

    #[test]
    fn integer_and_decimal_literals_are_one_number_class() {
        let ctx = context(&[], "");
        assert_exact(
            "12 3.25",
            &ctx,
            &[(0..2, TokenClass::Number), (3..7, TokenClass::Number)],
        );
        // A leading-dot decimal is one literal, not a delimiter followed by a number.
        assert_exact(
            ".5 + 1",
            &ctx,
            &[
                (0..2, TokenClass::Number),
                (3..4, TokenClass::Operator),
                (5..6, TokenClass::Number),
            ],
        );
    }

    #[test]
    fn repeating_decimals_are_the_grammars_only_exact_rational_literal() {
        let ctx = context(&[], "");
        // `0.(3)` is one Repr token in the parser; `1/3` is Number, Operator, Number. The grammar
        // has no slash-literal, so only the repeating form may be painted Rational.
        assert_exact(
            "0.(3) + 1/3",
            &ctx,
            &[
                (0..5, TokenClass::Rational),
                (6..7, TokenClass::Operator),
                (8..9, TokenClass::Number),
                (9..10, TokenClass::Operator),
                (10..11, TokenClass::Number),
            ],
        );
        assert_exact("0.1(6)", &ctx, &[(0..6, TokenClass::Rational)]);
        assert_exact("0(3)", &ctx, &[(0..4, TokenClass::Rational)]);
        assert_exact("12(34)", &ctx, &[(0..6, TokenClass::Rational)]);
    }

    #[test]
    fn plain_strings_include_quotes_and_escaped_quotes() {
        let ctx = context(&[], "");
        assert_exact(r#""a\"b""#, &ctx, &[(0..6, TokenClass::String)]);
        assert_exact("\"abc", &ctx, &[(0..4, TokenClass::String)]);
        // A `#` inside a string is string text, not a comment (the grammar has no comments).
        assert_exact("\"a#b\"", &ctx, &[(0..5, TokenClass::String)]);
    }

    #[test]
    fn interpolation_braces_and_their_expression_are_classified_separately() {
        let ctx = context(&["symbol", "sqrt"], "symbol(\"x\")");
        assert_exact(
            r#"$"x={x + 1}""#,
            &ctx,
            &[
                (0..4, TokenClass::String),
                (4..5, TokenClass::Interpolation),
                (5..6, TokenClass::Symbol),
                (7..8, TokenClass::Operator),
                (9..10, TokenClass::Number),
                (10..11, TokenClass::Interpolation),
                (11..12, TokenClass::String),
            ],
        );
        // `{{` and `}}` are literal text, not interpolation delimiters.
        assert_exact(r#"$"{{x}}""#, &ctx, &[(0..8, TokenClass::String)]);
        // The expression inside the interpolation uses the normal classes and the engine's
        // builtin set.
        assert_exact(
            r#"$"{sqrt(4)}""#,
            &ctx,
            &[
                (0..2, TokenClass::String),
                (2..3, TokenClass::Interpolation),
                (3..7, TokenClass::Builtin),
                (7..8, TokenClass::Delimiter),
                (8..9, TokenClass::Number),
                (9..10, TokenClass::Delimiter),
                (10..11, TokenClass::Interpolation),
                (11..12, TokenClass::String),
            ],
        );
    }

    #[test]
    fn every_shared_keyword_is_painted_as_a_keyword() {
        let ctx = context(&[], "");
        // The test iterates the shared table, so a keyword added to the engine/table without a
        // highlighter change fails here instead of silently becoming plain text.
        for keyword in crate::language::KEYWORDS {
            assert_exact(keyword, &ctx, &[(0..keyword.len(), TokenClass::Keyword)]);
        }
        assert_exact(
            "if else for in return break continue while",
            &ctx,
            &[
                (0..2, TokenClass::Keyword),
                (3..7, TokenClass::Keyword),
                (8..11, TokenClass::Keyword),
                (12..14, TokenClass::Keyword),
                (15..21, TokenClass::Keyword),
                (22..27, TokenClass::Keyword),
                (28..36, TokenClass::Keyword),
                (37..42, TokenClass::Keyword),
            ],
        );
        assert_exact("func", &ctx, &[(0..4, TokenClass::Keyword)]);
    }

    #[test]
    fn the_declared_function_name_is_a_function_not_a_builtin() {
        let ctx = context(&["sqrt"], "");
        // `sqrt` is a builtin, but in the declaration position it is the declared name.
        assert_exact(
            "func sqrt(x) = x",
            &ctx,
            &[
                (0..4, TokenClass::Keyword),
                (5..9, TokenClass::Function),
                (9..10, TokenClass::Delimiter),
                (11..12, TokenClass::Delimiter),
                (13..14, TokenClass::Operator),
            ],
        );
        // Only the identifier immediately after `func` is declared; the next one is plain.
        assert_exact(
            "func f g",
            &ctx,
            &[(0..4, TokenClass::Keyword), (5..6, TokenClass::Function)],
        );
    }

    #[test]
    fn builtins_come_from_the_engine_snapshot_not_a_desktop_list() {
        let ctx = context(&["sqrt", "squeeze"], "");
        assert!(ctx.is_builtin("sqrt"));
        assert!(ctx.is_builtin("squeeze"));
        // These were in the old hardcoded list but are not in the engine registry; a supplied
        // snapshot must not resurrect them.
        assert!(!ctx.is_builtin("round"));
        assert!(!ctx.is_builtin("matrix"));
        assert!(!ctx.is_builtin("product"));
        assert!(!ctx.is_builtin("eval"));
        assert_exact(
            "sqrt(4) + squeeze(a) + round(b)",
            &ctx,
            &[
                (0..4, TokenClass::Builtin),
                (4..5, TokenClass::Delimiter),
                (5..6, TokenClass::Number),
                (6..7, TokenClass::Delimiter),
                (8..9, TokenClass::Operator),
                (10..17, TokenClass::Builtin),
                (17..18, TokenClass::Delimiter),
                (19..20, TokenClass::Delimiter),
                (21..22, TokenClass::Operator),
                (28..29, TokenClass::Delimiter),
                (30..31, TokenClass::Delimiter),
            ],
        );
    }

    #[test]
    fn user_functions_are_not_builtins() {
        let state = EngineState {
            variables: Vec::new(),
            functions: vec![EngineFunction {
                name: "myfunc".to_string(),
                parameters: vec!["a".to_string()],
                is_builtin: false,
            }],
        };
        let ctx = SyntaxContext::from_engine_state(&state, "");
        assert!(!ctx.is_builtin("myfunc"));
        assert_exact(
            "myfunc(1)",
            &ctx,
            &[
                (6..7, TokenClass::Delimiter),
                (7..8, TokenClass::Number),
                (8..9, TokenClass::Delimiter),
            ],
        );
    }

    #[test]
    fn operators_and_delimiters_use_the_parser_punctuator_set() {
        let ctx = context(&[], "");
        assert_exact(
            "a = (b + 1) * [2] / 3 - -4 % 5 ^ 2 == 1 != 0 < 2 <= 3 > 0 >= 1 .. 5",
            &ctx,
            &[
                (2..3, TokenClass::Operator),
                (4..5, TokenClass::Delimiter),
                (7..8, TokenClass::Operator),
                (9..10, TokenClass::Number),
                (10..11, TokenClass::Delimiter),
                (12..13, TokenClass::Operator),
                (14..15, TokenClass::Delimiter),
                (15..16, TokenClass::Number),
                (16..17, TokenClass::Delimiter),
                (18..19, TokenClass::Operator),
                (20..21, TokenClass::Number),
                (22..23, TokenClass::Operator),
                (24..25, TokenClass::Operator),
                (25..26, TokenClass::Number),
                (27..28, TokenClass::Operator),
                (29..30, TokenClass::Number),
                (31..32, TokenClass::Operator),
                (33..34, TokenClass::Number),
                (35..37, TokenClass::Operator),
                (38..39, TokenClass::Number),
                (40..42, TokenClass::Operator),
                (43..44, TokenClass::Number),
                (45..46, TokenClass::Operator),
                (47..48, TokenClass::Number),
                (49..51, TokenClass::Operator),
                (52..53, TokenClass::Number),
                (54..55, TokenClass::Operator),
                (56..57, TokenClass::Number),
                (58..60, TokenClass::Operator),
                (61..62, TokenClass::Number),
                (63..65, TokenClass::Operator),
                (66..67, TokenClass::Number),
            ],
        );
    }

    #[test]
    fn symbol_calls_declare_the_symbolic_tokens_used_in_expressions() {
        let source = "t = symbol(\"t\")\nu = symbol(\"u\", real)\nt + u + \"t\" + v";
        let ctx = context(&["symbol"], source);
        assert!(ctx.is_symbol("t"));
        assert!(ctx.is_symbol("u"));
        assert!(!ctx.is_symbol("v"));
        assert_exact(
            "t = symbol(\"t\")",
            &ctx,
            &[
                (0..1, TokenClass::Symbol),
                (2..3, TokenClass::Operator),
                (4..10, TokenClass::Builtin),
                (10..11, TokenClass::Delimiter),
                (11..14, TokenClass::String),
                (14..15, TokenClass::Delimiter),
            ],
        );
        assert_exact(
            "t + u + \"t\" + v",
            &ctx,
            &[
                (0..1, TokenClass::Symbol),
                (2..3, TokenClass::Operator),
                (4..5, TokenClass::Symbol),
                (6..7, TokenClass::Operator),
                (8..11, TokenClass::String),
                (12..13, TokenClass::Operator),
            ],
        );
    }

    #[test]
    fn layout_job_sections_are_the_classified_spans_in_order() {
        let ctx = context(&["sqrt"], "");
        let line = "func f(x) = sqrt(1/3)";
        let job = job(line, FontId::monospace(13.0), &DARK, &ctx);
        let spans = classify(line, &ctx);
        assert_eq!(job.sections.len(), spans.len());
        for (section, span) in job.sections.iter().zip(spans.iter()) {
            assert_eq!(
                section.byte_range.start.0, span.range.start,
                "section start must match the classified span"
            );
            assert_eq!(
                section.byte_range.end.0, span.range.end,
                "section end must match the classified span"
            );
            assert_eq!(section.format.font_id, FontId::monospace(13.0));
            assert_eq!(section.format.color, span.class.color(&DARK));
            assert_eq!(
                &line[section.byte_range.start.0..section.byte_range.end.0],
                &line[span.range.clone()]
            );
        }
    }

    /// The engine's registry, not a desktop copy, is the builtin authority. Every name the real
    /// engine publishes as a builtin must classify as exactly one Builtin span; a name the old
    /// hardcoded list invented must not.
    #[test]
    #[cfg(feature = "real-engine")]
    fn every_real_engine_registry_builtin_is_painted_as_builtin() {
        use crate::engine::{Evaluator, RealEngine};

        let state = RealEngine::new()
            .state()
            .expect("the real engine publishes a state");
        let context = SyntaxContext::from_engine_state(&state, "");
        let mut checked = 0usize;
        for function in state
            .functions
            .iter()
            .filter(|function| function.is_builtin)
        {
            let classes: Vec<TokenClass> = classify(&function.name, &context)
                .into_iter()
                .map(|span| span.class)
                .collect();
            assert_eq!(
                classes,
                vec![TokenClass::Builtin],
                "{} is published as a builtin and must be painted as one",
                function.name
            );
            checked += 1;
        }
        assert!(
            checked > 100,
            "the registry published only {checked} builtins"
        );
        assert!(
            context.is_builtin("squeeze"),
            "squeeze is in the engine registry"
        );
        assert!(
            !context.is_builtin("round"),
            "round is not in the engine registry"
        );
    }
}
