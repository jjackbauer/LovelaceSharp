using Lovelace.Symbolics;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// One differential case. <paramref name="Domain"/> is not decoration: it states the domain and
/// branch assumptions under which the kernel form and the SymPy form are allowed to be compared,
/// so a later reader can tell a genuine disagreement from a comparison made off the domain.
/// </summary>
internal sealed record DerivativeCase(string Domain, Expr Expr, string Sympy, IReadOnlyList<IReadOnlyList<string>> Points);

/// <summary>A solve case: the equation is <c>Expr == 0</c> and the domain is the solver's.
/// <paramref name="KernelStatus"/> is the disposition the kernel is REQUIRED to report for that
/// equation: <see cref="SolveStatus.Solved"/> for every case whose kernel half is a complete
/// answer, and an explicitly DECLARED non-answer only where SymPy's solution set is empty — so the
/// comparison there is "SymPy proves there is nothing and the kernel represents nothing", never a
/// kernel non-answer matched against a list of roots.</summary>
internal sealed record SolveCase(string Domain, Expr Expr, string Sympy, SolveStatus KernelStatus = SolveStatus.Solved);

/// <summary>A real-root case, compared against sympy.real_roots.</summary>
internal sealed record RootCase(string Domain, Expr Expr, string Sympy);

/// <summary>A factorization case over Q[x], compared against sympy.factor_list.</summary>
internal sealed record FactorCase(string Domain, Expr Expr, string Sympy);

/// <summary>A limit case with its direction; <paramref name="SympyPoint"/> is python source for
/// the point (a literal or <c>sympy.oo</c>).</summary>
internal sealed record LimitCase(string Domain, Expr Expr, string Sympy, Expr Point, string SympyPoint, LimitDirection Direction);

/// <summary>A matrix case: <paramref name="Operation"/> is det / trace / rank / inverse.</summary>
internal sealed record MatrixCase(string Domain, string Operation, Expr[][] Kernel, string Sympy, IReadOnlyList<IReadOnlyList<string>> Points);

/// <summary>
/// The differential oracle's corpora: every case carries the SymPy expression it is compared
/// with and the domain/branch assumptions the comparison is valid under. The cases are chosen so
/// that BOTH sides have a definite answer on the stated domain — a case whose kernel half is
/// Unevaluated would compare a non-answer and is not a corpus entry, UNLESS the case declares that
/// kernel non-answer in SolveCase.KernelStatus because SymPy's side is the EMPTY set: there the
/// definite claim on both sides is "nothing is a solution", and the case exists to prove the kernel
/// does not represent a root it cannot justify (see the sqrt(x) + 2 case).
/// </summary>
internal static class OracleCorpus
{
    /// <summary>The original sample points of the derivative corpus: real, non-zero, no pole.</summary>
    internal static readonly IReadOnlyList<IReadOnlyList<string>> RealPoints = new[]
    {
        new[] { "1/3" }, new[] { "-1/2" }, new[] { "7/5" },
    };

    /// <summary>Strictly positive points, for cases whose function is real only on x &gt; 0.</summary>
    internal static readonly IReadOnlyList<IReadOnlyList<string>> PositivePoints = new[]
    {
        new[] { "1/3" }, new[] { "4/5" }, new[] { "7/5" },
    };

    /// <summary>Points for the 2-variable matrix cases; det x^2 - y is non-zero at both.</summary>
    internal static readonly IReadOnlyList<IReadOnlyList<string>> MatrixPoints = new[]
    {
        new[] { "1/3", "7/5" }, new[] { "2", "-3/4" },
    };

    private static readonly IReadOnlyList<IReadOnlyList<string>> NoPoints = Array.Empty<IReadOnlyList<string>>();

