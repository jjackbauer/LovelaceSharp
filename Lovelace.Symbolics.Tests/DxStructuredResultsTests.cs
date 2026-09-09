using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// DX-convergence Phase 3 contract tests: the structured *_full builtins, first-class
/// domain values, type()/inspect(), and record-typed compiler introspection.
/// </summary>
public class DxStructuredResultsTests
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
    // SolveResult
    // ------------------------------------------------------------------

    [Fact]
    public void SolveFull_ExposesStatusSolutionsAndConditions()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var result = engine.Evaluate("solve_full((x^2 - 1)/(x - 1) == 0, x)");
        Assert.Equal(ValueKind.Record, result.Kind);
        var r = result.AsRecord();
        Assert.Equal("SolveResult", r.TypeName);
        Assert.Equal("Solved", (string)((Value)r.Fields[0].Value!).AsText());
        Assert.Equal("[-1]", ValueFormatter.Format((Value)r.Fields[3].Value!));
        // the excluded-denominator condition is exposed structurally
        var conditions = (Value)r.Fields[4].Value!;
        Assert.Equal("[x - 1 != 0] (Vector)", ValueFormatter.FormatTyped(conditions));
        // property access works on records
        Assert.Equal("Solved", engine.Evaluate("solve_full(x^2 - 4 == 0, x).status").AsText());
        Assert.Equal("[-2, 2]", ValueFormatter.Format(engine.Evaluate("solve_full(x^2 - 4 == 0, x).solutions")));
    }

    [Fact]
    public void SolveFull_DomainFieldAndRealOption()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = engine.Evaluate("solve_full(x^2 + 1 == 0, x, real)").AsRecord();
        Assert.Equal("NoSolutions", (string)((Value)r.Fields[0].Value!).AsText());
        Assert.Equal("Real", (string)((Value)r.Fields[2].Value!).AsText());
        Assert.Equal("Complex", engine.Evaluate("solve_full(x^2 + 1 == 0, x).domain").AsText());
    }

    [Fact]
    public void DomainValues_AreFirstClass()
    {
        var engine = NewEngine();
        Assert.Equal(ValueKind.Domain, engine.Evaluate("real").Kind);
        Assert.Equal("real (Domain)", ValueFormatter.FormatTyped(engine.Evaluate("real")));
        Assert.Equal("complex", engine.Evaluate("type(complex)").AsText());
    }

    // ------------------------------------------------------------------
    // TransformResult
    // ------------------------------------------------------------------

    [Fact]
    public void SimplifyFull_CarriesConditionsAndSteps()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = engine.Evaluate("simplify_full(x/x)").AsRecord();
        Assert.Equal("TransformResult", r.TypeName);
        Assert.Equal("1", ValueFormatter.Format((Value)r.Fields[0].Value!));
        Assert.True(((Value)r.Fields[1].Value!).AsBoolean());                       // changed
        Assert.Equal("[x != 0] (Vector)", ValueFormatter.FormatTyped((Value)r.Fields[2].Value!));
        var steps = (Value)r.Fields[3].Value!;
        Assert.Equal(ValueKind.Vector, steps.Kind);
        var first = (Value)steps.AsVector()[0];
        Assert.Equal("RewriteStep", first.AsRecord().TypeName);
        Assert.Equal("rat.cancel-x-over-x", ((Value)first.AsRecord().Fields[0].Value!).AsText());
        // property access into nested records
        Assert.Equal("rat.cancel-x-over-x", engine.Evaluate("simplify_full(x/x).steps[0].rule_id").AsText());
    }

    [Fact]
    public void SimplifyFull_ExpLog_CarriesFiniteCondition()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = engine.Evaluate("simplify_full(exp(log(x)))").AsRecord();
        Assert.Equal("x", ValueFormatter.Format((Value)r.Fields[0].Value!));
        Assert.Equal("[finite(log(x))] (Vector)", ValueFormatter.FormatTyped((Value)r.Fields[2].Value!));
    }

    // ------------------------------------------------------------------
    // LimitResult
    // ------------------------------------------------------------------

    [Fact]
    public void LimitFull_ExposesExistenceAndOneSidedValues()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = engine.Evaluate("limit_full(1/x, x, 0)").AsRecord();
        Assert.Equal("LimitResult", r.TypeName);
        Assert.False(((Value)r.Fields[1].Value!).AsBoolean());   // exists
        Assert.Equal("-inf", ValueFormatter.Format((Value)r.Fields[3].Value!));
        Assert.Equal("inf", ValueFormatter.Format((Value)r.Fields[4].Value!));
        // the projection stays a readable string (compatibility), the record is structural
        Assert.Equal("does not exist (left: -inf, right: +inf)", engine.Evaluate("limit(1/x, x, 0)").AsText());
    }

    // ------------------------------------------------------------------
    // IntegrationResult / OptimizationResult
    // ------------------------------------------------------------------

    [Fact]
    public void IntegrateFull_ReportsStatusAndVerification()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = engine.Evaluate("integrate_full(x^2, x)").AsRecord();
        Assert.Equal("IntegrationResult", r.TypeName);
        Assert.Equal("SolvedExact", ((Value)r.Fields[0].Value!).AsText());
        Assert.True(((Value)r.Fields[3].Value!).AsBoolean());    // verified
        Assert.Equal("1/3*x^3", ValueFormatter.Format((Value)r.Fields[1].Value!));
    }

    [Fact]
    public void OptimizeFull_ReportsCosts()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = engine.Evaluate("optimize_full(x^2 + 2*x + 1, [x])").AsRecord();
        Assert.Equal("OptimizationResult", r.TypeName);
        Assert.True(engine.Evaluate("optimize_full(x^2 + 2*x + 1, [x]).estimated_cost_before").Kind == ValueKind.Integer);
    }

    // ------------------------------------------------------------------
    // CompilationResult
    // ------------------------------------------------------------------

    [Fact]
    public void CompileFull_ExposesKernelMetadata_AndBatchAcceptsRecord()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var k = engine.Evaluate("compile_full(x^2 + 1, [x])").AsRecord();
        Assert.Equal("CompilationResult", k.TypeName);
        Assert.Equal("[x] (Vector)", ValueFormatter.FormatTyped((Value)k.Fields[1].Value!));
        Assert.Equal("2", ValueFormatter.Format((Value)k.Fields[4].Value!));   // mathir_version
        // evalir_batch accepts the record (its ir field), matching the compile() text path
        var batch = engine.Evaluate("evalir_batch(compile_full(x^2 + 1, [x]), [1, 2, 3], 40)");
        Assert.Equal("[2, 5, 10] (Vector)", ValueFormatter.FormatTyped(batch));
    }

    // ------------------------------------------------------------------
    // type / inspect
    // ------------------------------------------------------------------

    [Fact]
    public void Type_And_Inspect_AreStructural()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        Assert.Equal("Symbolic", engine.Evaluate("type(x)").AsText());
        Assert.Equal("SolveResult", engine.Evaluate("type(solve_full(x^2 - 4 == 0, x))").AsText());
        var insp = engine.Evaluate("inspect(x^2 + 1)").AsRecord();
        Assert.Equal("Inspection", insp.TypeName);
        Assert.Equal("Symbolic", ((Value)insp.Fields[0].Value!).AsText());
        Assert.Equal("complex", ((Value)insp.Fields[1].Value!).AsText());   // unconstrained symbol domain
        Assert.Equal("[x] (Vector)", ValueFormatter.FormatTyped((Value)insp.Fields[3].Value!));
    }

    // ------------------------------------------------------------------
    // Matrix shape (Phase 6)
    // ------------------------------------------------------------------

    [Fact]
    public void Jacobian_And_Hessian_PreserveMatrixShape()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        engine.Evaluate("y = symbol(\"y\")");
        var j = engine.Evaluate("jacobian([x*y, x+y], [x, y])");
        Assert.Equal(ValueKind.Array, j.Kind);
        Assert.Equal(2, j.AsArrayValue().Rank);
        Assert.Equal("[[y, x], [1, 1]] (Array)", ValueFormatter.FormatTyped(j));
        var h = engine.Evaluate("hessian(x*y, [x, y])");
        Assert.Equal(ValueKind.Array, h.Kind);
        Assert.Equal(2, h.AsArrayValue().Rank);
        Assert.Equal("[[0, 1], [1, 0]] (Array)", ValueFormatter.FormatTyped(h));
    }

    [Fact]
    public void LinsolveFull_ExposesDetCondition()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        engine.Evaluate("y = symbol(\"y\")");
        var r = engine.Evaluate("linsolve_full([[x, 1], [0, y]], [0, 1])").AsRecord();
        Assert.Equal("MatrixSolveResult", r.TypeName);
        Assert.Equal("Solved", ((Value)r.Fields[0].Value!).AsText());
        var conditions = (Value)r.Fields[2].Value!;
        Assert.Equal("[x*y != 0] (Vector)", ValueFormatter.FormatTyped(conditions));
    }

    [Fact]
    public void LinsolveFull_SingularSystem_ReportsStatusNotThrow()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = engine.Evaluate("linsolve_full([[x, x], [x, x]], [1, 1])").AsRecord();
        Assert.Equal("NoSolutions", ((Value)r.Fields[0].Value!).AsText());
        // the convenience projection keeps its throwing behavior (compatibility)
        Assert.Throws<InvalidOperationException>(() => engine.Evaluate("linsolve([[x, x], [x, x]], [1, 1])"));
    }

    [Fact]
    public void InvFull_ExposesDetCondition()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        engine.Evaluate("y = symbol(\"y\")");
        var r = engine.Evaluate("inv_full([[x, 0], [0, y]])").AsRecord();
        Assert.Equal("MatrixInverseResult", r.TypeName);
        Assert.Equal("Solved", ((Value)r.Fields[0].Value!).AsText());
        Assert.Equal("[x*y != 0] (Vector)", ValueFormatter.FormatTyped((Value)r.Fields[2].Value!));
    }

    [Fact]
    public void EvalirBatch_AcceptsNestedRowValues()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        engine.Evaluate("y = symbol(\"y\")");
        // multi-parameter kernels accept row-shaped array literals (row-major)
        var k = engine.Evaluate("compile_full(x + y, [x, y])");
        var batch = engine.Evaluate("evalir_batch(compile_full(x + y, [x, y]), [[1, 2], [3, 4]], 40)");
        Assert.Equal("[3, 7] (Vector)", ValueFormatter.FormatTyped(batch));
        Assert.Equal(ValueKind.Record, k.Kind);
    }
}
