using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// An equation whose solution set is INFINITE over the requested domain must not be published as
/// <c>Solved</c> / <c>completeness: Complete</c> with a finite list that represents one member.
/// The reported repro is <c>solve_full(exp(x) == c, x)</c>: over the complex field
/// <c>exp(x) = c</c> has the infinite family <c>log(c) + 2*pi*k*i</c> (SymPy:
/// <c>solveset(exp(x) - c, x, S.Complexes)</c> is that ImageSet over the integers), and the kernel
/// used to answer with the single principal value <c>log(c)</c> under a Complete claim.
///
/// <para>The second defect pinned here is one probe away: a candidate that still CONTAINS the
/// solved variable is a circular answer, not a value (<c>exp(x) == x</c> "solved" to <c>log(x)</c>,
/// <c>log(x) == x</c> to <c>exp(x)</c>, <c>x - sin(x) == 0</c> to <c>sin(x)</c>), and it used to
/// enter a Solved set through the linear-coefficient path and through the exp inverse. No returned
/// solution value or family template may mention the variable being solved for, and a set that lost
/// a circular candidate is not complete.</para>
/// </summary>
public class InfiniteFamilyCompletenessTests
{
    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    private static Value F(RecordValue record, string name) =>
        (Value)record.Fields.First(f => f.Name == name).Value!;