    internal static IReadOnlyList<DerivativeCase> Derivatives(ExprContext ctx)
    {
        Symbol x = ctx.Symbol("x");
        return new[]
        {
            new DerivativeCase(
                "polynomial, entire over the reals; compared at x = 1/3, -1/2, 7/5",
                Exprs.Power(x, 7), "x**7", RealPoints),
            new DerivativeCase(
                "sin(x^2), entire over the reals; compared at x = 1/3, -1/2, 7/5",
                Exprs.Function(ctx.Function("sin"), Exprs.Power(x, 2)), "sin(x**2)", RealPoints),
            new DerivativeCase(
                "exp(-x^2), entire over the reals; compared at x = 1/3, -1/2, 7/5",
                Exprs.Function(ctx.Function("exp"), Exprs.Negate(Exprs.Power(x, 2))), "exp(-x**2)", RealPoints),
            new DerivativeCase(
                "x*log(x) on the PRINCIPAL branch of log, which is what makes this case comparable at a " +
                "NEGATIVE point: log is real for x > 0 and is log|x| + i*pi for x < 0 (the branch cut runs " +
                "along the negative real axis with arg(z) = +pi, i.e. taken from above), and that is exactly " +
                "what SymPy returns there — log(-1/2) = -log(2) + I*pi. Compared at x = 1/3, -1/2, 7/5. " +
                "REPAIR (round 03): the corpus used to compare this case only at positive points, because " +
                "the kernel's numeric evaluator raised EvaluationException at x = -1/2 while SymPy returned " +
                "that complex value, so the comparison put a non-answer against an answer and was unsound " +
                "there (see the note at the top of this file: a case whose kernel half is a non-answer is " +
                "not a corpus entry). The evaluator now returns the same principal value SymPy does, so the " +
                "excluded point is REINSTATED and the comparison is live again.",
                Exprs.Multiply(x, Exprs.Function(ctx.Function("log"), x)), "x*log(x)", RealPoints),
            new DerivativeCase(
                "sin(x)/x on x != 0 (the removable singularity is not sampled); compared at x = 1/3, -1/2, 7/5",
                Exprs.Divide(Exprs.Function(ctx.Function("sin"), x), x), "sin(x)/x", RealPoints),
            new DerivativeCase(
                "tan(x^3) away from the poles x^3 = pi/2 + k*pi; compared at x = 1/3, -1/2, 7/5",
                Exprs.Function(ctx.Function("tan"), Exprs.Power(x, 3)), "tan(x**3)", RealPoints),
        };
    }

    internal static IReadOnlyList<SolveCase> Solve(ExprContext ctx)
    {
        Symbol x = ctx.Symbol("x");
        const string domain =
            "complete solution set over the complex field (the kernel default): the distinct-root count is " +
            "compared with sympy.solve, and every kernel root is substituted back and must expand to exactly 0";
        return new[]
        {
            new SolveCase(domain, Exprs.Add(Exprs.Power(x, 2), Exprs.Multiply(-4, x), 4), "x**2 - 4*x + 4"),
            new SolveCase(domain, Exprs.Add(Exprs.Power(x, 2), Exprs.Multiply(-5, x), 6), "x**2 - 5*x + 6"),
            new SolveCase(domain, Exprs.Add(Exprs.Power(x, 3), 1), "x**3 + 1"),
            new SolveCase(domain, Exprs.Add(Exprs.Power(x, 4), Exprs.Multiply(-5, Exprs.Power(x, 2)), 4), "x**4 - 5*x**2 + 4"),
            // round 03: a NEGATIVE discriminant, so the corpus exercises the closed-form complex
            // values instead of only real roots. sympy.solve(x**2 + 1, x) = [-I, I], and the two
            // kernel roots substitute back to exactly 0 only because i^2 folds to -1.
            new SolveCase(domain, Exprs.Add(Exprs.Power(x, 2), 1), "x**2 + 1"),
            new SolveCase(domain, Exprs.Add(Exprs.Power(x, 2), 4), "x**2 + 4"),
            // cycle 5 (round 05): the inverse branch that squaring cannot preserve. sympy.solve
            // returns [] because the PRINCIPAL square root is never -2, so the branch candidate
            // x = (-2)^2 = 4 is EXTRANEOUS (sqrt(4) = +2). Before the fix the kernel returned that
            // candidate as a Solved complete set (complete true, represented_count 1); now it
            // represents nothing and does not claim completeness. The kernel half is deliberately
            // NOT a complete answer: rejecting every candidate numerically is not a proof that the
            // solution set is empty, so NoSolutions would be a claim the verification cannot back.
            new SolveCase(
                "the kernel default domain (complex), and the case's kernel half is deliberately NOT a " +
                "complete answer: sympy.solve(sqrt(x) + 2, x) is the EMPTY set, because the principal " +
                "square root is never a negative real. The kernel must represent NO root here; it reports " +
                "Unevaluated (not NoSolutions) because its inverse-branch gate rejects the extraneous " +
                "candidate with a NUMERIC residual test (sqrt(4) = 2, not -2) and a numeric rejection of " +
                "every candidate is not a proof of emptiness. The comparison is: sympy's distinct-root " +
                "count is 0, the kernel's represented count is 0, and the kernel does not claim completeness.",
                Exprs.Add(Exprs.Power(x, Exprs.Rational(1, 2)), 2), "sqrt(x) + 2", SolveStatus.Unevaluated),
        };
    }

