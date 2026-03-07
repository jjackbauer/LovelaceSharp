using System.Diagnostics.CodeAnalysis;
using System.Globalization;
using System.Numerics;
using System.Threading;
using Lovelace.Natural;

using Int = Lovelace.Integer.Integer;
using Nat = Lovelace.Natural.Natural;

namespace Lovelace.Real;

/// <summary>
/// Arbitrary-precision real number (ℝ as fixed-point decimal with optional period).
/// Extends <see cref="Int"/> by adding a decimal exponent and period metadata to
/// support exact rational representation via periodic decimal notation (e.g. <c>"0.(3)"</c>).
/// Corresponds to C++ <c>RealLovelace</c>.
/// </summary>
public class Real :
    Int,
    INumber<Real>,
    ISignedNumber<Real>,
    IComparable<Real>,
    IEquatable<Real>,
    IParsable<Real>,
    ISpanParsable<Real>,
    ISpanFormattable,
    IAdditionOperators<Real, Real, Real>,
    ISubtractionOperators<Real, Real, Real>,
    IMultiplyOperators<Real, Real, Real>,
    IDivisionOperators<Real, Real, Real>,
    IUnaryNegationOperators<Real, Real>,
    IIncrementOperators<Real>,
    IDecrementOperators<Real>,
    IComparisonOperators<Real, Real, bool>
{
    // -------------------------------------------------------------------------
    // Static configuration (C++ casasDecimaisExibicao / new MaxComputationDecimalPlaces)
    // -------------------------------------------------------------------------

    private static long _displayDecimalPlaces = 100L;
    private static long _maxComputationDecimalPlaces = 1000L;

    /// <summary>
    /// Controls how many fractional digits appear in <see cref="ToString()"/> for non-periodic values.
    /// Default is 100. Corresponds to C++ <c>casasDecimaisExibicao</c>.
    /// Reads and writes are atomic via <see cref="Interlocked"/>.
    /// </summary>
    public static long DisplayDecimalPlaces
    {
        get => Interlocked.Read(ref _displayDecimalPlaces);
        set => Interlocked.Exchange(ref _displayDecimalPlaces, value);
    }

    /// <summary>
    /// Hard cap on how many digits are generated during division and arithmetic
    /// before giving up on period detection; used as the approximation cutoff for irrationals.
    /// Default is 1000. Has no C++ counterpart (new C# addition).
    /// Reads and writes are atomic via <see cref="Interlocked"/>.
    /// </summary>
    public static long MaxComputationDecimalPlaces
    {
        get => Interlocked.Read(ref _maxComputationDecimalPlaces);
        set => Interlocked.Exchange(ref _maxComputationDecimalPlaces, value);
    }

    // -------------------------------------------------------------------------
    // Instance properties
    // -------------------------------------------------------------------------

    /// <summary>
    /// The decimal exponent. A value of -2 means there are 2 fractional digits stored
    /// to the right of the decimal point; 0 means the value is an integer.
    /// Corresponds to C++ <c>expoente</c>.
    /// </summary>
    public long Exponent { get; set; }

    /// <summary>
    /// Zero-based fractional-digit index where the repeating block begins.
    /// Meaningful only when <see cref="IsPeriodic"/> is <see langword="true"/>.
    /// New C# addition with no C++ counterpart.
    /// </summary>
    public long PeriodStart { get; private set; }

    /// <summary>
    /// Length of the repeating block. <c>0</c> = non-periodic.
    /// New C# addition with no C++ counterpart.
    /// </summary>
    public long PeriodLength { get; private set; }

    /// <summary>
    /// Computed property. Returns <see langword="true"/> when <see cref="PeriodLength"/> &gt; 0.
    /// New C# addition with no C++ counterpart.
    /// </summary>
    public bool IsPeriodic => PeriodLength > 0;

    // -------------------------------------------------------------------------
    // INumberBase<Real> — required static constants
    // -------------------------------------------------------------------------

    /// <inheritdoc cref="Int.One"/>
    public static new Real One => new(Nat.One, false, 0L);

    /// <inheritdoc cref="Int.Zero"/>
    public static new Real Zero => new();

    /// <inheritdoc cref="Int.Radix"/>
    public static new int Radix => 10;

    /// <inheritdoc cref="Int.AdditiveIdentity"/>
    public static new Real AdditiveIdentity => Zero;

    /// <inheritdoc cref="Int.MultiplicativeIdentity"/>
    public static new Real MultiplicativeIdentity => One;

    /// <inheritdoc cref="Int.NegativeOne"/>
    public static new Real NegativeOne => new(Nat.One, true, 0L);

    // -------------------------------------------------------------------------
    // Constructors
    // -------------------------------------------------------------------------

    /// <summary>
    /// Default constructor — value zero, exponent 0, no period.
    /// Corresponds to C++ <c>RealLovelace()</c>.
    /// </summary>
    public Real() : base()
    {
        Exponent = 0L;
        PeriodStart = 0L;
        PeriodLength = 0L;
    }

    /// <summary>
    /// Constructs from a <see cref="double"/> by parsing its round-trip string representation.
    /// Corresponds to C++ <c>RealLovelace(const double A)</c>.
    /// </summary>
    public Real(double value) : this(Parse(value.ToString("R", CultureInfo.InvariantCulture), null)) { }

    /// <summary>
    /// Constructs by parsing a decimal string (optionally signed, with decimal point,
    /// or in periodic notation <c>"0.(142857)"</c>).
    /// Corresponds to C++ <c>RealLovelace(string A)</c>.
    /// </summary>
    public Real(string value) : this(Parse(value, null)) { }

    /// <summary>
    /// Copy constructor — deep-copies digits, exponent, sign, and period metadata.
    /// Corresponds to C++ <c>RealLovelace(const RealLovelace &amp;A)</c>.
    /// </summary>
    public Real(Real other) : base(other.ToNatural(), Int.IsNegative(other))
    {
        Exponent = other.Exponent;
        PeriodStart = other.PeriodStart;
        PeriodLength = other.PeriodLength;
    }

    /// <summary>
    /// Constructs from an <see cref="Int"/> — copies digits and sign, sets <c>Exponent = 0</c>.
    /// Corresponds to C++ <c>RealLovelace(const InteiroLovelace &amp;A)</c>.
    /// </summary>
    public Real(Int other) : base(other.ToNatural(), Int.IsNegative(other))
    {
        Exponent = 0L;
        PeriodStart = 0L;
        PeriodLength = 0L;
    }

    /// <summary>
    /// Constructs by parsing a <see cref="ReadOnlySpan{T}">ReadOnlySpan&lt;char&gt;</see> in the same
    /// formats accepted by <see cref="Parse(string,IFormatProvider?)"/>.
    /// Mandatory commodity parsing constructor — new C# addition with no C++ counterpart.
    /// </summary>
    /// <param name="value">A character span representing the decimal (optionally signed, with decimal
    /// point, or periodic notation such as <c>"0.(3)"</c>).</param>
    /// <exception cref="FormatException">Thrown when <paramref name="value"/> is not a valid decimal.</exception>
    public Real(ReadOnlySpan<char> value) : this(Parse(value, null)) { }

    /// <summary>
    /// Constructs from a <see cref="decimal"/> value, preserving up to 29 significant digits.
    /// Uses <c>"G29"</c> with <see cref="CultureInfo.InvariantCulture"/> to avoid scientific notation.
    /// New C# addition with no C++ counterpart.
    /// </summary>
    /// <param name="value">The <see cref="decimal"/> to represent.</param>
    public Real(decimal value) : this(value.ToString("G29", CultureInfo.InvariantCulture)) { }

    /// <summary>
    /// Internal constructor used by arithmetic operations and factory methods.
    /// </summary>
    internal Real(Nat magnitude, bool isNegative, long exponent,
                  long periodStart = 0L, long periodLength = 0L)
        : base(magnitude, isNegative)
    {
        Exponent = exponent;
        PeriodStart = periodStart;
        PeriodLength = periodLength;
    }

    // -------------------------------------------------------------------------
    // Assign
    // -------------------------------------------------------------------------

    /// <summary>
    /// Returns a new <see cref="Real"/> that is a deep copy of <paramref name="other"/>
    /// (digits, exponent, sign, zero flag, <see cref="PeriodStart"/>, <see cref="PeriodLength"/>).
    /// Corresponds to C++ <c>atribuir(const RealLovelace &amp;A)</c>.
    /// </summary>
    public Real Assign(Real other) =>
        new(other.ToNatural(), Int.IsNegative(other), other.Exponent, other.PeriodStart, other.PeriodLength);

    // -------------------------------------------------------------------------
    // INumberBase<Real> — static predicates
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static new bool IsZero(Real value) => Int.IsZero(value);

    /// <inheritdoc/>
    public static new bool IsPositive(Real value) => Int.IsPositive(value);

    /// <inheritdoc/>
    public static new bool IsNegative(Real value) => Int.IsNegative(value);

    /// <inheritdoc/>
    public static new bool IsEvenInteger(Real value) =>
        !value.IsPeriodic && value.Exponent >= 0L && Int.IsEvenInteger(value);

    /// <inheritdoc/>
    public static new bool IsOddInteger(Real value) =>
        !value.IsPeriodic && value.Exponent >= 0L && Int.IsOddInteger(value);

    /// <inheritdoc/>
    public static new bool IsInteger(Real value) =>
        !value.IsPeriodic && value.Exponent >= 0L;

    // Required INumberBase<T> classification stubs
    /// <inheritdoc/>
    public static new bool IsCanonical(Real value) => true;
    /// <inheritdoc/>
    public static new bool IsComplexNumber(Real value) => false;
    /// <inheritdoc/>
    public static new bool IsFinite(Real value) => true;
    /// <inheritdoc/>
    public static new bool IsImaginaryNumber(Real value) => false;
    /// <inheritdoc/>
    public static new bool IsInfinity(Real value) => false;
    /// <inheritdoc/>
    public static new bool IsNaN(Real value) => false;
    /// <inheritdoc/>
    public static new bool IsNegativeInfinity(Real value) => false;
    /// <inheritdoc/>
    public static new bool IsNormal(Real value) => !IsZero(value);
    /// <inheritdoc/>
    public static new bool IsPositiveInfinity(Real value) => false;
    /// <inheritdoc/>
    public static new bool IsRealNumber(Real value) => true;
    /// <inheritdoc/>
    public static new bool IsSubnormal(Real value) => false;

    // -------------------------------------------------------------------------
    // Magnitude / conversion helpers  (INumberBase<Real>)
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static new Real Abs(Real value) => new(value.ToNatural(), false, value.Exponent, value.PeriodStart, value.PeriodLength);

    /// <inheritdoc/>
    public static new Real MaxMagnitude(Real x, Real y)
        => Abs(x).CompareTo(Abs(y)) >= 0 ? x : y;

    /// <inheritdoc/>
    public static new Real MaxMagnitudeNumber(Real x, Real y) => MaxMagnitude(x, y);

    /// <inheritdoc/>
    public static new Real MinMagnitude(Real x, Real y)
        => Abs(x).CompareTo(Abs(y)) <= 0 ? x : y;

    /// <inheritdoc/>
    public static new Real MinMagnitudeNumber(Real x, Real y) => MinMagnitude(x, y);

    /// <inheritdoc/>
    public static new bool TryConvertFromChecked<TOther>(TOther value, [MaybeNullWhen(false)] out Real result)
        where TOther : INumberBase<TOther>
    { result = Zero; return false; }

    /// <inheritdoc/>
    public static new bool TryConvertFromSaturating<TOther>(TOther value, [MaybeNullWhen(false)] out Real result)
        where TOther : INumberBase<TOther>
    { result = Zero; return false; }

    /// <inheritdoc/>
    public static new bool TryConvertFromTruncating<TOther>(TOther value, [MaybeNullWhen(false)] out Real result)
        where TOther : INumberBase<TOther>
    { result = Zero; return false; }

    /// <inheritdoc/>
    public static new bool TryConvertToChecked<TOther>(Real value, [MaybeNullWhen(false)] out TOther result)
        where TOther : INumberBase<TOther>
    { result = default; return false; }

    /// <inheritdoc/>
    public static new bool TryConvertToSaturating<TOther>(Real value, [MaybeNullWhen(false)] out TOther result)
        where TOther : INumberBase<TOther>
    { result = default; return false; }

    /// <inheritdoc/>
    public static new bool TryConvertToTruncating<TOther>(Real value, [MaybeNullWhen(false)] out TOther result)
        where TOther : INumberBase<TOther>
    { result = default; return false; }

    // -------------------------------------------------------------------------
    // Equality / Comparison  (IEquatable<Real>, IComparable<Real>)
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public bool Equals(Real? other)
    {
        if (other is null) return false;
        if (IsPeriodic != other.IsPeriodic) return false;
        if (IsPeriodic)
        {
            // Two periodic reals are equal when their ToString representations match.
            return ToString() == other.ToString();
        }
        // Non-periodic: compare exponent-aligned digit sequences.
        return CompareTo(other) == 0;
    }

    /// <inheritdoc/>
    public int CompareTo(Real? other)
    {
        if (other is null) return 1;
        // Delegate to arithmetic comparison (handles exponent alignment).
        // For now, use the inherited Integer comparison on the magnitude as a placeholder;
        // full exponent-aware comparison is implemented with the comparison operators checklist item.
        // Positive vs negative fast-path.
        bool thisNeg = Int.IsNegative(this);
        bool otherNeg = Int.IsNegative(other);
        if (!thisNeg && otherNeg) return 1;
        if (thisNeg && !otherNeg) return -1;

        // Same sign: compare magnitudes digit-by-digit (exponent-aligned).
        string digitsA = ToNatural().ToString();
        string digitsB = other.ToNatural().ToString();
        long intLenA = (long)digitsA.Length + Exponent;
        long intLenB = (long)digitsB.Length + other.Exponent;
        int signMul = thisNeg ? -1 : 1;

        // Different integer-part lengths → longer magnitude wins.
        if (intLenA != intLenB)
            return (intLenA > intLenB ? 1 : -1) * signMul;

        // Equal integer-part lengths: compare digit by digit from most significant.
        // For periodic values we extend the comparison window to cover the fractional period.
        long storedMax = Math.Max((long)digitsA.Length, (long)digitsB.Length);
        long maxPositions = (IsPeriodic || other.IsPeriodic)
            ? storedMax + DisplayDecimalPlaces
            : storedMax;

        for (long i = 0; i < maxPositions; i++)
        {
            char dA = GetDigitAtPosition(this,  i, intLenA, digitsA);
            char dB = GetDigitAtPosition(other, i, intLenB, digitsB);
            if (dA != dB)
                return (dA > dB ? 1 : -1) * signMul;
        }
        return 0;
    }

    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is Real r && Equals(r);

    /// <inheritdoc/>
    public override int GetHashCode() => ToString().GetHashCode();

    // -------------------------------------------------------------------------
    // Comparison operators  (IComparisonOperators<Real,Real,bool>)
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static bool operator ==(Real left, Real right) => left.Equals(right);
    /// <inheritdoc/>
    public static bool operator !=(Real left, Real right) => !left.Equals(right);
    /// <inheritdoc/>
    public static bool operator >(Real left, Real right) => left.CompareTo(right) > 0;
    /// <inheritdoc/>
    public static bool operator >=(Real left, Real right) => left.CompareTo(right) >= 0;
    /// <inheritdoc/>
    public static bool operator <(Real left, Real right) => left.CompareTo(right) < 0;
    /// <inheritdoc/>
    public static bool operator <=(Real left, Real right) => left.CompareTo(right) <= 0;

    // -------------------------------------------------------------------------
    // Arithmetic operators
    // -------------------------------------------------------------------------

    // operator+ is implemented below in the Add section.

    /// <summary>Subtracts <paramref name="right"/> from <paramref name="left"/>.
    /// Implemented as <c>Add(left, Negate(right))</c>.
    /// Corresponds to C++ <c>subtrair</c>.
    /// </summary>
    public static Real Subtract(Real left, Real right) => Add(left, Negate(right));

    /// <inheritdoc cref="Subtract"/>
    public static Real operator -(Real left, Real right) => Subtract(left, right);

    /// <summary>
    /// Multiplies two <see cref="Real"/> values.
    /// Non-periodic path: multiplies raw magnitudes as integers; result exponent =
    /// <paramref name="left"/>.Exponent + <paramref name="right"/>.Exponent.
    /// Periodic path: expands each operand to <see cref="MaxComputationDecimalPlaces"/>
    /// fractional digits using <see cref="GetDecimalDigit"/> and runs period detection on
    /// the result.
    /// Corresponds to C++ <c>multiplicar</c>.
    /// </summary>
    public static Real Multiply(Real left, Real right)
    {
        bool eitherPeriodic = left.IsPeriodic || right.IsPeriodic;
        long resultExp = left.Exponent + right.Exponent;

        if (!eitherPeriodic)
        {
            // Non-periodic path: multiply raw magnitudes (the decimal point is handled
            // entirely by the exponent — no alignment shift is needed).
            Int leftInt  = new Int(left.ToNatural(),  Int.IsNegative(left));
            Int rightInt = new Int(right.ToNatural(), Int.IsNegative(right));
            var product  = leftInt * rightInt;
            return Normalize(new Real(product.ToNatural(), Int.IsNegative(product), resultExp));
        }
        else
        {
            // Periodic path: expand both operands to MaxComputationDecimalPlaces fractional
            // digits (resolving any period), multiply via the non-periodic path, then detect
            // a repeating suffix in the result and normalise (including 0.999… → 1).
            long workingFrac   = MaxComputationDecimalPlaces;
            Real expandedLeft  = ExpandToNonPeriodic(left,  workingFrac);
            Real expandedRight = ExpandToNonPeriodic(right, workingFrac);
            Real rawProduct    = Multiply(expandedLeft, expandedRight); // non-periodic path
            return DetectAndNormalizePeriod(rawProduct);
        }
    }

    /// <inheritdoc cref="Multiply"/>
    public static Real operator *(Real left, Real right) => Multiply(left, right);

    /// <summary>
    /// Divides <paramref name="left"/> by <paramref name="right"/> using remainder-tracked
    /// long division to detect repeating decimal periods exactly.
    /// A <see cref="Dictionary{TKey,TValue}"/> maps each remainder string to the first fractional-digit
    /// position at which it appeared; when the same remainder recurs, <see cref="PeriodStart"/>
    /// and <see cref="PeriodLength"/> are set and the loop terminates immediately — yielding an
    /// exact rational result.  When no period is found within
    /// <see cref="MaxComputationDecimalPlaces"/> steps the division is truncated (irrational
    /// approximation).
    /// Corresponds to C++ <c>dividir</c> (which was an empty stub).
    /// </summary>
    /// <exception cref="DivideByZeroException">Thrown when <paramref name="right"/> is zero.</exception>
    public static Real Divide(Real left, Real right)
    {
        if (Real.IsZero(right))
            throw new DivideByZeroException("Cannot divide a Real by zero.");

        if (Real.IsZero(left))
            return Zero;

        bool resultNeg = Real.IsNegative(left) != Real.IsNegative(right);

        // The exponent adjustment: dividing (leftMag × 10^leftExp) by (rightMag × 10^rightExp)
        // equals (leftMag / rightMag) × 10^(leftExp − rightExp).
        long exponentAdjustment = left.Exponent - right.Exponent;

        // Work with absolute-value magnitudes (Natural).
        Nat numerator   = left.ToNatural();
        Nat denominator = right.ToNatural();
        Nat ten         = new Nat(10UL);

        // Integer-part long division.
        Nat quotient  = Nat.DivRem(numerator, denominator, out Nat remainder);
        string quotDigits = Nat.IsZero(quotient) ? "0" : quotient.ToString();

        // Fractional-digit generation loop.
        var fracDigits       = new System.Collections.Generic.List<char>();
        var remainderHistory = new System.Collections.Generic.Dictionary<string, long>();
        long periodStart  = 0L;
        long periodLength = 0L;
        bool foundPeriod  = false;
        long position     = 0L;

        while (!Nat.IsZero(remainder) && position < MaxComputationDecimalPlaces)
        {
            string remKey = remainder.ToString();

            if (remainderHistory.TryGetValue(remKey, out long firstPos))
            {
                // We have seen this remainder before — the result is periodic.
                periodStart  = firstPos;
                periodLength = position - firstPos;
                foundPeriod  = true;
                break;
            }

            remainderHistory[remKey] = position;

            // Next fractional digit: multiply remainder × 10, then divide.
            remainder    = remainder * ten;
            Nat digitNat = Nat.DivRem(remainder, denominator, out remainder);

            // digitNat is guaranteed to be in [0, 9].
            char digitChar = (char)('0' + (Nat.IsZero(digitNat) ? 0 : int.Parse(digitNat.ToString())));
            fracDigits.Add(digitChar);
            position++;
        }

        // Build combined digit string: integer part + fractional digits generated.
        string fracStr   = new string(fracDigits.ToArray());
        string allDigits = quotDigits + fracStr;

        // fracLen is the count of fractional digits actually stored.
        long fracLen = (long)fracDigits.Count;

        // For periodic results the stored fraction is exactly periodStart + periodLength chars.
        // (The loop breaks without adding the repeated digit, so fracDigits already has the right count.)
        long resultExponent = -fracLen + exponentAdjustment;

        if (!Nat.TryParse(allDigits, null, out Nat mag))
            mag = Nat.Zero;

        bool actualNeg = resultNeg && !Nat.IsZero(mag);
        var divResult = new Real(mag, actualNeg, resultExponent,
                                 foundPeriod ? periodStart  : 0L,
                                 foundPeriod ? periodLength : 0L);
        return foundPeriod ? divResult : Normalize(divResult);
    }

    /// <inheritdoc cref="Divide"/>
    public static Real operator /(Real left, Real right) => Divide(left, right);

    /// <summary>Returns the arithmetic negation of <paramref name="value"/>.
    /// Zero remains positive; for all other values the sign bit is flipped
    /// while magnitude, <see cref="Exponent"/>, <see cref="PeriodStart"/> and
    /// <see cref="PeriodLength"/> are preserved.
    /// Overrides <see cref="Int.Negate"/> returning <see cref="Real"/>.
    /// Corresponds to C++ <c>inverterSinal()</c> on <c>RealLovelace</c>.
    /// </summary>
    public static new Real Negate(Real value)
    {
        bool isNeg = Int.IsZero(value) ? false : !Int.IsNegative(value);
        return Normalize(new Real(value.ToNatural(), isNeg, value.Exponent, value.PeriodStart, value.PeriodLength));
    }

    /// <inheritdoc cref="Negate"/>
    public static Real operator -(Real value) => Negate(value);

    /// <summary>
    /// Returns the multiplicative inverse of this value: <c>1 / this</c>.
    /// Delegates to <see cref="Divide"/> so period detection is inherited.
    /// Corresponds to C++ <c>inverter()</c> (which was an empty stub).
    /// </summary>
    /// <exception cref="DivideByZeroException">Thrown when this value is zero.</exception>
    public Real Invert() => Real.One / this;

    /// <summary>
    /// Raises this value to the power of <paramref name="exponent"/>.
    /// <para>
    /// Integer-exponent fast path: when <paramref name="exponent"/> has no fractional part
    /// (or all fractional digits are zero), the exponent is extracted as a <see langword="long"/>
    /// and binary exponentiation via <see cref="Multiply"/> is used.  Signs follow the same
    /// rules as <see cref="Int.Pow"/>: negative base with an even exponent produces a positive
    /// result; negative base with an odd exponent produces a negative result.
    /// </para>
    /// <para>
    /// Non-integer exponents and negative exponents are not yet implemented.
    /// </para>
    /// </summary>
    /// <exception cref="NotImplementedException">Thrown for non-integer or negative exponents.</exception>
    public Real Pow(Real exponent)
    {
        // x^0 = 1 for any base (including zero).
        if (Real.IsZero(exponent)) return Real.One;

        // 0^n = 0 for any positive exponent.
        if (Real.IsZero(this)) return Real.Zero;

        // Extract integer value of exponent from its string representation.
        // "3.0" → 3, "1.0" → 1.  Fractional part must be all zeros.
        string expStr = exponent.ToString();
        long n;
        int dotIdx = expStr.IndexOf('.');
        if (dotIdx >= 0)
        {
            string fracPart = expStr[(dotIdx + 1)..];
            if (fracPart.Any(c => c != '0'))
                throw new NotImplementedException("Non-integer exponents are not yet supported.");
            n = long.Parse(expStr[..dotIdx], System.Globalization.CultureInfo.InvariantCulture);
        }
        else
        {
            n = long.Parse(expStr, System.Globalization.CultureInfo.InvariantCulture);
        }

        if (n < 0)
            throw new NotImplementedException("Negative exponents are not yet supported.");

        // Binary exponentiation: O(log n) multiplications.
        Real result = Real.One;
        Real baseVal = this;
        while (n > 0)
        {
            if (n % 2 == 1)
                result = result * baseVal;
            baseVal = baseVal * baseVal;
            n /= 2;
        }
        return result;
    }

    /// <summary>
    /// Increments <paramref name="value"/> by <see cref="Real.One"/>.
    /// Implements <see cref="IIncrementOperators{Real}"/>.
    /// </summary>
    public static Real operator ++(Real value) => value + Real.One;

    /// <summary>
    /// Decrements <paramref name="value"/> by <see cref="Real.One"/>.
    /// Implements <see cref="IDecrementOperators{Real}"/>.
    /// </summary>
    public static Real operator --(Real value) => value - Real.One;

    /// <summary>
    /// Truncated remainder: <c>left - Truncate(left / right) * right</c>.
    /// Mirrors <see cref="System.Decimal"/> <c>%</c> (truncated towards zero).
    /// Implements <see cref="IModulusOperators{Real,Real,Real}"/>.
    /// </summary>
    /// <exception cref="DivideByZeroException">Thrown when <paramref name="right"/> is zero.</exception>
    public static Real operator %(Real left, Real right) =>
        left - Truncate(left / right) * right;

    /// <summary>Unary plus — returns the value unchanged.</summary>
    public static Real operator +(Real value) =>
        new(value.ToNatural(), Int.IsNegative(value), value.Exponent, value.PeriodStart, value.PeriodLength);

    /// <summary>Explicit <see cref="IUtf8SpanFormattable"/> implementation to resolve ambiguity with <see cref="Int"/>.</summary>
    bool IUtf8SpanFormattable.TryFormat(Span<byte> utf8Destination, out int bytesWritten, ReadOnlySpan<char> format, IFormatProvider? provider)
    {
        var s = ToString();
        var bytes = System.Text.Encoding.UTF8.GetBytes(s);
        if (bytes.Length > utf8Destination.Length) { bytesWritten = 0; return false; }
        bytes.CopyTo(utf8Destination);
        bytesWritten = bytes.Length;
        return true;
    }

    // -------------------------------------------------------------------------
    // Parsing  (IParsable<Real>, ISpanParsable<Real>)  — separate checklist items
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static new Real Parse(string s, IFormatProvider? provider)
    {
        ArgumentNullException.ThrowIfNull(s);
        if (!TryParse(s.AsSpan(), provider, out var result))
            throw new FormatException($"The string '{s}' is not a valid decimal representation of a Real number.");
        return result;
    }

    /// <inheritdoc/>
    public static new Real Parse(ReadOnlySpan<char> s, IFormatProvider? provider)
    {
        if (!TryParse(s, provider, out var result))
            throw new FormatException("The input is not a valid decimal representation of a Real number.");
        return result;
    }

    /// <inheritdoc/>
    public static new bool TryParse(
        [NotNullWhen(true)] string? s,
        IFormatProvider? provider,
        [MaybeNullWhen(false)] out Real result)
    {
        if (s is null) { result = Zero; return false; }
        return TryParse(s.AsSpan(), provider, out result);
    }

    /// <inheritdoc/>
    /// <remarks>
    /// Accepted formats:
    /// <list type="bullet">
    ///   <item><c>[sign] digits</c> — integer, e.g. <c>"42"</c>, <c>"-7"</c></item>
    ///   <item><c>[sign] digits '.' digits</c> — decimal, e.g. <c>"3.14"</c></item>
    ///   <item><c>[sign] digits '.' digits '(' digits ')'</c> — periodic, e.g. <c>"0.(3)"</c>, <c>"0.1(6)"</c></item>
    /// </list>
    /// Leading zeros in the integer part are stripped by <see cref="Nat.TryParse"/>.
    /// Trailing zeros after the decimal point are preserved as significant digits.
    /// Corresponds to C++ <c>ler()</c>.
    /// </remarks>
    public static new bool TryParse(
        ReadOnlySpan<char> s,
        IFormatProvider? provider,
        [MaybeNullWhen(false)] out Real result)
    {
        result = Zero;
        s = s.Trim();
        if (s.IsEmpty) return false;

        // --- sign ---
        bool isNeg = false;
        if (s[0] == '-') { isNeg = true; s = s[1..]; }
        else if (s[0] == '+') { s = s[1..]; }
        if (s.IsEmpty) return false;          // bare "+" or "-"

        // --- locate structural characters ---
        int dotPos    = s.IndexOf('.');
        int parenOpen = s.IndexOf('(');
        int parenClose = s.LastIndexOf(')');

        ReadOnlySpan<char> integerPart;
        ReadOnlySpan<char> nonRepeating;
        ReadOnlySpan<char> periodic;
        bool hasPeriod = false;

        if (dotPos < 0)
        {
            // Integer — no decimal point; periods are not allowed here.
            if (parenOpen >= 0) return false;
            integerPart  = s;
            nonRepeating = ReadOnlySpan<char>.Empty;
            periodic     = ReadOnlySpan<char>.Empty;
        }
        else
        {
            integerPart = s[..dotPos];
            if (integerPart.IsEmpty) return false;  // reject ".5" (no leading digit)

            if (parenOpen >= 0)
            {
                // Periodic notation — validate structure.
                if (parenClose < 0)                   return false; // unclosed (
                if (parenOpen < dotPos + 1)            return false; // ( before .
                if (parenClose != s.Length - 1)        return false; // ) not last
                if (parenClose <= parenOpen + 1)       return false; // empty () period

                nonRepeating = s[(dotPos + 1)..parenOpen];
                periodic     = s[(parenOpen + 1)..parenClose];
                hasPeriod    = true;
            }
            else
            {
                nonRepeating = s[(dotPos + 1)..];
                periodic     = ReadOnlySpan<char>.Empty;
            }
        }

        // --- validate: every character must be a decimal digit ---
        foreach (char c in integerPart)  if (c < '0' || c > '9') return false;
        foreach (char c in nonRepeating) if (c < '0' || c > '9') return false;
        foreach (char c in periodic)     if (c < '0' || c > '9') return false;

        // --- build combined digit string ---
        // Layout: integerPart + nonRepeating + periodic (exactly one copy of the period)
        string allDigits = integerPart.ToString() + nonRepeating.ToString() + periodic.ToString();
        if (allDigits.Length == 0) return false;

        // --- compute metadata ---
        long exponent;
        long periodStart;
        long periodLength;

        if (hasPeriod)
        {
            periodStart  = (long)nonRepeating.Length;
            periodLength = (long)periodic.Length;
            // Fractional digits stored = nonRepeating + one period block.
            exponent = -(periodStart + periodLength);
        }
        else
        {
            periodStart  = 0L;
            periodLength = 0L;
            exponent     = -(long)nonRepeating.Length;
        }

        // --- parse digits as Natural (leading zeros stripped automatically) ---
        if (!Nat.TryParse(allDigits, null, out var magnitude))
            return false;

        // Normalise sign: -0 is represented as positive zero.
        bool actualNeg = isNeg && !Nat.IsZero(magnitude);
        result = new Real(magnitude, actualNeg, exponent, periodStart, periodLength);
        if (periodLength == 0)
            result = Normalize(result);
        return true;
    }

    // Convenience overloads without IFormatProvider (used by tests / callers)
    /// <summary>Parses a decimal string into a <see cref="Real"/>.</summary>
    /// <exception cref="FormatException">Thrown for invalid input.</exception>
    public static new Real Parse(string s) => Parse(s, null);

    /// <summary>Attempts to parse a decimal string into a <see cref="Real"/>.</summary>
    public static bool TryParse([NotNullWhen(true)] string? s, [MaybeNullWhen(false)] out Real result)
        => TryParse(s, null, out result);

    // NumberStyles overloads required by INumberBase<Real>
    /// <inheritdoc/>
    public static new Real Parse(string s, NumberStyles style, IFormatProvider? provider)
        => Parse(s, provider);

    /// <inheritdoc/>
    public static new Real Parse(ReadOnlySpan<char> s, NumberStyles style, IFormatProvider? provider)
        => Parse(s, provider);

    /// <inheritdoc/>
    public static new bool TryParse([NotNullWhen(true)] string? s, NumberStyles style, IFormatProvider? provider,
        [MaybeNullWhen(false)] out Real result)
        => TryParse(s, provider, out result);

    /// <inheritdoc/>
    public static new bool TryParse(ReadOnlySpan<char> s, NumberStyles style, IFormatProvider? provider,
        [MaybeNullWhen(false)] out Real result)
        => TryParse(s, provider, out result);

    // -------------------------------------------------------------------------
    // Formatting  (ISpanFormattable)  — separate checklist item
    // -------------------------------------------------------------------------

    /// <summary>
    /// Converts the real number to its string representation.
    /// Inserts a decimal point at the position dictated by <see cref="Exponent"/>;
    /// prefixes sign; non-periodic values are truncated at <see cref="DisplayDecimalPlaces"/>
    /// fractional digits; periodic values emit the full non-repeating part followed by
    /// <c>(repeating_block)</c> — e.g. <c>"0.(3)"</c>, <c>"-3.(142857)"</c>.
    /// </summary>
    public override string ToString()
    {
        if (Int.IsZero(this)) return "0";

        string digits = ToNatural().ToString();
        bool negative = Int.IsNegative(this);
        string sign = negative ? "-" : "";

        if (Exponent == 0L && !IsPeriodic)
            return sign + digits;

        if (Exponent < 0L && !IsPeriodic)
        {
            long fracLen = -Exponent;
            if (fracLen >= digits.Length)
            {
                // All digits are fractional, need leading "0."
                string padded = digits.PadLeft((int)fracLen, '0');
                string fracPart = padded[..Math.Min((int)fracLen, (int)(fracLen + DisplayDecimalPlaces))];
                // Trim trailing zeros for the non-periodic case
                string intPart = "0";
                return sign + intPart + "." + fracPart;
            }
            else
            {
                int splitAt = (int)(digits.Length - fracLen);
                string intPart = digits[..splitAt];
                string fracPart = digits[splitAt..];
                if (fracPart.Length > (int)DisplayDecimalPlaces)
                    fracPart = fracPart[..(int)DisplayDecimalPlaces];
                return sign + intPart + "." + fracPart;
            }
        }

        if (IsPeriodic)
        {
            // Emit non-repeating part then (period).
            long fracLen = -Exponent;
            string padded = digits.Length < fracLen ? digits.PadLeft((int)fracLen, '0') : digits;
            int splitAt = Math.Max(0, (int)(padded.Length - fracLen));
            string intPart = splitAt == 0 ? "0" : padded[..splitAt];
            string allFrac = padded[splitAt..];

            string nonRepeating = PeriodStart <= allFrac.Length
                ? allFrac[..(int)PeriodStart]
                : allFrac.PadRight((int)PeriodStart, '0');
            string period = allFrac.Length >= PeriodStart + PeriodLength
                ? allFrac[(int)PeriodStart..(int)(PeriodStart + PeriodLength)]
                : digits[(int)PeriodStart..(int)(PeriodStart + PeriodLength)];

            return sign + intPart + (fracLen > 0 ? "." : "") + nonRepeating + "(" + period + ")";
        }

        // Positive exponent (integer shifted left — no decimal point).
        return sign + digits;
    }

    /// <inheritdoc/>
    public new string ToString(string? format, IFormatProvider? formatProvider) => ToString();

    /// <inheritdoc/>
    public new bool TryFormat(
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
    // Private helpers
    // -------------------------------------------------------------------------

    /// <summary>
    /// Strips the fractional part of <paramref name="r"/>, truncating toward zero.
    /// Returns a <see cref="Real"/> with <see cref="Exponent"/> = 0 and no period metadata.
    /// If <paramref name="r"/> has no fractional part (<c>Exponent &gt;= 0</c>), returns a
    /// normalised copy with <c>Exponent = 0</c>.
    /// Used exclusively by <see cref="operator%(Real,Real)"/>.
    /// New C# addition with no C++ counterpart.
    /// </summary>
    private static Real Truncate(Real r)
    {
        if (r.Exponent >= 0)
        {
            // Already an integer value — return as Real with Exponent = 0.
            var mag = r.ToNatural();
            bool neg = Int.IsNegative(r) && !Nat.IsZero(mag);
            return new Real(mag, neg, 0L);
        }

        // Exponent < 0: there are fractional digits to strip.
        string digits    = r.ToNatural().ToString();
        long   fracCount = -r.Exponent;                    // digits belonging to the fraction
        long   intCount  = (long)digits.Length - fracCount;

        if (intCount <= 0)
        {
            // The value is a pure fraction (e.g. 0.5, 0.001) — integer part is 0.
            return Zero;
        }

        string intDigits = digits[..(int)intCount];
        if (!Nat.TryParse(intDigits, null, out var intMag))
            intMag = Nat.Zero;

        bool isNeg = Int.IsNegative(r) && !Nat.IsZero(intMag);
        return new Real(intMag, isNeg, 0L);
    }

    /// <summary>
    /// Reconstructs the digit at fractional position <paramref name="position"/> on demand.
    /// Positions are zero-based from the decimal point (0 = first fractional digit).
    /// For non-periodic values or positions before <see cref="PeriodStart"/>, returns the
    /// stored digit (or 0 if beyond stored range).
    /// For positions at or after <see cref="PeriodStart"/>, wraps into the period via modulo.
    /// Corresponds to new C# addition <c>GetDecimalDigit(long position)</c>.
    /// </summary>
    private byte GetDecimalDigit(long position)
    {
        string digits = ToNatural().ToString();
        long fracLen = -Exponent; // number of stored fractional digits (one period block for periodic)
        if (fracLen <= 0) return 0; // no fractional part at all

        if (!IsPeriodic || position < PeriodStart)
        {
            long idx = (long)digits.Length - fracLen + position;
            if (idx < 0 || idx >= (long)digits.Length) return 0;
            return (byte)(digits[(int)idx] - '0');
        }
        else
        {
            // Wrap position into the stored period block.
            long periodicOffset = (position - PeriodStart) % PeriodLength;
            long idx = (long)digits.Length - fracLen + PeriodStart + periodicOffset;
            return (byte)(digits[(int)idx] - '0');
        }
    }

    /// <summary>
    /// Returns a string of <paramref name="count"/> characters, each the ASCII digit
    /// of the fractional position returned by <see cref="GetDecimalDigit"/>.
    /// </summary>
    private string FracDigitString(long count)
    {
        var sb = new System.Text.StringBuilder((int)count);
        for (long i = 0; i < count; i++)
            sb.Append((char)('0' + GetDecimalDigit(i)));
        return sb.ToString();
    }

    /// <summary>
    /// Appends <paramref name="zeros"/> trailing zero-digits to the digit representation,
    /// effectively multiplying the magnitude by 10^<paramref name="zeros"/>, and returns
    /// the result as a signed <see cref="Int"/>.
    /// Corresponds to C++ <c>toInteiroLovelace(long long int zeros)</c>.
    /// </summary>
    private Int ToInteger(long zeros)
    {
        if (zeros < 0) throw new ArgumentOutOfRangeException(nameof(zeros));
        string digits = ToNatural().ToString();
        string padded = zeros > 0 ? digits + new string('0', (int)zeros) : digits;
        if (!Nat.TryParse(padded, null, out var mag))
            mag = Nat.Zero;
        return new Int(mag, Int.IsNegative(this));
    }

    /// <summary>
    /// Expands <paramref name="r"/> into a non-periodic <see cref="Real"/> with exactly
    /// <paramref name="fracDigits"/> fractional digit positions, using
    /// <see cref="GetDecimalDigit"/> for periodic values.
    /// </summary>
    private static Real ExpandToNonPeriodic(Real r, long fracDigits)
    {
        string digits    = r.ToNatural().ToString();
        long   fracLen   = -r.Exponent;
        long   intPartLen = Math.Max(0L, (long)digits.Length - fracLen);
        string intPart   = intPartLen > 0 ? digits[..(int)intPartLen] : "0";

        // Build the fractional portion using GetDecimalDigit so period is respected.
        string fracPart  = r.FracDigitString(fracDigits);
        string allDigits = intPart + fracPart;

        if (!Nat.TryParse(allDigits, null, out var mag))
            mag = Nat.Zero;

        bool isNeg = Int.IsNegative(r) && !Nat.IsZero(mag);
        return new Real(mag, isNeg, -fracDigits);
    }

    /// <summary>
    /// Finds the smallest repeating suffix of <paramref name="fracPart"/>:
    /// the minimum period length <c>p</c> and non-repeating prefix length <c>s</c>
    /// such that <c>fracPart[s..]</c> repeats with period <c>p</c>.
    /// Returns <c>(0, 0)</c> when no period is found.
    /// <para>
    /// When <paramref name="slack"/> is greater than zero, the last <paramref name="slack"/>
    /// characters are excluded from the period-consistency check.  This accommodates the
    /// single-digit truncation error that can arise when expanding a periodic operand to
    /// a finite precision before performing arithmetic.
    /// </para>
    /// </summary>
    private static (long start, long len) FindSmallestPeriod(string fracPart, int slack = 0)
    {
        int n            = fracPart.Length;
        int effectiveLen = n - slack;
        if (effectiveLen < 2) return (0, 0);

        for (int p = 1; p <= effectiveLen / 2; p++)
        {
            for (int s = 0; s + 2 * p <= effectiveLen; s++)
            {
                bool ok = true;
                for (int i = s + p; i < effectiveLen; i++)
                {
                    if (fracPart[i] != fracPart[s + (i - s) % p])
                    {
                        ok = false;
                        break;
                    }
                }
                if (ok) return (s, p);
            }
        }
        return (0, 0);
    }

    /// <summary>
    /// Returns the canonical (trailing-zero-free) form of a non-periodic <see cref="Real"/>.
    /// For example, <c>8.000</c> (exp=-3, digits="8000") becomes <c>8</c> (exp=0, digits="8").
    /// No-op for periodic values or values with <see cref="Exponent"/> ≥ 0.
    /// </summary>
    private static Real Normalize(Real r)
    {
        if (r.IsPeriodic || r.Exponent >= 0 || Int.IsZero(r))
            return r;

        string digits   = r.ToNatural().ToString();
        long   maxStrip = -r.Exponent; // number of fractional digit slots stored

        int stripped = 0;
        while (stripped < (int)maxStrip
               && stripped < digits.Length
               && digits[digits.Length - 1 - stripped] == '0')
            stripped++;

        if (stripped == 0)
            return r;

        long   newExp    = r.Exponent + stripped;
        string newDigits = stripped >= digits.Length ? "0" : digits[..^stripped];
        if (!Nat.TryParse(newDigits, null, out Nat newMag))
            newMag = Nat.Zero;
        bool isNeg = Int.IsNegative(r) && !Nat.IsZero(newMag);
        return new Real(newMag, isNeg, newExp);
    }

    /// <summary>
    /// Returns the digit character at logical position <paramref name="pos"/> (0 = most
    /// significant integer digit) within <paramref name="r"/>, using
    /// <see cref="GetDecimalDigit"/> for the periodic fractional part.
    /// </summary>
    private static char GetDigitAtPosition(Real r, long pos, long intLen, string digits)
    {
        if (r.IsPeriodic && pos >= intLen)
            return (char)('0' + r.GetDecimalDigit(pos - intLen));
        return pos < (long)digits.Length ? digits[(int)pos] : '0';
    }

    /// <summary>
    /// Inspects the fractional part of a non-periodic <see cref="Real"/> for a repeating
    /// suffix and returns it with period metadata set.  Also normalises the all-nines
    /// period (0.999… → 1.0).
    /// </summary>
    private static Real DetectAndNormalizePeriod(Real r)
    {
        if (r.Exponent >= 0L || Int.IsZero(r))
            return r; // integer value — no fractional period possible

        string digits  = r.ToNatural().ToString();
        long   fracLen = -r.Exponent;
        long   intPartLen = Math.Max(0L, (long)digits.Length - fracLen);
        string intPart   = intPartLen > 0 ? digits[..(int)intPartLen] : "0";
        string fracPart  = digits.Length > (int)intPartLen
            ? digits[(int)intPartLen..]
            : new string('0', (int)fracLen);

        // Pad to expected length if leading zeros were stripped.
        if ((long)fracPart.Length < fracLen)
            fracPart = fracPart.PadLeft((int)fracLen, '0');

        (long pStart, long pLen) = FindSmallestPeriod(fracPart);
        // A single-digit truncation error can prevent exact period detection.
        // If no period was found, retry ignoring the last character (slack = 1).
        if (pLen == 0 && fracPart.Length > 2)
            (pStart, pLen) = FindSmallestPeriod(fracPart, 1);
        if (pLen == 0) return r; // no period detected

        string periodStr = fracPart[(int)pStart..(int)(pStart + pLen)];

        // Check for all-nines period (0.999… = 1.0 etc.)
        bool allNines = true;
        foreach (char c in periodStr) { if (c != '9') { allNines = false; break; } }

        if (allNines)
        {
            // Increment the combined integer+non-repeating part by 1.
            string nonRepeating = fracPart[..(int)pStart];
            string combined = intPart + nonRepeating;
            if (!Nat.TryParse(combined, null, out var cMag))
                cMag = Nat.Zero;
            var incremented = new Int(cMag, false) + Int.One;
            bool isNeg = Int.IsNegative(r) && !Nat.IsZero(incremented.ToNatural());
            long newExp = -(long)nonRepeating.Length;
            return new Real(incremented.ToNatural(), isNeg, newExp);
        }

        // Build periodic Real: store intPart + nonRepeating + one period block.
        string storedDigits = intPart + fracPart[..(int)(pStart + pLen)];
        if (!Nat.TryParse(storedDigits, null, out var mag))
            mag = Nat.Zero;
        bool negative = Int.IsNegative(r) && !Nat.IsZero(mag);
        return new Real(mag, negative, -(pStart + pLen), pStart, pLen);
    }

    // -------------------------------------------------------------------------
    // Add / operator+  (IAdditionOperators<Real,Real,Real>)
    // -------------------------------------------------------------------------

    /// <summary>
    /// Adds two <see cref="Real"/> values.
    /// Non-periodic path: aligns exponents via <see cref="ToInteger"/> then delegates to
    /// <see cref="Int"/> addition.  Periodic path: expands each operand to
    /// <see cref="MaxComputationDecimalPlaces"/> fractional digits using
    /// <see cref="GetDecimalDigit"/> and runs period detection on the result.
    /// Result exponent = min(<paramref name="left"/>.Exponent, <paramref name="right"/>.Exponent).
    /// Corresponds to C++ <c>somar</c>.
    /// </summary>
    public static Real Add(Real left, Real right)
    {
        bool eitherPeriodic = left.IsPeriodic || right.IsPeriodic;

        long exA = left.Exponent;
        long exB = right.Exponent;
        long resultExp = Math.Min(exA, exB);

        if (!eitherPeriodic)
        {
            // Non-periodic path: shift the operand with the larger exponent left.
            Int leftInt = (exA == resultExp)
                ? new Int(left.ToNatural(),  Int.IsNegative(left))
                : left.ToInteger(exA - resultExp);

            Int rightInt = (exB == resultExp)
                ? new Int(right.ToNatural(), Int.IsNegative(right))
                : right.ToInteger(exB - resultExp);

            var sum = leftInt + rightInt;
            return Normalize(new Real(sum.ToNatural(), Int.IsNegative(sum), resultExp));
        }
        else
        {
            // Periodic path: expand both operands to MaxComputationDecimalPlaces fractional
            // digits (resolving any period), add via the non-periodic path, then detect
            // a repeating suffix in the result and normalise (including 0.999… → 1).
            long workingFrac = MaxComputationDecimalPlaces;
            Real expandedLeft  = ExpandToNonPeriodic(left,  workingFrac);
            Real expandedRight = ExpandToNonPeriodic(right, workingFrac);
            Real rawSum        = Add(expandedLeft, expandedRight); // non-periodic path
            return DetectAndNormalizePeriod(rawSum);
        }
    }

    /// <inheritdoc cref="Add"/>
    public static Real operator +(Real left, Real right) => Add(left, right);
}
