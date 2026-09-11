using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// The structured diagnostic vocabulary (alignment plan section C; addendum D1): every rich
/// result record ENDS with a "diagnostics" field that is an ARRAY of Diagnostic records — never a
/// free-text value, never Null, and an empty array when there is nothing to report. The kernel's
/// human-readable Note/FailureReason text survives only as a Diagnostic MESSAGE; it is never a
/// value on the wire.
/// </summary>
public class DiagnosticContractTests
{
    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    private static Value Field(RecordValue record, string name) =>
        (Value)record.Fields.First(f => f.Name == name).Value!;

    private static readonly string[] DiagnosticFieldNames =
        { "code", "category", "message", "recoverable", "location", "details" };

    private static readonly string[] ErrorCategoryNames =
    {
        "ParseError", "DomainError", "UnsupportedOperation", "BudgetExceeded",
        "NoSolution", "TypeMismatch", "InternalInvariantFailure",
    };

    /// <summary>Every rich result record and the builtin that emits it.</summary>
    public static TheoryData<string, string> RichResults => new()
    {
        { "SolveResult", "solve_full(x^4 - x^2 - 1 == 0, x)" },
        { "SystemSolveResult", "solve_system_full([x + y == 1, x - y == 3], [x, y])" },
        { "LimitResult", "limit_full(sin(x)/x, x, 0)" },
        { "IntegrationResult", "integrate_full(x^2, x)" },
        { "TransformResult", "simplify_full(sin(x)^2 + cos(x)^2)" },
        { "OptimizationResult", "optimize_full(x^2 + 2*x + 1, [x])" },
        { "CompilationResult", "compile_full(x^2 + 1, [x])" },
    };

    [Fact]
    public void ErrorCategory_HasExactlyTheSevenFrozenMembers()
    {
        // The type is looked up by NAME so this test compiles before the vocabulary exists: the
        // gate must fail on the assertion, not on a missing type.
        var type = typeof(RecordValue).Assembly.GetType("Lovelace.Abstractions.ErrorCategory");
        Assert.NotNull(type);
        Assert.True(type!.IsEnum, "ErrorCategory is an enum");
        Assert.Equal(ErrorCategoryNames, Enum.GetNames(type));
        Assert.Equal(7, Enum.GetValues(type).Length);
    }