    internal static IReadOnlyList<RootCase> Roots(ExprContext ctx)
    {
        Symbol x = ctx.Symbol("x");
        const string domain =
            "complete set of DISTINCT real roots (SolveDomain.Real; a degree >= 4 factor is returned as RootOf, " +
            "so multiplicity is not represented and the distinct-set reading is used on both sides): the count and " +
            "every value are compared with sympy.real_roots at 30 significant digits";
        return new[]
        {
            new RootCase(domain, Exprs.Add(Exprs.Power(x, 4), Exprs.Multiply(-5, Exprs.Power(x, 2)), 4), "x**4 - 5*x**2 + 4"),
            new RootCase(domain, Exprs.Add(Exprs.Power(x, 2), -2), "x**2 - 2"),
            new RootCase(domain, Exprs.Add(Exprs.Power(x, 4), -2), "x**4 - 2"),
            new RootCase(domain, Exprs.Add(Exprs.Power(x, 5), Exprs.Multiply(-3, x), 1), "x**5 - 3*x + 1"),
            new RootCase(domain, Exprs.Add(Exprs.Power(x, 3), -2), "x**3 - 2"),
            new RootCase(domain + "as above, plus a repeated root: (x - 1)^3 collapses to the single distinct value 1 on both sides",
                Exprs.Add(Exprs.Power(x, 3), Exprs.Multiply(-3, Exprs.Power(x, 2)), Exprs.Multiply(3, x), -1),
                "x**3 - 3*x**2 + 3*x - 1"),
            new RootCase(domain, Exprs.Add(Exprs.Power(x, 5), Exprs.Negate(x), -1), "x**5 - x - 1"),
            new RootCase(domain + "no real root at all: the kernel must claim NoSolutions and sympy.real_roots must be empty",
                Exprs.Add(Exprs.Power(x, 2), 1), "x**2 + 1"),
        };
    }

    internal static IReadOnlyList<FactorCase> Factorizations(ExprContext ctx)
    {
        Symbol x = ctx.Symbol("x");
        const string domain =
            "univariate polynomial over Q[x]: the degree/multiplicity profile of the kernel factorization is " +
            "compared with sympy.factor_list, and the kernel product must expand back to the exact input " +
            "(the kernel returns an irreducible residual whole rather than claiming it is reducible). " +
            "NOTE: a polynomial with a NEGATIVE leading coefficient is deliberately absent — Factoring.Factor " +
            "drops the sign there (observed: Factor(-x^2 + 1) = (x - 1)*(x + 1), which expands to x^2 - 1, not " +
            "to -x^2 + 1). That is a kernel defect outside this round's scope; add the case once it is fixed.";
        return new[]
        {
            new FactorCase(domain, Exprs.Add(Exprs.Power(x, 2), Exprs.Multiply(-5, x), 6), "x**2 - 5*x + 6"),
            new FactorCase(domain, Exprs.Add(Exprs.Power(x, 3), 1), "x**3 + 1"),
            new FactorCase(domain, Exprs.Add(Exprs.Power(x, 4), Exprs.Multiply(-5, Exprs.Power(x, 2)), 4), "x**4 - 5*x**2 + 4"),
            new FactorCase(domain + "square-free decomposition: (x - 1)^3 has one distinct factor of degree 1 with multiplicity 3",
                Exprs.Add(Exprs.Power(x, 3), Exprs.Multiply(-3, Exprs.Power(x, 2)), Exprs.Multiply(3, x), -1),
                "x**3 - 3*x**2 + 3*x - 1"),
            new FactorCase(domain, Exprs.Add(Exprs.Multiply(2, Exprs.Power(x, 2)), -8), "2*x**2 - 8"),
            new FactorCase(domain, Exprs.Add(Exprs.Power(x, 4), -1), "x**4 - 1"),
            new FactorCase(domain + "irreducible over Q: the residual quadratic is returned whole, exactly as sympy.factor_list keeps it",
                Exprs.Add(Exprs.Power(x, 2), -2), "x**2 - 2"),
            new FactorCase(domain, Exprs.Add(Exprs.Multiply(6, Exprs.Power(x, 3)), Exprs.Multiply(-6, x)), "6*x**3 - 6*x"),
            new FactorCase(domain + "a repeated linear factor with content 1: (x + 1)^2 * (x - 1)",
                Exprs.Add(Exprs.Power(x, 3), Exprs.Power(x, 2), Exprs.Negate(x), -1), "x**3 + x**2 - x - 1"),
        };
    }

