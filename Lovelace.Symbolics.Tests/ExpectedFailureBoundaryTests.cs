using Lovelace.Symbolics;
using Xunit;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// The expected-failure boundary (Cycle 2, P0 §2.1/§76). Every broad
/// <c>catch (Exception)</c> in the symbolic kernel was narrowed to the *typed* failure the
/// kernel actually anticipates; anything else is a defect and must propagate. These tests pin
/// that boundary from the outside: a registered function whose numeric evaluator throws an
/// unexpected exception must escape limit/fold evaluation instead of being reported as a
/// mathematical non-answer.
/// </summary>
public class ExpectedFailureBoundaryTests
{
    private const string BoomMessage = "boom: deliberately unexpected failure";

    /// <summary>A context with one function whose numeric evaluator always throws
    /// <see cref="InvalidOperationException"/> — never an <see cref="EvaluationException"/>,
    /// which is the only failure the kernel is allowed to absorb.</summary>
    private static ExprContext ContextWithThrowingFunction()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        ctx.Functions.Register(new FunctionDefinition(
            "boom",
            ctx.Function("boom"),
            1,
            numericEvaluator: (_, _) => throw new InvalidOperationException(BoomMessage)));
        return ctx;
    }

    [Fact]
    public void EvaluateToExpr_ExactArgument_ThrowingEvaluatorPropagates()
    {
        var ctx = ContextWithThrowingFunction();
        var e = Exprs.Function(ctx.Function("boom"), Exprs.Integer(2));

        var ex = Assert.Throws<InvalidOperationException>(
            () => Evaluation.EvaluateToExpr(e, ctx, new Dictionary<Symbol, Num>()));
        Assert.Equal(BoomMessage, ex.Message);
    }

    [Fact]
    public void EvaluateToExpr_ApproximateArgument_ThrowingEvaluatorPropagates()
    {
        var ctx = ContextWithThrowingFunction();
        // a Real beyond the exact-decimal window is an approximate constant, which selects the
        // approximate folding branch (the site that used to swallow every exception)
        var approx = Rl.Parse("0." + new string('1', 25), null);

        // the fold runs eagerly when the node is constructed, so both steps are inside the guard
        var ex = Assert.Throws<InvalidOperationException>(() =>
        {
            var e = Exprs.Function(ctx.Function("boom"), Exprs.Real(RealLiteral.FromRealExact(approx)));
            Evaluation.EvaluateToExpr(e, ctx, new Dictionary<Symbol, Num>());
        });
        Assert.Equal(BoomMessage, ex.Message);
    }

    [Fact]
    public void Limit_ThrowingEvaluatorPropagates()
    {
        var ctx = ContextWithThrowingFunction();
        var x = ctx.Symbol("x");
        var e = Exprs.Function(ctx.Function("boom"), Exprs.Symbol(x));

        var ex = Assert.Throws<InvalidOperationException>(
            () => Limits.Limit(e, x, Exprs.Zero, LimitDirection.TwoSided, ctx));
        Assert.Equal(BoomMessage, ex.Message);
    }

    // -----------------------------------------------------------------------
    // The narrow paths still behave exactly as before
    // -----------------------------------------------------------------------

    [Fact]
    public void Limit_TwoSidedPole_StillReportsOpposingSides()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");

        var result = Limits.Limit(Exprs.Divide(Exprs.One, Exprs.Symbol(x)), x, Exprs.Zero,
            LimitDirection.TwoSided, ctx);

        Assert.Equal(LimitStatus.DoesNotExist, result.Status);
        Assert.Equal(LimitStatus.MinusInfinity, result.FromLeft!.Status);
        Assert.Equal(LimitStatus.PlusInfinity, result.FromRight!.Status);
    }

    [Fact]
    public void TryCompare_ReportsUnorderedInsteadOfThrowing()
    {
        var real = NumOps.FromLong(2L);
        Assert.True(NumOps.TryCompare(real, NumOps.FromLong(3L), out int less));
        Assert.True(less < 0);
        Assert.True(NumOps.TryCompare(NumOps.FromLong(3L), real, out int greater));
        Assert.True(greater > 0);

        var complex = NumOps.FromComplex(new Lovelace.Complex.Complex(
            Rl.Parse("1", null), Rl.Parse("1", null)));
        Assert.False(NumOps.TryCompare(real, complex, out int unordered));
        Assert.Equal(0, unordered);
        // the throwing form keeps its kind-specific message for callers that need it
        Assert.Throws<InvalidOperationException>(() => NumOps.Compare(real, complex));
    }
}
