# Round 4a — the `Enum` structured kind (decision **D2**, alignment addendum §9.1)

```text
repo    C:\Users\ricar\dev\LovelaceSharp      branch main (Cycles-3 work uncommitted)
change  one bounded, additive change: a first-class Enum structured kind, emitted for every
        enum-valued field of every rich result record
gate    dotnet build + 7 suites + Native AOT publish + the published binary on
        x = symbol("x"); solve_full(x^2 - 4 == 0, x)
```

## 0. The conflict, and the resolution implemented

| Document | Says |
|---|---|
| `docs/symbolics/a-plus-convergence-alignment-plan.md:360` (§F, frozen) | `{"kind":"Enum","type":"SolveStatus","value":"Partial"}` |
| shipped `docs/symbolics/dsh-protocol.md:37,57,65` (before this round) | `{"kind":"Text","value":"Solved"}`; the value-form list had **no** `Enum` kind |
| `docs/symbolics/a-plus-cycle-3-alignment.md:461-468` (§9.1 item 2, the addendum) | **"Resolution: implement the §F form."** — additive, AOT-safe, "no change to any existing value kind, and no reflection. `dsh-protocol.md` is amended in the same round." |

Implemented exactly that: one new payload carrier, one new `ValueKind` **appended last**, one
`PayloadMap` case in each direction, one projection arm, two formatter arms, one type-name arm,
one equality arm, the schema kinds, and the twelve emissions in the plugin. `Natural=0`,
`Integer=1`, `Real=2` are untouched (pinned by a new test).

---

## 1. Step A — the gate, observed RED

Command (per suite, logs under `docs/goal-cycle-3/round-4a/step-a-*.log`):

```powershell
dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo
```

### Observed — Lovelace.Symbolics.Tests: **19 failed**, 385 passed, 404 total

```text
  Failed ...DxSemanticClosureTests.Language_SolvePrincipalBranch_ReturnsAllRoots [8 ms]
  Failed ...EnumValueEmissionTests.SystemSolveResult_StatusAndSolutionExactness_AreEnums [359 ms]
  Failed ...DxStructuredResultsTests.SolveFull_DomainFieldAndRealOption [346 ms]
  Failed ...DxStructuredResultsTests.SimplifyFull_CarriesConditionsAndSteps [3 ms]
  Failed ...EnumValueEmissionTests.SolveResult_StatusAndCompleteness_AreEnumValues [3 ms]
  Failed ...EnumValueEmissionTests.SolutionFamily_Exactness_IsAnEnum [1 ms]
  Failed ...DxSemanticClosureTests.Language_SolveQuartic_MixedRoots_IsPartial_NotComplete [2 ms]
  Failed ...EnumValueEmissionTests.IntegrationResult_StatusAndExactness_AreEnums [1 ms]
  Failed ...EnumValueEmissionTests.LimitResult_StatusAndExactness_AreEnums [1 ms]
  Failed ...DxStructuredResultsTests.LimitFull_ExposesExistenceAndOneSidedValues [1 ms]
  Failed ...EnumValueEmissionTests.Type_OfAnEnumValue_IsItsDeclaredEnumTypeName [1 ms]
  Failed ...EnumValueEmissionTests.TransformResult_StatusAndRewriteStepClassification_AreEnums [< 1 ms]
  Failed ...EnumValueEmissionTests.SolveResult_Partial_CarriesTheSameNamesAsBefore [4 ms]
  Failed ...DxSemanticClosureTests.Language_SolveMultiplicityAndNoSolutions_AreHonest [2 ms]
  Failed ...DxStructuredResultsTests.Type_UsesOneVocabulary_ForEveryKind(source: "solve_full(symbol(\"z\")^2
         - 4 == 0, symbol(\"z\")).status"···, expected: "SolveStatus") [< 1 ms]
  Failed ...DxStructuredResultsTests.IntegrateFull_ReportsStatusAndVerification [< 1 ms]
  Failed ...DxStructuredResultsTests.SolveSystemFull_CarriesDomainCompleteAndPerSolutionExactness [3 ms]
  Failed ...DxStructuredResultsTests.SolveSystemFull_EveryEmittedSystemSolutionCarriesExactnessAndBindings [1 ms]
  Failed ...DxStructuredResultsTests.SolveFull_ExposesStatusSolutionsAndConditions [1 ms]
Failed!  - Failed:    19, Passed:   385, Skipped:     0, Total:   404 - Lovelace.Symbolics.Tests.dll
```

