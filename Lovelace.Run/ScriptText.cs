using System.Text;

namespace Lovelace.Run;

/// <summary>
/// The ONE reader every surface of this runner is handed its script text through, and the ONE
/// definition of what that text is.
///
/// The protocol defines every published position — <c>timings[].position</c> and the error
/// envelope's <c>diagnostics[].position</c>/<c>line</c>/<c>column</c> — as an offset into the
/// characters of the text the CALLER supplied (docs/symbolics/dsh-protocol.md, "Positions and
/// locations"). "The text the caller supplied" is what the surface RECEIVED, character for
/// character:
///
/// <list type="bullet">
/// <item><c>--eval</c>: the argument string, exactly as the process received it
/// (<see cref="FromArgument"/>);</item>
/// <item><c>--stdin</c>: the characters the reader produced, exactly as it produced them
/// (<see cref="FromStandardInputAsync"/>);</item>
/// <item><c>--file</c>: the named file's characters, decoded from its bytes with a leading
/// byte-order mark KEPT, as the U+FEFF character it is (<see cref="FromFileAsync"/>).</item>
/// </list>
///
/// All three therefore agree for byte-identical input, which is what makes a reported position
/// usable: slicing the text a caller handed over at the reported position names the same statement
/// whichever surface carried it. Each surface has exactly one entry point here, so a new surface
/// cannot invent a fourth decode and drift again.
///
/// The surface that used to disagree is <c>--file</c>: <see cref="File.ReadAllText(string)"/>
/// detects the encoding from the byte-order mark and CONSUMES it, so a "UTF-8 with BOM" file
/// arrived one character shorter than the very same bytes on <c>--eval</c>/<c>--stdin</c> and every
/// published position moved by one. Measured on the round-20 control tree (HEAD, d87e910) with
/// <c>a = 1\nb = 2\ndet(a)\n</c> in <c>out/bom.lv</c>: <c>--file</c> answered position 12 with
/// timings [0, 6, 12] where <c>--eval</c> answered 13 with [1, 7, 13] for the same text.
/// </summary>
internal static class ScriptText
{
    /// <summary>The <c>--eval</c> surface's text: the argument exactly as the process received it.
    /// Taking it through here is what keeps this surface and <see cref="FromFileAsync"/> under one
    /// definition instead of two conventions.</summary>
    internal static string FromArgument(string argument) => argument;

    /// <summary>The <c>--stdin</c> surface's text: the characters the reader produced. The runner
    /// is handed <see cref="Console.In"/> by the process entry point and a test's reader in process;
    /// both are the caller's text as received, so no normalization happens here.</summary>
    internal static async Task<string> FromStandardInputAsync(TextReader stdin) =>
        await stdin.ReadToEndAsync();

    /// <summary>The <c>--file</c> surface's text: the named file's characters, a leading byte-order
    /// mark included (see <see cref="Decode"/>).</summary>
    internal static async Task<string> FromFileAsync(string path) =>
        Decode(await File.ReadAllBytesAsync(path));

    /// <summary>
    /// The characters of a script file's BYTES. The encoding is detected from a byte-order mark
    /// exactly as <see cref="File.ReadAllText(string)"/> detects it — UTF-8, UTF-16 LE/BE or
    /// UTF-32, and UTF-8 when there is no mark — but the mark is NOT consumed: the caller's text
    /// keeps it as its first character, U+FEFF.
    ///
    /// Two readers therefore agree on one text: this one and the <c>--eval</c>/<c>--stdin</c> route,
    /// both of which receive the same U+FEFF when the caller supplied one. The ENGINE is the layer
    /// that drops the mark, because the tokenizer does not accept U+FEFF
    /// (<c>Lovelace.Suite/ScriptSource.cs:31-32</c>), and <see cref="ScriptPositions"/> maps the
    /// engine's offsets back across that one skipped character — so the script runs identically and
    /// the positions still name the caller's own text.
    /// </summary>
    internal static string Decode(byte[] bytes)
    {
        if (bytes.Length == 0)
            return string.Empty;

        using var stream = new MemoryStream(bytes, writable: false);
        using var reader = new StreamReader(stream, Encoding.UTF8, detectEncodingFromByteOrderMarks: true);
        string text = reader.ReadToEnd();
        // reader.CurrentEncoding is only settled once the first bytes were read; the preamble the
        // reader consumed is exactly what makes the caller's text one U+FEFF longer than the text
        // it returned.
        return HasByteOrderMark(bytes, reader.CurrentEncoding) ? "\uFEFF" + text : text;
    }

    /// <summary>Whether <paramref name="bytes"/> begins with the byte-order mark of the encoding
    /// the reader detected. A UTF-8 mark is <c>EF BB BF</c>, UTF-16 LE <c>FF FE</c>, UTF-16 BE
    /// <c>FE FF</c> and UTF-32 <c>FF FE 00 00</c>; an encoding without a mark (or a file without
    /// one) reports <see langword="false"/> and the text is returned untouched.</summary>
    private static bool HasByteOrderMark(byte[] bytes, Encoding encoding)
    {
        byte[] preamble = encoding.GetPreamble();
        if (preamble.Length == 0 || bytes.Length < preamble.Length)
            return false;

        for (int i = 0; i < preamble.Length; i++)
        {
            if (bytes[i] != preamble[i])
                return false;
        }

        return true;
    }
}
