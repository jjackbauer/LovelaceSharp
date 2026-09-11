using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Lovelace.Symbolics.Rewriting;
using Xunit;
using Rat = Lovelace.Rational.Rational;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// DX-convergence Phase 1 regression corpus: safe-vs-full simplify, exp/log definedness,
/// the explicit solver domain contract, contradictory-condition handling, power-domain
/// inference, and the budget/trace fixes.
/// </summary>
public class DxSemanticClosureTests
{
    private static ExprContext NewCtx()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx;
    }

    /// <summary>An enum-valued field's whole observable identity: the structured kind, the
    /// declared enum type name, the member, and the two renderings (which must show the bare
    /// member name).</summary>
    private static void AssertEnumField(object? payload, string typeName, string memberName)
    {
        var value = (Value)payload!;
        var dto = StructuredProjection.ToStructured(value);
        Assert.Equal(ValueKind.Enum, value.Kind);
        Assert.Equal(typeName, value.AsEnum().TypeName);
        Assert.Equal(memberName, value.AsEnum().Name);
        Assert.Equal("Enum", dto.Kind);
        Assert.Equal(typeName, dto.Type);
        Assert.Equal(memberName, dto.Value);
        Assert.Equal(memberName, ValueFormatter.Format(value));
        Assert.Equal(memberName, ValueFormatter.FormatTyped(value));
    }

    private static void AssertEnumField(SuiteEngine engine, string expression, string typeName, string memberName) =>
        AssertEnumField(engine.Evaluate(expression), typeName, memberName);

    // ------------------------------------------------------------------
    // Safe simplify: conditional rules fire only under proven conditions
    // ------------------------------------------------------------------

    [Fact]
    public void Simplify_XOverX_StaysUnevaluated_WithoutAssumptions()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var e = Exprs.Divide(x, x);
        var result = Simplify.SimplifyExpr(e, ctx);
        Assert.Equal("x/x", Printing.PrettyPrint(result));
    }

    [Fact]
    public void Simplify_XOverX_Cancels_UnderNonZeroAssumption()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        using var assumptions = ctx.WithAssumptions(
            AssumptionSet.Empty.Add(new SymbolPropertyAssumption(x, SymbolPredicate.NonZero)));
        Assert.Equal(Exprs.One, Simplify.SimplifyExpr(Exprs.Divide(x, x), ctx));
    }

    [Fact]
    public void Simplify_ExpLog_StaysUnevaluated_WithoutAssumptions()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var e = Exprs.Function(ctx.Function("exp"), Exprs.Function(ctx.Function("log"), x));
        var result = Simplify.SimplifyExpr(e, ctx);
        Assert.Equal("exp(log(x))", Printing.PrettyPrint(result));
    }

    [Fact]
    public void Simplify_ExpLog_Fires_UnderPositiveAssumption()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        using var assumptions = ctx.WithAssumptions(
            AssumptionSet.Empty.Add(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Zero)));
        var e = Exprs.Function(ctx.Function("exp"), Exprs.Function(ctx.Function("log"), x));
        Assert.Equal(x, Simplify.SimplifyExpr(e, ctx));
    }

    [Fact]
    public void Simplify_UniversalRule_Fires_WithoutAssumptions()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var e = Exprs.Add(
            Exprs.Power(Exprs.Function(ctx.Function("sin"), x), 2),
            Exprs.Power(Exprs.Function(ctx.Function("cos"), x), 2));
        Assert.Equal(Exprs.One, Simplify.SimplifyExpr(e, ctx));
    }

    // ------------------------------------------------------------------
    // Full transform: conditions and provenance
    // ------------------------------------------------------------------

    [Fact]
    public void Transform_XOverX_CarriesNonZeroCondition()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var result = Simplify.Transform(Exprs.Divide(x, x), ctx, new Simplify.Options(Trace: true));
        Assert.Equal(Exprs.One, result.Expression);
        Assert.False(result.Conditions.IsUnsatisfiable);
        Assert.Equal(Tristate.True, result.Conditions.Ask(new SymbolPropertyAssumption(x, SymbolPredicate.NonZero)));
        Assert.Contains(result.Steps, s => s.RuleId == "rat.cancel-x-over-x" && s.Classification == RuleClassification.Conditional);
    }

    [Fact]
    public void Transform_ExpLog_CarriesFiniteLogCondition()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var e = Exprs.Function(ctx.Function("exp"), Exprs.Function(ctx.Function("log"), x));
        var result = Simplify.Transform(e, ctx, new Simplify.Options(Trace: true));
        Assert.Equal(x, result.Expression);
        var step = Assert.Single(result.Steps, s => s.RuleId == "logexp.exp-log");
        Assert.Equal(RuleClassification.Conditional, step.Classification);
        Assert.Contains(step.Conditions.Atoms, a =>
            a is ExpressionPropertyAssumption { P: SymbolPredicate.Finite } ep &&
            ep.E is FunctionExpr f && f.Function.Name == "log");
    }

    [Fact]
    public void Transform_TraceIsOptIn_ButConditionsAlwaysSurvive()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        // without Trace the steps are empty but the semantic conditions still come through
        var result = Simplify.Transform(Exprs.Divide(x, x), ctx);
        Assert.Empty(result.Steps);
        Assert.Equal(Tristate.True, result.Conditions.Ask(new SymbolPropertyAssumption(x, SymbolPredicate.NonZero)));
        Assert.False(result.BudgetExceeded);
    }

    [Fact]
    public void Transform_BudgetExceeded_IsReported()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var e = Exprs.Divide(x, x);
        var result = Simplify.Transform(e, ctx, new Simplify.Options(MaxSteps: 0));
        Assert.True(result.BudgetExceeded);
    }

    // ------------------------------------------------------------------
    // Solver domain contract
    // ------------------------------------------------------------------

    [Fact]
    public void Solve_QuadraticNegativeDiscriminant_DefaultDomainIsComplex()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var set = Solvers.Solve(Exprs.Add(Exprs.Power(x, 2), Exprs.One), x, ctx);
        Assert.Equal(SolutionKind.Exact, set.Kind);
        Assert.Equal(2, set.Solutions.Count);
    }

    [Fact]
    public void Solve_QuadraticNegativeDiscriminant_RealDomain_IsEmpty()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var set = Solvers.Solve(Exprs.Add(Exprs.Power(x, 2), Exprs.One), x, ctx, SolveDomain.Real);
        Assert.Equal(SolutionKind.Empty, set.Kind);
        Assert.Equal("no real solutions", set.Note);
    }

    [Fact]
    public void Solve_QuarticNoRealRoots_DefaultDomain_ReportsUnevaluated()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var set = Solvers.Solve(Exprs.Add(Exprs.Power(x, 4), Exprs.One), x, ctx);
        Assert.Equal(SolutionKind.Unevaluated, set.Kind);
        Assert.Contains("complex algebraic roots", set.Note);
    }

    [Fact]
    public void Solve_QuarticNoRealRoots_RealDomain_IsEmpty()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var set = Solvers.Solve(Exprs.Add(Exprs.Power(x, 4), Exprs.One), x, ctx, SolveDomain.Real);
        Assert.Equal(SolutionKind.Empty, set.Kind);
        Assert.Equal("no real solutions", set.Note);
    }

    [Fact]
    public void Solve_QuarticWithRealRoots_DefaultDomain_KeepsRootOfRoots()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var set = Solvers.Solve(
            Exprs.Subtract(Exprs.Power(x, 4), Exprs.Multiply(5, Exprs.Power(x, 2))), x, ctx);
        Assert.Equal(SolutionKind.Exact, set.Kind);
        Assert.Equal(3, set.Solutions.Count);   // -sqrt5, 0, +sqrt5 over the reals
    }

    [Fact]
    public void Solve_CubicRealDomain_KeepsOnlyRealRoots()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var eq = Exprs.Subtract(Exprs.Power(x, 3), Exprs.One);
        Assert.Equal(3, Solvers.Solve(eq, x, ctx, SolveDomain.Complex).Solutions.Count);
        var real = Solvers.Solve(eq, x, ctx, SolveDomain.Real);
        Assert.Equal(SolutionKind.Exact, real.Kind);
        Assert.Single(real.Solutions);
    }

    // ------------------------------------------------------------------
    // Contradictory conditions and bound reasoning
    // ------------------------------------------------------------------

    [Fact]
    public void Assume_ContradictoryRelations_Throws()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var set = AssumptionSet.Empty.Add(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Zero));
        Assert.Throws<AssumptionContradictionException>(
            () => set.Add(new SymbolRelationAssumption(x, RelOp.Lt, Exprs.Zero)));
    }

    [Fact]
    public void Assume_CompatibleRelations_DoNotThrow()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var set = AssumptionSet.Empty.Add(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Zero));
        set = set.Add(new SymbolRelationAssumption(x, RelOp.Lt, Exprs.Integer(5L)));
        Assert.Equal(Tristate.True, set.Ask(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Zero)));
    }

    [Fact]
    public void AskRelation_ProvesWeakerBounds()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var set = AssumptionSet.Empty.Add(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Integer(5L)));
        Assert.Equal(Tristate.True, set.Ask(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Zero)));
        Assert.Equal(Tristate.True, set.Ask(new SymbolRelationAssumption(x, RelOp.Ge, Exprs.Integer(5L))));
        Assert.Equal(Tristate.True, set.Ask(new SymbolRelationAssumption(x, RelOp.Ne, Exprs.Zero)));
        Assert.Equal(Tristate.True, set.Ask(new SymbolPropertyAssumption(x, SymbolPredicate.NonZero)));
        Assert.Equal(Tristate.Unknown, set.Ask(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Integer(6L))));
        Assert.Equal(Tristate.Unknown, set.Ask(new SymbolRelationAssumption(x, RelOp.Lt, Exprs.Integer(10L))));
    }

    [Fact]
    public void AskRelation_RefutesContradictoryBounds()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var set = AssumptionSet.Empty.Add(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Integer(5L)));
        Assert.Equal(Tristate.False, set.Ask(new SymbolRelationAssumption(x, RelOp.Lt, Exprs.Integer(5L))));
        Assert.Equal(Tristate.False, set.Ask(new SymbolRelationAssumption(x, RelOp.Le, Exprs.Integer(4L))));
    }

    [Fact]
    public void Unsatisfiable_Sentinel_IsSticky()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        Assert.True(AssumptionSet.Unsatisfiable.IsUnsatisfiable);
        Assert.False(AssumptionSet.Empty.IsUnsatisfiable);
        var added = AssumptionSet.Unsatisfiable.Add(new SymbolPropertyAssumption(x, SymbolPredicate.Positive));
        Assert.True(added.IsUnsatisfiable);
        Assert.False(AssumptionSet.Unsatisfiable.Equals(AssumptionSet.Empty));
    }

    // ------------------------------------------------------------------
    // Power-domain inference
    // ------------------------------------------------------------------

    [Fact]
    public void DomainOf_NegativeIntegerPower_IsRationalNotInteger()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        using var assumptions = ctx.WithAssumptions(
            AssumptionSet.Empty.Add(new SymbolDomainAssumption(x, Domain.Integer)));
        var inv = Exprs.Power(x, Exprs.Rational(Rat.MinusOne));
        Assert.Equal(Domain.Rational, Domains.DomainOf(inv, ctx));
        Assert.Equal(Domain.Integer, Domains.DomainOf(Exprs.Power(x, Exprs.Integer(2)), ctx));
    }

    // ------------------------------------------------------------------
    // Language-level regression cases
    // ------------------------------------------------------------------

    [Fact]
    public void Language_SolveQuartic_ReportsUnevaluatedNote()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        engine.Evaluate("x = symbol(\"x\")");
        var result = engine.Evaluate("solve(x^4 + 1 == 0, x)");
        Assert.Equal("unevaluated: complex algebraic roots not supported (RootOf is real-only in v1).",
            ValueFormatter.FormatTyped(result));
    }

    [Fact]
    public void Language_SolveQuartic_MixedRoots_IsPartial_NotComplete()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        engine.Evaluate("x = symbol(\"x\")");
        // 0 < real roots < degree: the solver must not claim a complete complex solution set
        AssertEnumField(engine, "solve_full(x^4 - x^2 - 1 == 0, x).status", "SolveStatus", "Partial");
        Assert.False(engine.Evaluate("solve_full(x^4 - x^2 - 1 == 0, x).complete").AsBoolean());
        Assert.Equal("2", engine.Evaluate("solve_full(x^4 - x^2 - 1 == 0, x).unrepresented_count").AsInteger().ToString());
        // and the convenience API must not hand back an apparently complete vector
        Assert.Equal(ValueKind.Text, engine.Evaluate("solve(x^4 - x^2 - 1 == 0, x)").Kind);
        // the real domain is complete and still returns a vector
        Assert.Equal(ValueKind.Vector, engine.Evaluate("solve(x^4 - x^2 - 1 == 0, x, real)").Kind);
    }

    [Fact]
    public void Language_SolveUnsupportedDomain_IsRejected()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        engine.Evaluate("x = symbol(\"x\")");
        var ex = Assert.ThrowsAny<Exception>(() => engine.Evaluate("solve(x^2 + 1 == 0, x, integer)"));
        Assert.Contains("currently supports domains real and complex; got integer", ex.Message);
        var ex2 = Assert.ThrowsAny<Exception>(() => engine.Evaluate("solve(x^2 + 1 == 0, x, rational)"));
        Assert.Contains("currently supports domains real and complex; got rational", ex2.Message);
    }

    [Fact]
    public void Language_SolvePrincipalBranch_ReturnsAllRoots()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        engine.Evaluate("x = symbol(\"x\")");
        // (x+1)^2 = 4 has TWO solutions: emitting only the principal root was a wrong answer
        Assert.Equal("[-3, 1]", ValueFormatter.Format(engine.Evaluate("solve((x+1)^2 - 4 == 0, x)")));
        // (x+1)^3 = 8 has three
        var cube = engine.Evaluate("solve_full((x+1)^3 == 8, x)").AsRecord();
        AssertEnumField(cube.Fields[0].Value, "SolveStatus", "Solved");
        Assert.Equal(3, ((Value)cube.Fields[5].Value!).AsVector().Count);
    }

    [Fact]
    public void Language_SolveMultiplicityAndNoSolutions_AreHonest()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        engine.Evaluate("x = symbol(\"x\")");
        // multiplicity survives the factor -> root projection
        var repeated = engine.Evaluate("solve_full(x^2 - 2*x + 1 == 0, x)").AsRecord();
        var sol = (RecordValue)((Value)((Value)repeated.Fields[5].Value!).AsVector()[0]).AsRecord();
        Assert.Equal("2", ((Value)sol.Fields[2].Value!).AsInteger().ToString());
        // every candidate excluded by a pole is NoSolutions, never Solved with an empty vector
        AssertEnumField(engine, "solve_full((x^2-1)/(x^2-1) == 0, x).status", "SolveStatus", "NoSolutions");
    }

    [Fact]
    public void Language_Simplify_IsSafeByDefault()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        engine.Evaluate("x = symbol(\"x\")");
        var result = engine.Evaluate("simplify(x/x)");
        Assert.Equal("x/x (Symbolic)", ValueFormatter.FormatTyped(result));
        engine.Evaluate("assume(x != 0)");
        var cancelled = engine.Evaluate("simplify(x/x)");
        Assert.Equal("1 (Symbolic)", ValueFormatter.FormatTyped(cancelled));
    }

    [Fact]
    public void Language_Assume_ContradictionThrows()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        engine.Evaluate("x = symbol(\"x\")");
        engine.Evaluate("assume(x > 0)");
        var ex = Assert.Throws<AssumptionContradictionException>(() => engine.Evaluate("assume(x < 0)"));
        Assert.Contains("contradicts", ex.Message);
    }

    [Fact]
    public void Pretty_FractionCoefficient_FoldsIntoDenominator()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var term = Exprs.Multiply(Exprs.Rational(-1, 2), Exprs.Power(Exprs.Add(x, Exprs.One), Exprs.Rational(-1)));
        // -1/2 · (x+1)^-1 renders as -1/(2*(x + 1)), never -1/2/((x + 1)) or -1/2*(x + 1)
        Assert.Equal("-1/(2*(x + 1))", Printing.PrettyPrint(term));
        Assert.Equal("x/x", Printing.PrettyPrint(Exprs.Divide(x, x)));
        Assert.Equal("2/x", Printing.PrettyPrint(Exprs.Divide(Exprs.Integer(2), x)));
    }
}
