# Round 30 — implementation record

| Item | Status | Artifact |
|---|---|---|
| **19** — `--omit-display` decision recorded | **done** | `docs/symbolics/a-plus-cycle-3-alignment.md` §13 (new) |
| **18** — quotable benchmark run (longer job) | **done** | `benchmarks/symbench-longrun.md` (new) + `benchmarks/raw-longrun.txt` |

**Scope compliance.** Files written: `docs/symbolics/a-plus-cycle-3-alignment.md` (one appended
section), `benchmarks/symbench-longrun.md`, `benchmarks/raw-longrun.txt`,
`benchmarks/raw-longrun-cpu.txt`, `benchmarks/bdn-reports-longrun/` (10 copied BDN reports), and
this file. **No production or test code was touched. No git commit was made.**
`benchmarks/symbench-baseline.md` (15 878 B, mtime 2026-09-10 20:08:38) and
`benchmarks/raw-baseline.txt` (62 868 B, mtime 2026-09-10 18:12:41) are **untouched** — both still
carry their 2026-09-10 timestamps.

**Wall clock.** Benchmark process started 09:15:35, finished 09:27:08 local → **11 min 33 s**
(BDN's own accounting: `Global total time: 00:11:29 (689.36 sec), executed benchmarks: 14`).
Well inside the 35-minute ceiling; nothing had to be killed and no partial transcript was needed.

---

## 1. Item 19 — `--omit-display` is DECLINED, and now recorded

### 1.1 What was written

A new dated section, **§13 "Item 19 — `--omit-display` is DECLINED (recorded 2026-09-11, Cycle-3
round 30)"**, appended to `docs/symbolics/a-plus-cycle-3-alignment.md` (the file grew from 39 560 B
to 45 437 B). It is a decision record, not a proposal, with four parts:

**§13.1 Decision.** DECLINED — the flag will not be implemented; the runner keeps emitting the human
projection of every value alongside its structure and keeps refusing the flag as an unknown argument.

**§13.2 Reason — exactly which protocol fields it would strip, and where the protocol documents them:**

| Would be stripped | Declared / built at | Documented at |
|---|---|---|
| `result.display` | `Lovelace.Run/RunProtocol.cs:27` (`ResultDto(string Kind, string Display, string Typed, StructuredValueDto Structured)`), built at `Runner.cs:203` via `ValueFormatter.Format(result)` | `docs/symbolics/dsh-protocol.md:107` — `"display": "SolveResult(status: Solved, ...)"` inside the envelope's `result` object |
| `result.typed` | same record, `ValueFormatter.FormatTyped(result)` | `docs/symbolics/dsh-protocol.md:108` — `"typed": "SolveResult(...) (SolveResult)"` |
| `variables[].display` (if applied to variables too) | `RunProtocol.cs:17`, populated at `Runner.cs:185` | same envelope contract; `--omit-variables` already covers the *whole* array legitimately |

The line reference is the protocol's own **"A real solve envelope (abridged)"** example
(`dsh-protocol.md:98-135`), where `display` and `typed` sit on the `result` object next to
`structured` — i.e. they are part of the documented value shape that every consumer codes against,
and the envelope is **versioned** (`dsh-protocol.md:16`, invariant 4), so deleting them is a protocol
revision, not a host preference. Reinforcing citation: `dsh-protocol.md:3-5`, the document's opening
contract sentence — the envelope is "designed so that an agent never has to parse a display string to
recover mathematical meaning", which defines the display string as the human half that rides
*alongside* structure. The cost side is empty: no probe or measurement shows envelope size to be a
problem, so the flag would trade a documented contract field for an unmeasured saving.

**§13.3 Revisit condition.** Only via an explicit, reviewed revision of `dsh-protocol.md` that
(a) removes `display`/`typed` from the documented envelope, (b) says what replaces the human
projection, and (c) states the compatibility story for consumers and the golden fixtures. **Never as
a convenience flag**, and never in a host-only change that leaves the protocol document describing
fields the runner no longer emits.

**§13.4 Verification** — the observed refusal (below), plus its source: the structural usage refusal
at `Lovelace.Run/Runner.cs:125` and the usage text at `Runner.cs:345-346`, which lists only
`--omit-functions` and `--omit-variables`.

