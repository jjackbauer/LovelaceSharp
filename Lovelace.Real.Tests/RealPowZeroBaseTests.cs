using Lovelace.Real;
using Xunit;

using Int = Lovelace.Integer.Integer;

namespace Lovelace.Real.Tests;

/// <summary>
/// A zero base under a NEGATIVE exponent has no value, and the tower must refuse it exactly as the
/// integer path does rather than answer with a number.  <c>Int.Pow</c> already refuses
/// <c>0^-1</c> with a typed <see cref="ArgumentOutOfRangeException"/> naming the BASE;
/// <c>Real.Pow</c> used to short-circuit <c>0^n = 0</c> before it ever looked at the exponent's
/// sign, so <c>0^(-1.0)</c> returned 0 with the digits certified exact.
///
/// <para>The assertions here are about the refusal's identity (type, parameter, message), which is
/// what a host turns into a diagnostic, not about a spelling of the value.</para>
/// </summary>
public class RealPowZeroBaseTests
{
    /// <summary>The message the integer path produces for <c>0^-1</c> — and the message
    /// <c>0^(-1.0)</c> must produce too.</summary>
    private static ArgumentOutOfRangeException IntegerRefusal()
    {
        Int zero = Int.Zero;
        return Assert.Throws<ArgumentOutOfRangeException>(() => zero.Pow(new Int(-1L)));
    }

    [Fact]
    public void Pow_ZeroBaseNegativeRealExponent_RefusesWithTheIntegerPathsRefusal()
    {
        ArgumentOutOfRangeException expected = IntegerRefusal();

        var actual = Assert.Throws<ArgumentOutOfRangeException>(
            () => Real.Zero.Pow(new Real("-1")));

        Assert.Equal(expected.Message, actual.Message);
        Assert.Equal(expected.ParamName, actual.ParamName);
    }

    [Fact]
    public void Pow_ZeroBaseNegativeRealExponent_DoesNotReturnZero()
    {
        Real? result = null;
        try
        {
            result = Real.Zero.Pow(new Real("-1.0"));
        }
        catch (ArgumentOutOfRangeException)
        {
            // the refusal is the point
        }

        Assert.True(result is null,
            $"0^(-1.0) returned {result?.ToString() ?? "null"} instead of refusing");
    }

    [Theory]
    [InlineData("-1")]
    [InlineData("-1.0")]
    [InlineData("-2")]
    [InlineData("-0.5")]
    public void Pow_ZeroBaseNegativeExponent_Refuses(string exponent)
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => Real.Zero.Pow(new Real(exponent)));
    }

    [Theory]
    [InlineData("1")]
    [InlineData("2")]
    [InlineData("5.0")]
    public void Pow_ZeroBasePositiveExponent_IsStillZero(string exponent)
    {
        Real result = Real.Zero.Pow(new Real(exponent));

        Assert.True(Real.IsZero(result), $"0^{exponent} returned {result}");
    }

    /// <summary>A zero base with a POSITIVE fractional exponent keeps the value it always had: the
    /// refusal above is about the sign, not about the exponent being an integer.</summary>
    [Fact]
    public void Pow_ZeroBasePositiveFractionalExponent_IsStillZero()
    {
        Assert.True(Real.IsZero(Real.Zero.Pow(new Real("0.5"))));
    }

    /// <summary>A nonzero base under a negative exponent is untouched by this fix: the tower still
    /// reports that it does not implement it (the route that does is the integer tower's, and
    /// <c>2^-100000</c> goes through it).</summary>
    [Fact]
    public void Pow_NonZeroBaseNegativeExponent_KeepsItsExistingRefusal()
    {
        Assert.Throws<NotImplementedException>(() => new Real("2").Pow(new Real("-3")));
    }
}
