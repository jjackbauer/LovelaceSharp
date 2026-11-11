//! The reference lexer surface: [`TokenKind`], [`Token`] and [`Tokenizer`].
//!
//! This module is the Rust equivalent of the C# `Lovelace.Suite` lexer vocabulary
//! (`TokenKind`, `Token` and `Tokenizer.Tokenize`). It is additive: the parser keeps its own
//! [`crate::syntax::Tok`] vocabulary and none of its behaviour changes.
//!
//! The classification rules are the reference `Tokenizer.Tokenize` rules, not the script
//! normaliser's: every whitespace character (including a newline) separates tokens and is
//! otherwise dropped, and a numeric literal keeps its optional periodic `(digits)` suffix as
//! part of the same token.

/// Identifies the lexical category of a [`Token`].
///
/// The discriminants are the reference ordinals: the first twenty members (0–19) are the
/// original REPL tokens and the v1 additions are appended after [`TokenKind::Eof`] (20–29), so
/// the published ordinals stay stable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i32)]
pub enum TokenKind {
    /// An integer, decimal or periodic numeric literal.
    NumberLiteral = 0,
    /// An identifier: `[a-zA-Z_][a-zA-Z0-9_]*`.
    Identifier = 1,
    /// `+`
    Plus = 2,
    /// `-`
    Minus = 3,
    /// `*`
    Star = 4,
    /// `/`
    Slash = 5,
    /// `%`
    Percent = 6,
    /// `^`
    Caret = 7,
    /// `!`
    Bang = 8,
    /// `=`
    Equals = 9,
    /// `==`
    DoubleEquals = 10,
    /// `!=`
    BangEquals = 11,
    /// `>`
    Greater = 12,
    /// `<`
    Less = 13,
    /// `>=`
    GreaterEquals = 14,
    /// `<=`
    LessEquals = 15,
    /// `(`
    LParen = 16,
    /// `)`
    RParen = 17,
    /// `,`
    Comma = 18,
    /// End of input.
    Eof = 19,
    /// `..` range operator.
    DotDot = 20,
    /// `{`
    LBrace = 21,
    /// `}`
    RBrace = 22,
    /// `[`
    LBracket = 23,
    /// `]`
    RBracket = 24,
    /// A plain string literal; the text is the content between the quotes.
    StringLiteral = 25,
    /// An interpolated string literal; the text is the content between the quotes.
    InterpolatedString = 26,
    /// `;`
    Semicolon = 27,
    /// `:`
    Colon = 28,
    /// `.` record member access.
    Dot = 29,
}

impl TokenKind {
    /// Every variant, in discriminant order.
    pub const ALL: [TokenKind; 30] = [
        TokenKind::NumberLiteral,
        TokenKind::Identifier,
        TokenKind::Plus,
        TokenKind::Minus,
        TokenKind::Star,
        TokenKind::Slash,
        TokenKind::Percent,
        TokenKind::Caret,
        TokenKind::Bang,
        TokenKind::Equals,
        TokenKind::DoubleEquals,
        TokenKind::BangEquals,
        TokenKind::Greater,
        TokenKind::Less,
        TokenKind::GreaterEquals,
        TokenKind::LessEquals,
        TokenKind::LParen,
        TokenKind::RParen,
        TokenKind::Comma,
        TokenKind::Eof,
        TokenKind::DotDot,
        TokenKind::LBrace,
        TokenKind::RBrace,
        TokenKind::LBracket,
        TokenKind::RBracket,
        TokenKind::StringLiteral,
        TokenKind::InterpolatedString,
        TokenKind::Semicolon,
        TokenKind::Colon,
        TokenKind::Dot,
    ];

    /// The number of variants the reference enum declares.
    pub const COUNT: usize = 30;

    /// The reference ordinal: the value of a C# `(int)kind` cast.
    #[must_use]
    pub const fn ordinal(self) -> i32 {
        self as i32
    }

