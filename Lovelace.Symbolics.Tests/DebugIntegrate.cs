using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

public class DebugIntegrate
{
    [Fact]
    public void RationalDenominator_IntegratesWithSelfVerification()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
        var e = Exprs.Divide(1, Exprs.Subtract(Exprs.Power(x, 2), 1));
        var result = Integration.Integrate(e, x, ctx);
        Assert.NotEqual(Exprs.Integral(e, x), result);   // not the unevaluated fallback
        var d = Calculus.Diff(result, x, ctx);
        Assert.NotEqual(Exprs.Integral(d, x), result);   // sanity
    }
}
