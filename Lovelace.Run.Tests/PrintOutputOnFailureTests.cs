using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// docs/symbolics/dsh-protocol.md:9-10 — invariant 1: "stdout carries the envelope and nothing
/// else. Anything the script prints with print(...) is captured and returned in the top-level
/// output array." The sentence is not scoped to a successful run, and the runner's own header
/// states it the same way (Lovelace.Run/Runner.cs:21-23, Lovelace.Run/Program.cs:10-13).
///
/// Cycle-6 audit E filed the violation as F5 (P1): <c>print("hello"); det(1)</c> answers exit 1
/// with an envelope whose key set carries NO output array — the line the script printed is gone,
/// although the SAME envelope's <c>timings[0].hasOutput</c> is true, i.e. the capture happened.
/// The recorded-but-never-closed cycle-5 item is A2-wire F9.
///
/// The contract these tests pin: every envelope the runner emits carries the top-level
/// <c>output</c> array, holding exactly the lines the script printed before the run ended — all of
/// them on success, the ones committed before the failure on a failure, and never one that did not
/// run. The cancelled envelope additionally keeps its already-published <c>partialOutput</c> (the
/// same lines), so no recorded shape regresses.
/// </summary>
public class PrintOutputOnFailureTests
{
    private static async Task<(int ExitCode, string Raw, JsonNode Envelope)> RunAsync(
        string script, params string[] extraArguments)
    {
        var arguments = new List<string> { "--eval", script, "--omit-functions", "--omit-variables" };
        arguments.AddRange(extraArguments);

        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(arguments.ToArray(), stdout, stderr);
        string raw = stdout.ToString();
        return (exitCode, raw, TestSupport.ParseExactlyOneJsonDocument(raw, $"stdout of '{script}'"));
    }

    /// <summary>The top-level output array, read as the LINES the protocol says it carries.</summary>
    private static string[] Output(JsonNode envelope) =>
        envelope["output"]!.AsArray().Select(node => node!.GetValue<string>()).ToArray();

    /// <summary>
    /// The finding itself: the line printed by the statement BEFORE the failing one must cross in
    /// the failure envelope's output array. Pre-fix the key is absent, so this fails on the NotNull
    /// assertion — the envelope is the whole stdout, so there is nowhere else for the line to be.
    /// </summary>
    [Fact]
    public async Task AFailingRun_KeepsTheLinePrintedBeforeTheFailure()
    {
        var (exitCode, raw, envelope) = await RunAsync("print(\"hello\"); det(1)");

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"a failing run reports ok == false. stdout: {raw}");
        Assert.NotNull(envelope["output"]);
        Assert.Equal(new[] { "hello" }, Output(envelope));
        Assert.Contains("\"output\":[\"hello\"]", raw);

        // the capture the audit's deciding field points at: the printing statement DID write output
        Assert.True(envelope["timings"]![0]!["hasOutput"]!.GetValue<bool>(),
            $"the printing statement must report hasOutput. stdout: {raw}");
    }

    /// <summary>
    /// "Only what was printed BEFORE the failure": the print after the failing statement never runs,
    /// so its text must appear NOWHERE in the envelope. The marker is deliberately a word no message
    /// of this run can spell.
    /// </summary>
    [Fact]
    public async Task AFailingRun_KeepsNothingThatNeverRan()
    {
        var (exitCode, raw, envelope) = await RunAsync(
            "print(\"before\"); print(\"also before\"); det(1); print(\"UNREACHED\")");

        Assert.Equal(1, exitCode);
        Assert.Equal(new[] { "before", "also before" }, Output(envelope));
        Assert.DoesNotContain("UNREACHED", raw);
    }

    /// <summary>
    /// The key is the contract, not merely the data: a failure that printed nothing still carries
    /// the array (empty), exactly as the success envelope does — a consumer reads one key on both
    /// paths instead of testing for its existence.
    /// </summary>
    [Fact]
    public async Task AFailingRun_ThatPrintedNothing_CarriesAnEmptyOutputArray()
    {
        var (exitCode, raw, envelope) = await RunAsync("det(1)");

        Assert.Equal(1, exitCode);
        Assert.NotNull(envelope["output"]);
        Assert.Empty(envelope["output"]!.AsArray());
        Assert.False(envelope["timings"]![0]!["hasOutput"]!.GetValue<bool>(), raw);
    }

    /// <summary>
    /// A parse failure ends the run before the FIRST statement executes, so the print in the script
    /// produced nothing and the array must be empty — never the source text, never a stale line.
    /// </summary>
    [Fact]
    public async Task AFailingRun_OnAParseFailure_CarriesAnEmptyOutputArray()
    {
        var (exitCode, raw, envelope) = await RunAsync("print(\"UNREACHED\"); 1+1;\n)");

        Assert.Equal(1, exitCode);
        Assert.NotNull(envelope["output"]);
        Assert.Empty(envelope["output"]!.AsArray());
        Assert.DoesNotContain("UNREACHED", raw);
    }

    // ------------------------------------------------------------------
    // Controls: the paths that already carried their output must not move
    // ------------------------------------------------------------------

    /// <summary>The success path: the same key, the same order, the same array.</summary>
    [Fact]
    public async Task TheSuccessPath_IsUnchanged()
    {
        var (exitCode, raw, envelope) = await RunAsync("print(\"hello\"); 1+1");

        Assert.Equal(0, exitCode);
        Assert.True(envelope["ok"]!.GetValue<bool>(), raw);
        Assert.Equal(new[] { "hello" }, Output(envelope));
        Assert.Contains("\"output\":[\"hello\"]", raw);
    }

    /// <summary>
    /// The cancelled envelope keeps the recorded cycle-3/4/5 shape (partialOutput carrying the
    /// committed lines) AND satisfies invariant 1 in the documented key — the two are the same lines.
    /// </summary>
    [Fact]
    public async Task TheCancelledPath_KeepsPartialOutput_AndCarriesTheDocumentedOutput()
    {
        var (exitCode, raw, envelope) = await RunAsync(
            "x = symbol(\"x\"); print(\"hello\"); sum(1..10000000)", "--cancel-after", "100");

        Assert.Equal(1, exitCode);
        Assert.Equal("Cancelled", envelope["code"]!.GetValue<string>());
        Assert.Equal("BudgetExceeded", envelope["category"]!.GetValue<string>());
        Assert.Equal(new[] { "hello" },
            envelope["partialOutput"]!.AsArray().Select(node => node!.GetValue<string>()).ToArray());
        Assert.Equal(new[] { "hello" }, Output(envelope));
    }
}
