using Lovelace.Abstractions;

namespace Lovelace.Suite;

/// <summary>One declared field of a well-known record.</summary>
/// <param name="Name">The field name exactly as the kernel emits it (snake_case, part of the contract).</param>
/// <param name="Kind">The payload kind a consumer should expect, for docs and tooling.</param>
public sealed record RecordFieldSchema(string Name, string Kind);

/// <summary>The declared shape of a well-known result record.</summary>
public sealed record RecordSchema(string TypeName, IReadOnlyList<RecordFieldSchema> Fields)
{
    /// <summary>Field names this schema requires, in order.</summary>
    public IReadOnlyList<string> FieldNames { get; } = Fields.Select(f => f.Name).ToArray();

    /// <summary>
    /// Checks a record against this schema: every declared field present, and no undeclared field
    /// carried. Returns the violations (empty means conformant) — a value, not an exception, so a
    /// host can report schema drift as a diagnostic.
    /// </summary>
    public IReadOnlyList<string> Validate(RecordValue record)
    {
        var violations = new List<string>();
        if (!string.Equals(record.TypeName, TypeName, StringComparison.Ordinal))
            violations.Add($"expected record '{TypeName}', got '{record.TypeName}'");

        var present = record.Fields.Select(f => f.Name).ToHashSet(StringComparer.Ordinal);
        foreach (var field in Fields)
        {
            if (!present.Contains(field.Name))
                violations.Add($"{TypeName}: missing declared field '{field.Name}'");
        }
        var declared = FieldNames.ToHashSet(StringComparer.Ordinal);
        foreach (var field in record.Fields)
        {
            if (!declared.Contains(field.Name))
                violations.Add($"{TypeName}: undeclared field '{field.Name}'");
        }
        return violations;
    }
}

/// <summary>
/// Statically registered schemas for the well-known result records (alignment-plan §80). No
/// reflection and no assembly scanning: the table is data, so it works under Native AOT and can
/// drive validation, docs, Studio rendering and the DSH schema from one source. A record type with
/// no entry is simply unknown to the registry (<see cref="For"/> returns null) — the registry
/// describes what it knows rather than guessing.
/// </summary>
public static class RecordSchemas
{
    public static IReadOnlyDictionary<string, RecordSchema> All { get; } =
        Build().ToDictionary(s => s.TypeName, StringComparer.Ordinal);

    /// <summary>The schema for a record type name, or null when the registry does not know it.</summary>
    public static RecordSchema? For(string typeName) =>
        All.TryGetValue(typeName, out var s) ? s : null;

    /// <summary>Schema violations for a record of a known type; empty for an unknown type.</summary>
    public static IReadOnlyList<string> Validate(RecordValue record) =>
        For(record.TypeName)?.Validate(record) ?? Array.Empty<string>();

