using System.Globalization;
using Rl = global::Lovelace.Real.Real;
using Nat = global::Lovelace.Natural.Natural;
using Int = global::Lovelace.Integer.Integer;

namespace Lovelace.Complex;

/// <summary>
/// Arbitrary-precision elementary functions over <see cref="Rl"/> (real) and
/// <see cref="Complex"/> values (SYM-13b). Pure <see cref="Rl"/> arithmetic; deterministic;
/// no IEEE floating point. Series divisions use a truncating (non-periodic) division, and
/// results are computed at the ambient <see cref="Rl.MaxComputationDecimalPlaces"/> plus
/// internal guard digits, then truncated back to that precision on return.
/// </summary>
public static class ComplexMath
{
    /// <summary>Extra working digits carried internally so the final truncated result keeps the
    /// ambient precision (Ln's argument reduction amplifies error by ~2^16).</summary>
    private const long GuardDigits = 20;

    // ------------------------------------------------------------------
    // Real — exponential / logarithm
    // ------------------------------------------------------------------

    /// <summary>Computes <c>e^x</c> as <c>e^n · e^f</c> with <c>n = trunc(x)</c> and
    /// <c>|f| &lt; 1</c>; <c>e^f</c> comes from a Taylor series and <c>e^n</c> from binary
    /// exponentiation of <see cref="Rl.E"/>.</summary>
    public static Rl Exp(Rl x)
    {
        ArgumentNullException.ThrowIfNull(x);
        long digits = AmbientDigits();
        if (Rl.IsZero(x)) return Rl.One;

        long work = digits + GuardDigits;
        using var scope = Rl.WithPrecision(work, work);

        (long n, Rl f) = SplitIntegerFractional(x);
        Rl expF = ExpSeries(f, work);
        Rl expN = IntPow(Rl.E, n);
        return TruncateFrac(expF * expN, digits);
    }

    /// <summary>Computes the natural logarithm. Throws <see cref="ArgumentException"/> for
    /// <c>x &lt;= 0</c>.</summary>
    public static Rl Ln(Rl x)
    {
        ArgumentNullException.ThrowIfNull(x);
        if (Rl.IsZero(x) || Rl.IsNegative(x))
            throw new ArgumentException("Ln(x) requires x > 0.", nameof(x));

        long digits = AmbientDigits();
        long work = digits + GuardDigits;
        using var scope = Rl.WithPrecision(work, work);

        (long exp10, Rl m) = Decompose(x);
        Rl ln10 = FromLong(3) * AtanhLn(new Rl("2"), work) + AtanhLn(new Rl("1.25"), work);
        return TruncateFrac(AtanhLn(m, work) + FromLong(exp10) * ln10, digits);
    }

    // ------------------------------------------------------------------
    // Real — trigonometry
    // ------------------------------------------------------------------

    /// <summary>Computes <c>sin(x)</c> via argument reduction modulo π/2 and a Taylor series.</summary>
    public static Rl Sin(Rl x)
    {
        ArgumentNullException.ThrowIfNull(x);
        return SinCos(x).sin;
    }

    /// <summary>Computes <c>cos(x)</c> via argument reduction modulo π/2 and a Taylor series.</summary>
    public static Rl Cos(Rl x)
    {
        ArgumentNullException.ThrowIfNull(x);
        return SinCos(x).cos;
    }

    /// <summary>Computes <c>tan(x) = sin(x) / cos(x)</c>.</summary>
    /// <remarks>
    /// The quotient is taken from the UNTRUNCATED pair.  Rounding the pair to the ambient precision
    /// first flushes a cosine that is genuinely tiny — <c>cos(π/2 − 10^-100)</c> IS <c>10^-100</c> —
    /// to zero, which is how <c>tan(pi(100)/2)</c> was refused as a division by zero although the
    /// value is ≈ 2.43e100.
    /// </remarks>
    public static Rl Tan(Rl x)
    {
        ArgumentNullException.ThrowIfNull(x);
        long digits = AmbientDigits();
        long work = digits + GuardDigits;
        (Rl sin, Rl cos) = SinCosAtPrecision(x, digits);
        using var scope = Rl.WithPrecision(work, work);
        return TruncateFrac(DivideTruncate(sin, cos, work), digits);
    }

