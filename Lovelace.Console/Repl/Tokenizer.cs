namespace Lovelace.Console.Repl;

/// <summary>
/// Lexer that converts a raw input string into a <see cref="List{T}"/> of
/// <see cref="Token"/> values, always terminated by a
/// <see cref="TokenKind.Eof"/> sentinel token.
/// </summary>
public sealed class Tokenizer
{
    /// <summary>
    /// Scans <paramref name="input"/> and returns the complete token list,
    /// always terminated by a <see cref="TokenKind.Eof"/> token.
    /// </summary>
    /// <param name="input">The source text to tokenize. Must not be null.</param>
    /// <returns>
    /// A <see cref="List{T}"/> of tokens in source order, ending with
    /// <see cref="TokenKind.Eof"/>.
    /// </returns>
    /// <exception cref="InvalidOperationException">
    /// Thrown when an unrecognised character is encountered.
    /// </exception>
    public List<Token> Tokenize(string input)
    {
        var tokens = new List<Token>();
        int pos = 0;

        while (pos < input.Length)
        {
            // Skip whitespace
            if (char.IsWhiteSpace(input[pos]))
            {
                pos++;
                continue;
            }

            // Number literal: leading digit, or leading '.' followed by a digit
            if (char.IsAsciiDigit(input[pos]) ||
                (input[pos] == '.' && pos + 1 < input.Length && char.IsAsciiDigit(input[pos + 1])))
            {
                int start = pos;
                tokens.Add(ScanNumberLiteral(input, ref pos, start));
                continue;
            }

            // Identifier: [a-zA-Z_][a-zA-Z0-9_]*
            if (char.IsAsciiLetter(input[pos]) || input[pos] == '_')
            {
                int start = pos;
                while (pos < input.Length && (char.IsAsciiLetterOrDigit(input[pos]) || input[pos] == '_'))
                    pos++;
                tokens.Add(new Token(TokenKind.Identifier, input.Substring(start, pos - start), start));
                continue;
            }

            // Two-char operators — must be checked before their single-char prefixes
            if (pos + 1 < input.Length)
            {
                char c0 = input[pos];
                char c1 = input[pos + 1];

                TokenKind? twoChar = (c0, c1) switch
                {
                    ('=', '=') => TokenKind.DoubleEquals,
                    ('!', '=') => TokenKind.BangEquals,
                    ('>', '=') => TokenKind.GreaterEquals,
                    ('<', '=') => TokenKind.LessEquals,
                    _          => (TokenKind?)null,
                };

                if (twoChar is { } kind)
                {
                    tokens.Add(new Token(kind, input.Substring(pos, 2), pos));
                    pos += 2;
                    continue;
                }
            }

            // Single-char operators and punctuation
            {
                TokenKind? oneChar = input[pos] switch
                {
                    '+' => TokenKind.Plus,
                    '-' => TokenKind.Minus,
                    '*' => TokenKind.Star,
                    '/' => TokenKind.Slash,
                    '%' => TokenKind.Percent,
                    '^' => TokenKind.Caret,
                    '!' => TokenKind.Bang,
                    '=' => TokenKind.Equals,
                    '>' => TokenKind.Greater,
                    '<' => TokenKind.Less,
                    '(' => TokenKind.LParen,
                    ')' => TokenKind.RParen,
                    ',' => TokenKind.Comma,
                    _   => (TokenKind?)null,
                };

                if (oneChar is { } kind)
                {
                    tokens.Add(new Token(kind, input[pos].ToString(), pos));
                    pos++;
                    continue;
                }
            }

            throw new InvalidOperationException(
                $"Unexpected character '{input[pos]}' at position {pos}.");
        }

        tokens.Add(new Token(TokenKind.Eof, string.Empty, pos));
        return tokens;
    }

    // -------------------------------------------------------------------------
    // Internal scanner helpers
    // -------------------------------------------------------------------------

    /// <summary>
    /// Scans a complete number literal beginning at <paramref name="start"/>.
    /// <para>
    /// Grammar (all parts after leading digits are optional):
    /// <code>
    ///   number ::= digits? '.' digits ('(' digits ')')?
    ///            | digits ('.' digits? ('(' digits ')')?)?
    /// </code>
    /// In practice the tokenizer accepts: leading digits, optional <c>.</c> then
    /// optional trailing digits, then optional <c>(digits)</c> for periodic notation.
    /// The raw text is captured verbatim so the evaluator can choose the correct
    /// numeric type later (type inference is deferred).
    /// </para>
    /// </summary>
    private static Token ScanNumberLiteral(string input, ref int pos, int start)
    {
        // Leading digits (zero or more — a leading '.' is also valid entry point)
        while (pos < input.Length && char.IsAsciiDigit(input[pos]))
            pos++;

        // '.' followed by optional digits
        if (pos < input.Length && input[pos] == '.')
        {
            pos++; // consume '.'

            while (pos < input.Length && char.IsAsciiDigit(input[pos]))
                pos++;
        }

        // Optional periodic suffix '(' digits ')'
        if (pos < input.Length && input[pos] == '(')
        {
            pos++; // consume '('

            while (pos < input.Length && char.IsAsciiDigit(input[pos]))
                pos++;

            if (pos < input.Length && input[pos] == ')')
                pos++; // consume ')'
        }

        string text = input.Substring(start, pos - start);
        return new Token(TokenKind.NumberLiteral, text, start);
    }
}
