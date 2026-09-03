namespace Lovelace.Suite;

// -------------------------------------------------------------------------
// TokenKind — discriminates the lexical token types
// -------------------------------------------------------------------------

/// <summary>
/// Identifies the lexical category of a <see cref="Token"/> produced by the
/// <see cref="Tokenizer"/>. The first twenty members (0–19) are the original
/// REPL tokens and must retain their ordinals; v1 additions are appended after
/// <see cref="Eof"/> so existing ordinal assertions stay valid.
/// </summary>
public enum TokenKind
{
    // Literals and names
    NumberLiteral,   // 0  — integer, decimal, or periodic literal
    Identifier,      // 1  — [a-zA-Z_][a-zA-Z0-9_]*

    // Arithmetic operators
    Plus,            // 2  — +
    Minus,           // 3  — -
    Star,            // 4  — *
    Slash,           // 5  — /
    Percent,         // 6  — %
    Caret,           // 7  — ^
    Bang,            // 8  — !

    // Assignment / equality
    Equals,          // 9  — =
    DoubleEquals,    // 10 — ==
    BangEquals,      // 11 — !=

    // Relational
    Greater,         // 12 — >
    Less,            // 13 — <
    GreaterEquals,   // 14 — >=
    LessEquals,      // 15 — <=

    // Grouping / punctuation
    LParen,          // 16 — (
    RParen,          // 17 — )
    Comma,           // 18 — ,

    // Sentinel
    Eof,             // 19 — end of input

    // ---- v1 additions (appended to preserve the 0–19 ordinals) ----
    DotDot,           // 20 — ..  (range)
    LBrace,           // 21 — {
    RBrace,           // 22 — }
    LBracket,         // 23 — [
    RBracket,         // 24 — ]
    StringLiteral,    // 25 — "..."   (Text = raw content between quotes)
    InterpolatedString, // 26 — $"..." (Text = raw content between quotes)
    Semicolon,        // 27 — ;
    Colon,            // 28 — :
}

// -------------------------------------------------------------------------
// Token — immutable lexical unit produced by the Tokenizer
// -------------------------------------------------------------------------

/// <summary>
/// An immutable lexical unit produced by <see cref="Tokenizer.Tokenize"/>.
/// Structural equality is provided by the record.
/// </summary>
/// <param name="Kind">The lexical category of this token.</param>
/// <param name="Text">
/// The raw source text of this token. For <see cref="TokenKind.StringLiteral"/>
/// and <see cref="TokenKind.InterpolatedString"/> this is the content between
/// the quotes (without the quotes). For <see cref="TokenKind.Eof"/> it is empty.
/// </param>
/// <param name="Position">Zero-based character index in the original input.</param>
public sealed record Token(TokenKind Kind, string Text, int Position);
