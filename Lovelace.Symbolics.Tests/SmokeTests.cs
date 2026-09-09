using Lovelace.Symbolics;
using Lovelace.Symbolics.Rewriting;
using Lovelace.MathIR;
using Xunit;
using Rat = Lovelace.Rational.Rational;

namespace Lovelace.Symbolics.Tests;

public class AcceptanceTests : IDisposable
{
    private readonly ExprContext _ctx = new();

    public AcceptanceTests() => Exprs.Current = _ctx;

    public void Dispose() => Exprs.ClearCurrent();

    private Expr P(string text) => Printing.CanonicalParse(text, _ctx);

    [Fact]
    public void Diff_Polynomial_ReturnsCanonicalDerivative()
    {
        var x = _ctx.Symbol("x");
        var f = Exprs.Add(Exprs.Power(x, 3), Exprs.Multiply(2, Exprs.Power(x, 2)), Exprs.Multiply(5, x), 7);
        var d = Calculus.Diff(f, x, _ctx);
        Assert.Equal("3*x^2 + 4*x + 5", Printing.PrettyPrint(d));
    }

    [Fact]
    public void Diff_ProductAndChain_Work()
    {
        var x = _ctx.Symbol("x");
        var f = Exprs.Multiply(Exprs.Function(_ctx.Function("exp"), Exprs.Negate(Exprs.Power(x, 2))), Exprs.Function(_ctx.Function("sin"), x));
        var d = Calculus.Diff(f, x, _ctx);
        var expected = Exprs.Multiply(
            Exprs.Function(_ctx.Function("exp"), Exprs.Negate(Exprs.Power(x, 2))),
            Exprs.Subtract(
                Exprs.Function(_ctx.Function("cos"), x),
                Exprs.Multiply(2, x, Exprs.Function(_ctx.Function("sin"), x))));
        // equivalent by exact expansion (canonical forms differ pre-simplify)
        Assert.Equal(Algebra.Expand(expected, _ctx), Algebra.Expand(d, _ctx));
    }

    [Fact]
    public void CanonicalRules_HoldInvariants()
    {
        var x = _ctx.Symbol("x");
        Assert.Equal(x, Exprs.Add(x, 0));
        Assert.Equal(x, Exprs.Multiply(x, 1));
        Assert.Equal(Exprs.Integer(0), Exprs.Multiply(x, 0));
        Assert.Equal(Exprs.Multiply(2, x), Exprs.Add(x, x));
        Assert.Equal(Exprs.Multiply(5, x), Exprs.Add(Exprs.Multiply(2, x), Exprs.Multiply(3, x)));
        Assert.Equal(Exprs.Power(x, 2), Exprs.Multiply(x, x));
        Assert.Equal(Exprs.Power(x, 5), Exprs.Multiply(Exprs.Power(x, 2), Exprs.Power(x, 3)));
        Assert.Equal(Exprs.Integer(8), Exprs.Power(Exprs.Integer(2), Exprs.Integer(3)));
        Assert.Equal(x, Exprs.Add(Exprs.Add(x, 1), Exprs.Negate(Exprs.One)));
    }

    [Fact]
    public void Factor_SquaresDifference_FullyFactors()
    {
        var x = _ctx.Symbol("x");
        var p = Exprs.Add(Exprs.Power(x, 4), Exprs.Multiply(-5, Exprs.Power(x, 2)), 4);
        var f = Factoring.Factor(p, _ctx);
        var expanded = Algebra.Expand(f, _ctx);
        Assert.Equal(p, expanded);
        Assert.Equal("(x - 2)*(x - 1)*(x + 1)*(x + 2)", Printing.PrettyPrint(f));
    }

    [Fact]
    public void Solve_QuarticWithRationalRoots_ReturnsAllFour()
    {
        var x = _ctx.Symbol("x");
        var p = Exprs.Add(Exprs.Power(x, 4), Exprs.Multiply(-5, Exprs.Power(x, 2)), 4);
        var set = Solvers.Solve(Exprs.Relation(RelOp.Eq, p, Exprs.Integer(0)), x, _ctx);
        Assert.Equal(SolutionKind.Exact, set.Kind);
        var roots = set.Solutions.Select(s => s.Value).ToList();
        var two = Exprs.Integer(2);
        Assert.Contains(two, roots);
        Assert.Contains(Exprs.Negate(two), roots);
        Assert.Contains(Exprs.One, roots);
        Assert.Contains(Exprs.Negate(Exprs.One), roots);
    }

    [Fact]
    public void Solve_Quadratic_ExactFormula()
    {
        var x = _ctx.Symbol("x");
        var set = Solvers.Solve(Exprs.Relation(RelOp.Eq,
            Exprs.Add(Exprs.Power(x, 2), Exprs.Multiply(-3, x), 2), Exprs.Integer(0)), x, _ctx);
        Assert.Equal(SolutionKind.Exact, set.Kind);
        var roots = set.Solutions.Select(s => s.Value).ToList();
        Assert.Contains(Exprs.Integer(1), roots);
        Assert.Contains(Exprs.Integer(2), roots);
    }

