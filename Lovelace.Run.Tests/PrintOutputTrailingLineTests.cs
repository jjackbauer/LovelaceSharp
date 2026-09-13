using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// Audit N (wave 5, finding N-1, P1): the LAST line of captured print output was dropped while the
/// same statement's <c>hasOutput</c> stayed true. <c>Runner.SplitLines</c> stripped EVERY trailing
/// newline (<c>text.TrimEnd('\r','\n')</c>) instead of exactly the one terminator the capture path
/// appends, so a script that printed a blank line last lost it — while an INTERIOR blank line was
/// kept, which is what makes the old behaviour a defect rather than a convention, and what makes the
/// envelope's own <c>hasOutput</c> flag contradict its <c>output</c> array.
/// The two Studio hosts (<c>EngineHost.SplitLines</c>, <c>IncrementalRunner.SplitLines</c>) already
/// normalise and then drop exactly one trailing empty element, so these tests also pin the CLI to
/// the answer the hosts already give.
/// </summary>
public class PrintOutputTrailingLineTests
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

    private static bool[] HasOutput(JsonNode envelope) =>
        envelope["timings"]!.AsArray().Select(t => t!["hasOutput"]!.GetValue<bool>()).ToArray();

    /// <summary>The serialized output array exactly as it appears on stdout.</summary>
    private static string OutputSegment(string raw)
    {
        int start = raw.IndexOf("\"output\":", StringComparison.Ordinal);
        Assert.True(start >= 0, $"the envelope has no output array: {raw}");
        int end = raw.IndexOf(']', start);
        Assert.True(end > start, $"the output array is unterminated: {raw}");
        return raw.Substring(start, end - start + 1);
    }

    /// <summary>The defect itself: the trailing blank line the script printed is an element, and the
    /// statement that printed it SAYS it printed (so the two machine-readable claims agree).</summary>
    [Fact]
    public async Task ATrailingBlankPrint_IsKept_AndItsStatementSaysItPrinted()
    {
        var (exitCode, raw, envelope) = await RunAsync("print(\"a\"); print()");

        Assert.Equal(0, exitCode);
        Assert.Equal(new[] { "a", "" }, Output(envelope));
        Assert.Equal(new[] { true, true }, HasOutput(envelope));
        Assert.Equal("\"output\":[\"a\",\"\"]", OutputSegment(raw));
    }

    /// <summary>Every trailing blank line, not only the first: the old splitter removed all of them.</summary>
    [Fact]
    public async Task TwoTrailingBlankPrints_AreBothKept()
    {
        var (exitCode, _, envelope) = await RunAsync("print(\"a\"); print(); print()");

        Assert.Equal(0, exitCode);
        Assert.Equal(new[] { "a", "", "" }, Output(envelope));
        Assert.Equal(new[] { true, true, true }, HasOutput(envelope));
    }

    /// <summary>Control: a single blank print was already one empty element, and must stay one —
    /// the fix removes exactly one terminator, so it must not invent or drop an element here.</summary>
    [Fact]
    public async Task ASingleBlankPrint_IsStillOneEmptyElement()
    {
        var (exitCode, _, envelope) = await RunAsync("print()");

        Assert.Equal(0, exitCode);
        Assert.Equal(new[] { "" }, Output(envelope));
    }

    /// <summary>Control: an INTERIOR blank line was always kept; the fix must not change it.</summary>
    [Fact]
    public async Task AnInteriorBlankPrint_IsUnchanged()
    {
        var (exitCode, _, envelope) = await RunAsync("print(\"a\"); print(\"\"); print(\"b\")");

        Assert.Equal(0, exitCode);
        Assert.Equal(new[] { "a", "", "b" }, Output(envelope));
    }

    /// <summary>Control for the other direction: an ordinary last line must NOT gain a phantom empty
    /// element, i.e. exactly one terminator is removed and no more.</summary>
    [Fact]
    public async Task ALonePrint_HasNoTrailingEmptyElement()
    {
        var (exitCode, _, envelope) = await RunAsync("print(\"a\")");

        Assert.Equal(0, exitCode);
        Assert.Equal(new[] { "a" }, Output(envelope));
    }

    /// <summary>The failing path shares the capture and the splitter (the envelope keeps what the
    /// run printed before it failed), so it carried the same loss.</summary>
    [Fact]
    public async Task TheFailingPath_KeepsItsTrailingBlankLine()
    {
        var (exitCode, _, envelope) = await RunAsync("print(\"a\"); print(); det(1)");

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>());
        Assert.Equal(new[] { "a", "" }, Output(envelope));
        Assert.Equal(new[] { true, true, false }, HasOutput(envelope));
    }

    /// <summary>The invariant the defect broke, stated directly: for a script whose printing
    /// statements print exactly one line each, the number of statements that say they printed equals
    /// the number of lines the envelope carries. The script ends on a blank print so the invariant
    /// is the defect's own shape, not a control.</summary>
    [Fact]
    public async Task PrintingStatements_AndOutputLines_AgreeInCount()
    {
        var (_, _, envelope) = await RunAsync("print(\"a\"); print(); print(\"b\"); print()");

        Assert.Equal(new[] { "a", "", "b", "" }, Output(envelope));
        Assert.Equal(HasOutput(envelope).Count(printed => printed), Output(envelope).Length);
    }
}
