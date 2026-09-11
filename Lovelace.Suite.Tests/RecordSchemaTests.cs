using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// The record schema registry (Cycle 2 item 22) must describe the records the kernel ACTUALLY
/// produces. A schema table nobody checks is documentation, and documentation drifts — this test
/// validates every record a representative corpus produces, so a renamed, added or dropped field
/// fails the build instead of silently diverging from the declared shape.
/// </summary>
public class RecordSchemaTests
{
    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    private static void Collect(Value value, List<RecordValue> into, int depth = 0)
    {
        if (depth > 6) return;
        switch (value.Kind)
        {
            case ValueKind.Record:
                var record = value.AsRecord();
                into.Add(record);
                foreach (var f in record.Fields)
                    if (f.Value is Value inner) Collect(inner, into, depth + 1);
                break;
            case ValueKind.Vector or ValueKind.Array:
                foreach (var element in value.AsVector()) Collect(element, into, depth + 1);
                break;
        }
    }

    [Fact]
    public void EveryProducedRecord_ConformsToItsDeclaredSchema()
    {
        var engine = NewEngine();
        var scripts = new[]
        {
            "x = symbol(\"x\"); solve_full(x^2 - 4 == 0, x)",
            "x = symbol(\"x\"); solve_full(sin(x) == 0, x)",
            "x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x + y == 1, x - y == 3], [x, y])",
            // the non-full call form publishes the SAME record type (the frozen contract declares
            // one), so the corpus drives it too: a record only one of the two forms produces would
            // leave half the contract unvalidated
            "x = symbol(\"x\"); y = symbol(\"y\"); solve_system([x + y == 1, x - y == 3], [x, y])",
            "x = symbol(\"x\"); simplify_full(x/x)",
            "x = symbol(\"x\", real); simplify_full(sqrt(x^2))",
            // a PARTIAL solve and a refuted transformation are the two runs that carry a
            // structured Diagnostic, so the corpus exercises the "Diagnostic" schema too
            "x = symbol(\"x\"); solve_full(x^4 - x^2 - 1 == 0, x)",
            "x = symbol(\"x\"); assume(x == 0); simplify_full(x/x)",
            "x = symbol(\"x\"); limit_full(1/x, x, 0)",
            "x = symbol(\"x\"); integrate_full(x^2, x)",
            "x = symbol(\"x\"); optimize_full(x^2 + 2*x + 1, [x])",
            "x = symbol(\"x\"); compile_full(x^2 + 1, [x])",
            "x = symbol(\"x\"); inspect(x^2 + 1)",
            // Round 6: neither matrix record was ever produced by this corpus, which is exactly why
            // the registry could drift unnoticed. A singular AND a solved run of each is now a row.
            "x = symbol(\"x\"); y = symbol(\"y\"); inv_full([[x, 1], [0, y]])",
            "x = symbol(\"x\"); inv_full([[x, x], [x, x]])",
            "x = symbol(\"x\"); y = symbol(\"y\"); linsolve_full([[x, 1], [0, y]], [0, 1])",
            "x = symbol(\"x\"); linsolve_full([[x, x], [x, x]], [1, 1])",
        };

        var records = new List<RecordValue>();
        foreach (var script in scripts)
            Collect(engine.Evaluate(script), records);

        Assert.NotEmpty(records);
        var violations = new List<string>();
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (var record in records)
        {
            seen.Add(record.TypeName);
            violations.AddRange(RecordSchemas.Validate(record));
        }

        Assert.True(violations.Count == 0,
            "schema drift:\n  " + string.Join("\n  ", violations) + "\nrecord types seen: " + string.Join(", ", seen));
        // the corpus must actually exercise the schemas, not pass because nothing was compared
        Assert.Contains("SolveResult", seen);
        Assert.Contains("Solution", seen);
        Assert.Contains("SolutionFamily", seen);
        Assert.Contains("SystemSolveResult", seen);
        Assert.Contains("SystemSolution", seen);
        // the structural binding record must actually be produced by the corpus: an unregistered
        // record type is invisible to this test, so the corpus asserts it was seen
        Assert.Contains("Binding", seen);
        Assert.Contains("TransformResult", seen);
        Assert.Contains("RewriteStep", seen);
        Assert.Contains("LimitResult", seen);
        Assert.Contains("IntegrationResult", seen);
        Assert.Contains("OptimizationResult", seen);
        Assert.Contains("CompilationResult", seen);
        Assert.Contains("ParameterInfo", seen);
        Assert.Contains("Inspection", seen);
        // the two matrix records join the registry in Round 6. Asserting the SEEN set (not just the
        // schema table) is what proves the corpus actually drove inv_full/linsolve_full: a schema
        // entry nothing produces is documentation, and this is the drift the round closes.
        Assert.Contains("MatrixInverseResult", seen);
        Assert.Contains("MatrixSolveResult", seen);
        // the diagnostic vocabulary is part of the same registry: a Diagnostic record that the
        // corpus did not actually produce would make the Diagnostic schema untested
        Assert.Contains("Diagnostic", seen);
    }