**§13.5** links `benchmarks/symbench-longrun.md` (item 18) so the two records live together.

### 1.2 Observed evidence — the refusal, with its exit code

Command (Windows, transcript merged, exit code captured by `cmd.exe`):

```text
cmd /v:on /c "out\aot\Lovelace.Run.exe --omit-display 1> %TEMP%\omit-display.txt 2>&1 & set RC=!ERRORLEVEL! & type %TEMP%\omit-display.txt & echo EXIT=!RC!"
```

Observed output, verbatim:

```text
Error: Unknown argument '--omit-display'.
Lovelace.Run — evaluate a Lovelace script and emit a JSON envelope.

Usage:
  Lovelace.Run --eval "<script>" [options]
  Lovelace.Run <file.ls> [options]
  Lovelace.Run --stdin [options]

Options:
  --eval <script>      evaluate the given script text
  --file <path>        read the script from a file
  --stdin              read the script from standard input
  --plot-dir <dir>     directory for plot() SVG output
  --plot-file <name>   filename for plot() SVG output (default: plot.svg)
  --omit-functions     omit the builtin registry from the envelope (agent loops)
  --omit-variables     omit the variables array from the envelope (agent loops)
  --print-budget <n>   abbreviate structured renderings beyond n nodes, reporting the truncation
  --cancel-after <ms>  cancel the evaluation after the given time, returning the partial result
  --json               emit JSON (default)
  --text               emit a human-readable summary
  --help, -h           show this help
EXIT=2
```

`--omit-display` is therefore **neither implemented nor silently ignored**: it exits **2** with a
structural usage error, which is the behaviour §7 item 3 of the addendum asks the cycle to preserve.
No flag was implemented; the runner was not changed.

---

## 2. Item 18 — a quotable benchmark run (MediumRun)

Full method and numbers: **`benchmarks/symbench-longrun.md`** (18 958 B). Summary here.

### 2.1 Method (read from the project, not guessed)

`symbench/Program.cs` is a plain `BenchmarkSwitcher` over `DefaultConfig.Instance`, so BDN's own
command line applies; the baseline's recorded invocation (`symbench-baseline.md` line 10) uses
`--job short`, and BDN accepts the longer built-in job names in the same position. Command actually
run (background process, polled; transcript to a **new** file):

```text
dotnet run -c Release --project symbench -- --filter "*SolverBenchmarks.Factor_SexticRationalRoots" "*SimplificationBenchmarks*" "*PrintingBenchmarks*" "*StructuredResultBenchmarks*" "*HelpCatalogBenchmarks*" --job medium 2>&1 | Tee-Object -FilePath benchmarks/raw-longrun.txt
```

* **Job kind:** `MediumRun(IterationCount=15, LaunchCount=2, WarmupCount=10)` — 5× the measured
  iterations and 2× the launches of the baseline's ShortRun; 50 measured executions per row instead
  of 6. `Error` is half of a **99.9 %** confidence interval. Every class carries `[MemoryDiagnoser]`.
* **Iterations:** 15 measured × 2 launches, after 10 warmups per launch (BDN's pilot phase chose the
  operation count per row).
* **Raw transcript:** `benchmarks/raw-longrun.txt` — 197 279 B, 3 553 lines, UTF-8,
  SHA-256 `B9E1F87EDCDCB6B44C7B0A360C8D71AF5AF86FAE94ACEA63A9D76443927ECF8C` (PS 5.1 `Tee-Object`
  wrote UTF-16LE; transcoded in place, content all-ASCII so the transcode is lossless).
* **Per-class reports:** `benchmarks/bdn-reports-longrun/` (BDN's own `-report-github.md`/`.csv` for
  the five classes; `BenchmarkDotNet.Artifacts/` is gitignored at `.gitignore:47`).
* **Provenance:** HEAD `35e2609f541c14d3933987657ec27909e7424d11`, dirty tree (62 porcelain
  entries), .NET SDK 10.0.103, BDN 0.15.8, Ryzen 9 5900X / 24 logical cores / Windows 11 Pro.

Transcript tail (observed):

