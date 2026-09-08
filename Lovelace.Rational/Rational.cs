using System.Diagnostics.CodeAnalysis;
using System.Globalization;
using Int = global::Lovelace.Integer.Integer;
using Nat = global::Lovelace.Natural.Natural;

namespace Lovelace.Rational;

/// <summary>
/// An exact arbitrary-precision rational number: an immutable, normalized pair
/// <c>numerator / denominator</c> with <c>denominator &gt; 0</c> and
/// <c>gcd(numerator, denominator) = 1</c> (sign lives on the numerator; zero is 0/1).
/// This is the coefficient domain of the symbolic kernel.
/// </summary>
public sealed class Rational : IEquatable<Rational>, IComparable<Rational>, IComparable,
    IParsable<Rational>, IFormattable
{
    private readonly Int _numerator;
    private readonly Int _denominator;

    private Rational(Int numerator, Int denominator)
    {
        _numerator = numerator;
        _denominator = denominator;
    }

    // -----------------------------------------------------------------
    // Identities and construction
    // -----------------------------------------------------------------

    public static Rational Zero { get; } = new(Int.Zero, Int.One);
    public static Rational One { get; } = new(Int.One, Int.One);
    public static Rational MinusOne { get; } = new(Int.NegativeOne, Int.One);

    /// <summary>The (possibly negative, possibly zero) numerator.</summary>
    public Int Numerator => _numerator;

    /// <summary>The strictly positive denominator.</summary>
    public Int Denominator => _denominator;

    /// <summary>Constructs the normalized rational <paramref name="numerator"/> / <paramref name="denominator"/>.</summary>
    /// <exception cref="DivideByZeroException">When the denominator is zero.</exception>
    public static Rational From(Int numerator, Int denominator)
    {
        if (Int.IsZero(denominator))
            throw new DivideByZeroException("Rational denominator cannot be zero.");
        if (Int.IsZero(numerator))
            return Zero;
        var g = Int.Gcd(numerator, denominator);
        numerator = numerator / g;
        denominator = denominator / g;
        if (Int.IsNegative(denominator))
        {
            numerator = numerator.Negate();
            denominator = denominator.Negate();
        }
        return new Rational(numerator, denominator);
    }

    /// <summary>Constructs the integer rational <paramref name="numerator"/> / 1.</summary>
    public static Rational From(Int numerator) => new(numerator, Int.One);

    /// <summary>Constructs the normalized rational <paramref name="num"/> / <paramref name="den"/> from machine integers.</summary>
    public static Rational From(long num, long den) => From(new Int(num), new Int(den));

    /// <summary>Constructs an integer rational from a machine integer.</summary>
    public static implicit operator Rational(long value) => FromLong(value);

    /// <summary>Constructs a rational from a machine integer.</summary>
    public static Rational FromLong(long value) => From(Int.Parse(value.ToString(CultureInfo.InvariantCulture), null));

    // -----------------------------------------------------------------
    // Predicates
    // -----------------------------------------------------------------

    public bool IsZero => Int.IsZero(_numerator);
    public bool IsOne => _denominator == Int.One && _numerator == Int.One;
    public bool IsMinusOne => _denominator == Int.One && _numerator == Int.NegativeOne;
    public bool IsNegative => Int.IsNegative(_numerator);
    public bool IsPositive => !IsZero && !IsNegative;
    public bool IsInteger => _denominator == Int.One;
    public int Sign => IsZero ? 0 : (IsNegative ? -1 : 1);

    // -----------------------------------------------------------------
    // Arithmetic (exact)
    // -----------------------------------------------------------------

    public static Rational Add(Rational a, Rational b) =>
        From(a._numerator * b._denominator + b._numerator * a._denominator, a._denominator * b._denominator);

    public static Rational Subtract(Rational a, Rational b) =>
        From(a._numerator * b._denominator - b._numerator * a._denominator, a._denominator * b._denominator);

    public static Rational Multiply(Rational a, Rational b) =>
        From(a._numerator * b._numerator, a._denominator * b._denominator);

    /// <exception cref="DivideByZeroException">When <paramref name="b"/> is zero.</exception>
    public static Rational Divide(Rational a, Rational b)
    {
        if (b.IsZero)
            throw new DivideByZeroException("Cannot divide by zero.");
        return From(a._numerator * b._denominator, a._denominator * b._numerator);
    }

    public static Rational Negate(Rational a) => new(a._numerator.Negate(), a._denominator);

    public static Rational Abs(Rational a) => a.IsNegative ? Negate(a) : a;

    /// <summary>Exact integer power; <c>a^0 = 1</c>; negative exponents require a non-zero base.</summary>
    public static Rational Pow(Rational a, int exponent)
    {
        if (exponent == 0)
            return One;
        if (a.IsZero)
            return Zero;
        var abs = exponent < 0 ? -exponent : exponent;
        var e = new Int((long)abs);
        var num = a._numerator.Pow(e);
        var den = a._denominator.Pow(e);
        var result = exponent < 0 ? From(den, num) : From(num, den);
        return result;
    }

    /// <summary>Rational floor.</summary>
    public Int Floor()
    {
        var q = _numerator / _denominator;
        // truncation toward zero; adjust for negatives so that result <= value
        if (Int.IsNegative(_numerator) && !IsInteger)
            return q - Int.One;
        return q;
    }

    // -----------------------------------------------------------------
    // Conversion
    // -----------------------------------------------------------------

    /// <summary>Exact conversion when the value is an integer; throws otherwise.</summary>
    public Int ToInteger() => IsInteger
        ? _numerator
        : throw new InvalidOperationException($"Rational {this} is not an integer.");

    /// <summary>Exact conversion when the value is an integer.</summary>
    public bool TryToInteger(out Int value)
    {
        value = _numerator;
        return IsInteger;
    }

    private static Rational ParseDecimal(string text)
    {
        var neg = text.StartsWith('-');
        if (neg) text = text[1..];
        var dot = text.IndexOf('.');
        if (dot < 0)
            return From(Int.Parse(text, null)) * (neg ? MinusOne : One);
        var intPart = text[..dot];
        var frac = text[(dot + 1)..];
        var n = Int.Parse(intPart + frac, null);
        var d = Int.One;
        for (int i = 0; i < frac.Length; i++)
            d = d * new Int(10L);
        var r = From(n, d);
        return neg ? Negate(r) : r;
    }

    public static Rational ParsePeriodic(string text)
    {
        // forms: "0.(3)", "0.1(6)", "-2.(142857)"
        var neg = text.StartsWith('-');
        if (neg) text = text[1..];
        int open = text.IndexOf('('), close = text.IndexOf(')');
        var head = text[..open];           // "0." or "0.1"
        var period = text[(open + 1)..close];
        var headR = ParseDecimal(head);
        // period contributes p / (10^len - 1) scaled by 10^(-fracLen(head))
        var fracLen = head.Contains('.') ? head.Length - head.IndexOf('.') - 1 : 0;
        var p = Int.Parse(period, null);
        var den = Int.One;
        for (int i = 0; i < period.Length; i++)
            den = den * new Int(10L);
        den = den - Int.One;
        var shift = Int.One;
        for (int i = 0; i < fracLen; i++)
            shift = shift * new Int(10L);
        var tail = From(p, den * shift);
        var result = Add(headR, tail);
        return neg ? Negate(result) : result;
    }

    /// <summary>Decimal expansion as text, with period notation when a repeat is detected.</summary>
    public string ToDecimalString(int maxFractionalDigits)
    {
        var n = Int.Abs(_numerator);
        var d = _denominator;
        var q = n / d;
        var rem = n - q * d;
        var sb = new System.Text.StringBuilder((IsNegative ? "-" : "") + q);
        if (Int.IsZero(rem))
            return sb.ToString();
        sb.Append('.');
        var seen = new Dictionary<Int, int>();
        var frac = new System.Text.StringBuilder();
        int pos = 0;
        while (!Int.IsZero(rem))
        {
            if (seen.TryGetValue(rem, out var start))
            {
                var head = frac.ToString()[..start];
                var period = frac.ToString()[start..];
                return sb.Append(head).Append('(').Append(period).Append(')').ToString();
            }
            seen[rem] = pos;
            rem = rem * new Int(10L);
            var digit = rem / d;
            rem = rem - digit * d;
            frac.Append(digit);
            pos++;
            if (pos > maxFractionalDigits)
                return sb.Append(frac).ToString();
        }
        return sb.Append(frac).ToString();
    }

    // -----------------------------------------------------------------
    // Comparison / equality
    // -----------------------------------------------------------------

    public int CompareTo(Rational? other)
    {
        if (other is null) return 1;
        var lhs = _numerator * other._denominator;
        var rhs = other._numerator * _denominator;
        return lhs.CompareTo(rhs);
    }

    public int CompareTo(object? obj) => obj is Rational r ? CompareTo(r) : 1;

    public bool Equals(Rational? other) =>
        other is not null && _numerator == other._numerator && _denominator == other._denominator;

    public override bool Equals(object? obj) => obj is Rational r && Equals(r);

    public override int GetHashCode() => HashCode.Combine(_numerator, _denominator);

    // -----------------------------------------------------------------
    // Operators
    // -----------------------------------------------------------------

    public static Rational operator +(Rational a, Rational b) => Add(a, b);
    public static Rational operator -(Rational a, Rational b) => Subtract(a, b);
    public static Rational operator *(Rational a, Rational b) => Multiply(a, b);
    public static Rational operator /(Rational a, Rational b) => Divide(a, b);
    public static Rational operator -(Rational a) => Negate(a);
    public static bool operator ==(Rational? a, Rational? b) => a is null ? b is null : a.Equals(b);
    public static bool operator !=(Rational? a, Rational? b) => !(a == b);
    public static bool operator <(Rational a, Rational b) => a.CompareTo(b) < 0;
    public static bool operator >(Rational a, Rational b) => a.CompareTo(b) > 0;
    public static bool operator <=(Rational a, Rational b) => a.CompareTo(b) <= 0;
    public static bool operator >=(Rational a, Rational b) => a.CompareTo(b) >= 0;

    // -----------------------------------------------------------------
    // Parsing / formatting
    // -----------------------------------------------------------------

    /// <summary>Parses "p/q", "p", or a decimal "p.q" (also with period notation "p.(q)").</summary>
    public static Rational Parse(string s, IFormatProvider? provider)
    {
        if (s.Contains('(', StringComparison.Ordinal))
            return ParsePeriodic(s);
        var slash = s.IndexOf('/');
        if (slash >= 0)
            return From(Int.Parse(s[..slash], null), Int.Parse(s[(slash + 1)..], null));
        return ParseDecimal(s);
    }

    public static bool TryParse([NotNullWhen(true)] string? s, IFormatProvider? provider, [MaybeNullWhen(false)] out Rational result)
    {
        result = null;
        try
        {
            if (s is null) return false;
            result = Parse(s, provider);
            return true;
        }
        catch (Exception)
        {
            return false;
        }
    }

    public static Rational Parse(string s) => Parse(s, null);

    public static bool TryParse([NotNullWhen(true)] string? s, [MaybeNullWhen(false)] out Rational result) =>
        TryParse(s, null, out result);

    public string ToString(string? format, IFormatProvider? formatProvider) => ToString();

    public override string ToString() =>
        IsInteger ? _numerator.ToString() : _numerator.ToString() + "/" + _denominator.ToString();
}
