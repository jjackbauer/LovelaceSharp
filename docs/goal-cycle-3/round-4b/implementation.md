# Round 4b — the structured Diagnostic vocabulary on all seven rich records

Objective (one bounded change): introduce `Diagnostic` / `ErrorCategory` in
`Lovelace.Abstractions` and put a structured `diagnostics` **array** on the seven rich result
records, removing every free-text `diagnostics` value from the machine API (alignment plan
section C; addendum decision D1).

Status: **complete**. Every required test fails before the change and passes after it; all five
suites end `0 failed`; the Native AOT published binary carries the array.

---

## 1. Source change summary

| File | Change |
|---|---|
| `Lovelace.Abstractions/Diagnostic.cs` (new, 118 lines) | `enum ErrorCategory` (exactly the seven members), `record DiagnosticLocation(StartLine, StartColumn, EndLine, EndColumn)`, `record Diagnostic(Code, Category, Message, Recoverable, Location, Details)` with the kernel factory `Diagnostic.Of(...)` (Location = null, Details = empty list), and `DiagnosticProjection` — the one wire projection: `ToRecordValue(Diagnostic)` builds a `RecordValue` with WIRE TYPE NAME `Diagnostic` and fields `code, category, message, recoverable, location, details` in that order; `category` is an `EnumValue` of type `ErrorCategory`; `location` is `null` (wire `Null`) when absent; `details` is an Array (empty array, never null, never ""). `Lovelace.Suite/Diagnostics.cs` is untouched. |
| `Lovelace.Symbolics/SymbolicsPlugin.cs` `220, 255, 308, 347, 420, 490` | Every rich emission now passes a diagnostic list through one helper `Diagnostics(params Diagnostic?[])` (line 501): `IntegrationResult` (220), `TransformResult` (255, appended **last** after `budget_kind`), `LimitResult` (308), `SystemSolveResult` (347), `SolveResult` (420), `OptimizationResult` (490, appended last after `policy`). Helpers added at 494-573: `SolveDiagnostic`/`SolveDiagnosticCode`/`SolveDiagnosticCategory`, `SystemSolveDiagnostic`, `LimitDiagnostic`, `IntegrationDiagnostic`, `TransformDiagnostic`. No emission writes `r.Note ?? ""` / `r.FailureReason ?? ""` into a field any more. |
| `Lovelace.MathIR/Evaluator.cs` `415-421` | `CompilationResult` gains the last field `diagnostics`, built through the same `DiagnosticProjection.ToRecordValues(Array.Empty<Diagnostic>())` so the empty array shape is shared, not re-spelled. |
| `Lovelace.Suite/RecordSchemas.cs` `69, 77, 84, 91, 94, 98, 102, 103-108` | `diagnostics` is `Array` on all seven records (SolveResult/SystemSolveResult/LimitResult/IntegrationResult changed from `Text`; TransformResult/OptimizationResult/CompilationResult gained the field, last); a `Diagnostic` schema is registered with exactly `code:Text, category:Enum, message:Text, recoverable:Boolean, location:Record|Null, details:Array`. |
| `Lovelace.Symbolics.Tests/DiagnosticContractTests.cs` (new) | The 14 required contract tests (see section 6). |
| `Lovelace.Suite.Tests/RecordSchemaTests.cs` | The three old `("diagnostics","Text")` expectations corrected, `TransformResult`/`OptimizationResult`/`CompilationResult` added to the schema test, a new `Diagnostic`-schema fact, and the produced-record corpus extended with a partial solve + a refuted transformation plus `Assert.Contains("Diagnostic", seen)`. |
| `Lovelace.Run.Tests/fixtures/{integration_result,optimization_result}.ls` + `.json` (new) | The two missing rich records are now pinned on the wire. |
| `Lovelace.Run.Tests/GoldenEnvelopeTests.cs` | Two corpus rows and two new wire tests: `RichResultDiagnostics_CrossTheWireAsAnArray` (7 cases) and `WireDiagnostic_CarriesTheSixFrozenFieldsInOrder`. |
| `docs/symbolics/dsh-protocol.md` | New `### The Diagnostic form` section (six fields in order, enum-valued category, Null location, Array details, empty-array rule, D1 note that the kernel note is now a message); `ErrorCategory` added to the enum-valued list; the abridged solve envelope example now shows `diagnostics` as an Array; the error-envelope category list corrected to the seven-member taxonomy and its `diagnostics` array distinguished from the Diagnostic record. |