    /// <summary>Each of the seven rich records, driven through its own builtin, ends with a
    /// diagnostics field that is an Array kind on the wire — a Text value fails here (D1).</summary>
    [Theory]
    [MemberData(nameof(RichResults))]
    public void EveryRichResult_EndsWithADiagnosticsArrayNeverText(string typeName, string script)
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\"); y = symbol(\"y\")");
        var record = engine.Evaluate(script).AsRecord();
        Assert.Equal(typeName, record.TypeName);

        // the ordered field list is the contract: diagnostics is the LAST field
        Assert.Equal("diagnostics", record.Fields[^1].Name);

        var diagnostics = Field(record, "diagnostics");
        Assert.NotEqual(ValueKind.Text, diagnostics.Kind);
        Assert.NotEqual(ValueKind.Void, diagnostics.Kind);   // never Null
        Assert.Equal(ValueKind.Vector, diagnostics.Kind);
        var structured = StructuredProjection.ToStructured(diagnostics);
        Assert.Equal("Array", structured.Kind);
        Assert.Null(structured.Value);   // an Array carries elements, never a scalar text value
        Assert.NotNull(structured.Elements);
        Assert.Equal(structured.Elements!.Length, (int)structured.Shape![0]);
    }

    /// <summary>A partial solve must SAY what is missing in structure: at least one Diagnostic with
    /// a stable, non-empty code and an ErrorCategory enum member (never a category string).</summary>
    [Fact]
    public void PartialSolve_CarriesADiagnosticWithAStableCodeAndAnErrorCategoryMember()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var record = engine.Evaluate("solve_full(x^4 - x^2 - 1 == 0, x)").AsRecord();
        Assert.Equal("Partial", Field(record, "status").AsEnum().Name);

        var diagnostics = Field(record, "diagnostics").AsVector();
        Assert.True(diagnostics.Count >= 1,
            "a partial solve carries at least one Diagnostic saying what is missing");

        var diagnostic = diagnostics[0].AsRecord();
        Assert.Equal("Diagnostic", diagnostic.TypeName);
        Assert.Equal(DiagnosticFieldNames, diagnostic.Fields.Select(f => f.Name).ToArray());

        var code = Field(diagnostic, "code");
        Assert.Equal(ValueKind.Text, code.Kind);
        Assert.False(string.IsNullOrWhiteSpace(code.AsText()), "the diagnostic code is stable non-empty text");
        Assert.Equal("solve.unrepresented-roots", code.AsText());

        var category = Field(diagnostic, "category");
        Assert.Equal(ValueKind.Enum, category.Kind);
        Assert.Equal("ErrorCategory", category.AsEnum().TypeName);
        Assert.Contains(category.AsEnum().Name, ErrorCategoryNames);

        var message = Field(diagnostic, "message");
        Assert.Equal(ValueKind.Text, message.Kind);
        Assert.False(string.IsNullOrWhiteSpace(message.AsText()));

        Assert.Equal(ValueKind.Boolean, Field(diagnostic, "recoverable").Kind);

        // a kernel-level diagnostic has no source span at this layer: location is Null, not ""
        var location = Field(diagnostic, "location");
        Assert.Equal(ValueKind.Void, location.Kind);
        Assert.Equal("Null", StructuredProjection.ToStructured(location).Kind);

        // details is an empty ARRAY, never null and never an empty string
        var details = Field(diagnostic, "details");
        Assert.Equal(ValueKind.Vector, details.Kind);
        Assert.Empty(details.AsVector());
        Assert.Equal("Array", StructuredProjection.ToStructured(details).Kind);
    }

    /// <summary>A solve with nothing to report (no note, nothing unrepresented) carries an EMPTY
    /// diagnostics array — the shape is Array with zero elements, not Null and not "".</summary>
    [Fact]
    public void CompleteSolveWithoutANote_CarriesAnEmptyDiagnosticsArray()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var record = engine.Evaluate("solve_full(x^2 - 4 == 0, x)").AsRecord();
        Assert.Equal("Solved", Field(record, "status").AsEnum().Name);

        var diagnostics = Field(record, "diagnostics");
        Assert.Equal(ValueKind.Vector, diagnostics.Kind);
        Assert.Empty(diagnostics.AsVector());

        var structured = StructuredProjection.ToStructured(diagnostics);
        Assert.Equal("Array", structured.Kind);
        Assert.Empty(structured.Elements!);
    }

    /// <summary>An ordinary transformation reports no diagnostic: the array is present and empty.</summary>
    [Fact]
    public void SatisfiedTransformation_CarriesAnEmptyDiagnosticsArray()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var record = engine.Evaluate("simplify_full(sin(x)^2 + cos(x)^2)").AsRecord();
        Assert.Equal("Satisfied", Field(record, "status").AsEnum().Name);
        Assert.Empty(Field(record, "diagnostics").AsVector());
    }

    /// <summary>A transformation stopped by the step budget reports a BudgetExceeded diagnostic.
    /// The input is 420 independent exp(log(x+k)) redexes against the shipped MaxSteps = 400.</summary>
    [Fact]
    public void BoundedTransformation_CarriesABudgetExceededDiagnostic()
    {
        var terms = Enumerable.Range(1, 420).Select(i => "exp(log(x+" + i + "))");
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var record = engine.Evaluate("simplify_full(" + string.Join(" + ", terms) + ")").AsRecord();

        Assert.Equal("BudgetExceeded", Field(record, "status").AsEnum().Name);
        Assert.True(Field(record, "budget_exceeded").AsBoolean());

        var diagnostics = Field(record, "diagnostics").AsVector();
        Assert.True(diagnostics.Count >= 1,
            "a budget-exceeded transformation carries a Diagnostic, not an empty array");
        var codes = diagnostics.Select(d => Field(d.AsRecord(), "code").AsText()).ToArray();
        Assert.Contains("transform.budget-exceeded", codes);
        Assert.Contains(diagnostics, d =>
            Field(d.AsRecord(), "category").AsEnum().Name == "BudgetExceeded");
    }

    /// <summary>A transformation whose collected conditions are refuted by the active assumptions
    /// reports a DomainError diagnostic (the branch has no model).</summary>
    [Fact]
    public void UnsatisfiableTransformation_CarriesADomainErrorDiagnostic()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        engine.Evaluate("assume(x == 0)");
        var record = engine.Evaluate("simplify_full(x/x)").AsRecord();

        Assert.Equal("Unsatisfiable", Field(record, "status").AsEnum().Name);
        var diagnostics = Field(record, "diagnostics").AsVector();
        Assert.True(diagnostics.Count >= 1,
            "an unsatisfiable transformation carries a Diagnostic, not an empty array");
        var codes = diagnostics.Select(d => Field(d.AsRecord(), "code").AsText()).ToArray();
        Assert.Contains("transform.unsatisfiable-conditions", codes);
        Assert.Contains(diagnostics, d =>
            Field(d.AsRecord(), "category").AsEnum().Name == "DomainError");
    }

    /// <summary>D1: the kernel's free-text note now lives inside a Diagnostic MESSAGE; the record
    /// carries no text diagnostics field at all.</summary>
    [Fact]
    public void KernelNotes_CrossAsDiagnosticMessages()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var solve = engine.Evaluate("solve_full(x == x, x)").AsRecord();
        Assert.Equal("Unevaluated", Field(solve, "status").AsEnum().Name);
        var solveDiagnostic = Field(solve, "diagnostics").AsVector()[0].AsRecord();
        Assert.Equal("solve.unevaluated", Field(solveDiagnostic, "code").AsText());
        Assert.Equal("UnsupportedOperation", Field(solveDiagnostic, "category").AsEnum().Name);
        Assert.Equal("0 = 0: every value is a solution.", Field(solveDiagnostic, "message").AsText());

        var limitEngine = NewEngine();
        limitEngine.Evaluate("a = symbol(\"a\"); x = symbol(\"x\")");
        var limit = limitEngine.Evaluate("limit_full(a/x, x, 0)").AsRecord();
        Assert.Equal("Unevaluated", Field(limit, "status").AsEnum().Name);
        var limitDiagnostic = Field(limit, "diagnostics").AsVector()[0].AsRecord();
        Assert.Equal("limit.unevaluated", Field(limitDiagnostic, "code").AsText());
        Assert.Equal("UnsupportedOperation", Field(limitDiagnostic, "category").AsEnum().Name);
        Assert.Equal("leading coefficient is symbolic", Field(limitDiagnostic, "message").AsText());
    }
}
