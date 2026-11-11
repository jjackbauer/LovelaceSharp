# Round 30 — ranked blocker families: the dispatch/semantics wave

Scope: `rust/lovelace-lang/src`, `rust/lovelace-symbolics/src`, `rust/lovelace-run/src`.
No `.cs` file, fixture, corpus or `parity/` artifact was changed; this report is the only
non-rust file. Oracle for every probe: the pinned runner
`dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --file <script> --omit-functions`
(`--file` is the same evaluation path as `--eval`; it avoids shell quoting of the scripts).
Comparison is byte-for-byte on the JSON envelope after dropping only the declared volatile fields.

Baseline: HEAD `ab12078`, cargo suite 4,304 tests green, corpus 212/214 (sym/071, sym/076),
fixtures oracle 20/20 + rust 20/20, CLI matrix 16/16. 976 deferred rows before this round
(the porting report `unblocked-porting.md` maps 140 of the round-29 rows).

## 1. Families selected, and why

The round-29 ledger ranked the unfixed causes. Four of them are DISPATCH/SEMANTICS corrections
(the fix names an existing rule and applies it) rather than a new subsystem, and were taken first;
the three cheap dispatch causes in ledger section 6 follow. The circular-family and Real-trig causes
need a family representation / a range-reduction subsystem and were left (section 5).

| Family | Rows | What it needed | Disposition |
|---|---:|---|---|
| Complex closed forms | 11 | exact power rule for a negative rational base; the kernel rule already existed but the wire power path did not use it | **FIXED** |
| inspect row #4 | 1 | `inspect((-4)^(1/2))`: the evaluation itself (same rule as above) | **FIXED** |
| Symbolics help metadata aliases | 17 | a help renderer over the existing registry + the category-alias table | **FIXED** |
| Symbolics pretty/latex grouping | 18 | one printer area: divisor flattening, subtrahend grouping, radical spelling, negative exponent | **FIXED 16 of 18** (2 need a complex leaf; section 5) |
| System builtin argument-shape refusals | 7 | argument-shape guards in front of `solve_system`/`solve_system_full`/`jacobian` | **FIXED** |
| Builtin shape guard/prose (audit reproductions) | 7 | guards for `matmul`/`symbol`/`assume`/`transpose` + one declared form | **FIXED 5 of 7** (the two whole-registry sweeps are not ported) |
| limit short-form record message | 3 (+1) | distinguish "leading coefficient is symbolic" from "coefficient does not evaluate at the point" | **FIXED** |
| **total expected to unblock** | **61** | | |

## 2. Per-family fixes and before/after probes

### 2.1 Complex closed forms (11 rows) + inspect #4 (1 row)

Cause: `Expr::pow` answered a negative rational base under a half-integer exponent (the existing
`principal_half_power`) but never folded a positive UNIT-fraction exponent whose root is exact
(`4^(1/2)`, `(-8)^(1/3)`), and the numeric power path of the language refused every non-integer
exponent before the kernel could see it.

Fix:
- `Expr::pow` (lovelace-symbolics): after the negative-base half power, an exact n-th-root test
  (`rational_root`/`exact_root_u128`) folds `4^(1/2) = 2`, `(1/4)^(1/2) = 1/2` and
  `(-8)^(1/3) = -2`; an inexact root and every other denominator stay a power, so
  `2^(1/2)` and `(-8)^(2/3)` are untouched.
- `pow` (lovelace-lang): the reference's ONE exact complex fallback (`NumericOps.PowerReal`): an
  exact negative base under the principal square root is routed to the kernel's `Power`
  (`(-a)^(1/2) = i*sqrt(a)`); every other non-integer exponent keeps the typed refusal
  (`2^(1/2)`, `(-4)^(3/2)`, `(-4)^(-1/2)` still refuse, exactly as the oracle does).
