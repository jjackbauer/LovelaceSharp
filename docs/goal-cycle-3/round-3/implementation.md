# Round 3 — the solver's structured results are structural on the wire

Bounded change: **A1 (structural system bindings) + A2 (family parameter / variable / parameter
domain)** of `docs/symbolics/a-plus-cycle-3-alignment.md` §10 Phase A. The frozen contract is
`docs/symbolics/a-plus-convergence-alignment-plan.md` §C (types) and §J (intentional breaks); the
wire field-name resolution is the addendum §9.1 (`Binding` uses `name`/`value`).

Out of scope and untouched: diagnostics restructuring, the `Enum` value kind, `Lovelace.Run/Runner.cs`,
`Lovelace.Run/RunProtocol.cs`, the slnx, `ci.yml`, everything under `docs/goal-cycle-3/` except this
round's directory.

---

## 1. What changed (SCOPE)

| Path | Change |
| --- | --- |
| `Lovelace.Symbolics/Solvers/Solve.cs` | `SystemSolution` gains `public SolutionExactness Exactness { get; init; } = SolutionExactness.Exact;` (the positional `Assignment`/`Conditions` shape is unchanged, so every construction site keeps compiling). Both Gröbner-elimination construction sites set `Exactness = SolutionExactness.Exact` **explicitly**. The existing, previously unused `Binding(string Name, Expr Value)` type is now used by the projection. |
| `Lovelace.Symbolics/SymbolicsPlugin.cs` | `solve_system_full`: `assignment` (array of `"x = 2"` strings) → `bindings` (array of `RecordValue("Binding", name, value)`); `SystemSolution` gains `exactness`; `SystemSolveResult` gains `domain` (Domain) and `complete` (Boolean) **after `status`**. `solve_full`: `variable` and the family `parameter` are the Symbol **expressions** (Symbolic), not name strings. `ParameterDomainOf` is a total, explicit switch over both declared arms with a throw arm for undeclared ones. |
| `Lovelace.Suite/RecordSchemas.cs` | `SolutionFamily.parameter` Text→Symbolic; `SolveResult.variable` Text→Symbolic; `SystemSolution` `assignment`→`bindings` + `exactness`; `SystemSolveResult` +`domain`,`complete` (after `status`, matching the emitted order); new `Binding` entry. |
| `Lovelace.Suite.Tests/RecordSchemaTests.cs` | Drift corpus asserts `Binding` is actually produced; new `SolverSchemas_DeclareTheStructuralKinds` pins names, kinds **and order** for the five solve schemas. |
| `Lovelace.Symbolics.Tests/DxStructuredResultsTests.cs` | Six new wire tests (see §2). |
| `Lovelace.Run.Tests/fixtures/{record,solve_result,nested_record,system_solve}.json` | Regenerated goldens (4 of 14 files; the other 10 are byte-identical). |
| `docs/symbolics/dsh-protocol.md` | **Unchanged — nothing to change.** The document does not mention the system-solve shape or the family parameter: `Select-String -Path docs/symbolics/dsh-protocol.md -Pattern 'assignment|parameter|variable|binding'` → **0 matches**. Quoted unchanged lines that remain accurate: line 11 *"Every field is one of `scalar | symbolic | array | record | domain | null`; nothing degrades to text because the serializer lacked a case."* and line 22 *"snake_case record field names (record field names come from the kernel records and are part of the contract)"* — this round makes those two lines true for the three flattened fields. |

---

## 2. STEP A — tests first, observed RED

The assertions were written before any production edit. The only production change made *before*
running them is a **visibility-only** edit (`private static MathDomain ParameterDomainOf` →
`internal static`, zero behaviour change): `Lovelace.Symbolics.csproj:13` already declares
`<InternalsVisibleTo Include="Lovelace.Symbolics.Tests" />`, and the `NonNegativeIntegers` arm
cannot be reached from the language, so the mapping is pinnable only by a direct call.

```
$ dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo
```

