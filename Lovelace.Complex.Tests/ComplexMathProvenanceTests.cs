using Lovelace.Complex;
using Lovelace.Real;
using Xunit;

using Rl = Lovelace.Real.Real;

namespace Lovelace.Complex.Tests;

/// <summary>
/// Properties of the exactness a value carries out of <see cref="ComplexMath"/>.
///
/// <para>Every public entry point ends in a truncation to the ambient precision, and the truncation
/// used to be performed by rendering a decimal string and parsing it back —
/// <c>Rl.Parse(...)</c> — which rebuilt the value with a fresh, EXACT provenance.  A truncated
/// transcendental therefore crossed the wire as <c>exact: true</c> with a rational
/// numerator/denominator (the audit's P0 "evalf certifies a truncated transcendental as exact"),
/// and so did the truncating divisions inside the series.  These properties pin the flag to the
/// operation: a value that lost digits is not exact, a value that lost none keeps whatever it
/// had.</para>
///
/// <para>The second property is the audit's F6: a non-zero value whose leading zeros reach past the
/// digit budget was truncated to exactly <c>0</c>.  Zero is not a truncation of such a value's
/// digits, it is the loss of the value.</para>
///
/// <para>Both run under a 30-place budget: the ambient 1000-place default makes the series carry
/// ~1000 digits per term in its own representation, which is a pre-existing cost this file has no
/// business paying.</para>
/// </summary>
public class ComplexMathProvenanceTests
{
    private const long Budget = 30L;

    /// <summary>Every truncated transcendental says so, whatever produced it.</summary>
    [Fact]
    public void TruncatedTranscendentals_AreNotExact()
    {
        using var scope = Rl.WithPrecision(Budget, Budget);

        Rl[] arguments =
        {
            Rl.One,
            new Rl("0.5"),
            new Rl("2"),
            Rl.Divide(Rl.One, new Rl("3")),
            Rl.Divide(new Rl("7"), new Rl("5")),
        };

        foreach (Rl x in arguments)
        {
            Rl sin = ComplexMath.Sin(x);
            Rl cos = ComplexMath.Cos(x);
            Rl tan = ComplexMath.Tan(x);
            Rl exp = ComplexMath.Exp(x);
            Rl ln = ComplexMath.Ln(x);

            Assert.False(sin.IsExact, $"sin({x}) = {sin} claims exactness");
            Assert.False(cos.IsExact, $"cos({x}) = {cos} claims exactness");
            Assert.False(tan.IsExact, $"tan({x}) = {tan} claims exactness");
            Assert.False(exp.IsExact, $"exp({x}) = {exp} claims exactness");
            Assert.False(ln.IsExact, $"ln({x}) = {ln} claims exactness");
        }

        Assert.False(ComplexMath.Atan(Rl.One).IsExact, "atan(1) claims exactness");
    }

    /// <summary>The values that are exact stay exact: the guards and the exact multiples.</summary>
    [Fact]
    public void ExactResults_KeepTheirExactness()
    {
        using var scope = Rl.WithPrecision(Budget, Budget);

        Assert.True(ComplexMath.Sin(Rl.Zero).IsExact, "sin(0) lost its exactness");
        Assert.Equal(Rl.Zero, ComplexMath.Sin(Rl.Zero));
        Assert.True(ComplexMath.Cos(Rl.Zero).IsExact, "cos(0) lost its exactness");
        Assert.Equal(Rl.One, ComplexMath.Cos(Rl.Zero));
        Assert.True(ComplexMath.Exp(Rl.Zero).IsExact, "exp(0) lost its exactness");
        Assert.Equal(Rl.One, ComplexMath.Exp(Rl.Zero));

        // Values only for the π multiples: the reduction multiplies by an inexact π, so the exact
        // 0 and −1 it lands on carry the approximation's provenance, exactly as Divide(0, inexact)
        // yields an inexact zero.  The guards above are the cases that are exact without any
        // approximation being touched.
        Assert.Equal(Rl.Zero, ComplexMath.Sin(Rl.Pi));
        Assert.Equal(-Rl.One, ComplexMath.Cos(Rl.Pi));
    }

    /// <summary>
    /// A non-zero result smaller than the truncation window keeps its significant digits instead of
    /// being flushed to zero — the shape the audit measured coming back as exactly 0.
    /// </summary>
    [Fact]
    public void ValuesSmallerThanTheDigitWindow_AreNotFlushedToZero()
    {
        using var scope = Rl.WithPrecision(Budget, Budget);

        Rl tiny = Rl.Parse("0." + new string('0', 99) + "1");
        Rl expNeg70 = ComplexMath.Exp(new Rl("-70"));

        Rl sinTiny = ComplexMath.Sin(tiny);
        Rl tanTiny = ComplexMath.Tan(tiny);

        Assert.False(Rl.IsZero(sinTiny), $"sin(1e-100) = {sinTiny} was flushed to zero");
        Assert.True(Rl.Abs(sinTiny - tiny) < tiny * Rl.Parse("0.01"),
            $"sin(1e-100) = {sinTiny}, expected {tiny}");
        Assert.False(Rl.IsZero(tanTiny), $"tan(1e-100) = {tanTiny} was flushed to zero");
        Assert.True(Rl.Abs(tanTiny - tiny) < tiny * Rl.Parse("0.01"),
            $"tan(1e-100) = {tanTiny}, expected {tiny}");

        // e^-70 = 3.9754…e-31: just below a 30-place window, and very much not zero.
        Assert.False(Rl.IsZero(expNeg70), $"exp(-70) = {expNeg70} was flushed to zero");
        Assert.True(Rl.Abs(expNeg70) > Rl.Parse("0." + new string('0', 30) + "1"),
            $"exp(-70) = {expNeg70} is smaller than 1e-31");
    }

    /// <summary>
    /// The provenance survives the truncation of a value that already carried it: a truncated
    /// operand must not become exact by being truncated again.
    /// </summary>
    [Fact]
    public void TruncatingAnInexactValue_DoesNotMakeItExact()
    {
        using var scope = Rl.WithPrecision(Budget, Budget);

        Rl inexact = ComplexMath.Sin(Rl.One);
        Assert.False(inexact.IsExact);

        Rl truncatedAgain = ComplexMath.Cos(inexact);
        Assert.False(truncatedAgain.IsExact, $"cos(sin(1)) = {truncatedAgain} claims exactness");
        Assert.False(ComplexMath.Exp(inexact).IsExact, "exp(sin(1)) claims exactness");
    }
}
