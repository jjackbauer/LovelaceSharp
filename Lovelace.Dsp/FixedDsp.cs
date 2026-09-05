using Cplx64 = global::Lovelace.Complex.LComplex64;
using Cplx128 = global::Lovelace.Complex.LComplex128;
using LReal64 = global::Lovelace.Real.LReal64;
using LReal128 = global::Lovelace.Real.LReal128;

namespace Lovelace.Dsp;

/// <summary>
/// Whole-array DSP operations over the fixed-width complex structs
/// (<see cref="Cplx64"/> over <c>LReal64</c> — up to 19 significant digits — and
/// <see cref="Cplx128"/> over <c>LReal128</c> — up to 38 significant digits). Mirrors
/// <see cref="DspMath"/>'s semantics exactly, but throws
/// <see cref="global::Lovelace.Real.LRealPromoteException"/> (from the components) rather than
/// rounding when a result exceeds the fixed width — the caller should silently promote to
/// <see cref="DspMath"/> (the arbitrary-precision class form). No IEEE floating point is used
/// anywhere.
/// </summary>
public static class FixedDsp
{
    // -----------------------------------------------------------------
    // Convolution
    // -----------------------------------------------------------------

    /// <summary>Linear convolution y[n] = Σ_m x[m]·h[n−m] over fixed-width 64-bit components.</summary>
    public static Cplx64[] Convolve(IReadOnlyList<Cplx64> x, IReadOnlyList<Cplx64> h)
    {
        ArgumentNullException.ThrowIfNull(x);
        ArgumentNullException.ThrowIfNull(h);
        if (x.Count == 0 || h.Count == 0)
            return [];

        var result = new Cplx64[x.Count + h.Count - 1];
        for (int n = 0; n < result.Length; n++)
        {
            Cplx64 sum = Cplx64.Zero;
            int lo = Math.Max(0, n - h.Count + 1);
            int hi = Math.Min(x.Count - 1, n);
            for (int m = lo; m <= hi; m++)
                sum += x[m] * h[n - m];
            result[n] = sum;
        }
        return result;
    }

    /// <summary>Linear convolution y[n] = Σ_m x[m]·h[n−m] over fixed-width 128-bit components.</summary>
    public static Cplx128[] Convolve(IReadOnlyList<Cplx128> x, IReadOnlyList<Cplx128> h)
    {
        ArgumentNullException.ThrowIfNull(x);
        ArgumentNullException.ThrowIfNull(h);
        if (x.Count == 0 || h.Count == 0)
            return [];

        var result = new Cplx128[x.Count + h.Count - 1];
        for (int n = 0; n < result.Length; n++)
        {
            Cplx128 sum = Cplx128.Zero;
            int lo = Math.Max(0, n - h.Count + 1);
            int hi = Math.Min(x.Count - 1, n);
            for (int m = lo; m <= hi; m++)
                sum += x[m] * h[n - m];
            result[n] = sum;
        }
        return result;
    }

    // -----------------------------------------------------------------
    // Difference equation (impulse / step response)
    // -----------------------------------------------------------------

    /// <summary>IIR/FIR impulse response (direct form, zero initial state) over fixed-width 64-bit components.</summary>
    public static Cplx64[] ImpulseResponse(IReadOnlyList<Cplx64> a, IReadOnlyList<Cplx64> b, long n)
        => DifferenceEquation(a, b, n, stepInput: false);

    /// <summary>IIR/FIR step response (direct form, zero initial state) over fixed-width 64-bit components.</summary>
    public static Cplx64[] StepResponse(IReadOnlyList<Cplx64> a, IReadOnlyList<Cplx64> b, long n)
        => DifferenceEquation(a, b, n, stepInput: true);

    /// <summary>IIR/FIR impulse response (direct form, zero initial state) over fixed-width 128-bit components.</summary>
    public static Cplx128[] ImpulseResponse(IReadOnlyList<Cplx128> a, IReadOnlyList<Cplx128> b, long n)
        => DifferenceEquation(a, b, n, stepInput: false);

    /// <summary>IIR/FIR step response (direct form, zero initial state) over fixed-width 128-bit components.</summary>
    public static Cplx128[] StepResponse(IReadOnlyList<Cplx128> a, IReadOnlyList<Cplx128> b, long n)
        => DifferenceEquation(a, b, n, stepInput: true);

