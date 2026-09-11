# symbench long run — A+ Cycle-3 round 30 (alignment-plan item 18)

| Item | Value |
|---|---|
| Date | 2026-09-11, sweep 09:15:35 → 09:27:08 local (11 min 33 s wall clock) |
| HEAD | `35e2609f541c14d3933987657ec27909e7424d11` — "docs(symbolics): Cycle-2 final report" (`main`) |
| Working tree | **dirty** — 62 entries in `git status --porcelain` (Cycle-1/2/3 work uncommitted) |
| Machine | AMD Ryzen 9 5900X, 24 logical / 12 physical cores — Windows 11 Pro 10.0.26100 |
| Toolchain | .NET SDK **10.0.103**; runtime .NET 10.0.3 (10.0.326.7603), X64 RyuJIT x86-64-v3, GC = Concurrent Workstation; BenchmarkDotNet **0.15.8** |
| Job | **`MediumRun(IterationCount=15, LaunchCount=2, WarmupCount=10)`** (BDN `--job medium`) |
| Command | `dotnet run -c Release --project symbench -- --filter "*SolverBenchmarks.Factor_SexticRationalRoots" "*SimplificationBenchmarks*" "*PrintingBenchmarks*" "*StructuredResultBenchmarks*" "*HelpCatalogBenchmarks*" --job medium` (see §1) |
| Rows | **14 measured, 18 skipped** (of 32 rows in `symbench`); 5 classes |
| Raw transcript | `benchmarks/raw-longrun.txt` — 197 279 bytes, 3 553 lines, UTF-8, SHA-256 `B9E1F87EDCDCB6B44C7B0A360C8D71AF5AF86FAE94ACEA63A9D76443927ECF8C` |
| Per-class reports | `benchmarks/bdn-reports-longrun/` (copied out of `BenchmarkDotNet.Artifacts/`, which is gitignored) |
| Load log | `benchmarks/raw-longrun-cpu.txt` — 126 samples of `\Processor(_Total)\% Processor Time` at 5 s |
| Process exit | `BENCH_EXIT=0`, `Global total time: 00:11:29 (689.36 sec), executed benchmarks: 14` |

> **This file does not replace `benchmarks/symbench-baseline.md`.** The baseline and its transcript
> (`benchmarks/raw-baseline.txt`) are untouched; this is a *second* measurement, taken with a longer
> job on the round-30 tree. Baseline numbers are cited here only with the caveat that they are
> ShortRun means whose 99.9 % intervals were routinely as wide as the mean itself (baseline §1).

---

## 1. Method

```text
dotnet run -c Release --project symbench -- --filter "*SolverBenchmarks.Factor_SexticRationalRoots" ^
  "*SimplificationBenchmarks*" "*PrintingBenchmarks*" "*StructuredResultBenchmarks*" ^
  "*HelpCatalogBenchmarks*" --job medium 2>&1 | Tee-Object -FilePath benchmarks/raw-longrun.txt
```

* **Job kind.** BDN resolved `--job medium` to `MediumRun(IterationCount=15, LaunchCount=2,
  WarmupCount=10)` — visible in the transcript at every `// Benchmark: … : MediumRun(...)` line and
  in each report header. Against the baseline's ShortRun (3 warmup / 1 launch / 3 measured) that is
  **5× the measured iterations, 2× the launches and 50 measured executions instead of 6**; each
  row's `Error` here is half of a **99.9 % confidence interval**.
* **Selection.** The class/method filters were chosen to cover the rows the round's changes touch:
  the factor row, the rewrite (simplify) rows, the structured-serialization rows, the help-catalog
  rows, plus the two printer rows. `--filter "*SolverBenchmarks.Factor_SexticRationalRoots"` pins the
  single factor row so the two unmeasured `SolverBenchmarks` rows (diff, quartic solve) do not pad
  the wall clock. The transcript confirms `Found 2 / 2 / 3 / 1 / 6 benchmark(s)` per class = 14.
* **Allocation column.** Every class in `symbench` carries `[MemoryDiagnoser]`, so `Allocated` is
  BDN's per-operation managed allocation — the deterministic column.
* **Scope of the measurement.** No production, test or benchmark source was modified for this run;
  it measures the round-30 working tree as it stands.
