# Round 2 triage — `wip-r22-caps-UNVERIFIED.diff` (rows 3 and 4)

**VERDICT: LANDABLE AS-IS** — the patch applies cleanly to the base, builds with 0 warnings /
0 errors, the whole solution's 15 test projects pass on the patched tree, and the new tests
demonstrably fail on a pre-fix control tree, so rows 3 (`evalf(f, 0)`) and 4 (`capabilities()`
under-claims) are closed by tests with teeth.

Base = `HEAD` = `300f6bb`. Host = Windows PowerShell 5.1, `dotnet` SDK **10.0.103**, TFM `net10.0`.
Scratch trees (both created by this triage, main working tree never written to):
`.worktrees/c6-r22` (patched) and `.worktrees/c6-r22-control` (control). Every command below ran
inside one of those two trees; both the patched and the control runs are on the same platform.

## Steps, commands, observed results

| # | exact command | observed result |
|---|---|---|
| 1 | `git -C C:\Users\ricar\dev\LovelaceSharp worktree add .worktrees/c6-r22 HEAD` | exit 0 — `HEAD is now at 300f6bb docs(cycle-6): round 1 landed…` |
| 2 | `git -C .worktrees/c6-r22 apply --check --verbose docs/goal-cycle-5/patches/wip-r22-caps-UNVERIFIED.diff` | **exit 0**, no diagnostics — no conflicting hunk |
| 3 | `git -C .worktrees/c6-r22 apply --verbose <patch>` | **exit 0**; `Applied patch …cleanly` ×5; `M` on the 4 existing files + `?? Lovelace.Symbolics.Tests/EvalfDigitCountContractTests.cs` |
| 4 | `dotnet build Lovelace.Symbolics.Tests\Lovelace.Symbolics.Tests.csproj --configuration Release --nologo` | exit 0 — `Build succeeded. 0 Warning(s) 0 Error(s)` |
| 5 | `dotnet build Lovelace.Run.Tests\Lovelace.Run.Tests.csproj --configuration Release --nologo` | exit 0 — `Build succeeded. 0 Warning(s) 0 Error(s)` |
| 6 | `dotnet test Lovelace.Symbolics.Tests\…csproj -c Release --no-build --nologo` | exit 0 — `Passed!  - Failed:     0, Passed:  1065, Skipped:     8, Total:  1073` |
| 7 | `dotnet test Lovelace.Run.Tests\…csproj -c Release --no-build --nologo` | exit 0 — `Passed!  - Failed:     0, Passed:   186, Skipped:     0, Total:   186` |
| 8 | `dotnet test LovelaceSharp.slnx --configuration Release --nologo` | **exit 0** — 15 test projects, every one `Passed!`, 0 failed (5261 passed / 8 skipped overall) |
| 9 | `git -C …c6-r22-control apply --include=Lovelace.Run.Tests/BuiltinSurfaceContractTests.cs --include=Lovelace.Run.Tests/fixtures/capabilities.json --include=Lovelace.Symbolics.Tests/CapabilitiesBuiltinTests.cs --include=Lovelace.Symbolics.Tests/EvalfDigitCountContractTests.cs <patch>` | exit 0 — `Skipped patch 'Lovelace.Symbolics/SymbolicsPlugin.cs'.`; `git diff --stat -- Lovelace.Symbolics/SymbolicsPlugin.cs` empty (product pristine) |
| 10 | control builds (same two `dotnet build` commands) | exit 0 both |
| 11 | `dotnet test Lovelace.Symbolics.Tests\… --no-build` (control) | **exit 1** — `Failed!  - Failed:    38, Passed:  1027, Skipped:     8, Total:  1073` |
| 12 | `dotnet test Lovelace.Run.Tests\… --no-build` (control) | **exit 1** — `Failed!  - Failed:    12, Passed:   174, Skipped:     0, Total:   186` |
| 13 | `dotnet Lovelace.Run.dll --eval "evalf(sin(1), 0)" --json --omit-functions` (both trees) | base: exit 0, `ok:true`, `display "0.8"`; patched: exit 1, `InvalidArgument/TypeMismatch`, `got 0.` |
| 14 | `dotnet Lovelace.Run.dll --file ex7.ls --omit-functions` (`ex7.ls` = `evalf(sin(1), 0)`) | base: exit 0, `"display":"0.8"`; patched: exit 1, `InvalidArgument/TypeMismatch` |
| 15 | `dotnet Lovelace.Run.dll --eval "capabilities()" --json --omit-functions` (patched) | `entryCount=19`; `firstEntryFields=operation_class,code,category,message,trigger,scope`; `oldIdPresent=False` |