    [Fact]
    public void Series_SinOverX_GivesExpectedCoefficients()
    {
        var x = _ctx.Symbol("x");
        var s = Series.Of(Exprs.Divide(Exprs.Function(_ctx.Function("sin"), x), x), x, Exprs.Integer(0), 6, _ctx);
        Assert.Equal(Exprs.One, s.Coefficients[0]);
        Assert.Equal(Exprs.Integer(0), s.Coefficients[1]);
        Assert.Equal(Exprs.Rational(-1, 6), s.Coefficients[2]);
        Assert.Equal(Exprs.Integer(0), s.Coefficients[3]);
        Assert.Equal(Exprs.Rational(1, 120), s.Coefficients[4]);
    }

    [Fact]
    public void Limit_SinOverX_AtZero_IsOne()
    {
        var x = _ctx.Symbol("x");
        var r = Limits.Limit(Exprs.Divide(Exprs.Function(_ctx.Function("sin"), x), x), x, Exprs.Integer(0), LimitDirection.TwoSided, _ctx);
        Assert.Equal(LimitStatus.Value, r.Status);
        Assert.Equal(Exprs.One, r.Value);
    }

    [Fact]
    public void Integrate_Table_AllVerify()
    {
        var x = _ctx.Symbol("x");
        var cases = new (Expr, Expr)[]
        {
            (Exprs.Power(x, 7), Exprs.Divide(Exprs.Power(x, 8), 8)),
            (Exprs.Add(Exprs.Multiply(3, Exprs.Power(x, 2)), Exprs.Multiply(2, x), 1),
             Exprs.Add(Exprs.Power(x, 3), Exprs.Power(x, 2), x)),
            (Exprs.Divide(1, Exprs.Subtract(Exprs.Power(x, 2), 1)),
             Exprs.Subtract(
                 Exprs.Multiply(Exprs.Rational(1, 2), Exprs.Function(_ctx.Function("log"), Exprs.Subtract(x, 1))),
                 Exprs.Multiply(Exprs.Rational(1, 2), Exprs.Function(_ctx.Function("log"), Exprs.Add(x, 1))))),
            (Exprs.Function(_ctx.Function("sin"), x), Exprs.Negate(Exprs.Function(_ctx.Function("cos"), x))),
            (Exprs.Function(_ctx.Function("exp"), Exprs.Multiply(3, x)), Exprs.Divide(Exprs.Function(_ctx.Function("exp"), Exprs.Multiply(3, x)), 3)),
            (Exprs.Multiply(2, x, Exprs.Function(_ctx.Function("cos"), Exprs.Power(x, 2))),
             Exprs.Function(_ctx.Function("sin"), Exprs.Power(x, 2))),
            (Exprs.Multiply(x, Exprs.Function(_ctx.Function("exp"), x)),
             Exprs.Multiply(Exprs.Subtract(x, 1), Exprs.Function(_ctx.Function("exp"), x))),
        };
        foreach (var (integrand, expected) in cases)
        {
            var result = Integration.Integrate(integrand, x, _ctx);
            Assert.Equal(expected, result);
            // mandatory self-verification: differentiate and compare (structural or cross-multiplied)
            Assert.True(Integration.Verify(result, integrand, x, _ctx));
        }
    }

    [Fact]
    public void Det_SymbolicMatrix_Bareiss()
    {
        var x = _ctx.Symbol("x");
        var y = _ctx.Symbol("y");
        var a = SymbolicMatrix.From(new[] { new Expr[] { x, Exprs.One }, new Expr[] { y, x } });
        var det = a.Det(_ctx);
        Assert.Equal(Exprs.Subtract(Exprs.Power(x, 2), y), det);
    }

    [Fact]
    public void Inverse_SymbolicMatrix_TimesOriginalIsIdentity()
    {
        var x = _ctx.Symbol("x");
        var y = _ctx.Symbol("y");
        var a = SymbolicMatrix.From(new[] { new Expr[] { x, Exprs.One }, new Expr[] { y, x } });
        var inv = a.Inverse(_ctx);
        var product = SymbolicMatrix.Multiply(a, inv);
        // A·A⁻¹ = I verified numerically (canonical form does not factor common denominators)
        var bindings = new Dictionary<Symbol, Num> { [x] = new NumRat(Rat.FromLong(3L)), [y] = new NumRat(Rat.FromLong(2L)) };
        Assert.Equal(0, NumOps.Compare(Evaluation.EvaluateToNum(product[0, 0], _ctx, bindings), NumOps.FromLong(1)));
        Assert.Equal(0, NumOps.Compare(Evaluation.EvaluateToNum(product[0, 1], _ctx, bindings), NumOps.FromLong(0)));
        Assert.Equal(0, NumOps.Compare(Evaluation.EvaluateToNum(product[1, 0], _ctx, bindings), NumOps.FromLong(0)));
        Assert.Equal(0, NumOps.Compare(Evaluation.EvaluateToNum(product[1, 1], _ctx, bindings), NumOps.FromLong(1)));
    }