* **Transcript encoding.** Windows PowerShell 5.1 `Tee-Object` wrote UTF-16LE. The file was
  transcoded in place to UTF-8 (`[System.IO.File]::ReadAllText` → `WriteAllText` with
  `UTF8Encoding($false)`); the content is all-ASCII, so the transcode is lossless
  (394 560 UTF-16 bytes → 197 279 UTF-8 bytes for the same character count).

## 2. Was the machine idle? — **No. Say it plainly.**

The run took place on a **shared workstation with a resident desktop**, not an idle box:

| Evidence | Value |
|---|---|
| `\Processor(_Total)\% Processor Time`, 126 samples over the run | **min 4.7 % · median 10.1 % · mean 10.9 % · max 20.8 %** (24 logical cores) |
| Resident applications during the run | `Notepad`, 3× `chrome`, `Code` (VS Code), `steam` + 3× `steamwebhelper` — all present before, during and after |
| Other builds/tests in flight | **None observed.** The only `dotnet` processes were BDN's own restore/build and its per-benchmark child processes |

Interpretation, honestly:

* The machine was **not idle** — roughly 2.5 logical cores' worth of work was in flight on average,
  about one of which is the benchmark itself. The baseline's caveat ("other builds/tests were in
  flight") is *weaker* here: no second build was running, but the desktop was not quiet either.
* The **within-run** evidence says the noise was low: `Error` is ≤ 2.5 % of the mean on 12 of the 14
  rows (§3), which is what an uncontended run looks like.
* The load was **steady** rather than spiky (no sample above 21 %), so the rows are not visibly
  contaminated by a burst of background work.
* **What this does not license:** treating these numbers as lab-grade, or comparing them to the
  baseline's ShortRun means as if both were taken under the same conditions. They were not.

**Label for every number below: quotable for *this* machine, on *this* day, as a MediumRun
measurement — with its error bar. Not a controlled-environment figure.**

---

## 3. Rows measured — 14, with error bars and allocations

Verbatim BenchmarkDotNet reports. `Error` = half of the **99.9 %** confidence interval; `Allocated`
is per operation.

### 3.1 `SymBench.SolverBenchmarks` — the factor row

```
| Method                     | Mean     | Error   | StdDev  | Gen0    | Gen1   | Allocated |
|--------------------------- |---------:|--------:|--------:|--------:|-------:|----------:|
| Factor_SexticRationalRoots | 187.6 μs | 1.65 μs | 2.26 μs | 46.1426 | 1.2207 | 755.16 KB |
```

`x^6 - 14x^4 + 49x^2 - 36 = (x-1)(x+1)(x-2)(x+2)(x-3)(x+3)`. **First recorded number for this row** —
`Factor_SexticRationalRoots` appears in no baseline table, so there is nothing to compare it to.

### 3.2 `SymBench.SimplificationBenchmarks` — the rewrite rows

```
| Method                 | Mean     | Error    | StdDev   | Gen0    | Gen1   | Allocated |
|----------------------- |---------:|---------:|---------:|--------:|-------:|----------:|
| Simplify_Safe          | 57.16 μs | 0.620 μs | 0.869 μs | 12.2681 | 0.1831 | 201.09 KB |
| Simplify_Full_TraceOff | 56.06 μs | 0.775 μs | 1.160 μs | 12.1460 | 0.1831 | 199.24 KB |
| Simplify_Full_TraceOn  | 56.75 μs | 0.576 μs | 0.862 μs | 12.1460 | 0.1831 | 199.24 KB |
```

No baseline counterpart exists for these rows either (the Phase-A rows were never folded into the
baseline tables). Within this run they are comparable to each other: `TraceOn` is 0.69 μs above
`TraceOff` and `Simplify_Safe` is 1.10 μs above it — **both differences are inside the combined
error bars**, and the two full-simplify rows allocate byte-identical 199.24 KB. The defensible
statement is a *bounded null*: at this workload the provenance trace costs nothing measurable
(≤ ~1.5 μs, 99.9 %), not "trace is free".

### 3.3 `SymBench.PrintingBenchmarks`

```
| Method                                   | Mean     | Error    | StdDev   | Gen0   | Allocated |
|----------------------------------------- |---------:|---------:|---------:|-------:|----------:|
| PrettyPrint_TranscendentalComposition    | 967.5 ns | 23.92 ns | 35.07 ns | 0.2441 |   3.99 KB |
| CanonicalPrint_TranscendentalComposition | 713.0 ns | 19.65 ns | 29.41 ns | 0.1898 |   3.11 KB |
```

