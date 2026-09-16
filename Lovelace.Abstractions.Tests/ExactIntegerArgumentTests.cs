using Lovelace.Abstractions;
using Int = Lovelace.Integer.Integer;
using Nat = Lovelace.Natural.Natural;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Abstractions.Tests;

/// <summary>
/// ADV-01 — the exact-integer argument contract.
/// <para>
/// These tests are the CONTROL for the whole integer-argument class. They pin the rule the product
/// now enforces (<i>an argument that requires an integer must either represent an exact integer or be
/// refused explicitly; it is never silently rounded or truncated</i>) at the three numeric types and
/// at the contract itself, so the class cannot regress one call site at a time.
/// </para>
/// </summary>
public class ExactIntegerArgumentTests
{
    // ---------------------------------------------------------------------
    // The predicate on each numeric type
    // ---------------------------------------------------------------------

    [Theory]
    [InlineData("0")]
    [InlineData("1")]
    [InlineData("42")]
    [InlineData("9223372036854775807")]          // long.MaxValue
    [InlineData("9223372036854775808")]          // long.MaxValue + 1: refuses, never wraps
    [InlineData("99999999999999999999999999")]
    public void Natural_TryGetExactInt64_AnswersOnlyForValuesTheTypeCanCarry(string text)
    {
        var value = Nat.Parse(text, null);

        bool ok = value.TryGetExactInt64(out long result);

        Assert.Equal(text != "9223372036854775808" && text.Length <= 19, ok);
        if (ok)
            Assert.Equal(text, result.ToString());
    }

    [Theory]
    [InlineData("0")]
    [InlineData("-1")]
    [InlineData("-9223372036854775808")]         // long.MinValue: representable, and the sign is applied after the range check
    [InlineData("9223372036854775807")]
    [InlineData("9223372036854775808")]          // one past the top: refuses
    [InlineData("-9223372036854775809")]         // one past the bottom: refuses
    public void Integer_TryGetExactInt64_HandlesBothEndsOfTheRange(string text)
    {
        var value = Int.Parse(text, null);

        bool ok = value.TryGetExactInt64(out long result);

        Assert.Equal(text != "9223372036854775808" && text != "-9223372036854775809", ok);
        if (ok)
            Assert.Equal(text, result.ToString());
    }

    [Theory]
    [InlineData("2", "2")]
    [InlineData("2.0", "2")]                     // exactly integral: the predicate sees the integer 2
    [InlineData("2.000", "2")]
    [InlineData("-2.0", "-2")]
    [InlineData("+7.000", "7")]
    [InlineData("1000000000000000000.000", "1000000000000000000")]
    [InlineData("2.5", null)]                    // NOT an integer: refused, never floored
    [InlineData("2.9999", null)]
    [InlineData("-2.5", null)]
    [InlineData("0.0000001", null)]
    [InlineData("-0.0", "0")]                    // zero IS the integer 0 at every scale and sign
    public void Real_TryGetExactInt64_PublishesNoFloor(string text, string? expected)
    {
        var value = Rl.Parse(text, null);

        bool ok = value.TryGetExactInt64(out long result);

        Assert.Equal(expected is not null, ok);
        if (expected is not null)
            Assert.Equal(expected, result.ToString());
    }

