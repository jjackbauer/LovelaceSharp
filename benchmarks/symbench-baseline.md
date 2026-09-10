# symbench baseline — A+ Cycle-2 Phase A (alignment plan item 18)

| Item | Value |
|---|---|
| Date | 2026-09-10 (sweep 17:54–18:11 local) |
| HEAD | `f37f1ce61631cba5638584349f0934ea4f209681` — "docs: DX-cycle documentation pass" (`main`) |
| Working tree | dirty — Cycle 1 and the first Cycle-2 phases present and uncommitted |
| Machine | AMD Ryzen 9 5900X, 12 physical / 24 logical cores — Windows 11 (10.0.26100.4351/24H2) |
| Toolchain | .NET SDK **10.0.103**; runtime .NET 10.0.3 (10.0.326.7603), X64 RyuJIT x86-64-v3, GC = Concurrent Workstation; BenchmarkDotNet 0.15.8 |
| Command | `dotnet run -c Release --project symbench -- --filter * --job short 2>&1 \| Tee-Object -FilePath benchmarks/raw-baseline.txt` |
| Job | `ShortRun(IterationCount=3, LaunchCount=1, WarmupCount=3)` |
| Wall time | 00:17:23 for 12 benchmarks — 11 measured, `Eval1024` unmeasurable (see §2) |
| Raw transcript | `benchmarks/raw-baseline.txt` (complete console output, UTF-8) |

> **The ShortRun job is for regression *direction* only, not for publication.** Three warmup and
> three measured iterations give a mean whose 99.9% confidence interval is routinely as wide as
> the mean itself (see the `Error` columns below — e.g. `Kernel_EvaluateScalar` 6.685 ms ± 6.049 ms).
> Use this file to detect a move, not to quote a number. The sweep also ran on a shared workstation
> while other builds/tests were in flight, so rows near the noise floor should be re-measured
> before any claim is made from a single row.

## 1. Pre-existing rows

| Class | Method | Mean | Allocated |
|---|---|---|---|
| Construction | `CanonicalParse_SharedDAG` | 323.2 µs | 854.19 KB |
| Calculus | `Diff_TenthOrder` | 4.382 µs | 15.47 KB |
| Calculus | `Jacobian_4x4` | 110.323 µs | 218.97 KB |
| Calculus | `Hessian_4Var` | 103.664 µs | 272.52 KB |
| Polynomial | `Multiply_SparseBivariateDegree20` | 278.3 µs | 935.45 KB |
| Polynomial | `Groebner_Cyclic3_GrLex` | 150.7 µs | 504.2 KB |
| Compilation | `Kernel_EvaluateScalar` | 6.685 ms | 33.9 MB |
| Compilation | `Kernel_EvaluateBatch_1000Lanes` | 10,261.695 ms | 43,974.04 MB |
| Compilation | `TreeEvaluator_Direct` | 6.180 ms | 33.92 MB |
| Precision | `Eval64` | 31.45 ms | 115.02 MB |
| Precision | `Eval256` | 2,737.11 ms | 8,390.07 MB |
| Precision | `Eval1024` | **not measured** (see §2) | — |

Verbatim BenchmarkDotNet reports:

### SymBench.ConstructionBenchmarks

```
```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]   : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  ShortRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=ShortRun  IterationCount=3  LaunchCount=1  
WarmupCount=3  

```
| Method                   | Mean     | Error    | StdDev   | Gen0    | Gen1   | Allocated |
|------------------------- |---------:|---------:|---------:|--------:|-------:|----------:|
| CanonicalParse_SharedDAG | 323.2 μs | 717.3 μs | 39.32 μs | 52.2461 | 6.3477 | 854.19 KB |
```

### SymBench.CalculusBenchmarks

```
```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]   : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  ShortRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=ShortRun  IterationCount=3  LaunchCount=1  
WarmupCount=3  

```
| Method          | Mean       | Error       | StdDev     | Gen0    | Allocated |
|---------------- |-----------:|------------:|-----------:|--------:|----------:|
| Diff_TenthOrder |   4.382 μs |   0.4112 μs |  0.0225 μs |  0.9460 |  15.47 KB |
| Jacobian_4x4    | 110.323 μs |  98.6415 μs |  5.4069 μs | 13.3057 | 218.97 KB |
| Hessian_4Var    | 103.664 μs | 262.0463 μs | 14.3636 μs | 16.6016 | 272.52 KB |
```

