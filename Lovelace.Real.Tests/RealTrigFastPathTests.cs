using System.Diagnostics;
using System.Reflection;
using Lovelace.Real;

using Int = Lovelace.Integer.Integer;
using Nat = Lovelace.Natural.Natural;

namespace Lovelace.Real.Tests;

/// <summary>
/// N23 equivalence and cost tests for the trigonometric fast paths.
///
/// <para>Every fast path added for N23 is value-preserving by construction and is pinned here
/// against the exact expression it replaces: the special-angle table is cross-checked against
/// <c>pi * num / den</c> computed through the public <see cref="Real.Divide(Real, Real)"/>, the
/// argument reduction's integer quotient against <c>Truncate(value / twoPi)</c>, and the series
/// termination test against <c>Abs(term) &lt; 10^-guard</c>.  Nothing here relaxes an existing
/// assertion; the file adds the cross-checks the cheap paths need in order to be trustworthy.</para>
/// </summary>
public class RealTrigFastPathTests
{
    private const BindingFlags Priv = BindingFlags.NonPublic | BindingFlags.Static;

    /// <summary>The rows of the private special-angle table: multiples of π/6 and π/4 in [0, 2π).</summary>
    private static readonly (long Num, long Den)[] Table =
    {
        (0, 1), (1, 6), (1, 4), (1, 3), (1, 2), (2, 3), (3, 4), (5, 6),
        (1, 1), (7, 6), (5, 4), (4, 3), (3, 2), (5, 3), (7, 4), (11, 6),
    };

    private static readonly long[] Precisions = { 20, 80, 200 };

    private static Real SpecialAngle(Real pi, long num, long den) =>
        num == 0 ? Real.Zero : pi * new Real(new Int(num)) / new Real(new Int(den));

    private static (bool Hit, Real Sin, Real Cos) TrySpecialAngle(Real x, Real pi)
    {
        object?[] args = { x, pi, null, null };
        bool hit = (bool)typeof(Real).GetMethod("TrySpecialAngle", Priv)!.Invoke(null, args)!;
        return (hit, (Real)args[2]!, (Real)args[3]!);
    }

    /// <summary>
    /// Every angle built the original way (<c>pi*num/den</c> through the public divide) must still
    /// be recognised by the table, with the exact value it promises: this is what makes the cheap
    /// construction and the "x stores nothing that far down" pre-reject safe — a miss would send a
    /// special angle down the Taylor path and change its digits.
    /// </summary>
    [Fact]
    public void SpecialAngles_BuiltTheOriginalWay_StillHitTheTableExactly()
    {
        foreach (long p in Precisions)
        {
            using var scope = Real.WithPrecision(p, p);
            Real pi = Real.Pi;
            Real half = new Real("0.5");
            Real negHalf = new Real("-0.5");
            Real negOne = new Real("-1");
            Real sqrt2Half = Real.Sqrt(new Real("2")) / new Real("2");
            Real sqrt3Half = Real.Sqrt(new Real("3")) / new Real("2");

            var expected = new Dictionary<(long, long), (Real Sin, Real Cos)>
            {
                [(0, 1)] = (Real.Zero, Real.One),
                [(1, 6)] = (half, sqrt3Half),
                [(1, 4)] = (sqrt2Half, sqrt2Half),
                [(1, 3)] = (sqrt3Half, half),
                [(1, 2)] = (Real.One, Real.Zero),
                [(2, 3)] = (sqrt3Half, negHalf),
                [(3, 4)] = (sqrt2Half, -sqrt2Half),
                [(5, 6)] = (half, -sqrt3Half),
                [(1, 1)] = (Real.Zero, negOne),
                [(7, 6)] = (negHalf, -sqrt3Half),
                [(5, 4)] = (-sqrt2Half, -sqrt2Half),
                [(4, 3)] = (-sqrt3Half, negHalf),
                [(3, 2)] = (negOne, Real.Zero),
                [(5, 3)] = (-sqrt3Half, half),
                [(7, 4)] = (-sqrt2Half, sqrt2Half),
                [(11, 6)] = (negHalf, sqrt3Half),
            };

            foreach (var (num, den) in Table)
            {
                Real angle = SpecialAngle(pi, num, den);
                (bool hit, Real sin, Real cos) = TrySpecialAngle(angle, pi);

                Assert.True(hit, $"pi*{num}/{den} at {p} places fell through the special-angle table");
                Assert.Equal(expected[(num, den)].Sin, sin);
                Assert.Equal(expected[(num, den)].Cos, cos);
            }
        }
    }