### 3.4 `SymBench.StructuredResultBenchmarks` — structured records and the JSON projection

```
| Method                                         | Mean          | Error       | StdDev      | Gen0    | Gen1   | Allocated |
|----------------------------------------------- |--------------:|------------:|------------:|--------:|-------:|----------:|
| Record_Construct_SolveResult                   |  1,081.541 ns |  21.2296 ns |  30.4469 ns |  0.2918 | 0.0038 |    4912 B |
| Record_FieldLookup_ByName                      |      4.409 ns |   0.0244 ns |   0.0325 ns |       - |      - |         - |
| Record_MemberAccess_Solutions                  |    744.635 ns |  34.2562 ns |  50.2124 ns |  0.1497 |      - |    2512 B |
| Json_SerializeSolveResult                      |  3,492.425 ns |  19.9873 ns |  28.0195 ns |  0.4730 | 0.0038 |    7969 B |
| Json_SerializeSolveResult_ShippedProjection    |  3,822.354 ns |  64.0355 ns |  91.8378 ns |  0.6180 | 0.0076 |   10353 B |
| Json_SerializeSolveResult_ShippedProjection_64 | 61,003.621 ns | 432.1121 ns | 591.4800 ns | 10.2539 |      - |  172603 B |
```

BDN warnings attached to this class (quoted from the transcript, not suppressed):

```text
// * Warnings *
MultimodalDistribution
  StructuredResultBenchmarks.Record_MemberAccess_Solutions: MediumRun -> It seems that the distribution is bimodal (mValue = 3.87)

// * Hints *
Outliers
  StructuredResultBenchmarks.Record_FieldLookup_ByName: MediumRun -> 4 outliers were removed (5.80 ns..6.40 ns)
  StructuredResultBenchmarks.Json_SerializeSolveResult: MediumRun -> 2 outliers were removed (3.61 us, 3.62 us)
```

`Record_MemberAccess_Solutions` is flagged **bimodal** — its 34 ns error bar is optimistic and its
mean should be read as "≈0.74 μs, with a second mode".

### 3.5 `SymBench.HelpCatalogBenchmarks` — the help-catalog rows

```
| Method                   | Mean      | Error     | StdDev    | Gen0   | Gen1   | Allocated |
|------------------------- |----------:|----------:|----------:|-------:|-------:|----------:|
| Help_Overview            |  4.532 μs | 0.0742 μs | 0.1110 μs | 0.7324 | 0.0076 |  12.05 KB |
| Help_Funcs_AllCategories | 10.918 μs | 0.2135 μs | 0.3129 μs | 3.1586 | 0.1221 |  51.65 KB |
```

### 3.6 Deltas against the recorded Cycle-2 numbers (read the caveats, not just the arrows)

Baseline columns: **B(r25)** = baseline §2b, ShortRun job, pre-memoisation; **B(r36)** = baseline §2c,
ShortRun job, post-memoisation. Neither baseline column recorded an error bar for these rows.

| Row | B(r25) mean | B(r36) mean | This run (MediumRun) | B(r25) alloc | B(r36) alloc | This run alloc |
|---|---|---|---|---|---|---|
| `Help_Overview` | 10.65 μs | 4.201 μs | **4.532 ± 0.0742 μs** | 21.49 KB | 11.93 KB | **12.05 KB** |
| `Help_Funcs_AllCategories` | 16.40 μs | 9.641 μs | **10.918 ± 0.2135 μs** | 59.72 KB | 50.16 KB | **51.65 KB** |
| `Record_Construct_SolveResult` | 1,049.9 ns | — | **1,081.541 ± 21.23 ns** | 4,944 B | — | **4,912 B** |
| `Record_FieldLookup_ByName` | 4.887 ns | — | **4.409 ± 0.0244 ns** | 0 B | — | **0 B** |
| `Record_MemberAccess_Solutions` | 745.3 ns | — | **744.635 ± 34.26 ns** (bimodal) | 2,512 B | — | **2,512 B** |
| `Json_SerializeSolveResult` (local copy) | 4,186.6 ns | — | **3,492.425 ± 19.99 ns** | 8,041 B | — | **7,969 B** |
| `…ShippedProjection` | 3,781.8 ns | — | **3,822.354 ± 64.04 ns** | 10,353 B | — | **10,353 B** |
| `…ShippedProjection_64` (32× data) | 61,084.3 ns | — | **61,003.621 ± 432.11 ns** | 172,603 B | — | **172,603 B** |
| `PrettyPrint_TranscendentalComposition` | 850.2 ns | — | **967.5 ± 23.92 ns** | 3.73 KB | — | **3.99 KB** |
| `CanonicalPrint_TranscendentalComposition` | 682.4 ns | — | **713.0 ± 19.65 ns** | 3.11 KB | — | **3.11 KB** |
| `Factor_SexticRationalRoots` | — | — | **187.6 ± 1.65 μs** | — | — | **755.16 KB** |
| `Simplify_Safe` / `…TraceOff` / `…TraceOn` | — | — | **57.16 ± 0.620 / 56.06 ± 0.775 / 56.75 ± 0.576 μs** | — | — | **201.09 / 199.24 / 199.24 KB** |