Logs kept verbatim in the scratch trees: `patched-build-*.log`, `patched-test-*.log`,
`patched-test-full.log` (`.worktrees/c6-r22/`) and `control-build-*.log`, `control-test-*.log`
(`.worktrees/c6-r22-control/`); probe script `probe-evalf.ps1`, script `ex7.ls`.

## Control result (step 5 of the brief) and what it proves

Control = the patch's four test-side files (3 modified + 1 new, including the golden fixture),
applied to the same base **without** `Lovelace.Symbolics/SymbolicsPlugin.cs`. Both projects
report the *same totals* as the patched run (1073 and 186), so the new test file compiles and
executes in the control — it is a real run, not a build failure.

Failing in control, i.e. **these tests have teeth** (12 + 38 = 50 cases):

* `Lovelace.Run.Tests` (12): all 10 `BuiltinSurfaceContractTests.Evalf_RefusesADigitCountItCannotHonour_AsARecoverableArgumentError` cases — first error `Assert.Equal() Failure: Values differ / Expected: 1 / Actual: 0` at `BuiltinSurfaceContractTests.cs:108`; `Evalf_DigitsOne_IsTheSmallestAcceptedCount` — same 1-vs-0 failure at `BuiltinSurfaceContractTests.cs:133`; `GoldenEnvelopeTests.EnvelopeMatchesGoldenFixture(name: "capabilities")` — `capabilities: envelope does not match the golden fixture: $.result.structured.fields[2].value.elements: array length 19 != 16`.
* `Lovelace.Symbolics.Tests` (38): 27 × `EvalfDigitCountContractTests.NonPositiveDigits_AreARecoverableArgumentError_ForEveryShape` and 4 × `DigitsOutsideTheAdvertisedRange_AreTheSameTypedRefusal` (failed for every non-positive / out-of-range count — the pre-fix tree *answers*: `evalf(sin(1),0)`=`0.8`, `evalf(cos(1),0)`=`0.5`, `evalf(exp(1),0)`=`2.7`, `evalf(1/3,0)`=`0`, `evalf(sqrt(2),0)`=`1`, `evalf(pi,0)`=`3`, and `evalf(sin(1),10^30)` → exit 1 `ArithmeticError/DomainError` "Value was either too large or too small for an Int64."); plus 7 `CapabilitiesBuiltinTests` cases.
  * 6 of those 7 fail with `System.InvalidOperationException : Sequence contains no matching element` — the new `scope` field is read through `Field()` → `record.Fields.First(f => f.Name == name)` (`CapabilitiesBuiltinTests.cs:97-98`) and is absent at base (`UnsupportedOperations_AreRecordsCarryingCodeCategoryAndMessage`, `PowerExponentClass_RefusesWhatItSays_AndNamesWhatIsSupported`, `SymbolicPlot_IsATypedRefusal_NeverAnInternalInvariantFailure`, `AdvertisedCodesAndCategories_MatchTheLiveEnvelope`, `SolveDomainStatement_NamesItsOperation_AndEveryDomainStaysUsableElsewhere`, `ExactnessBestEffort_IsBackedByLiveRefusalsTheListDoesNotCarry`).
  * 1 is a direct behavioural tooth: `IntegrationRefusal_IsTypedStructuredAndClassLevel` — `Assert.Equal() Failure: Values differ / Expected: 2 / Actual: 1` (the `integrate_full` record carries one top-level diagnostic at base, two after the patch).

