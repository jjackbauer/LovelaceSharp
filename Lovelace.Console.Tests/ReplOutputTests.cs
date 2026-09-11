using Lovelace.Console.Repl;

namespace Lovelace.Console.Tests;

/// <summary>
/// Output-text tests for the interactive REPL. Each test drives a real
/// <see cref="Lovelace.Console.Repl.ReplSession"/> headlessly (injected
/// <see cref="StringReader"/>/<see cref="StringWriter"/>) and asserts on the
/// transcript a user would see. Expectations below are the OBSERVED output of the
/// current implementation, not invented text.
/// </summary>
public class ReplOutputTests
{
    // -----------------------------------------------------------------
    // injection seam
    // -----------------------------------------------------------------

    [Fact]
    public void Constructors_KeepTheDefaultInteractiveConsolePath()
    {
        // Parameterless construction still selects the real-console editing path...
        Assert.True(new LineEditor().IsInteractive);
        // ...while injecting a reader switches to ReadLine, which is what makes a
        // redirected/headless session possible.
        Assert.False(new LineEditor(new StringReader(""), new StringWriter()).IsInteractive);
        // The parameterless session constructor still works for the real CLI.
        Assert.NotNull(new ReplSession());
    }

    // -----------------------------------------------------------------
    // help / discoverability
    // -----------------------------------------------------------------

    [Fact]
    public async Task Help_Overview_ListsNonEmptyCategories()
    {
        string t = await ReplHarness.RunAsync("help\nexit\n");

        Assert.StartsWith("» help", t);
        Assert.Contains("LovelaceSharp — help", t);
        Assert.Contains("Categories:", t);

        // The category block is derived from the live builtin registry: every line looks
        // like "  Symbolics        36 function(s)" and there must be several of them.
        var categories = ReplHarness.Lines(t)
            .Where(l => l.StartsWith("  ", StringComparison.Ordinal) && l.EndsWith(" function(s)", StringComparison.Ordinal))
            .ToArray();
        Assert.True(categories.Length >= 5,
            $"expected at least 5 categories, found {categories.Length}:{Environment.NewLine}{t}");
        Assert.Contains(categories, c => c.TrimStart().StartsWith("Symbolics", StringComparison.Ordinal));
        Assert.Contains(categories, c => c.TrimStart().StartsWith("Calculus", StringComparison.Ordinal));

        Assert.Contains("help <category>       list a category's functions", t);
        Assert.Contains("funcs [category]      categorized function listing", t);
    }

    [Fact]
    public async Task Help_Solve_ShowsSignatureAndSummary()
    {
        string t = await ReplHarness.RunAsync("help solve\nexit\n");

        ReplHarness.AssertHasLine(t, "solve(f, x [, domain])");
        Assert.Contains("Solves an equation for x.", t);
        Assert.Contains("Examples:", t);
        Assert.Contains("Returns: Vector | Text", t);
        Assert.Contains("See also: solve_full, solve_system, linsolve", t);
        Assert.Contains("Plugin: Lovelace.Symbolics", t);
    }

    [Fact]
    public async Task Funcs_Symbolics_ListsTheSymbolicsCategory()
    {
        string t = await ReplHarness.RunAsync("funcs symbolics\nexit\n");

        var lines = ReplHarness.Lines(t);
        Assert.Equal("Symbolics", lines[1]); // lines[0] is the echoed "» funcs symbolics" prompt line
        ReplHarness.AssertHasLine(t, "  factor(f)");
        ReplHarness.AssertHasLine(t, "  simplify(f)");
        // symbol's second parameter is declared optional (MinArity 1) and the signature renderer
        // distinguishes required from optional parameters, so the listing must say so
        ReplHarness.AssertHasLine(t, "  symbol(name [, domain])");
        Assert.True(lines.Count(l => l.StartsWith("  ", StringComparison.Ordinal)) >= 30,
            $"expected a full category listing:{Environment.NewLine}{t}");
    }

    // -----------------------------------------------------------------
    // pretty / precision knobs
    // -----------------------------------------------------------------

    [Fact]
    public async Task SetPrettyUnicode_ChangesGlyphs_AndAsciiRestoresThem()
    {
        string t = await ReplHarness.RunAsync(
            "set pretty unicode\ninf\nsqrt(2)\nset pretty ascii\ninf\nsqrt(2)\nexit\n");

        ReplHarness.AssertHasLine(t, "Pretty output: unicode.");
        ReplHarness.AssertHasLineStartingWith(t, "= ∞ (Symbolic)");
        ReplHarness.AssertHasLine(t, "Pretty output: ascii.");
        ReplHarness.AssertHasLineStartingWith(t, "= inf (Symbolic)");

        int unicodeGlyph = t.IndexOf("= ∞ (Symbolic)", StringComparison.Ordinal);
        int asciiMarker = t.IndexOf("Pretty output: ascii.", StringComparison.Ordinal);
        int asciiGlyph = t.IndexOf("= inf (Symbolic)", StringComparison.Ordinal);
        Assert.True(unicodeGlyph >= 0 && unicodeGlyph < asciiMarker && asciiMarker < asciiGlyph,
            $"unicode glyph must precede the ascii switch which must precede the ascii glyph:{Environment.NewLine}{t}");

        // sqrt(2) is purely numeric, so both modes render the same digits (documented
        // finding: the glyph difference shows up on symbolic values such as inf/∞).
        var sqrtLines = ReplHarness.Lines(t)
            .Where(l => l.StartsWith("= 1.4142135623730950488016887242096980785696718753769", StringComparison.Ordinal))
            .ToArray();
        Assert.Equal(2, sqrtLines.Length);
    }

