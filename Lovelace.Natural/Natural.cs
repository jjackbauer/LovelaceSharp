using System.Diagnostics.CodeAnalysis;
using System.Globalization;
using System.Numerics;
using Lovelace.Representation;

namespace Lovelace.Natural;

/// <summary>
/// Arbitrary-precision natural number (ℕ₀, i.e. non-negative integers).
/// Backed by <see cref="DigitStore"/> which packs two BCD digits per byte.
/// Corresponds to the arithmetic layer of the C++ <c>Lovelace</c> class.
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
    // Backing store
    // -------------------------------------------------------------------------

    private DigitStore _store;

    // -------------------------------------------------------------------------
    // Static configuration properties (C++ algarismosExibicao / Precisao)
    // -------------------------------------------------------------------------

    /// <summary>
    /// Maximum number of digits to display when formatting.
    /// -1 means "no limit" (display all digits). Matches C++ <c>algarismosExibicao</c>.
    /// </summary>
    public static long DisplayDigits { get; set; } = -1L;

    /// <summary>
    /// Precision hint. Stub — C++ body was absent. Matches C++ <c>Precisao</c>.
    /// </summary>
    public static long Precision { get; set; } = -1L;

    // -------------------------------------------------------------------------
    // INumberBase<Natural> — required static properties
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static Natural One => new(1UL);

    /// <inheritdoc/>
    public static int Radix => 10;

    /// <inheritdoc/>
    public static Natural Zero => new();

    /// <inheritdoc/>
    public static Natural AdditiveIdentity => Zero;

    /// <inheritdoc/>
    public static Natural MultiplicativeIdentity => One;

    // -------------------------------------------------------------------------
    // Constructors
    // -------------------------------------------------------------------------

    /// <summary>Default constructor — produces zero.</summary>
    public Natural()
    {
        _store = new DigitStore();
    }

    /// <summary>Copy constructor — deep copy of <paramref name="other"/>.</summary>
    public Natural(Natural other)
    {
        _store = new DigitStore(other._store);
    }

    /// <summary>Constructs a <see cref="Natural"/> from an unsigned 64-bit integer.</summary>
    public Natural(ulong value)
    {
        _store = new DigitStore();
        if (value == 0) return; // _store already represents zero

        // Extract digits least-significant first (mirrors C++ atribuir(unsigned long long)).
        long pos = 0;
        while (value > 0)
        {
            _store.SetDigit(pos++, (byte)(value % 10));
            value /= 10;
        }
    }

    /// <summary>Constructs a <see cref="Natural"/> from a non-negative <see cref="int"/>.</summary>
    /// <exception cref="ArgumentOutOfRangeException">Thrown when <paramref name="value"/> is negative.</exception>
    public Natural(int value)
    {
        if (value < 0)
            throw new ArgumentOutOfRangeException(nameof(value), "Value must be non-negative.");

        _store = new DigitStore();
        ulong uval = (ulong)value;
        if (uval == 0) return;

        long pos = 0;
        while (uval > 0)
        {
            _store.SetDigit(pos++, (byte)(uval % 10));
            uval /= 10;
        }
    }

    /// <summary>Constructs a <see cref="Natural"/> by parsing a decimal digit string.</summary>
    /// <param name="s">A string of decimal digits, optionally with leading zeros.</param>
    /// <exception cref="FormatException">Thrown when <paramref name="s"/> is empty or contains non-digit characters.</exception>
    public Natural(string s)
    {
        var parsed = Parse(s, null); // throws FormatException on invalid input
        _store = parsed._store;
    }

    /// <summary>Constructs a <see cref="Natural"/> by parsing a span of decimal digit characters.</summary>
    /// <param name="s">A read-only span of decimal digits, optionally with leading zeros.</param>
    /// <exception cref="FormatException">Thrown when <paramref name="s"/> is empty or contains non-digit characters.</exception>
    public Natural(ReadOnlySpan<char> s)
    {
        var parsed = Parse(s, null); // throws FormatException on invalid input
        _store = parsed._store;
    }

    // -------------------------------------------------------------------------
    // INumberBase<Natural> — classification predicates
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static bool IsZero(Natural value) => value._store.IsZero;

    /// <inheritdoc/>
    /// <remarks>
    /// A Natural is even when its least-significant digit is even (0, 2, 4, 6, 8).
    /// Zero is even (its LSD is 0). Mirrors C++ <c>ePar</c>: <c>!eImpar()</c>.
    /// </remarks>
    public static bool IsEvenInteger(Natural value) => !IsOddInteger(value);

    /// <inheritdoc/>
    /// <remarks>
    /// A Natural is odd when bit 0 of its least-significant digit is set.
    /// Zero is never odd. Mirrors C++ <c>eImpar</c>: <c>!eZero() &amp;&amp; (getDigito(0) % 2)</c>.
    /// </remarks>
    public static bool IsOddInteger(Natural value)
        => !value._store.IsZero && (value._store.GetDigit(0) % 2 != 0);

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
        if (_store.IsZero && other._store.IsZero) return true;
        if (_store.DigitCount != other._store.DigitCount) return false;
        for (long i = 0; i < _store.DigitCount; i++)
            if (_store.GetDigit(i) != other._store.GetDigit(i)) return false;
        return true;
    }

    /// <inheritdoc/>
    public int CompareTo(Natural? other)
    {
        if (other is null) return 1;

        // Compare digit counts first (more digits → larger number).
        long aCount = _store.IsZero ? 0 : _store.DigitCount;
        long bCount = other._store.IsZero ? 0 : other._store.DigitCount;

        if (aCount != bCount) return aCount.CompareTo(bCount);

        // Same digit count — compare from most-significant to least-significant.
        for (long i = aCount - 1; i >= 0; i--)
        {
            int cmp = _store.GetDigit(i).CompareTo(other._store.GetDigit(i));
            if (cmp != 0) return cmp;
        }
        return 0;
    }

    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is Natural n && Equals(n);

    /// <inheritdoc/>
    public override int GetHashCode() => ToString().GetHashCode();

    /// <summary>
    /// Implements non-generic <see cref="IComparable.CompareTo(object?)"/>
    /// required by <see cref="INumber{T}"/>.
    /// </summary>
    public int CompareTo(object? obj)
    {
        if (obj is null) return 1;
        if (obj is Natural other) return CompareTo(other);
        throw new ArgumentException($"Object must be of type {nameof(Natural)}.", nameof(obj));
    }

    // -------------------------------------------------------------------------
    // Unary operators (required by INumber<T>)
    // -------------------------------------------------------------------------

    /// <summary>Unary plus — returns a copy of <paramref name="value"/> unchanged.</summary>
    public static Natural operator +(Natural value) => new(value);

    /// <summary>
    /// Unary negation — not representable in ℕ₀.
    /// </summary>
    /// <exception cref="InvalidOperationException">Always thrown — naturals cannot be negated.</exception>
    public static Natural operator -(Natural value)
        => throw new InvalidOperationException("Cannot negate a Natural number; the result would be negative.");

    // -------------------------------------------------------------------------
    // Arithmetic operators
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static Natural operator +(Natural left, Natural right)
    {
        // Identity shortcuts
        if (IsZero(left)) return new Natural(right);
        if (IsZero(right)) return new Natural(left);

        var result = new Natural();
        long maxDigi = Math.Max(left._store.DigitCount, right._store.DigitCount);

        // Digit-by-digit addition with carry (position 0 = LSD, mirrors C++ somar).
        int carry = 0;
        for (long c = 0; c < maxDigi; c++)
        {
            int sum = left._store.GetDigit(c) + right._store.GetDigit(c) + carry;
            carry   = sum / 10;
            result._store.SetDigit(c, (byte)(sum % 10));
        }

        // Propagate any final carry into a new most-significant digit.
        if (carry > 0)
            result._store.SetDigit(maxDigi, (byte)carry);

        return result;
    }

    /// <summary>
    /// Subtracts <paramref name="right"/> from <paramref name="left"/>.
    /// </summary>
    /// <exception cref="InvalidOperationException">
    /// Thrown when <paramref name="right"/> &gt; <paramref name="left"/>, since the result
    /// would be negative and cannot be represented as a <see cref="Natural"/>.
    /// </exception>
    public static Natural operator -(Natural left, Natural right)
    {
        // ℕ₀ contract: subtraction must not produce a negative result.
        if (right > left)
            throw new InvalidOperationException(
                "Subtraction would produce a negative result, which cannot be represented as a Natural.");

        // Identity shortcuts.
        if (IsZero(right)) return new Natural(left);
        if (left == right) return new Natural(); // zero

        // Standard borrow subtraction (mirrors C++ subtrair, but without the
        // operand-swap shortcut that the C++ uses for |A−B|).
        // carry=1 means "no pending borrow"; carry=0 means "borrow from next digit".
        var result = new Natural();
        long aCount = left._store.DigitCount;
        int carry = 1;

        for (long c = 0; c < aCount; c++)
        {
            int current = 10 + left._store.GetDigit(c) - right._store.GetDigit(c) - (1 - carry);
            carry = current / 10;     // 1 if no borrow needed next round, 0 if borrow
            result._store.SetDigit(c, (byte)(current % 10));
        }

        // Strip any leading zeros produced by the subtraction
        // (e.g. 1000 − 1 writes 4 digits: 9, 9, 9, 0 → trim the leading 0).
        result._store.TrimLeadingZeros();

        return result;
    }

    /// <inheritdoc/>
    /// <remarks>
    /// Long (grade-school) multiplication. Mirrors C++ <c>multiplicar</c>:
    /// the smaller operand drives the outer loop (fewer partial products);
    /// each partial product is shifted by the outer-loop digit's position and
    /// accumulated via <c>operator+</c>.
    /// </remarks>
    public static Natural operator *(Natural left, Natural right)
    {
        // Absorbing element: 0 × anything = 0.
        if (IsZero(left) || IsZero(right)) return new Natural();

        // Put the smaller operand in the outer loop to minimise iterations,
        // mirroring the C++ assignment: aux = log ? B : *this.
        bool leftIsLarger = left > right;
        Natural aux  = leftIsLarger ? right : left;  // smaller (outer loop)
        Natural aux1 = leftIsLarger ? left  : right; // larger  (inner loop)

        var result = new Natural();

        for (long c = 0; c < aux._store.DigitCount; c++)
        {
            int multiplicador = aux._store.GetDigit(c);
            if (multiplicador == 0) continue;

            // Build partial product shifted left by c positions.
            var temp = new Natural();

            // Fill positions 0..c-1 with zeros to satisfy SetDigit's sequential-write
            // constraint (position <= DigitCount) and produce the correct positional shift.
            // Setting position 0 also clears the IsZero flag on the DigitStore.
            for (long c1 = 0; c1 < c; c1++)
                temp._store.SetDigit(c1, 0);

            // Multiply each digit of aux1 by multiplicador and store at position c2+c.
            int overflow = 0;
            long c2 = 0;
            for (; c2 < aux1._store.DigitCount; c2++)
            {
                int produto = aux1._store.GetDigit(c2) * multiplicador + overflow;
                temp._store.SetDigit(c2 + c, (byte)(produto % 10));
                overflow = produto / 10;
            }
            if (overflow > 0)
                temp._store.SetDigit(c2 + c, (byte)overflow);

            result += temp;
        }

        return result;
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
    /// <remarks>
    /// Mirrors C++ <c>incrementar()</c>: <c>somar(aux=1)</c>, i.e. adds 1 to the value.
    /// The C# compiler derives both prefix (<c>++n</c>) and postfix (<c>n++</c>) semantics
    /// from this single definition.
    /// </remarks>
    public static Natural operator ++(Natural value) => value + One;

    /// <inheritdoc/>
    /// <remarks>
    /// Mirrors C++ <c>decrementar()</c>: <c>subtrair(aux=1)</c>, i.e. subtracts 1.
    /// Throws <see cref="InvalidOperationException"/> when <paramref name="value"/> is zero,
    /// since ℕ₀ has no negative values.
    /// </remarks>
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
    /// Divides <paramref name="left"/> by <paramref name="right"/> using long division,
    /// returning the quotient and setting <paramref name="remainder"/> to the remainder.
    /// Mirrors C++ <c>dividir</c>.
    /// </summary>
    /// <exception cref="DivideByZeroException">Thrown when <paramref name="right"/> is zero.</exception>
    public static Natural DivRem(Natural left, Natural right, out Natural remainder)
    {
        if (IsZero(right))
            throw new DivideByZeroException("Cannot divide by zero.");

        if (IsZero(left))
        {
            remainder = new Natural();
            return new Natural();
        }

        if (left == right)
        {
            remainder = new Natural();
            return One;
        }

        if (left < right)
        {
            remainder = new Natural(left);
            return new Natural();
        }

        // Long division: process dividend digits from MSD to LSD.
        // quotientDigits[0] = most-significant quotient digit.
        long n = left._store.DigitCount;
        var quotientDigits = new byte[n];
        var partial = new Natural();

        for (long i = n - 1; i >= 0; i--)
        {
            // Bring down the next digit of the dividend (partial = partial * 10 + d).
            partial = BringDownDigit(partial, left._store.GetDigit(i));

            // Trial division: find q ∈ [0..9] such that q*right <= partial < (q+1)*right.
            byte q = 0;
            var qTimesDivisor = new Natural();
            for (byte k = 1; k <= 9; k++)
            {
                var candidate = right * new Natural((ulong)k);
                if (candidate <= partial)
                {
                    q = k;
                    qTimesDivisor = candidate;
                }
                else
                    break;
            }

            partial = partial - qTimesDivisor;
            quotientDigits[n - 1 - i] = q; // index 0 = MSD
        }

        // Strip leading zeros from quotientDigits.
        int start = 0;
        while (start < quotientDigits.Length - 1 && quotientDigits[start] == 0)
            start++;

        int qLen = quotientDigits.Length - start;
        var quotient = new Natural();
        // Write digits from LSD (position 0) to MSD (position qLen-1).
        for (int j = 0; j < qLen; j++)
            quotient._store.SetDigit(j, quotientDigits[start + qLen - 1 - j]);

        // If all quotient digits were zero (shouldn't reach here, but guard)
        if (qLen == 1 && quotientDigits[start] == 0)
            quotient = new Natural();

        quotient._store.TrimLeadingZeros();

        remainder = partial;
        return quotient;
    }

    /// <summary>
    /// Convenience instance overload: divides this by <paramref name="divisor"/>.
    /// </summary>
    /// <exception cref="DivideByZeroException">Thrown when <paramref name="divisor"/> is zero.</exception>
    public Natural DivRem(Natural divisor, out Natural remainder) => DivRem(this, divisor, out remainder);

    /// <summary>
    /// Returns <paramref name="partial"/> * 10 + <paramref name="digit"/>.
    /// Used internally by the long-division algorithm to bring down one dividend digit.
    /// </summary>
    private static Natural BringDownDigit(Natural partial, byte digit)
    {
        if (IsZero(partial) && digit == 0)
            return new Natural();

        var result = new Natural();
        // Write digit at position 0 (ones place = LSD of the new value).
        result._store.SetDigit(0, digit);
        // Write the old partial digits shifted up by one decimal place.
        long n = IsZero(partial) ? 0 : partial._store.DigitCount;
        for (long i = 0; i < n; i++)
            result._store.SetDigit(i + 1, partial._store.GetDigit(i));

        return result;
    }

    /// <summary>
    /// Raises this instance to the power of <paramref name="exponent"/>.
    /// Mirrors C++ <c>exponenciar</c>: initialises result to 1 and multiplies by
    /// <c>this</c> once for every unit of <paramref name="exponent"/>.
    /// Any base raised to the power of zero returns 1.
    /// </summary>
    public Natural Pow(Natural exponent)
    {
        var result = One;
        if (!IsZero(exponent))
            for (var c = new Natural(); c < exponent; c++)
                result *= this;
        return result;
    }

    /// <summary>
    /// Returns the factorial of this instance (this!).
    /// Mirrors C++ <c>fatorial</c>: returns 1 for 0! and 1!, then accumulates
    /// the product 2 × 3 × … × this for larger values.
    /// </summary>
    public Natural Factorial()
    {
        var result = One;
        if (!IsZero(this))
            for (var aux = new Natural(2UL); aux <= this; aux++)
                result *= aux;
        return result;
    }

    // -------------------------------------------------------------------------
    // ISpanFormattable / IFormattable / ToString
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public override string ToString() => _store.ToString();

    /// <inheritdoc/>
    /// <remarks>
    /// Supports format specifier <c>"N"</c> or <c>"n"</c> to insert a comma thousands
    /// separator every three digits (mirrors C++ <c>imprimir(char separador)</c>).
    /// All other format strings fall back to the plain decimal representation.
    /// </remarks>
    public string ToString(string? format, IFormatProvider? formatProvider)
    {
        if (string.Equals(format, "N", StringComparison.OrdinalIgnoreCase))
            return _store.ToString(',');
        return _store.ToString();
    }

    /// <inheritdoc/>
    public bool TryFormat(Span<char> destination, out int charsWritten, ReadOnlySpan<char> format, IFormatProvider? provider)
    {
        string s = format.IsEmpty
            ? _store.ToString()
            : ToString(format.ToString(), provider);

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
    /// <remarks>
    /// Accepts a non-empty span of decimal digit characters ('0'–'9').
    /// Leading zeros are silently trimmed; a span of all zeros produces zero.
    /// Returns <see langword="false"/> for empty spans, <see langword="null"/> spans,
    /// or spans containing any non-digit character.
    /// Mirrors C++ <c>operator&gt;&gt;</c>.
    /// </remarks>
    public static bool TryParse(ReadOnlySpan<char> s, IFormatProvider? provider, [MaybeNullWhen(false)] out Natural result)
    {
        result = null;

        if (s.IsEmpty) return false;

        // Validate: every character must be a decimal digit.
        foreach (char ch in s)
        {
            if (ch < '0' || ch > '9') return false;
        }

        // Skip leading zeros.
        int start = 0;
        while (start < s.Length - 1 && s[start] == '0')
            start++;

        ReadOnlySpan<char> digits = s[start..];

        // A single '0' (or all-zero input) → zero.
        if (digits.Length == 1 && digits[0] == '0')
        {
            result = new Natural();
            return true;
        }

        // Build the Natural: position 0 is the LSD (rightmost character).
        var n = new Natural();
        int len = digits.Length;
        for (int i = 0; i < len; i++)
        {
            // digits[len - 1 - i] is the digit at decimal position i (0 = LSD).
            n._store.SetDigit(i, (byte)(digits[len - 1 - i] - '0'));
        }

        result = n;
        return true;
    }

    // Convenience overloads without IFormatProvider (used by tests / callers)
    /// <summary>Parses a decimal string into a <see cref="Natural"/>.</summary>
    /// <exception cref="FormatException">Thrown for invalid input.</exception>
    public static Natural Parse(string s) => Parse(s, null);

    /// <summary>Attempts to parse a decimal string into a <see cref="Natural"/>.</summary>
    public static bool TryParse([NotNullWhen(true)] string? s, [MaybeNullWhen(false)] out Natural result)
        => TryParse(s, null, out result);

    // -------------------------------------------------------------------------
    // INumberBase<Natural> — numeric style parse overloads
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static Natural Parse(string s, NumberStyles style, IFormatProvider? provider)
        => Parse(s, provider);

    /// <inheritdoc/>
    public static Natural Parse(ReadOnlySpan<char> s, NumberStyles style, IFormatProvider? provider)
        => Parse(s, provider);

    /// <inheritdoc/>
    public static bool TryParse([NotNullWhen(true)] string? s, NumberStyles style, IFormatProvider? provider, [MaybeNullWhen(false)] out Natural result)
        => TryParse(s, provider, out result);

    /// <inheritdoc/>
    public static bool TryParse(ReadOnlySpan<char> s, NumberStyles style, IFormatProvider? provider, [MaybeNullWhen(false)] out Natural result)
        => TryParse(s, provider, out result);

    // -------------------------------------------------------------------------
    // INumberBase<Natural> — generic conversion helpers
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    static bool INumberBase<Natural>.TryConvertFromChecked<TOther>(TOther value, [MaybeNullWhen(false)] out Natural result)
        => throw new NotImplementedException();

    /// <inheritdoc/>
    static bool INumberBase<Natural>.TryConvertFromSaturating<TOther>(TOther value, [MaybeNullWhen(false)] out Natural result)
        => throw new NotImplementedException();

    /// <inheritdoc/>
    static bool INumberBase<Natural>.TryConvertFromTruncating<TOther>(TOther value, [MaybeNullWhen(false)] out Natural result)
        => throw new NotImplementedException();

    /// <inheritdoc/>
    static bool INumberBase<Natural>.TryConvertToChecked<TOther>(Natural value, [MaybeNullWhen(false)] out TOther result)
        => throw new NotImplementedException();

    /// <inheritdoc/>
    static bool INumberBase<Natural>.TryConvertToSaturating<TOther>(Natural value, [MaybeNullWhen(false)] out TOther result)
        => throw new NotImplementedException();

    /// <inheritdoc/>
    static bool INumberBase<Natural>.TryConvertToTruncating<TOther>(Natural value, [MaybeNullWhen(false)] out TOther result)
        => throw new NotImplementedException();
}