    /// <summary>
    /// Angles that are not rational multiples of π must not be captured, including angles that sit
    /// one unit in the last place away from a special one — the pre-reject must not manufacture a
    /// match, and the cheap angle must not either.
    /// </summary>
    [Fact]
    public void NonSpecialAngles_AreNotCapturedByTheTable()
    {
        foreach (long p in Precisions)
        {
            using var scope = Real.WithPrecision(p, p);
            Real pi = Real.Pi;

            foreach (Real angle in new[]
                     {
                         new Real("0.5"), new Real("1"), new Real("2"), new Real("7"),
                         new Real("1") / new Real("3"),
                         new Real("0.52359877559829887307710723054658381403286156656251763682915743205130273438103483310467247089035284466"),
                     })
            {
                Assert.False(TrySpecialAngle(angle, pi).Hit, $"{angle} at {p} places was captured by the table");
            }

            // One unit in the last place below each special angle: never a special angle.
            foreach (var (num, den) in Table)
            {
                if (num == 0)
                    continue;
                Real angle = SpecialAngle(pi, num, den);
                Real ulp = new Real("0." + new string('0', (int)p - 1) + "1");
                Assert.False(TrySpecialAngle(angle - ulp, pi).Hit,
                    $"pi*{num}/{den} - 1ulp at {p} places was captured by the table");
            }
        }
    }

    /// <summary>
    /// The reduction's integer quotient must equal <c>Truncate(Divide(value, twoPi))</c> — the
    /// expression it replaces — for angles below, at and far above one turn.
    /// </summary>
    [Fact]
    public void ArgumentReduction_IntegerQuotient_MatchesTheDividedForm()
    {
        string[] values = { "0.5", "1", "2", "6", "6.28318530717958647692528676655900576839433879875021", "12", "100", "1000", "12345.6789", "0.000001" };

        foreach (long p in Precisions)
        {
            using var scope = Real.WithPrecision(p, p);
            Real pi = Real.Pi;
            Real twoPi = pi * new Real("2");
            MethodInfo reduce = typeof(Real).GetMethod("ReduceToTwoPi", Priv)!;
            MethodInfo whole = typeof(Real).GetMethod("WholeQuotient", Priv)!;
            MethodInfo truncate = typeof(Real).GetMethod("Truncate", Priv)!;

            foreach (string text in values)
            {
                Real value = new Real(text);
                Real viaDivide = (Real)truncate.Invoke(null, new object[] { value / twoPi })!;
                Real viaInteger = (Real)whole.Invoke(null, new object[] { value, twoPi })!;

                Assert.Equal(viaDivide.Exponent, viaInteger.Exponent);
                Assert.Equal(viaDivide.ToNatural().ToString(), viaInteger.ToNatural().ToString());
                Assert.Equal(viaDivide, viaInteger);

                Real reduced = (Real)reduce.Invoke(null, new object[] { value, pi, twoPi })!;
                Real expected = value - twoPi * viaDivide;
                if (expected < Real.Zero) expected += twoPi;
                Assert.Equal(expected, reduced);
            }
        }
    }

    /// <summary>
    /// The series termination test must agree with the <c>Abs(term) &lt; 10^-guard</c> it replaces,
    /// on both sides of the boundary and on values far away from it.
    /// </summary>
    [Fact]
    public void SeriesTermination_IntegerTest_MatchesTheThresholdComparison()
    {
        const long guard = 60L;
        Real threshold = new Real("0." + new string('0', (int)guard) + "1");
        MethodInfo below = typeof(Real).GetMethod("IsBelowDecimalGuard", Priv)!;

        string[] terms =
        {
            "0", "0.1", "1",
            // The threshold literal is "0." + guard zeros + "1" = 10^-(guard+1): sit exactly on it,
            // just under it and just over it.
            "0." + new string('0', (int)guard) + "1",
            "0." + new string('0', (int)guard) + "09",
            "0." + new string('0', (int)guard) + "11",
            "0." + new string('0', (int)guard - 1) + "1",
            "0." + new string('0', (int)guard) + "9",
            "0." + new string('0', (int)guard - 1) + "9",
            "12.5",
            "0.999999999999999999999999999999999999999999999999999999999999",
        };

        foreach (string text in terms)
        {
            Real term = new Real(text);
            foreach (Real candidate in new[] { term, -term })
            {
                bool expected = Real.Abs(candidate) < threshold;
                object?[] args = { candidate, guard, null, 0L };
                bool actual = (bool)below.Invoke(null, args)!;
                Assert.True(expected == actual,
                    $"term '{text}' (exp={candidate.Exponent}, digits={candidate.ToNatural().ToString().Length}) expected {expected} but the integer test said {actual}");
            }
        }
    }

