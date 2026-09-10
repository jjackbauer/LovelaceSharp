using Lovelace.Abstractions;
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
        // solutions are structured objects, each carrying its OWN conditions
        var solutions = (Value)r.Fields[5].Value!;
        Assert.Equal(ValueKind.Vector, solutions.Kind);
        var solution = (RecordValue)((Value)solutions.AsVector()[0]).AsRecord();
        Assert.Equal("Solution", solution.TypeName);
        Assert.Equal("-1", ValueFormatter.Format((Value)solution.Fields[0].Value!));
        Assert.Equal("[x - 1 != 0]", ValueFormatter.Format((Value)solution.Fields[1].Value!));
        Assert.Equal("1", ((Value)solution.Fields[2].Value!).AsInteger().ToString());
        Assert.Equal("Exact", (string)((Value)solution.Fields[3].Value!).AsText());
        // property access works on records
        Assert.Equal("Solved", engine.Evaluate("solve_full(x^2 - 4 == 0, x).status").AsText());
        Assert.Equal("True", engine.Evaluate("solve_full(x^2 - 4 == 0, x).complete").AsBoolean().ToString());
        var values = engine.Evaluate("solve_full(x^2 - 4 == 0, x).solutions").AsVector();
        Assert.Equal(2, values.Count);
        Assert.Equal("[x - 1 != 0]", ValueFormatter.Format(engine.Evaluate("solve_full((x^2 - 1)/(x - 1) == 0, x).common_conditions")));
    }

    [Fact]
    public void SolveFull_DomainFieldAndRealOption()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = engine.Evaluate("solve_full(x^2 + 1 == 0, x, real)").AsRecord();
        Assert.Equal("NoSolutions", (string)((Value)r.Fields[0].Value!).AsText());
        // the domain is a first-class Domain value, never text
        Assert.Equal(ValueKind.Domain, ((Value)r.Fields[2].Value!).Kind);
        Assert.Equal(MathDomain.Real, ((Value)r.Fields[2].Value!).AsDomain());
        Assert.Equal(MathDomain.Complex, engine.Evaluate("solve_full(x^2 + 1 == 0, x).domain").AsDomain());
        // partial results are never marked complete
        Assert.Equal("Partial", engine.Evaluate("solve_full(x^4 - x^2 - 1 == 0, x).status").AsText());
        Assert.False(engine.Evaluate("solve_full(x^4 - x^2 - 1 == 0, x).complete").AsBoolean());
        Assert.Equal("2", engine.Evaluate("solve_full(x^4 - x^2 - 1 == 0, x).unrepresented_count").AsInteger().ToString());
    }

    [Fact]
    public void DomainValues_AreFirstClass()
    {
        var engine = NewEngine();
        Assert.Equal(ValueKind.Domain, engine.Evaluate("real").Kind);
        Assert.Equal("real (Domain)", ValueFormatter.FormatTyped(engine.Evaluate("real")));
        // one naming convention: the ValueKind name, with domains distinguished as Domain
        Assert.Equal("Domain", engine.Evaluate("type(complex)").AsText());
        // the domain the value denotes is available from inspect().domain, as a Domain value
        Assert.Equal(ValueKind.Domain, engine.Evaluate("inspect(complex).domain").Kind);
        Assert.Equal("complex", ValueFormatter.Format(engine.Evaluate("inspect(complex).domain")));
        Assert.Equal("real", ValueFormatter.Format(engine.Evaluate("inspect(real).domain")));
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
        Assert.Equal("Satisfied", (string)((Value)r.Fields[0].Value!).AsText());    // status
        Assert.Equal("x/x", ValueFormatter.Format((Value)r.Fields[1].Value!));      // original
        Assert.Equal("1", ValueFormatter.Format((Value)r.Fields[2].Value!));        // expression
        Assert.True(((Value)r.Fields[3].Value!).AsBoolean());                       // changed
        Assert.Equal("[x != 0] (Vector)", ValueFormatter.FormatTyped((Value)r.Fields[4].Value!));
        var steps = (Value)r.Fields[5].Value!;
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
        Assert.Equal("x", ValueFormatter.Format((Value)r.Fields[2].Value!));
        // the condition is a structured record, never the string "finite(log(x))"
        var conditions = (Value)r.Fields[4].Value!;
        Assert.Equal(ValueKind.Vector, conditions.Kind);
        var leaf = conditions.AsVector()[0];
        Assert.Equal(ValueKind.Record, leaf.Kind);
        Assert.Equal("FiniteCondition", leaf.AsRecord().TypeName);
        Assert.Equal("log(x)", ValueFormatter.Format((Value)leaf.AsRecord().Fields[0].Value!));
        Assert.Equal("[FiniteCondition(expression: log(x))] (Vector)", ValueFormatter.FormatTyped(conditions));
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
        static Value F(RecordValue rec, string name) => (Value)rec.Fields.First(f => f.Name == name).Value!;

        Assert.Equal("LimitResult", r.TypeName);
        Assert.False(F(r, "exists").AsBoolean());
        Assert.Equal("-inf", ValueFormatter.Format(F(r, "left")));
        Assert.Equal("inf", ValueFormatter.Format(F(r, "right")));
        // each one-sided value carries the constraint it holds under
        Assert.Equal("[x < 0] (Vector)", ValueFormatter.FormatTyped(F(r, "left_conditions")));
        Assert.Equal("[x > 0] (Vector)", ValueFormatter.FormatTyped(F(r, "right_conditions")));
        Assert.Equal("Exact", F(r, "exactness").AsText());
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
        static Value F(RecordValue rec, string name) => (Value)rec.Fields.First(f => f.Name == name).Value!;

        Assert.Equal("IntegrationResult", r.TypeName);
        Assert.Equal("SolvedExact", F(r, "status").AsText());
        Assert.True(F(r, "verified").AsBoolean());
        Assert.Equal("1/3*x^3", ValueFormatter.Format(F(r, "expression")));
        // the strategy and the verification test are exposed, not just the boolean
        Assert.Equal("table", F(r, "method").AsText());
        Assert.StartsWith("differentiation+", F(r, "verification_method").AsText());
    }

    [Fact]
    public void OptimizeFull_ReportsCosts()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var r = engine.Evaluate("optimize_full(x^2 + 2*x + 1, [x])").AsRecord();
        static Value F(RecordValue rec, string name) => (Value)rec.Fields.First(f => f.Name == name).Value!;

        Assert.Equal("OptimizationResult", r.TypeName);
        Assert.Equal("cse+horner+precision-aware", F(r, "policy").AsText());
        Assert.Equal(ValueKind.Vector, F(r, "transformations").Kind);
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
        static Value Field(RecordValue r, string name) =>
            (Value)r.Fields.First(f => f.Name == name).Value!;

        // parameters carry name AND domain (a Domain value), not just a name
        var parameters = Field(k, "parameters");
        var first = parameters.AsVector()[0];
        Assert.Equal("ParameterInfo", first.AsRecord().TypeName);
        Assert.Equal("x", ((Value)first.AsRecord().Fields[0].Value!).AsText());
        Assert.Equal(ValueKind.Domain, ((Value)first.AsRecord().Fields[1].Value!).Kind);

        Assert.Equal(ValueKind.Domain, Field(k, "result_domain").Kind);
        Assert.Equal("caller_supplied", Field(k, "precision_policy").AsText());
        Assert.Equal("none", Field(k, "optimization_policy").AsText());
        Assert.Equal("2", ValueFormatter.Format(Field(k, "mathir_version")));
        // evalir_batch accepts the record (its ir field), matching the compile() text path
        var batch = engine.Evaluate("evalir_batch(compile_full(x^2 + 1, [x]), [1, 2, 3], 40)");
        Assert.Equal("[2, 5, 10] (Vector)", ValueFormatter.FormatTyped(batch));
    }

    [Fact]
    public void EvalirBatch_RowOrientedInput_EvaluatesRowByRow()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\"); y = symbol(\"y\")");
        var batch = engine.Evaluate("evalir_batch(compile(x*y + 1, [x, y]), [[1, 2], [3, 4]], 40)");
        Assert.Equal("[3, 13] (Vector)", ValueFormatter.FormatTyped(batch));
    }

    [Fact]
    public void EvalirBatch_RowWidthMismatch_IsRejectedWithTheDocumentedShape()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\"); y = symbol(\"y\")");
        // three values per row for a two-parameter kernel: rejected, never re-flowed into
        // different rows (which would silently pair the wrong parameters)
        var ex = Assert.Throws<InvalidOperationException>(() =>
            engine.Evaluate("evalir_batch(compile(x*y + 1, [x, y]), [[1, 2, 5], [3, 4, 6]], 40)"));
        Assert.Equal("evalir_batch(): kernel expects 2 parameters per row; row 0 contains 3.", ex.Message);
    }

    [Fact]
    public void EvalirBatch_FlatInputNotAWholeNumberOfRows_IsRejected()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\"); y = symbol(\"y\")");
        var ex = Assert.Throws<InvalidOperationException>(() =>
            engine.Evaluate("evalir_batch(compile(x*y + 1, [x, y]), [1, 2, 3], 40)"));
        Assert.Equal(
            "evalir_batch(): kernel expects 2 parameters per row; got 3 values, which is not a whole number of rows.",
            ex.Message);
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
        static Value F(RecordValue rec, string name) => (Value)rec.Fields.First(f => f.Name == name).Value!;

        Assert.Equal("Symbolic", F(insp, "type").AsText());
        // domain is a Domain VALUE, not lowercased text
        Assert.Equal(ValueKind.Domain, F(insp, "domain").Kind);
        Assert.Equal("complex", ValueFormatter.Format(F(insp, "domain")));
        Assert.Equal("[x] (Vector)", ValueFormatter.FormatTyped(F(insp, "free_symbols")));
        // both print forms are exposed, plus the array metadata
        Assert.Equal("x^2 + 1", F(insp, "pretty").AsText());
        Assert.Equal("(add (rat 1 1) (pow (sym x) (rat 2 1)))", F(insp, "canonical").AsText());
        Assert.Equal(ValueKind.Void, F(insp, "shape").Kind);
        Assert.Equal(ValueKind.Void, F(insp, "rank").Kind);
        Assert.Equal(ValueKind.Domain, F(insp, "element_domain").Kind);
        Assert.Equal(ValueKind.Vector, F(insp, "assumptions").Kind);
    }

    [Theory]
    [InlineData("1", "Natural")]
    [InlineData("1/2", "Real")]
    [InlineData("1.5", "Real")]
    [InlineData("[1, 2]", "Vector")]
    [InlineData("[[1, 2], [3, 4]]", "Array")]
    [InlineData("symbol(\"x\")", "Symbolic")]
    [InlineData("1 > 0", "Boolean")]
    [InlineData("\"text\"", "Text")]
    [InlineData("complex", "Domain")]
    [InlineData("rational()", "Domain")]
    [InlineData("solve_full(symbol(\"z\")^2 - 4 == 0, symbol(\"z\"))", "SolveResult")]
    public void Type_UsesOneVocabulary_ForEveryKind(string source, string expected)
    {
        var engine = NewEngine();
        Assert.Equal(expected, engine.Evaluate($"type({source})").AsText());
    }

    [Fact]
    public void Inspect_ReportsTheAssumptionsThatConstrainTheFreeSymbols()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\"); y = symbol(\"y\")");
        engine.Evaluate("assume(x > 5)");
        var insp = engine.Evaluate("inspect(x^2 + y)").AsRecord();
        var assumptions = (Value)insp.Fields.First(f => f.Name == "assumptions").Value!;

        // x is constrained, y is not: exactly the relevant assumption is reported, structurally
        var leaf = Assert.Single(assumptions.AsVector());
        Assert.Equal(ValueKind.Symbolic, leaf.Kind);
        Assert.Equal("x > 5", Lovelace.Symbolics.Printing.PrettyPrint(leaf.AsSymbolic()));
    }

    [Fact]
    public void Inspect_ArrayShapeRankAndElementDomain_AreStructural()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var insp = engine.Evaluate("inspect([[x, 1], [2, 3]])").AsRecord();
        static Value F(RecordValue rec, string name) => (Value)rec.Fields.First(f => f.Name == name).Value!;

        Assert.Equal("Array", F(insp, "type").AsText());
        Assert.Equal("[2, 2] (Vector)", ValueFormatter.FormatTyped(F(insp, "shape")));
        Assert.Equal("2", ValueFormatter.Format(F(insp, "rank")));
        Assert.Equal(ValueKind.Domain, F(insp, "element_domain").Kind);
        Assert.Equal("complex", ValueFormatter.Format(F(insp, "element_domain")));
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
