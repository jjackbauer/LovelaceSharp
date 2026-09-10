using Lovelace.Abstractions;
using Lovelace.Dsp;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit.Abstractions;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Executable documentation for the builtin descriptors. <see cref="BuiltinDescriptor.Examples"/>
/// promises "executable lovelace snippets (kept in sync with the doctests)"; this suite is what
/// makes that literal: every example a user can read through <c>help &lt;function&gt;</c> — the
/// core metadata table plus the Dsp / Symbolics / MathIR plugin descriptors — is evaluated
/// against a fresh engine primed with the documented prelude
/// (<see cref="HelpService.ExamplePreludeSource"/>). A snippet that does not run is a help-text
/// bug, and this test names the function, the snippet and the failure.
/// </summary>
public class DescriptorExampleTests
{
    /// <summary>One runnable descriptor example, attributed to its function and — when the
    /// function comes from an extension — the plugin that registered it.</summary>
    public sealed record DescriptorExample(string Function, string? Plugin, string Source);

    private readonly ITestOutputHelper _output;

    public DescriptorExampleTests(ITestOutputHelper output) => _output = output;

    /// <summary>Wires the engine exactly as the CLI hosts do (Lovelace.Run/Program.cs): the
    /// suite plus the Dsp, Symbolics and MathIR plugins.</summary>
    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    /// <summary>
    /// Every example the help surface can print, in deterministic (function, then source) order.
    /// Descriptors resolve exactly as <see cref="HelpService"/> resolves them — the descriptor the
    /// plugin registered at load wins over <see cref="CoreBuiltinMetadata"/> — and the core table
    /// is enumerated in full, so a metadata entry whose function never registered is still run
    /// (and still fails loudly) rather than being silently skipped.
    /// </summary>
    public static IReadOnlyList<DescriptorExample> Corpus()
    {
        var engine = NewEngine();

        var names = new SortedSet<string>(StringComparer.Ordinal);
        foreach (var (name, fn) in engine.Functions)
        {
            if (fn.IsBuiltin)
                names.Add(name);
        }
        foreach (var descriptor in CoreBuiltinMetadata.Descriptors)
            names.Add(descriptor.Name);

        var corpus = new List<DescriptorExample>();
        foreach (var name in names)
        {
            string? plugin = engine.Functions.TryGetValue(name, out var fn) && fn.IsBuiltin
                ? fn.PluginName
                : null;
            var descriptor = engine.InterpreterBuiltinDescriptors.TryGetValue(name, out var registered)
                ? registered
                : CoreBuiltinMetadata.TryGet(name);
            if (descriptor is null)
                continue;
            foreach (var example in descriptor.Examples)
                corpus.Add(new DescriptorExample(name, plugin, example));
        }
        return corpus;
    }

    public static IEnumerable<object[]> Examples() =>
        Corpus().Select(e => new object[] { e.Function, e.Source });

    /// <summary>Guards the corpus itself: an enumeration that silently loses a plugin or the core
    /// table would let every example "pass" by never running.</summary>
    [Fact]
    public void Corpus_ReachesCoreMetadataAndEveryPlugin()
    {
        var corpus = Corpus();
        int core = corpus.Count(e => e.Plugin is null);
        var byPlugin = corpus.Where(e => e.Plugin is not null)
            .GroupBy(e => e.Plugin!, StringComparer.Ordinal)
            .OrderBy(g => g.Key, StringComparer.Ordinal)
            .Select(g => $"{g.Key}={g.Count()}")
            .ToList();
        string summary = $"descriptor examples: {corpus.Count} (core={core}, {string.Join(", ", byPlugin)})";
        _output.WriteLine(summary);

        Assert.True(corpus.Count > 0, "no descriptor examples were discovered; " + summary);
        Assert.True(core > 0, "no core metadata examples were discovered; " + summary);
        foreach (var plugin in new[] { "Lovelace.Dsp", "Lovelace.Symbolics", "Lovelace.MathIR" })
            Assert.True(corpus.Any(e => e.Plugin == plugin), $"no examples discovered from {plugin}; " + summary);
    }

    [Theory]
    [MemberData(nameof(Examples))]
    public async Task Example_Executes(string function, string example)
    {
        var engine = NewEngine();
        string dir = Path.Combine(Path.GetTempPath(), "lovelace-descriptor-doctest-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);
        try
        {
            engine.PlotOutputDirectory = dir;
            engine.PlotFileName = "doctest.svg";

            // One fresh engine per example: the prelude is evaluated once and nothing an example
            // defines (assumptions, k = compile(...), precision) can leak into the next one.
            await engine.EvaluateAsync(HelpService.ExamplePreludeSource);

            try
            {
                await engine.EvaluateAsync(ScriptSource.ToSemicolonStatements(example));
            }
            catch (Exception ex)
            {
                string diagnostics = string.Join("; ", engine.Diagnostics.Select(d => d.ToString()));
                Assert.Fail(
                    $"the descriptor example for '{function}' does not execute: {example}" +
                    Environment.NewLine +
                    $"  {ex.GetType().Name}: {ex.Message}" +
                    (diagnostics.Length > 0 ? Environment.NewLine + "  diagnostics: " + diagnostics : string.Empty));
            }
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }
}