    /// <summary>Computes <c>atan(x)</c> via half-angle reduction and an alternating series.</summary>
    public static Rl Atan(Rl x)
    {
        ArgumentNullException.ThrowIfNull(x);
        long digits = AmbientDigits();
        long work = digits + GuardDigits;
        using var scope = Rl.WithPrecision(work, work);

        bool negate = Rl.IsNegative(x);
        Rl ax = Rl.Abs(x);
        bool big = ax > Rl.One;
        Rl arg = big ? DivideTruncate(Rl.One, ax, work) : ax;

        // Half-angle reduction: atan(t) = 2·atan(t / (1 + sqrt(1 + t²))); apply three times.
        Rl y = arg;
        for (int i = 0; i < 3; i++)
            y = DivideTruncate(y, Rl.One + Rl.Sqrt(Rl.One + y * y), work);

        Rl atanArg = AtanSeries(y, work) * new Rl("8");
        Rl result = big ? Rl.Pi / new Rl("2") - atanArg : atanArg;
        if (negate) result = -result;
        return TruncateFrac(result, digits);
    }

    /// <summary>Two-argument arctangent, returning the angle of <c>(x, y)</c> in <c>(-π, π]</c>.</summary>
    public static Rl Atan2(Rl y, Rl x)
    {
        ArgumentNullException.ThrowIfNull(y);
        ArgumentNullException.ThrowIfNull(x);

        Rl pi = Rl.Pi;
        Rl halfPi = pi / new Rl("2");

        bool xZero = Rl.IsZero(x);
        bool yZero = Rl.IsZero(y);
        if (xZero)
        {
            if (yZero) return Rl.Zero;
            return Rl.IsPositive(y) ? halfPi : -halfPi;
        }

        Rl at = Atan(DivideTruncate(y, x, AmbientDigits() + GuardDigits));
        if (Rl.IsPositive(x)) return at;
        return Rl.IsNegative(y) ? at - pi : at + pi;
    }

    // ------------------------------------------------------------------
    // Real — power
    // ------------------------------------------------------------------

    /// <summary>Computes <c>b^e = e^(e·ln b)</c>. Throws <see cref="ArgumentException"/> unless
    /// <c>b &gt; 0</c>.</summary>
    public static Rl Pow(Rl b, Rl e)
    {
        ArgumentNullException.ThrowIfNull(b);
        ArgumentNullException.ThrowIfNull(e);
        if (Rl.IsZero(b) || Rl.IsNegative(b))
            throw new ArgumentException("Pow(b, e) requires b > 0.", nameof(b));
        return Exp(e * Ln(b));
    }

    // ------------------------------------------------------------------
    // Real — inverse trigonometric/hyperbolic (used by Lovelace.Symbolics)
    // ------------------------------------------------------------------

    /// <summary>Computes <c>asin(x)</c> for <c>|x| ≤ 1</c>.</summary>
    public static Rl AsinReal(Rl x)
    {
        ArgumentNullException.ThrowIfNull(x);
        if (x > Rl.One || x < Rl.NegativeOne)
            throw new ArgumentException("AsinReal(x) requires |x| <= 1.", nameof(x));
        if (x == Rl.One) return Rl.Pi / new Rl("2");
        if (x == Rl.NegativeOne) return -(Rl.Pi / new Rl("2"));
        return Atan(DivideTruncate(x, Rl.Sqrt(Rl.One - x * x), AmbientDigits() + GuardDigits));
    }

    /// <summary>Computes <c>acos(x)</c> for <c>|x| ≤ 1</c>.</summary>
    public static Rl AcosReal(Rl x)
    {
        ArgumentNullException.ThrowIfNull(x);
        return Rl.Pi / new Rl("2") - AsinReal(x);
    }

    /// <summary>Computes <c>asinh(x)</c>.</summary>
    public static Rl AsinhReal(Rl x)
    {
        ArgumentNullException.ThrowIfNull(x);
        return Ln(x + Rl.Sqrt(x * x + Rl.One));
    }

    /// <summary>Computes <c>acosh(x)</c> for <c>x ≥ 1</c>.</summary>
    public static Rl AcoshReal(Rl x)
    {
        ArgumentNullException.ThrowIfNull(x);
        if (x < Rl.One)
            throw new ArgumentException("AcoshReal(x) requires x >= 1.", nameof(x));
        return Ln(x + Rl.Sqrt(x * x - Rl.One));
    }