```text
// ***** BenchmarkRunner: End *****
Run time: 00:04:58 (298.13 sec), executed benchmarks: 6

Global total time: 00:11:29 (689.36 sec), executed benchmarks: 14
// * Artifacts cleanup *
Artifacts cleanup is finished
BENCH_EXIT=0
BENCH_DONE=2026-09-11T09:27:08.3501669-03:00
```

### 2.2 Rows measured — 14, each with mean, error bar and allocation

| Class | Method | Mean | Error (99.9 %) | Allocated |
|---|---|---|---|---|
| Solver | `Factor_SexticRationalRoots` | 187.6 μs | ± 1.65 μs | 755.16 KB |
| Simplification | `Simplify_Safe` | 57.16 μs | ± 0.620 μs | 201.09 KB |
| Simplification | `Simplify_Full_TraceOff` | 56.06 μs | ± 0.775 μs | 199.24 KB |
| Simplification | `Simplify_Full_TraceOn` | 56.75 μs | ± 0.576 μs | 199.24 KB |
| Printing | `PrettyPrint_TranscendentalComposition` | 967.5 ns | ± 23.92 ns | 3.99 KB |
| Printing | `CanonicalPrint_TranscendentalComposition` | 713.0 ns | ± 19.65 ns | 3.11 KB |
| Structured | `Record_Construct_SolveResult` | 1 081.541 ns | ± 21.2296 ns | 4 912 B |
| Structured | `Record_FieldLookup_ByName` | 4.409 ns | ± 0.0244 ns | **0 B** |
| Structured | `Record_MemberAccess_Solutions` | 744.635 ns | ± 34.2562 ns *(BDN: bimodal, mValue 3.87)* | 2 512 B |
| Structured | `Json_SerializeSolveResult` (local copy) | 3 492.425 ns | ± 19.9873 ns | 7 969 B |
| Structured | `Json_SerializeSolveResult_ShippedProjection` | 3 822.354 ns | ± 64.0355 ns | 10 353 B |
| Structured | `…_ShippedProjection_64` (32× data) | 61 003.621 ns | ± 432.1121 ns | 172 603 B |
| HelpCatalog | `Help_Overview` | 4.532 μs | ± 0.0742 μs | 12.05 KB |
| HelpCatalog | `Help_Funcs_AllCategories` | 10.918 μs | ± 0.2135 μs | 51.65 KB |

`Error` ≤ 2.5 % of the mean on 12 of 14 rows (exceptions: `CanonicalPrint` 2.76 %,
`Record_MemberAccess_Solutions` 4.60 %, which is the bimodal one).

### 2.3 Machine-idle judgement — **the machine was NOT idle**

| Evidence | Value |
|---|---|
| `\Processor(_Total)\% Processor Time`, 126 samples at 5 s across the whole run | min 4.7 % · median 10.1 % · mean 10.9 % · max 20.8 % (24 logical cores) |
| Resident desktop processes | `Notepad`, 3× `chrome`, `Code`, `steam` + 3× `steamwebhelper`, present throughout |
| Other builds/tests in flight | **none** — the only `dotnet` processes were BDN's own restore/build and its benchmark children |

Honest label: a **shared workstation with a resident desktop**, not an idle box. That is *less* bad
than the baseline's condition (the baseline ran while other builds/tests were in flight) and the
within-run error bars are small, but no number here is lab-grade, and cross-job comparisons against
the baseline's ShortRun means remain the weak link. Full log: `benchmarks/raw-longrun-cpu.txt`.

### 2.4 What the error bars do and do not support

**Supported:**
* Every row above is quotable **with its error bar** for this tree on this machine.
* **Structured serialization is sub-linear in result size** — through the shipped projection, 32× the
  solutions costs **15.96× the time** (3 822.354 ns → 61 003.621 ns) and **16.67× the bytes**
  (10 353 B → 172 603 B). Both are far below 32× and the largest relative error in the pair is 1.7 %,
  so the error bars cannot bridge the gap. This is the one improvement-class claim the run supports.