- The quadratic/cubic root builders now use the NAMED `i` constant instead of a symbol spelled
  `i`, so a complex solution's canonical form is `(i)`, its `free_symbols` is empty, and its
  exactness is the reference's `AlgebraicExact`. The real-domain filter and the exactness
  classifier were updated to see BOTH spellings (a regression this round introduced and then
  fixed: with only the named constant, the real-domain filter stopped recognising complex roots
  and answered `x^2 + 1` over the reals as Solved; caught by the full suite, fixed, re-measured).

| Probe | Oracle | Before | After |
|---|---|---|---|
| `(-4)^(1/2)` | `2*i` | `UnsupportedOperation: Non-integer exponents are not yet supported.` | `2*i` |
| `(-1)^(1/2)` / `(-3)^(1/2)` | `i` / `i*sqrt(3)` | refusal | equal |
| `inspect((-4)^(1/2))` | `Inspection(type: Symbolic, domain: complex, exact: True, ..., canonical: (mul (rat 2 1) (i)), pretty: 2*i, ...)` | refusal | oracle-equal |
| kernel `pow(-8, 1/3)` | `(rat -2 1)` | `(pow (rat -8 1) (rat 1 3))` | `(rat -2 1)` |
| kernel `pow(-8, 2/3)` | `(pow (rat -8 1) (rat 2 3))` | same | same (control) |
| kernel `sqrt(-1)`, `sqrt(-4)`, `sqrt(-2)`, `sqrt(-9)`, `sqrt(-1/4)` | `(i)`, `(mul (rat 2 1) (i))`, `(mul (i) (pow (rat 2 1) (rat 1 2)))`, `(mul (rat 3 1) (i))`, `(mul (rat 1 2) (i))` | already equal | equal (pinned by the new tests) |
| kernel `pow(-4, 3/2)` / `pow(-4, -1/2)` | `(mul (rat -8 1) (i))` / `(mul (rat -1 2) (i))` | already equal | equal |
| `i^2`, `i^3`, `i^4`, `i^-1` | `-1`, `-i`, `1`, `-i` | already equal | equal |
| `solve_full(x^2+1==0, x)` solution canonical | `(i)`, `(mul (rat -1 1) (i))` | `(sym i)`, `(mul (rat -1 1) (sym i))` | oracle-equal |
| exactness of the same roots | `AlgebraicExact` | `AlgebraicExact` (the classifier counted the symbol named `i`) | `AlgebraicExact`, now from `has_imaginary_unit` (the transient `Exact` after the named-`i` change is the regression recorded in 4.1) |
| `solve_full(x^2+1==0, x, real)` | `NoSolutions` | `NoSolutions` (the named-`i` change transiently made it Solved; see 4.1) | `NoSolutions` |
| `solve_full(x^2+2*x+5==0, x)` | `1/2*(-4*i - 2)`, `1/2*(4*i - 2)` (both expand to 0) | values right, ordering only | equal up to the C# test's own sort; substitution expands each root to 0 |

### 2.2 Symbolics help metadata aliases (17 rows)

Cause: the help/introspection surface did not exist in Rust at all, so the category-alias table and
the renderer built from the registry metadata could not be exercised. This is a metadata renderer,
not a second source of truth: it reads the existing `registry::BUILTINS` descriptors.

Fix: new `rust/lovelace-lang/src/help.rs` — the `HelpService` mirror: `signature` (required /
optional / variadic arity from the descriptor), `function` (signature, summary, examples,
`Returns:`, `See also:`, `Plugin:`), `category`, `funcs` (category listing, name-search
fallback, all-categories listing), `lookup` (function wins, then alias, then registered spelling)
and `overview`, over the reference's 25-entry category-alias table and 12-category display order.

