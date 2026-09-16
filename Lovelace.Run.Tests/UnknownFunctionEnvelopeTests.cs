using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// The unknown-function envelope on the wire: an unresolved NAME is a vocabulary failure with its own
/// code and category, and the registry's answer — the closest registered names, any syntax note, and
/// how many names exist — travels with it. Nothing that has nothing to offer changes shape: a name no
/// registered name is close to keeps the message it always had and omits the optional fields.
/// </summary>
public class UnknownFunctionEnvelopeTests
{
    private static async Task<(int ExitCode, string Stdout, string Stderr, JsonNode Envelope)> RunAsync(
        params string[] extraArguments)
    {
        var arguments = new List<string> { "--eval", "determinant([[1,2],[3,4]])", "--omit-functions" };
        arguments.AddRange(extraArguments);

        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(arguments.ToArray(), stdout, stderr);
        string raw = stdout.ToString();
        return (exitCode, raw, stderr.ToString(), TestSupport.ParseExactlyOneJsonDocument(raw, "runner stdout"));
    }

    private static async Task<JsonNode> RunScriptAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        await Runner.RunAsync(new[] { "--eval", script, "--omit-functions", "--omit-variables" },
            stdout, stderr);
        return TestSupport.ParseExactlyOneJsonDocument(stdout.ToString(), "runner stdout");
    }

    [Fact]
    public async Task AnUnknownName_CrossesAsNameResolution_WithTheRegistrysAnswer()
    {
        var (exitCode, raw, _, envelope) = await RunAsync();

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. stdout: {raw}");
        Assert.Equal("UnknownFunction", envelope["code"]!.GetValue<string>());
        Assert.Equal("UnsupportedOperation", envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>(),
            "a caller can use a name that exists and retry");

        var suggestions = envelope["suggestions"]!.AsArray().Select(node => node!.GetValue<string>()).ToArray();
        Assert.Contains("det", suggestions);
        Assert.True(envelope["registrySize"]!.GetValue<int>() > 0,
            "the envelope says how many names the engine does have");
        Assert.Contains("Did you mean", envelope["message"]!.GetValue<string>());
        // the diagnostics array is what an L1 caller reads; it repeats the same answer
        Assert.Contains("Did you mean",
            envelope["diagnostics"]![0]!["message"]!.GetValue<string>());
    }

    [Fact]
    public async Task ASyntaxName_CarriesANoteAndNoFunctionSuggestion()
    {
        var envelope = await RunScriptAsync("range(5)");

        Assert.Equal("UnknownFunction", envelope["code"]!.GetValue<string>());
        Assert.Contains("1..n", envelope["hints"]![0]!.GetValue<string>());
        Assert.Contains("1..n", envelope["message"]!.GetValue<string>());
        Assert.Null(envelope["suggestions"]);
    }

    [Fact]
    public async Task ANameNothingIsCloseTo_KeepsItsMessageAndOmitsTheOptionalFields()
    {
        var envelope = await RunScriptAsync("nosuchfn(1)");

        Assert.Equal("UnknownFunction", envelope["code"]!.GetValue<string>());
        Assert.Equal("Unknown function 'nosuchfn'.", envelope["message"]!.GetValue<string>());
        Assert.Null(envelope["suggestions"]);
        Assert.Null(envelope["hints"]);
    }

    [Fact]
    public async Task TheTextSurface_CarriesTheSameAnswer()
    {
        // --text writes no JSON to stdout, so this path is driven without the envelope parser.
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(
            new[] { "--eval", "determinant([[1,2],[3,4]])", "--omit-functions", "--omit-variables", "--text" },
            stdout, stderr);

        Assert.Equal(1, exitCode);
        Assert.Contains("Error [UnknownFunction/UnsupportedOperation]", stderr.ToString());
        Assert.Contains("Did you mean 'det'?", stderr.ToString());
    }
}