    [Fact]
    public void UnknownRecordType_IsReportedAsUnknown_NotAsConforming()
    {
        var record = new RecordValue("NotARealResult", new RecordField("field", "value"));
        Assert.Null(RecordSchemas.For("NotARealResult"));
        Assert.Empty(RecordSchemas.Validate(record));   // unknown: nothing claimed, nothing violated
    }

    [Fact]
    public void Validate_ReportsMissingAndUndeclaredFields()
    {
        var schema = RecordSchemas.For("ParameterInfo");
        Assert.NotNull(schema);
        var wrong = new RecordValue("ParameterInfo", new RecordField("name", "x"), new RecordField("extra", 1L));
        var violations = schema!.Validate(wrong);
        Assert.Contains(violations, v => v.Contains("missing declared field 'domain'"));
        Assert.Contains(violations, v => v.Contains("undeclared field 'extra'"));
    }

    /// <summary>
    /// The declared KIND of every solve field is the machine contract a consumer reads: the
    /// solver carries structure (a Binding record per assignment, a Symbol for the variable and
    /// the family parameter), never a flattened string. Field order is pinned too — the registry
    /// is the documentation of the wire order as well as the wire names.
    /// <para>
    /// An enum-valued field is declared <c>Enum</c>, never <c>Text</c>: decision D2
    /// (alignment addendum §9.1) makes the enum a first-class structured kind, so no consumer has
    /// to recognise <c>Solved</c>/<c>Partial</c>/<c>Exact</c> from its spelling.
    /// </para>
    /// </summary>
    [Fact]
    public void SolverSchemas_DeclareTheStructuralKinds()
    {
        AssertSchema("SystemSolveResult",
            ("status", "Enum"), ("domain", "Domain"), ("complete", "Boolean"), ("completeness", "Enum"),
            ("solutions", "Array"), ("diagnostics", "Array"));
        AssertSchema("SystemSolution",
            ("bindings", "Array"), ("conditions", "Array"), ("exactness", "Enum"));
        AssertSchema("Binding", ("name", "Symbolic"), ("value", "Symbolic"));
        AssertSchema("SolveResult",
            ("status", "Enum"), ("variable", "Symbolic"), ("domain", "Domain"), ("complete", "Boolean"),
            ("completeness", "Enum"), ("solutions", "Array"), ("families", "Array"),
            ("common_conditions", "Array"), ("represented_count", "Integer"),
            ("unrepresented_count", "Integer"), ("unrepresented_reason", "Text"), ("diagnostics", "Array"));
        AssertSchema("SolutionFamily",
            ("template", "Symbolic"), ("parameter", "Symbolic"), ("period", "Symbolic"),
            ("parameter_domain", "Domain"), ("conditions", "Array"), ("exactness", "Enum"));
        AssertSchema("Solution",
            ("value", "Symbolic"), ("conditions", "Array"), ("multiplicity", "Integer"), ("exactness", "Enum"));
        AssertSchema("TransformResult",
            ("status", "Enum"), ("original", "Symbolic"), ("expression", "Symbolic"), ("changed", "Boolean"),
            ("conditions", "Array"), ("steps", "Array"), ("budget_exceeded", "Boolean"), ("budget_kind", "Text"),
            ("diagnostics", "Array"));
        AssertSchema("RewriteStep",
            ("rule_id", "Text"), ("classification", "Enum"), ("before", "Symbolic"), ("after", "Symbolic"),
            ("required_conditions", "Array"));
        AssertSchema("LimitResult",
            ("status", "Enum"), ("exists", "Boolean|Null"), ("value", "Symbolic|Null"), ("left", "Symbolic|Null"),
            ("left_conditions", "Array"), ("right", "Symbolic|Null"), ("right_conditions", "Array"),
            ("conditions", "Array"), ("exactness", "Enum"), ("diagnostics", "Array"));
        AssertSchema("IntegrationResult",
            ("status", "Enum"), ("expression", "Symbolic"), ("conditions", "Array"), ("verified", "Boolean"),
            ("method", "Text"), ("verification_method", "Text"), ("exactness", "Enum"), ("diagnostics", "Array"));
        AssertSchema("OptimizationResult",
            ("original", "Symbolic"), ("optimized", "Symbolic"), ("estimated_cost_before", "Integer"),
            ("estimated_cost_after", "Integer"), ("shared_subtrees", "Integer"), ("horner_rewrites", "Integer"),
            ("transformations", "Array"), ("target", "Text"), ("policy", "Text"), ("diagnostics", "Array"));
        AssertSchema("CompilationResult",
            ("ir", "Text"), ("parameters", "Array"), ("result_domain", "Domain"), ("result_type", "Text"),
            ("precision_policy", "Text"), ("optimization_policy", "Text"), ("target", "Text"),
            ("mathir_version", "Integer"), ("exact", "Boolean"), ("diagnostics", "Array"));
        // the two matrix records: the same status/complete/completeness vocabulary as every other
        // solve record, the payload in the middle, and diagnostics LAST as an Array (never Text)
        AssertSchema("MatrixInverseResult",
            ("status", "Enum"), ("complete", "Boolean"), ("completeness", "Enum"),
            ("inverse", "Array"), ("conditions", "Array"), ("diagnostics", "Array"));
        AssertSchema("MatrixSolveResult",
            ("status", "Enum"), ("complete", "Boolean"), ("completeness", "Enum"),
            ("solutions", "Array"), ("conditions", "Array"), ("diagnostics", "Array"));
    }