Representative assertion text (the exact defect):

```text
   Assert.Equal() Failure: Strings differ
Expected: "Enum"
Actual:   "Text"
   Assert.Equal() Failure: Strings differ
Expected: "SolveStatus"
Actual:   "Text"
```

### Observed — Lovelace.Suite.Tests: **3 failed**, 620 passed, 623 total

```text
  Failed Lovelace.Suite.Tests.RecordSchemaTests.SolverSchemas_DeclareTheStructuralKinds [2 ms]
   Assert.Equal() Failure: Collections differ
Expected: ["Enum", "Domain", "Boolean", "Array", "Text"]
Actual:   ["Text", "Domain", "Boolean", "Array", "Text"]
  Failed Lovelace.Suite.Tests.EnumPayloadSeamTests.EnumValue_RoundTripsThroughTheModusSeam [1 ms]
   Assert.Equal() Failure: Strings differ
Expected: "EnumValue"          # the payload that crossed the Modus boundary
Actual:   "String"
  Failed Lovelace.Suite.Tests.EnumPayloadSeamTests.EnumValues_AreEqualExactlyWhenTypeNameAndNameMatch [3 ms]
   Assert.NotNull() Failure: Value is null     # SolveStatus.Partial and Completeness.Partial compared EQUAL
Failed!  - Failed:     3, Passed:   620, Skipped:     0, Total:   623 - Lovelace.Suite.Tests.dll
```

### Observed — Lovelace.Studio.Tests: **1 failed**, 21 passed, 22 total

```text
  Failed ...StructuredPayloadTests.Evaluate_GivenSolveFull_CarriesStructuredSolveResultWithTypeNameAndSolutionsShape
   Assert.Equal() Failure: Strings differ
Expected: "Enum"
Actual:   "Text"
Failed!  - Failed:     1, Passed:    21, Skipped:     0, Total:    22 - Lovelace.Studio.Tests.dll
```

### Observed — Lovelace.Run.Tests: **1 failed**, 17 passed, 18 total

```text
  Failed Lovelace.Run.Tests.GoldenEnvelopeTests.SolveRecordCarriesTheStructuralSolveContract [3 ms]
   Assert.Equal() Failure: Strings differ
Expected: "Enum"
Actual:   "Text"
Failed!  - Failed:     1, Passed:    17, Skipped:     0, Total:    18 - Lovelace.Run.Tests.dll
```

### Observed — Lovelace.Console.Tests: 0 failed, 15 passed (kind-agnostic; unaffected)

**24 observed failures, 0 compile errors.** Step A deliberately expressed the new assertions
through the *observable* API (the structured kind name, the projection, the payload type name) so
they compile against the pre-change source and fail as **assertions** rather than as a build
error. Step B then strengthened every one of them to the typed API (`ValueKind.Enum`,
`AsEnum()`, `Assert.IsType<EnumValue>`) — assertions were only added, never weakened. The one
compile error Step B produced (`Assert.Equal("BudgetExceeded", transform.Status)` in
`RewriteProtocolTests.cs:119`, because the property became an enum) was replaced by the enum
comparison **plus** the wire-spelling assertion, i.e. strengthened.

---

## 2. Source change summary (all additive)

