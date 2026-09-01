using Lovelace.Suite;
using Nat = global::Lovelace.Natural.Natural;

namespace Lovelace.Suite.Tests;

public class SuiteEngineTests
{
    [Fact]
    public async Task Evaluate_GivenAssignment_ExposesVariableInVariablesDictionary()
    {
        var engine = new SuiteEngine();

        await engine.EvaluateAsync("x = 42");

        Assert.True(engine.Variables.ContainsKey("x"));
        Assert.Equal(ValueKind.Natural, engine.Variables["x"].Kind);
        Assert.Equal(Nat.Parse("42", null), engine.Variables["x"].AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenFunctionDefinition_ExposesSignatureAndBody()
    {
        var engine = new SuiteEngine();

        await engine.EvaluateAsync("func f(a, b) { a + b }");

        Assert.True(engine.Functions.ContainsKey("f"));
        var fn = engine.Functions["f"];
        Assert.False(fn.IsBuiltin);
        Assert.Equal(["a", "b"], fn.Parameters);
        Assert.NotEmpty(fn.Body);
    }

    [Fact]
    public async Task Evaluate_GivenBuiltin_IsMarkedBuiltin()
    {
        var engine = new SuiteEngine();

        await engine.EvaluateAsync("abs(-5)");

        Assert.True(engine.Functions["abs"].IsBuiltin);
    }

    [Fact]
    public async Task Evaluate_GivenResult_StoresInUnderscoreVariable()
    {
        var engine = new SuiteEngine();

        await engine.EvaluateAsync("1 + 1");

        Assert.True(engine.Variables.ContainsKey("_"));
        Assert.Equal(Nat.Parse("2", null), engine.Variables["_"].AsNatural());
    }

    [Fact]
    public async Task CaptureState_GivenVariablesAndFunctions_ReturnsMatchingSnapshot()
    {
        var engine = new SuiteEngine();
        await engine.EvaluateAsync("x = 3.14");
        await engine.EvaluateAsync("func f(y) = y * 2");

        var snapshot = engine.CaptureState();

        Assert.True(snapshot.Variables.ContainsKey("x"));
        Assert.Equal(ValueKind.Real, snapshot.Variables["x"].Kind);
        Assert.True(snapshot.Functions.ContainsKey("f"));
        Assert.False(snapshot.Functions["f"].IsBuiltin);
    }

    [Fact]
    public void CaptureState_IsImmutable()
    {
        var engine = new SuiteEngine();
        var snapshot = engine.CaptureState();
        int varCount = snapshot.Variables.Count;

        engine.SetVariable("x", new Value(Nat.Parse("1", null)));

        Assert.Equal(varCount, snapshot.Variables.Count);
        Assert.False(snapshot.Variables.ContainsKey("x"));
    }

    [Fact]
    public async Task VariableChanged_GivenAssignment_RaisesEvent()
    {
        var engine = new SuiteEngine();
        var names = new List<string>();
        engine.VariableChanged += (_, e) => names.Add(e.Name);

        await engine.EvaluateAsync("x = 7");

        Assert.Contains("x", names);
    }

    [Fact]
    public async Task FunctionDefined_GivenDefinition_RaisesEvent()
    {
        var engine = new SuiteEngine();
        string? name = null;
        engine.FunctionDefined += (_, e) => name = e.Definition.Name;

        await engine.EvaluateAsync("func g() { 1 }");

        Assert.Equal("g", name);
    }

    [Fact]
    public void SetVariableAndRemoveVariable_Work()
    {
        var engine = new SuiteEngine();
        engine.SetVariable("x", new Value(Nat.Parse("5", null)));

        Assert.True(engine.TryGetVariable("x", out var value));
        Assert.Equal(Nat.Parse("5", null), value.AsNatural());

        Assert.True(engine.RemoveVariable("x"));
        Assert.False(engine.Variables.ContainsKey("x"));
    }

    [Fact]
    public async Task Evaluate_GivenError_RecordsDiagnosticWithPosition()
    {
        var engine = new SuiteEngine();

        await Assert.ThrowsAsync<InvalidOperationException>(() => engine.EvaluateAsync("1 + @"));

        Assert.NotEmpty(engine.Diagnostics);
        Assert.True(engine.Diagnostics[0].Position >= 0);
    }

    [Fact]
    public void Parse_GivenStatements_ReturnsProgram()
    {
        var engine = new SuiteEngine();

        var program = engine.Parse("x = 1; y = 2; x + y");

        Assert.Equal(3, program.Statements.Count);
    }

    [Fact]
    public void ParseExpression_GivenExpression_ReturnsExpr()
    {
        var engine = new SuiteEngine();

        var expr = engine.ParseExpression("1 + 2");

        Assert.IsType<BinaryExpr>(expr);
    }

    [Fact]
    public async Task EvaluateAsync_GivenScript_RecordsElapsedTime()
    {
        var engine = new SuiteEngine();

        await engine.EvaluateAsync("1 + 1");

        Assert.True(engine.LastElapsed >= TimeSpan.Zero);
        Assert.False(string.IsNullOrWhiteSpace(engine.LastElapsedDisplay));
    }

    [Fact]
    public async Task EvaluateAsync_GivenMultipleStatements_RecordsPerStatementTimings()
    {
        var engine = new SuiteEngine();

        await engine.EvaluateAsync("x = 1; y = 2; x + y");

        Assert.Equal(3, engine.OperationTimings.Count);
        Assert.Equal(0, engine.OperationTimings[0].Position);
        Assert.True(engine.OperationTimings[1].Position > engine.OperationTimings[0].Position);
        Assert.True(engine.OperationTimings[2].Position > engine.OperationTimings[1].Position);
        Assert.All(engine.OperationTimings, t => Assert.True(t.Elapsed >= TimeSpan.Zero));
        Assert.All(engine.OperationTimings, t => Assert.False(string.IsNullOrWhiteSpace(t.ElapsedDisplay)));
    }
}
