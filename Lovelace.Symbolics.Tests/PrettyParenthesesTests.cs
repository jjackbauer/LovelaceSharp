using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Round 24 (Cycle 3): the pretty printer must emit the MINIMAL parentheses.
///
/// Acceptance is not "the known strings look nicer" but a round-trip property over a corpus:
/// <c>parse(pretty(e))</c> must canonicalise to the SAME canonical form as <c>e</c>. The
/// "parse" here is the real language parser (Lovelace.Suite), the same one that produced the
/// expressions in the first place, and the comparison is on canonical S-expression text, never
/// on printed infix text.
///
/// Minimality is the second, independent property: no rendering may wrap a subexpression in a
/// pair of parentheses that is itself immediately wrapped by another pair — <c>((E))</c>.
/// </summary>
public class PrettyParenthesesTests
{
    private const string Prelude =
        "x = symbol(\"x\"); y = symbol(\"y\"); a = symbol(\"a\"); b = symbol(\"b\");";

    /// <summary>The corpus. Every entry is infix source; the expression is obtained by evaluating
    /// it, the rendering is obtained from the pretty printer, and the reparse evaluates that
    /// rendering with the same bindings. Precedence-sensitive shapes: subtraction, unary minus,
    /// division, nested powers, powers of negatives, products of sums, function application,
    /// negative exponents, rational coefficients.</summary>
    public static readonly string[] Corpus =
    {
        // the three reported reproductions
        "(x + 1)^12",
        "exp(-x^2)",
        "integrate_full(exp(-x^2), x)",
        "factor(-x^2 - 2*x - 1)",
        // subtraction
        "x - (y - a)",
        "x - y - a",
        "a - (x + y)",
        // unary minus and powers of negatives
        "-x^2",
        "(-x)^2",
        "(-x)^3",
        "-(x + y)",
        "(-x - 1)^2",
        "(-x)^(1/2)",
        // nested powers
        "(x^y)^2",
        "x^(y^2)",
        // products of sums
        "(x + y)*(x - y)",
        "x*(y + a)",
        "-(x + y)*(x - y)",
        // division and negative exponents
        "x/(y + 1)",
        "(x + 1)/y",
        "x^(-2)",
        "(x + 1)^(-1)",
        "1/(2*(x + 1))",
        "-1/(2*(x + 1))",
        // rational coefficients
        "(1/2)*x + 1/3",
        "x/2",
        // function application
        "exp(-(x + 1)^2)",
        "sin(x + y) + cos(x^2)",
        "sqrt(x + y)",
        "log(x + 1)/(x - 1)",
        "diff(exp(x)*x^2, x)",
        "sin((x + y)/(x - y))",
    };