    /// <summary>
    /// The predicate answers the VALUE's integrality, not the stored shape's. Two cases make the
    /// distinction observable, and they pull in OPPOSITE directions — which is why both are pinned:
    /// <list type="bullet">
    /// <item><c>-0.0</c> and <c>0.00</c> store magnitude 0 at a NEGATIVE exponent, so
    ///   <see cref="Rl.IsInteger"/> (the <c>INumberBase</c> classification, which requires
    ///   <c>Exponent &gt;= 0</c>) answers <c>false</c> while the value IS the integer 0. Delegating to
    ///   it refused a count of 0 the caller had written as <c>0.0</c> — the same false-refusal class
    ///   this predicate exists to prevent. (Found by the ADV-02 plugin worktree against the first
    ///   version of this method; pinned by the DSP suite's
    ///   <c>TheReader_AcceptsEveryPayloadTheCoreCanSend</c> as well.)</item>
    /// <item><c>1.(0)</c> denotes the integer 1 but carries a PERIOD, and a periodic value is not
    ///   read as an integer here — the rule the rest of the tower uses.</item>
    /// </list>
    /// </summary>
    [Fact]
    public void Real_TryGetExactInt64_AnswersTheValuesIntegrality()
    {
        foreach (string zeroForm in new[] { "-0.0", "0.0", "0.00", "+0.000" })
        {
            var zero = Rl.Parse(zeroForm, null);
            Assert.True(zero.TryGetExactInt64(out long value), $"{zeroForm} should read as the integer 0");
            Assert.Equal(0L, value);
        }

        // the classification the predicate deliberately does NOT delegate to
        Assert.False(Rl.IsInteger(Rl.Parse("-0.0", null)));

        var periodicOne = Rl.Parse("1.(0)", null);
        Assert.True(periodicOne.IsPeriodic);
        Assert.False(periodicOne.TryGetExactInt64(out _));
    }

    /// <summary>
    /// The inheritance trap, pinned: <c>Real</c> derives from <c>Integer</c>, so a payload typed as
    /// <c>Integer</c> can be a non-integral <c>Real</c>. If the base rule were not virtual, this call
    /// would answer <c>true</c> and the contract would publish <c>2</c> for <c>2.5</c> — the exact
    /// defect the contract exists to refuse.
    /// </summary>
    [Fact]
    public void Real_ThroughAnIntegerTypedReference_IsStillNotAnInteger()
    {
        Int asInteger = Rl.Parse("2.5", null);

        Assert.False(asInteger.TryGetExactInt64(out long floored));
        Assert.Equal(0L, floored);                // nothing was published, not even a stale zero
    }

    /// <summary>A periodic <c>Real</c> is not an integer, whatever its period is.</summary>
    // ---------------------------------------------------------------------
    // The contract
    // ---------------------------------------------------------------------

    private static bool ReadPayload(object? payload, out long value)
    {
        switch (payload)
        {
            case Rl real:
                return real.TryGetExactInt64(out value);
            case Nat natural:
                return natural.TryGetExactInt64(out value);
            case Int integer:
                return integer.TryGetExactInt64(out value);
            case long l:
                value = l;
                return true;
            case int i:
                value = i;
                return true;
            default:
                value = 0L;
                return false;
        }
    }

    private static ArgumentContractException RefusePayload(object? payload) =>
        payload is Rl
            ? ArgumentContractException.NotAnInteger("probe", "count", Int64Range.NonNegative, "Real", 2)
            : ArgumentContractException.NotAnInteger("probe", "count", Int64Range.NonNegative,
                ArgumentContractException.DescribePayload(payload), 2);

    private static long Require(object? payload, Int64Range bounds) =>
        ExactIntegerArgument.Require(payload, "count", bounds, ReadPayload, RefusePayload, "probe", position: 2);

    [Fact]
    public void Require_ReturnsTheExactValueTheCallerWrote()
    {
        Assert.Equal(7L, Require(new Nat(7UL), Int64Range.NonNegative));
        Assert.Equal(-7L, Require(new Int(-7), Int64Range.Unbounded));
        Assert.Equal(2L, Require(Rl.Parse("2.0", null), Int64Range.NonNegative));
        Assert.Equal(5L, Require(new Int(5), Int64Range.Positive));
    }

    [Fact]
    public void Require_RefusesANonIntegralRealInsteadOfFlooringIt()
    {
        var error = Assert.Throws<ArgumentContractException>(
            () => Require(Rl.Parse("2.5", null), Int64Range.Unbounded));

        Assert.Equal(ArgumentContractFailure.NotAnInteger, error.Failure);
        Assert.Equal("count", error.Parameter);
        Assert.Equal("probe", error.Builtin);
        Assert.Null(error.Received);
    }