```text
  Failed Lovelace.Symbolics.Tests.DxStructuredResultsTests.SolveSystemFull_CarriesDomainCompleteAndPerSolutionExactness [6 ms]
  Error Message:
   Assert.Equal() Failure: Collections differ
                     ↓ (pos 1)
Expected: ["status", "domain", "complete", "solutions", "diagnostics"]
Actual:   ["status", "solutions", "diagnostics"]
                     ↑ (pos 1)
  Failed Lovelace.Symbolics.Tests.DxStructuredResultsTests.SolveSystemFull_EveryEmittedSystemSolutionCarriesExactnessAndBindings [2 ms]
  Error Message:
   System.InvalidOperationException : Sequence contains no matching element
  Failed Lovelace.Symbolics.Tests.DxStructuredResultsTests.ParameterDomainOf_MapsEveryDeclaredArm [8 ms]
  Error Message:
   Assert.Throws() Failure: No exception was thrown
Expected: typeof(System.ArgumentOutOfRangeException)
  Failed Lovelace.Symbolics.Tests.DxStructuredResultsTests.SolveFull_VariableIsSymbolic_NotTheNameString [< 1 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: Symbolic
Actual:   Text
  Failed Lovelace.Symbolics.Tests.DxStructuredResultsTests.SolveFull_FamilyParameterIsSymbolic_AndItsDomainStaysADomain [< 1 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: Symbolic
Actual:   Text
  Failed Lovelace.Symbolics.Tests.DxStructuredResultsTests.SolveSystemFull_EmitsBindingRecords_NeverTheFlattenedString [1 ms]
  Error Message:
   System.InvalidOperationException : Sequence contains no matching element

Failed!  - Failed:     6, Passed:   389, Skipped:     0, Total:   395, Duration: 13 s - Lovelace.Symbolics.Tests.dll (net10.0)
```

```
$ dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo
```

```text
  Failed Lovelace.Suite.Tests.RecordSchemaTests.SolverSchemas_DeclareTheStructuralKinds [5 ms]
  Error Message:
   Assert.Equal() Failure: Collections differ
                     ↓ (pos 1)
Expected: ["status", "domain", "complete", "solutions", "diagnostics"]
Actual:   ["status", "solutions", "diagnostics"]
                     ↑ (pos 1)
  Failed Lovelace.Suite.Tests.RecordSchemaTests.EveryProducedRecord_ConformsToItsDeclaredSchema [64 ms]
  Error Message:
   Assert.Contains() Failure: Item not found in set
Set:       ["SolveResult", "Solution", "SolutionFamily", "SystemSolveResult", "SystemSolution", ···]
Not found: "Binding"

Failed!  - Failed:     2, Passed:   619, Skipped:     0, Total:   621, Duration: 8 s - Lovelace.Suite.Tests.dll (net10.0)
```

The 14 golden-fixture tests were still green **before** the production change (Step A touched no
emitted value), which is what makes Step B's red a signal rather than noise:

```
$ dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo
```

```text

Passed!  - Failed:     0, Passed:    18, Skipped:     0, Total:    18, Duration: 725 ms - Lovelace.Run.Tests.dll (net10.0)
```

Full logs: `step-a-symbolics.log`, `step-a-suite.log`, `step-a-run-tests.log` (UTF-16, as
PowerShell 5.1 writes them); the UTF-8 excerpts embedded above are
`excerpt-step-a-*.txt`.

### 2.1 One test-authoring defect found and corrected (disclosed)

The first version of `SolveSystemFull_EmitsBindingRecords_NeverTheFlattenedString` looked for
`bindings` on the **SystemSolveResult**; `bindings` is a field of the **SystemSolution** (§C and
the schema instruction both place it there). The Step-A log therefore shows that one test failing
with `Sequence contains no matching element` at the wrong lookup. The path was corrected (the
assertions are unchanged) and the corrected assertion was then observed RED **against the old
emission**, by temporarily restoring only the old emission block, and GREEN again after restoring
the new one:

```text
$ dotnet test Lovelace.Symbolics.Tests/... --filter "FullyQualifiedName~SolveSystemFull_EmitsBindingRecords_NeverTheFlattenedString"
  Failed Lovelace.Symbolics.Tests.DxStructuredResultsTests.SolveSystemFull_EmitsBindingRecords_NeverTheFlattenedString [81 ms]
  Error Message:
   System.InvalidOperationException : Sequence contains no matching element
  Stack Trace:
     at System.Linq.ThrowHelper.ThrowNoMatchException()
   at System.Linq.Enumerable.First[TSource](IEnumerable`1 source, Func`2 predicate)
   at Lovelace.Symbolics.Tests.DxStructuredResultsTests.Field(RecordValue record, String name) in ...\DxStructuredResultsTests.cs:line 401
   at Lovelace.Symbolics.Tests.DxStructuredResultsTests.SolveSystemFull_EmitsBindingRecords_NeverTheFlattenedString() in ...\DxStructuredResultsTests.cs:line 418

Failed!  - Failed:     1, Passed:     0, Skipped:     0, Total:     1, Duration: 92 ms - Lovelace.Symbolics.Tests.dll (net10.0)

$ dotnet test Lovelace.Symbolics.Tests/... --filter "FullyQualifiedName~SolveSystemFull_EmitsBindingRecords_NeverTheFlattenedString"   (new emission restored)
Passed!  - Failed:     0, Passed:     1, Skipped:     0, Total:     1, Duration: 84 ms - Lovelace.Symbolics.Tests.dll (net10.0)
```

