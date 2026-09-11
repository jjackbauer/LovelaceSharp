using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;
using Rat = Lovelace.Rational.Rational;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// The kernel's exactness claim, stated once and checked three ways.
///
/// <para>THE RULE. <see cref="Expr.IsExact"/> is true EXACTLY when no leaf of the expression is
/// outside the exact basis. The exact basis is the rational/integer/complex constant, the symbol,
/// the imaginary unit, and the algebraic number (RootOf). Outside it are exactly two carriers:
/// the <see cref="RealConstantExpr"/> leaf, which is where a truncated decimal approximation
/// lives, and the transcendental/indeterminate named constants Pi, E and Infinity. A composite
/// node is exact exactly when its children are: for an application that means the FUNCTION'S OWN
/// IDENTITY IS NOT CONSULTED. sin(x), exp(x), log(x), diff(sin(x), x) and every other elementary
/// closed form over exact arguments are exact; there is no per-function whitelist.</para>
///
/// <para>The three checks are independent of each other. (1) <c>ExactByLeafRule</c> is a second,
/// structural implementation of the rule, asserted against the kernel's cached flag over a corpus.
/// (2) The language surface is what an agent reads: <c>inspect(...).exact</c> and a solution's
/// <c>value.exact</c>, read from the published records. (3) The negative half is checked
/// explicitly, because a widened flag that also turns an approximation exact would be worse than
/// the bug it repaired.</para>
///
/// <para>THE AGREEMENT RULE. For every published <see cref="Solution"/>,
/// <c>value.IsExact == (exactness != SolutionExactness.Approximate)</c>. Both fields are read off
/// the SAME published record; <c>Approximate</c> is the only member that claims an approximation,
/// so it is the only member whose value may be inexact.</para>
/// </summary>
public class ExactnessFlagTests
{
    private static ExprContext NewContext()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx;
    }

    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    private static Value Field(RecordValue record, string name) =>
        (Value)record.Fields.First(f => f.Name == name).Value!;

    /// <summary>A RealConstant leaf carrying a VALUE that happens to be exactly rational. It is
    /// still an inexact leaf: the node kind is the carrier, and the flag cannot depend on how many
    /// digits the literal happens to carry. The numeric mirror says the same thing —
    /// <c>NumReal.IsExact</c> is false for a NumReal holding exactly 3/2.</summary>
    private static Expr RealLeaf(long num, long den) =>
        Exprs.Real(RealLiteral.FromRationalExact(Rat.From(num, den)));

    /// <summary>x^5 - x - 1, the classic quintic with no radical solution.</summary>
    private static Polynomial Quintic(ExprContext ctx, Symbol x) =>
        Polynomial.FromExpr(Exprs.Subtract(Exprs.Subtract(Exprs.Power(x, 5), x), 1), ctx, new[] { x });

    /// <summary>The rule under test, implemented a second time and independently of the kernel's
    /// cached flag: an expression is exact exactly when every leaf of it is in the exact basis.</summary>
    private static bool ExactByLeafRule(Expr e) => e switch
    {
        RealConstantExpr => false,                                             // the approximation carrier
        NamedConstantExpr n => n.Constant == NamedConstant.I,                  // Pi/E/Infinity are not exact
        AddExpr a => a.Terms.All(ExactByLeafRule),
        MultiplyExpr m => m.Factors.All(ExactByLeafRule),
        PowerExpr p => ExactByLeafRule(p.Base) && ExactByLeafRule(p.Exponent),
        FunctionExpr f => f.Arguments.All(ExactByLeafRule),
        RelationExpr r => ExactByLeafRule(r.Left) && ExactByLeafRule(r.Right),
        AndExpr an => an.Operands.All(ExactByLeafRule),
        OrExpr or => or.Operands.All(ExactByLeafRule),
        NotExpr nt => ExactByLeafRule(nt.Operand),
        PiecewiseExpr pw => pw.Branches.All(b => ExactByLeafRule(b.Guard) && ExactByLeafRule(b.Value))
                            && ExactByLeafRule(pw.Otherwise),
        DerivativeExpr d => ExactByLeafRule(d.Operand),
        IntegralExpr i => ExactByLeafRule(i.Operand),
        OrderExpr od => ExactByLeafRule(od.Variable) && ExactByLeafRule(od.Point) && ExactByLeafRule(od.Degree),
        RootOfExpr => true,                                                    // an algebraic number is exact
        _ => true,                                                             // rational/complex/symbol leaves
    };

    // ------------------------------------------------------------------
    // 1. the rule, over a corpus, against an independent implementation
    // ------------------------------------------------------------------

    /// <summary>Every corpus member's cached flag must equal the leaf rule. The corpus mixes the
    /// previously-whitelisted functions, the previously-excluded ones, radicals, RootOf, and
    /// approximations buried at every depth.</summary>
    [Fact]
    public void CachedFlag_EqualsTheLeafRule_OverTheWholeCorpus()
    {
        var ctx = NewContext();
        // the approximate-fold path inside Exprs.Function numerically evaluates a Real argument,
        // so it needs the working precision every other numeric test establishes
        using var precision = Lovelace.Real.Real.WithPrecision(40, 20);
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var corpus = new List<Expr>
        {
            Exprs.Integer(1),
            Exprs.Rational(2, 3),
            x,
            Exprs.I,
            Exprs.Complex(Rat.From(1, 2), Rat.From(-3, 4)),
            Exprs.RootOf(Quintic(ctx, x), 0),
            Exprs.Add(x, 1),
            Exprs.Multiply(2, x),
            Exprs.Power(x, 2),
            Exprs.Power(x, Exprs.Rational(1, 2)),
            Exprs.Power(x, Exprs.Rational(3, 2)),
            Exprs.Power(Exprs.Integer(2), Exprs.Rational(1, 2)),
            Exprs.Power(Exprs.Integer(2), Exprs.Rational(1, 3)),
            Exprs.Multiply(Exprs.I, Exprs.Power(Exprs.Integer(3), Exprs.Rational(1, 2))),
            Exprs.Function(ctx.Function("abs"), x),
            Exprs.Function(ctx.Function("sin"), x),
            Exprs.Function(ctx.Function("cos"), x),
            Exprs.Function(ctx.Function("exp"), x),
            Exprs.Function(ctx.Function("log"), x),
            Exprs.Function(ctx.Function("tan"), x),
            Exprs.Function(ctx.Function("atan"), x),
            Exprs.Function(ctx.Function("sinh"), x),
            Exprs.Function(ctx.Function("min"), x, y),
            Exprs.Relation(RelOp.Lt, x, Exprs.Function(ctx.Function("exp"), y)),
            Exprs.Derivative(Exprs.Function(ctx.Function("sin"), x), x),
            // and now the same shapes with an approximation in them
            RealLeaf(3, 2),
            Exprs.Add(x, RealLeaf(3, 2)),
            Exprs.Multiply(x, RealLeaf(3, 2)),
            Exprs.Power(RealLeaf(3, 2), 2),
            Exprs.Power(x, Exprs.Add(x, RealLeaf(3, 2))),
            Exprs.Power(x, Exprs.Symbol("pi")),
            Exprs.Function(ctx.Function("sin"), RealLeaf(3, 2)),
            Exprs.Function(ctx.Function("exp"), RealLeaf(3, 2)),
            Exprs.Function(ctx.Function("min"), x, RealLeaf(3, 2)),
            Exprs.Relation(RelOp.Lt, x, RealLeaf(3, 2)),
            Exprs.Derivative(Exprs.Function(ctx.Function("sin"), RealLeaf(3, 2)), x),
        };

        foreach (var e in corpus)
        {
            Assert.Equal(ExactByLeafRule(e), e.IsExact);
            // and the second direction, so a rule that is uniformly true cannot pass
            if (!ExactByLeafRule(e))
                Assert.False(e.IsExact, Printing.CanonicalPrint(e) + " carries an approximation leaf");
        }

        // the approximation cases are present, so the check above is not vacuous
        Assert.Contains(corpus, e => !ExactByLeafRule(e));
        Assert.Contains(corpus, e => ExactByLeafRule(e));
    }

    // ------------------------------------------------------------------
    // 2. function applications of exact arguments are exact
    // ------------------------------------------------------------------

    /// <summary>Not six names: every elementary function. The first six are the old whitelist and
    /// were already true; the rest are the defect.</summary>
    [Theory]
    [InlineData("abs", 1)]
    [InlineData("sign", 1)]
    [InlineData("floor", 1)]
    [InlineData("ceil", 1)]
    [InlineData("min", 2)]
    [InlineData("max", 2)]
    [InlineData("sin", 1)]
    [InlineData("cos", 1)]
    [InlineData("tan", 1)]
    [InlineData("asin", 1)]
    [InlineData("acos", 1)]
    [InlineData("atan", 1)]
    [InlineData("exp", 1)]
    [InlineData("log", 1)]
    [InlineData("sinh", 1)]
    [InlineData("cosh", 1)]
    [InlineData("tanh", 1)]
    [InlineData("asinh", 1)]
    [InlineData("acosh", 1)]
    [InlineData("atanh", 1)]
    public void ElementaryFunctionOfAnExactArgument_IsExact(string name, int arity)
    {
        var ctx = NewContext();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var args = arity == 1 ? new Expr[] { x } : new Expr[] { x, y };
        var e = Exprs.Function(ctx.Function(name), args);
        Assert.Equal(NodeKind.Function, e.Kind);
        Assert.True(e.IsExact, name + "(...) over exact arguments must be exact");
    }

    /// <summary>The widened rule must not reach through an approximation: a function application is
    /// inexact exactly when one of its arguments is.</summary>
    [Theory]
    [InlineData("sin")]
    [InlineData("cos")]
    [InlineData("exp")]
    [InlineData("log")]
    [InlineData("abs")]
    [InlineData("ceil")]
    public void ElementaryFunctionOfAnApproximateArgument_IsInexact(string name)
    {
        var ctx = NewContext();
        using var precision = Lovelace.Real.Real.WithPrecision(40, 20);
        var e = Exprs.Function(ctx.Function(name), RealLeaf(3, 2));
        Assert.False(e.IsExact, name + "(approximate) must stay inexact");
    }

    // ------------------------------------------------------------------
    // 3. radicals and RootOf
    // ------------------------------------------------------------------

    [Fact]
    public void RadicalsOfExactBases_AreExact_AndApproximateBasesAreNot()
    {
        var ctx = NewContext();
        using var precision = Lovelace.Real.Real.WithPrecision(40, 20);
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");

        // sqrt(2): the exact closed form the old rule reported as an approximation
        Assert.True(Exprs.Sqrt(Exprs.Integer(2)).IsExact);
        Assert.True(Exprs.Power(Exprs.Integer(2), Exprs.Rational(1, 2)).IsExact);
        // sqrt(x), x^(3/2), 2^(1/3), i*sqrt(3)
        Assert.True(Exprs.Power(x, Exprs.Rational(1, 2)).IsExact);
        Assert.True(Exprs.Power(x, Exprs.Rational(3, 2)).IsExact);
        Assert.True(Exprs.Power(Exprs.Integer(2), Exprs.Rational(1, 3)).IsExact);
        Assert.True(Exprs.Sqrt(Exprs.Integer(-3)).IsExact);

        // a radical of an approximate base is still an approximation
        Assert.False(Exprs.Power(RealLeaf(3, 2), Exprs.Rational(1, 2)).IsExact);
        // ... and so is a radical whose SYMBOLIC exponent carries one
        Assert.False(Exprs.Power(x, Exprs.Add(y, RealLeaf(1, 2))).IsExact);

        // The one boundary of the rule, pinned so it cannot drift silently: a NUMERIC exponent is
        // canonicalized to a rational constant before the flag is computed (Constructors.Power),
        // so a Real numeric exponent never reaches the tree as a leaf — x^1.5 and x^(3/2) are the
        // same node and both are exact. The approximation survives as a leaf everywhere else
        // (Add, Multiply and function arguments), which is what the corpus test checks.
        Assert.Equal(Exprs.Power(x, Exprs.Rational(3, 2)), Exprs.Power(x, RealLeaf(3, 2)));
        Assert.True(Exprs.Power(x, RealLeaf(3, 2)).IsExact);

        // RootOf: an algebraic number, exact at construction
        Assert.True(Exprs.RootOf(Quintic(ctx, x), 0).IsExact);
    }

    // ------------------------------------------------------------------
    // 4. the language surface an agent reads
    // ------------------------------------------------------------------

    /// <summary>One engine serves the whole list: a SuiteEngine + plugin load is the expensive
    /// fixture here, and every case is a read-only inspection after the one symbol declaration.</summary>
    [Fact]
    public void InspectExact_IsTrue_ForExactClosedForms()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var expressions = new[]
        {
            "sin(x)", "cos(x)", "sqrt(x)", "log(x)", "exp(x)", "diff(sin(x), x)",
            "abs(x)", "x + 1", "sqrt(-2)", "x^(3/2)",
        };
        foreach (var expression in expressions)
        {
            var exact = engine.Evaluate("inspect(" + expression + ").exact");
            Assert.Equal(ValueKind.Boolean, exact.Kind);
            Assert.True(exact.AsBoolean(), "inspect(" + expression + ").exact");
        }
    }

    /// <summary>The negative half at the language surface: an expression built over the truncated
    /// decimal of an irrational keeps reporting exact = false. sqrt(2) is a Real VALUE in the
    /// language (the numeric sqrt), so the symbolic carrier is x*sqrt(2).</summary>
    [Fact]
    public void InspectExact_IsFalse_WhenARealApproximationIsInTheTree()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        var mixed = engine.Evaluate("inspect(x*sqrt(2)).exact");
        Assert.Equal(ValueKind.Boolean, mixed.Kind);
        Assert.False(mixed.AsBoolean(), "x*sqrt(2) carries the truncated decimal of an irrational");

        var substituted = engine.Evaluate("inspect(subs(x^2 + 1, x, sqrt(2))).exact");
        Assert.Equal(ValueKind.Boolean, substituted.Kind);
        Assert.False(substituted.AsBoolean(), "subs(...) over a Real approximation");
    }

    /// <summary>The Real VALUE path is a different mechanism: the language evaluates sqrt(2) to a
    /// Real, so its Inspection reports the Real's own provenance rather than a symbolic radical's —
    /// since the wire round, `inspect(<Real>).exact` is a Boolean and answers `false` for a
    /// truncated decimal, which is exactly what a Real sqrt(2) is.</summary>
    [Fact]
    public void SqrtOfTwoInTheLanguage_IsARealValue_NotASymbolicRadical()
    {
        var engine = NewEngine();
        var inspection = engine.Evaluate("inspect(sqrt(2))").AsRecord();
        Assert.Equal("Real", Field(inspection, "type").AsText());
        var exact = Field(inspection, "exact");
        Assert.Equal(ValueKind.Boolean, exact.Kind);
        Assert.False(exact.AsBoolean());
    }

    /// <summary>The quintic's RootOf root, as the solver publishes it: already exact before this
    /// round and still exact after (the control for the RootOf leaf).</summary>
    [Fact]
    public void RootOfRootPublishedByTheSolver_IsExact()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = engine.Evaluate("solve_full(x^5 - x - 1 == 0, x, real)").AsRecord();
        var solutions = Field(r, "solutions").AsVector();
        Assert.NotEmpty(solutions);
        foreach (var element in solutions)
        {
            var solution = element.AsRecord();
            Assert.True(Field(solution, "value").AsSymbolic().IsExact);
            Assert.Equal("AlgebraicExact", Field(solution, "exactness").AsEnum().Name);
        }
    }

    // ------------------------------------------------------------------
    // 5. the agreement rule
    // ------------------------------------------------------------------

    /// <summary>For every solution of every published result: the value's own exact flag and the
    /// record's exactness enum must agree. Both fields are read from the SAME Solution record.</summary>
    [Fact]
    public void PublishedSolutionValueExact_AgreesWithItsExactnessEnum()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var scripts = new[]
        {
            "solve_full(x^2 - 2 == 0, x, real)",      // irrational algebraic roots
            "solve_full(x^2 - 4 == 0, x, real)",      // rational roots
            "solve_full(x^2 + 1 == 0, x)",            // complex roots
            "solve_full(x^3 - 2 == 0, x, real)",      // a cube radical
            "solve_full(x^5 - x - 1 == 0, x, real)",  // RootOf
            "solve_full(exp(x) == 1, x)",             // a fold to a rational
            "solve_full((x^2 - 1)/(x - 1) == 0, x)",  // rational with a condition
        };
        foreach (var script in scripts)
        {
            var r = engine.Evaluate(script).AsRecord();
            var solutions = Field(r, "solutions").AsVector();
            foreach (var element in solutions)
            {
                var solution = element.AsRecord();
                var exactness = Field(solution, "exactness").AsEnum().Name;
                var value = Field(solution, "value").AsSymbolic();
                bool claimsExact = exactness != "Approximate";
                Assert.True(value.IsExact == claimsExact,
                    script + ": value " + Printing.PrettyPrint(value) + " has exact=" + value.IsExact +
                    " but exactness=" + exactness);
            }
        }
    }

    /// <summary>The same rule read off the kernel objects, for every member the solver can publish
    /// for a <see cref="Solution"/>: Exact and AlgebraicExact are exactness CLAIMS, so neither may
    /// be paired with an inexact value; Approximate is the only member that may.</summary>
    [Fact]
    public void SolverSolutionExactness_AndValueExact_Agree()
    {
        var ctx = NewContext();
        var x = ctx.Symbol("x");
        var cases = new[]
        {
            Exprs.Subtract(Exprs.Power(x, 2), 2),
            Exprs.Subtract(Exprs.Power(x, 2), 4),
            Exprs.Add(Exprs.Power(x, 2), 1),
            Exprs.Subtract(Exprs.Power(x, 3), 2),
            Exprs.Subtract(Exprs.Function(ctx.Function("exp"), x), 1),
        };

        var published = new List<SolutionExactness>();
        foreach (var f in cases)
        {
            var set = Solvers.Solve(f, x, ctx, SolveDomain.Real);
            foreach (var s in set.Solutions)
            {
                published.Add(s.Exactness);
                Assert.True(s.Value.IsExact == (s.Exactness != SolutionExactness.Approximate),
                    Printing.PrettyPrint(s.Value) + " exact=" + s.Value.IsExact + " exactness=" + s.Exactness);
            }
        }

        // the member that must never be paired with an inexact value is the one exercised here
        Assert.Contains(SolutionExactness.Exact, published);
        Assert.DoesNotContain(SolutionExactness.Approximate, published);
    }
}