    [Fact]
    public async Task SetDisplay_TightensRenderedPrecision()
    {
        string t = await ReplHarness.RunAsync("sqrt(2)\nset display 5\nsqrt(2)\nexit\n");

        ReplHarness.AssertHasLine(t, "Display digits set to 5.");

        // Default rendering: the full 100-digit expansion.
        ReplHarness.AssertHasLineStartingWith(t, "= 1.4142135623730950488016887242096980785696718753769");
        // After the knob: exactly 5 fractional digits.
        ReplHarness.AssertHasLineStartingWith(t, "= 1.41421 (Real)");

        int before = t.IndexOf("= 1.4142135623730950488016887242096980785696718753769", StringComparison.Ordinal);
        int knob = t.IndexOf("Display digits set to 5.", StringComparison.Ordinal);
        int after = t.IndexOf("= 1.41421 (Real)", StringComparison.Ordinal);
        Assert.True(before >= 0 && before < knob && knob < after,
            $"the precision knob must take effect between the two evaluations:{Environment.NewLine}{t}");
    }

    // -----------------------------------------------------------------
    // variables / functions
    // -----------------------------------------------------------------

    [Fact]
    public async Task Vars_ListsVariableAfterDefiningVariableAndFunction()
    {
        string t = await ReplHarness.RunAsync("a = 5\nfunc f(x) = x^2\nvars\nf(3)\nexit\n");

        ReplHarness.AssertHasLineStartingWith(t, "= 5 (Natural)");      // a = 5
        ReplHarness.AssertHasLine(t, "  a = 5 (Natural)");              // listed by vars
        ReplHarness.AssertHasLine(t, "  _ = 5 (Natural)");              // last-expression binding
        ReplHarness.AssertHasLineStartingWith(t, "= 9 (Natural)");      // func f(x) = x^2 is callable
    }

    [Fact]
    public async Task MultiLineFunctionBlock_AccumulatesUntilBracesBalance()
    {
        string t = await ReplHarness.RunAsync("func g(x) {\n  x + 1\n}\ng(2)\nexit\n");

        Assert.Contains("…   x + 1", t);        // continuation prompts
        Assert.Contains("… }", t);
        ReplHarness.AssertHasLineStartingWith(t, "= 3 (Natural)");
    }

    [Fact]
    public async Task Print_GoesToTheInjectedWriter()
    {
        string t = await ReplHarness.RunAsync("print(\"hi\")\nexit\n");

        ReplHarness.AssertHasLine(t, "hi");
    }

    // -----------------------------------------------------------------
    // values
    // -----------------------------------------------------------------

    [Fact]
    public async Task Matrix_RendersAsNestedBrackets()
    {
        string t = await ReplHarness.RunAsync("[[1,2],[3,4]]\nexit\n");

        ReplHarness.AssertHasLineStartingWith(t, "= [[1, 2], [3, 4]] (Array)");
        Assert.DoesNotContain("Error", t);
    }

    // -----------------------------------------------------------------
    // errors and loop control
    // -----------------------------------------------------------------

    [Fact]
    public async Task Error_IsRenderedAndTheSessionKeepsAcceptingInput()
    {
        string t = await ReplHarness.RunAsync("diff(x, 3)\nhelp\nexit\n");

        ReplHarness.AssertHasLineStartingWith(t, "Error: Undefined variable 'x'.");
        ReplHarness.AssertHasLine(t, "Bye!");

        int error = t.IndexOf("Error: Undefined variable 'x'.", StringComparison.Ordinal);
        int recovery = t.IndexOf("LovelaceSharp — help", StringComparison.Ordinal);
        Assert.True(error >= 0 && recovery > error,
            $"the session must accept the next command after an error:{Environment.NewLine}{t}");
    }

    [Fact]
    public async Task SyntaxError_PrintsCaretUnderTheErrorPosition()
    {
        string t = await ReplHarness.RunAsync("[1,2;3,4]\nexit\n");

        ReplHarness.AssertHasLine(t, "[1,2;3,4]");
        Assert.Contains("Error: Expected 'RBracket' but found ';' at position 4.", t);

        string caretLine = ReplHarness.Lines(t).First(l => l.Contains('^'));
        Assert.Equal(4, caretLine.IndexOf('^'));
    }

    [Fact]
    public async Task Exit_TerminatesTheLoopWithoutReadingFurtherInput()
    {
        string t = await ReplHarness.RunAsync("exit\n1+1\nexit\n");

        ReplHarness.AssertHasLine(t, "Bye!");
        Assert.DoesNotContain("= 2 (Natural)", t); // nothing after exit is evaluated
        Assert.Equal(1, ReplHarness.Lines(t).Count(l => l == "» exit"));
    }

    [Fact]
    public async Task Quit_AlsoTerminatesTheLoop()
    {
        string t = await ReplHarness.RunAsync("quit\n1+1\n");

        ReplHarness.AssertHasLine(t, "Bye!");
        Assert.DoesNotContain("= 2 (Natural)", t);
    }

    [Fact]
    public async Task EndOfInput_TerminatesWithBye()
    {
        string t = await ReplHarness.RunAsync("1+1\n");

        ReplHarness.AssertHasLineStartingWith(t, "= 2 (Natural)");
        ReplHarness.AssertHasLine(t, "Bye!");
    }
}