(`step-a-corrected-test-red.log`.) `git diff` after the experiment shows the new emission back in
place (Step-B source diff, §3.1).

### 2.2 The six new Symbolics tests

| Test | What it pins |
| --- | --- |
| `SolveSystemFull_EmitsBindingRecords_NeverTheFlattenedString` | every element of `bindings` is a `Binding` **Record** (never `Text`); `name` is Symbolic with canonical `(sym x)`/`(sym y)`; `value` is Symbolic with pretty `2`/`-1` |
| `SolveSystemFull_CarriesDomainCompleteAndPerSolutionExactness` | emitted field order is exactly `status, domain, complete, solutions, diagnostics`; `domain` is `ValueKind.Domain` = complex; `complete` is a `Boolean` true; the `SystemSolution` carries `exactness` = `"Exact"` |
| `SolveSystemFull_EveryEmittedSystemSolutionCarriesExactnessAndBindings` | the 4-solution circle/hyperbola system: **every** `SystemSolution` has `exactness` and a `Binding` array |
| `SolveFull_FamilyParameterIsSymbolic_AndItsDomainStaysADomain` | `solve_full(sin(x) == 0, x)`: `parameter` is Symbolic (pretty `k`, canonical `(sym k)`), `parameter_domain` is still a Domain value (integer) |
| `SolveFull_VariableIsSymbolic_NotTheNameString` | `solve_full(x^2 - 4 == 0, x)`: `variable` is Symbolic (canonical `(sym x)`) |
| `ParameterDomainOf_MapsEveryDeclaredArm` | the pinned table is exactly the declared enum (`Enum.GetValues<ParameterDomain>()`), each arm maps to `MathDomain.Integer`, and an **undeclared** arm `(ParameterDomain)99` throws `ArgumentOutOfRangeException` |

The Suite suite adds `SolverSchemas_DeclareTheStructuralKinds` and extends the drift corpus with
`SolutionFamily`, `SystemSolveResult`, `SystemSolution` and `Binding` (`Binding` was previously
invisible to the registry).

---

## 3. STEP B — the change

### 3.1 Source diff (summary)

```
$ git diff --stat -- Lovelace.Symbolics Lovelace.Suite Lovelace.Suite.Tests Lovelace.Symbolics.Tests
 Lovelace.Suite.Tests/RecordSchemaTests.cs          |  39 ++++++
 Lovelace.Suite/RecordSchemas.cs                    |  12 +-
 .../DxStructuredResultsTests.cs                    | 150 +++++++++++++++++++++
 Lovelace.Symbolics/Solvers/Solve.cs                |  21 ++-
 Lovelace.Symbolics/SymbolicsPlugin.cs              |  42 ++++--
 5 files changed, 248 insertions(+), 16 deletions(-)
```

Full unified diff: `source.diff`.

The five production hunks:

1. **`Solve.cs:836-843`** — `SystemSolution` keeps its positional `(Assignment, Conditions)` shape
   and gains `public SolutionExactness Exactness { get; init; } = SolutionExactness.Exact;`.
2. **`Solve.cs:921-926` and `Solve.cs:944-948`** — the two system-solution construction sites (the
   no-variables-left base case and the univariate back-substitution) set
   `Exactness = SolutionExactness.Exact` explicitly, with the comment that the Gröbner elimination
   and the recursive substitution are exact operations.
3. **`SymbolicsPlugin.cs:335-346`** — `solve_system_full` emits
   `new RecordField("bindings", vs.Select(v => BindingRecord(new Binding(v.Name, sol.Assignment[v]))))`,
   `exactness`, and `SystemSolveResult` = `status, domain, complete, solutions, diagnostics`.
4. **`SymbolicsPlugin.cs:400,408`** — `parameter` = `Exprs.Symbol(f.Parameter)`,
   `variable` = `Exprs.Symbol(sx)`.
5. **`SymbolicsPlugin.cs:614-630`** — `ParameterDomainOf` is an explicit, total switch; the new
   `BindingRecord(Binding)` helper (line ~618) is the single place the binding pair is projected.

### 3.2 Decisions taken (inference where the brief was silent) — [INFERRED]

* **`Binding.name` is emitted as the Symbol (Symbolic), and the kernel `Binding` is used.** §9.1 fixes
  the field *names*; the brief fixes the field *values* (`RecordField("name", <the symbol>)`) and the
  required test asks for a **canonical form**, which only a Symbolic value has. The kernel
  `Binding(string Name, Expr Value)` is constructed for the pair and projected by `BindingRecord`.
  Precedent: `DomainCondition.variable` already emits `Exprs.Symbol(...)` and the registry already
  declares it `Symbolic` (`SymbolicsPlugin.cs:794`, `RecordSchemas.cs:108`).
