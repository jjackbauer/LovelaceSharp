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

        // the classes the round-03 capability list must carry. Round 03 (goal cycle 4) replaced the
        // negative-base entry by TWO entries, because the live envelopes turn out to be identical
        // while the operation classes are not: a positive base with a non-integer exponent
        // (2^(1/2)) and a negative base with an exponent whose denominator is >= 3 ((-8)^(1/3)).
        var classes = operations.Select(o => Field(o.AsRecord(), "operation_class").AsText()).ToArray();
        Assert.Contains("pow.non-integer-exponent", classes);
        Assert.Contains("pow.negative-base-unrepresentable-exponent", classes);
        Assert.Contains("solve.unsupported-domain", classes);
        Assert.Contains("rootof.complex-algebraic", classes);

        // ... and the class round 03 MADE SUPPORTED must be gone. Advertising it would now be
        // false: the live call answers, so there is no error left to promise.
        Assert.DoesNotContain("complex.sqrt-negative", classes);
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

    /// <summary>The (code, category, message) the LIVE call actually produced for a trigger: the
    /// top-level envelope when the call failed, otherwise the FIRST structured Diagnostic the
    /// returned record carries. Both shapes are real published surfaces, and an entry may only be
    /// advertised if the live call reproduces it exactly.</summary>
    private static (string Code, string Category, string Message) LiveOutcome(JsonNode envelope, string operationClass)
    {
        if (!envelope["ok"]!.GetValue<bool>())
            return (envelope["code"]!.GetValue<string>(),
                    envelope["category"]!.GetValue<string>(),
                    envelope["message"]!.GetValue<string>());

        JsonNode structured = envelope["result"]!["structured"]!;
        if (!string.Equals(structured["kind"]!.GetValue<string>(), "Record", StringComparison.Ordinal))
            throw new Xunit.Sdk.XunitException(
                $"'{operationClass}': the live call SUCCEEDED with a {structured["kind"]!.GetValue<string>()} result and reported nothing to compare against");
        JsonNode? diagnostic = StructuredFields(structured)["diagnostics"]!["elements"]!.AsArray().FirstOrDefault();
        if (diagnostic is null)
            throw new Xunit.Sdk.XunitException(
                $"'{operationClass}': the live call SUCCEEDED and reported no diagnostic, so the list advertises an unsupported operation the runtime can actually perform");
        JsonNode fields = StructuredFields(diagnostic);
        return (fields["code"]!["value"]!.GetValue<string>(),
                fields["category"]!["value"]!.GetValue<string>(),
                fields["message"]!["value"]!.GetValue<string>());
    }

    /// <summary>
    /// The honesty property, now EXHAUSTIVE over the advertised list instead of hard-coded for a
    /// fixed four: every entry the record publishes is RUN through the published
    /// <c>Lovelace.Run</c> envelope and its advertised code, category and message must EQUAL what
    /// the live call produced. Round 03 replaced the four literal probes with this loop, so adding
    /// an entry without a truthful trigger — or removing one for a class the runtime still
    /// rejects — fails here instead of in a user's lap.
    /// </summary>
    [Fact]
    public async Task AdvertisedCodesAndCategories_MatchTheLiveEnvelope()
    {
        var advertised = AdvertisedByOperationClass(NewEngine());
        Assert.NotEmpty(advertised);

        var failures = new List<string>();
        foreach (var (operationClass, entry) in advertised)
        {
            JsonNode envelope;
            try
            {
                envelope = await RunEnvelopeAsync(entry.Trigger);
            }
            catch (Xunit.Sdk.XunitException ex)
            {
                failures.Add($"'{operationClass}' trigger '{entry.Trigger}': {ex.Message}");
                continue;
            }
            var observed = LiveOutcome(envelope, operationClass);
            if (observed.Code != entry.Code || observed.Category != entry.Category || observed.Message != entry.Message)
                failures.Add(
                    $"'{operationClass}' trigger '{entry.Trigger}': advertised {entry.Code}/{entry.Category}/\"{entry.Message}\" " +
                    $"but the live call produced {observed.Code}/{observed.Category}/\"{observed.Message}\"");
        }
        Assert.True(failures.Count == 0, string.Join("\n", failures));
        _output.WriteLine($"live-verified {advertised.Count} advertised unsupported operation class(es)");
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
