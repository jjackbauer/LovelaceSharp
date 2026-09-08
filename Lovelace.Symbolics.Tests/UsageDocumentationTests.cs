using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Executable usage documentation: every lovelace/result pair in
/// Lovelace.Symbolics/README.md is evaluated in a fresh engine with the symbolic
/// plugins loaded, and the documented output asserted exactly. If the usage guide drifts
/// from the engine, these tests fail until the guide (or the engine) is fixed.
/// </summary>
public class UsageDocumentationTests
{
    private static readonly string DocPath = Path.Combine(AppContext.BaseDirectory, "README.md");

    public static IEnumerable<object[]> Examples() =>
        UsageDocParser.Parse(DocPath).Select(e => new object[] { e.Script, e.Expected });

    [Theory]
    [MemberData(nameof(Examples))]
    public async Task DocumentedExample_MatchesEngine(string script, string expected)
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));

        string source = ScriptSource.ToSemicolonStatements(script);

        if (expected.StartsWith("error: ", StringComparison.Ordinal))
        {
            var message = expected["error: ".Length..];
            var ex = await Assert.ThrowsAnyAsync<Exception>(() => engine.EvaluateAsync(source));
            Assert.Equal(message, ex.Message);
            return;
        }

        if (expected.StartsWith("prints: ", StringComparison.Ordinal))
        {
            var text = expected["prints: ".Length..];
            var writer = new StringWriter();
            var result = await engine.EvaluateAsync(source, writer);
            Assert.Equal(ValueKind.Void, result.Kind);
            Assert.Equal(text, writer.ToString().TrimEnd('\r', '\n'));
            return;
        }

        var value = await engine.EvaluateAsync(source);
        Assert.Equal(expected, ValueFormatter.FormatTyped(value));
    }
}

/// <summary>Extracts the runnable examples and C# snippets from the usage guide.</summary>
public static class UsageDocParser
{
    public sealed record Example(string Script, string Expected);
    public sealed record Snippet(string Name, string Code);

    public static List<Example> Parse(string path)
    {
        var text = File.ReadAllText(path);
        var lines = text.Split('\n');
        var examples = new List<Example>();
        for (int i = 0; i < lines.Length; i++)
        {
            if (lines[i].TrimEnd('\r') == Fence("lovelace"))
            {
                var script = ReadBlock(lines, ref i);
                if (i + 1 < lines.Length && lines[i + 1].TrimEnd('\r') == Fence("result"))
                {
                    i++;
                    var expected = ReadBlock(lines, ref i);
                    examples.Add(new Example(script, expected));
                }
            }
        }
        return examples;
    }

    public static List<Snippet> ParseCSharp(string path)
    {
        var text = File.ReadAllText(path);
        var lines = text.Split('\n');
        var snippets = new List<Snippet>();
        for (int i = 0; i < lines.Length; i++)
        {
            if (lines[i].TrimEnd('\r') == Fence("csharp"))
            {
                var code = ReadBlock(lines, ref i);
                snippets.Add(new Snippet("snippet-" + snippets.Count, code));
            }
        }
        return snippets;
    }

    private static string Fence(string tag) => "```" + tag;

    private static string ReadBlock(string[] lines, ref int i)
    {
        var block = new List<string>();
        i++;
        while (i < lines.Length && lines[i].TrimEnd('\r') != "```")
        {
            block.Add(lines[i].TrimEnd('\r'));
            i++;
        }
        return string.Join('\n', block).Trim();
    }
}