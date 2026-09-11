using System.Diagnostics.CodeAnalysis;
using System.Globalization;
using System.Numerics;
using System.Runtime.CompilerServices;
using System.Threading;
using Lovelace.Natural;

using Int = Lovelace.Integer.Integer;
using Nat = Lovelace.Natural.Natural;

[assembly: InternalsVisibleTo("Lovelace.Real.Tests")]

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
    // Per-call-stack precision override set by Sqrt to avoid clobbering the global
    // value when tests run in parallel.  Flows down the call tree (into Divide) but
    // does not propagate sideways to sibling tasks.
    private static readonly AsyncLocal<long?> _localMaxComputationDecimalPlaces = new();
    private static readonly AsyncLocal<long?> _localDisplayDecimalPlaces = new();

    // 640320³ — shared by Pi and PiSegment.
    private static readonly Nat s_chudnovskyC3 = Nat.Parse("262537412640768000", null);

    /// <summary>
    /// Controls how many fractional digits appear in <see cref="ToString()"/> for non-periodic values.
    /// Default is 100. Corresponds to C++ <c>casasDecimaisExibicao</c>.
    /// Reads and writes are atomic via <see cref="Interlocked"/>.
    /// </summary>
    public static long DisplayDecimalPlaces
    {
        get => _localDisplayDecimalPlaces.Value ?? Interlocked.Read(ref _displayDecimalPlaces);
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
        // Prefer the per-call-stack override (set by Sqrt) over the global default.
        get => _localMaxComputationDecimalPlaces.Value ?? Interlocked.Read(ref _maxComputationDecimalPlaces);
        // Sets the global default (existing public API); does not affect any active local scope.
        set => Interlocked.Exchange(ref _maxComputationDecimalPlaces, value);
    }

    // Establishes a call-stack-local precision override for the duration of the returned scope.
    // Restoring the previous local value on Dispose ensures nesting works correctly.
    // Internal (not private) so that Lovelace.Real.Tests can exercise AsyncLocal semantics directly.
    internal static PrecisionScope WithLocalPrecision(long precision)
    {
        long? previous = _localMaxComputationDecimalPlaces.Value;
        _localMaxComputationDecimalPlaces.Value = precision;
        return new PrecisionScope(previous);
    }

    /// <summary>
    /// Establishes a scope that overrides BOTH the computation cap and the display
    /// precision for the duration of the returned scope (restored on Dispose). Public so
    /// the scripting engine and its hosts can make precision session/evaluation-local.
    /// </summary>
    public static IDisposable WithPrecision(long computationDecimalPlaces, long displayDecimalPlaces)
    {
        long? previousComputation = _localMaxComputationDecimalPlaces.Value;
        long? previousDisplay = _localDisplayDecimalPlaces.Value;
        _localMaxComputationDecimalPlaces.Value = computationDecimalPlaces;
        _localDisplayDecimalPlaces.Value = displayDecimalPlaces;
        return new RealPrecisionScope(previousComputation, previousDisplay);
    }

    /// <summary>
    /// Updates the ACTIVE local scope in place. Used by <c>setprecision</c> so a precision
    /// change made mid-evaluation takes effect for the remainder of that evaluation.
    /// </summary>
    public static void SetLocalPrecision(long computationDecimalPlaces, long displayDecimalPlaces)
    {
        _localMaxComputationDecimalPlaces.Value = computationDecimalPlaces;
        _localDisplayDecimalPlaces.Value = displayDecimalPlaces;
    }

    private readonly struct RealPrecisionScope : IDisposable
    {
        private readonly long? _computation;
        private readonly long? _display;
        public RealPrecisionScope(long? computation, long? display)
        {
            _computation = computation;
            _display = display;
        }
        public void Dispose()
        {
            _localMaxComputationDecimalPlaces.Value = _computation;
            _localDisplayDecimalPlaces.Value = _display;
        }
    }

    /// <summary>Synchronous <see cref="IProgress{T}"/> — invokes the handler inline (no thread-pool hop).</summary>
    private sealed class InlineProgress<T> : IProgress<T>
    {
        private readonly Action<T> _report;
        public InlineProgress(Action<T> report) => _report = report;
        public void Report(T value) => _report(value);
    }

    internal readonly struct PrecisionScope : IDisposable
    {
        private readonly long? _saved;
        public PrecisionScope(long? saved) => _saved = saved;
        public void Dispose() => _localMaxComputationDecimalPlaces.Value = _saved;
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
    public static bool IsZero(Real value) => Int.IsZero(value);

    /// <inheritdoc/>
    public static bool IsPositive(Real value) => Int.IsPositive(value);

    /// <inheritdoc/>
    public static bool IsNegative(Real value) => Int.IsNegative(value);

    /// <inheritdoc/>
    public static bool IsEvenInteger(Real value) =>
        !value.IsPeriodic && value.Exponent >= 0L && Int.IsEvenInteger(value);

    /// <inheritdoc/>
    public static bool IsOddInteger(Real value) =>
        !value.IsPeriodic && value.Exponent >= 0L && Int.IsOddInteger(value);

    /// <inheritdoc/>
    public static bool IsInteger(Real value) =>
        !value.IsPeriodic && value.Exponent >= 0L;

    // Required INumberBase<T> classification stubs
    /// <inheritdoc/>
    public static bool IsCanonical(Real value) => true;
    /// <inheritdoc/>
    public static bool IsComplexNumber(Real value) => false;
    /// <inheritdoc/>
    public static bool IsFinite(Real value) => true;
    /// <inheritdoc/>
    public static bool IsImaginaryNumber(Real value) => false;
    /// <inheritdoc/>
    public static bool IsInfinity(Real value) => false;
    /// <inheritdoc/>
    public static bool IsNaN(Real value) => false;
    /// <inheritdoc/>
    public static bool IsNegativeInfinity(Real value) => false;
    /// <inheritdoc/>
    public static bool IsNormal(Real value) => !IsZero(value);
    /// <inheritdoc/>
    public static bool IsPositiveInfinity(Real value) => false;
    /// <inheritdoc/>
    public static bool IsRealNumber(Real value) => true;
    /// <inheritdoc/>
    public static bool IsSubnormal(Real value) => false;

    // -------------------------------------------------------------------------
    // Magnitude / conversion helpers  (INumberBase<Real>)
    // -------------------------------------------------------------------------

    /// <inheritdoc/>
    public static Real Abs(Real value) => new(value.ToNatural(), false, value.Exponent, value.PeriodStart, value.PeriodLength);

    /// <inheritdoc/>
    public static Real MaxMagnitude(Real x, Real y)
        => Abs(x).CompareTo(Abs(y)) >= 0 ? x : y;

    /// <inheritdoc/>
    public static Real MaxMagnitudeNumber(Real x, Real y) => MaxMagnitude(x, y);

    /// <inheritdoc/>
    public static Real MinMagnitude(Real x, Real y)
        => Abs(x).CompareTo(Abs(y)) <= 0 ? x : y;

    /// <inheritdoc/>
    public static Real MinMagnitudeNumber(Real x, Real y) => MinMagnitude(x, y);

    /// <inheritdoc/>
    public static bool TryConvertFromChecked<TOther>(TOther value, [MaybeNullWhen(false)] out Real result)
        where TOther : INumberBase<TOther>
    { result = Zero; return false; }

    /// <inheritdoc/>
    public static bool TryConvertFromSaturating<TOther>(TOther value, [MaybeNullWhen(false)] out Real result)
        where TOther : INumberBase<TOther>
    { result = Zero; return false; }

    /// <inheritdoc/>
    public static bool TryConvertFromTruncating<TOther>(TOther value, [MaybeNullWhen(false)] out Real result)
        where TOther : INumberBase<TOther>
    { result = Zero; return false; }

    /// <inheritdoc/>
    public static bool TryConvertToChecked<TOther>(Real value, [MaybeNullWhen(false)] out TOther result)
        where TOther : INumberBase<TOther>
    { result = default; return false; }

    /// <inheritdoc/>
    public static bool TryConvertToSaturating<TOther>(Real value, [MaybeNullWhen(false)] out TOther result)
        where TOther : INumberBase<TOther>
    { result = default; return false; }

    /// <inheritdoc/>
    public static bool TryConvertToTruncating<TOther>(Real value, [MaybeNullWhen(false)] out TOther result)
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

        // Zero has no significant digits; the exponent-aligned integer-length comparison
        // below mis-orders it otherwise (e.g. 0.5 vs 0, or -0 vs 0), so short-circuit zero.
        bool thisZero = Int.IsZero(this);
        bool otherZero = Int.IsZero(other);
        if (thisZero || otherZero)
        {
            if (thisZero && otherZero) return 0;
            if (thisZero) return Int.IsNegative(other) ? 1 : -1;
            return Int.IsNegative(this) ? -1 : 1;
        }

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
    public static bool operator ==(Real? left, Real? right) => left!.Equals(right);
    /// <inheritdoc/>
    public static bool operator !=(Real? left, Real? right) => !left!.Equals(right);
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
    /// Periodic path (when either operand is periodic): converts both operands to exact
    /// fractions (<see cref="ToExactRational"/>), multiplies them exactly, and divides the
    /// products so <see cref="Divide"/> re-derives the exact period of the result. This makes
    /// <c>(a/b)*b == a</c> hold exactly whenever <c>a/b</c> is representable, instead of
    /// truncating each periodic operand at <see cref="MaxComputationDecimalPlaces"/> digits
    /// (which returned <c>0.99…984</c> for <c>(1/17)*17</c>). The result is truncated only when
    /// the product's own period does not fit in <see cref="MaxComputationDecimalPlaces"/>.
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
            // Periodic path — exact.  A periodic Real denotes an exact rational (its magnitude,
            // Exponent and period block fix a fraction), and a non-periodic Real is a finite
            // decimal, hence also rational, so the product is always a rational.
            //
            // Expanding the operands to MaxComputationDecimalPlaces fractional digits instead
            // TRUNCATES them: 1/17 expanded to 1000 places is slightly less than 1/17, so
            // (1/17)*17 came out as 0.99…984 — a truncated operand that DetectAndNormalizePeriod
            // cannot fold (the tail is not all nines).  Widening the expansion only moves that
            // boundary, so multiply the exact fractions and let Divide's remainder-tracked long
            // division re-derive the exact period of the product.
            (Int leftNum, Int leftDen)   = ToExactRational(left);
            (Int rightNum, Int rightDen) = ToExactRational(right);

            Int num = leftNum * rightNum;
            Int den = leftDen * rightDen;

            // Reduce first: the long division below is cheaper on smaller operands, and the
            // identity case (p/q)*q cancels completely (e.g. 17/17 → 1/1).
            Int gcd = Int.Gcd(num, den);
            if (gcd > Int.One)
            {
                num = num.DivRem(gcd, out _);
                den = den.DivRem(gcd, out _);
            }

            // Divide is exact for rational operands: it detects the repeating period of the
            // quotient by tracking remainders.  If the period exceeds
            // MaxComputationDecimalPlaces the quotient is truncated, exactly as for a division
            // whose period does not fit the budget.
            return Divide(new Real(num), new Real(den));
        }
    }

    /// <summary>
    /// Converts <paramref name="value"/> to an exact fraction (numerator, positive denominator)
    /// using only <see cref="Int"/> arithmetic — the periodic operand of <see cref="Multiply"/>.
    /// <para>
    /// A non-periodic value is a finite decimal: <c>magnitude · 10^Exponent</c>.
    /// </para>
    /// <para>
    /// A periodic value stores its digits as integer part, then <see cref="PeriodStart"/> (<c>s</c>)
    /// non-repeating fractional digits, then one <see cref="PeriodLength"/> (<c>p</c>)-digit block
    /// <c>B</c>. With the integral prefix <c>P</c> (the magnitude's digits above the block) the
    /// stored digits are <c>magnitude = P·10^p + B</c> at scale <c>10^-(s+p)</c>, and the repeating
    /// tail contributes the geometric series <c>B·10^-(s+p)/(1 − 10^-p)</c>, giving
    /// <c>(P·(10^p − 1) + B) / (10^s · (10^p − 1))</c>.
    /// </para>
    /// </summary>
    private static (Int Num, Int Den) ToExactRational(Real value)
    {
        Int ten = new Int(10L);
        Nat magnitude = value.ToNatural();
        Int num;
        Int den;

        if (!value.IsPeriodic)
        {
            num = new Int(magnitude, false);
            den = Int.One;
            if (value.Exponent > 0L)
                num = num * ten.Pow(new Int(value.Exponent));
            else if (value.Exponent < 0L)
                den = ten.Pow(new Int(-value.Exponent));
        }
        else
        {
            long s = value.PeriodStart;
            long p = value.PeriodLength;

            Nat tenToP = new Nat(10UL).Pow(new Nat((ulong)p));
            Nat prefix = Nat.DivRem(magnitude, tenToP, out Nat block);

            Int one      = Int.One;
            Int tenToPInt = new Int(tenToP, false);
            Int repUnit   = tenToPInt - one;               // 10^p − 1
            num = new Int(prefix, false) * repUnit + new Int(block, false);
            den = repUnit;
            if (s > 0L)
                den = den * ten.Pow(new Int(s));

            // A periodic value stores exactly the non-repeating prefix plus one period block, so
            // Exponent == -(s + p).  Fold any residual scale difference into the fraction instead
            // of relying on that invariant.
            long scaleDelta = value.Exponent + s + p;
            if (scaleDelta > 0L)
                num = num * ten.Pow(new Int(scaleDelta));
            else if (scaleDelta < 0L)
                den = den * ten.Pow(new Int(-scaleDelta));
        }

        if (Int.IsNegative(value) && !Int.IsZero(num))
            num = num.Negate();

        return (num, den);
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

        // Periodic guard: mirrors the Add / Multiply pattern.
        // Only periodic operands are expanded; non-periodic operands are used as-is to
        // avoid introducing trailing zeros that could produce a spurious all-zero period
        // in the result (e.g. expanding non-periodic 1 to 1000 decimal places yields
        // 1.0000…0, and dividing that by an expanded periodic denominator produces a
        // quotient whose trailing zeros mislead DetectAndNormalizePeriod).
        // A period of length ≥ workingFrac in the raw quotient is an artifact of the
        // finite expansion (the remainder repeats at the expansion boundary), not a true
        // mathematical period — strip it before running period detection.
        if (left.IsPeriodic || right.IsPeriodic)
        {
            long workingFrac   = MaxComputationDecimalPlaces;
            Real expandedLeft  = left.IsPeriodic  ? ExpandToNonPeriodic(left,  workingFrac) : left;
            Real expandedRight = right.IsPeriodic ? ExpandToNonPeriodic(right, workingFrac) : right;
            Real rawQuotient   = Divide(expandedLeft, expandedRight);
            if (rawQuotient.IsPeriodic && rawQuotient.PeriodLength >= workingFrac)
                rawQuotient = new Real(rawQuotient.ToNatural(), Int.IsNegative(rawQuotient), rawQuotient.Exponent);
            return DetectAndNormalizePeriod(rawQuotient);
        }

        bool resultNeg = Real.IsNegative(left) != Real.IsNegative(right);

        // The exponent adjustment: dividing (leftMag × 10^leftExp) by (rightMag × 10^rightExp)
        // equals (leftMag / rightMag) × 10^(leftExp − rightExp).
        long exponentAdjustment = left.Exponent - right.Exponent;

        // Work with absolute-value magnitudes (Natural), aligned to ONE decimal exponent before
        // the digit division: (A × 10^(eA−eB)) / B when eA ≥ eB, and A / (B × 10^(eB−eA))
        // otherwise.  The loop below places the quotient's decimal point at the boundary between
        // the integer digits and the fractional digits it accumulates (result exponent =
        // −(stored fractional digits)), so the scale difference must be folded into the operands.
        // Carrying it in the result exponent instead leaves the quotient's stored digits ending
        // somewhere other than its decimal point: the leading fractional zeros of the raw quotient
        // A/B shift the value (1 / 0.9 returned 1) and consume the digit budget (1 / 0.99862…
        // returned 0), and PeriodStart/PeriodLength end up naming fractional positions the
        // magnitude never stored (1 / 0.99 then threw from ToString).
        Nat numerator   = left.ToNatural();
        Nat denominator = right.ToNatural();
        if (exponentAdjustment > 0L)
            numerator = numerator.ShiftLeftDecimal(exponentAdjustment);
        else if (exponentAdjustment < 0L)
            denominator = denominator.ShiftLeftDecimal(-exponentAdjustment);

        Nat ten = new Nat(10UL);

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
        // The operands were aligned to a single exponent above, so the decimal point sits exactly
        // fracLen digits from the end of the digit string: no leftover scale term.
        long resultExponent = -fracLen;

        if (!Nat.TryParse(allDigits, null, out Nat? mag))
            mag = Nat.Zero;

        bool actualNeg = resultNeg && !Nat.IsZero(mag);
        var divResult = new Real(mag, actualNeg, resultExponent,
                                 foundPeriod ? periodStart  : 0L,
                                 foundPeriod ? periodLength : 0L);
        return foundPeriod ? divResult : Normalize(divResult);
    }

    /// <inheritdoc cref="Divide"/>
    public static Real operator /(Real left, Real right) => Divide(left, right);

    /// <summary>
    /// Divides two non-periodic <see cref="Real"/> values and truncates the quotient to
    /// exactly <paramref name="fracDigits"/> fractional digits, using a single fixed-point
    /// integer division (no period detection). This is the fast path used by
    /// <see cref="PiTo(long)"/> and <see cref="Sqrt(Real, long)"/>, whose operands are
    /// irrational truncations and never produce a true repeating period.
    /// </summary>
    /// <exception cref="DivideByZeroException">Thrown when <paramref name="right"/> is zero.</exception>
    internal static Real DivideNonPeriodic(Real left, Real right, long fracDigits)
    {
        if (IsZero(right))
            throw new DivideByZeroException("Cannot divide a Real by zero.");

        if (IsZero(left))
            return Zero;

        bool resultNeg = IsNegative(left) != IsNegative(right);
        Nat numerator   = left.ToNatural();
        Nat denominator = right.ToNatural();
        long exponentAdjustment = left.Exponent - right.Exponent;

        // scaled = floor(numerator · 10^fracDigits / denominator) — one integer division.
        Nat scaled = Nat.DivRem(numerator.ShiftLeftDecimal(fracDigits), denominator, out _);

        return Normalize(new Real(scaled, resultNeg, -fracDigits + exponentAdjustment));
    }

    /// <summary>Returns the arithmetic negation of <paramref name="value"/>.
    /// Zero remains positive; for all other values the sign bit is flipped
    /// while magnitude, <see cref="Exponent"/>, <see cref="PeriodStart"/> and
    /// <see cref="PeriodLength"/> are preserved.
    /// Overrides <see cref="Int.Negate"/> returning <see cref="Real"/>.
    /// Corresponds to C++ <c>inverterSinal()</c> on <c>RealLovelace</c>.
    /// </summary>
    public static Real Negate(Real value)
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

    // -------------------------------------------------------------------------
    // Domain-specific operations — Sqrt
    // -------------------------------------------------------------------------

    /// <summary>
    /// Computes the principal square root of <paramref name="value"/> to
    /// <see cref="MaxComputationDecimalPlaces"/> fractional digits using Newton-Raphson
    /// (Heron's method): <c>x_{n+1} = (x_n + value / x_n) / 2</c>.
    /// Seeded from the <see cref="double"/> approximation; iterates until two consecutive
    /// results agree to <see cref="MaxComputationDecimalPlaces"/> decimal places.
    /// Delegates to the internal <c>Sqrt(Real, long)</c> overload.
    /// New C# addition with no C++ counterpart.
    /// </summary>
    /// <exception cref="ArithmeticException">Thrown when <paramref name="value"/> is negative.</exception>
    public static Real Sqrt(Real value) => Sqrt(value, MaxComputationDecimalPlaces, null);

    /// <summary>
    /// Computes the principal square root of each element in <paramref name="values"/> concurrently,
    /// dispatching each element to the thread pool via <see cref="Task.WhenAll"/>.
    /// Results are returned in the same order as the inputs.
    /// New C# addition with no C++ counterpart.
    /// </summary>
    /// <param name="values">The batch of values to compute square roots for.</param>
    /// <returns>An array of results in the same order as <paramref name="values"/>.</returns>
    /// <exception cref="AggregateException">
    /// Thrown when one or more elements cause an exception (e.g.,
    /// <see cref="ArithmeticException"/> for negative values).
    /// </exception>
    public static Real[] Sqrt(IReadOnlyList<Real> values)
    {
        if (values.Count == 0)
            return Array.Empty<Real>();

        var tasks = new Task<Real>[values.Count];
        for (int i = 0; i < values.Count; i++)
        {
            var v = values[i];
            tasks[i] = Task.Run(() => Sqrt(v));
        }
        return Task.WhenAll(tasks).GetAwaiter().GetResult();
    }

    /// <summary>
    /// Internal Newton-Raphson square root with caller-specified precision; imposes no
    /// additional cap on <paramref name="precision"/> beyond what the underlying arithmetic
    /// provides.  Used exclusively by <see cref="PiTo(long)"/> to compute <c>√10005</c> at
    /// guard-digit precision.
    /// </summary>
    /// <exception cref="ArithmeticException">Thrown when <paramref name="value"/> is negative.</exception>
    private static Real Sqrt(Real value, long precision, IProgress<double>? progress)
    {
        if (IsNegative(value))
            throw new ArithmeticException("Square root is not defined for negative numbers.");

        if (IsZero(value))
            return Zero;

        // Guard digits absorb truncation drift through Newton-Raphson iterations.
        const long guard = 50L;
        long targetPrecision = precision + guard;

        // Option A — Pre-expand periodic inputs: a periodic Real stores only one period
        // block in its backing Nat (e.g. 0.(3) is stored as Nat=3, Exp=-1, representing
        // 0.3 not 1/3).  Expanding here fixes both the seeding path (double.TryParse
        // fails on "0.(3)"-style notation) and the value/x division inside the NR loop
        // (which would otherwise compute sqrt(0.3) instead of sqrt(1/3)).
        if (value.IsPeriodic)
            value = ExpandToNonPeriodic(value, targetPrecision);

        // Option B — Exponent-aware seeding.
        // Builds the NR seed from value.ToNatural() digits and value.Exponent directly,
        // replacing the old 20-char ToString() truncation that silently collapsed
        // very small values to 0.0 (e.g. Nat="1", Exponent=-28 → string "0.0000…0001"
        // (30 chars) → truncated "0.000000000000000000" → 0.0 → fallback seed 1.0).
        // With a seed 14 orders of magnitude off, NR halves ~4 times and produces
        // ≈0.125 instead of 1e-14.  The new path uses the Exponent directly to place
        // the seed in the correct order of magnitude, eliminating the truncation loss.
        Real x;
        {
            string natDigits = value.ToNatural().ToString();
            int take = Math.Min(natDigits.Length, 15);
            double lead = double.Parse(natDigits[..take],
                                       System.Globalization.CultureInfo.InvariantCulture);
            long exp10 = (long)(natDigits.Length - take) + value.Exponent;
            // Normalise lead into [1, 10) and track the exponent offset.
            while (lead >= 10.0) { lead /= 10.0; exp10++; }
            while (lead > 0.0 && lead < 1.0) { lead *= 10.0; exp10--; }
            // Make exp10 even so that seedExp = exp10/2 is exact (C# truncates toward 0
            // for negative integers, which would lose half an order of magnitude).
            if (exp10 % 2 != 0) { lead *= 10.0; exp10--; }
            long seedExp = exp10 / 2;
            // sqrt(lead) ∈ [1, sqrt(10)) ≈ [1, 3.162…); "G15" never triggers scientific
            // notation for values in this range, so TryParse can always consume the string.
            double seedSig = Math.Sqrt(lead);
            string seedStr = seedSig.ToString("G15",
                                              System.Globalization.CultureInfo.InvariantCulture);
            if (!TryParse(seedStr, null, out Real? seedReal) || IsZero(seedReal))
                seedReal = One;
            // Compose the seed Real: seedReal encodes the significand; seedExp shifts
            // the order of magnitude so the combined value approximates sqrt(value).
            x = new Real(seedReal.ToNatural(), false, seedReal.Exponent + seedExp);
            if (IsZero(x) || IsNegative(x)) x = One;
        }

        Real two = new Real(2.0);

        // Progressive precision: double the working precision each iteration from the
        // seed's ~16 fractional digits up to targetPrecision.  Each iteration's
        // division is given enough budget for xFracDigits + currentTarget so the
        // quotient value/x has currentTarget significant fractional digits.
        long currentTarget = 16L;

        while (currentTarget < targetPrecision)
        {
            currentTarget = Math.Min(currentTarget * 2, targetPrecision);
            long xFracDigits = x.Exponent < 0 ? -x.Exponent : 0L;
            long divPrecision = xFracDigits + currentTarget;

            // value/x is an irrational truncation (x is a Newton iterate), so the
            // single fixed-point division fast path is exact here.
            x = (x + DivideNonPeriodic(value, x, divPrecision)) / two;
            progress?.Report((double)currentTarget / targetPrecision);

            // Exact convergence (perfect squares): if x*x == value, stop early.
            if (IsZero(x * x - value))
                return Normalize(new Real(x.ToNatural(), false, x.Exponent));
        }

        // Strip guard-digit tail so callers see exactly `precision` fractional digits.
        return Normalize(TruncateFracDigits(
            new Real(x.ToNatural(), false, x.Exponent), precision));

        // Truncates x to at most maxFrac fractional digits by dropping the tail.
        static Real TruncateFracDigits(Real x, long maxFrac)
        {
            long storedFrac = -x.Exponent;
            long toDrop = storedFrac - maxFrac;
            if (toDrop <= 0L) return x;

            string natStr = x.ToNatural().ToString();
            int keepLen = (int)((long)natStr.Length - toDrop);
            if (keepLen <= 0) return Real.Zero;

            string truncStr = natStr[..keepLen];
            if (!Nat.TryParse(truncStr, null, out Nat? truncNat))
                return x;
            return new Real(truncNat, false, x.Exponent + toDrop);
        }
    }

    // -------------------------------------------------------------------------
    // Domain-specific operations — Pi
    // -------------------------------------------------------------------------

    /// <summary>
    /// Computes π to <paramref name="digits"/> decimal places using the Chudnovsky algorithm:
    /// <c>π = 426880·√10005 / Σ_k [ (-1)^k·(6k)!·(13591409 + 545140134k) / ((3k)!·(k!)³·640320^(3k)) ]</c>.
    /// Each term contributes ~14.18 digits of precision.
    /// Internally computes with <c>digits + 10</c> guard digits (including
    /// passing <c>digits + 10</c> to the internal <see cref="Sqrt(Real, long)"/> overload)
    /// and truncates to <paramref name="digits"/> fractional places before returning.
    /// <para>
    /// The series is accumulated as an exact rational (Integer numerator / Natural denominator)
    /// to avoid precision loss from intermediate Real divisions.  Only the final step
    /// <c>426880 · √10005 · denS / numS</c> performs a Real division at guard-digit precision.
    /// </para>
    /// New C# addition with no C++ counterpart.
    /// </summary>
    /// <param name="digits">The number of fractional decimal places to compute. Must be between 1 and
    /// <see cref="MaxComputationDecimalPlaces"/> inclusive.</param>
    /// <returns>A non-negative, non-periodic <see cref="Real"/> with <see cref="Exponent"/> equal to
    /// <c>-digits</c>.</returns>
    /// <exception cref="ArgumentOutOfRangeException">Thrown when <paramref name="digits"/> is ≤ 0 or
    /// exceeds <see cref="MaxComputationDecimalPlaces"/>.</exception>
    public static Real PiTo(long digits, IProgress<double>? progress = null)
    {
        if (digits <= 0 || digits > MaxComputationDecimalPlaces)
            throw new ArgumentOutOfRangeException(nameof(digits));

        long guardDigits = digits + 10;

        // Combine the two concurrent phases (√10005 Newton chain and the BSP series) into one
        // fraction, each contributing half, so the bar advances with whichever phase is working
        // and never jumps to 100% while the other is still running.
        double sqrtDone = 0.0, bspDone = 0.0;
        object progressSync = new();
        IProgress<double>? sqrtProg = progress is null ? null : new InlineProgress<double>(f =>
        {
            lock (progressSync) { sqrtDone = f; progress.Report(0.9 * (sqrtDone + bspDone) / 2.0); }
        });
        IProgress<double>? bspProg = progress is null ? null : new InlineProgress<double>(f =>
        {
            lock (progressSync) { bspDone = f; progress.Report(0.9 * (sqrtDone + bspDone) / 2.0); }
        });

        // √10005 is independent of the Chudnovsky series; compute it concurrently so its
        // Newton-division chain overlaps the BSP term computation and merge.
        Task<Real> sqrtTask = Task.Run(() => Sqrt(new Real("10005"), guardDigits, sqrtProg));

        // Chudnovsky series accumulated via Binary Splitting (BSP) in parallel.
        // PiSegment(0, range) covers all terms 0..numTerms, producing (P, Q, T)
        // where Q = denS and T = numS from the original sequential recurrence.
        // Then π = 426880 · √10005 · Q / T.
        long numTerms = (long)Math.Ceiling((double)guardDigits / 14.0) + 2;
        long range = numTerms + 1; // half-open: [0, range)

        // Divide [0, range) into independent sub-ranges and compute concurrently.
        int degree = Math.Max(1, Math.Min(Environment.ProcessorCount, (int)Math.Min(range, 64L)));

        Nat denS;
        Int numS;

        if (degree <= 1 || range <= 1)
        {
            var (_, q, t) = PiSegment(0, range);
            denS = q; numS = t;
            bspProg?.Report(1.0);
        }
        else
        {
            long chunkSize = range / degree;
            var tasks = new Task<(Nat P, Nat Q, Int T)>[degree];

            // Progress is cost-weighted: the Chudnovsky terms grow super-exponentially with the
            // term index, so a chunk's cost is dominated by its largest term (~end). Weighting each
            // chunk by end^2 makes the bar move roughly proportionally to real work instead of
            // jumping to ~(degree-1)/degree and stalling on the final, dominant chunk.
            long[] chunkCost = new long[degree];
            long totalCost = 0;
            for (int i = 0; i < degree; i++)
            {
                long end = (i == degree - 1) ? range : (i + 1) * chunkSize;
                chunkCost[i] = end * end;
                totalCost += chunkCost[i];
            }

            long costDone = 0;
            for (int i = 0; i < degree; i++)
            {
                long start = i * chunkSize;
                long end = (i == degree - 1) ? range : start + chunkSize;
                int idx = i;
                tasks[i] = Task.Run(() =>
                {
                    var result = PiSegment(start, end);
                    long done = Interlocked.Add(ref costDone, chunkCost[idx]);
                    bspProg?.Report((double)done / totalCost);
                    return result;
                });
            }
            var segments = Task.WhenAll(tasks).GetAwaiter().GetResult();

            // Merge BSP triples left-to-right:
            // T(a,b) = T(a,m)·Q(m,b) + P(a,m)·T(m,b)
            var (accP, accQ, accT) = segments[0];
            for (int i = 1; i < segments.Length; i++)
            {
                var (rP, rQ, rT) = segments[i];
                accT = accT * new Int(rQ, false) + new Int(accP, false) * rT;
                accQ = accQ * rQ;
                accP = accP * rP;
            }
            denS = accQ; numS = accT;
        }

        Real sqrt10005 = sqrtTask.GetAwaiter().GetResult();

        // π = 426880 · √10005 · denS / numS. The operands are irrational truncations, so
        // use the single fixed-point division fast path (no period detection needed).
        Real realNumS = new Real(numS);
        Real realDenS = new Real(new Int(denS, false));
        Real numerator = new Real("426880") * sqrt10005 * realDenS;
        Real pi = DivideNonPeriodic(numerator, realNumS, guardDigits);

        progress?.Report(1.0);

        // Truncate to exactly `digits` fractional places.
        return TruncatePiFracDigits(pi, digits);

        static Real TruncatePiFracDigits(Real x, long maxFrac)
        {
            long storedFrac = -x.Exponent;
            long toDrop = storedFrac - maxFrac;
            if (toDrop <= 0L) return x;

            string natStr = x.ToNatural().ToString();
            int keepLen = (int)((long)natStr.Length - toDrop);
            if (keepLen <= 0) return Real.Zero;

            string truncStr = natStr[..keepLen];
            if (!Nat.TryParse(truncStr, null, out Nat? truncNat))
                return x;
            return new Real(truncNat, false, x.Exponent + toDrop);
        }
    }

    /// <summary>
    /// Computes Euler's number <c>e = Σ 1/k!</c> to <paramref name="digits"/> decimal places by
    /// summing the Taylor series until terms fall below a guard-digit threshold, then truncating.
    /// Mirrors <see cref="PiTo(long)"/> in structure (guard digits + final truncation).
    /// </summary>
    /// <param name="digits">The number of fractional decimal places to compute. Must be between 1 and
    /// <see cref="MaxComputationDecimalPlaces"/> inclusive.</param>
    /// <returns>A non-negative, non-periodic <see cref="Real"/> with <see cref="Exponent"/> equal to
    /// <c>-digits</c>.</returns>
    /// <exception cref="ArgumentOutOfRangeException">Thrown when <paramref name="digits"/> is ≤ 0 or
    /// exceeds <see cref="MaxComputationDecimalPlaces"/>.</exception>
    public static Real ETo(long digits, IProgress<double>? progress = null)
    {
        if (digits <= 0 || digits > MaxComputationDecimalPlaces)
            throw new ArgumentOutOfRangeException(nameof(digits));

        long guardDigits = digits + 10;

        // Choose n so the tail 1/n! < 10^-(guardDigits+1): n! >= 10^(guardDigits+1).
        Nat limit = new Nat(10UL).Pow(new Nat((ulong)(guardDigits + 1)));
        Nat nFact = Nat.One;
        long n = 1;
        while (nFact < limit)
        {
            n++;
            nFact = nFact * new Nat((ulong)n);
        }

        // e = Σ_{k=0}^{n} 1/k! = (Σ_{k=0}^{n} n!/k!) / n! — accumulate the exact rational,
        // then perform a single non-periodic division (mirrors PiTo's structure).
        Int sumNum = new Int(nFact, false);   // n!/0!
        Nat term = nFact;                     // n!/k!, starts at n!/0!
        for (long k = 1; k <= n; k++)
        {
            term = Nat.DivRem(term, new Nat((ulong)k), out _);   // n!/k!
            sumNum = sumNum + new Int(term, false);
        }

        Real e = DivideNonPeriodic(new Real(sumNum), new Real(new Int(nFact, false)), guardDigits);

        progress?.Report(1.0);
        return TruncateFracDigits(e, digits);

        static Real TruncateFracDigits(Real x, long maxFrac)
        {
            long storedFrac = -x.Exponent;
            long toDrop = storedFrac - maxFrac;
            if (toDrop <= 0L) return x;

            string natStr = x.ToNatural().ToString();
            int keepLen = (int)((long)natStr.Length - toDrop);
            if (keepLen <= 0) return new Real("0");

            string truncStr = natStr[..keepLen];
            if (!Nat.TryParse(truncStr, null, out Nat? truncNat))
                return x;
            return new Real(truncNat, false, x.Exponent + toDrop);
        }
    }

    /// <summary>
    /// Asynchronously computes Euler's number <c>e</c> to <paramref name="digits"/> decimal places
    /// by offloading the CPU-bound <see cref="ETo(long)"/> call to the thread pool.
    /// </summary>
    public static Task<Real> EToAsync(long digits) => EToAsync(digits, null);

    /// <summary>
    /// Asynchronously computes Euler's number <c>e</c>, reporting series progress (0..1).
    /// </summary>
    public static Task<Real> EToAsync(long digits, IProgress<double>? progress) => Task.Run(() => ETo(digits, progress));

    // -------------------------------------------------------------------------
    // Cached transcendental constants (upgrade-on-demand: the cache keeps the highest
    // precision computed so far; lower-precision scopes receive a partial version)
    // -------------------------------------------------------------------------

    private static readonly object s_constantsLock = new();
    private static Real? s_pi;
    private static long s_piDigits;
    private static Real? s_e;
    private static long s_eDigits;

    /// <summary>
    /// π (pi). Computed lazily at the active computation precision. The cache keeps the highest
    /// precision computed so far and upgrades on demand, so a low-precision scope (e.g. a plugin's
    /// fast default budget) can never pin the process-wide constant at a low digit count.
    /// Lower-precision scopes receive a partial (truncated) version of the cached value — the
    /// prefix of a higher-precision computation is exactly what computing at the lower precision
    /// would produce, so the result is identical to <see cref="PiTo(long)"/> at that precision.
    /// </summary>
    public static Real Pi => CachedConstant(ref s_pi, ref s_piDigits, digits => PiTo(digits));

    /// <summary>Euler's number <c>e</c>, cached with the same upgrade-on-demand policy as <see cref="Pi"/>.</summary>
    public static Real E => CachedConstant(ref s_e, ref s_eDigits, digits => ETo(digits));

    /// <summary>
    /// Shared cached-constant accessor. Upgrades the cache by computing at the active precision
    /// when the cached value is shorter, and serves a truncated partial version when the active
    /// precision is lower than the cached one (a lower-precision constant is just a prefix of the
    /// higher-precision one).
    /// </summary>
    private static Real CachedConstant(ref Real? cache, ref long cachedDigits, Func<long, Real> compute)
    {
        long need = MaxComputationDecimalPlaces;
        lock (s_constantsLock)
        {
            if (cachedDigits < need)
            {
                cache = compute(need);
                cachedDigits = need;
            }
            return need >= cachedDigits ? cache! : PartialFracDigits(cache!, need);
        }
    }

    /// <summary>Truncates <paramref name="x"/> to at most <paramref name="maxFrac"/> fractional digits by dropping the tail.</summary>
    private static Real PartialFracDigits(Real x, long maxFrac)
    {
        long storedFrac = -x.Exponent;
        long toDrop = storedFrac - maxFrac;
        if (toDrop <= 0L) return x;

        string natStr = x.ToNatural().ToString();
        long keepLen = natStr.Length - toDrop;
        if (keepLen <= 0) return new Real("0");

        string truncStr = natStr.Substring(0, (int)keepLen);
        if (!Nat.TryParse(truncStr, null, out Nat? truncNat)) return x;
        return new Real(truncNat, false, x.Exponent + toDrop);
    }

    // -------------------------------------------------------------------------
    // Domain-specific operations — Sin / Cos
    // -------------------------------------------------------------------------

    private static Real? s_sqrt2Half;
    private static long s_sqrt2HalfDigits;
    private static Real? s_sqrt3Half;
    private static long s_sqrt3HalfDigits;

    private static Real Sqrt2Half => CachedConstant(ref s_sqrt2Half, ref s_sqrt2HalfDigits,
        _ => Sqrt(new Real("2")) / new Real("2"));

    private static Real Sqrt3Half => CachedConstant(ref s_sqrt3Half, ref s_sqrt3HalfDigits,
        _ => Sqrt(new Real("3")) / new Real("2"));

    /// <summary>
    /// Computes sin(x) to <see cref="MaxComputationDecimalPlaces"/> decimal places. Angles that are
    /// exact rational multiples of the cached <see cref="Pi"/> hit the special-angle table and
    /// return exact rational or <see cref="Sqrt(Real)"/>-based values; all other angles use a
    /// Taylor series at the active precision. No IEEE floating point is used.
    /// </summary>
    public static Real Sin(Real value) => Sin(value, MaxComputationDecimalPlaces, null);

    /// <summary>Computes sin(x) to <paramref name="digits"/> decimal places.</summary>
    public static Real Sin(Real value, long digits) => Sin(value, digits, null);

    /// <summary>
    /// Computes cos(x) to <see cref="MaxComputationDecimalPlaces"/> decimal places, with the same
    /// special-angle exactness as <see cref="Sin(Real)"/>.
    /// </summary>
    public static Real Cos(Real value) => Cos(value, MaxComputationDecimalPlaces, null);

    /// <summary>Computes cos(x) to <paramref name="digits"/> decimal places.</summary>
    public static Real Cos(Real value, long digits) => Cos(value, digits, null);

    private static Real Sin(Real value, long digits, IProgress<double>? progress)
    {
        ArgumentNullException.ThrowIfNull(value);
        if (digits <= 0 || digits > MaxComputationDecimalPlaces)
            throw new ArgumentOutOfRangeException(nameof(digits));

        // sin is odd: sin(−x) = −sin(x), exactly, so a negative angle is the negated sine of its
        // magnitude.  Routing it through the reduction instead rebuilds the angle as 2π + value at
        // the ACTIVE scale: with π/6 truncated at the precision cap, 2π − π/6 lands one unit in the
        // last place above 11π/6, TrySpecialAngle's exact comparison misses, and an angle that has
        // an exact value silently falls through to the Taylor path (−0.4999…484 instead of −0.5).
        // Negating the positive-argument result keeps the symmetry exact at every precision.
        if (value < Zero)
            return -Sin(-value, digits, progress);

        long guard = digits + 10;
        Real pi = Pi;
        Real twoPi = pi * new Real("2");
        Real halfPi = HalfOf(pi);

        Real x = ReduceToTwoPi(value, pi, twoPi);

        if (TrySpecialAngle(x, pi, out Real sinValue, out _))
            return sinValue;

        bool negative = false;
        if (x <= pi) { if (x > halfPi) x = pi - x; }
        else if (x <= pi + halfPi) { x = x - pi; negative = true; }
        else { x = twoPi - x; negative = true; }

        Real sin = SinTaylor(x, guard);
        return negative ? -sin : sin;
    }

    private static Real Cos(Real value, long digits, IProgress<double>? progress)
    {
        ArgumentNullException.ThrowIfNull(value);
        if (digits <= 0 || digits > MaxComputationDecimalPlaces)
            throw new ArgumentOutOfRangeException(nameof(digits));

        // cos is even: cos(−x) = cos(x), exactly, for the same reason Sin uses its odd symmetry —
        // the reduction would otherwise place the negated angle one unit in the last place away
        // from the special angle it is supposed to match.
        if (value < Zero)
            return Cos(-value, digits, progress);

        long guard = digits + 10;
        Real pi = Pi;
        Real twoPi = pi * new Real("2");
        Real halfPi = HalfOf(pi);

        Real x = ReduceToTwoPi(value, pi, twoPi);

        if (TrySpecialAngle(x, pi, out _, out Real cosValue))
            return cosValue;

        bool negate = false;
        if (x <= pi) { if (x > halfPi) { x = pi - x; negate = true; } }
        else if (x <= pi + halfPi) { x = x - pi; negate = true; }
        else { x = twoPi - x; }

        Real cos = CosTaylor(x, guard);
        return negate ? -cos : cos;
    }

    /// <summary>Reduces <paramref name="value"/> into [0, 2π) without truncating small angles.</summary>
    private static Real ReduceToTwoPi(Real value, Real pi, Real twoPi)
    {
        if (value >= Zero && value < twoPi)
            return value;
        Real reduced = value - twoPi * WholeQuotient(value, twoPi);
        return reduced < Zero ? reduced + twoPi : reduced;
    }

    /// <summary>
    /// The integer part of <paramref name="left"/> / <paramref name="right"/> (truncated toward
    /// zero), computed with a single integer division instead of <see cref="Divide"/>'s
    /// remainder-tracking decimal loop.
    /// <para>
    /// The value is identical to <c>Truncate(Divide(left, right))</c> for finite operands: Divide
    /// yields the exact rational quotient truncated at <see cref="MaxComputationDecimalPlaces"/>
    /// further places, and truncating that again at the decimal point is the same as truncating the
    /// exact quotient (nested floor), whatever the remainder did — including the case where the
    /// digit loop stopped early on a repeating remainder, since a detected period still represents
    /// the exact quotient.  Operands carrying a period have no finite magnitude to divide as
    /// integers, so they keep the original path.
    /// </para>
    /// </summary>
    private static Real WholeQuotient(Real left, Real right)
    {
        if (left.IsPeriodic || right.IsPeriodic)
            return Truncate(Divide(left, right));

        Nat numerator   = left.ToNatural();
        Nat denominator = right.ToNatural();
        long exponentAdjustment = left.Exponent - right.Exponent;
        if (exponentAdjustment > 0L)
            numerator = numerator.ShiftLeftDecimal(exponentAdjustment);
        else if (exponentAdjustment < 0L)
            denominator = denominator.ShiftLeftDecimal(-exponentAdjustment);

        Nat whole = Nat.DivRem(numerator, denominator, out _);
        bool isNeg = Int.IsNegative(left) != Int.IsNegative(right) && !Nat.IsZero(whole);
        return new Real(whole, isNeg, 0L);
    }

    /// <summary>
    /// Exactly <c><paramref name="value"/> / 2</c>.  Halving a decimal only moves its point one
    /// place, and <paramref name="value"/> here is stored at the active computation precision
    /// (<see cref="Pi"/> and <see cref="E"/> are), so one integer halving of the magnitude
    /// reproduces <c>Divide(value, 2)</c> digit for digit — Divide truncates the exact rational
    /// quotient at the active precision, which for this operand shape is exactly
    /// <c>floor(magnitude / 2)</c> at the operand's own scale.  Anything not in that shape keeps
    /// the general path.
    /// </summary>
    private static Real HalfOf(Real value)
    {
        if (value.IsPeriodic || value.Exponent != -MaxComputationDecimalPlaces)
            return value / new Real("2");

        Nat halved = Nat.DivRem(value.ToNatural(), new Nat(2UL), out _);
        return Normalize(new Real(halved, Int.IsNegative(value), value.Exponent));
    }

    /// <summary>
    /// Returns the exact sin/cos for the 16 special angles that are rational multiples of π
    /// (multiples of π/6 and π/4 in [0, 2π)), or <see langword="false"/> for any other angle.
    /// <para>
    /// Each candidate angle is <c>π·num/den</c> at π's own scale, which — because Divide aligns the
    /// operands to a single exponent — is exactly <c>floor(π's magnitude · num / den)</c> at that
    /// scale, so one integer division reproduces the value that <c>pi * num / den</c> produces
    /// without Divide's remainder-tracking decimal loop.  A non-periodic <paramref name="x"/> whose
    /// own last stored place sits above the angle's (the angle is normalised, so its last stored
    /// digit is non-zero) cannot equal it and is rejected before the comparison is attempted.
    /// </para>
    /// </summary>
    private static bool TrySpecialAngle(Real x, Real pi, out Real sin, out Real cos)
    {
        sin = Zero;
        cos = Zero;

        Real half = new Real("0.5");
        Real negHalf = new Real("-0.5");
        Real negOne = new Real("-1");
        Real sqrt2Half = Sqrt2Half;
        Real sqrt3Half = Sqrt3Half;

        // (numerator, denominator, sin, cos) for multiples of π/6 and π/4 in [0, 2π).
        (long Num, long Den, Real Sin, Real Cos)[] table =
        {
            (0, 1, Zero, One),
            (1, 6, half, sqrt3Half),
            (1, 4, sqrt2Half, sqrt2Half),
            (1, 3, sqrt3Half, half),
            (1, 2, One, Zero),
            (2, 3, sqrt3Half, negHalf),
            (3, 4, sqrt2Half, -sqrt2Half),
            (5, 6, half, -sqrt3Half),
            (1, 1, Zero, negOne),
            (7, 6, negHalf, -sqrt3Half),
            (5, 4, -sqrt2Half, -sqrt2Half),
            (4, 3, -sqrt3Half, negHalf),
            (3, 2, negOne, Zero),
            (5, 3, -sqrt3Half, half),
            (7, 4, -sqrt2Half, sqrt2Half),
            (11, 6, negHalf, sqrt3Half),
        };

        Nat piMagnitude = pi.ToNatural();

        foreach (var (num, den, s, c) in table)
        {
            Real angle = num == 0
                ? Zero
                : Normalize(new Real(Nat.DivRem(piMagnitude * new Nat((ulong)num), new Nat((ulong)den), out _),
                                      false, pi.Exponent));

            if (!x.IsPeriodic && x.Exponent > angle.Exponent)
                continue;

            if (x.Equals(angle))
            {
                sin = s;
                cos = c;
                return true;
            }
        }
        return false;
    }

    /// <summary>sin(x) = x − x³/3! + x⁵/5! − … for |x| ≤ π/2.</summary>
    private static Real SinTaylor(Real x, long guard)
    {
        Real x2 = x * x;
        Real term = x;
        Real sum = x;

        Nat? tenPower = null;
        long tenPowerExponent = 0L;

        for (long k = 1; ; k++)
        {
            long denom = (2 * k) * (2 * k + 1);
            term = DivideNonPeriodic(-term * x2, new Real(new Int(denom)), guard);
            sum = sum + term;
            if (IsBelowDecimalGuard(term, guard, ref tenPower, ref tenPowerExponent))
                break;
        }
        return sum;
    }

    /// <summary>cos(x) = 1 − x²/2! + x⁴/4! − … for |x| ≤ π/2.</summary>
    private static Real CosTaylor(Real x, long guard)
    {
        Real x2 = x * x;
        Real term = One;
        Real sum = One;

        Nat? tenPower = null;
        long tenPowerExponent = 0L;

        for (long k = 1; ; k++)
        {
            long denom = (2 * k - 1) * (2 * k);
            term = DivideNonPeriodic(-term * x2, new Real(new Int(denom)), guard);
            sum = sum + term;
            if (IsBelowDecimalGuard(term, guard, ref tenPower, ref tenPowerExponent))
                break;
        }
        return sum;
    }

    /// <summary>
    /// The Taylor loops' termination test <c>|term| &lt; 10^-(guard+1)</c> — the literal threshold was
    /// <c>"0." + guard zeros + "1"</c> — decided exactly with integer arithmetic.  Multiplying both
    /// sides by <c>10^-Exponent</c> turns it into <c>magnitude &lt; 10^k</c> with
    /// <c>k = -(guard+1) - Exponent</c> (the term's magnitude is at least one, so <c>k ≤ 0</c>
    /// settles it immediately), which is one <see cref="Nat"/> comparison.
    /// <para>
    /// <paramref name="tenPower"/> carries <c>10^k</c> over from the previous term: as the series
    /// converges its terms' exponents fall by a fixed step, so k only ever grows and the next power
    /// is one multiply by <c>10^(k - previous k)</c> — a short multiply of the carried value.  The
    /// equivalent <c>Abs(term) &lt; threshold</c> had to render both operands in decimal, which at
    /// the default precision means rendering a 200k-digit magnitude once per term.
    /// </para>
    /// </summary>
    private static bool IsBelowDecimalGuard(Real term, long guard, ref Nat? tenPower, ref long tenPowerExponent)
    {
        if (Int.IsZero(term))
            return true; // 0 < 10^-(guard+1)

        long k = -(guard + 1L) - term.Exponent;
        if (k <= 0L)
            return false; // |term| >= 10^Exponent >= 10^-(guard+1)

        if (tenPower is null || tenPowerExponent > k)
        {
            tenPower = TenToThe(k);
            tenPowerExponent = k;
        }
        else if (tenPowerExponent < k)
        {
            tenPower = tenPower * TenToThe(k - tenPowerExponent);
            tenPowerExponent = k;
        }

        return term.ToNatural().CompareTo(tenPower) < 0;
    }

    // -------------------------------------------------------------------------
    // Domain-specific operations — Exp
    // -------------------------------------------------------------------------

    /// <summary>
    /// Computes exp(x) = eˣ to <see cref="MaxComputationDecimalPlaces"/> decimal places using a
    /// Taylor series with argument reduction. No IEEE floating point is used.
    /// </summary>
    public static Real Exp(Real value) => Exp(value, MaxComputationDecimalPlaces, null);

    /// <summary>Computes exp(x) to <paramref name="digits"/> decimal places.</summary>
    public static Real Exp(Real value, long digits) => Exp(value, digits, null);

    private static Real Exp(Real value, long digits, IProgress<double>? progress)
    {
        ArgumentNullException.ThrowIfNull(value);
        if (digits <= 0 || digits > MaxComputationDecimalPlaces)
            throw new ArgumentOutOfRangeException(nameof(digits));
        long guard = digits + 10;

        if (value.Equals(Zero))
            return One;

        // exp(x) = exp(x / 2^k)^(2^k): halve until |x/2^k| < 1 for fast Taylor convergence.
        long n = 1;
        Real y = value;
        while (Abs(y) >= One)
        {
            y = y / new Real("2");
            n <<= 1;
        }

        Real expY = ExpTaylor(y, guard);
        return expY.Pow(new Real(new Int(n)));
    }

    /// <summary>exp(y) = Σ yᵏ/k! for |y| &lt; 1.</summary>
    private static Real ExpTaylor(Real y, long guard)
    {
        Real term = One;
        Real sum = One;
        Real threshold = new Real("0." + new string('0', (int)guard) + "1");

        for (long k = 1; ; k++)
        {
            term = DivideNonPeriodic(term * y, new Real(new Int(k)), guard);
            sum = sum + term;
            if (Abs(term) < threshold)
                break;
        }
        return sum;
    }

    /// <summary>
    /// Asynchronously computes π to <paramref name="digits"/> decimal places by offloading
    /// the CPU-bound <see cref="PiTo(long)"/> call to the thread pool via <see cref="Task.Run"/>.
    /// New C# addition with no C++ counterpart.
    /// </summary>
    /// <param name="digits">The number of fractional decimal places to compute. Must be between 1 and
    /// <see cref="MaxComputationDecimalPlaces"/> inclusive.</param>
    /// <returns>A <see cref="Task{Real}"/> that completes with the same value as <see cref="PiTo(long)"/>.</returns>
    /// <exception cref="ArgumentOutOfRangeException">Propagated from <see cref="PiTo(long)"/> when
    /// <paramref name="digits"/> is ≤ 0 or exceeds <see cref="MaxComputationDecimalPlaces"/>.</exception>
    public static Task<Real> PiToAsync(long digits) => PiToAsync(digits, null);

    /// <summary>
    /// Asynchronously computes π, reporting series-chunk progress (0..1).
    /// </summary>
    public static Task<Real> PiToAsync(long digits, IProgress<double>? progress) => Task.Run(() => PiTo(digits, progress));

    /// <summary>
    /// Asynchronously computes the square root of <paramref name="value"/> by offloading
    /// the CPU-bound <see cref="Sqrt(Real)"/> call to the thread pool via <see cref="Task.Run"/>.
    /// New C# addition with no C++ counterpart.
    /// </summary>
    /// <param name="value">The value whose square root to compute. Must be non-negative.</param>
    /// <returns>A <see cref="Task{Real}"/> that completes with the same value as <see cref="Sqrt(Real)"/>.</returns>
    /// <exception cref="ArithmeticException">Propagated from <see cref="Sqrt(Real)"/> when
    /// <paramref name="value"/> is negative.</exception>
    public static Task<Real> SqrtAsync(Real value) => SqrtAsync(value, null);

    /// <summary>
    /// Asynchronously computes the square root, reporting Newton-iteration progress (0..1).
    /// </summary>
    public static Task<Real> SqrtAsync(Real value, IProgress<double>? progress) => Task.Run(() => Sqrt(value, MaxComputationDecimalPlaces, progress));

    // -------------------------------------------------------------------------
    // Binary Splitting (BSP) decomposition — internal helper for Pi
    // -------------------------------------------------------------------------

    /// <summary>
    /// Computes the Binary Splitting (BSP) triple <c>(P, Q, T)</c> for the Chudnovsky series
    /// over the half-open term range [<paramref name="termStart"/>, <paramref name="termEnd"/>).
    /// <list type="bullet">
    ///   <item><c>P(a,b)</c> = ∏ a_k for k ∈ [a,b), where a_0 = 1</item>
    ///   <item><c>Q(a,b)</c> = ∏ b_k for k ∈ [a,b), where b_0 = 1</item>
    ///   <item><c>T(a,b)</c> = the Chudnovsky partial-sum numerator contribution</item>
    /// </list>
    /// Satisfies the BSP merge identity:
    /// <c>T(a,b) = T(a,m)·Q(m,b) + P(a,m)·T(m,b)</c>.
    /// Calling <c>PiSegment(0, numTerms+1)</c> produces <c>T = numS</c> and <c>Q = denS</c>
    /// equivalent to the sequential Chudnovsky loop, so <c>π = 426880·√10005·Q/T</c>.
    /// </summary>
    internal static (Nat P, Nat Q, Int T) PiSegment(long termStart, long termEnd)
    {
        const long A = 13591409L;
        const long B = 545140134L;

        if (termEnd - termStart == 1)
        {
            long k = termStart;
            if (k == 0)
                return (Nat.One, Nat.One, new Int(new Nat((ulong)A), false));

            long k6 = 6 * k;
            long k3 = 3 * k;

            // a_k = (6k)(6k-1)(6k-2)(6k-3)(6k-4)(6k-5)
            Nat ak = new Nat((ulong)(k6 * (k6 - 1) * (k6 - 2)))
                   * new Nat((ulong)((k6 - 3) * (k6 - 4) * (k6 - 5)));

            // b_k = (3k)(3k-1)(3k-2) · k³ · 640320³
            Nat bk = new Nat((ulong)(k3 * (k3 - 1) * (k3 - 2)))
                   * new Nat((ulong)(k * k * k)) * s_chudnovskyC3;

            // T(k, k+1) = (-1)^k · a_k · (A + B·k)
            Nat ck = new Nat((ulong)(A + B * k));
            Int tk = new Int(ak * ck, k % 2 == 1);

            return (ak, bk, tk);
        }

        long mid = (termStart + termEnd) / 2;
        var (lP, lQ, lT) = PiSegment(termStart, mid);
        var (rP, rQ, rT) = PiSegment(mid, termEnd);

        // Merge: P = P_L·P_R, Q = Q_L·Q_R, T = T_L·Q_R + P_L·T_R
        return (lP * rP, lQ * rQ, lT * new Int(rQ, false) + new Int(lP, false) * rT);
    }

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
    public static bool TryParse(
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
    public static bool TryParse(
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
    public static Real Parse(string s) => Parse(s, null);

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
    public static bool TryParse([NotNullWhen(true)] string? s, NumberStyles style, IFormatProvider? provider,
        [MaybeNullWhen(false)] out Real result)
        => TryParse(s, provider, out result);

    /// <inheritdoc/>
    public static bool TryParse(ReadOnlySpan<char> s, NumberStyles style, IFormatProvider? provider,
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
            // Emit non-repeating part then (period), read through GetDecimalDigit — the accessor
            // that owns the period arithmetic (wrap-around and out-of-stored-range zeros).  Slicing
            // the digit string with PeriodStart/PeriodLength directly assumes the magnitude stores
            // every position those two name; when it does not, the slice ran past the end of the
            // string and ToString threw ArgumentOutOfRangeException ("Index and length must refer
            // to a location within the string. (Parameter 'length')") for the value instead of
            // rendering it.
            long fracLen = -Exponent;
            string padded = digits.Length < fracLen ? digits.PadLeft((int)fracLen, '0') : digits;
            int splitAt = Math.Max(0, (int)(padded.Length - fracLen));
            string intPart = splitAt == 0 ? "0" : padded[..splitAt];

            var renderedFraction = new System.Text.StringBuilder();
            for (long position = 0; position < PeriodStart; position++)
                renderedFraction.Append((char)('0' + GetDecimalDigit(position)));
            string nonRepeating = renderedFraction.ToString();
            renderedFraction.Clear();
            for (long offset = 0; offset < PeriodLength; offset++)
                renderedFraction.Append((char)('0' + GetDecimalDigit(PeriodStart + offset)));
            string period = renderedFraction.ToString();

            // A periodic value always carries a fractional part (PeriodLength > 0), so the decimal
            // point is always emitted: "1.(1)", never "1(1)".
            return sign + intPart + "." + nonRepeating + "(" + period + ")";
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
            if (idx < 0 || idx >= (long)digits.Length) return 0;
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
    /// <para>
    /// The magnitude is shifted with integer arithmetic (<see cref="Nat.ShiftLeftDecimal"/>) rather
    /// than by appending characters to the rendered digits and parsing them back: the shifted value
    /// is the same integer, and this runs on every addition that has to align two exponents — for
    /// the trigonometric series that meant a render-and-reparse of a 200k-digit magnitude per term.
    /// </para>
    /// </summary>
    private Int ToInteger(long zeros)
    {
        if (zeros < 0) throw new ArgumentOutOfRangeException(nameof(zeros));
        Nat magnitude = ToNatural();
        if (zeros > 0L)
            magnitude = magnitude.ShiftLeftDecimal(zeros);
        return new Int(magnitude, Int.IsNegative(this));
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
    /// <para>
    /// The trailing zeros are counted by decimal division on the binary magnitude rather than by
    /// rendering the digits and parsing them back: the digit string of a value carries as many
    /// places as the value has digits, and this routine runs after every multiply, divide and add,
    /// so a rendering here costs O(digits) per arithmetic step (at the default precision the
    /// trigonometric series moved ~200k-digit magnitudes through it once per term).
    /// </para>
    /// </summary>
    private static Real Normalize(Real r)
    {
        if (r.IsPeriodic || r.Exponent >= 0 || Int.IsZero(r))
            return r;

        long maxStrip = -r.Exponent; // number of fractional digit slots stored
        Nat magnitude = StripTrailingDecimalZeros(r.ToNatural(), maxStrip, out long stripped);
        if (stripped == 0L)
            return r;

        bool isNeg = Int.IsNegative(r) && !Nat.IsZero(magnitude);
        return new Real(magnitude, isNeg, r.Exponent + stripped);
    }

    /// <summary>Upper bound on the galloping strip step (10^2^±40 is far past any storable value).</summary>
    private const long MaxStripStep = 1L << 40;

    /// <summary>
    /// Returns <paramref name="value"/> with up to <paramref name="cap"/> trailing decimal zeros
    /// removed, and reports how many were removed in <paramref name="stripped"/>.  Purely binary:
    /// a zero can only be stripped when the magnitude divides by ten exactly, and the step doubles
    /// after each success and halves after each failure, so an ordinary magnitude (no trailing
    /// zeros) costs a handful of short divisions, a long zero run costs O(log) of them, and no
    /// decimal rendering happens at all.  Equivalent to counting the trailing <c>'0'</c> characters
    /// of the decimal representation, which is what <see cref="Exponent"/> bookkeeping requires.
    /// </summary>
    private static Nat StripTrailingDecimalZeros(Nat value, long cap, out long stripped)
    {
        stripped = 0L;
        long step = 18L; // 10^18 is the largest power of ten that fits in one 64-bit limb

        while (stripped < cap && step > 0L)
        {
            long trial = Math.Min(step, cap - stripped);
            Nat quotient = Nat.DivRem(value, TenToThe(trial), out Nat remainder);

            if (Nat.IsZero(remainder))
            {
                value = quotient;
                stripped += trial;
                if (step < MaxStripStep)
                    step <<= 1;
            }
            else if (trial > 1L)
            {
                step = trial >> 1;
            }
            else
            {
                break; // the magnitude's last digit is not a zero
            }
        }

        return value;
    }

    /// <summary>Returns 10^<paramref name="exponent"/> as a <see cref="Nat"/> by binary
    /// exponentiation — the power-of-ten the series termination test needs, built without a
    /// decimal round trip.</summary>
    private static Nat TenToThe(long exponent)
    {
        Nat result = new(1UL);
        Nat square = new(10UL);
        ulong e = (ulong)exponent;
        while (e > 0UL)
        {
            if ((e & 1UL) != 0UL)
                result = result * square;
            e >>= 1;
            if (e != 0UL)
                square = square * square;
        }
        return result;
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

        // An all-zero period repeats nothing: 0.(0) is 0 and 3.0(0) is 3, so the canonical form
        // carries no period and no trailing zeros.  This is the shape the periodic guard's finite
        // expansion leaves behind — dividing by an operand expanded to `workingFrac` digits yields
        // an exact result whose fraction is zeros followed by one truncation digit, which the
        // slack=1 retry above reports as a zero period.
        bool allZeros = true;
        foreach (char c in periodStr) { if (c != '0') { allZeros = false; break; } }

        if (allZeros)
        {
            string nonRepeating = fracPart[..(int)pStart];
            string combined = intPart + nonRepeating;
            if (!Nat.TryParse(combined, null, out var zMag))
                zMag = Nat.Zero;
            bool isNeg = Int.IsNegative(r) && !Nat.IsZero(zMag);
            return Normalize(new Real(zMag, isNeg, -(long)nonRepeating.Length));
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
