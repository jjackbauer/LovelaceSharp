using System.Globalization;
using BenchmarkDotNet.Attributes;
using Lovelace.Real;

namespace PrecBench;

// ---------------------------------------------------------------------------
// Lovelace.Real benchmarks. P8 = 8 significant digits (7 fractional places),
// P16 = 16 significant digits (15 fractional places), on [1,10)-normalized
// operands so fractional places == significant digits minus one integer digit.
// ---------------------------------------------------------------------------

public abstract class LovelaceBenchmarks
{
    protected abstract long FractionalDigits { get; }
    protected abstract string OperandA { get; }
    protected abstract string OperandB { get; }

    private Real _a = null!;
    private Real _b = null!;
    private long _savedMax;
    private long _savedDisp;

    [GlobalSetup]
    public void GlobalSetup()
    {
        _savedMax = Real.MaxComputationDecimalPlaces;
        _savedDisp = Real.DisplayDecimalPlaces;
        Real.MaxComputationDecimalPlaces = FractionalDigits;
        Real.DisplayDecimalPlaces = FractionalDigits;
        _a = new Real(OperandA);
        _b = new Real(OperandB);
    }

    [GlobalCleanup]
    public void GlobalCleanup()
    {
        Real.MaxComputationDecimalPlaces = _savedMax;
        Real.DisplayDecimalPlaces = _savedDisp;
    }

    [Benchmark] public Real Add() => _a + _b;
    [Benchmark] public Real Sub() => _a - _b;
    [Benchmark] public Real Mul() => _a * _b;
    [Benchmark] public Real Div() => _a / _b;
    [Benchmark] public Real Sqrt() => Real.Sqrt(_a);
}

[MemoryDiagnoser]
public class LovelaceP8Benchmarks : LovelaceBenchmarks
{
    protected override long FractionalDigits => 7;
    protected override string OperandA => "2.3456789";
    protected override string OperandB => "1.2345678";
}

[MemoryDiagnoser]
public class LovelaceP16Benchmarks : LovelaceBenchmarks
{
    protected override long FractionalDigits => 15;
    protected override string OperandA => "2.345678901234567";
    protected override string OperandB => "1.234567890123456";
}

// ---------------------------------------------------------------------------
// Native IEEE-754 baselines. Very fast ops are batched via OperationsPerInvoke
// with a loop-carried dependency so the JIT cannot hoist the operation or
// dead-code-eliminate it; the returned value is consumed by BenchmarkDotNet.
// ---------------------------------------------------------------------------

[MemoryDiagnoser]
public class FloatBenchmarks
{
    private float _a;
    private float _b;

    [GlobalSetup]
    public void GlobalSetup()
    {
        _a = float.Parse("2.3456789", CultureInfo.InvariantCulture);
        _b = float.Parse("1.2345678", CultureInfo.InvariantCulture);
    }

    [Benchmark(OperationsPerInvoke = 16)]
    public float Add() { float acc = 0f; for (int i = 0; i < 16; i++) acc = acc + _a; return acc; }

    [Benchmark(OperationsPerInvoke = 16)]
    public float Sub() { float acc = 0f; for (int i = 0; i < 16; i++) acc = acc - _b; return acc; }

    [Benchmark(OperationsPerInvoke = 16)]
    public float Mul() { float acc = 1f; for (int i = 0; i < 16; i++) acc = acc * _a; return acc; }

    [Benchmark(OperationsPerInvoke = 16)]
    public float Div() { float acc = 10f; for (int i = 0; i < 16; i++) acc = acc / _a; return acc; }

    [Benchmark(OperationsPerInvoke = 16)]
    public float Sqrt() { float acc = 2.3456789f; for (int i = 0; i < 16; i++) acc = MathF.Sqrt(acc); return acc; }
}

[MemoryDiagnoser]
public class DoubleBenchmarks
{
    private double _a;
    private double _b;

    [GlobalSetup]
    public void GlobalSetup()
    {
        _a = double.Parse("2.345678901234567", CultureInfo.InvariantCulture);
        _b = double.Parse("1.234567890123456", CultureInfo.InvariantCulture);
    }

    [Benchmark(OperationsPerInvoke = 16)]
    public double Add() { double acc = 0.0; for (int i = 0; i < 16; i++) acc = acc + _a; return acc; }

    [Benchmark(OperationsPerInvoke = 16)]
    public double Sub() { double acc = 0.0; for (int i = 0; i < 16; i++) acc = acc - _b; return acc; }

    [Benchmark(OperationsPerInvoke = 16)]
    public double Mul() { double acc = 1.0; for (int i = 0; i < 16; i++) acc = acc * _a; return acc; }

    [Benchmark(OperationsPerInvoke = 16)]
    public double Div() { double acc = 10.0; for (int i = 0; i < 16; i++) acc = acc / _a; return acc; }

    [Benchmark(OperationsPerInvoke = 16)]
    public double Sqrt() { double acc = 2.345678901234567; for (int i = 0; i < 16; i++) acc = Math.Sqrt(acc); return acc; }
}
