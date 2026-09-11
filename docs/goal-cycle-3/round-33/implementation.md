# Round 33 — N18: indeterminate forms returned a definite wrong value

**Status: FIXED and verified.** No git commit was made.
Scope honoured: `Lovelace.Symbolics/Constructors.cs` (the symbolic constant-folding code) and
`Lovelace.Symbolics.Tests/**` only. The N19 limit work was not touched.

---

## 1. Located rule (diagnosis, done BEFORE any change)

`inf` is the symbolic constant `NamedConstant.Infinity` (`Lovelace.Suite/Interpreter.cs:321` →
`new Value(Lovelace.Symbolics.Exprs.Infinity)`), so both reproductions are decided by the eager
canonicalizer in `Lovelace.Symbolics/Constructors.cs`, **not** by floating-point arithmetic.
Two rules fire, both guarded by the *same* predicate, and that predicate does not know about
Infinity.

### (a) `a - a = 0` — the zero-coefficient drop in `AddImpl`

```
Lovelace.Symbolics/Constructors.cs:194   if (coef.IsZero)                 // pre-fix line numbers
Lovelace.Symbolics/Constructors.cs:196       // a zero coefficient over a pole-carrying remainder must not vanish:
Lovelace.Symbolics/Constructors.cs:197       // dropping 0·x⁻¹ would define the sum at 0 where it is undefined
Lovelace.Symbolics/Constructors.cs:198       if (HasPoleRisk(rest))
Lovelace.Symbolics/Constructors.cs:199           terms.Add(Multiply(Rational(coef), rest));
Lovelace.Symbolics/Constructors.cs:200       continue;
...
Lovelace.Symbolics/Constructors.cs:205   if (terms.Count == 0)
Lovelace.Symbolics/Constructors.cs:206       return Rational(Rat.Zero);   // <-- the definite 0
```

`Exprs.Subtract(a, b)` is `Add(a, Multiply(-1, b))` (`Constructors.cs:631`). For `inf - inf`
the dictionary of like terms accumulates `like[inf] = 1 + (-1) = 0`; `coef.IsZero` is then true
and the term survives **only if** `HasPoleRisk(inf)` is true. It is false, so the term is
dropped, `terms.Count == 0`, and line 206 returns a definite `0`.

### (b) `0 * a = 0` — the zero-factor drop in `MultiplyImpl`

```
Lovelace.Symbolics/Constructors.cs:291   bool anyPoleRisk = flat.Any(HasPoleRisk);   // pre-fix
Lovelace.Symbolics/Constructors.cs:302       // 0·(pole) must stay visible: 0/x is undefined at 0 while 0 is defined
Lovelace.Symbolics/Constructors.cs:303       if (!anyPoleRisk) return Rational(Rat.Zero);   // <-- the definite 0
```
(same guard repeated for the RationalConstant at :315, RealConstant at :328 and
ComplexConstant at :342 branches)

### (c) The shared hole: `HasPoleRisk`

```
Lovelace.Symbolics/Constructors.cs:251   internal static bool HasPoleRisk(Expr e) => e switch
Lovelace.Symbolics/Constructors.cs:253       PowerExpr p => ... negative exponent or non-provable exponent
Lovelace.Symbolics/Constructors.cs:256-262   Add / Multiply / Function / Piecewise / Derivative / Integral / Relation recursion
Lovelace.Symbolics/Constructors.cs:263       _ => false,        // <-- NamedConstantExpr(Infinity) lands here
```

**Condition under which each rule currently fires (pre-fix):** the identity is applied whenever
the collapsing zero coefficient / zero factor meets a remainder for which `HasPoleRisk` returns
false — which is *every* remainder that is not a pole, including the Infinity named constant
(`_ => false`). The control `inf + 1` proves the folding code can decline: there the coefficient
is non-zero, so no identity is available and the sum stays canonical (`1 + inf`). The defect is
therefore a **missing definedness guard**, exactly as suspected, not a missing fold.

---

## 2. Step A — failing assertions first (observed, pre-fix)

New file `Lovelace.Symbolics.Tests/IndeterminateFormTests.cs` (11 tests) was written and run
**before** the kernel was touched:

