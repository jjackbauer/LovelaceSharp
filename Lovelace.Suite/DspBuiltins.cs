using Cplx = global::Lovelace.Complex.Complex;
using Int = global::Lovelace.Integer.Integer;
using Rl = global::Lovelace.Real.Real;
using Lovelace.Dsp;

namespace Lovelace.Suite;

/// <summary>
/// Registers the DSP builtins (<c>conv</c>, <c>dft</c>, <c>fft</c>, <c>filter</c>,
/// <c>movingavg</c>, <c>impulse</c>, <c>step</c>, <c>cosine</c>, <c>exponential</c>,
/// <c>powerseries</c>, <c>noise</c>, <c>delay</c>, <c>scale</c>, and the complex accessors
/// <c>re</c>/<c>im</c>/<c>conj</c>) over the language's arbitrary-precision arrays. Values are
/// converted Real ↔ Complex at the boundary; no IEEE floating point is involved.
/// </summary>
/// <remarks>
/// Registration is opt-in: hosts call <see cref="SuiteEngine.RegisterDspBuiltins"/> rather than
/// having the interpreter load DSP unconditionally. Transcendental calls resolve the active
/// precision scope (<see cref="Rl.MaxComputationDecimalPlaces"/>) so <c>setprecision</c> governs
/// them exactly like the other builtins.
/// </remarks>
public static class DspBuiltins
{
    private static readonly Rl s_zero = new Rl("0");

    public static void Register(Interpreter interpreter)
    {
        ArgumentNullException.ThrowIfNull(interpreter);

        interpreter.RegisterBuiltin("conv", new[] { "x", "h" }, args =>
        {
            Require(args, 2, "conv");
            return FromComplexArray(DspMath.Convolve(ToComplexArray(args[0]), ToComplexArray(args[1])));
        });

        interpreter.RegisterBuiltin("dft", new[] { "x" }, args =>
        {
            Require(args, 1, "dft");
            return FromComplexArray(DspMath.Dft(ToComplexArray(args[0])));
        });

        interpreter.RegisterBuiltin("fft", new[] { "x" }, args =>
        {
            Require(args, 1, "fft");
            return FromComplexArray(DspMath.Fft(ToComplexArray(args[0])));
        });

        interpreter.RegisterBuiltin("filter", new[] { "a", "b", "n" }, args =>
        {
            Require(args, 3, "filter");
            long n = ToLong(args[2], "filter");
            return FromComplexArray(DspMath.ImpulseResponse(ToComplexArray(args[0]), ToComplexArray(args[1]), n));
        });

        interpreter.RegisterBuiltin("movingavg", new[] { "x", "w" }, args =>
        {
            Require(args, 2, "movingavg");
            long w = ToLong(args[1], "movingavg");
            var x = ToComplexArray(args[0]);
            return FromComplexArray(Signal.Sample(new DspMath.MovingAverage(w, new Sequence(0, x.Length - 1, x)), 0, x.Length - 1));
        });

        interpreter.RegisterBuiltin("impulse", new[] { "n" }, args =>
        {
            Require(args, 1, "impulse");
            long n = ToLong(args[0], "impulse");
            return FromComplexArray(Signal.Sample(new Impulse(), 0, n - 1));
        });

        interpreter.RegisterBuiltin("step", new[] { "n" }, args =>
        {
            Require(args, 1, "step");
            long n = ToLong(args[0], "step");
            return FromComplexArray(Signal.Sample(new Step(), 0, n - 1));
        });

        interpreter.RegisterBuiltin("cosine", new[] { "freq", "phase", "n" }, args =>
        {
            Require(args, 3, "cosine");
            long n = ToLong(args[2], "cosine");
            var cosine = new Cosine(ToReal(args[0]), ToReal(args[1]));
            return FromComplexArray(Signal.Sample(cosine, 0, n - 1));
        });

        interpreter.RegisterBuiltin("exponential", new[] { "c", "n" }, args =>
        {
            Require(args, 2, "exponential");
            long n = ToLong(args[1], "exponential");
            var exponential = new Exponential(ToComplex(args[0]));
            return FromComplexArray(Signal.Sample(exponential, 0, n - 1));
        });

        interpreter.RegisterBuiltin("powerseries", new[] { "k", "a", "n" }, args =>
        {
            Require(args, 3, "powerseries");
            long n = ToLong(args[2], "powerseries");
            var series = new PowerSeries(ToComplex(args[0]), ToComplex(args[1]));
            return FromComplexArray(Signal.Sample(series, 0, n - 1));
        });

        interpreter.RegisterBuiltin("noise", new[] { "scale", "disp", "seed", "n" }, args =>
        {
            Require(args, 4, "noise");
            int seed = checked((int)ToLong(args[2], "noise"));
            long n = ToLong(args[3], "noise");
            var noise = new Noise(ToReal(args[0]), ToReal(args[1]), seed);
            return FromComplexArray(Signal.Sample(noise, 0, n - 1));
        });

        interpreter.RegisterBuiltin("delay", new[] { "x", "k" }, args =>
        {
            Require(args, 2, "delay");
            long k = ToLong(args[1], "delay");
            var x = ToComplexArray(args[0]);
            return FromComplexArray(Signal.Sample(new Delay(k, new Sequence(0, x.Length - 1, x)), 0, x.Length - 1));
        });

        interpreter.RegisterBuiltin("scale", new[] { "x", "k" }, args =>
        {
            Require(args, 2, "scale");
            var x = ToComplexArray(args[0]);
            var k = ToComplex(args[1]);
            return FromComplexArray(Signal.Sample(new Scalar(k, new Sequence(0, x.Length - 1, x)), 0, x.Length - 1));
        });

        interpreter.RegisterBuiltin("re", new[] { "x" }, args =>
        {
            Require(args, 1, "re");
            return new Value(ToComplex(args[0]).Re);
        });

        interpreter.RegisterBuiltin("im", new[] { "x" }, args =>
        {
            Require(args, 1, "im");
            return new Value(ToComplex(args[0]).Im);
        });

        interpreter.RegisterBuiltin("conj", new[] { "x" }, args =>
        {
            Require(args, 1, "conj");
            return new Value(ToComplex(args[0]).Conjugate);
        });
    }

