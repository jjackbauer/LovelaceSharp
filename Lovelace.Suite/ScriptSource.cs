using System.Text;

namespace Lovelace.Suite;

/// <summary>
/// Host-side script-text normalization shared by every front-end. The engine's
/// program grammar is semicolon-separated while script files, the REPL buffer,
/// and the web IDE are newline-separated, so each host rewrites newlines to
/// <c>;</c> before evaluation. This keeps that one transformation in a single
/// place instead of duplicating it per front-end.
/// </summary>
public static class ScriptSource
{
    /// <summary>
    /// Normalizes CRLF/CR to LF, then rewrites top-level newlines (brace,
    /// bracket, paren, and string aware) to <c>;</c> so a multi-line script
    /// parses as one program.
    /// </summary>
    /// <remarks>
    /// The rewrite is length-preserving: each newline becomes exactly one
    /// character (a <c>;</c>, or a space when suppressed), so the engine's
    /// <c>position</c> diagnostics still index into the input and hosts can
    /// recompute line/column from their original source.
    /// </remarks>
    public static string ToSemicolonStatements(string source)
    {
        string text = source.Replace("\r\n", "\n").Replace('\r', '\n');
        // Strip a leading UTF-8 byte-order mark (files saved as "UTF-8 with BOM",
        // and some hosts' stdin pipes prepend one); the tokenizer treats \uFEFF
        // as an unexpected character rather than whitespace.
        if (text.Length > 0 && text[0] == '\uFEFF')
            text = text[1..];

        var sb = new StringBuilder(text.Length);
        int depth = 0;
        bool inString = false;
        char? last = null; // last non-whitespace char emitted outside a string

        for (int i = 0; i < text.Length; i++)
        {
            char c = text[i];

            if (inString)
            {
                sb.Append(c);
                if (c == '"')
                {
                    inString = false;
                    last = c;
                }
                continue;
            }

            if (c == '"')
            {
                inString = true;
                sb.Append(c);
                last = c;
                continue;
            }

            switch (c)
            {
                case '{':
                case '[':
                case '(':
                    depth++;
                    sb.Append(c);
                    last = c;
                    break;
                case '}':
                case ']':
                case ')':
                    if (depth > 0) depth--;
                    sb.Append(c);
                    last = c;
                    break;
                case '\n':
                    if (depth == 0)
                    {
                        // Suppress a separator after ';' (trailing/blank lines) and at the start.
                        char replacement = last is null || last == ';' ? ' ' : ';';
                        sb.Append(replacement);
                        if (replacement == ';') last = ';';
                    }
                    else
                    {
                        sb.Append('\n');
                    }
                    break;
                default:
                    if (!char.IsWhiteSpace(c)) last = c;
                    sb.Append(c);
                    break;
            }
        }

        return sb.ToString();
    }
}
