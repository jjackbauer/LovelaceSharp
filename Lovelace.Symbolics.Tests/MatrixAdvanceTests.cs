using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>Tests for symbolic-matrix Rank and the condition-carrying Inverse/Solve variants.</summary>
public class MatrixAdvanceTests : IDisposable
{
    private readonly ExprContext _ctx = new();

    public MatrixAdvanceTests() => Exprs.Current = _ctx;

    public void Dispose() => Exprs.ClearCurrent();

    [Fact]
    public void Rank_GenericOverSymbols_And_Numeric()
    {
        var x = _ctx.Symbol("x");
        var y = _ctx.Symbol("y");

        var symbolic = SymbolicMatrix.From(new[] { new Expr[] { x, 1 }, new Expr[] { y, x } });
        Assert.Equal(2, symbolic.Rank(_ctx));

        var dependent = SymbolicMatrix.From(new[] { new Expr[] { x, x }, new Expr[] { x, x } });
        Assert.Equal(1, dependent.Rank(_ctx));

        var numeric = SymbolicMatrix.From(new[] { new Expr[] { 1, 2 }, new Expr[] { 2, 4 } });
        Assert.Equal(1, numeric.Rank(_ctx));

        var identity = SymbolicMatrix.From(new[]
        {
            new Expr[] { 1, 0, 0 },
            new Expr[] { 0, 1, 0 },
            new Expr[] { 0, 0, 1 },
        });
        Assert.Equal(3, identity.Rank(_ctx));
    }

    [Fact]
    public void InverseWithConditions_NonSingular_MatchesInverse_And_CarriesCondition()
    {
        var x = _ctx.Symbol("x");
        var y = _ctx.Symbol("y");
        var a = SymbolicMatrix.From(new[] { new Expr[] { x, 1 }, new Expr[] { 0, y } });

        var result = a.InverseWithConditions(_ctx);
        Assert.NotNull(result.Matrix);
        Assert.Null(result.Note);
        Assert.NotEmpty(result.Conditions.Atoms);

        var expected = a.Inverse(_ctx);
        AssertMatrixEqual(expected, result.Matrix!);
    }

    [Fact]
    public void InverseWithConditions_Singular_ReturnsNullMatrix_And_Note()
    {
        var a = SymbolicMatrix.From(new[] { new Expr[] { 1, 2 }, new Expr[] { 2, 4 } });

        var result = a.InverseWithConditions(_ctx);
        Assert.Null(result.Matrix);
        Assert.Equal("matrix is singular", result.Note);
        Assert.Empty(result.Conditions.Atoms);
    }

    [Fact]
    public void SolveWithConditions_MatchesSolve_And_CarriesCondition()
    {
        var x = _ctx.Symbol("x");
        var y = _ctx.Symbol("y");
        var a = SymbolicMatrix.From(new[] { new Expr[] { x, 1 }, new Expr[] { 0, y } });
        var b = new Expr[] { 1, 1 };

        var result = a.SolveWithConditions(b, _ctx);
        Assert.NotNull(result.Vector);
        Assert.Null(result.Note);
        Assert.NotEmpty(result.Conditions.Atoms);

        var expected = SymbolicMatrix.Solve(a, b, _ctx);
        Assert.Equal(expected.Length, result.Vector!.Length);
        for (int i = 0; i < expected.Length; i++)
            Assert.Equal(expected[i], result.Vector[i]);
    }

    [Fact]
    public void Inverse_Invertible_BehaviorPreserved()
    {
        var x = _ctx.Symbol("x");
        var y = _ctx.Symbol("y");
        var a = SymbolicMatrix.From(new[] { new Expr[] { x, 1 }, new Expr[] { 0, y } });

        var inv = a.Inverse(_ctx);
        Assert.NotNull(inv);
        AssertMatrixEqual(a.InverseWithConditions(_ctx).Matrix!, inv);
    }

    private static void AssertMatrixEqual(SymbolicMatrix expected, SymbolicMatrix actual)
    {
        Assert.Equal(expected.Rows, actual.Rows);
        Assert.Equal(expected.Columns, actual.Columns);
        for (int i = 0; i < expected.Rows; i++)
            for (int j = 0; j < expected.Columns; j++)
                Assert.Equal(expected[i, j], actual[i, j]);
    }
}