    /// <summary>Computes <c>atanh(x)</c> for <c>|x| &lt; 1</c>.</summary>
    public static Rl AtanhReal(Rl x)
    {
        ArgumentNullException.ThrowIfNull(x);
        if (x >= Rl.One || x <= Rl.NegativeOne)
            throw new ArgumentException("AtanhReal(x) requires |x| < 1.", nameof(x));
        return (Ln(Rl.One + x) - Ln(Rl.One - x)) / new Rl("2");
    }

    // ------------------------------------------------------------------
    // Complex — exponential / trigonometry
    // ------------------------------------------------------------------

    /// <summary>Computes <c>e^(a+bi) = e^a·(cos b + i·sin b)</c>.</summary>
    public static Complex Exp(Complex z)
    {
        ArgumentNullException.ThrowIfNull(z);
        long digits = AmbientDigits();
        Rl e = Exp(z.Re);
        return TruncateComplex(new Complex(e * Cos(z.Im), e * Sin(z.Im)), digits);
    }

    /// <summary>Computes <c>sin(a+bi) = sin a·cosh b + i·cos a·sinh b</c>.</summary>
    public static Complex Sin(Complex z)
    {
        ArgumentNullException.ThrowIfNull(z);
        long digits = AmbientDigits();
        return TruncateComplex(
            new Complex(Sin(z.Re) * RealCosh(z.Im), Cos(z.Re) * RealSinh(z.Im)), digits);
    }

    /// <summary>Computes <c>cos(a+bi) = cos a·cosh b − i·sin a·sinh b</c>.</summary>
    public static Complex Cos(Complex z)
    {
        ArgumentNullException.ThrowIfNull(z);
        long digits = AmbientDigits();
        return TruncateComplex(
            new Complex(Cos(z.Re) * RealCosh(z.Im), -Sin(z.Re) * RealSinh(z.Im)), digits);
    }

    /// <summary>Computes <c>tan(z) = sin(z) / cos(z)</c>.</summary>
    public static Complex Tan(Complex z)
    {
        ArgumentNullException.ThrowIfNull(z);
        return Sin(z) / Cos(z);
    }

    /// <summary>Computes <c>sinh(a+bi) = sinh a·cos b + i·cosh a·sin b</c>.</summary>
    public static Complex Sinh(Complex z)
    {
        ArgumentNullException.ThrowIfNull(z);
        long digits = AmbientDigits();
        return TruncateComplex(
            new Complex(RealSinh(z.Re) * Cos(z.Im), RealCosh(z.Re) * Sin(z.Im)), digits);
    }

    /// <summary>Computes <c>cosh(a+bi) = cosh a·cos b + i·sinh a·sin b</c>.</summary>
    public static Complex Cosh(Complex z)
    {
        ArgumentNullException.ThrowIfNull(z);
        long digits = AmbientDigits();
        return TruncateComplex(
            new Complex(RealCosh(z.Re) * Cos(z.Im), RealSinh(z.Re) * Sin(z.Im)), digits);
    }

    // ------------------------------------------------------------------
    // Complex — logarithm / root / power
    // ------------------------------------------------------------------

    /// <summary>Principal square root: <c>s + t·i</c> with <c>s = √((|z|+a)/2)</c> and
    /// <c>t = √((|z|−a)/2)</c>, choosing <c>t ≥ 0</c> when <c>Im ≥ 0</c>.</summary>
    public static Complex Sqrt(Complex z)
    {
        ArgumentNullException.ThrowIfNull(z);
        long digits = AmbientDigits();
        Rl a = z.Re;
        Rl b = z.Im;
        Rl mag = z.Magnitude;
        Rl s = Rl.Sqrt((mag + a) / new Rl("2"));
        Rl diff = mag - a;
        if (Rl.IsNegative(diff)) diff = Rl.Zero;
        Rl t = Rl.Sqrt(diff / new Rl("2"));
        return TruncateComplex(Rl.IsNegative(b) ? new Complex(s, -t) : new Complex(s, t), digits);
    }