```
  Failed Lovelace.Symbolics.Tests.IndeterminateFormTests.ZeroTimesInfinity_IsNotDefiniteZero [< 1 ms]
  Error Message:
   0 * inf is indeterminate and must not fold to a definite 0; got: 0
  Stack Trace:
     at Lovelace.Symbolics.Tests.IndeterminateFormTests.ZeroTimesInfinity_IsNotDefiniteZero() in ...\IndeterminateFormTests.cs:line 60
  Failed Lovelace.Symbolics.Tests.IndeterminateFormTests.InfinityMinusInfinity_IsNotDefiniteZero [< 1 ms]
  Error Message:
   inf - inf is indeterminate and must not fold to a definite 0; got: 0
  Stack Trace:
     at Lovelace.Symbolics.Tests.IndeterminateFormTests.InfinityMinusInfinity_IsNotDefiniteZero() in ...\IndeterminateFormTests.cs:line 49

Failed!  - Failed:     2, Passed:     8, Skipped:     0, Total:    10, Duration: 161 ms - Lovelace.Symbolics.Tests.dll (net10.0)
```

Same defect through the published/JIT runner, **before** the fix
(`docs/goal-cycle-3/round-33/probe.ps1`, `Lovelace.Run.dll --file <case>.ls --omit-functions`):

```
n18_inf_minus_inf          -> 0            (exit 0)
n18_zero_times_inf         -> 0            (exit 0)
n18_control_inf_plus_one   -> 1 + inf      (exit 0)
ctrl_sym_minus_sym         -> 0            (exit 0)
ctrl_zero_times_sym        -> 0            (exit 0)
```

---

## 3. The change (one bounded change)

A single new predicate plus the two call sites:

```
Lovelace.Symbolics/Constructors.cs:268-290   internal static bool HasUndefinedRisk(Expr e) => e switch
Lovelace.Symbolics/Constructors.cs:278           NamedConstantExpr n => n.Constant == NamedConstant.Infinity,
                                                 PowerExpr p  => negative/unknown exponent, or HasPoleRisk / HasUndefinedRisk on the base
                                                 Add / Multiply / Function / Piecewise / Derivative / Integral / Relation recurse
                                                 _ => false
Lovelace.Symbolics/Constructors.cs:200       if (HasUndefinedRisk(rest))            // AddImpl zero-coefficient guard
Lovelace.Symbolics/Constructors.cs:317       bool anyUndefinedRisk = flat.Any(HasUndefinedRisk);   // MultiplyImpl
Lovelace.Symbolics/Constructors.cs:330/342/355/369   if (!anyUndefinedRisk) return Rational(Rat.Zero);
```

`HasPoleRisk` is kept intact (its name stays accurate) and is still consulted by the new
predicate; `HasUndefinedRisk` is the guard the two folding rules now use.
`Pi`, `E` and `I` are deliberately **not** hazards — they are finite, so `pi - pi = 0` and
`0·pi = 0` still fold. Both guards recurse structurally, so the hole is also closed one level
down (e.g. `(inf + x) - (inf + x)`).

Net effect: the tree is no longer erased; it collapses to the canonical *indeterminate product*
`0*inf`, exactly the treatment `0/x` and `x⁻¹ - x⁻¹` already received from the pole guard.

---

## 4. Chosen correct result, and why

**Chosen: an unevaluated expression — the canonical product `0*inf` — for both `inf - inf` and
`0 * inf`, with exit code 0.** Preference order in the brief puts unevaluated first, and this is
also what the codebase already does for every other indeterminate-looking form:

* it is the *established* pattern in the very same rules: `0·x⁻¹` is kept visible rather than
  dropped (`Constructors.cs:328-330`), and `x⁻¹ - x⁻¹` stays `x^-1 - x^-1`.
  `0*inf` is the natural extension of that contract to the non-finite constant;
* it matches the honest control `inf + 1 → 1 + inf`: the kernel declines to invent a value;
* no new structural error code is introduced, so no consumer (interpreter, MathIR, Studio) has to
  learn a new failure vocabulary for what is a *value* question, not a *typing* question. A typed
  error remains the right answer for `0/0`, which the interpreter already raises.

Rejected: returning `0` (forbidden — it asserts a value the form does not have, and it silently
turns `inf - inf` into a finite result that later arithmetic treats as trustworthy).

---

## 5. Sibling identities in the same fold code (N18 sweep)

| identity | where | status | same root cause? |
|---|---|---|---|
| `a - a = 0` | `AddImpl`, `Constructors.cs:194-208` (zero coefficient) | **FIXED** | yes |
| `0 * a = 0` | `MultiplyImpl`, `Constructors.cs:317-369` (four numeric branches) | **FIXED** | yes |
| `a / a = 1` | `Divide` → `Multiply(a, a⁻¹)`; the pos/neg exponent split at `Constructors.cs:413-428` | **no hole** — `inf/inf` stays `inf/inf` (exit 0), `x/x` stays `x/x`; the pole-preserving merge already refuses to cancel | n/a |
| `0 / 0` | `Divide(0,0)` → `0·0⁻¹`; the kernel keeps it visible because `HasPoleRisk(0⁻¹) = true` | **no hole** — kernel stays unevaluated; the runner surfaces it as the typed recoverable error `Cannot divide by zero.` (exit 1) | n/a (already guarded) |
| `a^0 = 1` | `Power`, `Constructors.cs:454` (`if (eRat is { IsZero: true }) return Rational(Rat.One);`) | **fired: `inf^0 → 1`** | **no** — it is a different rule (exponent-zero, not a zero coefficient/factor) and `x^0 = 1` is this kernel's convention for every base, including `0^0 = 1`. Left unchanged, and pinned by a documenting test. |

