using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;
using Rat = global::Lovelace.Rational.Rational;

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
        "x = symbol(\"x\"); y = symbol(\"y\"); a = symbol(\"a\"); b = symbol(\"b\"); " +
        "z = symbol(\"z\"); w = symbol(\"w\");";

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
        // negative NUMERIC bases: the value is an atom, but its RENDERING is not — "-1" is a
        // unary minus applied to the constant 1, and a unary minus binds looser than ^, so the
        // printed base must be delimited or the text denotes -(1^x) instead of (-1)^x.
        "(-1)^x",
        "(-2)^x",
        "(-1/2)^x",
        "(-1.5)^x",
        "(-3)^(-x)",
        // a fraction as a power base: the VALUE is one atom in the precedence table, but "1/2" is
        // a DIVISION as text and / binds looser than ^, so the base must be delimited —
        // 1/2^x denotes (2^x)^-1.
        "(1/2)^x",
        "(2/3)^x",
        "(-3/2)^x",
        // flat denominators: x/(y*z) is ONE divisor, the product y*z, while x/y/z divides twice
        // and is a different value. The printer spelled both as x/(y*z), so one of the two values
        // re-parsed as the other; the mirror row x/(y*z) must keep its single divisor.
        "(x/y)/z",
        "x/y/z",
        "x/y/z/w",
        "(x/y)/(z/w)",
        "x/(y*z)",
        "(x*y)/(z*w)",
        "(x/y)*z",
        // the shape the usage guide documents: each term is (-1/2)*(x+1)^-1, a rational
        // coefficient with a denominator of its own, so the term divides twice — the guide's
        // "-1/(2*(x + 1)) + 1/(2*(x - 1))" re-parses to a different canonical form
        "apart(1/(x^2 - 1), x)",
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

    /// <summary>(B1) The same defect class as the negative base, one shape further: the base is
    /// the rational constant 1/2, an ATOM in the precedence table, but its text is a division.
    /// 1/2^x re-parses as (2^x)^-1 — a different value at every x — so the base carries the pair,
    /// and the text the power raises is one delimited group and nothing else.</summary>
    [Fact]
    public async Task ReportedCase_PowerOfARationalConstant_HasADelimitedBase()
    {
        var (_, pretty, canonical) = await BuildAsync("(1/2)^x");
        Assert.Equal("(1/2)^x", pretty);
        Assert.Equal("(pow (rat 1 2) (sym x))", canonical);
        AssertDelimitedBase(pretty, "(1/2)");

        var (_, third, _) = await BuildAsync("(2/3)^x");
        Assert.Equal("(2/3)^x", third);
    }

    /// <summary>(B3) The mirror of the flat-denominator fix, pinned as text: the number of
    /// divisions in the rendering equals the number of denominator factors in the value, so a
    /// product that is itself ONE divisor keeps its single "/" while two separate denominator
    /// factors divide twice.</summary>
    [Fact]
    public async Task ReportedCase_FlatDenominators_DivideOncePerFactor()
    {
        var (_, flat, _) = await BuildAsync("(x/y)/z");
        Assert.Equal("x/y/z", flat);
        var (_, flat3, _) = await BuildAsync("x/y/z/w");
        Assert.Equal("x/w/y/z", flat3);
        var (_, mirror, _) = await BuildAsync("x/(y*z)");
        Assert.Equal("x/(y*z)", mirror);
    }

    // ------------------------------------------------------------------
    // 3b. A shape the language cannot spell: a complex constant base
    // ------------------------------------------------------------------

    /// <summary>(B2) A complex constant base is a model-level shape — the language folds complex
    /// constants out of user arithmetic — so the corpus cannot write it and the re-parse has no
    /// complex constant to rebuild from the text; the property asserted is therefore that the
    /// text the power raises is ONE delimited group. The value is an atom in the precedence table
    /// while its text is a SUM, and + binds looser than ^, so 1 + i^x denotes 1 + (i^x): a
    /// different value. The leading-minus rule already covered the negative-real-part twin; the
    /// total rule covers both, in both notations, without naming the kind.</summary>
    [Fact]
    public void ModelLevel_PowerOfAComplexConstant_HasADelimitedBase()
    {
        var one = Rat.From(1, 1);
        var positive = Exprs.Power(Exprs.Complex(one, one), Exprs.Symbol("x"));       // 1 + i
        var negative = Exprs.Power(Exprs.Complex(Rat.From(-1, 1), one), Exprs.Symbol("x")); // -1 + i

        AssertDelimitedBase(Printing.PrettyPrint(positive), "(1 + i)");
        AssertDelimitedBase(Printing.PrettyPrint(negative), "(-1 + i)");
    }

    /// <summary>Asserts that <paramref name="rendering"/> raises a power whose base text is
    /// exactly the one delimited group <paramref name="expectedBase"/>: the base cannot leak an
    /// operator of its own to the power's level, which is the whole property when the language
    /// cannot spell the shape back.</summary>
    private static void AssertDelimitedBase(string rendering, string expectedBase)
    {
        int caret = rendering.IndexOf('^', StringComparison.Ordinal);
        Assert.True(caret > 0, "no power in \"" + rendering + "\"");
        var basis = rendering[..caret];
        Assert.Equal(expectedBase, basis);
        Assert.StartsWith("(", basis, StringComparison.Ordinal);
        Assert.Equal(basis.Length - 1, MatchingClose(basis, 0));   // the group IS the whole base
    }

    /// <summary>The offset of the parenthesis that closes the one at <paramref name="open"/>,
    /// or -1 when the text has no such pair.</summary>
    private static int MatchingClose(string text, int open)
    {
        int depth = 0;
        for (int i = open; i < text.Length; i++)
        {
            if (text[i] == '(') depth++;
            else if (text[i] == ')' && --depth == 0) return i;
        }
        return -1;
    }

    public static TheoryData<string> CorpusCases()
    {
        var data = new TheoryData<string>();
        foreach (var s in Corpus) data.Add(s);
        return data;
    }
}