    /// <summary>Principal logarithm <c>ln|z| + i·atan2(Im, Re)</c>. Throws
    /// <see cref="ArgumentException"/> for <c>z == 0</c>.</summary>
    public static Complex Log(Complex z)
    {
        ArgumentNullException.ThrowIfNull(z);
        if (Rl.IsZero(z.Re) && Rl.IsZero(z.Im))
            throw new ArgumentException("log(0) is undefined.", nameof(z));
        long digits = AmbientDigits();
        return TruncateComplex(new Complex(Ln(z.Magnitude), Atan2(z.Im, z.Re)), digits);
    }

    /// <summary>Principal complex power <c>b^e = e^(e·log b)</c>.</summary>
    public static Complex Pow(Complex b, Complex e)
    {
        ArgumentNullException.ThrowIfNull(b);
        ArgumentNullException.ThrowIfNull(e);
        return Exp(e * Log(b));
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    private static long AmbientDigits()
    {
        long d = Rl.MaxComputationDecimalPlaces;
        return d < 1 ? 1 : d;
    }

    private static Rl FromLong(long n) => Rl.Parse(n.ToString(CultureInfo.InvariantCulture));

    private static Rl Pow10(long e)
    {
        if (e == 0) return Rl.One;
        if (e > 0) return Rl.Parse("1" + new string('0', (int)e));
        return Rl.Parse("0." + new string('0', (int)(-e - 1)) + "1");
    }

    private static Rl TinyThreshold(long work) => new Rl("0." + new string('0', (int)work) + "1");

    /// <summary>Truncating (non-periodic) division to <paramref name="fracDigits"/> fractional
    /// digits, mirroring <c>Real.DivideNonPeriodic</c> via public <see cref="Nat"/> arithmetic.
    /// Avoids the public <c>/</c> operator's period detection, which corrupts series sums.
    /// <para>
    /// A division that dropped a remainder is a truncation and carries the provenance of one; one
    /// that came out even is exact.  Returning the digit-built value without that flag is how a
    /// truncated transcendental crossed the wire as an exact rational (audit F1): the series'
    /// intermediate quotients all claimed exactness, and the final rounding had nothing to inherit.
    /// </para>
    /// </summary>
    private static Rl DivideTruncate(Rl left, Rl right, long fracDigits)
    {
        if (Rl.IsZero(right)) throw new DivideByZeroException("Cannot divide a Real by zero.");
        if (Rl.IsZero(left)) return Rl.Zero;

        bool resultNeg = Rl.IsNegative(left) != Rl.IsNegative(right);
        Nat numerator = left.ToNatural();
        Nat denominator = right.ToNatural();
        long exponentAdjustment = left.Exponent - right.Exponent;

        long n = fracDigits + (exponentAdjustment > 0 ? exponentAdjustment : 0);
        Nat scaled = Nat.DivRem(numerator.ShiftLeftDecimal(n), denominator, out Nat remainder);
        var result = new Rl(new Int(scaled, resultNeg));
        result.Exponent = -n + exponentAdjustment;
        return Nat.IsZero(remainder) ? result : result * InexactOne;
    }

    /// <summary>Splits <paramref name="x"/> into <c>n = trunc(x)</c> and <c>f = x − n</c> with
    /// <c>|f| &lt; 1</c>, by truncating the decimal string at the decimal point.</summary>
    private static (long n, Rl f) SplitIntegerFractional(Rl x)
    {
        string s = x.ToString();
        int dot = s.IndexOf('.');
        string intPart = dot >= 0 ? s.Substring(0, dot) : s;
        if (intPart.Length == 0) intPart = "0";
        long n = long.Parse(intPart, CultureInfo.InvariantCulture);
        Rl f = x - Rl.Parse(intPart);
        return (n, f);
    }

    /// <summary>Computes <c>b^n</c> for integer <paramref name="n"/> by binary exponentiation.</summary>
    private static Rl IntPow(Rl b, long n)
    {
        if (n < 0) return DivideTruncate(Rl.One, IntPow(b, -n), AmbientDigits() + GuardDigits);
        Rl result = Rl.One;
        Rl factor = b;
        while (n > 0)
        {
            if ((n & 1) == 1) result = result * factor;
            factor = factor * factor;
            n >>= 1;
        }
        return result;
    }

    /// <summary>Taylor series for <c>e^g</c> with <c>|g| &lt; 1</c>.</summary>
    private static Rl ExpSeries(Rl g, long work)
    {
        Rl term = Rl.One;
        Rl sum = Rl.One;
        Rl thr = TinyThreshold(work);
        long maxK = 2 * work + 60;
        for (long k = 1; k <= maxK; k++)
        {
            term = DivideTruncate(term * g, FromLong(k), work);
            sum = sum + term;
            if (Rl.Abs(term) < thr) break;
        }
        return sum;
    }

    /// <summary>Decomposes positive <paramref name="x"/> into <c>m · 10^L</c> with
    /// <c>m ∈ [1, 10)</c>, using the decimal structure of its string form.</summary>
    private static (long L, Rl m) Decompose(Rl x)
    {
        string s = x.ToString();
        long L;
        if (s.Contains('.'))
        {
            int dot = s.IndexOf('.');
            string intPart = s.Substring(0, dot);
            if (intPart == "0")
            {
                string fracPart = s.Substring(dot + 1);
                int zeros = 0;
                while (zeros < fracPart.Length && fracPart[zeros] == '0') zeros++;
                L = -(zeros + 1);
            }
            else
            {
                L = intPart.Length - 1;
            }
        }
        else
        {
            L = s.Length - 1;
        }
        return (L, x * Pow10(-L));
    }

    /// <summary>Computes <c>ln(m)</c> for <c>m ∈ [1, 10)</c> via a reduced atanh series: 16 square
    /// roots shrink <c>m</c> toward 1, then <c>ln(m) = 65536·2·(t + t³/3 + t⁵/5 + …)</c> with
    /// <c>t = (m′−1)/(m′+1)</c>.</summary>
    private static Rl AtanhLn(Rl m, long work)
    {
        Rl x = m;
        for (int i = 0; i < 16; i++) x = Rl.Sqrt(x);

        Rl t = DivideTruncate(x - Rl.One, x + Rl.One, work);
        Rl t2 = t * t;
        Rl pow = t;
        Rl sum = t;
        Rl thr = TinyThreshold(work);
        long maxK = ((work + 20) * 2) / 19 + 5;

        for (long k = 1; k <= maxK; k++)
        {
            pow = pow * t2;
            Rl contrib = DivideTruncate(pow, FromLong(2 * k + 1), work);
            sum = sum + contrib;
            if (Rl.Abs(contrib) < thr) break;
        }

        return sum * new Rl("2") * FromLong(65536);
    }

    /// <summary>Computes sin/cos of <paramref name="x"/> together, at the ambient precision.</summary>
    private static (Rl sin, Rl cos) SinCos(Rl x)
    {
        long digits = AmbientDigits();
        (Rl sin, Rl cos) = SinCosAtPrecision(x, digits);
        return (TruncateFrac(sin, digits), TruncateFrac(cos, digits));
    }

    /// <summary>
    /// The UNTRUNCATED sin/cos pair of <paramref name="x"/>, reduced and summed at
    /// <paramref name="digits"/> plus the internal guard.
    /// <para>
    /// The places π must carry for the reduction to measure the angle instead of measuring π: the
    /// argument's own fractional places, plus the precision the result is asked for, plus the guard
    /// — or the ambient precision when the ambient π already reaches as far down as the argument
    /// does.  Reducing an argument that carries 100 places against a 30-place π measures
    /// <c>argument − k·π_30</c> (an artifact of order 1e-30) instead of the argument's own distance
    /// to a multiple of π (order 1e-100), which is how <c>sin(pi(100)·2)</c> came back as
    /// <c>+1e-30</c> instead of <c>−1.64e-100</c>.  A coarser argument is already resolved by the
    /// ambient π, and an exact multiple of the ambient π must keep reducing exactly to zero.
    /// </para>
    /// </summary>
    private static long PiDigitsFor(Rl x, long digits)
    {
        long argumentPlaces = x.IsPeriodic ? 0L : -x.Exponent;
        if (argumentPlaces < 0L)
            argumentPlaces = 0L;

        long ambient = AmbientDigits();
        return argumentPlaces <= ambient ? ambient : argumentPlaces + digits + GuardDigits;
    }

    /// <summary>
    /// Reduces <paramref name="x"/> modulo π/2 against a π that resolves it and sums the series,
    /// returning the pair before either half is rounded to the ambient precision (see
    /// <see cref="Tan"/>).  Every series division is a truncating one, so the pair is an
    /// approximation and its provenance says so.
    /// </summary>
    private static (Rl sin, Rl cos) SinCosAtPrecision(Rl x, long digits)
    {
        // sin(0) = 0 and cos(0) = 1 exactly: the reduction below multiplies by an inexact π, which
        // would otherwise leave the exact zeros carrying an approximation's provenance.
        if (Rl.IsZero(x))
            return (Rl.Zero, Rl.One);

        long work = digits + GuardDigits;
        long piDigits = PiDigitsFor(x, digits);

        Rl pi;
        using (Rl.WithPrecision(piDigits, work))
            pi = Rl.Pi;                // the ambient π unless the argument reaches further down

        using var scope = Rl.WithPrecision(work, work);

        Rl twoPi = pi * new Rl("2");
        Rl halfPi = pi / new Rl("2");
        Rl quarterPi = pi / new Rl("4");

        Rl xr = ReduceToTwoPi(x, pi, twoPi);
        long k = TruncToLong(xr / halfPi);
        Rl r = xr - FromLong(k) * halfPi;      // r in [0, π/2)

        bool swap = false;
        if (r > quarterPi) { r = halfPi - r; swap = true; }   // fold to [0, π/4]

        Rl s = SinTaylor(r, work);
        Rl c = CosTaylor(r, work);
        if (swap) (s, c) = (c, s);

        Rl sin, cos;
        switch (k % 4)
        {
            case 1: sin = c; cos = -s; break;
            case 2: sin = -s; cos = -c; break;
            case 3: sin = -c; cos = s; break;
            default: sin = s; cos = c; break;
        }

        return (sin, cos);
    }

    /// <summary>Reduces <paramref name="value"/> into <c>[0, 2π)</c>.</summary>
    private static Rl ReduceToTwoPi(Rl value, Rl pi, Rl twoPi)
    {
        if (value >= Rl.Zero && value < twoPi) return value;
        long k = TruncToLong(value / twoPi);
        Rl reduced = value - twoPi * FromLong(k);
        if (reduced < Rl.Zero) reduced = reduced + twoPi;
        return reduced;
    }

    /// <summary>Truncates <paramref name="q"/> toward zero to a <see cref="long"/> using its
    /// decimal string.</summary>
    private static long TruncToLong(Rl q)
    {
        string s = q.ToString();
        int dot = s.IndexOf('.');
        string intPart = dot >= 0 ? s.Substring(0, dot) : s;
        if (intPart.Length == 0) intPart = "0";
        return long.Parse(intPart, CultureInfo.InvariantCulture);
    }

    /// <summary>sin(r) = r − r³/3! + r⁵/5! − … for <c>|r| ≤ π/4</c>.</summary>
    private static Rl SinTaylor(Rl r, long work)
    {
        Rl x2 = r * r;
        Rl term = r;
        Rl sum = r;
        Rl thr = TinyThreshold(work);
        long maxK = 5 * work + 60;
        for (long k = 1; k <= maxK; k++)
        {
            long denom = (2 * k) * (2 * k + 1);
            term = DivideTruncate(-term * x2, FromLong(denom), work);
            sum = sum + term;
            if (Rl.Abs(term) < thr) break;
        }
        return sum;
    }

    /// <summary>cos(r) = 1 − r²/2! + r⁴/4! − … for <c>|r| ≤ π/4</c>.</summary>
    private static Rl CosTaylor(Rl r, long work)
    {
        Rl x2 = r * r;
        Rl term = Rl.One;
        Rl sum = Rl.One;
        Rl thr = TinyThreshold(work);
        long maxK = 5 * work + 60;
        for (long k = 1; k <= maxK; k++)
        {
            long denom = (2 * k - 1) * (2 * k);
            term = DivideTruncate(-term * x2, FromLong(denom), work);
            sum = sum + term;
            if (Rl.Abs(term) < thr) break;
        }
        return sum;
    }

    /// <summary>atan(y) = y − y³/3 + y⁵/5 − … for small <c>y</c>.</summary>
    private static Rl AtanSeries(Rl y, long work)
    {
        Rl y2 = y * y;
        Rl pow = y;
        Rl sum = y;
        Rl thr = TinyThreshold(work);
        long maxK = 3 * work + 60;
        for (long k = 1; k <= maxK; k++)
        {
            pow = pow * y2;
            Rl contrib = DivideTruncate(pow, FromLong(2 * k + 1), work);
            sum = (k & 1) == 1 ? sum - contrib : sum + contrib;
            if (Rl.Abs(contrib) < thr) break;
        }
        return sum;
    }

    /// <summary>cosh(x) = (e^x + e^−x)/2.</summary>
    private static Rl RealCosh(Rl x) => (Exp(x) + Exp(-x)) / new Rl("2");

    /// <summary>sinh(x) = (e^x − e^−x)/2.</summary>
    private static Rl RealSinh(Rl x) => (Exp(x) - Exp(-x)) / new Rl("2");

    /// <summary>
    /// Exactly <c>1</c>, carrying the provenance of an approximation.
    /// <para>
    /// <see cref="Rl.IsExact"/> is a read-only flag whose setter (<c>Real.MarkInexact</c>) is
    /// <c>internal</c> to Lovelace.Real, so an assembly outside it cannot clear the flag directly —
    /// but multiplication and addition inherit the provenance of their operands, and π is a
    /// truncation.  <c>π/π</c> is therefore the value 1 flagged inexact, and multiplying by it
    /// leaves every digit of a result exactly where it was while giving it the provenance a
    /// truncation must carry.  Without that, a value that lost digits here crosses the wire as an
    /// exact rational (the audit's P0: "evalf certifies a truncated transcendental as exact").
    /// </para>
    /// </summary>
    private static Rl? s_inexactOne;

    private static Rl InexactOne => s_inexactOne ??= Rl.Pi / Rl.Pi;

    /// <summary>
    /// Truncates a non-periodic real to at most <paramref name="maxFrac"/> fractional digits
    /// (toward zero), and carries the truncation in the result's provenance: a value that lost
    /// digits is not exact, whatever produced it.
    /// <para>
    /// A value SMALLER than the window — every one of its digits lies below the last place kept —
    /// keeps <paramref name="maxFrac"/> SIGNIFICANT digits instead of being flushed to zero.  Zero is
    /// not a truncation of such a value's digits, it is the loss of the value (audit F6: "evalf
    /// returns exactly 0 for a nonzero value whose leading zeros exceed the digit budget").
    /// </para>
    /// <para>
    /// The result is built from the kept digits and the exponent directly.  Rebuilding it by parsing
    /// a rendered string (the previous <c>Rl.Parse</c> of a decimal body) is where
    /// <see cref="Rl.IsExact"/> was thrown away.
    /// </para>
    /// </summary>
    private static Rl TruncateFrac(Rl x, long maxFrac)
    {
        if (maxFrac < 0) maxFrac = 0;
        if (Rl.IsZero(x) || x.IsPeriodic) return x;

        long storedFrac = -x.Exponent;
        if (storedFrac <= maxFrac) return x;

        string digits = x.ToNatural().ToString();
        long toDrop = storedFrac - maxFrac;
        long keepLen = (long)digits.Length - toDrop;

        if (keepLen <= 0)
        {
            // The window sits above the value: keep maxFrac significant digits rather than nothing.
            keepLen = maxFrac < (long)digits.Length ? maxFrac : (long)digits.Length;
            if (keepLen <= 0) return Rl.Zero * InexactOne;
            toDrop = (long)digits.Length - keepLen;
        }

        if (!Nat.TryParse(digits.Substring(0, (int)keepLen), null, out Nat? kept))
            return Rl.Zero * InexactOne;

        var truncated = new Rl(new Int(kept, Rl.IsNegative(x)));
        truncated.Exponent = x.Exponent + toDrop;
        return truncated * InexactOne;
    }

    /// <summary>Truncates both components of a complex value to <paramref name="digits"/>
    /// fractional digits.</summary>
    private static Complex TruncateComplex(Complex z, long digits) =>
        new Complex(TruncateFrac(z.Re, digits), TruncateFrac(z.Im, digits));
}
