using System.Text.Json.Nodes;
using Lovelace.Dsp;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;

namespace Lovelace.Run.Tests;

/// <summary>
/// Every <c>functions[]</c> entry publishes the arity metadata the arity contract is COMPUTED
/// from, so an agent can tell which of a builtin's declared parameters are optional without
/// parsing an error message: <c>parameters</c> (the declared signature), <c>parameterCount</c>,
/// <c>minArity</c> (the resolved lower bound) and <c>variadic</c> (true when the last parameter may
/// repeat, which removes the upper bound) (A2-F15).
///
/// <para>The entries used to carry only <c>{name, parameters, builtin, plugin}</c>, so
/// <c>symbol(name [, domain])</c> and <c>solve(f, x [, domain])</c> looked exactly like builtins
/// whose whole signature is mandatory, and the fact that <c>concat([1],[2])</c> is accepted while
/// <c>eye(2,2,2)</c> is refused was not recoverable from the envelope.</para>
/// </summary>
public class FunctionArityMetadataTests
{
    /// <summary>The same plugin set the runner loads, so the registry compared against is the
    /// registry published.</summary>
    private static SuiteEngine LiveEngine()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    private static async Task<JsonArray> RegistryAsync()
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync("x = symbol(\"x\"); x + 1");
        Assert.True(exitCode == 0, $"registry probe exited {exitCode}. stderr: {stderr.Trim()}");
        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "registry envelope");
        return envelope["functions"]!.AsArray();
    }

    /// <summary>The property, over the LIVE registry: what the wire publishes IS what the engine
    /// declares, entry by entry, and the range is well formed for every entry.</summary>
    [Fact]
    public async Task EveryRegistryEntry_PublishesItsDeclaredArity()
    {
        JsonArray functions = await RegistryAsync();
        SuiteEngine engine = LiveEngine();

        Assert.NotEmpty(functions);
        Assert.Equal(engine.Functions.Count, functions.Count);

        foreach (JsonNode? entry in functions)
        {
            string name = entry!["name"]!.GetValue<string>();
            Assert.True(engine.Functions.TryGetValue(name, out Lovelace.Suite.FunctionDefinition? definition),
                $"the registry published '{name}', which the engine does not have");

            string[] parameters = entry["parameters"]!.AsArray().Select(p => p!.GetValue<string>()).ToArray();
            Assert.Equal(definition!.Parameters.ToArray(), parameters);

            Assert.True(entry["parameterCount"] is not null, $"'{name}': no parameterCount");
            Assert.True(entry["minArity"] is not null, $"'{name}': no minArity");
            Assert.True(entry["variadic"] is not null, $"'{name}': no variadic");

            int parameterCount = entry["parameterCount"]!.GetValue<int>();
            int minArity = entry["minArity"]!.GetValue<int>();
            bool variadic = entry["variadic"]!.GetValue<bool>();
            var (declaredMin, declaredMax) = definition.DeclaredArity();

            Assert.Equal(parameters.Length, parameterCount);
            Assert.Equal(declaredMin, minArity);
            Assert.Equal(definition.Variadic, variadic);
            // the published range is well formed and matches the declared upper bound
            Assert.InRange(minArity, 0, parameterCount);
            Assert.Equal(variadic ? int.MaxValue : parameterCount, declaredMax);
        }
    }

    /// <summary>The rows the finding names, pinned: an optional TRAILING parameter is visible as
    /// <c>minArity &lt; parameterCount</c>, and a repeating tail as <c>variadic</c>.</summary>
    [Fact]
    public async Task OptionalAndVariadicBuiltins_AreVisibleInTheRegistry()
    {
        JsonObject byName = Index(await RegistryAsync());

        AssertArity(byName, "solve", minArity: 2, parameterCount: 3, variadic: false);
        AssertArity(byName, "symbol", minArity: 1, parameterCount: 2, variadic: false);
        AssertArity(byName, "concat", minArity: 2, parameterCount: 3, variadic: false);
        AssertArity(byName, "dft", minArity: 1, parameterCount: 2, variadic: false);
        AssertArity(byName, "abs", minArity: 1, parameterCount: 1, variadic: false);
        AssertArity(byName, "print", minArity: 0, parameterCount: 1, variadic: true);
        AssertArity(byName, "zeros", minArity: 1, parameterCount: 1, variadic: true);

        // at least one entry of each shape exists, so the two rows above are not the only evidence
        Assert.Contains(byName, pair => pair.Value!["variadic"]!.GetValue<bool>());
        Assert.Contains(byName, pair => pair.Value!["minArity"]!.GetValue<int>() <
                                        pair.Value["parameterCount"]!.GetValue<int>());
    }

    /// <summary>The published range is the range the call site ENFORCES: two arguments to a
    /// three-parameter builtin with minArity 2 is accepted, one argument is refused with the
    /// documented recoverable argument error — computed from the entry, not guessed.</summary>
    [Fact]
    public async Task ThePublishedRange_IsTheRangeTheCallSiteEnforces()
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(
            "concat([1], [2])", "--omit-functions");
        Assert.True(exitCode == 0, $"concat with minArity arguments was refused: {stderr.Trim()}");
        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "concat envelope");
        Assert.True(envelope["ok"]!.GetValue<bool>());

        var (refusedExit, refusedStdout, _) = await TestSupport.RunScriptAsync(
            "concat([1])", "--omit-functions");
        Assert.Equal(1, refusedExit);
        JsonNode refused = TestSupport.ParseExactlyOneJsonDocument(refusedStdout, "arity refusal envelope");
        Assert.False(refused["ok"]!.GetValue<bool>());
        Assert.Equal("InvalidArgument", refused["code"]!.GetValue<string>());
        Assert.Equal("TypeMismatch", refused["category"]!.GetValue<string>());
    }

    private static JsonObject Index(JsonArray functions)
    {
        var index = new JsonObject();
        foreach (JsonNode? entry in functions)
            index[entry!["name"]!.GetValue<string>()] = entry!.DeepClone();
        return index;
    }

    private static void AssertArity(JsonObject byName, string name, int minArity, int parameterCount, bool variadic)
    {
        JsonNode? entry = byName[name];
        Assert.True(entry is not null, $"'{name}' is missing from the registry");
        Assert.Equal(minArity, entry!["minArity"]!.GetValue<int>());
        Assert.Equal(parameterCount, entry["parameterCount"]!.GetValue<int>());
        Assert.Equal(variadic, entry["variadic"]!.GetValue<bool>());
    }
}
