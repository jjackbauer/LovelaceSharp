# Round 26 — Closing the §116 metamorphic set: Gap 1 (A·A⁻¹ = I) and Gap 2 (cancel off poles)

Status: tests written and green. **No production code was touched.**

Artifact: `Lovelace.Symbolics.Tests/MetamorphicSetClosureTests.cs` (new file, 7 tests).

## Test-first honesty

Both capabilities already existed and every assertion below passed on the first full run of the
file (7/7). **These are not failing-first fixes; they are new coverage for two properties that had
no test.** The only two red runs during development were my own test bugs, and both were fixed in
the test, not worked around:

1. a mutation I built to be invisible at a sample point was not actually equal to the true inverse
   there (my algebra, not the kernel's);
2. the corpus case `(x-1)^2/(x-1)` does not reduce — that turned out to be a genuine kernel
   limitation, reported in §5 rather than asserted away.

What *did* have to be designed (and is the substance of this round) is the assertion path: the
obvious route is unavailable, so the test states the identity in a form the kernel can decide
exactly.

---

## Gap 1 — A·A⁻¹ = I, symbolically, under the condition the kernel emits

`SymbolicMatrix.InverseWithConditions` (SymbolicMatrix.cs:235) returns `adj/det` per entry plus
`AssumptionSet` = { `ExpressionPropertyAssumption(det, NonZero)` }.

### Why the naive route does not work (probe evidence, not an assumption)

For `A = [[x,1],[y,x]]`, `det = x² − y`, the kernel's own product A·A⁻¹ has entry (0,0)
`-y/(x^2 - y) + x^2/(x^2 - y)`, and

| attempt | observed canonical result |
| --- | --- |
| `Simplify.SimplifyExpr((A·A⁻¹)[0,0] − 1)` | `-y/(x^2-y) + x^2/(x^2-y) - 1` — **not** 0 |
| `Algebra.Expand(det · ((A·A⁻¹)[0,0] − 1))` | `y - 2*y*x^2/(x^2-y) - x^2 + x^4/(x^2-y) + y^2/(x^2-y)` — expand does not cancel the negative powers |

This confirms the comment the old numeric smoke test carried at SmokeTests.cs:164: the kernel
does not put a sum over a common denominator. Per the round's instruction, the fallback is **not**
numeric substitution — it is step 2 below (normalising by the denominator the condition names).

### The corpus (6 matrices, all symbolic)

| # | matrix | independently known determinant (ground truth) |
| --- | --- | --- |
| 1 | 2×2 general `[[a,b],[c,d]]` | `a*d − b*c` |
| 2 | 2×2 `[[x,1],[y,x]]` (the old numeric smoke matrix) | `x² − y` |
| 3 | 3×3 tridiagonal `[[x,1,0],[1,y,1],[0,1,z]]` | `x*y*z − x − z` |
| 4 | 3×3 cyclic `[[x,y,1],[0,x,y],[y,0,x]]` | `x³ + y³ − x*y` |
| 5 | 3×3 parameter cycle `[[1,a,0],[0,1,b],[c,0,1]]` | `1 + a*b*c` |
| 6 | 3×3 Vandermonde `[[1,x,x²],[1,y,y²],[1,z,z²]]` | `(y−x)(z−x)(z−y)` (expanded by `Algebra.Expand`) |

Matrices 3–6 satisfy the "at least one with a symbolic entry that makes the determinant genuinely
symbolic" requirement four times over (every determinant is a non-constant polynomial).

### What is asserted

Per corpus member:

1. **The condition is det ≠ 0 and the det is the right polynomial.**
   `result.Conditions.Atoms` has exactly one atom; it is an `ExpressionPropertyAssumption` with
   `P == SymbolPredicate.NonZero`; its expression equals `a.Det(ctx)`; and its *polynomial* equals
   the independently listed determinant of the table (exact polynomial equality — the ground truth
   is written by hand, it is not the kernel's `Det` output re-used as its own witness). The
   condition polynomial is asserted non-zero, so `0 != 0` cannot pass as a condition.
2. **The emitted denominator really is that determinant.** Every non-zero entry of the emitted
   inverse, split by the kernel's own `RationalFunctions.TryRationalize`, has denominator
   polynomial exactly equal to the condition's determinant. This is what licenses reading
   "A·A⁻¹ = I" as an identity on `det ≠ 0`.
3. **The identity itself.** With `inv[k,j] = n_kj/d_kj` and `D = Π_k d_kj`, the claim
   `(A·A⁻¹)[i,j] = δ_ij` is equivalent to the polynomial identity
   `Σ_k a_ik·n_kj·(D/d_kj) = δ_ij·D`, and that is what the test decides — with
   `Polynomial.TryFromExpr` + exact rational polynomial arithmetic (`Multiply`/`Add`/`Subtract`,
   exact `DivRem`). Result: zero polynomial for all 4+9+9+9+9+9 = 49 entries across the six
   matrices.

### Why this is a real check and not a tautology

* The candidate inverse is the **kernel's output**, never reconstructed from theory: the test
  factors nothing, calls no `Det`-based adjugate, and never re-derives `A⁻¹`.
* Polynomial equality over Q is exact and complete: if the emitted adjugate had a wrong entry, a
  wrong sign, or were transposed, the cleared difference would be a non-zero polynomial and
  `diff.IsZero` would be false. Nothing about the check "assumes" the answer.
* **It is demonstrated to fail** in
  `InverseIdentityCheck_RejectsInversesThatPassTheNumericSpotCheck`, which feeds four corrupted
  inverses of matrix 2 through the same checker: (a) transposed adjugate, (b) sign-flipped
  cofactor, (c) `+1` in a numerator, and (d) the decisive one — the numerator `-1` replaced by
  `-1 - (x-3)²`, an inverse that is **wrong identically but takes exactly the true inverse's values
  at (x,y) = (3,2)**. The test asserts, with the old smoke test's own numeric comparison, that the
  product of that fake inverse with A evaluates to `I` at `(3,2)` — i.e. the numeric check the
  previous test relied on *accepts it* — and then asserts the symbolic checker rejects it. That is
  the difference between "equal at a sample" and "equal identically"; a test that cannot fail is
  exactly what this rules out. The same test also asserts the *unmutated* inverse is accepted, so
  the checker is not merely rejecting everything.
* `InverseIdentity_HoldsOnlyWhereDetIsNonZero` closes the loop on the condition: for matrix 2 the
  determinant vanishes at `(x,y) = (2,4)` and evaluating the emitted inverse entries there raises
  `EvaluationException`, while at `(x,y) = (2,3)` (`det = 1`) the product evaluates to `I`. The
  identity is claimed exactly where the condition holds.

---

## Gap 2 — cancel preserves value off poles, and the pole is excluded

### Corpus — 9 members (8 with a removable singularity) plus one documented gap case

| input | kernel `cancel` output | poles | pole removed |
| --- | --- | --- | --- |
| `x/x` | `1` | 0 | yes |
| `x^2/x` | `x` | 0 | yes |
| `(x^2-1)/(x-1)` | `x + 1` | 1 | yes |
| `(x^2-4)/(x-2)` | `x + 2` | 2 | yes |
| `(x^2-2x+1)/(x-1)` | `x - 1` | 1 | yes |
| `(x^3-x)/(x^2-x)` | `x + 1` | 0, 1 | yes |
| `(x^3-8)/(x-2)` | `x^2 + 2*x + 4` | 2 | yes |
| `(x*y-y)/(x-1)` (multivariate) | `y` | 1 | yes |
| `(x^2+1)/(x-1)` (irreducible) | unchanged | 1 | **no** — nothing to cancel |
| `(x-1)^2/(x-1)` | unchanged | 1 | **no** — kernel gap, see §5 |

### Assertions

* **Structural**: the reduced form is the expected expression (so a "cancel" that returns the input
  is caught, and an over-eager cancellation is caught too).
* **(a) Off-pole numeric agreement, exact.** Sampled at `x ∈ {-4, -3/2, -1/3, 2/3, 4/3, 5/2, 3, 7}`
  (and `y = 3` for the multivariate member). Rationals of mixed sign and magnitude, exact
  arithmetic, so equality is asserted with `NumOps.Compare == 0` — **no tolerance**, because the
  identity is algebraic, not analytic.
* **Why those points are off-pole, and that this is not assumed**: the poles of the corpus are the
  roots of the removed common factors, all in `{0, 1, 2}`; the sample set is disjoint from them
  (asserted), and for every sampled point the test additionally asserts that the *original*
  evaluates without `EvaluationException`. So each point is machine-verified to be in the domain of
  both sides before the values are compared; the off-pole claim is checked, not stipulated.
* **(b) The pole is excluded.** At each declared pole the ORIGINAL throws
  `EvaluationException` (observed: `x/x` at 0, `(x^2-1)/(x-1)` at 1, `(x^3-x)/(x^2-x)` at 0
  and 1, `(x^3-8)/(x-2)` at 2, `(x*y-y)/(x-1)` at 1, `(x^2-2x+1)/(x-1)` at 1, …). Where the
  factor was genuinely removed the reduced form evaluates there without error, so `cancel` is NOT
  allowed to claim equality *at* the pole. For the irreducible member nothing is removed and the
  reduced form still throws at the pole — asserted, so "cancel always removes poles" is also
  rejected.
* **How the kernel represents the exclusion** (asserted in
  `Cancel_PoleExclusionIsCarriedAsANonZeroConditionByTheRewritePath`): the rat rewrite
  `rat.cancel-x-over-x` (Simplify.cs:264) is `RuleClassification.Conditional` and its
  `ConditionBuilder` is `NonZeroOf(cu)`; `Simplify.Transform(x/x)` returns expression `1` with
  exactly one condition — `SymbolPropertyAssumption(x, NonZero)` — and that step. Safe mode refuses
  the rewrite while the condition is unproven (`Simplify.SimplifyExpr(x/x) == x/x`), and licenses
  exactly that rewrite once `x != 0` is assumed. At the user surface the same exclusion appears as
  `simplify_full(x/x).conditions == [x != 0]` (observed, and already covered by
  DxStructuredResultsTests:122).

---

## 5. Findings outside this round's scope (production code NOT changed)

1. **The `cancel` builtin attaches no condition.** `RationalFunctions.Cancel` returns a bare
   expression, so `cancel((x^2-1)/(x-1))` yields `x + 1` with no `x != 1` attached, even though the
   rewrite path represents exactly that exclusion for the `x/x` shape. `cancel` is documented as an
   algebraic reduction, so this may be intended, but the value a user receives is defined where the
   input is not. The tests assert the true, checkable facts (values agree off-pole; the original is
   undefined at the pole) and do not assert "no condition exists".
2. **`cancel((x-1)^2/(x-1))` does not reduce.** `Polynomial.TryFromExpr` rejects `Pow(sum, k)`
   with reason *"Non-integer or non-variable powers are not polynomial."*, so `Cancel` gives up and
   returns the input; the expanded spelling `(x^2-2x+1)/(x-1)` *does* reduce to `x - 1`, which pins
   the cause to the parser, not to `GcdUnivariate`. Value-preserving but incomplete. Documented as
   a characterization test (`Cancel_UnexpandedPowerOfASum_IsNotReduced_KnownGap`) with the boundary
   asserted both ways; not fixed here. Note the rewrite path does not cover this shape either
   (`Simplify.Transform((x^2-1)/(x-1))` produces no steps), so the general rational reduction is
   `cancel`-only.

---

## 6. Verification (observed output)

All three commands were run after the final edit of the test file. Observed output (build/test
tails; the build's per-project lines were captured with the last 8 lines only):

```
> dotnet build LovelaceSharp.slnx -c Release --nologo
  Lovelace.Console.Tests -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Console.Tests\bin\Release\net10.0\Lovelace.Console.Tests.dll
  Lovelace.Studio.Tests -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Studio.Tests\bin\Release\net10.0\Lovelace.Studio.Tests.dll

Build succeeded.
    0 Warning(s)
    0 Error(s)

Time Elapsed 00:00:02.23
BUILD_EXIT=0
```

```
> dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo
  Skipped Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_MatrixOperations_Agree [1 ms]
  Skipped Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Derivatives_Agree [1 ms]
  Skipped Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Limits_Agree [1 ms]
  Skipped Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Factorization_Agrees [1 ms]

Passed!  - Failed:     0, Passed:   603, Skipped:     6, Total:   609, Duration: 13 s - Lovelace.Symbolics.Tests.dll (net10.0)
SYM_EXIT=0
```

The 6 skips are the pre-existing SymPy-oracle cases (skipped when the Python oracle is absent);
they are not related to this round and nothing was skipped or disabled here.

```
> dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo
[xUnit.net 00:00:00.22] Lovelace.Suite.Tests: Skipping test case with duplicate ID 'c9911250b046dcf84796305f808001a176905a85' ('...DocumentedExample_MatchesEngine(script: "1..5", ...)' and '...DocumentedExample_MatchesEngine(script: "1..5", ...)')
[xUnit.net 00:00:00.22] Lovelace.Suite.Tests: Skipping test case with duplicate ID '3aa1a90b76d7e8316e6da758d86b4c779819760b' (… same pattern, 4 more …)

Passed!  - Failed:     0, Passed:   637, Skipped:     0, Total:   637, Duration: 8 s - Lovelace.Suite.Tests.dll (net10.0)
SUITE_EXIT=0
```

The duplicate-ID notices are pre-existing xunit theory-data collisions in
`LanguageDocumentationTests`, not failures.

Focused run of the new file only (same build), for the count attributable to this round:

```
> dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~MetamorphicSetClosureTests"

Passed!  - Failed:     0, Passed:     7, Skipped:     0, Total:     7, Duration: 179 ms - Lovelace.Symbolics.Tests.dll (net10.0)
```

**Counts**: 7 new tests (3 for Gap 1, 4 for Gap 2). Symbolics suite: **609 total, 603 passed, 6
skipped, 0 failed** (602 + 7). Suite suite: **637 passed, 0 failed**. All suites end 0 failed.
Working tree: the only paths this round added are
`Lovelace.Symbolics.Tests/MetamorphicSetClosureTests.cs` and `docs/goal-cycle-3/round-26/implementation.md`
(the other entries in `git status` pre-date this round). No commit was made, by instruction.


## 7. Files

* `Lovelace.Symbolics.Tests/MetamorphicSetClosureTests.cs` — new: 7 tests (Gap 1: 3, Gap 2: 4).
* `docs/goal-cycle-3/round-26/implementation.md` — this report.
* No production file was modified; `git status` untouched by any commit (no commit made).
