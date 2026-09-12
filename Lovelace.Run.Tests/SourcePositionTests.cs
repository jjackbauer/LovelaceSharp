using System.Text;
using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// The machine API's source positions must name the real place in the CALLER's script.
///
/// docs/symbolics/dsh-protocol.md:153-155 — "`timings[].position` is the zero-based source offset of the
/// statement" — and :225-228 — the error envelope's `diagnostics` array is the parser's
/// source-position form (`message`/`position`/`line`/`column`). Both are defined against the text
/// the caller supplied, whatever its line endings.
///
/// Three live P1s (round-18 audit E, findings F1/F2/F3; amendment P.2 rows E-P1a/b/c) sit behind
/// these assertions:
///
///  * a RUNTIME failure reported `0/1/1` whatever statement failed, although the engine's own timing
///    ledger carries the failing statement's offset;
///  * `line`/`column` computed on the runner's semicolon-joined text (which has no top-level
///    newlines at all), so every failure on source line 3 reported line 1;
///  * `timings[].position` short by one character per CRLF line break, because the runner's
///    normalization collapses CRLF to LF before the length-preserving rewrite
///    (`Lovelace.Suite/ScriptSource.cs:27`).
///
/// Every expected offset below is computed by the TEST from the exact text (or the exact bytes) the
/// test handed to the runner — never read back out of the envelope — and the self-checks pin the
/// literals so a fixture drift cannot silently move the expectation.
/// </summary>
public class SourcePositionTests
{
    // -----------------------------------------------------------------
    // F1 — a runtime failure in the third statement
    // -----------------------------------------------------------------

    /// <summary>The failing call is the third statement, at offset 12 of the text the caller passed.
    /// The envelope must name THAT statement: position 12, line 3, column 1.</summary>
    [Fact]
    public async Task RuntimeFailureInTheThirdStatement_NamesThatStatement_Lf()
    {
        string source = "a = 1\nb = 2\ndet(1)";
        int failing = source.IndexOf("det(1)", StringComparison.Ordinal);
        Assert.Equal(12, failing);   // self-check: the fixture is the one the audit measured

        var (exitCode, envelope, _) = await RunAsync("--eval", source);

        Assert.Equal(1, exitCode);
        Assert.Equal(failing, DiagnosticPosition(envelope));
        Assert.Equal(3, DiagnosticLine(envelope));
        Assert.Equal(1, DiagnosticColumn(envelope));
        // all three statements ran; the LAST one is the one that failed
        Assert.Equal(new[] { 0, 6, 12 }, TimingsPositions(envelope));
        Assert.Equal(failing, TimingsPositions(envelope)[^1]);

        // the diagnostic and the timing ledger of the SAME envelope agree on the place
        Assert.Equal(TimingsPositions(envelope)[^1], DiagnosticPosition(envelope));
    }

    /// <summary>The same script with Windows line endings: the caller's offsets are 0, 6, 12.</summary>
    [Fact]
    public async Task RuntimeFailureInTheThirdStatement_NamesThatStatement_Crlf()
    {
        string source = "a = 1\r\nb = 2\r\ndet(1)";
        int failing = source.IndexOf("det(1)", StringComparison.Ordinal);
        Assert.Equal(14, failing);   // 5 + CRLF + 5 + CRLF: the CRLF source is two characters longer

        var (exitCode, envelope, _) = await RunAsync("--eval", source);

        Assert.Equal(1, exitCode);
        Assert.Equal(failing, DiagnosticPosition(envelope));
        Assert.Equal(3, DiagnosticLine(envelope));
        Assert.Equal(1, DiagnosticColumn(envelope));
        Assert.Equal(new[] { 0, 7, 14 }, TimingsPositions(envelope));
        Assert.Equal(failing, TimingsPositions(envelope)[^1]);
    }

    // -----------------------------------------------------------------
    // F2 — a parse failure on the third SOURCE LINE
    // -----------------------------------------------------------------

    /// <summary>A lexer/parser refusal is the one diagnostic that carries an engine position of its
    /// own (scraped from "at position N"). It must be reported as a place in the caller's script: the
    /// offending `)` is at offset 10, on line 3, column 1.</summary>
    [Fact]
    public async Task ParseFailureOnTheThirdSourceLine_ReportsThatLine_Lf()
    {
        string source = "1+1;\n2+2;\n)";
        int offending = source.LastIndexOf(')');
        Assert.Equal(10, offending);

        var (exitCode, envelope, _) = await RunAsync("--eval", source);

        Assert.Equal(1, exitCode);
        Assert.Equal(offending, DiagnosticPosition(envelope));
        Assert.Equal(3, DiagnosticLine(envelope));
        Assert.Equal(1, DiagnosticColumn(envelope));
        // nothing was parsed, so no statement ran
        Assert.Empty(envelope["timings"]!.AsArray());
    }

