using System.Text.Json.Nodes;
using Lovelace.Dsp;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// ADV-08 — the registry is a DISCOVERY surface, and it can be queried without evaluating anything.
/// <para>
/// Before this, the only way to ask the runtime what it could do was to evaluate a script and read the
/// registry out of the resulting envelope: ~42 KB with the descriptor metadata, plus the cost of an
/// evaluation, plus a revision bump. The harnessability benchmark measured what that costs an agent —
/// 185 registry lookups over 20 runs in its registry-bearing arm, and every task whose repair hinged on
/// naming a builtin stalling without one (RESULTS.md §6).
/// </para>
/// <para>
/// These tests pin the three properties the surface has to have to be usable by a machine: it answers
/// WITHOUT evaluating, it says when its answer is a PAGE, and what it says about a function is what the
/// help surface and the descriptor say.
/// </para>
/// </summary>
public class RegistryQueryTests
{
    private static async Task<(int ExitCode, JsonNode Envelope)> QueryAsync(params string[] args)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(args, stdout, stderr, new StringReader(string.Empty));
        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(stdout.ToString(), $"stdout of {string.Join(' ', args)}"));
    }

    private static SuiteEngine LiveEngine()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    [Fact]
    public async Task AListingAnswersWithoutEvaluatingAnything()
    {
        // No --eval, no --file, no --stdin: nothing to evaluate. If the query touched the evaluation path
        // at all it would have to refuse or read stdin, and the test would see it.
        var (exitCode, envelope) = await QueryAsync("--functions", "det");

        Assert.Equal(0, exitCode);
        Assert.True(envelope["ok"]!.GetValue<bool>());
        // no result, no variables, no timings, and no revision: a READ leaves no trace of a run
        Assert.Null(envelope["result"]);
        Assert.Null(envelope["timings"]);
        Assert.Null(envelope["revision"]);
        Assert.Equal("det", envelope["pattern"]!.GetValue<string>());

        JsonArray functions = envelope["functions"]!.AsArray();
        JsonNode entry = Assert.Single(functions)!;
        Assert.Equal("det", entry["name"]!.GetValue<string>());
        Assert.Equal("Linear Algebra", entry["category"]!.GetValue<string>());
        Assert.Equal(1, entry["parameterCount"]!.GetValue<int>());
    }

    /// <summary>The listing must not carry the expensive half of the descriptor: a page that ships every
    /// summary and example is a page an agent cannot afford to ask for.</summary>
    [Fact]
    public async Task AListingCarriesNamesAndArity_NotDocumentation()
    {
        var (_, envelope) = await QueryAsync("--functions", "sin");

        JsonArray functions = envelope["functions"]!.AsArray();
        Assert.NotEmpty(functions);
        foreach (JsonNode? entry in functions)
        {
            JsonNode node = entry!;
            Assert.NotNull(node["name"]);
            Assert.NotNull(node["parameters"]);
            Assert.NotNull(node["minArity"]);
            Assert.NotNull(node["parameterCount"]);
            Assert.NotNull(node["variadic"]);
            Assert.NotNull(node["category"]);
            Assert.Null(node["summary"]);
            Assert.Null(node["examples"]);
            Assert.Null(node["related"]);
        }
    }

    [Fact]
    public async Task DescribingOneFunction_AnswersTheSameMetadataTheDescriptorDeclares()
    {
        SuiteEngine engine = LiveEngine();
        var (exitCode, envelope) = await QueryAsync("--function", "det");

        Assert.Equal(0, exitCode);
        JsonNode entry = envelope["function"]!;
        Assert.Null(envelope["functions"]);

        // `det` is a CORE builtin, so its descriptor lives in the core catalogue rather than in the
        // plugin-registered map — the same two-step resolution HelpService uses, and the same one
        // FunctionEntry now uses, which is the point: one resolution, three surfaces.
        Lovelace.Abstractions.BuiltinDescriptor descriptor =
            engine.InterpreterBuiltinDescriptors.TryGetValue("det", out Lovelace.Abstractions.BuiltinDescriptor? plugin)
                ? plugin
                : CoreBuiltinMetadata.TryGet("det")!;
        Assert.NotNull(descriptor);
        Assert.Equal(descriptor.Name, entry["name"]!.GetValue<string>());
        Assert.Equal(descriptor.Summary, entry["summary"]!.GetValue<string>());
        Assert.Equal(descriptor.ReturnKind, entry["returnKind"]!.GetValue<string>());
        Assert.Equal(descriptor.Category, entry["category"]!.GetValue<string>());
        Assert.Equal(
            descriptor.Parameters.ToArray(),
            entry["parameters"]!.AsArray().Select(p => p!.GetValue<string>()).ToArray());
        Assert.Equal(
            descriptor.Related.ToArray(),
            entry["related"]!.AsArray().Select(p => p!.GetValue<string>()).ToArray());
    }

    /// <summary>A page says it is a page. Without this an agent compares the array length against a limit
    /// it guessed, which is exactly the kind of inference the structured surface exists to remove.</summary>
    [Fact]
    public async Task APageReportsItsOwnTruncation()
    {
        var (_, page) = await QueryAsync("--functions", "--function-limit", "3");
        Assert.Equal(3, page["returned"]!.GetValue<int>());
        Assert.True(page["total"]!.GetValue<int>() > 3);
        Assert.True(page["truncated"]!.GetValue<bool>());

        // and a pattern that matches everything the limit allows is NOT reported as truncated
        var (_, whole) = await QueryAsync("--functions", "zzz-no-such-fragment");
        Assert.Equal(0, whole["total"]!.GetValue<int>());
        Assert.False(whole["truncated"]!.GetValue<bool>());
    }

    /// <summary>The listing is deterministic and ordinal: two queries over one registry cannot disagree,
    /// and a consumer may binary-search the answer.</summary>
    [Fact]
    public async Task AListingIsOrdinalByConstruction()
    {
        var (_, first) = await QueryAsync("--functions", "--function-limit", "200");
        string[] names = first["functions"]!.AsArray().Select(f => f!["name"]!.GetValue<string>()).ToArray();

        Assert.Equal(names.OrderBy(n => n, StringComparer.Ordinal).ToArray(), names);

        var (_, again) = await QueryAsync("--functions", "--function-limit", "200");
        Assert.Equal(
            names,
            again["functions"]!.AsArray().Select(f => f!["name"]!.GetValue<string>()).ToArray());
    }

    /// <summary>An unknown NAME is the SAME refusal a call to it produces — the agent that asks about a
    /// name it half-remembers gets the registry's suggestion, not an empty answer.</summary>
    [Fact]
    public async Task AnUnknownNameIsTheSameRefusalACallProduces()
    {
        var (exitCode, envelope) = await QueryAsync("--function", "determinant");

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>());
        Assert.Equal("UnknownFunction", envelope["code"]!.GetValue<string>());
        Assert.Equal("UnsupportedOperation", envelope["category"]!.GetValue<string>());
        Assert.Equal(
            new[] { "det" },
            envelope["suggestions"]!.AsArray().Select(s => s!.GetValue<string>()).ToArray());
        Assert.True(envelope["registrySize"]!.GetValue<int>() >= 100);
    }

    /// <summary>A user-defined function has no descriptor, and the answer says so by OMISSION rather than
    /// by publishing a placeholder sentence as if it were documentation.</summary>
    [Fact]
    public async Task TheEnvelope_StillCarriesTheFullRegistryForScripts()
    {
        // the enquiry surface is ADDITIVE: the envelope a script produces still carries the registry, with
        // the descriptor metadata on each entry
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(
            new[] { "--eval", "1 + 1" }, stdout, stderr, new StringReader(string.Empty));
        Assert.Equal(0, exitCode);

        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout.ToString(), "registry envelope");
        JsonNode det = envelope["functions"]!.AsArray().Single(f => f!["name"]!.GetValue<string>() == "det")!;
        Assert.Equal("Linear Algebra", det["category"]!.GetValue<string>());
        Assert.NotNull(det["summary"]);
    }
}