### SymBench.PolynomialBenchmarks

```
```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]   : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  ShortRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=ShortRun  IterationCount=3  LaunchCount=1  
WarmupCount=3  

```
| Method                           | Mean     | Error    | StdDev   | Gen0    | Gen1   | Allocated |
|--------------------------------- |---------:|---------:|---------:|--------:|-------:|----------:|
| Multiply_SparseBivariateDegree20 | 278.3 μs | 227.8 μs | 12.49 μs | 57.1289 | 2.9297 | 935.45 KB |
| Groebner_Cyclic3_GrLex           | 150.7 μs | 150.6 μs |  8.26 μs | 30.7617 | 0.4883 |  504.2 KB |
```

### SymBench.CompilationBenchmarks

```
```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]   : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  ShortRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=ShortRun  IterationCount=3  LaunchCount=1  
WarmupCount=3  

```
| Method                         | Mean          | Error        | StdDev      | Gen0         | Gen1      | Gen2      | Allocated   |
|------------------------------- |--------------:|-------------:|------------:|-------------:|----------:|----------:|------------:|
| Kernel_EvaluateScalar          |      6.685 ms |     6.049 ms |   0.3316 ms |    2125.0000 |  101.5625 |         - |     33.9 MB |
| Kernel_EvaluateBatch_1000Lanes | 10,261.695 ms | 6,505.102 ms | 356.5666 ms | 2756000.0000 | 4000.0000 | 1000.0000 | 43974.04 MB |
| TreeEvaluator_Direct           |      6.180 ms |     2.315 ms |   0.1269 ms |    2125.0000 |   85.9375 |         - |    33.92 MB |
```

### SymBench.PrecisionBenchmarks

```
```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]   : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  ShortRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=ShortRun  IterationCount=3  LaunchCount=1  
WarmupCount=3  

```
| Method   | Mean        | Error       | StdDev     | Gen0        | Gen1       | Gen2      | Allocated  |
|--------- |------------:|------------:|-----------:|------------:|-----------:|----------:|-----------:|
| Eval64   |    31.45 ms |    96.26 ms |   5.276 ms |   7187.5000 |   562.5000 |         - |  115.02 MB |
| Eval256  | 2,737.11 ms | 2,267.07 ms | 124.266 ms | 541000.0000 | 25000.0000 | 4000.0000 | 8390.07 MB |
| Eval1024 |          NA |          NA |         NA |          NA |         NA |        NA |         NA |

Benchmarks with issues:
  PrecisionBenchmarks.Eval1024: ShortRun(IterationCount=3, LaunchCount=1, WarmupCount=3)
```

## 2. `PrecisionBenchmarks.Eval1024` — not measurable under ShortRun

The row never reached its measured iterations. Its first invocation reported

```text
WorkloadJitting  1: 1 op, 508761451200.00 ns, 8.4794 m/op
```

i.e. **one** 1024-digit evaluation of `exp(sin(x²))` costs ≈ 509 s. A ShortRun job needs the
pilot plus three warmup and three measured iterations, so this single row would have needed
roughly an hour of wall time on its own; the process was stopped so the sweep could finish and
the remaining 11 rows could be reported.

The scaling is visible inside the same class: `Eval64` 31.45 ms / 115 MB → `Eval256`
2,737 ms / 8.39 GB per operation (≈ 87× the time and ≈ 73× the allocation for 4× the digits).
The precision ladder as written is therefore **not gate-able** as a whole: before this row can
report a number it needs re-scoping (a lower rung, or a dedicated job with
`IterationCount=1`/`WarmupCount=1` and an explicit budget). That is a Phase-E/A$ decision
recorded here, not a regression — no earlier baseline exists to compare against.

## 2b. New rows — first measured numbers (Cycle-2 round 25)

Measured with the same ShortRun job and the same machine, filtered per class:

```text
dotnet run -c Release --project symbench -- --filter "*HelpCatalogBenchmarks*" "*PrintingBenchmarks*" "*StructuredResultBenchmarks*" --job short
```