    /// <summary>The same refusal through `--file`, with the CRLF bytes the test writes: the offending
    /// byte is the 13th, at offset 12, on line 3, column 1.</summary>
    [Fact]
    public async Task ParseFailureOnTheThirdSourceLine_ReportsThatLine_CrlfFile()
    {
        byte[] bytes = Encoding.ASCII.GetBytes("1+1;\r\n2+2;\r\n)");
        string path = Path.Combine(Path.GetTempPath(), "lovelace-positions-" + Guid.NewGuid().ToString("N") + ".ls");
        await File.WriteAllBytesAsync(path, bytes);
        try
        {
            byte[] onDisk = await File.ReadAllBytesAsync(path);
            int offending = Array.IndexOf(onDisk, (byte)')');
            string text = Encoding.ASCII.GetString(onDisk);
            var (expectedLine, expectedColumn) = Locate(text, offending);
            Assert.Equal(12, offending);        // 31 2B 31 3B 0D 0A 32 2B 32 3B 0D 0A 29
            Assert.Equal((3, 1), (expectedLine, expectedColumn));

            var (exitCode, envelope, _) = await RunAsync("--file", path, "--omit-functions");

            Assert.Equal(1, exitCode);
            Assert.Equal(offending, DiagnosticPosition(envelope));
            Assert.Equal(expectedLine, DiagnosticLine(envelope));
            Assert.Equal(expectedColumn, DiagnosticColumn(envelope));
            Assert.Empty(envelope["timings"]!.AsArray());
        }
        finally
        {
            File.Delete(path);
        }
    }

    // -----------------------------------------------------------------
    // F3 — timings[].position is a caller offset for EVERY line ending
    // -----------------------------------------------------------------

    /// <summary>Three statements with CRLF separators start at the caller's offsets 0, 6 and 12.</summary>
    [Fact]
    public async Task TimingsPositions_AreCallerOffsets_Crlf()
    {
        string source = "1+1;\r\n2+2;\r\n3+3";
        int[] expected =
        [
            source.IndexOf("1+1", StringComparison.Ordinal),
            source.IndexOf("2+2", StringComparison.Ordinal),
            source.IndexOf("3+3", StringComparison.Ordinal),
        ];
        Assert.Equal(new[] { 0, 6, 12 }, expected);

        var (exitCode, envelope, _) = await RunAsync("--eval", source);

        Assert.Equal(0, exitCode);
        Assert.Equal(expected, TimingsPositions(envelope));
    }

    /// <summary>No regression: LF and CR sources were already correct and keep their offsets.</summary>
    [Theory]
    [InlineData("1+1;\n2+2;\n3+3")]
    [InlineData("1+1;\r2+2;\r3+3")]
    public async Task TimingsPositions_AreCallerOffsets_LfAndCr(string source)
    {
        int[] expected =
        [
            source.IndexOf("1+1", StringComparison.Ordinal),
            source.IndexOf("2+2", StringComparison.Ordinal),
            source.IndexOf("3+3", StringComparison.Ordinal),
        ];
        Assert.Equal(new[] { 0, 5, 10 }, expected);

        var (exitCode, envelope, _) = await RunAsync("--eval", source);

        Assert.Equal(0, exitCode);
        Assert.Equal(expected, TimingsPositions(envelope));
    }

    /// <summary>A CR-only source reports its parse failure at the caller's place too: offset 10,
    /// line 3, column 1.</summary>
    [Fact]
    public async Task ParseFailureOnTheThirdSourceLine_ReportsThatLine_CrOnly()
    {
        string source = "1+1;\r2+2;\r)";
        int offending = source.LastIndexOf(')');
        var (expectedLine, expectedColumn) = Locate(source, offending);
        Assert.Equal(10, offending);
        Assert.Equal((3, 1), (expectedLine, expectedColumn));

        var (exitCode, envelope, _) = await RunAsync("--eval", source);

        Assert.Equal(1, exitCode);
        Assert.Equal(offending, DiagnosticPosition(envelope));
        Assert.Equal(expectedLine, DiagnosticLine(envelope));
        Assert.Equal(expectedColumn, DiagnosticColumn(envelope));
    }

    // -----------------------------------------------------------------
    // No regression: the single-statement and single-line cases
    // -----------------------------------------------------------------

    /// <summary>A ONE-statement script that fails really is at offset 0: line 1, column 1.</summary>
    [Fact]
    public async Task SingleStatementFailure_KeepsPositionZeroLineOneColumnOne()
    {
        var (exitCode, envelope, _) = await RunAsync("--eval", "det(1)");

        Assert.Equal(1, exitCode);
        Assert.Equal(0, DiagnosticPosition(envelope));
        Assert.Equal(1, DiagnosticLine(envelope));
        Assert.Equal(1, DiagnosticColumn(envelope));
        Assert.Equal(new[] { 0 }, TimingsPositions(envelope));
    }

