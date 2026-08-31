using Lovelace.Suite;
using Lovelace.Studio;

namespace Lovelace.Studio.Tests;

public class EngineHostTests
{
    private static (SuiteEngine Engine, EngineHost Host, string PlotDir) CreateHost()
    {
        var engine = new SuiteEngine();
        string dir = Path.Combine(Path.GetTempPath(), "lovelace-studio-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);
        engine.PlotOutputDirectory = dir;
        engine.PlotFileName = "plot.svg";
        return (engine, new EngineHost(engine), dir);
    }

    [Fact]
    public async Task Evaluate_GivenScript_ReturnsResultAndUpdatedVariables()
    {
        var (_, host, dir) = CreateHost();
        try
        {
            var response = await host.EvaluateAsync("x = 42");

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
        var (_, host, dir) = CreateHost();
        try
        {
            var response = await host.EvaluateAsync("print(\"hello\")\nplot(1..3, [1, 4, 9])");

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
        var (_, host, dir) = CreateHost();
        try
        {
            var response = await host.EvaluateAsync("x = 1\n1 + @");

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
        var (_, host, dir) = CreateHost();
        try
        {
            await host.EvaluateAsync("x = 3.14\nfunc f(y) = y * 2");

            var state = host.GetState();

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
        var (_, host, dir) = CreateHost();
        try
        {
            await host.EvaluateAsync("x = 1\ny = 2");

            var state = host.DeleteVariable("x");

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
        var (_, host, dir) = CreateHost();
        try
        {
            var response = await host.EvaluateAsync(
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
        var (_, host, dir) = CreateHost();
        try
        {
            await host.EvaluateAsync("x = 1\nfunc f(y) = y");

            var state = host.ClearVariables();

            Assert.Empty(state.Variables);
            Assert.Contains(state.Functions, f => f.Name == "f");
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }
}