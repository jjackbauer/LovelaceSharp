using System.Text.Json.Nodes;
using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;
using Xunit.Abstractions;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Round 22: the runtime's real-only limits must be LEARNABLE FROM STRUCTURE. <c>capabilities()</c>
/// publishes the supported/rejected domains and the known-unsupported operation classes, each
/// carrying the exact error code and <see cref="ErrorCategory"/> that the live call produces.
/// <para>
/// The last test is the one that keeps the statement honest: it runs the claimed-unsupported
/// operations through the published <c>Lovelace.Run</c> envelope and asserts the observed
/// code/category EQUAL the advertised ones. A capability table that drifts from the kernel fails
/// here, not in a user's lap.
/// </para>
/// </summary>
public class CapabilitiesBuiltinTests
{
    private readonly ITestOutputHelper _output;

    public CapabilitiesBuiltinTests(ITestOutputHelper output) => _output = output;

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

    private static RecordValue Capabilities(SuiteEngine engine) => engine.Evaluate("capabilities()").AsRecord();

    private static IReadOnlyDictionary<string, (string Code, string Category, string Message, string Trigger)>
        AdvertisedByOperationClass(SuiteEngine engine)
    {
        var map = new Dictionary<string, (string, string, string, string)>(StringComparer.Ordinal);
        foreach (Value entry in Field(Capabilities(engine), "unsupported_operations").AsVector())
        {
            var record = entry.AsRecord();
            map[Field(record, "operation_class").AsText()] = (
                Field(record, "code").AsText(),
                Field(record, "category").AsEnum().Name,
                Field(record, "message").AsText(),
                Field(record, "trigger").AsText());
        }
        return map;
    }

    // ------------------------------------------------------------------
    // Shape
    // ------------------------------------------------------------------

    [Fact]
    public void Capabilities_ReturnsARecordNamedCapabilitiesResult()
    {
        var engine = NewEngine();
        var value = engine.Evaluate("capabilities()");

        Assert.Equal(ValueKind.Record, value.Kind);
        Assert.Equal("CapabilitiesResult", value.AsRecord().TypeName);
    }

    [Fact]
    public void SupportedDomains_AreRealAndComplex_AsDomainValues()
    {
        var engine = NewEngine();
        var domains = Field(Capabilities(engine), "supported_domains").AsVector();

        Assert.Equal(new[] { MathDomain.Real, MathDomain.Complex },
            domains.Select(d => d.Kind == ValueKind.Domain ? d.AsDomain() : throw new Xunit.Sdk.XunitException($"not a Domain value: {d.Kind}")).ToArray());
    }

    [Fact]
    public void UnsupportedDomains_AreIntegerAndRational_AsDomainValues()
    {
        var engine = NewEngine();
        var domains = Field(Capabilities(engine), "unsupported_domains").AsVector();

        Assert.Equal(new[] { MathDomain.Integer, MathDomain.Rational },
            domains.Select(d => d.Kind == ValueKind.Domain ? d.AsDomain() : throw new Xunit.Sdk.XunitException($"not a Domain value: {d.Kind}")).ToArray());
    }

    [Fact]
    public void UnsupportedOperations_AreRecordsCarryingCodeCategoryAndMessage()
    {
        var engine = NewEngine();
        var operations = Field(Capabilities(engine), "unsupported_operations").AsVector();

        Assert.NotEmpty(operations);
        foreach (Value element in operations)
        {
            Assert.Equal(ValueKind.Record, element.Kind);
            var record = element.AsRecord();
            Assert.Equal("UnsupportedCapability", record.TypeName);

            Assert.False(string.IsNullOrWhiteSpace(Field(record, "operation_class").AsText()));
            Assert.False(string.IsNullOrWhiteSpace(Field(record, "code").AsText()));
            Assert.False(string.IsNullOrWhiteSpace(Field(record, "message").AsText()));
            Assert.False(string.IsNullOrWhiteSpace(Field(record, "trigger").AsText()));

            Value category = Field(record, "category");
            Assert.Equal(ValueKind.Enum, category.Kind);
            Assert.Equal(DiagnosticProjection.CategoryTypeName, category.AsEnum().TypeName);
            Assert.True(Enum.IsDefined(typeof(ErrorCategory), category.AsEnum().Name),
                $"'{category.AsEnum().Name}' is not an ErrorCategory member");
        }

        // the four classes the round brief names must all be present
        var classes = operations.Select(o => Field(o.AsRecord(), "operation_class").AsText()).ToArray();
        Assert.Contains("complex.sqrt-negative", classes);
        Assert.Contains("pow.non-integer-exponent", classes);
        Assert.Contains("solve.unsupported-domain", classes);
        Assert.Contains("rootof.complex-algebraic", classes);
    }

    [Fact]
    public void Exactness_SaysWhetherTheSetsAreExhaustiveOrBestEffort()
    {
        var engine = NewEngine();
        Value exactness = Field(Capabilities(engine), "exactness");

        Assert.Equal(ValueKind.Enum, exactness.Kind);
        Assert.Equal("CapabilitiesExactness", exactness.AsEnum().TypeName);
        Assert.Contains(exactness.AsEnum().Name, new[] { "Exhaustive", "BestEffort" });
    }

