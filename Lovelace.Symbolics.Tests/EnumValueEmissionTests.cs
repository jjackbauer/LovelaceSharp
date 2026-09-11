using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Decision D2 (alignment addendum §9.1, resolving alignment-plan §F against the shipped
/// protocol): every enum-valued field of every rich result record crosses the wire as a
/// first-class <c>Enum</c> value carrying the DECLARED enum type name — never as <c>Text</c> an
/// agent would have to recognise from its spelling.
/// <para>
/// Each assertion names the enum type AND the member, so a rename fails here rather than
/// silently re-spelling the protocol.
/// </para>
/// </summary>
public class EnumValueEmissionTests
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

    /// <summary>The one observable identity of an enum-valued field: the structured kind, the
    /// declared enum type name and the member — plus the two human renderings, which must show
    /// the bare member name (never "EnumValue { ... }").</summary>
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

    // ------------------------------------------------------------------
    // SolveResult / Solution / SolutionFamily
    // ------------------------------------------------------------------

    [Fact]
    public void SolveResult_StatusAndCompleteness_AreEnumValues()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = engine.Evaluate("solve_full(x^2 - 4 == 0, x)").AsRecord();

        AssertEnumField(F(r, "status"), "SolveStatus", "Solved");
        AssertEnumField(F(r, "completeness"), "Completeness", "Complete");
        // and every solution carries its own exactness enum, with the same name as before
        var solution = F(r, "solutions").AsVector()[0].AsRecord();
        AssertEnumField(F(solution, "exactness"), "SolutionExactness", "Exact");
    }

    [Fact]
    public void SolveResult_Partial_CarriesTheSameNamesAsBefore()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = engine.Evaluate("solve_full(x^4 - x^2 - 1 == 0, x)").AsRecord();

        AssertEnumField(F(r, "status"), "SolveStatus", "Partial");
        AssertEnumField(F(r, "completeness"), "Completeness", "Partial");
        // the derived convenience flag next to the Completeness enum stays a Boolean
        Assert.Equal(ValueKind.Boolean, F(r, "complete").Kind);
    }

    [Fact]
    public void SolutionFamily_Exactness_IsAnEnum()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = engine.Evaluate("solve_full(sin(x) == 0, x)").AsRecord();

        var family = F(r, "families").AsVector()[0].AsRecord();
        AssertEnumField(F(family, "exactness"), "SolutionExactness", "ParametricExact");
    }

    [Fact]
    public void SystemSolveResult_StatusAndSolutionExactness_AreEnums()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\"); y = symbol(\"y\")");
        var r = engine.Evaluate("solve_system_full([x + y == 1, x - y == 3], [x, y])").AsRecord();

        AssertEnumField(F(r, "status"), "SolveStatus", "Solved");
        var solution = F(r, "solutions").AsVector()[0].AsRecord();
        AssertEnumField(F(solution, "exactness"), "SolutionExactness", "Exact");
    }

    // ------------------------------------------------------------------
    // LimitResult / IntegrationResult / TransformResult / RewriteStep
    // ------------------------------------------------------------------

    [Fact]
    public void LimitResult_StatusAndExactness_AreEnums()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = engine.Evaluate("limit_full(1/x, x, 0)").AsRecord();

        AssertEnumField(F(r, "status"), "LimitStatus", "DoesNotExist");
        AssertEnumField(F(r, "exactness"), "SolutionExactness", "Exact");
        AssertEnumField(F(engine.Evaluate("limit_full(sin(x)/x, x, 0)").AsRecord(), "status"),
            "LimitStatus", "Value");
    }

    [Fact]
    public void IntegrationResult_StatusAndExactness_AreEnums()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = engine.Evaluate("integrate_full(x^2, x)").AsRecord();

        AssertEnumField(F(r, "status"), "IntegrationStatus", "SolvedExact");
        AssertEnumField(F(r, "exactness"), "SolutionExactness", "Exact");
    }

    [Fact]
    public void TransformResult_StatusAndRewriteStepClassification_AreEnums()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = engine.Evaluate("simplify_full(sin(x)^2 + cos(x)^2)").AsRecord();

        AssertEnumField(F(r, "status"), "TransformStatus", "Satisfied");
        var step = F(r, "steps").AsVector()[0].AsRecord();
        AssertEnumField(F(step, "classification"), "RuleClassification", "Universal");
        // rule_id is a NAME, not a member of an enum: it stays Text
        Assert.Equal(ValueKind.Text, F(step, "rule_id").Kind);
    }

    // ------------------------------------------------------------------
    // The type-name vocabulary
    // ------------------------------------------------------------------

    [Fact]
    public void Type_OfAnEnumValue_IsItsDeclaredEnumTypeName()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        // the same convention a record follows: a consumer switches on the DECLARED type name,
        // never on the transport kind ("Enum")
        Assert.Equal("SolveStatus", engine.Evaluate("type(solve_full(x^2 - 4 == 0, x).status)").AsText());
        Assert.Equal("Completeness", engine.Evaluate("type(solve_full(x^2 - 4 == 0, x).completeness)").AsText());
        Assert.Equal("TransformStatus", engine.Evaluate("type(simplify_full(x/x).status)").AsText());
        Assert.Equal("RuleClassification",
            engine.Evaluate("type(simplify_full(x/x).steps[0].classification)").AsText());
    }
}
