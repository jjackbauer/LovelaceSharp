using Lovelace.MathIR;
using Lovelace.Symbolics;
using Xunit;
using Int = Lovelace.Integer.Integer;
using Rat = Lovelace.Rational.Rational;

namespace Lovelace.Symbolics.Tests;

/// <summary>Compilation API: kernels, batch evaluation, and the backend seam (Phase 3).</summary>
public class CompilationTests
{
    private static ExprContext NewCtx()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx;
    }

    [Fact]
    public void Kernel_ScalarEvaluation()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var e = Exprs.Add(Exprs.Power(x, 2), 1);
        var kernel = Compilation.CompileKernel(e, new[] { x }, ctx);
        var result = kernel.Evaluate(NumOps.FromLong(2));
        Assert.Equal(0, NumOps.Compare(result, NumOps.FromLong(5)));
        Assert.True(result.IsExact);
    }

    [Fact]
    public void Kernel_NamedAndPositional_Evaluate()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var e = Exprs.Add(Exprs.Multiply(x, y), 1);
        var kernel = Compilation.CompileKernel(e, new[] { x, y }, ctx);
        Assert.Equal(0, NumOps.Compare(kernel.Evaluate(NumOps.FromLong(2), NumOps.FromLong(3)), NumOps.FromLong(7)));
        Assert.Equal(0, NumOps.Compare(
            kernel.Evaluate(new Dictionary<string, Num> { ["x"] = NumOps.FromLong(2), ["y"] = NumOps.FromLong(3) }),
            NumOps.FromLong(7)));
        Assert.Throws<ArgumentException>(() => kernel.Evaluate(NumOps.FromLong(1)));
    }

    [Fact]
    public void Kernel_BatchEvaluation_Columnwise()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var e = Exprs.Add(Exprs.Power(x, 2), 1);
        var kernel = Compilation.CompileKernel(e, new[] { x }, ctx);
        var results = kernel.EvaluateBatch(new Dictionary<string, Num[]>
        {
            ["x"] = new Num[] { NumOps.FromLong(1), NumOps.FromLong(2), NumOps.FromLong(3) },
        });
        Assert.Equal(3, results.Length);
        Assert.Equal(0, NumOps.Compare(results[0], NumOps.FromLong(2)));
        Assert.Equal(0, NumOps.Compare(results[1], NumOps.FromLong(5)));
        Assert.Equal(0, NumOps.Compare(results[2], NumOps.FromLong(10)));
        Assert.Throws<ArgumentException>(() => kernel.EvaluateBatch(new Dictionary<string, Num[]>
        {
            ["x"] = new Num[] { NumOps.FromLong(1), NumOps.FromLong(2) },
            ["y"] = new Num[] { NumOps.FromLong(3) },
        }));
    }

    [Fact]
    public void Kernel_ExactTiers_Preserved()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var e = Exprs.Add(Exprs.Multiply(2, Exprs.Power(x, 3)), Exprs.Rational(1, 2));
        var kernel = Compilation.CompileKernel(e, new[] { x }, ctx);
        var result = kernel.Evaluate(NumOps.FromRat(Rat.From(1, 2)));
        // 2*(1/2)^3 + 1/2 = 1/4 + 1/2 = 3/4 — exact rational
        Assert.IsType<NumRat>(result);
        Assert.Equal(0, NumOps.Compare(result, NumOps.FromRat(Rat.From(3, 4))));
    }

    [Fact]
    public void Kernel_Transcendental_ApproximateTier()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var e = Exprs.Function(ctx.Function("exp"), x);
        var kernel = Compilation.CompileKernel(e, new[] { x }, ctx);
        using var scope = Lovelace.Real.Real.WithPrecision(40, 20);
        var result = kernel.Evaluate(NumOps.FromLong(0));
        Assert.False(result.IsExact);
        Assert.Equal(0, NumOps.Compare(result, NumOps.FromLong(1)));
    }

    [Fact]
    public void Kernel_BatchLazySelect_MixedGuards()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var pw = Exprs.Piecewise(
            new[] { new PiecewiseBranch(Exprs.Relation(RelOp.Gt, x, Exprs.Zero), Exprs.Divide(Exprs.One, x)) },
            Exprs.Negate(x));
        var kernel = Compilation.CompileKernel(pw, new[] { x }, ctx);
        var columns = new Dictionary<string, Num[]>();
        columns["x"] = new Num[] { NumOps.FromLong(2), NumOps.FromLong(-3), NumOps.FromRat(Rat.From(1, 2)) };
        var results = kernel.EvaluateBatch(columns);
        Assert.Equal(0, NumOps.Compare(results[0], NumOps.FromRat(Rat.From(1, 2))));
        Assert.Equal(0, NumOps.Compare(results[1], NumOps.FromLong(3)));
        Assert.Equal(0, NumOps.Compare(results[2], NumOps.FromLong(2)));
    }

    [Fact]
    public void Kernel_BatchMatchesScalarEvaluation()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var e = Exprs.Add(Exprs.Function(ctx.Function("sin"), Exprs.Multiply(x, y)), Exprs.Power(x, 2));
        var kernel = Compilation.CompileKernel(e, new[] { x, y }, ctx);
        using var scope = Lovelace.Real.Real.WithPrecision(40, 20);
        Num[] xs = { NumOps.FromRat(Rat.From(1, 2)), NumOps.FromRat(Rat.From(-3, 4)), NumOps.FromRat(Rat.From(7, 5)) };
        Num[] ys = { NumOps.FromRat(Rat.From(2, 3)), NumOps.FromRat(Rat.From(5, 4)), NumOps.FromRat(Rat.From(-1, 3)) };
        var columns = new Dictionary<string, Num[]>();
        columns["x"] = xs;
        columns["y"] = ys;
        var batch = kernel.EvaluateBatch(columns);
        for (int i = 0; i < xs.Length; i++)
        {
            var scalar = kernel.Evaluate(xs[i], ys[i]);
            Assert.Equal(0, NumOps.Compare(batch[i], scalar));
        }
    }

    [Fact]
    public void Kernel_LazySelect_UntakenBranchNeverEvaluates()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        // piecewise(x == 0: 1, otherwise 1/x) must evaluate to 1 at x = 0 — the untaken
        // branch 1/x would throw if the Select were strict
        var pw = Exprs.Piecewise(
            new[] { new PiecewiseBranch(Exprs.Relation(RelOp.Eq, x, Exprs.Zero), Exprs.One) },
            Exprs.Divide(Exprs.One, x));
        var kernel = Compilation.CompileKernel(pw, new[] { x }, ctx);
        Assert.Equal(0, NumOps.Compare(kernel.Evaluate(NumOps.FromLong(0)), NumOps.FromLong(1)));
        Assert.Equal(0, NumOps.Compare(kernel.Evaluate(NumOps.FromLong(2)), NumOps.FromRat(Rat.From(1, 2))));
    }

    [Fact]
    public void CompilationResult_IrText_RoundTrips()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var e = Exprs.Add(Exprs.Power(x, 5), Exprs.Multiply(3, x));
        var compilation = Compilation.Compile(e, ctx, x);
        var back = IrProgram.Deserialize(compilation.IrText);
        Assert.Equal(compilation.Program.Serialize(), back.Serialize());
    }
}