    /// <summary>
    /// A long series run keeps the carried power of ten in step as the term exponents fall: the
    /// integer test must keep matching the threshold comparison for many successive terms, and the
    /// loop it drives must end on the same term as the private series it replicates.
    /// </summary>
    [Fact]
    public void SeriesTermination_IntegerTest_TracksTheCarriedPowerAcrossManyTerms()
    {
        const long guard = 200L;
        Real threshold = new Real("0." + new string('0', (int)guard) + "1");
        MethodInfo below = typeof(Real).GetMethod("IsBelowDecimalGuard", Priv)!;
        MethodInfo cosTaylor = typeof(Real).GetMethod("CosTaylor", Priv)!;

        using var scope = Real.WithPrecision(200, 200);

        Real x = new Real("0.5");
        Real x2 = x * x;
        Real term = Real.One;
        Real sum = Real.One;

        object?[] args = { term, guard, null, 0L };
        long exponent = 0L;
        Nat? power = null;
        long iterations = 0L;

        for (long k = 1; ; k++)
        {
            long denom = (2 * k - 1) * (2 * k);
            term = Real.DivideNonPeriodic(-term * x2, new Real(new Int(denom)), guard);
            sum = sum + term;

            bool expected = Real.Abs(term) < threshold;
            args[0] = term;
            args[2] = power;
            args[3] = exponent;
            bool actual = (bool)below.Invoke(null, args)!;
            power = (Nat?)args[2];
            exponent = (long)args[3]!;

            Assert.Equal(expected, actual);

            iterations = k;
            if (actual)
                break;
        }

        Assert.True(iterations > 20, $"the series ended after only {iterations} terms");
        Assert.Equal((Real)cosTaylor.Invoke(null, new object[] { x, guard })!, sum);
    }

    /// <summary>
    /// The whole point of N23: at the default 1000-place precision a single cosine must not take
    /// tens of seconds.  The bound is loose — an order of magnitude above the measured cost — since
    /// it guards against the per-term decimal-rendering regression, not against machine speed.
    /// <para>
    /// Category "Timing": this is the one assertion in this project whose verdict depends on the
    /// wall clock, so it must not be measured under coverage instrumentation. CI runs it in its own
    /// step, with no coverage collector attached (cycle 5: instrumented runs made a healthy 1 s
    /// cosine exceed the 10 s budget and turned the fast-tests job red for a reason that had nothing
    /// to do with the product).
    /// </para>
    /// </summary>
    [Fact]
    [Trait("Category", "Timing")]
    public void Cos_AtDefaultPrecision_StaysInteractive()
    {
        using var scope = Real.WithPrecision(1000, 40);
        Real arg = new Real("0.5");

        Real.Cos(arg); // warm the cached π and the JIT

        var sw = Stopwatch.StartNew();
        Real value = Real.Cos(arg);
        sw.Stop();

        Assert.False(value.IsPeriodic);
        Assert.True(sw.ElapsedMilliseconds < 10_000,
            $"cos(0.5) at 1000 places took {sw.ElapsedMilliseconds} ms");
    }

