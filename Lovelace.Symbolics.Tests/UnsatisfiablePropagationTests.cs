using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Unsatisfiable-condition propagation (§21) and the structured condition payload (Cycle 2
/// item 2). A condition set with no model must be reported as such — never as a live branch,
/// and never as the string "unsatisfiable".
/// </summary>
public class UnsatisfiablePropagationTests
{
    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    // ------------------------------------------------------------------
    // Kernel: a rule requirement the active assumptions refute
    // ------------------------------------------------------------------

    [Fact]
    public void Transform_RequirementRefutedByAssumptions_IsUnsatisfiable()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
        // x/x may only be cancelled when x != 0 — which this set refutes
        ctx.Assumptions = AssumptionSet.Empty.Add(new SymbolRelationAssumption(x, RelOp.Eq, Exprs.Zero));

        var r = Simplify.Transform(Exprs.Divide(x, x), ctx);

        Assert.True(r.Conditions.IsUnsatisfiable,
            $"expected an unsatisfiable branch, got [{string.Join(" | ", r.Conditions.Atoms)}]");
    }

    [Fact]
    public void Transform_RequirementConsistentWithAssumptions_KeepsItsConditions()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
        ctx.Assumptions = AssumptionSet.Empty.Add(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Integer(5)));

        var r = Simplify.Transform(Exprs.Divide(x, x), ctx);

        Assert.False(r.Conditions.IsUnsatisfiable);
        Assert.Contains(r.Conditions.Atoms, a => a is SymbolPropertyAssumption { P: SymbolPredicate.NonZero });
    }

    // ------------------------------------------------------------------
    // Language surface: the payload is structured, never a string
    // ------------------------------------------------------------------

    [Fact]
    public void SimplifyFull_UnsatisfiableConditions_SerializesAsAnExplicitMarker()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        engine.Evaluate("assume(x == 0)");

        var record = engine.Evaluate("simplify_full(x/x)").AsRecord();
        var conditions = (Value)record.Fields[4].Value!;

        Assert.Equal(ValueKind.Vector, conditions.Kind);
        var leaf = conditions.AsVector()[0];
        Assert.Equal(ValueKind.Record, leaf.Kind);
        Assert.Equal("UnsatisfiableConditions", leaf.AsRecord().TypeName);
        Assert.Empty(leaf.AsRecord().Fields);
    }

    [Fact]
    public void SimplifyFull_DomainCondition_CarriesADomainValueNotText()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\", real)");

        var record = engine.Evaluate("simplify_full(sqrt(x^2))").AsRecord();
        var conditions = (Value)record.Fields[4].Value!;
        var leaf = conditions.AsVector()[0];

        Assert.Equal(ValueKind.Record, leaf.Kind);
        var condition = leaf.AsRecord();
        Assert.Equal("DomainCondition", condition.TypeName);
        Assert.Equal("variable", condition.Fields[0].Name);
        Assert.Equal(ValueKind.Symbolic, ((Value)condition.Fields[0].Value!).Kind);
        Assert.Equal("domain", condition.Fields[1].Name);
        Assert.Equal(ValueKind.Domain, ((Value)condition.Fields[1].Value!).Kind);
        Assert.Equal(MathDomain.Real, ((Value)condition.Fields[1].Value!).AsDomain());
        // the raw C# record rendering must not appear anywhere in the display form
        Assert.DoesNotContain("SymbolDomainAssumption", ValueFormatter.FormatTyped(conditions));
    }

    [Fact]
    public void ConditionPayload_IsStructuredForEveryReachableAtomKind()
    {
        static Value Conditions(Value result) => (Value)result.AsRecord().Fields[4].Value!;

        // a property atom projects to its relation form — a Symbolic value, not text
        var property = NewEngine();
        property.Evaluate("x = symbol(\"x\")");
        property.Evaluate("assume_positive(x)");
        var relationLeaf = Conditions(property.Evaluate("simplify_full(x/x)")).AsVector()[0];
        Assert.Equal(ValueKind.Symbolic, relationLeaf.Kind);

        // a predicate with no relation form projects to a predicate record
        var finite = NewEngine();
        finite.Evaluate("x = symbol(\"x\")");
        var finiteLeaf = Conditions(finite.Evaluate("simplify_full(exp(log(x)))")).AsVector()[0];
        Assert.Equal(ValueKind.Record, finiteLeaf.Kind);
        Assert.Equal("FiniteCondition", finiteLeaf.AsRecord().TypeName);

        // and no leaf anywhere renders as raw C# record syntax
        foreach (var leaf in new[] { relationLeaf, finiteLeaf })
            Assert.DoesNotContain("Assumption", ValueFormatter.Format(leaf));
    }
}