    private static async Task<(Expr Expr, string Pretty, string Canonical)> BuildAsync(string source)
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new SymbolicsPlugin());
        await engine.EvaluateAsync(Prelude);
        var value = await engine.EvaluateAsync(source);
        // integrate_full returns a structured IntegrationResult record; the expression it carries
        // is the thing the printer renders (the reproduction in the defect report).
        if (value.Kind == ValueKind.Record)
        {
            var record = value.AsRecord();
            value = (Value)record.Fields.First(f => f.Name == "expression").Value!;
        }
        var expr = value.AsSymbolic();
        return (expr, Printing.PrettyPrint(expr), Printing.CanonicalPrint(expr));
    }

    private static async Task<(string Canonical, string Pretty)> ReparseAsync(string pretty)
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new SymbolicsPlugin());
        await engine.EvaluateAsync(Prelude);
        var value = await engine.EvaluateAsync(pretty);
        var expr = value.AsSymbolic();
        return (Printing.CanonicalPrint(expr), Printing.PrettyPrint(expr));
    }

    // ------------------------------------------------------------------
    // 1. Round trip: parse(pretty(e)) canonicalises to the same form as e
    // ------------------------------------------------------------------

    [Theory]
    [MemberData(nameof(CorpusCases))]
    public async Task RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm(string source)
    {
        var (_, pretty, canonical) = await BuildAsync(source);
        var (reparsedCanonical, _) = await ReparseAsync(pretty);

        Assert.Equal(canonical, reparsedCanonical);
    }

    // ------------------------------------------------------------------
    // 2. Minimality: no redundant pair around the same subexpression
    // ------------------------------------------------------------------

    [Theory]
    [MemberData(nameof(CorpusCases))]
    public async Task Minimality_NoDoubledParenthesisPair(string source)
    {
        var (_, pretty, _) = await BuildAsync(source);
        var doubled = DoubledPairs(pretty).ToArray();

        Assert.True(doubled.Length == 0,
            "redundant parentheses in \"" + source + "\" -> " + pretty +
            " : " + string.Join(" ", doubled));
    }

    /// <summary>Every <c>((E))</c> in <paramref name="text"/>: an open paren whose immediate
    /// successor is an open paren and whose match closes exactly one character after the inner
    /// match. Legitimate nesting such as <c>((x + 1)*(x - 1))^2</c> is not reported because the
    /// inner pair closes before the outer one does.</summary>
    private static IEnumerable<string> DoubledPairs(string text)
    {
        var match = new int[text.Length];
        var stack = new Stack<int>();
        for (int i = 0; i < text.Length; i++)
        {
            if (text[i] == '(') stack.Push(i);
            else if (text[i] == ')')
            {
                var open = stack.Pop();
                match[open] = i;
                match[i] = open;
            }
        }

        for (int i = 0; i + 1 < text.Length; i++)
        {
            if (text[i] != '(' || text[i + 1] != '(') continue;
            if (match[i] == match[i + 1] + 1)
                yield return text[i..(match[i] + 1)];
        }
    }

    // ------------------------------------------------------------------
    // 3. The three reported renderings, exactly
    // ------------------------------------------------------------------

    [Fact]
    public async Task ReportedCase_PowerOfASum_HasOnePair()
    {
        var (_, pretty, _) = await BuildAsync("(x + 1)^12");
        Assert.Equal("(x + 1)^12", pretty);
    }

    [Fact]
    public async Task ReportedCase_ExpOfNegativeSquare_HasNoArgumentParens()
    {
        var (_, pretty, _) = await BuildAsync("exp(-x^2)");
        Assert.Equal("exp(-x^2)", pretty);
    }

    [Fact]
    public async Task ReportedCase_FactoredSquare_HasOnePair()
    {
        var (_, pretty, _) = await BuildAsync("factor(-x^2 - 2*x - 1)");
        Assert.Equal("-(x + 1)^2", pretty);
    }

    // ------------------------------------------------------------------
    // 4. Mode coherence: Canonical and Debug are unaffected by the pretty fix
    // ------------------------------------------------------------------

    [Theory]
    [MemberData(nameof(CorpusCases))]
    public async Task CanonicalMode_StillRoundTripsThroughCanonicalParse(string source)
    {
        var (expr, _, canonical) = await BuildAsync(source);
        var reparsed = Printing.CanonicalParse(canonical, Exprs.Current);
        Assert.Equal(canonical, Printing.CanonicalPrint(reparsed));
        // and the canonical form is what Canonical mode prints
        Assert.Equal(canonical, Printing.PrettyPrint(expr, new Printing.PrintOptions(Printing.PrintMode.Canonical)));
    }

    [Theory]
    [MemberData(nameof(CorpusCases))]
    public async Task DebugMode_IsStructuralAndContainsNoPrecedenceParentheses(string source)
    {
        var (expr, _, _) = await BuildAsync(source);
        var debug = Printing.PrettyPrint(expr, new Printing.PrintOptions(Printing.PrintMode.Debug));
        // structural form: kinds as prefixes, only call arguments are parenthesised
        Assert.Equal(Printing.DebugPrint(expr), debug);
        Assert.DoesNotContain("((", debug);
    }

    public static TheoryData<string> CorpusCases()
    {
        var data = new TheoryData<string>();
        foreach (var s in Corpus) data.Add(s);
        return data;
    }
}
