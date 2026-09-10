using System.Text.Json;
using Lovelace.Studio;
using Lovelace.Suite;

namespace Lovelace.Studio.Tests;

/// <summary>
/// The structured payload is the contract the UI renders from: every value Studio returns carries
/// the shared <c>Lovelace.Suite.StructuredProjection</c> form (never re-derived from display text),
/// so a record keeps its type name and an array keeps its shape.
/// </summary>
public class StructuredPayloadTests
{
    private static (Session Session, EngineHost Host, string Dir) CreateHost()
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

    // The same response /api/evaluate hands back (via GET /api/run/{id}), serialized with the
    // endpoint's own source-generated context so the assertion is on the real wire shape.
    private static JsonDocument SerializeWire(EvaluateResponse response) =>
        JsonDocument.Parse(JsonSerializer.Serialize(response, StudioJsonContext.Default.EvaluateResponse));

    [Fact]
    public async Task Evaluate_GivenSolveFull_CarriesStructuredSolveResultWithTypeNameAndSolutionsShape()
    {
        var (session, host, dir) = CreateHost();
        try
        {
            var response = await host.EvaluateAsync(session, "x = symbol(\"x\")\nsolve_full(x^2 - 4 == 0, x)");

            // object-level: the DTO carries the payload and the record type name, not just "Record"
            var result = Assert.IsType<ValueResult>(response.Result);
            Assert.Equal("Record", result.Kind);
            Assert.Equal("SolveResult", result.TypeName);
            var structured = Assert.IsType<StructuredValueDto>(result.Structured);
            Assert.Equal("Record", structured.Kind);
            Assert.Equal("SolveResult", structured.Type);

            var solutions = structured.Fields!.Single(f => f.Name == "solutions").Value;
            Assert.Equal("Array", solutions.Kind);
            Assert.Equal(new long[] { 2 }, solutions.Shape);
            var elements = Assert.IsType<StructuredValueDto[]>(solutions.Elements);
            Assert.Equal(2, elements.Length);
            Assert.All(elements, e => Assert.Equal("Solution", e.Type));
            foreach (var solution in elements)
            {
                Assert.Equal("Symbolic", solution.Fields!.Single(f => f.Name == "value").Value.Kind);
                Assert.Equal("Array", solution.Fields!.Single(f => f.Name == "conditions").Value.Kind);
                Assert.Equal("Integer", solution.Fields!.Single(f => f.Name == "multiplicity").Value.Kind);
                Assert.Equal("Text", solution.Fields!.Single(f => f.Name == "exactness").Value.Kind);
            }

            // the same fields on the wire (camelCase, source-generated context)
            using var wire = SerializeWire(response);
            var root = wire.RootElement;
            Assert.Equal("SolveResult", root.GetProperty("result").GetProperty("typeName").GetString());
            var wireStructured = root.GetProperty("result").GetProperty("structured");
            Assert.Equal("Record", wireStructured.GetProperty("kind").GetString());
            Assert.Equal("SolveResult", wireStructured.GetProperty("type").GetString());
            var wireSolutions = wireStructured.GetProperty("fields").EnumerateArray()
                .Single(f => f.GetProperty("name").GetString() == "solutions").GetProperty("value");
            Assert.Equal("Array", wireSolutions.GetProperty("kind").GetString());
            Assert.Equal(new[] { 2L }, wireSolutions.GetProperty("shape").EnumerateArray().Select(s => s.GetInt64()));
            Assert.Equal(2, wireSolutions.GetProperty("elements").GetArrayLength());

            // a flattened variable row keeps the same structure
            var row = response.Variables.Single(v => v.Name == "_");
            Assert.Equal("SolveResult", row.TypeName);
            Assert.Equal("SolveResult", row.Structured!.Type);

            // and so does the per-statement timing row
            var lastStep = response.Timings[^1];
            Assert.Equal("SolveResult", lastStep.Structured!.Type);
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public async Task SetFormat_GivenUnicode_EngineRendersUnicodeAndStateReportsIt()
    {
        var (session, host, dir) = CreateHost();
        try
        {
            await host.EvaluateAsync(session, "x = symbol(\"x\")");

            Assert.False(host.GetState(session).Unicode); // ASCII by default
            var ascii = await host.EvaluateAsync(session, "x <= 1");
            Assert.Equal("x <= 1", ascii.Result!.Display);

            var state = host.SetFormat(session, true);

            Assert.True(state.Unicode);
            Assert.True(session.Engine.UnicodeOutput);
            var unicode = await host.EvaluateAsync(session, "x <= 1");
            Assert.Equal("x ≤ 1", unicode.Result!.Display);
            // the workspace table is re-rendered from the live values under the new format
            Assert.Contains(state.Variables, v => v.Name == "_" && v.Display == "x ≤ 1");
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public async Task InspectSymbolic_GivenExpression_CarriesStructuredFormOfInspectedValue()
    {
        var (session, host, _) = CreateHost();
        await host.EvaluateAsync(session, "x = symbol(\"x\")");
        var response = await host.InspectSymbolicAsync(session, "x^2 + 1");

        Assert.NotNull(response.Tree);
        Assert.NotNull(response.Structured);
        Assert.Equal("Symbolic", response.Structured!.Kind);
        Assert.Equal("x^2 + 1", response.Structured.Pretty);
        Assert.Equal(new[] { "x" }, response.Structured.FreeSymbols);
    }
}