* **`SystemSolveResult.status` is now `result.Status.ToString()`** (kernel derivation) instead of the
  inline copy. Both `status` and `complete` therefore come from one expression; the only behaviour
  change is that a **truncated** system enumeration (`MaxSolutions = 128`) now reports `Partial`
  instead of a `Solved` that contradicted `complete = false`. Alignment addendum §10 acceptance
  gate: *"no path that emits a status and a completeness derived from different expressions
  (RISK-003)"*.
* **A `Binding` schema entry was registered** (`("name","Symbolic"),("value","Symbolic")`), one more
  than the four edits listed in the brief: without it the brand-new record type would be invisible
  to the drift test the brief relies on.
* **`solve_system` (the non-`_full` convenience builtin) still returns prose** — it is the
  human-facing projection; the machine API is `solve_system_full`.
* **`ParameterDomainOf` is `internal`, not `private`** — required to pin the unreachable arm (§2).

---

## 4. Golden fixtures — the diff was read before regenerating

Four goldens went red in Step B, exactly the four whose wire changed:

```text
  Failed Lovelace.Run.Tests.GoldenEnvelopeTests.EnvelopeMatchesGoldenFixture(name: "solve_result", expectedExitCode: 0) [24 ms]
  Error Message:
   solve_result: envelope does not match the golden fixture:
$.result.structured.fields[1].value: keys [kind, value] != [canonical, domain, exact, freeSymbols, kind, nodeCount, pretty]
$.result.structured.fields[1].value.kind: expected "Text", got "Symbolic"
  Failed Lovelace.Run.Tests.GoldenEnvelopeTests.EnvelopeMatchesGoldenFixture(name: "system_solve", expectedExitCode: 0) [15 ms]
  Error Message:
   system_solve: envelope does not match the golden fixture:
$.result.display: expected "SystemSolveResult(status: Solved, solutions: [SystemSolution(assignment: [x = 2, y = -1], conditions: [])], diagnostics: )", got "SystemSolveResult(status: Solved, domain: complex, complete: True, solutions: [SystemSolution(bindings: [Binding(name: x, value: 2), Binding(name: y, value: -1)], conditions: [], exactness: Exact)], diagnostics: )"
$.result.structured.fields: array length 3 != 5
$.result.structured.fields[1].name: expected "solutions", got "domain"
$.result.structured.fields[1].value: keys [elements, kind, shape, type] != [domain, kind]
$.result.structured.fields[1].value.kind: expected "Array", got "Domain"
$.result.structured.fields[2].name: expected "diagnostics", got "complete"
$.result.structured.fields[2].value.kind: expected "Text", got "Boolean"
$.result.structured.fields[2].value.value: expected "", got "true"
$.result.typed: expected "SystemSolveResult(status: Solved, solutions: [SystemSolution(assignment: [x = 2, y = -1], conditions: [])], diagnostics: ) (SystemSolveResult)", got "SystemSolveResult(status: Solved, domain: complex, complete: True, solutions: [SystemSolution(bindings: [Binding(name: x, value: 2), Binding(name: y, value: -1)], conditions: [], exactness: Exact)], diagnostics: ) (SystemSolveResult)"
$.variables[0].display: expected "SystemSolveResult(status: Solved, solutions: [SystemSolution(assignment: [x = 2, y = -1], conditions: [])], diagnostics: )", got "SystemSolveResult(status: Solved, domain: complex, complete: True, solutions: [SystemSolution(bindings: [Binding(name: x, value: 2), Binding(name: y, value: -1)], conditions: [], exactness: Exact)], diagnostics: )"
  Failed Lovelace.Run.Tests.GoldenEnvelopeTests.EnvelopeMatchesGoldenFixture(name: "nested_record", expectedExitCode: 0) [8 ms]
  Error Message:
   nested_record: envelope does not match the golden fixture:
$.result.structured.fields[1].value: keys [kind, value] != [canonical, domain, exact, freeSymbols, kind, nodeCount, pretty]
$.result.structured.fields[1].value.kind: expected "Text", got "Symbolic"
$.result.structured.fields[6].value.elements[0].fields[1].value: keys [kind, value] != [canonical, domain, exact, freeSymbols, kind, nodeCount, pretty]
$.result.structured.fields[6].value.elements[0].fields[1].value.kind: expected "Text", got "Symbolic"
  Failed Lovelace.Run.Tests.GoldenEnvelopeTests.EnvelopeMatchesGoldenFixture(name: "record", expectedExitCode: 0) [3 ms]
  Error Message:
   record: envelope does not match the golden fixture:
$.result.structured.fields[1].value: keys [kind, value] != [canonical, domain, exact, freeSymbols, kind, nodeCount, pretty]
$.result.structured.fields[1].value.kind: expected "Text", got "Symbolic"

Failed!  - Failed:     4, Passed:    14, Skipped:     0, Total:    18, Duration: 457 ms - Lovelace.Run.Tests.dll (net10.0)
```

