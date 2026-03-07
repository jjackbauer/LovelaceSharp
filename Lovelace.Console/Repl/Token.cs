namespace Lovelace.Console.Repl;

// -------------------------------------------------------------------------
// TokenKind — discriminates the 20 lexical token types
// -------------------------------------------------------------------------

/// <summary>
/// Identifies the lexical category of a <see cref="Token"/> produced by the
/// <see cref="Tokenizer"/>. Values are assigned in declaration order (0–19).
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
}

// -------------------------------------------------------------------------
// Token — immutable lexical unit produced by the Tokenizer
// -------------------------------------------------------------------------

/// <summary>
/// An immutable lexical unit produced by <see cref="Tokenizer.Tokenize"/>.
/// <para>
/// Structural equality is provided automatically by the record: two
/// <see cref="Token"/> values are equal when <see cref="Kind"/>,
/// <see cref="Text"/>, and <see cref="Position"/> are equal.
/// </para>
/// </summary>
/// <param name="Kind">The lexical category of this token.</param>
/// <param name="Text">
/// The raw source text of this token (e.g. <c>"42"</c>, <c>"abc"</c>,
/// <c>"+"</c>). For <see cref="TokenKind.Eof"/> the text is an empty string.
/// </param>
/// <param name="Position">
/// Zero-based character index in the original input string where this token
/// starts.
/// </param>
public sealed record Token(TokenKind Kind, string Text, int Position);
