# Cycle-6 · round-17 · cp1 — the three round-13 P1s: F2 (huge allocation), F3 (failed plot write), F4 (len of a zero-dimension array)

**Tree used.** A scratch worktree of my own: `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-cp1`, created with
`git -C C:\Users\ricar\dev\LovelaceSharp worktree add .worktrees/c6-cp1 HEAD` (detached HEAD `54b1753`).
Nothing is committed, staged or pushed: `git -C .worktrees\c6-cp1 status --short` contains 3 modified product files,
1 new product file and 5 new test files, and nothing else (§4, §5).
**Control tree.** `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-cp1ctl`, same command, same detached HEAD `54b1753`;
only the five new test files were copied into it (§7).
**Targets.** The pre-fix observations (§1) are on the published Native AOT binary the audit named,
`C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe` — untouched by this round; the main checkout's product
tree was not modified. The after-state was measured twice: on the worktree's own Release build
(`dotnet exec .worktrees\c6-cp1\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll`) and on a **fresh Native AOT
publish of the fixed tree** (§6), `.worktrees\c6-cp1\out\aot\Lovelace.Run.exe`, 5 800 448 bytes, 2026-09-12 16:36:02.

Shell is Windows PowerShell 5.1. Every envelope below is the process's own stdout, trimmed to the deciding fields
where it says so.

---

## 1. Pre-fix behaviour, re-observed (requirement 1)

### F2 — a huge but well-formed allocation is an internal failure

```powershell
& 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --eval 'zeros(1000000000)' --json --omit-functions --omit-variables
```

Observed (my own run, 46.0 s wall; stdout 429 bytes; stderr 0 bytes; exit **1**):

```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,
 "code":"InternalError","category":"InternalInvariantFailure",
 "message":"Insufficient memory to continue the execution of the program.","recoverable":false,
 "elapsed":"3.36 s","timings":[{"position":0,"elapsed":{"value":3.36,"unit":"s"},"resultKind":"Vector","hasOutput":false}]}
```

Two neighbouring shapes, measured on the same binary before the fix:

| script (via `--stdin`, same flags) | exit | code / category | recoverable | message |
|---|---|---|---|---|
| `zeros(1000000000,1000000000)` | 1 | `ArithmeticError` / `DomainError` | true | "Arithmetic operation resulted in an overflow." |
| `zeros(-1)` | 1 | `ArithmeticError` / `DomainError` | true | "Arithmetic operation resulted in an overflow." |

`zeros(10^9, 10^9)` needs 10^18 elements: the pre-fix product `checked((int)total)` overflowed an `int`, so that
shape reached the wire as an *arithmetic* error that names neither the request nor a limit.

### F3 — a failed plot write is an internal failure, the failed read is typed

```powershell
'plot([1,2,3])' | & 'C:\...\out\aot\Lovelace.Run.exe' --stdin --json --omit-functions --omit-variables --plot-dir "$env:TEMP\lv17cp1\w1" --plot-file 'nope/x.svg'
```

Observed (exit **1**, stdout 664 bytes, stderr 0):

```json
{"ok":false,"code":"InternalError","category":"InternalInvariantFailure",
 "message":"Could not find a part of the path 'C:\\Users\\ricar\\AppData\\Local\\Temp\\lv17cp1\\w1\\nope\\x.svg'.",
 "recoverable":false,"diagnostics":[{"message":"Could not find a part of the path …","position":0,"line":1,"column":1}]}
```

Same binary, same session, the control rows:

| probe | exit | code / category | recoverable |
|---|---|---|---|
| `plot([1,2,3])` + `--plot-file ok.svg` | 0 | (success, SVG written, result `Text` names the path) | — |
| `--file C:\definitely\missing\x.ls` (the READ side of the same class) | 1 | `FileReadError` / `ParseError` | **true** |

So the read side of a bad caller path was already typed and recoverable, and the write side was not.

### F4 — `len()` refuses the arrays `shape()` reports and the printer prints

```powershell
'len([[],[]])' | & '...\Lovelace.Run.exe' --stdin --json --omit-functions --omit-variables
```

Observed (exit **1**, stdout 571 bytes, stderr 0):

