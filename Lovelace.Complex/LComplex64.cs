using LReal = global::Lovelace.Real.LReal64;

namespace Lovelace.Complex;

/// <summary>
/// Fixed-width complex number whose real and imaginary parts are each an <see cref="LReal"/>
/// (up to <see cref="LReal.MaxSignificantDigits"/> significant digits, exact decimal). Mirrors
/// <see cref="Complex"/>'s exact arithmetic but throws
/// <see cref="global::Lovelace.Real.LRealPromoteException"/> (from the components) rather than
/// rounding when a result needs more than the fixed width. The arbitrary-precision fallback is
/// <see cref="Complex"/> — the class form, matching how <c>Real</c> pairs with
/// <c>LReal64</c>/<c>LReal128</c>.
/// </summary>
public readonly struct LComplex64 : IEquatable<LComplex64>
{
    public const int MaxSignificantDigits = LReal.MaxSignificantDigits;

    /// <summary>The real component (fixed-width).</summary>
    public LReal Re { get; }

    /// <summary>The imaginary component (fixed-width).</summary>
    public LReal Im { get; }

    public LComplex64(LReal re, LReal im)
    {
        Re = re;
        Im = im;
    }

    /// <summary>Constructs a purely real complex number (<c>im = 0</c>).</summary>
    public LComplex64(LReal re) : this(re, LReal.Zero) { }

    /// <summary>Additive identity <c>0 + 0i</c>.</summary>
    public static LComplex64 Zero => default;

    /// <summary>Multiplicative identity <c>1 + 0i</c>.</summary>
    public static LComplex64 One => new(LReal.One, LReal.Zero);

    /// <summary>Imaginary unit <c>0 + 1i</c>.</summary>
    public static LComplex64 I => new(LReal.Zero, LReal.One);

    // -----------------------------------------------------------------
    // Arithmetic
    // -----------------------------------------------------------------

    public static LComplex64 operator +(LComplex64 a, LComplex64 b) => new(a.Re + b.Re, a.Im + b.Im);
    public static LComplex64 operator -(LComplex64 a, LComplex64 b) => new(a.Re - b.Re, a.Im - b.Im);
    public static LComplex64 operator -(LComplex64 value) => new(-value.Re, -value.Im);
    public static LComplex64 operator *(LComplex64 a, LComplex64 b) =>
        new(a.Re * b.Re - a.Im * b.Im, a.Re * b.Im + a.Im * b.Re);
    public static LComplex64 operator *(LComplex64 a, LReal scalar) => new(a.Re * scalar, a.Im * scalar);
    public static LComplex64 operator *(LReal scalar, LComplex64 a) => a * scalar;
    public static LComplex64 operator /(LComplex64 a, LComplex64 b) => a * b.Reciprocal;
    public static LComplex64 operator /(LComplex64 a, LReal scalar) => new(a.Re / scalar, a.Im / scalar);

    // -----------------------------------------------------------------
    // Properties
    // -----------------------------------------------------------------

    /// <summary>Complex conjugate <c>re − im·i</c>.</summary>
    public LComplex64 Conjugate => new(Re, -Im);

    /// <summary><c>|z|² = re² + im²</c> (exact for rational components; no square root).</summary>
    public LReal MagnitudeSquared => Re * Re + Im * Im;

    /// <summary>Multiplicative inverse <c>1/z = conj(z) / |z|²</c>.</summary>
    public LComplex64 Reciprocal => Conjugate / MagnitudeSquared;

    // -----------------------------------------------------------------
    // Conversion
    // -----------------------------------------------------------------

    /// <summary>Converts an arbitrary-precision <see cref="Complex"/> if both components fit the fixed width.</summary>
    public static bool TryFromComplex(Complex c, out LComplex64 result)
    {
        result = default;
        if (!LReal.TryFromReal(c.Re, out var re)) return false;
        if (!LReal.TryFromReal(c.Im, out var im)) return false;
        result = new LComplex64(re, im);
        return true;
    }

    /// <summary>Converts this value back to the arbitrary-precision <see cref="Complex"/> type.</summary>
    public Complex ToComplex() => new(Re.ToReal(), Im.ToReal());

    // -----------------------------------------------------------------
    // Equality
    // -----------------------------------------------------------------

    public bool Equals(LComplex64 other) => Re == other.Re && Im == other.Im;

    public override bool Equals(object? obj) => obj is LComplex64 c && Equals(c);

    public override int GetHashCode() => HashCode.Combine(Re, Im);

    public static bool operator ==(LComplex64 left, LComplex64 right) => left.Equals(right);
    public static bool operator !=(LComplex64 left, LComplex64 right) => !left.Equals(right);

    // -----------------------------------------------------------------
    // Formatting / parsing
    // -----------------------------------------------------------------

    public override string ToString()
    {
        if (Im.IsZero) return Re.ToString();
        if (Re.IsZero) return Im.ToString() + "i";
        bool imNegative = Im.IsNegative;
        string imAbs = imNegative ? (-Im).ToString() : Im.ToString();
        return $"{Re}{(imNegative ? " - " : " + ")}{imAbs}i";
    }

    public static LComplex64 Parse(string s) => Parse(s, null);

    public static LComplex64 Parse(string s, IFormatProvider? provider)
    {
        ArgumentNullException.ThrowIfNull(s);
        string text = s.Trim();
        if (text.Length == 0)
            throw new FormatException("Input string was empty.");

        if (!text.EndsWith('i'))
            return new LComplex64(LReal.Parse(text, provider));

        string body = text[..^1].TrimEnd();
        if (body.Length == 0 || body == "+" || body == "-")
            return new LComplex64(LReal.Zero, LReal.Parse(body == "-" ? "-1" : "1", provider));

        int split = -1;
        for (int i = body.Length - 1; i > 0; i--)
        {
            if (body[i] is '+' or '-') { split = i; break; }
        }

        if (split < 0)
            return new LComplex64(LReal.Zero, LReal.Parse(body, provider));

        string rePart = body[..split].Trim();
        string imPart = body[split..].Trim();
        char sign = imPart[0];
        string imDigits = imPart[1..].Trim();
        if (imDigits.Length == 0) imDigits = "1";
        imPart = sign == '-' ? "-" + imDigits : imDigits;
        return new LComplex64(LReal.Parse(rePart, provider), LReal.Parse(imPart, provider));
    }
}