    /// <summary>
    /// The Diagnostic form (alignment plan section C): exactly six fields, in this order.
    /// <c>category</c> is an Enum (the ErrorCategory member travels with its declared type name),
    /// <c>location</c> is a wire location record or Null — never an empty string — and
    /// <c>details</c> is an Array, never Null.
    /// </summary>
    [Fact]
    public void DiagnosticSchema_DeclaresTheSixFrozenFields()
    {
        AssertSchema("Diagnostic",
            ("code", "Text"), ("category", "Enum"), ("message", "Text"), ("recoverable", "Boolean"),
            ("location", "Record|Null"), ("details", "Array"));
    }

    // ------------------------------------------------------------------
    // Round 6: the two matrix result records join the solve vocabulary
    // ------------------------------------------------------------------

    private static Value Field(RecordValue record, string name) =>
        (Value)record.Fields.First(f => f.Name == name).Value!;

    /// <summary>
    /// A singular matrix is a PROVABLY empty solution set, which is a complete answer: the record
    /// must say so in the same vocabulary every other solve record uses — an Enum status of the
    /// declared type SolveStatus, the derived Boolean, the Completeness enum — and it must carry
    /// the reason as ONE structured Diagnostic (stable code, ErrorCategory member, message,
    /// recoverable, Null location, empty details) instead of a prose string.
    /// </summary>
    [Theory]
    [InlineData("inv_full([[x, x], [x, x]])", "MatrixInverseResult", "inverse")]
    [InlineData("linsolve_full([[x, x], [x, x]], [1, 1])", "MatrixSolveResult", "solutions")]
    public void SingularMatrix_IsACompleteNoSolutions_CarryingAStructuredDiagnostic(
        string call, string typeName, string payloadField)
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        var record = engine.Evaluate(call).AsRecord();

        Assert.Equal(typeName, record.TypeName);
        // the registry must KNOW the type: Validate() returns empty for an unknown type, so
        // without this assertion "zero violations" would pass vacuously on an unregistered record
        Assert.NotNull(RecordSchemas.For(record.TypeName));
        Assert.Empty(RecordSchemas.Validate(record));

        // the vocabulary: Enum + the derived Boolean + Enum, never a text status
        Value status = Field(record, "status");
        Assert.Equal(ValueKind.Enum, status.Kind);
        Assert.Equal("SolveStatus", status.AsEnum().TypeName);
        Assert.Equal("NoSolutions", status.AsEnum().Name);
        Assert.True(Field(record, "complete").AsBoolean(),
            $"{call}: a provably empty solution set is a complete answer");
        Value completeness = Field(record, "completeness");
        Assert.Equal(ValueKind.Enum, completeness.Kind);
        Assert.Equal("Completeness", completeness.AsEnum().TypeName);
        Assert.Equal("Complete", completeness.AsEnum().Name);

        // the payload field stays an Array in the middle of the record, conditions after it
        Assert.True(Field(record, payloadField).Kind is ValueKind.Vector or ValueKind.Array);
        Assert.Equal("Array", StructuredProjection.ToStructured(Field(record, payloadField)).Kind);
        Assert.Equal("Array", StructuredProjection.ToStructured(Field(record, "conditions")).Kind);

