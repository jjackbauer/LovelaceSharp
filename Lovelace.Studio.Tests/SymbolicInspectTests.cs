using Lovelace.Studio;

namespace Lovelace.Studio.Tests;

public class SymbolicInspectTests
{
    private static (Session Session, EngineHost Host) CreateHost()
    {
        var registry = new SessionRegistry();
        return (registry.Create(), new EngineHost(registry));
    }

    [Fact]
    public async Task Inspect_GivenSymbolicExpression_ReturnsAllSurfaces()
    {
        var (session, host) = CreateHost();
        await host.EvaluateAsync(session, "x = symbol(\"x\")");
        var response = await host.InspectSymbolicAsync(session, "x^2 + 2*x + 1");

        Assert.Equal("(add (rat 1 1) (pow (sym x) (rat 2 1)) (mul (rat 2 1) (sym x)))", response.Canonical);
        Assert.Equal("x^2 + 2*x + 1", response.Pretty);
        Assert.NotNull(response.Tree);
        Assert.Equal("Add", response.Tree!.Kind);
        Assert.Equal(3, response.Tree.Children.Length);
        Assert.NotNull(response.MathIR);
        Assert.StartsWith("#!mathir 2", response.MathIR);
    }

    [Fact]
    public async Task Inspect_GivenNonSymbolicValue_ReportsDiagnostic()
    {
        var (session, host) = CreateHost();
        var response = await host.InspectSymbolicAsync(session, "42");
        Assert.Null(response.Canonical);
        Assert.Single(response.Diagnostics);
    }

    [Fact]
    public async Task Inspect_TraceAndAssumptions_Surfaces()
    {
        var (session, host) = CreateHost();
        await host.EvaluateAsync(session, "x = symbol(\"x\")");
        await host.EvaluateAsync(session, "assume_positive(x)");
        var response = await host.InspectSymbolicAsync(session, "sqrt(x^2)");
        // under the positive assumption the sqrt-square rule fires with a trace step
        Assert.Contains(response.TraceSteps, s => s.Contains("pow.sqrt-square-nonnegative"));
        Assert.NotEmpty(response.Assumptions);
    }
}
