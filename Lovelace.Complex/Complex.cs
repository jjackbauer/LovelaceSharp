using Lovelace.Real;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Complex;

/// <summary>
/// Arbitrary-precision complex number: a pair of arbitrary-precision <see cref="Rl"/> components.
/// There is no IEEE floating point anywhere in this type — the real and imaginary parts are
/// Lovelace <see cref="Rl"/> values computed at the active precision. This is the unbounded class
/// form; the fixed-width <see langword="readonly struct"/> forms are
/// <see cref="LComplex64"/> (over <c>LReal64</c>) and <see cref="LComplex128"/> (over <c>LReal128</c>).
/// </summary>
public sealed class Complex : IEquatable<Complex>
{
    /// <summary>The real component.</summary>
    public Rl Re { get; }

    /// <summary>The imaginary component.</summary>
    public Rl Im { get; }

    /// <summary>Constructs a complex number from arbitrary-precision real and imaginary parts.</summary>
    public Complex(Rl re, Rl im)
    {
        Re = re ?? throw new ArgumentNullException(nameof(re));
        Im = im ?? throw new ArgumentNullException(nameof(im));
    }

    /// <summary>Constructs a purely real complex number (<c>im = 0</c>).</summary>
    public Complex(Rl re) : this(re, new Rl("0")) { }

    /// <summary>Additive identity <c>0 + 0i</c>. Allocated fresh per access (patterns §5).</summary>
    public static Complex Zero => new(new Rl("0"), new Rl("0"));

    /// <summary>Multiplicative identity <c>1 + 0i</c>. Allocated fresh per access (patterns §5).</summary>
    public static Complex One => new(new Rl("1"), new Rl("0"));

    /// <summary>Imaginary unit <c>0 + 1i</c>. Allocated fresh per access (patterns §5).</summary>
    public static Complex I => new(new Rl("0"), new Rl("1"));

    /// <summary>π (pi), the real-valued complex form of the cached <see cref="Rl.Pi"/>.</summary>
    public static Complex Pi => new(Rl.Pi, new Rl("0"));

    /// <summary>Euler's number <c>e</c>, the real-valued complex form of the cached <see cref="Rl.E"/>.</summary>
    public static Complex E => new(Rl.E, new Rl("0"));

    // -----------------------------------------------------------------
    // Arithmetic
    // -----------------------------------------------------------------

    /// <summary>Component-wise addition.</summary>
    public static Complex operator +(Complex a, Complex b) =>
        Binary(a, b,
            static (x, y) => new Complex(x.Re + y.Re, x.Im + y.Im),
            static (x, y) => x + y,
            static (x, y) => x + y);

    /// <summary>Component-wise subtraction.</summary>
    public static Complex operator -(Complex a, Complex b) =>
        Binary(a, b,
            static (x, y) => new Complex(x.Re - y.Re, x.Im - y.Im),
            static (x, y) => x - y,
            static (x, y) => x - y);

    /// <summary>Negation.</summary>
    public static Complex operator -(Complex value) => new(-value.Re, -value.Im);

    /// <summary>Complex multiplication <c>(a+bi)(c+di) = (ac−bd) + (ad+bc)i</c>.</summary>
    public static Complex operator *(Complex a, Complex b) =>
        Binary(a, b,
            static (x, y) => new Complex(x.Re * y.Re - x.Im * y.Im, x.Re * y.Im + x.Im * y.Re),
            static (x, y) => x * y,
            static (x, y) => x * y);

    /// <summary>Scales both components by a real scalar.</summary>
    public static Complex operator *(Complex a, Rl scalar) => new(a.Re * scalar, a.Im * scalar);

    /// <summary>Scales both components by a real scalar.</summary>
    public static Complex operator *(Rl scalar, Complex a) => a * scalar;

    /// <summary>Complex division <c>a / b = a · (1/b)</c>.</summary>
    public static Complex operator /(Complex a, Complex b) =>
        Binary(a, b,
            static (x, y) => x * y.Reciprocal,
            static (x, y) => x / y,
            static (x, y) => x / y);

    /// <summary>Divides both components by a real scalar.</summary>
    public static Complex operator /(Complex a, Rl scalar) => new(a.Re / scalar, a.Im / scalar);

    // -----------------------------------------------------------------
    // Fixed-width fast path (mirrors NumericOps.ApplyRealBinary)
    // -----------------------------------------------------------------

    /// <summary>
    /// Applies a binary operation, trying the fixed-width <see cref="LComplex64"/> then
    /// <see cref="LComplex128"/> fast path when limited precision is requested
    /// (<see cref="Rl.MaxComputationDecimalPlaces"/> ≤ 37, so the fixed-width structs can hold the
    /// operands), and falling back to the arbitrary-precision <paramref name="slow"/> path when a
    /// component does not fit or the operation throws <see cref="LRealPromoteException"/>. The
    /// fallback guarantees exactness — the fast path never rounds.
    /// </summary>
    private static Complex Binary(
        Complex a,
        Complex b,
        Func<Complex, Complex, Complex> slow,
        Func<LComplex64, LComplex64, LComplex64> fast64,
        Func<LComplex128, LComplex128, LComplex128> fast128)
    {
        if (Rl.MaxComputationDecimalPlaces <= 37)
        {
            if (LComplex64.TryFromComplex(a, out var a64) && LComplex64.TryFromComplex(b, out var b64))
            {
                try { return fast64(a64, b64).ToComplex(); }
                catch (LRealPromoteException) { }
            }

            if (LComplex128.TryFromComplex(a, out var a128) && LComplex128.TryFromComplex(b, out var b128))
            {
                try { return fast128(a128, b128).ToComplex(); }
                catch (LRealPromoteException) { }
            }
        }

        return slow(a, b);
    }

