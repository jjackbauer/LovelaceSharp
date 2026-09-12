using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Round-20 audit I, I-3: a budgeted abbreviation could split an identifier mid-token. With a
/// 54-character symbol name and <c>MaxNodes = 1</c> the pretty body was 48 identifier characters
/// followed by " …" — an identifier the value does not contain — while the printer documents
/// "abbreviated (never silently truncated mid-token)" (<c>Printing.cs:340-345</c>) and
/// <c>SafeCut</c> "Cuts at a token boundary so a number or identifier is never split mid-token"
/// (<c>:402-409</c>). The character allowance <c>Math.Max(48, 6 × limit)</c> can fall INSIDE the
/// rendering's FIRST token; <c>SafeCut</c> then walked back to position 0, gave up and returned the
/// raw cut (<c>:408</c>), which is exactly the split it promises never to make.
///
/// <para>The rule pinned here: the retained text always ends at a token BOUNDARY — the first
/// character the cut drops is never a token character — so the abbreviation never ends in the middle
/// of a number or identifier and never names an identifier the value does not contain. When the
/// allowance falls inside the first token, that whole token is kept (a strict prefix whenever
/// anything follows it); when the rendering IS one token, nothing can be retained and the
/// abbreviation is the ellipsis alone, because a prefix that reproduces the whole token would be
/// passed off as an abbreviation that dropped nothing. An UNtruncated rendering is untouched.</para>
/// </summary>
public class PrintBudgetTokenBoundaryTests
{
    /// <summary>The finding's own name: 54 characters, longer than the 48-character floor of the
    /// allowance, so the allowance falls inside the first token.</summary>
    private static readonly string LongName = new string('a', 54);

    private static Expr SumWith(string name)
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return Exprs.Add(Exprs.Symbol(ctx.Symbol(name)), Exprs.One);
    }

    private static Printing.PrintOutcome Abbreviate(Expr e, int maxNodes) =>
        Printing.Print(e, new Printing.PrintOptions(Budget: new Printing.PrintBudget(MaxNodes: maxNodes)));

    private static bool IsTokenChar(char c) => char.IsLetterOrDigit(c) || c == '.';

    /// <summary>The promise, stated as the property it is: the retained body is a strict prefix of
    /// the real rendering whose NEXT character (the first one dropped) is not a token character —
    /// i.e. the cut lands between tokens, never inside one.</summary>
    private static void AssertEndsAtATokenBoundary(string full, Printing.PrintOutcome outcome, string what)
    {
        Assert.True(outcome.Truncated, $"{what}: the rendering was not abbreviated");
        Assert.EndsWith(" …", outcome.Text);
        string body = outcome.Text[..^2];
        Assert.True(body.Length < full.Length,
            $"{what}: '{outcome.Text}' abbreviates nothing ('{full}' is {full.Length} chars)");
        Assert.StartsWith(body, full, StringComparison.Ordinal);
        // A cut is INSIDE a token exactly when BOTH sides of it are token characters: the retained
        // text ends with one AND the dropped text begins with one. A body that ends with a COMPLETE
        // token is not a split just because its last character is a letter or a digit.
        bool retainsTokenChar = body.Length > 0 && IsTokenChar(body[^1]);
        bool dropsTokenChar = IsTokenChar(full[body.Length]);
        Assert.False(retainsTokenChar && dropsTokenChar,
            $"{what}: the cut falls between '{body[^1]}' and '{full[body.Length]}', both token " +
            $"characters, so the retained text ends inside the token " +
            $"…'{full[Math.Max(0, body.Length - 8)..Math.Min(full.Length, body.Length + 8)]}'");
    }

    /// <summary>The finding: a 54-character identifier at MaxNodes 1 came back as 48 of its
    /// characters plus " …". The whole identifier is the boundary the allowance falls short of, and
    /// it is what has to be kept: an abbreviation may return MORE than the allowance by one token,
    /// never less than a token.</summary>
    [Fact]
    public void AnIdentifierLongerThanTheAllowance_IsKeptWhole()
    {
        var e = SumWith(LongName);
        string full = Printing.PrettyPrint(e);
        Assert.Equal(LongName + " + 1", full);

        var outcome = Abbreviate(e, maxNodes: 1);

        string body = outcome.Text.EndsWith(" …") ? outcome.Text[..^2] : outcome.Text;
        Assert.True(body == LongName,
            $"the pretty body is {body.Length} characters of a {LongName.Length}-character identifier: '{body}'");
        Assert.EndsWith(" …", outcome.Text);
        Assert.Equal("node-budget", outcome.Truncation!.Reason);
        Assert.Equal(1, outcome.Truncation.Budget);
        AssertEndsAtATokenBoundary(full, outcome, "the pretty form of a 54-character symbol");
    }

    /// <summary>The same value in the canonical form starts with an open paren, so the cut already
    /// found a boundary before the identifier: the ordinary path is unchanged by the fix above.</summary>
    [Fact]
    public void TheCanonicalFormOfTheSameValue_AlsoEndsAtABoundary()
    {
        var e = SumWith(LongName);
        var canonical = new Printing.PrintOptions(Printing.PrintMode.Canonical,
            Budget: new Printing.PrintBudget(MaxNodes: 1));
        string full = Printing.PrettyPrint(e, new Printing.PrintOptions(Printing.PrintMode.Canonical));

        var outcome = Printing.Print(e, canonical);

        AssertEndsAtATokenBoundary(full, outcome, "the canonical form of a 54-character symbol");
        Assert.StartsWith("(add (rat 1 1) (sym ", outcome.Text, StringComparison.Ordinal);
    }

    /// <summary>A rendering the allowance can hold whole is NOT touched: the promise is about
    /// abbreviations only (a 44-character name + " + 1" is exactly the 48-character allowance).</summary>
    [Fact]
    public void ARenderingWithinTheAllowance_IsUntouched()
    {
        var e = SumWith(new string('a', 44));
        string full = Printing.PrettyPrint(e);
        Assert.Equal(48, full.Length);

        var outcome = Abbreviate(e, maxNodes: 1);

        Assert.False(outcome.Truncated);
        Assert.Null(outcome.Truncation);
        Assert.Equal(full, outcome.Text);
    }

    /// <summary>The degenerate end of the same rule: a rendering that IS one token longer than the
    /// allowance. Keeping the whole token would drop nothing, so the honest abbreviation retains
    /// nothing; what it must never do is hand back the first 48 characters of a 54-character name.
    /// </summary>
    [Fact]
    public void ARenderingThatIsOneLongToken_RetainsNothing()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        Expr e = Exprs.Symbol(ctx.Symbol(LongName));
        string full = Printing.PrettyPrint(e);
        Assert.Equal(LongName, full);

        var outcome = Abbreviate(e, maxNodes: 0);

        Assert.True(outcome.Truncated, "a value the budget stopped must say so");
        Assert.Equal("…", outcome.Text);
        Assert.Equal("node-budget", outcome.Truncation!.Reason);
    }
}