Evidence (the rows are not wire-reachable; the C# assertions are transcribed):
- 13 aliases (`symbolic`, `symbolics`, `calculus`, `matrices`, `matrix`, `linear algebra`,
  `mathir`, `ir`, `compilation`, `solving`, `optimization`, `dsp`, `introspection`) resolve
  and none answers "No category".
- every one of the 123 registry builtins renders help with no "(no summary registered)",
  no "(no further help registered)" and no "Returns: Value" (the registry carries a summary and a
  declared return kind for all 123).
- `solve(f, x [, domain])`, `solve_full(f, x [, domain])`, `diff(f, x)` — the optional group is
  present exactly where `min_arity < parameters.len()`.
- `funcs("solve")` finds `solve(` and `solve_full(`; `funcs(None)` lists Solving, Symbolics and DSP.

### 2.3 Pretty / LaTeX grouping (16 of 18 rows)

Cause: the product arm accumulated ALL negative powers into one parenthesised denominator and the
additive arm dropped a subtrahend's parentheses; a 1/2 exponent printed as `x^(1/2)` and a
negative integer exponent as `x^(-2)`.

Fix (lovelace-symbolics, the ONE printer):
- `product_text` is the reference's `ShapeProduct`: one rational coefficient folded into the
  numerator, a denominator only for factors raised to EXACTLY -1, ONE `/` per denominator factor,
  and a divisor delimited by the one operand rule (product/sum/quotient delimited; power/symbol not).
  Denominator bases are rendered in canonical order.
- `signed_text` delimit a subtracted magnitude that is itself a sum: `x - (y - a)`.
- `power_text` renders `e^(1/2)` as `sqrt(e)`, `e^(-1/2)` as `1/sqrt(e)`, keeps a negative
  INTEGER exponent unparenthesised (`x^-2`), and a quotient base delimited.

Before/after over the full 47-source PrettyParenthesesTests corpus (oracle vs Rust): **11 → 1
differing renderings**. The one remaining difference is `diff(exp(x)*x^2, x)` term order
(`2*x*exp(x) + exp(x)*x^2` vs `exp(x)*x^2 + 2*x*exp(x)`), outside the grouping rows.

| Source | Oracle | Before | After |
|---|---|---|---|
| `x - (y - a)` | `x - (y - a)` | `x - y - a` | equal |
| `a - (x + y)` | `a - (x + y)` | `a - x + y` | equal |
| `(x/y)/z` | `x/y/z` | `x/(y*z)` | equal |
| `x/y/z` | `x/y/z` | `x/(y*z)` | equal |
| `x/y/z/w` | `x/w/y/z` | `x/(w*y*z)` | equal |
| `(x/y)/(z/w)` | `x/y/(z/w)` | `x/((z/w)*y)` | equal |
| `x/(y*z)` | `x/(y*z)` | `x/(y*z)` | equal (control) |
| `(-x)^(1/2)` | `sqrt(-x)` | `(-x)^(1/2)` | equal |
| `x^(1/2)` / `(x + 1)^(1/2)` | `sqrt(x)` / `sqrt(x + 1)` | `x^(1/2)` / `(x + 1)^(1/2)` | equal |
| `x^(-1/2)` | `1/sqrt(x)` | `x^(-1/2)` | equal |
| `x^(-2)` / `(x + 1)^(-1)` | `x^-2` / `(x + 1)^-1` | `x^(-2)` / `(x + 1)^(-1)` | equal |
| `1/(2*(x + 1))` | `(2*(x + 1))^-1` | `(2*(x + 1))^(-1)` | equal |
| `factor(-x^2 - 2*x - 1)` | `-(x + 1)^2` | `-(x + 1)^2` | equal |
| LaTeX of the eight grouping rows | `x - \left(y - a\right)`, `\frac{\frac{x}{y}}{z}`, `\frac{\frac{\frac{x}{w}}{y}}{z}`, `\frac{\frac{x}{y}}{\frac{z}{w}}`, `\sqrt{-x}`, `\frac{1}{\sqrt{x}}` | LaTeX arm already equal, but its tree DIFFERED from the pretty tree | equal, and the pretty tree now matches |

The LaTeX `GroupingParity` rows are unblocked by the pretty fix: the C# test compares the pretty
tree with the LaTeX tree, and the LaTeX arm was already the reference's; the Rust test now pins
both renderings for those eight sources.

### 2.4 System builtin argument-shape refusals (7 rows)

Cause: the system-solving builtins treated a scalar where a list is required as a one-element list
(`solve_system(x + y == 2, x - y == 0)` crossed as `InvalidOperation` prose) or accepted it
(`jacobian(x*y, [x, y])` answered a Jacobian).

Fix: a list-argument guard (`argument_list`) in front of `solve_system`, `solve_system_full` and
`jacobian`, publishing `InvalidArgument/TypeMismatch` with the reference's sentence naming the
builtin, the 1-based position, the expected list and the arrived kind.

| Call | Oracle | Before | After |
|---|---|---|---|
| `solve_system(x + y == 2, x - y == 0)` | `InvalidArgument: solve_system(): argument 1 must be a list of equations, e.g. [x + y == 2, x - y == 0]; got Symbolic.` | `InvalidOperation: solve_system needs variable symbols.` | equal |
| `solve_system_full(...)` | `... argument 1 must be a list of equations ...; got Symbolic.` | same prose | equal |
| `jacobian(x*y, [x, y])` | `... argument 1 must be a list of expressions, e.g. [x*y, x + y]; got Symbolic.` | accepted, `[[y, x]]` | equal |
| `solve_system([x + y == 2], x)` | `... argument 2 must be a list of parameter symbols, e.g. [x, y]; got Symbolic.` | `SystemSolveResult(Unevaluated)` | equal |
| `solve_system_full([x + y == 2], x)` | same | same | equal |
| `jacobian([x*y], x)` | `... argument 2 must be a list of parameter symbols ...` | accepted, `[[y]]` | equal |
| `solve_system([inspect(x^2)], [x, y])` | `... argument 1 must be a list of equations ...; got Vector.` | `InvalidOperation: solve_system needs equations.` | equal |

Positive control: the declared list forms still evaluate.

### 2.5 Builtin shape guard/prose — the audit's five reproductions (5 of 7 rows)

Cause: `det` already had the guard; `matmul`, `symbol`, `assume` and `transpose` did not (a scalar
was coerced, accepted, or refused as prose / as an arity error).

| Call | Oracle | Before | After |
|---|---|---|---|
| `det(1)` | `det(): argument 1 must be a square matrix; got Natural.` | equal | equal (control) |
| `matmul(1, 1)` | `matmul(): argument 1 must be an array or vector; got Natural.` | `InvalidOperation: Expected a matrix, but got 'Natural'.` | equal |
| `symbol(1)` | `symbol(): argument 1 must be a text name such as "x"; got Natural.` | answered `1` | equal |
| `assume(1)` | `assume(): argument 1 must be a relation such as x > 5; got Natural.` | answered `1` | equal |
| `transpose(1, 1)` | `transpose(): argument 1 must be an array or vector; got Natural.` | `transpose(): expected 1 to 2 arguments; got 2.` | equal |
| `assume_real("y")` (DeclaredForm #8) | `y` | `InvalidOperation: assume_real needs a symbolic variable.` | `y` |

`transpose` also gained its declared optional `perm` (the second argument) through the array
permutation API; the declared forms `det([[1,2],[3,4]])`, `matmul(...)`, `transpose(...)`,
`trace(...)`, `symbol("x")`, `assume(x > 5)` all still evaluate.

### 2.6 limit short-form record message (3 rows + DiagnosticContractTests row)

Cause: a series that cannot decide the limit always reported "coefficient does not evaluate at the
point"; the reference distinguishes a leading coefficient that is still symbolic.

Fix: when the series is empty, the message is "leading coefficient is symbolic" if the expression
carries a free symbol other than the limit variable, otherwise the existing point-evaluation
message.

| Call | Oracle | Before | After |
|---|---|---|---|
| `limit(a/x, x, 0)` | `diagnostics[0].message: leading coefficient is symbolic` | `coefficient does not evaluate at the point` | equal |
| `limit_left(a/x, x, 0)`, `limit_right(a/x, x, 0)` | same | same | equal |
| `limit(sin(x), x, inf)` (+ left/right) | `coefficient does not evaluate at the point` | equal | equal (control) |
| `limit_full(a/x, x, 0)` (DiagnosticContractTests) | same symbolic message | prose | equal |

## 3. Tests added

| File | Change |
|---|---|
| `rust/lovelace-symbolics/src/complex_closed_forms_round30.rs` | new: 9 kernel tests transcribing the 11 ComplexClosedFormTests methods (radicals, negative-base half powers, `i` powers, exact-third-root boundary, positive-base refusal, non-negative sqrt control); each asserts the canonical form, its round-trip and the real/imaginary parts through a test-side evaluator |
| `rust/lovelace-lang/src/help.rs` | new module + 5 unit tests reproducing the 17 HelpMetadataCompletenessTests rows |
| `rust/lovelace-run/src/round30_run_batch.rs` | appended 10 wire tests: the two complex quadratic solution sets (canonical roots, exactness, substitution-to-zero), inspect #4, the six system shape rows + the element row, the audit's five reproductions, the declared forms, the limit message and the short/full record identity |
| `rust/lovelace-run/src/pretty_batch.rs` | the six grouping sources added to ROUND_TRIP; the factored-square, flat-denominator and LaTeX grouping tests added |
| `rust/lovelace-lang/src/latex_and_sqrt_parity.rs` | one stale expectation corrected against the oracle: `x^(1/2)` renders `sqrt(x)` (the canonical form is unchanged; the re-parse assertion still holds) |

The kernel and wire tests are direct transcriptions of the C# assertions; the oracle strings are
the ones measured above. No assertion was relaxed.

## 4. Gates

```
$ cd rust; cargo test --workspace --no-fail-fast --offline
array 58, desktop 67, int 125, json 7, lang 1,140, nat 134, real 2,416, run 346,
symbolics 61 (+1 integration) = 4,355 tests collected, 0 failed (every target "ok";
the remaining 10 targets are doc-test/empty binaries).
Exit 0. (round-29 baseline 4,304; the porting agent's round-30 tests took it to 4,328;
+27 this wave: 9 kernel complex, 5 help, 10 wire, 3 pretty.)
```

The full run was compiled before the last TEST-only edit (the `assume_real("y")` row added to
`the_declared_audit_forms_stay_callable`); the targeted rerun after that edit is:

```
$ cargo test -p lovelace-run --offline round30     -> 12 passed, 0 failed
$ cargo test -p lovelace-lang --offline help::     ->  5 passed, 0 failed
$ cargo test -p lovelace-symbolics --offline complex_closed_forms -> 9 passed, 0 failed
```

```
$ rust/target/release/lovelace-corpus.exe       (from the repo root)
FAIL sym/071    (pre-existing)
FAIL sym/076    (pre-existing; linsolve simplifies x/x to 1 before printing)
corpus: 212 passed, 2 failed, 214 total
```

```
$ node parity/tools/gate-fixtures.mjs
oracle: 20/20
rust:   20/20
PASS: oracle 20/20, rust 20/20

$ node parity/tools/gate-cli.mjs
CLI matrix: 16/16
PASS: every RUN-001..RUN-014 row behaves like the oracle
```

```
$ node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize
generated: 200 cases, 157 match, 43 mismatch, 0 error
regressions replayed: 1,752, 706 match, 1,046 mismatch (the stored known-difference corpus)
harness exit: 1 (standing mismatches)
```

### 4.1 Regressions found and resolved during the round (recorded, not hidden)

The first full-suite run after the production edits reported 14 failures in `lovelace-lang`.
Thirteen were in test modules this round did not write and were fixed in PRODUCTION, not by
editing them: the named-`i` change had (a) stopped the real-domain filter from recognising a
complex root (`solve(x^2+1==0,x,real)` answered Solved, the cubic real rows kept three roots, and
five solve/descriptor rows moved) and (b) dropped the `AlgebraicExact` classification
(`contains_sym("i")` no longer fired). `has_imaginary_unit` (named constant OR symbol) restored
both; the failing modules were `robustness_round13`, `round27_batch`, `round28_batch` and
nine `suite_parity_batch` rows. The fourteenth,
`latex_and_sqrt_parity::nested_power_renderings_round_trip_through_the_parser`, pinned the OLD
Rust rendering of `x^(1/2)`; the oracle for the same script is `sqrt(x)` (measured), so the
expectation was corrected to the oracle value — a strict parity strengthening, not a weakened
assertion (the canonical re-parse assertion is unchanged).

The 43 generated mismatches are the standing classes, none of them on a surface this round changed:
array `norm` digit width (oracle 100 places, Rust 190), `linsolve` structural simplification,
`reshape` category prose, `inv` singular message, `print-budget`/record fields, the BOM/comment
syntax exit codes, and the two limit-expression gaps (`limit((1+1/x)^x, x, inf)` unevaluated;
one-sided limit conditions absent). This was measured: a scan of all 1,089 mismatch records found
ZERO mismatch whose differences are a rendering form (sqrt/`^-`/flat denominator), and every call
form changed in this round was probed directly against the oracle (section 2) and is byte-equal.

## 5. What remains, ranked

1. **Symbolics circular family (18 rows)** — not attempted as a family. The solver has no family
   representation for an inverse branch: `exp(x) == c` is Unevaluated where the oracle answers the
   family `log(c) + 2*k*pi*i` with condition `c != 0`; `sqrt(x) == x` and `log(x) == c` are
   Unevaluated; `sin/cos/tan` families are absent; the real-domain filter is ignored for
   `exp(x) == 5` (Rust answers the complex family); and `x*exp(x) == 1` is WRONGLY Solved with
   ±1. The ten circular rows whose result is already Unevaluated could be mapped with tests, but
   two of the ten still diverge, so the family is not complete.
2. **Pretty/latex: the two complex-constant rows** (`ModelLevel_PowerOfAComplexConstant`,
   `KnownRendering_PowerOfAComplexConstant`) — the Rust expression model has no complex constant
   leaf, so the rendered base cannot be built. Adding that leaf is a model extension, not a printer
   fix.
3. **The two whole-registry shape sweeps** (`BuiltinArgumentShapeSweepTests` #0 and the wide
   lattice #0) — not ported: a faithful port drives every one of the 123 builtins with 3 and 8
   wrong shapes and requires the documented grammar on every `InvalidArgument`.
4. **Real trig (12 rows)** — unchanged: needs a special-angle table and a big-magnitude range
   reduction (the 1e20/1e100 turn quotient overflows i64).
5. **Indeterminate forms (11 rows)** — not probed this round.
6. One renderer difference outside the ranked families: the additive display order for a sum
   containing a non-monomial product (`diff(exp(x)*x^2, x)`), a consequence of the reference's
   conditional polynomial ordering.

## 6. Files changed

| File | Change |
|---|---|
| `rust/lovelace-symbolics/src/lib.rs` | exact n-th root fold; named `i` in the quadratic/cubic roots; `has_imaginary_unit`; the printer rewrite (product/signed/power) |
| `rust/lovelace-symbolics/src/complex_closed_forms_round30.rs` | new kernel tests (9) |
| `rust/lovelace-lang/src/builtins.rs` | numeric negative-base principal-sqrt fallback; list-argument guards; `matmul`/`symbol`/`assume`/`transpose` guards and `perm`; `assume_real`/`assume_integer` text names; limit message; imaginary-unit detection in the real filter and exactness classifier |
| `rust/lovelace-lang/src/help.rs` | new: the help/introspection mirror + 5 tests |
| `rust/lovelace-lang/src/lib.rs` | `pub mod help;` |
| `rust/lovelace-lang/src/latex_and_sqrt_parity.rs` | one oracle-verified rendering expectation |
| `rust/lovelace-run/src/round30_run_batch.rs` | 10 wire tests appended |
| `rust/lovelace-run/src/pretty_batch.rs` | the unblocked pretty/latex rows |
| `docs/goal-parity-contract/round-30/ranked-families.md` | this report |
