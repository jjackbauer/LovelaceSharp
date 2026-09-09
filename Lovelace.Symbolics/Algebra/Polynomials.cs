using Rat = global::Lovelace.Rational.Rational;

namespace Lovelace.Symbolics;

/// <summary>
/// Public facade over the sparse <see cref="Polynomial"/> internals. Univariate algorithms
/// here treat variable index 0 as the main variable; coefficients are exact <see cref="Rat"/>
/// values throughout.
/// </summary>
public static class Polynomials
{
    private static readonly MonomialComparer GrLex = new(MonomialOrder.GrLex);

    // -----------------------------------------------------------------
    // Construction / inspection
    // -----------------------------------------------------------------

    /// <summary>Converts an expression to a polynomial in the given variables.</summary>
    /// <exception cref="NotPolynomialException">When the expression is not a polynomial.</exception>
    public static Polynomial FromExpr(Expr e, ExprContext ctx, params Symbol[] vars) =>
        Polynomial.FromExpr(e, ctx, vars);

    /// <summary>The degree in variable 0, or -1 for the zero polynomial.</summary>
    public static int Degree(Polynomial p) => UnivariateDegree(p);

    /// <summary>Coefficient of the exact monomial x0^exps[0]·x1^exps[1]·…, or 0 when absent.</summary>
    public static Rat Coefficient(Polynomial p, params int[] exps)
    {
        int varCount = p.Order.Variables.Length;
        foreach (var (m, c) in p.Terms)
        {
            bool match = true;
            for (int i = 0; i < exps.Length; i++)
            {
                if (exps[i] != m[i]) { match = false; break; }
            }
            if (!match) continue;
            for (int i = exps.Length; i < varCount; i++)
            {
                if (m[i] != 0) { match = false; break; }
            }
            if (match) return c;
        }
        return Rat.Zero;
    }

    /// <summary>Terms sorted deterministically by GrLex.</summary>
    public static IReadOnlyList<(Monomial, Rat)> Terms(Polynomial p)
    {
        var list = new List<(Monomial, Rat)>();
        foreach (var (m, c) in p.Terms) list.Add((m, c));
        list.Sort((x, y) => GrLex.Compare(x.Item1, y.Item1));
        return list;
    }

    /// <summary>The rational content of the polynomial.</summary>
    public static Rat Content(Polynomial p) => Factoring.PrimitivePart(p).Content;

    /// <summary>The primitive part (coefficients have content 1).</summary>
    public static Polynomial PrimitivePart(Polynomial p) => Factoring.PrimitivePart(p).Primitive;

    // -----------------------------------------------------------------
    // Arithmetic
    // -----------------------------------------------------------------

    /// <summary>Univariate division under Lex order.</summary>
    public static (Polynomial Quotient, Polynomial Remainder) Divide(Polynomial a, Polynomial b) =>
        a.DivRem(b, MonomialOrder.Lex);

    /// <summary>Monic Euclidean GCD over Q.</summary>
    public static Polynomial Gcd(Polynomial a, Polynomial b) => Polynomial.GcdUnivariate(a, b);

    /// <summary>Least common multiple; 0 when either argument is zero.</summary>
    public static Polynomial Lcm(Polynomial a, Polynomial b)
    {
        if (a.IsZero || b.IsZero) return Polynomial.Zero(a.Order);
        var g = Gcd(a, b);
        var q = a.DivRem(g, MonomialOrder.Lex).Quotient;
        return Polynomial.Multiply(q, b);
    }

    // -----------------------------------------------------------------
    // Structure
    // -----------------------------------------------------------------

    /// <summary>Yun's square-free decomposition: (factor, multiplicity) pairs.</summary>
    public static List<(Polynomial Factor, int Multiplicity)> SquareFree(Polynomial p) =>
        Polynomial.SquareFreeUnivariate(p);

    /// <summary>Factorization over Q (content + square-free + rational-root linear factors).</summary>
    public static Factoring.PolyFactors Factor(Polynomial p) => Factoring.FactorPoly(p);

