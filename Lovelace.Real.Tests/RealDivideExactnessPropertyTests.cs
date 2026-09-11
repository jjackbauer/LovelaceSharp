using Lovelace.Real;
using Xunit;

using Int = Lovelace.Integer.Integer;

namespace Lovelace.Real.Tests;

/// <summary>
/// Properties of exact rational division and its multiplication twin:
/// <c>(a/b)/(c/d)</c> and <c>(a/b)·(d/c)</c> are the same value, and when that value is
/// representable both spellings must carry the same exactness.
///
/// <para>The defect these pin: <see cref="Real.Divide(Real, Real)"/> expanded every periodic
/// operand to <see cref="Real.MaxComputationDecimalPlaces"/> places before dividing — a truncation —
/// so the quotient of two exact periodic values was flagged inexact even when it is exactly
/// representable, and the two spellings disagreed: <c>(5/6)/(7/11)</c> came back
/// <c>exact:false</c> while <c>(5/6)*(11/7)</c> came back <c>exact:true</c> with numerator 55/42
/// on the identical value.  The quotient is decided by the exact fractions the operands denote,
/// which is the path <see cref="Real.Multiply(Real, Real)"/> already used.</para>
/// </summary>
public class RealDivideExactnessPropertyTests
{
    /// <summary>Numerators and denominators of the family: short, coprime-ish, and small enough
    /// that every quotient's period fits the default budget.</summary>
    private static readonly long[] Values = { 1L, 2L, 3L, 5L, 6L, 7L, 11L, 13L, 17L, 23L };

    private static Real Of(long n) => new Real(new Int(n));

    /// <summary>Every quotient of the family is exactly what its multiplication twin says: same
    /// value, same exactness, and the fraction the audit's probe named (a·d)/(b·c).</summary>
    [Fact]
    public void DivisionAndItsMultiplicationTwin_AgreeOnValueAndExactness()
    {
        foreach (long a in Values)
        foreach (long b in Values)
        foreach (long c in Values)
        foreach (long d in Values)
        {
            Real left = Real.Divide(Of(a), Of(b));
            Real right = Real.Divide(Of(c), Of(d));

            Real quotient = Real.Divide(left, right);
            Real twin = Real.Multiply(left, Real.Divide(Of(d), Of(c)));
            Real direct = Real.Divide(Of(a * d), Of(b * c));

            string what = $"({a}/{b})/({c}/{d})";
            Assert.True(quotient == twin, $"{what} = {quotient} but the twin {a}/{b}*{d}/{c} = {twin}");
            Assert.True(quotient == direct, $"{what} = {quotient} but ({a}*{d})/({b}*{c}) = {direct}");
            Assert.Equal(twin.IsExact, quotient.IsExact);

            // The period of every value in this family fits, so the exact spelling must be exact
            // and the quotient must multiply back to its dividend.
            Assert.True(quotient.IsExact, $"{what} = {quotient} was reported inexact");
            Assert.True(Real.Multiply(quotient, right) == left,
                $"{what} * ({c}/{d}) did not return {a}/{b}");

            // Dividing by a ratio is multiplying by its inverse — the identity the asymmetry broke.
            Assert.True(quotient == Real.Multiply(left, Real.Divide(Of(d), Of(c))),
                $"{what} disagrees with multiplying by the inverse");
        }
    }

    /// <summary>
    /// Terminating quotients of periodic operands stay exact, and carry the exact rational rather
    /// than a truncated operand: <c>0.(3)/0.(3) = 1</c>, <c>0.(9)/2 = 0.5</c>, <c>0.(3)/0.5 = 0.(6)</c>.
    /// </summary>
    [Fact]
    public void DividingPeriodicOperands_KeepsRepresentableQuotientsExact()
    {
        // Each operand is a literal (periodic ones included), the expectation is the fraction
        // 1/1, 1/2, 2/3, 4/1, 2/1 built by exact division of integers.
        (string Left, string Right, string ExpNum, string ExpDen)[] cases =
        {
            ("0.(3)", "0.(3)", "1", "1"),
            ("0.(9)", "2", "1", "2"),
            ("0.(3)", "0.5", "2", "3"),
            ("0.(142857)", "0.(142857)", "1", "1"),
            ("1.(3)", "0.(3)", "4", "1"),
            ("0.(6)", "0.(3)", "2", "1"),
            ("0.1(6)", "0.(3)", "1", "2"),
            ("0.(9)", "0.(3)", "3", "1"),
        };

        foreach ((string leftText, string rightText, string expNum, string expDen) in cases)
        {
            Real left = Real.Parse(leftText);
            Real right = Real.Parse(rightText);
            Real expected = Real.Divide(Real.Parse(expNum), Real.Parse(expDen));

            Real quotient = Real.Divide(left, right);

            Assert.True(quotient == expected, $"{leftText}/{rightText} = {quotient}, expected {expected}");
            Assert.True(quotient.IsExact, $"{leftText}/{rightText} = {quotient} was reported inexact");
        }

        // The audit's own probe, whose two spellings disagreed: (5/6)/(7/11) against (5/6)*(11/7).
        Real fiveSixths = Real.Divide(Of(5L), Of(6L));
        Real sevenElevenths = Real.Divide(Of(7L), Of(11L));
        Real byDivision = Real.Divide(fiveSixths, sevenElevenths);
        Real byMultiplication = Real.Multiply(fiveSixths, Real.Divide(Of(11L), Of(7L)));

        Assert.True(byDivision == byMultiplication,
            $"(5/6)/(7/11) = {byDivision} but (5/6)*(11/7) = {byMultiplication}");
        Assert.True(byDivision.IsExact && byMultiplication.IsExact,
            $"(5/6)/(7/11) exact = {byDivision.IsExact}, (5/6)*(11/7) exact = {byMultiplication.IsExact}");
        Assert.True(byDivision == Real.Divide(Of(55L), Of(42L)),
            $"(5/6)/(7/11) = {byDivision}, expected 55/42");
    }

    /// <summary>
    /// The other half of the contract: a quotient whose period does NOT fit the budget is still a
    /// truncation and must say so, and an inexact operand must not be laundered into an exact
    /// quotient by the exact-fraction path.
    /// </summary>
    [Fact]
    public void QuotientsThatCannotBeRepresented_StayInexact()
    {
        Int ninetySeven = new Int(97L);

        using (Real.WithPrecision(10, 10))
        {
            Real periodBeyondBudget = Real.Divide(Real.Divide(Real.One, new Real(new Int(3L))), new Real(ninetySeven));

            Assert.False(periodBeyondBudget.IsExact,
                $"1/291 has a 96-place period, past the 10-place budget, but was reported exact");
            Assert.True(periodBeyondBudget > Real.Zero && periodBeyondBudget < Real.One,
                $"1/291 at a 10-place budget = {periodBeyondBudget}");
        }

        Real irrational = Real.Sqrt(new Real("2"));
        Real third = Real.Parse("0.(3)");

        Assert.False(Real.Divide(irrational, third).IsExact,
            "sqrt(2)/0.(3) is irrational and must not be exact");
        Assert.False(Real.Divide(third, irrational).IsExact,
            "0.(3)/sqrt(2) is irrational and must not be exact");
    }
}
