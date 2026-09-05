using Cplx = global::Lovelace.Complex.Complex;
using Int = global::Lovelace.Integer.Integer;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Dsp;

/// <summary>A discrete-time signal: an arbitrary-precision complex value at each integer index n.</summary>
public interface ISignal
{
    Cplx Get(long n);
}

/// <summary>Sampling helpers over an <see cref="ISignal"/>.</summary>
public static class Signal
{
    /// <summary>Samples <paramref name="signal"/> over the inclusive range [begin, end].</summary>
    public static Cplx[] Sample(ISignal signal, long begin, long end)
    {
        ArgumentNullException.ThrowIfNull(signal);
        if (end < begin)
            throw new ArgumentOutOfRangeException(nameof(end), "end must be ≥ begin.");
        if (end - begin + 1 > int.MaxValue)
            throw new ArgumentOutOfRangeException(nameof(end),
                $"The sample range [{begin}, {end}] spans more than {int.MaxValue} samples.");
        var result = new Cplx[checked((int)(end - begin + 1))];
        for (long n = begin; n <= end; n++)
            result[n - begin] = signal.Get(n);
        return result;
    }
}

/// <summary>A finite complex sequence over [Lower, Upper]; zero outside its support.</summary>
public sealed class Sequence : ISignal
{
    public long Lower { get; }
    public long Upper { get; }
    public Cplx[] Values { get; }

    public Sequence(long lower, long upper, Cplx[] values)
    {
        ArgumentNullException.ThrowIfNull(values);
        if (values.Length != upper - lower + 1)
            throw new ArgumentException(
                $"Sequence over [{lower},{upper}] requires {upper - lower + 1} value(s), but got {values.Length}.", nameof(values));
        Lower = lower;
        Upper = upper;
        Values = values;
    }

    public Cplx Get(long n) => n >= Lower && n <= Upper ? Values[n - Lower] : Cplx.Zero;
}

/// <summary>Unit impulse δ[n]: 1 at n == 0, else 0.</summary>
public sealed record Impulse : ISignal
{
    public Cplx Get(long n) => n == 0 ? Cplx.One : Cplx.Zero;
}

/// <summary>Unit step u[n]: 1 for n ≥ 0, else 0.</summary>
public sealed record Step : ISignal
{
    public Cplx Get(long n) => n >= 0 ? Cplx.One : Cplx.Zero;
}

/// <summary>Scales a signal by a complex constant: k·x(n).</summary>
public sealed record Scalar(Cplx K, ISignal X) : ISignal
{
    public Cplx Get(long n) => K * X.Get(n);
}

/// <summary>Point-wise sum: x(n) + y(n).</summary>
public sealed record Sum(ISignal X, ISignal Y) : ISignal
{
    public Cplx Get(long n) => X.Get(n) + Y.Get(n);
}

/// <summary>Point-wise product: x(n) · y(n).</summary>
public sealed record Product(ISignal X, ISignal Y) : ISignal
{
    public Cplx Get(long n) => X.Get(n) * Y.Get(n);
}

/// <summary>Delays a signal by a sample count: x(n − amount).</summary>
public sealed record Delay(long Amount, ISignal X) : ISignal
{
    public Cplx Get(long n) => X.Get(n - Amount);
}

/// <summary>Power series: k · n · aⁿ.</summary>
public sealed record PowerSeries(Cplx K, Cplx A) : ISignal
{
    public Cplx Get(long n) => K * DspMath.PowInt(A, n) * DspUtil.Real(n);
}

/// <summary>Shared helpers for the DSP core.</summary>
internal static class DspUtil
{
    /// <summary>The exact real zero, allocated fresh per access (patterns §5).</summary>
    public static Rl Zero => new Rl("0");

    /// <summary>The exact real number n, as a complex value.</summary>
    public static Cplx Real(long n) => new Cplx(new Rl(new Int(n)), Zero);

    /// <summary>The exact real number n.</summary>
    public static Rl RealOf(long n) => new Rl(new Int(n));
}

/// <summary>
/// Cosine signal cos(2π·freq·n + phase); freq in cycles/sample, phase in radians. When
/// <see cref="Digits"/> is null the transcendental is evaluated at the active precision
/// (<see cref="Rl.MaxComputationDecimalPlaces"/>).
/// </summary>
public sealed record Cosine(Rl Frequency, Rl Phase, long? Digits = null) : ISignal
{
    public Cplx Get(long n)
    {
        Rl angle = Rl.Pi * new Rl("2") * Frequency * DspUtil.RealOf(n) + Phase;
        return new Cplx(Rl.Cos(angle, Digits ?? Rl.MaxComputationDecimalPlaces), DspUtil.Zero);
    }
}

/// <summary>Complex exponential signal e^(c·n), evaluated at the active precision unless <see cref="Digits"/> is set.</summary>
public sealed record Exponential(Cplx C, long? Digits = null) : ISignal
{
    public Cplx Get(long n) => (C * DspUtil.Real(n)).Exp(Digits ?? Rl.MaxComputationDecimalPlaces);
}

/// <summary>
/// Random noise: scale·u + displacement per component, with u uniform in [0,1). Digits are drawn
/// from an integer RNG and assembled into an exact <see cref="Rl"/>, so no IEEE floating point is
/// used. Seeded for reproducibility.
/// </summary>
public sealed class Noise : ISignal
{
    private readonly Rl _scale;
    private readonly Rl _disp;
    private readonly Random _random;
    private readonly int _digits;

    public Noise(Rl scale, Rl disp, int seed, int digits = 30)
    {
        _scale = scale ?? throw new ArgumentNullException(nameof(scale));
        _disp = disp ?? throw new ArgumentNullException(nameof(disp));
        _random = new Random(seed);
        _digits = digits;
    }

    public Cplx Get(long n)
    {
        Rl uRe = NextUnit(_random, _digits);
        Rl uIm = NextUnit(_random, _digits);
        return new Cplx(_scale * uRe + _disp, _scale * uIm + _disp);
    }

    private static Rl NextUnit(Random random, int digits)
    {
        var sb = new System.Text.StringBuilder("0.");
        for (int i = 0; i < digits; i++)
            sb.Append((char)('0' + random.Next(0, 10)));
        return new Rl(sb.ToString());
    }
}
