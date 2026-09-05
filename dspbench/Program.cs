using BenchmarkDotNet.Configs;
using BenchmarkDotNet.Running;
using DspBench;

// dspbench — fixed-width DSP benchmarks (scalar complex ops, convolution, FIR filter,
// moving average, and the script-level e2e twin), over LComplex64/LComplex128 with the
// Complex class at a fixed precision knob for comparison.
//
// Throughput half (BenchmarkDotNet). The correctness half lives in ../Lovelace.Dsp.Tests.
// Run a quick smoke test with e.g.:
//   dotnet run -c Release --project dspbench -- --filter *Convolve* --job short

// BDN rebuilds the benchmark in an isolated output dir with shared compilation disabled; that cold
// deterministic rebuild of the Lovelace.Real chain exceeds the default build timeout on this machine.
var config = ManualConfig.Create(DefaultConfig.Instance);
config.BuildTimeout = TimeSpan.FromMinutes(10);

BenchmarkSwitcher.FromAssembly(typeof(LComplexStruct64Benchmarks).Assembly).Run(args, config);
