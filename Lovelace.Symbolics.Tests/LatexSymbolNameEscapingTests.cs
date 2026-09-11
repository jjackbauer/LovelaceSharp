using System.Text;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Round 37 (Cycle 3, item B): a symbol NAME is opaque to the kernel (identity is the name
/// string, ordinal), but LaTeX gives ten characters a meaning of their own. The LaTeX arm must
/// therefore render a name as literal characters, escaped, so the rendering can neither change
/// what the name says nor corrupt the surrounding document.
///
/// DECISION (underscore): <c>x_1</c> renders as <c>x\_1</c> — an escaped literal, NOT a
/// subscript. A name is an atom; splitting it into a base and a subscript would invent structure
/// the kernel does not have, is undefined for a leading, trailing or doubled underscore
/// (<c>_x</c>, <c>x_</c>, <c>a__b</c>), and cannot be inverted. The subscript form is prettier
/// for the common case and costs exactly those cases; the literal form is never wrong.
///
/// HOW AN ESCAPED OCCURRENCE IS DISTINGUISHED FROM A BARE ONE: the text is tokenised into
/// control sequences before it is searched. A backslash followed by letters is a control WORD
/// (optionally terminated by the empty group <c>{}</c> and/or one space, which TeX swallows); a
/// backslash followed by a non-letter is a control SYMBOL that consumes that one character.
/// Every character a control sequence consumes is escaped; every character left over that is in
/// <see cref="Specials"/> is BARE. Grepping for <c>_</c> would find both <c>x_1</c> (broken) and
/// <c>x\_1</c> (correct); this test cannot.
/// </summary>
public class LatexSymbolNameEscapingTests
{
    /// <summary>The ten characters LaTeX treats as special: # $ % &amp; _ { } ~ ^ \.</summary>
    private const string Specials = "#$%&_{}~^\\";

    /// <summary>Awkward names: one per special character (the underscore one being the common
    /// case), plus a name that is nothing but an escape, a doubled underscore, a lone closing
    /// brace (a name whose bare brace would close the caller's group early) and all ten at once.
    /// </summary>
    public static readonly string[] AwkwardNames =
    {
        "x_1",          // underscore — the common case
        "p%q",          // percent — a bare one comments out the rest of the source line
        "a&b",          // ampersand — a bare one is an alignment tab
        "h#1",          // hash — a bare one is a macro parameter
        "c{d}",         // braces — bare ones open/close a group
        "o}e",          // a lone closing brace
        @"b\s",         // backslash — a bare one starts a control sequence
        "e^f",          // caret — a bare one is a superscript token
        "t~u",          // tilde — a bare one is an unbreakable space
        "m$n",          // dollar — a bare one toggles math mode
        "__",           // underscore only: the name is not "a subscript of nothing"
        "x_1%&#'{}~^\\" // all ten, in one name
    };

    private static string Latex(Expr e) =>
        Printing.PrettyPrint(e, new Printing.PrintOptions(Printing.PrintMode.Latex));

    // ------------------------------------------------------------------
    // 1. No bare special character survives in the rendering of a name
    // ------------------------------------------------------------------

    [Theory]
    [MemberData(nameof(AwkwardNameCases))]
    public void LatexSymbolName_LeavesNoBareSpecialCharacter(string name)
    {
        string latex = Latex(Exprs.Symbol(name));
        var bare = BareSpecials(latex);
        Assert.True(bare.Count == 0,
            "symbol \"" + name + "\" renders as \"" + latex + "\" and leaves " + bare.Count +
            " bare LaTeX special character(s): " +
            string.Join(", ", bare.Select(b => "'" + b.Char + "' at offset " + b.Index)));
    }

    // ------------------------------------------------------------------
    // 2. The escaping is lossless: decoding the escapes gives the name back
    //    (this is what "the rendering still says the same name" means)
    // ------------------------------------------------------------------

    [Theory]
    [MemberData(nameof(AwkwardNameCases))]
    public void LatexSymbolName_DecodesBackToTheNameExactly(string name)
    {
        string latex = Latex(Exprs.Symbol(name));
        Assert.Equal(name, DecodeEscapes(latex));
    }

    // ------------------------------------------------------------------
    // 3. The pin: only the LaTeX arm escapes; the pinned arms keep the name verbatim
    // ------------------------------------------------------------------

    [Theory]
    [MemberData(nameof(AwkwardNameCases))]
    public void OnlyTheLatexArmEscapes_PrettyAndCanonicalKeepTheNameVerbatim(string name)
    {
        var e = Exprs.Symbol(name);
        Assert.Equal(name, Printing.PrettyPrint(e));
        Assert.Equal("(sym " + name + ")", Printing.CanonicalPrint(e));
        Assert.Equal("(sym " + name + ")",
            Printing.PrettyPrint(e, new Printing.PrintOptions(Printing.PrintMode.Canonical)));
    }

    // ------------------------------------------------------------------
    // 4. The decision, spelled out: underscore is a literal, not a subscript
    // ------------------------------------------------------------------

    [Fact]
    public void LatexSymbolName_UnderscoreIsAnEscapedLiteral_NotASubscript()
    {
        Assert.Equal(@"x\_1", Latex(Exprs.Symbol("x_1")));
        // a subscript rendering would produce "x_{1}" here; that is the road not taken, and the
        // two are different strings, so pinning one pins the decision
        Assert.DoesNotContain("_{", Latex(Exprs.Symbol("x_1")));
        // the same name as the base of a power: the name is escaped, the power still groups
        Assert.Equal(@"x\_1^{2}", Latex(Exprs.Power(Exprs.Symbol("x_1"), Exprs.Integer(2))));
    }

    [Theory]
    [MemberData(nameof(DocumentedEscapes))]
    public void LatexSymbolName_UsesTheDocumentedEscape(string name, string expected)
    {
        Assert.Equal(expected, Latex(Exprs.Symbol(name)));
    }

    public static TheoryData<string, string> DocumentedEscapes() => new()
    {
        { "#", @"\#" },
        { "$", @"\$" },
        { "%", @"\%" },
        { "&", @"\&" },
        { "_", @"\_" },
        { "{", @"\{" },
        { "}", @"\}" },
        { "~", @"\textasciitilde{}" },
        { "^", @"\textasciicircum{}" },
        { @"\", @"\textbackslash{}" },
        { "x_1", @"x\_1" },
    };

    // ------------------------------------------------------------------
    // 5. Contexts: the escape is per name, and the surrounding structure is untouched
    // ------------------------------------------------------------------

    [Fact]
    public void LatexSymbolName_InsideLargerExpressions_IsStillEscaped()
    {
        // a product of two awkward names: the separator is the printer's, the names are escaped
        Assert.Equal(@"a\%b \cdot c\&d",
            Latex(Exprs.Multiply(Exprs.Symbol("a%b"), Exprs.Symbol("c&d"))));
        // a derivative's variable list is a list of symbol NAMES too
        Assert.Equal(@"\operatorname{diff}(f, x\_1)",
            Latex(Exprs.Derivative(Exprs.Symbol("f"), Exprs.Current.Symbol("x_1"))));
    }

    // ==================================================================
    // The two helpers the assertions are stated in terms of
    // ==================================================================

    /// <summary>Every LaTeX special character in <paramref name="text"/> that no control
    /// sequence consumes — see the class comment for the tokenisation this rests on.</summary>
    private static List<(int Index, char Char)> BareSpecials(string text)
    {
        var bare = new List<(int, char)>();
        for (int i = 0; i < text.Length; i++)
        {
            char c = text[i];
            if (c == '\\')
            {
                int j = i + 1;
                if (j < text.Length && char.IsLetter(text[j]))
                {
                    while (j < text.Length && char.IsLetter(text[j])) j++;
                    if (j + 1 < text.Length && text[j] == '{' && text[j + 1] == '}')
                        j += 2;                                  // the escape's empty-group terminator
                    if (j < text.Length && text[j] == ' ')
                        j++;                                     // the terminator TeX swallows
                }
                else if (j < text.Length)
                {
                    j++;                                         // the control symbol's character
                }
                i = j - 1;
                continue;
            }
            if (Specials.IndexOf(c) >= 0)
                bare.Add((i, c));
        }
        return bare;
    }

    /// <summary>The escapes this printer emits, decoded back to the characters they stand for.</summary>
    private static readonly Dictionary<string, char> WordEscapes = new()
    {
        ["textasciitilde"] = '~',
        ["textasciicircum"] = '^',
        ["textbackslash"] = '\\',
    };

    /// <summary>Inverse of the escaping on a rendering that consists only of escaped name
    /// characters (a bare symbol): control symbols give their character back, the three control
    /// words in <see cref="WordEscapes"/> give theirs, anything else is not part of a name.</summary>
    private static string DecodeEscapes(string text)
    {
        var sb = new StringBuilder();
        for (int i = 0; i < text.Length; i++)
        {
            char c = text[i];
            if (c != '\\')
            {
                sb.Append(c);
                continue;
            }
            int j = i + 1;
            if (j < text.Length && char.IsLetter(text[j]))
            {
                int start = j;
                while (j < text.Length && char.IsLetter(text[j])) j++;
                string word = text[start..j];
                if (j + 1 < text.Length && text[j] == '{' && text[j + 1] == '}') j += 2;
                if (j < text.Length && text[j] == ' ') j++;
                sb.Append(WordEscapes.TryGetValue(word, out var decoded) ? decoded.ToString() : "\\" + word);
                i = j - 1;
                continue;
            }
            if (j < text.Length)
            {
                sb.Append(text[j]);
                i = j;
            }
        }
        return sb.ToString();
    }

    public static TheoryData<string> AwkwardNameCases()
    {
        var data = new TheoryData<string>();
        foreach (var name in AwkwardNames) data.Add(name);
        return data;
    }
}