    // -----------------------------------------------------------------
    // Properties
    // -----------------------------------------------------------------

    /// <summary>Complex conjugate <c>re − im·i</c>.</summary>
    public Complex Conjugate => new(Re, -Im);

    /// <summary><c>|z|² = re² + im²</c> (exact for rational components; no square root).</summary>
    public Rl MagnitudeSquared => Re * Re + Im * Im;

    /// <summary>
    /// Magnitude <c>|z| = √(re² + im²)</c> at the active precision
    /// (<see cref="Rl.Sqrt(Rl)"/> resolves <see cref="Rl.MaxComputationDecimalPlaces"/>).
    /// </summary>
    public Rl Magnitude => Rl.Sqrt(MagnitudeSquared);

    /// <summary>Multiplicative inverse <c>1/z = conj(z) / |z|²</c>.</summary>
    public Complex Reciprocal => Conjugate / MagnitudeSquared;

    /// <summary>Complex exponential <c>eᶻ = e^(re)·(cos(im) + i·sin(im))</c> at the default precision.</summary>
    public Complex Exp() => Exp(Rl.MaxComputationDecimalPlaces);

    /// <summary>Complex exponential <c>eᶻ</c> to <paramref name="digits"/> decimal places.</summary>
    public Complex Exp(long digits)
    {
        Rl eRe = Rl.Exp(Re, digits);
        return new Complex(eRe * Rl.Cos(Im, digits), eRe * Rl.Sin(Im, digits));
    }

    // -----------------------------------------------------------------
    // Equality
    // -----------------------------------------------------------------

    /// <inheritdoc/>
    public bool Equals(Complex? other) =>
        other is not null && Re == other.Re && Im == other.Im;

    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is Complex c && Equals(c);

    /// <inheritdoc/>
    public override int GetHashCode() => HashCode.Combine(Re, Im);

    /// <summary>Component-wise equality.</summary>
    public static bool operator ==(Complex? left, Complex? right) =>
        left is null ? right is null : left.Equals(right);

    /// <summary>Component-wise inequality.</summary>
    public static bool operator !=(Complex? left, Complex? right) => !(left == right);

    // -----------------------------------------------------------------
    // Formatting / parsing
    // -----------------------------------------------------------------

    /// <summary>
    /// Renders as <c>"re"</c>, <c>"im i"</c>, or <c>"re ± im i"</c>
    /// (e.g. <c>"2"</c>, <c>"4i"</c>, <c>"1.5 + 0.5i"</c>, <c>"1.5 - 0.5i"</c>).
    /// </summary>
    public override string ToString()
    {
        bool imZero = Rl.IsZero(Im);
        bool reZero = Rl.IsZero(Re);
        if (imZero) return Re.ToString();
        if (reZero) return Im.ToString() + "i";
        bool imNegative = Rl.IsNegative(Im);
        string imAbs = imNegative ? (-Im).ToString() : Im.ToString();
        return $"{Re}{(imNegative ? " - " : " + ")}{imAbs}i";
    }

    /// <summary>
    /// Parses the forms produced by <see cref="ToString()"/>: <c>"re"</c>, <c>"im i"</c>, or
    /// <c>"re ± im i"</c>, plus <c>"i"</c> and <c>"-i"</c>. The real and imaginary parts are
    /// parsed with <see cref="Rl.Parse(string, IFormatProvider?)"/>.
    /// </summary>
    public static Complex Parse(string s) => Parse(s, null);

    /// <summary>Parses a complex number with a format provider.</summary>
    public static Complex Parse(string s, IFormatProvider? provider)
    {
        ArgumentNullException.ThrowIfNull(s);
        string text = s.Trim();
        if (text.Length == 0)
            throw new FormatException("Input string was empty.");

        if (!text.EndsWith('i'))
            return new Complex(Rl.Parse(text, provider), new Rl("0"));

        string body = text[..^1].TrimEnd();
        if (body.Length == 0 || body == "+" || body == "-")
            return new Complex(new Rl("0"), Rl.Parse(body == "-" ? "-1" : "1", provider));

        // Find the last '+'/'-' that is not a leading sign → splits real and imaginary parts.
        int split = -1;
        for (int i = body.Length - 1; i > 0; i--)
        {
            if (body[i] is '+' or '-') { split = i; break; }
        }

        if (split < 0)
            return new Complex(new Rl("0"), Rl.Parse(body, provider));   // pure imaginary "2i", "-2i"

        string rePart = body[..split].Trim();
        string imPart = body[split..].Trim();
        char sign = imPart[0];
        string imDigits = imPart[1..].Trim();
        if (imDigits.Length == 0) imDigits = "1";
        imPart = sign == '-' ? "-" + imDigits : imDigits;
        return new Complex(Rl.Parse(rePart, provider), Rl.Parse(imPart, provider));
    }
}
