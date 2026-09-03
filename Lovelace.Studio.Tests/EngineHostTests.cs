using Lovelace.Studio;

namespace Lovelace.Studio.Tests;

public class EngineHostTests
{
    private static (Session Session, EngineHost Host, string PlotDir) CreateHost()
    {
        var registry = new SessionRegistry();
        var host = new EngineHost(registry);
        var session = registry.Create();
        string dir = Path.Combine(Path.GetTempPath(), "lovelace-studio-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);
        session.Engine.PlotOutputDirectory = dir;
        session.Engine.PlotFileName = "plot.svg";
        return (session, host, dir);
    }

    [Fact]
    public async Task Evaluate_GivenScript_ReturnsResultAndUpdatedVariables()
    {
        var (session, host, dir) = CreateHost();
        try
        {
            var response = await host.EvaluateAsync(session, "x = 42");

            Assert.NotNull(response.Result);
            Assert.Equal("Natural", response.Result!.Kind);
            Assert.Equal("42", response.Result.Display);
            Assert.Equal("42 (Natural)", response.Result.Typed);
            Assert.Contains(response.Variables, v => v.Name == "x" && v.Kind == "Natural" && v.Display == "42");
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public async Task Evaluate_GivenPrintAndPlot_ReturnsLogsAndPlotSvg()
    {
        var (session, host, dir) = CreateHost();
        try
        {
            var response = await host.EvaluateAsync(session, "print(\"hello\")\nplot(1..3, [1, 4, 9])");

            Assert.Contains("hello", response.Logs);
            Assert.NotNull(response.Plot);
            Assert.StartsWith("<svg", response.Plot!.Svg);
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public async Task Evaluate_GivenError_ReturnsDiagnosticWithLineAndColumn()
    {
        var (session, host, dir) = CreateHost();
        try
        {
            var response = await host.EvaluateAsync(session, "x = 1\n1 + @");

            Assert.Null(response.Result);
            Assert.NotEmpty(response.Diagnostics);
            Assert.Equal(2, response.Diagnostics[0].Line);
            Assert.Equal(5, response.Diagnostics[0].Column);
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public async Task State_GivenVariablesAndFunctions_ReturnsSnapshotMatchingEngine()
    {
        var (session, host, dir) = CreateHost();
        try
        {
            await host.EvaluateAsync(session, "x = 3.14\nfunc f(y) = y * 2");

            var state = host.GetState(session);

            Assert.Contains(state.Variables, v => v.Name == "x" && v.Kind == "Real");
            Assert.Contains(state.Functions, f => f.Name == "f" && !f.IsBuiltin && f.Parameters.SequenceEqual(new[] { "y" }));
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public async Task DeleteVariable_GivenExistingName_RemovesItAndReturnsUpdatedState()
    {
        var (session, host, dir) = CreateHost();
        try
        {
            await host.EvaluateAsync(session, "x = 1\ny = 2");

            var state = host.DeleteVariable(session, "x");

            Assert.DoesNotContain(state.Variables, v => v.Name == "x");
            Assert.Contains(state.Variables, v => v.Name == "y");
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public async Task Evaluate_GivenSampleScript_ReturnsFunctionVariablesAndPlot()
    {
        var (session, host, dir) = CreateHost();
        try
        {
            var response = await host.EvaluateAsync(session,
                "func square(x) = x^2\nsquare(5)\nplot(1..5, [1,4,9,16,25])");

            Assert.Contains(response.Functions, f => f.Name == "square" && !f.IsBuiltin);
            Assert.Contains(response.Variables, v => v.Name == "_");
            Assert.NotNull(response.Plot);
            Assert.StartsWith("<svg", response.Plot!.Svg);
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public async Task Clear_GivenVariables_EmptiesWorkspace()
    {
        var (session, host, dir) = CreateHost();
        try
        {
            await host.EvaluateAsync(session, "x = 1\nfunc f(y) = y");

            var state = host.ClearVariables(session);

            Assert.Empty(state.Variables);
            Assert.Contains(state.Functions, f => f.Name == "f");
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public async Task Evaluate_GivenScript_ReturnsElapsedTime()
    {
        var (session, host, dir) = CreateHost();
        try
        {
            var response = await host.EvaluateAsync(session, "1 + 1");

            Assert.False(string.IsNullOrWhiteSpace(response.Elapsed));
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public async Task Evaluate_GivenPrintStatement_ReturnsOutputScopedToThatLine()
    {
        var (session, host, dir) = CreateHost();
        try
        {
            var response = await host.EvaluateAsync(session, "x = 11\nprint(x)");

            Assert.Equal(2, response.Timings.Length);
            Assert.Equal("11", response.Timings[0].Result);
            Assert.Null(response.Timings[0].Output);
            Assert.Null(response.Timings[1].Result);
            Assert.Equal("11", response.Timings[1].Output);
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public async Task Evaluate_GivenMultiLineScript_ReturnsPerLineTimings()
    {
        var (session, host, dir) = CreateHost();
        try
        {
            var response = await host.EvaluateAsync(session, "x = 1\ny = 2\nx + y");

            Assert.Equal(3, response.Timings.Length);
            Assert.Equal(1, response.Timings[0].Line);
            Assert.Equal("x = 1", response.Timings[0].Text);
            Assert.Equal("1", response.Timings[0].Result);
            Assert.Equal(2, response.Timings[1].Line);
            Assert.Equal("y = 2", response.Timings[1].Text);
            Assert.Equal("2", response.Timings[1].Result);
            Assert.Equal(3, response.Timings[2].Line);
            Assert.Equal("x + y", response.Timings[2].Text);
            Assert.Equal("3", response.Timings[2].Result);
            Assert.All(response.Timings, t => Assert.False(string.IsNullOrWhiteSpace(t.Elapsed)));
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    // -------------------------------------------------------------------
    // Sessions — independent state and precision across sessions.
    // -------------------------------------------------------------------

    [Fact]
    public async Task Sessions_GivenTwoSessions_HaveIndependentVariables()
    {
        var registry = new SessionRegistry();
        var host = new EngineHost(registry);
        var a = registry.Create();
        var b = registry.Create();
        try
        {
            await host.EvaluateAsync(a, "x = 1");
            await host.EvaluateAsync(b, "x = 2");

            var sa = host.GetState(a);
            var sb = host.GetState(b);

            Assert.Equal("1", sa.Variables.Single(v => v.Name == "x").Display);
            Assert.Equal("2", sb.Variables.Single(v => v.Name == "x").Display);
        }
        finally
        {
            registry.Remove(a.Id);
            registry.Remove(b.Id);
        }
    }

    [Fact]
    public async Task Precision_GivenTwoSessions_IsIndependent()
    {
        var registry = new SessionRegistry();
        var host = new EngineHost(registry);
        var a = registry.Create();
        var b = registry.Create();
        try
        {
            host.SetPrecision(a, 50);
            host.SetPrecision(b, 200);
            Assert.Equal(50, a.Precision);
            Assert.Equal(200, b.Precision);

            var ra = await host.EvaluateAsync(a, "sqrt(2)");
            var rb = await host.EvaluateAsync(b, "sqrt(2)");

            Assert.NotNull(ra.Result);
            Assert.NotNull(rb.Result);
            Assert.True(ra.Result!.Display.Length < rb.Result!.Display.Length,
                $"session A ({ra.Result.Display.Length} chars) should be shorter than session B ({rb.Result.Display.Length} chars)");
        }
        finally
        {
            registry.Remove(a.Id);
            registry.Remove(b.Id);
        }
    }

    // -------------------------------------------------------------------
    // Incremental compute — reuse vs recompute.
    // -------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_RepeatedScript_ReusesAllStatements()
    {
        var (session, host, dir) = CreateHost();
        try
        {
            const string script = "a = 2^10\nb = sqrt(a)\nc = a + b";
            await host.EvaluateAsync(session, script);
            var second = await host.EvaluateAsync(session, script);

            Assert.Equal(3, second.ReusedCount);
            Assert.All(second.Timings, t => Assert.Equal("reuse", t.Mode));
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public async Task Evaluate_MidScriptEdit_RecomputesOnlyDependents()
    {
        var (session, host, dir) = CreateHost();
        try
        {
            await host.EvaluateAsync(session, "a = 2^10\nb = sqrt(a)\nc = a + b");
            var second = await host.EvaluateAsync(session, "a = 2^10\nb = sqrt(a)\nd = a * b");

            Assert.Equal(2, second.ReusedCount);
            Assert.Equal("reuse", second.Timings[0].Mode);
            Assert.Equal("reuse", second.Timings[1].Mode);
            Assert.Equal("compute", second.Timings[2].Mode);
            Assert.Contains(second.Variables, v => v.Name == "d");
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public async Task Evaluate_PrintStatement_AlwaysRecomputes()
    {
        var (session, host, dir) = CreateHost();
        try
        {
            const string script = "x = 5\nprint(x)";
            await host.EvaluateAsync(session, script);
            var second = await host.EvaluateAsync(session, script);

            Assert.Equal("reuse", second.Timings[0].Mode);
            Assert.Equal("compute", second.Timings[1].Mode);
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public async Task Evaluate_DependentOnChangedVariable_Recomputes()
    {
        var (session, host, dir) = CreateHost();
        try
        {
            await host.EvaluateAsync(session, "x = 1\ny = x + 1");
            // First line changes (x 1→2); y's content is identical but it depends on x.
            var second = await host.EvaluateAsync(session, "x = 2\ny = x + 1");

            Assert.Equal("compute", second.Timings[0].Mode);
            Assert.Equal("compute", second.Timings[1].Mode);
            Assert.Equal("3", second.Variables.Single(v => v.Name == "y").Display);
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }
}
