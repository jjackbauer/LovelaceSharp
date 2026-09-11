using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// docs/symbolics/dsh-protocol.md:9-10 — "Anything the script prints with <c>print(...)</c> is
/// captured and returned in the top-level <c>output</c> array." The array carries the LINES the
/// script printed; it must not carry the host's line terminator. On Windows the capture path ends
/// every line with CRLF, so an element that still holds the CR cannot be compared to the text the
/// script printed.
/// </summary>
public class PrintOutputTests
{
    private static async Task<(int ExitCode, string Raw, JsonNode Envelope)> RunAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(
            new[] { "--eval", script, "--omit-functions", "--omit-variables" }, stdout, stderr);
        string raw = stdout.ToString();
        return (exitCode, raw, TestSupport.ParseExactlyOneJsonDocument(raw, $"stdout of '{script}'"));
    }

    private static string[] Output(JsonNode envelope) =>
        envelope["output"]!.AsArray().Select(node => node!.GetValue<string>()).ToArray();

    /// <summary>The serialized output array exactly as it appears on stdout.</summary>
    private static string OutputSegment(string raw)
    {
        int start = raw.IndexOf("\"output\":", StringComparison.Ordinal);
        Assert.True(start >= 0, $"the envelope has no output array: {raw}");
        int end = raw.IndexOf(']', start);
        Assert.True(end > start, $"the output array is unterminated: {raw}");
        return raw.Substring(start, end - start + 1);
    }

    /// <summary>The element count, the exact serialized segment, and the absence of the CR escape
    /// (a carriage return on the wire is the two characters backslash-r or the \u000d escape).</summary>
    private static void AssertWireOutputIs(string raw, string expectedSegment)
    {
        string segment = OutputSegment(raw);
        Assert.Equal(expectedSegment, segment);
        Assert.DoesNotContain("\\r", segment);
        Assert.DoesNotContain("\\u000d", segment);
    }

    /// <summary>The absence check alone, for segments whose spelling the JSON encoder escapes.</summary>
    private static void AssertWireOutputHasNoTerminator(string raw)
    {
        string segment = OutputSegment(raw);
        Assert.DoesNotContain("\\r", segment);
        Assert.DoesNotContain("\\u000d", segment);
    }

    [Fact]
    public async Task TwoPrints_CrossAsTwoLines_WithoutTheHostsLineTerminator()
    {
        var (exitCode, raw, envelope) = await RunAsync("print(\"A\"); print(\"B\")");

        Assert.Equal(0, exitCode);
        Assert.Equal(new[] { "A", "B" }, Output(envelope));
        AssertWireOutputIs(raw, "\"output\":[\"A\",\"B\"]");
    }

    /// <summary>Every element — not just the last — is free of the terminator: three prints is
    /// where the old capture lost only the final CR.</summary>
    [Fact]
    public async Task ThreePrints_CrossAsThreeCleanLines()
    {
        var (exitCode, raw, envelope) = await RunAsync("print(\"A\"); print(\"B\"); print(\"C\")");

        Assert.Equal(0, exitCode);
        Assert.Equal(new[] { "A", "B", "C" }, Output(envelope));
        AssertWireOutputIs(raw, "\"output\":[\"A\",\"B\",\"C\"]");
    }

    /// <summary>Non-string arguments travel the same path (they are formatted and then written).
    /// The JSON encoder escapes some punctuation (\u002B for '+'), so the byte-level claim here is
    /// the absence of a terminator rather than a hand-written spelling of the escaped segment.</summary>
    [Fact]
    public async Task PrintedValues_AreAlsoFreeOfTheTerminator()
    {
        var (exitCode, raw, envelope) = await RunAsync(
            "x = symbol(\"x\"); print(x^2 + 1); print(42); print([1, 2])");

        Assert.Equal(0, exitCode);
        Assert.Equal(new[] { "x^2 + 1", "42", "[1, 2]" }, Output(envelope));
        AssertWireOutputHasNoTerminator(raw);
    }

    /// <summary>A printed BLANK line is an empty string — never a bare carriage return, which is
    /// what the old per-element trim left behind for a leading blank line.</summary>
    [Fact]
    public async Task APrintedBlankLine_IsAnEmptyElement()
    {
        var (exitCode, raw, envelope) = await RunAsync("print(\"\"); print(\"A\")");

        Assert.Equal(0, exitCode);
        Assert.Equal(new[] { "", "A" }, Output(envelope));
        AssertWireOutputIs(raw, "\"output\":[\"\",\"A\"]");
    }

    /// <summary>The positive control for the absence check: nothing in the output ever contains a
    /// CR, whatever the script prints.</summary>
    [Fact]
    public async Task NoOutputElement_EverContainsACarriageReturn()
    {
        var (_, _, envelope) = await RunAsync("print(\"A\"); print(\"B\"); print(1); print(\"\")");

        foreach (string line in Output(envelope))
            Assert.DoesNotContain('\r', line);
    }
}
