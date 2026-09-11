using System.Reflection;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using Lovelace.Abstractions;
using Lovelace.Dsp;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;
using SuiteFunctionDefinition = Lovelace.Suite.FunctionDefinition;

namespace Lovelace.Run.Tests;

/// <summary>
/// The arity contract of docs/symbolics/dsh-protocol.md:187-199 ("A call with the wrong number of
/// arguments is rejected at the call site ... a recoverable argument error (code: InvalidArgument,
/// category: TypeMismatch) naming the builtin and both counts ... The counts come from the builtin's
/// declared metadata (Parameters, MinArity, Variadic)"), proved as a PROPERTY over the LIVE registry
/// rather than with a hand-written list of names.
/// <para>
/// Every registered builtin — core and plugin alike — is driven with every argument count in
/// <c>0..max+2</c>, where the declared range <c>[min, max]</c> is computed from the engine's own
/// metadata: the registered parameter list is the upper bound, <c>MinArity</c> the lower bound and
/// <c>Variadic</c> removes the upper bound. A count outside that range MUST cross as the one
/// documented shape; a count inside it MUST NOT be refused as a wrong-arity call. The two
/// directions matter: the first fails when the validator is missing (the pre-fix bug), the second
/// fails when the validator is widened to accept everything.
/// </para>
/// </summary>
public class ArityPropertyTests
{
    /// <summary>The registry the published runner wires: core builtins plus the three plugins
    /// (Lovelace.Run/Program.cs is the reference wiring).</summary>
    private static SuiteEngine NewRegistry() 
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    private static async Task<JsonNode> RunAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        await Runner.RunAsync(new[] { "--eval", script, "--omit-functions", "--omit-variables" }, stdout, stderr);
        return TestSupport.ParseExactlyOneJsonDocument(stdout.ToString(), $"stdout of '{script}'");
    }

    private static string Call(string builtin, int count) =>
        count == 0 ? $"{builtin}()" : $"{builtin}({string.Join(", ", Enumerable.Repeat("1", count))})";

    /// <summary>
    /// Reads a property of <see cref="FunctionDefinition"/> by name. The declared arity travels with
    /// the function after the fix; reading it reflectively keeps this file compilable against the
    /// pre-fix control tree, where the properties do not exist and the registered parameter list is
    /// then taken as the whole declaration — which is exactly the declaration the pre-fix engine
    /// fails to honour.
    /// </summary>
    private static object? Declaration(SuiteFunctionDefinition fn, string property) =>
        typeof(SuiteFunctionDefinition).GetProperty(property, BindingFlags.Public | BindingFlags.Instance)?.GetValue(fn);

    /// <summary>The engine's DECLARED argument-count range for a builtin.</summary>
    private static (int Min, int Max) DeclaredArity(SuiteFunctionDefinition fn, BuiltinDescriptor? descriptor)
    {
        int declared = fn.Parameters.Count;
        int minArity;
        bool variadic;
        if (descriptor is not null)
        {
            minArity = descriptor.MinArity;
            variadic = descriptor.Variadic;
        }
        else
        {
            // literal names on purpose: this file must also COMPILE against the pre-fix control
            // tree, where FunctionDefinition does not expose these properties at all
            minArity = Declaration(fn, "MinArity") as int? ?? -1;
            variadic = Declaration(fn, "Variadic") as bool? ?? false;
        }

        int min = minArity >= 0 ? Math.Min(minArity, declared) : declared;
        return (min, variadic ? int.MaxValue : declared);
    }

    private static string ExpectedMessage(string builtin, int min, int max, int actual)
    {
        string expected = max == int.MaxValue
            ? $"at least {min} {(min == 1 ? "argument" : "arguments")}"
            : min == max
                ? $"{min} {(min == 1 ? "argument" : "arguments")}"
                : $"{min} to {max} arguments";
        return $"{builtin}(): expected {expected}; got {actual}.";
    }

    private static readonly Regex AnyArityMessage =
        new(@"^[A-Za-z_][A-Za-z0-9_]*\(\): expected .*; got \d+\.$", RegexOptions.Compiled);

    [Fact]
    public async Task EveryBuiltin_RefusesEveryCountOutsideItsDeclaredArity_WithTheOneDocumentedShape()
    {
        var engine = NewRegistry();
        var registry = engine.Functions.Values.Where(fn => fn.IsBuiltin)
            .OrderBy(fn => fn.Name, StringComparer.Ordinal)
            .ToArray();

        // the property is only meaningful over a non-trivial registry
        Assert.True(registry.Length >= 100, $"the live registry exposed {registry.Length} builtins");

        var violations = new List<string>();
        var noWrongArityExists = new List<string>();
        var probed = 0;

        foreach (var fn in registry)
        {
            Assert.True(Regex.IsMatch(fn.Name, "^[A-Za-z_][A-Za-z0-9_]*$"),
                $"'{fn.Name}' is not a plain identifier, so the registry cannot be driven by name");

            engine.InterpreterBuiltinDescriptors.TryGetValue(fn.Name, out var descriptor);
            var (min, max) = DeclaredArity(fn, descriptor);
            int probeMax = max == int.MaxValue ? fn.Parameters.Count + 2 : max + 2;
            int refusals = 0;

            for (int count = 0; count <= probeMax; count++)
            {
                probed++;
                JsonNode envelope = await RunAsync(Call(fn.Name, count));
                bool ok = envelope["ok"]!.GetValue<bool>();
                string message = envelope["message"]?.GetValue<string>() ?? string.Empty;
                bool refusedAsArity = !ok && AnyArityMessage.IsMatch(message);

                bool outside = count < min || count > max;
                if (outside)
                {
                    if (!refusedAsArity)
                    {
                        violations.Add(
                            $"{fn.Name}(): {count} argument(s) is OUTSIDE the declared range [{min}, {max}] but the call " +
                            $"was not refused as a wrong-arity call — ok={ok}, code={envelope["code"]?.GetValue<string>() ?? "-"}, " +
                            $"category={envelope["category"]?.GetValue<string>() ?? "-"}, message='{message}'");
                        continue;
                    }

                    refusals++;
                    Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
                    Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
                    Assert.True(envelope["recoverable"]!.GetValue<bool>(),
                        $"{fn.Name}(): an argument count is a caller mistake the caller can fix and retry");
                    Assert.Equal(ExpectedMessage(fn.Name, min, max, count), message);
                }
                else if (refusedAsArity)
                {
                    violations.Add(
                        $"{fn.Name}(): {count} argument(s) is INSIDE the declared range [{min}, {max}] but the call was " +
                        $"refused as a wrong-arity call: '{message}'");
                }
            }

            // a builtin whose declared range covers every count (0..n) has no wrong-arity call to
            // refuse at all — the declaration says so, it is not the validator being widened
            if (refusals == 0)
                noWrongArityExists.Add($"{fn.Name} (declared [{min}, {max}])");
        }

        Assert.True(violations.Count == 0,
            $"{violations.Count} violation(s) across the {registry.Length} registered builtins ({probed} probes) of the " +
            "documented arity contract:" + Environment.NewLine + string.Join(Environment.NewLine, violations.Take(40)));

        // print is the ONE builtin that legitimately accepts every count (0..n values followed by a
        // newline), which its metadata declares as MinArity 0 + Variadic. Any other builtin with no
        // wrong-arity call would mean the declaration had been widened to dodge the validator.
        Assert.Equal(new[] { "print" }, noWrongArityExists
            .Select(entry => entry.Split(' ')[0])
            .ToArray());
    }

    /// <summary>
    /// The complement of the property: a builtin that legitimately accepts a SHORTER or a LONGER
    /// form than its parameter list declares keeps working, because its DECLARATION says so
    /// (MinArity / Variadic) rather than the validator being widened. Every entry here is a
    /// documented call form of the descriptor's own examples.
    /// </summary>
    [Theory]
    [InlineData("pi()")]
    [InlineData("e()")]
    [InlineData("zeros(2, 2)")]
    [InlineData("ones(3)")]
    [InlineData("eye(3)")]
    [InlineData("eye(2, 3)")]
    [InlineData("transpose([[1, 2], [3, 4]])")]
    [InlineData("sum([1, 2, 3])")]
    [InlineData("prod([1, 2, 3])")]
    [InlineData("min([3, 1, 2])")]
    [InlineData("max([3, 1, 2])")]
    [InlineData("mean([1, 2, 3])")]
    [InlineData("norm([3, 4])")]
    [InlineData("reshape([1, 2, 3, 4], 2, 2)")]
    [InlineData("concat([1, 2], [3, 4])")]
    [InlineData("print(\"A\", \"B\")")]
    [InlineData("plot([1, 2, 3])")]
    [InlineData("symbol(\"x\")")]
    [InlineData("x = symbol(\"x\"); solve(x^2 - 4 == 0, x)")]
    [InlineData("x = symbol(\"x\"); solve_full(x^2 - 4 == 0, x)")]
    [InlineData("dft([0, 1, 0, 0])")]
    [InlineData("noise(1, 0, 4)")]
    [InlineData("noise(1, 0, 4, 3)")]
    public async Task ADocumentedShorterOrLongerForm_StaysCallable(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        string plotDirectory = Path.Combine(Path.GetTempPath(), "lovelace-arity-plot");
        int exitCode = await Runner.RunAsync(
            new[] { "--eval", script, "--omit-functions", "--omit-variables", "--plot-dir", plotDirectory },
            stdout, stderr);

        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout.ToString(), $"stdout of '{script}'");
        Assert.True(exitCode == 0, $"'{script}' exited {exitCode}: {envelope.ToJsonString()}");
        Assert.True(envelope["ok"]!.GetValue<bool>(), $"'{script}' must still evaluate: {envelope.ToJsonString()}");
    }

    /// <summary>
    /// The declaration has to TRAVEL with the function, otherwise the call-site validator cannot be
    /// the one source: a plugin builtin registered with a descriptor must carry that descriptor's
    /// MinArity and Variadic on its own definition.
    /// </summary>
    [Fact]
    public void DeclaredArity_TravelsWithEveryPluginBuiltin()
    {
        var engine = NewRegistry();
        var checkedBuiltins = 0;

        foreach (var (name, fn) in engine.Functions.OrderBy(kv => kv.Key, StringComparer.Ordinal))
        {
            if (!fn.IsBuiltin || !engine.InterpreterBuiltinDescriptors.TryGetValue(name, out var descriptor))
                continue;

            checkedBuiltins++;
            // literal names: FunctionDefinition.MinArity/Variadic are read reflectively so this test
            // also compiles against the pristine control tree
            Assert.True(Declaration(fn, "MinArity") is int,
                $"{name}: the descriptor's declared MinArity ({descriptor.MinArity}) is not carried by the function, " +
                "so the one call-site validator cannot enforce what the descriptor declares");
            Assert.True(Declaration(fn, "Variadic") is bool,
                $"{name}: the descriptor's declared Variadic ({descriptor.Variadic}) is not carried by the function");
            Assert.Equal(descriptor.MinArity, (int)Declaration(fn, "MinArity")!);
            Assert.Equal(descriptor.Variadic, (bool)Declaration(fn, "Variadic")!);
        }

        Assert.True(checkedBuiltins >= 40, $"expected the plugin descriptors to cover the plugin registry, saw {checkedBuiltins}");
    }
}
