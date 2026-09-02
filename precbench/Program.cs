using BenchmarkDotNet.Configs;
using BenchmarkDotNet.Running;
using PrecBench;

// precbench — Lovelace.Real 8/16-digit precision benchmark vs float/double.
//
// Throughput half (BenchmarkDotNet). The accuracy half lives in ../precbench.Tests.
// Run a quick smoke test with e.g.:
//   dotnet run -c Release --project precbench -- --filter *Add* --job short

// BDN rebuilds the benchmark in an isolated output dir with shared compilation
// disabled; that cold deterministic rebuild of the Lovelace.Real chain exceeds
// the 120s default build timeout on this machine, so raise it.
var config = ManualConfig.Create(DefaultConfig.Instance);
config.BuildTimeout = TimeSpan.FromMinutes(10);

BenchmarkSwitcher.FromAssembly(typeof(LovelaceP8Benchmarks).Assembly).Run(args, config);
