# Triage — r20 `RealLiteral.FromRealExact` drops provenance (goal-cycle-6, round 2, row 1)

**VERDICT: NEEDS REPAIR** — the patch applies clean to HEAD `300f6bb` with exit 0, all three affected
projects build in Release with 0 warnings / 0 errors, and 13 of its 32 new test cases demonstrably have
teeth (they fail on the pristine base and pass on the patched tree) — **but the patch is not landable
as-is because one of its own new tests fails on the patched tree**, so it turns CI red:
`TruncatingEvaluationStaysInexactTests.TruncatedValueThatHappensToBeARational_IsStillInexact` fails at
`TruncatingEvaluationStaysInexactTests.cs:360` with `Assert.IsType: Expected typeof(NumInt), Actual typeof(NumRat)`,
and `dotnet test` on the patched `Lovelace.Symbolics.Tests` exits 1 with `Failed: 1, Passed: 1055, Skipped: 8, Total: 1064`.
The failure is an authoring bug on the test's own control line, not a defect in the product change (see below),
but it still has to be repaired before landing.

Shorthands used in the commands below (all read-only with respect to the main checkout):

| name | value |
|---|---|
| `$MAIN` | `C:\Users\ricar\dev\LovelaceSharp` |
| `$P` | `$MAIN\docs\goal-cycle-5\patches\wip-r20-exact2-UNVERIFIED.diff` (SHA256 `6A915DDE51DBFB69848FA6D964233D051B4F316CD8D77F22ADE964205EE76FE1`) |
| `$T` | `$MAIN\.worktrees\c6-r20` (scratch tree, `git worktree add .worktrees/c6-r20 HEAD`) |
| `$C` | `$MAIN\.worktrees\c6-r20-ctl` (pristine control: HEAD **without** the product changes) |

