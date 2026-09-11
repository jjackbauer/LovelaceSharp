using Lovelace.Real;
using Xunit;

using Int = Lovelace.Integer.Integer;

namespace Lovelace.Real.Tests;

/// <summary>
/// Properties of the periodic literals whose exact rational value is an integer: <c>0.(9)</c> is
/// <c>1</c>, <c>12.(9)</c> is <c>13</c>, <c>0.(99)</c> is <c>1</c>, <c>1.9(9)</c> is <c>2</c>.
///
/// <para>The defect these pin: equality and ordering were decided on the printed forms
/// (<c>ToString() == other.ToString()</c> for periodic pairs), so <c>0.(9) == 1</c> was
/// <c>false</c>, <c>0.(9) &lt; 1</c> was <c>true</c> and <c>0.(9) &gt;= 1</c> was <c>false</c> —
/// while the same object satisfied <c>0.(9) − 1 == 0</c> and crossed the wire with numerator 1 /
/// denominator 1.  A periodic decimal denotes an exact rational
/// (<see cref="Real"/> carries the period, not a truncation), so the comparison layer must compare
/// that rational, which is what these properties assert over the whole family.</para>
/// </summary>
public class RealPeriodicIntegerValuePropertyTests
{
    /// <summary>The integers the prefixes of the family take.</summary>
    private static readonly long[] Prefixes = { 0L, 1L, 2L, 7L, 12L, 123L, 9999L };

    /// <summary>Period lengths of the all-nines blocks: 0.(9), 0.(99), 0.(999), …</summary>
    private static readonly int[] NinesLengths = { 1, 2, 3, 5, 8 };

    /// <summary>Every assertion that "this periodic literal IS that integer" must hold together.</summary>
    private static void AssertIsTheInteger(string literal, Real value, Real expected, string what)
    {
        Assert.True(value == expected, $"{what}: '{literal}' == {expected} was false");
        Assert.True(expected == value, $"{what}: {expected} == '{literal}' was false");
        Assert.False(value != expected, $"{what}: '{literal}' != {expected} was true");
        Assert.False(value < expected, $"{what}: '{literal}' < {expected} was true");
        Assert.False(value > expected, $"{what}: '{literal}' > {expected} was true");
        Assert.True(value <= expected, $"{what}: '{literal}' <= {expected} was false");
        Assert.True(value >= expected, $"{what}: '{literal}' >= {expected} was false");
        Assert.Equal(0, value.CompareTo(expected));
        Assert.Equal(expected.GetHashCode(), value.GetHashCode());

        // The arithmetic layer already agreed with the value; the comparison layer must not be the
        // only one that disagrees.
        Real difference = value - expected;
        Assert.True(difference == Real.Zero, $"{what}: '{literal}' - {expected} = {difference}");
        Assert.True(difference.IsExact, $"{what}: '{literal}' - {expected} lost exactness");
        Assert.True(value + value == expected + expected, $"{what}: doubling '{literal}' disagrees");
        Assert.True(value * new Real(new Int(3)) == expected * new Real(new Int(3)),
            $"{what}: tripling '{literal}' disagrees");
    }

    /// <summary>
    /// <c>prefix.(9…9)</c> with a p-digit block of nines is <c>prefix + 1</c>: the family runs over
    /// every combination of prefix and block length, positive and negative.
    /// </summary>
    [Fact]
    public void PeriodicLiteralOfNines_IsTheNextInteger()
    {
        foreach (long prefix in Prefixes)
        {
            foreach (int p in NinesLengths)
            {
                string literal = prefix + ".(" + new string('9', p) + ")";
                Real expected = new Real(new Int(prefix + 1L));

                AssertIsTheInteger(literal, Real.Parse(literal), expected, "plain");

                string negated = "-" + literal;
                AssertIsTheInteger(negated, Real.Parse(negated), -expected, "negated");
            }
        }
    }

    /// <summary>
    /// A periodic tail after a non-repeating nine: <c>k.9(9)</c> is <c>k + 1</c>, the same family
    /// reached through the non-repeating part.
    /// </summary>
    [Fact]
    public void PeriodicTailAfterNine_IsTheNextInteger()
    {
        for (long k = 0; k <= 5; k++)
        {
            string literal = k + ".9(9)";
            Real expected = new Real(new Int(k + 1L));

            AssertIsTheInteger(literal, Real.Parse(literal), expected, "nine-tail");
        }
    }

    /// <summary>
    /// The same value built two ways — the literal, and the integer plus the periodic unit — must be
    /// indistinguishable to the comparison layer: equal, unordered, and interchangeable in a sum.
    /// </summary>
    [Fact]
    public void PeriodicUnit_AddedToAnInteger_IsTheNextInteger()
    {
        Real unit = Real.Parse("0.(9)");

        for (long k = 0; k <= 8; k++)
        {
            Real sum = new Real(new Int(k)) + unit;
            Real expected = new Real(new Int(k + 1L));

            AssertIsTheInteger($"{k} + 0.(9)", sum, expected, "sum");

            // ... and the whole family is one value class: every member equals every other.
            Assert.True(sum == Real.Parse(k + ".(9)"), $"{k} + 0.(9) != {k}.(9)");
            Assert.True(sum == Real.Parse("0.(9)") * new Real(new Int(k + 1L)),
                $"scaling 0.(9) disagrees at k = {k}");
        }
    }

    /// <summary>
    /// Ordering around the family: a periodic integer is above everything below it and below
    /// everything above it — the strict bounds the audit's <c>0.(9) &lt; 1</c> got wrong.
    /// </summary>
    [Fact]
    public void PeriodicInteger_OrdersAgainstItsNeighbours()
    {
        foreach (long prefix in Prefixes)
        {
            foreach (int p in NinesLengths)
            {
                Real value = Real.Parse(prefix + ".(" + new string('9', p) + ")");
                Real below = new Real(new Int(prefix)) - Real.Parse("0.0000001");
                Real above = new Real(new Int(prefix + 1L)) + Real.Parse("0.0000001");

                Assert.True(below < value, $"'{prefix}.({new string('9', p)})' must be above {below}");
                Assert.True(value < above, $"'{prefix}.({new string('9', p)})' must be below {above}");
                Assert.True(value > below && value < above, $"ordering is not transitive at {prefix}/({p})");
            }
        }
    }
}
