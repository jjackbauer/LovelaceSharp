using System.Diagnostics.CodeAnalysis;
using System.Globalization;
using System.Numerics;
// 'Natural' cannot be used as an alias because the 'Lovelace.Natural' namespace
// shadows it inside 'Lovelace.Integer'. Use 'Nat' instead.
using Nat = global::Lovelace.Natural.Natural;

namespace Lovelace.Integer;

/// <summary>
/// Arbitrary-precision signed integer (ℤ).
/// Backed by a <see cref="Nat"/> magnitude and a boolean sign flag.
/// Corresponds to C++ <c>InteiroLovelace</c>.
/// </summary>
public class Integer :
    ISignedNumber<Integer>,
    INumber<Integer>,
    IComparable<Integer>,
    IEquatable<Integer>,
    IParsable<Integer>,
    ISpanParsable<Integer>,
    ISpanFormattable,
    IUnaryNegationOperators<Integer, Integer>,
    IUnaryPlusOperators<Integer, Integer>,
    IAdditionOperators<Integer, Integer, Integer>,
    ISubtractionOperators<Integer, Integer, Integer>,
    IMultiplyOperators<Integer, Integer, Integer>,
    IDivisionOperators<Integer, Integer, Integer>,
    IModulusOperators<Integer, Integer, Integer>,
    IIncrementOperators<Integer>,
    IDecrementOperators<Integer>,
    IComparisonOperators<Integer, Integer, bool>
{
    // -------------------------------------------------------------------------
    // Private fields
    // -------------------------------------------------------------------------

    private readonly Nat _magnitude;
    private readonly bool _isNegative;

    // -------------------------------------------------------------------------
    // INumberBase<Integer> — required static properties
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static Integer One => new(Nat.One, false);

    /// <inheritdoc/>
    public static int Radix => 10;

    /// <inheritdoc/>
    public static Integer Zero => new();

    /// <inheritdoc/>
    public static Integer AdditiveIdentity => Zero;

    /// <inheritdoc/>
    public static Integer MultiplicativeIdentity => One;

    /// <inheritdoc/>
    public static Integer NegativeOne => new(Nat.One, true);

    // -------------------------------------------------------------------------
    // Constructors
    // -------------------------------------------------------------------------

    /// <summary>Default constructor — magnitude=zero, sign=positive.</summary>
    public Integer()
    {
        _magnitude = new Nat();
        _isNegative = false;
    }

    /// <summary>
    /// Internal raw constructor. Normalises sign of zero (zero is always
    /// positive, mirrors C++ <c>zerar</c> which calls <c>setSinal(true)</c>).
    /// </summary>
    public Integer(Nat magnitude, bool isNegative)
    {
        _magnitude = magnitude;
        // Normalise: zero has no sign.
        _isNegative = isNegative && !Nat.IsZero(magnitude);
    }

    /// <summary>
    /// Wraps a <see cref="Nat"/> with positive sign.
    /// Maps from <c>InteiroLovelace(const Lovelace&amp;)</c> which sets <c>sinal=true</c>.
    /// </summary>
    public Integer(Nat magnitude)
    {
        _magnitude = new Nat(magnitude);
        _isNegative = false;
    }

    /// <summary>
    /// Constructs from a <see cref="long"/>; infers sign from sign of value.
    /// Maps from <c>atribuir(long long int)</c>.
    /// </summary>
    public Integer(long value)
    {
        if (value >= 0)
        {
            _magnitude = new Nat((ulong)value);
            _isNegative = false;
        }
        else
        {
            _isNegative = true;
            // long.MinValue = -9223372036854775808 has no positive long counterpart.
            _magnitude = value == long.MinValue
                ? Nat.Parse("9223372036854775808", null)
                : new Nat((ulong)(-value));
        }
    }

    /// <summary>
    /// Constructs from an <see cref="int"/>; delegates to <see cref="Integer(long)"/>.
    /// Maps from <c>atribuir(const int&amp;)</c>.
    /// </summary>
    public Integer(int value) : this((long)value) { }

    /// <summary>
    /// Constructs by parsing a decimal string, optionally prefixed with <c>-</c>.
    /// </summary>
    /// <exception cref="FormatException">Thrown when <paramref name="s"/> is invalid.</exception>
    public Integer(string s)
    {
        var parsed = Parse(s, null);
        _magnitude = parsed._magnitude;
        _isNegative = parsed._isNegative;
    }

    /// <summary>
    /// Constructs by parsing a span of decimal characters, optionally prefixed with <c>-</c>.
    /// </summary>
    /// <exception cref="FormatException">Thrown when <paramref name="s"/> is invalid.</exception>
    public Integer(ReadOnlySpan<char> s)
    {
        var parsed = Parse(s, null);
        _magnitude = parsed._magnitude;
        _isNegative = parsed._isNegative;
    }

    // -------------------------------------------------------------------------
    // Internal helper
    // -------------------------------------------------------------------------

    /// <summary>
    /// Returns the magnitude (absolute value) as a <see cref="Nat"/>.
    /// Maps from C++ <c>toLovelace(Lovelace&amp;)</c>.
    /// </summary>
    public Nat ToNatural() => _magnitude;

    // -------------------------------------------------------------------------
    // Static predicates (INumber<T> / INumberBase<T> requirements)
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static bool IsZero(Integer value) => Nat.IsZero(value._magnitude);

    /// <inheritdoc/>
    public static bool IsPositive(Integer value) => !value._isNegative;

    /// <inheritdoc/>
    public static bool IsNegative(Integer value) => value._isNegative;

    /// <inheritdoc/>
    public static bool IsEvenInteger(Integer value) => Nat.IsEvenInteger(value._magnitude);

    /// <inheritdoc/>
    public static bool IsOddInteger(Integer value) => Nat.IsOddInteger(value._magnitude);

    // Required INumberBase<T> classification stubs
    /// <inheritdoc/>
    public static bool IsCanonical(Integer value) => true;
    /// <inheritdoc/>
    public static bool IsComplexNumber(Integer value) => false;
    /// <inheritdoc/>
    public static bool IsFinite(Integer value) => true;
    /// <inheritdoc/>
    public static bool IsImaginaryNumber(Integer value) => false;
    /// <inheritdoc/>
    public static bool IsInfinity(Integer value) => false;
    /// <inheritdoc/>
    public static bool IsInteger(Integer value) => true;
    /// <inheritdoc/>
    public static bool IsNaN(Integer value) => false;
    /// <inheritdoc/>
    public static bool IsNegativeInfinity(Integer value) => false;
    /// <inheritdoc/>
    public static bool IsNormal(Integer value) => !IsZero(value);
    /// <inheritdoc/>
    public static bool IsPositiveInfinity(Integer value) => false;
    /// <inheritdoc/>
    public static bool IsRealNumber(Integer value) => true;
    /// <inheritdoc/>
    public static bool IsSubnormal(Integer value) => false;

    // -------------------------------------------------------------------------
    // Sign property
    // -------------------------------------------------------------------------

    /// <summary>Returns -1, 0, or +1 depending on the integer's sign.</summary>
    public int Sign => IsZero(this) ? 0 : (_isNegative ? -1 : 1);

    // -------------------------------------------------------------------------
    // Negation  (IUnaryNegationOperators<T,T> / ISignedNumber<T>)
    // -------------------------------------------------------------------------

    /// <summary>
    /// Returns a copy with the sign flipped; zero remains positive.
    /// Maps from C++ <c>inverterSinal()</c>.
    /// </summary>
    public Integer Negate()
    {
        if (IsZero(this)) return new Integer();
        return new Integer(_magnitude, !_isNegative);
    }

    /// <inheritdoc/>
    public static Integer operator -(Integer value) => value.Negate();

    /// <inheritdoc/>
    public static Integer operator +(Integer value) => new(new Nat(value._magnitude), value._isNegative);

    // -------------------------------------------------------------------------
    // Addition  (IAdditionOperators<T,T,T>)
    // -------------------------------------------------------------------------

    /// <summary>
    /// Adds two integers. Same-sign: add magnitudes, keep sign.
    /// Different-sign: subtract magnitudes, sign follows the larger magnitude.
    /// Maps from C++ <c>somar</c>.
    /// </summary>
    public Integer Add(Integer other)
    {
        if (_isNegative == other._isNegative)
        {
            var mag = _magnitude + other._magnitude;
            return new Integer(mag, _isNegative);
        }

        // Different signs: subtract smaller from larger magnitude.
        int cmp = _magnitude.CompareTo(other._magnitude);
        if (cmp == 0) return new Integer(); // opposite signs, equal magnitudes → zero

        bool resultNegative = (cmp > 0) ? _isNegative : other._isNegative;
        var result = cmp > 0
            ? _magnitude - other._magnitude
            : other._magnitude - _magnitude;
        return new Integer(result, resultNegative);
    }

    /// <inheritdoc/>
    public static Integer operator +(Integer left, Integer right) => left.Add(right);

    // -------------------------------------------------------------------------
    // Subtraction  (ISubtractionOperators<T,T,T>)
    // -------------------------------------------------------------------------

    /// <summary>
    /// Subtracts <paramref name="other"/> from this integer.
    /// Implemented as <c>Add(other.Negate())</c>.
    /// Maps from C++ <c>subtrair</c>.
    /// </summary>
    public Integer Subtract(Integer other) => Add(other.Negate());

    /// <inheritdoc/>
    public static Integer operator -(Integer left, Integer right) => left.Subtract(right);

    // -------------------------------------------------------------------------
    // Multiplication  (IMultiplyOperators<T,T,T>)
    // -------------------------------------------------------------------------

    /// <summary>
    /// Multiplies two integers. Result is negative iff exactly one operand is
    /// negative (XOR of signs). Maps from C++ <c>multiplicar</c>.
    /// </summary>
    public Integer Multiply(Integer other)
    {
        var mag = _magnitude * other._magnitude;
        bool resultNeg = _isNegative != other._isNegative && !Nat.IsZero(mag);
        return new Integer(mag, resultNeg);
    }

    /// <inheritdoc/>
    public static Integer operator *(Integer left, Integer right) => left.Multiply(right);

    // -------------------------------------------------------------------------
    // Division / Remainder  (IDivisionOperators, IModulusOperators)
    // -------------------------------------------------------------------------

    /// <summary>
    /// Divides this integer by <paramref name="divisor"/> and returns the quotient;
    /// <paramref name="remainder"/> receives the remainder.
    /// Quotient and remainder signs follow the equal-signs rule.
    /// Maps from C++ <c>dividir(B, quociente, resto)</c>.
    /// </summary>
    public Integer DivRem(Integer divisor, out Integer remainder)
    {
        var quotMag = Nat.DivRem(_magnitude, divisor._magnitude, out var remMag);
        bool sameSign = _isNegative == divisor._isNegative;
        remainder = new Integer(remMag, !sameSign && !Nat.IsZero(remMag));
        return new Integer(quotMag, !sameSign && !Nat.IsZero(quotMag));
    }

    /// <inheritdoc/>
    public static Integer operator /(Integer left, Integer right) => left.DivRem(right, out _);

    /// <inheritdoc/>
    public static Integer operator %(Integer left, Integer right)
    {
        left.DivRem(right, out var rem);
        return rem;
    }

    // -------------------------------------------------------------------------
    // Exponentiation
    // -------------------------------------------------------------------------

    /// <summary>
    /// Raises this integer to <paramref name="exponent"/>.
    /// Guards: base ≠ 0, exponent &gt; 0.
    /// Result is negative iff base is negative AND exponent is odd.
    /// Maps from C++ <c>exponenciar(X)</c>.
    /// </summary>
    public Integer Pow(Integer exponent)
    {
        if (IsZero(this))
            throw new ArgumentOutOfRangeException(nameof(exponent), "Base cannot be zero.");
        if (exponent <= Zero)
            throw new ArgumentOutOfRangeException(nameof(exponent), "Exponent must be positive.");

        var mag = _magnitude.Pow(exponent._magnitude);
        bool resultNeg = _isNegative && Nat.IsOddInteger(exponent._magnitude);
        return new Integer(mag, resultNeg);
    }

    // -------------------------------------------------------------------------
    // Factorial
    // -------------------------------------------------------------------------

    /// <summary>
    /// Returns n! for this integer.
    /// Throws <see cref="InvalidOperationException"/> for negative values.
    /// Note: C++ source has an inverted sign check (a bug) — this implementation is correct.
    /// Maps from C++ <c>fatorial()</c>.
    /// </summary>
    public Integer Factorial()
    {
        if (_isNegative)
            throw new InvalidOperationException("Factorial is not defined for negative integers.");
        return new Integer(_magnitude.Factorial());
    }

    // -------------------------------------------------------------------------
    // Increment / Decrement  (IIncrementOperators<T>, IDecrementOperators<T>)
    // -------------------------------------------------------------------------

    /// <summary>Adds one. Maps from C++ <c>incrementar()</c>.</summary>
    public Integer Increment() => Add(new Integer(1L));

    /// <summary>Subtracts one. Maps from C++ <c>decrementar()</c>.</summary>
    public Integer Decrement() => Subtract(new Integer(1L));

    /// <inheritdoc/>
    public static Integer operator ++(Integer value) => value.Increment();

    /// <inheritdoc/>
    public static Integer operator --(Integer value) => value.Decrement();

    // -------------------------------------------------------------------------
    // Equality / Comparison  (IEquatable<T>, IComparable<T>)
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public bool Equals(Integer? other)
    {
        if (other is null) return false;
        // Both zero → equal regardless of sign.
        if (IsZero(this) && IsZero(other)) return true;
        if (_isNegative != other._isNegative) return false;
        return _magnitude.Equals(other._magnitude);
    }

    /// <inheritdoc/>
    public int CompareTo(Integer? other)
    {
        if (other is null) return 1;

        // Cross-sign: positive > negative.
        if (!_isNegative && other._isNegative) return 1;
        if (_isNegative && !other._isNegative) return -1;

        // Same sign: delegate to magnitude comparison.
        int cmp = _magnitude.CompareTo(other._magnitude);
        // Both negative: larger magnitude = smaller number → flip.
        return _isNegative ? -cmp : cmp;
    }

    /// <summary>Non-generic <see cref="IComparable"/> implementation.</summary>
    public int CompareTo(object? obj)
    {
        if (obj is null) return 1;
        if (obj is Integer other) return CompareTo(other);
        throw new ArgumentException($"Object must be of type {nameof(Integer)}.", nameof(obj));
    }

    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is Integer n && Equals(n);

    /// <inheritdoc/>
    public override int GetHashCode() => ToString().GetHashCode();

    /// <inheritdoc/>
    public static bool operator ==(Integer left, Integer right) => left.Equals(right);
    /// <inheritdoc/>
    public static bool operator !=(Integer left, Integer right) => !left.Equals(right);
    /// <inheritdoc/>
    public static bool operator >(Integer left, Integer right) => left.CompareTo(right) > 0;
    /// <inheritdoc/>
    public static bool operator >=(Integer left, Integer right) => left.CompareTo(right) >= 0;
    /// <inheritdoc/>
    public static bool operator <(Integer left, Integer right) => left.CompareTo(right) < 0;
    /// <inheritdoc/>
    public static bool operator <=(Integer left, Integer right) => left.CompareTo(right) <= 0;

    // -------------------------------------------------------------------------
    // Formatting  (ISpanFormattable)
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public override string ToString()
    {
        if (IsZero(this)) return "0";
        return _isNegative ? "-" + _magnitude.ToString() : _magnitude.ToString();
    }

    /// <inheritdoc/>
    public string ToString(string? format, IFormatProvider? formatProvider) => ToString();

    /// <inheritdoc/>
    public bool TryFormat(
        Span<char> destination,
        out int charsWritten,
        ReadOnlySpan<char> format,
        IFormatProvider? provider)
    {
        var s = ToString();
        if (s.Length > destination.Length)
        {
            charsWritten = 0;
            return false;
        }
        s.AsSpan().CopyTo(destination);
        charsWritten = s.Length;
        return true;
    }

    // -------------------------------------------------------------------------
    // Parsing  (IParsable<T>, ISpanParsable<T>)
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static Integer Parse(string s, IFormatProvider? provider)
    {
        ArgumentNullException.ThrowIfNull(s);
        return Parse(s.AsSpan(), provider);
    }

    /// <inheritdoc/>
    public static Integer Parse(ReadOnlySpan<char> s, IFormatProvider? provider)
    {
        s = s.Trim();
        if (s.IsEmpty)
            throw new FormatException("Input string was not in a correct format.");

        bool isNeg = false;
        if (s[0] == '-')
        {
            isNeg = true;
            s = s[1..];
        }

        if (s.IsEmpty)
            throw new FormatException("Input string was not in a correct format.");

        foreach (char c in s)
        {
            if (c < '0' || c > '9')
                throw new FormatException(
                    $"Input string was not in a correct format. Invalid character: '{c}'.");
        }

        var mag = Nat.Parse(s, provider);
        // Normalise: -0 → positive zero.
        bool actualNeg = isNeg && !Nat.IsZero(mag);
        return new Integer(mag, actualNeg);
    }

    /// <inheritdoc/>
    public static bool TryParse(
        [NotNullWhen(true)] string? s,
        IFormatProvider? provider,
        [MaybeNullWhen(false)] out Integer result)
    {
        if (s is null) { result = Zero; return false; }
        return TryParse(s.AsSpan(), provider, out result);
    }

    /// <inheritdoc/>
    public static bool TryParse(
        ReadOnlySpan<char> s,
        IFormatProvider? provider,
        [MaybeNullWhen(false)] out Integer result)
    {
        try
        {
            result = Parse(s, provider);
            return true;
        }
        catch
        {
            result = Zero;
            return false;
        }
    }

    // NumberStyles overloads required by INumberBase<T>
    /// <inheritdoc/>
    public static Integer Parse(string s, NumberStyles style, IFormatProvider? provider)
        => Parse(s, provider);

    /// <inheritdoc/>
    public static Integer Parse(ReadOnlySpan<char> s, NumberStyles style, IFormatProvider? provider)
        => Parse(s, provider);

    /// <inheritdoc/>
    public static bool TryParse([NotNullWhen(true)] string? s, NumberStyles style, IFormatProvider? provider,
        [MaybeNullWhen(false)] out Integer result)
        => TryParse(s, provider, out result);

    /// <inheritdoc/>
    public static bool TryParse(ReadOnlySpan<char> s, NumberStyles style, IFormatProvider? provider,
        [MaybeNullWhen(false)] out Integer result)
        => TryParse(s, provider, out result);

    // -------------------------------------------------------------------------
    // INumberBase<Integer> — magnitude / conversion helpers
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static Integer Abs(Integer value) => new(value._magnitude, false);

    /// <inheritdoc/>
    public static Integer MaxMagnitude(Integer x, Integer y)
        => Abs(x).CompareTo(Abs(y)) >= 0 ? x : y;

    /// <inheritdoc/>
    public static Integer MaxMagnitudeNumber(Integer x, Integer y) => MaxMagnitude(x, y);

    /// <inheritdoc/>
    public static Integer MinMagnitude(Integer x, Integer y)
        => Abs(x).CompareTo(Abs(y)) <= 0 ? x : y;

    /// <inheritdoc/>
    public static Integer MinMagnitudeNumber(Integer x, Integer y) => MinMagnitude(x, y);

    /// <inheritdoc/>
    public static bool TryConvertFromChecked<TOther>(TOther value, [MaybeNullWhen(false)] out Integer result)
        where TOther : INumberBase<TOther>
    { result = Zero; return false; }

    /// <inheritdoc/>
    public static bool TryConvertFromSaturating<TOther>(TOther value, [MaybeNullWhen(false)] out Integer result)
        where TOther : INumberBase<TOther>
    { result = Zero; return false; }

    /// <inheritdoc/>
    public static bool TryConvertFromTruncating<TOther>(TOther value, [MaybeNullWhen(false)] out Integer result)
        where TOther : INumberBase<TOther>
    { result = Zero; return false; }

    /// <inheritdoc/>
    public static bool TryConvertToChecked<TOther>(Integer value, [MaybeNullWhen(false)] out TOther result)
        where TOther : INumberBase<TOther>
    { result = default; return false; }

    /// <inheritdoc/>
    public static bool TryConvertToSaturating<TOther>(Integer value, [MaybeNullWhen(false)] out TOther result)
        where TOther : INumberBase<TOther>
    { result = default; return false; }

    /// <inheritdoc/>
    public static bool TryConvertToTruncating<TOther>(Integer value, [MaybeNullWhen(false)] out TOther result)
        where TOther : INumberBase<TOther>
    { result = default; return false; }
}
