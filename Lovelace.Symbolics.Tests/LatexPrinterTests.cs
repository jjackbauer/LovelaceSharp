using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;
using Rat = global::Lovelace.Rational.Rational;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Round 28 (Cycle 3, item 13): <see cref="Printing.PrintMode.Latex"/>.
///
/// LaTeX cannot be re-parsed into the kernel, so "the rendering round-trips" is not available as
/// the acceptance property. The two properties that ARE available, and that this file asserts,
/// are:
///
/// 1. GROUPING PARITY. The LaTeX rendering and the Pretty rendering of the same
///    <see cref="Expr"/> are both denoised into a structural tree by the two parsers below (the
///    LaTeX denoiser resolves LaTeX's OWN grouping constructs — <c>{...}</c> groups,
///    <c>\frac{A}{B}</c>, <c>\sqrt{A}</c>, <c>\left(...\right)</c>, <c>{-}</c> as a superscript
///    group, <c>\left|A\right|</c> — into the grouping they denote, and the infix parser resolves
///    parentheses; tokens such as <c>*</c> vs <c>\cdot</c> are normalised away). A dropped
///    delimiter in either rendering changes its tree, so tree equality is exactly the claim "the
///    structural nesting is the same".
///
/// 2. DELIMITERS ARE LOAD-BEARING. Every explicit <c>\left(...\right)</c> / <c>\left|...\right|</c>
///    pair in the LaTeX rendering is removed and the text re-parsed: if the tree is unchanged the
///    pair was decoration and the test fails. This is the "exactly where the Pretty rendering
///    needs parentheses" half, without an oracle: it is measured on the LaTeX text itself.
///
/// Both checks run over <see cref="Corpus"/>, which is precedence-sensitive by construction.
/// </summary>
public class LatexPrinterTests
{
    private const string Prelude =
        "x = symbol(\"x\"); y = symbol(\"y\"); a = symbol(\"a\"); b = symbol(\"b\"); " +
        "z = symbol(\"z\"); w = symbol(\"w\");";

