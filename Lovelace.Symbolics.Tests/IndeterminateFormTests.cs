using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// N18: an indeterminate form must never fold to a definite value.
/// <para>
/// <c>a - a = 0</c> and <c>0 * a = 0</c> are identities over DEFINED, FINITE values only.
/// With <c>a</c> bound to the symbolic constant Infinity they are indeterminate, so the
/// zero-coefficient / zero-factor guards must keep the form visible instead of erasing it.
/// The positive controls pin the identities for symbols and for finite named constants
/// (Pi), which must keep folding.
/// </para>
/// </summary>
public class IndeterminateFormTests
{
    private static ExprContext NewCtx()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx;
    }

    /// <summary>A definite zero: the folded rational constant 0.</summary>
    private static bool IsDefiniteZero(Expr e) => e is RationalConstantExpr rc && rc.Value.IsZero;

    private static bool MentionsInfinity(Expr e) => e switch
    {
        NamedConstantExpr n => n.Constant == NamedConstant.Infinity,
        AddExpr a => a.Terms.Any(MentionsInfinity),
        MultiplyExpr m => m.Factors.Any(MentionsInfinity),
        PowerExpr p => MentionsInfinity(p.Base) || MentionsInfinity(p.Exponent),
        FunctionExpr f => f.Arguments.Any(MentionsInfinity),
        _ => false,
    };

    private static string Describe(Expr e) => Printing.PrettyPrint(e);

    // ------------------------------------------------------------------
    // N18: the defect — indeterminate forms must not return a definite 0
    // ------------------------------------------------------------------

    [Fact]
    public void InfinityMinusInfinity_IsNotDefiniteZero()
    {
        NewCtx();
        var e = Exprs.Subtract(Exprs.Infinity, Exprs.Infinity);
        Assert.False(IsDefiniteZero(e),
            "inf - inf is indeterminate and must not fold to a definite 0; got: " + Describe(e));
        Assert.True(MentionsInfinity(e),
            "the infinity operand must survive the fold instead of being erased; got: " + Describe(e));
        // chosen outcome: the canonical indeterminate product 0·inf, exactly the treatment
        // 0/x already gets (the zero coefficient stays visible over a hazardous remainder)
        Assert.Equal("0*inf", Describe(e));
    }

    [Fact]
    public void ZeroTimesInfinity_IsNotDefiniteZero()
    {
        NewCtx();
        var e = Exprs.Multiply(Exprs.Zero, Exprs.Infinity);
        Assert.False(IsDefiniteZero(e),
            "0 * inf is indeterminate and must not fold to a definite 0; got: " + Describe(e));
        Assert.True(MentionsInfinity(e),
            "the infinity factor must survive the fold instead of being erased; got: " + Describe(e));
        Assert.Equal("0*inf", Describe(e));
    }

    [Fact]
    public void InfinityPlusOne_StaysUnevaluated()
    {
        // Control: the honest treatment that inf - inf and 0 * inf must match.
        NewCtx();
        var e = Exprs.Add(Exprs.Infinity, Exprs.One);
        Assert.False(IsDefiniteZero(e), "inf + 1 has no numeric value; got: " + Describe(e));
        Assert.True(MentionsInfinity(e), "inf must survive; got: " + Describe(e));
    }

    [Fact]
    public void NestedInfinityDifference_IsNotDefiniteZero()
    {
        // Same hole one level down: the guard must look through the operand structure.
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var sum = Exprs.Add(Exprs.Infinity, x);
        var e = Exprs.Subtract(sum, sum);
        Assert.False(IsDefiniteZero(e),
            "(inf + x) - (inf + x) is indeterminate and must not fold to a definite 0; got: " + Describe(e));
    }

    [Fact]
    public void IndeterminateDifferenceEmbeddedInSum_DoesNotResurrectZero()
    {
        // The Add path re-applies the same guard: (inf - inf) + 1 must not become 1.
        NewCtx();
        var e = Exprs.Add(Exprs.Subtract(Exprs.Infinity, Exprs.Infinity), Exprs.One);
        Assert.False(IsDefiniteZero(e), "(inf - inf) + 1 must not fold to 1; got: " + Describe(e));
        Assert.True(MentionsInfinity(e), "inf must survive; got: " + Describe(e));
    }

    // ------------------------------------------------------------------
    // Forbidden-regression guards: the identities stay valid where they are valid
    // ------------------------------------------------------------------

    [Fact]
    public void SymbolMinusItself_StillFoldsToZero()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        Assert.Equal(Exprs.Zero, Exprs.Subtract(x, x));
    }

    [Fact]
    public void ZeroTimesSymbol_StillFoldsToZero()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        Assert.Equal(Exprs.Zero, Exprs.Multiply(Exprs.Zero, x));
    }

    [Fact]
    public void FiniteNamedConstantIdentities_StillFold()
    {
        // Pi is named but finite: pi - pi = 0 and 0 * pi = 0 remain correct.
        NewCtx();
        Assert.Equal(Exprs.Zero, Exprs.Subtract(Exprs.Pi, Exprs.Pi));
        Assert.Equal(Exprs.Zero, Exprs.Multiply(Exprs.Zero, Exprs.Pi));
        Assert.Equal(Exprs.Zero, Exprs.Subtract(Exprs.E, Exprs.E));
    }

    // ------------------------------------------------------------------
    // Sibling identities in the same fold code (N18 sweep)
    // ------------------------------------------------------------------

    [Fact]
    public void Sibling_InfinityOverInfinity_StaysUnevaluated()
    {
        NewCtx();
        var e = Exprs.Divide(Exprs.Infinity, Exprs.Infinity);
        Assert.False(IsDefiniteZero(e), "inf/inf must not be 0; got: " + Describe(e));
        Assert.False(e is RationalConstantExpr rc && rc.Value.IsOne,
            "inf/inf must not be 1; got: " + Describe(e));
    }

    [Fact]
    public void Sibling_ZeroOverZero_IsNotDefiniteZero()
    {
        NewCtx();
        var e = Exprs.Divide(Exprs.Zero, Exprs.Zero);
        Assert.False(IsDefiniteZero(e), "0/0 must not fold to 0; got: " + Describe(e));
    }

    [Fact]
    public void Sibling_InfinityToTheZero()
    {
        // inf^0 = 1 is preserved by convention (matching 0^0 = 1 in this kernel) and is NOT
        // the N18 root cause; this test documents the decision rather than changing it.
        NewCtx();
        var e = Exprs.Power(Exprs.Infinity, Exprs.Zero);
        Assert.Equal(Exprs.One, e);
    }
}
