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
    // A value is held in one or both of two forms, materialized lazily and cached:
    //   _limbs   — binary limbs (null until converted from the decimal string)
    //   _decimal — canonical decimal digits (null until converted from the limbs)
    // At least one is always non-null; zero is "0" / empty limbs. Limbs and the
    // decimal string are immutable once created (operators return new instances);
    // only the two lazy-cache fields themselves are written, thread-safely.
    // -------------------------------------------------------------------------

    private ulong[]? _limbs;

    /// <summary>Canonical decimal digit string; null until computed from <see cref="_limbs"/>.</summary>
    private string? _decimal;

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
    public Natural() { _limbs = s_empty; _decimal = "0"; }

    /// <summary>Copy constructor — deep copy of <paramref name="other"/>'s limbs; shares the
    /// cached decimal string (strings are immutable).</summary>
    public Natural(Natural other)
    {
        _decimal = other._decimal;
        var src = other.GetLimbs();
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

    /// <summary>Constructs a <see cref="Natural"/> by parsing a decimal digit string (limbs are
    /// materialized lazily; only the canonical decimal string is stored).</summary>
    public Natural(string s)
    {
        var canonical = TryCanonicalize(s.AsSpan());
        if (canonical is null)
            throw new FormatException($"The string '{s}' is not a valid decimal representation of a Natural number.");
        _decimal = canonical;
        _limbs = canonical == "0" ? s_empty : null;
    }

    /// <summary>Constructs a <see cref="Natural"/> by parsing a span of decimal digit characters
    /// (limbs are materialized lazily; only the canonical decimal string is stored).</summary>
    public Natural(ReadOnlySpan<char> s)
    {
        var canonical = TryCanonicalize(s);
        if (canonical is null)
            throw new FormatException("The input is not a valid decimal representation of a Natural number.");
        _decimal = canonical;
        _limbs = canonical == "0" ? s_empty : null;
    }

    // -------------------------------------------------------------------------
    // Lazy representation accessors — each form is materialized once on first use
    // -------------------------------------------------------------------------

    /// <summary>Returns the binary limbs, converting from the decimal string on first use.</summary>
    private ulong[] GetLimbs()
    {
        var limbs = _limbs;
        if (limbs is not null) return limbs;
        var computed = ParseDigits(_decimal!).GetLimbs();
        Interlocked.CompareExchange(ref _limbs, computed, null);
        return _limbs!;
    }

    /// <summary>Returns the canonical decimal string, converting from the limbs on first use.</summary>
    private string GetDecimal()
    {
        var dec = _decimal;
        if (dec is not null) return dec;
        var computed = _limbs!.Length == 0 ? "0" : ToStringRecursive(_limbs);
        Interlocked.CompareExchange(ref _decimal, computed, null);
        return _decimal!;
    }

    // -------------------------------------------------------------------------
    // INumberBase<Natural> — classification predicates
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static bool IsZero(Natural value)
    {
        var dec = value._decimal;
        if (dec is not null) return dec == "0";
        return value._limbs!.Length == 0;
    }

    /// <inheritdoc/>
    public static bool IsEvenInteger(Natural value) => !IsOddInteger(value);

    /// <inheritdoc/>
    public static bool IsOddInteger(Natural value)
    {
        var dec = value._decimal;
        if (dec is not null) return ((dec[^1] - '0') & 1) != 0;
        var limbs = value._limbs!;
        return limbs.Length != 0 && (limbs[0] & 1UL) != 0;
    }

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
        var a = GetLimbs();
        var b = other.GetLimbs();
        if (a.Length != b.Length) return false;
        for (int i = 0; i < a.Length; i++)
            if (a[i] != b[i]) return false;
        return true;
    }

    /// <inheritdoc/>
    public int CompareTo(Natural? other)
    {
        if (other is null) return 1;
        return CompareLimbs(GetLimbs(), other.GetLimbs());
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
        var a = left.GetLimbs();
        var b = right.GetLimbs();
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
        var a = left.GetLimbs();
        var b = right.GetLimbs();
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
        var a = left.GetLimbs();
        var b = right.GetLimbs();
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
        var u = left.GetLimbs();
        var v = right.GetLimbs();
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

        if ((long)u.Length + v.Length >= DivNewtonThreshold)
        {
            (Natural qNewton, Natural remNewton) = DivRemNewton(u, v);
            remainder = remNewton;
            return qNewton;
        }

        (Natural qKnuth, Natural remKnuth) = DivRemKnuth(u, v);
        remainder = remKnuth;
        return qKnuth;
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
    public override string ToString() => GetDecimal();

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
        string hi = ToStringRecursive(q.GetLimbs());
        string lo = ToStringRecursive(r.GetLimbs());
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
        var canonical = TryCanonicalize(s);
        if (canonical is null)
        {
            result = null;
            return false;
        }

        result = canonical == "0" ? s_zero : FromDecimalString(canonical);
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

    /// <summary>
    /// Dividends+divisors with at least this many combined limbs use Newton-reciprocal division.
    /// Knuth's O(n·m) has a small constant and wins up to ~5M digits; Newton (O(M(n) log n))
    /// takes over beyond that, where the quadratic Knuth cost explodes.
    /// </summary>
    private const long DivNewtonThreshold = 1L << 18; // 262144 limbs (≈ 5M decimal digits)

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

    // -------------------------------------------------------------------------
    // Fast division — Newton reciprocal (O(M(n) log n))
    // -------------------------------------------------------------------------

    /// <summary>Bit length of a canonical non-zero limb array.</summary>
    private static int BitLength(ulong[] a)
        => a.Length * 64 - BitOperations.LeadingZeroCount(a[^1]);

    /// <summary>Returns 2^<paramref name="bit"/> as a canonical limb array.</summary>
    private static ulong[] PowerOfTwo(int bit)
    {
        var r = new ulong[bit / 64 + 1];
        r[bit / 64] = 1UL << (bit % 64);
        return r;
    }

    /// <summary>Right-shifts <paramref name="a"/> by <paramref name="bits"/> bits (floor ÷ 2^bits).</summary>
    private static ulong[] ShiftRightByBits(ulong[] a, int bits)
    {
        int wordShift = bits / 64;
        int bitShift = bits % 64;
        int srcLen = a.Length - wordShift;
        if (srcLen <= 0) return Array.Empty<ulong>();
        var r = new ulong[srcLen];
        if (bitShift == 0)
        {
            Array.Copy(a, wordShift, r, 0, srcLen);
        }
        else
        {
            for (int i = 0; i < srcLen; i++)
            {
                ulong low = a[i + wordShift];
                ulong high = (i + wordShift + 1 < a.Length) ? a[i + wordShift + 1] : 0UL;
                r[i] = (low >> bitShift) | (high << (64 - bitShift));
            }
        }
        return TrimCopy(r, r.Length);
    }

    /// <summary>Returns <paramref name="a"/> + 1 as a canonical limb array.</summary>
    private static ulong[] AddOneLimbs(ulong[] a)
    {
        var r = new ulong[a.Length + 1];
        Array.Copy(a, r, a.Length);
        int i = 0;
        while (true)
        {
            r[i]++;
            if (r[i] != 0) break;
            i++;
        }
        return TrimCopy(r, r.Length);
    }

    /// <summary>Result sizes with at most this many bits use the direct base case.</summary>
    private const int ReciprocalBaseBits = 64;

    /// <summary>
    /// Returns floor(2^<paramref name="k"/> / d) for a normalized d (top bit set), k ≥ bitlen(d).
    /// Recursive half-precision Newton: the working size stays proportional to the result
    /// (no scale overshoot), and the result is exact after ±1 correction.
    /// </summary>
    private static ulong[] Reciprocal(ulong[] d, int k)
    {
        int n = BitLength(d);
        int q = k - n + 1;                 // bits of the result
        if (q <= ReciprocalBaseBits)
            return ReciprocalBase(d, n, k);

        int q0 = (q + 1) / 2;
        int f0 = n + q0 - 1;               // half-precision scale
        ulong[] x0 = Reciprocal(d, f0);    // floor(2^f0 / d), q0 bits

        ulong[] dx0 = Multiply(d, x0);
        ulong[] two = PowerOfTwo(f0 + 1);
        ulong[] m = SubRaw(two, dx0);      // 2^(f0+1) − d·x0 ≥ 2^f0 > 0
        ulong[] x = Multiply(x0, m);       // ≈ 2^(2·f0) / d

        int shift = 2 * f0 - k;
        if (shift > 0)
            x = ShiftRightByBits(x, shift);

        return CorrectReciprocal(x, d, k);
    }

    /// <summary>Base case (q ≤ 64 bits): seed from the top limb, shift, and correct.</summary>
    private static ulong[] ReciprocalBase(ulong[] d, int n, int k)
    {
        ulong dTop = d[^1];
        UInt128 two64 = (UInt128)1 << 64;
        UInt128 q1 = two64 / dTop;
        UInt128 r1 = two64 % dTop;
        UInt128 x0 = (q1 << 64) + ((r1 << 64) / dTop);   // ≈ 2^(n+64) / d

        ulong[] x = TrimCopy(new[] { (ulong)x0, (ulong)(x0 >> 64) }, 2);
        int shift = n + 64 - k;            // = 65 − q
        if (shift > 0)
            x = ShiftRightByBits(x, shift);
        return CorrectReciprocal(x, d, k);
    }

    /// <summary>Adjusts <paramref name="x"/> (within ±2 of floor(2^k/d)) to the exact floor.</summary>
    private static ulong[] CorrectReciprocal(ulong[] x, ulong[] d, int k)
    {
        ulong[] pk = PowerOfTwo(k);
        ulong[] xd = Multiply(x, d);
        while (CompareLimbs(xd, pk) > 0)   // x·d > 2^k: x too big
        {
            x = SubtractOneLimbs(x);
            xd = Multiply(x, d);
        }
        ulong[] xp1 = AddOneLimbs(x);
        ulong[] xp1d = Multiply(xp1, d);
        while (CompareLimbs(xp1d, pk) <= 0)  // (x+1)·d ≤ 2^k: x too small
        {
            x = xp1;
            xp1 = AddOneLimbs(x);
            xp1d = Multiply(xp1, d);
        }
        return x;
    }

    /// <summary>Returns <paramref name="a"/> − 1 as a canonical limb array (a &gt; 0).</summary>
    private static ulong[] SubtractOneLimbs(ulong[] a)
    {
        var r = (ulong[])a.Clone();
        int i = 0;
        while (true)
        {
            if (r[i] != 0)
            {
                r[i]--;
                break;
            }
            r[i] = ulong.MaxValue;   // borrow
            i++;
        }
        return TrimCopy(r, r.Length);
    }

    /// <summary>Newton-reciprocal quotient/remainder. Caller guarantees u ≥ v, v.Length ≥ 2.</summary>
    private static (Natural quotient, Natural remainder) DivRemNewton(ulong[] u, ulong[] v)
    {
        int shift = BitOperations.LeadingZeroCount(v[^1]);
        ulong[] vn = TrimCopy(ShiftLeftBits(v, shift), v.Length);
        ulong[] un = TrimCopy(ShiftLeftBits(u, shift), u.Length + 1);

        int nBits = BitLength(un);
        int k = nBits + 64;

        ulong[] R = Reciprocal(vn, k);          // floor(2^k / vn)
        ulong[] q = ShiftRightByBits(Multiply(un, R), k);  // ≤ floor(un / vn)
        ulong[] qv = Multiply(q, vn);
        ulong[] r = SubRaw(un, qv);             // qv ≤ un, so no underflow

        while (CompareLimbs(r, vn) >= 0)
        {
            r = SubRaw(r, vn);
            q = AddOneLimbs(q);
        }

        ulong[] rem = ShiftRightByBits(r, shift);
        return (Make(q, q.Length), Make(rem, rem.Length));
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

    /// <summary>Validates a decimal digit span and returns its canonical (leading-zero-stripped)
    /// form, or null when empty or containing a non-digit.</summary>
    private static string? TryCanonicalize(ReadOnlySpan<char> s)
    {
        if (s.IsEmpty) return null;
        foreach (char ch in s)
            if (ch < '0' || ch > '9') return null;
        int start = 0;
        while (start < s.Length - 1 && s[start] == '0') start++;
        return s[start..].ToString();
    }

    /// <summary>Creates a <see cref="Natural"/> held only as a canonical decimal string, with
    /// limbs materialized lazily on first arithmetic use. Caller guarantees a non-zero value.</summary>
    private static Natural FromDecimalString(string canonical)
    {
        var n = new Natural();
        n._decimal = canonical;
        n._limbs = null;
        return n;
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
