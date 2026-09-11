using Lovelace.Symbolics;
using Rl = Lovelace.Real.Real;
using Rat = Lovelace.Rational.Rational;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Regression guards for the defect the SymPy differential oracle found on its first real
/// comparison (EVD-160…EVD-164). The kernel's derivative of <c>tan(x^3)</c> is
/// <c>3*x^2*cos(x^3)^(-2)</c>, which is mathematically correct, but NUMERICALLY EVALUATING it at
/// <c>x = 1/3</c> used to return 0 where SymPy returns 0.333790999179746492481090015847. The cause
/// was <c>Lovelace.Real</c> division returning a wrong quotient when the divisor is just below 1
/// (<c>1/0.9</c> gave 1, <c>1/0.9986288849…</c> gave 0), reached through
/// <c>NumOps.Divide</c> from the negative-integer-power path.
///
/// <para>Every expected value below is SymPy 1.14.0 ground truth, obtained independently of this
/// implementation, and every comparison is digit-exact (never through a rendered string) so a
/// formatting bug can never masquerade as agreement.</para>
/// </summary>
public class NegativePowerEvaluationTests
{
    private static ExprContext Fresh(out Symbol x)
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        x = ctx.Symbol("x");
        return ctx;
    }

    private static Num At(string canonical, ExprContext ctx, Symbol x, string point)
    {
        Expr e = Printing.CanonicalParse(canonical, ctx);
        return Evaluation.EvaluateToNum(e, ctx, new Dictionary<Symbol, Num>
        {
            [x] = NumOps.FromRat(Rat.Parse(point, null)),
        });
    }

    /// <summary>Asserts |actual - sympyValue| &lt; 1e-20, reporting both sides on failure.</summary>
    private static void AssertAgrees(string label, Num actual, string sympyValue, ExprContext ctx)
    {
        Num expected = NumOps.FromReal(Rl.Parse(sympyValue, null));
        Num tolerance = NumOps.FromReal(Rl.Parse("0.00000000000000000001", null));
        Num difference = NumOps.Abs(NumOps.Subtract(actual, expected), ctx);
        Assert.True(
            NumOps.Compare(difference, tolerance) < 0,
            $"{label}: the kernel evaluated to {OracleCompare.Show(actual)}, SymPy 1.14.0 says {sympyValue}.");
    }

    /// <summary>The minimal statement of the defect: a negative integer power of a Real is its
    /// reciprocal. cos(1/27) is 0.9993142073…, so its -2 power is slightly ABOVE 1, never 0 or 1.</summary>
    [Fact]
    public void NegativeIntegerPowerOfAReal_IsTheReciprocal()
    {
        using var precision = Rl.WithPrecision(80, 40);
        ExprContext ctx = Fresh(out Symbol x);

        Num positive = At("(pow (fn cos (pow (sym x) (rat 3 1))) (rat 2 1))", ctx, x, "1/3");
        AssertAgrees("cos(1/27)^2", positive, "0.998628884998283893103390698137479406248279", ctx);

        Num negative = At("(pow (fn cos (pow (sym x) (rat 3 1))) (rat -2 1))", ctx, x, "1/3");
        AssertAgrees("cos(1/27)^-2", negative, "1.00137299753923947744327004754154925203741", ctx);
    }

    /// <summary>The whole expression the oracle disagreed on, through the same
    /// canonical-print → canonical-parse → evaluate path the oracle itself uses.</summary>
    [Fact]
    public void DerivativeOfTanOfXCubed_EvaluatesToTheSymPyValue()
    {
        using var precision = Rl.WithPrecision(80, 40);
        ExprContext ctx = Fresh(out Symbol x);

        Num value = At(
            "(mul (rat 3 1) (pow (sym x) (rat 2 1)) (pow (fn cos (pow (sym x) (rat 3 1))) (rat -2 1)))",
            ctx, x, "1/3");
        AssertAgrees("3*x^2*cos(x^3)^-2 at x = 1/3", value,
            "0.333790999179746492481090015847183084012471", ctx);
    }

    /// <summary>A quotient just above 1 is where Real division used to collapse to 1 or 0:
    /// 1/0.9 = 1.111…, while the mathematically identical 10/9 was always correct.</summary>
    [Theory]
    [InlineData("(pow (real 0.9) (rat -1 1))", "1.111111111111111111111111111111111111111")]
    [InlineData("(pow (real 0.99) (rat -1 1))", "1.010101010101010101010101010101010101010")]
    [InlineData("(pow (real 0.9993142073433579945) (rat -1 1))", "1.000686263290967473969590465527195755929")]
    [InlineData("(pow (real 0.99862888499828389309965043538706203025) (rat -1 1))", "1.001372997539239477447020588085923996879")]
    [InlineData("(pow (real 0.5) (rat -1 1))", "2.0")]
    public void ReciprocalOfARealJustBelowOne_KeepsItsQuotient(string canonical, string sympyValue)
    {
        using var precision = Rl.WithPrecision(80, 40);
        ExprContext ctx = Fresh(out Symbol x);
        AssertAgrees(canonical, At(canonical, ctx, x, "1/3"), sympyValue, ctx);
    }
}