No other file was edited. `.github/workflows/ci.yml` is **unchanged by this round** (see section 7).

---

## 2. Step A — the new assertions, before the change (OBSERVED)

### 2.1 `dotnet test Lovelace.Symbolics.Tests --filter "FullyQualifiedName~DiagnosticContractTests"`

    Failed Lovelace.Symbolics.Tests.DiagnosticContractTests.CompleteSolveWithoutANote_CarriesAnEmptyDiagnosticsArray
      Error Message:
    Expected: Vector
    Actual:   Text
    Failed Lovelace.Symbolics.Tests.DiagnosticContractTests.PartialSolve_CarriesADiagnosticWithAStableCodeAndAnErrorCategoryMember
    Failed Lovelace.Symbolics.Tests.DiagnosticContractTests.EveryRichResult_EndsWithADiagnosticsArrayNeverText(typeName: "LimitResult", ...)
      Error Message:
    Expected: Not Text
    Actual:       Text
    Failed ...EveryRichResult_EndsWithADiagnosticsArrayNeverText(typeName: "SolveResult", ...)
      Expected: Not Text / Actual: Text
    Failed ...EveryRichResult_EndsWithADiagnosticsArrayNeverText(typeName: "SystemSolveResult", ...)
      Expected: Not Text / Actual: Text
    Failed ...EveryRichResult_EndsWithADiagnosticsArrayNeverText(typeName: "TransformResult", ...)
      Expected: "diagnostics" / Actual:   "budget_kind"
    Failed ...EveryRichResult_EndsWithADiagnosticsArrayNeverText(typeName: "IntegrationResult", ...)
      Expected: Not Text / Actual: Text
    Failed ...EveryRichResult_EndsWithADiagnosticsArrayNeverText(typeName: "OptimizationResult", ...)
      Expected: "diagnostics" / Actual:   "policy"
    Failed ...EveryRichResult_EndsWithADiagnosticsArrayNeverText(typeName: "CompilationResult", ...)
      Expected: "diagnostics" / Actual:   "exact"
    Failed Lovelace.Symbolics.Tests.DiagnosticContractTests.KernelNotes_CrossAsDiagnosticMessages
      Error Message:
    System.InvalidCastException : Unable to cast object of type 'System.String' to type 'Lovelace.Abstractions.ArrayValue'.
    Failed Lovelace.Symbolics.Tests.DiagnosticContractTests.BoundedTransformation_CarriesABudgetExceededDiagnostic
      Error Message:
    System.InvalidOperationException : Sequence contains no matching element
    Failed Lovelace.Symbolics.Tests.DiagnosticContractTests.ErrorCategory_HasExactlyTheSevenFrozenMembers
      Error Message:
    Assert.NotNull() Failure: Value is null
    Failed Lovelace.Symbolics.Tests.DiagnosticContractTests.UnsatisfiableTransformation_CarriesADomainErrorDiagnostic
      Error Message:
    System.InvalidOperationException : Sequence contains no matching element

    Failed!  - Failed:    14, Passed:     0, Skipped:     0, Total:    14, Duration: 869 ms

Full log: `round-4b/step-a-symbolics.log`.

### 2.2 `dotnet test Lovelace.Suite.Tests --filter "FullyQualifiedName~RecordSchemaTests"`

    Failed Lovelace.Suite.Tests.RecordSchemaTests.SolverSchemas_DeclareTheStructuralKinds
     Assert.Equal() Failure: Collections differ
    Expected: ["Enum", "Domain", "Boolean", "Array", "Array"]
    Actual:   ["Enum", "Domain", "Boolean", "Array", "Text"]
    Failed Lovelace.Suite.Tests.RecordSchemaTests.DiagnosticSchema_DeclaresTheSixFrozenFields
     Assert.NotNull() Failure: Value is null
    Failed Lovelace.Suite.Tests.RecordSchemaTests.EveryProducedRecord_ConformsToItsDeclaredSchema
     Assert.Contains() Failure: Item not found in set
    Set:       ["SolveResult", "Solution", "SolutionFamily", "SystemSolveResult", "SystemSolution", ...]
    Not found: "Diagnostic"

    Failed!  - Failed:     3, Passed:     2, Skipped:     0, Total:     5, Duration: 275 ms