```json
{"ok":false,"code":"InvalidArgument","category":"TypeMismatch",
 "message":"Array dimensions must be positive, but got 0. (Parameter 'shape')","recoverable":true,
 "diagnostics":[{"message":"Array dimensions must be positive, but got 0. (Parameter 'shape')","position":0,"line":1,"column":1}]}
```

Identical refusal for `len([[]])`, `len(zeros(2,0))`, `len(zeros(0,3))`, `len(zeros(0,0))`, `len(zeros(1,0))`.
The *same* values, on the same binary, in the same session:

| probe | exit | result |
|---|---|---|
| `[[],[]]` | 0 | `[[], []] (Array)`, structured `shape:[2,0]` |
| `shape([[],[]])` | 0 | `[2, 0] (Vector)` |
| `numel(zeros(2,0))` | 0 | `0 (Natural)` |
| `zeros(2,0)[0]` | 0 | `[] (Vector)` |
| `len([])` | 0 | `0 (Natural)` |
| `len(zeros(2,3))` | 0 | `2 (Natural)` |

---

## 2. Failing-first (requirement 2)

Five new test files were written before any product change and run against the unmodified tree
(`.worktrees\c6-cp1`, HEAD `54b1753`). Commands, with `$env:LOVELACE_REQUIRE_SYMPY='1'` and the oracle on `PATH`:

```powershell
dotnet test Lovelace.Suite.Tests\Lovelace.Suite.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~LenZeroDimensionTests|FullyQualifiedName~PlotFileWriteRefusalTests"
dotnet test Lovelace.Suite.Tests\Lovelace.Suite.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~ArrayAllocationRefusalTests"
dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~PlotFileErrorTests|FullyQualifiedName~AllocationRefusalEnvelopeTests"
```

Observed (excerpts; the full text is in the run transcripts quoted here):

```text
  Failed Lovelace.Suite.Tests.PlotFileWriteRefusalTests.TheSuiteDeclaresThePlotFileWriteRefusalTheRunnerClassifies [61 ms]
   Assert.NotNull() Failure: Value is null
  Failed Lovelace.Suite.Tests.PlotFileWriteRefusalTests.PlotFileUnderAMissingDirectory_IsRefusedByType [4 ms]
   Assert.Equal() Failure: Strings differ
  Failed Lovelace.Suite.Tests.PlotFileWriteRefusalTests.EmptyPlotFile_IsRefusedByType [1 ms]
  Failed Lovelace.Suite.Tests.LenZeroDimensionTests.Len_IsTheFirstDimensionOfTheValue(source: "len([[],[]])", expected: "2") [< 1 ms]
  … len(zeros(0,0)) / len(zeros(0,3)) / len(zeros(1,0)) / len([[]]) / len(zeros(2,0)) likewise …
Failed!  - Failed:     9, Passed:     7, Skipped:     0, Total:    16, Duration: 304 ms - Lovelace.Suite.Tests.dll (net10.0)

  Failed Lovelace.Suite.Tests.ArrayAllocationRefusalTests.Zeros_TenToTheNinth_IsRefusedWithoutAllocating [4 s]
   Assert.NotNull() Failure: Value is null
  Failed Lovelace.Suite.Tests.ArrayAllocationRefusalTests.Zeros_ElementProductBeyondTheBudget_IsRefusedByNameAndLimit [3 ms]
   Assert.Equal() Failure: Strings differ
  Failed Lovelace.Suite.Tests.ArrayAllocationRefusalTests.TheSuiteDeclaresTheAllocationRefusalTheRunnerClassifies [< 1 ms]
   Assert.NotNull() Failure: Value is null
Failed!  - Failed:     3, Passed:     1, Skipped:     0, Total:     4, Duration: 4 s - Lovelace.Suite.Tests.dll (net10.0)

  Failed Lovelace.Run.Tests.PlotFileErrorTests.PlotFileUnderAMissingDirectory_IsAnErrorEnvelopeWithExitOne [242 ms]
  Failed Lovelace.Run.Tests.PlotFileErrorTests.EmptyPlotFile_IsAnErrorEnvelopeWithExitOne [164 ms]
  Failed Lovelace.Run.Tests.AllocationRefusalEnvelopeTests.HugeZeros_CrossesAsARecoverableBudgetRefusal [27 s]
  Failed Lovelace.Run.Tests.AllocationRefusalEnvelopeTests.HugeZerosProduct_CrossesAsARecoverableBudgetRefusal [260 ms]
Failed!  - Failed:     4, Passed:     2, Skipped:     0, Total:     6, Duration: 28 s - Lovelace.Run.Tests.dll (net10.0)
```

