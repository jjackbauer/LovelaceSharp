# Cycle-6 · round-21 · audit K — **K-2** the library's structured projection lost the digits the CLI publishes

**Verdict: FIXED.** Branch `r21-k2proj`, commit **`33560e964c88820ea15d13fc2a4933a6f4cc433c`**
(short `33560e9`), based on `7df2a68` — see *Deviations* for why the branch is one commit past the
briefed `d3a1f50`. Control tree: `.worktrees/r21-k2proj-ctl` detached at `d3a1f50`.

| | |
|---|---|
| finding | K-2 (P1) — `SuiteEngine.ProjectValue` published a Real's structured `Value` cut to DISPLAY precision, with `Truncated: null`, while the CLI published all of it |
| files changed | `Lovelace.Suite/StructuredProjection.cs`, `Lovelace.Suite/SuiteEngine.cs`, `Lovelace.Run/Runner.cs`, new `Lovelace.Run.Tests/LibraryProjectionAgreementTests.cs` |
| new tests | 15 (6 of them fail on `d3a1f50` in a pristine control worktree) |
| full solution | **5584 tests, 5576 passed, 0 failed, 8 skipped**, build **0 warnings / 0 errors** |

---

## 1 · The digit counts on `d3a1f50` (library vs CLI)

Measured in one process by a temporary probe test (`Lovelace.Run.Tests/ZzK2Probe.cs`, run and then
deleted — it is in no commit), which drove the library façade and the in-process CLI over the same
computation:

```
dotnet test .worktrees\r21-k2proj-ctl\Lovelace.Run.Tests\Lovelace.Run.Tests.csproj -c Release \
    --filter "FullyQualifiedName~ZzK2Probe" --nologo -v q          # writes %TEMP%\k2-probe.txt
```

**Before (`d3a1f50` control tree):**

```
A facade  comp=1100 display=100 pi(1100): decimals=100 truncated=null display=100
B facade  default comp=1000 display=100 pi(1000): decimals=100 truncated=null
C cli     setprecision(1100); pi(1100): decimals=1100 truncated=null
D facade == cli: False
E facade  comp=5000 display=100 evalf(sqrt(2),5000): decimals=100 truncated=null reason=null budget=null
F facade  evalf(sqrt(2),5)='1.41421'  1/7='0.(142857)'  1/8='0.125'
```

**After (fix tree, same probe):**

```
A facade  comp=1100 display=100 pi(1100): decimals=1100 truncated=null display=100
B facade  default comp=1000 display=100 pi(1000): decimals=1000 truncated=null
C cli     setprecision(1100); pi(1100): decimals=1100 truncated=null
D facade == cli: True
E facade  comp=5000 display=100 evalf(sqrt(2),5000): decimals=1000 truncated=True reason=digit-cap budget=1000
F facade  evalf(sqrt(2),5)='1.41421'  1/7='0.(142857)'  1/8='0.125'
```

The CLI half was also measured directly from the built binary in both trees:

```
$exe = .worktrees\<tree>\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.exe
& $exe --eval "setprecision(1100); pi(1100)" --omit-functions   # then count digits of result.structured.value
  CLI [r21-k2proj]     decimals=1100 head=3.14159265358979323846 tail=24972177528347913151 truncated=
  CLI [r21-k2proj-ctl] decimals=1100 head=3.14159265358979323846 tail=24972177528347913151 truncated=
```

So on `d3a1f50`: **library façade 100, CLI 1100, `Truncated: null` on both** — the machine API
dropping 1 000 digits of a value it owns and its own completeness flag denying it, exactly as the
auditor reported (A: `display=100` at the same time is the display doing its documented job).
After the fix the façade and the CLI publish the **same string** (D: `True`).

## 2 · Root cause, in the source

* `SuiteEngine.ProjectValue` (`SuiteEngine.cs:115-120` at `d3a1f50`) wrapped the projection in
  `Rl.WithPrecision(ComputationDecimalPlaces, DisplayDecimalPlaces)` — the display bound — and the
  shared `StructuredProjection.Real` renders a Real with `value.ToString()`, which
  `Lovelace.Real/Real.cs:2481-2545` cuts at `DisplayDecimalPlaces` for a non-periodic value.
* `Runner.StructuredValue` (`Runner.cs:557-575` at `d3a1f50`, from `b3b74b8`) wrapped the same
  call in `Rl.WithPrecision(engine.ComputationDecimalPlaces, 1_000_000_000L)` instead, so only the
  CLI carried the digits. `Lovelace.Studio/EngineHost.cs:116,142,291,301,337` calls
  `StructuredProjection.ToStructured` with **no** scope at all, so Studio was on the process
  default (100) too — a third host of the same defect, not named in the audit.

## 3 · The fix — one projection

