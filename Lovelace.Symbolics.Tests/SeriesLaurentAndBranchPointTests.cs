using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Round 20, the second half of the series work: an expansion taken across a POLE or a BRANCH POINT.
/// <para>
/// Found by the orchestrator while verifying H-1, and byte-identical on the pre-fix binary, so these are
/// pre-existing rather than regressions. On HEAD:
/// <c>series(1/x, x, 0, 2)</c> published <c>1 + O(x^2)</c> — the pole was DROPPED, so at x = 0.1 the
/// expansion said 1 where the function is 10; <c>series(1/x^2, x, 0, 2)</c> failed with
/// "Series division by a zero series."; <c>series(sin(x)/x, x, 0, 3)</c> published <c>1 + O(x^3)</c>,
/// dropping the x^2 term while still claiming x^3 accuracy; and <c>series(sqrt(x), x, 0, 2)</c>
/// published <c>1/2*x*1/sqrt(0) + O(x^2)</c> — a division by zero published as a value.
/// </para>
/// <para>
/// Two mechanisms: the fraction path never asked either operand for enough terms to survive the
/// leading-index shift (x^2's leading term is invisible in a 2-coefficient expansion; sin(x)'s x^3
/// coefficient is invisible at order 3), and <c>Series.Divide</c> dropped the <c>(la - lb)</c> shift from
/// the quotient's leading power. At a branch point the coefficients are not values at all, so the
/// expansion is not a power series and the SOURCE FUNCTION is published unchanged — which is exactly
/// SymPy 1.14.0's answer (<c>series(sqrt(x), x, 0, 2) = sqrt(x)</c>).
/// </para>
/// <para>
/// SymPy 1.14.0 ground truth, measured on the ground-truth interpreter:
/// <c>series(1/x, x, 0, 2) = 1/x</c>, <c>series(x**-2, x, 0, 2) = x**(-2)</c>,
/// <c>series(sin(x)/x, x, 0, 3) = 1 - x**2/6 + O(x**3)</c>, <c>series(sqrt(x), x, 0, 2) = sqrt(x)</c>,
/// <c>series(1/(x-1), x, 0, 2) = -1 - x + O(x**2)</c> (the control, unchanged by this landing).
/// </para>
/// </summary>
public class SeriesLaurentAndBranchPointTests
{
    private static async Task<JsonNode> EnvelopeAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exit = await Lovelace.Run.Runner.RunAsync(
            new[] { "--eval", script, "--json" }, stdout, stderr);
        JsonNode? envelope = JsonNode.Parse(stdout.ToString());
        Assert.True(envelope is not null, $"'{script}' produced no envelope (exit {exit}). stderr: {stderr}");
        return envelope!;
    }

    /// <summary>Each row is the expansion SymPy publishes, as the published display shows it. A Laurent
    /// tail is a NEGATIVE power (the value is a value, not a refusal) and a branch point publishes the
    /// function itself.</summary>
    [Theory]
    [InlineData("series(1/x, x, 0, 2)", "x^-1 + O(x^2)")]
    [InlineData("series(1/x^2, x, 0, 2)", "x^-2 + O(x^2)")]
    [InlineData("series(sin(x)/x, x, 0, 3)", "1 - 1/6*x^2 + O(x^3)")]
    [InlineData("series(sqrt(x), x, 0, 2)", "sqrt(x)")]
    [InlineData("series(sqrt(abs(x)), x, 0, 2)", "sqrt(x)")]
    public async Task Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion(string call, string expected)
    {
        JsonNode envelope = await EnvelopeAsync($"x = symbol(\"x\"); {call}");

        Assert.True(envelope["ok"]!.GetValue<bool>(), $"{call} must answer: {envelope["message"]}");
        Assert.Equal(expected, envelope["result"]!["display"]!.GetValue<string>());
        // the branch-point path used to publish a division by zero; nothing in the envelope may carry it
        Assert.DoesNotContain("sqrt(0)", envelope.ToJsonString());
        Assert.DoesNotContain("/0", envelope.ToJsonString());
    }

    /// <summary>The denominator path is not uniformly broken, and this landing must not move it: an
    /// expansion whose leading term is a constant keeps the value it always had.</summary>
    [Fact]
    public async Task Series_OfARemovableDenominator_IsUnchanged()
    {
        JsonNode envelope = await EnvelopeAsync("x = symbol(\"x\"); series(1/(x - 1), x, 0, 2)");

        Assert.True(envelope["ok"]!.GetValue<bool>(), $"it must answer: {envelope["message"]}");
        Assert.Equal("-x + O(x^2) - 1", envelope["result"]!["display"]!.GetValue<string>());
    }
}
