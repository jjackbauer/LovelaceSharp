# Round 03 — the complex treatment of the rewrite-rule falsification gate

**Scope:** `Lovelace.Symbolics.Tests/FalsificationGateTests.cs` only (plus this document).
No production file was touched. `AtomHolds`, `PredicateHolds`, `Environments` and `Licensed` are
byte-identical to before: `git diff` shows **no removed line** mentioning them (the only added
references are calls to the pre-existing `AtomHoldsForTesting` accessor).

**Result:** every one of the 9 registered rewrite rules now has a stated complex treatment —
5 complex-**sampled**, 4 complex-**exclusion-proven** with an observed complex counterexample.
The assumption guard was not loosened, and no rule was compared at a complex point where it makes
no claim.

---

## 1. The 9 registered rules and their disposition

Disposition is computed at run time from the sweep, never hard-coded in the classifier:

* **complex-sampled** — the gate compared the rule's identity at ≥ 1 complex sweep point
  (`report.ComplexCompared` records rule, environment index and probe label).
* **complex-exclusion-proven** — **no** complex sweep point was compared, **every**
  (licensed environment, sweep point) pair is excluded by a **named failing atom**, and at least one
  complex point is **observed** where the identity fails.

| # | rule id | effective gate atoms (licensed envs) | disposition | complex sweep points compared | complex sweep points excluded by a named atom | observed complex counterexample | \|lhs−rhs\|² |
|---|---------|--------------------------------------|-------------|------------------------------:|--------------------------------------------:|---------------------------------|-------------:|
| 1 | `trig.pythagorean-sin2-cos2` | `{}` / `{Real}` | complex-sampled | 12 | 12 | — | — |
| 2 | `pow.sqrt-square-nonnegative` | `{NonNegative}` / `{NonNegative, Real}` | **complex-exclusion-proven** | 0 | 24 (12 per env) | `z = -1+i` | 7.99999999999999999999999999999999999999840000000000000000000000000000000000000008 |
| 3 | `pow.sqrt-square-real` | `{Real}` | **complex-exclusion-proven** | 0 | 12 | `z = 1+i` | 1.17157287525380990239662255158060384286036239115918795081252269802626601401347208 |
| 4 | `rat.cancel-x-over-x` | `{NonZero}` / `{Real}` | complex-sampled | 12 | 12 | — | — |
| 5 | `rat.cancel-zero-over-x` | `{NonZero}` / `{Real}` | complex-sampled | 12 | 12 | — | — |
| 6 | `logexp.exp-log` | `{Finite(log x)}` / `{Real}` | complex-sampled | 12 | 12 | — | — |
| 7 | `logexp.log-exp` | `{Real}` (realness is what discharges its strip condition) | **complex-exclusion-proven** | 0 | 12 | `z = 5i` | 39.47841760435743447533796399950460454125305343242719314921651611141238997164659364 |
| 8 | `abs.abs-square` | `{Real}` | **complex-exclusion-proven** | 0 | 12 | `z = 1+i` | 7.9999999999999999999999999999999999999991868229369548511720080901463285391281766813285584914208719806715813629799812554612525295214393237138388451216679481901056 |
| 9 | `abs.abs-neg` | `{}` / `{Real}` | complex-sampled | 12 | 12 | — | — |

Counts (asserted explicitly in `ComplexTreatment_PartitionsTheRegistry_WithExplicitCounts`):

| quantity | value |
|----------|------:|
| registered rules (`Simplify.RulesForTesting`) | **9** |
| `Simplify.ShippedRuleIds` | 9 |
| complex-sampled | **5** |
| complex-exclusion-proven | **4** |
| sampled + exclusion-proven | **9** |
| complex points skipped with **no** stated reason (`UnexplainedSkips`) | **0** |
| atom kinds the guard could not decide (`UnhandledAtoms`) | **0** |

The five sampled ids and the four proven ids are asserted as exact sets, so a rule that is added,
removed or switches disposition fails the partition test rather than silently changing the picture.

Observed classifier output (detailed run, §4):

