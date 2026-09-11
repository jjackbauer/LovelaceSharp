# Cycle 3 — Round 5: NoSolutions/completeness pairing + builtin arity validation

Branch `main` @ `35e2609` plus the uncommitted Cycle-3 work. No git commit was made; no
`dotnet build-server shutdown` was run.

Two bounded parts, implemented test-first in one round:

* **Part A** — `NoSolutions` is a PROVABLY EMPTY solution set over the requested domain, so it is a
  complete answer: `complete: true` / `completeness: Complete`. `complete` and `completeness` are
  now derived by ONE named function of the effective status, used by BOTH result records, which also
  closes RISK-003 (the latent branch that could emit `NoSolutions` with `completeness: Complete`
  while `complete` was false).
* **Part B** — the arity DECLARED on a `BuiltinDescriptor` (`Parameters`, `MinArity`, `Variadic`) is
  now enforced once, in `Lovelace.Suite/ModusHost.cs`, at the three handler-building registration
  sites. `compile_full(x^2 + 1)` fails with a typed, recoverable argument error naming the builtin
  and both counts instead of a framework index exception.

## Where the mapping lives

`Lovelace.Symbolics/SymbolicsPlugin.cs:25` — `public static class SolveCompletenessMapping`, method
`Of(SolveStatus status, Completeness partialSubset = Completeness.Unknown)` returning
`(bool Complete, Completeness Completeness)`:

```csharp
public static (bool Complete, Completeness Completeness) Of(
    SolveStatus status, Completeness partialSubset = Completeness.Unknown) => status switch
{
    SolveStatus.Solved => (true, Completeness.Complete),
    SolveStatus.NoSolutions => (true, Completeness.Complete),
    SolveStatus.Partial => (false, Completeness.Partial),
    SolveStatus.Unevaluated => (false, Completeness.Unknown),
    SolveStatus.BudgetExceeded => (false, partialSubset),
    _ => (false, Completeness.Unknown),
};
```

It sits in the file that BUILDS both `SolveResult` and `SystemSolveResult`, and both builders call it
(`SymbolicsPlugin.cs:373` for the system record, `SymbolicsPlugin.cs:442` for `solve_full`), so the two
fields of a record are read off one expression and the two records cannot drift apart.
`Lovelace.Symbolics/Solvers/Solve.cs` is outside this round's scope, so the kernel-level
`SolutionSet.Complete` / `SystemSolveResult.Complete` properties were left untouched; they are now
consulted ONLY as the `BudgetExceeded` fallback (`partialSubset`). **Inference (labelled):** a future
round could move the mapping next to `SolveStatus` once `Solve.cs` is in scope; the wire contract is
identical either way.

---

# STEP A — the new assertions fail before the change

## Part A (in-process, `Lovelace.Symbolics.Tests/SolveCompletenessPairingTests.cs`)

Command: `dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo`
(full log: `docs/goal-cycle-3/round-5/step-a-symbolics.log`)

```text
  Failed Lovelace.Symbolics.Tests.SolveCompletenessPairingTests.NoSolutions_IsACompleteAnswer(expression: "solve_full(x/x == 0, x)") [54 ms]
  Error Message:
   solve_full(x/x == 0, x): a provably empty solution set is a complete answer
  Failed Lovelace.Symbolics.Tests.SolveCompletenessPairingTests.NoSolutions_IsACompleteAnswer(expression: "solve_full(x^2 + 1 == 0, x, real)") [< 1 ms]
  Error Message:
   solve_full(x^2 + 1 == 0, x, real): a provably empty solution set is a complete answer
  Failed Lovelace.Symbolics.Tests.SolveCompletenessPairingTests.NoSolutions_IsACompleteAnswer(expression: "solve_full((x^2-1)/(x^2-1) == 0, x)") [4 ms]
  Error Message:
   solve_full((x^2-1)/(x^2-1) == 0, x): a provably empty solution set is a complete answer
  Failed Lovelace.Symbolics.Tests.SolveCompletenessPairingTests.SystemSolve_NoSolutions_IsACompleteAnswer [1 ms]
  Error Message:
   an inconsistent polynomial system has a provably empty solution set

Failed!  - Failed:     4, Passed:   421, Skipped:     0, Total:   425, Duration: 13 s - Lovelace.Symbolics.Tests.dll (net10.0)
```

