using Lovelace.Symbolics;
using Xunit;
using Rat = Lovelace.Rational.Rational;

namespace Lovelace.Symbolics.Tests;

public class PolynomialsApiTests : IDisposable
{
    private readonly ExprContext _ctx = new();

    public PolynomialsApiTests() => Exprs.Current = _ctx;

    public void Dispose() => Exprs.ClearCurrent();

    private Polynomial P(params Expr[] terms) => Polynomials.FromExpr(Exprs.Add(terms), _ctx, _ctx.Symbol("x"));

    [Fact]
    public void FromExpr_RoundTrip_DegreeCoefficientTerms()
    {
        var x = _ctx.Symbol("x");
        var p = Polynomials.FromExpr(Exprs.Add(Exprs.Power(x, 2), Exprs.Multiply(2, x), 1), _ctx, x);

        Assert.Equal(2, Polynomials.Degree(p));
        Assert.Equal(Rat.One, Polynomials.Coefficient(p, 2));
        Assert.Equal(Rat.FromLong(2L), Polynomials.Coefficient(p, 1));
        Assert.Equal(Rat.One, Polynomials.Coefficient(p, 0));
        Assert.Equal(Rat.Zero, Polynomials.Coefficient(p, 3));
        Assert.Equal(3, Polynomials.Terms(p).Count);
    }

    [Fact]
    public void Content_PrimitivePart_SplitCoefficients()
    {
        var x = _ctx.Symbol("x");
        var p = P(Exprs.Multiply(2, Exprs.Power(x, 2)), Exprs.Multiply(4, x), 6);

        Assert.Equal(Rat.FromLong(2L), Polynomials.Content(p));
        Assert.Equal(P(Exprs.Power(x, 2), Exprs.Multiply(2, x), 3), Polynomials.PrimitivePart(p));
    }

    [Fact]
    public void Divide_X3Minus1_By_XMinus1()
    {
        var x = _ctx.Symbol("x");
        var a = Polynomials.FromExpr(Exprs.Subtract(Exprs.Power(x, 3), 1), _ctx, x);
        var b = Polynomials.FromExpr(Exprs.Subtract(x, 1), _ctx, x);

        var (q, r) = Polynomials.Divide(a, b);

        Assert.Equal(P(Exprs.Power(x, 2), x, 1), q);
        Assert.True(r.IsZero);
    }

    [Fact]
    public void Gcd_Lcm_AreConsistent()
    {
        var x = _ctx.Symbol("x");
        var a = Polynomials.FromExpr(Exprs.Subtract(Exprs.Power(x, 2), 1), _ctx, x);
        var b = Polynomials.FromExpr(Exprs.Add(Exprs.Power(x, 2), Exprs.Multiply(2, x), 1), _ctx, x);

        var g = Polynomials.Gcd(a, b);
        var l = Polynomials.Lcm(a, b);

        Assert.Equal(P(x, 1), g); // monic x + 1
        Assert.Equal(Polynomial.Multiply(a, b), Polynomial.Multiply(g, l));
    }

    [Fact]
    public void SquareFree_X2Times_XMinus1()
    {
        var x = _ctx.Symbol("x");
        var p = Polynomials.FromExpr(Exprs.Subtract(Exprs.Power(x, 3), Exprs.Power(x, 2)), _ctx, x);

        var factors = Polynomials.SquareFree(p).OrderBy(f => f.Factor.TotalDegree).ToArray();

        Assert.Equal(2, factors.Length);
        Assert.Equal(P(x, -1), factors[0].Factor); // (x-1)
        Assert.Equal(1, factors[0].Multiplicity);
        Assert.Equal(P(x), factors[1].Factor); // x
        Assert.Equal(2, factors[1].Multiplicity);
    }

    [Fact]
    public void Factor_QuarticWithRationalRoots_FourLinearFactors()
    {
        var x = _ctx.Symbol("x");
        var p = Polynomials.FromExpr(Exprs.Add(Exprs.Power(x, 4), Exprs.Multiply(-5, Exprs.Power(x, 2)), 4), _ctx, x);

        var f = Polynomials.Factor(p);

        Assert.Equal(4, f.Factors.Count);
        Assert.All(f.Factors, fac =>
        {
            Assert.Equal(1, fac.Multiplicity);
            Assert.Equal(1, fac.Factor.TotalDegree);
        });
        // content 1 and the product of the factors reconstructs the original
        Assert.Equal(Rat.One, f.Content);
        var product = Polynomial.FromConstant(p.Order, f.Content);
        foreach (var (fac, mult) in f.Factors)
            product = Polynomial.Multiply(product, Polynomial.Pow(fac, mult));
        Assert.Equal(p, product);
    }

    [Fact]
    public void Resultant_MatchesKnownValues()
    {
        var x = _ctx.Symbol("x");
        Polynomial F(params Expr[] terms) => Polynomials.FromExpr(Exprs.Add(terms), _ctx, x);

        var f1 = F(Exprs.Power(x, 2), -2);
        var g1 = F(x, -1);
        Assert.Equal(Rat.FromLong(-1L), Polynomials.Resultant(f1, g1));

        var f2 = F(x, -1);
        var g2 = F(x, -1);
        Assert.Equal(Rat.Zero, Polynomials.Resultant(f2, g2));

        var f3 = F(x, -1);
        var g3 = F(x, -2);
        Assert.Equal(Rat.One, Polynomials.Resultant(f3, g3));
    }

    [Fact]
    public void Discriminant_MatchesKnownValues()
    {
        var x = _ctx.Symbol("x");
        Polynomial F(params Expr[] terms) => Polynomials.FromExpr(Exprs.Add(terms), _ctx, x);

        var q1 = F(Exprs.Power(x, 2), Exprs.Multiply(3, x), 2);
        Assert.Equal(Rat.One, Polynomials.Discriminant(q1));

        var q2 = F(Exprs.Power(x, 2), -2);
        Assert.Equal(Rat.FromLong(8L), Polynomials.Discriminant(q2));
    }
}