Every changed value was read and is intended:

| Fixture | Field | Before | After |
| --- | --- | --- | --- |
| `record.json`, `solve_result.json`, `nested_record.json` | `SolveResult.variable` | `Text("x")` | `Symbolic` pretty `x`, canonical `(sym x)` |
| `nested_record.json` | `SolutionFamily.parameter` | `Text("k")` | `Symbolic` pretty `k`, canonical `(sym k)` |
| `system_solve.json` | `SystemSolveResult` fields | `status, solutions, diagnostics` | `status, domain, complete, solutions, diagnostics` |
| `system_solve.json` | `SystemSolution` fields | `assignment` (Array of `Text("x = 2")`), `conditions` | `bindings` (Array of `Binding` records), `conditions`, `exactness` |

No arithmetic, no ordering, no condition and no count changed anywhere. The other ten fixtures are
byte-identical, and re-running the generator is byte-identical
(`REGENERATION_IDENTICAL=True`).

```diff
diff --git a/docs/goal-cycle-3/round-3/fixtures-before/nested_record.json b/Lovelace.Run.Tests/fixtures/nested_record.json
index 5495e8f..dbaa74a 100644
--- a/docs/goal-cycle-3/round-3/fixtures-before/nested_record.json
+++ b/Lovelace.Run.Tests/fixtures/nested_record.json
@@ -22,8 +22,15 @@
         {
           "name": "variable",
           "value": {
-            "kind": "Text",
-            "value": "x"
+            "kind": "Symbolic",
+            "pretty": "x",
+            "canonical": "(sym x)",
+            "domain": "complex",
+            "exact": true,
+            "nodeCount": 1,
+            "freeSymbols": [
+              "x"
+            ]
           }
         },
         {
@@ -88,8 +95,15 @@
                   {
                     "name": "parameter",
                     "value": {
-                      "kind": "Text",
-                      "value": "k"
+                      "kind": "Symbolic",
+                      "pretty": "k",
+                      "canonical": "(sym k)",
+                      "domain": "complex",
+                      "exact": true,
+                      "nodeCount": 1,
+                      "freeSymbols": [
+                        "k"
+                      ]
                     }
                   },
                   {
diff --git a/docs/goal-cycle-3/round-3/fixtures-before/record.json b/Lovelace.Run.Tests/fixtures/record.json
index a49e73a..bc3e805 100644
--- a/docs/goal-cycle-3/round-3/fixtures-before/record.json
+++ b/Lovelace.Run.Tests/fixtures/record.json
@@ -22,8 +22,15 @@
         {
           "name": "variable",
           "value": {
-            "kind": "Text",
-            "value": "x"
+            "kind": "Symbolic",
+            "pretty": "x",
+            "canonical": "(sym x)",
+            "domain": "complex",
+            "exact": true,
+            "nodeCount": 1,
+            "freeSymbols": [
+              "x"
+            ]
           }
         },
         {
diff --git a/docs/goal-cycle-3/round-3/fixtures-before/solve_result.json b/Lovelace.Run.Tests/fixtures/solve_result.json
index ebb4b7f..f688ad1 100644
--- a/docs/goal-cycle-3/round-3/fixtures-before/solve_result.json
+++ b/Lovelace.Run.Tests/fixtures/solve_result.json
@@ -22,8 +22,15 @@
         {
           "name": "variable",
           "value": {
-            "kind": "Text",
-            "value": "x"
+            "kind": "Symbolic",
+            "pretty": "x",
+            "canonical": "(sym x)",
+            "domain": "complex",
+            "exact": true,
+            "nodeCount": 1,
+            "freeSymbols": [
+              "x"
+            ]
           }
         },
         {
diff --git a/docs/goal-cycle-3/round-3/fixtures-before/system_solve.json b/Lovelace.Run.Tests/fixtures/system_solve.json
index 7e3ce84..7145a42 100644
--- a/docs/goal-cycle-3/round-3/fixtures-before/system_solve.json
+++ b/Lovelace.Run.Tests/fixtures/system_solve.json
@@ -6,8 +6,8 @@
   "revision": "<volatile>",
   "result": {
     "kind": "Record",
-    "display": "SystemSolveResult(status: Solved, solutions: [SystemSolution(assignment: [x = 2, y = -1], conditions: [])], diagnostics: )",
-    "typed": "SystemSolveResult(status: Solved, solutions: [SystemSolution(assignment: [x = 2, y = -1], conditions: [])], diagnostics: ) (SystemSolveResult)",
+    "display": "SystemSolveResult(status: Solved, domain: complex, complete: True, solutions: [SystemSolution(bindings: [Binding(name: x, value: 2), Binding(name: y, value: -1)], conditions: [], exactness: Exact)], diagnostics: )",
+    "typed": "SystemSolveResult(status: Solved, domain: complex, complete: True, solutions: [SystemSolution(bindings: [Binding(name: x, value: 2), Binding(name: y, value: -1)], conditions: [], exactness: Exact)], diagnostics: ) (SystemSolveResult)",
     "structured": {
       "kind": "Record",
       "type": "SystemSolveResult",
@@ -19,6 +19,20 @@
             "value": "Solved"
           }
         },
+        {
+          "name": "domain",
+          "value": {
+            "kind": "Domain",
+            "domain": "complex"
+          }
+        },
+        {
+          "name": "complete",
+          "value": {
+            "kind": "Boolean",
+            "value": "true"
+          }
+        },
         {
           "name": "solutions",
           "value": {
@@ -33,7 +47,7 @@
                 "type": "SystemSolution",
                 "fields": [
                   {
-                    "name": "assignment",
+                    "name": "bindings",
                     "value": {
                       "kind": "Array",
                       "type": "Vector",
@@ -42,12 +56,68 @@
                       ],
                       "elements": [
                         {
-                          "kind": "Text",
-                          "value": "x = 2"
+                          "kind": "Record",
+                          "type": "Binding",
+                          "fields": [
+                            {
+                              "name": "name",
+                              "value": {
+                                "kind": "Symbolic",
+                                "pretty": "x",
+                                "canonical": "(sym x)",
+                                "domain": "complex",
+                                "exact": true,
+                                "nodeCount": 1,
+                                "freeSymbols": [
+                                  "x"
+                                ]
+                              }
+                            },
+                            {
+                              "name": "value",
+                              "value": {
+                                "kind": "Symbolic",
+                                "pretty": "2",
+                                "canonical": "(rat 2 1)",
+                                "domain": "rational",
+                                "exact": true,
+                                "nodeCount": 1,
+                                "freeSymbols": []
+                              }
+                            }
+                          ]
                         },
                         {
-                          "kind": "Text",
-                          "value": "y = -1"
+                          "kind": "Record",
+                          "type": "Binding",
+                          "fields": [
+                            {
+                              "name": "name",
+                              "value": {
+                                "kind": "Symbolic",
+                                "pretty": "y",
+                                "canonical": "(sym y)",
+                                "domain": "complex",
+                                "exact": true,
+                                "nodeCount": 1,
+                                "freeSymbols": [
+                                  "y"
+                                ]
+                              }
+                            },
+                            {
+                              "name": "value",
+                              "value": {
+                                "kind": "Symbolic",
+                                "pretty": "-1",
+                                "canonical": "(rat -1 1)",
+                                "domain": "rational",
+                                "exact": true,
+                                "nodeCount": 1,
+                                "freeSymbols": []
+                              }
+                            }
+                          ]
                         }
                       ]
                     }
@@ -62,6 +132,13 @@
                       ],
                       "elements": []
                     }
+                  },
+                  {
+                    "name": "exactness",
+                    "value": {
+                      "kind": "Text",
+                      "value": "Exact"
+                    }
                   }
                 ]
               }
@@ -83,7 +160,7 @@
     {
       "name": "_",
       "kind": "Record",
-      "display": "SystemSolveResult(status: Solved, solutions: [SystemSolution(assignment: [x = 2, y = -1], conditions: [])], diagnostics: )"
+      "display": "SystemSolveResult(status: Solved, domain: complex, complete: True, solutions: [SystemSolution(bindings: [Binding(name: x, value: 2), Binding(name: y, value: -1)], conditions: [], exactness: Exact)], diagnostics: )"
     },
     {
       "name": "x",
```