        // diagnostics: an ARRAY with exactly one Diagnostic — never the prose "matrix is singular"
        Value diagnostics = Field(record, "diagnostics");
        Assert.True(diagnostics.Kind is ValueKind.Vector or ValueKind.Array,
            $"{call}: diagnostics must be an array, got {diagnostics.Kind}");
        var wireDiagnostics = StructuredProjection.ToStructured(diagnostics);
        Assert.Equal("Array", wireDiagnostics.Kind);
        Assert.Null(wireDiagnostics.Value);                  // never a free-text diagnostics value
        Assert.NotNull(wireDiagnostics.Elements);
        var element = Assert.Single(wireDiagnostics.Elements!);
        Assert.Equal("Record", element.Kind);

        RecordValue diagnostic = Assert.Single(diagnostics.AsVector()).AsRecord();
        Assert.Equal(DiagnosticProjection.TypeName, diagnostic.TypeName);   // the ONE Round-4b vocabulary
        Assert.NotNull(RecordSchemas.For(diagnostic.TypeName));             // reused, not duplicated
        Assert.Empty(RecordSchemas.Validate(diagnostic));                   // the ONE "Diagnostic" schema

        Assert.Equal("matrix.singular", Field(diagnostic, "code").AsText());
        Value category = Field(diagnostic, "category");
        Assert.Equal(ValueKind.Enum, category.Kind);
        Assert.Equal("ErrorCategory", category.AsEnum().TypeName);
        Assert.Equal(nameof(ErrorCategory.NoSolution), category.AsEnum().Name);
        Assert.False(string.IsNullOrWhiteSpace(Field(diagnostic, "message").AsText()),
            "the human sentence moves INTO the diagnostic message");
        Assert.True(Field(diagnostic, "recoverable").AsBoolean());
        Assert.Equal(ValueKind.Void, Field(diagnostic, "location").Kind);   // Null, never ""
        Assert.Equal("Null", StructuredProjection.ToStructured(Field(diagnostic, "location")).Kind);
        Assert.Empty(Field(diagnostic, "details").AsVector());
        Assert.Empty(StructuredProjection.ToStructured(Field(diagnostic, "details")).Elements!);
    }

    /// <summary>
    /// A solved matrix is the mirror image: Solved / complete true / Completeness.Complete and an
    /// EMPTY diagnostics array — an Array with zero elements, not Null and not the empty string.
    /// </summary>
    [Theory]
    [InlineData("inv_full([[x, 1], [0, y]])", "MatrixInverseResult", "inverse")]
    [InlineData("linsolve_full([[x, 1], [0, y]], [0, 1])", "MatrixSolveResult", "solutions")]
    public void SolvedMatrix_IsComplete_WithAnEmptyDiagnosticsArray(string call, string typeName, string payloadField)
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\"); y = symbol(\"y\")");

        var record = engine.Evaluate(call).AsRecord();

        Assert.Equal(typeName, record.TypeName);
        Assert.NotNull(RecordSchemas.For(record.TypeName));
        Assert.Empty(RecordSchemas.Validate(record));

        Value status = Field(record, "status");
        Assert.Equal(ValueKind.Enum, status.Kind);
        Assert.Equal("SolveStatus", status.AsEnum().TypeName);
        Assert.Equal("Solved", status.AsEnum().Name);
        Assert.True(Field(record, "complete").AsBoolean());
        Assert.Equal("Completeness", Field(record, "completeness").AsEnum().TypeName);
        Assert.Equal("Complete", Field(record, "completeness").AsEnum().Name);

        Assert.True(Field(record, payloadField).Kind is ValueKind.Vector or ValueKind.Array);
        Assert.Equal("Array", StructuredProjection.ToStructured(Field(record, "conditions")).Kind);

        // zero elements: an Array, NOT Null, NOT ""
        Value diagnostics = Field(record, "diagnostics");
        Assert.True(diagnostics.Kind is ValueKind.Vector or ValueKind.Array,
            $"{call}: an empty diagnostics field is still an array, got {diagnostics.Kind}");
        Assert.Empty(diagnostics.AsVector());
        var wire = StructuredProjection.ToStructured(diagnostics);
        Assert.Equal("Array", wire.Kind);
        Assert.Null(wire.Value);
        Assert.NotNull(wire.Elements);
        Assert.Empty(wire.Elements!);
    }

    private static void AssertSchema(string typeName, params (string Name, string Kind)[] expected)
    {
        var schema = RecordSchemas.For(typeName);
        Assert.NotNull(schema);
        Assert.Equal(expected.Select(f => f.Name).ToArray(), schema!.Fields.Select(f => f.Name).ToArray());
        Assert.Equal(expected.Select(f => f.Kind).ToArray(), schema.Fields.Select(f => f.Kind).ToArray());
    }
}
