using System.Globalization;
using System.Numerics;
using Nat = global::Lovelace.Natural.Natural;

namespace Lovelace.Real;

/// <summary>
/// Thrown when an operation on <see cref="LReal64"/> (or <see cref="LReal128"/>) cannot be
/// represented exactly within the fixed significand width. The caller should promote to the
/// arbitrary-precision <see cref="Real"/> type instead of rounding.
/// </summary>
public sealed class LRealPromoteException : Exception
{
    public LRealPromoteException(string message) : base(message) { }
}

/// <summary>
/// Fixed-width, exact-decimal real number with a 64-bit significand (up to 19 significant
/// digits) and exact periodic-fraction support. Value = significand × 10^exponent.
/// Mirrors <see cref="Real"/>'s observable decimal/period semantics; throws
/// <see cref="LRealPromoteException"/> rather than rounding when a result needs more than
/// 19 significant digits (or a period longer than 19 digits).
/// </summary>
public readonly struct LReal64 : IComparable<LReal64>, IEquatable<LReal64>
{
    public const int MaxSignificantDigits = 19;

    // Working fractional digits used when expanding periodic operands (19 sig = 1 int + 18 frac).
    private const int WorkingFractionalDigits = 18;

    /// <summary>Display precision used by <see cref="ToString"/> for non-periodic fractions. Defaults to float-class (7 fractional digits); storage remains 19 significant digits (exact).</summary>
    public static int DisplayDecimalPlaces { get; set; } = 7;

    private readonly ulong _sig;   // significant digits (leading/trailing zeros stripped when non-periodic)
    private readonly int _exp;     // decimal exponent
    private readonly int _pStart;  // period start (fractional digit index); meaningful when periodic
    private readonly int _pLen;    // period length; 0 => non-periodic
    private readonly bool _neg;

    private LReal64(ulong sig, int exp, int pStart, int pLen, bool neg)
    {
        _sig = sig; _exp = exp; _pStart = pStart; _pLen = pLen; _neg = neg;
    }

    public static LReal64 Zero => default;
    public static LReal64 One => new(1UL, 0, 0, 0, false);
    public static LReal64 NegativeOne => new(1UL, 0, 0, 0, true);

    public bool IsZero => _sig == 0;
    public bool IsNegative => _neg && _sig != 0;
    public bool IsPeriodic => _pLen > 0;
    public bool IsInteger => !IsPeriodic && _exp >= 0;

    /// <summary>Decimal exponent (value = significand × 10^exponent).</summary>
    public int Exponent => _exp;
    /// <summary>Significant digits as an unsigned integer.</summary>
    public ulong Significand => _sig;

    /// <summary>Converts an arbitrary-precision <see cref="Real"/> to this type if it fits (≤ 19 significant digits).</summary>
    public static bool TryFromReal(Real r, out LReal64 result)
    {
        result = default;
        if (r.Exponent > int.MaxValue || r.Exponent < int.MinValue) return false;
        string digits = r.ToNatural().ToString();
        if (!ulong.TryParse(digits, NumberStyles.None, CultureInfo.InvariantCulture, out ulong sig)) return false;
        result = new LReal64(sig, (int)r.Exponent, (int)r.PeriodStart, (int)r.PeriodLength, Real.IsNegative(r));
        return true;
    }

    /// <summary>Converts this value back to the arbitrary-precision <see cref="Real"/> type.</summary>
    public Real ToReal()
    {
        if (_sig == 0) return Real.Zero;
        return new Real(Nat.Parse(_sig.ToString(), null), _neg, _exp, _pStart, _pLen);
    }

    private static LRealPromoteException Promote() =>
        new($"LReal64: result requires more than {MaxSignificantDigits} significant digits; promote to Real.");

    // ------------------------------------------------------------------
    // Parsing
    // ------------------------------------------------------------------

    public static LReal64 Parse(string s) => Parse(s, null);
    public static LReal64 Parse(string s, IFormatProvider? provider)
    {
        if (!TryParse(s, provider, out var r))
            throw new FormatException($"The string '{s}' is not a valid decimal representation of a Real number.");
        return r;
    }

    public static bool TryParse(string? s, out LReal64 result) => TryParse(s, null, out result);

    public static bool TryParse(string? s, IFormatProvider? provider, out LReal64 result)
    {
        result = Zero;
        if (s is null) return false;
        s = s.Trim();
        if (s.Length == 0) return false;

        bool neg = false;
        if (s[0] == '-') { neg = true; s = s[1..]; }
        else if (s[0] == '+') { s = s[1..]; }
        if (s.Length == 0) return false;

        int dotPos = s.IndexOf('.');
        int parenOpen = s.IndexOf('(');
        int parenClose = s.LastIndexOf(')');

        string integerPart, nonRepeating, periodic;
        bool hasPeriod = false;

        if (dotPos < 0)
        {
            if (parenOpen >= 0) return false;
            integerPart = s; nonRepeating = ""; periodic = "";
        }
        else
        {
            integerPart = s[..dotPos];
            if (integerPart.Length == 0) return false; // reject ".5"
            if (parenOpen >= 0)
            {
                if (parenClose < 0) return false;
                if (parenOpen < dotPos + 1) return false;
                if (parenClose != s.Length - 1) return false;
                if (parenClose <= parenOpen + 1) return false;
                nonRepeating = s[(dotPos + 1)..parenOpen];
                periodic = s[(parenOpen + 1)..parenClose];
                hasPeriod = true;
            }
            else
            {
                nonRepeating = s[(dotPos + 1)..];
                periodic = "";
            }
        }

        foreach (char c in integerPart) if (c < '0' || c > '9') return false;
        foreach (char c in nonRepeating) if (c < '0' || c > '9') return false;
        foreach (char c in periodic) if (c < '0' || c > '9') return false;

        string allDigits = integerPart + nonRepeating + periodic;
        if (allDigits.Length == 0) return false;

        int exp, pStart = 0, pLen = 0;
        if (hasPeriod)
        {
            pStart = nonRepeating.Length;
            pLen = periodic.Length;
            exp = -(pStart + pLen);
        }
        else
        {
            exp = -nonRepeating.Length;
        }

        if (!UInt128.TryParse(allDigits, NumberStyles.None, CultureInfo.InvariantCulture, out var mag))
            return false;

        if (mag > ulong.MaxValue) return false; // > 19 significant digits
        bool actualNeg = neg && mag != 0;

        result = pLen == 0
            ? Normalize(new LReal64((ulong)mag, exp, 0, 0, actualNeg))
            : new LReal64((ulong)mag, exp, pStart, pLen, actualNeg);
        return true;
    }

    // ------------------------------------------------------------------
    // Normalization (strip trailing zeros for non-periodic, exp < 0)
    // ------------------------------------------------------------------

    private static LReal64 Normalize(LReal64 r)
    {
        if (r._pLen > 0 || r._exp >= 0 || r._sig == 0) return r;
        UInt128 s = r._sig;
        int e = r._exp;
        while (e < 0 && s % 10 == 0) { s /= 10; e++; }
        return new LReal64((ulong)s, e, 0, 0, r._neg);
    }

    private static LReal64 FromNormalized(UInt128 mag, int exp, bool neg)
    {
        if (mag == 0) return Zero;
        UInt128 s = mag; int e = exp;
        while (e < 0 && s % 10 == 0) { s /= 10; e++; }
        if (s > ulong.MaxValue) throw Promote();
        return new LReal64((ulong)s, e, 0, 0, neg);
    }

    // ------------------------------------------------------------------
    // Arithmetic
    // ------------------------------------------------------------------

    public static LReal64 Add(LReal64 a, LReal64 b)
    {
        if (a.IsZero) return b;
        if (b.IsZero) return a;

        if (a.IsPeriodic || b.IsPeriodic)
        {
            var ea = ExpandToNonPeriodic(a, WorkingFractionalDigits);
            var eb = ExpandToNonPeriodic(b, WorkingFractionalDigits);
            var raw = Add(ea, eb);
            return DetectAndNormalizePeriod(raw);
        }

        int resultExp = Math.Min(a._exp, b._exp);
        int shiftA = a._exp - resultExp;
        int shiftB = b._exp - resultExp;

        UInt128 magA = Shift(a._sig, shiftA);
        UInt128 magB = Shift(b._sig, shiftB);

        UInt128 sum; bool neg;
        if (a._neg == b._neg) { sum = magA + magB; neg = a._neg; }
        else if (magA >= magB) { sum = magA - magB; neg = a._neg; }
        else { sum = magB - magA; neg = b._neg; }

        return FromNormalized(sum, resultExp, neg);
    }

    public static LReal64 Subtract(LReal64 a, LReal64 b) => Add(a, Negate(b));

    public static LReal64 Multiply(LReal64 a, LReal64 b)
    {
        if (a.IsZero || b.IsZero) return Zero;

        if (a.IsPeriodic || b.IsPeriodic)
        {
            var ea = ExpandToNonPeriodic(a, WorkingFractionalDigits);
            var eb = ExpandToNonPeriodic(b, WorkingFractionalDigits);
            var raw = Multiply(ea, eb);
            return DetectAndNormalizePeriod(raw);
        }

        int exp = a._exp + b._exp;
        UInt128 prod = (UInt128)a._sig * b._sig;
        return FromNormalized(prod, exp, a._neg != b._neg);
    }

    public static LReal64 Divide(LReal64 a, LReal64 b)
    {
        if (b.IsZero) throw new DivideByZeroException("Cannot divide by zero.");
        if (a.IsZero) return Zero;

        if (a.IsPeriodic || b.IsPeriodic)
        {
            var ea = ExpandToNonPeriodic(a, WorkingFractionalDigits);
            var eb = ExpandToNonPeriodic(b, WorkingFractionalDigits);
            var raw = Divide(ea, eb);
            if (raw.IsPeriodic && raw._pLen >= WorkingFractionalDigits)
                raw = new LReal64(raw._sig, raw._exp, 0, 0, raw._neg);
            return DetectAndNormalizePeriod(raw);
        }

        bool neg = a._neg != b._neg;
        UInt128 num = a._sig, den = b._sig;
        UInt128 quot = num / den, rem = num % den;
        UInt128 sig = quot;

        int fracCount = 0;
        int expAdj = a._exp - b._exp;
        var history = new Dictionary<UInt128, int>();
        int pStart = 0, pLen = 0; bool found = false;

        const int MaxLoop = 256;
        while (rem != 0 && fracCount < MaxLoop)
        {
            if (history.TryGetValue(rem, out int firstPos))
            {
                pStart = firstPos; pLen = fracCount - firstPos; found = true; break;
            }
            history[rem] = fracCount;
            rem *= 10;
            UInt128 digit = rem / den;
            rem %= den;
            sig = sig * 10 + digit;
            if (sig > ulong.MaxValue) throw Promote(); // period/quotient exceeds width
            fracCount++;
        }

        int resultExp = -fracCount + expAdj;

        if (found)
        {
            if (pLen > MaxSignificantDigits) throw Promote();
            return new LReal64((ulong)sig, resultExp, pStart, pLen, neg);
        }
        if (rem == 0)
        {
            return FromNormalized(sig, resultExp, neg);
        }
        throw Promote(); // exhausted without terminating or a short period
    }

    public static LReal64 Negate(LReal64 v)
    {
        bool neg = v.IsZero ? false : !v._neg;
        return new LReal64(v._sig, v._exp, v._pStart, v._pLen, neg);
    }

    public static LReal64 Abs(LReal64 v) => v.IsNegative ? Negate(v) : v;

    // ------------------------------------------------------------------
    // Operators
    // ------------------------------------------------------------------

    public static LReal64 operator +(LReal64 a, LReal64 b) => Add(a, b);
    public static LReal64 operator -(LReal64 a, LReal64 b) => Subtract(a, b);
    public static LReal64 operator *(LReal64 a, LReal64 b) => Multiply(a, b);
    public static LReal64 operator /(LReal64 a, LReal64 b) => Divide(a, b);
    public static LReal64 operator -(LReal64 a) => Negate(a);

    public static bool operator ==(LReal64 a, LReal64 b) => a.Equals(b);
    public static bool operator !=(LReal64 a, LReal64 b) => !a.Equals(b);
    public static bool operator <(LReal64 a, LReal64 b) => a.CompareTo(b) < 0;
    public static bool operator >(LReal64 a, LReal64 b) => a.CompareTo(b) > 0;
    public static bool operator <=(LReal64 a, LReal64 b) => a.CompareTo(b) <= 0;
    public static bool operator >=(LReal64 a, LReal64 b) => a.CompareTo(b) >= 0;

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    private static UInt128 Shift(ulong sig, int n)
    {
        UInt128 r = sig;
        for (int i = 0; i < n; i++)
        {
            try { checked { r *= 10; } }
            catch (OverflowException) { throw Promote(); }
        }
        return r;
    }

    private int GetDecimalDigit(int position)
    {
        if (_exp >= 0) return 0;
        string digits = _sig.ToString();
        int fracLen = -_exp;
        if (!IsPeriodic || position < _pStart)
        {
            int idx = digits.Length - fracLen + position;
            if (idx < 0 || idx >= digits.Length) return 0;
            return digits[idx] - '0';
        }
        int periodicOffset = (position - _pStart) % _pLen;
        int pIdx = digits.Length - fracLen + _pStart + periodicOffset;
        if (pIdx < 0 || pIdx >= digits.Length) return 0;
        return digits[pIdx] - '0';
    }

    private static LReal64 ExpandToNonPeriodic(LReal64 r, int fracDigits)
    {
        string digits = r._sig.ToString();
        int fracLen = r._exp < 0 ? -r._exp : 0;
        int intPartLen = Math.Max(0, digits.Length - fracLen);
        string intPart = intPartLen > 0 ? digits[..intPartLen] : "0";

        var sb = new System.Text.StringBuilder(intPart.Length + fracDigits);
        sb.Append(intPart);
        for (int i = 0; i < fracDigits; i++)
            sb.Append((char)('0' + r.GetDecimalDigit(i)));

        if (!UInt128.TryParse(sb.ToString(), NumberStyles.None, CultureInfo.InvariantCulture, out var mag))
            throw Promote();
        if (mag > ulong.MaxValue) throw Promote();
        bool neg = r._neg && mag != 0;
        return new LReal64((ulong)mag, -fracDigits, 0, 0, neg);
    }

    private static LReal64 DetectAndNormalizePeriod(LReal64 r)
    {
        if (r._exp >= 0 || r._sig == 0) return r;
        string digits = r._sig.ToString();
        int fracLen = -r._exp;
        int intPartLen = Math.Max(0, digits.Length - fracLen);
        string intPart = intPartLen > 0 ? digits[..intPartLen] : "0";
        string fracPart = digits.Length > intPartLen ? digits[intPartLen..] : new string('0', fracLen);
        if (fracPart.Length < fracLen) fracPart = fracPart.PadLeft(fracLen, '0');

        var (pStart, pLen) = FindSmallestPeriod(fracPart);
        if (pLen == 0 && fracPart.Length > 2) (pStart, pLen) = FindSmallestPeriod(fracPart, 1);
        if (pLen == 0) return r;

        string periodStr = fracPart[pStart..(pStart + pLen)];
        bool allNines = periodStr.All(c => c == '9');

        if (allNines)
        {
            string nonRepeating = fracPart[..pStart];
            string combined = intPart + nonRepeating;
            UInt128 cMag = combined.Length == 0 ? 0 : UInt128.Parse(combined, NumberStyles.None, CultureInfo.InvariantCulture);
            cMag += 1;
            int newExp = -nonRepeating.Length;
            if (cMag > ulong.MaxValue) throw Promote();
            bool neg = r._neg && cMag != 0;
            return new LReal64((ulong)cMag, newExp, 0, 0, neg);
        }

        string stored = intPart + fracPart[..(pStart + pLen)];
        if (!UInt128.TryParse(stored, NumberStyles.None, CultureInfo.InvariantCulture, out var mag))
            throw Promote();
        if (mag > ulong.MaxValue) throw Promote();
        bool negative = r._neg && mag != 0;
        return new LReal64((ulong)mag, -(pStart + pLen), pStart, pLen, negative);
    }

    private static (int start, int len) FindSmallestPeriod(string fracPart, int slack = 0)
    {
        int n = fracPart.Length;
        int effectiveLen = n - slack;
        if (effectiveLen < 2) return (0, 0);
        for (int p = 1; p <= effectiveLen / 2; p++)
        {
            for (int s = 0; s + 2 * p <= effectiveLen; s++)
            {
                bool ok = true;
                for (int i = s + p; i < effectiveLen; i++)
                {
                    if (fracPart[i] != fracPart[s + (i - s) % p]) { ok = false; break; }
                }
                if (ok) return (s, p);
            }
        }
        return (0, 0);
    }

    // ------------------------------------------------------------------
    // Equality / comparison / formatting
    // ------------------------------------------------------------------

    public int CompareTo(LReal64 other)
    {
        bool thisNeg = IsNegative, otherNeg = other.IsNegative;
        if (!thisNeg && otherNeg) return 1;
        if (thisNeg && !otherNeg) return -1;

        string digitsA = _sig.ToString();
        string digitsB = other._sig.ToString();
        long intLenA = digitsA.Length + _exp;
        long intLenB = digitsB.Length + other._exp;
        int signMul = thisNeg ? -1 : 1;

        if (intLenA != intLenB)
            return (intLenA > intLenB ? 1 : -1) * signMul;

        long storedMax = Math.Max(digitsA.Length, digitsB.Length);
        long maxPositions = (IsPeriodic || other.IsPeriodic) ? storedMax + DisplayDecimalPlaces : storedMax;

        for (long i = 0; i < maxPositions; i++)
        {
            char dA = GetDigitAtPosition(this, i, intLenA, digitsA);
            char dB = GetDigitAtPosition(other, i, intLenB, digitsB);
            if (dA != dB) return (dA > dB ? 1 : -1) * signMul;
        }
        return 0;
    }

    private static char GetDigitAtPosition(LReal64 r, long pos, long intLen, string digits)
    {
        if (r.IsPeriodic && pos >= intLen)
            return (char)('0' + r.GetDecimalDigit((int)(pos - intLen)));
        return pos < digits.Length ? digits[(int)pos] : '0';
    }

    public bool Equals(LReal64 other)
    {
        if (IsPeriodic != other.IsPeriodic) return false;
        if (IsPeriodic) return ToString() == other.ToString();
        return CompareTo(other) == 0;
    }

    public override bool Equals(object? obj) => obj is LReal64 r && Equals(r);

    public override int GetHashCode() => ToString().GetHashCode();

    public override string ToString()
    {
        if (_sig == 0) return "0";
        string digits = _sig.ToString();
        string sign = _neg ? "-" : "";

        if (_exp == 0 && !IsPeriodic) return sign + digits;

        if (_exp < 0 && !IsPeriodic)
        {
            int fracLen = -_exp;
            if (fracLen >= digits.Length)
            {
                string padded = digits.PadLeft(fracLen, '0');
                string fracPart = padded[..Math.Min(fracLen, fracLen + DisplayDecimalPlaces)];
                return sign + "0." + fracPart;
            }
            else
            {
                int splitAt = digits.Length - fracLen;
                string intPart = digits[..splitAt];
                string fracPart = digits[splitAt..];
                if (fracPart.Length > DisplayDecimalPlaces) fracPart = fracPart[..DisplayDecimalPlaces];
                return sign + intPart + "." + fracPart;
            }
        }

        if (IsPeriodic)
        {
            int fracLen = -_exp;
            string padded = digits.Length < fracLen ? digits.PadLeft(fracLen, '0') : digits;
            int splitAt = Math.Max(0, padded.Length - fracLen);
            string intPart = splitAt == 0 ? "0" : padded[..splitAt];
            string allFrac = padded[splitAt..];
            string nonRepeating = _pStart <= allFrac.Length ? allFrac[.._pStart] : allFrac.PadRight(_pStart, '0');
            string period = allFrac.Length >= _pStart + _pLen
                ? allFrac[_pStart..(_pStart + _pLen)]
                : (digits.Length >= _pStart + _pLen ? digits[_pStart..(_pStart + _pLen)] : allFrac);
            return sign + intPart + (fracLen > 0 ? "." : "") + nonRepeating + "(" + period + ")";
        }

        return sign + digits;
    }
}