**Pass in both trees — no teeth (must be reported as such).** `EvalfDigitCountContractTests.DigitsOne_IsAccepted_ForEveryShape` (9 cases) and `CountsThatWereHonoured_KeepAnsweringTheRequestedCount` (1 case): 0 failures in the control. These 10 of the new file's 41 cases are regression guards for behaviour the patch does not change (the file's own docstrings say so), not falsifiers of rows 3/4. The golden fixture `Lovelace.Run.Tests/fixtures/capabilities.json` also fails in the control, but that is snapshot coupling to the product change (it *is* the product's live output), not independent evidence of teeth.

## Seam check (new file / new member / new wire field)

* **New file**: `Lovelace.Symbolics.Tests/EvalfDigitCountContractTests.cs` (168 lines, public class at line 30). No production file is added.
* **New wire field**: `scope` — a 6th field on **every** element of `capabilities().unsupported_operations`, `{"kind":"Null"}` when unstated (live probe step 15; `SymbolicsPlugin.cs:974`). The golden `capabilities.json` is updated in lockstep (array shape 16 → 19, 6th field on each entry, `display`/`typed` re-rendered).
* **New wire codes** (3): `solve.unevaluated-in-record-diagnostics`, `system-solve.unevaluated-in-record-diagnostics`, `integration.no-closed-form-in-record-diagnostics`.
* **Renamed wire code**: `pow.non-integer-exponent` → `pow.non-integer-exponent-of-a-positive-base` (`SymbolicsPlugin.cs:763,848`); the old id is gone from the live output (`oldIdPresent=False`) and an assertion forbids re-adding it.
* **Changed record shape**: `integrate_full`'s `diagnostics` now carries **two** top-level elements when a note exists (was one) — `SymbolicsPlugin.cs:636`. The `integration_result` golden is unaffected because its script `integrate_full(x^2, x)` produces no note.
* **No new public API**: `DigitCount` (`SymbolicsPlugin.cs:1682`), `IntegrationDiagnostics` (:636) and `UnsupportedCapability` (:974) are all `private static`.
* No other test project in the solution references the changed surface (repo-wide grep for `UnsupportedCapability`/`integration.unevaluated`/`capabilities()` in `*.cs` returns only `Lovelace.Symbolics`, its own test project and `Lovelace.Run.Tests`); step 8 confirms it empirically.

## Discrepancy found (not a landability blocker)

Row 3's *stated* symptom is stale at this base. `docs/goal-cycle-6/goal.md:36` says
"`evalf(f, 0)` raises `InternalError/InternalInvariantFailure` (NullReferenceException) on valid
input", and the patch's comments repeat it (`…// used to raise NullReferenceException`, patch lines
22-23), but at `300f6bb` `evalf(sin(1), 0)` answers `0.8` with exit 0 through **both** `--eval`
and `--file` (steps 13-14). `git log --oneline 9e761ba..HEAD -- Lovelace.Symbolics/SymbolicsPlugin.cs`
returns one commit, `86a5fc8` ("evalf honours its digit count…"), which rewrote the `evalf`
argument path; the cycle-5 audit's `InternalError` (audit2 B1-differential.md:1482, EX7) was
recorded at `9e761ba` and is no longer reachable here. The tests still fail on the pre-fix tree,
so the row is closed — but for the "silently answers" reason (`0.8`/`0`/`1`/`3`), not the
internal-invariant reason the comments cite.

## What I could not determine

1. Whether the `InternalError` shape survives in the **Native AOT** `Lovelace.Run.exe` (the audit's EX7 used the published exe). I built and ran only the framework-dependent `Lovelace.Run.dll`; the AOT publish was not exercised.
2. Which 8 `Lovelace.Symbolics.Tests` cases are skipped (skipped in both trees, count identical) — I did not identify them or check whether the patch should have enabled any.
3. CI's own leg: no `--collect:"XPlat Code Coverage"`, no Linux runner, no `[Trait("Category","Heavy")]` filter on `Lovelace.Real.Tests` (I ran that project whole). All runs are Windows/.NET 10.0.103.
4. Whether the two `scope` texts enumerate *every* refused shape — I verified the shapes the tests drive, not an exhaustive sweep of non-integer exponents.
5. Whether anything outside this repository (e.g. `harness/`, Studio web bundles) keys on the removed id `pow.non-integer-exponent`. A ripgrep over the checkout found it only in `docs/`, `Lovelace.Symbolics/` and the two test projects, but ripgrep honours `.gitignore`, so ignored/untracked bundles were not searched.