**16 failed / 10 passed** pre-fix; the 10 passing cases are the success controls and the neighbouring shapes
(`len([1,2,3])`, `len([])`, `len(1..0)`, `len(zeros(2,3))`, `len(zeros(2,3,4))`, `zeros(2,3)`, `zeros(2,0)`,
`zeros(1000000)`, the zero-sized print/shape/index case, and the two process-level success controls).

Two honest notes:

* `Zeros_TenToTheNinth_IsRefusedWithoutAllocating` failed pre-fix with **"Value is null"** — the CoreCLR test host
  *did* allocate the 8 GiB buffer (4 s) for the shape that makes the published AOT binary raise
  `OutOfMemoryException`. The same request is served by one runtime and refused by the other, which is why the
  budget below is a documented constant and not a probe of free memory.
* The first versions of the four "typed refusal" assertions compared `GetType().Name` (the simple name) with the
  full type name; after the product fix they still failed on that test-side mistake, which was corrected to
  `GetType().FullName` before the after-run. No product behaviour depended on it, and no assertion was weakened
  or removed (the type is still compared exactly).

---

## 3. The contract chosen for (3) (requirement 3)

**Chosen: admit zero-sized dimensions along a non-zero axis; do NOT refuse the construction.**
`len` answers the first dimension of the array VALUE, which is the same shape `shape()`, `numel()`, the
indexer and the printer already read.

The contract, and where it is written:

| source | text |
|---|---|
| `Lovelace.Suite\CoreBuiltinMetadata.cs:78` | `Add("len", ["v"], BuiltinCategories.Arrays, "Length of a vector (first dimension of an array).", … "Natural", ["shape"]);` |
| `Lovelace.Suite\docs\Language.md:456-463` | `len([5, 6, 7, 8])` → `4 (Natural)` |
| `docs\architecture\typed-array-migration-plan.md:35` | `| D5 | Zero-length dimensions | **Support** (`zeros(0)`, reshape-to-0, broadcasting edges). | EMP-001 |` |
| `Lovelace.Suite.Tests\EmptyReductionTests.cs:41-57` | the shipped suite already pins D5: `zeros(0)` is an empty vector, `reshape(zeros(0), 2, 0)` is a `[2, 0]` array |
| `Lovelace.Suite\ArrayAllocationBudget.cs` (this round) | the allocation budget counts a zero dimension as an empty array and never refuses it |

The refusal the audit found came from neither of those contracts: it was the positive-dimension rule of the
generic kernel type, `Lovelace.Array\NdArray.cs:33-34`, reached through `Value.AsArray()`
(`Lovelace.Suite\Value.cs:209` → `TypedArrayAdapter.ToNdArray`) while the value itself is an
`ArrayValue`/`DenseArray<Value>`, which supports zero dimensions per D5.

**Rejected alternative:** refusing the CONSTRUCTION everywhere. That would contradict D5 and would have to break
`zeros(0)`, `reshape(zeros(0), 2, 0)`, the empty-reduction identities `sum([])=0`/`prod([])=1`, the
established `matmul(zeros(0,2), zeros(2,0)) = []` behaviour, and the printing/shaping the audit itself measured
working on the published binary — i.e. it would have to make `shape()` and the printer refuse values they
correctly describe today. Nothing in the descriptor, the language document or the migration plan asks for that.

**Successes kept (requirement 3).** `zeros(2,0)` still prints `[[], []] (Array)`, still reports
`shape = [2, 0]`, still indexes as `zeros(2,0)[0] = [] (Vector)`, and `numel(zeros(2,0)) = 0`; `zeros(2,3)`
and `zeros(1000000)` are unchanged; `eye(0,0)` and `zeros(-1)` keep their pre-existing refusals
(§5, after-column).

---

## 4. The diff (product; test files in §5)