    [Fact]
    public void Hessian_Computes()
    {
        var x = _ctx.Symbol("x");
        var y = _ctx.Symbol("y");
        var f = Exprs.Multiply(
            Exprs.Add(Exprs.Power(x, 2), Exprs.Power(y, 2)),
            Exprs.Function(_ctx.Function("exp"), Exprs.Negate(Exprs.Add(x, y))));
        var h = SymbolicMatrix.Hessian(f, new[] { x, y }, _ctx);
        Assert.Equal(2, h.Rows);
        Assert.Equal(2, h.Columns);
        // symmetry of mixed partials
        Assert.Equal(h[0, 1], h[1, 0]);
    }

    [Fact]
    public void Simplify_SqrtSquare_UnderAssumptions()
    {
        var x = _ctx.Symbol("x");
        var e = Exprs.Power(Exprs.Power(x, 2), Exprs.Rational(1, 2));
        Assert.Equal(e, Simplify.SimplifyExpr(e, _ctx));          // unconstrained: unchanged
        _ctx.Assumptions = _ctx.Assumptions.Add(new SymbolPropertyAssumption(x, SymbolPredicate.NonNegative));
        Assert.Equal(x, Simplify.SimplifyExpr(e, _ctx));          // x >= 0: x
        _ctx.Assumptions = AssumptionSet.Empty;
        _ctx.Assumptions = _ctx.Assumptions.Add(new SymbolDomainAssumption(x, Domain.Real));
        var r = Simplify.SimplifyExpr(e, _ctx);
        Assert.Equal(Exprs.Function(_ctx.Function("abs"), x), r); // real: abs(x)
    }

    [Fact]
    public void Simplify_Pythagorean_CollapsesToOne()
    {
        var x = _ctx.Symbol("x");
        var e = Exprs.Add(
            Exprs.Power(Exprs.Function(_ctx.Function("sin"), x), 2),
            Exprs.Power(Exprs.Function(_ctx.Function("cos"), x), 2));
        Assert.Equal(Exprs.One, Simplify.SimplifyExpr(e, _ctx));
    }

    [Fact]
    public void Optimize_Lower_Evaluate_RoundTrip()
    {
        var x = _ctx.Symbol("x");
        var g = Exprs.Add(Exprs.Power(x, 5), Exprs.Multiply(2, Exprs.Power(x, 4)), Exprs.Multiply(3, Exprs.Power(x, 3)));
        var opt = Optimizer.Optimize(g, new[] { x }, _ctx);
        var prog = Lowering.Lower(opt.Expression, _ctx, new[] { x });
        var result = IrEvaluator.Evaluate(prog, new Dictionary<Symbol, Num> { [x] = new NumRat(Rat.FromLong(2L)) }, _ctx);
        Assert.Equal(0, NumOps.Compare(result, new NumRat(Rat.FromLong(32 + 32 + 24))));
    }

    [Fact]
    public void PrintParse_RoundTrips()
    {
        var x = _ctx.Symbol("x");
        var e = Exprs.Divide(Exprs.Add(Exprs.Power(x, 2), 1), Exprs.Subtract(x, 1));
        var text = Printing.CanonicalPrint(e);
        var back = Printing.CanonicalParse(text, _ctx);
        Assert.Equal(e, back);
        Assert.Same(e, back);   // hash-consing: same reference
    }

    [Fact]
    public void Rational_NormalizationAndArithmetic()
    {
        var a = Rat.From(2, 4);
        Assert.True(Rat.From(1, 2) == a);
        Assert.Equal("1/2", a.ToString());
        Assert.True(Rat.From(5, 6) == Rat.From(1, 2) + Rat.From(1, 3));
        Assert.True(Rat.From(1, 3) == Rat.FromLong(2L) / Rat.FromLong(6L));
    }

    [Fact]
    public void Interning_EqualStructureSharesReference()
    {
        var x = _ctx.Symbol("x");
        var a = Exprs.Add(Exprs.Power(x, 2), x);
        var b = Exprs.Add(x, Exprs.Power(x, 2));
        Assert.Same(a, b);
    }

    [Fact]
    public void TermOrder_IsAntisymmetricAndNumbersFirst()
    {
        var x = _ctx.Symbol("x");
        var y = _ctx.Symbol("y");
        var items = new Expr[]
        {
            Exprs.Integer(3), Exprs.Rational(1, 2), x, y,
            Exprs.Power(x, 2), Exprs.Multiply(x, y), Exprs.Add(x, y),
            Exprs.Function(_ctx.Function("sin"), x), Exprs.Named(NamedConstant.Pi),
        };
        for (int i = 0; i < items.Length; i++)
            for (int j = 0; j < items.Length; j++)
            {
                int cmp = TermOrder.Compare(items[i], items[j]);
                int rev = TermOrder.Compare(items[j], items[i]);
                if (i == j)
                    Assert.Equal(0, cmp);
                else
                    Assert.Equal(-cmp, rev);
                if (TermOrder.IsNumericConstant(items[i]) && !TermOrder.IsNumericConstant(items[j]))
                    Assert.True(cmp < 0);
            }
    }
}