    /// The reference member name, as the C# enum's `ToString` publishes it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            TokenKind::NumberLiteral => "NumberLiteral",
            TokenKind::Identifier => "Identifier",
            TokenKind::Plus => "Plus",
            TokenKind::Minus => "Minus",
            TokenKind::Star => "Star",
            TokenKind::Slash => "Slash",
            TokenKind::Percent => "Percent",
            TokenKind::Caret => "Caret",
            TokenKind::Bang => "Bang",
            TokenKind::Equals => "Equals",
            TokenKind::DoubleEquals => "DoubleEquals",
            TokenKind::BangEquals => "BangEquals",
            TokenKind::Greater => "Greater",
            TokenKind::Less => "Less",
            TokenKind::GreaterEquals => "GreaterEquals",
            TokenKind::LessEquals => "LessEquals",
            TokenKind::LParen => "LParen",
            TokenKind::RParen => "RParen",
            TokenKind::Comma => "Comma",
            TokenKind::Eof => "Eof",
            TokenKind::DotDot => "DotDot",
            TokenKind::LBrace => "LBrace",
            TokenKind::RBrace => "RBrace",
            TokenKind::LBracket => "LBracket",
            TokenKind::RBracket => "RBracket",
            TokenKind::StringLiteral => "StringLiteral",
            TokenKind::InterpolatedString => "InterpolatedString",
            TokenKind::Semicolon => "Semicolon",
            TokenKind::Colon => "Colon",
            TokenKind::Dot => "Dot",
        }
    }

    /// The variant with the given reference ordinal, or `None` when the ordinal is not declared.
    #[must_use]
    pub const fn from_ordinal(ordinal: i32) -> Option<TokenKind> {
        match ordinal {
            0 => Some(TokenKind::NumberLiteral),
            1 => Some(TokenKind::Identifier),
            2 => Some(TokenKind::Plus),
            3 => Some(TokenKind::Minus),
            4 => Some(TokenKind::Star),
            5 => Some(TokenKind::Slash),
            6 => Some(TokenKind::Percent),
            7 => Some(TokenKind::Caret),
            8 => Some(TokenKind::Bang),
            9 => Some(TokenKind::Equals),
            10 => Some(TokenKind::DoubleEquals),
            11 => Some(TokenKind::BangEquals),
            12 => Some(TokenKind::Greater),
            13 => Some(TokenKind::Less),
            14 => Some(TokenKind::GreaterEquals),
            15 => Some(TokenKind::LessEquals),
            16 => Some(TokenKind::LParen),
            17 => Some(TokenKind::RParen),
            18 => Some(TokenKind::Comma),
            19 => Some(TokenKind::Eof),
            20 => Some(TokenKind::DotDot),
            21 => Some(TokenKind::LBrace),
            22 => Some(TokenKind::RBrace),
            23 => Some(TokenKind::LBracket),
            24 => Some(TokenKind::RBracket),
            25 => Some(TokenKind::StringLiteral),
            26 => Some(TokenKind::InterpolatedString),
            27 => Some(TokenKind::Semicolon),
            28 => Some(TokenKind::Colon),
            29 => Some(TokenKind::Dot),
            _ => None,
        }
    }
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One lexical token, the Rust equivalent of the C# `Token` record.
///
/// Structural equality covers all three fields, exactly as the reference record's equality does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    /// The lexical category of this token.
    pub kind: TokenKind,
    /// The raw source text. For [`TokenKind::StringLiteral`] and
    /// [`TokenKind::InterpolatedString`] this is the content between the quotes (without the
    /// quotes); for [`TokenKind::Eof`] it is empty.
    pub text: String,
    /// Zero-based UTF-16 code-unit index of the token in the original input.
    pub position: usize,
}

/// A tokenizer refusal.
///
/// The reference throws `InvalidOperationException` for an unrecognised character and
/// `FormatException` for an unterminated string. Rust publishes both as this one value: the
/// message is the reference sentence and `position` is its typed copy of the offset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexError {
    /// The complete refusal sentence, the same text the reference exception carries.
    pub message: String,
    /// Zero-based UTF-16 code-unit offset the failure refers to.
    pub position: usize,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LexError {}

/// The reference tokenizer facade: `new Tokenizer().Tokenize(input)` in C#.
#[derive(Clone, Copy, Debug, Default)]
pub struct Tokenizer;

impl Tokenizer {
    /// Scans `input` and returns the complete token list, always terminated by
    /// [`TokenKind::Eof`].
    ///
    /// # Errors
    ///
    /// Returns a [`LexError`] when the input contains an unrecognised character or an
    /// unterminated string.
    pub fn tokenize(&self, input: &str) -> Result<Vec<Token>, LexError> {
        tokenize(input)
    }
}

