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
            "x = symbol(\"x\"); simplify_full(x/x)",
            "x = symbol(\"x\", real); simplify_full(sqrt(x^2))",
            "x = symbol(\"x\"); limit_full(1/x, x, 0)",
            "x = symbol(\"x\"); integrate_full(x^2, x)",
            "x = symbol(\"x\"); optimize_full(x^2 + 2*x + 1, [x])",
            "x = symbol(\"x\"); compile_full(x^2 + 1, [x])",
            "x = symbol(\"x\"); inspect(x^2 + 1)",
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
        Assert.Contains("TransformResult", seen);
        Assert.Contains("RewriteStep", seen);
        Assert.Contains("LimitResult", seen);
        Assert.Contains("IntegrationResult", seen);
        Assert.Contains("OptimizationResult", seen);
        Assert.Contains("CompilationResult", seen);
        Assert.Contains("ParameterInfo", seen);
        Assert.Contains("Inspection", seen);
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
}