---

## 5. STEP C — observed GREEN

```
$ dotnet build LovelaceSharp.slnx -c Release --nologo
    0 Warning(s)
    0 Error(s)
Time Elapsed 00:00:05.04
BUILD_EXIT=0

$ dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:    18, Skipped:     0, Total:    18, Duration: 774 ms - Lovelace.Run.Tests.dll (net10.0)
RUN_TESTS_EXIT=0

$ dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   395, Skipped:     0, Total:   395, Duration: 14 s - Lovelace.Symbolics.Tests.dll (net10.0)
SYMBOLICS_EXIT=0

$ dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   621, Skipped:     0, Total:   621, Duration: 8 s - Lovelace.Suite.Tests.dll (net10.0)
SUITE_EXIT=0
```

Extra regression check of a structured-payload consumer that is not in the required list:
`dotnet test Lovelace.Studio.Tests/Lovelace.Studio.Tests.csproj -c Release --nologo` →
`Passed!  - Failed: 0, Passed: 22, Skipped: 0, Total: 22` (`STUDIO_EXIT=0`).

### 5.1 Publish (Native AOT)

```
$ dotnet publish Lovelace.Run/Lovelace.Run.csproj -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o out/aot
  Lovelace.Run -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Run\bin\Release\net10.0\win-x64\Lovelace.Run.dll
  Generating native code
  Lovelace.Run -> C:\Users\ricar\dev\LovelaceSharp\out\aot\
PUBLISH_EXIT=0
exe_bytes=5614080  mtime=09/10/2026 23:52:34
```

