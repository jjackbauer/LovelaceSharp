using System.Globalization;
using Lovelace.Abstractions;
using Cplx = global::Lovelace.Complex.Complex;
using Cplx128 = global::Lovelace.Complex.LComplex128;
using Int = global::Lovelace.Integer.Integer;
using Nat = global::Lovelace.Natural.Natural;
using Rl = global::Lovelace.Real.Real;
using LPromote = global::Lovelace.Real.LRealPromoteException;

namespace Lovelace.Dsp;

/// <summary>
/// The DSP extension as a Modus plugin: implements <see cref="IModusPlugin"/> so hosts opt in
/// through <c>LoadPlugin(new DspPlugin())</c>, exactly like any other extension package.
/// Registers the DSP builtins (<c>conv</c>, <c>dft</c>, <c>fft</c>, <c>filter</c>,
/// <c>movingavg</c>, <c>impulse</c>, <c>step</c>, <c>cosine</c>, <c>exponential</c>,
/// <c>powerseries</c>, <c>noise</c>, <c>delay</c>, <c>scale</c>, and the complex accessors
/// <c>re</c>/<c>im</c>/<c>conj</c>) over the language's arbitrary-precision scalars.
/// </summary>
/// <remarks>
/// The plugin depends only on <c>Lovelace.Abstractions</c> and the scalar projects
/// (<c>Natural</c>/<c>Integer</c>/<c>Real</c>/<c>Complex</c>): arguments arrive via
/// <see cref="IModusContext.RegisterBuiltin"/> as the public scalar types (arrays as flat
/// lists of them), and the plugin returns those same types — the core owns the
/// <c>Value</c> mapping. Real ↔ Complex coercion lives here; no IEEE floating point is
/// involved. Transcendental calls resolve the active precision scope
/// (<see cref="Rl.MaxComputationDecimalPlaces"/>) so <c>setprecision</c> governs them exactly
/// like the other builtins.
/// </remarks>
public sealed class DspPlugin : IModusPlugin
{
    /// <summary>
    /// The fixed-width budget gate: below this many computation digits the whole-array
    /// operations run on <see cref="Cplx128"/> (38 significant digits, exact decimal, no IEEE)
    /// and silently promote to the arbitrary-precision <see cref="DspMath"/> path on
    /// <see cref="LPromote"/>. Matches the scalar gate in <c>Lovelace.Complex.Complex.Binary</c>.
    /// </summary>
    private const long FixedBudget = 37L;

    /// <summary>Plugin identity, reported to the host.</summary>
    public string Name => "Lovelace.Dsp";

