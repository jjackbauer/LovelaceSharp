# Round 13 — robustness fixes (four production defects)

- **Date**: 2026-11-10
- **Oracle**: `Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll`, sha256
  `f292249fbd8d8a20e60e48942992a141f6315c84e14f291ecb53b453fde4a74a`
- **Scope**: `rust/lovelace-symbolics/src`, `rust/lovelace-real/src`,
  `rust/lovelace-lang/src` (production and tests). No `.cs`, fixture, corpus or
  `parity/` file was modified.
- **Baseline binary** (built before these changes):
  `rust/target/tmp/round13-baseline/lovelace-run-before.exe`, sha256
  `20a20ab12b74ad72b54739b80d1213a8750140fcf26bc15a83bba68a807eaeaa`
- **Final binary**: `rust/target/release/lovelace-run.exe`, sha256
  `21910e0d7599d62bbad6a89a9aa8ed23018405822e11aef170394aff6c021c81`

| Defect | Before | After | Oracle |
|---|---|---|---|
| 1. cubic solve panic | exit 101, empty stdout, `index out of bounds: the len is 4 but the index is 4` | exit 0, `Solved`, `2^(1/3)` | identical record |
| 2. 420-redex stack overflow | test thread `STATUS_STACK_OVERFLOW` (0xC00000FD); runner `Satisfied`, 0 steps, no budget | exit 0, `BudgetExceeded`, 400 steps, 400 conditions, diagnostic | same budget record |
| 3. root multiplicity | `x^2 - 2x + 1` → `1 × 1`, `Partial`; `(x-1)^2` → one `1 × 1` | `1 × 2`, `Solved`; `(x-1)^2` → two `1 × 1` | identical records |
| 4. exact terminating quotient | `1/(2^4000)`: 1003-char display, `exact=false`, no numerator/denominator | same value `exact=true`, numerator `1`, 1205-digit denominator | `exact=true`, numerator `1`, same denominator |

All probes below ran through the runner (`--omit-functions`) against the frozen
oracle; exit codes are the process exit codes.

---

## 1. CRASH — `solve_full(x^3 - 2, x, real)`

### Before

```text
$ rust/target/tmp/round13-baseline/lovelace-run-before.exe --eval \
    'x = symbol("x"); solve_full(x^3 - 2, x, real)' --omit-functions
exit=101   stdout: 0 bytes
stderr: thread 'lovelace-run' (10840) panicked at lovelace-symbolics\src\lib.rs:2382:13:
        index out of bounds: the len is 4 but the index is 4
```

### Root cause

`solve_biquadratic` read `coeffs[4], coeffs[2], coeffs[0]` without checking the
degree. A cubic with no rational root (`x^3 - 2`: four coefficients) falls through
the rational-root and quadratic paths and reached the biquadratic helper, which is
only valid for a quartic.

### Fix (`rust/lovelace-symbolics/src/lib.rs`)

- `solve_biquadratic` now refuses any coefficient list whose length is not 5.
- A remaining cubic is solved with the reference's Cardano algorithm
  (`cubic_root_expressions`): monic normalisation, the `x = t - A/3` depression,
  `disc = Q^2/4 + P^3/27`, the omega rotations for `disc >= 0` and the
  `acos`/cosine form for `disc < 0`.
- `Expr::has_algebraic_power` (and `acos` detection) makes a radical or
  trigonometric root `AlgebraicExact` instead of `Exact`.

### After (oracle-identical)

```text
$ rust/target/release/lovelace-run.exe --eval \
    'x = symbol("x"); solve_full(x^3 - 2, x, real)' --omit-functions
exit=0
SolveResult(status: Solved, variable: x, domain: real, complete: True, completeness: Complete,
solutions: [Solution(value: 2^(1/3), conditions: [], multiplicity: 1, exactness: AlgebraicExact)],
families: [], common_conditions: [], represented_count: 1, unrepresented_count: 0,
unrepresented_reason: , diagnostics: [])
```