    internal static IReadOnlyList<LimitCase> Limits(ExprContext ctx)
    {
        Symbol x = ctx.Symbol("x");
        FunctionId sin = ctx.Function("sin");
        FunctionId cos = ctx.Function("cos");
        FunctionId tan = ctx.Function("tan");
        Expr infinite = Exprs.Infinity;

        const string twoSidedAgrees =
            "two-sided real limit at a point where both sides agree, so sympy.limit(dir='+') is the same value " +
            "the kernel's TwoSided evaluation must produce";
        const string poleRight =
            "one-sided real limit from the RIGHT at an odd pole (the two sides disagree, so a two-sided reading " +
            "would be DoesNotExist); compared with sympy.limit(..., dir='+')";
        const string poleLeft =
            "one-sided real limit from the LEFT at an odd pole; compared with sympy.limit(..., dir='-')";
        const string oddPole =
            "two-sided real limit at an ODD pole: the sides disagree, so the kernel must report DoesNotExist with " +
            "both one-sided results, each compared with sympy.limit(dir='-') and sympy.limit(dir='+')";
        const string evenPole =
            "two-sided real limit at an EVEN pole: both sides run to +infinity, compared with sympy.limit(dir='+')";
        const string atInfinity =
            "real limit at infinity by exact degree comparison of the rational function; compared with " +
            "sympy.limit(..., x, oo) / (..., x, -oo)";

        return new[]
        {
            new LimitCase(twoSidedAgrees,
                Exprs.Divide(Exprs.Function(sin, x), x), "sin(x)/x", Exprs.Zero, "0", LimitDirection.TwoSided),
            new LimitCase(twoSidedAgrees,
                Exprs.Divide(Exprs.Subtract(1, Exprs.Function(cos, x)), Exprs.Power(x, 2)),
                "(1 - cos(x))/x**2", Exprs.Zero, "0", LimitDirection.TwoSided),
            new LimitCase(twoSidedAgrees,
                Exprs.Divide(Exprs.Subtract(Exprs.Power(x, 3), 8), Exprs.Subtract(x, 2)),
                "(x**3 - 8)/(x - 2)", Exprs.Integer(2), "2", LimitDirection.TwoSided),
            new LimitCase(twoSidedAgrees,
                Exprs.Divide(Exprs.Subtract(Exprs.Power(x, 2), 1), Exprs.Subtract(x, 1)),
                "(x**2 - 1)/(x - 1)", Exprs.One, "1", LimitDirection.TwoSided),
            new LimitCase(twoSidedAgrees,
                Exprs.Divide(Exprs.Function(tan, x), x), "tan(x)/x", Exprs.Zero, "0", LimitDirection.TwoSided),
            new LimitCase(evenPole,
                Exprs.Divide(Exprs.One, Exprs.Power(x, 2)), "1/x**2", Exprs.Zero, "0", LimitDirection.TwoSided),
            new LimitCase(evenPole,
                Exprs.Divide(Exprs.One, Exprs.Power(Exprs.Subtract(x, 1), 2)), "1/(x - 1)**2",
                Exprs.One, "1", LimitDirection.TwoSided),
            new LimitCase(poleRight,
                Exprs.Divide(Exprs.One, x), "1/x", Exprs.Zero, "0", LimitDirection.FromRight),
            new LimitCase(poleLeft,
                Exprs.Divide(Exprs.One, x), "1/x", Exprs.Zero, "0", LimitDirection.FromLeft),
            new LimitCase(oddPole,
                Exprs.Divide(Exprs.One, x), "1/x", Exprs.Zero, "0", LimitDirection.TwoSided),
            new LimitCase(oddPole,
                Exprs.Divide(Exprs.One, Exprs.Subtract(x, 1)), "1/(x - 1)", Exprs.One, "1", LimitDirection.TwoSided),
            new LimitCase(atInfinity,
                Exprs.Divide(
                    Exprs.Add(Exprs.Multiply(3, Exprs.Power(x, 2)), Exprs.Multiply(5, x)),
                    Exprs.Subtract(Exprs.Multiply(2, Exprs.Power(x, 2)), x)),
                "(3*x**2 + 5*x)/(2*x**2 - x)", infinite, "sympy.oo", LimitDirection.TwoSided),
            new LimitCase(atInfinity,
                Exprs.Divide(
                    Exprs.Add(Exprs.Multiply(3, Exprs.Power(x, 2)), Exprs.Multiply(5, x)),
                    Exprs.Subtract(Exprs.Multiply(2, Exprs.Power(x, 2)), x)),
                "(3*x**2 + 5*x)/(2*x**2 - x)", Exprs.Negate(infinite), "-sympy.oo", LimitDirection.TwoSided),
            new LimitCase(atInfinity,
                Exprs.Divide(x, Exprs.Add(Exprs.Power(x, 2), 1)), "x/(x**2 + 1)",
                infinite, "sympy.oo", LimitDirection.TwoSided),
            new LimitCase(atInfinity,
                Exprs.Divide(
                    Exprs.Subtract(Exprs.Multiply(2, Exprs.Power(x, 3)), x),
                    Exprs.Add(Exprs.Power(x, 3), 1)),
                "(2*x**3 - x)/(x**3 + 1)", infinite, "sympy.oo", LimitDirection.TwoSided),
            new LimitCase(atInfinity,
                Exprs.Subtract(Exprs.Power(x, 3), Exprs.Multiply(2, x)), "x**3 - 2*x",
                infinite, "sympy.oo", LimitDirection.TwoSided),
            new LimitCase(atInfinity,
                Exprs.Subtract(Exprs.Power(x, 3), Exprs.Multiply(2, x)), "x**3 - 2*x",
                Exprs.Negate(infinite), "-sympy.oo", LimitDirection.TwoSided),
        };
    }