    /// <summary>Precedence-sensitive corpus: nested subtraction, powers of sums, powers of
    /// negatives, products of sums, nested fractions, negative exponents, unary minus, function
    /// application, abs and radicals.</summary>
    public static readonly string[] Corpus =
    {
        // nested subtraction and unary minus
        "x - (y - a)",
        "x - y - a",
        "a - (x + y)",
        "-x^2",
        "(-x)^2",
        "(-x)^3",
        "-(x + y)",
        "(-x - 1)^2",
        // negative numeric bases: the sign belongs to the numeral, so the base text begins with
        // a unary minus and has to be delimited by the same one rule Pretty uses
        "(-1)^x",
        "(-2)^x",
        "(-1/2)^x",
        "(-1.5)^x",
        // a fraction as a power base, and flat denominators: the same one rule in the notation
        // where \frac{1}{2} is already an atom and every division is its own \frac
        "(1/2)^x",
        "(2/3)^x",
        "(-3/2)^x",
        "(x/y)/z",
        "x/y/z",
        "x/y/z/w",
        "(x/y)/(z/w)",
        "x/(y*z)",
        "(x*y)/(z*w)",
        "(x/y)*z",
        "apart(1/(x^2 - 1), x)",
        // powers of sums, nested powers
        "(x + 1)^12",
        "(x + y)^2",
        "(x^y)^2",
        "x^(y^2)",
        // products of sums
        "(x + y)*(x - y)",
        "x*(y + a)",
        "-(x + y)*(x - y)",
        // fractions and negative exponents
        "x/(y + 1)",
        "(x + 1)/y",
        "x^(-2)",
        "(x + 1)^(-1)",
        "1/(2*(x + 1))",
        "-1/(2*(x + 1))",
        "(1/2)*x + 1/3",
        "x/2",
        "x/(2*y)",
        // function application, radicals, abs
        "exp(-x^2)",
        "sin(x + y) + cos(x^2)",
        "sqrt(x + y)",
        "(-x)^(1/2)",
        "x^(-1/2)",
        "log(x + 1)/(x - 1)",
        "sin((x + y)/(x - y))",
        "abs(x)",
        "abs(x + y)*(x - 1)",
        "abs(x - y)^2",
    };

    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new SymbolicsPlugin());
        return engine;
    }

    /// <summary>The expression the corpus row denotes, built through the language (so the corpus
    /// is exactly what a user can write), never through the kernel API.</summary>
    private static async Task<Expr> BuildAsync(string source)
    {
        var engine = NewEngine();
        await engine.EvaluateAsync(Prelude);
        return (await engine.EvaluateAsync(source)).AsSymbolic();
    }

    private static string Latex(Expr e) =>
        Printing.PrettyPrint(e, new Printing.PrintOptions(Printing.PrintMode.Latex));

    private static string Pretty(Expr e) => Printing.PrettyPrint(e);

    // ------------------------------------------------------------------
    // 1. Grouping parity
    // ------------------------------------------------------------------

    [Theory]
    [MemberData(nameof(CorpusCases))]
    public async Task GroupingParity_LatexAndPrettyHaveTheSameStructuralNesting(string source)
    {
        var expr = await BuildAsync(source);
        string pretty = Pretty(expr);
        string latex = Latex(expr);

        string fromPretty = ShapeTree.ParseInfix(pretty).Shape();
        string fromLatex = ShapeTree.ParseLatex(latex).Shape();

        Assert.True(fromPretty == fromLatex,
            "\"" + source + "\" renders with a different grouping:" + Environment.NewLine +
            "  pretty: " + pretty + Environment.NewLine +
            "          -> " + fromPretty + Environment.NewLine +
            "  latex : " + latex + Environment.NewLine +
            "          -> " + fromLatex);
    }

    // ------------------------------------------------------------------
    // 2. No decoration: every delimiter changes the meaning if removed
    // ------------------------------------------------------------------

    [Theory]
    [MemberData(nameof(CorpusCases))]
    public async Task Delimiters_AreLoadBearing(string source)
    {
        var expr = await BuildAsync(source);
        string latex = Latex(expr);
        string withDelimiters = ShapeTree.ParseLatex(latex).Shape();

        var redundant = new List<string>();
        foreach (var (open, close) in DelimiterPairs(latex))
        {
            string without = latex.Remove(close, "\\right)".Length).Remove(open, "\\left(".Length);
            string shape;
            try
            {
                shape = ShapeTree.ParseLatex(without).Shape();
            }
            catch (Exception)
            {
                continue;   // removing it made the text unparseable: the delimiter carries the parse
            }
            if (shape == withDelimiters) redundant.Add(latex[open..(close + 7)]);
        }

        Assert.True(redundant.Count == 0,
            "\"" + source + "\" -> " + latex + " has delimiters that change nothing: " +
            string.Join(" ", redundant));
    }

    /// <summary>The <c>\left...\right</c> pairs of a LaTeX rendering, innermost-nested, as
    /// (open, close) offsets. Call arguments use plain parentheses and are not pairs here.</summary>
    private static IEnumerable<(int Open, int Close)> DelimiterPairs(string text)
    {
        var stack = new Stack<int>();
        for (int i = 0; i < text.Length; i++)
        {
            if (text[i] != '\\') continue;
            if (text.AsSpan(i).StartsWith("\\left(") || text.AsSpan(i).StartsWith("\\left|"))
            {
                stack.Push(i);
                i += 5;
            }
            else if (text.AsSpan(i).StartsWith("\\right)" ) || text.AsSpan(i).StartsWith("\\right|"))
            {
                if (stack.Count == 0) throw new InvalidOperationException("unbalanced \\right in " + text);
                yield return (stack.Pop(), i);
                i += 6;
            }
        }
        if (stack.Count != 0) throw new InvalidOperationException("unbalanced \\left in " + text);
    }

    // ------------------------------------------------------------------
    // 3. Known renderings, reasoned about by hand
    // ------------------------------------------------------------------

    [Theory]
    [MemberData(nameof(KnownRenderings))]
    public async Task KnownRenderings_AreExact(string source, string expected)
    {
        var expr = await BuildAsync(source);
        Assert.Equal(expected, Latex(expr));
    }

    /// <summary>The required rational constant. At the language level a bare <c>1/2</c> is a
    /// numeric value, not a symbolic expression, so this one is built from the model directly —
    /// it is the printer being pinned, not the evaluator.</summary>
    [Fact]
    public void KnownRendering_RationalConstant_IsAFraction()
    {
        Assert.Equal("\\frac{1}{2}", Latex(Exprs.Rational(1, 2)));
        Assert.Equal("\\frac{3}{4}", Latex(Exprs.Rational(3, 4)));
        Assert.Equal("-\\frac{1}{2}", Latex(Exprs.Rational(-1, 2)));
        Assert.Equal("7", Latex(Exprs.Integer(7)));
    }

    /// <summary>A negative NUMERIC base is not an atom as text: LaTeX spells -1 with a leading
    /// minus (and -3/2 as -\frac{3}{2}), which binds looser than the superscript, so the base has
    /// to carry \left(...\right) — otherwise the rendering denotes -(1^{x}) instead of (-1)^{x}.</summary>
    [Fact]
    public async Task KnownRendering_PowerOfANegativeNumericBase_HasDelimitedBase()
    {
        var expr = await BuildAsync("(-1)^x");
        Assert.Equal("\\left(-1\\right)^{x}", Latex(expr));
    }

    /// <summary>The same one rule in LaTeX notation, where the answer comes out the other way
    /// round: <c>\frac{1}{2}</c> IS an atom, so the superscript applies to the whole fraction and
    /// the base carries no <c>\left...\right</c> — a pair here would be exactly the decoration
    /// <see cref="Delimiters_AreLoadBearing"/> rejects. Pretty's spelling of the same value,
    /// <c>1/2</c>, is a division and does need the pair; only the text can tell the two apart.</summary>
    [Fact]
    public void KnownRendering_PowerOfARationalBase_KeepsTheFractionUnwrapped()
    {
        var expr = Exprs.Power(Exprs.Rational(1, 2), Exprs.Symbol("x"));
        Assert.Equal("\\frac{1}{2}^{x}", Latex(expr));
    }

    /// <summary>A complex constant base is a model-level shape (the language folds complex
    /// constants out of user arithmetic). LaTeX spells it as a parenthesised group, which already
    /// delimits it, so no <c>\left...\right</c> is added; Pretty spells the same value
    /// <c>1 + i</c> and does need the pair.</summary>
    [Fact]
    public void KnownRendering_PowerOfAComplexConstant_IsAlreadyAGroup()
    {
        var one = Rat.From(1, 1);
        var expr = Exprs.Power(Exprs.Complex(one, one), Exprs.Symbol("x"));
        Assert.Equal("(1 + 1 i)^{x}", Latex(expr));
    }

    /// <summary>Flat denominators: ONE <c>\frac</c> per division, so the two values stay
    /// distinct — <c>\frac{x}{y \cdot z}</c> is x/(y*z), while (x/y)/z nests.</summary>
    [Fact]
    public async Task KnownRendering_FlatDenominators_DivideOncePerFactor()
    {
        Assert.Equal("\\frac{\\frac{x}{y}}{z}", Latex(await BuildAsync("(x/y)/z")));
        Assert.Equal("\\frac{x}{y \\cdot z}", Latex(await BuildAsync("x/(y*z)")));
    }

    /// <summary>Guard against a vacuous parity test: if the Latex arm ever fell back to the
    /// Pretty rendering, <see cref="GroupingParity_LatexAndPrettyHaveTheSameStructuralNesting"/>
    /// would pass while testing nothing. The corpus is precedence-sensitive, so the two forms must
    /// differ on the overwhelming majority of it.</summary>
    [Fact]
    public async Task LatexArm_IsNotThePrettyArm()
    {
        var same = new List<string>();
        foreach (var source in Corpus)
        {
            var expr = await BuildAsync(source);
            if (Latex(expr) == Pretty(expr)) same.Add(source);
        }
        Assert.True(same.Count <= 2,
            "the LaTeX rendering is the pretty rendering for " + same.Count + " corpus rows: " +
            string.Join(", ", same));
    }

    public static TheoryData<string, string> KnownRenderings() => new()
    {
        // the four required by the item that are symbolic at the language level
        { "x^2", "x^{2}" },
        { "sqrt(x)", "\\sqrt{x}" },
        { "(x + 1)^2", "\\left(x + 1\\right)^{2}" },
        { "x - (y - a)", "x - \\left(y - a\\right)" },
        // and the neighbouring shapes the same decisions govern
        { "(x + y)*(x - y)", "\\left(x + y\\right) \\cdot \\left(x - y\\right)" },
        { "x^(-2)", "x^{-2}" },
        // a bare 1/(x + 1) is a POWER of a sum, not a product with a denominator, so the
        // power arm owns it: the fraction form needs a multiplication to fold
        { "1/(x + 1)", "\\left(x + 1\\right)^{-1}" },
        { "-x^2", "-x^{2}" },
        { "abs(x)", "\\left|x\\right|" },
        { "x^(y^2)", "x^{y^{2}}" },
        { "sin(x)", "\\sin(x)" },
    };

    // ------------------------------------------------------------------
    // 4. The new arm does not disturb the two pinned modes
    // ------------------------------------------------------------------

    [Theory]
    [MemberData(nameof(CorpusCases))]
    public async Task CanonicalAndPretty_AreUntouchedByTheLatexArm(string source)
    {
        var expr = await BuildAsync(source);
        Assert.Equal(Printing.CanonicalPrint(expr),
            Printing.PrettyPrint(expr, new Printing.PrintOptions(Printing.PrintMode.Canonical)));
        Assert.Equal(Printing.DebugPrint(expr),
            Printing.PrettyPrint(expr, new Printing.PrintOptions(Printing.PrintMode.Debug)));
        // an unbudgeted Print() under Latex is the Latex rendering; Canonical stays Canonical
        Assert.Equal(Latex(expr), Printing.Print(expr,
            new Printing.PrintOptions(Printing.PrintMode.Latex)).Text);
        Assert.StartsWith("(", Printing.Print(expr,
            new Printing.PrintOptions(Printing.PrintMode.Canonical)).Text);
    }

    public static TheoryData<string> CorpusCases()
    {
        var data = new TheoryData<string>();
        foreach (var s in Corpus) data.Add(s);
        return data;
    }

    // ==================================================================
    // The two denoisers: infix (Pretty) and LaTeX, onto ONE structural tree
    // ==================================================================

    /// <summary>A node of the denoised structural tree. Parentheses and braces never become
    /// nodes: they only change the shape, which is the thing being compared.</summary>
    public abstract record ShapeTree
    {
        public sealed record Atom(string Text) : ShapeTree;
        public sealed record Neg(ShapeTree Operand) : ShapeTree;
        public sealed record Bin(string Op, ShapeTree Left, ShapeTree Right) : ShapeTree;
        public sealed record Call(string Name, IReadOnlyList<ShapeTree> Args) : ShapeTree;

        public string Shape() => this switch
        {
            Atom a => "A:" + a.Text,
            Neg n => "neg(" + n.Operand.Shape() + ")",
            Bin b => b.Op + "(" + b.Left.Shape() + "," + b.Right.Shape() + ")",
            Call c => "call:" + c.Name + "(" + string.Join(",", c.Args.Select(x => x.Shape())) + ")",
            _ => throw new InvalidOperationException(),
        };

        // ---- tokenizer -------------------------------------------------

        private enum Kind { Num, Ident, Op, Frac, Sqrt }

        private readonly record struct Tok(Kind Kind, string Text);

        private sealed class Parser
        {
            private readonly List<Tok> _t;
            private readonly bool _latex;
            private int _p;

            public Parser(List<Tok> t, bool latex)
            {
                _t = t;
                _latex = latex;
            }

            private bool Is(string op) => _p < _t.Count && _t[_p].Kind == Kind.Op && _t[_p].Text == op;
            private void Take() => _p++;
            private void Expect(string op)
            {
                if (!Is(op)) throw new FormatException("expected '" + op + "' at token " + _p + " of " + Describe());
                _p++;
            }
            private string Describe() => string.Join(" ", _t.Select(x => x.Text));

            public ShapeTree ParseAll()
            {
                var tree = ParseRelation();
                if (_p != _t.Count) throw new FormatException("trailing tokens in " + Describe());
                return tree;
            }

            // relations are loosest; = != < <= > >=
            private ShapeTree ParseRelation()
            {
                var left = ParseAdditive();
                if (_p < _t.Count && _t[_p].Kind == Kind.Op &&
                    _t[_p].Text is "=" or "!=" or "<" or "<=" or ">" or ">=")
                {
                    var op = _t[_p].Text;
                    Take();
                    return new Bin("rel:" + op, left, ParseAdditive());
                }
                return left;
            }

            // + - are left-associative and looser than unary minus: -x*y is -(x*y)
            private ShapeTree ParseAdditive()
            {
                var left = ParseUnary();
                while (Is("+") || Is("-"))
                {
                    var op = _t[_p].Text;
                    Take();
                    left = new Bin(op == "+" ? "add" : "sub", left, ParseUnary());
                }
                return left;
            }

            private ShapeTree ParseUnary()
            {
                if (Is("-")) { Take(); return new Neg(ParseUnary()); }
                if (Is("+")) { Take(); return ParseUnary(); }
                return ParseMultiplicative();
            }

            private ShapeTree ParseMultiplicative()
            {
                var left = ParsePower();
                while (Is("*") || Is("/"))
                {
                    var op = _t[_p].Text;
                    Take();
                    left = new Bin(op == "*" ? "mul" : "div", left, ParsePower());
                }
                return left;
            }

            // ^ is right-associative; its operand is a unary expression (x^-2, x^(y^2), x^{y^2})
            private ShapeTree ParsePower()
            {
                var b = ParseAtom();
                if (!Is("^")) return b;
                Take();
                return new Bin("pow", b, ParseUnary());
            }

            private ShapeTree ParseAtom()
            {
                if (_p >= _t.Count) throw new FormatException("unexpected end of " + Describe());
                var tok = _t[_p];
                if (tok.Kind == Kind.Frac)
                {
                    Take();
                    // \frac{A}{B} is exactly the quotient of its two groups
                    return new Bin("div", ParseGroup(), ParseGroup());
                }
                if (tok.Kind == Kind.Sqrt)
                {
                    Take();
                    return new Call("sqrt", new[] { ParseGroup() });
                }
                if (tok.Kind == Kind.Op)
                {
                    switch (tok.Text)
                    {
                        case "(": Take(); { var inner = ParseRelation(); Expect(")"); return inner; }
                        case "{": Take(); { var inner = ParseRelation(); Expect("}"); return inner; }
                        case "|": Take(); { var inner = ParseRelation(); Expect("|"); return new Call("abs", new[] { inner }); }
                    }
                }
                if (tok.Kind == Kind.Num) { Take(); return new Atom(tok.Text); }
                if (tok.Kind == Kind.Ident)
                {
                    Take();
                    if (Is("("))
                    {
                        Take();
                        var args = new List<ShapeTree>();
                        if (!Is(")"))
                        {
                            args.Add(ParseRelation());
                            while (Is(",")) { Take(); args.Add(ParseRelation()); }
                        }
                        Expect(")");
                        return new Call(tok.Text, args);
                    }
                    return new Atom(tok.Text);
                }
                throw new FormatException("unexpected token '" + tok.Text + "' in " + Describe());
            }

            private ShapeTree ParseGroup()
            {
                if (_latex)
                {
                    Expect("{");
                    var inner = ParseRelation();
                    Expect("}");
                    return inner;
                }
                Expect("(");
                var g = ParseRelation();
                Expect(")");
                return g;
            }
        }

        private static List<Tok> TokenizeInfix(string s)
        {
            var toks = new List<Tok>();
            int i = 0;
            while (i < s.Length)
            {
                char c = s[i];
                if (char.IsWhiteSpace(c)) { i++; continue; }
                if (char.IsDigit(c))
                {
                    int j = i;
                    while (j < s.Length && char.IsDigit(s[j])) j++;
                    toks.Add(new Tok(Kind.Num, s[i..j]));
                    i = j;
                    continue;
                }
                if (char.IsLetter(c) || c == '_')
                {
                    int j = i;
                    while (j < s.Length && (char.IsLetterOrDigit(s[j]) || s[j] == '_')) j++;
                    toks.Add(new Tok(Kind.Ident, s[i..j]));
                    i = j;
                    continue;
                }
                if (c == '!' && i + 1 < s.Length && s[i + 1] == '=') { toks.Add(new Tok(Kind.Op, "!=")); i += 2; continue; }
                if ((c == '<' || c == '>') && i + 1 < s.Length && s[i + 1] == '=') { toks.Add(new Tok(Kind.Op, c + "=")); i += 2; continue; }
                toks.Add(new Tok(Kind.Op, c.ToString()));
                i++;
            }
            return toks;
        }

        private static List<Tok> TokenizeLatex(string s)
        {
            var toks = new List<Tok>();
            int i = 0;
            while (i < s.Length)
            {
                char c = s[i];
                if (char.IsWhiteSpace(c)) { i++; continue; }
                if (char.IsDigit(c))
                {
                    int j = i;
                    while (j < s.Length && char.IsDigit(s[j])) j++;
                    toks.Add(new Tok(Kind.Num, s[i..j]));
                    i = j;
                    continue;
                }
                if (c == '\\')
                {
                    int j = i + 1;
                    while (j < s.Length && char.IsLetter(s[j])) j++;
                    string word = s[(i + 1)..j];
                    switch (word)
                    {
                        case "left":
                            i = j + 1;
                            toks.Add(new Tok(Kind.Op, s[j] == '|' ? "|" : "("));
                            continue;
                        case "right":
                            i = j + 1;
                            toks.Add(new Tok(Kind.Op, s[j] == '|' ? "|" : ")"));
                            continue;
                        case "frac":
                            i = j;
                            toks.Add(new Tok(Kind.Frac, "\\frac"));
                            continue;
                        case "sqrt":
                            i = j;
                            toks.Add(new Tok(Kind.Sqrt, "\\sqrt"));
                            continue;
                        case "cdot":
                            i = j;
                            toks.Add(new Tok(Kind.Op, "*"));
                            continue;
                        case "operatorname":
                        {
                            int k = j + 1;   // skip '{'
                            int m = k;
                            while (m < s.Length && s[m] != '}') m++;
                            toks.Add(new Tok(Kind.Ident, s[k..m]));
                            i = m + 1;
                            continue;
                        }
                        case "pi":
                            i = j;
                            toks.Add(new Tok(Kind.Ident, "pi"));
                            continue;
                        case "infty":
                            i = j;
                            toks.Add(new Tok(Kind.Ident, "inf"));
                            continue;
                        case "le": i = j; toks.Add(new Tok(Kind.Op, "<=")); continue;
                        case "ge": i = j; toks.Add(new Tok(Kind.Op, ">=")); continue;
                        case "ne": i = j; toks.Add(new Tok(Kind.Op, "!=")); continue;
                        default:
                            // a control word that names a function: \sin, \exp, \tanh, ...
                            i = j;
                            toks.Add(new Tok(Kind.Ident, word));
                            continue;
                    }
                }
                if (char.IsLetter(c) || c == '_')
                {
                    int j = i;
                    while (j < s.Length && (char.IsLetterOrDigit(s[j]) || s[j] == '_')) j++;
                    toks.Add(new Tok(Kind.Ident, s[i..j]));
                    i = j;
                    continue;
                }
                toks.Add(new Tok(Kind.Op, c.ToString()));
                i++;
            }
            return toks;
        }

        public static ShapeTree ParseInfix(string text) => new Parser(TokenizeInfix(text), false).ParseAll();

        public static ShapeTree ParseLatex(string text) => new Parser(TokenizeLatex(text), true).ParseAll();
    }
}
