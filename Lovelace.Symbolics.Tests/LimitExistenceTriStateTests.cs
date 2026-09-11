using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// N19 (round 34): the `exists` field of `limit_full`'s LimitResult is THREE-VALUED.
/// <c>true</c> only when a limit was determined, <c>false</c> only when non-existence was
/// PROVEN (<see cref="LimitStatus.DoesNotExist"/>), and the protocol Null (`{kind:Null}`,
/// carried as `Value.Void`) when the engine did not determine it (Unevaluated/Failed).
/// "Not determined" must never be reported as "does not exist".
/// </summary>
public class LimitExistenceTriStateTests
{
    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    private static Value F(RecordValue rec, string name) =>
        (Value)rec.Fields.First(f => f.Name == name).Value!;

    private static RecordValue Full(SuiteEngine engine, string call) => engine.Evaluate(call).AsRecord();

    private static void AssertStatus(RecordValue r, string member) =>
        Assert.Equal(member, F(r, "status").AsEnum().Name);

    /// <summary>Undetermined: the squeezed limit x·sin(1/x) → 0 that the engine cannot solve.
    /// The engine may say Unevaluated; it may not say "does not exist".</summary>
    [Fact]
    public void UndeterminedLimit_ReportsNullExists_NotFalse()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = Full(engine, "limit_full(x*sin(1/x), x, 0)");

        AssertStatus(r, "Unevaluated");                       // status unchanged
        Assert.Equal(ValueKind.Void, F(r, "exists").Kind);    // NULL, never false
        Assert.Equal("Null", StructuredProjection.ToStructured(F(r, "exists")).Kind);
        Assert.NotEqual(ValueKind.Boolean, F(r, "exists").Kind);
    }

    /// <summary>A determined limit is unaffected: exists is the Boolean true and the value is 1.</summary>
    [Fact]
    public void DeterminedLimit_StillReportsTrueExistsAndValue()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = Full(engine, "limit_full(sin(x)/x, x, 0)");

        AssertStatus(r, "Value");
        Assert.Equal(ValueKind.Boolean, F(r, "exists").Kind);
        Assert.True(F(r, "exists").AsBoolean());
        Assert.Equal("1", ValueFormatter.Format(F(r, "value")));
    }

    /// <summary>Proven non-existence is still a Boolean false — the true case must not be lost.
    /// 1/x at 0 is an odd pole: −∞ from the left, +∞ from the right.</summary>
    [Fact]
    public void ProvenNonExistence_StillReportsFalseExists()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = Full(engine, "limit_full(1/x, x, 0)");

        AssertStatus(r, "DoesNotExist");
        Assert.Equal(ValueKind.Boolean, F(r, "exists").Kind);
        Assert.False(F(r, "exists").AsBoolean());
        // the sides that PROVE the disagreement are still exposed
        Assert.Equal("-inf", ValueFormatter.Format(F(r, "left")));
        Assert.Equal("inf", ValueFormatter.Format(F(r, "right")));
    }

    /// <summary>One-sided fields for an undetermined side: nothing is asserted. An undetermined
    /// side is Null (not a fabricated value) and carries no condition — an empty array, which is
    /// an array, not Null and not "".</summary>
    [Fact]
    public void UndeterminedSides_AreNullWithNoConditions()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = Full(engine, "limit_full(x*sin(1/x), x, 0)");

        Assert.Equal(ValueKind.Void, F(r, "left").Kind);
        Assert.Equal(ValueKind.Void, F(r, "right").Kind);
        Assert.Equal("Null", StructuredProjection.ToStructured(F(r, "left")).Kind);
        foreach (var name in new[] { "left_conditions", "right_conditions" })
        {
            var field = F(r, name);
            Assert.True(field.Kind is ValueKind.Vector or ValueKind.Array);
            Assert.Empty(field.AsVector());
        }
    }

    /// <summary>The one-sided builtins are equally honest for an undetermined side, and a
    /// determined side still reports its value.</summary>
    [Fact]
    public void UndeterminedOneSidedLimits_ReportNoValue()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        Assert.StartsWith("unevaluated", engine.Evaluate("limit_left(x*sin(1/x), x, 0)").AsText());
        Assert.StartsWith("unevaluated", engine.Evaluate("limit_right(x*sin(1/x), x, 0)").AsText());
        Assert.Equal("-inf", ValueFormatter.Format(engine.Evaluate("limit_left(1/x, x, 0)")));
        Assert.Equal("inf", ValueFormatter.Format(engine.Evaluate("limit_right(1/x, x, 0)")));
    }

    /// <summary>Kernel level: an undetermined one-sided result is Unevaluated with NO value —
    /// nothing to mistake for a claimed limit.</summary>
    [Fact]
    public void KernelUndeterminedSide_HasNoValue()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
        var f = Exprs.Multiply(x, Exprs.Function(ctx.Function("sin"), Exprs.Divide(Exprs.One, x)));

        foreach (var direction in new[] { LimitDirection.TwoSided, LimitDirection.FromLeft, LimitDirection.FromRight })
        {
            var r = Limits.Limit(f, x, Exprs.Zero, direction, ctx);
            Assert.Equal(LimitStatus.Unevaluated, r.Status);
            Assert.Null(r.Value);
            Assert.Null(r.FromLeft);
            Assert.Null(r.FromRight);
        }
    }
}