| file:line | change |
|---|---|
| `Lovelace.Abstractions/EnumValue.cs:20` (new file) | `public sealed record EnumValue(string TypeName, string Name);` — lives in Abstractions so a plugin never references Suite (the `RecordValue` pattern) |
| `Lovelace.Suite/Value.cs:38` | `ValueKind.Enum` **APPENDED as the last member** (comment says why: Natural/Integer/Real are load-bearing for `WidenPair`) |
| `Lovelace.Suite/Value.cs:160` | `Value(EnumValue)` constructor |
| `Lovelace.Suite/Value.cs:224` | `AsEnum()` accessor |
| `Lovelace.Suite/Value.cs:308` | `ToString` arm: `"Enum: SolveStatus.Partial"` (was the `_ => throw` default) |
| `Lovelace.Suite/PayloadMap.cs:31` | `Wrap`: `EnumValue => new Value(enumValue)` (was the "unsupported result type" throw) |
| `Lovelace.Suite/PayloadMap.cs:86` | `Unwrap`: `ValueKind.Enum => value.AsEnum()` (was the "unsupported argument kind" throw) |
| `Lovelace.Suite/StructuredProjection.cs:64` | `ValueKind.Enum => new StructuredValueDto("Enum", Type: TypeName, Value: Name)` — **was the `_ => Null` default** |
| `Lovelace.Suite/ValueFormatter.cs:26,80` | display **and** typed form render the bare member name (never `EnumValue { ... }`, never the `value.ToString()` default) |
| `Lovelace.Suite/Interpreter.cs:1022` | the type-name vocabulary arm: `ValueKind.Enum => v.AsEnum().TypeName` |
| `Lovelace.Suite/StructuralEquality.cs:46` | deep equality: equal iff **type name and name** match; difference messages name both |
| `Lovelace.Suite/RecordSchemas.cs:66,67,71,74,76,79,82,85,88,90,92,93` | exactly the 12 enum-valued fields move `Text → Enum`; `diagnostics`/`rule_id`/`method`/`policy` stay `Text` |
| `Lovelace.Symbolics/Simplify.cs:16` | new kernel enum `TransformStatus { Satisfied, BudgetExceeded, Unsatisfiable }`; `TransformResult.Status` is now that type (was a computed C# string) |
| `Lovelace.Symbolics/SymbolicsPlugin.cs:495` | `EnumField<TEnum>(typeName, member)` — the one place a type name is paired with a member |
| `Lovelace.Symbolics/SymbolicsPlugin.cs:211,217,242,249,298,306,338,342,396,404,407,411` | the twelve emissions (see §below) |
| `Lovelace.Studio/IncrementalRunner.cs:157` | `ValueHasher.Canonical` names the enum instead of falling through to `ToString()` |

### The twelve emissions (each wraps an existing enum; no new member names)

| record field | emitted as |
|---|---|
| `SolveResult.status` | `EnumValue("SolveStatus", status)` |
| `SolveResult.completeness` | `EnumValue("Completeness", …)` |
| `Solution.exactness` | `EnumValue("SolutionExactness", …)` |
| `SolutionFamily.exactness` | `EnumValue("SolutionExactness", …)` |
| `SystemSolveResult.status` | `EnumValue("SolveStatus", result.Status)` |
| `SystemSolution.exactness` | `EnumValue("SolutionExactness", …)` |
| `LimitResult.status` | `EnumValue("LimitStatus", r.Status)` |
| `LimitResult.exactness` | `EnumValue("SolutionExactness", r.Exactness)` |
| `IntegrationResult.status` | `EnumValue("IntegrationStatus", r.Kind)` |
| `IntegrationResult.exactness` | `EnumValue("SolutionExactness", …)` |
| `TransformResult.status` | `EnumValue("TransformStatus", r.Status)` |
| `RewriteStep.classification` | `EnumValue("RuleClassification", s.Classification)` |

Unchanged by design: Booleans (`complete`, `verified`, `changed`, `exists`, `exact`) stay
Boolean; `type`/`method`/`target`/`policy`/`unrepresented_reason`/`rule_id` stay Text;
`diagnostics` is untouched (a later round restructures it).

### What `type()` now answers for an enum

`ValueKind.Enum => v.AsEnum().TypeName` — the **declared enum type name**, mirroring the existing
Record arm (`type(r)` is `SolveResult`, not `Record`): `type(solve_full(x^2-4==0,x).status)` is
**`SolveStatus`**, `.completeness` is **`Completeness`**, `simplify_full(x/x).status` is
**`TransformStatus`**, `.steps[0].classification` is **`RuleClassification`**. The shipped
vocabulary test `Type_UsesOneVocabulary_ForEveryKind` stays green and gained a row pinning this arm.

---

## 3. ValueKind switch sweep (Suite / Console / Studio)

**Arms added because the site must handle Enum** (each was a silently-wrong default):

| site | before | after |
|---|---|---|
| `StructuredProjection.cs:65` | `_ => Null` (an enum field projected to JSON null) | `Enum` arm |
| `ValueFormatter.cs:30` | `_ => value.ToString()` ("EnumValue { TypeName = … }") | bare member name |
| `Value.cs:294` | `_ => throw Unknown kind` | `Enum: Type.Name` |
| `Interpreter.cs:1019` | `_ => Kind.ToString()` ("Enum") | declared enum type name |
| `StructuralEquality.cs:86` | `default => null` (**any two enums compared equal**) | type+name comparison |
| `IncrementalRunner.cs:159` (Studio) | `_ => v.ToString()` | explicit `E:Type.Name` arm |
| `PayloadMap.cs` (Wrap + Unwrap) | throw on an Enum payload | round-trip |

**Examined, no arm required** (reason in each case): `NumericOps.cs` (all arms are numeric; a
non-numeric operand reaches a *named* `InvalidOperationException`, exactly as `Text` does today);
`Interpreter.cs:499/516` (numeric narrowing), `:571` (unary negation), `:589` (factorial),
`:696` (indexing), `:849/856` (range/index conversion), `:918/937/963` (condition kinds — each
throws with the actual kind), `:1054/1061/1111/1112` (`inspect` fields answer `Null`/empty for
every kind that has no domain/canonical form — the documented uniform `Inspection` shape, not a
silent degradation); `Plotting.cs:558` (`ToReal` throws naming the kind, and an enum has no
numeric value); `TypedArrayAdapter.cs:50` (dtype metadata; `Enum` behaves like the other
non-numeric element kinds); `ModusHost.cs`, `SuiteEngine.cs:277`, `ReplSession.cs:124,247`,
`EngineHost.cs:113,299,343`, `IncrementalRunner.cs:278` (Void guards and `Kind.ToString()` —
kind-agnostic, no arm possible).

Outside the sanctioned sweep and **not touched**: `symbench/Benchmarks.cs:503-526` (a deliberately
frozen benchmark-local copy of the projection — it still maps `Enum` to `Null`, see §7) and
`Lovelace.Knowledge/Observation.cs:26-60` (it reads the runner's JSON, it is not a `ValueKind`
switch); `Lovelace.Knowledge` is not in SCOPE.

---

## 4. Step C — the goldens, regenerated and audited

```powershell
Copy-Item Lovelace.Run.Tests/fixtures/*.json docs/goal-cycle-3/round-4a/fixtures-before/
node docs/goal-cycle-3/round-2/generate-fixtures.js       # the corpus generator, unchanged
```

```text
changed fixtures: limit_result.json, nested_record.json, record.json, solve_result.json,
                  system_solve.json, transform_result.json          (the six predicted)
```

### Every leaf change in the corpus, enumerated (`node docs/goal-cycle-3/round-4a/verify-golden-diff.js`)

```text
--- limit_result.json (4 leaf changes) ---
  $.result.structured.fields.0.value.kind: "Text" -> "Enum"
  $.result.structured.fields.0.value.type ADDED = "LimitStatus"
  $.result.structured.fields.8.value.kind: "Text" -> "Enum"
  $.result.structured.fields.8.value.type ADDED = "SolutionExactness"
--- nested_record.json (6 leaf changes) ---
  $.result.structured.fields.0.value.kind: "Text" -> "Enum"
  $.result.structured.fields.0.value.type ADDED = "SolveStatus"
  $.result.structured.fields.4.value.kind: "Text" -> "Enum"
  $.result.structured.fields.4.value.type ADDED = "Completeness"
  $.result.structured.fields.6.value.elements.0.fields.5.value.kind: "Text" -> "Enum"
  $.result.structured.fields.6.value.elements.0.fields.5.value.type ADDED = "SolutionExactness"
--- record.json (8 leaf changes) ---
  $.result.structured.fields.0.value.kind: "Text" -> "Enum"
  $.result.structured.fields.0.value.type ADDED = "SolveStatus"
  $.result.structured.fields.4.value.kind: "Text" -> "Enum"
  $.result.structured.fields.4.value.type ADDED = "Completeness"
  $.result.structured.fields.5.value.elements.0.fields.3.value.kind: "Text" -> "Enum"
  $.result.structured.fields.5.value.elements.0.fields.3.value.type ADDED = "SolutionExactness"
  $.result.structured.fields.5.value.elements.1.fields.3.value.kind: "Text" -> "Enum"
  $.result.structured.fields.5.value.elements.1.fields.3.value.type ADDED = "SolutionExactness"
--- solve_result.json (8 leaf changes) ---
  $.result.structured.fields.0.value.kind: "Text" -> "Enum"
  $.result.structured.fields.0.value.type ADDED = "SolveStatus"
  $.result.structured.fields.4.value.kind: "Text" -> "Enum"
  $.result.structured.fields.4.value.type ADDED = "Completeness"
  $.result.structured.fields.5.value.elements.0.fields.3.value.kind: "Text" -> "Enum"
  $.result.structured.fields.5.value.elements.0.fields.3.value.type ADDED = "SolutionExactness"
  $.result.structured.fields.5.value.elements.1.fields.3.value.kind: "Text" -> "Enum"
  $.result.structured.fields.5.value.elements.1.fields.3.value.type ADDED = "SolutionExactness"
--- system_solve.json (4 leaf changes) ---
  $.result.structured.fields.0.value.kind: "Text" -> "Enum"
  $.result.structured.fields.0.value.type ADDED = "SolveStatus"
  $.result.structured.fields.3.value.elements.0.fields.2.value.kind: "Text" -> "Enum"
  $.result.structured.fields.3.value.elements.0.fields.2.value.type ADDED = "SolutionExactness"
--- transform_result.json (4 leaf changes) ---
  $.result.structured.fields.0.value.kind: "Text" -> "Enum"
  $.result.structured.fields.0.value.type ADDED = "TransformStatus"
  $.result.structured.fields.5.value.elements.0.fields.1.value.kind: "Text" -> "Enum"
  $.result.structured.fields.5.value.elements.0.fields.1.value.type ADDED = "RuleClassification"
files changed: limit_result.json, nested_record.json, record.json, solve_result.json,
               system_solve.json, transform_result.json
total leaf changes: 34
```

**Intent confirmation.** 34 leaf changes, and *every one* is either `kind: "Text" -> "Enum"` or an
added `type` on the very same path. No member name changed (`Solved`, `Complete`, `Partial`,
`Exact`, `AlgebraicExact`, `ParametricExact`, `Value`, `Satisfied`, `Universal` all identical);
no `display`/`typed`/`variables`/`output`/`timings` byte changed; no other fixture changed. The
human surface is therefore byte-stable while the machine surface gains the type — which is exactly
the intended revision. The full unified diff is in `golden.diff` and reproduced in Appendix A.

---

## 5. Step C — observed GREEN

```powershell
dotnet build LovelaceSharp.slnx -c Release --nologo
dotnet test <suite>/<suite>.csproj -c Release --nologo            # 7 suites
dotnet publish Lovelace.Run/Lovelace.Run.csproj -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o out/aot
out/aot/Lovelace.Run.exe --file aot-solve-full.ls --omit-functions
```

```text
Build succeeded.
    0 Warning(s)
    0 Error(s)

Lovelace.Abstractions.Tests  Passed! - Failed: 0, Passed:  20, Total:  20
Lovelace.Knowledge.Tests     Passed! - Failed: 0, Passed:  28, Total:  28
Lovelace.Symbolics.Tests     Passed! - Failed: 0, Passed: 404, Total: 404
Lovelace.Suite.Tests         Passed! - Failed: 0, Passed: 624, Total: 624
Lovelace.Studio.Tests        Passed! - Failed: 0, Passed:  22, Total:  22
Lovelace.Run.Tests           Passed! - Failed: 0, Passed:  18, Total:  18
Lovelace.Console.Tests       Passed! - Failed: 0, Passed:  15, Total:  15

publish (tail):
  Lovelace.Run -> ...\Lovelace.Run\bin\Release\net10.0\win-x64\Lovelace.Run.dll
  Generating native code
  Lovelace.Run -> C:\Users\ricar\dev\LovelaceSharp\out\aot\
```

`Lovelace.Symbolics.Tests` 404 and `Lovelace.Suite.Tests` 624 are the *post-change* totals; the
pre-change totals are the Step-A totals minus the new tests (404-9 = 395, 623-2 = 621), i.e. this
change adds 12 test cases and modifies assertions in 4 existing files. **Label: derived from the
Step-A logs, not a separately measured baseline.**

### The published binary on the required script

```lovelace
x = symbol("x"); solve_full(x^2 - 4 == 0, x)
```

```text
result.kind      = Record   result.type = SolveResult
status           = {"kind":"Enum","type":"SolveStatus","value":"Solved"}
domain           = {"kind":"Domain","domain":"complex"}
complete         = {"kind":"Boolean","value":"true"}
completeness     = {"kind":"Enum","type":"Completeness","value":"Complete"}
solutions[0].exactness = {"kind":"Enum","type":"SolutionExactness","value":"Exact"}
unrepresented_reason  = {"kind":"Text","value":""}   (unchanged Text)
```

That is the §F form (`{"kind":"Enum","type":"SolveStatus","value":"…"}`) produced by the published
Native AOT executable. The full envelope is `aot-solve-full.json`; the extractor is
`aot-field-shapes.js`.

### Packaging gate (`.github/workflows/ci.yml`)

The value assertions were kept and the kind assertions added for `status`, `completeness` and
each solution's `exactness` (`status.kind == 'Enum'`, `status.type == 'SolveStatus'`, …), in both
the complete-solve and the partial-solve smoke.

**Defect found and repaired in the same file (pre-existing, not introduced here).** The smoke
asserted `fields['complete']['value'] is True` / `is False`. A structured Boolean crosses as the
*string* `"true"`/`"false"` — `dsh-protocol.md` documents `{ "kind": "Boolean", "value": "true" }`
and `Lovelace.Run.Tests/fixtures/record.json` pins it — so the step would have aborted on that
line before ever reaching the new Enum assertions. Repaired to `== 'true'`/`== 'false'` **plus**
`kind == 'Boolean'`, i.e. the same intent with the representation corrected and one assertion added.

GitHub Actions cannot run here (no runner, no `python3` on this machine), so the workflow's five
scenarios were re-executed against the published binary with node performing the same JSON
assertions: `node docs/goal-cycle-3/round-4a/ci-smoke-check.js` →
**ALL CI SMOKE ASSERTIONS PASS (local node proxy)** (30/30, `ci-smoke-check.log`). This is a local
proxy for the assertions, **not** a run of the workflow; the workflow itself is unverified here.

---

## 6. New test counts

| suite | new test cases | new file | assertions changed in existing files |
|---|---|---|---|
| Lovelace.Symbolics.Tests | **+9** (8 facts in `EnumValueEmissionTests` + 1 theory row in `Type_UsesOneVocabulary_ForEveryKind`) | `Lovelace.Symbolics.Tests/EnumValueEmissionTests.cs` | `DxStructuredResultsTests.cs`, `DxSemanticClosureTests.cs`, `RewriteProtocolTests.cs` |
| Lovelace.Suite.Tests | **+3** | `Lovelace.Suite.Tests/EnumPayloadSeamTests.cs` | `RecordSchemaTests.cs` (12 schema kinds + 5 new record schemas pinned) |
| Lovelace.Run.Tests | 0 | — | `GoldenEnvelopeTests.cs` (kind/type assertions added) |
| Lovelace.Studio.Tests | 0 | — | `StructuredPayloadTests.cs` (Text → Enum, +type, +value) |
| Lovelace.Console.Tests | 0 | — | — |

Required tests, and where they live:

* SolveResult `status`+`completeness` are Enum values of type `SolveStatus`/`Completeness` with the
  same member names — `EnumValueEmissionTests.SolveResult_StatusAndCompleteness_AreEnumValues`,
  `…SolveResult_Partial_CarriesTheSameNamesAsBefore`;
* LimitResult `status`+`exactness` — `…LimitResult_StatusAndExactness_AreEnums`;
* TransformResult `status` of type `TransformStatus` — `…TransformResult_StatusAndRewriteStepClassification_AreEnums`;
* RewriteStep `classification` of type `RuleClassification` — same test;
* EnumValue round-trips the Modus seam **through a builtin call** —
  `EnumPayloadSeamTests.EnumValue_RoundTripsThroughTheModusSeam` (a test plugin's `enum_echo`
  builtin records the payload it received: `Assert.IsType<EnumValue>` with both `TypeName` and
  `Name` preserved);
* display **and** typed rendering show the bare name — asserted inside every `AssertEnumField`
  helper (`Format` and `FormatTyped` both equal the member name).

---

## 7. Not verified / residual (stated plainly)

1. **GitHub Actions was not executed.** No runner here and no `python3` locally; the aot-smoke
   assertions were re-run with node over the same published binary (§5). The `fast-tests` job's
   suite list is unchanged except that it already contained Run.Tests.
2. **`MatrixSolveResult.status` / `MatrixInverseResult.status` remain `Text`.** They are *not* in the
   EMISSIONS enumeration, their values are string literals written by the core
   (`Lovelace.Suite/Interpreter.cs:1274,1279,1333`), there is no kernel enum for them, and they have
   no `RecordSchemas` entry (that registry item is D8). Deliberately untouched — if the reviewer
   wants them included it is a follow-on change with its own schema entry, not a silent extra here.
3. **`symbench/Benchmarks.cs:503-526`** keeps its frozen local projection where a new kind falls to
   `Null`. It is a benchmark harness outside SCOPE and its corpus builds its own SolveResult with
   string fields; if it is ever pointed at an enum-valued record it would report `Null` for that
   field. Flagged, not fixed.
4. `ValueKind.Enum` is appended last as instructed, so a mixed enum/numeric operation still fails in
   `WidenPair` with `"Cannot widen from Natural to Enum: …"` rather than a message naming the
   operator. Not silent, identical in shape to `Text` today, and out of SCOPE to reword.
5. The goldens under `Lovelace.Run.Tests/fixtures/` are untracked in this working tree, so the
   before/after state is preserved as `fixtures-before/` + `verify-golden-diff.js` rather than as
   git history.

---

## Appendix A — the golden diff (verbatim, git's CRLF warnings stripped)

```diff
########## limit_result.json ##########
--- a/docs/goal-cycle-3/round-4a/fixtures-before/limit_result.json
+++ "b/C:\\Users\\ricar\\dev\\LovelaceSharp\\Lovelace.Run.Tests\\fixtures\\limit_result.json"
@@ -15,7 +15,8 @@
         {
           "name": "status",
           "value": {
-            "kind": "Text",
+            "kind": "Enum",
+            "type": "LimitStatus",
             "value": "Value"
           }
         },
@@ -86,7 +87,8 @@
         {
           "name": "exactness",
           "value": {
-            "kind": "Text",
+            "kind": "Enum",
+            "type": "SolutionExactness",
             "value": "Exact"
           }
         },
########## nested_record.json ##########
--- a/docs/goal-cycle-3/round-4a/fixtures-before/nested_record.json
+++ "b/C:\\Users\\ricar\\dev\\LovelaceSharp\\Lovelace.Run.Tests\\fixtures\\nested_record.json"
@@ -15,7 +15,8 @@
         {
           "name": "status",
           "value": {
-            "kind": "Text",
+            "kind": "Enum",
+            "type": "SolveStatus",
             "value": "Solved"
           }
         },
@@ -50,7 +51,8 @@
         {
           "name": "completeness",
           "value": {
-            "kind": "Text",
+            "kind": "Enum",
+            "type": "Completeness",
             "value": "Complete"
           }
         },
@@ -139,7 +141,8 @@
                   {
                     "name": "exactness",
                     "value": {
-                      "kind": "Text",
+                      "kind": "Enum",
+                      "type": "SolutionExactness",
                       "value": "ParametricExact"
                     }
                   }
########## record.json ##########
--- a/docs/goal-cycle-3/round-4a/fixtures-before/record.json
+++ "b/C:\\Users\\ricar\\dev\\LovelaceSharp\\Lovelace.Run.Tests\\fixtures\\record.json"
@@ -15,7 +15,8 @@
         {
           "name": "status",
           "value": {
-            "kind": "Text",
+            "kind": "Enum",
+            "type": "SolveStatus",
             "value": "Solved"
           }
         },
@@ -50,7 +51,8 @@
         {
           "name": "completeness",
           "value": {
-            "kind": "Text",
+            "kind": "Enum",
+            "type": "Completeness",
             "value": "Complete"
           }
         },
@@ -101,7 +103,8 @@
                   {
                     "name": "exactness",
                     "value": {
-                      "kind": "Text",
+                      "kind": "Enum",
+                      "type": "SolutionExactness",
                       "value": "Exact"
                     }
                   }
@@ -145,7 +148,8 @@
                   {
                     "name": "exactness",
                     "value": {
-                      "kind": "Text",
+                      "kind": "Enum",
+                      "type": "SolutionExactness",
                       "value": "Exact"
                     }
                   }
########## solve_result.json ##########
--- a/docs/goal-cycle-3/round-4a/fixtures-before/solve_result.json
+++ "b/C:\\Users\\ricar\\dev\\LovelaceSharp\\Lovelace.Run.Tests\\fixtures\\solve_result.json"
@@ -15,7 +15,8 @@
         {
           "name": "status",
           "value": {
-            "kind": "Text",
+            "kind": "Enum",
+            "type": "SolveStatus",
             "value": "Partial"
           }
         },
@@ -50,7 +51,8 @@
         {
           "name": "completeness",
           "value": {
-            "kind": "Text",
+            "kind": "Enum",
+            "type": "Completeness",
             "value": "Partial"
           }
         },
@@ -101,7 +103,8 @@
                   {
                     "name": "exactness",
                     "value": {
-                      "kind": "Text",
+                      "kind": "Enum",
+                      "type": "SolutionExactness",
                       "value": "AlgebraicExact"
                     }
                   }
@@ -145,7 +148,8 @@
                   {
                     "name": "exactness",
                     "value": {
-                      "kind": "Text",
+                      "kind": "Enum",
+                      "type": "SolutionExactness",
                       "value": "AlgebraicExact"
                     }
                   }
########## system_solve.json ##########
--- a/docs/goal-cycle-3/round-4a/fixtures-before/system_solve.json
+++ "b/C:\\Users\\ricar\\dev\\LovelaceSharp\\Lovelace.Run.Tests\\fixtures\\system_solve.json"
@@ -15,7 +15,8 @@
         {
           "name": "status",
           "value": {
-            "kind": "Text",
+            "kind": "Enum",
+            "type": "SolveStatus",
             "value": "Solved"
           }
         },
@@ -136,7 +137,8 @@
                   {
                     "name": "exactness",
                     "value": {
-                      "kind": "Text",
+                      "kind": "Enum",
+                      "type": "SolutionExactness",
                       "value": "Exact"
                     }
                   }
########## transform_result.json ##########
--- a/docs/goal-cycle-3/round-4a/fixtures-before/transform_result.json
+++ "b/C:\\Users\\ricar\\dev\\LovelaceSharp\\Lovelace.Run.Tests\\fixtures\\transform_result.json"
@@ -15,7 +15,8 @@
         {
           "name": "status",
           "value": {
-            "kind": "Text",
+            "kind": "Enum",
+            "type": "TransformStatus",
             "value": "Satisfied"
           }
         },
@@ -86,7 +87,8 @@
                   {
                     "name": "classification",
                     "value": {
-                      "kind": "Text",
+                      "kind": "Enum",
+                      "type": "RuleClassification",
                       "value": "Universal"
                     }
                   },
```

## Appendix B — the CI smoke proxy, observed

```text
ok   solve: ok/protocol/versions
ok   solve: record shape
ok   solve: status value
ok   solve: status kind is Enum
ok   solve: status type is SolveStatus
ok   solve: completeness kind is Enum
ok   solve: completeness type is Completeness
ok   solve: complete kind is Boolean
ok   solve: complete is "true"
ok   solve: domain
ok   solve: solutions shape
ok   solve: first solution symbolic
ok   solve: exactness value
ok   solve: exactness kind is Enum
ok   solve: exactness type
ok   partial: status Partial
ok   partial: status kind is Enum
ok   partial: status type is SolveStatus
ok   partial: completeness kind is Enum
ok   partial: completeness Partial
ok   partial: complete kind is Boolean
ok   partial: complete is "false"
ok   partial: unrepresented_count 2
ok   jacobian: rank-2 shape
ok   jacobian: first element
ok   print: output captured
ok   print: result is 2
ok   error: ok false
ok   error: message names the domain
ok   error: code and category
ALL CI SMOKE ASSERTIONS PASS (local node proxy)
```
