using Cplx = global::Lovelace.Complex.Complex;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Dsp;

/// <summary>
/// Whole-array DSP operations over arbitrary-precision complex values. No IEEE floating point
/// is used anywhere in this module.
/// </summary>
public static class DspMath
{
    /// <summary>
    /// Standard linear convolution y[n] = Σ_m x[m] · h[n−m]. The result has
    /// <c>x.Count + h.Count − 1</c> samples.
    /// </summary>
    public static Cplx[] Convolve(IReadOnlyList<Cplx> x, IReadOnlyList<Cplx> h)
    {
        ArgumentNullException.ThrowIfNull(x);
        ArgumentNullException.ThrowIfNull(h);
        if (x.Count == 0 || h.Count == 0)
            return [];

        var result = new Cplx[x.Count + h.Count - 1];
        for (int n = 0; n < result.Length; n++)
        {
            Cplx sum = Cplx.Zero;
            int lo = Math.Max(0, n - h.Count + 1);
            int hi = Math.Min(x.Count - 1, n);
            for (int m = lo; m <= hi; m++)
                sum += x[m] * h[n - m];
            result[n] = sum;
        }
        return result;
    }

    /// <summary>Integer power aⁿ (negative n raises the reciprocal).</summary>
    public static Cplx PowInt(Cplx a, long n)
    {
        if (n == 0)
            return Cplx.One;
        Cplx result = Cplx.One;
        Cplx b = n > 0 ? a : a.Reciprocal;
        long e = Math.Abs(n);
        while (e > 0)
        {
            if ((e & 1) == 1)
                result *= b;
            b *= b;
            e >>= 1;
        }
        return result;
    }

    /// <summary>IIR/FIR difference-equation impulse response (direct form, zero initial state).</summary>
    public static Cplx[] ImpulseResponse(IReadOnlyList<Cplx> a, IReadOnlyList<Cplx> b, long n)
        => DifferenceEquation(a, b, n, stepInput: false);

    /// <summary>IIR/FIR difference-equation step response (direct form, zero initial state).</summary>
    public static Cplx[] StepResponse(IReadOnlyList<Cplx> a, IReadOnlyList<Cplx> b, long n)
        => DifferenceEquation(a, b, n, stepInput: true);

    /// <summary>
    /// Direct-form IIR/FIR difference equation driven by an impulse or a unit step, with zero
    /// initial state. Shared by <see cref="ImpulseResponse"/> and <see cref="StepResponse"/>.
    /// </summary>
    private static Cplx[] DifferenceEquation(IReadOnlyList<Cplx> a, IReadOnlyList<Cplx> b, long n, bool stepInput)
    {
        ArgumentNullException.ThrowIfNull(a);
        ArgumentNullException.ThrowIfNull(b);
        if (a.Count == 0)
            throw new ArgumentException("The denominator coefficient vector 'a' must not be empty.", nameof(a));

        var x = new Cplx[b.Count];   // input history
        var y = new Cplx[a.Count];   // output history
        for (int i = 0; i < x.Length; i++) x[i] = stepInput ? Cplx.One : Cplx.Zero;
        for (int i = 0; i < y.Length; i++) y[i] = Cplx.Zero;
        if (!stepInput && b.Count > 0)
            x[0] = Cplx.One;         // impulse input

        var response = new Cplx[n];
        for (long k = 0; k < n; k++)
        {
            Cplx acc = Cplx.Zero;
            for (int j = 1; j < a.Count; j++) acc -= a[j] * y[j - 1];
            for (int j = 0; j < b.Count; j++) acc += b[j] * x[j];
            acc /= a[0];
            response[k] = acc;

            for (int j = a.Count - 1; j > 0; j--) y[j] = y[j - 1];
            if (a.Count > 0) y[0] = acc;
            if (!stepInput)
            {
                for (int j = b.Count - 1; j > 0; j--) x[j] = x[j - 1];
                if (b.Count > 0) x[0] = Cplx.Zero;
            }
        }
        return response;
    }

    /// <summary>Moving average over exactly <c>window</c> samples ending at n.</summary>
    public sealed class MovingAverage : ISignal
    {
        private readonly Cplx _invWindow;

        public long Window { get; }

        public ISignal X { get; }

        public MovingAverage(long window, ISignal x)
        {
            if (window <= 0)
                throw new ArgumentOutOfRangeException(nameof(window), "Window must be positive.");
            Window = window;
            X = x ?? throw new ArgumentNullException(nameof(x));
            _invWindow = DspUtil.Real(window);
        }

        public Cplx Get(long n)
        {
            Cplx sum = Cplx.Zero;
            for (long k = n - Window + 1; k <= n; k++)
                sum += X.Get(k);
            return sum / _invWindow;
        }
    }