Allocation-column consistency across three separate jobs (ShortRun r25, ShortRun r36, MediumRun
r30) is itself a result: the three large structured rows are **byte-identical** (10,353 B,
172,603 B, 2,512 B) and `CanonicalPrint` and `Record_FieldLookup` are too (3.11 KB, 0 B), while the
smallest rows moved by ≤ 1 % (`Record_Construct` −32 B, `Json_SerializeSolveResult` −72 B) and the
two help rows by +1.0 % / +3.0 %. The allocation column is therefore reproducible to the byte on
rows big enough to be insensitive to per-iteration amortisation, and to ~1 % on the tiny ones.

---

## 4. Row coverage — what was measured and what was skipped

**Measured (14 of 32 `symbench` rows):** `SolverBenchmarks.Factor_SexticRationalRoots`;
`SimplificationBenchmarks` (3 — `Simplify_Safe`, `Simplify_Full_TraceOff`, `Simplify_Full_TraceOn`);
`PrintingBenchmarks` (2); `StructuredResultBenchmarks` (6); `HelpCatalogBenchmarks` (2).

**Skipped (18), by class:**

| Class | Skipped rows | Why |
|---|---|---|
| `ConstructionBenchmarks` | `CanonicalParse_SharedDAG` | not touched by this round's changes; unchanged code, baseline row already exists |
| `CalculusBenchmarks` | `Diff_TenthOrder`, `Jacobian_4x4`, `Hessian_4Var` | same |
| `PolynomialBenchmarks` | `Multiply_SparseBivariateDegree20`, `Groebner_Cyclic3_GrLex` | same |
| `CompilationBenchmarks` | `Kernel_EvaluateScalar`, `Kernel_EvaluateBatch_1000Lanes`, `TreeEvaluator_Direct` | multi-second rows; would dominate the wall clock for no round-30 question |
| `PrecisionBenchmarks` | `Eval64`, `Eval256`, `Eval1024` | `Eval256` ≈ 2.7 s/op, `Eval1024` ≈ 509 s/op (baseline §2) — cannot fit any bounded job |
| `ConstructionCanonicalizationBenchmarks` | `Construct_SharedDAG`, `ConstructAndCanonicalize_SharedDAG` | not touched |
| `SolverBenchmarks` | `Diff_TranscendentalComposition`, `Solve_QuarticAlgebraic` | filter pinned to the factor row |
| `MathIrEvaluationBenchmarks` | `MathIR_Evaluate_Scalar`, `MathIR_Evaluate_Batch_1000Lanes` | not touched |

**The assumption row does not exist — it was not skipped, it is absent.** `Select-String -Pattern
'assum' -Path symbench/*.cs` returns **0 hits**: `symbench` contains no benchmark of
`AssumptionSet.Add`, so the "53× faster at 800 atoms" claim is **not measured by this run and is not
measurable without adding a row to `symbench/Benchmarks.cs` — which is out of this round's scope
(no code changes).** It remains supported only by the Cycle-2 scaling record cited in the alignment
addendum §3.9 (100→400 atoms, exponent ≈ 1.8).

Also absent from `symbench`: any `factor()`-termination workload other than the sextic above, and
any row that exercises the LaTeX printer mode.

---

## 5. What is — and is not — defensible from this run

**Defensible:**

1. **The §3 numbers for the current tree are quotable with their error bars.** MediumRun
   (15 iterations × 2 launches, 99.9 % CI); `Error` ≤ 2.5 % of the mean on 12 of 14 rows. Quote the
   row *with* its `Error`, and with the machine caveat in §2.