```
COMPLEX TREATMENT (registered / complex-sampled / complex-exclusion-proven) = 9 / 5 / 4
  abs.abs-neg: ComplexSampled complexPointsCompared=12 envs=2 skips[named-atom=12,domain=0,unexplained=0]
  abs.abs-square: ComplexExclusionProven complexPointsCompared=0 envs=1 skips[named-atom=12,domain=0,unexplained=0] witness=z = 1+i (re=1, im=1)
  logexp.exp-log: ComplexSampled complexPointsCompared=12 envs=2 skips[named-atom=12,domain=0,unexplained=0]
  logexp.log-exp: ComplexExclusionProven complexPointsCompared=0 envs=1 skips[named-atom=12,domain=0,unexplained=0] witness=z = 5i (re=0, im=5)
  pow.sqrt-square-nonnegative: ComplexExclusionProven complexPointsCompared=0 envs=2 skips[named-atom=24,domain=0,unexplained=0] witness=z = -1+i (re=-1, im=1)
  pow.sqrt-square-real: ComplexExclusionProven complexPointsCompared=0 envs=1 skips[named-atom=12,domain=0,unexplained=0] witness=z = 1+i (re=1, im=1)
  rat.cancel-x-over-x: ComplexSampled complexPointsCompared=12 envs=2 skips[named-atom=12,domain=0,unexplained=0]
  rat.cancel-zero-over-x: ComplexSampled complexPointsCompared=12 envs=2 skips[named-atom=12,domain=0,unexplained=0]
  trig.pythagorean-sin2-cos2: ComplexSampled complexPointsCompared=12 envs=2 skips[named-atom=12,domain=0,unexplained=0]
```

---

## 2. The four exclusion proofs (observed, not asserted from theory)

Each control attacks the rule's **own instantiated LHS/RHS** (`GateReport.LicensedShapes`, recorded
by the gate itself, so a control cannot drift from what the gate swept) at complex points that are
deliberately **outside** the rule's assumption boundary. The gate still returns early at those
points — the guard is unchanged — and the control is what observes the failure.

### 2.1 `abs.abs-square` — `|z|²` vs `z²` at `z = 1+i`

```
EXCLUSION PROOF abs.abs-square: abs(x)^2 vs x^2 at z = 1+i (re=1, im=1) =>
  lhs=1.99999999999999999999999999999999999999979670573423871279300202253658213478204416,
  rhs=2*i,
  |lhs-rhs|^2 = 7.9999999999999999999999999999999999999991868229369548511720080901463285391281766813285584914208719806715813629799812554612525295214393237138388451216679481901056;
  blocked by SymbolDomainAssumption { S = x, D = Real } under {SymbolDomainAssumption { S = x, D = Real }}
```

`|1+i|² = 2` but `(1+i)² = 2i`: the identity fails by `|Δ|² = 8`. The realness atom is
load-bearing — without it the rule would be **false**, not merely unproven.

### 2.2 `pow.sqrt-square-nonnegative` — `sqrt(z²)` vs `z` at `z = -1+i`

```
EXCLUSION PROOF pow.sqrt-square-nonnegative: sqrt(x^2) vs x at z = -1+i (re=-1, im=1) =>
  lhs=0.9999999999999999999999999999999999999998 - 4999999999999999999999999999999999999999/5000000000000000000000000000000000000000*i,
  rhs=-1 + i,
  |lhs-rhs|^2 = 7.99999999999999999999999999999999999999840000000000000000000000000000000000000008;
  blocked by SymbolPropertyAssumption { S = x, P = NonNegative }
    under {SymbolPropertyAssumption { S = x, P = NonNegative }}
```

`(-1+i)² = -2i`, whose principal root is `1-i`, not `-1+i`: `|Δ|² = 8`. Note the blocking
environment here is the rule's own declared condition alone — the control point satisfies every
atom the rule has **except** the order predicate `x ≥ 0`, which is exactly the atom that must fail
for a complex number. (The second licensed environment `{NonNegative, Real}` blocks the same point
with both atoms; the witness is taken from the sharper first environment.)

### 2.3 `pow.sqrt-square-real` — `sqrt(z²)` vs `|z|` at `z = 1+i`

```
EXCLUSION PROOF pow.sqrt-square-real: sqrt(x^2) vs abs(x) at z = 1+i (re=1, im=1) =>
  lhs=0.9999999999999999999999999999999999999998 + 4999999999999999999999999999999999999999/5000000000000000000000000000000000000000*i,
  rhs=1.4142135623730950488016887242096980785696,
  |lhs-rhs|^2 = 1.17157287525380990239662255158060384286036239115918795081252269802626601401347208;
  blocked by SymbolDomainAssumption { S = x, D = Real } under {SymbolDomainAssumption { S = x, D = Real }}
```

`sqrt((1+i)²) = sqrt(2i) = 1+i` (principal branch), while `|1+i| = √2` — a complex value against a
real one, `|Δ|² = (1-√2)² + 1 = 1.17157…` (the observed digits agree). Over the complexes
`sqrt(z²) = |z|` is false, so the real-only classification is necessary.

