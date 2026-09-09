using Lovelace.Symbolics;
using Xunit;
using Int = Lovelace.Integer.Integer;
using Rat = Lovelace.Rational.Rational;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Concurrency safety: parallel symbol/function creation must never duplicate ids, and shared
/// read-only kernel operations (parse, simplify, diff, compile) must agree with serial results.
/// </summary>
public class ConcurrencyTests
{
    [Fact]
    public async Task ParallelSymbolCreation_NoDuplicateIds()
    {
        var ctx = new ExprContext();
        var tasks = Enumerable.Range(0, 8).Select(t => Task.Run(() =>
        {
            var names = Enumerable.Range(0, 250).Select(i => $"x{t}_{i}").ToArray();
            foreach (var n in names)
                ctx.Symbol(n);
            return names.Select(ctx.Symbol).ToArray();
        })).ToArray();
        var all = (await Task.WhenAll(tasks)).SelectMany(a => a).ToArray();
        // every name got a unique id within the context
        Assert.Equal(all.Length, all.Select(s => s.Id).Distinct().Count());
        // and the id→name map round-trips
        foreach (var s in all)
            Assert.Equal(s.Name, ctx.Symbol(s.Id).Name);
    }

    [Fact]
    public async Task ParallelFunctionCreation_NoDuplicateIds()
    {
        var ctx = new ExprContext();
        var tasks = Enumerable.Range(0, 8).Select(t => Task.Run(() =>
        {
            var names = Enumerable.Range(0, 250).Select(i => $"fn{t}_{i}").ToArray();
            foreach (var n in names)
                ctx.Function(n);
            return names.Select(ctx.Function).ToArray();
        })).ToArray();
        var all = (await Task.WhenAll(tasks)).SelectMany(a => a).ToArray();
        Assert.Equal(all.Length, all.Select(f => f.Id).Distinct().Count());
        foreach (var f in all)
            Assert.Equal(f.Name, ctx.Function(f.Id).Name);
    }

    [Fact]
    public async Task ParallelParse_AgreesWithSerial()
    {
        var ctx = new ExprContext();
        var texts = Enumerable.Range(0, 40)
            .Select(i => Printing.CanonicalPrint(
                Exprs.Add(Exprs.Power(Exprs.Symbol("x"), Exprs.Integer(i % 9 + 1)), Exprs.Integer(i + 1))))
            .ToArray();
        var serial = texts.Select(t => Printing.CanonicalParse(t, ctx)).ToArray();
        var parallel = await Task.WhenAll(texts.Select(t => Task.Run(() =>
            Printing.CanonicalParse(t, ctx))));
        for (int i = 0; i < texts.Length; i++)
            Assert.Equal(serial[i], parallel[i]);
    }

    [Fact]
    public async Task ParallelSimplifyDiff_AgreeWithSerial()
    {
        var ctx = new ExprContext();
        var x = ctx.Symbol("x");
        var exprs = Enumerable.Range(0, 40).Select(i =>
            Exprs.Power(Exprs.Function(ctx.Function("sin"), Exprs.Power(x, 2)), Exprs.Integer(i % 7 + 1)))
            .ToArray();
        var serialDiff = exprs.Select(e => Printing.CanonicalPrint(Calculus.Diff(e, x, ctx))).ToArray();
        var serialSimp = exprs.Select(e => Printing.CanonicalPrint(Simplify.SimplifyExpr(e, ctx))).ToArray();
        var parallelDiff = await Task.WhenAll(exprs.Select(e => Task.Run(() =>
            Printing.CanonicalPrint(Calculus.Diff(e, x, ctx)))));
        var parallelSimp = await Task.WhenAll(exprs.Select(e => Task.Run(() =>
            Printing.CanonicalPrint(Simplify.SimplifyExpr(e, ctx)))));
        for (int i = 0; i < exprs.Length; i++)
        {
            Assert.Equal(serialDiff[i], parallelDiff[i]);
            Assert.Equal(serialSimp[i], parallelSimp[i]);
        }
    }

    [Fact]
    public async Task ParallelCompilation_AgreesWithSerial()
    {
        var ctx = new ExprContext();
        var x = ctx.Symbol("x");
        var e = Exprs.Add(Exprs.Power(x, 5), Exprs.Multiply(2, Exprs.Power(x, 4)), Exprs.Multiply(3, Exprs.Power(x, 3)));
        var serial = Lovelace.MathIR.Lowering.Lower(e, ctx, new[] { x }).Serialize();
        var parallel = await Task.WhenAll(Enumerable.Range(0, 16).Select(_ => Task.Run(() =>
            Lovelace.MathIR.Lowering.Lower(e, ctx, new[] { x }).Serialize())));
        foreach (var p in parallel)
            Assert.Equal(serial, p);
    }

    [Fact]
    public async Task AssumptionScopes_RestoreOnDispose()
    {
        var ctx = new ExprContext();
        var x = ctx.Symbol("x");
        var e = Exprs.Power(Exprs.Power(x, 2), Exprs.Rational(1, 2));
        var baseline = Printing.CanonicalPrint(Simplify.SimplifyExpr(e, ctx));
        var scoped = await Task.Run(() =>
        {
            using (ctx.WithAssumptions(AssumptionSet.Empty.Add(new SymbolDomainAssumption(x, Domain.Real))))
                return Printing.CanonicalPrint(Simplify.SimplifyExpr(e, ctx));
        });
        Assert.Equal(Printing.CanonicalPrint(Exprs.Function(ctx.Function("abs"), x)), scoped);
        // after dispose the scope is gone: unrelated work sees the original assumptions again
        Assert.Equal(baseline, Printing.CanonicalPrint(Simplify.SimplifyExpr(e, ctx)));
        Assert.True(ctx.Assumptions.Atoms.Length == 0);
    }
}