* `Record_FieldLookup_ByName` is **allocation-free** (0 B) at 4.409 ± 0.0244 ns.
* The provenance trace costs nothing measurable at this workload (`TraceOn` − `TraceOff` = 0.69 μs,
  intervals overlap) — a bounded null, not a saving.
* The help catalog's allocation is deterministic and lower than pre-memoisation
  (`Help_Overview` 21.49 KB → 12.05 KB; `Help_Funcs_AllCategories` 59.72 KB → 51.65 KB).

**Not supported (and not claimed):**
* No "improved the mean by X % against the baseline" claim. The baseline means are ShortRun means
  with **no recorded error bars**, taken on a busier machine; the help-catalog speed-up
  (10.65 → 4.53 μs) is directionally supported by the allocation drop and the memoisation, but the
  improvement is not *measured as a delta*. Cycle-3's defensible claim stays **"no regression"**.
* `Factor_SexticRationalRoots`, the three simplify rows and both printing rows have no comparable
  before/after pair under this job: the first two groups are **first records** (no baseline row at
  all), and `PrettyPrint` is the one row whose allocation moved *up* against the baseline
  (3.73 KB → 3.99 KB, +7 %; mean 850.2 → 967.5 ns) — recorded as **a row to watch**, not a regression
  claim and not dismissed as noise.
* `Record_MemberAccess_Solutions`' error bar is optimistic (BDN flags a bimodal distribution).

### 2.5 Row coverage

**Measured: 14 of the 32 rows in `symbench`** — the factor row; the three rewrite rows
(`Simplify_Safe`, `Simplify_Full_TraceOff`, `Simplify_Full_TraceOn`); the structured-serialization
and record rows (6); the help-catalog rows (2); plus the two printer rows.

**Skipped: 18** — `Construction` (1), `Calculus` (3), `Polynomial` (2), `Compilation` (3,
multi-second rows), `Precision` (3 — `Eval256` ≈ 2.7 s/op and `Eval1024` ≈ 509 s/op cannot fit a
bounded job), `ConstructionCanonicalization` (2), `SolverBenchmarks` diff + quartic solve (2,
excluded by the pinned filter), `MathIR` (2). None of them is touched by this round's changes; the
per-row reasons are tabulated in `benchmarks/symbench-longrun.md` §4.

**The assumption row does not exist.** `Select-String -Pattern 'assum' -Path symbench/*.cs` returns
**0 hits** — `symbench` has no `AssumptionSet.Add` benchmark, so the "53× faster at 800 atoms"
change is **not measured by this run**, and measuring it would require adding a row to
`symbench/Benchmarks.cs`, which is out of this round's scope (no code changes). It remains supported
only by the Cycle-2 scaling record cited in alignment addendum §3.9 (100→400 atoms, exponent ≈ 1.8).

---

## 3. Verification checklist (all observed, none assumed)

| Required observation | Where it is pasted | Result |
|---|---|---|
| `--omit-display` refusal **with exit code** | §1.2 above and alignment §13.4 | `Error: Unknown argument '--omit-display'.` + usage, **EXIT=2** |
| Benchmark **command line** | §2.1 and `benchmarks/symbench-longrun.md` §1/§6 | `… --filter … --job medium … \| Tee-Object -FilePath benchmarks/raw-longrun.txt` |
| Benchmark **transcript tail** | §2.1 above | `Global total time: 00:11:29 (689.36 sec), executed benchmarks: 14`, `BENCH_EXIT=0` |
| **Rows measured, with means and error bars** | §2.2 above; full tables in `benchmarks/symbench-longrun.md` §3 | 14 rows, each with `Mean` **and** `Error` (99.9 %) **and** `Allocated` |
| **Wall clock the run actually took** | §2.1 above | **11 min 33 s** (09:15:35 → 09:27:08); BDN accounting `00:11:29` |
| Machine-idle judgement | §2.3 above + `benchmarks/raw-longrun-cpu.txt` | **Not idle** — resident desktop; total CPU mean 10.9 %, max 20.8 % |

**Forbidden-action check:** the existing baseline and its transcript were not overwritten
(§"Scope compliance"); no mean is presented without its error bar; no improvement is claimed that
the error bars do not support (§2.4); `--omit-display` was not implemented and the runner was not
changed; **no git commit was made**.
