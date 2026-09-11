using Lovelace.Symbolics;
using Rl = Lovelace.Real.Real;
using Rat = Lovelace.Rational.Rational;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Round 03 (goal cycle 4): the closed-form complex values the kernel now answers EXACTLY.
///
/// <para>Every expected value in this file is SymPy 1.14.0 ground truth, obtained by running the
/// oracle interpreter directly (the commands and their output are transcribed in
/// <c>docs/goal-cycle-4/round-03/implementation.md</c>). The branch is the PRINCIPAL branch,
/// which is what <c>(-a)**e</c> means in SymPy for exact positive <c>a</c> — verified per case
/// rather than assumed:</para>
/// <code>
/// (-1)**Rational(1,2)   = I                    (-4)**Rational(1,2)  = 2*I
/// (-3)**Rational(1,2)   = sqrt(3)*I            (-1/4)**Rational(1,2) = I/2
/// (-4)**Rational(3,2)   = -8*I                 (-4)**Rational(-1,2) = -I/2
/// solve(x**2 + 1, x)    = [-I, I]              solve(x**2 + 2*x + 5, x) = [-1 - 2*I, -1 + 2*I]
/// </code>
///
/// <para>The bounded target set of this round is the exponent with denominator exactly 2 over an
/// exact negative rational base: <c>(-a)^(m/2) = a^(m/2)·i^m</c>. Denominators &gt;= 3 over a
/// negative base (whose principal value is a root of unity that is NOT a complex constant) and
/// general non-integer exponents of POSITIVE bases stay unsupported and keep their typed errors —
/// see <see cref="PositiveBaseNonIntegerPower_IsStillLeftUnevaluated"/> and
/// <see cref="NegativeBaseThirdRoot_IsOutsideTheBoundedSet"/>.</para>
/// </summary>
public class ComplexClosedFormTests
{
    private static ExprContext NewContext()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx;
    }

    /// <summary>1e-25, evaluated at 40 digits — far below the 30 correct digits the values are
    /// compared at, so an exact value passes and any decimal approximation fails.</summary>
    private static readonly string TightTolerance = "0." + new string('0', 24) + "1";

    /// <summary>Asserts the expression's numeric value has exactly the given real and imaginary
    /// parts. Comparing the two parts SEPARATELY is deliberate: it also fails when a real value is
    /// returned where a complex one is expected (the imaginary part would be 0), which a
    /// modulus-only comparison would accept.</summary>
    private static void AssertParts(Expr e, ExprContext ctx, string expectedRe, string expectedIm, string label)
    {
        using var precision = Rl.WithPrecision(40, 20);
        Num n = Evaluation.EvaluateToNum(e, ctx, new Dictionary<Symbol, Num>());
        Rl allowed = Rl.Parse(TightTolerance, null);
        Num re = NumOps.Abs(NumOps.Subtract(NumOps.Re(n, ctx), NumOps.FromReal(Rl.Parse(expectedRe, null))), ctx);
        Num im = NumOps.Abs(NumOps.Subtract(NumOps.Im(n, ctx), NumOps.FromReal(Rl.Parse(expectedIm, null))), ctx);
        Assert.True(NumOps.Compare(re, NumOps.FromReal(allowed)) < 0,
            $"{label}: real part {NumOps.ToReal(NumOps.Re(n, ctx))} != {expectedRe} (form {Printing.CanonicalPrint(e)})");
        Assert.True(NumOps.Compare(im, NumOps.FromReal(allowed)) < 0,
            $"{label}: imaginary part {NumOps.ToReal(NumOps.Im(n, ctx))} != {expectedIm} (form {Printing.CanonicalPrint(e)})");
    }

    /// <summary>Canonical print → canonical parse → structural equality. The value the kernel
    /// newly produces has to survive the published wire form, not merely evaluate correctly.</summary>
    private static void AssertRoundTrips(Expr e, ExprContext ctx, string expectedCanonical)
    {
        string canonical = Printing.CanonicalPrint(e);
        Assert.Equal(expectedCanonical, canonical);
        Expr reparsed = Printing.CanonicalParse(canonical, ctx);
        Assert.Equal(canonical, Printing.CanonicalPrint(reparsed));
        Assert.True(e.Equals(reparsed), $"{expectedCanonical} did not round-trip to an equal expression");
    }

    // ------------------------------------------------------------------
    // Target 1: sqrt of an exact negative value
    // ------------------------------------------------------------------

    [Fact]
    public void SqrtOfNegativeOne_IsTheExactImaginaryUnit()
    {
        var ctx = NewContext();
        Expr value = Exprs.Sqrt(Exprs.Integer(-1));

        Assert.Equal(NodeKind.NamedConstant, value.Kind);
        Assert.True(value.IsExact, "the imaginary unit must be EXACT, not an approximation");
        Assert.Equal(Exprs.I, value);
        AssertRoundTrips(value, ctx, "(i)");
        AssertParts(value, ctx, "0", "1", "sqrt(-1)");
    }

    [Fact]
    public void SqrtOfNegativeFour_IsExactlyTwoI()
    {
        var ctx = NewContext();
        Expr value = Exprs.Sqrt(Exprs.Integer(-4));

        Assert.True(value.IsExact);
        AssertRoundTrips(value, ctx, "(mul (rat 2 1) (i))");
        AssertParts(value, ctx, "0", "2", "sqrt(-4)");
    }

    [Fact]
    public void SqrtOfNegativePerfectSquares_AreExactImaginaryConstants()
    {
        var ctx = NewContext();
        Expr nine = Exprs.Sqrt(Exprs.Integer(-9));
        Assert.True(nine.IsExact);
        AssertRoundTrips(nine, ctx, "(mul (rat 3 1) (i))");
        AssertParts(nine, ctx, "0", "3", "sqrt(-9)");

        // sqrt(-1/4) = I/2 in SymPy: a negative RATIONAL base is the same branch
        Expr quarter = Exprs.Sqrt(Exprs.Rational(Rat.From(-1, 4)));
        Assert.True(quarter.IsExact);
        AssertRoundTrips(quarter, ctx, "(mul (rat 1 2) (i))");
        AssertParts(quarter, ctx, "0", "0.5", "sqrt(-1/4)");
    }

    [Fact]
    public void SqrtOfNegativeNonSquare_IsTheExactImaginaryMultipleOfTheRadical()
    {
        var ctx = NewContext();
        // SymPy: (-2)**Rational(1,2) = sqrt(2)*I — exact as a radical, never a decimal
        Expr value = Exprs.Sqrt(Exprs.Integer(-2));
        Expr expected = Exprs.Multiply(
            Exprs.I, Exprs.Power(Exprs.Integer(2), Exprs.Rational(1, 2)));
        Assert.True(value.Equals(expected),
            $"sqrt(-2) built {Printing.CanonicalPrint(value)}, expected {Printing.CanonicalPrint(expected)}");
        AssertRoundTrips(value, ctx, "(mul (i) (pow (rat 2 1) (rat 1 2)))");
        AssertParts(value, ctx, "0",
            "1.4142135623730950488016887242096980785696718753769480731767", "sqrt(-2)");
    }

    // ------------------------------------------------------------------
    // Target 2: negative base raised to an exponent with denominator 2
    // ------------------------------------------------------------------

    [Fact]
    public void NegativeBaseRationalPowers_MatchTheSympyPrincipalBranch()
    {
        var ctx = NewContext();
        Expr half = Exprs.Rational(1, 2);

        Expr minus1 = Exprs.Power(Exprs.Integer(-1), half);
        AssertRoundTrips(minus1, ctx, "(i)");
        AssertParts(minus1, ctx, "0", "1", "(-1)^(1/2)");

        Expr minus4 = Exprs.Power(Exprs.Integer(-4), half);
        AssertRoundTrips(minus4, ctx, "(mul (rat 2 1) (i))");
        AssertParts(minus4, ctx, "0", "2", "(-4)^(1/2)");

        // SymPy: (-3)**Rational(1,2) = sqrt(3)*I
        Expr minus3 = Exprs.Power(Exprs.Integer(-3), half);
        AssertRoundTrips(minus3, ctx, "(mul (i) (pow (rat 3 1) (rat 1 2)))");
        AssertParts(minus3, ctx, "0",
            "1.7320508075688772935274463415058723669428052538103806280558", "(-3)^(1/2)");

        // SymPy: (-4)**Rational(3,2) = -8*I
        Expr threeHalves = Exprs.Power(Exprs.Integer(-4), Exprs.Rational(3, 2));
        AssertRoundTrips(threeHalves, ctx, "(mul (rat -8 1) (i))");
        AssertParts(threeHalves, ctx, "0", "-8", "(-4)^(3/2)");

        // SymPy: (-4)**Rational(-1,2) = -I/2
        Expr minusHalf = Exprs.Power(Exprs.Integer(-4), Exprs.Rational(-1, 2));
        AssertRoundTrips(minusHalf, ctx, "(mul (rat -1 2) (i))");
        AssertParts(minusHalf, ctx, "0", "-0.5", "(-4)^(-1/2)");
    }

    /// <summary>The imaginary unit's own integer powers must fold, or a complex solution can never
    /// substitute back to EXACTLY zero and the differential oracle's solve corpus would reject it.</summary>
    [Fact]
    public void ImaginaryUnitIntegerPowers_FoldExactly()
    {
        var ctx = NewContext();
        AssertRoundTrips(Exprs.Power(Exprs.I, Exprs.Integer(2)), ctx, "(rat -1 1)");
        AssertRoundTrips(Exprs.Power(Exprs.I, Exprs.Integer(3)), ctx, "(mul (rat -1 1) (i))");
        AssertRoundTrips(Exprs.Power(Exprs.I, Exprs.Integer(4)), ctx, "(rat 1 1)");
        AssertRoundTrips(Exprs.Power(Exprs.I, Exprs.Integer(-1)), ctx, "(mul (rat -1 1) (i))");
    }

    // ------------------------------------------------------------------
    // Target 3: quadratic with a negative discriminant
    // ------------------------------------------------------------------

    /// <summary>c2·x^2 + c1·x + c0 == 0.</summary>
    private static SolutionSet SolveQuadratic(ExprContext ctx, Symbol x, long c0, long c1, long c2)
    {
        Expr relation = Exprs.Relation(RelOp.Eq,
            Exprs.Add(
                Exprs.Multiply(c2, Exprs.Power(Exprs.Symbol(x), Exprs.Integer(2))),
                Exprs.Multiply(c1, Exprs.Symbol(x)),
                c0),
            Exprs.Zero);
        return Solvers.Solve(relation, x, ctx);
    }

    /// <summary>Substituting every returned root into the polynomial must expand to EXACTLY zero —
    /// the same acceptance rule the differential oracle applies to its solve corpus.</summary>
    private static void AssertRootsSatisfy(Expr polynomial, Symbol x, SolutionSet set, ExprContext ctx, string label)
    {
        foreach (Solution solution in set.Solutions)
        {
            Expr expanded = Algebra.Expand(
                Evaluation.Substitute(polynomial, ctx, new Dictionary<Symbol, Expr> { [x] = solution.Value }), ctx);
            Assert.True(expanded is RationalConstantExpr zero && zero.Value.IsZero,
                $"{label}: root {Printing.CanonicalPrint(solution.Value)} expands to {Printing.CanonicalPrint(expanded)}, not 0");
        }
    }

    [Fact]
    public void QuadraticWithNegativeDiscriminant_YieldsTheExactComplexSolutionSet()
    {
        var ctx = NewContext();
        Symbol x = ctx.Symbol("x");
        Expr polynomial = Exprs.Add(Exprs.Power(Exprs.Symbol(x), Exprs.Integer(2)), Exprs.One);
        SolutionSet set = SolveQuadratic(ctx, x, 1, 0, 1);   // x^2 + 1 == 0

        Assert.Equal(SolveStatus.Solved, set.Status);
        Assert.Equal(SolveDomain.Complex, set.Domain);
        Assert.Equal(0, set.UnrepresentedCount);
        // SymPy: solve(x**2 + 1, x) = [-I, I]
        Assert.Equal(
            new[] { "(i)", "(mul (rat -1 1) (i))" },
            set.Solutions.Select(s => Printing.CanonicalPrint(s.Value)).OrderBy(t => t, StringComparer.Ordinal).ToArray());
        AssertRootsSatisfy(polynomial, x, set, ctx, "x^2 + 1");
    }

    /// <summary>SymPy: solve(x**2 + 4, x) = [-2*I, 2*I] and solve(x**2 + 2*x + 5, x) =
    /// [-1 - 2*I, -1 + 2*I]. The second has a NON-ZERO real part, so it exercises the
    /// "real part plus an imaginary multiple" shape rather than a bare imaginary constant. Both its
    /// roots are returned as the exact unexpanded quotient <c>(-b ± sqrt(disc)) / 2a</c>, which is
    /// asserted as printed AND by value; the exactness of the shape is proven by
    /// <see cref="AssertRootsSatisfy"/> expanding each root back to zero.</summary>
    [Fact]
    public void QuadraticWithNegativeDiscriminant_CoversANonZeroRealPart()
    {
        var ctx = NewContext();
        Symbol x = ctx.Symbol("x");

        Expr four = Exprs.Add(Exprs.Power(Exprs.Symbol(x), Exprs.Integer(2)), Exprs.Integer(4));
        SolutionSet setFour = SolveQuadratic(ctx, x, 4, 0, 1);
        Assert.Equal(SolveStatus.Solved, setFour.Status);
        Assert.Equal(
            new[] { "(mul (rat -2 1) (i))", "(mul (rat 2 1) (i))" },
            setFour.Solutions.Select(s => Printing.CanonicalPrint(s.Value)).OrderBy(t => t, StringComparer.Ordinal).ToArray());
        AssertRootsSatisfy(four, x, setFour, ctx, "x^2 + 4");

        Expr shifted = Exprs.Add(
            Exprs.Power(Exprs.Symbol(x), Exprs.Integer(2)),
            Exprs.Multiply(2, Exprs.Symbol(x)),
            5);
        SolutionSet setShifted = SolveQuadratic(ctx, x, 5, 2, 1);
        Assert.Equal(SolveStatus.Solved, setShifted.Status);
        Assert.Equal(0, setShifted.UnrepresentedCount);
        Expr[] roots = setShifted.Solutions
            .OrderBy(s => Printing.CanonicalPrint(s.Value), StringComparer.Ordinal)
            .Select(s => s.Value)
            .ToArray();
        Assert.Equal(2, roots.Length);
        Assert.Equal("(mul (rat 1 2) (add (rat -2 1) (mul (rat -4 1) (i))))", Printing.CanonicalPrint(roots[0]));
        Assert.Equal("(mul (rat 1 2) (add (rat -2 1) (mul (rat 4 1) (i))))", Printing.CanonicalPrint(roots[1]));
        AssertParts(roots[0], ctx, "-1", "-2", "the root -1 - 2i of x^2 + 2*x + 5");
        AssertParts(roots[1], ctx, "-1", "2", "the root -1 + 2i of x^2 + 2*x + 5");
        AssertRootsSatisfy(shifted, x, setShifted, ctx, "x^2 + 2*x + 5");
    }

    // ------------------------------------------------------------------
    // The boundary: what this round deliberately did NOT widen
    // ------------------------------------------------------------------

    /// <summary>A positive base under a non-integer exponent is a DIFFERENT and much larger
    /// feature (a real irrational the Real type refuses to approximate); it stays unevaluated in
    /// the kernel and keeps its typed error at the host boundary.</summary>
    [Fact]
    public void PositiveBaseNonIntegerPower_IsStillLeftUnevaluated()
    {
        var ctx = NewContext();
        AssertRoundTrips(
            Exprs.Power(Exprs.Integer(2), Exprs.Rational(1, 2)), ctx, "(pow (rat 2 1) (rat 1 2))");
    }

    /// <summary>The denominator-3 case is OUTSIDE this round's bounded set: the principal value is
    /// a root of unity that is not a complex constant. This pins the CURRENT kernel behaviour so a
    /// later round changing it must change this test deliberately.</summary>
    [Fact]
    public void NegativeBaseThirdRoot_IsOutsideTheBoundedSet()
    {
        var ctx = NewContext();
        // the kernel's real-root shortcut still fires for an odd denominator
        AssertRoundTrips(Exprs.Power(Exprs.Integer(-8), Exprs.Rational(1, 3)), ctx, "(rat -2 1)");
        // and a non-unit denominator-3 exponent is still left alone entirely
        AssertRoundTrips(Exprs.Power(Exprs.Integer(-8), Exprs.Rational(2, 3)), ctx, "(pow (rat -8 1) (rat 2 3))");
    }

    [Fact]
    public void SqrtOfANonNegativeValue_IsUnchanged()
    {
        var ctx = NewContext();
        AssertRoundTrips(Exprs.Sqrt(Exprs.Integer(4)), ctx, "(rat 2 1)");
        AssertRoundTrips(Exprs.Sqrt(Exprs.Integer(9)), ctx, "(rat 3 1)");
        AssertRoundTrips(Exprs.Sqrt(Exprs.Integer(2)), ctx, "(pow (rat 2 1) (rat 1 2))");
        Assert.Equal(NodeKind.RationalConstant, Exprs.Sqrt(Exprs.Integer(0)).Kind);
    }
}
