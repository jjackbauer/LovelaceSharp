using System.Text;
using Rat = global::Lovelace.Rational.Rational;
using Rl = global::Lovelace.Real.Real;
using Int = global::Lovelace.Integer.Integer;

namespace Lovelace.Symbolics;

/// <summary>
/// Conversions between the exact Rational coefficient type and the existing Real type.
/// Lives in Symbolics (which references both) to keep Lovelace.Rational layered on
/// Integer only.
/// </summary>
public static class RationalReal
{
    /// <summary>
    /// Converts to a Real truncated to <paramref name="digits"/> fractional digits — an
    /// explicit approximate boundary. Period detection is deliberately NOT used: the exact
    /// periodic Real would change the value class from 'approximation at the requested
    /// precision' to 'exact periodic number', and elementary-function paths mishandle
    /// periodic operands.
    /// </summary>
    public static Rl ToReal(Rat value, int digits)
    {
        if (value.IsInteger)
            return new Rl(value.ToInteger());
        return Rl.Parse(TruncatingDecimalString(value, digits), null);
    }

    private static string TruncatingDecimalString(Rat value, int digits)
    {
        var neg = value.IsNegative;
        var num = Int.Abs(value.Numerator);
        var den = value.Denominator;
        var whole = num.DivRem(den, out var remainder);
        var sb = new StringBuilder();
        if (neg)
            sb.Append('-');
        sb.Append(whole.ToString());
        if (digits > 0)
        {
            sb.Append('.');
            var ten = new Int(10L);
            for (int i = 0; i < digits; i++)
            {
                remainder = remainder * ten;
                var digit = remainder.DivRem(den, out remainder);
                sb.Append(digit.ToString());
            }
        }
        return sb.ToString();
    }

    /// <summary>
    /// Converts an existing Real to an exact rational using its full-precision magnitude and
    /// exponent — never the display-truncated ToString form. Periodic values expand their
    /// exact repeating decimal.
    /// </summary>
    public static Rat FromReal(Rl value)
    {
        if (value.IsPeriodic)
            return Rat.ParsePeriodic(value.ToString());   // period notation is never truncated
        var d = new Int(value.ToNatural());
        if (Rl.IsNegative(value))
            d = d.Negate();
        var ten = new Int(10L);
        if (value.Exponent > 0)
            return Rat.From(d * ten.Pow(new Int(value.Exponent)));
        if (value.Exponent < 0)
            return Rat.From(d, ten.Pow(new Int(-value.Exponent)));
        return Rat.From(d);
    }
}