    /// <summary>Registers the DSP builtins on the Modus context.</summary>
    public void Register(IModusContext context)
    {
        ArgumentNullException.ThrowIfNull(context);

        Register(context, "conv", new[] { "x", "h" }, args =>
        {
            Require(args, 2, "conv");
            var x = ToComplexArray(args[0]);
            var h = ToComplexArray(args[1]);
            return FromComplexArray(FixedOrExact(x, h, FixedDsp.Convolve, () => DspMath.Convolve(x, h)));
        });

        Register(context, "dft", new[] { "x" }, args =>
        {
            // dft(x): whole-signal DFT; dft(x, n): zero-padded (or truncated) to n samples.
            if (args.Count == 1)
            {
                Require(args, 1, "dft");
                return FromComplexArray(DspMath.Dft(ToComplexArray(args[0])));
            }
            if (args.Count == 2)
            {
                // the transform length is bounded where it is READ, so an out-of-range length is
                // refused by the same contract every other integer argument uses
                int n = (int)RequireInteger(args[1], "dft", "n", new Int64Range(0L, int.MaxValue), 2);
                return FromComplexArray(DspMath.Dft(DftPadded(ToComplexArray(args[0]), n)));
            }
            throw new InvalidOperationException($"dft() expects 1 or 2 argument(s), but got {args.Count}.");
        });

        Register(context, "fft", new[] { "x" }, args =>
        {
            Require(args, 1, "fft");
            return FromComplexArray(DspMath.Fft(ToComplexArray(args[0])));
        });

        Register(context, "filter", new[] { "a", "b", "x" }, args =>
        {
            Require(args, 3, "filter");
            var a = ToComplexArray(args[0]);
            var b = ToComplexArray(args[1]);
            // filter(a, b, x): difference-equation filter of the signal x (D8 fixed — pure function).
            // filter(a, b, n): impulse response of length n (scalar third argument).
            if (args[2] is IReadOnlyList<object?> x)
                return FromComplexArray(DspMath.Filter(a, b, ToComplexArray(x)));
            long n = ToNonNegative(args[2], "filter", "x", 3);
            return FromComplexArray(FixedOrExact(a, b,
                (fa, fb) => FixedDsp.ImpulseResponse(fa, fb, n),
                () => DspMath.ImpulseResponse(a, b, n)));
        });

        Register(context, "movingavg", new[] { "x", "w" }, args =>
        {
            Require(args, 2, "movingavg");
            // The window is read through the contract with its OWN bound rather than a separate
            // guard, so a zero or negative window crosses as the same recoverable argument error as
            // every other integer argument here instead of the InvalidOperation/DomainError pair the
            // runner reserves for a genuine domain refusal.
            long w = RequireInteger(args[1], "movingavg", "w", Int64Range.Positive, 2);
            var x = ToComplexArray(args[0]);
            if (x.Length == 0)
                return Array.Empty<Cplx>();
            return FromComplexArray(FixedOrExact(x, w,
                (fx, fw) => FixedDsp.MovingAverage(fx, fw),
                () => Signal.Sample(new DspMath.MovingAverage(w, new Sequence(0, x.Length - 1, x)), 0, x.Length - 1)));
        });

        Register(context, "impulse", new[] { "n" }, args =>
        {
            Require(args, 1, "impulse");
            long n = ToNonNegative(args[0], "impulse", "n", 1);
            return n == 0 ? Array.Empty<Cplx>() : FromComplexArray(Signal.Sample(new Impulse(), 0, n - 1));
        });

        Register(context, "step", new[] { "n" }, args =>
        {
            Require(args, 1, "step");
            long n = ToNonNegative(args[0], "step", "n", 1);
            return n == 0 ? Array.Empty<Cplx>() : FromComplexArray(Signal.Sample(new Step(), 0, n - 1));
        });

        Register(context, "cosine", new[] { "freq", "phase", "n" }, args =>
        {
            Require(args, 3, "cosine");
            long n = ToNonNegative(args[2], "cosine", "n", 3);
            var cosine = new Cosine(ToReal(args[0]), ToReal(args[1]));
            return n == 0 ? Array.Empty<Cplx>() : FromComplexArray(Signal.Sample(cosine, 0, n - 1));
        });

        Register(context, "exponential", new[] { "c", "n" }, args =>
        {
            Require(args, 2, "exponential");
            long n = ToNonNegative(args[1], "exponential", "n", 2);
            var exponential = new Exponential(ToComplex(args[0]));
            return n == 0 ? Array.Empty<Cplx>() : FromComplexArray(Signal.Sample(exponential, 0, n - 1));
        });

        Register(context, "powerseries", new[] { "k", "a", "n" }, args =>
        {
            Require(args, 3, "powerseries");
            long n = ToNonNegative(args[2], "powerseries", "n", 3);
            var series = new PowerSeries(ToComplex(args[0]), ToComplex(args[1]));
            return n == 0 ? Array.Empty<Cplx>() : FromComplexArray(Signal.Sample(series, 0, n - 1));
        });

        // The declared parameters are the FOUR-argument form's, so position 3 is named `seed` — which
        // is what the code reads there. Before this the registration said ("scale", "disp", "n") while
        // the four-argument branch read args[2] as the SEED and args[3] as the count, so the same
        // position carried two names and the registry did not describe the call it was validating
        // (the metadata table below already said ("scale", "disp", "n", "seed")).
        //
        // OPEN, and deliberately NOT decided here: `noise(1, 0, 2)` is the three-argument COUNT form and
        // `noise(1, 0, 2, 2)` is the four-argument SEED form, so position 3 means "how many samples" in
        // one call and "which seed" in the other. The comment above has always claimed the short form
        // defaults the seed to 0, and measured: `noise(1, 0, 2)` ANSWERS DIFFERENTLY from
        // `noise(1, 0, 0, 2)` (both are two samples with seed 0), so the source comment is wrong about
        // its own code. Whether the short form should take a seed, and how that could be done without
        // breaking every recorded `noise(scale, disp, n)` call, is a language-surface decision with an
        // API-compatibility consequence — recorded in the cycle evidence rather than guessed at.
        Register(context, "noise", new[] { "scale", "disp", "seed", "n" }, args =>
        {
            // noise(scale, disp, n) — seed defaults to 0 (reproducible); noise(scale, disp, seed, n)
            // takes an explicit seed. Digits are drawn at the active precision (no hard-coded budget).
            int seed = 0;
            long n;
            if (args.Count == 3)
            {
                Require(args, 3, "noise");
                n = ToNonNegative(args[2], "noise", "n", 3);
            }
            else if (args.Count == 4)
            {
                seed = ToSeed(args[2], "noise", "seed", 3);
                n = ToNonNegative(args[3], "noise", "n", 4);
            }
            else
            {
                throw new InvalidOperationException($"noise() expects 3 or 4 argument(s), but got {args.Count}.");
            }
            var noise = new Noise(ToReal(args[0]), ToReal(args[1]), seed);
            return n == 0 ? Array.Empty<Cplx>() : FromComplexArray(Signal.Sample(noise, 0, n - 1));
        });

        Register(context, "delay", new[] { "x", "k" }, args =>
        {
            Require(args, 2, "delay");
            long k = ToLong(args[1], "delay", "k", 2);
            var x = ToComplexArray(args[0]);
            return FromComplexArray(Signal.Sample(new Delay(k, new Sequence(0, x.Length - 1, x)), 0, x.Length - 1));
        });

        // scale(x, c): the multiplier is a COMPLEX SCALAR, not an integer. The descriptor used to
        // declare "k", which named an integer argument this builtin never reads — ToComplex takes any
        // numeric payload — so the registry advertised a refusal that did not exist and
        // scale([1, 2, 3], 2.5) was accepted while its own published signature said it could not be.
        // The parameter is NAMED after what it is; the acceptance is unchanged.
        Register(context, "scale", new[] { "x", "c" }, args =>
        {
            Require(args, 2, "scale");
            var x = ToComplexArray(args[0]);
            var c = ToComplex(args[1]);
            return FromComplexArray(Signal.Sample(new Scalar(c, new Sequence(0, x.Length - 1, x)), 0, x.Length - 1));
        });

        Register(context, "re", new[] { "x" }, args =>
        {
            Require(args, 1, "re");
            return args[0] is IReadOnlyList<object?> elements
                ? MapElements(elements, c => c.Re)
                : ToComplex(args[0]).Re;
        });

        Register(context, "im", new[] { "x" }, args =>
        {
            Require(args, 1, "im");
            return args[0] is IReadOnlyList<object?> elements
                ? MapElements(elements, c => c.Im)
                : ToComplex(args[0]).Im;
        });

        Register(context, "conj", new[] { "x" }, args =>
        {
            Require(args, 1, "conj");
            return args[0] is IReadOnlyList<object?> elements
                ? MapElements(elements, c => c.Conjugate)
                : ToComplex(args[0]).Conjugate;
        });
    }

