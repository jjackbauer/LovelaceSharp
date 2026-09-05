using System.Numerics;
using System.Text;

namespace Lovelace.Knowledge;

/// <summary>
/// An exact rational number (<c>Num / Den</c>) used to model Lovelace numeric
/// operands without floating point. It round-trips with the Lovelace literal
/// syntax: whole numbers (<c>42</c>), decimals (<c>3.14</c>), and periodic
/// fractions (<c>0.(3)</c>, <c>0.1(6)</c>).
/// </summary>
/// <remarks>
/// The canonical form is a reduced fraction with <c>Den &gt;= 1</c> and
/// <c>gcd(|Num|, Den) == 1</c>. The sign lives in <c>Num</c>. This type is the
/// exact arithmetic backbone of boundary detection and guard fitting; it is
/// never serialized directly (numeric values are persisted as Lovelace literals).
/// </remarks>
public readonly struct ExactNumber : IEquatable<ExactNumber>, IComparable<ExactNumber>
{
    public BigInteger Num { get; }
    public BigInteger Den { get; }

    public static ExactNumber Zero => new(BigInteger.Zero, BigInteger.One);
    public static ExactNumber One => new(BigInteger.One, BigInteger.One);

    public ExactNumber(BigInteger num, BigInteger den)
    {
        if (den.IsZero)
            throw new DivideByZeroException("ExactNumber denominator cannot be zero.");
        if (den.Sign < 0)
        {
            num = -num;
            den = -den;
        }
        var g = BigInteger.GreatestCommonDivisor(BigInteger.Abs(num), den);
        if (g > BigInteger.One)
        {
            num /= g;
            den /= g;
        }
        Num = num;
        Den = den;
    }

    public ExactNumber(BigInteger integer) : this(integer, BigInteger.One) { }

    public bool IsZero => Num.IsZero;
    public bool IsNegative => Num.Sign < 0;
    public bool IsInteger => Den == BigInteger.One;

    public ExactNumber Negate() => new(-Num, Den);
    public ExactNumber Abs() => IsNegative ? Negate() : this;
    public static ExactNumber Add(ExactNumber a, ExactNumber b) =>
        new(a.Num * b.Den + b.Num * a.Den, a.Den * b.Den);
    public static ExactNumber Subtract(ExactNumber a, ExactNumber b) =>
        new(a.Num * b.Den - b.Num * a.Den, a.Den * b.Den);
    public ExactNumber Add(ExactNumber other) => Add(this, other);
    public ExactNumber Subtract(ExactNumber other) => Subtract(this, other);

    public int CompareTo(ExactNumber other) =>
        (Num * other.Den).CompareTo(other.Num * Den);

    public bool Equals(ExactNumber other) =>
        Num == other.Num && Den == other.Den;

    public override bool Equals(object? obj) => obj is ExactNumber n && Equals(n);
    public override int GetHashCode() => HashCode.Combine(Num, Den);
    public static bool operator ==(ExactNumber a, ExactNumber b) => a.Equals(b);
    public static bool operator !=(ExactNumber a, ExactNumber b) => !a.Equals(b);
    public static bool operator <(ExactNumber a, ExactNumber b) => a.CompareTo(b) < 0;
    public static bool operator >(ExactNumber a, ExactNumber b) => a.CompareTo(b) > 0;
    public static bool operator <=(ExactNumber a, ExactNumber b) => a.CompareTo(b) <= 0;
    public static bool operator >=(ExactNumber a, ExactNumber b) => a.CompareTo(b) >= 0;

    // ---------------------------------------------------------------------
    // Parsing (Lovelace literal → rational)
    // ---------------------------------------------------------------------

    /// <summary>
    /// Parses a Lovelace numeric literal: <c>42</c>, <c>-7</c>, <c>3.14</c>,
    /// <c>.5</c>, <c>0.(3)</c>, <c>0.1(6)</c>, <c>-1.2(34)</c>. Throws
    /// <see cref="FormatException"/> on malformed input.
    /// </summary>
    public static ExactNumber Parse(string literal)
    {
        if (string.IsNullOrWhiteSpace(literal))
            throw new FormatException($"Cannot parse empty number literal.");

        string s = literal.Trim();
        bool neg = false;
        if (s[0] == '-') { neg = true; s = s[1..]; }
        else if (s[0] == '+') { s = s[1..]; }

        string intPart = "0";
        string? pre = null;   // digits between '.' and '('
        string? period = null; // digits between '(' and ')'

        int dot = s.IndexOf('.');
        int open = s.IndexOf('(');

        if (open >= 0)
        {
            // periodic form:  I.P(Q)
            if (open < dot)
                throw new FormatException($"Malformed periodic literal '{literal}'.");
            if (!s.EndsWith(')'))
                throw new FormatException($"Unterminated period in '{literal}'.");
            if (dot >= 0)
            {
                intPart = s[..dot];
                pre = s[(dot + 1)..open];
            }
            else
            {
                intPart = s[..open];
                pre = "";
            }
            period = s[(open + 1)..^1];
            if (period.Length == 0)
                throw new FormatException($"Empty period in '{literal}'.");
        }
        else if (dot >= 0)
        {
            intPart = s[..dot];
            pre = s[(dot + 1)..];
        }
        else
        {
            intPart = s;
        }

        var intVal = BigInteger.Parse(string.IsNullOrEmpty(intPart) ? "0" : intPart);
        var preVal = BigInteger.Parse(string.IsNullOrEmpty(pre) ? "0" : pre);
        var preScale = Pow10(pre is null ? 0 : pre.Length);

        BigInteger num;
        BigInteger den;
        if (period is null)
        {
            num = intVal * preScale + preVal;
            den = preScale;
        }
        else
        {
            var periodVal = BigInteger.Parse(period);
            var periodScale = Pow10(period.Length);
            // value = int + pre/10^p + period / (10^p * (10^q - 1))
            var repUnit = periodScale - BigInteger.One; // 10^q - 1
            num = intVal * preScale * repUnit + preVal * repUnit + periodVal;
            den = preScale * repUnit;
        }

        if (neg) num = -num;
        return new ExactNumber(num, den);
    }

    private static BigInteger Pow10(int n)
    {
        BigInteger v = BigInteger.One;
        for (int i = 0; i < n; i++) v *= 10;
        return v;
    }

    // ---------------------------------------------------------------------
    // Formatting (rational → Lovelace literal)
    // ---------------------------------------------------------------------

    /// <summary>
    /// Emits a Lovelace literal that parses back to this exact value: an integer
    /// when integral, a terminating decimal when the denominator is 2<sup>a</sup>5<sup>b</sup>,
    /// and the shortest periodic form otherwise.
    /// </summary>
    public string ToLovelaceLiteral()
    {
        if (IsInteger)
            return Num.ToString();

        bool neg = Num.Sign < 0;
        var abs = BigInteger.Abs(Num);
        var intPart = abs / Den;
        var rem = abs % Den;

        // Long division over the fractional digits; detect termination (rem == 0)
        // or the repeating cycle via the first repeated remainder.
        var seen = new Dictionary<BigInteger, int>();
        var digits = new List<int>();
        int i = 0;
        int periodStart = -1;
        while (!rem.IsZero && !seen.ContainsKey(rem))
        {
            seen[rem] = i;
            rem *= 10;
            digits.Add((int)(rem / Den));
            rem %= Den;
            i++;
        }
        if (rem.IsZero)
        {
            periodStart = -1; // terminating
        }
        else
        {
            periodStart = seen[rem];
        }

        var sb = new StringBuilder();
        if (neg) sb.Append('-');
        sb.Append(intPart).Append('.');
        for (int d = 0; d < digits.Count; d++)
        {
            if (d == periodStart) sb.Append('(');
            sb.Append((char)('0' + digits[d]));
        }
        if (periodStart >= 0) sb.Append(')');
        return sb.ToString();
    }

    public override string ToString() => ToLovelaceLiteral();
}