Platform: Windows PowerShell 5.1 + `cmd /v:on` for exit codes, `dotnet --version` = `10.0.103`.
Both the patched and the control run used this same platform. Raw logs live in `$T\_c6logs\` and `$C\_c6logs\`.

## Step / command / observed result

| # | exact command | observed result |
|---|---|---|
| 1 | `git -C $MAIN worktree add .worktrees/c6-r20 HEAD` | exit 0; `HEAD is now at 300f6bb`; `git rev-parse HEAD` → `300f6bb639d70233b4fcc060aa2731979fed42f9` |
| 2 | `cmd /c "cd /d $T & git apply --check --verbose $P 2>&1"` | **exit 0**; 6 × `Checking patch <file>...`, no `error:` line. Applies as-is. |
| 3 | `cmd /c "cd /d $T & git apply --verbose $P 2>&1"` | **exit 0**; 6 × `Applied patch <file> cleanly.` No whitespace warning, no `.rej`, no `.orig`. Result: `M Lovelace.Real/Real.cs`, `M Lovelace.Symbolics/Constructors.cs`, `M Lovelace.Symbolics/Evaluation.cs`, `M Lovelace.Symbolics/Expr.cs`, `?? Lovelace.Symbolics.Tests/C5Probe.cs`, `?? Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs`. `git diff --stat` = 4 files, 96 insertions(+), 9 deletions(-). |
| 4 | `dotnet build $T\Lovelace.Real\Lovelace.Real.csproj --configuration Release --nologo` | exit 0. `Build succeeded. 0 Warning(s) 0 Error(s)` (5.33 s) |
| 5 | `dotnet build $T\Lovelace.Symbolics\Lovelace.Symbolics.csproj --configuration Release --nologo` | exit 0. `Build succeeded. 0 Warning(s) 0 Error(s)` (2.76 s) |
| 6 | `dotnet build $T\Lovelace.Symbolics.Tests\Lovelace.Symbolics.Tests.csproj --configuration Release --nologo` | exit 0. `Build succeeded. 0 Warning(s) 0 Error(s)` (7.50 s) — the two new test files compile against the patched product. |
| 7 | `dotnet test $T\Lovelace.Symbolics.Tests\Lovelace.Symbolics.Tests.csproj -c Release --nologo --no-build` | **exit 1**. `Failed: 1, Passed: 1055, Skipped: 8, Total: 1064, Duration: 4 m 58 s`. First (and only) failure: `TruncatingEvaluationStaysInexactTests.TruncatedValueThatHappensToBeARational_IsStillInexact` → `Assert.IsType() Failure: Value is not the exact type / Expected: typeof(Lovelace.Symbolics.NumInt) / Actual: typeof(Lovelace.Symbolics.NumRat)` at `$T/Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs:360`. The 8 skips are pre-existing `[SKIP]` oracle tests (SympyOracle_*, SystemSolveProofTests, InfiniteFamilyCompletenessTests). |
| 8 | `dotnet test $T\…csproj -c Release --no-build --filter FullyQualifiedName~TruncatingEvaluationStaysInexactTests` | **exit 1**. `Failed: 1, Passed: 31, Skipped: 0, Total: 32, Duration: 2 m 1 s` — the matched patched-side number for the control in row 13. |
| 9 | `git -C $MAIN worktree add .worktrees/c6-r20-ctl HEAD` then `Copy-Item` of exactly the two new test files from `$T` to `$C` | exit 0; `git -C $C status --porcelain` = `?? Lovelace.Symbolics.Tests/C5Probe.cs` and `?? Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs` **only** — no product file was copied; the control product is pristine `300f6bb`. |
| 10 | `dotnet build $C\Lovelace.Symbolics.Tests\Lovelace.Symbolics.Tests.csproj -c Release --nologo` | exit 0. `Build succeeded. 0 Warning(s) 0 Error(s)` (7.99 s) — the new tests compile against the **unpatched** product. |
| 11 | `dotnet test $C\…csproj -c Release --no-build --filter FullyQualifiedName~TruncatingEvaluationStaysInexactTests` | **exit 1**. `Failed: 14, Passed: 18, Skipped: 0, Total: 32, Duration: 2 s` — the control. |
| 12 | `dotnet test $C\…csproj -c Release --no-build --filter FullyQualifiedName~C5Probe` | exit 0. `Failed: 0, Passed: 1, Skipped: 0, Total: 1, Duration: 2 m 29 s` — passes on **both** trees. |
| 13 | `certutil -hashfile $P SHA256` | `6a915dde51dbfb69848fa6d964233d051b4f316cd8d77f22ade964205ee76fe1` |
| 14 | `dotnet test $T\LovelaceSharp.slnx -c Release --nologo` (collateral: the patch changes a public equality contract) | **exit 1**, and the only failure in the whole solution is the row-7 one. All 15 test projects ran; 14 are green: `Abstractions.Tests` 20/20, `Natural.Tests` 195/195, `Array.Tests` 19/19, `Representation.Tests` 91/91, `Knowledge.Tests` 28/28, `Integer.Tests` 148/148, `precbench.Tests` 13/13, `Complex.Tests` 96/96, `Real.Tests` 2489/2489, `Dsp.Tests` 61/61, `Suite.Tests` 813/813, `Run.Tests` 175/175, `Console.Tests` 15/15, `Studio.Tests` 22/22. `Lovelace.Symbolics.Tests` → `Failed: 1, Passed: 1055, Skipped: 8, Total: 1064, Duration: 4 m 8 s`. The `RealLiteral.Equals`/`GetHashCode` route change breaks nothing outside the new file — including the suites that consume it (`Suite.Tests`, `Run.Tests`, `Representation.Tests`). |

## Control result and what it proves

Same platform, same filter, same two files, base commit `300f6bb` **without** the four product files:

| run | tree | total | failed | passed | exit |
|---|---|---|---|---|---|
| patched | `$T` (product + tests) | 32 | **1** | 31 | 1 |
| control | `$C` (tests only, pristine product) | 32 | **14** | 18 | 1 |

- **13 of the 32 new cases have teeth**: they fail on the pristine tree and pass on the patched one.
  `FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm` 8/12 (`evalf(sin(pi(30)/6), 40)`
  published `0.4999…9` with `exact=True` on the base — the exact laundering the row names;
  same for `cos(pi(30)/6)`, `sin(pi(30)/4)`, `atan(pi(30)/4)`, `exp(pi(30)/10)`, `sin(pi(45)/6)`,
  `evalf(sin(pi(30)/6), 100)` and `evalf(subs(x + 1, x, pi(30)), 40)`);
  `SymbolicTreeFlag_AndNumericReadBack_Agree` 3/4 (`subs(x + 1, x, pi(30))` → `4.141592653589793238462643383279` with `exact=True`
  on the base while the symbolic tree already said `exact=false`); plus
  `LiteralRoundTrip_PreservesBothTheDigitsAndTheRoute` (`pi to 30 places lost digits … must not come back
  exact through the symbolic literal`) and `FunctionOfATruncatedArgument_IsInexact`
  (`tan(e to 49 places) = -0.45054953406980749571063417770127929443957091173210837839273` claimed exact).
- **18 cases pass in both trees and therefore have no teeth**, reported as required:
  - `FunctionOfATruncatedValue_…` 4/12 — `evalf(sin(pi(1)*1/6), 40)`, `evalf(cos(pi(1)*1/6), 40)`,
    `evalf(sin(pi(1)*1/4), 40)`, `evalf(atan(1), 40)`. The test's own doc comment predicts exactly this
    ("they already answered false on the defective tree BY COINCIDENCE — their 40-digit renderings happen to
    land exactly on the width heuristic's boundary"), and the control confirms it.
  - `ExactControls_StayExact` (1), `LiteralRoundTrip_KeepsGenuinelyExactValuesExact` (1) and
    `ElementaryFunctionOfAnExactArgument_StaysExact` (11) — these are the file's declared *positive* controls;
    green on both trees is their job, not a defect.
  - `SymbolicTreeFlag_AndNumericReadBack_Agree("subs(x + 1, x, sqrt(2))")` (1) — passes on the base too.
  - `C5Probe.ProbeFunctions` (1) — see the seam check; it contains **no assertion** and passes on both trees.
- **1 case fails on both trees** — `TruncatedValueThatHappensToBeARational_IsStillInexact`. It is not a
  tooth; it is a broken test, and it fails for a *different* reason in each tree:
  - control → `$C/…/TruncatingEvaluationStaysInexactTests.cs:340`, `Assert.False(back.IsExact, "the truncated
    argument must come back inexact")` → message `the truncated argument must come back inexact`;
  - patched → `$T/…/TruncatingEvaluationStaysInexactTests.cs:360`, `Assert.IsType<NumInt>(Evaluation.EvaluateToNum(Exprs.Rational(1, 2), ctx, NoBindings))`
    → `Expected: typeof(NumInt) / Actual: typeof(NumRat)`.
  Reading the two together is the useful part: the product change *does* carry the truncated π/6 argument
  back through the literal inexactly (lines 340–356 now pass on the patched tree), and the test then dies on
  a hard-coded concrete-type assertion about an unrelated exact value. The test file itself knows better two
  hundred lines earlier — `TruncatingEvaluationStaysInexactTests.cs:311-312` normalises the tier with
  `NumOps.RatOf(value)` and its comment says "Exprs.Integer canonicalises to a RationalConstant, so the fold
  comes back on the exact rational tier". Line 360 contradicts that and is the single blocker to a green run.

## Seam check (new public member / new file / new wire field)

- **New file (2):** `Lovelace.Symbolics.Tests/C5Probe.cs` (63 lines) and
  `Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs` (399 lines).
  `C5Probe.cs` is a **WIP probe, not a test**: it has no `Assert` call anywhere (its only Xunit members are
  `[Fact]` at `:17`), it writes a diagnostic trace to `%TEMP%\c5-probe.txt` via
  `System.IO.File.WriteAllText` (`:20`) and `AppendAllText` (`:15`), and it costs
  **2 m 29 s** of test wall-clock for one always-green case. It carries cycle-5 naming into a cycle-6 round.
  Landing it adds a filesystem-writing, assertion-free, multi-minute test to CI; it should be dropped.
- **New public member on the product surface (4):**
  - `public static Real AsInexact(Real value)` — `$T/Lovelace.Real/Real.cs:219` (the copying half of the
    private `MarkInexact`).
  - `public bool RealLiteral.IsExact { get; }` — `$T/Lovelace.Symbolics/Expr.cs:47`.
  - `public RealLiteral(Int digits, long exponent10, bool exact)` — `$T/Lovelace.Symbolics/Expr.cs:51`
    (the two-argument form `:49` now chains to it with `exact: true`, so existing callers keep their meaning).
  - `public RealLiteral WithExactness(bool exact)` — `$T/Lovelace.Symbolics/Expr.cs:70`.
- **Changed public equality contract (the widest seam):** `RealLiteral.Equals` now includes `IsExact`
  (`Expr.cs:208-209`) and `GetHashCode` combines it (`Expr.cs:216`), while `CompareTo` (`:218`) and
  `ToRational()` still order/compare on digits and exponent alone. This is deliberate — `Exprs.Real`
  hash-conses on the literal (`Expr.cs`/`Constructors.cs:71`, `n._hash = Expr.Combine(…, value.GetHashCode())`)
  so the route must be in the key — but it is a semantic change to a public value type and is the reason the
  collateral run in row 14 exists.
- **Collateral of the equality change: none observed.** All 15 test projects in `LovelaceSharp.slnx` were
  built and run in Release (row 14) and the single failure is the new file's own row-7 assertion; every suite
  that consumes `RealLiteral` (`Lovelace.Symbolics.Tests`, `Lovelace.Suite.Tests`, `Lovelace.Run.Tests`) is
  otherwise green, so the widened `Equals`/`GetHashCode` does not disturb existing interning or comparison
  behaviour in the exercised paths.
- **New wire field: none.** The patch touches no DTO, protocol or serialization file (`RunProtocol.cs`,
  `Representation`, `Knowledge` are absent from the diff), and `RealLiteral` is not referenced outside
  `Lovelace.Symbolics` (22 mentions in `Expr.cs`), `Lovelace.Suite/NumericOps.cs` (1) and test files.
- **`Evaluation.cs` is documentation-only.** The objective names it, but its whole hunk (`-392,7 +392,16`) is
  a `///` summary rewrite: patched `Evaluation.cs:395-404` are comment lines and the method body at `:405`
  (`public static Expr NumToExpr(Num n)`) is unchanged, including the `NumReal rl => Exprs.Real(RealLiteral.FromRealExact(rl.V))`
  arm at `:412` that already read the flag. No behaviour changed in that file.

## What I could not determine

- **Whether the row is closed under repetition.** The solution-wide run in row 14 was a single observation
  on an otherwise-idle solution; I did not run `Lovelace.Symbolics.Tests` twice, so whether the ~2-minute
  `FunctionOfATruncatedArgument_IsInexact` case is flaky is not established.
- **Whether the row is actually closed end-to-end.** I verified the `RealLiteral` round-trip and the
  `evalf/inspect` agreement through `Lovelace.Symbolics.Tests`, but I did not run the CLI or the
  `Lovelace.Run` envelope against `evalf(sin(pi(30)/6), 40)`, so I cannot say what the published wire
  `exact` field reads for a row-1 expression.
- **Whether `Rl.AsInexact`'s copy is load-bearing.** `Real.cs:219` copies an exact `Real` before marking it;
  I did not construct a case where the aliasing would be observable, so I cannot say whether the copy is
  required or merely defensive.
- **The CI cost of the two slow cases.** `C5Probe.ProbeFunctions` took 2 m 29 s and
  `FunctionOfATruncatedArgument_IsInexact` pushed the filtered patched run to 2 m 1 s, while the full
  `Lovelace.Symbolics.Tests` run took 4 m 58 s standalone and 4 m 8 s inside the solution; each figure was
  measured **once**, with other agents' worktrees active on this box, so they are indicative rather than a
  measured CI budget.
- **Whether `RealLiteral.IsExact` should be excluded from `Equals` while being included in `GetHashCode`.**
  The current choice keeps `Equals` consistent with `GetHashCode` (both include it), which is the safe
  direction; whether the interning key *needs* the route, or whether it could be keyed on digits only and the
  node rebuilt, is a design question I did not test.

## Concurrent activity in `$MAIN` (observed, not caused by this run)

- Before step 1, `git -C $MAIN status --porcelain` printed exactly one line: `?? docs/goal-cycle-5/waiter6.ps1`.
  By the end of the run it also printed `M Lovelace.Real/Real.cs`, `M Lovelace.Symbolics/Constructors.cs`,
  `M Lovelace.Symbolics/Evaluation.cs`, `M Lovelace.Symbolics/Expr.cs` and
  `?? Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs` (plus unrelated `docs/` edits and this
  round-2 directory). `HEAD` is still `300f6bb639d70233b4fcc060aa2731979fed42f9`.
- The four product files in `$MAIN` are **byte-identical** to the patch-applied `$T` — `Real.cs` `6B0B327D…`,
  `Constructors.cs` `9A9F53D4…`, `Evaluation.cs` `54275D6A…`, `Expr.cs` `F336F5A4…` on both sides — so another
  agent applied this patch's product half to the main checkout while this triage was running.
- `$MAIN`'s test file is **not** the diff's version (`4937A168…` vs `$T`'s `6B258F76…`; 407 vs 399 lines) and
  `C5Probe.cs` is **absent** from `$MAIN`. The line-360 defect reported above has already been rewritten there:
  `$MAIN/Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs:366-368` now reads
  `var foldedHalf = Evaluation.EvaluateToNum(Exprs.Rational(1, 2), ctx, NoBindings);` /
  `Assert.Equal(Rat.From(1, 2), NumOps.RatOf(foldedHalf));` / `Assert.True(foldedHalf.IsExact, …)`, and the
  `Assert.IsType<NumInt>` is gone (`grep NumInt` over that file returns only this comment at `:362`).
- **I did not build or test that repaired combination.** It is another agent's in-flight edit, and running
  `dotnet build`/`dotnet test` inside `$MAIN` would write `bin/`/`obj/` into the main checkout, which this task
  forbids. The verdict above is about `docs/goal-cycle-5/patches/wip-r20-exact2-UNVERIFIED.diff` **as preserved**,
  not about the current state of `$MAIN`; whether the repaired tree is green is untested here.

## Hygiene / scope notes

- Only reads and builds happened in `$MAIN`; all writes landed in `$T`, `$C` and this file. Nothing was
  committed, pushed, repaired or improved, and no product or test file was edited — the patch was applied
  verbatim and observed.
- `.worktrees/c5-*` and the pre-existing `.worktrees/binary` were not touched.
- `$MAIN` carries uncommitted/untracked material that is **not** from this run (other agents are active in the
  same workspace): `?? docs/goal-cycle-5/waiter6.ps1` was already present, and
  `docs/goal-cycle-6/round-2/triage-r21.md` / `triage-r22.md` appeared during the run.
- Raw evidence: `$T\_c6logs\build-tests.log`, `$T\_c6logs\test-patched.log`,
  `$T\_c6logs\test-patched-newtests.log`, `$T\_c6logs\test-solution.log`,
  `$C\_c6logs\build-ctl.log`, `$C\_c6logs\test-ctl-a.log`, `$C\_c6logs\test-ctl-b.log`.
