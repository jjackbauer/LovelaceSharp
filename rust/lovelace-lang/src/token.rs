//! Script normalization, tokenizer, and token definitions.

/// One lexical token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    /// The lexical category.
    pub kind: TokenKind,
    /// The raw token text (string content for string tokens).
    pub text: String,
    /// Zero-based character offset into the normalized source.
    pub position: usize,
}

/// The lexical categories produced by [`tokenize`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// A numeric literal.
    Number,
    /// An identifier.
    Ident,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `^`
    Caret,
    /// `!`
    Bang,
    /// `=`
    Equals,
    /// `==`
    DoubleEquals,
    /// `!=`
    BangEquals,
    /// `>`
    Greater,
    /// `<`
    Less,
    /// `>=`
    GreaterEquals,
    /// `<=`
    LessEquals,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `,`
    Comma,
    /// `..`
    DotDot,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// A plain string literal.
    StringLiteral,
    /// An interpolated string literal.
    InterpolatedString,
    /// `;`
    Semicolon,
    /// `:`
    Colon,
    /// `.`
    Dot,
    /// End of input.
    Eof,
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Normalizes script text for the program grammar.
///
/// The language's program grammar is semicolon-separated while scripts are written
/// with newlines, so top-level newlines become semicolons. Newlines inside brackets,
/// braces, parentheses or strings are left alone. The rewrite is length-preserving.
pub fn normalize_source(source: &str) -> String {
    let text = source.replace("\r\n", "\n").replace('\r', "\n");
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);

    let mut out = String::with_capacity(text.len());
    let mut depth: i32 = 0;
    let mut in_string = false;
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
                if depth > 0 {
                    depth -= 1;
                }
                out.push(c);
                last = Some(c);
            }
            '\n' => {
                if depth == 0 {
                    let replacement = if last.is_none() || last == Some(';') { ' ' } else { ';' };
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

/// Tokenizes a normalized script.
///
/// # Errors
///
/// Returns a human-readable message when the input contains an unexpected
/// character or an unterminated string.
pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut pos = 0usize;

    while pos < chars.len() {
        let c = chars[pos];
        if c.is_whitespace() {
            pos += 1;
            continue;
        }

        if c == '$' && pos + 1 < chars.len() && chars[pos + 1] == '"' {
            let token = scan_string(&chars, &mut pos, TokenKind::InterpolatedString, 2)?;
            tokens.push(token);
            continue;
        }

        if c == '"' {
            let token = scan_string(&chars, &mut pos, TokenKind::StringLiteral, 1)?;
            tokens.push(token);
            continue;
        }

        if c.is_ascii_digit() || (c == '.' && pos + 1 < chars.len() && chars[pos + 1].is_ascii_digit()) {
            let start = pos;
            while pos < chars.len() && chars[pos].is_ascii_digit() {
                pos += 1;
            }
            if pos < chars.len()
                && chars[pos] == '.'
                && !(pos + 1 < chars.len() && chars[pos + 1] == '.')
            {
                pos += 1;
                while pos < chars.len() && chars[pos].is_ascii_digit() {
                    pos += 1;
                }
            }
            if pos < chars.len() && chars[pos] == '(' {
                pos += 1;
                while pos < chars.len() && chars[pos].is_ascii_digit() {
                    pos += 1;
                }
                if pos < chars.len() && chars[pos] == ')' {
                    pos += 1;
                }
            }
            let text: String = chars[start..pos].iter().collect();
            tokens.push(Token { kind: TokenKind::Number, text, position: start });
            continue;
        }

        if c.is_ascii_alphabetic() || c == '_' {
            let start = pos;
            while pos < chars.len() && (chars[pos].is_ascii_alphanumeric() || chars[pos] == '_') {
                pos += 1;
            }
            let text: String = chars[start..pos].iter().collect();
            tokens.push(Token { kind: TokenKind::Ident, text, position: start });
            continue;
        }

        if pos + 1 < chars.len() {
            let two = (chars[pos], chars[pos + 1]);
            let kind = match two {
                ('=', '=') => Some(TokenKind::DoubleEquals),
                ('!', '=') => Some(TokenKind::BangEquals),
                ('>', '=') => Some(TokenKind::GreaterEquals),
                ('<', '=') => Some(TokenKind::LessEquals),
                ('.', '.') => Some(TokenKind::DotDot),
                _ => None,
            };
            if let Some(kind) = kind {
                let text: String = chars[pos..pos + 2].iter().collect();
                tokens.push(Token { kind, text, position: pos });
                pos += 2;
                continue;
            }
        }

        let kind = match c {
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
        if let Some(kind) = kind {
            tokens.push(Token { kind, text: c.to_string(), position: pos });
            pos += 1;
            continue;
        }

        return Err(format!("Unexpected character '{c}' at position {pos}."));
    }

    tokens.push(Token { kind: TokenKind::Eof, text: String::new(), position: pos });
    Ok(tokens)
}

fn scan_string(
    chars: &[char],
    pos: &mut usize,
    kind: TokenKind,
    skip: usize,
) -> Result<Token, String> {
    let start = *pos;
    *pos += skip;
    let content_start = *pos;
    while *pos < chars.len() && chars[*pos] != '"' {
        *pos += 1;
    }
    if *pos >= chars.len() {
        return Err(format!("Unterminated string literal at position {start}."));
    }
    let text: String = chars[content_start..*pos].iter().collect();
    *pos += 1;
    Ok(Token { kind, text, position: start })
}