The oracle prints the same record byte for byte. The complex spelling
`solve_full(x^3 - 2, x)` returns the same three roots (only the solution ORDER
differs from the oracle; values and exactness match). `x^3 - 3x + 1` (three real
roots) and `x^3 + x + 1` no longer panic either; `x^3 - 3x + 1` matches the
oracle on values and exactness, with a different order.

### Regression tests

`rust/lovelace-symbolics/src/lib.rs`:
`Solve_GivenCubicWithoutRationalRoot_ReturnsCardanoRoots`,
`Solve_GivenCubicWithoutRationalRoot_DoesNotPanicForEitherDiscriminantSign`.
`rust/lovelace-lang/src/robustness_round13.rs`:
`round13_solve_cubic_without_rational_root_returns_the_oracle_record`,
`round13_solve_cubic_without_rational_root_never_panics_in_any_branch`.

---

## 2. OVERFLOW — 420-redex `simplify_full`

The C# row (`DiagnosticContractTests.BoundedTransformation_CarriesABudgetExceededDiagnostic`)
builds `exp(log(x+1)) + ... + exp(log(x+420))` against `MaxSteps = 400`.

### Before

Two different observations, both real:

1. The runner's main thread did not abort at 420 terms — it answered the WRONG record:

```text
$ ...lovelace-run-before.exe --eval 'x = symbol("x"); simplify_full(<420 terms>)' --omit-functions
exit=0
status: Satisfied   budget_exceeded: false   budget_kind: ""   changed: false
steps: 0            conditions: 0
(oracle: status BudgetExceeded, budget_exceeded true, budget_kind rewrite_steps,
          changed true, steps 400, conditions 400, one transform.budget-exceeded diagnostic)
```

2. On the test harness's 2 MiB thread (how the C# row is actually ported), the same
input aborted the process before the budget could matter. The first run of the new
regression test printed:

```text
thread 'robustness_round13::round13_simplify_420_redexes_returns_budget_exceeded_instead_of_aborting'
has overflowed its stack
error: test failed ... (exit code: 0xc00000fd, STATUS_STACK_OVERFLOW)
```

A bare 300-term sum (no `simplify_full`) reproduced the same overflow, which
identified the recursion: a flat operator chain parses LEFT-DEEP, and the evaluator
recursed once per term. The parsed-tree budget (512) admits 420 terms, so the
stack — not the guard — failed first.

### Root cause

- `Engine::eval_expr` evaluated `Binary(op, l, r)` by recursing into `l`; a flat
  chain of N terms costs N frames. 420 terms is legitimate "wide" input.
- `simplify_full` had no rewrite budget at all: `TransformResult.budget_exceeded`
  was hard-coded `false`, and `transform.budget-exceeded` was never emitted.
- The conditional `logexp.exp-log` rule (the rewrite this input exercises) only fired
  for a symbol already known positive, so the reference's FiniteCondition path was
  missing.

### Fix (`rust/lovelace-lang/src/lib.rs`, `builtins.rs`)

- `eval_expr` now folds the SAME-operator left spine iteratively (strict left-to-right
  association, exactly the recursive semantics) and recurses only into the operands.
- `simplify_full` carries the documented `RewriteBudget { MaxSteps = 400,
  Kind = "rewrite_steps" }`; running out returns a `BudgetExceeded` TransformResult
  with `budget_exceeded: true`, `budget_kind: "rewrite_steps"`, the partial
  expression, and the `transform.budget-exceeded` diagnostic (message identical to
  the oracle).
- `simplify_full` applies `exp(log(u)) -> u` conditionally and records
  `FiniteCondition(expression: log(u))` records in both the step and the result's
  `conditions`, which is what makes this input consume the budget exactly as the
  oracle does.

### After