    private static void Require(IReadOnlyList<Value> args, int expected, string name)
    {
        if (args.Count != expected)
            throw new InvalidOperationException($"{name}() expects {expected} argument(s), but got {args.Count}.");
    }

    private static long ToLong(Value v, string name)
    {
        if (v.Kind == ValueKind.Natural) return long.Parse(v.AsNatural().ToString());
        if (v.Kind == ValueKind.Integer) return long.Parse(v.AsInteger().ToString());
        throw new InvalidOperationException($"{name}() expects a Natural or Integer argument, but got '{v.Kind}'.");
    }

    private static Rl ToReal(Value v) => v.Kind switch
    {
        ValueKind.Natural => new Rl(new Int(v.AsNatural())),
        ValueKind.Integer => new Rl(v.AsInteger()),
        ValueKind.Real => v.AsReal(),
        _ => throw new InvalidOperationException($"Expected a numeric argument, but got '{v.Kind}'."),
    };

    private static Cplx ToComplex(Value v) => v.Kind switch
    {
        ValueKind.Complex => v.AsComplex(),
        ValueKind.Real => new Cplx(v.AsReal(), s_zero),
        ValueKind.Integer => new Cplx(new Rl(v.AsInteger()), s_zero),
        ValueKind.Natural => new Cplx(new Rl(new Int(v.AsNatural())), s_zero),
        _ => throw new InvalidOperationException($"DSP builtins expect numeric/complex elements, but got '{v.Kind}'."),
    };

    private static Cplx[] ToComplexArray(Value v)
    {
        // Reuse the typed-array adapter for the boxed-element walk, then coerce each Value to Complex.
        var elements = TypedArrayAdapter.ToElements(v.AsArrayValue());
        var result = new Cplx[elements.Count];
        for (int i = 0; i < elements.Count; i++)
            result[i] = ToComplex(elements[i]);
        return result;
    }

    private static Value FromComplexArray(Cplx[] values)
    {
        var boxed = new Value[values.Length];
        for (int i = 0; i < values.Length; i++)
            boxed[i] = new Value(values[i]);
        return new Value(boxed);
    }
}
