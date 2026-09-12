# Round 3 — falsification report A (claims 1–4)

**Method / state.** No product or test file was edited; this report is the only file written.
All observations were made in two throwaway copies, never in the main working tree:

| copy | source | product | test file |
|---|---|---|---|
| `C:\Users\ricar\dev\ls-falsify-r3\prefix` | `.worktrees/c6-r20-ctl` (HEAD `300f6bb`, `git diff --stat` empty) | pristine pre-fix | landed `TruncatingEvaluationStaysInexactTests.cs` installed |
| `C:\Users\ricar\dev\ls-falsify-r3\fixed` | `.worktrees/c6-r20` | landed diff | landed `TruncatingEvaluationStaysInexactTests.cs` installed |

Product identity was hash-checked, not assumed — `Get-FileHash` main vs `fixed`:
`Lovelace.Real/Real.cs` `6B0B327D6F93`, `Lovelace.Symbolics/Constructors.cs` `9A9F53D4FCA9`,
`Lovelace.Symbolics/Evaluation.cs` `54275D6A1836`, `Lovelace.Symbolics/Expr.cs` `F336F5A4AC25`,
`Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs` `4937A1683C9F` (all equal).
Wire statements use the real runner, not the test's own projection:

```
dotnet <tree>\Lovelace.Run\bin\Debug\net10.0\Lovelace.Run.dll --file <tmp.ls> --json --omit-functions [--omit-variables]
```

The `fixed` copy additionally carries the round-2 probe `Lovelace.Symbolics.Tests/C5Probe.cs` (absent from
the landed tree; assertion-free, always green). It is re-flagged under claim 4; it makes the suite runs a
superset, not an equality.

