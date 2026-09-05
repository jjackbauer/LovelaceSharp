using BenchmarkDotNet.Attributes;
using Lovelace.Complex;
using Lovelace.Suite;
using Cplx = global::Lovelace.Complex.Complex;
using Cplx64 = global::Lovelace.Complex.LComplex64;
using Cplx128 = global::Lovelace.Complex.LComplex128;
using Rl = global::Lovelace.Real.Real;
using Lovelace.Dsp;

namespace DspBench;

// ---------------------------------------------------------------------------
// dspbench — fixed-width DSP benchmarks.
//
// The recommended workload. Fixed-width complex arithmetic runs on LComplex64
// (a pair of LReal64 — 19 significant digits) and LComplex128 (a pair of LReal128 —
// 38 significant digits), with the Complex class included at a fixed precision
// knob (18 ≤ 37, so its operators silently dispatch to the structs and promote on
// overflow). The arbitrary-precision ladder was dropped: fixed width is the path
// a user should actually run, and the suite completes in minutes instead of hours.
//
// Axes:
//   * LComplexStruct*  = the fixed structs, called raw.
//   * ComplexClass*    = the class at a fixed knob (struct fast path, silent promotion).
//   * Fixed* workloads = whole-array DSP over the structs (convolve, FIR filter,
//                        moving average) — inputs sized to fit the width, so the
//                        fixed path never promotes and never rounds.
//   * DspScript*       = the same workload end to end through the language
//                        (setprecision drives the knob).
// ---------------------------------------------------------------------------

public abstract class FixedWorkloadBase
{
    /// <summary>Precision knob for the class rows (≤ 37 → the struct fast path is active).</summary>
    protected const long ClassKnob = 18;

    protected long SavedMax;
    protected long SavedDisplay;

    /// <summary>Pins the class precision knob; call at the top of each [GlobalSetup].</summary>
    protected void PinPrecision()
    {
        SavedMax = Rl.MaxComputationDecimalPlaces;
        SavedDisplay = Rl.DisplayDecimalPlaces;
        Rl.MaxComputationDecimalPlaces = ClassKnob;
        Rl.DisplayDecimalPlaces = ClassKnob;
    }

    [GlobalCleanup]
    public void RestorePrecision()
    {
        Rl.MaxComputationDecimalPlaces = SavedMax;
        Rl.DisplayDecimalPlaces = SavedDisplay;
    }

    /// <summary>Builds the 256-tap FIR kernel (weights i%17 over 256 = 2⁸ → terminating decimals).</summary>
    protected static (Cplx64[] Fixed64, Cplx128[] Fixed128, Cplx[] ClassForm) Kernel256()
    {
        var k64 = new Cplx64[256];
        var k128 = new Cplx128[256];
        var kc = new Cplx[256];
        for (int i = 0; i < 256; i++)
        {
            kc[i] = new Cplx(new Rl((i % 17).ToString()) / new Rl("256"), new Rl("0"));
            Cplx64.TryFromComplex(kc[i], out k64[i]);
            Cplx128.TryFromComplex(kc[i], out k128[i]);
        }
        return (k64, k128, kc);
    }

    /// <summary>Builds a ramp-ish complex signal (small integers — fit every width).</summary>
    protected static (Cplx64[] Fixed64, Cplx128[] Fixed128, Cplx[] ClassForm) Ramp(int n)
    {
        var r64 = new Cplx64[n];
        var r128 = new Cplx128[n];
        var rc = new Cplx[n];
        for (int i = 0; i < n; i++)
        {
            rc[i] = new Cplx(new Rl((i % 100).ToString()), new Rl((i % 97).ToString()));
            Cplx64.TryFromComplex(rc[i], out r64[i]);
            Cplx128.TryFromComplex(rc[i], out r128[i]);
        }
        return (r64, r128, rc);
    }
}

// ---------------------------------------------------------------------------
// Scalar ops — the free class at a fixed knob vs the fixed structs (precbench
// structure). Add/Sub/Mul use wide operands sized to the target width; Div uses
// its own pair (b = 1+2i, |b|² = 5) so the quotient terminates within every width.
// ---------------------------------------------------------------------------

