using Lovelace.Suite;
using Xunit;

namespace Lovelace.Symbolics.Tests;

public class SuiteIntegrationTests
{
    [Fact]
    public void Symbol_Builtin_Works()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new Lovelace.Symbolics.SymbolicsPlugin());
        var result = engine.Evaluate("z = symbol(\"x\")");
        Assert.Equal(ValueKind.Symbolic, result.Kind);
    }
}