| # | Claim | Verdict | Evidence (file:line / command) | Counterexample attempted | Reason |
|---|-------|---------|--------------------------------|--------------------------|--------|
| 1 | Row 1 is closed at the wire: with the landed change `evalf(sin(pi(30)/6), 40)` crosses the runner's JSON envelope as `exact:false` with no numerator and no denominator, **and no member of that family (a trigonometric or exponential function of a truncated constant, evaluated at a higher digit count) can still be published as `exact:true` with a rational numerator/denominator.** | **Falsified** | **First limb holds.** Fixed runner, body `evalf(sin(pi(30)/6), 40)` → `"structured":{"kind":"Real","value":"0.499999999999999999999999999999","exact":false}`; `variables[0].structured` is the same object and the envelope contains no `numerator`/`denominator` key. **Second limb broken.** Same runner, body `evalf(cos(pi(30)), 40)` → `"structured":{"kind":"Real","value":"-1","exact":true,"numerator":"-1","denominator":"1"}` — `cos` of the truncated constant `pi(30)` at a higher digit count (40 > 30), published `exact:true` **with** a rational form. Reproduced as `evalf(cos(pi(30))*2, 40)` (`num -2 den 1`), `evalf(2*cos(pi(30)), 40)` (`-2/1`), `evalf(cos(pi(30)/1), 40)` and `evalf(cos(pi(30)+0), 40)` (`-1/1`); `evalf(cos(pi(30)), d)` is exact for d ∈ {30, 40, 60, 100}. Mechanical cause: `Real.Cos`'s special-angle fast path returns the table's exact `-1` (`Lovelace.Real/Real.cs:1742-1743`, entry `(1, 1, Zero, negOne)` at `:1935`) without `MarkInexact`, so the value crosses as an exact literal and `Lovelace.Suite/StructuredProjection.cs:168-182` publishes the rational form. The landed change reaches `FromRealExact`/`ToReal` but not this fold. | Constructed `cos(pi(30))`: a trigonometric function of a truncated constant evaluated at a higher digit count, still published `exact:true` with numerator/denominator — the claim's second limb does not survive. Sweeps: 192 expressions (8 functions × 12 truncated constants × {40,60} digits) on the fixed runner → exactly 2 `exact:true` hits, both `cos(pi(30))`; `cos|sin(pi(k))`, k = 1..50 at 40 digits → 1 hit, `cos(pi(30))`. | Conjunction with a false second limb. The defect is pre-existing (identical on `prefix`) and outside the two edited boundaries: the closed-form `cos` fold never consults the literal's route. |
| 2 | The change does not over-correct: every result that was exact before the change is still exact after it (genuinely exact values such as `1/2`, integer arithmetic, `sin(0)`, exact closed forms), **and for any expression the symbolic tree's `inspect(...).exact` and the numeric read-back `evalf(...).exact` agree with each other.** | **Falsified** | **Agreement limb.** One runner, one tree: `[inspect(cos(pi(30))).exact, evalf(cos(pi(30)), 40)]` → `tree=false ; kind=Real exact=True num=-1 den=1`; `[inspect(cos(pi(30)*2)).exact, evalf(cos(pi(30)*2), 40)]` → `tree=false ; exact=True num=1 den=1`. The same expression read two ways gives two answers on the landed tree — the very tree-false/numeric-true signature the new file's docstring calls the defect. **Over-correction limb.** Raw envelopes, body `x = symbol("x"); y = symbol("y"); e = subs(subs(x + y, x, pi(30)), y, 1 - pi(30)); evalf(e, 40)`: `prefix` → `{"kind":"Real","value":"1","exact":true,"numerator":"1","denominator":"1"}`; `fixed` → `{"kind":"Real","value":"1","exact":false}` (no rational form). Same shape with `y = 2 - sqrt(2)` (value 2) and `y = 1/2 - pi(30)` (value 1): `exact:true` pre-fix, `exact:false` post-fix. The fold is lossless — the truncated digits cancel exactly and the emitted literal is exactly the integer 1 / 2 — yet the wire no longer publishes the rational form. (`Add`/`Mul` mark the synthesised literal inexact when `any` operand was inexact: `Lovelace.Symbolics/Constructors.cs:211`, `:436`.) | Both limbs attacked; both broke. (a) agreement: `cos(pi(30))`, `cos(pi(30)*2)`; (b) over-correction: the exact-cancellation family above. Secondary (pre-existing, present on both trees, not caused by the change): `inspect(sin(1)).exact = true` vs `evalf(sin(1), 40).exact = false`, likewise `exp(1)`, `atan(1)`. | Limb 2 is false on a target-family expression; limb 1 is false for exactly-representable results (1, 2, 1/2-shaped) that were exact before. The route-based answer is internally defensible, but the claim's absolute wording ("every result that was exact before ... is still exact after it") is contradicted. |
| 3 | The new test file has teeth: at least 13 of its 32 cases fail against the pristine pre-fix product and pass against the fixed one, and the assertion that replaced `Assert.IsType<NumInt>(...)` at line 360 is no weaker than it was. | **Supported** | **Pre-fix:** `cd C:\Users\ricar\dev\ls-falsify-r3\prefix; dotnet test Lovelace.Symbolics.Tests\Lovelace.Symbolics.Tests.csproj --filter "FullyQualifiedName~TruncatingEvaluationStaysInexactTests" --logger "trx;LogFileName=prefix.trx" -v minimal` → `Failed: 14, Passed: 18, Total: 32` (`prefix\Lovelace.Symbolics.Tests\TestResults\prefix.trx`). **Post-fix:** `cd C:\Users\ricar\dev\ls-falsify-r3\fixed; dotnet test LovelaceSharp.slnx -c Release --logger "trx;LogFileName=relfull.trx" -v minimal` → exit 0, `Lovelace.Symbolics.Tests` `Passed! Failed: 0, Passed: 1056, Skipped: 8, Total: 1064, 4 m 23 s`; the class filter over `fixed\Lovelace.Symbolics.Tests\TestResults\relfull.trx` gives 32 cases, 32 Passed. So 14 cases fail pre-fix and pass post-fix (≥ 13). **Line 360:** `git diff --no-index .worktrees/c6-r20/Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs` is exactly one hunk. The removed `Assert.IsType<NumInt>(Evaluation.EvaluateToNum(Exprs.Rational(1, 2), ctx, NoBindings))` was unsatisfiable: `Exprs.Rational(1,2)` builds a `RationalConstantExpr` and `Lovelace.Symbolics/Evaluation.cs:388 RationalConstantExpr r => new NumRat(r.Value)`; an F# probe against the fixed DLLs printed `Exprs.Rational(1,2) node type = RationalConstantExpr`, `EvaluateToNum type = NumRat exact=true`, `NumOps.RatOf = 1/2`. The replacement (`Assert.Equal(Rat.From(1, 2), NumOps.RatOf(foldedHalf)); Assert.True(foldedHalf.IsExact, …)`) fails for any wrong value and throws for anything off the exact tier (`NumOps.RatOf` throws `"Value is not exact."` at `Evaluation.cs:77`). | Tried to drop the tooth count below 13 (measured 14) and to find a wrong state the new assertion pair accepts (off-tier throws, wrong value fails, `IsExact=false` fails). The claim survived both. | 14 ≥ 13, and the replaced assertion is strictly more informative than one that could never pass on either tree. |
| 4 | Nothing was weakened to reach green: across the landed diff no existing assertion was deleted, loosened, skipped or re-expected, and the landed tree's tests pass. | **Supported** | **Diff:** `git diff --name-only` = the 4 product files + 4 docs files; `git status --porcelain` adds only the untracked new test file (`Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs`) and other agents' docs. No tracked test file is modified; `git diff --name-status | Select-String '^D'` = 0; `git diff | Select-String '^\+.*\bSkip\b'` = 0; the only removed line in the whole tracked diff that contains `Assert.` is a docs table row in `docs/goal-cycle-6/round-1/falsify-A.md` (prose). The new file contains no `Skip`. **Landed tests:** the Release run above → `exit=0`, all 15 projects `Passed!`, every one `Failed: 0` (`relfull.log`): Symbolics 1056/0/8, Real 2489, Suite 813, Run 175, Dsp 61, Complex 96, Studio 22, Console 15, precbench 13, … . | Searched the landed diff for a deleted, loosened, skipped or re-expected **test** assertion; searched the landed suite for a hidden failure. The only in-test change anywhere is the line-360 hunk inside the new file (analysed in claim 3), and the suite is green in Release. | Relative to HEAD the test file is new, so no assertion existing at HEAD was touched; "re-expected" applies only to the round-2 draft, which is not in the landed tree, and that single hunk replaces an assertion that could never pass. Disclosures below do not weaken the claim. |