/// Scans `input` into tokens, always terminated by one [`TokenKind::Eof`] token.
///
/// The scan reproduces the reference `Tokenizer.Tokenize` order: interpolated strings before
/// plain strings, numbers before punctuation, two-character operators before their one-character
/// prefixes, then single characters. Whitespace is skipped without producing a token.
///
/// # Errors
///
/// Returns a [`LexError`] when the input contains an unrecognised character or an
/// unterminated string.
pub fn tokenize(input: &str) -> Result<Vec<Token>, LexError> {
    let chars: Vec<char> = input.chars().collect();
    let mut offsets: Vec<usize> = Vec::with_capacity(chars.len());
    let mut unit = 0usize;
    for c in &chars {
        offsets.push(unit);
        unit += c.len_utf16();
    }
    let end = unit;

    let mut tokens = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        // Interpolated string `$"..."` must be checked before the plain quote.
        if c == '$' && i + 1 < chars.len() && chars[i + 1] == '"' {
            tokens.push(scan_quoted_string(
                &chars,
                &offsets,
                &mut i,
                TokenKind::InterpolatedString,
                2,
            )?);
            continue;
        }

        if c == '"' {
            tokens.push(scan_quoted_string(
                &chars,
                &offsets,
                &mut i,
                TokenKind::StringLiteral,
                1,
            )?);
            continue;
        }

        // A number starts with a digit, or with a '.' that is followed by a digit.
        if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
            tokens.push(scan_number(&chars, &offsets, &mut i));
            continue;
        }

        if c.is_ascii_alphabetic() || c == '_' {
            tokens.push(scan_identifier(&chars, &offsets, &mut i));
            continue;
        }

        // Two-character operators, before their single-character prefixes.
        if i + 1 < chars.len() {
            let two_char = match (chars[i], chars[i + 1]) {
                ('=', '=') => Some(TokenKind::DoubleEquals),
                ('!', '=') => Some(TokenKind::BangEquals),
                ('>', '=') => Some(TokenKind::GreaterEquals),
                ('<', '=') => Some(TokenKind::LessEquals),
                ('.', '.') => Some(TokenKind::DotDot),
                _ => None,
            };
            if let Some(kind) = two_char {
                tokens.push(Token {
                    kind,
                    text: chars[i..i + 2].iter().collect(),
                    position: offsets[i],
                });
                i += 2;
                continue;
            }
        }

        // Single-character operators and punctuation.
        let one_char = match c {
            '+' => Some(TokenKind::Plus),
            '-' => Some(TokenKind::Minus),
            '*' => Some(TokenKind::Star),
            '/' => Some(TokenKind::Slash),
            '%' => Some(TokenKind::Percent),
            '^' => Some(TokenKind::Caret),
            '!' => Some(TokenKind::Bang),
            '=' => Some(TokenKind::Equals),
            '>' => Some(TokenKind::Greater),
            '<' => Some(TokenKind::Less),
            '(' => Some(TokenKind::LParen),
            ')' => Some(TokenKind::RParen),
            ',' => Some(TokenKind::Comma),
            '{' => Some(TokenKind::LBrace),
            '}' => Some(TokenKind::RBrace),
            '[' => Some(TokenKind::LBracket),
            ']' => Some(TokenKind::RBracket),
            ';' => Some(TokenKind::Semicolon),
            ':' => Some(TokenKind::Colon),
            '.' => Some(TokenKind::Dot),
            _ => None,
        };
        if let Some(kind) = one_char {
            tokens.push(Token {
                kind,
                text: c.to_string(),
                position: offsets[i],
            });
            i += 1;
            continue;
        }

        return Err(LexError {
            message: format!("Unexpected character '{c}' at position {}.", offsets[i]),
            position: offsets[i],
        });
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        text: String::new(),
        position: end,
    });
    Ok(tokens)
}

/// Scans a quoted string. `skip` is the number of leading characters to consume: 1 for `"`,
/// 2 for `$"`.
fn scan_quoted_string(
    chars: &[char],
    offsets: &[usize],
    pos: &mut usize,
    kind: TokenKind,
    skip: usize,
) -> Result<Token, LexError> {
    let start = *pos;
    *pos += skip;
    let content_start = *pos;

    while *pos < chars.len() && chars[*pos] != '"' {
        *pos += 1;
    }
    if *pos >= chars.len() {
        return Err(LexError {
            message: format!(
                "Unterminated string literal at position {}.",
                offsets[start]
            ),
            position: offsets[start],
        });
    }

    let text: String = chars[content_start..*pos].iter().collect();
    *pos += 1;
    Ok(Token {
        kind,
        text,
        position: offsets[start],
    })
}

/// Scans the complete numeric literal beginning at `pos`: digits, an optional decimal part, and
/// an optional periodic `(digits)` suffix, captured verbatim.
fn scan_number(chars: &[char], offsets: &[usize], pos: &mut usize) -> Token {
    let start = *pos;

    while *pos < chars.len() && chars[*pos].is_ascii_digit() {
        *pos += 1;
    }

    // '.' followed by optional digits, but not when the next character is also '.' (the range
    // operator).
    if *pos < chars.len()
        && chars[*pos] == '.'
        && !(*pos + 1 < chars.len() && chars[*pos + 1] == '.')
    {
        *pos += 1;
        while *pos < chars.len() && chars[*pos].is_ascii_digit() {
            *pos += 1;
        }
    }

    // Optional periodic suffix '(' digits ')'. The reference consumes the opening parenthesis
    // even when no closing parenthesis follows, so the text remains verbatim.
    if *pos < chars.len() && chars[*pos] == '(' {
        *pos += 1;
        while *pos < chars.len() && chars[*pos].is_ascii_digit() {
            *pos += 1;
        }
        if *pos < chars.len() && chars[*pos] == ')' {
            *pos += 1;
        }
    }

    Token {
        kind: TokenKind::NumberLiteral,
        text: chars[start..*pos].iter().collect(),
        position: offsets[start],
    }
}

/// Scans the identifier beginning at `pos`: `[a-zA-Z_][a-zA-Z0-9_]*`.
fn scan_identifier(chars: &[char], offsets: &[usize], pos: &mut usize) -> Token {
    let start = *pos;
    while *pos < chars.len() && (chars[*pos].is_ascii_alphanumeric() || chars[*pos] == '_') {
        *pos += 1;
    }
    Token {
        kind: TokenKind::Identifier,
        text: chars[start..*pos].iter().collect(),
        position: offsets[start],
    }
}
