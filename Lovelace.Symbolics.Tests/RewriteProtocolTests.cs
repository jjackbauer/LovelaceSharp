using Lovelace.Symbolics.Rewriting;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Rewrite-engine failure semantics: applicability is a value (never an exception), a defect in a
/// rule surfaces instead of being reported as "not applicable", conditions encoded in a rule's
/// precondition are declared and traced, and the budget counts applied rewrites.
/// </summary>
public class RewriteProtocolTests
{
    private sealed class BoomException : Exception;

    private static readonly Pattern Any = new WildPat("a");

    private static RewriteRule ThrowingPrecondition() => new(
        "test.boom-precondition",
        "test",
        new FunPat("boom", new Pattern[] { Any }),
        (Func<Match, ExprContext, RuleApplicability>)((m, c) => throw new BoomException()),
        (m, c) => Exprs.Zero);

    [Fact]
    public void ThrowingPrecondition_Propagates()
    {
        var ctx = new ExprContext();
        var f = Exprs.Function(ctx.Function("boom"), Exprs.Symbol(ctx.Symbol("x")));
        // a rule defect is a defect: it must NOT be reported as "not applicable"
        Assert.Throws<BoomException>(() =>
            RewriteEngine.Apply(f, ctx, new[] { ThrowingPrecondition() }));
    }

    [Fact]
    public void RuleEvaluationException_IsTheOnlyCaughtState()
    {
        var ctx = new ExprContext();
        var f = Exprs.Function(ctx.Function("undecidable"), Exprs.Symbol(ctx.Symbol("x")));
        var rule = new RewriteRule(
            "test.undecidable",
            "test",
            new FunPat("undecidable", new Pattern[] { Any }),
            (Func<Match, ExprContext, RuleApplicability>)((m, c) => throw new RuleEvaluationException("cannot decide here")),
            (m, c) => Exprs.Zero);
        var diagnostics = new List<RuleAttemptDiagnostic>();
        var result = RewriteEngine.Apply(f, ctx, new[] { rule }, null, null, null, diagnostics);
        Assert.Equal(f, result);                       // unchanged
        var attempt = Assert.Single(diagnostics, d => d.PatternMatched);
        Assert.Equal(RuleApplicability.Unknown, attempt.Precondition);
    }

    [Fact]
    public void UnknownApplicability_NeverFires()
    {
        var ctx = new ExprContext();
        var x = ctx.Symbol("x");
        var f = Exprs.Function(ctx.Function("maybe"), Exprs.Symbol(x));
        var rule = new RewriteRule("test.unknown", "test",
            new FunPat("maybe", new Pattern[] { Any }),
            (m, c) => RuleApplicability.Unknown,
            (m, c) => Exprs.Zero);
        Assert.Equal(f, RewriteEngine.Apply(f, ctx, new[] { rule }));
    }

    [Fact]
    public void ConditionalRule_CarriesDeclaredConditions()
    {
        var ctx = new ExprContext();
        var x = ctx.Symbol("x");
        var result = Simplify.Transform(Exprs.Divide(Exprs.Symbol(x), Exprs.Symbol(x)), ctx,
            new Simplify.Options(Trace: true));
        Assert.Equal(Printing.CanonicalPrint(Exprs.One), Printing.CanonicalPrint(result.Expression));
        Assert.Contains(result.Conditions.Atoms,
            a => a is ExpressionPropertyAssumption or SymbolPropertyAssumption);
        Assert.Single(result.Steps);
        Assert.NotEmpty(result.Steps[0].Conditions.Atoms);
        Assert.Equal("rat.cancel-x-over-x", result.Steps[0].RuleId);
    }

    [Fact]
    public void ConditionalRule_DoesNotFireOnUnknown_ButFiresWhenProven()
    {
        var ctx = new ExprContext();
        var x = ctx.Symbol("x");
        var expr = Exprs.Divide(Exprs.Symbol(x), Exprs.Symbol(x));
        // no assumption: safe mode refuses, so the expression is unchanged
        Assert.NotEqual(Printing.CanonicalPrint(Exprs.One), Printing.CanonicalPrint(Simplify.SimplifyExpr(expr, ctx)));
        // with x != 0 assumed, it fires
        var withAssumption = new ExprContext();
        var y = withAssumption.Symbol("y");
        withAssumption.Assumptions = AssumptionSet.Empty.Add(new SymbolPropertyAssumption(y, SymbolPredicate.NonZero));
        var exprY = Exprs.Divide(Exprs.Symbol(y), Exprs.Symbol(y));
        Assert.Equal(Printing.CanonicalPrint(Exprs.One), Printing.CanonicalPrint(Simplify.SimplifyExpr(exprY, withAssumption)));
    }

    [Fact]
    public void DomainSpecificRule_DeclaresItsCondition()
    {
        var ctx = new ExprContext();
        var x = ctx.Symbol("x");
        ctx.Assumptions = AssumptionSet.Empty.Add(new SymbolDomainAssumption(x, Domain.Real));
        var transform = Simplify.Transform(Exprs.Power(Exprs.Power(Exprs.Symbol(x), Exprs.Integer(2)), Exprs.Rational(1, 2)), ctx,
            new Simplify.Options(Trace: true));
        var step = Assert.Single(transform.Steps);
        Assert.Equal("pow.sqrt-square-real", step.RuleId);
        Assert.Equal(RuleClassification.DomainSpecific, step.Classification);
        // the domain condition is part of the provenance, not hidden in a lambda
        Assert.NotEmpty(step.Conditions.Atoms);
    }

    [Fact]
    public void Budget_CountsAppliedRewrites_AndStopsFurtherGroups()
    {
        var ctx = new ExprContext();
        var x = ctx.Symbol("x");
        var transform = Simplify.Transform(Exprs.Divide(Exprs.Symbol(x), Exprs.Symbol(x)), ctx,
            new Simplify.Options(MaxSteps: 0));
        Assert.True(transform.BudgetExceeded);
        Assert.Equal("BudgetExceeded", transform.Status);
        Assert.Equal("rewrite_steps", transform.BudgetKind);
        Assert.Empty(transform.Steps);
    }

    [Fact]
    public void TraceDisabled_KeepsConditionsAndAllocatesNoSteps()
    {
        var ctx = new ExprContext();
        var x = ctx.Symbol("x");
        var transform = Simplify.Transform(Exprs.Divide(Exprs.Symbol(x), Exprs.Symbol(x)), ctx);
        Assert.Empty(transform.Steps);
        Assert.NotEmpty(transform.Conditions.Atoms);
        Assert.Equal(Printing.CanonicalPrint(Exprs.One), Printing.CanonicalPrint(transform.Expression));
    }

    [Fact]
    public void EveryRule_HasAStableUniqueId_AndUniversalRulesAreUnconditional()
    {
        var ctx = new ExprContext();
        var rules = Simplify.RulesForTesting(ctx);
        var ids = rules.Select(r => r.Id).ToArray();
        Assert.Equal(ids.Length, ids.Distinct(StringComparer.Ordinal).Count());
        foreach (var rule in rules.Where(r => r.Classification == RuleClassification.Universal))
        {
            Assert.True(rule.DeclaredConditions.Atoms.Length == 0,
                $"universal rule {rule.Id} must not declare conditions");
            Assert.Null(rule.ConditionBuilder);
        }
    }
}