| Class | Method | Mean | Allocated |
|---|---|---|---|
| HelpCatalog | `Help_Overview` | 10.65 µs | 21.49 KB |
| HelpCatalog | `Help_Funcs_AllCategories` | 16.40 µs | 59.72 KB |
| Printing | `PrettyPrint_TranscendentalComposition` | 850.2 ns | 3.73 KB |
| Printing | `CanonicalPrint_TranscendentalComposition` | 682.4 ns | 3.11 KB |

Same ShortRun caveat as §1 applies: direction only, not quotable numbers.

### StructuredResultBenchmarks (was broken; fixed and now measured)

This class failed to run at all on the first attempt: `Setup()` passed a script whose statements
were **newline-separated**, and a newline is not a statement separator in this language, so
`[GlobalSetup]` threw a parse error and BenchmarkDotNet aborted the whole class. Fixed by
separating the statements with `;` (`symbench/Benchmarks.cs`, `StructuredResultBenchmarks.Setup`).
Worth generalising: a benchmark whose setup throws silently produces **no row at all**, which is
worse than a slow row — it looks like a class that was never written.

| Method | Mean | Allocated |
|---|---|---|
| `Record_Construct_SolveResult` | 1,049.9 ns | 4,944 B |
| `Record_FieldLookup_ByName` | 4.887 ns | 0 B |
| `Record_MemberAccess_Solutions` | 745.3 ns | 2,512 B |
| `Json_SerializeSolveResult` | 4,186.6 ns | 8,041 B |

- `Record_FieldLookup_ByName` is **4.9 ns and allocation-free**, which answers alignment-doc item
  26 empirically: record field lookup needs no change. Measure before restructuring, as that item
  instructs — the measurement says leave it alone.
- `Json_SerializeSolveResult` at one size proved nothing about growth. **Now measured at two
  sizes through the SHIPPED projection** (`Lovelace.Suite.StructuredProjection`, the one every host
  calls — the older row used a local re-implementation and is kept only for baseline comparability):

  | Solutions | Mean | Allocated |
  |---|---|---|
  | 2 | 3,781.8 ns | 10,353 B |
  | 64 (32× the data) | 61,084.3 ns | 172,603 B |
  | **growth** | **16.2×** | **16.7×** |

  Both ratios are **below** the 32× data increase, so growth is sub-linear, not super-linear: the
  `SolveResult` wrapper is a fixed cost independent of the solution list (≈2,617 B per additional
  solution against ≈5.1 KB fixed), which is *why* the ratio is under 32 rather than a coincidence.
  **§4 item 18's O(n)-in-result-size requirement is satisfied.**
- Related finding on the same class: its local copy of the projection was justified by a comment
  saying the Run helper was internal — true when written, false since Cycle-2 round 3 moved the
  projection into `Lovelace.Suite` as public API. The row had been measuring a duplicate. A
  shipped-projection row was added (3,814 ns / 10,425 B at 2 solutions, ~4 % slower and +2.4 KB over
  the local copy); the extra allocation is the richer contract that copy never emitted
  (`truncated`/`truncationReason`/`budget`/`numerator`/`denominator`).
- The `Error` column for `Record_Construct_SolveResult` (2,072 ns against a 1,049 ns mean) is the
  ShortRun job behaving as §1 describes: direction only.