`Lovelace.Suite/StructuredProjection.cs`

* `StructuredDecimalDigits = 1_000_000_000L` moved here from `Runner` (the runner's private copy is
  gone) and `ToStructured` now establishes the structured rendering room itself:
  `using var _ = Rl.WithPrecision(Rl.MaxComputationDecimalPlaces, StructuredDecimalDigits);` — the
  computation precision in force is preserved, only the room changes. Every host that reaches the
  projection (façade, runner, Studio, tests) gets the same room.
* `Real` now renders through `StoredDigits(value)`, which asks for **exactly the fractional digits
  the value stores** (`-Exponent`; the count `Real.ToString()` emits when it is not cut) instead of
  the ambient display bound: a periodic value carries its block and is never bounded, and a
  non-periodic one cannot exceed its own count. The room is a room, not a cap, so nothing is
  dropped and no truncation field is claimed. `evalf`'s real clamp is untouched: it still crosses
  as `truncated: true` / `truncationReason: "digit-cap"` / `budget: 1000` (line E above), because
  that is the value's own `ClampNotice`, not a rendering bound.
* `StructuredDecimalDigits` remains what it always was for the one path that cannot count its own
  digits — the symbolic printer rendering a **Real literal inside an expression**
  (`Lovelace.Symbolics/Printing.cs:641,897` call `Real.ToString()`), which is why the constant
  could not simply be deleted.

`Lovelace.Suite/SuiteEngine.cs` — the façade keeps establishing the engine's computation precision;
its doc-comment no longer claims the structured digits follow the display precision (the auditor's
honest caveat: the old wording matched the bug). It now states the contract — display governs
`FormatValue`/`display`/`typed`, the structure carries the stored digits.

`Lovelace.Run/Runner.cs` — `StructuredValue` and the private constant are deleted; the result
payload and every variable go through `engine.ProjectValue(...)`, i.e. the same public façade a host
calls. That is the structural half of "one projection": there is no runner-only rule left to drift.

## 4 · Test-first: the new suite fails on `d3a1f50`

`Lovelace.Run.Tests/LibraryProjectionAgreementTests.cs` (new, 15 tests) drives
`SuiteEngine.ProjectValue` — the auditor's own shape — and compares it with the in-process CLI
envelope for the same script.

```
git -C C:\Users\ricar\dev\LovelaceSharp worktree add .worktrees/r21-k2proj-ctl d3a1f50
Copy-Item .worktrees\r21-k2proj\Lovelace.Run.Tests\LibraryProjectionAgreementTests.cs \
          .worktrees\r21-k2proj-ctl\Lovelace.Run.Tests\
cd .worktrees\r21-k2proj-ctl
dotnet test .\Lovelace.Run.Tests\Lovelace.Run.Tests.csproj -c Release \
    --filter "FullyQualifiedName~LibraryProjectionAgreementTests" --nologo \
    --logger "console;verbosity=detailed"
```

**Control output (`d3a1f50`), 6 failed / 9 passed / 15 total** (full log:
`.worktrees/r21-k2proj-ctl/control-test-output.log`):

```
  Failed ...TheFacade_PublishesEveryStoredDigit_WhenTheDisplayBoundIsLower [80 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 1100
Actual:   100
  Failed ...TheFacade_AndTheCli_AgreeOnOneStructuredValue(script: "sqrt(2)") [60 ms]
  Error Message:
   Assert.Equal() Failure: Strings differ
                                  ↓ (pos 102)
Expected: ···"88503875343276415727350138462309122970249"···
Actual:   ···"97379907324784621070388503875343276415727"
  Failed ...TheFacade_AndTheCli_AgreeOnOneStructuredValue(script: "evalf(pi, 1000)") [40 ms]
  Error Message:
   Assert.Equal() Failure: Strings differ
                                  ↓ (pos 102)
Expected: ···"86280348253421170679821480865132823066470"···
Actual:   ···"45923078164062862089986280348253421170679"
  Failed ...TheFacade_AndTheCli_PublishTheSameDigits_ForTheFindingsScript [17 ms]
  Error Message:
   Assert.Equal() Failure: Strings differ
                                  ↓ (pos 102)
Expected: ···"86280348253421170679821480865132823066470"···
Actual:   ···"45923078164062862089986280348253421170679"
  Failed ...AClampedValue_IsMarked_ThroughTheFacade_AsItIsOnTheCli [168 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 1000
Actual:   100
  Failed ...NestedReals_ThroughTheFacade_CrossInFullToo [6 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 1100
Actual:   100

Test Run Failed.
Total tests: 15
     Passed: 9
     Failed: 6
```

`pos 102` is the same cut in every string case: `"3."` + 100 digits, the display default.

**After the fix, same command in `.worktrees/r21-k2proj`** (log `fix-test-output.log`):