Full log: `round-4b/step-a-suite.log`.

### 2.3 `dotnet test Lovelace.Run.Tests` (before the goldens were regenerated)

    Failed ...RichResultDiagnostics_CrossTheWireAsAnArray(fixture: "compilation", recordType: "CompilationResult")
     Expected: "diagnostics" / Actual:   "exact"
    Failed ...(fixture: "integration_result", ...)  Expected: "Array" / Actual: "Text"
    Failed ...(fixture: "limit_result", ...)        Expected: "Array" / Actual: "Text"
    Failed ...(fixture: "optimization_result", ...) Expected: "diagnostics" / Actual: "policy"
    Failed ...(fixture: "record", recordType: "SolveResult")  Expected: "Array" / Actual: "Text"
    Failed ...(fixture: "system_solve", ...)         Expected: "Array" / Actual: "Text"
    Failed ...(fixture: "transform_result", ...)    Expected: "diagnostics" / Actual: "budget_kind"
    Failed Lovelace.Run.Tests.GoldenEnvelopeTests.WireDiagnostic_CarriesTheSixFrozenFieldsInOrder
    Failed Lovelace.Run.Tests.GoldenEnvelopeTests.EnvelopeMatchesGoldenFixture (7 changed + 2 new goldens)

    Failed!  - Failed:    10, Passed:    18, Skipped:     0, Total:    28, Duration: 804 ms

Full log: `round-4b/step-a-run.log`.

---

## 3. Step B → Step C — the regenerated goldens (OBSERVED diff, all intended)

Goldens were regenerated by running the built runner over each fixture and normalising the four
volatile keys (`revision`, `elapsed`, `elapsedTime`, `timings`), exactly as the golden test
does (`round-4b/regenerate-fixtures.js`). The pre-regeneration copies are kept in
`round-4b/fixtures-before/`; the full diff is `round-4b/golden.diff`. Every changed line is a
diagnostics change — confirmed by reading the diff:

