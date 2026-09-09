using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;
using Rat = Lovelace.Rational.Rational;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Symbolics.Tests;

/// <summary>First-class relations (And/Or/Not) and parametric solution families (Phase 3).</summary>
public class RelationsAndFamiliesTests
{
    private static ExprContext NewCtx()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx;
    }

    // ------------------------------------------------------------------
    // And / Or / Not
    // ------------------------------------------------------------------

    [Fact]
    public void LogicalCanonicalization()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var gt = Exprs.Relation(RelOp.Gt, x, Exprs.Zero);
        var lt = Exprs.Relation(RelOp.Lt, x, Exprs.One);
        // double negation collapses
        Assert.Equal(gt, Exprs.Not(Exprs.Not(gt)));
        // duplicates collapse, constants fold
        Assert.Equal(gt, Exprs.And(gt, gt));
        Assert.Equal(gt, Exprs.And(gt, Exprs.One));
        Assert.Equal(Exprs.Zero, Exprs.And(gt, Exprs.Zero));
        Assert.Equal(Exprs.One, Exprs.Or(gt, Exprs.One));
        Assert.Equal(Exprs.Zero, Exprs.Or(Exprs.Zero, Exprs.Zero));
        Assert.Equal(gt, Exprs.Or(gt, Exprs.Zero));
        // round-trip through canonical text
        var combo = Exprs.And(gt, lt);
        var text = Printing.CanonicalPrint(combo);
        Assert.Equal(combo, Printing.CanonicalParse(text, ctx));
        Assert.Equal("x < 1 and x > 0", Printing.PrettyPrint(combo));   // canonical operand order
    }

    [Fact]
    public void KleeneEvaluation_AndOrNot()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var cond = Exprs.And(
            Exprs.Relation(RelOp.Gt, x, Exprs.Zero),
            Exprs.Relation(RelOp.Lt, x, Exprs.One));
        var half = new Dictionary<Symbol, Num> { [x] = NumOps.FromRat(Rat.From(1, 2)) };
        var two = new Dictionary<Symbol, Num> { [x] = NumOps.FromRat(Rat.FromLong(2)) };
        Assert.Equal(Tristate.True, Evaluation.EvaluateCondition(cond, ctx, half));
        Assert.Equal(Tristate.False, Evaluation.EvaluateCondition(cond, ctx, two));
        Assert.Equal(Tristate.Unknown, Evaluation.EvaluateCondition(cond, ctx, new Dictionary<Symbol, Num>()));
        // numeric evaluation: 1/0 with Unknown rejected
        Assert.Equal(0, NumOps.Compare(NumOps.FromLong(1), Evaluation.EvaluateToNum(cond, ctx, half)));
        Assert.Equal(0, NumOps.Compare(NumOps.FromLong(0), Evaluation.EvaluateToNum(cond, ctx, two)));
        Assert.Throws<EvaluationException>(() => Evaluation.EvaluateToNum(cond, ctx, new Dictionary<Symbol, Num>()));
        // Not + Or
        var neg = Exprs.Not(Exprs.Relation(RelOp.Gt, x, Exprs.Zero));
        Assert.Equal(Tristate.False, Evaluation.EvaluateCondition(neg, ctx, half));
        var or = Exprs.Or(Exprs.Relation(RelOp.Gt, x, Exprs.One), Exprs.Relation(RelOp.Lt, x, Exprs.Zero));
        Assert.Equal(Tristate.False, Evaluation.EvaluateCondition(or, ctx, half));
        Assert.Equal(Tristate.True, Evaluation.EvaluateCondition(or, ctx, two));
    }

    [Fact]
    public void Piecewise_WithCompoundGuard_Evaluates()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var pw = Exprs.Piecewise(
            new[]
            {
                new PiecewiseBranch(
                    Exprs.And(Exprs.Relation(RelOp.Gt, x, Exprs.Zero), Exprs.Relation(RelOp.Lt, x, Exprs.One)),
                    Exprs.One),
            },
            Exprs.Zero);
        var half = new Dictionary<Symbol, Num> { [x] = NumOps.FromRat(Rat.From(1, 2)) };
        var two = new Dictionary<Symbol, Num> { [x] = NumOps.FromRat(Rat.FromLong(2)) };
        Assert.Equal(0, NumOps.Compare(NumOps.FromLong(1), Evaluation.EvaluateToNum(pw, ctx, half)));
        Assert.Equal(0, NumOps.Compare(NumOps.FromLong(0), Evaluation.EvaluateToNum(pw, ctx, two)));
        // and it lowers to MathIR with the same semantics
        var prog = Lowering.Lower(pw, ctx, new[] { x });
        using var scope = Rl.WithPrecision(40, 20);
        Assert.Equal(0, NumOps.Compare(NumOps.FromLong(1), IrEvaluator.Evaluate(prog, half, ctx)));
        Assert.Equal(0, NumOps.Compare(NumOps.FromLong(0), IrEvaluator.Evaluate(prog, two, ctx)));
    }

    [Fact]
    public void Assume_ConjunctionAndNegation_ThroughEngine()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        engine.Evaluate("x = symbol(\"x\")");
        engine.Evaluate("assume(and(x > 0, x < 5))");
        Assert.Equal("x > 0; x < 5", engine.Evaluate("assumptions()").AsText());
        engine.Evaluate("assume(not(x == 2))");
        Assert.Equal("x > 0; x < 5; x != 2", engine.Evaluate("assumptions()").AsText());
        // logical builtins produce symbolic conditions
        var combo = engine.Evaluate("and(x > 0, x < 1)");
        Assert.Equal(ValueKind.Symbolic, combo.Kind);
        Assert.Equal("x < 1 and x > 0", ValueFormatter.Format(combo));
    }

    // ------------------------------------------------------------------
    // Parametric families
    // ------------------------------------------------------------------

    [Fact]
    public void Solve_SinZero_ReturnsFamily()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var set = Solvers.Solve(Exprs.Relation(RelOp.Eq, Exprs.Function(ctx.Function("sin"), x), Exprs.Zero), x, ctx);
        Assert.Equal(SolutionKind.Exact, set.Kind);
        Assert.Empty(set.Solutions);
        Assert.Single(set.Families);
        Assert.Equal("k*pi", Printing.PrettyPrint(set.Families[0].Template));
        Assert.Equal("k", set.Families[0].Parameter.Name);
    }

    [Fact]
    public void Solve_CosHalf_ReturnsTwoFamilies()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var set = Solvers.Solve(Exprs.Relation(RelOp.Eq, Exprs.Function(ctx.Function("cos"), x), Exprs.Rational(1, 2)), x, ctx);
        Assert.Equal(2, set.Families.Count);
    }

    [Fact]
    public void Solve_SinBeyondRange_EmptyWithNote()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var set = Solvers.Solve(Exprs.Relation(RelOp.Eq, Exprs.Function(ctx.Function("sin"), x), Exprs.Integer(2)), x, ctx);
        Assert.Equal(SolutionKind.Empty, set.Kind);
        Assert.NotNull(set.Note);
    }

    [Fact]
    public void Solve_ExpZero_Empty()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var set = Solvers.Solve(Exprs.Relation(RelOp.Eq, Exprs.Function(ctx.Function("exp"), x), Exprs.Zero), x, ctx);
        Assert.Equal(SolutionKind.Empty, set.Kind);
    }

    [Theory]
    [InlineData(0)]
    [InlineData(1)]
    [InlineData(2)]
    public void SinFamily_MembersSatisfyEquation_Numerically(int k)
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var set = Solvers.Solve(Exprs.Relation(RelOp.Eq, Exprs.Function(ctx.Function("sin"), x), Exprs.Zero), x, ctx);
        var family = set.Families[0];
        var member = Evaluation.Substitute(family.Template, ctx,
            new Dictionary<Symbol, Expr> { [family.Parameter] = Exprs.Integer(k) });
        using var scope = Rl.WithPrecision(50, 20);
        var residual = Evaluation.EvaluateToNum(
            Exprs.Function(ctx.Function("sin"), member), ctx, new Dictionary<Symbol, Num>());
        Assert.True(NumOps.Compare(NumOps.Abs(residual, ctx), NumOps.FromReal(Rl.Parse("0." + new string('0', 19) + "1", null))) < 0);
    }

    [Fact]
    public void Solve_SinZero_EngineDisplaysFamily()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        engine.Evaluate("x = symbol(\"x\")");
        var result = engine.Evaluate("solve(sin(x) == 0, x)");
        Assert.Equal("k*pi for integer k", ValueFormatter.Format(result));
    }

    // ------------------------------------------------------------------
    // Integration structured result
    // ------------------------------------------------------------------

    [Fact]
    public void Integrate_StructuredResult_ReportsConditions()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var r = Integration.IntegrateResult(Exprs.Divide(Exprs.One, x), x, ctx);
        Assert.Equal(IntegrationKind.SolvedConditional, r.Kind);
        Assert.Contains(r.Conditions.Atoms, a =>
            a is ExpressionPropertyAssumption { P: SymbolPredicate.Positive } ep && ep.E is SymbolExpr);
        var poly = Integration.IntegrateResult(Exprs.Power(x, 5), x, ctx);
        Assert.Equal(IntegrationKind.SolvedExact, poly.Kind);
        var hard = Integration.IntegrateResult(Exprs.Function(ctx.Function("exp"), Exprs.Power(x, 2)), x, ctx);
        Assert.Equal(IntegrationKind.Unevaluated, hard.Kind);
        Assert.IsType<IntegralExpr>(hard.Expression);
    }
}
