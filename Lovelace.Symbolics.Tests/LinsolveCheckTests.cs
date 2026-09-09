using Lovelace.Symbolics;
using Xunit;
using Rat = Lovelace.Rational.Rational;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Symbolics.Tests;

public class LinsolveCheckTests
{
    [Fact]
    public void Solve_VerifiesByNumericSubstitution()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var A = SymbolicMatrix.From(new[] { new Expr[] { x, 1 }, new Expr[] { 0, y } });
        var b = new Expr[] { Exprs.Zero, Exprs.One };
        var v = SymbolicMatrix.Solve(A, b, ctx);
        using var scope = Rl.WithPrecision(50, 20);
        Rat[] px = { Rat.From(2, 1), Rat.From(-1, 2) };
        Rat[] py = { Rat.From(3, 1), Rat.From(5, 2) };
        for (int t = 0; t < 2; t++)
        {
            var bindings = new Dictionary<Symbol, Num>
            {
                [x] = NumOps.FromRat(px[t]),
                [y] = NumOps.FromRat(py[t]),
            };
            var v0 = Evaluation.EvaluateToNum(v[0], ctx, bindings);
            var v1 = Evaluation.EvaluateToNum(v[1], ctx, bindings);
            var r0 = NumOps.Add(NumOps.Multiply(NumOps.FromRat(px[t]), v0), v1);
            var r1 = NumOps.Multiply(NumOps.FromRat(py[t]), v1);
            Assert.Equal(0, NumOps.Compare(r0, NumOps.FromLong(0)));
            Assert.Equal(0, NumOps.Compare(r1, NumOps.FromLong(1)));
        }
    }

    [Fact]
    public void Solve_3x3_VerifiesByNumericSubstitution()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var A = SymbolicMatrix.From(new[]
        {
            new Expr[] { x, 1, 0 },
            new Expr[] { 1, x, 1 },
            new Expr[] { 0, 1, y },
        });
        var b = new Expr[] { Exprs.One, Exprs.Zero, Exprs.One };
        var v = SymbolicMatrix.Solve(A, b, ctx);
        using var scope = Rl.WithPrecision(50, 20);
        var bindings = new Dictionary<Symbol, Num>
        {
            [x] = NumOps.FromRat(Rat.From(2, 1)),
            [y] = NumOps.FromRat(Rat.From(3, 1)),
        };
        var xv = Evaluation.EvaluateToNum(v[0], ctx, bindings);
        var yv = Evaluation.EvaluateToNum(v[1], ctx, bindings);
        var zv = Evaluation.EvaluateToNum(v[2], ctx, bindings);
        var two = NumOps.FromLong(2);
        var three = NumOps.FromLong(3);
        var row0 = NumOps.Add(NumOps.Multiply(two, xv), yv);
        var row1 = NumOps.Add(NumOps.Add(xv, NumOps.Multiply(two, yv)), zv);
        var row2 = NumOps.Add(yv, NumOps.Multiply(three, zv));
        Assert.Equal(0, NumOps.Compare(row0, NumOps.FromLong(1)));
        Assert.Equal(0, NumOps.Compare(row1, NumOps.FromLong(0)));
        Assert.Equal(0, NumOps.Compare(row2, NumOps.FromLong(1)));
    }
}
