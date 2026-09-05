using Lovelace.Suite;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Executable documentation: reads <c>Language.md</c> (copied to the test output), evaluates
/// every <c>lovelace</c> example in a fresh <see cref="SuiteEngine"/>, and asserts the
/// documented <c>result</c> matches the engine exactly. If the language drifts from its
/// reference, these tests fail until the reference is updated.
/// </summary>
public class LanguageDocumentationTests
{
    private static readonly string DocPath = Path.Combine(AppContext.BaseDirectory, "Language.md");

    public static IEnumerable<object[]> Examples() =>
        DocExampleParser.Parse(DocPath).Select(e => new object[] { e.Script, e.Expected });

    [Theory]
    [MemberData(nameof(Examples))]
    public async Task DocumentedExample_MatchesEngine(string script, string expected)
    {
        Assert.True(File.Exists(DocPath), $"Language.md not found at {DocPath}.");

        // The CLI hosts opt into the DSP extension, so the reference surface includes it.
        var engine = new SuiteEngine();
        engine.RegisterDspBuiltins();
        var tmp = Path.Combine(Path.GetTempPath(), "lovelace-doctest-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(tmp);

        try
        {
            engine.PlotOutputDirectory = tmp;
            engine.PlotFileName = "doctest.svg";

            string source = ScriptSource.ToSemicolonStatements(script);

            if (expected.StartsWith("error: ", StringComparison.Ordinal))
            {
                var message = expected["error: ".Length..];
                var ex = await Assert.ThrowsAnyAsync<Exception>(() => engine.EvaluateAsync(source));
                Assert.Equal(message, ex.Message);
                return;
            }

            if (expected.StartsWith("plot: ", StringComparison.Ordinal))
            {
                var title = expected["plot: ".Length..];
                await engine.EvaluateAsync(source);
                Assert.NotNull(engine.LastPlot);
                Assert.Equal(title, engine.LastPlot!.Title);
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
        finally
        {
            Directory.Delete(tmp, recursive: true);
        }
    }
}

/// <summary>Extracts runnable examples from the language reference markdown.</summary>
public static class DocExampleParser
{
    public sealed record Example(string Name, string Script, string Expected);

    public static List<Example> Parse(string path)
    {
        var lines = File.ReadAllLines(path);
        var examples = new List<Example>();
        string heading = "example";

        for (int i = 0; i < lines.Length; i++)
        {
            var trimmed = lines[i].Trim();

            if (trimmed.StartsWith('#'))
            {
                heading = trimmed.TrimStart('#').Trim();
                continue;
            }

            if (FenceInfo(trimmed) != "lovelace")
                continue;

            var script = ReadBody(lines, ref i);

            // The next non-blank line must be the matching result fence.
            int j = i + 1;
            while (j < lines.Length && string.IsNullOrWhiteSpace(lines[j]))
                j++;

            if (j >= lines.Length || FenceInfo(lines[j].Trim()) != "result")
                throw new InvalidOperationException(
                    $"Language.md: the `lovelace` block under '{heading}' is not followed by a `result` block.");

            i = j; // point at the result fence
            var expected = ReadBody(lines, ref i);

            examples.Add(new Example(heading, script, expected.TrimEnd()));
        }

        return examples;
    }

    /// <summary>Returns the info string of a 3-backtick fence, or <c>null</c> if not one.</summary>
    private static string? FenceInfo(string trimmed)
    {
        if (!trimmed.StartsWith("```") || (trimmed.Length > 3 && trimmed[3] == '`'))
            return null;
        return trimmed[3..].Trim().ToLowerInvariant();
    }

    /// <summary>Reads a fence body; on return <paramref name="i"/> is at the closing fence.</summary>
    private static string ReadBody(string[] lines, ref int i)
    {
        i++; // skip opening fence
        var body = new List<string>();
        while (i < lines.Length && !lines[i].TrimStart().StartsWith("```"))
        {
            body.Add(lines[i]);
            i++;
        }
        return string.Join("\n", body).TrimEnd();
    }
}
