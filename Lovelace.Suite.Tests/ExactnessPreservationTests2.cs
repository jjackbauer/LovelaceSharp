using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Exactness is preserved end-to-end (Cycle 2 item 7). The payload form is one thing; what an agent
/// relies on is that exactness survives the operations it applies. The engine loads the same
/// plugins the hosts do — without them the language has no evalf/subs.
/// </summary>
public class ExactnessPreservationTests2
{
    private static StructuredValueDto Run(string source)
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return StructuredProjection.ToStructured(engine.Evaluate(source));
    }

    [Theory]
    [InlineData("1/3 * 3")]
    [InlineData("1/3 - 1/3")]
    [InlineData("1/6 + 1/3")]
    [InlineData("1/3 + 0")]
    public void Arithmetic_DoesNotDegradeExactness(string source)
    {
        var sv = Run(source);
        Assert.True(sv.Exact, source + " lost exactness: kind=" + sv.Kind + " value=" + sv.Value);
    }

    [Fact]
    public void Substitution_PreservesTheExactRational()
    {
        var sv = Run("x = symbol(\"x\"); subs(1/3, x, 2)");
        Assert.Equal("Symbolic", sv.Kind);
        Assert.True(sv.Exact);
        Assert.Equal("(rat 1 3)", sv.Canonical);
    }

    [Fact]
    public void ExactRationalTimesInteger_StaysExact()
    {
        var sv = Run("1/3 * 3");
        Assert.True(sv.Exact);
        Assert.Contains("1", sv.Value ?? sv.Pretty ?? "");
    }
}
