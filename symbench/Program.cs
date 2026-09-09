using BenchmarkDotNet.Configs;
using BenchmarkDotNet.Running;
using SymBench;

// symbench — the symbolic-numeric pipeline benchmark (Phase 5 of the hardening plan).
//
// What is measured and why: construction/canonicalization of a shared DAG, high-order
// derivatives, Jacobian/Hessian, sparse polynomial arithmetic, a Gröbner basis, the
// compile-and-evaluate pipeline, batch (vectorized MathIR) execution, tree-vs-MathIR
// evaluation, and arbitrary-precision evaluation at 64/256/1024 digits.
//
// Invariant under benchmark: performance is never gained through silent precision
// reduction — every row runs at its declared precision scope.

var config = ManualConfig.Create(DefaultConfig.Instance);
config.BuildTimeout = TimeSpan.FromMinutes(10);

BenchmarkSwitcher.FromAssembly(typeof(ConstructionBenchmarks).Assembly).Run(args, config);