    private static ExprContext NewContext(out Symbol x, params string[] others)
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        x = ctx.Symbol("x");
        foreach (string other in others)
            ctx.Symbol(other);
        return ctx;
    }

    private static Expr Exp(ExprContext ctx, Expr e) => Exprs.Function(ctx.Function("exp"), e);
    private static Expr Log(ExprContext ctx, Expr e) => Exprs.Function(ctx.Function("log"), e);
    private static Expr Sqrt(Expr e) => Exprs.Power(e, Exprs.Rational(1, 2));

    /// <summary>The kernel's own structural contract, restated on the test side: a value mentions
    /// the solved symbol when a symbol node for it survives under the ordinary arithmetic nodes.
    /// A RootOf's defining polynomial is opaque (x is that value's bound variable).</summary>
    private static bool Mentions(Expr e, Symbol x) => e switch
    {
        SymbolExpr s => s.Symbol.Name == x.Name,
        AddExpr a => a.Terms.Any(t => Mentions(t, x)),
        MultiplyExpr m => m.Factors.Any(f => Mentions(f, x)),
        PowerExpr p => Mentions(p.Base, x) || Mentions(p.Exponent, x),
        FunctionExpr f => f.Arguments.Any(a => Mentions(a, x)),
        _ => false,
    };

    /// <summary>Residual of <paramref name="equation"/> at a candidate, evaluated at 40 digits —
    /// the same numeric sense of "is a solution" the kernel's own gate uses.</summary>
    private static bool IsSolution(ExprContext ctx, Expr equation, Symbol x, Expr value)
    {
        Expr at = Evaluation.Substitute(equation, ctx, new Dictionary<Symbol, Expr> { [x] = value });
        using var scope = Rl.WithPrecision(40, 20);
        Num residual = Evaluation.EvaluateToNum(at, ctx, new Dictionary<Symbol, Num>());
        return NumOps.Compare(NumOps.Abs(residual, ctx),
            NumOps.FromReal(Rl.Parse("0." + new string('0', 24) + "1", null))) < 0;
    }

    private static Expr MemberOf(ExprContext ctx, SolutionFamily family, long k) =>
        Evaluation.Substitute(family.Template, ctx,
            new Dictionary<Symbol, Expr> { [family.Parameter] = Exprs.Integer(k) });

    // ------------------------------------------------------------------
    // 1. The reported defect: exp(x) = c over the complex field
    // ------------------------------------------------------------------

    /// <summary>Before the fix this was Solved / complete true / represented_count 1 holding the
    /// single value log(c), i.e. one member of an infinite solution set with a Complete claim.</summary>
    [Fact]
    public void SolveFull_ExpSymbolic_Complex_RepresentsTheWholeFamily()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\"); c = symbol(\"c\")");

        var r = engine.Evaluate("solve_full(exp(x) == c, x)").AsRecord();

        Assert.Equal("Solved", F(r, "status").AsEnum().Name);
        Assert.True(F(r, "complete").AsBoolean());
        Assert.Equal("Complete", F(r, "completeness").AsEnum().Name);

        // The finite list that omitted every k != 0 member is gone; the answer is the family.
        Assert.Empty(F(r, "solutions").AsVector());
        var families = F(r, "families").AsVector();
        Assert.Single(families);
        var family = families[0].AsRecord();
        Assert.Equal(MathDomain.Integer, F(family, "parameter_domain").AsDomain());
        Assert.Equal("ParametricExact", F(family, "exactness").AsEnum().Name);

        // exp(x) = 0 has no solution, so the family is valid under c != 0 and says so.
        var conditions = F(family, "conditions").AsVector();
        Assert.Single(conditions);
        Assert.Equal("c != 0", ValueFormatter.Format(conditions[0]));
    }

    /// <summary>Every member the family claims is a solution of the original equation, at several
    /// k — and the members are pairwise distinct, which is exactly why the old one-element answer
    /// could not be complete.</summary>
    [Fact]
    public void Kernel_ExpSymbolic_Complex_EveryFamilyMemberSolves()
    {
        ExprContext ctx = NewContext(out Symbol x, "c");
        Symbol c = ctx.Symbol("c");
        Expr equation = Exprs.Subtract(Exp(ctx, x), c);

        SolutionSet set = Solvers.Solve(Exprs.Relation(RelOp.Eq, Exp(ctx, x), c), x, ctx);

        Assert.Equal(SolveStatus.Solved, set.Status);
        Assert.Equal(Completeness.Complete, set.Complete);
        Assert.Empty(set.Solutions);
        Assert.Single(set.Families);
        SolutionFamily family = set.Families[0];
        Assert.Equal(ParameterDomain.Integers, family.Domain);
        Assert.False(Mentions(family.Template, x),
            "a family template is a value in its parameter and must not contain the solved variable");

        // for a concrete c = 3, every member of the family is a solution ...
        Expr concrete = Exprs.Subtract(Exp(ctx, x), Exprs.Integer(3));
        foreach (long k in new long[] { -3, -1, 0, 1, 4 })
        {
            Expr member = Evaluation.Substitute(MemberOf(ctx, family, k), ctx,
                new Dictionary<Symbol, Expr> { [c] = Exprs.Integer(3) });
            Assert.True(IsSolution(ctx, concrete, x, member),
                "k = " + k + " gives " + Printing.PrettyPrint(member) + ", which does not solve exp(x) = 3");
        }

        // ... and k = 0 and k = 1 are DIFFERENT solutions, so a one-element set omits the rest.
        using (Rl.WithPrecision(40, 20))
        {
            Num zero = Evaluation.EvaluateToNum(
                Evaluation.Substitute(MemberOf(ctx, family, 0), ctx, new Dictionary<Symbol, Expr> { [c] = Exprs.Integer(3) }),
                ctx, new Dictionary<Symbol, Num>());
            Num one = Evaluation.EvaluateToNum(
                Evaluation.Substitute(MemberOf(ctx, family, 1), ctx, new Dictionary<Symbol, Expr> { [c] = Exprs.Integer(3) }),
                ctx, new Dictionary<Symbol, Num>());
            Assert.False(NumOps.IsZero(NumOps.Subtract(one, zero)),
                "the family's k = 0 and k = 1 members must be distinct solutions");
        }
    }

    /// <summary>The numeric-c complex call is the same defect: exp(x) = 5 has the family
    /// log(5) + 2*pi*k*i, not the single value log(5).</summary>
    [Fact]
    public void SolveFull_ExpNumeric_Complex_IsNotThePrincipalValueAlone()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        var r = engine.Evaluate("solve_full(exp(x) == 5, x)").AsRecord();

        Assert.Equal("Solved", F(r, "status").AsEnum().Name);
        Assert.True(F(r, "complete").AsBoolean());
        Assert.Empty(F(r, "solutions").AsVector());
        var family = F(r, "families").AsVector()[0].AsRecord();
        Assert.Equal(MathDomain.Integer, F(family, "parameter_domain").AsDomain());
        Expr template = F(family, "template").AsSymbolic();
        Assert.False(Mentions(template, NewSymbolX()));
    }

    private static Symbol NewSymbolX()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx.Symbol("x");
    }

    // ------------------------------------------------------------------
    // 2. The real domain keeps its (correct) answer
    // ------------------------------------------------------------------

    [Fact]
    public void RealDomain_ExpKeepsTheSingleSolution()
    {
        ExprContext ctx = NewContext(out Symbol x);
        SolutionSet positive = Solvers.Solve(Exprs.Relation(RelOp.Eq, Exp(ctx, x), Exprs.Integer(5)), x, ctx, SolveDomain.Real);
        Assert.Equal(SolveStatus.Solved, positive.Status);
        Assert.Equal(Completeness.Complete, positive.Complete);
        Assert.Single(positive.Solutions);
        Assert.Empty(positive.Families);
        Assert.True(IsSolution(ctx, Exprs.Subtract(Exp(ctx, x), Exprs.Integer(5)), x, positive.Solutions[0].Value));

        SolutionSet negative = Solvers.Solve(Exprs.Relation(RelOp.Eq, Exp(ctx, x), Exprs.Integer(-5)), x, ctx, SolveDomain.Real);
        Assert.Equal(SolveStatus.NoSolutions, negative.Status);
        Assert.Empty(negative.Families);

        SolutionSet zero = Solvers.Solve(Exprs.Relation(RelOp.Eq, Exp(ctx, x), Exprs.Zero), x, ctx, SolveDomain.Real);
        Assert.Equal(SolveStatus.NoSolutions, zero.Status);
    }

    [Fact]
    public void RealDomain_ExpSymbolic_KeepsItsBehaviour_AndNeverCarriesTheComplexPeriod()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\"); c = symbol(\"c\")");

        var r = engine.Evaluate("solve_full(exp(x) == c, x, real)").AsRecord();

        Assert.Equal(MathDomain.Real, F(r, "domain").AsDomain());
        Assert.Equal("Solved", F(r, "status").AsEnum().Name);
        Assert.Single(F(r, "solutions").AsVector());
        Assert.Empty(F(r, "families").AsVector());   // the complex period 2*pi*i never leaks into the reals
    }

    // ------------------------------------------------------------------
    // 3. The rule is the inverse-family rule, not an exp special case
    // ------------------------------------------------------------------

    /// <summary>The families cycle 4 shipped, and the single-valued inverse that must NOT become
    /// one: log is a function (principal branch), so log(x) = c has exactly one solution e^c
    /// (SymPy: solveset(log(x) - c, x, S.Complexes) = {exp(c)}).</summary>
    [Fact]
    public void TheOtherInverses_KeepTheirOwnHonestAnswer()
    {
        ExprContext ctx = NewContext(out Symbol x, "c");
        Symbol c = ctx.Symbol("c");

        SolutionSet sinHalf = Solvers.Solve(Exprs.Relation(RelOp.Eq, Exprs.Function(ctx.Function("sin"), x), Exprs.Rational(1, 2)), x, ctx);
        Assert.Equal(SolveStatus.Solved, sinHalf.Status);
        Assert.Equal(2, sinHalf.Families.Count);
        Assert.All(sinHalf.Families, f => Assert.Equal(ParameterDomain.Integers, f.Domain));

        SolutionSet cosHalf = Solvers.Solve(Exprs.Relation(RelOp.Eq, Exprs.Function(ctx.Function("cos"), x), Exprs.Rational(1, 2)), x, ctx);
        Assert.Equal(SolveStatus.Solved, cosHalf.Status);
        Assert.Equal(2, cosHalf.Families.Count);

        SolutionSet tanOne = Solvers.Solve(Exprs.Relation(RelOp.Eq, Exprs.Function(ctx.Function("tan"), x), Exprs.One), x, ctx);
        Assert.Equal(SolveStatus.Solved, tanOne.Status);
        Assert.Single(tanOne.Families);

        SolutionSet logC = Solvers.Solve(Exprs.Relation(RelOp.Eq, Log(ctx, x), c), x, ctx);
        Assert.Equal(SolveStatus.Solved, logC.Status);
        Assert.Equal(Completeness.Complete, logC.Complete);
        Assert.Single(logC.Solutions);
        Assert.Empty(logC.Families);
        Assert.False(Mentions(logC.Solutions[0].Value, x));
    }

    // ------------------------------------------------------------------
    // 4. Circular candidates are not values and never enter a Solved set
    // ------------------------------------------------------------------

    public static IEnumerable<object[]> CircularEquations() => new[]
    {
        new object[] { "exp(x) == x", SolveDomain.Complex },
        new object[] { "log(x) == x", SolveDomain.Complex },
        new object[] { "x - log(x) == 0", SolveDomain.Complex },
        new object[] { "x - sin(x) == 0", SolveDomain.Complex },
        new object[] { "x - cos(x) == 0", SolveDomain.Complex },
        new object[] { "x*exp(x) == 1", SolveDomain.Complex },
        new object[] { "x + exp(x) == 0", SolveDomain.Complex },
        new object[] { "exp(x) == x", SolveDomain.Real },
        new object[] { "log(x) == x", SolveDomain.Real },
        new object[] { "x + exp(x) == 0", SolveDomain.Real },
    };

    /// <summary>Every one of these has at least one solution (exp(x) = x has one, x*exp(x) = 1 has
    /// one: SymPy answers LambertW), so an empty-set claim would be as wrong as the circular one —
    /// and before the fix several of them returned Solved/complete with a value containing x.</summary>
    [Theory]
    [MemberData(nameof(CircularEquations))]
    public void CircularCandidates_AreNotRepresentedAndAreNotACompleteSet(string script, SolveDomain domain)
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        string call = "solve_full(" + script + ", x" + (domain == SolveDomain.Real ? ", real" : "") + ")";

        var r = engine.Evaluate(call).AsRecord();
        string status = F(r, "status").AsEnum().Name;

        Assert.NotEqual("Solved", status);
        Assert.NotEqual("NoSolutions", status);   // the equations have solutions; nothing is proven empty
        Assert.False(F(r, "complete").AsBoolean());
        Assert.NotEqual("Complete", F(r, "completeness").AsEnum().Name);

        foreach (var s in F(r, "solutions").AsVector())
            Assert.DoesNotContain("(sym x)", Printing.CanonicalPrint(F(s.AsRecord(), "value").AsSymbolic()));
        foreach (var f in F(r, "families").AsVector())
            Assert.DoesNotContain("(sym x)", Printing.CanonicalPrint(F(f.AsRecord(), "template").AsSymbolic()));
    }

    /// <summary>The inverse branch itself still works when its result IS a value: sqrt(x) = x has
    /// the two solutions 0 and 1, and both are represented (the circular rule must not reject
    /// legitimate candidates).</summary>
    [Fact]
    public void TheLegitimateInverseBranch_StillRepresentsItsValues()
    {
        ExprContext ctx = NewContext(out Symbol x);
        SolutionSet set = Solvers.Solve(Exprs.Relation(RelOp.Eq, Sqrt(x), x), x, ctx);

        Assert.Equal(SolveStatus.Solved, set.Status);
        Assert.Equal(Completeness.Complete, set.Complete);
        Assert.Equal(2, set.Solutions.Count);
        Assert.All(set.Solutions, s => Assert.False(Mentions(s.Value, x)));
        foreach (Solution s in set.Solutions)
            Assert.True(IsSolution(ctx, Exprs.Subtract(Sqrt(x), x), x, s.Value));
    }

    // ------------------------------------------------------------------
    // 5. Nothing else moves (the acceptance controls)
    // ------------------------------------------------------------------

    [Fact]
    public void AcceptanceControls_NothingElseMoves()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        var quadratic = engine.Evaluate("solve_full(x^2 - 4 == 0, x)").AsRecord();
        Assert.Equal("Solved", F(quadratic, "status").AsEnum().Name);
        Assert.True(F(quadratic, "complete").AsBoolean());
        Assert.Equal(2, F(quadratic, "solutions").AsVector().Count);

        var noReal = engine.Evaluate("solve_full(x^2 + 1 == 0, x, real)").AsRecord();
        Assert.Equal("NoSolutions", F(noReal, "status").AsEnum().Name);
        Assert.True(F(noReal, "complete").AsBoolean());

        var extraneous = engine.Evaluate("solve_full(sqrt(x) + 2 == 0, x)").AsRecord();
        Assert.Equal("Unevaluated", F(extraneous, "status").AsEnum().Name);
        Assert.False(F(extraneous, "complete").AsBoolean());
        Assert.Equal("0", F(extraneous, "represented_count").AsInteger().ToString());

        var quartic = engine.Evaluate("solve_full(x^4 - x^2 - 1 == 0, x)").AsRecord();
        Assert.Equal("Partial", F(quartic, "status").AsEnum().Name);
        Assert.False(F(quartic, "complete").AsBoolean());
        Assert.Equal("2", F(quartic, "represented_count").AsInteger().ToString());
        Assert.Equal("2", F(quartic, "unrepresented_count").AsInteger().ToString());
    }

    // ------------------------------------------------------------------
    // 6. The oracle: the complex answer is SymPy's ImageSet, not one value
    // ------------------------------------------------------------------

    [RequiresSympyFact]
    public void SympyOracle_ExpOverComplexes_IsAnImageSet_AndTheTemplateMatchesIt()
    {
        SympySession sympy = SympyOracle.Require();

        string answer = sympy.Eval(
            "x, c = sympy.symbols('x c')\n" +
            "print(sympy.solveset(sympy.exp(x) - c, x, sympy.S.Complexes))");
        Assert.Contains("ImageSet", answer);          // an infinite family, not a finite set
        Assert.Contains("Integers", answer);          // parameterized by the integers
        // SymPy prints the period as I*2*_n*pi: the family steps by 2*pi*i per integer step.
        Assert.Contains("2*_n*pi", answer);

        // The kernel's template is that family: SymPy proves exp(log(c) + 2*pi*i*k) - c = 0 for an
        // integer k, and for every concrete integer in -3..3 (SymPy needs the integer assumption to
        // reduce exp(2*pi*i*k) to 1, which is exactly the parameter domain the family declares).
        string residual = sympy.Eval(
            "c = sympy.symbols('c')\n" +
            "k = sympy.symbols('k', integer=True)\n" +
            "print(sympy.simplify(sympy.exp(sympy.log(c) + 2*sympy.pi*sympy.I*k) - c))");
        Assert.Equal("0", residual);
        string concrete = sympy.Eval(
            "c = sympy.symbols('c')\n" +
            "print([sympy.simplify(sympy.exp(sympy.log(c) + 2*sympy.pi*sympy.I*k) - c) for k in range(-3, 4)])");
        Assert.Equal("[0, 0, 0, 0, 0, 0, 0]", concrete);

        // ... and over the REALS SymPy confirms the one solution the kernel reports for c > 0.
        string realAnswer = sympy.Eval(
            "x = sympy.symbols('x')\n" +
            "print(sympy.solveset(sympy.exp(x) - 5, x, sympy.S.Reals))");
        Assert.Equal("{log(5)}", realAnswer);
    }
}