    /// <summary>All rational roots via the rational root theorem.</summary>
    public static List<Rat> Roots(Polynomial p) => p.RationalRoots();

    // -----------------------------------------------------------------
    // Resultant / Discriminant
    // -----------------------------------------------------------------

    /// <summary>
    /// Resultant of two univariate polynomials as the determinant of the Sylvester matrix,
    /// computed exactly (fraction-free Bareiss) over Q. Zero for a zero argument.
    /// </summary>
    public static Rat Resultant(Polynomial f, Polynomial g)
    {
        var (m, fc) = UnivariateCoefficients(f);
        var (n, gc) = UnivariateCoefficients(g);
        if (m < 0 || n < 0) return Rat.Zero;

        int size = m + n;
        if (size == 0) return Rat.One; // both constants

        var matrix = new Rat[size, size];
        for (int i = 0; i < size; i++)
            for (int j = 0; j < size; j++)
                matrix[i, j] = Rat.Zero;

        // n rows of f (ascending coefficients, shifted)
        for (int i = 0; i < n; i++)
            for (int j = 0; j <= m; j++)
                matrix[i, i + j] = fc[j];

        // m rows of g (ascending coefficients, shifted)
        for (int i = 0; i < m; i++)
            for (int j = 0; j <= n; j++)
                matrix[n + i, i + j] = gc[j];

        return Determinant(matrix);
    }

    /// <summary>
    /// Discriminant of a univariate polynomial: (-1)^(n(n-1)/2) · res(f, f') / lc(f).
    /// </summary>
    public static Rat Discriminant(Polynomial f)
    {
        var (n, fc) = UnivariateCoefficients(f);
        if (n < 0) return Rat.Zero;
        if (n == 0) return Rat.One;

        var lc = fc[n];
        var derivative = f.Derivative(0);
        var res = Resultant(f, derivative);

        bool flip = ((n * (n - 1)) / 2) % 2 != 0;
        var signed = flip ? Rat.Negate(res) : res;
        return signed / lc;
    }

    // -----------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------

    private static int UnivariateDegree(Polynomial p)
    {
        if (p.IsZero) return -1;
        int d = -1;
        foreach (var (m, _) in p.Terms)
            if (m[0] > d) d = m[0];
        return d;
    }

    private static (int Degree, Rat[] Coefficients) UnivariateCoefficients(Polynomial p)
    {
        int degree = UnivariateDegree(p);
        if (degree < 0) return (-1, Array.Empty<Rat>());
        var coeffs = new Rat[degree + 1];
        for (int i = 0; i <= degree; i++) coeffs[i] = Rat.Zero;
        foreach (var (m, c) in p.Terms) coeffs[m[0]] = c;
        return (degree, coeffs);
    }

    /// <summary>Exact determinant over Q via fraction-free Bareiss elimination.</summary>
    private static Rat Determinant(Rat[,] a)
    {
        int n = a.GetLength(0);
        if (n == 0) return Rat.One;
        if (n == 1) return a[0, 0];

        var m = new Rat[n, n];
        for (int i = 0; i < n; i++)
            for (int j = 0; j < n; j++)
                m[i, j] = a[i, j];

        Rat prev = Rat.One;
        int sign = 1;

        for (int k = 0; k < n - 1; k++)
        {
            int pivot = k;
            while (pivot < n && m[pivot, k].IsZero) pivot++;
            if (pivot == n) return Rat.Zero;
            if (pivot != k)
            {
                for (int j = 0; j < n; j++)
                    (m[pivot, j], m[k, j]) = (m[k, j], m[pivot, j]);
                sign = -sign;
            }

            for (int i = k + 1; i < n; i++)
            {
                for (int j = k + 1; j < n; j++)
                    m[i, j] = (m[i, j] * m[k, k] - m[i, k] * m[k, j]) / prev;
                m[i, k] = Rat.Zero;
            }

            prev = m[k, k];
        }

        var det = m[n - 1, n - 1];
        return sign < 0 ? Rat.Negate(det) : det;
    }
}