    /// <summary>Complete help metadata for every DSP builtin: no user-facing function may ship
    /// without a summary, parameters and a return kind.</summary>
    private static readonly Dictionary<string, BuiltinDescriptor> Metadata = new(StringComparer.Ordinal)
    {
        ["conv"] = new("conv", new[] { "x", "h" }, BuiltinCategories.Dsp,
            "Linear convolution of two signals.", ["conv([1, 2], [1, 1])"], "Vector", ["filter", "fft"]),
        ["dft"] = new("dft", new[] { "x", "n" }, BuiltinCategories.Dsp,
            "Discrete Fourier transform; an optional length n zero-pads or truncates the signal.",
            ["dft([0, 1, 0, 0])"], "Vector", ["fft"], MinArity: 1),
        // `fft` takes ONE argument and the body enforces it (Require(args, 1, "fft")), so `fft(x, n)` is
        // an ARITY refusal, not an integer-argument refusal. That distinction matters to the ADV-03
        // census: a matrix row for an `n` here would never reach the argument, and the brief's reading
        // that `fft` accepts an optional length beside `dft` is WRONG — only `dft` declares one.
        ["fft"] = new("fft", new[] { "x" }, BuiltinCategories.Dsp,
            "Fast Fourier transform of a signal.", ["fft([0, 1, 0, 0])"], "Vector", ["dft", "conv"]),
        ["filter"] = new("filter", new[] { "a", "b", "x" }, BuiltinCategories.Dsp,
            "Difference-equation filter of a signal (or the impulse response of length n).",
            ["filter([1], [1], [1, 2, 3])"], "Vector", ["conv", "movingavg"]),
        ["movingavg"] = new("movingavg", new[] { "x", "w" }, BuiltinCategories.Dsp,
            "Moving average of a signal with window w.", ["movingavg([1, 2, 3], 2)"], "Vector", ["filter"]),
        ["impulse"] = new("impulse", new[] { "n" }, BuiltinCategories.Dsp,
            "Unit impulse sequence of length n.", ["impulse(4)"], "Vector", ["step"]),
        ["step"] = new("step", new[] { "n" }, BuiltinCategories.Dsp,
            "Unit step sequence of length n.", ["step(4)"], "Vector", ["impulse"]),
        ["cosine"] = new("cosine", new[] { "freq", "phase", "n" }, BuiltinCategories.Dsp,
            "Cosine sequence of length n at the given frequency and phase.", ["cosine(0.25, 0, 8)"], "Vector", ["exponential"]),
        ["exponential"] = new("exponential", new[] { "c", "n" }, BuiltinCategories.Dsp,
            "Exponential sequence c^k of length n.", ["exponential(0.5, 4)"], "Vector", ["cosine", "powerseries"]),
        ["powerseries"] = new("powerseries", new[] { "k", "a", "n" }, BuiltinCategories.Dsp,
            "Geometric power series of length n.", ["powerseries(2, 0.5, 4)"], "Vector", ["exponential"]),
        ["noise"] = new("noise", new[] { "scale", "disp", "n", "seed" }, BuiltinCategories.Dsp,
            "Deterministic seeded noise sequence (the same seed always yields the same signal).",
            ["noise(1, 0, 4, 3)"], "Vector", ["cosine"], MinArity: 3),
        ["delay"] = new("delay", new[] { "x", "k" }, BuiltinCategories.Dsp,
            "Delays a signal by k samples.", ["delay([1, 2, 3], 1)"], "Vector", ["scale"]),
        ["scale"] = new("scale", new[] { "x", "c" }, BuiltinCategories.Dsp,
            "Scales every sample of a signal by the scalar c.", ["scale([1, 2, 3], 2)", "scale([1, 2, 3], 0.5)"], "Vector", ["delay"]),
        ["re"] = new("re", new[] { "x" }, BuiltinCategories.Dsp,
            "Real part of a complex value or signal.", ["re(fft([0, 1, 0, 0])[1])"], "Real | Vector", ["im", "conj", "abs"]),
        ["im"] = new("im", new[] { "x" }, BuiltinCategories.Dsp,
            "Imaginary part of a complex value or signal.", ["im(fft([0, 1, 0, 0])[1])"], "Real | Vector", ["re", "conj"]),
        ["conj"] = new("conj", new[] { "x" }, BuiltinCategories.Dsp,
            "Complex conjugate of a complex value or signal.", ["conj(fft([0, 1, 0, 0])[1])"], "Complex | Vector", ["re", "im"]),
    };

