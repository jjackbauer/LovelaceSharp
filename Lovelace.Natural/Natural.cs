using System.Diagnostics.CodeAnalysis;
using System.Globalization;
using System.Numerics;
using System.Threading;
using System.Threading.Tasks;

namespace Lovelace.Natural;

/// <summary>
/// Arbitrary-precision natural number (ℕ₀, i.e. non-negative integers).
/// Backed by little-endian 64-bit binary limbs (base 2⁶⁴) rather than BCD, so every
/// arithmetic word processes ~19.3 decimal digits instead of one decimal digit.
/// </summary>
public sealed class Natural :
    INumber<Natural>,
    IComparable<Natural>,
    IEquatable<Natural>,
    IParsable<Natural>,
    ISpanParsable<Natural>,
    ISpanFormattable,
    IAdditionOperators<Natural, Natural, Natural>,
    ISubtractionOperators<Natural, Natural, Natural>,
    IMultiplyOperators<Natural, Natural, Natural>,
    IDivisionOperators<Natural, Natural, Natural>,
    IModulusOperators<Natural, Natural, Natural>,
    IIncrementOperators<Natural>,
    IDecrementOperators<Natural>,
    IComparisonOperators<Natural, Natural, bool>
{
    // -------------------------------------------------------------------------
    // Backing store — little-endian 64-bit limbs (base 2^64).
    // Canonical form: no most-significant zero limbs; zero is the empty array.
    // Instances are immutable (operators return new instances).
    // -------------------------------------------------------------------------

    private readonly ulong[] _limbs;

    private static readonly ulong[] s_empty = Array.Empty<ulong>();
    private static readonly Natural s_zero = new();
    private static readonly Natural s_one = new(1UL);

    /// <summary>Private ownership constructor. Assumes <paramref name="limbs"/> is canonical.</summary>
    private Natural(ulong[] limbs) => _limbs = limbs;

    // -------------------------------------------------------------------------
    // Static configuration properties (C++ algarismosExibicao / Precisao)
    // -------------------------------------------------------------------------

    private static long _displayDigits = -1L;
    private static long _precision = -1L;

    /// <summary>Maximum number of digits to display when formatting; -1 means no limit.</summary>
    public static long DisplayDigits
    {
        get => Interlocked.Read(ref _displayDigits);
        set => Interlocked.Exchange(ref _displayDigits, value);
    }

    /// <summary>Precision hint. Stub — C++ body was absent.</summary>
    public static long Precision
    {
        get => Interlocked.Read(ref _precision);
        set => Interlocked.Exchange(ref _precision, value);
    }

    // -------------------------------------------------------------------------
    // INumberBase<Natural> — required static properties
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static Natural One => s_one;

    /// <inheritdoc/>
    public static int Radix => 10;

    /// <inheritdoc/>
    public static Natural Zero => s_zero;

    /// <inheritdoc/>
    public static Natural AdditiveIdentity => Zero;

    /// <inheritdoc/>
    public static Natural MultiplicativeIdentity => One;

    // -------------------------------------------------------------------------
    // Constructors
    // -------------------------------------------------------------------------

    /// <summary>Default constructor — produces zero.</summary>
    public Natural() => _limbs = s_empty;

    /// <summary>Copy constructor — deep copy of <paramref name="other"/>.</summary>
    public Natural(Natural other)
    {
        var src = other._limbs;
        _limbs = src.Length == 0 ? s_empty : (ulong[])src.Clone();
    }

    /// <summary>Constructs a <see cref="Natural"/> from an unsigned 64-bit integer.</summary>
    public Natural(ulong value)
    {
        _limbs = value == 0 ? s_empty : new[] { value };
    }

    /// <summary>Constructs a <see cref="Natural"/> from a non-negative <see cref="int"/>.</summary>
    /// <exception cref="ArgumentOutOfRangeException">Thrown when <paramref name="value"/> is negative.</exception>
    public Natural(int value)
    {
        if (value < 0)
            throw new ArgumentOutOfRangeException(nameof(value), "Value must be non-negative.");
        _limbs = value == 0 ? s_empty : new[] { (ulong)value };
    }

    /// <summary>Constructs a <see cref="Natural"/> by parsing a decimal digit string.</summary>
    public Natural(string s) => _limbs = Parse(s, null)._limbs;

    /// <summary>Constructs a <see cref="Natural"/> by parsing a span of decimal digit characters.</summary>
    public Natural(ReadOnlySpan<char> s) => _limbs = Parse(s, null)._limbs;

    // -------------------------------------------------------------------------
    // INumberBase<Natural> — classification predicates
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static bool IsZero(Natural value) => value._limbs.Length == 0;

    /// <inheritdoc/>
    public static bool IsEvenInteger(Natural value) => !IsOddInteger(value);

    /// <inheritdoc/>
    public static bool IsOddInteger(Natural value)
        => value._limbs.Length != 0 && (value._limbs[0] & 1UL) != 0;

    /// <inheritdoc/>
    public static bool IsCanonical(Natural value) => true;

    /// <inheritdoc/>
    public static bool IsComplexNumber(Natural value) => false;

    /// <inheritdoc/>
    public static bool IsFinite(Natural value) => true;

    /// <inheritdoc/>
    public static bool IsImaginaryNumber(Natural value) => false;

    /// <inheritdoc/>
    public static bool IsInfinity(Natural value) => false;

    /// <inheritdoc/>
    public static bool IsInteger(Natural value) => true;

    /// <inheritdoc/>
    public static bool IsNaN(Natural value) => false;

    /// <inheritdoc/>
    public static bool IsNegative(Natural value) => false;

    /// <inheritdoc/>
    public static bool IsNegativeInfinity(Natural value) => false;

    /// <inheritdoc/>
    public static bool IsNormal(Natural value) => !IsZero(value);

    /// <inheritdoc/>
    public static bool IsPositive(Natural value) => true;

    /// <inheritdoc/>
    public static bool IsPositiveInfinity(Natural value) => false;

    /// <inheritdoc/>
    public static bool IsRealNumber(Natural value) => true;

    /// <inheritdoc/>
    public static bool IsSubnormal(Natural value) => false;

    // -------------------------------------------------------------------------
    // INumberBase<Natural> — magnitude helpers
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static Natural Abs(Natural value) => new(value);

    /// <inheritdoc/>
    public static Natural MaxMagnitude(Natural x, Natural y) => x >= y ? x : y;

    /// <inheritdoc/>
    public static Natural MaxMagnitudeNumber(Natural x, Natural y) => MaxMagnitude(x, y);

    /// <inheritdoc/>
    public static Natural MinMagnitude(Natural x, Natural y) => x <= y ? x : y;

    /// <inheritdoc/>
    public static Natural MinMagnitudeNumber(Natural x, Natural y) => MinMagnitude(x, y);

    // -------------------------------------------------------------------------
    // IEquatable<Natural> / IComparable<Natural>
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public bool Equals(Natural? other)
    {
        if (other is null) return false;
        var a = _limbs;
        var b = other._limbs;
        if (a.Length != b.Length) return false;
        for (int i = 0; i < a.Length; i++)
            if (a[i] != b[i]) return false;
        return true;
    }

    /// <inheritdoc/>
    public int CompareTo(Natural? other)
    {
        if (other is null) return 1;
        return CompareLimbs(_limbs, other._limbs);
    }

    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is Natural n && Equals(n);

    /// <inheritdoc/>
    public override int GetHashCode() => ToString().GetHashCode();

    /// <summary>Non-generic <see cref="IComparable.CompareTo(object?)"/> required by <see cref="INumber{T}"/>.</summary>
    public int CompareTo(object? obj)
    {
        if (obj is null) return 1;
        if (obj is Natural other) return CompareTo(other);
        throw new ArgumentException($"Object must be of type {nameof(Natural)}.", nameof(obj));
    }

    // -------------------------------------------------------------------------
    // Unary operators (required by INumber<T>)
    // -------------------------------------------------------------------------

    /// <summary>Unary plus — returns a copy.</summary>
    public static Natural operator +(Natural value) => new(value);

    /// <summary>Unary negation — not representable in ℕ₀.</summary>
    /// <exception cref="InvalidOperationException">Always thrown.</exception>
    public static Natural operator -(Natural value)
        => throw new InvalidOperationException("Cannot negate a Natural number; the result would be negative.");

    // -------------------------------------------------------------------------
    // Arithmetic operators
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static Natural operator +(Natural left, Natural right)
    {
        var a = left._limbs;
        var b = right._limbs;
        if (a.Length == 0) return new Natural(right);
        if (b.Length == 0) return new Natural(left);

        int n = Math.Max(a.Length, b.Length);
        var r = new ulong[n + 1];
        ulong carry = 0;
        for (int i = 0; i < n; i++)
        {
            ulong av = i < a.Length ? a[i] : 0UL;
            ulong bv = i < b.Length ? b[i] : 0UL;
            ulong s = av + bv;
            ulong c0 = s < av ? 1UL : 0UL;
            ulong s2 = s + carry;
            ulong c1 = s2 < s ? 1UL : 0UL;
            r[i] = s2;
            carry = c0 + c1;
        }
        if (carry != 0)
        {
            r[n] = carry;
            return new Natural(r);
        }
        return Make(r, n);
    }

    /// <inheritdoc/>
    /// <exception cref="InvalidOperationException">Thrown when <paramref name="right"/> &gt; <paramref name="left"/>.</exception>
    public static Natural operator -(Natural left, Natural right)
    {
        var a = left._limbs;
        var b = right._limbs;
        if (right > left)
            throw new InvalidOperationException(
                "Subtraction would produce a negative result, which cannot be represented as a Natural.");
        if (b.Length == 0) return new Natural(left);
        if (left == right) return s_zero;

        var r = new ulong[a.Length];
        ulong borrow = 0;
        for (int i = 0; i < a.Length; i++)
        {
            ulong bv = i < b.Length ? b[i] : 0UL;
            ulong s1 = a[i] - bv;
            ulong b1 = s1 > a[i] ? 1UL : 0UL;
            ulong s2 = s1 - borrow;
            ulong b2 = s2 > s1 ? 1UL : 0UL;
            r[i] = s2;
            borrow = b1 + b2;
        }
        return Make(r, a.Length);
    }

    /// <inheritdoc/>
    public static Natural operator *(Natural left, Natural right)
    {
        var a = left._limbs;
        var b = right._limbs;
        if (a.Length == 0 || b.Length == 0) return s_zero;
        var product = Multiply(a, b);
        return Make(product, product.Length);
    }

    /// <inheritdoc/>
    public static Natural operator /(Natural left, Natural right) => DivRem(left, right, out _);

    /// <inheritdoc/>
    public static Natural operator %(Natural left, Natural right)
    {
        DivRem(left, right, out var remainder);
        return remainder;
    }

    // -------------------------------------------------------------------------
    // Increment / Decrement operators
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static Natural operator ++(Natural value) => value + One;

    /// <inheritdoc/>
    public static Natural operator --(Natural value) => value - One;

    // -------------------------------------------------------------------------
    // Comparison operators
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static bool operator ==(Natural? left, Natural? right)
    {
        if (left is null && right is null) return true;
        if (left is null || right is null) return false;
        return left.Equals(right);
    }

    /// <inheritdoc/>
    public static bool operator !=(Natural? left, Natural? right) => !(left == right);

    /// <inheritdoc/>
    public static bool operator >(Natural left, Natural right) => left.CompareTo(right) > 0;

    /// <inheritdoc/>
    public static bool operator >=(Natural left, Natural right) => left.CompareTo(right) >= 0;

    /// <inheritdoc/>
    public static bool operator <(Natural left, Natural right) => left.CompareTo(right) < 0;

    /// <inheritdoc/>
    public static bool operator <=(Natural left, Natural right) => left.CompareTo(right) <= 0;

    // -------------------------------------------------------------------------
    // Domain-specific operations (not part of INumber<T>)
    // -------------------------------------------------------------------------

    /// <summary>
    /// Divides <paramref name="left"/> by <paramref name="right"/>, returning the quotient and
    /// setting <paramref name="remainder"/> to the remainder.
    /// </summary>
    /// <exception cref="DivideByZeroException">Thrown when <paramref name="right"/> is zero.</exception>
    public static Natural DivRem(Natural left, Natural right, out Natural remainder)
    {
        var u = left._limbs;
        var v = right._limbs;
        if (v.Length == 0)
            throw new DivideByZeroException("Cannot divide by zero.");
        if (u.Length == 0)
        {
            remainder = s_zero;
            return s_zero;
        }

        int cmp = CompareLimbs(u, v);
        if (cmp < 0)
        {
            remainder = new Natural(left);
            return s_zero;
        }
        if (cmp == 0)
        {
            remainder = s_zero;
            return s_one;
        }

        if (v.Length == 1)
        {
            // Short division by a single 64-bit limb.
            var q = new ulong[u.Length];
            ulong rem = 0;
            for (int i = u.Length - 1; i >= 0; i--)
            {
                UInt128 cur = ((UInt128)rem << 64) | u[i];
                q[i] = (ulong)(cur / (UInt128)v[0]);
                rem = (ulong)(cur % (UInt128)v[0]);
            }
            remainder = rem == 0 ? s_zero : new Natural(rem);
            return Make(q, u.Length);
        }

        (Natural quotient, Natural remKnuth) = DivRemKnuth(u, v);
        remainder = remKnuth;
        return quotient;
    }

    /// <summary>
    /// Returns this value × 10^k (appends <paramref name="k"/> zero decimal digits).
    /// </summary>
    public Natural ShiftLeftDecimal(long k)
    {
        if (IsZero(this) || k <= 0)
            return new Natural(this);
        return this * Pow10(k);
    }

    /// <summary>Convenience instance overload: divides this by <paramref name="divisor"/>.</summary>
    public Natural DivRem(Natural divisor, out Natural remainder) => DivRem(this, divisor, out remainder);

    /// <summary>
    /// Raises this instance to the power of <paramref name="exponent"/> using binary
    /// (repeated-squaring) exponentiation.
    /// </summary>
    public Natural Pow(Natural exponent)
    {
        if (IsZero(exponent))
            return s_one;

        var result = s_one;
        var b = new Natural(this);
        var e = new Natural(exponent);

        while (!IsZero(e))
        {
            if (IsOddInteger(e))
                result *= b;
            b *= b;
            e = DivRem(e, new Natural(2UL), out _);
        }

        return result;
    }

    /// <summary>
    /// Returns the factorial of this instance (this!). Large values partition the factor range
    /// [2..n] across <see cref="Environment.ProcessorCount"/> sub-ranges multiplied concurrently.
    /// </summary>
    public Natural Factorial()
    {
        if (IsZero(this)) return s_one;

        int processorCount = Environment.ProcessorCount;
        if (!ulong.TryParse(ToString(), out ulong n) || n <= (ulong)(processorCount * 2))
        {
            var seqResult = s_one;
            for (var aux = new Natural(2UL); aux <= this; aux++)
                seqResult *= aux;
            return seqResult;
        }

        int t = processorCount;
        var partials = new Natural[t];
        for (int i = 0; i < t; i++) partials[i] = s_one;

        ulong totalFactors = n - 1UL;
        ulong rangeSize = (totalFactors + (ulong)t - 1UL) / (ulong)t;

        Parallel.For(0, t, i =>
        {
            ulong start = 2UL + (ulong)i * rangeSize;
            ulong end = start + rangeSize - 1UL;
            if (end > n) end = n;
            if (start > n) return;

            var sub = s_one;
            for (ulong k = start; k <= end; k++)
                sub *= new Natural(k);
            partials[i] = sub;
        });

        var result = s_one;
        foreach (var p in partials)
            result *= p;
        return result;
    }

    // -------------------------------------------------------------------------
    // ISpanFormattable / IFormattable / ToString
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public override string ToString()
    {
        if (_limbs.Length == 0) return "0";
        return ToStringRecursive(_limbs);
    }

    /// <summary>Divide-and-conquer binary→decimal conversion: split the value at 10^half and recurse.</summary>
    private static string ToStringRecursive(ulong[] limbs)
    {
        if (limbs.Length == 0) return string.Empty;
        if (limbs.Length == 1) return limbs[0].ToString(CultureInfo.InvariantCulture);

        // Exact bit length, then a safe upper bound on the decimal digit count.
        long bits = 64L * (limbs.Length - 1) + (64 - BitOperations.LeadingZeroCount(limbs[^1]));
        int digits = (int)((bits * 30103L) / 100000L) + 2;
        int half = digits / 2;

        Natural q = DivRem(new Natural(limbs), Pow10(half), out Natural r);
        string hi = ToStringRecursive(q._limbs);
        string lo = ToStringRecursive(r._limbs);
        return hi + lo.PadLeft(half, '0');
    }

    /// <inheritdoc/>
    public string ToString(string? format, IFormatProvider? formatProvider)
    {
        if (string.Equals(format, "N", StringComparison.OrdinalIgnoreCase))
            return GroupThousands(ToString());
        return ToString();
    }

    /// <inheritdoc/>
    public bool TryFormat(Span<char> destination, out int charsWritten, ReadOnlySpan<char> format, IFormatProvider? provider)
    {
        string s = format.IsEmpty ? ToString() : ToString(format.ToString(), provider);
        if (s.Length <= destination.Length)
        {
            s.AsSpan().CopyTo(destination);
            charsWritten = s.Length;
            return true;
        }
        charsWritten = 0;
        return false;
    }

    // -------------------------------------------------------------------------
    // IParsable<Natural> / ISpanParsable<Natural>
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static Natural Parse(string s, IFormatProvider? provider)
    {
        if (!TryParse(s.AsSpan(), provider, out var result))
            throw new FormatException($"The string '{s}' is not a valid decimal representation of a Natural number.");
        return result;
    }

    /// <inheritdoc/>
    public static bool TryParse([NotNullWhen(true)] string? s, IFormatProvider? provider, [MaybeNullWhen(false)] out Natural result)
        => TryParse(s.AsSpan(), provider, out result);

    /// <inheritdoc/>
    public static Natural Parse(ReadOnlySpan<char> s, IFormatProvider? provider)
    {
        if (!TryParse(s, provider, out var result))
            throw new FormatException("The input is not a valid decimal representation of a Natural number.");
        return result;
    }

    /// <inheritdoc/>
    public static bool TryParse(ReadOnlySpan<char> s, IFormatProvider? provider, [MaybeNullWhen(false)] out Natural result)
    {
        result = null;

        if (s.IsEmpty) return false;

        foreach (char ch in s)
        {
            if (ch < '0' || ch > '9') return false;
        }

        int start = 0;
        while (start < s.Length - 1 && s[start] == '0')
            start++;

        ReadOnlySpan<char> digits = s[start..];

        if (digits.Length == 1 && digits[0] == '0')
        {
            result = s_zero;
            return true;
        }

        result = ParseDigits(digits);
        return true;
    }

    /// <summary>Parses a decimal string into a <see cref="Natural"/>.</summary>
    public static Natural Parse(string s) => Parse(s, null);

    /// <summary>Attempts to parse a decimal string into a <see cref="Natural"/>.</summary>
    public static bool TryParse([NotNullWhen(true)] string? s, [MaybeNullWhen(false)] out Natural result)
        => TryParse(s, null, out result);

    // -------------------------------------------------------------------------
    // INumberBase<Natural> — numeric style parse overloads
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static Natural Parse(string s, NumberStyles style, IFormatProvider? provider) => Parse(s, provider);

    /// <inheritdoc/>
    public static Natural Parse(ReadOnlySpan<char> s, NumberStyles style, IFormatProvider? provider) => Parse(s, provider);

    /// <inheritdoc/>
    public static bool TryParse([NotNullWhen(true)] string? s, NumberStyles style, IFormatProvider? provider, [MaybeNullWhen(false)] out Natural result)
        => TryParse(s, provider, out result);

    /// <inheritdoc/>
    public static bool TryParse(ReadOnlySpan<char> s, NumberStyles style, IFormatProvider? provider, [MaybeNullWhen(false)] out Natural result)
        => TryParse(s, provider, out result);

    // -------------------------------------------------------------------------
    // INumberBase<Natural> — generic conversion helpers (stubs, unchanged)
    // -------------------------------------------------------------------------

    static bool INumberBase<Natural>.TryConvertFromChecked<TOther>(TOther value, [MaybeNullWhen(false)] out Natural result)
        => throw new NotImplementedException();

    static bool INumberBase<Natural>.TryConvertFromSaturating<TOther>(TOther value, [MaybeNullWhen(false)] out Natural result)
        => throw new NotImplementedException();

    static bool INumberBase<Natural>.TryConvertFromTruncating<TOther>(TOther value, [MaybeNullWhen(false)] out Natural result)
        => throw new NotImplementedException();

    static bool INumberBase<Natural>.TryConvertToChecked<TOther>(Natural value, [MaybeNullWhen(false)] out TOther result)
        => throw new NotImplementedException();

    static bool INumberBase<Natural>.TryConvertToSaturating<TOther>(Natural value, [MaybeNullWhen(false)] out TOther result)
        => throw new NotImplementedException();

    static bool INumberBase<Natural>.TryConvertToTruncating<TOther>(Natural value, [MaybeNullWhen(false)] out TOther result)
        => throw new NotImplementedException();

    // -------------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------------

    /// <summary>Builds a canonical Natural from a buffer of <paramref name="len"/> significant LSD-first limbs.</summary>
    private static Natural Make(ulong[] buf, int len)
    {
        while (len > 0 && buf[len - 1] == 0) len--;
        if (len == 0) return s_zero;
        if (len == buf.Length) return new Natural(buf);
        var copy = new ulong[len];
        Array.Copy(buf, copy, len);
        return new Natural(copy);
    }

    private static int CompareLimbs(ulong[] a, ulong[] b)
    {
        if (a.Length != b.Length) return a.Length.CompareTo(b.Length);
        for (int i = a.Length - 1; i >= 0; i--)
        {
            if (a[i] != b[i]) return a[i] < b[i] ? -1 : 1;
        }
        return 0;
    }

    /// <summary>Schoolbook multiply using native 64×64→128 products (<see cref="UInt128"/>).</summary>
    private static ulong[] SchoolbookMultiply(ulong[] a, ulong[] b)
    {
        var r = new ulong[a.Length + b.Length];
        for (int i = 0; i < a.Length; i++)
        {
            ulong ai = a[i];
            if (ai == 0) continue;

            UInt128 carry = 0;
            int k = i;
            for (int j = 0; j < b.Length; j++, k++)
            {
                UInt128 t = (UInt128)ai * b[j] + r[k] + carry;
                r[k] = (ulong)t;
                carry = t >> 64;
            }
            while (carry != 0)
            {
                UInt128 t = r[k] + carry;
                r[k] = (ulong)t;
                carry = t >> 64;
                k++;
            }
        }
        return r;
    }

    /// <summary>Operands with at most this many limbs use schoolbook; larger recurse via Karatsuba.</summary>
    private const int KaratsubaThreshold = 40;

    /// <summary>Multiply dispatcher: schoolbook below the threshold, Karatsuba above.</summary>
    private static ulong[] Multiply(ulong[] a, ulong[] b)
    {
        if (a.Length == 0 || b.Length == 0) return Array.Empty<ulong>();
        if (a.Length <= KaratsubaThreshold || b.Length <= KaratsubaThreshold)
            return SchoolbookMultiply(a, b);

        int maxLen = Math.Max(a.Length, b.Length);
        int minLen = Math.Min(a.Length, b.Length);

        // NTT (exact convolution) for very large operands, within the transform-size bound.
        long total = (long)a.Length + b.Length;
        if (total >= NttThreshold && total * NttPieces <= MaxNttLength)
            return NttMultiply(a, b);

        // Karatsuba degenerates when the operands are very unbalanced (one operand's high
        // half is empty); schoolbook is O(a·b) and beats it there.
        if (maxLen >= 2 * minLen)
            return SchoolbookMultiply(a, b);

        int m = (maxLen + 1) / 2;

        (ulong[] a0, ulong[] a1) = Split(a, m);
        (ulong[] b0, ulong[] b1) = Split(b, m);

        ulong[] z0 = Multiply(a0, b0);
        ulong[] z2 = Multiply(a1, b1);
        ulong[] sumA = AddRaw(a0, a1);
        ulong[] sumB = AddRaw(b0, b1);
        ulong[] z1 = Multiply(sumA, sumB);
        z1 = SubRaw(z1, z0);
        z1 = SubRaw(z1, z2);

        var r = new ulong[a.Length + b.Length];
        AddShifted(r, z0, 0);
        AddShifted(r, z1, m);
        AddShifted(r, z2, 2 * m);
        return r;
    }

    /// <summary>Splits <paramref name="a"/> into (low m limbs, remaining high limbs), both canonical.</summary>
    private static (ulong[] lo, ulong[] hi) Split(ulong[] a, int m)
    {
        if (a.Length <= m)
            return (TrimCopy(a, a.Length), Array.Empty<ulong>());
        var lo = new ulong[m];
        Array.Copy(a, 0, lo, 0, m);
        var hi = new ulong[a.Length - m];
        Array.Copy(a, m, hi, 0, a.Length - m);
        return (TrimCopy(lo, m), hi);
    }

    /// <summary>Canonical limb sum (no trailing zeros).</summary>
    private static ulong[] AddRaw(ulong[] a, ulong[] b)
    {
        if (a.Length == 0) return b;
        if (b.Length == 0) return a;
        int n = Math.Max(a.Length, b.Length);
        var r = new ulong[n + 1];
        ulong carry = 0;
        for (int i = 0; i < n; i++)
        {
            ulong av = i < a.Length ? a[i] : 0UL;
            ulong bv = i < b.Length ? b[i] : 0UL;
            ulong s = av + bv;
            ulong c0 = s < av ? 1UL : 0UL;
            ulong s2 = s + carry;
            ulong c1 = s2 < s ? 1UL : 0UL;
            r[i] = s2;
            carry = c0 + c1;
        }
        if (carry != 0)
        {
            r[n] = carry;
            return TrimCopy(r, n + 1);
        }
        return TrimCopy(r, n);
    }

    /// <summary>Canonical limb difference (caller guarantees a ≥ b).</summary>
    private static ulong[] SubRaw(ulong[] a, ulong[] b)
    {
        if (b.Length == 0) return a;
        var r = new ulong[a.Length];
        ulong borrow = 0;
        for (int i = 0; i < a.Length; i++)
        {
            ulong bv = i < b.Length ? b[i] : 0UL;
            ulong s1 = a[i] - bv;
            ulong b1 = s1 > a[i] ? 1UL : 0UL;
            ulong s2 = s1 - borrow;
            ulong b2 = s2 > s1 ? 1UL : 0UL;
            r[i] = s2;
            borrow = b1 + b2;
        }
        return TrimCopy(r, a.Length);
    }

    /// <summary>Adds <paramref name="x"/> · 2^(64·<paramref name="shift"/>) into the zero-initialised <paramref name="r"/>.</summary>
    private static void AddShifted(ulong[] r, ulong[] x, int shift)
    {
        if (x.Length == 0) return;
        UInt128 carry = 0;
        for (int i = 0; i < x.Length; i++)
        {
            UInt128 s = (UInt128)r[i + shift] + x[i] + carry;
            r[i + shift] = (ulong)s;
            carry = s >> 64;
        }
        int k = shift + x.Length;
        while (carry != 0)
        {
            UInt128 s = r[k] + carry;
            r[k] = (ulong)s;
            carry = s >> 64;
            k++;
        }
    }

    /// <summary>Copies <paramref name="len"/> limbs and trims trailing zeros to a canonical array.</summary>
    private static ulong[] TrimCopy(ulong[] src, int len)
    {
        while (len > 0 && src[len - 1] == 0) len--;
        if (len == 0) return Array.Empty<ulong>();
        var r = new ulong[len];
        Array.Copy(src, r, len);
        return r;
    }

    // -------------------------------------------------------------------------
    // Fast multiplication — Number-Theoretic Transform (exact convolution) over
    // 64-bit limbs split into base-2^16 pieces.
    // -------------------------------------------------------------------------

    /// <summary>998244353 = 119·2^23 + 1 — first NTT prime, length ≤ 2^23, primitive root 3.</summary>
    private const long NttPrime1 = 998244353L;

    /// <summary>469762049 = 7·2^26 + 1 — second NTT prime (CRT), length ≤ 2^26, primitive root 3.</summary>
    private const long NttPrime2 = 469762049L;

    /// <summary>Primitive root shared by both NTT primes.</summary>
    private const long NttRoot = 3L;

    /// <summary>Largest transform length either prime supports (bounded by <see cref="NttPrime1"/>).</summary>
    private const int MaxNttLength = 1 << 23;

    /// <summary>Each 64-bit limb is split into four base-2^16 pieces.</summary>
    private const int NttBaseBits = 16;
    private const int NttPieces = 4;

    /// <summary>
    /// Operands whose combined limb count reaches this value use the NTT product. The exact
    /// convolution is cheap to pack (base-2^16 bit-slices) but the transform itself has a
    /// higher constant than Karatsuba, so the threshold sits well above practical small sizes.
    /// </summary>
    private const long NttThreshold = 100000; // a+b limbs (≈ 1.9M decimal digits); NTT wins past this crossover

    private static long ModPow(long b, long e, long mod)
    {
        long result = 1;
        b %= mod;
        while (e > 0)
        {
            if ((e & 1) != 0)
                result = result * b % mod;
            b = b * b % mod;
            e >>= 1;
        }
        return result;
    }

    private static readonly long InvP1ModP2 = ModPow(NttPrime1 % NttPrime2, NttPrime2 - 2, NttPrime2);

    /// <summary>In-place iterative Cooley–Tukey NTT modulo <paramref name="prime"/>.</summary>
    private static void Ntt(long[] a, int n, bool invert, long prime)
    {
        for (int i = 1, j = 0; i < n; i++)
        {
            int bit = n >> 1;
            for (; (j & bit) != 0; bit >>= 1)
                j ^= bit;
            j ^= bit;
            if (i < j)
                (a[i], a[j]) = (a[j], a[i]);
        }

        long root = invert ? ModPow(NttRoot, prime - 2, prime) : NttRoot;

        for (int len = 2; len <= n; len <<= 1)
        {
            long wlen = ModPow(root, (prime - 1) / len, prime);
            int half = len >> 1;
            for (int i = 0; i < n; i += len)
            {
                long w = 1;
                for (int j = 0; j < half; j++)
                {
                    long u = a[i + j];
                    long v = a[i + j + half] * w % prime;
                    long s = u + v;
                    a[i + j] = s < prime ? s : s - prime;
                    long d = u - v;
                    a[i + j + half] = d >= 0 ? d : d + prime;
                    w = w * wlen % prime;
                }
            }
        }

        if (invert)
        {
            long nInv = ModPow(n, prime - 2, prime);
            for (int i = 0; i < n; i++)
                a[i] = a[i] * nInv % prime;
        }
    }

    /// <summary>Splits <paramref name="limbs"/> into base-2^16 pieces (LSD-first) in <paramref name="pieces"/>.</summary>
    private static void PackPieces(ulong[] limbs, long[] pieces)
    {
        int idx = 0;
        for (int i = 0; i < limbs.Length; i++)
        {
            ulong v = limbs[i];
            pieces[idx++] = (long)(v & 0xFFFF);
            pieces[idx++] = (long)((v >> 16) & 0xFFFF);
            pieces[idx++] = (long)((v >> 32) & 0xFFFF);
            pieces[idx++] = (long)((v >> 48) & 0xFFFF);
        }
    }

    /// <summary>Two-prime CRT: reconstructs a convolution coefficient &lt; p1·p2 from its residues.</summary>
    private static ulong Crt(long r1, long r2)
    {
        long diff = (r2 - r1) % NttPrime2;
        if (diff < 0) diff += NttPrime2;
        long t = diff * InvP1ModP2 % NttPrime2;
        return (ulong)(r1 + t * NttPrime1);
    }

    /// <summary>
    /// Multiplies two limb arrays via a two-prime NTT over base-2^16 pieces. A convolution
    /// coefficient is &lt; (2^16−1)² · minPieceCount &lt; 2^32 · 2^23 &lt; p1·p2, so the CRT is exact.
    /// </summary>
    private static ulong[] NttMultiply(ulong[] a, ulong[] b)
    {
        int aPieces = a.Length * NttPieces;
        int bPieces = b.Length * NttPieces;
        int need = aPieces + bPieces;
        int size = 1;
        while (size < need)
            size <<= 1;

        long[] fa = new long[size];
        long[] fb = new long[size];
        PackPieces(a, fa);
        PackPieces(b, fb);

        // Prime 1 (fa is clobbered, so keep a copy of the packed 'a' for prime 2).
        long[] fa2 = (long[])fa.Clone();
        Ntt(fa, size, false, NttPrime1);
        Ntt(fb, size, false, NttPrime1);
        for (int i = 0; i < size; i++)
            fa[i] = fa[i] * fb[i] % NttPrime1;
        Ntt(fa, size, true, NttPrime1);

        // Prime 2 (fb was clobbered, re-pack 'b').
        Array.Clear(fb, 0, size);
        PackPieces(b, fb);
        Ntt(fa2, size, false, NttPrime2);
        Ntt(fb, size, false, NttPrime2);
        for (int i = 0; i < size; i++)
            fa2[i] = fa2[i] * fb[i] % NttPrime2;
        Ntt(fa2, size, true, NttPrime2);

        // CRT + carry-propagation back into 64-bit limbs.
        var result = new ulong[a.Length + b.Length];
        UInt128 carry = 0;
        int ci = 0;
        for (int limb = 0; limb < result.Length; limb++)
        {
            UInt128 val = carry;
            for (int p = 0; p < NttPieces; p++)
            {
                ulong coeff = Crt(fa[ci], fa2[ci]);
                val += (UInt128)coeff << (NttBaseBits * p);
                ci++;
            }
            result[limb] = (ulong)val;
            carry = val >> 64;
        }
        return result;
    }

    /// <summary>Knuth Algorithm D long division. Caller guarantees u ≥ v, v.Length ≥ 2.</summary>
    private static (Natural quotient, Natural remainder) DivRemKnuth(ulong[] u, ulong[] v)
    {
        int n = u.Length;
        int m = v.Length;
        int shift = BitOperations.LeadingZeroCount(v[m - 1]);

        ulong[] vn = ShiftLeftBits(v, shift);   // length m+1; vn[m] == 0 (top bit set after shift)
        ulong[] un = ShiftLeftBits(u, shift);   // length n+1

        var q = new ulong[n - m + 1];

        for (int j = n - m; j >= 0; j--)
        {
            UInt128 top = ((UInt128)un[j + m] << 64) | un[j + m - 1];
            // qhat may reach B (= 2^64) when the top dividend limb equals the top divisor
            // limb; the loop below clamps it to a single limb. rhat = top % vn[m-1] is
            // always < 2^64, so "rhat << 64" never overflows inside the loop.
            UInt128 qhat = top / (UInt128)vn[m - 1];
            UInt128 rhat = top % (UInt128)vn[m - 1];

            // Refine the estimate: enforce qhat < B and the two-limb test (m ≥ 2 guaranteed).
            while (true)
            {
                UInt128 rhs = ((UInt128)rhat << 64) | un[j + m - 2];
                bool tooBig = (qhat >> 64) != 0 || qhat * (UInt128)vn[m - 2] > rhs;
                if (!tooBig) break;
                qhat--;
                rhat += (UInt128)vn[m - 1];
                if ((rhat >> 64) != 0) break;
            }

            // Multiply-subtract: un[j..j+m] -= qhat * vn[0..m-1].
            var prod = new ulong[m + 1];
            UInt128 carry = 0;
            for (int i = 0; i < m; i++)
            {
                UInt128 t = qhat * (UInt128)vn[i] + carry;
                prod[i] = (ulong)t;
                carry = t >> 64;
            }
            prod[m] = (ulong)carry;

            ulong borrow = 0;
            for (int i = 0; i <= m; i++)
            {
                ulong uj = un[j + i];
                ulong pi = prod[i];
                ulong t = uj - pi;
                ulong b1 = t > uj ? 1UL : 0UL;
                ulong t2 = t - borrow;
                ulong b2 = t2 > t ? 1UL : 0UL;
                un[j + i] = t2;
                borrow = b1 + b2;
            }

            if (borrow != 0)
            {
                // qhat was one too high: add back vn.
                qhat--;
                UInt128 addCarry = 0;
                for (int i = 0; i < m; i++)
                {
                    UInt128 s = (UInt128)un[j + i] + vn[i] + addCarry;
                    un[j + i] = (ulong)s;
                    addCarry = s >> 64;
                }
                un[j + m] += (ulong)addCarry;
            }

            q[j] = (ulong)qhat;
        }

        Natural quotient = Make(q, n - m + 1);
        Natural remainder = Make(ShiftRightBits(un, m, shift), m);
        return (quotient, remainder);
    }

    /// <summary>Left-shifts the limbs of <paramref name="a"/> by <paramref name="shift"/> bits (0–63).</summary>
    private static ulong[] ShiftLeftBits(ulong[] a, int shift)
    {
        var r = new ulong[a.Length + 1];
        if (shift == 0)
        {
            Array.Copy(a, r, a.Length);
            return r;
        }
        ulong carry = 0;
        for (int i = 0; i < a.Length; i++)
        {
            ulong t = a[i];
            r[i] = (t << shift) | carry;
            carry = t >> (64 - shift);
        }
        r[a.Length] = carry;
        return r;
    }

    /// <summary>Right-shifts the first <paramref name="len"/> limbs of <paramref name="a"/> by <paramref name="shift"/> bits.</summary>
    private static ulong[] ShiftRightBits(ulong[] a, int len, int shift)
    {
        var r = new ulong[len];
        if (shift == 0)
        {
            Array.Copy(a, r, len);
            return r;
        }
        ulong carry = 0;
        for (int i = len - 1; i >= 0; i--)
        {
            ulong t = a[i];
            r[i] = (t >> shift) | carry;
            carry = t << (64 - shift);
        }
        return r;
    }

    /// <summary>Returns 10^k as a Natural (binary exponentiation).</summary>
    private static Natural Pow10(long k)
    {
        if (k <= 0) return s_one;
        var result = s_one;
        var b = new Natural(10UL);
        ulong e = (ulong)k;
        while (e > 0)
        {
            if ((e & 1UL) != 0) result *= b;
            e >>= 1;
            if (e != 0) b *= b;
        }
        return result;
    }

    private const int ParseChunkDigits = 19; // 10^19 < 2^64

    /// <summary>Converts a canonical decimal digit span (no leading zeros) into a Natural.</summary>
    private static Natural ParseDigits(ReadOnlySpan<char> digits) => ParseDigitsPair(digits).value;

    /// <summary>
    /// Divide-and-conquer decimal→binary conversion: returns (value, 10^length) for the span,
    /// so each level does one multiply + one add and the whole conversion is O(M(n) log n).
    /// </summary>
    private static (Natural value, Natural pow10) ParseDigitsPair(ReadOnlySpan<char> digits)
    {
        if (digits.Length <= ParseChunkDigits)
        {
            ulong v = 0;
            for (int i = 0; i < digits.Length; i++)
                v = v * 10UL + (uint)(digits[i] - '0');
            return (new Natural(v), Pow10Ulong(digits.Length));
        }

        int mid = digits.Length / 2;
        (Natural hi, Natural hiPow) = ParseDigitsPair(digits[..mid]);   // hiPow = 10^mid
        (Natural lo, Natural loPow) = ParseDigitsPair(digits[mid..]);   // loPow = 10^(len-mid)

        Natural value = hi * loPow + lo;
        Natural pow10 = hiPow * loPow;                                  // 10^len
        return (value, pow10);
    }

    private static Natural Pow10Ulong(int n)
    {
        ulong p = 1;
        for (int i = 0; i < n; i++) p *= 10UL;
        return new Natural(p);
    }

    private static string GroupThousands(string s)
    {
        int len = s.Length;
        if (len <= 3) return s;
        var sb = new System.Text.StringBuilder(len + len / 3);
        for (int i = 0; i < len; i++)
        {
            sb.Append(s[i]);
            int distFromRight = len - 1 - i;
            if (distFromRight > 0 && distFromRight % 3 == 0)
                sb.Append(',');
        }
        return sb.ToString();
    }
}