public abstract class ComplexClassBenchmarks
{
    protected abstract long FractionalDigits { get; }
    protected abstract string OperandA { get; }
    protected abstract string OperandB { get; }

    private Cplx _a = null!;
    private Cplx _b = null!;
    private Cplx _aDiv = null!;
    private Cplx _bDiv = null!;
    private long _savedMax;
    private long _savedDisplay;

    [GlobalSetup]
    public void GlobalSetup()
    {
        _savedMax = Rl.MaxComputationDecimalPlaces;
        _savedDisplay = Rl.DisplayDecimalPlaces;
        Rl.MaxComputationDecimalPlaces = FractionalDigits;
        Rl.DisplayDecimalPlaces = FractionalDigits;
        _a = Cplx.Parse(OperandA);
        _b = Cplx.Parse(OperandB);
        _aDiv = Cplx.Parse(DivOperandA);
        _bDiv = Cplx.Parse(DivOperandB);
    }

    [GlobalCleanup]
    public void GlobalCleanup()
    {
        Rl.MaxComputationDecimalPlaces = _savedMax;
        Rl.DisplayDecimalPlaces = _savedDisplay;
    }

    [Benchmark] public Cplx Add() => _a + _b;
    [Benchmark] public Cplx Sub() => _a - _b;
    [Benchmark] public Cplx Mul() => _a * _b;
    [Benchmark] public Cplx Div() => _aDiv / _bDiv;

    // Shared Div pair: (2.5+1.25i)/(1+2i) = 1−0.75i, terminating in every width.
    protected static string DivOperandA => "2.5+1.25i";
    protected static string DivOperandB => "1+2i";
}

// Knob = 18 fractional digits; 8-significant-digit components (fit LComplexStruct64 incl. |z|²).
[MemoryDiagnoser]
public class ComplexClassP18Benchmarks : ComplexClassBenchmarks
{
    protected override long FractionalDigits => 18;
    protected override string OperandA => "2.3456789+1.2345678i";
    protected override string OperandB => "1.2345678+2.3456789i";
}

// Knob = 37 fractional digits; 18-significant-digit components (fit LComplexStruct128 incl. |z|²).
[MemoryDiagnoser]
public class ComplexClassP37Benchmarks : ComplexClassBenchmarks
{
    protected override long FractionalDigits => 37;
    protected override string OperandA => "2.34567890123456789+1.23456789012345678i";
    protected override string OperandB => "1.23456789012345678+2.34567890123456789i";
}

// Fixed-width struct baselines — same Add/Sub/Mul operands as their matching class
// subclass, and the shared terminating Div pair.
[MemoryDiagnoser]
public class LComplexStruct64Benchmarks
{
    private LComplex64 _a;
    private LComplex64 _b;
    private LComplex64 _aDiv;
    private LComplex64 _bDiv;

    [GlobalSetup]
    public void GlobalSetup()
    {
        _a = LComplex64.Parse("2.3456789+1.2345678i");
        _b = LComplex64.Parse("1.2345678+2.3456789i");
        _aDiv = LComplex64.Parse("2.5+1.25i");
        _bDiv = LComplex64.Parse("1+2i");
    }

    [Benchmark] public LComplex64 Add() => _a + _b;
    [Benchmark] public LComplex64 Sub() => _a - _b;
    [Benchmark] public LComplex64 Mul() => _a * _b;
    [Benchmark] public LComplex64 Div() => _aDiv / _bDiv;
}

[MemoryDiagnoser]
public class LComplexStruct128Benchmarks
{
    private LComplex128 _a;
    private LComplex128 _b;
    private LComplex128 _aDiv;
    private LComplex128 _bDiv;

    [GlobalSetup]
    public void GlobalSetup()
    {
        _a = LComplex128.Parse("2.34567890123456789+1.23456789012345678i");
        _b = LComplex128.Parse("1.23456789012345678+2.34567890123456789i");
        _aDiv = LComplex128.Parse("2.5+1.25i");
        _bDiv = LComplex128.Parse("1+2i");
    }

    [Benchmark] public LComplex128 Add() => _a + _b;
    [Benchmark] public LComplex128 Sub() => _a - _b;
    [Benchmark] public LComplex128 Mul() => _a * _b;
    [Benchmark] public LComplex128 Div() => _aDiv / _bDiv;
}