### 2.4 `logexp.log-exp` — `log(exp(z))` vs `z` at `z = 5i`

```
EXCLUSION PROOF logexp.log-exp: log(exp(x)) vs x at z = 5i (re=0, im=5) =>
  lhs=-6415926535897932384626433832795028841971/5000000000000000000000000000000000000000*i,
  rhs=5*i,
  |lhs-rhs|^2 = 39.47841760435743447533796399950460454125305343242719314921651611141238997164659364;
  blocked by SymbolDomainAssumption { S = x, D = Real } under {SymbolDomainAssumption { S = x, D = Real }}
```

`lhs = i(5 − 2π) = −1.283185307179586476925286766559005768394…i` and `|Δ|² = (2π)² = 39.4784176…`,
both matching the observed digits.

**Honest note on the witness point.** This is the one rule whose witnessed failure lies **outside
its own side condition** as well: `log(exp(z)) = z` holds on the principal strip `Im z ∈ (−π, π]`,
and `5i` violates the strip. That is exactly why the witness is needed and why it is not a sweep
point: the sweep deliberately keeps `|Im z| ≤ 2 < π`, so *no sweep point can witness a failure of
this rule*. Two consequences are asserted directly
(`ComplexExclusion_LogExp_IsStoppedByRealness_NotByUndefinedness`), on a test-side mirror of the
registry's own condition (`Simplify.cs`, `logexp.log-exp`):

```
LOGEXP STRIP: strip atom holds at 1+i = True, at 5i = False; realness atom at 1+i = False
```

* at `1+i` the rule's **own** condition holds, yet the gate does not compare it there — the blocker
  is the realness atom, not "the rule is undefined over complex numbers";
* at `5i` the rule's own condition fails and the identity fails with it, so complex points really do
  include values where the claim is false. Treating a complex probe as real-like would therefore
  manufacture false confidence; the realness exclusion prevents precisely that.

---

## 3. Why the partition cannot be gamed

The audit (`ComplexTreatmentAudit`) enforces three things per rule, all as values a test can assert:

1. **No silent skip.** For every (licensed environment, sweep point) pair it re-evaluates each atom
   with the *same* guard the gate uses (`AtomHoldsForTesting`) and classifies the pair as
   *compared*, *blocked by a named atom*, or *undefined there* (`EvaluationException`). Anything
   else increments `UnexplainedSkips`; the partition test asserts it is 0 for all 9 rules, and also
   asserts `UnhandledAtoms` is empty, so no atom kind escapes the guard undecided.
2. **A real counterexample.** A rule may only be "exclusion-proven" if a witness was **observed**:
   atoms blocking, both sides evaluating, and `|lhs − rhs|² ≥ tol²` with the gate's own tolerance
   (1e-11). The test additionally requires `|Δ|² > 1`, i.e. an O(1) disagreement rather than
   rounding noise. If a rule is later "fixed" to hold over the complexes, the witness search finds
   no failure and the control fails loudly — and if its guard is dropped instead, the partition
   count changes and `ComplexTreatment_PartitionsTheRegistry_WithExplicitCounts` fails too.
3. **Exact points pinned.** The witness label of each of the four rules is asserted
   (`1+i`, `-1+i`, `1+i`, `5i`), so a control cannot drift to a friendlier point.

Gate strictness is preserved and re-verified in the same run:

* `Gate_Fails_WhenARuleThrows` (pre-existing) — still catches a deliberately throwing rule;
* `Gate_ReportsFalsification_WhenARuleIsWrong` (pre-existing) — still catches `sqrt(x²) → x`
  claimed unconditionally;
* `Gate_StillFalsifies_AComplexOnlyWrongRule` (**new**) — `z² → |z|²` is an identity on the reals
  and false at complex points, so only the complex region can catch it:
  `COMPLEX-ONLY WRONG-RULE CONTROL: Failed=True falsified=11`, first line
  `test.control-complex-only-wrong-rule at complex:1+i: x^2 != abs(x)^2`.
  This control exists specifically so the complex tolerance path cannot quietly stop reporting
  complex disagreements.

---

## 4. Exact commands and observed output

### 4.1 Gate subset (acceptance command)

```
> dotnet test Lovelace.Symbolics.Tests -c Release --nologo --filter "FullyQualifiedName~FalsificationGate"

Passed!  - Failed:     0, Passed:     9, Skipped:     0, Total:     9, Duration: 13 s - Lovelace.Symbolics.Tests.dll (net10.0)
```

