using System.Text;
using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// G-2 (round-20 audit G, cross-surface consistency): byte-identical script text must report the
/// SAME source positions on every surface it can arrive on.
///
/// The protocol defines both published position forms against one text — the text the CALLER handed
/// over (docs/symbolics/dsh-protocol.md, "Positions and locations"):
///
///   * `timings[].position` is "the zero-based source offset of the statement";
///   * the error envelope's `diagnostics[].position`/`line`/`column` is the parser's
///     source-position form.
///
/// This class drives the THREE surfaces of the runner — `--eval`, `--file` and `--stdin` — with the
/// four byte-level spellings of one script (no BOM / BOM / CRLF / BOM+CRLF) and asserts that all
/// three report the same failing statement, at the offset this test measured in the text it handed
/// over. The self-checks pin the literals, and the last assertion is the consumer-visible property
/// the contract exists for: slicing the caller's own text at the reported position returns the
/// failing statement.
///
/// Measured on HEAD (d87e910) with the published Native AOT binary: `--file out/bom.lv` answers
/// position 12 with timings [0, 6, 12] where `--eval`/`--stdin` answer 13 and [1, 7, 13], because
/// `File.ReadAllText` consumes the UTF-8 byte-order mark. On that tree the BOM rows below fail on
/// the `--file` surface alone; the other rows pass, which is what makes this a cross-surface
/// divergence and not a general position bug.
/// </summary>
public class SurfacePositionAgreementTests
{
    private const string Failing = "det(1)";

    /// <summary>One script, four byte-level spellings, each handed to ALL THREE surfaces. The file
    /// surface receives the text's UTF-8 bytes: a leading U+FEFF IS the UTF-8 byte-order mark
    /// EF BB BF, so the BOM rows write a real "UTF-8 with BOM" file.</summary>
    [Theory]
    [InlineData("a = 1\nb = 2\ndet(1)", 0, 6, 12)]              // LF, no BOM
    [InlineData("\uFEFFa = 1\nb = 2\ndet(1)", 1, 7, 13)]        // LF, BOM
    [InlineData("a = 1\r\nb = 2\r\ndet(1)", 0, 7, 14)]          // CRLF, no BOM
    [InlineData("\uFEFFa = 1\r\nb = 2\r\ndet(1)", 1, 8, 15)]    // CRLF, BOM
    public async Task EverySurface_ReportsTheSamePositions_ForTheSameText(
        string source, int first, int second, int failing)
    {
        // self-check: the pinned offsets really are the text's own, so a fixture drift cannot
        // silently move the expectation
        Assert.Equal(
            new[] { first, second, failing },
            new[]
            {
                source.IndexOf("a = 1", StringComparison.Ordinal),
                source.IndexOf("b = 2", StringComparison.Ordinal),
                source.IndexOf(Failing, StringComparison.Ordinal),
            });

        byte[] bytes = Encoding.UTF8.GetBytes(source);
        if (source.Length > 0 && source[0] == '\uFEFF')
        {
            // the text's first character IS the byte-order mark on disk
            Assert.Equal(new byte[] { 0xEF, 0xBB, 0xBF }, bytes.Take(3).ToArray());
        }

        string path = Path.Combine(Path.GetTempPath(),
            "lovelace-surface-" + Guid.NewGuid().ToString("N") + ".lv");
        await File.WriteAllBytesAsync(path, bytes);
        try
        {
            var eval = await RunAsync(null, "--eval", source);
            var file = await RunAsync(null, "--file", path);
            var stdin = await RunAsync(new StringReader(source), "--stdin");

            // The one assertion the finding is about: the three surfaces agree, and they agree with
            // the text this test handed over. One string per surface, so a divergence is reported as
            // the divergence it is — which surface said what — not as three anonymous failures.
            Assert.Equal(
                new[]
                {
                    $"--eval {failing} [{first}, {second}, {failing}]",
                    $"--file {failing} [{first}, {second}, {failing}]",
                    $"--stdin {failing} [{first}, {second}, {failing}]",
                },
                new[]
                {
                    Describe("--eval", eval),
                    Describe("--file", file),
                    Describe("--stdin", stdin),
                });

            foreach (var run in new[] { eval, file, stdin })
            {
                Assert.Equal(1, run.ExitCode);
                // line/column are read off the same text: the failing statement is on line 3,
                // column 1 on every surface
                Assert.Equal(3, DiagnosticLine(run.Envelope));
                Assert.Equal(1, DiagnosticColumn(run.Envelope));
                // the consumer's slice: the reported position names the failing statement in the
                // caller's own text, on every surface
                Assert.Equal(Failing,
                    source.Substring(DiagnosticPosition(run.Envelope), Failing.Length));
            }
        }
        finally
        {
            File.Delete(path);
        }
    }