    /// <summary>A failure in the SECOND statement of a single-line script: offset 5, line 1,
    /// column 6 — the value the old envelope reported as 0/1/1.</summary>
    [Fact]
    public async Task FailureInTheSecondStatementOfOneLine_NamesThatStatement()
    {
        string source = "1+1; det(1)";
        int failing = source.IndexOf("det(1)", StringComparison.Ordinal);
        Assert.Equal(5, failing);

        var (exitCode, envelope, _) = await RunAsync("--eval", source);

        Assert.Equal(1, exitCode);
        Assert.Equal(failing, DiagnosticPosition(envelope));
        Assert.Equal(1, DiagnosticLine(envelope));
        Assert.Equal(6, DiagnosticColumn(envelope));
    }

    /// <summary>A leading byte-order mark is part of the text the caller supplied, so the positions
    /// index into it: the engine's first statement (offset 0 of the text the engine sees, which drops
    /// the BOM) is at offset 1 of the caller's argument, and a consumer slicing the argument gets the
    /// statement back.</summary>
    [Fact]
    public async Task LeadingBom_PositionsStayOffsetsIntoTheCallersText()
    {
        string source = "\uFEFFdet(1)";
        Assert.Equal(1, source.IndexOf("det(1)", StringComparison.Ordinal));

        var (exitCode, envelope, _) = await RunAsync("--eval", source);

        Assert.Equal(1, exitCode);
        Assert.Equal(1, DiagnosticPosition(envelope));
        Assert.Equal(1, DiagnosticLine(envelope));
        Assert.Equal(2, DiagnosticColumn(envelope));
        Assert.Equal(new[] { 1 }, TimingsPositions(envelope));
    }

    /// <summary>The diagnostics DTO shape is unchanged: four fields, no more.</summary>
    [Fact]
    public async Task DiagnosticEntry_KeepsItsFourFieldShape()
    {
        var (_, envelope, _) = await RunAsync("--eval", "a = 1\nb = 2\ndet(1)");

        JsonObject diagnostic = envelope["diagnostics"]!.AsArray()[0]!.AsObject();
        Assert.Equal(
            new[] { "column", "line", "message", "position" },
            diagnostic.Select(p => p.Key).OrderBy(k => k, StringComparer.Ordinal).ToArray());
        Assert.Equal(
            new[]
            {
                "category", "code", "diagnostics", "elapsed", "elapsedTime", "mathIrVersion", "message",
                "ok", "protocolVersion", "recoverable", "symbolicFormatVersion", "timings",
            },
            envelope.AsObject().Select(p => p.Key).OrderBy(k => k, StringComparer.Ordinal).ToArray());
    }

    // -----------------------------------------------------------------
    // Plumbing
    // -----------------------------------------------------------------

    private static async Task<(int ExitCode, JsonNode Envelope, string Raw)> RunAsync(params string[] arguments)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(arguments, stdout, stderr);
        string raw = stdout.ToString();
        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(raw, "the envelope"), raw);
    }

    private static int DiagnosticPosition(JsonNode envelope) =>
        envelope["diagnostics"]!.AsArray()[0]!["position"]!.GetValue<int>();

    private static int DiagnosticLine(JsonNode envelope) =>
        envelope["diagnostics"]!.AsArray()[0]!["line"]!.GetValue<int>();

    private static int DiagnosticColumn(JsonNode envelope) =>
        envelope["diagnostics"]!.AsArray()[0]!["column"]!.GetValue<int>();

    private static int[] TimingsPositions(JsonNode envelope) =>
        envelope["timings"]!.AsArray().Select(t => t!["position"]!.GetValue<int>()).ToArray();

    /// <summary>
    /// The test's own oracle for "1-based line and column of an offset": walk the caller's text
    /// counting characters, treating CRLF as ONE line break and a lone CR or LF as one each — the
    /// rule the protocol's line/column fields are read under.
    /// </summary>
    private static (int Line, int Column) Locate(string source, int offset)
    {
        int line = 1;
        int column = 1;
        for (int i = 0; i < offset;)
        {
            if (source[i] == '\r' && i + 1 < source.Length && source[i + 1] == '\n')
            {
                line++;
                column = 1;
                i += 2;
            }
            else if (source[i] == '\r' || source[i] == '\n')
            {
                line++;
                column = 1;
                i++;
            }
            else
            {
                column++;
                i++;
            }
        }
        return (line, column);
    }
}