```
Test Run Successful.
Total tests: 15
     Passed: 15
```

The 9 that pass on `d3a1f50` are the controls that must not move: `evalf(sqrt(2), 5)` = `"1.41421"`,
`1/7` = `"0.(142857)"`, `1/8` = `"0.125"` with numerator 1 / denominator 8, `pi(30)`,
`setprecision(1100); pi(1100)` (that script raises the engine's own display, so it was already
whole), and the agreement cases for `e(1100)` and `setprecision(1100); sqrt(2)`.

## 5 · Nothing existing pinned the buggy behaviour

**No existing test pinned the library truncation as-is** — there is no assertion to weaken, and none
was touched. Every suite passes unchanged, in particular:

* `Lovelace.Run.Tests/StructuredRealPrecisionTests.cs` — 9/9 pass (`requested precision beyond the
  old cap`, `evalf(pi, 1000)` in full, `nested Reals`, `counts at or below the request`,
  `stored digits are not padded`, `the result's display follows the script's precision`).
* the print-budget suites: `PrintBudgetTotalityTests`, `PrintBudgetTokenBoundaryTests` — green.
* `EvalfDigitCapHonestyTests` — the CLI's `digit-cap` marker still green; the new façade test pins
  the same three fields on the library side.

## 6 · Totals (fix tree, `r21-k2proj` @ `33560e9`)

```
dotnet build .worktrees\r21-k2proj\LovelaceSharp.slnx -c Release --nologo
    Build succeeded.  0 Warning(s)  0 Error(s)

dotnet test  .worktrees\r21-k2proj\LovelaceSharp.slnx -c Release --nologo
```

```
Lovelace.Abstractions.Tests     20/20        Lovelace.Real.Tests       2495/2495
Lovelace.Representation.Tests   91/91        Lovelace.Suite.Tests       848/848
Lovelace.Knowledge.Tests        28/28        Lovelace.Symbolics.Tests  1185/1193 (8 skipped)
Lovelace.Array.Tests            19/19        Lovelace.Run.Tests         315/315
Lovelace.Natural.Tests         195/195       Lovelace.Studio.Tests       22/22
Lovelace.Integer.Tests         148/148       Lovelace.Complex.Tests     121/121
precbench.Tests                 13/13        Lovelace.Dsp.Tests          61/61
Lovelace.Console.Tests          15/15
Total: 5584 tests, 5576 passed, 0 failed, 8 skipped
```

(`Lovelace.Run.Tests` = 300 pre-existing + the 15 new; full logs `fix-fulltests-final.log`.)

## 7 · Deviations, and what I could not do

* **Base commit.** The brief named `d3a1f50` as main HEAD, but main advanced to `7df2a68` while I
  worked and that commit edits the **same file** (`Lovelace.Suite/StructuredProjection.cs`, the I-2
  `ClampNotice` work my fix must preserve). The branch is therefore based on `7df2a68`; the control
  tree is at `d3a1f50` exactly as briefed, and every failing proof above is from it. A branch off
  `d3a1f50` would have conflicted in `Real()` for no benefit.
* **Scope of edits.** Only the three named product files plus one new test file. No test was
  weakened, skipped, deleted or re-pinned. No other agent's files were touched.
* **Residual, unreachable.** A **Real literal inside a symbolic expression** is printed by
  `Lovelace.Symbolics/Printing.cs` through `Real.ToString()` under the projection's
  `StructuredDecimalDigits` room; a value with more than 10⁹ stored fractional digits would still
  be cut there and unmarked. That needs a value of roughly a gigabyte of decimal text — every route
  into one in this product is bounded at 1000 (`pi`/`e`/the `evalf` cap) — and closing it belongs to
  the printer, not to the projection. Real values themselves can no longer be cut: their room is
  their own digit count.
* **Third host improved without an edit.** Studio called `StructuredProjection.ToStructured`
  directly, i.e. with no precision scope, so it was on the process display default (100) as well.
  The shared room closes that too; `Lovelace.Studio.Tests` is green (22/22).
* **Not done:** no push, no merge into main (as briefed); no change to `docs/symbolics/dsh-protocol.md`
  (the contract it states is what the fix now obeys — the document needed no amendment).

## 8 · Where the evidence lives

| artefact | path |
|---|---|
| commit | `33560e964c88820ea15d13fc2a4933a6f4cc433c` on `r21-k2proj` |
| new test | `Lovelace.Run.Tests/LibraryProjectionAgreementTests.cs` |
| control failures | `.worktrees/r21-k2proj-ctl/control-test-output.log` |
| fix run | `.worktrees/r21-k2proj/fix-test-output.log` |
| solution build | `.worktrees/r21-k2proj/fix-build.log` |
| full test run | `.worktrees/r21-k2proj/fix-fulltests-final.log` |