2. **Structured serialization growth is sub-linear in result size, and the error bars cannot bridge
   the gap.** Through the shipped projection: 32× the solutions costs **15.96× the time**
   (3,822.354 ns → 61,003.621 ns) and **16.67× the bytes** (10,353 B → 172,603 B). Both are far below
   32×, and the largest relative error bar in the pair is 1.7 %, so this is not a "means inside their
   error bars" situation. The residual shape is a fixed per-envelope cost plus a per-solution cost,
   as baseline §2b already argued — now measured on a longer job. **This is the one improvement-class
   claim this run supports.**
3. **`Record_FieldLookup_ByName` is allocation-free and ~4.4 ns.** 0 B is deterministic; the time is
   quotable as 4.409 ns ± 0.0244 ns. Alignment item 26's "leave it alone" verdict is unchanged.
4. **The provenance trace is not measurably expensive at this workload.** `TraceOn` − `TraceOff` =
   0.69 μs with overlapping 99.9 % intervals — a bounded null, not a cost claim.
5. **The help catalog's allocation is deterministic and lower than the pre-memoisation baseline**
   (21.49 KB → 12.05 KB on `Help_Overview`; 59.72 KB → 51.65 KB on `Help_Funcs_AllCategories`).
   The *current* means (4.532 ± 0.0742 μs, 10.918 ± 0.2135 μs) are quotable on their own.

**Not defensible:**

1. **Any "we improved the mean by X % vs the baseline" claim.** The baseline means are ShortRun means
   with no recorded error bars, taken on a busier machine. The help-catalog *speed-up* (10.65 →
   4.53 μs) is the clearest such case: the allocation evidence and the memoisations explain it, but
   the baseline mean itself is not quotable, so the improvement is **supported in direction, not
   measured as a delta**. Cycle-3's honest claim remains "no regression", plus the allocation drop.
2. **Treating `Record_MemberAccess_Solutions`' error bar as real** — BDN flags the distribution as
   bimodal (mValue 3.87).
3. **Any claim that the pretty printer got faster or cheaper.** `PrettyPrint` is the one row whose
   allocation moved *up* against the baseline (3.73 KB → 3.99 KB, ≈ +260 B/op, +7 %) and whose mean is
   13.8 % above the baseline's ShortRun mean (967.5 ns vs 850.2 ns). Against an un-error-barred
   ShortRun row this is **directional only**; it is recorded as *a row to watch*, not as a regression
   and not as noise, because no comparable before/after pair exists under the same job.
4. **`Factor_SexticRationalRoots` and the three simplify rows as improvements.** They have no
   baseline counterpart at all. 187.6 ± 1.65 μs / 755.16 KB and 56–57 μs / ~200 KB are **records**,
   the first defensible numbers for those rows.
5. **Lab-grade precision.** The machine was not idle (§2). These figures describe this workstation as
   configured today, with the desktop running.

---

## 6. Reproduction

```text
dotnet run -c Release --project symbench -- --filter "*SolverBenchmarks.Factor_SexticRationalRoots" \
  "*SimplificationBenchmarks*" "*PrintingBenchmarks*" "*StructuredResultBenchmarks*" \
  "*HelpCatalogBenchmarks*" --job medium 2>&1 | Tee-Object -FilePath benchmarks/raw-longrun.txt
```

Notes for the next person:

* On Windows PowerShell 5.1, `Tee-Object` writes **UTF-16LE**; transcode before reading the file with
  text tooling (this run's transcript was transcoded to UTF-8 in place, content all-ASCII).
* BDN writes the per-class reports to `BenchmarkDotNet.Artifacts/results/`, which is **gitignored**
  (`.gitignore:47`); a copy of the five classes from this run is committed alongside this file in
  `benchmarks/bdn-reports-longrun/`.
* The console transcript is the only place the "Warnings"/"Hints" sections (bimodal distribution,
  removed outliers) and the per-class `Run time` lines exist.
* Wall clock for this run: **11 min 33 s** (09:15:35 → 09:27:08); BDN's own accounting was
  `Global total time: 00:11:29` including a 24 s restore+build. The full 32-row sweep under
  `--job medium` is *not* bounded by that figure — `Eval256`/`Eval1024` alone would dominate it.