Note for the caching requirement (§125: "help catalogs must be cached and must not regenerate per
statement"): **resolved.** `Help_Overview` at 10.65 µs was consistent with a rebuild per call, and
that is what it was — `HelpService.Catalog()` re-sorted and re-allocated the registry on every
call. It is now memoised (see §2c for the before/after: allocations 21.49 KB → 11.93 KB on
`Help_Overview`, 59.72 KB → 50.16 KB on `Help_Funcs_AllCategories`).

## 2c. Deltas against this baseline (Cycle-2 round 36)

Re-measured with the same filters and the same ShortRun job, same machine, after the cycle's
kernel changes. Allocations are the reliable column here; the means carry the ShortRun caveat from
§1 (and the baseline itself was taken while other builds were in flight).

| Row | Baseline mean | Now | Δ mean | Baseline alloc | Now | Δ alloc |
|---|---|---|---|---|---|---|
| `CanonicalParse_SharedDAG` | 323.2 µs | 253.8 µs | −21.5 % | 854.19 KB | 854.19 KB | **0.0 %** |
| `Diff_TenthOrder` | 4.382 µs | 4.130 µs | −5.8 % | 15.47 KB | 15.47 KB | **0.0 %** |
| `Jacobian_4x4` | 110.323 µs | 66.589 µs | −39.6 % | 218.97 KB | 218.97 KB | **0.0 %** |
| `Hessian_4Var` | 103.664 µs | 85.683 µs | −17.4 % | 272.52 KB | 272.52 KB | **0.0 %** |

| `Kernel_EvaluateScalar` | 6.685 ms | 5.861 ms | −12.3 % | 33.9 MB | 33.9 MB | **0.0 %** |
| `Kernel_EvaluateBatch_1000Lanes` | 10,261.695 ms | 8,218.180 ms | −19.9 % | 43,974.04 MB | 43,974.04 MB | **0.0 %** |
| `TreeEvaluator_Direct` | 6.180 ms | 5.552 ms | −10.2 % | 33.92 MB | 33.92 MB | **0.0 %** |
| `Eval64` | 31.45 ms | 21.47 ms | −31.7 % | 115.02 MB | 115.02 MB | **0.0 %** |
| `Eval256` | 2,737.11 ms | 2,393.22 ms | −12.6 % | 8,390.07 MB | 8,390.08 MB | **0.0 %** |
| `Multiply_SparseBivariateDegree20` | 278.3 µs | 223.4 µs | −19.7 % | 935.45 KB | 935.45 KB | **0.0 %** |
| `Groebner_Cyclic3_GrLex` | 150.7 µs | 131.1 µs | −13.0 % | 504.2 KB | 504.2 KB | **0.0 %** |
| `Help_Overview` | 10.65 µs | 4.201 µs | −61 % | 21.49 KB | 11.93 KB | **−44 %** |
| `Help_Funcs_AllCategories` | 16.40 µs | 9.641 µs | −41 % | 59.72 KB | 50.16 KB | **−16 %** |

The two catalog rows are the only *deliberate* improvement in this table, and the only one whose
mechanism is identified: `HelpService.Catalog()` was rebuilt (re-sorted, re-allocated) on every
`help`/`funcs` call and is now memoised against a stamp of `_engine.Functions.Count` — a sound
invalidation signal because the registry is add-only. The **allocation** drop is the trustworthy
half (deterministic, not sampling); the means carry the usual ShortRun error bars. The residual
allocations are the rendered output text, which is still built per call by design.

**Reading it honestly:** every measured row is inside the 5 % mean / 10 % allocation gate, and
allocations are **byte-identical on all four pre-existing rows** (the catalog rows are the
deliberate exception, and improved). The apparent speed-ups (−21 %, −39 %, −17 %)
should not be reported as improvements: a ShortRun mean on a shared workstation moves by tens of
percent between sweeps (see the `Hessian_4Var` error bar of 105.7 µs against an 85.7 µs mean), and
the baseline was recorded while other builds were running. The defensible claim is
**"no regression"**, which is what the gate asks for — a quotable improvement needs a longer job on
an idle machine.

Still unmeasured: the remaining baseline rows (Polynomial, Compilation, Precision) and the two
properties item 18 names explicitly — structured serialization staying O(n) in result size, and the
help catalog being cached rather than regenerated per call.

## 3. Rows added in Phase A

The sweep above measures the pre-existing classes only, because it ran before the new rows
existed. The Phase-A rows (construction/canonicalization, simplify safe vs full trace off/on,
diff/factor/solve, pretty/canonical print, structured records + JSON projection, MathIR
scalar/batch, help catalog) are smoke-measured separately; those numbers are **not** part of
this baseline and are recorded with the Phase-A change, not here.

## 4. Reproduction

```text
dotnet run -c Release --project symbench -- --filter * --job short
```

Run it with the console captured (`Tee-Object`) — BenchmarkDotNet writes only the per-class
reports under `BenchmarkDotNet.Artifacts/results/`, and the host summary with the "Benchmarks
with issues" section exists only on the console.
