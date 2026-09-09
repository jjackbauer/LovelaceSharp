using Lovelace.Symbolics;
using Xunit;
using Rat = Lovelace.Rational.Rational;

namespace Lovelace.Symbolics.Tests;

/// <summary>Gröbner bases (Buchberger): correctness-first invariants over standard examples.</summary>
public class GroebnerTests
{
    private static (ExprContext Ctx, Symbol X, Symbol Y) Setup()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return (ctx, ctx.Symbol("x"), ctx.Symbol("y"));
    }

    private static Polynomial Poly(Expr e, ExprContext ctx, Symbol x, Symbol y)
        => Polynomial.FromExpr(e, ctx, new[] { x, y });

    /// <summary>Every element of a computed basis reduces to zero modulo the basis (criterion).</summary>
    private static void AssertIsGroebnerBasis(List<Polynomial> basis, MonomialOrder order)
    {
        foreach (var b in basis)
        {
            var r = Groebner.Reduce(b, basis, order);
            Assert.True(r.IsZero, $"basis element does not reduce to 0: {Printing.PrettyPrint(b.ToExpr())}");
        }
    }

    [Fact]
    public void CircleHyperbola_GrLex()
    {
        var (ctx, x, y) = Setup();
        // I = (x^2 + y^2 - 1, xy) under grlex (x > y): reduced basis {x^2 + y^2 - 1, xy, y^3 - y}
        var basis = Groebner.Basis(new[]
        {
            Poly(Exprs.Add(Exprs.Power(x, 2), Exprs.Power(y, 2), -1), ctx, x, y),
            Poly(Exprs.Multiply(x, y), ctx, x, y),
        }, MonomialOrder.GrLex);
        Assert.Equal(3, basis.Count);
        AssertIsGroebnerBasis(basis, MonomialOrder.GrLex);
        // the eliminated polynomial y^3 - y is in the ideal and reduces to 0
        var target = Poly(Exprs.Subtract(Exprs.Power(y, 3), y), ctx, x, y);
        Assert.True(Groebner.Reduce(target, basis, MonomialOrder.GrLex).IsZero);
    }

    [Fact]
    public void Eliminate_X_FromCircleHyperbola()
    {
        var (ctx, x, y) = Setup();
        var generators = new[]
        {
            Poly(Exprs.Add(Exprs.Power(x, 2), Exprs.Power(y, 2), -1), ctx, x, y),
            Poly(Exprs.Multiply(x, y), ctx, x, y),
        };
        var elim = Groebner.Eliminate(generators, new[] { x }, MonomialOrder.Lex);
        // elimination ideal: (y^3 - y) — a single polynomial in y only
        Assert.Single(elim);
        Assert.Equal("y^3-y", Printing.PrettyPrint(elim[0].ToExpr()).Replace(" ", ""));
    }

    [Fact]
    public void TrivialIdeal_UnitGenerators_ProducesOne()
    {
        var (ctx, x, y) = Setup();
        // (x, x+1) = (1): the basis is {1}
        var basis = Groebner.Basis(new[]
        {
            Poly(x, ctx, x, y),
            Poly(Exprs.Add(x, 1), ctx, x, y),
        }, MonomialOrder.GrLex);
        Assert.Single(basis);
        Assert.Equal("1", Printing.PrettyPrint(basis[0].ToExpr()));
    }

    [Fact]
    public void Reduce_NormalForm_IsIdempotent()
    {
        var (ctx, x, y) = Setup();
        var basis = Groebner.Basis(new[]
        {
            Poly(Exprs.Add(Exprs.Power(x, 2), Exprs.Power(y, 2), -1), ctx, x, y),
            Poly(Exprs.Multiply(x, y), ctx, x, y),
        }, MonomialOrder.GrLex);
        var p = Poly(Exprs.Add(Exprs.Power(x, 3), Exprs.Multiply(Exprs.Power(x, 2), y), x, y), ctx, x, y);
        var r1 = Groebner.Reduce(p, basis, MonomialOrder.GrLex);
        var r2 = Groebner.Reduce(r1, basis, MonomialOrder.GrLex);
        Assert.True(Polynomial.Subtract(r1, r2).IsZero);
    }

    [Fact]
    public void LexOrder_OfOrderedMonomials()
    {
        var (ctx, x, y) = Setup();
        // (x^2, xy, y^2) with lex x>y is already a reduced Groebner basis
        var basis = Groebner.Basis(new[]
        {
            Poly(Exprs.Power(x, 2), ctx, x, y),
            Poly(Exprs.Multiply(x, y), ctx, x, y),
            Poly(Exprs.Power(y, 2), ctx, x, y),
        }, MonomialOrder.Lex);
        Assert.Equal(3, basis.Count);
        AssertIsGroebnerBasis(basis, MonomialOrder.Lex);
    }

    [Fact]
    public void SinglePolynomial_BasisIsItself()
    {
        var (ctx, x, y) = Setup();
        var basis = Groebner.Basis(new[]
        {
            Poly(Exprs.Add(Exprs.Power(x, 3), Exprs.Multiply(-2, x), 1), ctx, x, y),
        }, MonomialOrder.Lex);
        Assert.Single(basis);
        AssertIsGroebnerBasis(basis, MonomialOrder.Lex);
    }
}