    [Fact]
    public void Require_RefusesAnOutOfBoundsIntegerAndNamesTheBoundAndTheValue()
    {
        var error = Assert.Throws<ArgumentContractException>(() => Require(new Int(-1), Int64Range.NonNegative));

        Assert.Equal(ArgumentContractFailure.OutOfBounds, error.Failure);
        Assert.Equal(-1L, error.Received);
        Assert.Contains("-1", error.Message, StringComparison.Ordinal);
        Assert.Contains("non-negative", error.Message, StringComparison.Ordinal);
    }

    /// <summary>A value too wide to be a <see cref="long"/> is refused by NAME — the failure the raw
    /// CLR conversion message used to report without a number in it.</summary>
    [Fact]
    public void Require_RefusesAWiderThanInt64IntegerWithoutARawConversionError()
    {
        var huge = Nat.Parse("99999999999999999999999999", null);

        var error = Assert.Throws<ArgumentContractException>(() => Require(huge, Int64Range.NonNegative));

        Assert.Equal(ArgumentContractFailure.NotAnInteger, error.Failure);
        Assert.NotNull(error.Builtin);
    }

    [Fact]
    public void Require_RefusesAKindItCannotReadAtAll()
    {
        var error = Assert.Throws<ArgumentContractException>(() => Require("thirty", Int64Range.Unbounded));

        Assert.Equal(ArgumentContractFailure.NotAnInteger, error.Failure);
        Assert.Contains("Text", error.Message, StringComparison.Ordinal);
    }

    /// <summary>The property the whole contract exists for: NO accepted value differs from what the
    /// caller wrote, and no rejected value is silently altered.</summary>
    [Fact]
    public void Require_NeverPublishesAValueTheCallerDidNotWrite()
    {
        var rng = new Random(20260916);
        for (int i = 0; i < 500; i++)
        {
            long n = (long)(rng.NextDouble() * 2.0 * 1_000_000.0) - 1_000_000L;
            // exact: accepted and identical however it is written
            Assert.Equal(n, Require(new Int(n), Int64Range.Unbounded));
            Assert.Equal(n, Require(Rl.Parse(n.ToString() + ".000", null), Int64Range.Unbounded));
            // fractional: refused, never rounded to n or n+1
            Assert.Throws<ArgumentContractException>(
                () => Require(Rl.Parse(n.ToString() + ".5", null), Int64Range.Unbounded));
        }
    }

    [Fact]
    public void Refusal_IsAnArgumentException_SoTheExistingWireTaxonomyApplies()
    {
        var error = Assert.Throws<ArgumentContractException>(() => Require(new Int(-1), Int64Range.NonNegative));

        Assert.IsAssignableFrom<ArgumentException>(error);
    }

    /// <summary>
    /// The message speaks the ONE grammar the protocol document fixes for an argument refusal — the
    /// builtin, the 1-based POSITION and the parameter name:
    /// <c>probe(): argument 2 (count) must be …; got …</c>. The shape sweep over the live registry
    /// (<c>Lovelace.Run.Tests/BuiltinArgumentShapeSweepTests</c>) enforces the same grammar, so a
    /// refusal that names only the parameter would fail there; this pins it here, next to the type.
    /// </summary>
    [Fact]
    public void Refusal_SpeaksTheDocumentedArgumentGrammar()
    {
        var error = Assert.Throws<ArgumentContractException>(() => Require(new Int(-1), Int64Range.NonNegative));

        Assert.Equal("probe", error.Builtin);
        Assert.Equal(2, error.Position);
        Assert.Matches(
            @"^probe\(\): argument 2 \(count\) must be a Natural or Integer .+; got -1\.$",
            error.Message);
    }
}
