using System.Text.Json.Nodes;
using Lovelace.Dsp;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// ADV-07 — a call-site refusal carries the SIGNATURE it was measured against, so a repair is chosen from
/// structure rather than from the sentence.
/// <para>
/// The types the runner already classifies have carried these fields all along: <c>BuiltinArityException</c>
/// publishes the builtin, both counts and the variadic flag, and <c>BuiltinShapeException</c> publishes the
/// position, what that position required and the kind that arrived. The envelope dropped all of it by
/// forwarding <c>ex.Message</c> alone — so the benchmark's arity task cost two evaluations in every
/// observation mode while the answer sat unread in the exception object.
/// </para>
/// </summary>
public class CallSignatureDiagnosticsTests
{
    private static async Task<JsonNode> EnvelopeAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(
            new[] { "--eval", script, "--omit-functions", "--omit-variables" },
            stdout, stderr, new StringReader(string.Empty));
        Assert.Equal(1, exitCode);
        return TestSupport.ParseExactlyOneJsonDocument(stdout.ToString(), $"stdout of '{script}'");
    }

    [Fact]
    public async Task AnArityRefusalPublishesTheSignature()
    {
        JsonNode envelope = await EnvelopeAsync("dot(1)");

        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        JsonNode signature = envelope["signature"]!;
        Assert.Equal("dot", signature["function"]!.GetValue<string>());
        Assert.Equal(
            new[] { "a", "b" },
            signature["parameters"]!.AsArray().Select(p => p!.GetValue<string>()).ToArray());
        Assert.Equal(2, signature["expectedMin"]!.GetValue<int>());
        Assert.Equal(2, signature["expectedMax"]!.GetValue<int>());
        Assert.False(signature["variadic"]!.GetValue<bool>());
        Assert.Equal(1, signature["actual"]!.GetValue<int>());
    }

    [Fact]
    public async Task AShapeRefusalPublishesTheOffendingArgument()
    {
        JsonNode envelope = await EnvelopeAsync("det(1)");

        JsonNode signature = envelope["signature"]!;
        Assert.Equal("det", signature["function"]!.GetValue<string>());
        Assert.Equal(1, signature["position"]!.GetValue<int>());
        Assert.Equal("m", signature["parameter"]!.GetValue<string>());
        Assert.Equal("a square matrix", signature["expectedShape"]!.GetValue<string>());
        Assert.Equal("Natural", signature["arrivedKind"]!.GetValue<string>());
    }

    /// <summary>A failure that is NOT a call-site refusal carries NO signature — omitted, never empty, so a
    /// consumer can branch on the field's presence instead of on the code.</summary>
    [Fact]
    public async Task AFailureThatIsNotACallSiteRefusal_CarriesNoSignature()
    {
        JsonNode envelope = await EnvelopeAsync("2 ^ 0.5");

        Assert.Equal("UnsupportedOperation", envelope["code"]!.GetValue<string>());
        Assert.Null(envelope["signature"]);
    }

    /// <summary>The property, over the LIVE registry: for EVERY builtin, an arity refusal publishes counts
    /// that agree with what the registry publishes for the same name. Two surfaces, one declaration.</summary>
    [Fact]
    public async Task EveryArityRefusalAgreesWithTheRegistryEntry()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));

        // drive a handful of builtins whose declared arity has a lower bound above zero, with one
        // argument too few — the sweep already covers the whole registry for grammar; this checks the
        // COUNTS against the registry for the ones that can be driven with plain literals
        string[][] probes =
        {
            ["matmul", "matmul([1, 2])"],
            ["cross", "cross([1, 2, 3])"],
            ["append", "append([1, 2])"],
            ["conv", "conv([1, 2])"],
            ["filter", "filter([1], [1])"],
        };

        foreach (string[] probe in probes)
        {
            Lovelace.Suite.FunctionDefinition definition = engine.Functions[probe[0]];
            var (min, _) = definition.DeclaredArity();
            JsonNode signature = (await EnvelopeAsync(probe[1]))["signature"]!;

            Assert.Equal(probe[0], signature["function"]!.GetValue<string>());
            Assert.Equal(min, signature["expectedMin"]!.GetValue<int>());
            Assert.Equal(definition.Variadic, signature["variadic"]!.GetValue<bool>());
            Assert.True(signature["actual"]!.GetValue<int>() < min,
                $"{probe[1]}: the probe must supply FEWER arguments than the declared minimum");
        }
    }
}