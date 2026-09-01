using System.Buffers;
using System.Diagnostics.CodeAnalysis;
using System.Globalization;
using System.Numerics;
using System.Threading;
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

    // Backing fields for Interlocked-safe 64-bit access.
    // Plain auto-properties are not guaranteed atomic on 32-bit runtimes.
    private static long _displayDigits = -1L;
    private static long _precision     = -1L;

    /// <summary>
    /// Maximum number of digits to display when formatting.
    /// -1 means "no limit" (display all digits). Matches C++ <c>algarismosExibicao</c>.
    /// Reads and writes are atomic on both 32-bit and 64-bit runtimes via
    /// <see cref="Interlocked.Read(ref long)"/> and
    /// <see cref="Interlocked.Exchange(ref long, long)"/>.
    /// </summary>
    public static long DisplayDigits
    {
        get => Interlocked.Read(ref _displayDigits);
        set => Interlocked.Exchange(ref _displayDigits, value);
    }

    /// <summary>
    /// Precision hint. Stub — C++ body was absent. Matches C++ <c>Precisao</c>.
    /// Reads and writes are atomic on both 32-bit and 64-bit runtimes via
    /// <see cref="Interlocked.Read(ref long)"/> and
    /// <see cref="Interlocked.Exchange(ref long, long)"/>.
    /// </summary>
    public static long Precision
    {
        get => Interlocked.Read(ref _precision);
        set => Interlocked.Exchange(ref _precision, value);
    }

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

        // Snapshot both operands so the digit loop runs on plain byte[] with no
        // per-digit Monitor acquisition, then bulk-write the result.
        var (da, la) = left._store.RentDigitSnapshot();
        var (db, lb) = right._store.RentDigitSnapshot();
        try
        {
            int n = Math.Max(la, lb);
            byte[] result = ArrayPool<byte>.Shared.Rent(n + 1);
            try
            {
                int carry = 0;
                int i = 0;
                for (; i < n; i++)
                {
                    int av = i < la ? da[i] : 0;
                    int bv = i < lb ? db[i] : 0;
                    int s = av + bv + carry;
                    result[i] = (byte)(s % 10);
                    carry = s / 10;
                }
                int len = n;
                if (carry > 0)
                {
                    result[i] = (byte)carry;
                    len = n + 1;
                }
                var res = new Natural();
                res._store.SetDigitsBulk(new ReadOnlySpan<byte>(result, 0, len));
                return res;
            }
            finally
            {
                ArrayPool<byte>.Shared.Return(result);
            }
        }
        finally
        {
            left._store.ReturnDigitSnapshot(da);
            right._store.ReturnDigitSnapshot(db);
        }
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

        // Snapshot both operands so the borrow loop runs on plain byte[] with no
        // per-digit Monitor acquisition, then bulk-write the result.
        var (da, la) = left._store.RentDigitSnapshot();
        var (db, lb) = right._store.RentDigitSnapshot();
        try
        {
            byte[] result = ArrayPool<byte>.Shared.Rent(la);
            try
            {
                int borrow = 0;
                for (int i = 0; i < la; i++)
                {
                    int bv = i < lb ? db[i] : 0;
                    int diff = da[i] - bv - borrow;
                    if (diff < 0)
                    {
                        diff += 10;
                        borrow = 1;
                    }
                    else
                    {
                        borrow = 0;
                    }
                    result[i] = (byte)diff;
                }

                var res = new Natural();
                res._store.SetDigitsBulk(new ReadOnlySpan<byte>(result, 0, la));
                return res;
            }
            finally
            {
                ArrayPool<byte>.Shared.Return(result);
            }
        }
        finally
        {
            left._store.ReturnDigitSnapshot(da);
            right._store.ReturnDigitSnapshot(db);
        }
    }

    /// <inheritdoc/>
    /// <remarks>
    /// Long (grade-school) multiplication with direct in-place accumulation.
    /// The smaller operand (fewer digits) drives the outer loop; each of its digits
    /// is multiplied against every digit of the larger operand and accumulated
    /// straight into a single result digit buffer. No intermediate <c>Natural</c>
    /// per partial product and no repeated <c>operator+</c> — mathematically
    /// identical to the previous algorithm, so exactness is unchanged.
    /// </remarks>
    public static Natural operator *(Natural left, Natural right)
    {
        // Absorbing element: 0 × anything = 0.
        if (IsZero(left) || IsZero(right)) return new Natural();

        // Put the smaller operand (fewer digits) first to keep the recursion
        // balanced; MultiplyDigits dispatches to schoolbook or Karatsuba by size.
        bool leftIsLarger = left._store.DigitCount > right._store.DigitCount;
        Natural small = leftIsLarger ? right : left;
        Natural large = leftIsLarger ? left  : right;

        var (ds, smallLen) = small._store.RentDigitSnapshot();
        var (dl, largeLen) = large._store.RentDigitSnapshot();
        try
        {
            byte[] result = MultiplyDigits(ds, smallLen, dl, largeLen, out int resultLen);

            var res = new Natural();
            res._store.SetDigitsBulk(new ReadOnlySpan<byte>(result, 0, resultLen));
            return res;
        }
        finally
        {
            small._store.ReturnDigitSnapshot(ds);
            large._store.ReturnDigitSnapshot(dl);
        }
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

        // Fast path: recursive-Newton quotient. Exact after correction, and O(M(n))
        // for large operands regardless of quotient size.
        if (left._store.DigitCount >= FastDivThreshold && right._store.DigitCount >= FastDivThreshold)
            return DivRemFast(left, right, out remainder);

        // Long division, rewritten to run on pooled plain digit arrays instead of
        // repeatedly reading/writing the DigitStore through locked GetDigit/SetDigit.
        // A single leading-digit estimate selects each quotient digit; an exact
        // correction loop then guarantees q = floor(partial / divisor) before the
        // remainder is reduced — mathematically identical to the previous trial
        // loop, so exactness is unchanged.
        var (dA, n) = left._store.RentDigitSnapshot();   // dividend, LSD at 0
        var (dB, m) = right._store.RentDigitSnapshot();  // divisor,  LSD at 0
        try
        {
            var partial  = ArrayPool<byte>.Shared.Rent(m + 2);
            var qTimes   = ArrayPool<byte>.Shared.Rent(m + 2);
            var quotient = ArrayPool<byte>.Shared.Rent(n);
            try
            {
                Array.Clear(partial, 0, m + 2);
                Array.Clear(quotient, 0, n);

                int partialLen = 0; // significant digit count of partial (0 = zero)

                for (int i = n - 1; i >= 0; i--)
                {
                    // Bring down: partial = partial * 10 + dA[i].
                    for (int p = partialLen - 1; p >= 0; p--)
                        partial[p + 1] = partial[p];
                    partial[0] = dA[i];
                    if (partialLen > 0 || dA[i] != 0)
                        partialLen++;

                    int q = EstimateQuotientDigit(partial, partialLen, dB, m);

                    if (q > 0)
                    {
                        MultiplySingleDigit(dB, m, q, qTimes, out int qTimesLen);

                        // Correct downward while the estimate overshoots.
                        while (CompareDigits(qTimes, qTimesLen, partial, partialLen) > 0)
                        {
                            q--;
                            SubtractInPlace(qTimes, ref qTimesLen, dB, m);
                        }

                        // Correct upward while the estimate undershoots.
                        while (q < 9)
                        {
                            AddInPlace(qTimes, ref qTimesLen, dB, m);
                            if (CompareDigits(qTimes, qTimesLen, partial, partialLen) <= 0)
                                q++;
                            else
                            {
                                SubtractInPlace(qTimes, ref qTimesLen, dB, m);
                                break;
                            }
                        }

                        SubtractInPlace(partial, ref partialLen, qTimes, qTimesLen);
                    }

                    quotient[i] = (byte)q;
                }

                var quo = new Natural();
                quo._store.SetDigitsBulk(new ReadOnlySpan<byte>(quotient, 0, n));
                var rem = new Natural();
                rem._store.SetDigitsBulk(new ReadOnlySpan<byte>(partial, 0, partialLen));
                remainder = rem;
                return quo;
            }
            finally
            {
                ArrayPool<byte>.Shared.Return(partial);
                ArrayPool<byte>.Shared.Return(qTimes);
                ArrayPool<byte>.Shared.Return(quotient);
            }
        }
        finally
        {
            left._store.ReturnDigitSnapshot(dA);
            right._store.ReturnDigitSnapshot(dB);
        }
    }

    /// <summary>
    /// Exact quotient/remainder for large operands via the Newton-reciprocal digit-array
    /// path (<see cref="DivRemDigits"/>). Caller guarantees <c>left ≥ right</c> and both
    /// digit counts exceed <see cref="FastDivThreshold"/>.
    /// </summary>
    private static Natural DivRemFast(Natural left, Natural right, out Natural remainder)
    {
        var (n, nLen) = left._store.RentDigitSnapshot();
        var (d, dLen) = right._store.RentDigitSnapshot();
        try
        {
            byte[] q = DivRemDigits(n, nLen, d, dLen, out byte[] r, out int rLen, out int qLen);

            var quo = new Natural();
            quo._store.SetDigitsBulk(new ReadOnlySpan<byte>(q, 0, qLen));
            var rem = new Natural();
            rem._store.SetDigitsBulk(new ReadOnlySpan<byte>(r, 0, rLen));
            remainder = rem;
            return quo;
        }
        finally
        {
            left._store.ReturnDigitSnapshot(n);
            right._store.ReturnDigitSnapshot(d);
        }
    }

    /// <summary>
    /// Returns this value × 10^k (appends <paramref name="k"/> zero digits) without a
    /// full multiplication — a plain left-shift of the decimal digits.
    /// </summary>
    public Natural ShiftLeftDecimal(long k)
    {
        if (IsZero(this) || k <= 0)
            return new Natural(this);

        var (buf, len) = _store.RentDigitSnapshot();
        try
        {
            int newLen = len + (int)k;
            byte[] shifted = new byte[newLen];
            Array.Copy(buf, 0, shifted, (int)k, len);

            var res = new Natural();
            res._store.SetDigitsBulk(new ReadOnlySpan<byte>(shifted, 0, newLen));
            return res;
        }
        finally
        {
            _store.ReturnDigitSnapshot(buf);
        }
    }

    /// <summary>
    /// Convenience instance overload: divides this by <paramref name="divisor"/>.
    /// </summary>
    /// <exception cref="DivideByZeroException">Thrown when <paramref name="divisor"/> is zero.</exception>
    public Natural DivRem(Natural divisor, out Natural remainder) => DivRem(this, divisor, out remainder);

    /// <summary>
    /// Estimates a single base-10 quotient digit from the two most-significant digits
    /// of <paramref name="partial"/> and the one or two most-significant digits of the
    /// divisor. The estimate may be wrong; the caller corrects it with exact
    /// comparison. When <paramref name="partial"/> has fewer digits than the divisor
    /// it must be smaller, so the digit is 0.
    /// </summary>
    private static int EstimateQuotientDigit(byte[] partial, int partialLen, byte[] divisor, int mLen)
    {
        if (partialLen < mLen)
            return 0;

        int u = partialLen >= 2
            ? partial[partialLen - 1] * 10 + partial[partialLen - 2]
            : partial[partialLen - 1];

        int v = mLen >= 2
            ? divisor[mLen - 1] * 10 + divisor[mLen - 2]
            : divisor[mLen - 1];

        // When partial has one more digit than the divisor, the divisor's leading
        // TWO digits are too fine a granularity; use only its leading digit so the
        // estimate stays in the right decade.
        int qhat = (partialLen > mLen) ? u / divisor[mLen - 1] : u / v;
        return qhat > 9 ? 9 : qhat;
    }

    /// <summary>
    /// Multiplies the LSD-first digit array <paramref name="src"/> (length
    /// <paramref name="srcLen"/>) by a single digit <paramref name="q"/> (1–9),
    /// writing the result LSD-first into <paramref name="dst"/>. The result length
    /// is reported via <paramref name="dstLen"/>.
    /// </summary>
    private static void MultiplySingleDigit(byte[] src, int srcLen, int q, byte[] dst, out int dstLen)
    {
        int carry = 0;
        int i = 0;
        for (; i < srcLen; i++)
        {
            int p = src[i] * q + carry;
            dst[i] = (byte)(p % 10);
            carry = p / 10;
        }
        if (carry > 0)
        {
            dst[i] = (byte)carry;
            dstLen = srcLen + 1;
        }
        else
        {
            dstLen = srcLen;
        }
    }

    /// <summary>
    /// Compares two LSD-first digit arrays by value. Lengths must already be
    /// canonical (no leading zeros); a length of zero denotes the value zero.
    /// </summary>
    private static int CompareDigits(byte[] a, int aLen, byte[] b, int bLen)
    {
        if (aLen != bLen)
            return aLen.CompareTo(bLen);
        for (int i = aLen - 1; i >= 0; i--)
        {
            if (a[i] != b[i])
                return a[i].CompareTo(b[i]);
        }
        return 0;
    }

    /// <summary>
    /// Subtracts <paramref name="b"/> (length <paramref name="bLen"/>) from
    /// <paramref name="a"/> in place. Caller guarantees <c>a &gt;= b</c>; the
    /// updated significant length is written back through <paramref name="aLen"/>
    /// (leading zeros are trimmed).
    /// </summary>
    private static void SubtractInPlace(byte[] a, ref int aLen, byte[] b, int bLen)
    {
        int borrow = 0;
        for (int i = 0; i < bLen; i++)
        {
            int diff = a[i] - b[i] - borrow;
            if (diff < 0)
            {
                diff += 10;
                borrow = 1;
            }
            else
            {
                borrow = 0;
            }
            a[i] = (byte)diff;
        }
        for (int i = bLen; i < aLen && borrow > 0; i++)
        {
            int diff = a[i] - borrow;
            if (diff < 0)
            {
                diff += 10;
                borrow = 1;
            }
            else
            {
                borrow = 0;
            }
            a[i] = (byte)diff;
        }

        // Trim leading zeros introduced by the subtraction.
        while (aLen > 1 && a[aLen - 1] == 0)
            aLen--;
        if (aLen == 1 && a[0] == 0)
            aLen = 0;
    }

    /// <summary>
    /// Adds <paramref name="b"/> (length <paramref name="bLen"/>) to
    /// <paramref name="a"/> in place, propagating any final carry into a new
    /// most-significant digit when needed.
    /// </summary>
    private static void AddInPlace(byte[] a, ref int aLen, byte[] b, int bLen)
    {
        int carry = 0;
        int maxLen = aLen > bLen ? aLen : bLen;
        for (int i = 0; i < maxLen; i++)
        {
            int av = i < aLen ? a[i] : 0;
            int bv = i < bLen ? b[i] : 0;
            int s = av + bv + carry;
            a[i] = (byte)(s % 10);
            carry = s / 10;
        }
        aLen = maxLen;
        if (carry > 0)
        {
            a[aLen] = (byte)carry;
            aLen++;
        }
    }

    // -------------------------------------------------------------------------
    // Fast multiplication — Karatsuba over base-10 digit arrays
    // -------------------------------------------------------------------------

    /// <summary>
    /// Operands with this many digits or fewer use the grade-school product;
    /// larger operands recurse via Karatsuba.
    /// </summary>
    private const int KaratsubaThreshold = 1024;

    /// <summary>
    /// Multiplies two canonical LSD-first decimal-digit arrays and returns the
    /// canonical (trimmed) product. <paramref name="resultLen"/> receives the
    /// number of significant digits (0 for a zero product).
    /// </summary>
    private static byte[] MultiplyDigits(byte[] a, int aLen, byte[] b, int bLen, out int resultLen)
    {
        if (aLen == 0 || bLen == 0)
        {
            resultLen = 0;
            return Array.Empty<byte>();
        }

        // Number-Theoretic Transform (exact) for very large operands.
        if (aLen >= NttThreshold && bLen >= NttThreshold && (long)aLen + bLen <= (long)MaxNttLength * NttLimbDigits)
            return NttMultiply(a, aLen, b, bLen, out resultLen);

        if (aLen <= KaratsubaThreshold || bLen <= KaratsubaThreshold)
            return SchoolbookMultiply(a, aLen, b, bLen, out resultLen);

        // Split at m = ceil(maxLen/2): a = a0 + a1·10^m, b = b0 + b1·10^m.
        int m = (Math.Max(aLen, bLen) + 1) / 2;

        int a0Len = Math.Min(aLen, m);
        int a1Len = aLen - a0Len;
        int b0Len = Math.Min(bLen, m);
        int b1Len = bLen - b0Len;

        byte[] a1 = SliceDigits(a, a0Len, a1Len);
        byte[] b1 = SliceDigits(b, b0Len, b1Len);

        // z0 = a0·b0 (low), z2 = a1·b1 (high).
        byte[] z0 = MultiplyDigits(a, a0Len, b, b0Len, out int z0Len);
        byte[] z2 = MultiplyDigits(a1, a1Len, b1, b1Len, out int z2Len);

        // z1 = (a0+a1)(b0+b1) − z0 − z2.
        byte[] sumA = AddDigits(a, a0Len, a1, a1Len, out int sumALen);
        byte[] sumB = AddDigits(b, b0Len, b1, b1Len, out int sumBLen);
        byte[] z1 = MultiplyDigits(sumA, sumALen, sumB, sumBLen, out int z1Len);
        z1 = SubtractDigits(z1, z1Len, z0, z0Len, out z1Len);
        z1 = SubtractDigits(z1, z1Len, z2, z2Len, out z1Len);

        // result = z0 + z1·10^m + z2·10^(2m).
        byte[] s1 = ShiftLeftDigits(z1, z1Len, m, out int s1Len);
        byte[] s2 = ShiftLeftDigits(z2, z2Len, 2 * m, out int s2Len);
        byte[] acc = AddDigits(z0, z0Len, s1, s1Len, out int accLen);
        acc = AddDigits(acc, accLen, s2, s2Len, out accLen);

        resultLen = accLen;
        return acc;
    }

    /// <summary>
    /// Grade-school product of two canonical LSD-first digit arrays. Returns a
    /// canonical (trimmed) result.
    /// </summary>
    private static byte[] SchoolbookMultiply(byte[] a, int aLen, byte[] b, int bLen, out int resultLen)
    {
        byte[] result = new byte[aLen + bLen + 1];
        for (int i = 0; i < aLen; i++)
        {
            int mult = a[i];
            if (mult == 0) continue;

            int carry = 0;
            for (int j = 0; j < bLen; j++)
            {
                int t = result[i + j] + mult * b[j] + carry;
                result[i + j] = (byte)(t % 10);
                carry = t / 10;
            }
            int idx = i + bLen;
            while (carry > 0)
            {
                int t = result[idx] + carry;
                result[idx] = (byte)(t % 10);
                carry = t / 10;
                idx++;
            }
        }

        resultLen = aLen + bLen + 1;
        while (resultLen > 0 && result[resultLen - 1] == 0)
            resultLen--;
        return result;
    }

    /// <summary>
    /// Adds two canonical LSD-first digit arrays; returns a canonical sum.
    /// </summary>
    private static byte[] AddDigits(byte[] a, int aLen, byte[] b, int bLen, out int resultLen)
    {
        if (aLen == 0) { resultLen = bLen; return CopyDigits(b, bLen); }
        if (bLen == 0) { resultLen = aLen; return CopyDigits(a, aLen); }

        int maxLen = Math.Max(aLen, bLen);
        byte[] result = new byte[maxLen + 1];
        int carry = 0;
        for (int i = 0; i < maxLen; i++)
        {
            int s = (i < aLen ? a[i] : 0) + (i < bLen ? b[i] : 0) + carry;
            result[i] = (byte)(s % 10);
            carry = s / 10;
        }

        resultLen = maxLen;
        if (carry > 0)
        {
            result[maxLen] = (byte)carry;
            resultLen = maxLen + 1;
        }
        return result;
    }

    /// <summary>
    /// Subtracts <paramref name="b"/> from <paramref name="a"/> (a ≥ b); returns a
    /// canonical difference.
    /// </summary>
    private static byte[] SubtractDigits(byte[] a, int aLen, byte[] b, int bLen, out int resultLen)
    {
        if (bLen == 0) { resultLen = aLen; return CopyDigits(a, aLen); }

        byte[] result = new byte[aLen];
        int borrow = 0;
        for (int i = 0; i < aLen; i++)
        {
            int bv = i < bLen ? b[i] : 0;
            int d = a[i] - bv - borrow;
            if (d < 0) { d += 10; borrow = 1; } else { borrow = 0; }
            result[i] = (byte)d;
        }

        resultLen = aLen;
        while (resultLen > 0 && result[resultLen - 1] == 0)
            resultLen--;
        return result;
    }

    /// <summary>
    /// Left-shifts a canonical LSD-first digit array by <paramref name="k"/> decimal
    /// places (× 10^k), returning the canonical result.
    /// </summary>
    private static byte[] ShiftLeftDigits(byte[] a, int aLen, int k, out int resultLen)
    {
        if (aLen == 0 || k == 0) { resultLen = aLen; return CopyDigits(a, aLen); }

        byte[] result = new byte[aLen + k];
        Array.Copy(a, 0, result, k, aLen);
        resultLen = aLen + k;
        return result;
    }

    /// <summary>
    /// Copies <paramref name="len"/> digits from <paramref name="a"/> into a fresh
    /// array.
    /// </summary>
    private static byte[] CopyDigits(byte[] a, int len)
    {
        byte[] result = new byte[len];
        Array.Copy(a, 0, result, 0, len);
        return result;
    }

    /// <summary>
    /// Copies <paramref name="len"/> digits starting at <paramref name="start"/> into
    /// a fresh array.
    /// </summary>
    private static byte[] SliceDigits(byte[] a, int start, int len)
    {
        byte[] result = new byte[len];
        Array.Copy(a, start, result, 0, len);
        return result;
    }

    // -------------------------------------------------------------------------
    // Fast multiplication — Number-Theoretic Transform (exact convolution)
    // -------------------------------------------------------------------------

    /// <summary>998244353 = 119·2^23 + 1 — first NTT prime, supports lengths up to 2^23, primitive root 3.</summary>
    private const long NttPrime1 = 998244353L;

    /// <summary>469762049 = 7·2^26 + 1 — second NTT prime (CRT), supports lengths up to 2^26, primitive root 3.</summary>
    private const long NttPrime2 = 469762049L;

    /// <summary>Primitive root shared by both NTT primes.</summary>
    private const long NttRoot = 3L;

    /// <summary>Largest NTT length (a power of two) any prime supports; bounded by <see cref="NttPrime1"/> at 2^23.</summary>
    private const int MaxNttLength = 1 << 23;

    /// <summary>
    /// Decimal digits packed into each NTT limb. Grouping base-10 digits into
    /// base-10^5 limbs shrinks the transform length ~5× versus the old
    /// one-digit-per-coefficient scheme, at the cost of a second prime for the CRT.
    /// </summary>
    private const int NttLimbDigits = 5;

    /// <summary>10^<see cref="NttLimbDigits"/> — the limb radix.</summary>
    private const long NttLimbBase = 100000L;

    /// <summary>
    /// Operands with at least this many digits each use the NTT product instead of
    /// schoolbook multiplication. The packed NTT beats the former Karatsuba path
    /// across the board, so Karatsuba is superseded (its branch is unreachable when
    /// this value is ≤ <see cref="KaratsubaThreshold"/>).
    /// </summary>
    private const int NttThreshold = 256;

    /// <summary>
    /// Modular exponentiation (base^exp mod mod) for NTT roots and inverse lengths.
    /// </summary>
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

    /// <summary>Multiplicative inverse of <see cref="NttPrime1"/> modulo <see cref="NttPrime2"/> (CRT coefficient).</summary>
    private static readonly long InvP1ModP2 = ModPow(NttPrime1 % NttPrime2, NttPrime2 - 2, NttPrime2);

    /// <summary>
    /// In-place iterative Cooley–Tukey Number-Theoretic Transform modulo
    /// <paramref name="prime"/>. <paramref name="n"/> must be a power of two dividing
    /// <c>prime − 1</c> (both primes support n ≤ 2^23).
    /// </summary>
    private static void Ntt(long[] a, int n, bool invert, long prime)
    {
        // Bit-reversal permutation.
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

    /// <summary>
    /// Packs <paramref name="len"/> base-10 digits (LSD-first) into base-10^5 limbs,
    /// written into <paramref name="limbs"/> (already zero-initialised to the transform
    /// size; entries beyond the packed count stay zero).
    /// </summary>
    private static void PackLimbs(byte[] digits, int len, long[] limbs)
    {
        for (int i = 0; i < len; i += NttLimbDigits)
        {
            long limb = 0;
            long place = 1;
            int end = Math.Min(len, i + NttLimbDigits);
            for (int t = i; t < end; t++)
            {
                limb += digits[t] * place;
                place *= 10;
            }
            limbs[i / NttLimbDigits] = limb;
        }
    }

    /// <summary>
    /// Reconstructs the exact non-negative convolution coefficient from its residues
    /// modulo the two NTT primes. Because the coefficient is strictly less than the
    /// product of the primes (see the bound in <see cref="NttMultiply"/>), Garner's
    /// two-term CRT recovers it exactly with 64-bit arithmetic.
    /// </summary>
    private static ulong Crt(long r1, long r2)
    {
        long diff = (r2 - r1) % NttPrime2;
        if (diff < 0) diff += NttPrime2;
        long t = diff * InvP1ModP2 % NttPrime2;
        return (ulong)(r1 + t * NttPrime1);
    }

    /// <summary>
    /// Multiplies two base-10 digit arrays via a packed two-prime NTT: the digits are
    /// grouped into base-10^5 limbs, convolved exactly modulo each prime, reconstructed
    /// with the CRT, then carry-propagated back into base-10 digits.
    /// <para>
    /// Exactness bound: a convolution coefficient is at most
    /// <c>(10^5 − 1)² · 2^22 ≈ 4.2×10^16</c>, strictly below the prime product
    /// <c>NttPrime1 · NttPrime2 ≈ 4.7×10^17</c>, so the two residues determine it
    /// uniquely and every intermediate value fits in 64 bits.
    /// </para>
    /// </summary>
    private static byte[] NttMultiply(byte[] a, int aLen, byte[] b, int bLen, out int resultLen)
    {
        int aLimbs = (aLen + NttLimbDigits - 1) / NttLimbDigits;
        int bLimbs = (bLen + NttLimbDigits - 1) / NttLimbDigits;
        int need = aLimbs + bLimbs;
        int size = 1;
        while (size < need)
            size <<= 1;

        // Pack both operands into limb arrays padded to the transform size.
        long[] fa = new long[size];
        long[] fb = new long[size];
        PackLimbs(a, aLen, fa);
        PackLimbs(b, bLen, fb);

        // Convolve modulo the first prime (fa is overwritten in place, so keep a copy
        // of the packed 'a' limbs for the second prime).
        long[] fa2 = (long[])fa.Clone();
        Ntt(fa, size, false, NttPrime1);
        Ntt(fb, size, false, NttPrime1);
        for (int i = 0; i < size; i++)
            fa[i] = fa[i] * fb[i] % NttPrime1;
        Ntt(fa, size, true, NttPrime1);

        // Convolve modulo the second prime (fb was clobbered, so re-pack 'b').
        Array.Clear(fb, 0, size);
        PackLimbs(b, bLen, fb);
        Ntt(fa2, size, false, NttPrime2);
        Ntt(fb, size, false, NttPrime2);
        for (int i = 0; i < size; i++)
            fa2[i] = fa2[i] * fb[i] % NttPrime2;
        Ntt(fa2, size, true, NttPrime2);

        // CRT + carry propagation into base-10 digits (LSD-first).
        byte[] result = new byte[need * NttLimbDigits];
        ulong carry = 0;
        for (int i = 0; i < need; i++)
        {
            ulong val = Crt(fa[i], fa2[i]) + carry;
            ulong limb = val % (ulong)NttLimbBase;
            carry = val / (ulong)NttLimbBase;
            int at = i * NttLimbDigits;
            for (int t = 0; t < NttLimbDigits; t++)
            {
                result[at + t] = (byte)(limb % 10);
                limb /= 10;
            }
        }

        resultLen = need * NttLimbDigits;
        while (resultLen > 0 && result[resultLen - 1] == 0)
            resultLen--;
        return result;
    }

    // -------------------------------------------------------------------------
    // Fast division — Newton reciprocal over base-10 digit arrays
    // -------------------------------------------------------------------------

    /// <summary>
    /// Dividends and divisors with at least this many digits use the Newton-reciprocal
    /// quotient path; smaller ones use the exact grade-school long division.
    /// </summary>
    private const int FastDivThreshold = 1024;

    /// <summary>
    /// Reciprocal results with at most this many digits use the direct Newton loop
    /// (<see cref="ReciprocalFloorBase"/>); larger ones recurse to halve the precision.
    /// </summary>
    private const int ReciprocalBaseThreshold = 512;

    /// <summary>
    /// Computes the quotient and remainder of <paramref name="n"/> ÷ <paramref name="d"/>
    /// (both canonical LSD-first digit arrays) via Newton reciprocal. The caller
    /// guarantees <c>nLen ≥ dLen</c>, <c>d ≠ 0</c>, and both lengths exceed
    /// <see cref="FastDivThreshold"/>.
    /// </summary>
    private static byte[] DivRemDigits(byte[] n, int nLen, byte[] d, int dLen,
                                       out byte[] rem, out int remLen, out int qLen)
    {
        // R = floor(10^p / d) with p = nLen + 1 (one guard digit).
        int p = nLen + 1;
        byte[] r = ReciprocalFloor(d, dLen, p, out int rLen);

        // Q ≈ floor(n·R / 10^p) = floor(n/d), possibly one too low.
        byte[] nR = MultiplyDigits(n, nLen, r, rLen, out int nRLen);
        byte[] q = ShiftRightDigits(nR, nRLen, p, out qLen);

        // remainder = n − q·d, then correct upward while remainder ≥ d.
        byte[] qd = MultiplyDigits(q, qLen, d, dLen, out int qdLen);
        rem = SubtractDigits(n, nLen, qd, qdLen, out remLen);
        while (CompareDigits(rem, remLen, d, dLen) >= 0)
        {
            rem = SubtractDigits(rem, remLen, d, dLen, out remLen);
            q = AddOneDigits(q, qLen, out qLen);
        }

        return q;
    }

    /// <summary>
    /// Returns <c>floor(10^precision / d)</c> as a canonical LSD-first digit array via
    /// recursive Newton doubling. The result has <c>precision − dLen + 1</c> significant
    /// digits; each recursion level computes a half-precision reciprocal and refines it
    /// with a single Newton step, so the working size stays proportional to the result
    /// (no scale overshoot). Small results fall through to
    /// <see cref="ReciprocalFloorBase"/>.
    /// </summary>
    private static byte[] ReciprocalFloor(byte[] d, int dLen, int precision, out int outLen)
    {
        int q = precision - dLen + 1;   // digits of the result
        if (q <= ReciprocalBaseThreshold)
            return ReciprocalFloorBase(d, dLen, precision, out outLen);

        // Half-precision reciprocal at scale f0 = dLen + q0 − 1.
        int q0 = (q + 1) / 2;
        int f0 = dLen + q0 - 1;
        byte[] x0 = ReciprocalFloor(d, dLen, f0, out int x0Len);

        // One Newton step: x = x0·(2·10^f0 − d·x0) ≈ 10^(2·f0) / d.
        byte[] dx0 = MultiplyDigits(d, dLen, x0, x0Len, out int dx0Len);
        byte[] twoPow = TwoTimesPow10(f0, out int twoLen);
        byte[] m = SubtractDigits(twoPow, twoLen, dx0, dx0Len, out int mLen);
        byte[] x = MultiplyDigits(x0, x0Len, m, mLen, out int xLen);

        // Truncate the scale overshoot (2·f0 ≥ precision always holds), then correct.
        int shift = 2 * f0 - precision;
        if (shift > 0)
            x = ShiftRightDigits(x, xLen, shift, out xLen);

        x = CorrectReciprocal(x, xLen, d, dLen, precision, out xLen);
        outLen = xLen;
        return x;
    }

    /// <summary>
    /// Base case for <see cref="ReciprocalFloor"/>: small results (fewer than
    /// <see cref="ReciprocalBaseThreshold"/> digits) via a direct Newton loop seeded
    /// from the top digits of <paramref name="d"/>. When <paramref name="d"/> is much
    /// longer than the result, only its top digits are consulted.
    /// </summary>
    private static byte[] ReciprocalFloorBase(byte[] d, int dLen, int precision, out int outLen)
    {
        // The result has q = precision − dLen + 1 significant digits; only the top
        // ~q+guard digits of d influence it, so truncate d to bound the working size.
        const int guard = 16;
        int q = precision - dLen + 1;               // digits of the result (≥ 1)
        int t = Math.Min(dLen, q + guard);          // digits of the working divisor
        byte[] dTop = t == dLen ? d : SliceDigits(d, dLen - t, t);
        int p = precision - dLen + t;               // floor(10^p / dTop) = floor(10^precision / d)

        // Bootstrap from the top 15 digits of dTop: X0 = floor(10^32 / lead).
        const int k = 15;
        const int B = 32;
        ulong lead = 0;
        for (int i = 0; i < k; i++)
            lead = lead * 10UL + dTop[t - 1 - i];

        int s = t - k + B;                          // x ≈ 10^s / dTop
        byte[] x = FromBigInteger(System.Numerics.BigInteger.Pow(10, B) / lead, out int xLen);

        // Newton doubling until the leading `q + 8` digits are correct. Each step
        // doubles both the scale s and the number of correct digits (~16 initially).
        int correctDigits = 16;
        while (correctDigits < q + 8)
        {
            byte[] dx = MultiplyDigits(dTop, t, x, xLen, out int dxLen);
            byte[] twoPow = TwoTimesPow10(s, out int twoLen);
            byte[] m = SubtractDigits(twoPow, twoLen, dx, dxLen, out int mLen);
            x = MultiplyDigits(x, xLen, m, mLen, out xLen);
            s = 2 * s;
            correctDigits = 2 * correctDigits;
        }

        // Truncate the overshoot beyond p, then correct to the exact floor.
        if (s > p)
            x = ShiftRightDigits(x, xLen, s - p, out xLen);

        x = CorrectReciprocal(x, xLen, dTop, t, p, out xLen);

        // The top-digit shortcut yields floor(10^p/dTop) = floor(10^precision/d) ± 1;
        // correct against the full d so the returned value is exact. The recursive
        // Newton step requires x0 = floor(10^f0/d) exactly, otherwise 2·10^f0 − d·x0
        // can go negative.
        x = CorrectReciprocal(x, xLen, d, dLen, precision, out xLen);
        outLen = xLen;
        return x;
    }

    /// <summary>
    /// Adjusts <paramref name="x"/> (an estimate of <c>floor(10^precision/d)</c>) to
    /// the exact floor value.
    /// </summary>
    private static byte[] CorrectReciprocal(byte[] x, int xLen, byte[] d, int dLen,
                                            int precision, out int outLen)
    {
        byte[] p10 = Pow10Digits(precision, out int p10Len);

        // Decrease while x·d > 10^precision.
        byte[] prod = MultiplyDigits(x, xLen, d, dLen, out int prodLen);
        while (CompareDigits(prod, prodLen, p10, p10Len) > 0)
        {
            x = SubtractOneDigits(x, xLen, out xLen);
            prod = MultiplyDigits(x, xLen, d, dLen, out prodLen);
        }

        // Increase while (x+1)·d ≤ 10^precision.
        byte[] xp1 = AddOneDigits(x, xLen, out int xp1Len);
        prod = MultiplyDigits(xp1, xp1Len, d, dLen, out prodLen);
        while (CompareDigits(prod, prodLen, p10, p10Len) <= 0)
        {
            x = xp1; xLen = xp1Len;
            xp1 = AddOneDigits(x, xLen, out xp1Len);
            prod = MultiplyDigits(xp1, xp1Len, d, dLen, out prodLen);
        }

        outLen = xLen;
        return x;
    }

    /// <summary>Returns 10^n as a canonical LSD-first digit array (n+1 digits).</summary>
    private static byte[] Pow10Digits(int n, out int len)
    {
        byte[] r = new byte[n + 1];
        r[n] = 1;
        len = n + 1;
        return r;
    }

    /// <summary>Returns 2·10^s as a canonical LSD-first digit array (s+1 digits).</summary>
    private static byte[] TwoTimesPow10(int s, out int len)
    {
        byte[] r = new byte[s + 1];
        r[s] = 2;
        len = s + 1;
        return r;
    }

    /// <summary>Drops the <paramref name="k"/> least-significant digits (floor ÷ 10^k).</summary>
    private static byte[] ShiftRightDigits(byte[] a, int aLen, int k, out int outLen)
    {
        if (k <= 0) { outLen = aLen; return CopyDigits(a, aLen); }
        if (k >= aLen) { outLen = 0; return Array.Empty<byte>(); }

        outLen = aLen - k;
        byte[] r = new byte[outLen];
        Array.Copy(a, k, r, 0, outLen);
        return r;
    }

    /// <summary>Returns a + 1 as a canonical LSD-first digit array.</summary>
    private static byte[] AddOneDigits(byte[] a, int aLen, out int outLen)
    {
        byte[] r = new byte[aLen + 1];
        Array.Copy(a, 0, r, 0, aLen);
        int carry = 1;
        for (int i = 0; i <= aLen && carry > 0; i++)
        {
            int t = r[i] + carry;
            r[i] = (byte)(t % 10);
            carry = t / 10;
        }
        outLen = aLen + 1;
        while (outLen > 0 && r[outLen - 1] == 0)
            outLen--;
        return r;
    }

    /// <summary>Returns a − 1 (a &gt; 0) as a canonical LSD-first digit array.</summary>
    private static byte[] SubtractOneDigits(byte[] a, int aLen, out int outLen)
    {
        byte[] r = CopyDigits(a, aLen);
        int borrow = 1;
        for (int i = 0; i < aLen && borrow > 0; i++)
        {
            int t = r[i] - borrow;
            if (t < 0) { t += 10; borrow = 1; } else { borrow = 0; }
            r[i] = (byte)t;
        }
        outLen = aLen;
        while (outLen > 0 && r[outLen - 1] == 0)
            outLen--;
        return r;
    }

    /// <summary>Converts a (small, constant-size) BigInteger to an LSD-first digit array.</summary>
    private static byte[] FromBigInteger(System.Numerics.BigInteger v, out int len)
    {
        if (v.IsZero) { len = 0; return Array.Empty<byte>(); }

        string s = v.ToString();
        len = s.Length;
        byte[] r = new byte[len];
        for (int i = 0; i < len; i++)
            r[i] = (byte)(s[len - 1 - i] - '0');
        return r;
    }

    /// <summary>
    /// Raises this instance to the power of <paramref name="exponent"/>
    /// using binary (repeated-squaring) exponentiation — O(log n) multiplications
    /// rather than O(n). Any base raised to the power of zero returns 1.
    /// </summary>
    /// <remarks>
    /// Algorithm: maintain a running <c>result = 1</c> and a mutable copy of the
    /// base <c>b</c>. At each step, if the current exponent is odd, fold <c>b</c>
    /// into <c>result</c>; square <c>b</c>; then halve the exponent (integer ÷ 2).
    /// Repeat until the exponent reaches zero.
    /// </remarks>
    public Natural Pow(Natural exponent)
    {
        if (IsZero(exponent))
            return One;

        var result = One;
        var b      = new Natural(this);     // mutable copy of base
        var e      = new Natural(exponent); // mutable copy of exponent

        while (!IsZero(e))
        {
            if (IsOddInteger(e))
                result *= b;

            b *= b;
            e  = DivRem(e, new Natural(2UL), out _);
        }

        return result;
    }

    /// <summary>
    /// Returns the factorial of this instance (this!).
    /// Mirrors C++ <c>fatorial</c>: returns 1 for 0! and 1!, then accumulates
    /// the product 2 × 3 × … × this for larger values.
    /// </summary>
    /// <remarks>
    /// For values large enough to benefit from parallelism (greater than
    /// 2 × <see cref="Environment.ProcessorCount"/>), the range [2..n] is
    /// partitioned into <see cref="Environment.ProcessorCount"/> sub-ranges that
    /// are multiplied concurrently via <see cref="Parallel.For(int,int,Action{int})"/>.
    /// The resulting partial products are then combined serially.
    /// For small values or when the input does not fit in a <see cref="ulong"/>,
    /// a simple sequential loop is used instead.
    /// </remarks>
    public Natural Factorial()
    {
        if (IsZero(this)) return One;

        // Use the string representation to obtain the numeric value for range
        // partitioning. If this exceeds ulong.MaxValue the caller wants a
        // ludicrously large factorial; fall back to sequential in that case.
        int processorCount = Environment.ProcessorCount;
        if (!ulong.TryParse(ToString(), out ulong n) || n <= (ulong)(processorCount * 2))
        {
            // Sequential path — small values or astronomically large N.
            var seqResult = One;
            for (var aux = new Natural(2UL); aux <= this; aux++)
                seqResult *= aux;
            return seqResult;
        }

        // Parallel tree reduction.
        // Partition the factor range [2..n] into `processorCount` sub-ranges.
        // Each sub-range is multiplied independently, then all partial products
        // are combined serially (carry-chain addition prevents a fully parallel
        // reduction of the final combination step).

        int t = processorCount;
        var partials = new Natural[t];
        for (int i = 0; i < t; i++) partials[i] = One;

        ulong totalFactors = n - 1UL;                                           // factors: 2,3,…,n  → count = n-1
        ulong rangeSize    = (totalFactors + (ulong)t - 1UL) / (ulong)t;       // ceil division

        Parallel.For(0, t, i =>
        {
            ulong start = 2UL + (ulong)i * rangeSize;
            ulong end   = start + rangeSize - 1UL;
            if (end > n) end = n;
            if (start > n) return;          // this partition slot is empty

            var sub = One;
            for (ulong k = start; k <= end; k++)
                sub *= new Natural(k);
            partials[i] = sub;
        });

        // Serial combination of partial products.
        var result = One;
        foreach (var p in partials)
            result *= p;
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
