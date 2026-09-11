# Round 20 — Yun square-free decomposition: value test → degree test

Status: **FIXED — all four acceptance points hold.** No commit (per instruction).

## Root cause (confirmed, reproduced)

`Lovelace.Symbolics/Algebra/Polynomial.cs:380` (pre-fix) — the Yun loop guard read

```csharp
while (!b.IsOne && !b.IsZero)
```

`IsOne` (Polynomial.cs:122) is true **only** for the constant `1`. When Yun's state reaches the
constant `-1` the loop re-enters, and from there the body is a provable no-op:

* `g = GcdUnivariate(b, d)` with `b` a nonzero constant returns the **monic** gcd `1`;
* `b = b/g = b` (unchanged), `c = d/g = d`;
* `d = Subtract(c, b.Derivative(0)) = d - 0 = d` (a constant's derivative is zero).

`b`, `c` and `d` are all invariant, so the loop spins forever.

## Fix (one line, degree-based termination)

```csharp
// Terminate on DEGREE, never on the value 1. A Yun state that reaches the constant -1
// (e.g. from -x^2 - 2*x - 1) is neither IsOne nor IsZero, and from there the body is a
// no-op ... -2 or 1/3 reach the same state, so the value test is the bug, not the set of
// values it misses; TotalDegree is 0 for every nonzero constant and -1 for zero.
while (b.TotalDegree > 0)
```

* `TotalDegree` already exists at `Polynomial.cs:124` and returns `-1` for zero, so a zero `b`
  also terminates without a second test.
* **No special-casing of `-1`**: `-2`, `1/3` etc. reach the same state; the value test was the
  bug, not the set of values it misses.
* `NormalizeUnivariate` and `GcdUnivariate` are **unchanged** — the monic normalisation is
  load-bearing for `GcdUnivariate`'s Euclid loop (DEC-007).
* `Lovelace.Symbolics/Algebra/Factor.cs` needed **no change**. The terminal constant is residual
  content and is already accounted for: `FactorPoly` folds the primitive part's leading
  coefficient into `Content` (the N13 fix, Factor.cs:50/79). Concretely, for `-x^2 - 2*x - 1`
  the terminal `b` is `-1`, which is exactly `primitive.LeadingCoefficient = -1`, already in
  `content`; the emitted monic factor is `(x + 1)^2`. So the constant is neither dropped nor
  double-counted — verified by the metamorphic equalities below.

Files touched: `Lovelace.Symbolics/Algebra/Polynomial.cs` only. Exact diff:

```diff
@@ -377,7 +377,13 @@
         var c = fPrime.DivRem(a, MonomialOrder.Lex).Quotient;
         var d = Subtract(c, b.Derivative(0));
         int i = 1;
-        while (!b.IsOne && !b.IsZero)
+        // Terminate on DEGREE, never on the value 1. A Yun state that reaches the constant -1
+        // (e.g. from -x^2 - 2*x - 1) is neither IsOne nor IsZero, and from there the body is a
+        // no-op -- g = GcdUnivariate(b, d) is the monic gcd 1, so b = b/g = b, c = d/g = d and
+        // d = c - b' = d - 0 = d. b, c and d are invariant and the loop spins forever.
+        // -2 or 1/3 reach the same state, so the value test is the bug, not the set of values
+        // it misses; TotalDegree is 0 for every nonzero constant and -1 for zero.
+        while (b.TotalDegree > 0)
         {
```

## TEST-FIRST: the hang, before/after (hard 15 s kill each)

Probe harness: a throwaway xunit test (`YunProbeScratchTests`, since deleted) that ran each
`Factor` inside `Task.Run` and abandoned it after a hard 15 s `Wait`. Raw evidence:
`docs/goal-cycle-3/round-20/probe-raw.txt`.

### BEFORE (commit state, `Polynomial.cs:380` unmodified)

```
=== EightProbes === 2026-09-11T07:31:17
-x^2+1           |    13 ms | factor -> -(x - 1)*(x + 1) | expand -> -x^2 + 1 | expand(factor(p))==p : True
-6*x^3+6*x       |     0 ms | factor -> -6*x*(x - 1)*(x + 1) | expand -> -6*x^3 + 6*x | expand(factor(p))==p : True
-2*x^2+8         |     0 ms | factor -> -2*(x - 2)*(x + 2) | expand -> -2*x^2 + 8 | expand(factor(p))==p : True
-x^4+5*x^2-4     |     0 ms | factor -> -(x - 2)*(x - 1)*(x + 1)*(x + 2) | expand -> -x^4 + 5*x^2 - 4 | expand(factor(p))==p : True
x^2-1            |     0 ms | factor -> (x - 1)*(x + 1) | expand -> x^2 - 1 | expand(factor(p))==p : True
x^3-3x^2+3x-1    |     0 ms | factor -> ((x - 1))^3 | expand -> x^3 - 3*x^2 + 3*x - 1 | expand(factor(p))==p : True
-x^3-x^2+x+1     | HANG | did not return within 15 s, abandoned -> 15008 ms
-x^2-2x-1        | HANG | did not return within 15 s, abandoned -> 15014 ms
```

`factor(-x^2 - 2*x - 1)` and `factor(-x^3 - x^2 + x + 1)` **never return**; killed at 15 s each.
The positive repeated-root control `x^3 - 3*x^2 + 3*x - 1` returns normally.

### AFTER (fix applied)

```
=== EightProbes === 2026-09-11T07:32:09
-x^3-x^2+x+1     |     4 ms | factor -> -((x + 1))^2*(x - 1) | expand -> -x^3 - x^2 + x + 1 | expand(factor(p))==p : True
-x^2-2x-1        |     0 ms | factor -> -((x + 1))^2 | expand -> -x^2 - 2*x - 1 | expand(factor(p))==p : True
```

Both previously-hanging inputs return in <5 ms and satisfy `expand(factor(p)) == p`.

## ACCEPTANCE

| # | Claim | Result | Evidence |
|---|-------|--------|----------|
| 1 | `factor(-x^2 - 2*x - 1)` and `factor(-x^3 - x^2 + x + 1)` RETURN under a 15 s kill | **HOLDS** | probe table above: HANG/15 s → 0 ms and 4 ms |
| 2 | `expand(factor(p)) == p` for ALL 18 corpus entries | **HOLDS** | `FactorMetamorphic` 1/1 passed, 45 ms |
| 3 | Positive controls unchanged; N13 cases still hold | **HOLDS** | `factor(x^2-1) = (x - 1)*(x + 1)`; `factor(x^3-3*x^2+3*x-1) = ((x - 1))^3`; `expand(factor(-x^2+1)) = -x^2+1`; `expand(factor(-6*x^3+6*x)) = -6*x^3+6*x` |
| 4 | Symbolics suite COMPLETES again, with counts | **HOLDS** | `Failed: 0, Passed: 435, Skipped: 6, Total: 441, Duration: 14 s` |

`FactorMetamorphicTests.cs` was **not** modified, skipped, weakened or deleted, and no expected
value was changed.

## VERIFY — observed output

```
> dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~FactorMetamorphic"
Passed!  - Failed:     0, Passed:     1, Skipped:     0, Total:     1, Duration: 45 ms - Lovelace.Symbolics.Tests.dll (net10.0)

> dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo
  Skipped Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Factorization_Agrees [1 ms]
Passed!  - Failed:     0, Passed:   435, Skipped:     6, Total:   441, Duration: 14 s - Lovelace.Symbolics.Tests.dll (net10.0)
```

The 6 skips are all `DifferentialOracleTests.SympyOracle_*` (the external-sympy oracle, opt-in in
this environment) — pre-existing, unrelated to this change.

### Eight probes (post-fix, `expand(factor(p))`)

| p | factor(p) | expand(factor(p)) == p |
|---|---|---|
| `-x^2+1` | `-(x - 1)*(x + 1)` | True |
| `-6*x^3+6*x` | `-6*x*(x - 1)*(x + 1)` | True |
| `-2*x^2+8` | `-2*(x - 2)*(x + 2)` | True |
| `-x^4+5*x^2-4` | `-(x - 2)*(x - 1)*(x + 1)*(x + 2)` | True |
| `x^2-1` | `(x - 1)*(x + 1)` | True |
| `x^3-3*x^2+3*x-1` | `((x - 1))^3` | True |
| `-x^3-x^2+x+1` | `-((x + 1))^2*(x - 1)` | True |
| `-x^2-2*x-1` | `-((x + 1))^2` | True |

### Solution build

```
> dotnet build LovelaceSharp.slnx -c Release --nologo
Build succeeded.
    0 Warning(s)
    0 Error(s)
Time Elapsed 00:00:04.12
```

## Still failing / notes

* Nothing in scope is failing. No known regression.
* The 6 sympy-oracle skips are environmental and pre-existing.
* Scope discipline: only `Lovelace.Symbolics/Algebra/Polynomial.cs` changed; the scratch probe
  test was deleted after collecting evidence.

## Artifacts

* `Lovelace.Symbolics/Algebra/Polynomial.cs` — the fix (loop guard at the Yun decomposition).
* `docs/goal-cycle-3/round-20/implementation.md` — this report.
* `docs/goal-cycle-3/round-20/probe-raw.txt` — raw before/after probe log.
