using Lovelace.Suite;
using Xunit;

namespace Lovelace.Symbolics.Tests;

public class SuiteIntegrationTests2
{
    [Fact]
    public void Apart_ThroughEngine()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new Lovelace.Symbolics.SymbolicsPlugin());
        engine.Evaluate("x = symbol(\"x\")");
        var r = engine.Evaluate("apart(1/(x^2 - 1), x)");
        Assert.Equal(ValueKind.Symbolic, r.Kind);
    }
}