// ---------------------------------------------------------------------------
// Whole-array workloads over the fixed structs (the recommended path), with the
// class row at the fixed knob for comparison. Inputs are sized to fit the width,
// so the fixed path never promotes and never rounds.
// ---------------------------------------------------------------------------

[MemoryDiagnoser]
public class FixedConvolveBenchmarks : FixedWorkloadBase
{
    [Params(10_000, 100_000)]
    public int N = 0;

    private Cplx64[] _x64 = null!;
    private Cplx64[] _h64 = null!;
    private Cplx128[] _x128 = null!;
    private Cplx128[] _h128 = null!;
    private Cplx[] _xc = null!;
    private Cplx[] _hc = null!;

    [GlobalSetup]
    public void Setup()
    {
        PinPrecision();
        var x = Ramp(N);
        var h = Kernel256();
        (_x64, _x128, _xc) = x;
        (_h64, _h128, _hc) = h;
    }

    [Benchmark] public Cplx64[] Struct64() => FixedDsp.Convolve(_x64, _h64);
    [Benchmark] public Cplx128[] Struct128() => FixedDsp.Convolve(_x128, _h128);
    [Benchmark] public Cplx[] Class() => DspMath.Convolve(_xc, _hc);
}

[MemoryDiagnoser]
public class FixedFilterBenchmarks : FixedWorkloadBase
{
    [Params(10_000, 100_000)]
    public long N = 0;

    private Cplx64[] _a64 = null!;
    private Cplx64[] _b64 = null!;
    private Cplx128[] _a128 = null!;
    private Cplx128[] _b128 = null!;
    private Cplx[] _ac = null!;
    private Cplx[] _bc = null!;

    [GlobalSetup]
    public void Setup()
    {
        PinPrecision();
        // Pure FIR (a = [1]) — exact and width-bounded for any tap set.
        _a64 = new[] { Cplx64.One };
        _a128 = new[] { Cplx128.One };
        _ac = new[] { Cplx.One };
        var h = Kernel256();
        (_b64, _b128, _bc) = h;
    }

    [Benchmark] public Cplx64[] Struct64() => FixedDsp.ImpulseResponse(_a64, _b64, N);
    [Benchmark] public Cplx128[] Struct128() => FixedDsp.ImpulseResponse(_a128, _b128, N);
    [Benchmark] public Cplx[] Class() => DspMath.ImpulseResponse(_ac, _bc, N);
}

[MemoryDiagnoser]
public class FixedMovingAverageBenchmarks : FixedWorkloadBase
{
    private const long Window = 16;   // power of two → the window division always terminates

    [Params(10_000, 100_000)]
    public int N = 0;

    private Cplx64[] _x64 = null!;
    private Cplx128[] _x128 = null!;
    private Cplx[] _xc = null!;

    [GlobalSetup]
    public void Setup()
    {
        PinPrecision();
        var x = Ramp(N);
        (_x64, _x128, _xc) = x;
    }

    [Benchmark] public Cplx64[] Struct64() => FixedDsp.MovingAverage(_x64, Window);
    [Benchmark] public Cplx128[] Struct128() => FixedDsp.MovingAverage(_x128, Window);
    [Benchmark] public Cplx[] Class() =>
        Signal.Sample(new DspMath.MovingAverage(Window, new Sequence(0, _xc.Length - 1, _xc)), 0, _xc.Length - 1);
}

// ---------------------------------------------------------------------------
// The same workload end to end through the language: setprecision drives the knob
// (18/37 → the class builtins run the struct fast path with silent promotion).
// ---------------------------------------------------------------------------

[MemoryDiagnoser]
public class DspScriptBenchmarks
{
    [Params(18, 37)]
    public int Precision = 0;

    private SuiteEngine _engine = null!;
    private string _script = null!;

    [GlobalSetup]
    public void GlobalSetup()
    {
        _engine = new SuiteEngine();
        _engine.RegisterDspBuiltins();
        _script = $"setprecision({Precision}); conv(1..512, 1..128)";
    }

    [Benchmark]
    public Value Convolve() => _engine.Evaluate(_script);
}