The publish was repeated after the §2.1 experiment (the temporary edit moved a source timestamp even
though the restored content is identical). Proof that the tree really is identical:
`git diff -- Lovelace.Symbolics Lovelace.Suite Lovelace.Suite.Tests Lovelace.Symbolics.Tests`
hashed before (`source.diff`) and after (`source-after-experiment.diff`) the experiment:

```text
FC60DEA9025C6C8ACCA028D0F237A51E092F0C65BCA145C33821E4329ABA8D9D   source.diff
FC60DEA9025C6C8ACCA028D0F237A51E092F0C65BCA145C33821E4329ABA8D9D   source-after-experiment.diff
```

Freshness gate (the addendum's mechanical guard): published exe `09/10/2026 23:52:34` vs newest
first-party source `Lovelace.Symbolics/SymbolicsPlugin.cs @ 09/10/2026 23:51:29` → `STALE=False`.

### 5.2 The published binary on the required script

```
$ out\aot\Lovelace.Run.exe --file docs/goal-cycle-3/round-3/aot-solve-system-full.ls --omit-functions
AOT_RUN_EXIT=0
```

The freshly re-published binary was run again and its envelope is identical to the first published
run modulo the volatile keys (`revision`, `elapsed`, `elapsedTime`, `timings`):
`ENVELOPE_IDENTICAL_MODULO_VOLATILE=True` (compared against
`aot-solve-system-full.pre-experiment.json`).

Script (`aot-solve-system-full.ls`, no BOM — first bytes `120,32,61`):

```text
x = symbol("x"); y = symbol("y"); solve_system_full([x + y == 1, x - y == 3], [x, y])
```

`result.display`:

```text
SystemSolveResult(status: Solved, domain: complex, complete: True, solutions: [SystemSolution(bindings: [Binding(name: x, value: 2), Binding(name: y, value: -1)], conditions: [], exactness: Exact)], diagnostics: )
```

`SystemSolveResult` field names, in order: `status, domain, complete, solutions, diagnostics`.

The `SystemSolution` portion of the envelope (`aot-solve-system-full.json`):

```json
{
    "kind":  "Record",
    "type":  "SystemSolution",
    "fields":  [
                   {
                       "name":  "bindings",
                       "value":  {
                                     "kind":  "Array",
                                     "type":  "Vector",
                                     "shape":  [ 2 ],
                                     "elements":  [
                                                      {
                                                          "kind":  "Record",
                                                          "type":  "Binding",
                                                          "fields":  [
                                                                         { "name":  "name",
                                                                           "value":  { "kind":  "Symbolic", "pretty":  "x", "canonical":  "(sym x)", "domain":  "complex", "exact":  true, "nodeCount":  1, "freeSymbols":  [ "x" ] } },
                                                                         { "name":  "value",
                                                                           "value":  { "kind":  "Symbolic", "pretty":  "2", "canonical":  "(rat 2 1)", "domain":  "rational", "exact":  true, "nodeCount":  1, "freeSymbols":  [ ] } }
                                                                     ]
                                                      },
                                                      {
                                                          "kind":  "Record",
                                                          "type":  "Binding",
                                                          "fields":  [
                                                                         { "name":  "name",
                                                                           "value":  { "kind":  "Symbolic", "pretty":  "y", "canonical":  "(sym y)", "domain":  "complex", "exact":  true, "nodeCount":  1, "freeSymbols":  [ "y" ] } },
                                                                         { "name":  "value",
                                                                           "value":  { "kind":  "Symbolic", "pretty":  "-1", "canonical":  "(rat -1 1)", "domain":  "rational", "exact":  true, "nodeCount":  1, "freeSymbols":  [ ] } }
                                                                     ]
                                                      }
                                                  ]
                                 }
                   },
                   {
                       "name":  "conditions",
                       "value":  { "kind":  "Array", "type":  "Vector", "shape":  [ 0 ], "elements":  [ ] }
                   },
                   {
                       "name":  "exactness",
                       "value":  { "kind":  "Text", "value":  "Exact" }
                   }
               ]
}
```

(The raw `ConvertTo-Json -Depth 12` output is reproduced with only the indentation of leaf objects
collapsed; every field name, kind and value is verbatim.)

---

## 6. New test counts

| Suite | Before | After | Delta |
| --- | --- | --- | --- |
| `Lovelace.Symbolics.Tests` | 389 passed | **395 passed / 0 failed / 0 skipped** | **+6** |
| `Lovelace.Suite.Tests` | 620 passed | **621 passed / 0 failed / 0 skipped** | **+1** (plus the existing drift test extended) |
| `Lovelace.Run.Tests` | 18 passed | **18 passed / 0 failed / 0 skipped** | 0 (4 goldens regenerated) |
| `Lovelace.Studio.Tests` (extra) | 22 | **22 passed / 0 failed / 0 skipped** | 0 |

---

## 7. Not made to pass / limits (explicit)

1. **`ParameterDomain.NonNegativeIntegers` is unreachable from the language.** The enum arm exists
   (`Solve.cs:72`) but no kernel path produces it: every one of the five `new SolutionFamily(...)`
   sites passes `ParameterDomain.Integers` (`Solve.cs:763, 787, 794, 800, 802`), and a tree-wide
   search finds the identifier only in the enum declaration, the new switch arm and the new test.
   There is therefore **no** honest way to reach it through `solve_full`: the two arms have the same
   `MathDomain` projection (integer), so no observable input can distinguish them. The arm is pinned
   by calling the (now internal) projection directly from the test assembly, which the existing
   `InternalsVisibleTo` grant allows. The public surface is still covered for the reachable arm by
   `SolveFull_FamilyParameterIsSymbolic_AndItsDomainStaysADomain`.
2. **The `NonNegativeIntegers`/undeclared-arm assertions pass by construction**: both arms map to
   `MathDomain.Integer`, so the pre-change body (which ignored its argument) satisfied them; only
   the undeclared-arm `Assert.Throws` is red before the change (Step A output above). This is
   recorded rather than hidden.
3. **`exactness` is always `Exact` for system solutions.** The only system-solve path is exact
   (rational Gröbner basis + exact univariate solving + verified back-substitution); the kernel has
   no approximate system path to report. If a numeric fallback is ever added, it must set a
   different value at its construction site (`SymbolicsPlugin` projects whatever the kernel says).
4. **The frozen §C `SystemSolution(Bindings, Conditions, Exactness)` positional shape is not
   adopted** — the brief explicitly requires keeping the existing construction sites compiling and
   only adding the init-only `Exactness`. A later round that moves the member from
   `IReadOnlyDictionary<Symbol, Expr> Assignment` to `IReadOnlyList<Binding> Bindings` will change
   the kernel type, not the wire.
5. Nothing else failed. Every command above is quoted with its observed output; the only unverified
   environment claim is that no other suite consumes the changed wire (checked by grep for
   `assignment`/`"parameter"`/`"variable"` across `Lovelace.*`; only the kernel `Assignment` member,
   `DomainCondition.variable` and the new code match).

---

## 8. Reproduce

```
dotnet build LovelaceSharp.slnx -c Release --nologo
dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo
dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo
dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo
dotnet publish Lovelace.Run/Lovelace.Run.csproj -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o out/aot
out\aot\Lovelace.Run.exe --file docs/goal-cycle-3/round-3/aot-solve-system-full.ls --omit-functions
node docs/goal-cycle-3/round-2/generate-fixtures.js    # only to refresh goldens deliberately
```

Artifacts in this directory: `source.diff`, `source-after-experiment.diff`, `golden.diff`,
`fixtures-before/` (the pre-change goldens), `build-solution.log`, `test-*.log`, `step-a-*.log`,
`step-b-*.log`, `step-a-corrected-test-red.log`, `regenerate-fixtures.log`, `publish-aot.log`,
`aot-solve-system-full.ls`, `aot-solve-system-full.json`,
`aot-solve-system-full.pre-experiment.json`, `excerpt-*.txt`.
No git commit was made.
