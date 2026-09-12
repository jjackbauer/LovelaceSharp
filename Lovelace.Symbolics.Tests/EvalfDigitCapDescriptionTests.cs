using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Round-20 audit I, I-2, the description half: the <c>evalf</c> summary that <c>help</c> and
/// <c>capabilities()</c> publish claimed the published value "is bounded back to the N places that
/// were asked for" — false for N &gt; 1000, because the builtin's own computation cap stops at
/// 1000. The cap is documented (EVD-276) and stays; the summary has to state it, and it has to say
/// what a caller sees when it bites, so a reader can predict the envelope.
/// </summary>
public class EvalfDigitCapDescriptionTests
{
    private static string HelpText()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new SymbolicsPlugin());
        string? help = engine.Help.Function("evalf");
        Assert.True(help is not null, "evalf has no published help entry");
        return help!;
    }

    [Fact]
    public void TheSummary_NamesThePlaceCapAndTheMarkerThatReportsIt()
    {
        string help = HelpText();

        Assert.Contains("1000", help);
        Assert.Contains("digit-cap", help);
        Assert.Contains("truncated", help);
    }

    /// <summary>The sentence that denied the clamp is gone: the summary may not say the value is
    /// always bounded to the count that was asked for.</summary>
    [Fact]
    public void TheSummary_NoLongerDeniesTheClamp()
    {
        Assert.DoesNotContain("bounded back to the N places that were asked for", HelpText());
    }
}
