using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Exact-rational payload (Cycle 2 item 7, decision D1): an exact Real crosses as
/// {value, exact, numerator, denominator} so an agent never parses "0.(3)" back into 1/3, while a
/// truncated approximation advertises exact=false and carries no rational form. The payload
/// vocabulary is unchanged — fields are added to the Real form, not a new ValueKind.
/// The engine loads the same plugins the hosts do: without them the language has no evalf.
/// </summary>
public class ExactRationalPayloadTests2
{
    private static StructuredValueDto Structured(string source)
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return StructuredProjection.ToStructured(engine.Evaluate(source));
    }

    [Fact]
    public void ExactRational_CarriesNumeratorAndDenominator()
    {
        var sv = Structured("1/3");
        Assert.Equal("Real", sv.Kind);
        Assert.True(sv.Exact);
        Assert.Equal("1", sv.Numerator);
        Assert.Equal("3", sv.Denominator);
    }

    [Fact]
    public void ExactArithmetic_StaysExactAndRational()
    {
        var sv = Structured("1/3 + 1/6");
        Assert.True(sv.Exact);
        Assert.Equal("1", sv.Numerator);
        Assert.Equal("2", sv.Denominator);
    }

    [Fact]
    public void Approximation_CarriesNoRationalFormAndIsNotExact()
    {
        var sv = Structured("evalf(1/3, 50)");
        Assert.Equal("Real", sv.Kind);
        Assert.False(sv.Exact);
        Assert.Null(sv.Numerator);
        Assert.Null(sv.Denominator);
    }
}