    /// <summary>
    /// Registers a builtin through the typed <see cref="ScalarResult"/> channel — the plugin's
    /// results flow through the wrapper the core unwraps at the boundary. Every DSP builtin
    /// carries complete help metadata from <see cref="Metadata"/>.
    /// </summary>
    private static void Register(IModusContext context, string name, string[] parameters, Func<IReadOnlyList<object?>, object?> implementation) =>
        context.RegisterBuiltin(
            Metadata.TryGetValue(name, out var descriptor)
                ? descriptor
                : new BuiltinDescriptor(name, parameters, BuiltinCategories.Dsp, "(no summary registered)", Array.Empty<string>(), "Vector | Real"),
            args => ScalarResult.From(implementation(args)));

    /// <summary>
    /// Runs the whole-array fixed-width fast path (<see cref="FixedDsp"/> over <see cref="Cplx128"/>)
    /// when the active computation budget fits the fixed width, silently promoting to the
    /// arbitrary-precision <paramref name="exact"/> path when an operand does not fit or the
    /// operation overflows the fixed width (<see cref="LPromote"/>). The fast path never rounds —
    /// promotion preserves exactness.
    /// </summary>
    private static Cplx[] FixedOrExact(
        Cplx[] x,
        Cplx[] h,
        Func<Cplx128[], Cplx128[], Cplx128[]> fast,
        Func<Cplx[]> exact)
    {
        if (Rl.MaxComputationDecimalPlaces <= FixedBudget
            && TryConvert(x, out var x128) && TryConvert(h, out var h128))
        {
            try
            {
                return ConvertBack(fast(x128, h128));
            }
            catch (LPromote)
            {
                // Silent promotion: the fixed-width result would exceed 38 significant digits.
            }
        }
        return exact();
    }

