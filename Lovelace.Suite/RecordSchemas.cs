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
            ("status", "Text"), ("variable", "Text"), ("domain", "Domain"), ("complete", "Boolean"),
            ("completeness", "Text"), ("solutions", "Array"), ("families", "Array"),
            ("common_conditions", "Array"), ("represented_count", "Integer"),
            ("unrepresented_count", "Integer"), ("unrepresented_reason", "Text"), ("diagnostics", "Text")),
        Schema("Solution",
            ("value", "Symbolic"), ("conditions", "Array"), ("multiplicity", "Integer"), ("exactness", "Text")),
        Schema("SolutionFamily",
            ("template", "Symbolic"), ("parameter", "Text"), ("period", "Symbolic"),
            ("parameter_domain", "Domain"), ("conditions", "Array"), ("exactness", "Text")),
        Schema("SystemSolveResult", ("status", "Text"), ("solutions", "Array"), ("diagnostics", "Text")),
        Schema("SystemSolution", ("assignment", "Array"), ("conditions", "Array")),
        Schema("TransformResult",
            ("status", "Text"), ("original", "Symbolic"), ("expression", "Symbolic"), ("changed", "Boolean"),
            ("conditions", "Array"), ("steps", "Array"), ("budget_exceeded", "Boolean"), ("budget_kind", "Text")),
        Schema("RewriteStep",
            ("rule_id", "Text"), ("classification", "Text"), ("before", "Symbolic"), ("after", "Symbolic"),
            ("required_conditions", "Array")),
        Schema("LimitResult",
            ("status", "Text"), ("exists", "Boolean"), ("value", "Symbolic|Null"), ("left", "Symbolic|Null"),
            ("left_conditions", "Array"), ("right", "Symbolic|Null"), ("right_conditions", "Array"),
            ("conditions", "Array"), ("exactness", "Text"), ("diagnostics", "Text")),
        Schema("IntegrationResult",
            ("status", "Text"), ("expression", "Symbolic"), ("conditions", "Array"), ("verified", "Boolean"),
            ("method", "Text"), ("verification_method", "Text"), ("exactness", "Text"), ("diagnostics", "Text")),
        Schema("OptimizationResult",
            ("original", "Symbolic"), ("optimized", "Symbolic"), ("estimated_cost_before", "Integer"),
            ("estimated_cost_after", "Integer"), ("shared_subtrees", "Integer"), ("horner_rewrites", "Integer"),
            ("transformations", "Array"), ("target", "Text"), ("policy", "Text")),
        Schema("CompilationResult",
            ("ir", "Text"), ("parameters", "Array"), ("result_domain", "Domain"), ("result_type", "Text"),
            ("precision_policy", "Text"), ("optimization_policy", "Text"), ("target", "Text"),
            ("mathir_version", "Integer"), ("exact", "Boolean")),
        Schema("ParameterInfo", ("name", "Text"), ("domain", "Domain")),
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