    private static Cplx64[] DifferenceEquation(IReadOnlyList<Cplx64> a, IReadOnlyList<Cplx64> b, long n, bool stepInput)
    {
        ArgumentNullException.ThrowIfNull(a);
        ArgumentNullException.ThrowIfNull(b);
        if (a.Count == 0)
            throw new ArgumentException("The denominator coefficient vector 'a' must not be empty.", nameof(a));

        var x = new Cplx64[b.Count];   // input history
        var y = new Cplx64[a.Count];   // output history
        for (int i = 0; i < x.Length; i++) x[i] = stepInput ? Cplx64.One : Cplx64.Zero;
        if (!stepInput && b.Count > 0)
            x[0] = Cplx64.One;         // impulse input

        var response = new Cplx64[n];
        for (long k = 0; k < n; k++)
        {
            Cplx64 acc = Cplx64.Zero;
            for (int j = 1; j < a.Count; j++) acc -= a[j] * y[j - 1];
            for (int j = 0; j < b.Count; j++) acc += b[j] * x[j];
            acc /= a[0];
            response[k] = acc;

            for (int j = a.Count - 1; j > 0; j--) y[j] = y[j - 1];
            if (a.Count > 0) y[0] = acc;
            if (!stepInput)
            {
                for (int j = b.Count - 1; j > 0; j--) x[j] = x[j - 1];
                if (b.Count > 0) x[0] = Cplx64.Zero;
            }
        }
        return response;
    }

    private static Cplx128[] DifferenceEquation(IReadOnlyList<Cplx128> a, IReadOnlyList<Cplx128> b, long n, bool stepInput)
    {
        ArgumentNullException.ThrowIfNull(a);
        ArgumentNullException.ThrowIfNull(b);
        if (a.Count == 0)
            throw new ArgumentException("The denominator coefficient vector 'a' must not be empty.", nameof(a));

        var x = new Cplx128[b.Count];   // input history
        var y = new Cplx128[a.Count];   // output history
        for (int i = 0; i < x.Length; i++) x[i] = stepInput ? Cplx128.One : Cplx128.Zero;
        if (!stepInput && b.Count > 0)
            x[0] = Cplx128.One;         // impulse input

        var response = new Cplx128[n];
        for (long k = 0; k < n; k++)
        {
            Cplx128 acc = Cplx128.Zero;
            for (int j = 1; j < a.Count; j++) acc -= a[j] * y[j - 1];
            for (int j = 0; j < b.Count; j++) acc += b[j] * x[j];
            acc /= a[0];
            response[k] = acc;

            for (int j = a.Count - 1; j > 0; j--) y[j] = y[j - 1];
            if (a.Count > 0) y[0] = acc;
            if (!stepInput)
            {
                for (int j = b.Count - 1; j > 0; j--) x[j] = x[j - 1];
                if (b.Count > 0) x[0] = Cplx128.Zero;
            }
        }
        return response;
    }

    // -----------------------------------------------------------------
    // Moving average
    // -----------------------------------------------------------------

    /// <summary>Moving average over exactly <c>window</c> samples ending at n, over fixed-width 64-bit components.</summary>
    public static Cplx64[] MovingAverage(IReadOnlyList<Cplx64> x, long window)
    {
        ArgumentNullException.ThrowIfNull(x);
        if (window <= 0)
            throw new ArgumentOutOfRangeException(nameof(window), "Window must be positive.");

        var divisor = LReal64.Parse(window.ToString());
        var result = new Cplx64[x.Count];
        for (int n = 0; n < x.Count; n++)
        {
            Cplx64 sum = Cplx64.Zero;
            for (long k = n - window + 1; k <= n; k++)
                if (k >= 0)
                    sum += x[(int)k];
            result[n] = sum / divisor;
        }
        return result;
    }

    /// <summary>Moving average over exactly <c>window</c> samples ending at n, over fixed-width 128-bit components.</summary>
    public static Cplx128[] MovingAverage(IReadOnlyList<Cplx128> x, long window)
    {
        ArgumentNullException.ThrowIfNull(x);
        if (window <= 0)
            throw new ArgumentOutOfRangeException(nameof(window), "Window must be positive.");

        var divisor = LReal128.Parse(window.ToString());
        var result = new Cplx128[x.Count];
        for (int n = 0; n < x.Count; n++)
        {
            Cplx128 sum = Cplx128.Zero;
            for (long k = n - window + 1; k <= n; k++)
                if (k >= 0)
                    sum += x[(int)k];
            result[n] = sum / divisor;
        }
        return result;
    }
}