```text
$ rust/target/release/lovelace-run.exe --eval 'x = symbol("x"); simplify_full(<420 terms>)' --omit-functions
exit=0
status: BudgetExceeded   budget_exceeded: true   budget_kind: rewrite_steps
changed: true            steps: 400             conditions: 400
diagnostics: [Diagnostic(code: transform.budget-exceeded, category: BudgetExceeded,
  message: "the rewrite budget 'rewrite_steps' was exhausted; the returned expression is a partial result.")]
```

The oracle has the same status/budget/steps/conditions/diagnostic. The partial
expression is the same shape (400 rewrites collapsed into `400*x + constant` plus the
unrewritten `exp(log(...))` terms); the term ORDER differs because Rust canonicalises
sums differently, a pre-existing printer-ordering difference, not this defect.

### Regression tests

`rust/lovelace-lang/src/robustness_round13.rs`:
`round13_simplify_420_redexes_returns_budget_exceeded_instead_of_aborting` (the exact
C# row, run on the test thread that used to overflow),
`round13_simplify_below_the_budget_stays_satisfied`,
`round13_simplify_exp_log_carries_the_finite_condition_record`.

---

## 3. LOST INFORMATION — root multiplicity

### Before

```text
$ ...lovelace-run-before.exe --eval 'x = symbol("x"); solve_full(x^2 - 2*x + 1 == 0, x)'
exit=0
SolveResult(status: Partial, ..., complete: False, ..., solutions: [Solution(value: 1,
conditions: [], multiplicity: 1, exactness: Exact)], ..., represented_count: 1, ...)
```

### Root cause

- `solve_polynomial_named` deduplicated rational roots and always published
  multiplicity 1; the quadratic double root and the square-free multiplicity were
  discarded, so the record also under-counted the represented set (`Partial`).
- `solve_result` hard-coded `("multiplicity", 1)` for every solution variant.

### Fix (`rust/lovelace-symbolics/src/lib.rs`, `rust/lovelace-lang/src/builtins.rs`)

- New `Root { value, multiplicity }`; `SolveOutcome::Roots` carries it.
- Each rational root is divided out completely and counted; a zero-discriminant
  quadratic contributes one root with multiplicity 2; a linear remainder increments
  an existing root. `represented` is the SUM of multiplicities, so a repeated root no
  longer makes the result look Partial.
- `factor()` emits `(x - r)^m` for m > 1.
- The reference's structural branch is reproduced as well: `u^n = 0` with `u` a SUM
  is solved once per nth root (the C# dispatch does not expand a power of a sum), so
  `(x-1)^2 == 0` publishes two multiplicity-1 solutions.
- `SolveResult.solutions[].multiplicity` now comes from the factorization.

### After (both spellings oracle-identical)

```text
$ rust/target/release/lovelace-run.exe --eval 'x = symbol("x"); solve_full(x^2 - 2*x + 1 == 0, x)'
exit=0
SolveResult(status: Solved, ..., complete: True, ..., solutions: [Solution(value: 1,
conditions: [], multiplicity: 2, exactness: Exact)], ..., represented_count: 1, ...)

$ rust/target/release/lovelace-run.exe --eval 'x = symbol("x"); solve_full((x-1)^2 == 0, x)'
exit=0
SolveResult(status: Solved, ..., solutions: [Solution(value: 1, ..., multiplicity: 1, ...),
Solution(value: 1, ..., multiplicity: 1, ...)], ..., represented_count: 2, ...)
```

Also checked against the oracle: `x^3 - 3x^2 + 3x - 1` (1 × 3),
`(x^2 - 1)^2` (four × 1), `(x^2 - 2x + 1)^2` (two × 2), `x^2` (0 × 2).

### Regression tests

`rust/lovelace-symbolics/src/lib.rs`:
`Solve_GivenRepeatedRoot_PreservesMultiplicity`,
`Solve_GivenTripleRoot_PreservesMultiplicityThree`.
`rust/lovelace-lang/src/robustness_round13.rs`:
`round13_expanded_repeated_root_reports_its_multiplicity`,
`round13_factored_power_matches_the_oracles_inverse_branch`.

---

## 4. EXACTNESS — `Real::div` guard digit

### Probes

The four oracle probes at the default budget already agreed before the fix; the
defect appears as soon as a terminating expansion is longer than the digit budget
(the old loop pushed one digit PAST the budget and then flagged the value inexact).

```text
$ ...lovelace-run.exe --eval '1/4'   -> 0.25   exact=true numerator=1 denominator=4   (oracle identical)
$ ...lovelace-run.exe --eval '1/8'   -> 0.125  exact=true numerator=1 denominator=8   (oracle identical)
$ ...lovelace-run.exe --eval '10/3'  -> 3.(3)  exact=true numerator=10 denominator=3  (oracle identical)
$ ...lovelace-run.exe --eval '1/3'   -> 0.(3)  exact=true numerator=1 denominator=3   (oracle identical)
```

The defect probe, `1/(2^4000)` (a 4000-digit terminating expansion against the
default 1000-digit budget):

```text
BEFORE exit=0  display 1003 chars ("0." + 1001 digits: budget + one guard digit)
               exact=false   numerator absent   denominator absent
AFTER  exit=0  display 4002 chars ("0." + 4000 exact digits)
               exact=true    numerator=1         denominator=<same 1205-digit integer as the oracle>
ORACLE exit=0  display 102 chars (its display-precision cap)
               exact=true    numerator=1         denominator=<1205 digits>
```

At the `Real` API level the before/after is sharper, and it is exactly the three C#
`RealDivideTests` rows Round 12 refused (39- and 20-place `WithPrecision`):
`1/0.9993142073433579945` at 39 places now stores 39 digits (the old guard digit made it
40), and `1/0.99862888499828389309965043538706203025` at 20 places stores 20 (not 21),
so both equal the reference's exact truncations. More generally: `1/2048` at a 10-place budget
(`Real::from_i64(1).div(&Real::from_i64(2048), 10)`) used to store 11 digits and
report inexact; it now stores the exact 11-digit value and is exact, and a
NON-terminating quotient such as `1/291` at 10 places now stores exactly 10 digits
(no guard digit) and is the value that says inexact.

### Root cause

`Real::from_fraction` (the implementation of `Real::div`) charged a TERMINATING
expansion against `max_digits`, pushed one digit past it, and then set
`truncated = true`. A terminating rational is exact at every budget in the
reference.

### Fix (`rust/lovelace-real/src/lib.rs`)

- New `terminates(num, den)`: reduce, then strip all factors 2 and 5; if the
  remainder is 1 the expansion is finite and is generated in full, never truncated.
- A non-terminating expansion now stores exactly `max_digits` digits (no guard) and
  is the only one flagged inexact. The repeating block is decided at the budget
  boundary from the remainder that is already in `seen`, so a period that fits
  exactly at the boundary is still published exactly (the three
  `r7_sub_third_minus_*_threes` cases stay exact).

### Regression tests

`rust/lovelace-real/src/lib.rs`:
`div_round13_exact_terminating_quotients_are_exact_with_no_guard_digit`,
`div_round13_terminating_quotient_longer_than_the_budget_stays_exact`,
`div_round13_nonterminating_quotient_stores_exactly_the_budget`,
`div_round13_exact_long_terminating_quotient_is_not_truncated_by_the_default_budget`,
`div_round13_csharp_divisor_with_fractional_digits_rows` (the three C#
`RealDivideTests` rows Round 12 refused: the stored value was one guard digit too long).
`rust/lovelace-lang/src/robustness_round13.rs`:
`round13_exact_terminating_quotients_are_exact_with_no_guard_digit`,
`round13_terminating_quotient_longer_than_the_budget_stays_exact`.

**Not fixed (out of this defect):** the oracle's DISPLAY-precision cap. Its display
shows 100 decimals of `1/(2^4000)` while the exact value it publishes is the full
rational; Rust still renders the full expansion. The value, `exact` flag and
numerator/denominator now agree; the display-digit cap is a separate formatting
gap.

---

## Gates

### Rust tests

```text
$ cargo test --workspace        (cargo 1.99.1, workdir rust/)
exit=0
3657 passed, 0 failed, 0 ignored        (baseline 3,639 + 18 new tests)
```

Per crate: array 58, desktop 67, int 125, json 7, lang 727, nat 134, real 2,339,
run 162, recursion_depth 1, symbolics 37.

### Corpus

```text
$ rust/target/release/lovelace-corpus.exe
FAIL sym/071
FAIL sym/076
corpus: 212 passed, 2 failed, 214 total        exit=1
```

Identical before and after (`212/214`, `sym/071` and `sym/076` unchanged).
Note: a first run without the documented `TMP`/`TEMP` setting reported 211/214
because `lang/076`'s `plot()` could not write its SVG ("Access is denied"); with
`TMP=%repo%\rust\target\tmp` (the environment contract) it passes, as recorded
above. The baseline and final binaries both give 212/214 under the documented
environment.

### Protocol fixtures

```text
$ powershell -NoProfile -ExecutionPolicy Bypass -File parity/gate-fixtures.ps1
oracle: 20/20
rust: 20/20
PASS: oracle 20/20, rust 20/20        exit=0
```

Run with both the baseline and the final binary: identical (20/20, 20/20).

### Differential (before/after)

```text
$ node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize --out <tmp>
before: generated 200 -> 147 match / 53 mismatch / 0 error
        regressions replayed 1752 -> 438 match / 1314 mismatch / 0 error
        harness exit 1; rust sha 20a20ab1...; oracle sha f292249f...
after : generated 200 -> 147 match / 53 mismatch / 0 error
        regressions replayed 1752 -> 438 match / 1314 mismatch / 0 error
        harness exit 1; rust sha 21910e0d...; oracle sha f292249f...
```

**Delta: 0.** All 1,952 replayed/generated case statuses are identical before and
after (per-case comparison of both `cases.jsonl` files: 0 status changes; the
`stillMismatching` sets are the same 1,314 ids; no new regression). The generator's
`solve`/`solve_full` templates use `x^2-4`, `x^2+1`, `x^3-x` and
`simplify_full(x/x)`, and its arithmetic divisions are short terminating or periodic
quotients — none of the four defect shapes. The evidence for the fixes is therefore
the targeted oracle probes above, not the differential corpus; the 200-case gate is
unchanged, which also shows the fixes introduced no new mismatch.

## Files changed

| File | Change |
|---|---|
| `rust/lovelace-symbolics/src/lib.rs` | `Root` + multiplicity-aware `SolveOutcome::Roots`; square-free multiplicity; cubic Cardano solver; biquadratic length guard; `Expr::has_algebraic_power`; `apart`/tests updated; 4 new tests |
| `rust/lovelace-real/src/lib.rs` | `terminates`; exact terminating expansion; exact budget (no guard digit) and boundary period detection; 5 new tests |
| `rust/lovelace-lang/src/lib.rs` | iterative same-operator spine evaluation for flat binary chains; module registration |
| `rust/lovelace-lang/src/builtins.rs` | multiplicity in `SolveResult`; structural power-inverse branch; algebraic exactness; 400-step rewrite budget + `BudgetExceeded` record; conditional `logexp` with FiniteCondition records; `factor()` multiplicities |
| `rust/lovelace-lang/src/robustness_round13.rs` | new: 9 round-13 regression tests |

## Anything not fixed

- The four reported defects are fixed; no test, fixture or corpus example was
  weakened, skipped or deleted.
- The oracle's display-precision cap for very long exact Real quotients is still not
  ported (value/exact/numerator/denominator match; display length differs).
- Complex cubic root ORDER and the trigonometric cubic root ORDER differ from the
  oracle's canonical sort; values and exactness match.
- `solve_full((x-1)^2/(x-1) == 0)` still solves where the oracle refuses
  (`Unevaluated`): a pre-existing dispatch difference, unrelated to these defects.