Wall clock for that invocation (including restore check, build and test-host start): **18.3 s**.
Re-run on the final tree, with other agents building/testing the same checkout concurrently:

```
Passed!  - Failed:     0, Passed:     9, Skipped:     0, Total:     9, Duration: 17 s - Lovelace.Symbolics.Tests.dll (net10.0)
```

(wall clock 21.4 s; the 13 s → 17 s spread is shared-machine load, not test growth).

### 4.2 Whole Symbolics test project (acceptance command)

```
> dotnet test Lovelace.Symbolics.Tests -c Release --nologo

Passed!  - Failed:     0, Passed:   813, Skipped:     6, Total:   819, Duration: 25 s - Lovelace.Symbolics.Tests.dll (net10.0)
```

Wall clock for that invocation: **30.8 s**; **test time reported by the runner: 25 s**.
Re-run on the final tree (concurrent agents active on the same checkout):

```
Passed!  - Failed:     0, Passed:   813, Skipped:     6, Total:   819, Duration: 31 s - Lovelace.Symbolics.Tests.dll (net10.0)
```

(wall clock 36.7 s; runner test time 25 s → 31 s under shared-machine load).
The 6 skips are pre-existing `[Fact(Skip=…)]` sympy-oracle facts
(`DifferentialOracleTests.SympyOracle_{Solve,Roots,MatrixOperations,Derivatives,Limits,Factorization}_*`),
untouched by this round.

### 4.3 Gate cost

The gate does not repeat a registry sweep per fact: `SharedSweep` runs the sweep **once** and the
facts in the class read that same read-only `GateReport` (the sweep itself is unchanged, and the
existing registry test keeps every assertion). Gate subset test time is **13 s**, against ~15 s
before this round; the extra per-rule evidence costs one shared sweep, not one per fact. An earlier
version without the sharing measured 35 s of test time for the same 9 facts.

### 4.4 How the per-rule evidence was produced

The classifier/evidence lines above came from the same command with a detailed console logger:

```
> dotnet test Lovelace.Symbolics.Tests -c Release --nologo --filter "FullyQualifiedName~FalsificationGate" --logger "console;verbosity=detailed"
```

---

## 5. One change to the tolerance path, and why (with the evidence that forced it)

`ComparePoint` previously computed `delta = |lhs − rhs|` through `NumOps.Abs`, which for a complex
difference calls `Complex.Magnitude` → `Rl.Sqrt`. It now uses the **exact squared magnitude**
`re² + im²` for complex differences and keeps `|delta|` vs `tol` for real ones. Both sides are
non-negative, so `|Δ| ≥ tol ⟺ |Δ|² ≥ tol²`: the predicate is unchanged, and it is the sharper
form (no 40-digit square root before the comparison).

Reason, observed: with the concurrent uncommitted edit to `Lovelace.Real/Real.cs` in the working
tree, that square root throws for tiny magnitudes and aborts the **whole** gate inside
`ComparePoint`, e.g. the trig rule's complex delta at `z = i` has `magSq = 2.2640548519958362e-81`
and `Complex.Magnitude` threw `DivideByZeroException: Cannot divide a Real by zero`
(`Real.DivideNonPeriodic`, via `Real.Sqrt`, via `Complex.get_Magnitude`). Verified as a regression
from that uncommitted edit, not pre-existing: in a detached worktree at `3992d80` (HEAD, clean)
the same gate test passes with `trig.pythagorean-sin2-cos2: compared=78 regions=…,complex,…`.

`Lovelace.Real/**` is outside this round's scope, so it was not fixed here — this is reported to
the parent as a cross-cutting finding. Keeping the *rule-verdict* path off that square root is
correct on its own terms (a magnitude-defect must not decide a rule's verdict), and
`Gate_StillFalsifies_AComplexOnlyWrongRule` proves the complex path still reports real failures:
it falsifies the complex-only-wrong rule 11 times.

## 6. What is *not* claimed

* Complex sampling is falsification, not proof: "complex-sampled" means the identity was compared
  at the 12 complex sweep points, not that it is proven for all `z ∈ ℂ`.
* The four exclusion proofs are pointwise: each shows the identity is false at one exact complex
  point, which is what makes the real-only guard necessary. They do not claim the rules are false at
  *every* complex point (for `pow.sqrt-square-nonnegative`, `sqrt(z²) = z` does hold for
  `Re z > 0`).
* Evidence is a snapshot: it was produced with the working tree as it stood during this round,
  which included other agents' uncommitted edits to `Lovelace.Symbolics/**`, `Lovelace.Suite/**`
  and `Lovelace.Real/**` on top of `3992d80`.
