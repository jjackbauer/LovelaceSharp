using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Lovelace.Symbolics.Rewriting;
using Xunit;
using Int = Lovelace.Integer.Integer;
using Rat = Lovelace.Rational.Rational;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Kernel-hardening regression corpus: one test per semantic bug fixed in the hardening cycle,
/// plus the release gates from the alignment plan (canonicalization definedness, assumption
/// lattice, branch cuts, precision integrity, RootOf isolation, limits, arity).
/// </summary>
public class KernelHardeningTests
{
    private static ExprContext NewCtx()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx;
    }

    // ------------------------------------------------------------------
    // Canonicalization / definedness
    // ------------------------------------------------------------------

    [Fact]
    public void PowerOfSymbolicExponent_PreservesExponent()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var e = Exprs.Power(Exprs.Power(x, y), 2);
        // must stay (x^y)^2 — the old constructor silently folded to x^2, losing y
        var p = Assert.IsType<PowerExpr>(e);
        var inner = Assert.IsType<PowerExpr>(p.Base);
        Assert.Equal(y, ((SymbolExpr)inner.Exponent).Symbol);
        Assert.Equal(Rat.FromLong(2), ((RationalConstantExpr)p.Exponent).Value);
        Assert.Equal("(x^y)^2", Printing.PrettyPrint(e));
    }

    [Fact]
    public void XOverX_IsNotFoldedByConstruction()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        Assert.Equal("x/x", Printing.PrettyPrint(Exprs.Divide(x, x)));
        Assert.Equal("0/x", Printing.PrettyPrint(Exprs.Divide(Exprs.Zero, x)));
    }

    [Fact]
    public void Simplify_XOverX_CancelsWithCondition()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var result = Simplify.Transform(Exprs.Divide(x, x), ctx, new Simplify.Options(Trace: true));
        Assert.Equal(Exprs.One, result.Expression);
        Assert.Contains(result.Conditions.Atoms, a =>
            a is SymbolPropertyAssumption { S: var s, P: SymbolPredicate.NonZero } sp && sp.S.Name == "x");
        Assert.NotEmpty(result.Steps);
        Assert.All(result.Steps, s => Assert.Equal(RuleClassification.Conditional, s.Classification));
    }

    [Fact]
    public void Simplify_ZeroOverX_CancelsWithCondition()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var result = Simplify.Transform(Exprs.Divide(Exprs.Zero, x), ctx);
        Assert.Equal(Exprs.Zero, result.Expression);
        Assert.Contains(result.Conditions.Atoms, a =>
            a is SymbolPropertyAssumption { S: var s, P: SymbolPredicate.NonZero } sp && sp.S.Name == "x");
    }

    [Fact]
    public void AddDroppingZeroCoefficientOverPole_KeepsDefinedness()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var e = Exprs.Add(x, Exprs.Divide(Exprs.Zero, x));
        // x + 0/x is undefined at 0; dropping the term would define it
        Assert.Contains("0/x", Printing.PrettyPrint(e));
    }

    [Fact]
    public void ZeroPowZero_IsOne_DocumentedConvention()
    {
        NewCtx();
        Assert.Equal(Exprs.One, Exprs.Power(Exprs.Zero, Exprs.Zero));
    }

    // ------------------------------------------------------------------
    // Assumption lattice
    // ------------------------------------------------------------------

    [Fact]
    public void Integer_DoesNotImply_EvenOddNonZero()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var assumptions = AssumptionSet.Empty.Add(new SymbolDomainAssumption(x, Domain.Integer));
        using (ctx.WithAssumptions(assumptions))
        {
            Assert.Equal(Tristate.Unknown, assumptions.Ask(new SymbolPropertyAssumption(x, SymbolPredicate.Even)));
            Assert.Equal(Tristate.Unknown, assumptions.Ask(new SymbolPropertyAssumption(x, SymbolPredicate.Odd)));
            Assert.Equal(Tristate.Unknown, assumptions.Ask(new SymbolPropertyAssumption(x, SymbolPredicate.NonZero)));
        }
    }

    [Fact]
    public void EvenOdd_ImplyInteger_OddImpliesNonZero()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var even = AssumptionSet.Empty.Add(new SymbolPropertyAssumption(x, SymbolPredicate.Even));
        var odd = AssumptionSet.Empty.Add(new SymbolPropertyAssumption(x, SymbolPredicate.Odd));
        Assert.Equal(Tristate.True, even.Ask(new SymbolDomainAssumption(x, Domain.Integer)));
        Assert.Equal(Tristate.True, odd.Ask(new SymbolDomainAssumption(x, Domain.Integer)));
        Assert.Equal(Tristate.True, odd.Ask(new SymbolPropertyAssumption(x, SymbolPredicate.NonZero)));
        Assert.Equal(Tristate.Unknown, even.Ask(new SymbolPropertyAssumption(x, SymbolPredicate.NonZero)));
    }

    [Fact]
    public void Or_ReturnsFalse_WhenAllDisjunctsFalse()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        // NonPositive assumed => Positive is False; x < 0 assumed => Ge 0 is False;
        // every disjunct of NonNegative is provably false, so NonNegative is False
        var a = AssumptionSet.Empty
            .Add(new SymbolPropertyAssumption(x, SymbolPredicate.NonPositive))
            .Add(new SymbolRelationAssumption(x, RelOp.Lt, Exprs.Zero));
        Assert.Equal(Tristate.False, a.Ask(new SymbolPropertyAssumption(x, SymbolPredicate.NonNegative)));
    }

    // ------------------------------------------------------------------
    // Branch cuts / domain-aware rules
    // ------------------------------------------------------------------

    [Fact]
    public void LogExp_NotSimplified_Unconstrained()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var e = Exprs.Function(ctx.Function("log"), Exprs.Function(ctx.Function("exp"), x));
        Assert.Equal(e, Simplify.SimplifyExpr(e, ctx));
    }

    [Fact]
    public void LogExp_Simplified_UnderReal()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var e = Exprs.Function(ctx.Function("log"), Exprs.Function(ctx.Function("exp"), x));
        using (ctx.WithAssumptions(AssumptionSet.Empty.Add(new SymbolDomainAssumption(x, Domain.Real))))
        {
            Assert.Equal(x, Simplify.SimplifyExpr(e, ctx));
        }
    }

    [Fact]
    public void AbsSquare_NotSimplified_ForComplex()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var e = Exprs.Power(Exprs.Function(ctx.Function("abs"), x), 2);
        Assert.Equal(e, Simplify.SimplifyExpr(e, ctx));
    }

    [Fact]
    public void AbsSquare_Simplified_UnderReal()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var e = Exprs.Power(Exprs.Function(ctx.Function("abs"), x), 2);
        using (ctx.WithAssumptions(AssumptionSet.Empty.Add(new SymbolDomainAssumption(x, Domain.Real))))
        {
            Assert.Equal(Exprs.Power(x, 2), Simplify.SimplifyExpr(e, ctx));
        }
    }

    [Fact]
    public void ExpLog_Simplifies_Universally()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var e = Exprs.Function(ctx.Function("exp"), Exprs.Function(ctx.Function("log"), x));
        Assert.Equal(x, Simplify.SimplifyExpr(e, ctx));
    }

    [Fact]
    public void DomainPredicates_AreRealGuarded()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var a = AssumptionSet.Empty;   // unconstrained: x ranges over C
        // exp(x) positive is NOT provable for complex x
        Assert.Equal(Tristate.Unknown, a.Ask(new ExpressionPropertyAssumption(Exprs.Function(ctx.Function("exp"), x), SymbolPredicate.Positive)));
        // sqrt(x) nonnegative is NOT provable without real nonnegativity
        Assert.Equal(Tristate.Unknown, a.Ask(new ExpressionPropertyAssumption(Exprs.Sqrt(x), SymbolPredicate.NonNegative)));
        // x^2 nonnegative is NOT provable for complex x
        Assert.Equal(Tristate.Unknown, a.Ask(new ExpressionPropertyAssumption(Exprs.Power(x, 2), SymbolPredicate.NonNegative)));
        // under real assumptions the same queries succeed
        var real = AssumptionSet.Empty.Add(new SymbolDomainAssumption(x, Domain.Real));
        Assert.Equal(Tristate.True, real.Ask(new ExpressionPropertyAssumption(Exprs.Function(ctx.Function("exp"), x), SymbolPredicate.Positive)));
        Assert.Equal(Tristate.True, real.Ask(new ExpressionPropertyAssumption(Exprs.Power(x, 2), SymbolPredicate.NonNegative)));
        // but sqrt(x) still needs x >= 0
        Assert.Equal(Tristate.Unknown, real.Ask(new ExpressionPropertyAssumption(Exprs.Sqrt(x), SymbolPredicate.NonNegative)));
        var nonneg = real.Add(new SymbolPropertyAssumption(x, SymbolPredicate.NonNegative));
        Assert.Equal(Tristate.True, nonneg.Ask(new ExpressionPropertyAssumption(Exprs.Sqrt(x), SymbolPredicate.NonNegative)));
    }

    [Fact]
    public void IntervalCondition_ImZero_ForProvablyReal()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var im = Exprs.Function(ctx.Function("im"), x);
        var strip = new IntervalAssumption(im, Exprs.Negate(Exprs.Pi), true, Exprs.Pi, false);
        Assert.Equal(Tristate.Unknown, AssumptionSet.Empty.Ask(strip));
        var real = AssumptionSet.Empty.Add(new SymbolDomainAssumption(x, Domain.Real));
        Assert.Equal(Tristate.True, real.Ask(strip));
    }

    // ------------------------------------------------------------------
    // Function arity
    // ------------------------------------------------------------------

    [Fact]
    public void RegisteredFunctionArity_IsEnforced()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        Assert.Throws<ArgumentException>(() => Exprs.Function(ctx.Function("sin"), x, y));
        Assert.Throws<ArgumentException>(() => Exprs.Function(ctx.Function("log")));
        // variadic min/max accept any count >= 2
        Assert.NotNull(Exprs.Function(ctx.Function("min"), x, y, Exprs.One));
        // unknown names remain open for extensibility
        Assert.NotNull(Exprs.Function(ctx.Function("myfuturefn"), x, y));
    }

    // ------------------------------------------------------------------
    // Square-free / RootOf / solving
    // ------------------------------------------------------------------

    [Fact]
    public void YunSquareFree_MultiplicitiesAreCorrect()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        // x^2 (x - 1) = x^3 - x^2
        var p = Polynomial.FromExpr(Exprs.Subtract(Exprs.Power(x, 3), Exprs.Power(x, 2)), ctx, new[] { x });
        var factors = Polynomial.SquareFreeUnivariate(p)
            .OrderBy(f => f.Factor.TotalDegree).ToArray();
        Assert.Equal(2, factors.Length);
        Assert.Equal(1, factors[0].Multiplicity);   // (x-1) once
        Assert.Equal(2, factors[1].Multiplicity);   // x twice
    }

    [Fact]
    public void RationalRoots_IncludeZero()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        // x*(x-1/1000)*(x-1) has roots 0, 1/1000, 1
        var p = Polynomial.FromExpr(
            Exprs.Multiply(x, Exprs.Subtract(x, Exprs.Rational(1, 1000)), Exprs.Subtract(x, Exprs.One)),
            ctx, new[] { x });
        var roots = p.RationalRoots();
        Assert.Contains(Rat.Zero, roots);
        Assert.Contains(Rat.From(1, 1000), roots);
        Assert.Contains(Rat.One, roots);
    }

    [Fact]
    public void RootOf_CloseRoots_AllFoundAndIndexedStably()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var p = Polynomial.FromExpr(
            Exprs.Multiply(x, Exprs.Subtract(x, Exprs.Rational(1, 1000)), Exprs.Subtract(x, Exprs.One)),
            ctx, new[] { x });
        Assert.Equal(3, Roots.RealRootCount(p));
        // each index isolates the right root, in ascending order
        using var scope = Rl.WithPrecision(40, 20);
        var r0 = Roots.N(Exprs.RootOf(p, 0), 30, ctx);
        var r1 = Roots.N(Exprs.RootOf(p, 1), 30, ctx);
        var r2 = Roots.N(Exprs.RootOf(p, 2), 30, ctx);
        AssertClose(Rl.Parse("0", null), r0);
        AssertClose(Rl.Parse("0.001", null), r1);
        AssertClose(Rl.Parse("1", null), r2);
    }

    private static void AssertClose(Rl expected, Rl actual)
    {
        var eps = Rl.Parse("0." + new string('0', 25) + "1", null);
        Assert.True(actual > expected - eps && actual < expected + eps,
            $"expected {expected}, got {actual}");
    }

    [Fact]
    public void RootOf_MultipleRoot_IsSquareFreeNormalized()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        // (x-1)^2: square-free part has exactly one real root
        var p = Polynomial.FromExpr(Algebra.Expand(Exprs.Power(Exprs.Subtract(x, Exprs.One), 2)), ctx, new[] { x });
        var ro = (RootOfExpr)Exprs.RootOf(p, 0);
        Assert.Equal(1, Roots.RealRootCount(ro.DefiningPolynomial));
    }

    [Fact]
    public void RootOf_NoRealRoots_FailsExplicitly()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var p = Polynomial.FromExpr(Exprs.Add(Exprs.Power(x, 2), Exprs.One), ctx, new[] { x });
        Assert.Throws<InvalidOperationException>(() => Roots.N(Exprs.RootOf(p, 0), 20, ctx));
    }

    [Fact]
    public void Solve_NoRealRoots_ReportsEmptyNotInvented()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var set = Solvers.Solve(Exprs.Relation(RelOp.Eq, Exprs.Add(Exprs.Power(x, 4), Exprs.One), Exprs.Zero), x, ctx);
        Assert.Equal(SolutionKind.Empty, set.Kind);
    }

    [Theory]
    [InlineData("0", "0", "-1")]                   // x^3 - 1: Cardano roots share a cube-root base, Expand collapses
    [InlineData("0", "-1", "0")]                   // x^3 - x: factors to linear + quadratic
    public void CubicRoots_SatisfyPolynomial_Symbolically(string a, string b, string c)
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var poly = Exprs.Add(
            Exprs.Power(x, 3),
            Exprs.Multiply(Exprs.Rational(Rat.Parse(a, null)), Exprs.Power(x, 2)),
            Exprs.Multiply(Exprs.Rational(Rat.Parse(b, null)), x),
            Exprs.Rational(Rat.Parse(c, null)));
        var set = Solvers.Solve(Exprs.Relation(RelOp.Eq, poly, Exprs.Zero), x, ctx);
        Assert.Equal(SolutionKind.Exact, set.Kind);
        Assert.Equal(3, set.Solutions.Count);
        foreach (var sol in set.Solutions)
        {
            var at = Evaluation.Substitute(poly, ctx, new Dictionary<Symbol, Expr> { [x] = sol.Value });
            var expanded = Algebra.Expand(at, ctx);
            Assert.True(expanded is RationalConstantExpr rc && rc.Value.IsZero,
                $"root {Printing.PrettyPrint(sol.Value)} does not satisfy the cubic");
        }
    }

    [Theory]
    [InlineData("2", "3", "4")]                    // x^3 + 2x^2 + 3x + 4: Cardano, distinct cube-root bases
    [InlineData("0", "0", "1")]                     // x^3 + 1
    [InlineData("0", "-3", "1")]                    // x^3 - 3x + 1: three real roots (trigonometric branch)
    public void CubicRoots_SatisfyPolynomial_Numerically(string a, string b, string c)
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var poly = Exprs.Add(
            Exprs.Power(x, 3),
            Exprs.Multiply(Exprs.Rational(Rat.Parse(a, null)), Exprs.Power(x, 2)),
            Exprs.Multiply(Exprs.Rational(Rat.Parse(b, null)), x),
            Exprs.Rational(Rat.Parse(c, null)));
        var set = Solvers.Solve(Exprs.Relation(RelOp.Eq, poly, Exprs.Zero), x, ctx);
        Assert.Equal(SolutionKind.Exact, set.Kind);
        Assert.Equal(3, set.Solutions.Count);
        // Cardano's mixed cube-root terms need the u·v = −P/3 coupling, which Expand cannot
        // derive — verify by high-precision numeric residual instead
        using var scope = Rl.WithPrecision(60, 30);
        foreach (var sol in set.Solutions)
        {
            var at = Evaluation.Substitute(poly, ctx, new Dictionary<Symbol, Expr> { [x] = sol.Value });
            var residual = Evaluation.EvaluateToNum(at, ctx, new Dictionary<Symbol, Num>());
            var tolerance = NumOps.FromReal(Rl.Parse("0." + new string('0', 39) + "1", null));
            Assert.True(NumOps.Compare(NumOps.Abs(residual, ctx), tolerance) < 0,
                $"root {Printing.PrettyPrint(sol.Value)} has nonzero residual");
        }
    }

    [Fact]
    public void Solve_RationalEquation_HonorsExcludedDenominator()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        engine.Evaluate("x = symbol(\"x\")");
        var result = engine.Evaluate("solve((x^2 - 1)/(x - 1) == 0, x)");
        // x = 1 must be excluded: the pole of the cancelled denominator is not a solution
        Assert.Equal("[-1] (Vector)", ValueFormatter.FormatTyped(result));
    }

    // ------------------------------------------------------------------
    // Limits
    // ------------------------------------------------------------------

    [Fact]
    public void Limit_OneOverX_LeftMinusInf()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var r = Limits.Limit(Exprs.Divide(Exprs.One, x), x, Exprs.Zero, LimitDirection.FromLeft, ctx);
        Assert.Equal(LimitStatus.MinusInfinity, r.Status);
    }

    [Fact]
    public void Limit_OneOverX_RightPlusInf()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var r = Limits.Limit(Exprs.Divide(Exprs.One, x), x, Exprs.Zero, LimitDirection.FromRight, ctx);
        Assert.Equal(LimitStatus.PlusInfinity, r.Status);
    }

    [Fact]
    public void Limit_OneOverX_TwoSidedDoesNotExist()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var r = Limits.Limit(Exprs.Divide(Exprs.One, x), x, Exprs.Zero, LimitDirection.TwoSided, ctx);
        Assert.Equal(LimitStatus.DoesNotExist, r.Status);
        Assert.Equal(LimitStatus.MinusInfinity, r.FromLeft!.Status);
        Assert.Equal(LimitStatus.PlusInfinity, r.FromRight!.Status);
    }

    [Fact]
    public void Limit_OneOverXSquared_BothSidesPlusInf()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var left = Limits.Limit(Exprs.Divide(Exprs.One, Exprs.Power(x, 2)), x, Exprs.Zero, LimitDirection.FromLeft, ctx);
        var right = Limits.Limit(Exprs.Divide(Exprs.One, Exprs.Power(x, 2)), x, Exprs.Zero, LimitDirection.FromRight, ctx);
        Assert.Equal(LimitStatus.PlusInfinity, left.Status);
        Assert.Equal(LimitStatus.PlusInfinity, right.Status);
    }

    [Fact]
    public void Limit_SeriesOffset_OneMinusCosOverX_IsZero()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var f = Exprs.Divide(Exprs.Subtract(Exprs.One, Exprs.Function(ctx.Function("cos"), x)), x);
        var r = Limits.Limit(f, x, Exprs.Zero, LimitDirection.TwoSided, ctx);
        Assert.Equal(LimitStatus.Value, r.Status);
        Assert.Equal(Exprs.Zero, r.Value);
    }

    // ------------------------------------------------------------------
    // Precision integrity
    // ------------------------------------------------------------------

    [Fact]
    public void RealConstantAdd_Preserves500Digits()
    {
        NewCtx();
        var a = RealLiteral.Parse("0." + new string('1', 500));
        var b = RealLiteral.Parse("0." + new string('2', 500));
        var sum = (RealConstantExpr)Exprs.Add(Exprs.Real(a), Exprs.Real(b));
        // exact digit-wise sum, never a 64-digit truncation
        Assert.Equal("0." + new string('3', 500), sum.Value.ToString());
    }

    [Fact]
    public void RealConstantMultiply_PreservesPrecision()
    {
        var ctx = NewCtx();
        var a = RealLiteral.Parse("0." + new string('7', 500));
        var p = Exprs.Multiply(Exprs.Real(a), Exprs.Integer(3));
        // 3 * 0.777...7 (500 digits): full precision preserved, no 64-digit collapse
        var expected = Rat.Parse("0." + new string('7', 500), null) * Rat.FromLong(3);
        using var scope = Rl.WithPrecision(600, 50);
        var value = Evaluation.EvaluateToNum(p, ctx, new Dictionary<Symbol, Num>());
        Assert.Equal(expected, RealLiteral.FromRealExact(((NumReal)value).V).ToRational());
    }

    [Fact]
    public void HugeExponent_CompactForm_RoundTrips()
    {
        NewCtx();
        var lit = new RealLiteral(new Int(15), 1_000_000_000L);
        Assert.Equal("15e1000000000", lit.ToString());
        Assert.Equal(lit, RealLiteral.Parse(lit.ToString()));
    }

    [Fact]
    public void RationalPower_HugeExponent_IsExact()
    {
        NewCtx();
        // 2^65536 — well beyond int range on the old saturating path, exact here
        var e = Exprs.Power(Exprs.Rational(2L), Exprs.Rational(Rat.From(new Int(65536))));
        var rc = Assert.IsType<RationalConstantExpr>(e);
        Assert.Equal(new Int(2L).Pow(new Int(65536)), rc.Value.Numerator);
        // negative huge exponent: exact reciprocal
        var inv = Exprs.Power(Exprs.Rational(2L), Exprs.Rational(Rat.From(new Int(-65536))));
        var ric = Assert.IsType<RationalConstantExpr>(inv);
        Assert.Equal(new Int(2L).Pow(new Int(65536)), ric.Value.Denominator);
    }

    [Fact]
    public void MinusOne_HugeExponent_ParityIsExact()
    {
        NewCtx();
        var two40 = new Int(2L).Pow(new Int(40));
        var even = Exprs.Power(Exprs.Rational(-1L), Exprs.Rational(Rat.From(two40)));
        var odd = Exprs.Power(Exprs.Rational(-1L), Exprs.Rational(Rat.From(two40 + Int.One)));
        Assert.Equal(Exprs.One, even);
        Assert.Equal(Exprs.MinusOne, odd);
    }

    [Fact]
    public void NumericToSymbolicToNumeric_500Digits_RoundTrips()
    {
        NewCtx();
        var digits = new string('3', 500);
        using var scope = Rl.WithPrecision(600, 50);
        var r = Rl.Parse("0." + digits, null);
        var lit = RealLiteral.FromRealExact(r);
        Assert.Equal("0." + digits, lit.ToString());
        // symbolic → numeric: exact rational identity (never display-scoped truncation)
        Assert.Equal(Rat.Parse("0." + digits, null), lit.ToRational());
    }

    // ------------------------------------------------------------------
    // Calculus edge semantics
    // ------------------------------------------------------------------

    [Fact]
    public void Diff_Floor_IsUnevaluatedDerivative_NotZero()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var d = Calculus.Diff(Exprs.Function(ctx.Function("floor"), x), x, ctx);
        var de = Assert.IsType<DerivativeExpr>(Exprs.Divide(d, Exprs.One));  // strip the *1 from chain rule
        _ = de;
        // d floor(x)/dx must never be stated as a smooth 0
        Assert.NotEqual(Exprs.Zero, d);
    }

    [Fact]
    public void Diff_Abs_CarriesNonzeroCondition()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var d = Calculus.Diff(Exprs.Function(ctx.Function("abs"), x), x, ctx);
        var pw = Assert.IsType<PiecewiseExpr>(d);
        Assert.Single(pw.Branches);
        Assert.Equal(RelOp.Ne, ((RelationExpr)pw.Branches[0].Guard).Op);
    }

    [Fact]
    public void Diff_Piecewise_DifferentiatesBranches()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var pw = Exprs.Piecewise(
            new[] { new PiecewiseBranch(Exprs.Relation(RelOp.Gt, x, Exprs.Zero), Exprs.Power(x, 2)) },
            Exprs.Negate(Exprs.Power(x, 2)));
        var d = Calculus.Diff(pw, x, ctx);
        var dpw = Assert.IsType<PiecewiseExpr>(d);
        Assert.Equal(Exprs.Multiply(2, x), dpw.Branches[0].Value);
        Assert.Equal(Exprs.Multiply(-2, x), dpw.Otherwise);
    }

    [Fact]
    public void FreeOf_DoesNotTreatPiecewiseAsConstant()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var pw = Exprs.Piecewise(
            new[] { new PiecewiseBranch(Exprs.Relation(RelOp.Gt, x, Exprs.Zero), x) },
            y);
        Assert.False(Calculus.FreeOf(pw, x));
        Assert.True(Calculus.FreeOf(Exprs.Power(y, 2), x));
    }

    // ------------------------------------------------------------------
    // MathIR equivalence for the new canonical forms
    // ------------------------------------------------------------------

    [Fact]
    public void MathIR_HugeExponent_LowersAndEvaluates()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var e = Exprs.Power(x, Exprs.Rational(Rat.From(new Int(65536))));
        var prog = Lowering.Lower(e, ctx, new[] { x });
        using var scope = Rl.WithPrecision(40, 15);
        var value = IrEvaluator.Evaluate(prog, new Dictionary<Symbol, Num> { [x] = NumOps.FromLong(2) }, ctx);
        Assert.Equal(new Int(2L).Pow(new Int(65536)), ((NumInt)value).V);
    }

    [Fact]
    public void MathIR_Deserialize_RejectsOldVersion()
    {
        Assert.Throws<FormatException>(() => IrProgram.Deserialize("#!mathir 1\nparam x\n"));
    }
}