`git -C .worktrees\c6-cp1 status --short`: 3 modified product files, 1 new product file, 5 new test files — no other
path is touched (`Lovelace.Symbolics/**` and `Lovelace.Complex/**` are untouched, as the brief requires).

```diff
diff --git a/Lovelace.Run/Runner.cs b/Lovelace.Run/Runner.cs
@@ -314,6 +314,19 @@ public static class Runner
         Lovelace.Abstractions.InputDepthExceededException => ("DepthExceeded", "BudgetExceeded", true),
+        // A single request larger than the engine's allocation budget is refused BEFORE it is attempted
+        // (Lovelace.Suite.ArrayAllocationBudget), so it is the same kind of outcome as a depth refusal: a
+        // budget stop with its own code, recoverable, and never the internal-invariant class the
+        // OutOfMemoryException it used to be reached (F2-C).
+        Lovelace.Suite.ArrayAllocationRefusedException => ("AllocationRefused", "BudgetExceeded", true),
+        // the backstop for every allocating site the budget above does not cover (a range, a kernel, a
+        // string builder): an allocation the process could not serve is still a resource budget the caller
+        // can retry smaller — never "do not retry" about a merely-too-large request
+        OutOfMemoryException => ("AllocationRefused", "BudgetExceeded", true),
+        // An unwritable plot output path is a property of the caller's --plot-file (or of the engine's
+        // default name under the caller's --plot-dir), exactly like the plot DIRECTORY preflight above, so
+        // it crosses with its own code instead of the internal-invariant class (F3-C).
+        Lovelace.Suite.PlotFileWriteException => ("PlotFileError", "TypeMismatch", true),
         Lovelace.Symbolics.EvaluationException => ("EvaluationError", "DomainError", true),

diff --git a/Lovelace.Suite/EngineExceptions.cs b/Lovelace.Suite/EngineExceptions.cs
@@ -17,6 +17,30 @@
+/// <summary>
+/// A plot output path the process cannot write: … an unwritable combination is a property of the
+/// ARGUMENT, like an unreadable script file — never an internal invariant failure (F3-C …).
+/// </summary>
+public sealed class PlotFileWriteException : Exception
+{
+    /// <summary>The path the caller asked for (the plot directory joined with the plot file name).</summary>
+    public string Path { get; }
+
+    public PlotFileWriteException(string path, Exception cause)
+        : base($"Cannot write the plot file '{path}': {cause.Message}", cause)
+    {
+        Path = path;
+    }
+}
+
 /// <summary>
 /// An evaluation stopped by the caller's <see cref="CancellationToken"/>. …

diff --git a/Lovelace.Suite/Interpreter.cs b/Lovelace.Suite/Interpreter.cs
@@ len(v) / len(array) @@
                 ValueKind.Vector => Task.FromResult<Value>(new Value(new Nat(arg.AsVector().Count))),
-                ValueKind.Array  => Task.FromResult<Value>(Natural(arg.AsArray().Shape[0])),
+                // The FIRST DIMENSION of the value's own shape, which is what the descriptor promises
+                // ("Length of a vector (first dimension of an array)", CoreBuiltinMetadata.cs:78). It used
+                // to read AsArray(), whose NdArray<Value> conversion refuses any zero dimension
+                // (Lovelace.Array/NdArray.cs:33-34) … — F4-C.
+                ValueKind.Array  => Task.FromResult<Value>(Natural(arg.AsArrayValue().Shape.Span[0])),

@@ plot write @@
         string path = Path.Combine(PlotOutputDirectory, PlotFileName);
-        string full = Path.GetFullPath(path);
         string svg = new SvgPlotRenderer().Render(model);
-        File.WriteAllText(full, svg);
+        string full;
+        try
+        {
+            full = Path.GetFullPath(path);
+            File.WriteAllText(full, svg);
+        }
+        catch (Exception ex) when (IsUnwritablePlotFile(ex))
+        {
+            // … it crosses as the typed PlotFileWriteException (runner code PlotFileError/TypeMismatch,
+            // recoverable) instead of the raw IOException / UnauthorizedAccessException reaching the
+            // runner's generic handler as an internal invariant failure (F3-C). …
+            throw new PlotFileWriteException(path, ex);
+        }
+
         LastPlot = new PlotCapture(svg, title);
+
+    private static bool IsUnwritablePlotFile(Exception ex) =>
+        ex is ArgumentException or IOException or UnauthorizedAccessException or NotSupportedException;

@@ FillArrayValue @@
-    private static ArrayValue FillArrayValue(long[] shape, Value value)
+    private static ArrayValue FillArrayValue(long[] shape, Value value, string builtin)
     {
+        ArrayAllocationBudget.EnsureServable(shape, builtin);
         long total = 1;

@@ RegisterArrayBuiltins @@
-            Task.FromResult(WrapArrayValue(FillArrayValue(ParseShape(args, 0, "zeros"), NumericOps.Zero))),
+            Task.FromResult(WrapArrayValue(FillArrayValue(ParseShape(args, 0, "zeros"), NumericOps.Zero, "zeros"))),
-            Task.FromResult(WrapArrayValue(FillArrayValue(ParseShape(args, 0, "ones"), NumericOps.One))),
+            Task.FromResult(WrapArrayValue(FillArrayValue(ParseShape(args, 0, "ones"), NumericOps.One, "ones"))),
@@ eye @@
             if (rows < 1 || cols < 1)
                 throw new ArgumentException("eye() dimensions must be positive.");
 
+            // the same pre-allocation budget zeros()/ones() use: eye(10^9, 10^9) is a 10^18-element
+            // request whose product used to overflow an int and cross as an arithmetic error (F2-C)
+            ArrayAllocationBudget.EnsureServable(new[] { rows, cols }, "eye");
+
             long total = rows * cols;
```