Only the two rows marked FIXED share the root cause; nothing else was changed.

---

## 6. Step C — passing output (observed, post-fix)

```
Passed!  - Failed:     0, Passed:    11, Skipped:     0, Total:    11, Duration: 49 ms - Lovelace.Symbolics.Tests.dll (net10.0)
```
(new `IndeterminateFormTests` class; the two former failures now pass, including the pinned
`0*inf` shape assertion and the `(inf - inf) + 1` non-resurrection case)

### Full verification (observed)

```
> dotnet build LovelaceSharp.slnx -c Release --nologo

Build succeeded.
    1 Warning(s)      (pre-existing xUnit2013 in Lovelace.Suite.Tests/EmptyReductionTests.cs:47)
    0 Error(s)
Time Elapsed 00:00:03.20

> dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   734, Skipped:     6, Total:   740, Duration: 16 s - Lovelace.Symbolics.Tests.dll (net10.0)

> dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   640, Skipped:     0, Total:   640, Duration: 11 s - Lovelace.Suite.Tests.dll (net10.0)

> dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:    39, Skipped:     0, Total:    39, Duration: 257 ms - Lovelace.Run.Tests.dll (net10.0)
```

All suites end **0 failed**. (The 6 Symbolics skips are pre-existing and were not touched.)

### JIT runner probes (`probe.ps1`, post-fix binary)

```
n18_inf_minus_inf            exit=0 -> 0*inf        <-- was "0" (definite, wrong)
n18_zero_times_inf           exit=0 -> 0*inf        <-- was "0" (definite, wrong)
n18_control_inf_plus_one     exit=0 -> 1 + inf      <-- unchanged, honest control
ctrl_sym_minus_sym           exit=0 -> 0            <-- x - x still folds (required)
ctrl_zero_times_sym          exit=0 -> 0            <-- 0 * x still folds (required)
n18_inf_minus_inf_plus_1     exit=0 -> 1 + 0*inf    <-- no resurrection of the 0
sib_inf_over_inf             exit=0 -> inf/inf
sib_inf_pow_zero             exit=0 -> 1            (unchanged by decision, see §5)
sib_zero_over_zero           exit=1 -> ERR Cannot divide by zero.
sib_sym_over_sym             exit=0 -> x/x
sib_sym_pow_zero             exit=0 -> 1
ctrl_sym_pole_identity       exit=0 -> x^-1 - x^-1
```

Raw envelopes for every probe are stored beside the scripts as
`docs/goal-cycle-3/round-33/<case>.ls`, `<case>.out.txt`, `<case>.err.txt`.

---

## 7. Files changed (no commit)

| file | change |
|---|---|
| `Lovelace.Symbolics/Constructors.cs` | added `HasUndefinedRisk` (Infinity-aware, structural) at :268-290; `AddImpl` zero-coefficient guard now uses it (:200); `MultiplyImpl` zero-factor guard now uses it (:317, :330/342/355/369); stale comments updated |
| `Lovelace.Symbolics.Tests/IndeterminateFormTests.cs` | new: 11 tests — the 2 N18 assertions, the `inf + 1` control, the nested case, 3 forbidden-regression guards (`x - x`, `0*x`, `pi - pi`/`0*pi`), and 4 sibling sweep tests |
| `docs/goal-cycle-3/round-33/probe.ps1` | new: JIT-runner probe harness (12 cases, exit codes, raw envelopes) |
| `docs/goal-cycle-3/round-33/implementation.md` | this report |

No test was deleted, skipped or weakened; no expected value was changed to match the old output.

## 8. Not fixed (out of scope / deliberately unchanged)

* **`inf^0 → 1`** — a different rule, this kernel's own convention (`x^0 = 1`, `0^0 = 1`).
  Reported, documented by a test, deliberately unchanged.
* **`inf + 1`** — already correct.
* **N19 (limits)** — untouched, out of scope.
* `0*inf` is *rendered* as a product rather than as `inf - inf`; the form is indeterminate and
  unevaluated in both shapes, which is what the objective requires. No consumer currently branches
  on that rendering (all four suites pass).