The same defect observed on the PUBLISHED wire BEFORE the change (pre-change run of
`docs/goal-cycle-3/round-5/probe-baseline.ps1`, transcribed verbatim from that run's output):

```text
---- a1_ns_ratio   ok=true kind=Record status=NoSolutions complete=false completeness=Unknown unrepresented_count=0
---- a2_ns_xoverx  ok=true kind=Record status=NoSolutions complete=false completeness=Unknown unrepresented_count=0
---- a3_ns_real    ok=true kind=Record status=NoSolutions complete=false completeness=Unknown unrepresented_count=0
---- a4_partial    ok=true kind=Record status=Partial      complete=false completeness=Partial   unrepresented_count=2
---- a5_solved     ok=true kind=Record status=Solved       complete=true  completeness=Complete  unrepresented_count=0
---- a6_unevaluated ok=true kind=Record status=Unevaluated complete=false completeness=Unknown   unrepresented_count=4
---- a7_system_ns  ok=true kind=Record status=NoSolutions  complete=false completeness=
```

## Part A2 — the mapping itself does not exist yet

After adding `Lovelace.Symbolics.Tests/SolveCompletenessMappingTests.cs` (the direct mapping drive
required for the statuses the language cannot reach), the project fails to compile — logged in
`docs/goal-cycle-3/round-5/step-a2-symbolics.log`:

```text
Lovelace.Symbolics.Tests\SolveCompletenessMappingTests.cs(40,48): error CS0103: The name 'SolveCompletenessMapping' does not exist in the current context
Lovelace.Symbolics.Tests\SolveCompletenessMappingTests.cs(49,53): error CS0103: The name 'SolveCompletenessMapping' does not exist in the current context
Lovelace.Symbolics.Tests\SolveCompletenessMappingTests.cs(51,13): error CS0103: The name 'SolveCompletenessMapping' does not exist in the current context
Lovelace.Symbolics.Tests\SolveCompletenessMappingTests.cs(77,22): error CS0103: The name 'SolveCompletenessMapping' does not exist in the current context
```

## Part B (in-process host rule, `Lovelace.Suite.Tests/ModusArityTests.cs`)

Command: `dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo`
(full log: `docs/goal-cycle-3/round-5/step-a-suite.log`)

```text
  Failed Lovelace.Suite.Tests.ModusArityTests.ParametersOnlyRegistration_UsesTheSameRule [2 ms]
  Error Message:
   Assert.ThrowsAny() Failure: No exception was thrown
Expected: typeof(System.Exception)
  Failed Lovelace.Suite.Tests.ModusArityTests.ExactCount_RequiresTheDeclaredCount [< 1 ms]
  Error Message:
   Assert.ThrowsAny() Failure: No exception was thrown
Expected: typeof(System.Exception)
  Failed Lovelace.Suite.Tests.ModusArityTests.MinArity_AllowsTheOptionalTail_AndStillRefusesLess [< 1 ms]
  Error Message:
   Assert.ThrowsAny() Failure: No exception was thrown
Expected: typeof(System.Exception)
  Failed Lovelace.Suite.Tests.ModusArityTests.Variadic_RemovesTheUpperBound_AndHonoursTheLowerOne [< 1 ms]
  Error Message:
   Assert.ThrowsAny() Failure: No exception was thrown
Expected: typeof(System.Exception)

Failed!  - Failed:     4, Passed:   625, Skipped:     0, Total:   629, Duration: 8 s - Lovelace.Suite.Tests.dll (net10.0)
```

## Part B — the reproduced user mistake, through the published runner envelope

Command: `dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo`
(full log: `docs/goal-cycle-3/round-5/step-a-run.log`)

```text
  Failed Lovelace.Run.Tests.ArityValidationTests.TooFewArguments_IsATypedStructuralError_NeverAnInternalFailure [4 ms]
  Error Message:
   Assert.Equal() Failure: Strings differ
           ↓ (pos 0)
Expected: "compile_full(): expected 2 arguments; got"···
Actual:   "Index was out of range. Must be non-negat"···
           ↑ (pos 0)

Failed!  - Failed:     1, Passed:    32, Skipped:     0, Total:    33, Duration: 751 ms - Lovelace.Run.Tests.dll (net10.0)
```

The code/category/recoverable assertions in that test already held before the change (the framework
exception that escaped happened to be an `ArgumentOutOfRangeException`, an `ArgumentException`
subclass): the message was a framework string, and the CLASSIFICATION is what the new typed error
must keep. Both are asserted through the envelope.

---

# STEP B — the source change

## B1. One mapping, used by both records

| Change | Location |
|---|---|
| `SolveCompletenessMapping` (the one place) | `Lovelace.Symbolics/SymbolicsPlugin.cs:12-41` |
| `SystemSolveResult` record now derives `complete` from it | `Lovelace.Symbolics/SymbolicsPlugin.cs:373` |
| `SolveResult` (`solve_full`) derives BOTH `complete` and `completeness` from it, after the `Solved → NoSolutions` re-derivation | `Lovelace.Symbolics/SymbolicsPlugin.cs:442` |

Replaced expressions:

```csharp
// before (system record)
new RecordField("complete", result.Complete == Completeness.Complete),
// after
var (complete, _) = SolveCompletenessMapping.Of(result.Status, result.Complete);
new RecordField("complete", complete),

// before (solve_full)
new RecordField("complete", status == SolveStatus.Solved),
new RecordField("completeness", EnumField("Completeness",
    status == SolveStatus.Solved ? Completeness.Complete : set.Complete)),
// after
var (complete, completeness) = SolveCompletenessMapping.Of(status, set.Complete);
new RecordField("complete", complete),
new RecordField("completeness", EnumField("Completeness", completeness)),
```

Because both fields are destructured from one call, the RISK-003 three-way contradiction (status
`NoSolutions` + `completeness: Complete` + `complete: false`) is unreachable by construction.
No other status changed: `Solved`, `Partial` and `Unevaluated` keep exactly their previous pairs
(asserted and probe-verified below).

## B2. Arity enforcement

`Lovelace.Suite/ModusHost.cs`:

| Change | Location |
|---|---|
| `CheckArity` (the one rule) | `Lovelace.Suite/ModusHost.cs:115-132` |
| parameters-only overload handler | `Lovelace.Suite/ModusHost.cs:77` |
| descriptor overload (raw-object channel) handler | `Lovelace.Suite/ModusHost.cs:93` |
| descriptor overload (ScalarResult channel) handler | `Lovelace.Suite/ModusHost.cs:108` |
| `BuiltinArityException : ArgumentException` (typed, structural) | `Lovelace.Suite/ModusHost.cs:211-256` |

```csharp
private static void CheckArity(string name, IReadOnlyList<string> parameters, bool variadic,
    int minArity, IReadOnlyList<Value> args)
{
    int declared = parameters.Count;
    int min = minArity >= 0 ? Math.Min(minArity, declared) : declared;
    int max = variadic ? int.MaxValue : declared;
    if (args.Count >= min && args.Count <= max)
        return;
    throw new BuiltinArityException(name, min, declared, variadic, args.Count);
}
```

Message forms (all name the builtin and both counts):

| Shape | Message |
|---|---|
| exact (`MinArity == -1`, `min == max`) | `compile_full(): expected 2 arguments; got 1.` |
| bounded optional tail | `opt(): expected 1 to 2 arguments; got 0.` |
| variadic lower bound | `variadic_two(): expected at least 2 arguments; got 1.` |

`BuiltinArityException` derives from `ArgumentException`, so `Lovelace.Run/Runner.cs`'s existing
taxonomy maps it to `code: InvalidArgument`, `category: TypeMismatch`, `recoverable: true` — a
structure-preserving classification that needed no change to `Runner.cs` (which is out of scope).
The registration-time plumbing of `RegisterArrayBuiltin` was left alone (it already validates its
single argument).

## B3. Descriptors annotated (metadata corrections, no name special-cases)

| Descriptor | File:line | Annotation | Why |
|---|---|---|---|
| `symbol` | `Lovelace.Symbolics/SymbolicsPlugin.cs:160` | `MinArity: 1` | the body has always accepted `symbol("x")` (it only reads `args[1]` when `args.Count >= 2`); the domain is genuinely optional |

That is the ONLY descriptor this round added arity metadata to. Two descriptors already carried the
metadata and are exercised as regression evidence rather than re-annotated:

| Descriptor | File:line | Existing annotation |
|---|---|---|
| `dft` | `Lovelace.Dsp/DspPlugin.cs:220` | `MinArity: 1` (optional length `n`) |
| `noise` | `Lovelace.Dsp/DspPlugin.cs:240` | `MinArity: 3` (optional `seed`) |

`solve` and `solve_full` already declared `MinArity: 2`
(`Lovelace.Symbolics/SymbolicsPlugin.cs:411` and `:461` after this change), so `solve(f, x)` keeps
working. The validator was NOT widened and no builtin name is special-cased anywhere.

Source doc updates:

* `Lovelace.Abstractions/BuiltinDescriptor.cs:3-26` — the arity contract is now documented as
  enforced (upper bound = `Parameters.Count`, lower bound = `MinArity` with `-1` meaning exactly,
  `Variadic` removes the upper bound).
* `docs/symbolics/dsh-protocol.md:153-170` — the status → (complete, completeness) table, the
  `NoSolutions` reading, and where the mapping lives; `:187-198` — the arity error envelope shape.
* `Lovelace.Console.Tests/ReplOutputTests.cs:81` — the pinned `funcs symbolics` line is now
  `  symbol(name [, domain])`; the signature renderer already distinguishes required from optional
  parameters, so this is the help text telling the truth after the metadata correction (an exact
  whole-line assertion, not weakened).

---

# STEP C — goldens

Generator (this round's scripts): `docs/goal-cycle-3/round-5/run-fixtures.ps1` (runs the Release
runner over every `fixtures/*.ls` with `--file <path> --omit-functions`) and
`docs/goal-cycle-3/round-5/normalise-fixtures.js` (same volatile-key normalisation the golden tests
define: `revision`, `elapsed`, `elapsedTime`, `timings` → `"<volatile>"`, 2-space JSON).
Pre-regeneration snapshot: `docs/goal-cycle-3/round-5/fixtures-before/`.

New corpus row (`Lovelace.Run.Tests/GoldenEnvelopeTests.cs:25`): `{ "solve_nosolutions", 0 }`, with
`Lovelace.Run.Tests/fixtures/solve_nosolutions.ls`:

```text
x = symbol("x"); solve_full((x^2-1)/(x^2-1) == 0, x)
```

The corpus guard failed before the golden existed (log `step-c-corpus-before.log`):

```text
  Failed Lovelace.Run.Tests.GoldenEnvelopeTests.EnvelopeMatchesGoldenFixture(name: "solve_nosolutions", expectedExitCode: 0) [8 ms]
   missing golden fixture: C:\Users\ricar\dev\LovelaceSharp\Lovelace.Run.Tests\bin\Release\net10.0\fixtures\solve_nosolutions.json

Failed!  - Failed:     1, Passed:    17, Skipped:     0, Total:    18
```

## Golden diff (`docs/goal-cycle-3/round-5/golden.diff`)

```text
same    absent_field.json
same    array.json
same    compilation.json
same    complex.json
same    error_envelope.json
same    integration_result.json
same    limit_result.json
same    nested_record.json
same    optimization_result.json
same    print_purity.json
same    record.json
same    solve_result.json
same    symbolic.json
same    system_solve.json
same    transform_result.json
same    vector.json
ADDED   solve_nosolutions.json
changed/added files: 1 of 17 goldens
```

Every existing golden is byte-identical after regeneration: the 16 pre-existing rows (including the
`Solved`, `Partial` and error envelopes) are untouched, and the only new bytes are the new row. The
only changed value in the new golden is the intended one — the NoSolutions pairing
(`solve_nosolutions.json:16-58`):

```text
"status"       -> { "kind": "Enum",  "type": "SolveStatus",  "value": "NoSolutions" }
"complete"     -> { "kind": "Boolean", "value": "true" }
"completeness" -> { "kind": "Enum",  "type": "Completeness", "value": "Complete" }
display: "SolveResult(status: NoSolutions, variable: x, domain: complex, complete: True,
          completeness: Complete, solutions: [], families: [], common_conditions: [],
          represented_count: 0, unrepresented_count: 0, unrepresented_reason: ,
          diagnostics: [Diagnostic(code: solve.no-solutions, category: NoSolution,
          message: every root violates the denominator condition., recoverable: True, ...)])"
```

Before the change this envelope carried `complete: false` / `completeness: Unknown` (see the
pre-change probe output above); nothing else about the row changed.

## Step C passing

```text
dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:    34, Skipped:     0, Total:    34, Duration: 290 ms - Lovelace.Run.Tests.dll (net10.0)
```

---

# VERIFICATION

## Build

`dotnet build LovelaceSharp.slnx -c Release --nologo` (`docs/goal-cycle-3/round-5/final-build.log`):

```text
    49 Warning(s)
    0 Error(s)
Time Elapsed 00:00:05.14
```

Unchanged from the pre-change baseline build (49 warnings, all pre-existing in `Lovelace.Real`); no
new warning is attributed to any file this round touched.

## Test suites — all 15, 0 failed

Final run, `docs/goal-cycle-3/round-5/verify-suites.ps1` (per-suite logs `final-*.log`):

| Suite | Result |
|---|---|
| Lovelace.Abstractions.Tests | Passed! Failed: 0, Passed: 20, Total: 20 |
| Lovelace.Array.Tests | Passed! Failed: 0, Passed: 19, Total: 19 |
| Lovelace.Complex.Tests | Passed! Failed: 0, Passed: 83, Total: 83 |
| Lovelace.Console.Tests | Passed! Failed: 0, Passed: 15, Total: 15 |
| Lovelace.Dsp.Tests | Passed! Failed: 0, Passed: 61, Total: 61 |
| Lovelace.Integer.Tests | Passed! Failed: 0, Passed: 148, Total: 148 |
| Lovelace.Knowledge.Tests | Passed! Failed: 0, Passed: 28, Total: 28 |
| Lovelace.Natural.Tests | Passed! Failed: 0, Passed: 195, Total: 195 |
| Lovelace.Representation.Tests | Passed! Failed: 0, Passed: 91, Total: 91 |
| Lovelace.Run.Tests | Passed! Failed: 0, Passed: 34, Total: 34 |
| Lovelace.Studio.Tests | Passed! Failed: 0, Passed: 22, Total: 22 |
| Lovelace.Suite.Tests | Passed! Failed: 0, Passed: 629, Total: 629 |
| Lovelace.Symbolics.Tests | Passed! Failed: 0, Passed: 436, Total: 436 |
| precbench.Tests | Passed! Failed: 0, Passed: 13, Total: 13 |
| Lovelace.Real.Tests (`--filter "Category!=Heavy"`) | Passed! Failed: 0, Passed: 272, Total: 272 |

2066 tests across 15 suites, 0 failed. `Lovelace.MathIR` has **no** test project: `LovelaceSharp.slnx`
lists only `Lovelace.MathIR/Lovelace.MathIR.csproj` (verified by reading the solution file); the MathIR
surface is covered from `Lovelace.Run.Tests`, `Lovelace.Symbolics.Tests` and `Lovelace.Suite.Tests`,
which all pass.

The five suites named in the task, verbatim:

```text
Lovelace.Run.Tests        Passed!  - Failed:     0, Passed:    34, Skipped:     0, Total:    34
Lovelace.Symbolics.Tests  Passed!  - Failed:     0, Passed:   436, Skipped:     0, Total:   436
Lovelace.Suite.Tests      Passed!  - Failed:     0, Passed:   629, Skipped:     0, Total:   629
Lovelace.Studio.Tests     Passed!  - Failed:     0, Passed:    22, Skipped:     0, Total:    22
Lovelace.Console.Tests    Passed!  - Failed:     0, Passed:    15, Skipped:     0, Total:    15
Lovelace.MathIR           (no test project in LovelaceSharp.slnx)
```

## AOT publish

`dotnet publish Lovelace.Run/Lovelace.Run.csproj -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o out/aot`
(`docs/goal-cycle-3/round-5/publish-aot.log`):

```text
  Lovelace.Run -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Run\bin\Release\net10.0\win-x64\Lovelace.Run.dll
  Generating native code
  Lovelace.Run -> C:\Users\ricar\dev\LovelaceSharp\out\aot\
```

## Published binary (`out/aot/Lovelace.Run.exe`), full output of `published-probes.ps1`

```text
published binary: C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe
---- NoSolutions (denominator removes every root)  [probes/a1_ns_ratio.ls]
exit=0 type=SolveResult status=NoSolutions complete=true completeness=Complete
---- NoSolutions (x/x == 0)  [probes/a2_ns_xoverx.ls]
exit=0 type=SolveResult status=NoSolutions complete=true completeness=Complete
---- NoSolutions (x^2 + 1 == 0, real)  [probes/a3_ns_real.ls]
exit=0 type=SolveResult status=NoSolutions complete=true completeness=Complete
---- Solved control (x^2 - 4 == 0)  [probes/a5_solved.ls]
exit=0 type=SolveResult status=Solved complete=true completeness=Complete
---- Partial control (x^4 - x^2 - 1 == 0)  [probes/a4_partial.ls]
exit=0 type=SolveResult status=Partial complete=false completeness=Partial
---- Unevaluated control (x^4 + 1 == 0)  [probes/a6_unevaluated.ls]
exit=0 type=SolveResult status=Unevaluated complete=false completeness=Unknown
---- System NoSolutions (inconsistent)  [probes/a7_system_ns.ls]
exit=0 type=SystemSolveResult status=NoSolutions complete=true completeness=
---- Arity failure compile_full(x^2 + 1)  [probes/b1_arity.ls]
exit=1 code=InvalidArgument category=TypeMismatch recoverable=True
        message=compile_full(): expected 2 arguments; got 1.
---- Arity OK compile_full(x^2 + 1, [x])  [probes/b2_arity_ok.ls]
exit=0 type=CompilationResult
---- Optional trailing args: dft([0,1,0,0])  [probes/b3_dft_short.ls]
exit=0 kind=Array (no record fields)
---- Optional trailing args: solve(f, x)  [probes/b5_solve_short.ls]
exit=0 kind=Array (no record fields)
---- Optional trailing args: symbol(name)  [probes/b4_symbol_short.ls]
exit=0 kind=Text (no record fields)
```

`SystemSolveResult` has no `completeness` field (its five-field shape is pinned by the schema registry,
out of scope), so its `complete: true` for `NoSolutions` is the observable part of the shared mapping.
The inconsistent system used is `solve_system_full([x + y == 1, x + y == 2], [x, y])`.

## The required test-by-test results

| Required assertion | Test | Observed |
|---|---|---|
| `solve_full((x^2-1)/(x^2-1) == 0, x)`, `solve_full(x/x == 0, x)`, `solve_full(x^2 + 1 == 0, x, real)` → `NoSolutions` + complete TRUE + Complete | `SolveCompletenessPairingTests.NoSolutions_IsACompleteAnswer` (3 cases) | pass; published binary prints `complete=true completeness=Complete` |
| `solve_full(x^4 - x^2 - 1 == 0, x)` → `Partial`, complete FALSE, Partial, `unrepresented_count` 2 | `SolveCompletenessPairingTests.Partial_IsNeverComplete` | pass |
| `solve_full(x^2 - 4 == 0, x)` → `Solved`, complete true, Complete | `SolveCompletenessPairingTests.Solved_IsComplete` | pass |
| `solve_full(x^4 + 1 == 0, x)` → `Unevaluated`, complete false | `SolveCompletenessPairingTests.Unevaluated_IsNotComplete` | pass |
| inconsistent system → `NoSolutions` + complete true | `SolveCompletenessPairingTests.SystemSolve_NoSolutions_IsACompleteAnswer` | pass |
| mapping cannot disagree, every status | `SolveCompletenessMappingTests` (4 statuses via theory, 6 language rows, BudgetExceeded direct) | pass |
| `compile_full(x^2 + 1)` → typed message + not internal | `ArityValidationTests.TooFewArguments_IsATypedStructuralError_NeverAnInternalFailure` | pass; `InvalidArgument/TypeMismatch`, recoverable true, exact message |
| `compile_full(x^2 + 1, [x])` still succeeds; optional trailing parameters still callable | `ArityValidationTests.DeclaredArgumentCount_StillSucceeds`, `...OptionalTrailingParameters_StayCallableInTheShorterForm` (`solve(f, x)`, `symbol("x")`, `dft(x)`) | pass |
| every declared arity rule (exact / optional tail / variadic / parameters-only overload) | `ModusArityTests` (4 tests) | pass |

## Which statuses were reached which way

* Reached from the LANGUAGE: `Solved`, `Partial`, `NoSolutions`, `Unevaluated` (theory rows in
  `SolveCompletenessMappingTests.LanguageStatuses_ReportExactlyWhatTheMappingReturns`).
* Reached ONLY through the mapping directly: `BudgetExceeded`. Evidence that the language cannot
  reach it: the flag has no first writer — `SolutionSet.BudgetExceeded` is set only in
  `WithBudgetExceeded` (`Lovelace.Symbolics/Solvers/Solve.cs:173-179`), whose only caller is
  `InheritFrom` guarded by `if (inner.BudgetExceeded)` (`Solve.cs:315-316`); and
  `SystemSolveResult.Status` never returns it (`Solve.cs:859-869`). No language budget knob exists
  (`solve`/`solve_full` take only `f`, `x`, `domain`; `Lovelace.Console` has no budget command), and
  no probe in rounds 1–4 produced that status. The direct drive asserts
  `Of(BudgetExceeded) == (false, Unknown)` and `Of(BudgetExceeded, Partial) == (false, Partial)` —
  the kernel's Completeness for the partial subset, passed through.

## New test counts

| Suite | Before this round | After | Delta |
|---|---|---|---|
| Lovelace.Symbolics.Tests | 418 | 436 | +18 (`SolveCompletenessPairingTests` 7, `SolveCompletenessMappingTests` 11) |
| Lovelace.Suite.Tests | 625 | 629 | +4 (`ModusArityTests`) |
| Lovelace.Run.Tests | 28 | 34 | +5 (`ArityValidationTests`) +1 corpus row (`solve_nosolutions`) |
| Lovelace.Console.Tests | 15 | 15 | 0 (1 pinned signature line corrected) |

(Before-counts are the Step-A runs minus this round's new cases: 425−7, 629−4, 33−5.)

---

# Could not be made to pass / limitations (explicit)

1. **`Lovelace.MathIR` has no test project**, so "Lovelace.MathIR if one exists" resolves to: it does
   not exist. Checked in `LovelaceSharp.slnx`; MathIR is still exercised (compile/lower/evalir
   builtins) from the Run/Symbolics/Suite suites, all green.
2. **`Solve.cs` is out of scope**, so the kernel-level `SolutionSet.Complete` /
   `SystemSolveResult.Complete` properties still return `Unknown` for `NoSolutions`. The WIRE pair is
   now single-sourced, and those properties are only read as the `BudgetExceeded` fallback; moving
   the mapping into the kernel needs a later round with `Solve.cs` in scope.
3. **The `SystemSolveResult` record has no `completeness` field** (its field list is pinned by the
   out-of-scope schema registry), so the shared mapping is observable there only through `complete`.
4. **`Lovelace.Real.Tests`** was run with `--filter "Category!=Heavy"` (272 passed), matching CI; the
   heavy 1000-digit cases were not run this round.
5. The pre-change probe output quoted in Step A was transcribed from this round's first probe run
   (the code has since changed, so it is not re-runnable); the script that produced it is on disk at
   `docs/goal-cycle-3/round-5/probe-baseline.ps1`, and the same defect is pinned by the four Step-A
   test failures.
