namespace Lovelace.Suite;

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

            // Interpolated string $"..."  (must be checked before plain '"')
            if (input[pos] == '$' && pos + 1 < input.Length && input[pos + 1] == '"')
            {
                tokens.Add(ScanQuotedString(input, ref pos, TokenKind.InterpolatedString, skip: 2));
                continue;
            }

            // String literal "..."
            if (input[pos] == '"')
            {
                tokens.Add(ScanQuotedString(input, ref pos, TokenKind.StringLiteral, skip: 1));
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
                    ('.', '.') => TokenKind.DotDot,
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
                    '{' => TokenKind.LBrace,
                    '}' => TokenKind.RBrace,
                    '[' => TokenKind.LBracket,
                    ']' => TokenKind.RBracket,
                    ';' => TokenKind.Semicolon,
                    ':' => TokenKind.Colon,
                    _   => (TokenKind?)null,
                };

                if (oneChar is { } k)
                {
                    tokens.Add(new Token(k, input[pos].ToString(), pos));
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
    /// Scans a quoted string (<c>"..."</c> or <c>$"..."</c>). <paramref name="skip"/>
    /// is the number of leading characters to consume before the opening quote's
    /// content (1 for <c>"</c>, 2 for <c>$"</c>). The token text is the raw content
    /// between the quotes.
    /// </summary>
    private static Token ScanQuotedString(string input, ref int pos, TokenKind kind, int skip)
    {
        int start = pos;
        pos += skip;                       // consume '"' or '$"'
        int contentStart = pos;

        while (pos < input.Length && input[pos] != '"')
            pos++;

        string content = input.Substring(contentStart, pos - contentStart);

        if (pos < input.Length && input[pos] == '"')
            pos++;                         // consume closing quote

        return new Token(kind, content, start);
    }

    /// <summary>
    /// Scans a complete number literal beginning at <paramref name="start"/>.
    /// The raw text is captured verbatim so the evaluator can choose the correct
    /// numeric type later (type inference is deferred).
    /// </summary>
    private static Token ScanNumberLiteral(string input, ref int pos, int start)
    {
        // Leading digits
        while (pos < input.Length && char.IsAsciiDigit(input[pos]))
            pos++;

        // '.' followed by optional digits — but NOT when the next char is also '.'
        // (which would be the '..' range operator).
        if (pos < input.Length && input[pos] == '.' &&
            !(pos + 1 < input.Length && input[pos + 1] == '.'))
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
