using Rat = global::Lovelace.Rational.Rational;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Symbolics;

/// <summary>
/// Conversions between the exact Rational coefficient type and the existing Real type.
/// Lives in Symbolics (which references both) to keep Lovelace.Rational layered on
/// Integer only.
/// </summary>
public static class RationalReal
{
    /// <summary>
    /// Converts to a Real. Exact when the decimal expansion terminates or repeats
    /// (period detected during long division); truncated to <paramref name="digits"/>
    /// fractional digits otherwise.
    /// </summary>
    public static Rl ToReal(Rat value, int digits)
    {
        if (value.IsInteger)
            return new Rl(value.ToInteger());
        return Rl.Parse(value.ToDecimalString(digits), null);
    }

    /// <summary>
    /// Converts an existing Real to an exact rational. Finite decimals and periodic
    /// decimals convert exactly; a truncated non-periodic value converts to the rational
    /// its decimal text denotes.
    /// </summary>
    public static Rat FromReal(Rl value)
    {
        var text = value.ToString();
        if (text.Contains('(', StringComparison.Ordinal))
            return Rat.ParsePeriodic(text);
        return Rat.Parse(text, null);
    }
}