    /// <summary>
    /// Forward discrete Fourier transform: X[k] = Σ_n x[n]·e^(−j·2π·k·n/N), computed at the
    /// active precision (<see cref="Rl.MaxComputationDecimalPlaces"/>). The N distinct roots of
    /// unity are computed once (periodicity), with the angle reduced to lowest terms so rational
    /// roots (e.g. N = 3, 4, 6) are exact.
    /// </summary>
    public static Cplx[] Dft(IReadOnlyList<Cplx> x) => Dft(x, Rl.MaxComputationDecimalPlaces);

    /// <summary>
    /// Forward discrete Fourier transform to <paramref name="digits"/> decimal places.
    /// </summary>
    public static Cplx[] Dft(IReadOnlyList<Cplx> x, long digits)
    {
        ArgumentNullException.ThrowIfNull(x);
        long N = x.Count;
        if (N == 0)
            return [];

        // Precompute the N distinct roots of unity e^(−j·2π·k/N), k = 0..N−1.
        var roots = new Cplx[N];
        for (long k = 0; k < N; k++)
            roots[k] = RootOfUnity(k, N, digits);

        var result = new Cplx[N];
        for (long k = 0; k < N; k++)
        {
            Cplx sum = Cplx.Zero;
            for (long n = 0; n < N; n++)
                sum += x[(int)n] * roots[(int)((k * n) % N)];
            result[k] = sum;
        }
        return result;
    }

    /// <summary>
    /// Forward radix-2 decimation-in-time Cooley–Tukey FFT, O(N log N), for N a power of two,
    /// computed at the active precision (<see cref="Rl.MaxComputationDecimalPlaces"/>). Uses the
    /// same gcd-reduced roots of unity as <see cref="Dft"/>, so power-of-two twiddles hit exact
    /// special/sqrt values and the result agrees with <see cref="Dft"/> to within the last few
    /// digits (the butterfly summation order differs). Non-power-of-two lengths throw.
    /// </summary>
    public static Cplx[] Fft(IReadOnlyList<Cplx> x) => Fft(x, Rl.MaxComputationDecimalPlaces);

    /// <summary>Forward radix-2 Cooley–Tukey FFT to <paramref name="digits"/> decimal places.</summary>
    public static Cplx[] Fft(IReadOnlyList<Cplx> x, long digits)
    {
        ArgumentNullException.ThrowIfNull(x);
        int n = x.Count;
        if (n == 0)
            return [];
        if ((n & (n - 1)) != 0)
            throw new ArgumentException(
                $"FFT length must be a power of two, but got {n}.", nameof(x));

        int logN = 0;
        while ((1 << logN) < n)
            logN++;

        // Bit-reversal permutation into the working buffer.
        var a = new Cplx[n];
        for (int i = 0; i < n; i++)
            a[ReverseBits(i, logN)] = x[i];

        // Twiddle factors W_n[k] = e^(−j·2π·k/n) for k = 0 .. n/2−1, angle-reduced to lowest
        // terms so power-of-2 roots hit exact special/sqrt values (same construction as Dft).
        int halfN = n >> 1;
        var w = new Cplx[halfN];
        for (int k = 0; k < halfN; k++)
            w[k] = RootOfUnity(k, n, digits);

        // Cooley–Tukey butterflies: t = W · x[k+j+h/2], u = x[k+j];
        // x[k+j] = u + t, x[k+j+h/2] = u − t.
        for (int len = 2; len <= n; len <<= 1)
        {
            int half = len >> 1;
            int step = n / len;
            for (int i = 0; i < n; i += len)
            {
                for (int j = 0; j < half; j++)
                {
                    Cplx t = w[j * step] * a[i + j + half];
                    Cplx u = a[i + j];
                    a[i + j] = u + t;
                    a[i + j + half] = u - t;
                }
            }
        }

        return a;
    }

    /// <summary>
    /// The k-th root of unity e^(−j·2π·k/n), with the angle reduced to lowest terms
    /// (gcd → <c>Pi·num/den</c>) so rational roots are exact. Shared by <see cref="Dft"/>
    /// and <see cref="Fft"/> so the two transforms use one construction.
    /// </summary>
    private static Cplx RootOfUnity(long k, long n, long digits)
    {
        long g = Gcd(2 * k, n);
        long num = (2 * k) / g;
        long den = n / g;
        Rl angle = num == 0 ? DspUtil.Zero : Rl.Pi * DspUtil.RealOf(num) / DspUtil.RealOf(den);
        return new Cplx(Rl.Cos(angle, digits), -Rl.Sin(angle, digits));
    }

    private static int ReverseBits(int value, int bits)
    {
        int result = 0;
        for (int i = 0; i < bits; i++)
        {
            result = (result << 1) | (value & 1);
            value >>= 1;
        }
        return result;
    }

    private static long Gcd(long a, long b)
    {
        while (b != 0)
            (a, b) = (b, a % b);
        return a < 0 ? -a : a;
    }
}
