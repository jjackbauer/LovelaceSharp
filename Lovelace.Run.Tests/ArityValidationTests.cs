using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// Arity is DECLARED metadata (<see cref="Lovelace.Abstractions.BuiltinDescriptor.MinArity"/>,
/// <c>Variadic</c>, <c>Parameters</c>) and must be enforced at the call site, before a builtin body
/// can index a missing argument. The failure has to be structural on the wire: recoverable, a
/// non-internal code and category, and a message naming the builtin and both counts.
/// </summary>
public class ArityValidationTests
{
    /// <summary>Runs the PUBLISHED runner (the same entry point the AOT binary uses) over an
    /// inline script and parses the one JSON envelope it emits.</summary>
    private static async Task<(int ExitCode, JsonNode Envelope)> RunAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(
            new[] { "--eval", script, "--omit-functions", "--omit-variables" }, stdout, stderr);
        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(stdout.ToString(), $"stdout of '{script}'"));
    }

    [Fact]
    public async Task TooFewArguments_IsATypedStructuralError_NeverAnInternalFailure()
    {
        var (exitCode, envelope) = await RunAsync("x = symbol(\"x\"); compile_full(x^2 + 1)");

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), "a rejected call reports ok == false");

        // the classification is structural: a consumer branches on code/category, never on prose
        Assert.NotEqual("InternalError", envelope["code"]!.GetValue<string>());
        Assert.NotEqual("InternalInvariantFailure", envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>(),
            "a wrong argument count is a user mistake the caller can fix and retry");

        string message = envelope["message"]!.GetValue<string>();
        Assert.Equal("compile_full(): expected 2 arguments; got 1.", message);
    }

    [Fact]
    public async Task DeclaredArgumentCount_StillSucceeds()
    {
        var (exitCode, envelope) = await RunAsync("x = symbol(\"x\"); compile_full(x^2 + 1, [x])");

        Assert.Equal(0, exitCode);
        Assert.True(envelope["ok"]!.GetValue<bool>());
        Assert.Equal("CompilationResult", envelope["result"]!["structured"]!["type"]!.GetValue<string>());
    }

    /// <summary>Optional trailing parameters are DECLARED metadata too: a descriptor that names a
    /// parameter beyond <c>MinArity</c> must stay callable in the shorter form. <c>solve(f, x)</c>
    /// (domain omitted), <c>symbol("x")</c> (domain omitted) and <c>dft(x)</c> (length omitted) are
    /// the three shipped shapes.</summary>
    [Theory]
    [InlineData("x = symbol(\"x\"); solve(x^2 - 4 == 0, x)")]
    [InlineData("x = symbol(\"x\"); type(x)")]
    [InlineData("dft([0, 1, 0, 0])")]
    public async Task OptionalTrailingParameters_StayCallableInTheShorterForm(string script)
    {
        var (exitCode, envelope) = await RunAsync(script);

        Assert.True(exitCode == 0, $"'{script}' exited {exitCode}: {envelope.ToJsonString()}");
        Assert.True(envelope["ok"]!.GetValue<bool>(), $"'{script}' must evaluate");
    }
}