    internal static IReadOnlyList<MatrixCase> MatrixOperations(ExprContext ctx)
    {
        Symbol x = ctx.Symbol("x");
        Symbol y = ctx.Symbol("y");
        const string detDomain =
            "determinant as a polynomial identity in the entries over Q(x, y); compared with sympy Matrix.det() " +
            "at two rational parameter points";
        const string detNumericDomain =
            "determinant of an exact rational matrix over Q; compared with sympy Matrix.det() at two parameter points";
        const string traceDomain =
            "trace as a polynomial identity in the entries over Q(x, y); compared with sympy Matrix.trace()";
        const string traceNumericDomain =
            "trace of an exact rational matrix over Q; compared with sympy Matrix.trace()";
        const string rankNumericDomain =
            "rank of an exact rational matrix over Q (no parameters, so no sample points); compared with " +
            "sympy Matrix.rank()";
        const string rankSymbolicDomain =
            "GENERIC rank over the rational function field Q(x, y): the rank holds for every parameter value " +
            "outside the vanishing set of the pivot minors, which is the convention sympy Matrix.rank() applies " +
            "to symbolic entries; no sample points";
        const string inverseDomain =
            "inverse via the adjugate over the determinant, valid where det != 0 (verified at both sample points); " +
            "every entry is compared with sympy Matrix.inv() row-major";

        Expr[][] symbolic2x2 = { new Expr[] { x, Exprs.One }, new Expr[] { y, x } };
        Expr[][] numeric3x3 = { new Expr[] { 2, 0, 1 }, new Expr[] { 1, 3, 2 }, new Expr[] { 1, 1, 4 } };
        Expr[][] numericInvertible3x3 = { new Expr[] { 2, 0, 1 }, new Expr[] { 1, 3, 2 }, new Expr[] { 1, 1, 3 } };

        return new[]
        {
            new MatrixCase(detDomain, "det", symbolic2x2, "sympy.Matrix([[x, 1], [y, x]])", MatrixPoints),
            new MatrixCase(detDomain, "det",
                new[] { new Expr[] { x, Exprs.One, Exprs.Zero }, new Expr[] { Exprs.Zero, x, Exprs.One }, new Expr[] { Exprs.One, Exprs.Zero, x } },
                "sympy.Matrix([[x, 1, 0], [0, x, 1], [1, 0, x]])", MatrixPoints),
            new MatrixCase(detNumericDomain, "det", numeric3x3, "sympy.Matrix([[2, 0, 1], [1, 3, 2], [1, 1, 4]])", MatrixPoints),
            new MatrixCase(traceDomain, "trace", symbolic2x2, "sympy.Matrix([[x, 1], [y, x]])", MatrixPoints),
            new MatrixCase(traceNumericDomain, "trace", numeric3x3, "sympy.Matrix([[2, 0, 1], [1, 3, 2], [1, 1, 4]])", MatrixPoints),
            new MatrixCase(rankNumericDomain, "rank",
                new[] { new Expr[] { 1, 2 }, new Expr[] { 2, 4 } }, "sympy.Matrix([[1, 2], [2, 4]])", NoPoints),
            new MatrixCase(rankNumericDomain, "rank",
                new[] { new Expr[] { 1, 2, 3 }, new Expr[] { 4, 5, 6 }, new Expr[] { 7, 8, 9 } },
                "sympy.Matrix([[1, 2, 3], [4, 5, 6], [7, 8, 9]])", NoPoints),
            new MatrixCase(rankNumericDomain, "rank", numericInvertible3x3,
                "sympy.Matrix([[2, 0, 1], [1, 3, 2], [1, 1, 3]])", NoPoints),
            new MatrixCase(rankSymbolicDomain, "rank",
                new[] { new Expr[] { x, Exprs.One }, new Expr[] { Exprs.Zero, y } },
                "sympy.Matrix([[x, 1], [0, y]])", NoPoints),
            new MatrixCase(rankSymbolicDomain, "rank",
                new[] { new Expr[] { x, x }, new Expr[] { x, x } }, "sympy.Matrix([[x, x], [x, x]])", NoPoints),
            new MatrixCase(inverseDomain, "inverse", symbolic2x2, "sympy.Matrix([[x, 1], [y, x]])", MatrixPoints),
            new MatrixCase(inverseDomain, "inverse",
                new[] { new Expr[] { x, 2 }, new Expr[] { 3, y } }, "sympy.Matrix([[x, 2], [3, y]])", MatrixPoints),
            new MatrixCase(inverseDomain, "inverse", numeric3x3, "sympy.Matrix([[2, 0, 1], [1, 3, 2], [1, 1, 4]])", MatrixPoints),
        };
    }

