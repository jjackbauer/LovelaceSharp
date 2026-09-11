# Round 18 — factor() drops the sign of the leading coefficient

Date: 2026-02-14 (round 18) · Status: FIXED AND VERIFIED (except the two pre-existing hangs, reported not fixed)

## 1. Root cause (restated from the round-18 prompt; confirmed by source read)

`Lovelace.Symbolics/Algebra/Polynomial.cs:351-361` defines `NormalizeUnivariate(p)`, which divides
every coefficient by the leading coefficient "to make monic". `SquareFreeUnivariate`
(`Polynomial.cs:364-396`) calls it on its already-square-free path at line 373
(`result.Add((NormalizeUnivariate(f), 1));`) and also on the Yun-loop factors at line 387, and it
RETURNS THE MONIC POLYNOMIAL, silently discarding the leading-coefficient multiplier.
`Lovelace.Symbolics/Algebra/Factor.cs:46` consumes that value, and `FactorPoly` never multiplied the
discarded divisor back into `factors.Content`.

For `-x^2+1` the discarded multiplier is exactly `-1`, which is why the sign vanishes; for a
positive leading coefficient it is `+1` and nothing changes — hence the defect went unnoticed.

`NormalizeUnivariate` was NOT changed: `GcdUnivariate` (`Polynomial.cs:338-349`) depends on monic
normalisation for its Euclid loop.

## 2. The fix chosen

Of the two options offered, the FIRST was taken: **capture the leading coefficient of the input
polynomial in `FactorPoly` BEFORE the square-free decomposition and fold it into the returned
`Content`.** (The alternative — have `SquareFreeUnivariate` report the divisor it applied — was
rejected because it widens a public API used by other callers for no extra correctness.)

`Lovelace.Symbolics/Algebra/Factor.cs`, `FactorPoly`:

    // before the decomposition
    var leading = primitive.IsZero ? Rat.One : primitive.LeadingCoefficient(MonomialOrder.Lex);
    var sqfree = Polynomial.SquareFreeUnivariate(primitive);
    ...
    if (factors.Count == 0)
    {
        // Nothing was emitted: the primitive part itself is a unit, returned unmoved here —
        // so it already carries its own leading coefficient and must NOT be scaled again.
        factors.Add((primitive, 1));
    }
    else
    {
        // Every emitted factor is monic; restore the discarded leading coefficient.
        content = content * leading;
    }
    return new PolyFactors(content, factors);

Why this is the right place: every factor `SquareFreeUnivariate` emits is monic (the gcd loop and
the emitted quotients are monic by construction), and `PrimitivePart` already made the coefficients
coprime but NOT monic, so the primitive part satisfies
`primitive = leading * prod(factor_i^mult_i)`. Multiplying `Content` by `leading` therefore restores
exactly the unit that was dropped, in the one place where the factorization is rebuilt
(`Factor.Factor` line 25 rebuilds `content * prod(factor^mult)`).

The single degenerate branch is guarded: when the decomposition emits NOTHING
(`factors.Count == 0` — only reachable when the primitive part is a unit +-1), the fallback already
adds the primitive part *unnormalised*, so it must not be scaled a second time.

```
$ git diff --stat
 Lovelace.Symbolics/Algebra/Factor.cs | 22 +++++++++++++++++++---
```
No other source file was touched. `FactorMetamorphicTests.cs` was not modified, not skipped and not
weakened; no expected value was changed.

## 3. Probes (before / after)

Harness: `docs/goal-cycle-3/round-18/probe.ps1` (JIT runner `Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll --file <case>.ls --omit-functions`, 15-20 s hard kill per case).

| probe | source | BEFORE (pre-fix build) | AFTER (post-fix build) |
|---|---|---|---|
| A (required) | `x = symbol("x"); expand(factor(-x^2 + 1))` | `x^2 - 1`  **WRONG** | `-x^2 + 1`  **OK** |
| B (required) | `x = symbol("x"); expand(factor(-6*x^3 + 6*x))` | `6*x^3 - 6*x`  **WRONG** | `-6*x^3 + 6*x`  **OK** |
| C (positive control) | `x = symbol("x"); factor(x^2 - 1)` | `(x - 1)*(x + 1)` | `(x - 1)*(x + 1)` (unchanged) |
| D (hang probe) | `x = symbol("x"); expand(factor(-x^2 - 2*x - 1))` | HANG (killed after 15 s) | HANG (killed after 15 s) — unchanged |

## 4. Acceptance results

1. **HOLDS.** `expand(factor(-x^2 + 1)) = -x^2 + 1` and `expand(factor(-6*x^3 + 6*x)) = -6*x^3 + 6*x`
   (probes A and B above).
