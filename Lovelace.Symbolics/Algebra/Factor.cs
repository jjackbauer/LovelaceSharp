using Rat = global::Lovelace.Rational.Rational;
using Int = global::Lovelace.Integer.Integer;

namespace Lovelace.Symbolics;

/// <summary>
/// Factorization over Q (v1: content + square-free decomposition + rational-root linear
/// factors; residual factors are returned whole — never claimed irreducible beyond what is
/// proven). This is sound: solve() covers residual roots via RootOf.
/// </summary>
public static class Factoring
{
    /// <summary>Factors an expression when it is a polynomial over its symbols; otherwise returns it unchanged.</summary>
    public static Expr Factor(Expr e, ExprContext ctx)
    {
        var vars = CollectSymbolsSorted(e);
        if (vars.Length == 0)
            return e;
        if (vars.Length != 1)
            return e;   // multivariate factorization over Q is a later layer (Groebner track)
        if (!Polynomial.TryFromExpr(e, ctx, vars, out var poly, out _))
            return e;
        var factors = FactorPoly(poly);
        // rebuild: content * product of factor^mult
        var product = new List<Expr> { Exprs.Rational(factors.Content) };
        foreach (var (f, m) in factors.Factors)
        {
            var fe = f.ToExpr();
            product.Add(m == 1 ? fe : Exprs.Power(fe, Exprs.Integer(m)));
        }
        return product.Count == 1 ? product[0] : Exprs.Multiply(product);
    }

    public sealed record PolyFactors(Rat Content, List<(Polynomial Factor, int Multiplicity)> Factors);

    public static PolyFactors FactorPoly(Polynomial f)
    {
        if (f.IsZero)
            return new PolyFactors(Rat.Zero, new List<(Polynomial, int)>());

        // 1. content
        var (primitive, content) = PrimitivePart(f);
        var factors = new List<(Polynomial, int)>();

        // 2. square-free decomposition of the primitive part.
        // SquareFreeUnivariate returns MONIC factors, so the leading coefficient of the primitive
        // part is a unit that the decomposition does not carry. It must be folded back into the
        // content here, otherwise it is silently discarded (e.g. -x^2+1 factored to (x-1)*(x+1),
        // whose product is x^2-1 — the sign of the leading coefficient was lost).
        var leading = primitive.IsZero ? Rat.One : primitive.LeadingCoefficient(MonomialOrder.Lex);
        var sqfree = Polynomial.SquareFreeUnivariate(primitive);
        foreach (var (sf, mult) in sqfree)
        {
            // 3. rational-root linear extraction on each square-free factor
            var remaining = sf;
            var roots = remaining.RationalRoots();
            foreach (var r in roots)
            {
                int multiplicity = mult;
                // count the actual multiplicity of r in remaining (should be 1 after square-free)
                var lin = LinearFactor(sf.Order, r);
                factors.Add((lin, multiplicity));
                var (q, rem) = remaining.DivRem(lin, MonomialOrder.Lex);
                if (rem.IsZero)
                    remaining = q;
            }
            if (!remaining.IsOne && !remaining.IsZero)
                factors.Add((remaining, mult));
        }
        if (factors.Count == 0)
        {
            // Nothing was emitted: the primitive part itself is a unit, returned unmoved here —
            // so it already carries its own leading coefficient and must NOT be scaled again.
            factors.Add((primitive, 1));
        }
        else
        {
            // Every emitted factor is monic; restore the discarded leading coefficient.
            content = content * leading;
        }
        return new PolyFactors(content, factors);
    }

    private static Polynomial LinearFactor(VariableOrder order, Rat root)
    {
        // x - root
        var p = Polynomial.FromVar(order, 0);
        return Polynomial.Add(p, Polynomial.FromConstant(order, Rat.Negate(root)));
    }

    /// <summary>Splits a polynomial into (primitive part, content) using coefficient gcds.</summary>
    public static (Polynomial Primitive, Rat Content) PrimitivePart(Polynomial f)
    {
        if (f.IsZero)
            return (f, Rat.One);
        Rat content = Rat.Zero;
        foreach (var (_, c) in f.Terms)
        {
            if (content.IsZero)
                content = Rat.Abs(c);
            else
                content = GcdRational(content, Rat.Abs(c));
        }
        if (content.IsZero)
            return (f, Rat.One);
        var r = new Polynomial(f.Order);
        foreach (var (m, c) in f.Terms)
            r = Polynomial.Add(r, Polynomial.FromMonomial(f.Order, m, c / content));
        return (r, content);
    }

    /// <summary>GCD of two rationals: gcd(num)/lcm(den).</summary>
    public static Rat GcdRational(Rat a, Rat b)
    {
        var num = Int.Gcd(a.Numerator, b.Numerator);
        var den = Int.Lcm(a.Denominator, b.Denominator);
        return Rat.From(num, den);
    }

    private static Symbol[] CollectSymbolsSorted(Expr e)
    {
        var set = new SortedSet<Symbol>();
        void Walk(Expr x)
        {
            switch (x)
            {
                case SymbolExpr s: set.Add(s.Symbol); break;
                case AddExpr a: foreach (var t in a.Terms) Walk(t); break;
                case MultiplyExpr m: foreach (var f in m.Factors) Walk(f); break;
                case PowerExpr p: Walk(p.Base); Walk(p.Exponent); break;
                case FunctionExpr f: foreach (var arg in f.Arguments) Walk(arg); break;
            }
        }
        Walk(e);
        return set.ToArray();
    }
}