## Attacks attempted that did not succeed

1. **Break claim 1's first limb by other routes.** `w = pi(30)/6; evalf(sin(w), 40)`,
   `v = [pi(30)/6]; evalf(sin(v[0]), 40)`, `evalf(sin(pi(30)/6), 30)`, `evalf(sin(pi(30)/6), 100)`,
   `evalf(cos(pi(30)/6), 40)`, `evalf(atan(pi(30)/4), 40)`, `evalf(exp(pi(30)/10), 40)` and
   `evalf(exp(pi(30)/10), 100)` — every one published `exact:false` with no rational form on the fixed runner.
2. **Break claim 1's second limb by breadth.** 192-expression matrix (8 functions × 12 truncated
   constants × {40, 60}) plus a 100-expression `cos|sin(pi(k))` sweep, k = 1..50 — the only hits were
   `evalf(cos(pi(30)), 40)` and `…, 60)` (listed as the counterexample).
3. **A degenerate "member" I rejected.** `evalf(sin(pi(30)/6)*0 + 1/2, 40)` → `exact:true, 1/2`, and
   `evalf(sin(pi(30)/6)^0, 40)` → `1`: syntactically a trig function of a truncated constant, but the
   truncation is multiplied away, so the value does not depend on it. Not used; `cos(pi(30))` stands alone.
4. **Break the new mechanism itself.** F# probe on the fixed DLLs: `pi30.IsExact=false`,
   `RealLiteral.FromRealExact(pi30).IsExact=false`, `.ToReal().IsExact=false`, value preserved;
   `0.5` stays `IsExact=true` through the same round trip. The mechanism works — the hole is the fold.
5. **Satisfy claim 2 by marking everything inexact.** Exact controls on the fixed runner all still report
   `exact:true`: `evalf(1/2, 40)` = 1/2, `evalf(sqrt(4), 40)` = 2, `1/17` periodic with N/D,
   `evalf((-4)^(1/2), 40)`, `dft([1,2,3])[0]` = 6, `evalf(sin(0), 40)` = 0.
6. **Drop claim 3's tooth count below 13.** Measured 14 pre-fix failures (trx), each of which passes
   post-fix (32/32 in the Release trx).
7. **Show the line-360 replacement is weaker.** Off-tier results throw in `NumOps.RatOf`
   (`Evaluation.cs:77`), wrong values fail `Assert.Equal`, `IsExact=false` fails the added `Assert.True`;
   the old `IsType<NumInt>` was simply false (`NumRat` observed).
8. **Find a weakened assertion or a hidden landed failure (claim 4).** No test file is modified, no
   `Skip` added, no test deleted, no undocumented re-expectation; the Release suite is 15/15 green.
9. **Show the control tree was contaminated.** `git -C .worktrees/c6-r20-ctl diff --stat` is empty; the
   `prefix` copy's `Expr.cs` has no `WithExactness`/`IsExact` member and its wire behaviour still shows the
   pre-fix laundering (the cancellation expression → `exact:true, 1/1`).
10. **Make the landed suite fail through configuration.** The Debug all-projects run surfaced exactly one
    failure — `Lovelace.Suite.Tests.CancellationObservationTests.Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly`
    (`"cancellation was observed but only after 28682 ms; the budget was 1500 ms"`). It reproduces
    identically on the pristine `prefix` tree in Debug (1 failed / 1 total, 23 s) and passes in Release on
    the fixed tree (1 s), so it is a pre-existing timing/configuration issue, not a landed-tree regression.

## Disclosures / what I could not check

* The full Release suite ran on `fixed`, which carries `C5Probe.cs` (not in the landed tree, no assertions,
  passes). The landed tree's tests are a subset of what was run; the run is a superset, not an exact replica.
  Counts therefore read 1056 passed (vs 1055 without the probe).
* The 8 skipped `Lovelace.Symbolics.Tests` cases are the conditional SymPy-oracle comparisons; no SymPy on
  this host. They are skipped at HEAD too and are untouched by the diff — no verdict depends on them.
* CI on GitHub's runners is outside SCOPE; every statement above is from local runs in the throwaway copies.
* Additional same-class observations I did **not** root-cause and did **not** use as counterexamples because
  they are outside the claim-1 parenthetical family: `evalf(e(49), 60)` and `evalf(sqrt(e(49)), 60)` also
  publish `exact:true` with a numerator/denominator on the landed tree (identical on `prefix`).
* A Debug filtered run of the new class on this loaded box did not finish in > 35 min (dominated by
  `FunctionOfATruncatedArgument_IsInexact`, which alone exceeded 120 s in isolation and whose `tan` pairs
  ran for minutes in both trees), while Release completes the whole `Lovelace.Symbolics.Tests` project in
  4 m 23 s. Recorded as a runtime risk, not as a claim failure.
