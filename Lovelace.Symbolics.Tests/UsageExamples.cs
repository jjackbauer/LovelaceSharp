using Lovelace.MathIR;
using Lovelace.Symbolics;
using Xunit;
using Rat = Lovelace.Rational.Rational;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// The library-side usage examples: every C# snippet in Lovelace.Symbolics/README.md
/// appears verbatim (whitespace-normalized) in one of these compiling, asserting facts.
/// DocsSyncTests enforces that correspondence.
/// </summary>
public class UsageExamples
{
    [Fact]
    public void Snippet_16_1_ContextSymbolsAndConstruction()
    {
var ctx = new ExprContext();
Exprs.Current = ctx;
var x = ctx.Symbol("x");
Expr f = Exprs.Add(Exprs.Power(x, 2), Exprs.Multiply(2, x), 1);
Assert.Equal("1 + x^2 + 2*x", Printing.PrettyPrint(f));
    }

    [Fact]
    public void Snippet_16_2_EqualityAndHashConsing()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
var a = Exprs.Add(Exprs.Power(x, 2), x);
var b = Exprs.Add(x, Exprs.Power(x, 2));
Assert.Same(a, b);   // canonical construction: equal structure is the same reference
    }

    [Fact]
    public void Snippet_16_3_AssumptionsDriveSimplification()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
var e = Exprs.Power(Exprs.Power(x, 2), Exprs.Rational(1, 2));
Assert.Equal(e, Simplify.SimplifyExpr(e, ctx));   // unconstrained: unchanged
ctx.Assumptions = ctx.Assumptions.Add(new SymbolPropertyAssumption(x, SymbolPredicate.NonNegative));
Assert.Equal(x, Simplify.SimplifyExpr(e, ctx));   // x >= 0: sqrt(x^2) = x
    }

    [Fact]
    public void Snippet_16_4_CanonicalTextRoundTrip()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
        Expr f = Exprs.Add(Exprs.Power(x, 2), Exprs.Multiply(2, x), 1);
var text = Printing.CanonicalPrint(f);
var back = Printing.CanonicalParse(text, ctx);
Assert.Same(f, back);   // versioned, lossless, hash-consed round-trip
    }

    [Fact]
    public void Snippet_16_5_NumericEvaluationAtPrecision()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
using var scope = global::Lovelace.Real.Real.WithPrecision(50, 15);
var g = Exprs.Add(Exprs.Power(x, 2), 1);
var value = Evaluation.EvaluateToNum(g, ctx, new Dictionary<Symbol, Num> { [x] = new NumRat(Rat.FromLong(3L)) });
Assert.Equal(0, NumOps.Compare(value, NumOps.FromLong(10)));
    }

    [Fact]
    public void Snippet_16_6_MathIrLowerAndEvaluate()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
var prog = MathIR.Lowering.Lower(Exprs.Add(Exprs.Power(x, 2), 1), ctx, new[] { x });
var irValue = MathIR.IrEvaluator.Evaluate(prog, new Dictionary<Symbol, Num> { [x] = new NumRat(Rat.FromLong(4L)) }, ctx);
Assert.Equal(0, NumOps.Compare(irValue, NumOps.FromLong(17)));
    }

    [Fact]
    public void Snippet_16_7_SymbolicMatrices()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
var y = ctx.Symbol("y");
var m = SymbolicMatrix.From(new[] { new Expr[] { x, 1 }, new Expr[] { y, x } });
Assert.Equal(Exprs.Subtract(Exprs.Power(x, 2), y), m.Det(ctx));
    }

    [Fact]
    public void Snippet_16_8_ExactnessDiscipline()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
Assert.True(Exprs.Rational(Rat.From(1, 3)).IsExact);            // exact rational
Assert.False(Exprs.Power(Exprs.Rational(2L), Exprs.Rational(1, 2)).IsExact);  // sqrt(2) is not exact
    }
}