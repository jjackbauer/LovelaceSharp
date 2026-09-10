using Lovelace.Abstractions;
using Lovelace.Dsp;
using Lovelace.MathIR;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// DX-convergence Phase 5 contract tests: the plugin-aware help/funcs surface derived from
/// the builtin descriptor registry (the product contracts from the alignment plan §7.2).
/// </summary>
public class HelpServiceTests
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
    public void Help_Overview_ListsAllCategories()
    {
        var engine = NewEngine();
        var overview = engine.Help.Overview();
        // every category that actually has functions is listed with its count
        var listed = engine.Help.Funcs(null);
        foreach (var category in BuiltinCategories.Order.Where(c => listed.Contains(c, StringComparison.Ordinal)))
            Assert.Contains(category, overview);
        Assert.Contains("Symbolics", overview);
        Assert.Contains("Solving", overview);
        Assert.Contains("help <function>", overview);
    }

    [Fact]
    public void Help_Symbolics_ListsCoreFunctions()
    {
        var engine = NewEngine();
        var text = engine.Help.Category("symbolics");
        foreach (var name in new[] { "symbol(", "simplify(", "expand(", "factor(" })
            Assert.Contains(name, text);
    }

    [Fact]
    public void Help_Solve_ShowsSignatureSummaryExampleAndSeeAlso()
    {
        var engine = NewEngine();
        var text = engine.Help.Function("solve")!;
        // the domain parameter is optional, and the signature says so
        Assert.Contains("solve(f, x [, domain])", text);
        Assert.Contains("Solve", text);
        Assert.Contains("solve(x^2 - 4 == 0, x)", text);
        Assert.Contains("Returns: Vector | Text", text);
        Assert.Contains("See also:", text);
        Assert.Contains("solve_full", text);
    }

    [Fact]
    public void Help_Compile_ShowsDescriptor()
    {
        var engine = NewEngine();
        var text = engine.Help.Function("compile")!;
        Assert.Contains("compile(f, params)", text);
        Assert.Contains("MathIR", text);
        Assert.Contains("compile_full", text);
    }

    [Fact]
    public void Funcs_Calculus_ListsCalculusFunctions()
    {
        var engine = NewEngine();
        var text = engine.Help.Funcs("calculus");
        foreach (var name in new[] { "diff(", "integrate(", "series(", "limit(" })
            Assert.Contains(name, text);
    }

    [Fact]
    public void Funcs_All_IsCategorized()
    {
        var engine = NewEngine();
        var text = engine.Help.Funcs(null);
        Assert.Contains(BuiltinCategories.Symbolics, text);
        Assert.Contains(BuiltinCategories.Calculus, text);
        Assert.Contains(BuiltinCategories.Compilation, text);
        Assert.Contains(BuiltinCategories.Dsp, text);
    }

    [Fact]
    public void Help_UnknownArgument_Explains()
    {
        var engine = NewEngine();
        var text = engine.Help.Lookup("nonsense")!;
        Assert.Contains("No category", text);
    }

    [Fact]
    public void UnicodeOutput_ThreadsThroughEngineFormatting()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var inf = engine.Evaluate("inf");
        Assert.Equal("inf (Symbolic)", engine.FormatValueTyped(inf));
        engine.UnicodeOutput = true;
        Assert.Equal("∞ (Symbolic)", engine.FormatValueTyped(inf));
        engine.UnicodeOutput = false;
        Assert.Equal("inf (Symbolic)", engine.FormatValueTyped(inf));
    }
}