    /// <summary>
    /// The `--file` surface decodes the caller's FILE. The reader keeps the encoding detection
    /// `File.ReadAllText` had (a UTF-8/UTF-16 LE/BE/UTF-32 byte-order mark is honoured) while
    /// keeping the mark itself as the first character of the text, so a UTF-16 script reports
    /// exactly the place the same characters report on `--eval`.
    /// </summary>
    [Fact]
    public async Task FileSurface_KeepsTheByteOrderMark_ForAUtf16File()
    {
        string source = "\uFEFFa = 1\nb = 2\ndet(1)";
        int failing = source.IndexOf(Failing, StringComparison.Ordinal);
        Assert.Equal(13, failing);

        // UTF-16 LE with its own byte-order mark: FF FE followed by the text's characters
        byte[] bytes = Encoding.Unicode.GetPreamble()
            .Concat(Encoding.Unicode.GetBytes(source[1..]))
            .ToArray();
        Assert.Equal(new byte[] { 0xFF, 0xFE }, bytes.Take(2).ToArray());

        string path = Path.Combine(Path.GetTempPath(),
            "lovelace-surface-utf16-" + Guid.NewGuid().ToString("N") + ".lv");
        await File.WriteAllBytesAsync(path, bytes);
        try
        {
            var run = await RunAsync(null, "--file", path);

            Assert.Equal(1, run.ExitCode);
            Assert.Equal(failing, DiagnosticPosition(run.Envelope));
            Assert.Equal(Failing,
                source.Substring(DiagnosticPosition(run.Envelope), Failing.Length));
        }
        finally
        {
            File.Delete(path);
        }
    }

    // -----------------------------------------------------------------
    // Plumbing
    // -----------------------------------------------------------------

    /// <summary>Runs the runner in-process, with the surface's own reader: `--stdin` reads the
    /// characters a reader produces, so the test hands the runner the exact text a pipe would.</summary>
    private static async Task<(int ExitCode, JsonNode Envelope, string Raw)> RunAsync(
        TextReader? stdin, params string[] arguments)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(arguments, stdout, stderr, stdin);
        string raw = stdout.ToString();
        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(raw, "the envelope"), raw);
    }

    /// <summary>What one surface answered, as one comparable line: the failing statement's reported
    /// position and the timing ledger's positions.</summary>
    private static string Describe(string surface, (int ExitCode, JsonNode Envelope, string Raw) run) =>
        $"{surface} {DiagnosticPosition(run.Envelope)} " +
        $"[{string.Join(", ", TimingsPositions(run.Envelope))}]" +
        (run.ExitCode == 1 ? string.Empty : $" (exit {run.ExitCode})");

    private static int DiagnosticPosition(JsonNode envelope) =>
        envelope["diagnostics"]!.AsArray()[0]!["position"]!.GetValue<int>();

    private static int DiagnosticLine(JsonNode envelope) =>
        envelope["diagnostics"]!.AsArray()[0]!["line"]!.GetValue<int>();

    private static int DiagnosticColumn(JsonNode envelope) =>
        envelope["diagnostics"]!.AsArray()[0]!["column"]!.GetValue<int>();

    private static int[] TimingsPositions(JsonNode envelope) =>
        envelope["timings"]!.AsArray().Select(t => t!["position"]!.GetValue<int>()).ToArray();
}