    /// <summary>Single-array overload (movingavg).</summary>
    private static Cplx[] FixedOrExact(
        Cplx[] x,
        long window,
        Func<Cplx128[], long, Cplx128[]> fast,
        Func<Cplx[]> exact)
    {
        if (Rl.MaxComputationDecimalPlaces <= FixedBudget && TryConvert(x, out var x128))
        {
            try
            {
                return ConvertBack(fast(x128, window));
            }
            catch (LPromote)
            {
                // Silent promotion: the fixed-width result would exceed 38 significant digits.
            }
        }
        return exact();
    }

    private static bool TryConvert(Cplx[] values, out Cplx128[] result)
    {
        result = new Cplx128[values.Length];
        var poll = KernelCancellation.Capture();
        for (int i = 0; i < values.Length; i++)
        {
            poll.Poll(i);
            if (!Cplx128.TryFromComplex(values[i], out result[i]))
                return false;
        }
        return true;
    }

    private static Cplx[] ConvertBack(Cplx128[] values)
    {
        var result = new Cplx[values.Length];
        var poll = KernelCancellation.Capture();
        for (int i = 0; i < values.Length; i++)
        {
            poll.Poll(i);
            result[i] = values[i].ToComplex();
        }
        return result;
    }

    private static void Require(IReadOnlyList<object?> args, int expected, string name)
    {
        if (args.Count != expected)
            throw new InvalidOperationException($"{name}() expects {expected} argument(s), but got {args.Count}.");
    }

    // -------------------------------------------------------------------------
    // Integer arguments — the ONE contract (advancement cycle 7, ADV-02)
    // -------------------------------------------------------------------------
    //
    // A window, a delay, a transform length, a sample count and a seed are all integers, and every
    // one of them used to be refused through a bare InvalidOperationException — which the runner
    // classifies as InvalidOperation/DomainError, the exact pair a genuine DOMAIN refusal uses
    // (Lovelace.Run/Runner.cs Classify). A consumer could not branch on "you gave me an argument I
    // cannot honour" versus "your mathematics is undefined", and a fractional window reported as an
    // Int64-range problem named the wrong thing entirely.
    //
    // They now go through the shared contract, which derives from ArgumentException and so crosses as
    // the documented recoverable InvalidArgument/TypeMismatch pair, and which NAMES the value that
    // arrived instead of truncating it or wrapping it.

    /// <summary>The exact integer a payload denotes, from the payload's OWN representation.
    /// <para>
    /// <b>The Real arm is REPEATED, deliberately.</b> <c>Real</c> derives from <c>Integer</c>, so the
    /// plain <c>Int</c> arm would accept a non-integral <c>Real</c> through the base predicate and
    /// this helper would truncate — the very defect it exists to remove. <c>Real.TryGetExactInt64</c>
    /// is the override that answers whether the value IS an integer, and it answers false for 3.7.
    /// </para>
    /// <para>
    /// A count wider than Int64 answers <see langword="false"/> rather than throwing, so the contract
    /// refuses it and names it.</para>
    /// </summary>
    private static bool ReadExactInteger(object? payload, out long value)
    {
        switch (payload)
        {
            case Rl real:
                return real.TryGetExactInt64(out value);
            case Int integer:
                return integer.TryGetExactInt64(out value);
            case Nat natural:
                return natural.TryGetExactInt64(out value);
            case long l:
                value = l;
                return true;
            default:
                value = 0L;
                return false;
        }
    }

    /// <summary>Reads an integer argument in [<paramref name="bounds"/>], or refuses it as the
    /// documented argument error. The refusal distinguishes "that is not a kind of thing I take"
    /// from "that is outside the range I take", because the repair differs.</summary>
    private static long RequireInteger(
        object? value, string name, string parameter, Int64Range bounds, int position) =>
        ExactIntegerArgument.Require(
            value, parameter, bounds, ReadExactInteger,
            payload => DescribeIntegerRefusal(payload, name, parameter, bounds, position), name, position);