2. **16 of the 18 corpus entries COMPLETE and all 16 satisfy `expand(factor(p)) == p`; 0 wrong
   answers; 2 entries HANG and therefore cannot be judged.** Measured per entry with a 15 s hard
   kill each, by `docs/goal-cycle-3/round-18/corpus.ps1` (each case is its own process, so one hang
   cannot mask the rest). Raw run:```
   01__x_2___1                      PASS   factor(-x^2 + 1) ok
   02__x_3___x                      PASS   factor(-x^3 + x) ok
   03__x_4___5_x_2___4              PASS   factor(-x^4 + 5*x^2 - 4) ok
   04__2_x_2___8                    PASS   factor(-2*x^2 + 8) ok
   05__6_x_3___6_x                  PASS   factor(-6*x^3 + 6*x) ok
   06__x_2___2                      PASS   factor(-x^2 + 2) ok
   07__x_3___x_2___x___1            HANG   factor(-x^3 - x^2 + x + 1)
   08__x_2___2_x___1                HANG   factor(-x^2 - 2*x - 1)
   09__3_x_2___3                    PASS   factor(-3*x^2 + 3) ok
   10_x_2___1                       PASS   factor(x^2 - 1) ok
   11_x_3___1                       PASS   factor(x^3 + 1) ok
   12_x_4___5_x_2___4               PASS   factor(x^4 - 5*x^2 + 4) ok
   13_2_x_2___8                     PASS   factor(2*x^2 - 8) ok
   14_6_x_3___6_x                   PASS   factor(6*x^3 - 6*x) ok
   15_x_2___2                       PASS   factor(x^2 - 2) ok
   16_x_3___x_2___x___1             PASS   factor(x^3 + x^2 - x - 1) ok
   17_x_3___3_x_2___3_x___1         PASS   factor(x^3 - 3*x^2 + 3*x - 1) ok
   18_x_4___1                       PASS   factor(x^4 - 1) ok
   TOTAL 18: pass=16 fail=0 hang=2
   ```
   (equality is checked in the `.ls` itself: `expand(expand(factor(p)) - expand(p))` must print `0`.)
3. **HOLDS.** Positive control unchanged: `factor(x^2 - 1) = (x - 1)*(x + 1)`; every positive-leading
   corpus entry (10-18) still passes.
4. **DOES NOT HOLD — the hang is NOT fixed by the sign fix.** `expand(factor(-x^2 - 2*x - 1))` still
   does not return within a 15 s hard kill after the fix (and did not before it).
5. **The two corpus entries that still hang are exactly:**
   - `-x^3 - x^2 + x + 1` (= `-(x+1)^2*(x-1)`), corpus entry 7
   - `-x^2 - 2*x - 1` (= `-(x+1)^2`), corpus entry 8

   Both are the negative-leading polynomials WITH A REPEATED ROOT — the same two the round-13 bisect
   found hanging before any kernel change was landed
   (`docs/goal-cycle-3/evidence.md` EVD-068/EVD-069), so they are pre-existing and untouched by this
   fix. Positive-leading repeated-root controls return immediately
   (`x^3 - 3*x^2 + 3*x - 1` -> `((x - 1))^3`). Per the round-18 instruction the hang is NOT being
   fixed this round: it is reported here and left as separate work.

## 5. xunit run of the required command

`dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~FactorMetamorphic"`

does not complete: the single fact walks the corpus in-process and stops at the first hanging entry
(entry 7, `-x^3 - x^2 + x + 1`), so there is no partial pass/fail output to paste — the process has
to be killed. That is exactly why the per-entry `corpus.ps1` table above is used as the acceptance
evidence for point 2. Raw log: `docs/goal-cycle-3/round-18/xunit_after.txt`
(pre-fix attempt also killed: `xunit_before.txt`).

## 6. Regression check outside the metamorphic test

`dotnet test Lovelace.Symbolics.Tests -c Release --filter "FullyQualifiedName!~FactorMetamorphic"`
(post-fix, `docs/goal-cycle-3/round-18/suite_after.txt`):

    Passed!  - Failed: 0, Passed: 434, Skipped: 6, Total: 440, Duration: 14 s

The 6 skips are the pre-existing `[RequiresSympyFact]` oracle tests, not something this round touched.
`SmokeTests.Factor_SquaresDifference_FullyFactors` (print form
`(x - 2)*(x - 1)*(x + 1)*(x + 2)`) and
`PolynomialsApiTests.Factor_QuarticWithRationalRoots_FourLinearFactors` (Content == 1) are the ones
that touch `FactorPoly`'s content directly; both use positive-leading inputs, whose multiplier is
`+1`, so `Content` is provably unchanged for them.

### 6b. Extra edge probes (all post-fix, `extra.ps1`, each `expand(expand(factor(p)) - p)` must print 0)

| case | p | result |
|---|---|---|
| e1 | `2*x + 2` (integer content) | `0` |
| e2 | `-2*x - 2` (negative content) | `0` |
| e3 | `-x^2 + 1/4` (rational leading coeff + rational content) | `0` |
| e4 | `-2*x^2 + 3*x - 5` (negative leading, no rational root) | `0` |
| e5 | `-x^3 + 2*x^2 + x - 2` | `0` |
| e8 | `-x^5 + x` (degree 5, negative leading) | `0` |
| e7 | `factor(-x^2 + 1)` printed form | `-(x - 1)*(x + 1)` — the sign is now explicit in the product |
| e6 | `gcd(x^2 - 1, x - 1)` | not available in the .ls surface (`Unknown function 'gcd'`); `GcdUnivariate` is untouched by the diff, so the Euclid loop is unaffected by construction |

## 7. Artifacts

- `Lovelace.Symbolics/Algebra/Factor.cs` — the fix (only source file changed)
- `docs/goal-cycle-3/round-18/implementation.md` — this report
- `docs/goal-cycle-3/round-18/probe.ps1` — the 4 probes (A/B/C/D)
- `docs/goal-cycle-3/round-18/corpus.ps1` — the 18-entry per-process acceptance runner
- `docs/goal-cycle-3/round-18/corpus/*` — one `.ls`, `.out.txt`, `.err.txt` per corpus entry
- `docs/goal-cycle-3/round-18/xunit_after.txt`, `xunit_before.txt`, `suite_after.txt` — raw logs
