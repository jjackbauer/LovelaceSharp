using Lovelace.Abstractions;
using Lovelace.Dsp;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Discoverability contract: every user-facing builtin carries complete metadata, signatures
/// distinguish optional parameters from required ones, category aliases resolve, and a name
/// search works. No shipped configuration may render "(no summary registered)".
/// </summary>
public class HelpMetadataCompletenessTests
{
    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    [Fact]
    public void EveryBuiltin_HasCompleteMetadata()
    {
        var engine = NewEngine();
        var snapshot = engine.CaptureState();
        var holes = new List<string>();
        foreach (var fn in snapshot.Functions.Values.Where(f => f.IsBuiltin).OrderBy(f => f.Name, StringComparer.Ordinal))
        {
            var help = engine.Help.Function(fn.Name);
            if (help is null)
            {
                holes.Add(fn.Name + ": no help entry");
                continue;
            }
            if (help.Contains("(no summary registered)") || help.Contains("(no further help registered)"))
                holes.Add(fn.Name + ": no summary");
            else if (help.Contains("Returns: Value"))
                holes.Add(fn.Name + ": return kind not declared");
        }
        Assert.True(holes.Count == 0,
            "builtins with incomplete metadata: " + string.Join("; ", holes));
    }

    [Fact]
    public void Signature_DistinguishesOptionalParameters()
    {
        var engine = NewEngine();
        // domain is optional, so it must render as an optional group rather than a required one
        Assert.Contains("solve(f, x [, domain])", engine.Help.Function("solve")!);
        Assert.Contains("solve_full(f, x [, domain])", engine.Help.Function("solve_full")!);
        // a fully-required signature has no optional group
        Assert.Contains("diff(f, x)", engine.Help.Function("diff")!);
    }

    [Theory]
    [InlineData("symbolic")]
    [InlineData("symbolics")]
    [InlineData("calculus")]
    [InlineData("matrices")]
    [InlineData("matrix")]
    [InlineData("linear algebra")]
    [InlineData("mathir")]
    [InlineData("ir")]
    [InlineData("compilation")]
    [InlineData("solving")]
    [InlineData("optimization")]
    [InlineData("dsp")]
    [InlineData("introspection")]
    public void CategoryAliases_Resolve(string alias)
    {
        var engine = NewEngine();
        var text = engine.Help.Lookup(alias);
        Assert.NotNull(text);
        Assert.DoesNotContain("No category", text);
    }

    [Fact]
    public void Funcs_FallsBackToNameSearch()
    {
        var engine = NewEngine();
        var text = engine.Help.Funcs("solve");
        Assert.Contains("solve(", text);
        Assert.Contains("solve_full(", text);
        Assert.DoesNotContain("No category", text);
    }

    [Fact]
    public void Funcs_ListsEveryBuiltinUnderACategory()
    {
        var engine = NewEngine();
        var all = engine.Help.Funcs(null);
        Assert.Contains("Solving", all);
        Assert.Contains("Symbolics", all);
        Assert.Contains("DSP", all);
    }
}