(The elided comment bodies are unedited in the tree; the full diff is reproducible with
`git -C .worktrees\c6-cp1 diff`.)

**New file `Lovelace.Suite/ArrayAllocationBudget.cs`** (the whole of fix 1's budget; 128 lines, of which ~60 are the
rationale the project's other budgets carry). Its contract:

* `public const long MaxElements = 1L << 28;` — 268 435 456 elements, i.e. a 2 GiB single buffer of `Value`
  references. A fixed, documented number, for the reason §2 records: the same 10^9-element request is served by
  one runtime (CoreCLR test host, 4 s) and refused by another (the published AOT binary, `OutOfMemoryException`),
  so a machine-derived limit would make the refusal, and every test of it, depend on the host.
* `EnsureServable(shape, builtin)` measures the shape BEFORE the buffer exists and throws
  `ArrayAllocationRefusedException` (a `BudgetExceeded`-category refusal carrying `Shape`, the exact
  `RequestedElements` and the `Limit`). A zero dimension makes the array empty and is never refused (D5); a
  negative dimension is left to the existing path, so `zeros(-1)` keeps the refusal it had.
* `OutOfMemoryException` is classified by the runner as the same recoverable `AllocationRefused` — the backstop
  for allocating sites this budget does not cover.

---

## 5. Before / after, per finding (requirements 1 and 3)

### (1) F2 — a request the machine cannot serve

| probe (published AOT binary → fixed tree) | before | after |
|---|---|---|
| `zeros(1000000000)` | exit 1; stdout 429 B; `InternalError`/`InternalInvariantFailure`; `recoverable:false`; "Insufficient memory to continue the execution of the program."; 46.0 s wall | exit 1; stdout 983 B; `AllocationRefused`/`BudgetExceeded`; `recoverable:true`; "zeros() requested one array of shape [1000000000], which holds 1000000000 element(s) — more than the maximum single-allocation budget of 268435456 element(s). The request is refused BEFORE the allocation is attempted. …"; 126 ms wall (`elapsed` 643.8 µs) |
| `zeros(1000000000,1000000000)` | exit 1; `ArithmeticError`/`DomainError`; "Arithmetic operation resulted in an overflow."; neither the request nor a limit named | exit 1; `AllocationRefused`/`BudgetExceeded`; `recoverable:true`; names shape, 1000000000000000000 elements and the limit |
| `zeros(2,3)` / `zeros(1000000)` / `zeros(2,0)` | exit 0, unchanged | exit 0, identical envelopes (and in-process `shape [2,3]`, `Numel 10^6`, `shape [2,0]`) |
| `zeros(-1)`, `eye(0,0)` | exit 1; `ArithmeticError`/`DomainError`; `InvalidArgument`/`TypeMismatch` | **unchanged** — the budget deliberately does not answer for negative dimensions, and `eye`'s positive-dimension rule stands |

Test evidence: `Lovelace.Suite.Tests/ArrayAllocationRefusalTests.cs` (4 cases) and
`Lovelace.Run.Tests/AllocationRefusalEnvelopeTests.cs` (3 cases). The refusal is asserted **without performing the
allocation**: the process-level cases only launch the real runner and read its envelope, and the in-process case
measures the process-wide allocated bytes across the call and requires them to stay under 512 MB for a request that
alone needs 8 GiB (the request is refused by the budget check, so nothing multi-gigabyte is ever attempted).

### (2) F3 — a caller-supplied output path that cannot be written

| probe | before | after |
|---|---|---|
| `plot([1,2,3])` + `--plot-dir <tmp> --plot-file nope/x.svg` | exit 1; `InternalError`/`InternalInvariantFailure`; `recoverable:false`; "Could not find a part of the path '…\nope\x.svg'." | exit 1; `PlotFileError`/`TypeMismatch`; `recoverable:true`; "Cannot write the plot file '…\nope/x.svg': Could not find a part of the path '…\nope\x.svg'." |
| `--plot-file ""` (process API; PowerShell 5.1 drops an empty argument, so this row is not reproducible from pwsh — audit C, `docs/goal-cycle-6/round-13/audit-C-hostile.md:51-52`) | **recorded by the audit** (I did not re-observe it from this shell): exit 1; `InternalError`/`InternalInvariantFailure`; "Access to the path '…' is denied." | exit 1; `PlotFileError`/`TypeMismatch`; `recoverable:true` (observed by the `ArgumentList` process test, which the control run proves fails at HEAD) |
| `--plot-file ok.svg` | exit 0; SVG written; result `Text` names the absolute path; envelope `plot.path` | exit 0; **unchanged** (process test asserts exit 0, the file, and `plot.path`) |
| `--file C:\definitely\missing\x.ls` (read side) | exit 1; `FileReadError`/`ParseError`; `recoverable:true` | unchanged |

Test evidence: `Lovelace.Suite.Tests/PlotFileWriteRefusalTests.cs` (3 cases) and
`Lovelace.Run.Tests/PlotFileErrorTests.cs` (3 cases). The refusing filter is deliberately narrow
(`ArgumentException or IOException or UnauthorizedAccessException or NotSupportedException`, the same set the
runner's plot-directory preflight uses) and never swallows the diagnostic: the OS sentence is kept in the refusal's
message.

### (3) F4 — `len` of a zero-dimension array

| probe | before | after |
|---|---|---|
| `len([[],[]])` | exit 1; `InvalidArgument`/`TypeMismatch`; "Array dimensions must be positive, but got 0. (Parameter 'shape')" | exit 0; `2 (Natural)` |
| `len([[]])` | same refusal | `1 (Natural)` |
| `len(zeros(2,0))` | same refusal | `2 (Natural)` |
| `len(zeros(1,0))` | same refusal | `1 (Natural)` |
| `len(zeros(0,3))` / `len(zeros(0,0))` | same refusal | `0 (Natural)` |
| `len(zeros(2,3))` / `len(zeros(2,3,4))` / `len([1,2,3])` / `len([])` / `len(1..0)` | correct already | **unchanged** (`2`, `2`, `3`, `0`, `0`) |
| `[[],[]]`, `shape([[],[]])`, `zeros(2,0)[0]`, `numel(zeros(2,0))`, `rank(zeros(2,0))` | `[[], []]`, `[2, 0]`, `[]`, `0`, `2` | **unchanged** (pinned by `ZeroSizedArrays_StillPrintShapeAndIndexAsBefore`) |

Test evidence: `Lovelace.Suite.Tests/LenZeroDimensionTests.cs` (11 theory cases + 1 regression case).

---

## 6. The same probes on a fresh Native AOT publish of the fixed tree

The findings were filed against a Native AOT binary, so the after-state was ALSO measured on one:

```powershell
dotnet publish Lovelace.Run\Lovelace.Run.csproj --configuration Release -p:PublishAot=true -p:InvariantGlobalization=true -o out\aot
# → AOT_EXIT=0, out\aot\Lovelace.Run.exe, 5 800 448 bytes, 2026-09-12 16:36:02
```

| probe | exit | bytes | code / category | recoverable | wall |
|---|---|---|---|---|---|
| `zeros(1000000000)` | 1 | 983 | `AllocationRefused` / `BudgetExceeded` | true | 126 ms (envelope `elapsed` 643.8 µs) |
| `zeros(1000000000,1000000000)` | 1 | 1 025 | `AllocationRefused` / `BudgetExceeded` | true | 40 ms |
| `plot([1,2,3])` `--plot-file nope/x.svg` | 1 | 863 | `PlotFileError` / `TypeMismatch` | true | 36 ms |
| `plot([1,2,3])` `--plot-file ok.svg` | 0 | 16 992 | success, `plot.path` set | — | 42 ms |
| `len([[],[]])` | 0 | 455 | `2 (Natural)` | — | 40 ms |
| `len(zeros(2,0))` | 0 | 454 | `2 (Natural)` | — | 38 ms |
| `len(zeros(0,3))` | 0 | 454 | `0 (Natural)` | — | 37 ms |
| `shape([[],[]])` | 0 | 564 | `[2, 0] (Vector)` | — | 37 ms |
| `zeros(2,0)` | 0 | 478 | `[[], []] (Array)`, `shape:[2,0]` | — | 41 ms |

The pre-fix F2 run against the *same class* of binary took 46.0 s and answered with the CLR's OOM text; the fixed
binary answers from the budget check in 126 ms with a recoverable, typed code. The main checkout's
`out\aot\Lovelace.Run.exe` was NOT republished (the published artifact of `main` is still the pre-fix binary).

---

## 7. Control (requirement 5)

```powershell
git -C C:\Users\ricar\dev\LovelaceSharp worktree add .worktrees/c6-cp1ctl HEAD   # detached HEAD 54b1753
```

Only the five NEW test files were copied in — no product file, no other test:

| copied test file | SHA-256 |
|---|---|
| `Lovelace.Suite.Tests/LenZeroDimensionTests.cs` | `9AEE2BD9817E4827BA17B733F0EC50AE236EA3FFB5C747D472FA4F27ECB83ECA` |
| `Lovelace.Suite.Tests/ArrayAllocationRefusalTests.cs` | `C0802A30CF0A6A9F8EF118B17FB3B74B8149FB2C9557FFEDC6530AB8862F2672` |
| `Lovelace.Suite.Tests/PlotFileWriteRefusalTests.cs` | `C6BD4095F335CBAB230E27F02F113DEB7DE6D85D2B4AFABFA8E9B3CA67804B3C` |
| `Lovelace.Run.Tests/AllocationRefusalEnvelopeTests.cs` | `73DA582FE2A843D28CCFE0A45A4138628CD5E9205D9FF8AB1A7480E308AF4A36` |
| `Lovelace.Run.Tests/PlotFileErrorTests.cs` | `E981B61D7FBA2526E9B6E8C4A2FEEA706463278F8FD98A7D1F84E2F669C6B4EF` |

`git -C .worktrees\c6-cp1ctl status --short` lists those five files and nothing else. Result
(`control.log`), the same filters as §2:

```text
dotnet test Lovelace.Suite.Tests\Lovelace.Suite.Tests.csproj … --filter "FullyQualifiedName~LenZeroDimensionTests|FullyQualifiedName~ArrayAllocationRefusalTests|FullyQualifiedName~PlotFileWriteRefusalTests"
Failed!  - Failed:    12, Passed:     8, Skipped:     0, Total:    20, Duration: 5 s - Lovelace.Suite.Tests.dll (net10.0)   [CONTROL_SUITE_EXIT=1]

dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj … --filter "FullyQualifiedName~PlotFileErrorTests|FullyQualifiedName~AllocationRefusalEnvelopeTests"
Failed!  - Failed:     4, Passed:     2, Skipped:     0, Total:     6, Duration: 23 s - Lovelace.Run.Tests.dll (net10.0)   [CONTROL_RUN_EXIT=1]
```

**16 failed / 10 passed** — the identical failure sets and the identical counts as the failing-first run in §2,
including the 27 s `InternalError` answer of the HEAD runner to `zeros(1000000000)` and the four
`Assert.Equal() Failure: Strings differ` on the code assertions. The tests are therefore sensitive to exactly
the behaviour this round changed, and the 10 passing cases are the ones this round promises not to break.

---

## 8. Suite totals (requirement 4)

```powershell
# in .worktrees\c6-cp1, with LOVELACE_REQUIRE_SYMPY=1 and the oracle on PATH
dotnet test LovelaceSharp.slnx --configuration Release --nologo        # exit 0
```

| project | passed | failed | skipped |
|---|---|---|---|
| Lovelace.Abstractions.Tests | 20 | 0 | 0 |
| Lovelace.Array.Tests | 19 | 0 | 0 |
| Lovelace.Representation.Tests | 91 | 0 | 0 |
| Lovelace.Natural.Tests | 195 | 0 | 0 |
| Lovelace.Integer.Tests | 148 | 0 | 0 |
| precbench.Tests | 13 | 0 | 0 |
| Lovelace.Knowledge.Tests | 28 | 0 | 0 |
| Lovelace.Console.Tests | 15 | 0 | 0 |
| Lovelace.Complex.Tests | 115 | 0 | 0 |
| Lovelace.Real.Tests | 2495 | 0 | 0 |
| Lovelace.Dsp.Tests | 61 | 0 | 0 |
| Lovelace.Suite.Tests | 848 | 0 | 0 |
| Lovelace.Run.Tests | 253 | 0 | 0 |
| Lovelace.Studio.Tests | 22 | 0 | 0 |
| Lovelace.Symbolics.Tests | 1148 | 0 | 0 |
| **total** | **5471** | **0** | **0** |

The 26 new tests are inside those numbers (Suite 848 includes the 20 new Suite cases, Run 253 the 6 new Run cases).
Log: `full-suite.log`.

Timing subsets (`timing-subsets.log`; `--no-build`):

| subset | command filter | passed | failed |
|---|---|---|---|
| Suite Timing | `Lovelace.Suite.Tests --filter "Category=Timing"` | 8 | 0 |
| Run Timing | `Lovelace.Run.Tests --filter "Category=Timing"` | 5 | 0 |
| Real Timing | `Lovelace.Real.Tests --filter "Category=Timing"` | 1 | 0 |
| Symbolics Timing | `Lovelace.Symbolics.Tests --filter "Category=Timing"` | 1 | 0 |
| Symbolics Costly | `Lovelace.Symbolics.Tests --filter "Category=Costly"` | 32 | 0 |

---

## 9. Could not verify

| item | why |
|---|---|
| That the `OutOfMemoryException` backstop branch itself works | no test triggers a genuine CLR out-of-memory: the budget refuses every shape reachable from `zeros`/`ones`/`eye` before the allocator runs, and a test that forced a real OOM would be the multi-gigabyte allocation the brief forbids. The branch is asserted only by construction (`Lovelace.Run/Runner.cs`, the `OutOfMemoryException` arm) and by the pre-fix observation that an unclassified OOM crossed as `InternalError`. |
| Every other allocating site | the budget covers the shape-driven constructors (`zeros`, `ones`, `eye`). Ranges (`1..1000000000`), `concat`, broadcasting, `matmul` and string builders are not measured; they are covered only by that same unexercised backstop. |
| A full shape/size lattice for the budget boundary | I measured `zeros(1000000)` (builds) and 10^9 / 10^18 (refused) plus the existing suites' sizes; a binary search for the exact servable/refused boundary on this machine was not run (the constant is documented instead, §4). |
| `--plot-file ""` from PowerShell 5.1 | the shell drops an empty argument to a native command (the process then exits 2, "requires a name argument"); the empty-name case is covered by the `.ArgumentList` process test only. |
| `--text` mode on these failures | not re-measured this round (round 15 already recorded that `--text` leaves stdout empty for the plot-directory failure). |
| Whether a zero-dimension value of every rank/axis position is admitted by `len` | swept rank 1–3 and the `[1,0]`, `[2,0]`, `[0,0]`, `[0,3]` lattice plus the literal forms; not a generated sweep. |
| Eager evaluation at 10^9 elements on a machine with less memory than this one | the refusal is a documented constant, so its verdict does not depend on the host, but I measured only this host. |