| Golden | Removed | Added | Intended? |
|---|---|---|---|
| `record.json` | `{"kind":"Text","value":""}` | `{"kind":"Array","type":"Vector","shape":[0],"elements":[]}` + `diagnostics: []` in display/typed | yes — a complete solve reports nothing, as an empty array |
| `nested_record.json` | same | same | yes |
| `limit_result.json` | same | same | yes — ``solve(sin(x)/x)` has no failure reason |
| `system_solve.json` | same | same | yes — null kernel note ⇒ empty array |
| `solve_result.json` | `{"kind":"Text","value":""}` | Array shape [1] with one `Diagnostic`: code `solve.unrepresented-roots`, category Enum `ErrorCategory/UnsupportedOperation`, message = the unrepresented reason, `recoverable: true`, `location: Null`, `details: []` | yes — the required "partial solve carries a Diagnostic" |
| `transform_result.json` | (no diagnostics field) | field appended last, Array shape [0] | yes — ordinary transformation reports nothing |
| `compilation.json` | (no diagnostics field) | field appended last, Array shape [0] | yes — successful lowering reports nothing |
| `integration_result.json` | (new fixture) | full envelope, diagnostics Array shape [0] | yes |
| `optimization_result.json` | (new fixture) | full envelope, diagnostics Array shape [0] | yes |

Representative diff text (from `golden.diff`):

    ################ solve_result.json
    -   ... unrepresented_reason: complex algebraic roots not supported (RootOf is real-only in v1)., diagnostics: )
    +   ... unrepresented_reason: complex algebraic roots not supported (RootOf is real-only in v1)., diagnostics: [Diagnostic(code: solve.unrepresented-roots, category: UnsupportedOperation, message: complex algebraic roots not supported (RootOf is real-only in v1)., recoverable: True, location: , details: [])])
    -            "kind": "Text",
    -            "value": ""
    +            "kind": "Array",
    +            "type": "Vector",
    +            "shape": [ 1 ],
    +            "elements": [ { "kind": "Record", "type": "Diagnostic", "fields": [
    +              { "name": "code", "value": { "kind": "Text", "value": "solve.unrepresented-roots" } },
    +              { "name": "category", "value": { "kind": "Enum", "type": "ErrorCategory", "value": "UnsupportedOperation" } },
    +              { "name": "message", ... }, { "name": "recoverable", ... },
    +              { "name": "location", "value": { "kind": "Null" } },
    +              { "name": "details", "value": { "kind": "Array", "shape": [0], "elements": [] } } ] } ]

No value outside the `diagnostics` field (and its rendering inside `display`/`typed`)
changed in any golden.

---

## 4. Step C — verification (OBSERVED)

### 4.1 `dotnet build LovelaceSharp.slnx -c Release --nologo`

    Build succeeded.
        0 Warning(s)
        0 Error(s)

    Time Elapsed 00:00:02.46

(The first, cold build after the change reported `55 Warning(s) / 0 Error(s)` — all pre-existing
nullability warnings in `Lovelace.Real/Real.cs`.)

### 4.2 The five suites (`round-4b/verify-suites.log`)

    ########## dotnet test Lovelace.Run.Tests
    Passed!  - Failed:     0, Passed:    28, Skipped:     0, Total:    28
    ########## dotnet test Lovelace.Symbolics.Tests
    Passed!  - Failed:     0, Passed:   418, Skipped:     0, Total:   418
    ########## dotnet test Lovelace.Suite.Tests
    Passed!  - Failed:     0, Passed:   625, Skipped:     0, Total:   625
    ########## dotnet test Lovelace.Studio.Tests
    Passed!  - Failed:     0, Passed:    22, Skipped:     0, Total:    22
    ########## dotnet test Lovelace.Console.Tests
    Passed!  - Failed:     0, Passed:    15, Skipped:     0, Total:    15

### 4.3 `dotnet publish Lovelace.Run/Lovelace.Run.csproj -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o out/aot`

    Lovelace.Run -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Run\bin\Release\net10.0\win-x64\Lovelace.Run.dll
      Generating native code
      Lovelace.Run -> C:\Users\ricar\dev\LovelaceSharp\out\aot\

exit code 0. Full log: `round-4b/publish-aot.log`.

### 4.4 The error envelope is provably untouched

`error_envelope.json` is byte-identical before and after (`Get-FileHash` =
`E9D7F3FBA9D480BFDBFF2C164A39EA01455FA5CB1440D7020F76C978BF7C5471` on both the pre-regeneration
snapshot and the current fixture), so the envelope's own top-level
`code`/`category`/`message`/`recoverable` and its parse-site `diagnostics` array did not change.
Hashes of the 14 goldens: 7 changed (record, nested_record, solve_result, transform_result,
limit_result, system_solve, compilation), 7 identical (absent_field, array, complex,
error_envelope, print_purity, symbolic, vector).

---

## 5. The published AOT binary (OBSERVED), diagnostics portion

`out/aot/Lovelace.Run.exe --file published-solve.ls --omit-functions` where the script is
`x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)` (exit 0) — full envelope `round-4b/published-solve.json`:

    type=SolveResult
    status={"kind":"Enum","type":"SolveStatus","value":"Partial"}
    diagnostics={
     "kind": "Array",
     "type": "Vector",
     "shape": [ 1 ],
     "elements": [
      { "kind": "Record", "type": "Diagnostic", "fields": [
        { "name": "code",        "value": { "kind": "Text", "value": "solve.unrepresented-roots" } },
        { "name": "category",    "value": { "kind": "Enum", "type": "ErrorCategory", "value": "UnsupportedOperation" } },
        { "name": "message",     "value": { "kind": "Text", "value": "complex algebraic roots not supported (RootOf is real-only in v1)." } },
        { "name": "recoverable", "value": { "kind": "Boolean", "value": "true" } },
        { "name": "location",    "value": { "kind": "Null" } },
        { "name": "details",     "value": { "kind": "Array", "type": "Vector", "shape": [ 0 ], "elements": [] } } ] } ]
    }

`out/aot/Lovelace.Run.exe --file published-limit.ls --omit-functions` where the script is
`x = symbol("x"); limit_full(sin(x)/x, x, 0)` (exit 0) — `round-4b/published-limit.json`:

    type=LimitResult
    status={"kind":"Enum","type":"LimitStatus","value":"Value"}
    diagnostics={
     "kind": "Array",
     "type": "Vector",
     "shape": [ 0 ],
     "elements": []
    }

---

## 6. Required tests and new test counts

New/strengthened test cases:

| Suite | Before | After | New cases |
|---|---|---|---|
| `Lovelace.Symbolics.Tests` | 404 (418 − 14; the filtered Step-A run reported exactly 14 cases in the new class) | **418** | 14: `ErrorCategory_HasExactlyTheSevenFrozenMembers`, `EveryRichResult_EndsWithADiagnosticsArrayNeverText` ×7 (one per record, through its builtin), `PartialSolve_CarriesADiagnosticWithAStableCodeAndAnErrorCategoryMember`, `CompleteSolveWithoutANote_CarriesAnEmptyDiagnosticsArray`, `SatisfiedTransformation_CarriesAnEmptyDiagnosticsArray`, `BoundedTransformation_CarriesABudgetExceededDiagnostic`, `UnsatisfiableTransformation_CarriesADomainErrorDiagnostic`, `KernelNotes_CrossAsDiagnosticMessages` |
| `Lovelace.Suite.Tests` | 624 | **625** | 1 new fact (`DiagnosticSchema_DeclaresTheSixFrozenFields`) + the produced-record corpus now exercises and asserts the `Diagnostic` schema |
| `Lovelace.Run.Tests` | 18 | **28** | 10: `RichResultDiagnostics_CrossTheWireAsAnArray` ×7, `WireDiagnostic_CarriesTheSixFrozenFieldsInOrder`, plus the 2 new golden-corpus rows (integration_result, optimization_result) |
| `Lovelace.Studio.Tests` / `Lovelace.Console.Tests` | 22 / 15 | 22 / 15 | none |

Mapping to the required list (all met):

* each of the seven records, through its builtin, emits an Array and never Text —
  `EveryRichResult_EndsWithADiagnosticsArrayNeverText` (in-process) **and**
  `RichResultDiagnostics_CrossTheWireAsAnArray` (published envelope shape).
* partial solve `x^4 - x^2 - 1 == 0` carries ≥1 Diagnostic with a stable non-empty code
  (`solve.unrepresented-roots`) and an `ErrorCategory` member (`UnsupportedOperation`).
* a solve with a null note (nothing unrepresented) carries an **empty Array** — asserted as
  `ValueKind.Vector` + `Assert.Empty` + structured `kind == "Array"` + zero elements (not Null,
  not "").
* the six field names, in order, are asserted both in-process and on the wire.
* bounded transformation: **reachable** — `simplify_full` over 420 independent
  `exp(log(x+k))` redexes exhausts the shipped `MaxSteps = 400` and reports
  `status: BudgetExceeded` with code `transform.budget-exceeded` / category
  `BudgetExceeded`. Unsatisfiable conditions: **reachable** —
  `x = symbol("x"); assume(x == 0); simplify_full(x/x)` reports
  `status: Unsatisfiable` with code `transform.unsatisfiable-conditions` / category
  `DomainError`. Both were searched live before any code was written: the alternative
  `abs(-(x+k))` route is refused by the language (`abs() is not supported for values of kind
  'Symbolic'`, category `DomainError`), so the `exp(log(x+k))` route is the one used. Probe
  scripts kept in `round-4b/probes/`.
* "nothing on the wire still carries a free-text diagnostics value" — the seven-case wire theory
  asserts `kind == "Array"` and `value == null` for every record.

---

## 7. Judgment calls, and what I could NOT do

1. **D1 interpretation (inference, labelled).** D1 says "remove the free-text `Note` /
   `FailureReason` members from the emitted records". No emitted record ever had a field *named*
   `note`/`failure_reason`; the prose crossed as the `diagnostics` value. I therefore removed
   the prose **from the wire** (the field is now the Diagnostic array) and kept the kernel members
   `SolutionSet.Note`, `SystemSolveResult.Note`, `LimitResult.FailureReason`,
   `IntegrationResult.Note` as the source of a diagnostic **message** and of the human
   `display` projection the addendum explicitly allows ("a caller that wants prose reads
   `display`"). Deleting those kernel members would have required editing
   `Lovelace.Symbolics/Solvers/Solve.cs`, `Calculus/Limits.cs`, `Calculus/Integrate.cs` and
   `Matrices/SymbolicMatrix.cs` (outside SCOPE) and would have broken the human `solve`/
   `linsolve_full` projections.
2. **Two records outside the seven still carry a free-text `diagnostics` value.** They are not
   part of this objective (section C does not list them) and live in
   `Lovelace.Suite/Interpreter.cs`, which is **not in SCOPE** (only `RecordSchemas.cs` is), so I
   did not touch them:
   * `MatrixInverseResult` — `Lovelace.Suite/Interpreter.cs:1281` `new RecordField("diagnostics", "matrix is singular")` and `:1286` `new RecordField("diagnostics", "")`;
   * `MatrixSolveResult` — `Lovelace.Suite/Interpreter.cs:1340` `new RecordField("diagnostics", note ?? "")`.
   Their `status` fields are also still free text. Flagging rather than silently widening scope.
3. **`.github/workflows/ci.yml`: no change.** `Select-String -Path .github/workflows/ci.yml
   -Pattern 'diagnostic' -SimpleMatch` returns **0** matches, i.e. no smoke assertion asserts a
   diagnostics shape, so per the instruction the assertions were left alone. (`git diff` on that
   file shows only the pre-existing round-4a work: the D2 Enum-kind assertions and the Boolean
   representation repair — not this round.)
4. **New test file location.** `Lovelace.Symbolics.Tests/DiagnosticContractTests.cs` is not named
   in SCOPE (which names only *existing* tests asserting the old text diagnostics), but the
   REQUIRED TESTS section mandates new tests and this project already owns the SuiteEngine+MathIR
   harness that reaches all seven builtins. Test-only addition; no production file outside SCOPE
   was touched.
5. **`IntegrationResult` diagnostics are always empty in practice** — `Integration.IntegrateResult`
   never sets `Note` (only `Exact`/`Conditional`/`Unevaluated` are constructed). The field
   is present and empty; the mapping from a future note is in place
   (`SymbolicsPlugin.IntegrationDiagnostic`).
6. **`location` is always wire `Null` today**: no kernel call site has a source span, which is
   the contract ("Location is null for a kernel-level diagnostic"). The
   `DiagnosticLocation` wire record and its projection exist and are unit-reachable through
   `DiagnosticProjection.ToRecordValue(DiagnosticLocation)`, but no language-level path produces
   one yet, so the `{"kind":"Record","type":"DiagnosticLocation"}` form is **not** exercised
   end-to-end.

---

## 8. Evidence index (`docs/goal-cycle-3/round-4b/`)

| File | Content |
|---|---|
| `step-a-symbolics.log`, `step-a-suite.log`, `step-a-run.log` | Step-A observed failures (14/14, 3/5, 10/28) |
| `step-b-*.log` | the suites immediately after the source change, before the golden refresh (Run.Tests 9 golden mismatches; Symbolics/Suite green) |
| `step-c-run.log` | `dotnet test Lovelace.Run.Tests` green after regeneration (28/28) |
| `fixtures-before/` | the 14 goldens as they stood before regeneration |
| `golden.diff` | the grouped old→new diff of every changed golden, plus the diagnostics excerpt of the two new ones |
| `regenerate-fixtures.js`, `run-envelope.ps1` | the exact regeneration procedure |
| `verify-suites.ps1`, `verify-suites.log` | the five-suite verification |
| `publish-aot.log` | the Native AOT publish |
| `published-solve.ls` / `published-solve.json`, `published-limit.ls` / `published-limit.json`, `published-excerpt.js` | the two published-binary runs and their diagnostics excerpts |
| `probes/` | the live reachability probes: `budget-exp.ls` (BudgetExceeded), `unsat.ls` (unsatisfiable conditions), `budget-abs-refused.ls` (the refused alternative), `partial.ls` (the partial solve) |