    /// <summary>
    /// The kernel factorization written as <c>ddd:ddd</c> (degree:multiplicity) tokens, sorted —
    /// the same encoding <see cref="FactorProfileScript"/> makes SymPy print, so the two strings
    /// compare directly and a mismatch shows both without further decoding.
    /// </summary>
    internal static string DegreeProfile(Expr factored, ExprContext ctx, Symbol x)
    {
        var parts = new List<string>();
        Visit(factored);
        parts.Sort(StringComparer.Ordinal);
        return string.Join(";", parts);

        void Visit(Expr e)
        {
            switch (e)
            {
                case MultiplyExpr m:
                    foreach (Expr factor in m.Factors)
                        Visit(factor);
                    return;
                case PowerExpr p when IntegerExponent(p.Exponent) is int power && power > 1:
                    parts.Add($"{DegreeOf(p.Base, ctx, x):D3}:{power:D3}");
                    return;
                case RationalConstantExpr or IntegerConstantExpr or RealConstantExpr:
                    return;   // content: a degree-0 factor carries no degree/multiplicity information
                default:
                    parts.Add($"{DegreeOf(e, ctx, x):D3}:{1:D3}");
                    return;
            }
        }
    }

    private static int? IntegerExponent(Expr e) =>
        e is RationalConstantExpr rc && rc.Value.IsInteger
            && int.TryParse(rc.Value.ToInteger().ToString(), out int value)
                ? value
                : null;

    private static int DegreeOf(Expr e, ExprContext ctx, Symbol x) =>
        Polynomial.TryFromExpr(e, ctx, new[] { x }, out Polynomial poly, out _) ? poly.TotalDegree : -999;

    /// <summary>The SymPy program that prints the same <c>ddd:ddd</c> token string as
    /// <see cref="DegreeProfile"/>, plus the factored form for the mismatch message.</summary>
    internal static string FactorProfileScript(string sympy, Symbol x) =>
        $"{SympyScript.Symbols(new[] { x.Name })}\n" +
        $"coefficient, factors = sympy.factor_list({sympy}, {x.Name})\n" +
        "print(';'.join('%03d:%03d' % (sympy.degree(f, x), m) for f, m in " +
        "sorted(factors, key=lambda t: (sympy.degree(t[0], x), t[1]))))\n" +
        $"print(sympy.sstr(sympy.factor({sympy})))";
}
