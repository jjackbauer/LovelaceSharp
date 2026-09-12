using Lovelace.Suite;

namespace Lovelace.Run;

/// <summary>
/// The script text a CALLER supplied, together with the map back to it from the text the ENGINE is
/// handed (<see cref="Lovelace.Suite.ScriptSource.ToSemicolonStatements"/>).
///
/// Every source position this runner publishes — <c>timings[].position</c> and the error envelope's
/// <c>diagnostics[].position</c>/<c>line</c>/<c>column</c> — is defined by the protocol against the
/// text the CALLER supplied ("the zero-based source offset of the statement",
/// <c>docs/symbolics/dsh-protocol.md:153-155</c>; the diagnostics array is "the parser's
/// source-position form", <c>:225-228</c>). The engine, however, indexes into the REWRITTEN text,
/// and that rewrite is not a bijection for two inputs:
///
/// <list type="bullet">
/// <item>a CRLF pair is collapsed to one <c>\n</c> before the (length-preserving) newline-to-<c>;</c>
/// pass (<c>Lovelace.Suite/ScriptSource.cs:27</c>), so every offset after a Windows line break is
/// short by one per preceding break;</item>
/// <item>a leading U+FEFF is stripped (<c>:31-32</c>), shifting the first character of the caller's
/// text by one.</item>
/// </list>
///
/// The newline-to-<c>;</c> pass itself IS length-preserving (one character in, one character out, in
/// order — <c>ScriptSource.cs:39-98</c>), so the two cases above are the whole map: character
/// <c>i</c> of the engine's text is character <c>_sourceIndex[i]</c> of the caller's text. Both
/// <c>line</c>/<c>column</c> are then recomputed from the caller's text, so a failure on source
/// line 3 reports line 3 (the engine's own computation runs on the semicolon-joined text, which has
/// no top-level newlines at all).
/// </summary>
internal sealed class ScriptPositions
{
    /// <summary>Maps <paramref name="source"/> (the caller's text, exactly as received) and builds
    /// the index of the engine's text once, so every position in one envelope is mapped by the same
    /// rule.</summary>
    internal ScriptPositions(string source)
    {
        Source = source;
        EngineSource = ScriptSource.ToSemicolonStatements(source);
        _sourceIndex = BuildSourceIndex(source, EngineSource.Length);
    }

    /// <summary>The text the caller supplied — the text every published position indexes into.</summary>
    internal string Source { get; }

    /// <summary>The text the engine is evaluated on (the semicolon-joined form).</summary>
    internal string EngineSource { get; }

    /// <summary>The offset in <see cref="Source"/> that <paramref name="engineOffset"/> (an offset in
    /// <see cref="EngineSource"/>) names. Negative offsets clamp to the start; an offset at or past
    /// the end of the engine's text clamps to the end of the caller's text (a lexer that ran off the
    /// end reports the end).</summary>
    internal int ToSourceOffset(int engineOffset)
    {
        if (engineOffset < 0)
            return 0;
        if (engineOffset < _sourceIndex.Length)
            return _sourceIndex[engineOffset];
        return _sourceIndex.Length > 0 ? Source.Length : 0;
    }

    /// <summary>1-based line and column of a SOURCE offset, read off the caller's own text. Line
    /// breaks are CRLF, CR and LF, each counting once — so a Windows script and a classic-Mac script
    /// report the same line and column as the Unix one.</summary>
    internal (int Line, int Column) LineColumn(int sourceOffset) =>
        ComputeLineColumn(Source, sourceOffset);

    /// <summary>Character <c>i</c> of the engine's text was produced from character
    /// <c>_sourceIndex[i]</c> of the caller's text. A CRLF pair is the only many-to-one step (the
    /// pair maps to the single <c>\n</c> at the LF's index); the only skipped character is a leading
    /// U+FEFF.</summary>
    private readonly int[] _sourceIndex;

    private static int[] BuildSourceIndex(string source, int engineLength)
    {
        var index = new int[engineLength];
        int s = source.Length > 0 && source[0] == '\uFEFF' ? 1 : 0;
        for (int e = 0; e < engineLength; e++)
        {
            if (s + 1 < source.Length && source[s] == '\r' && source[s + 1] == '\n')
                s++;    // the CR half of the pair: the engine's one character belongs to the LF
            index[e] = s;
            s++;
        }
        return index;
    }

    /// <summary>1-based line and column of an offset, the rule the engine applies to its own
    /// (rewritten) text and this host applies to the caller's: CR, LF and CRLF each end a line
    /// exactly once.</summary>
    private static (int Line, int Column) ComputeLineColumn(string source, int position)
    {
        if (position < 0 || position > source.Length)
            return (1, position + 1);

        int line = 1;
        int lineStart = 0;
        for (int i = 0; i < position && i < source.Length; i++)
        {
            char c = source[i];
            if (c == '\n')
            {
                // the LF half of a CRLF: the break was counted when the CR was read
                if (i > 0 && source[i - 1] == '\r')
                    continue;
                line++;
                lineStart = i + 1;
            }
            else if (c == '\r')
            {
                line++;
                lineStart = i + ((i + 1 < source.Length && source[i + 1] == '\n') ? 2 : 1);
            }
        }

        return (line, Math.Max(1, position - lineStart + 1));
    }
}
