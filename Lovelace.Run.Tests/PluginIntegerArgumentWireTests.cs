using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// ADV-02 (advancement cycle 7): the WIRE contract for the two integer-argument defects this package
/// closes. Both were measured on the Native AOT runner published from <c>ce30fc3</c>:
///
/// <list type="bullet">
/// <item><c>evalir(compile(x*1/3, [x]), [1], 2.9)</c> answered <c>ok: true</c>, <c>output: ["0.33"]</c>
/// — 2.9 became 2 — and <c>digits = 0</c> / <c>digits = -5</c> both answered <c>ok: true</c> with
/// <c>0</c>;</item>
/// <item><c>evalir(..., 10^30)</c> crossed as <c>ArithmeticError</c>/<c>DomainError</c> with the raw
/// CLR sentence "Value was either too large or too small for an Int64.", naming no number;</item>
/// <item><c>movingavg([1,2,3,4,5], 3.7)</c> crossed as <c>InvalidOperation</c>/<c>DomainError</c> —
/// the pair a genuine DOMAIN refusal uses.</item>
/// </list>
///
/// <para>This file reads only fields the protocol already published (it references no type introduced
/// by this package), so it is also the control: on the unmodified tree it fails at RUNTIME, on the
/// code and category, rather than at compile time.</para>
/// </summary>
public class PluginIntegerArgumentWireTests
{
    private static async Task<(int ExitCode, JsonNode Envelope)> RunAsync(string script)
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(
            script, "--omit-functions", "--omit-variables");
        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(stdout, $"stdout of '{script}'"));
    }

    // ------------------------------------------------------------------
    // 1. the control: an integral digit count still answers the requested width
    // ------------------------------------------------------------------

    [Fact]
    public async Task AnIntegralDigitCount_StillAnswersTheRequestedWidth()
    {
        var (exit, envelope) = await RunAsync(
            "x = symbol(\"x\"); print(evalir(compile(x*1/3, [x]), [1], 40))");

        Assert.Equal(0, exit);
        Assert.True(envelope["ok"]!.GetValue<bool>());
        Assert.Equal(
            "0.3333333333333333333333333333333333333333",
            envelope["output"]!.AsArray()[0]!.GetValue<string>());
    }

    // ------------------------------------------------------------------
    // 2. a non-integral digit count is a recoverable ARGUMENT error, never a value
    // ------------------------------------------------------------------

    /// <summary>The finding. Before the fix this script exited 0 with <c>output: ["0.33"]</c>: the
    /// machine API altered the caller's precision and reported success.</summary>
    [Fact]
    public async Task ANonIntegralDigitCount_IsRefusedRatherThanFloored()
    {
        var (exit, envelope) = await RunAsync(
            "x = symbol(\"x\"); print(evalir(compile(x*1/3, [x]), [1], 2.9))");

        Assert.Equal(1, exit);
        Assert.False(envelope["ok"]!.GetValue<bool>(), "2.9 digits is not a request the engine can honour");
        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>());
        // the value the caller actually wrote is named, so the repair is readable
        Assert.Contains("2.9", envelope["message"]!.GetValue<string>());
        // and it is NOT the internal-invariant pair
        Assert.NotEqual("InternalError", envelope["code"]!.GetValue<string>());
        Assert.NotEqual("InternalInvariantFailure", envelope["category"]!.GetValue<string>());
    }

    /// <summary>A count outside the declared bound: <c>0</c> and a negative count used to be
    /// accepted and answered a plausible <c>0</c>.</summary>
    [Theory]
    [InlineData("0")]
    [InlineData("-5")]
    public async Task ADigitCountOutsideTheBound_IsRefused(string digits)
    {
        var (exit, envelope) = await RunAsync(
            $"x = symbol(\"x\"); print(evalir(compile(x*1/3, [x]), [1], {digits}))");

        Assert.Equal(1, exit);
        Assert.False(envelope["ok"]!.GetValue<bool>());
        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
        Assert.Contains(digits, envelope["message"]!.GetValue<string>());
    }

    /// <summary>The raw-CLR half of the finding: a count wider than Int64 used to cross as
    /// <c>ArithmeticError</c> with a sentence that named no number.</summary>
    [Fact]
    public async Task ADigitCountWiderThanInt64_IsRefusedByTheArgumentContract_AndIsNamed()
    {
        const string tooWide = "1000000000000000000000000000000";

        var (exit, envelope) = await RunAsync(
            $"x = symbol(\"x\"); print(evalir(compile(x*1/3, [x]), [1], 10^30))");

        Assert.Equal(1, exit);
        Assert.False(envelope["ok"]!.GetValue<bool>());
        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
        string message = envelope["message"]!.GetValue<string>();
        Assert.Contains(tooWide, message);
        Assert.DoesNotContain("too large or too small", message);
    }

    /// <summary><c>evalir_batch</c> carries the same argument and must carry the same contract.</summary>
    [Fact]
    public async Task EvalirBatch_CarriesTheSameContract()
    {
        var (exit, envelope) = await RunAsync(
            "x = symbol(\"x\"); print(evalir_batch(compile(x*1/3, [x]), [1, 2], 2.5))");

        Assert.Equal(1, exit);
        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
        Assert.Contains("2.5", envelope["message"]!.GetValue<string>());
    }

    // ------------------------------------------------------------------
    // 3. the DSP integer arguments are argument errors, not DOMAIN failures
    // ------------------------------------------------------------------

    /// <summary>The measured shape: <c>movingavg(x, 3.7)</c> crossed as
    /// <c>InvalidOperation</c>/<c>DomainError</c>. A caller cannot fix mathematics here — it wrote a
    /// window of 3.7 — so the pair was wrong.</summary>
    [Fact]
    public async Task ANonIntegralWindow_IsAnArgumentError_NotADomainFailure()
    {
        var (exit, envelope) = await RunAsync("print(movingavg([1, 2, 3, 4, 5], 3.7))");

        Assert.Equal(1, exit);
        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>());
        Assert.Contains("movingavg", envelope["message"]!.GetValue<string>());
        Assert.Contains("3.7", envelope["message"]!.GetValue<string>());
    }

    /// <summary>A count that does not fit the type keeps its own, wider refusal — and names the
    /// value.</summary>
    [Fact]
    public async Task ADelayWiderThanInt64_IsTheSameTypedRefusal()
    {
        const string tooWide = "1000000000000000000000000000000";

        var (exit, envelope) = await RunAsync($"print(delay([1, 2, 3, 4], {tooWide}))");

        Assert.Equal(1, exit);
        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
        Assert.Contains(tooWide, envelope["message"]!.GetValue<string>());
    }

    /// <summary>A negative sample count is an argument error against the parameter's own bound.</summary>
    [Fact]
    public async Task ANegativeSampleCount_IsAnArgumentError()
    {
        var (exit, envelope) = await RunAsync("print(impulse(-1))");

        Assert.Equal(1, exit);
        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
        Assert.Contains("-1", envelope["message"]!.GetValue<string>());
    }

    // ------------------------------------------------------------------
    // 4. the controls the repair must not have narrowed
    // ------------------------------------------------------------------

    /// <summary>Every accepted DSP shape still answers, so the repair narrowed nothing.</summary>
    [Theory]
    [InlineData("print(movingavg([1, 2, 3, 4, 5], 3))", "[0.(3), 1, 2, 3, 4]")]   // causal, zero-padded window
    [InlineData("print(delay([1, 2, 3, 4], 1))", "[0, 1, 2, 3]")]
    [InlineData("print(impulse(3))", "[1, 0, 0]")]
    [InlineData("print(len(step(4)))", "4")]
    public async Task TheAcceptedShapes_StillAnswer(string script, string expected)
    {
        var (exit, envelope) = await RunAsync(script);

        Assert.Equal(0, exit);
        Assert.True(envelope["ok"]!.GetValue<bool>(), envelope["message"]?.GetValue<string>());
        Assert.Equal(expected, envelope["output"]!.AsArray()[0]!.GetValue<string>());
    }
}