    /// <summary>
    /// <c>Normalize</c> now strips trailing zeros with integer division instead of rendering the
    /// digits and parsing them back; both must agree on every shape, including values whose whole
    /// fraction is zeros and values with more trailing zeros than the exponent can absorb.
    /// </summary>
    [Fact]
    public void Normalize_BinaryStrip_MatchesTheDigitStringStrip()
    {
        string[] values =
        {
            "8.000", "1", "0.500", "1.2300", "100", "0.000", "-1.250", "123.456", "0.0",
            "-0", "0.0000000000000000000000000000000000000000010000",
            "1" + "0", "1.00000000000000000000000000000000000000000000000000",
            "7.700000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        };

        MethodInfo normalize = typeof(Real).GetMethod("Normalize", Priv)!;

        foreach (string text in values)
        {
            Real value = new Real(text);
            Real actual = (Real)normalize.Invoke(null, new object[] { value })!;
            Real expected = ReferenceNormalize(value);

            Assert.Equal(expected.Exponent, actual.Exponent);
            Assert.Equal(expected.ToNatural().ToString(), actual.ToNatural().ToString());
            Assert.Equal(expected, actual);
        }
    }

    /// <summary>
    /// <c>ToInteger(zeros)</c> (the exponent alignment every addition runs) now shifts the binary
    /// magnitude instead of padding the rendered digits; both must produce the same integer.
    /// </summary>
    [Fact]
    public void ToInteger_BinaryShift_MatchesTheDigitPadding()
    {
        string[] values = { "1", "-1", "8.000", "0.5", "123.456", "100", "0", "0.000", "-0.25",
                            "3.14159265358979323846264338327950288419716939937510" };

        MethodInfo toInteger = typeof(Real).GetMethod("ToInteger", BindingFlags.NonPublic | BindingFlags.Instance)!;

        foreach (string text in values)
        {
            Real value = new Real(text);
            foreach (long zeros in new long[] { 0, 1, 17, 18, 19, 40 })
            {
                var actual = (Int)toInteger.Invoke(value, new object[] { zeros })!;
                var expected = new Int(
                    Nat.Parse(value.ToNatural().ToString() + new string('0', (int)zeros), null),
                    Int.IsNegative(value));

                Assert.Equal(expected.ToNatural().ToString(), actual.ToNatural().ToString());
                Assert.Equal(Int.IsNegative(expected), Int.IsNegative(actual));
            }
        }
    }

    /// <summary>
    /// Halving a π-shaped value must be exactly what the public divide produced — π/2 feeds the
    /// quadrant comparisons in both <see cref="Real.Sin(Real)"/> and <see cref="Real.Cos(Real)"/>,
    /// so a digit of difference would move an angle across a branch.
    /// </summary>
    [Fact]
    public void HalfOf_MatchesTheDividedForm_ForPiShapedValues()
    {
        MethodInfo halfOf = typeof(Real).GetMethod("HalfOf", Priv)!;
        Real two = new Real("2");

        foreach (long p in new long[] { 20, 80, 200, 1000 })
        {
            using var scope = Real.WithPrecision(p, p);
            Real pi = Real.Pi;
            Assert.Equal(-p, pi.Exponent);

            Real expected = pi / two;
            Real actual = (Real)halfOf.Invoke(null, new object[] { pi })!;

            Assert.Equal(expected.Exponent, actual.Exponent);
            Assert.Equal(expected.ToNatural().ToString(), actual.ToNatural().ToString());
            Assert.Equal(expected, actual);

            // Values that are not stored at the active precision keep the general path.
            foreach (Real other in new[] { new Real("3"), new Real("0.25"), new Real("1.5") })
                Assert.Equal(other / two, (Real)halfOf.Invoke(null, new object[] { other })!);
        }
    }

    /// <summary>
    /// The strip must also handle magnitudes carrying long zero runs and caps larger than the digit
    /// count (the galloping step doubles and halves), and it must never eat a non-zero digit.
    /// </summary>
    [Fact]
    public void Normalize_BinaryStrip_MatchesTheDigitStringStrip_ForLongZeroRuns()
    {
        MethodInfo normalize = typeof(Real).GetMethod("Normalize", Priv)!;

        foreach (int zeros in new[] { 0, 1, 17, 18, 19, 37, 200, 2000 })
        {
            foreach (long cap in new long[] { 0, 1, 18, 19, zeros, zeros + 1, zeros + 500 })
            {
                Nat magnitude = Nat.Parse("1" + new string('0', zeros), null);
                var value = new Real(magnitude, false, -cap);

                Real actual = (Real)normalize.Invoke(null, new object[] { value })!;
                Real expected = ReferenceNormalize(value);

                Assert.Equal(expected.Exponent, actual.Exponent);
                Assert.Equal(expected.ToNatural().ToString(), actual.ToNatural().ToString());
                Assert.Equal(expected, actual);
            }
        }
    }

    /// <summary>The digit-string trailing-zero strip this file pins the binary strip against
    /// (the implementation <c>Normalize</c> used before N23).</summary>
    private static Real ReferenceNormalize(Real r)
    {
        if (r.IsPeriodic || r.Exponent >= 0 || Int.IsZero(r))
            return r;

        string digits = r.ToNatural().ToString();
        long maxStrip = -r.Exponent;

        int stripped = 0;
        while (stripped < (int)maxStrip && stripped < digits.Length && digits[digits.Length - 1 - stripped] == '0')
            stripped++;

        if (stripped == 0)
            return r;

        string newDigits = stripped >= digits.Length ? "0" : digits[..^stripped];
        Nat magnitude = Nat.Parse(newDigits, null);
        bool isNeg = Int.IsNegative(r) && !Nat.IsZero(magnitude);
        return new Real(magnitude, isNeg, r.Exponent + stripped);
    }
}