    /// <summary>
    /// The refusal for a payload the reader rejected. The <c>Real</c> arm excludes it from the
    /// exact-integer-type arm explicitly, for the same subtype reason the reader repeats its arms:
    /// a bare Natural-or-Integer case would match a fractional <c>Real</c> and report "3.7 is too
    /// wide", which is not what is wrong with 3.7.
    /// </summary>
    private static ArgumentContractException DescribeIntegerRefusal(
        object? payload, string name, string parameter, Int64Range bounds, int position)
    {
        switch (payload)
        {
            case Rl real when real.TryGetExactInt64(out long exact):
                return ArgumentContractException.NotAnIntegerKind(
                    name, parameter, bounds, exact.ToString(), position);
            case Rl:
                return ArgumentContractException.NotAnInteger(
                    name, parameter, bounds, TypeName(payload) + " " + payload, position);
            case Int or Nat:
                return ArgumentContractException.TooWide(
                    name, parameter, bounds, payload.ToString()!, position);
            default:
                return ArgumentContractException.NotAnInteger(
                    name, parameter, bounds, payload is null ? "nothing" : TypeName(payload), position);
        }
    }

    /// <summary>An integer argument with no further constraint: the whole signed 64-bit range.</summary>
    private static long ToLong(object? value, string name, string parameter, int position) =>
        RequireInteger(value, name, parameter, Int64Range.Unbounded, position);

    /// <summary>A sample count: <c>0</c> is meaningful (the empty signal), a negative count is not.</summary>
    private static long ToNonNegative(object? value, string name, string parameter, int position) =>
        RequireInteger(value, name, parameter, Int64Range.NonNegative, position);

    /// <summary>A noise seed: the engine's generator takes an Int32.</summary>
    private static int ToSeed(object? value, string name, string parameter, int position) =>
        (int)RequireInteger(value, name, parameter, new Int64Range(int.MinValue, int.MaxValue), position);

    /// <summary>Zero-pads (or truncates) the input to exactly <paramref name="n"/> samples.</summary>
    private static Cplx[] DftPadded(Cplx[] x, int n)
    {
        if (n == x.Length)
            return x;
        var padded = new Cplx[n];
        int copy = Math.Min(x.Length, n);
        Array.Copy(x, padded, copy);
        var poll = KernelCancellation.Capture();
        for (int i = copy; i < n; i++)
        {
            poll.Poll(i);
            padded[i] = Cplx.Zero;
        }
        return padded;
    }

    private static object MapElements(IReadOnlyList<object?> elements, Func<Cplx, object> selector)
    {
        var result = new object?[elements.Count];
        var poll = KernelCancellation.Capture();
        for (int i = 0; i < elements.Count; i++)
        {
            poll.Poll(i);
            result[i] = selector(ToComplex(elements[i]));
        }
        return result;
    }

    private static Rl ToReal(object? value) => value switch
    {
        // Real derives from Integer, so match the narrowest reference types first.
        Rl real => real,
        Int integer => new Rl(integer),
        Nat natural => new Rl(new Int(natural)),
        _ => throw new InvalidOperationException($"Expected a numeric argument, but got '{TypeName(value)}'."),
    };

    private static Cplx ToComplex(object? value) => value switch
    {
        Cplx complex => complex,
        Rl real => new Cplx(real, DspUtil.Zero),
        Int integer => new Cplx(new Rl(integer), DspUtil.Zero),
        Nat natural => new Cplx(new Rl(new Int(natural)), DspUtil.Zero),
        _ => throw new InvalidOperationException($"DSP builtins expect numeric/complex elements, but got '{TypeName(value)}'."),
    };

    private static Cplx[] ToComplexArray(object? value)
    {
        if (value is not IReadOnlyList<object?> elements)
            throw new InvalidOperationException("DSP builtins expect an array argument, but got a scalar.");
        var result = new Cplx[elements.Count];
        var poll = KernelCancellation.Capture();
        for (int i = 0; i < elements.Count; i++)
        {
            poll.Poll(i);
            result[i] = ToComplex(elements[i]);
        }
        return result;
    }

    private static object FromComplexArray(Cplx[] values) => values;

    private static string TypeName(object? value) => value?.GetType().Name ?? "null";
}
