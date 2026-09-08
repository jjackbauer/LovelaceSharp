using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Keeps the usage guide honest about its library examples: every C# snippet in
/// Lovelace.Symbolics/README.md must appear verbatim (whitespace-normalized) inside
/// UsageExamples.cs, which compiles and runs each one as an asserting fact.
/// </summary>
public class DocsSyncTests
{
    private static readonly string DocPath = Path.Combine(AppContext.BaseDirectory, "README.md");
    private static readonly string ExamplesPath = Path.Combine(AppContext.BaseDirectory, "UsageExamples.cs");

    [Fact]
    public void Every_CSharpSnippet_AppearsInUsageExamples()
    {
        Assert.True(File.Exists(DocPath), "README.md not copied to the test output.");
        Assert.True(File.Exists(ExamplesPath), "UsageExamples.cs not copied to the test output.");

        var snippets = UsageDocParser.ParseCSharp(DocPath);
        Assert.NotEmpty(snippets);

        var source = Normalize(File.ReadAllText(ExamplesPath));
        foreach (var snippet in snippets)
        {
            var code = Normalize(snippet.Code);
            Assert.True(
                source.Contains(code, StringComparison.Ordinal),
                $"The C# snippet {snippet.Name} in README.md does not appear in UsageExamples.cs:\n{snippet.Code}");
        }
    }

    [Fact]
    public void Every_LovelaceExample_HasAnExpectedResult()
    {
        var examples = UsageDocParser.Parse(DocPath);
        Assert.NotEmpty(examples);
        foreach (var e in examples)
            Assert.False(string.IsNullOrWhiteSpace(e.Expected));
    }

    private static string Normalize(string s) =>
        string.Concat(s.Where(c => !char.IsWhiteSpace(c)));
}