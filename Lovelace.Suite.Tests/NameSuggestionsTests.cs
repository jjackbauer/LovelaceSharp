namespace Lovelace.Suite.Tests;

/// <summary>
/// The answer the engine hands back when it does not have the name a caller used. The harnessability
/// benchmark measured the cost of withholding it: every repair task that stalled in the observation
/// modes without registry access stalled on exactly this, and the names callers reach for are ordinary
/// English for the operation (<c>determinant</c>, <c>inverse</c>, <c>derive</c>).
/// </summary>
public class NameSuggestionsTests
{
    /// <summary>A registry shaped like the live one for the names these tests exercise.</summary>
    private static readonly string[] Registry =
    {
        "det", "diff", "inv", "mean", "len", "factor", "shape", "trace", "eye", "numel",
        "matmul", "transpose", "sum", "dot", "solve", "solve_full", "solve_system",
    };

    [Theory]
    [InlineData("determinant", "det")]
    [InlineData("inverse", "inv")]
    [InlineData("factorize", "factor")]
    [InlineData("derive", "diff")]
    [InlineData("average", "mean")]
    [InlineData("rows", "shape")]
    [InlineData("numelements", "numel")]
    [InlineData("tranpose", "transpose")]
    public void ARegisteredNameCloseToTheOneThatFailed_IsOfferedBack(string queried, string expected)
    {
        Assert.Equal(new[] { expected }, NameSuggestions.Suggest(Registry, queried));
    }

    /// <summary>
    /// Short names are dense in edit-distance space: <c>fft</c> is two edits from both <c>det</c> and
    /// <c>dot</c>. Offering either would be noise, and the message contract for a name with nothing
    /// close to it must stay exactly what it always was.
    /// </summary>
    [Theory]
    [InlineData("fft")]
    [InlineData("nosuchfn")]
    [InlineData("push")]
    [InlineData("zip")]
    public void ANameNothingIsCloseTo_IsNotGuessedAt(string queried)
    {
        Assert.Empty(NameSuggestions.Suggest(Registry, queried));
    }

    [Fact]
    public void ASynonymWhoseTargetIsNotRegistered_IsDropped()
    {
        Assert.Empty(NameSuggestions.Suggest(new[] { "det", "inv" }, "derive"));
    }

    /// <summary>The offer is capped, and it does not depend on the order the registry is walked in.</summary>
    [Fact]
    public void TheOfferedList_IsCappedAndIndependentOfRegistryOrder()
    {
        var declared = new[] { "solve_system_full", "solve", "solve_full", "solve_system", "squeeze" };
        var shuffled = new[] { "solve_system", "squeeze", "solve", "solve_system_full", "solve_full" };

        var first = NameSuggestions.Suggest(declared, "sol");
        var second = NameSuggestions.Suggest(shuffled, "sol");

        Assert.Equal(new[] { "solve", "solve_full", "solve_system" }, first);
        Assert.Equal(first, second);
    }

    /// <summary>A name that is language syntax, not a missing builtin, gets the syntax note instead.</summary>
    [Fact]
    public void ASyntaxName_CarriesItsNoteInsteadOfAFunctionSuggestion()
    {
        Assert.Empty(NameSuggestions.Suggest(Registry, "range"));
        Assert.NotEmpty(NameSuggestions.HintsFor("range"));
        Assert.Contains("1..n", NameSuggestions.HintsFor("range")[0]);
        Assert.Empty(NameSuggestions.HintsFor("det"));
    }

    [Fact]
    public void Rank_OrdersTheFourDegreesOfCloseness()
    {
        Assert.Equal(0, NameSuggestions.Rank("det", "DET"));
        Assert.Equal(1, NameSuggestions.Rank("det", "determinant"));
        Assert.Equal(2, NameSuggestions.Rank("transpose", "tranpose"));
        Assert.Equal(3, NameSuggestions.Rank("solve_full", "solve_x"));
        Assert.Equal(-1, NameSuggestions.Rank("det", "fft"));
    }

    [Theory]
    [InlineData("transpose", "tranpose", 1)]      // one deletion
    [InlineData("det", "dot", 1)]                 // one substitution
    [InlineData("transpose", "tranpsose", 2)]     // a transposition is two edits, not one
    [InlineData("solve_full", "solve_x", int.MaxValue)]   // too far apart to be worth measuring
    public void EditDistance_IsTheStructuralMetric(string a, string b, int expected)
    {
        Assert.Equal(expected, NameSuggestions.EditDistance(a, b));
    }
}
