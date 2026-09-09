using Lovelace.Symbolics;
using Xunit;
using Rat = Lovelace.Rational.Rational;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Symbolics.Tests;

/// <summary>Polynomial systems via Gröbner elimination (Phase 4).</summary>
public class SystemSolveTests
{
    private static ExprContext NewCtx()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx;
    }

    private static void VerifyNumeric(SystemSolveResult result, IReadOnlyList<Expr> equations, Symbol[] vars, ExprContext ctx)
    {
        Assert.Null(result.Note);
        using var scope = Rl.WithPrecision(50, 20);
        foreach (var sol in result.Solutions)
        {
            var bindings = new Dictionary<Symbol, Num>();
            foreach (var v in vars)
                bindings[v] = Evaluation.EvaluateToNum(sol.Assignment[v], ctx, new Dictionary<Symbol, Num>());
            foreach (var eq in equations)
            {
                var f = eq is RelationExpr r ? Exprs.Subtract(r.Left, r.Right) : eq;
                var residual = Evaluation.EvaluateToNum(f, ctx, bindings);
                var tol = NumOps.FromReal(Rl.Parse("0." + new string('0', 24) + "1", null));
                Assert.True(NumOps.Compare(NumOps.Abs(residual, ctx), tol) < 0,
                    $"assignment {string.Join(", ", vars.Select(v => v.Name + "=" + Printing.PrettyPrint(sol.Assignment[v])))} does not satisfy the system");
            }
        }
    }

    [Fact]
    public void CircleHyperbola_FourSolutions()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var eqs = new Expr[]
        {
            Exprs.Relation(RelOp.Eq, Exprs.Add(Exprs.Power(x, 2), Exprs.Power(y, 2)), Exprs.One),
            Exprs.Relation(RelOp.Eq, Exprs.Multiply(x, y), Exprs.Zero),
        };
        var result = SystemSolvers.Solve(eqs, new[] { x, y }, ctx);
        Assert.Equal(4, result.Solutions.Count);
        VerifyNumeric(result, eqs, new[] { x, y }, ctx);
    }

    [Fact]
    public void LinearSystem_SingleSolution()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var eqs = new Expr[]
        {
            Exprs.Relation(RelOp.Eq, Exprs.Add(x, y), Exprs.One),
            Exprs.Relation(RelOp.Eq, Exprs.Subtract(x, y), Exprs.Zero),
        };
        var result = SystemSolvers.Solve(eqs, new[] { x, y }, ctx);
        Assert.Single(result.Solutions);
        Assert.Equal(Exprs.Rational(1, 2), result.Solutions[0].Assignment[x]);
        Assert.Equal(Exprs.Rational(1, 2), result.Solutions[0].Assignment[y]);
    }

    [Fact]
    public void ParabolaLine_TwoSolutions()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var eqs = new Expr[]
        {
            Exprs.Relation(RelOp.Eq, Exprs.Subtract(Exprs.Power(x, 2), y), Exprs.Zero),
            Exprs.Relation(RelOp.Eq, Exprs.Subtract(y, 4), Exprs.Zero),
        };
        var result = SystemSolvers.Solve(eqs, new[] { x, y }, ctx);
        Assert.Equal(2, result.Solutions.Count);
        VerifyNumeric(result, eqs, new[] { x, y }, ctx);
    }

    [Fact]
    public void Inconsistent_NoSolutions()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var eqs = new Expr[]
        {
            Exprs.Relation(RelOp.Eq, Exprs.Subtract(x, 1), Exprs.Zero),
            Exprs.Relation(RelOp.Eq, Exprs.Subtract(x, 2), Exprs.Zero),
        };
        var result = SystemSolvers.Solve(eqs, new[] { x }, ctx);
        Assert.Empty(result.Solutions);
    }

    [Fact]
    public void NonPolynomial_ReportsNote()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var result = SystemSolvers.Solve(new[]
        {
            Exprs.Relation(RelOp.Eq, Exprs.Function(ctx.Function("sin"), x), y),
        }, new[] { x, y }, ctx);
        Assert.Empty(result.Solutions);
        Assert.NotNull(result.Note);
    }
}
