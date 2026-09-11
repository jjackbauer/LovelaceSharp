# Round 6 — MatrixInverseResult and MatrixSolveResult join the solve vocabulary

One bounded change: the two matrix result records now speak the same status / complete /
completeness / Diagnostic vocabulary as every other solve-shaped record, and the schema registry
plus its drift corpus can see them.

Labels: **observed** = the command was run in this workspace and its output is pasted below;
**derived** = arithmetic or reading of the cited source, not a separate execution.

---

## 1. Step A — the failing gate (observed before the change)

New assertions were written first, in the in-scope test file
`Lovelace.Suite.Tests/RecordSchemaTests.cs`:

* corpus rows driving `inv_full` and `linsolve_full` (singular **and** solved), lines 65-68;
* corpus coverage assertions `Assert.Contains("MatrixInverseResult", seen)` /
  `... "MatrixSolveResult" ...`, lines 106-107;
* declared-kind assertions for the two records, lines 186-191;
* two new theories, lines 223-335:
  `SingularMatrix_IsACompleteNoSolutions_CarryingAStructuredDiagnostic` and
  `SolvedMatrix_IsComplete_WithAnEmptyDiagnosticsArray`.

### Run 1 — before any source change

```
> dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~RecordSchemaTests"

  Failed Lovelace.Suite.Tests.RecordSchemaTests.SolvedMatrix_IsComplete_WithAnEmptyDiagnosticsArray(call: "inv_full([[x, 1], [0, y]])", typeName: "MatrixInverseResult", payloadField: "inverse") [41 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: Enum
Actual:   Text
     at Lovelace.Suite.Tests.RecordSchemaTests.SolvedMatrix_IsComplete_WithAnEmptyDiagnosticsArray(...) in RecordSchemaTests.cs:line 300
  Failed Lovelace.Suite.Tests.RecordSchemaTests.SolvedMatrix_IsComplete_WithAnEmptyDiagnosticsArray(call: "linsolve_full([[x, 1], [0, y]], [0, 1])", ...) [2 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: Enum
Actual:   Text
  Failed Lovelace.Suite.Tests.RecordSchemaTests.SolverSchemas_DeclareTheStructuralKinds [6 ms]
  Error Message:
   Assert.NotNull() Failure: Value is null
     at Lovelace.Suite.Tests.RecordSchemaTests.AssertSchema(String typeName, ValueTuple`2[] expected) in RecordSchemaTests.cs:line 325
     at Lovelace.Suite.Tests.RecordSchemaTests.SolverSchemas_DeclareTheStructuralKinds() in RecordSchemaTests.cs:line 186
  Failed Lovelace.Suite.Tests.RecordSchemaTests.SingularMatrix_IsACompleteNoSolutions_CarryingAStructuredDiagnostic(call: "inv_full([[x, x], [x, x]])", ...) [< 1 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: Enum
Actual:   Text
  Failed Lovelace.Suite.Tests.RecordSchemaTests.SingularMatrix_IsACompleteNoSolutions_CarryingAStructuredDiagnostic(call: "linsolve_full([[x, x], [x, x]], [1, 1])", ...) [< 1 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: Enum
Actual:   Text

Failed!  - Failed:     5, Passed:     4, Skipped:     0, Total:     9, Duration: 135 ms - Lovelace.Suite.Tests.dll (net10.0)
```

The failure mode is exactly the defect: **status is `Text` where every other solve record
publishes an `Enum`**, and **`RecordSchemas.For` returns null** for both record types.

### Run 2 — after making the two "zero violations" assertions non-vacuous

`RecordSchemas.Validate` returns an empty list for an **unknown** type, so
"validates with zero violations" passed vacuously before the registry existed. Each new test now
also asserts `Assert.NotNull(RecordSchemas.For(record.TypeName))` first:

```
> dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~RecordSchemaTests"

  Failed Lovelace.Suite.Tests.RecordSchemaTests.SolvedMatrix_IsComplete_WithAnEmptyDiagnosticsArray(call: "inv_full([[x, 1], [0, y]])", ...) [51 ms]
  Error Message:
   Assert.NotNull() Failure: Value is null
  Failed Lovelace.Suite.Tests.RecordSchemaTests.SolvedMatrix_IsComplete_WithAnEmptyDiagnosticsArray(call: "linsolve_full([[x, 1], [0, y]], [0, 1])", ...) [2 ms]
  Error Message:
   Assert.NotNull() Failure: Value is null
  Failed Lovelace.Suite.Tests.RecordSchemaTests.SolverSchemas_DeclareTheStructuralKinds [8 ms]
  Error Message:
   Assert.NotNull() Failure: Value is null
  Failed Lovelace.Suite.Tests.RecordSchemaTests.SingularMatrix_IsACompleteNoSolutions_CarryingAStructuredDiagnostic(call: "inv_full([[x, x], [x, x]])", ...) [< 1 ms]
  Error Message:
   Assert.NotNull() Failure: Value is null
  Failed Lovelace.Suite.Tests.RecordSchemaTests.SingularMatrix_IsACompleteNoSolutions_CarryingAStructuredDiagnostic(call: "linsolve_full([[x, x], [x, x]], [1, 1])", ...) [< 1 ms]
  Error Message:
   Assert.NotNull() Failure: Value is null

Failed!  - Failed:     5, Passed:     4, Skipped:     0, Total:     9, Duration: 172 ms - Lovelace.Suite.Tests.dll (net10.0)
```

**Honest note (observed):** the drift test `EveryProducedRecord_ConformsToItsDeclaredSchema` is one
of the 4 that PASSED before the change, even with the new corpus rows — an unregistered record is
invisible to `Validate`. That is precisely the drift this round closes; the failing half of the
gate for the registry is `SolverSchemas_DeclareTheStructuralKinds` (schema null) and the new
`Assert.NotNull(RecordSchemas.For(...))` guards above.

---

## 2. Source change (Step B)

| File:line | Change |
|---|---|
| `Lovelace.Suite/Interpreter.cs:7` | `using SolveStatus = Lovelace.Symbolics.SolveStatus;` — the solver's status enum crosses the seam by its declared type name. |
| `Lovelace.Suite/Interpreter.cs:8-10` | `using WireDiagnostic = Lovelace.Abstractions.Diagnostic;` — required because `Lovelace.Suite.Diagnostic` (`Lovelace.Suite/Diagnostics.cs:7`) is the engine's own record and shadows the Round-4b wire type inside this namespace. |
| `Lovelace.Suite/Interpreter.cs:1025-1037` | `MatrixResultRecord(...)` — the ONE builder for both records. Field order is exactly `status, complete, completeness, <payload>, conditions, diagnostics`; `complete` and `completeness` both come from `SolveCompletenessMapping.Of(status)`, and diagnostics are projected with `DiagnosticProjection.ToRecordValues`. |
| `Lovelace.Suite/Interpreter.cs:1042-1043` | `SingularMatrixDiagnostics(message)` → one `Diagnostic.Of("matrix.singular", ErrorCategory.NoSolution, message)` (recoverable defaults true, location null, details empty). |
| `Lovelace.Suite/Interpreter.cs:1312-1322` | `inv_full`: singular → `SolveStatus.NoSolutions` + the diagnostic; otherwise `SolveStatus.Solved` + `Array.Empty<WireDiagnostic>()`. |
| `Lovelace.Suite/Interpreter.cs:1372-1379` | `linsolve_full`: the kernel `note` becomes the diagnostic MESSAGE; solved → empty diagnostics array. |
| `Lovelace.Suite/RecordSchemas.cs:78-87` | `Schema("MatrixInverseResult", ...)` and `Schema("MatrixSolveResult", ...)` with the emitted names and kinds, `diagnostics` declared `Array`. |
| `Lovelace.Suite.Tests/RecordSchemaTests.cs:65-68, 106-107, 186-191, 223-325` | corpus rows, corpus-coverage assertions, declared-kind assertions, the two new theories. |
| `Lovelace.Run.Tests/GoldenEnvelopeTests.cs:35-36, 126-137` | two `Corpus` rows and two `RichResultFixtures` rows (the latter assert the diagnostics field is LAST and crosses the wire as an Array). |
| `Lovelace.Run.Tests/fixtures/matrix_inverse_result.{ls,json}`, `matrix_solve_result.{ls,json}` | new golden corpus rows. |
| `Lovelace.Symbolics.Tests/DxStructuredResultsTests.cs:379-380, 390-393, 406-407` | the three existing assertions of the OLD text shapes now assert the Enum vocabulary (and conditions are looked up by name, since the field order changed from index 2 to index 4). Assertions were strengthened, never weakened. |

The emitted shape, in full:

```csharp
var (complete, completeness) = Lovelace.Symbolics.SolveCompletenessMapping.Of(status);
return new RecordValue(typeName,
    new RecordField("status", new EnumValue("SolveStatus", status.ToString())),
    new RecordField("complete", complete),
    new RecordField("completeness", new EnumValue("Completeness", completeness.ToString())),
    new RecordField(payloadField, PayloadMap.Wrap(payload)),
    new RecordField("conditions", PayloadMap.Wrap(conditions)),
    new RecordField("diagnostics", DiagnosticProjection.ToRecordValues(diagnostics)));
```

`RecordValue` type names are unchanged (`MatrixInverseResult`, `MatrixSolveResult`) — observed in
the goldens below.

### The "Diagnostic" schema is reused, not duplicated

`Lovelace.Suite/RecordSchemas.cs` still has exactly ONE `Schema("Diagnostic", ...)` entry (lines
106-108, unchanged from Round 4b); no second entry was added. The new tests assert the nested
record's `TypeName == DiagnosticProjection.TypeName` and that it validates against
`RecordSchemas.For("Diagnostic")`; the drift corpus walks into the nested record and validates it
through that same entry.

---

## 3. Where the shared completeness mapping ended up

**It did not move.** The ONE mapping is
`Lovelace.Symbolics.SolveCompletenessMapping` at
`Lovelace.Symbolics/SymbolicsPlugin.cs:25-41`, and it is directly reachable from
`Lovelace.Suite` because `Lovelace.Suite/Lovelace.Suite.csproj:17` already references
`..\Lovelace.Symbolics\Lovelace.Symbolics.csproj`. The interpreter calls it at
`Lovelace.Suite/Interpreter.cs:1029`; no second mapping was written, and the existing call sites
(`SymbolicsPlugin.cs:373`, `:442`) are untouched.

Observed consequence of the reuse: a singular system reports `NoSolutions / complete true /
completeness Complete` exactly like the scalar solver, because
`SolveStatus.NoSolutions => (true, Completeness.Complete)` is the single line at
`SymbolicsPlugin.cs:35`.

---

## 4. Step C — golden corpus rows and the golden diff

New scripts (written with `[System.IO.File]::WriteAllText` + `UTF8Encoding(false)`):

```
Lovelace.Run.Tests/fixtures/matrix_inverse_result.ls : x = symbol("x"); inv_full([[x, x], [x, x]])
Lovelace.Run.Tests/fixtures/matrix_solve_result.ls   : x = symbol("x"); linsolve_full([[x, x], [x, x]], [1, 1])
```

Both are **singular symbolic** matrices. (Observed side note: a purely numeric matrix is rejected —
`out/aot/Lovelace.Run.exe --eval "inv_full([[1, 2], [2, 4]])"` → `ok=False
code=InvalidOperation message=inv_full() requires a symbolic matrix.` — consistent with
`SymbolicsPlugin.cs:803-804`.)

Goldens were generated by capturing the raw envelopes into `out/raw-r6/` and normalising them
(volatile keys → `"<volatile>"`, two-space indent) with `out/raw-r6/normalise-fixtures.js`, the
same rule `TestSupport.VolatileKeys` uses:

```
> node out/raw-r6/normalise-fixtures.js
ADDED   matrix_inverse_result.json (4175 bytes)
ADDED   matrix_solve_result.json (4173 bytes)
```

### Golden diff

Both goldens are NEW files; no pre-existing golden was regenerated:

```
> git diff --no-index --stat NUL Lovelace.Run.Tests/fixtures/matrix_inverse_result.json
 .../fixtures/matrix_inverse_result.json            | 144 +++++++++++++++++++++
 1 file changed, 144 insertions(+)

> git diff --no-index --stat NUL Lovelace.Run.Tests/fixtures/matrix_solve_result.json
 .../fixtures/matrix_solve_result.json              | 144 +++++++++++++++++++++
 1 file changed, 144 insertions(+)

> Get-ChildItem Lovelace.Run.Tests/fixtures/*.json | Sort-Object LastWriteTime -Descending | Select -First 6
Name                       LastWriteTime
----                       -------------
matrix_solve_result.json   9/11/2026 12:57:47 AM   <- written by this round
matrix_inverse_result.json 9/11/2026 12:57:47 AM   <- written by this round
record.json                9/11/2026 12:42:47 AM
print_purity.json          9/11/2026 12:42:47 AM
optimization_result.json   9/11/2026 12:42:47 AM
system_solve.json          9/11/2026 12:42:47 AM
```

The added envelope content (every changed value, and why it is intended):

```json
"result": {
  "kind": "Record",
  "display": "MatrixInverseResult(status: NoSolutions, complete: True, completeness: Complete, inverse: [], conditions: [], diagnostics: [Diagnostic(code: matrix.singular, category: NoSolution, message: matrix is singular, recoverable: True, location: , details: [])])",
  "structured": {
    "kind": "Record",
    "type": "MatrixInverseResult",
    "fields": [
      { "name": "status",       "value": { "kind": "Enum", "type": "SolveStatus",  "value": "NoSolutions" } },
      { "name": "complete",     "value": { "kind": "Boolean",                          "value": "true" } },
      { "name": "completeness", "value": { "kind": "Enum", "type": "Completeness", "value": "Complete" } },
      { "name": "inverse",      "value": { "kind": "Array", "type": "Vector", "shape": [0], "elements": [] } },
      { "name": "conditions",   "value": { "kind": "Array", "type": "Vector", "shape": [0], "elements": [] } },
      { "name": "diagnostics",  "value": { "kind": "Array", "type": "Vector", "shape": [1], "elements": [
          { "kind": "Record", "type": "Diagnostic", "fields": [
            { "name": "code",        "value": { "kind": "Text",    "value": "matrix.singular" } },
            { "name": "category",    "value": { "kind": "Enum",    "type": "ErrorCategory", "value": "NoSolution" } },
            { "name": "message",     "value": { "kind": "Text",    "value": "matrix is singular" } },
            { "name": "recoverable", "value": { "kind": "Boolean", "value": "true" } },
            { "name": "location",    "value": { "kind": "Null" } },
            { "name": "details",     "value": { "kind": "Array", "type": "Vector", "shape": [0], "elements": [] } } ] } ] } ] } }
```

```json
"result": {
  "kind": "Record",
  "display": "MatrixSolveResult(status: NoSolutions, complete: True, completeness: Complete, solutions: [], conditions: [], diagnostics: [Diagnostic(code: matrix.singular, category: NoSolution, message: matrix is singular, recoverable: True, location: , details: [])])",
  "structured": {
    "kind": "Record",
    "type": "MatrixSolveResult",
    "fields": [
      { "name": "status",       "value": { "kind": "Enum", "type": "SolveStatus",  "value": "NoSolutions" } },
      { "name": "complete",     "value": { "kind": "Boolean",                          "value": "true" } },
      { "name": "completeness", "value": { "kind": "Enum", "type": "Completeness", "value": "Complete" } },
      { "name": "solutions",    "value": { "kind": "Array", "type": "Vector", "shape": [0], "elements": [] } },
      { "name": "conditions",   "value": { "kind": "Array", "type": "Vector", "shape": [0], "elements": [] } },
      { "name": "diagnostics",  "value": { "kind": "Array", "type": "Vector", "shape": [1], "elements": [
          { "kind": "Record", "type": "Diagnostic", "fields": [
            { "name": "code",        "value": { "kind": "Text",    "value": "matrix.singular" } },
            { "name": "category",    "value": { "kind": "Enum",    "type": "ErrorCategory", "value": "NoSolution" } },
            { "name": "message",     "value": { "kind": "Text",    "value": "matrix is singular" } },
            { "name": "recoverable", "value": { "kind": "Boolean", "value": "true" } },
            { "name": "location",    "value": { "kind": "Null" } },
            { "name": "details",     "value": { "kind": "Array", "type": "Vector", "shape": [0], "elements": [] } } ] } ] } ] } }
```

Every value is intended: `status` moves Text→Enum `SolveStatus`, `complete`/`completeness` are
new fields derived from the Round-5 mapping, `inverse`/`solutions` and `conditions` keep their
payloads, and `diagnostics` moves `"matrix is singular"` (a string) → an Array of one Diagnostic
whose message carries that sentence. The rest of each golden (header, `output`, `variables`,
`functions`, volatile timing keys) is the standard envelope.

---

## 5. Step C — passing output (observed)

```
> dotnet build LovelaceSharp.slnx -c Release --nologo
Build succeeded.
    0 Warning(s)
    0 Error(s)
Time Elapsed 00:00:02.25
```

```
> dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:    38, Skipped:     0, Total:    38, Duration: 396 ms - Lovelace.Run.Tests.dll (net10.0)
> dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   633, Skipped:     0, Total:   633, Duration: 8 s - Lovelace.Suite.Tests.dll (net10.0)
> dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   436, Skipped:     0, Total:   436, Duration: 13 s - Lovelace.Symbolics.Tests.dll (net10.0)
> dotnet test Lovelace.Studio.Tests/Lovelace.Studio.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:    22, Skipped:     0, Total:    22, Duration: 1 s - Lovelace.Studio.Tests.dll (net10.0)
```

The focused gate after the change:

```
> dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~RecordSchemaTests"
Passed!  - Failed:     0, Passed:     9, Skipped:     0, Total:     9, Duration: 128 ms - Lovelace.Suite.Tests.dll (net10.0)
```

"All suites 0 failed" — the whole solution, not only the four required projects:

```
> dotnet test LovelaceSharp.slnx -c Release --nologo
Passed!  - Failed:     0, Passed:    28, Skipped:     0, Total:    28 - Lovelace.Knowledge.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:    91, Skipped:     0, Total:    91 - Lovelace.Representation.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:    20, Skipped:     0, Total:    20 - Lovelace.Abstractions.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:    19, Skipped:     0, Total:    19 - Lovelace.Array.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:   195, Skipped:     0, Total:   195 - Lovelace.Natural.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:   148, Skipped:     0, Total:   148 - Lovelace.Integer.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:    13, Skipped:     0, Total:    13 - precbench.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:   285, Skipped:     0, Total:   285 - Lovelace.Real.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:    15, Skipped:     0, Total:    15 - Lovelace.Console.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:    38, Skipped:     0, Total:    38 - Lovelace.Run.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:    83, Skipped:     0, Total:    83 - Lovelace.Complex.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:    22, Skipped:     0, Total:    22 - Lovelace.Studio.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:   633, Skipped:     0, Total:   633 - Lovelace.Suite.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:    61, Skipped:     0, Total:    61 - Lovelace.Dsp.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:   436, Skipped:     0, Total:   436 - Lovelace.Symbolics.Tests.dll (net10.0)
```

### New test counts

| Suite | Now | This round |
|---|---|---|
| `RecordSchemaTests` (filtered) | 9 | 5 pre-existing + **4 new** cases (2 theories × 2 rows) — derived from the filtered run |
| `Lovelace.Suite.Tests` | 633 | 629 before (633 − 4) — derived |
| `Lovelace.Run.Tests` | 38 | 34 before + **2** corpus rows + **2** rich-result rows — derived from the two TheoryData lists |
| `Lovelace.Symbolics.Tests` | 436 | count unchanged; 3 existing assertions strengthened (`DxStructuredResultsTests.cs:379-380, 390-393, 406-407`) |
| `Lovelace.Studio.Tests` | 22 | unchanged |

---

## 6. Publish output (observed)

```
> dotnet publish Lovelace.Run/Lovelace.Run.csproj -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o out/aot
  Lovelace.Run -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Run\bin\Release\net10.0\win-x64\Lovelace.Run.dll
  Generating native code
  Lovelace.Run -> C:\Users\ricar\dev\LovelaceSharp\out\aot\
```

Warnings emitted are all pre-existing and in files this round did not touch
(`Lovelace.Real/Real.cs`: CS0109 ×5, CS8767 ×4, CS8600 ×7); the ordinary Release solution build
reports 0 warnings. `out/aot/Lovelace.Run.exe` (5,632,512 bytes) was produced.

---

## 7. Published-binary excerpts (observed)

```
> out/aot/Lovelace.Run.exe --file Lovelace.Run.Tests/fixtures/matrix_inverse_result.ls --omit-functions
== inv_full ==
status       : kind=Enum type=SolveStatus value=NoSolutions
complete     : kind=Boolean value=true
completeness : kind=Enum type=Completeness value=Complete
diagnostics  : kind=Array count=1
  diagnostic : type=Diagnostic code=matrix.singular category=ErrorCategory.NoSolution message='matrix is singular' recoverable=true location=Null details=Array/0

> out/aot/Lovelace.Run.exe --file Lovelace.Run.Tests/fixtures/matrix_solve_result.ls --omit-functions
== linsolve_full ==
status       : kind=Enum type=SolveStatus value=NoSolutions
complete     : kind=Boolean value=true
completeness : kind=Enum type=Completeness value=Complete
diagnostics  : kind=Array count=1
  diagnostic : type=Diagnostic code=matrix.singular category=ErrorCategory.NoSolution message='matrix is singular' recoverable=true location=Null details=Array/0
```

(Probe script: `out/raw-r6/aot-probe.ps1`; it parses the AOT binary's JSON envelope and prints the
four fields.)

---

## 8. What I could NOT make pass / open notes

* **Nothing failed.** All 15 test projects end `Failed: 0`; the focused schema class ends 9/9.
* Two pre-existing warnings remain outside this round's scope and untouched files:
  `Lovelace.Suite.Tests/EmptyReductionTests.cs(47,9)` xUnit2013 (appears during test builds, not in
  the solution build), and the `Lovelace.Real` CS0109/CS8767/CS8600 set during the AOT publish.
* `docs/symbolics/dsh-protocol.md` was **not** edited: observed
  `grep -E "Matrix|inv_full|linsolve" docs/symbolics/dsh-protocol.md` → no matches, so it names
  neither record nor either builtin.
* **Not changed on purpose:** the kernel-level C# record `Lovelace.Symbolics.MatrixSolveResult`
  (`Lovelace.Symbolics/Matrices/SymbolicMatrix.cs:427`) and its `Note` field stay as they are —
  `MatrixAdvanceTests.cs:62` still asserts the kernel note `"matrix is singular"`, which is now the
  SOURCE of the wire diagnostic's message rather than the wire text itself.
* `RecordSchemas.Validate` still checks only field NAMES and the record type; declared KINDS are
  pinned separately by `SolverSchemas_DeclareTheStructuralKinds`. That is unchanged behaviour, noted
  so the kind assertions in the new tests are read as the guard they are.
* Round-4b's `docs/goal-cycle-3/round-4b/verify.ps1` drives `inv_full([[1, 2], [2, 4]])`; a numeric
  matrix never reaches the bridge (observed error above), so that older script cannot be replayed
  as-is. Out of scope here; flagged for the round record only.
