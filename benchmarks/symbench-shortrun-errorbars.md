# symbench ShortRun with error bars us Cycle 4 (us4.4)

| Item | Value |
|---|---|
| Date | 2026-09-11 13:07 local |
| HEAD | `32ebefd` |
| Command | `dotnet run -c Release --project symbench -- --filter * --job short` |
| Job | ShortRun (3 measured iterations, 1 launch, 3 warmups) |
| Machine | AMD Ryzen 9 5900X, 12C/24T us **NOT idle** (other builds, tests and three audit agents were in flight) |

> **Why this file exists.** `benchmarks/symbench-baseline.md` recorded a ShortRun sweep but its
> summary table kept only Mean and Allocated, dropping the `Error`/`StdDev` columns that
> BenchmarkDotNet had measured. Its own note warns that three iterations give a 99.9% interval
> "routinely as wide as the mean itself". Those columns are preserved below, so every number
> carries its own error bar and can be read as a range rather than a point.

> **Caveat, stated rather than buried.** This machine was not idle, so these intervals bound
> run-to-run noise *including* interference, and `Eval1024` (151 s) is dominated by its own
> allocation churn. No improvement claim should be quoted from this file; it is a regression
> *direction* check with quantified uncertainty.

## SymBench.CalculusBenchmarks

| Method          | Mean      | Error     | StdDev    | Gen0    | Allocated |
|---------------- |----------:|----------:|----------:|--------:|----------:|
| Diff_TenthOrder |  4.556 us |  7.581 us | 0.4156 us |  0.9460 |  15.47 KB |
| Jacobian_4x4    | 64.137 us |  8.862 us | 0.4858 us | 13.3057 | 218.97 KB |
| Hessian_4Var    | 77.079 us | 20.098 us | 1.1016 us | 16.6016 | 272.52 KB |

## SymBench.CompilationBenchmarks

| Method                         | Mean         | Error        | StdDev      | Gen0         | Gen1      | Gen2      | Allocated   |
|------------------------------- |-------------:|-------------:|------------:|-------------:|----------:|----------:|------------:|
| Kernel_EvaluateScalar          |     2.708 ms |     2.965 ms |   0.1625 ms |     863.2813 |   27.3438 |         - |    13.82 MB |
| Kernel_EvaluateBatch_1000Lanes | 3,721.074 ms | 3,486.079 ms | 191.0838 ms | 1094000.0000 | 3000.0000 | 1000.0000 | 17468.09 MB |
| TreeEvaluator_Direct           |     2.605 ms |     3.737 ms |   0.2048 ms |     867.1875 |   19.5313 |         - |    13.83 MB |

## SymBench.ConstructionBenchmarks

| Method                   | Mean     | Error    | StdDev   | Gen0    | Gen1   | Allocated |
|------------------------- |---------:|---------:|---------:|--------:|-------:|----------:|
| CanonicalParse_SharedDAG | 271.7 us | 216.6 us | 11.87 us | 52.2461 | 6.3477 | 854.19 KB |

## SymBench.ConstructionCanonicalizationBenchmarks

| Method                             | Mean     | Error    | StdDev    | Gen0     | Gen1    | Gen2    | Allocated |
|----------------------------------- |---------:|---------:|----------:|---------:|--------:|--------:|----------:|
| Construct_SharedDAG                | 2.901 ms | 3.074 ms | 0.1685 ms | 789.0625 | 82.0313 | 39.0625 |  12.73 MB |
| ConstructAndCanonicalize_SharedDAG | 3.052 ms | 7.141 ms | 0.3914 ms | 792.9688 | 82.0313 | 39.0625 |  12.78 MB |

## SymBench.HelpCatalogBenchmarks

| Method                   | Mean      | Error     | StdDev    | Gen0   | Gen1   | Allocated |
|------------------------- |----------:|----------:|----------:|-------:|-------:|----------:|
| Help_Overview            |  4.893 us | 8.5003 us | 0.4659 us | 0.7324 |      - |  12.07 KB |
| Help_Funcs_AllCategories | 10.303 us | 0.4883 us | 0.0268 us | 3.1738 | 0.1068 |  51.97 KB |

## SymBench.MathIrEvaluationBenchmarks

| Method                          | Mean         | Error         | StdDev      | Gen0         | Gen1      | Gen2      | Allocated   |
|-------------------------------- |-------------:|--------------:|------------:|-------------:|----------:|----------:|------------:|
| MathIR_Evaluate_Scalar          |     2.553 ms |     0.5547 ms |   0.0304 ms |     863.2813 |   27.3438 |         - |    13.82 MB |
| MathIR_Evaluate_Batch_1000Lanes | 3,543.146 ms | 1,861.1764 ms | 102.0174 ms | 1094000.0000 | 3000.0000 | 1000.0000 | 17468.09 MB |

## SymBench.PolynomialBenchmarks

| Method                           | Mean     | Error     | StdDev   | Gen0    | Gen1   | Allocated |
|--------------------------------- |---------:|----------:|---------:|--------:|-------:|----------:|
| Multiply_SparseBivariateDegree20 | 320.3 us | 182.77 us | 10.02 us | 58.1055 | 2.9297 | 949.23 KB |
| Groebner_Cyclic3_GrLex           | 132.4 us |  55.73 us |  3.05 us | 31.0059 | 0.4883 | 508.58 KB |

## SymBench.PrecisionBenchmarks

| Method   | Mean          | Error       | StdDev     | Gen0          | Gen1          | Gen2          | Allocated     |
|--------- |--------------:|------------:|-----------:|--------------:|--------------:|--------------:|--------------:|
| Eval64   |      10.06 ms |    20.62 ms |   1.130 ms |     2578.1250 |      140.6250 |             - |      41.36 MB |
| Eval256  |     731.86 ms |    88.91 ms |   4.873 ms |   169000.0000 |    10000.0000 |     1000.0000 |    2616.21 MB |
| Eval1024 | 151,240.63 ms | 3,519.90 ms | 192.937 ms | 76995000.0000 | 23876000.0000 | 10810000.0000 | 1106141.56 MB |

## SymBench.PrintingBenchmarks

| Method                                   | Mean     | Error    | StdDev   | Gen0   | Allocated |
|----------------------------------------- |---------:|---------:|---------:|-------:|----------:|
| PrettyPrint_TranscendentalComposition    | 924.1 ns | 554.2 ns | 30.38 ns | 0.2441 |   3.99 KB |
| CanonicalPrint_TranscendentalComposition | 636.0 ns | 264.0 ns | 14.47 ns | 0.1898 |   3.11 KB |

## SymBench.SimplificationBenchmarks

| Method                 | Mean     | Error    | StdDev   | Gen0    | Gen1   | Allocated |
|----------------------- |---------:|---------:|---------:|--------:|-------:|----------:|
| Simplify_Safe          | 54.95 us | 53.93 us | 2.956 us | 12.2681 | 0.1831 | 201.09 KB |
| Simplify_Full_TraceOff | 61.65 us | 42.98 us | 2.356 us | 12.0850 | 0.1221 | 199.21 KB |
| Simplify_Full_TraceOn  | 60.01 us | 89.16 us | 4.887 us | 12.1460 | 0.1831 | 199.24 KB |

Run 2 of the planned 2 repeats was still in flight when this artifact was written; the raw
transcript of the first sweep is `benchmarks/raw-shortrun-c4-run1.txt` and the per-class
reports are under `benchmarks/bdn-reports-shortrun-c4/run1/`.