    private static RecordSchema[] Build() => new[]
    {
        Schema("SolveResult",
            ("status", "Enum"), ("variable", "Symbolic"), ("domain", "Domain"), ("complete", "Boolean"),
            ("completeness", "Enum"), ("solutions", "Array"), ("families", "Array"),
            ("common_conditions", "Array"), ("represented_count", "Integer"),
            ("unrepresented_count", "Integer"), ("unrepresented_reason", "Text"), ("diagnostics", "Array")),
        Schema("Solution",
            ("value", "Symbolic"), ("conditions", "Array"), ("multiplicity", "Integer"), ("exactness", "Enum")),
        Schema("SolutionFamily",
            ("template", "Symbolic"), ("parameter", "Symbolic"), ("period", "Symbolic"),
            ("parameter_domain", "Domain"), ("conditions", "Array"), ("exactness", "Enum")),
        Schema("SystemSolveResult",
            ("status", "Enum"), ("domain", "Domain"), ("complete", "Boolean"),
            ("solutions", "Array"), ("diagnostics", "Array")),
        // the two matrix result records (inv_full / linsolve_full): the SAME solve vocabulary as
        // SolveResult/SystemSolveResult — an Enum status, the derived complete Boolean and the
        // Completeness Enum — with the payload in the middle and the Diagnostic array LAST. The
        // "Diagnostic" schema below is reused, never duplicated.
        Schema("MatrixInverseResult",
            ("status", "Enum"), ("complete", "Boolean"), ("completeness", "Enum"),
            ("inverse", "Array"), ("conditions", "Array"), ("diagnostics", "Array")),
        Schema("MatrixSolveResult",
            ("status", "Enum"), ("complete", "Boolean"), ("completeness", "Enum"),
            ("solutions", "Array"), ("conditions", "Array"), ("diagnostics", "Array")),
        Schema("SystemSolution",
            ("bindings", "Array"), ("conditions", "Array"), ("exactness", "Enum")),
        Schema("Binding", ("name", "Symbolic"), ("value", "Symbolic")),
        Schema("TransformResult",
            ("status", "Enum"), ("original", "Symbolic"), ("expression", "Symbolic"), ("changed", "Boolean"),
            ("conditions", "Array"), ("steps", "Array"), ("budget_exceeded", "Boolean"), ("budget_kind", "Text"),
            ("diagnostics", "Array")),
        Schema("RewriteStep",
            ("rule_id", "Text"), ("classification", "Enum"), ("before", "Symbolic"), ("after", "Symbolic"),
            ("required_conditions", "Array")),
        Schema("LimitResult",
            // "exists" is three-valued (N19): Boolean true/false when determined or proven,
            // Null when the engine did not determine it — never false for "unknown".
            ("status", "Enum"), ("exists", "Boolean|Null"), ("value", "Symbolic|Null"), ("left", "Symbolic|Null"),
            ("left_conditions", "Array"), ("right", "Symbolic|Null"), ("right_conditions", "Array"),
            ("conditions", "Array"), ("exactness", "Enum"), ("diagnostics", "Array")),
        Schema("IntegrationResult",
            ("status", "Enum"), ("expression", "Symbolic"), ("conditions", "Array"), ("verified", "Boolean"),
            ("method", "Text"), ("verification_method", "Text"), ("exactness", "Enum"), ("diagnostics", "Array")),
        Schema("OptimizationResult",
            ("original", "Symbolic"), ("optimized", "Symbolic"), ("estimated_cost_before", "Integer"),
            ("estimated_cost_after", "Integer"), ("shared_subtrees", "Integer"), ("horner_rewrites", "Integer"),
            ("transformations", "Array"), ("target", "Text"), ("policy", "Text"), ("diagnostics", "Array")),
        Schema("CompilationResult",
            ("ir", "Text"), ("parameters", "Array"), ("result_domain", "Domain"), ("result_type", "Text"),
            ("precision_policy", "Text"), ("optimization_policy", "Text"), ("target", "Text"),
            ("mathir_version", "Integer"), ("exact", "Boolean"), ("diagnostics", "Array")),
        Schema("ParameterInfo", ("name", "Text"), ("domain", "Domain")),
        // the diagnostic vocabulary every rich record's "diagnostics" array carries (section C):
        // the category is an Enum, the location is a wire record or Null, details is an Array
        Schema("Diagnostic",
            ("code", "Text"), ("category", "Enum"), ("message", "Text"), ("recoverable", "Boolean"),
            ("location", "Record|Null"), ("details", "Array")),
        Schema("Inspection",
            ("type", "Text"), ("domain", "Domain|Null"), ("exact", "Boolean|Null"), ("free_symbols", "Array"),
            ("node_count", "Integer|Null"), ("canonical", "Text|Null"), ("pretty", "Text|Null"),
            ("shape", "Array|Null"), ("rank", "Integer|Null"), ("element_domain", "Domain|Null"),
            ("assumptions", "Array"), ("members", "Array|Null")),
        Schema("DomainCondition", ("variable", "Symbolic"), ("domain", "Domain")),
        Schema("FiniteCondition", ("expression", "Symbolic")),
        Schema("PredicateCondition", ("predicate", "Text"), ("expression", "Symbolic")),
        Schema("IntervalCondition",
            ("expression", "Symbolic"), ("lower", "Symbolic|Null"), ("lower_open", "Boolean"),
            ("upper", "Symbolic|Null"), ("upper_open", "Boolean")),
    };

    private static RecordSchema Schema(string typeName, params (string Name, string Kind)[] fields) =>
        new(typeName, fields.Select(f => new RecordFieldSchema(f.Name, f.Kind)).ToArray());
}
