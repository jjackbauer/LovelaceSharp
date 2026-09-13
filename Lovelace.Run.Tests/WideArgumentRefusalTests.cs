using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// Audit O (wave 5, round 23) found four refusals that still crossed as RAW CLR text — the defect
/// class this cycle keeps closing, and the one K-1 closed for <c>pi(n)</c>/<c>e(n)</c> four lines
/// above the <c>setprecision</c> registration:
/// <list type="bullet">
/// <item><c>setprecision(10^30)</c> — "Value was either too large or too small for an Int64."</item>
/// <item><c>series(sin(x), x, 0, -1)</c> — "Specified argument was out of the range of valid values. (Parameter 'order')"</item>
/// <item><c>[1, 2, 3][10^30]</c> — the same raw Int64 text</item>
/// <item><c>zeros(-1)</c> / <c>zeros(2, -1)</c> — "Arithmetic operation resulted in an overflow."</item>
/// </list>
/// Each is an ARGUMENT problem, so each must cross as <c>InvalidArgument</c>/<c>TypeMismatch</c> with a
/// message naming the value and the bound — the grammar <c>pi(digits)</c> already uses.
/// </summary>
public class WideArgumentRefusalTests
{
    private static async Task<(int ExitCode, JsonNode Envelope)> RunAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(
            new[] { "--eval", script, "--omit-functions", "--omit-variables" }, stdout, stderr);
        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(stdout.ToString(), $"stdout of '{script}'"));
    }

    private static async Task<(string Code, string Category, string Message)> RefusalAsync(string script)
    {
        var (exitCode, envelope) = await RunAsync(script);
        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"the script answered: {script}");
        return (envelope["code"]!.GetValue<string>(),
                envelope["category"]!.GetValue<string>(),
                envelope["message"]!.GetValue<string>());
    }

    private const string HugeCount = "123456789012345678901234567890";
    private const string Int64Bound = "9223372036854775807";

    /// <summary>The raw CLR text of a conversion failure — what none of these messages may be.</summary>
    private static void AssertNotRawClrText(string message)
    {
        Assert.DoesNotContain("Int64", message);
        Assert.DoesNotContain("overflow", message, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("Parameter 'order'", message);
    }

    [Fact]
    public async Task Setprecision_WithACountWiderThanInt64_IsAnArgumentRefusal()
    {
        var (code, category, message) = await RefusalAsync($"setprecision({HugeCount})");

        Assert.Equal("InvalidArgument", code);
        Assert.Equal("TypeMismatch", category);
        Assert.Contains(HugeCount, message);
        Assert.Contains(Int64Bound, message);
        AssertNotRawClrText(message);
    }

    [Fact]
    public async Task Zeros_WithANegativeDimension_IsAnArgumentRefusal()
    {
        var (code, category, message) = await RefusalAsync("zeros(-1)");

        Assert.Equal("InvalidArgument", code);
        Assert.Equal("TypeMismatch", category);
        Assert.Contains("-1", message);
        AssertNotRawClrText(message);
    }

    [Fact]
    public async Task Zeros_WithANegativeTrailingDimension_NamesThePositionAndTheValue()
    {
        var (code, category, message) = await RefusalAsync("zeros(2, -1)");

        Assert.Equal("InvalidArgument", code);
        Assert.Equal("TypeMismatch", category);
        Assert.Contains("argument 2", message);
        Assert.Contains("-1", message);
        AssertNotRawClrText(message);
    }

    [Fact]
    public async Task Indexing_WithAnIndexWiderThanInt64_IsAnArgumentRefusal()
    {
        var (code, category, message) = await RefusalAsync($"[1, 2, 3][{HugeCount}]");

        Assert.Equal("InvalidArgument", code);
        Assert.Equal("TypeMismatch", category);
        Assert.Contains(HugeCount, message);
        Assert.Contains(Int64Bound, message);
        AssertNotRawClrText(message);
    }

    [Fact]
    public async Task Series_WithANonPositiveOrder_IsAnArgumentRefusal()
    {
        var (code, category, message) = await RefusalAsync("x = symbol(\"x\"); series(sin(x), x, 0, -1)");

        Assert.Equal("InvalidArgument", code);
        Assert.Equal("TypeMismatch", category);
        Assert.Contains("-1", message);
        AssertNotRawClrText(message);
    }

    [Fact]
    public async Task Series_WithAnOrderWiderThanInt64_IsAnArgumentRefusal()
    {
        var (code, category, message) =
            await RefusalAsync($"x = symbol(\"x\"); series(sin(x), x, 0, {HugeCount})");

        Assert.Equal("InvalidArgument", code);
        Assert.Equal("TypeMismatch", category);
        Assert.Contains(HugeCount, message);
        AssertNotRawClrText(message);
    }

    // ---- controls: the behaviour that must NOT move -------------------------------------------

    /// <summary>A zero dimension was and stays legal (an empty array).</summary>
    [Fact]
    public async Task Zeros_WithZero_StillAnswersAnEmptyArray()
    {
        var (exitCode, envelope) = await RunAsync("zeros(0)");

        Assert.Equal(0, exitCode);
        Assert.True(envelope["ok"]!.GetValue<bool>());
        Assert.Equal("[]", envelope["result"]!["display"]!.GetValue<string>());
    }

    /// <summary>A NEGATIVE index that fits is still the documented out-of-range refusal with its
    /// numbers — this fix is about width, not about that message.</summary>
    [Fact]
    public async Task Indexing_WithANegativeIndex_KeepsItsDocumentedRefusal()
    {
        var (code, _, message) = await RefusalAsync("[1, 2, 3][-1]");

        Assert.Equal("InvalidOperation", code);
        Assert.Contains("Index -1 is out of range for vector of length 3.", message);
    }

    /// <summary>A non-positive count that FITS is still the existing refusal, naming the number.</summary>
    [Fact]
    public async Task Setprecision_WithANonPositiveCount_KeepsItsDocumentedRefusal()
    {
        var (code, _, message) = await RefusalAsync("setprecision(-5)");

        Assert.Equal("InvalidOperation", code);
        Assert.Contains("positive digit count", message);
        Assert.Contains("-5", message);
    }

    /// <summary>The typed digit refusal K-1 landed is unchanged by this one.</summary>
    [Fact]
    public async Task Pi_WithACountWiderThanInt64_StillNamesTheCountAndTheCap()
    {
        var (code, category, message) = await RefusalAsync($"pi({HugeCount})");

        Assert.Equal("InvalidArgument", code);
        Assert.Equal("TypeMismatch", category);
        Assert.Contains(HugeCount, message);
    }
}