    // ------------------------------------------------------------------
    // The honesty test: advertised codes == the codes live calls produce
    // ------------------------------------------------------------------

    private static JsonNode StructuredFields(JsonNode structured)
    {
        var fields = new JsonObject();
        foreach (JsonNode? field in structured["fields"]!.AsArray())
            fields[field!["name"]!.GetValue<string>()] = field["value"]!.DeepClone();
        return fields;
    }

    private async Task<JsonNode> RunEnvelopeAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Lovelace.Run.Runner.RunAsync(
            new[] { "--eval", script, "--json", "--omit-functions" }, stdout, stderr);
        JsonNode? envelope = JsonNode.Parse(stdout.ToString());
        Assert.True(envelope is not null, $"'{script}' produced no envelope (exit {exitCode}). stderr: {stderr}");
        return envelope!;
    }

    [Fact]
    public async Task AdvertisedCodesAndCategories_MatchTheLiveEnvelope()
    {
        var advertised = AdvertisedByOperationClass(NewEngine());

        // 1. sqrt of a negative real — a top-level error envelope
        JsonNode sqrt = await RunEnvelopeAsync("sqrt(-1)");
        Assert.False(sqrt["ok"]!.GetValue<bool>());
        Assert.Equal(advertised["complex.sqrt-negative"].Code, sqrt["code"]!.GetValue<string>());
        Assert.Equal(advertised["complex.sqrt-negative"].Category, sqrt["category"]!.GetValue<string>());
        Assert.Equal(advertised["complex.sqrt-negative"].Message, sqrt["message"]!.GetValue<string>());

        // 2. a negative base raised to a non-integer power — a top-level error envelope
        JsonNode pow = await RunEnvelopeAsync("(-1)^(1/2)");
        Assert.False(pow["ok"]!.GetValue<bool>());
        Assert.Equal(advertised["pow.non-integer-exponent"].Code, pow["code"]!.GetValue<string>());
        Assert.Equal(advertised["pow.non-integer-exponent"].Category, pow["category"]!.GetValue<string>());
        Assert.Equal(advertised["pow.non-integer-exponent"].Message, pow["message"]!.GetValue<string>());

        // 3. solve over an integer domain — a top-level error envelope
        JsonNode solve = await RunEnvelopeAsync("solve(symbol(\"x\")^2 - 2 == 0, symbol(\"x\"), integer)");
        Assert.False(solve["ok"]!.GetValue<bool>());
        Assert.Equal(advertised["solve.unsupported-domain"].Code, solve["code"]!.GetValue<string>());
        Assert.Equal(advertised["solve.unsupported-domain"].Category, solve["category"]!.GetValue<string>());
        Assert.Equal(advertised["solve.unsupported-domain"].Message, solve["message"]!.GetValue<string>());

        // 4. complex algebraic roots of degree >= 4 — a structured Diagnostic inside a solve record
        JsonNode rootof = await RunEnvelopeAsync("solve_full(symbol(\"x\")^4 - symbol(\"x\")^2 - 1 == 0, symbol(\"x\"))");
        Assert.True(rootof["ok"]!.GetValue<bool>());
        JsonNode diagnostics = StructuredFields(rootof["result"]!["structured"]!)["diagnostics"]!;
        JsonNode diagnostic = diagnostics["elements"]!.AsArray()[0]!;
        JsonNode diagnosticFields = StructuredFields(diagnostic);
        Assert.Equal(advertised["rootof.complex-algebraic"].Code, diagnosticFields["code"]!["value"]!.GetValue<string>());
        Assert.Equal(advertised["rootof.complex-algebraic"].Category, diagnosticFields["category"]!["value"]!.GetValue<string>());
        Assert.Equal(advertised["rootof.complex-algebraic"].Message, diagnosticFields["message"]!["value"]!.GetValue<string>());
    }

    // ------------------------------------------------------------------
    // Registration / discoverability
    // ------------------------------------------------------------------

    [Fact]
    public void Capabilities_IsOneRegisteredBuiltin_WithCompleteMetadata()
    {
        var engine = NewEngine();
        var snapshot = engine.CaptureState();

        var builtins = snapshot.Functions.Values.Where(f => f.IsBuiltin).ToArray();
        int count = builtins.Length;
        _output.WriteLine($"builtin count with capabilities(): {count}");

        Assert.Single(builtins, f => f.Name == "capabilities");
        Assert.True(engine.Functions.TryGetValue("capabilities", out var fn), "capabilities() is not registered");
        Assert.Equal("Lovelace.Symbolics", fn!.PluginName);

        Assert.True(engine.InterpreterBuiltinDescriptors.TryGetValue("capabilities", out var descriptor),
            "capabilities() has no BuiltinDescriptor");
        Assert.Equal(BuiltinCategories.Introspection, descriptor!.Category);
        Assert.Empty(descriptor.Parameters);
        Assert.False(string.IsNullOrWhiteSpace(descriptor.Summary));
        Assert.NotEmpty(descriptor.Examples);
        Assert.NotEmpty(descriptor.Related);
        Assert.Equal("CapabilitiesResult", descriptor.ReturnKind);

        string? help = engine.Help.Function("capabilities");
        Assert.NotNull(help);
        Assert.DoesNotContain("(no summary registered)", help);
        Assert.DoesNotContain("Returns: Value", help);
    }
}
