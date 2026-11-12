# Round 57 — the RationalFunctions.Cancel block: seven mirrors and the additive rational/matrix surface

**Status: the seven census rows the round-50 re-sweep grouped under
"RationalFunctions.Cancel/CancelWithConditions and SymbolicMatrix inverse APIs" are unblocked.
The additive Rust surface is in place and every gate is green.** Four rows are the cancel API
itself; three are the `SymbolicMatrix.InverseWithConditions` rows that share the
`TryRationalize`/non-zero-condition machinery. Per this lane's scope, no `.cs` file, fixture,
`parity/` file or other document was touched; the manifest entries are still `deferred` and the
mapping proposal is section 4.

Measured 2026-11-11 (America/Sao_Paulo) on the current working tree.

## 1. What this round changed

* **New:** `rust/lovelace-symbolics/src/rational_functions.rs` — `try_rationalize`, `cancel`,
  `cancel_with_conditions`, `non_zero_condition`, `CancelResult` and `CancelStatus`, plus the
  sparse multivariate polynomial division, Euclidean gcd and canonical rendering the reference's
  `Polynomial.DivRem`/`GcdUnivariate`/`ToExpr` provide. Parsing reuses the existing
  `groebner::Polynomial::from_expr`; expansion reuses the crate's `expand`.
* **New:** `rust/lovelace-symbolics/src/symbolic_matrix.rs` — `SymbolicMatrix` (rows, get,
  transpose, multiply, Bareiss determinant) and `MatrixInverseResult` (adjugate-over-determinant
  inverse with the det != 0 condition). This is the smallest surface the three inverse rows need.
* **Edited:** `rust/lovelace-symbolics/src/groebner.rs` — one additive method,
  `Polynomial::div_rem` (the reference's multivariate `DivRem` under a monomial order). No
  existing method changed and no existing caller changed.
* **Edited:** `rust/lovelace-symbolics/src/lib.rs` — two additive `pub mod` lines.
* **Untouched:** every `.cs` file, fixture, corpus file, `parity/` file, the runner envelope and
  every existing symbolics behaviour. In particular the crate-root `cancel` function is unchanged
  (its measured `(x^2+1)/(x-1)` defect belongs to the separate `cancel` builtin row, not to the
  seven rows here).

## 2. What the C# source and tests observe

Sources read: `Lovelace.Symbolics/Algebra/RationalFunctions.cs`,
`Lovelace.Symbolics/Algebra/Polynomial.cs`, `Lovelace.Symbolics/Matrices/SymbolicMatrix.cs` and
`Lovelace.Symbolics.Tests/MetamorphicSetClosureTests.cs` (the seven test methods behind the seven
rows; `CancelFull_ReportsTheConditionsTheBareValueCannotCarry` was already ported in round 52).

1. **The returned pair.** `TryRationalize(e, ctx, out n, out d)` collects the expression's symbols
   (sorted); with no symbols it returns `(e, 1)`. It walks the canonical sum term by term and
   splits each term's factors: a factor that is a power with a negative integer exponent contributes
   that base repeatedly to the denominator, every other factor stays in the numerator. Each
   numerator and denominator must parse as a polynomial over the collected symbols
   (`Polynomial.TryFromExpr`); otherwise the call refuses. A single term is split exactly. A sum
   is deliberately **not** put over a common denominator: the reference returns `(e, 1)`.
2. **The bare value.** `Cancel(e, ctx)` is exactly `CancelWithConditions(e, ctx).Expression`. It
   carries no condition, and the reference's own documentation says that is a definedness
   extension: the result agrees with the input off the removed poles and may be defined at them.
3. **The cancelled factors.** `CancelWithConditions` reduces one additive term at a time with
   `SplitSingle` + `TryPolynomial` (a direct polynomial parse, or one expansion and a second
   parse when the parser refuses a power of a sum), then takes the monic Euclidean gcd
   `Polynomial.GcdUnivariate(n, d)`. When the gcd is one or zero the whole term is kept unchanged.
   Otherwise the quotient `n/g` and `d/g` are published as `n/g` when the reduced denominator is
   not the constant one, and as the bare numerator otherwise.
4. **The conditions.** The removed factor `g` is exactly what the reduced form may not assume away:
   one NonZero atom per removed factor. A symbol becomes a `SymbolPropertyAssumption`; anything
   else becomes an `ExpressionPropertyAssumption`. `NonZeroCondition(e)` builds the same atom in
   isolation. `CancelStatus` is `Exact` when there are no conditions and `Conditional` otherwise;
   `Changed` is structural inequality of expression and original.
5. **Refusals.** A term whose numerator or denominator is not a polynomial (for example `sin(x)`)
   is kept unchanged with no condition. A quotient with no common factor (for example
   `(x^2+1)/(x-1)`) is likewise unchanged, `Exact`, `Changed == false`; the pole is not removed.
   A constant expression is unchanged and exact.
6. **The three inverse rows.** `SymbolicMatrix.Det` is the reference's Bareiss fraction-free
   determinant; `InverseWithConditions` returns the adjugate-over-determinant matrix and exactly
   one `ExpressionPropertyAssumption(det, NonZero)` (even when the determinant is a bare symbol).
   A singular matrix returns a null matrix with the note `matrix is singular`. The rows assert
   (a) the condition is the independently known determinant and the cleared-denominator identity
   `A·A⁻¹ = I` holds as an exact polynomial identity; (b) an inverse that is correct only at the
   old numeric spot-check point (3,2) is rejected by the symbolic check; and (c) on the zero set of
   the determinant the inverse entries are undefined, while off it the product is the identity.

Deliberate representation differences (none of the seven rows observes them):

* The C# `bool TryRationalize(..., out n, out d)` becomes `Option<(Expr, Expr)>`; the default
  `(1, 1)` refusal pair cannot escape.
* The C# `CancelResult` record becomes a struct with the same three fields and `changed()` /
  `status()` methods; `CancelStatus` keeps the kernel spellings through `name()`.
* C# has no explicit division node (a quotient is a negative power). Rust has `Expr::Div`, so
  `split_single` accepts it as a quotient too; `Expr::div` still produces the reference's
  product-of-a-negative-power shape.
* A negative **non-integer** exponent is left in the numerator and makes the parse refuse. The
  reference's integer cast truncates such an exponent to zero and silently drops the factor; that
  is a reference bug this port does not repeat, and no row observes it.
* The matrix surface is exact-expression in / exact-expression out; there is no numeric evaluator
  or `Real` entry path in this crate. The three rows are symbolically decidable without one.

## 3. Member map

| C# member | Rust member | Observable |
|---|---|---|
| `RationalFunctions.TryRationalize(e, ctx, out n, out d)` | `rational_functions::try_rationalize(&e) -> Option<(Expr, Expr)>` | the numerator/denominator pair, or `None` when a part is not a polynomial |
| `RationalFunctions.Cancel(e, ctx)` | `rational_functions::cancel(&e) -> Expr` | the reduced value alone; no condition |
| `RationalFunctions.CancelWithConditions(e, ctx)` | `rational_functions::cancel_with_conditions(&e) -> CancelResult` | expression + conditions + original |
| `RationalFunctions.NonZeroCondition(e)` | `rational_functions::non_zero_condition(&e) -> AssumptionSet` | one symbol-property or expression-property NonZero atom |
| `CancelResult.Original` / `.Expression` / `.Conditions` | `CancelResult::original` / `.expression` / `.conditions` | the three published fields |
| `CancelResult.Changed` | `CancelResult::changed()` | expression != original |
| `CancelResult.Status` | `CancelResult::status()` | `CancelStatus::Exact` iff no conditions |
| `RationalFunctions.CancelStatus` | `rational_functions::CancelStatus` with `name()` | `Exact` / `Conditional` |
| `SymbolicMatrix.From(Expr[][])` | `symbolic_matrix::SymbolicMatrix::from_rows(Vec<Vec<Expr>>)` | `Option`; rectangular only |
| `SymbolicMatrix.Rows` / `.Columns` / indexer | `rows()` / `columns()` / `get(r, c) -> Option<&Expr>` | shape and entries |
| `SymbolicMatrix.Det(ctx)` | `SymbolicMatrix::det() -> Option<Expr>` | Bareiss exact determinant; `None` for non-square |
| `SymbolicMatrix.InverseWithConditions(ctx)` | `SymbolicMatrix::inverse_with_conditions() -> MatrixInverseResult` | `{ matrix, conditions, note }`; one det != 0 atom, or `None` + `matrix is singular` |
| `SymbolicMatrix.Multiply(a, b)` | `SymbolicMatrix::multiply(&self, &other) -> Option<SymbolicMatrix>` | exact product; `None` on shape mismatch |
| `SymbolicMatrix.Transpose()` | `SymbolicMatrix::transpose()` | transposed matrix |
| `Polynomial.DivRem(divisor, order)` | `groebner::Polynomial::div_rem(&divisor, order) -> (quotient, remainder)` | additive method used by the matrix Bareiss step; the rational module keeps a private sparse mirror; existing algorithms unchanged |
| `MatrixSolveResult` (matrix/conditions/note subset) | `MatrixInverseResult` with `is_solved()` | the inverse record the three rows read |
| `RationalFunctions.Apart(f, x, ctx)` | not ported | no row in this block observes partial fractions today |

## 4. The seven census ids and their Rust tests

All seven are currently `deferred` in `parity/manifest-map.json`; this lane was scoped not to edit
`parity/`. The proposed disposition for every row is `ported`.

| # | Census id | Rust test | What the mirror asserts |
|---:|---|---|---|
| 1 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MetamorphicSetClosureTests/Cancel_ExpandsAnUnexpandedPowerOfASumBeforeReducing#0` | `rational_functions::tests::cancel_expands_an_unexpanded_power_of_a_sum_before_reducing` | `(x-1)^2/(x-1)` and its expanded spelling both reduce to `x - 1`; `Polynomial::from_expr` still refuses a power of a sum and accepts a power of a symbol |
| 2 | `…/Cancel_PoleExclusionIsCarriedAsANonZeroConditionByTheRewritePath#0` | `rational_functions::tests::cancel_conditions_are_the_same_atoms_as_non_zero_condition`, together with the existing `lovelace-lang::suite_parity_batch::dxsemanticclosuretests_transform_xoverx_carriesnonzerocondition` and `…language_simplify_issafebydefault` | the structured cancel builds exactly `x != 0` for `x/x` (symbol-property atom) and `x - 1 != 0` for `(x-1)^2/(x-1)` (expression-property atom), the same atoms the rewrite path carries; the existing language tests cover the rewrite trace/rule id and the safe-mode refusal/licensing |
| 3 | `…/Cancel_PreservesValueOffPoles_AndThePoleIsExcluded#0` | `rational_functions::tests::cancel_preserves_value_off_poles_and_the_pole_is_excluded` | the ten-case corpus reduces to the documented expressions; at eight rational sample points the original is defined and the values agree exactly; at every pole the original is undefined and the reduced form is defined iff the pole was removed |
| 4 | `…/CancelWithConditions_BuildsTheSameConditionAtomsAsTheRewritePath#0` | `rational_functions::tests::cancel_with_conditions_reports_the_removed_factor_as_a_non_zero_atom` | `x/x` is `Conditional`, `Changed`, expression `1`, one `SymbolProperty(x, NonZero)`; `(x-1)^2/(x-1)` carries one `ExpressionProperty(x - 1, NonZero)`; the exact case is `Exact`, empty, unchanged, and the bare form agrees |
| 5 | `…/Inverse_TimesOriginal_IsIdentitySymbolically_UnderDetNonZeroCondition#0` | `symbolic_matrix::tests::inverse_times_original_is_identity_symbolically_under_det_non_zero_condition` | the five-matrix corpus (2x2 general, 2x2 `[[x,1],[y,x]]`, 3x3 tridiagonal, 3x3 cyclic, 3x3 Vandermonde): one `ExpressionProperty(det, NonZero)` atom equal to `a.det()`, det polynomial-equal to the independent ground truth, and the cleared-denominator identity proven exactly |
| 6 | `…/InverseIdentity_HoldsOnlyWhereDetIsNonZero#0` | `symbolic_matrix::tests::inverse_identity_holds_only_where_det_is_non_zero` | at `(x,y) = (2,4)` (det = 0) the inverse entries are undefined; at `(2,3)` (det = 1) the product entries evaluate exactly to `1, 0, 0, 1` |
| 7 | `…/InverseIdentityCheck_RejectsInversesThatPassTheNumericSpotCheck#0` | `symbolic_matrix::tests::inverse_identity_check_rejects_inverses_that_pass_the_numeric_spot_check` | the clean inverse passes; the transposed adjugate, a sign-flipped cofactor and a `+1` numerator perturbation are all rejected with a non-empty detail; a perturbation that vanishes at `(x,y) = (3,2)` passes the numeric spot check yet is rejected symbolically |

Supporting tests added alongside the seven mirrors:

| Module | Tests |
|---|---|
| `rational_functions` (13) | the four returned-pair/refusal tests, the ten-case reduction corpus, the irreducible/non-rational refusals, the two condition-atom tests, the exact-status test, the expansion test, the off-pole/pole test, the per-additive-term test, and the `0/x` pole test |
| `symbolic_matrix` (5) | the five-matrix identity test, the corruption-rejection test, the det-zero/off-zero test, the singular-matrix note test, and the non-square determinant refusal |

## 5. Gate outputs

Commands ran with the repository toolchain environment (`RUSTUP_HOME`, `CARGO_HOME`, `PATH`,
`TMP`, `TEMP`); Node gates ran from the repository root. The release runner was rebuilt before the
fixture and CLI gates.

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Cancel module | `cargo test -p lovelace-symbolics rational_functions` | `13 passed; 0 failed; 115 filtered out` | 0 |
| Matrix module | `cargo test -p lovelace-symbolics symbolic_matrix` | `5 passed; 0 failed; 123 filtered out` | 0 |
| Workspace suite | `cargo test --workspace -- --quiet` | 19 suites, `passed=5029`, `failed=0` (baseline 5,011 + 18 new) | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean, no warnings | 0 |
| Release runner | `cargo build --release -p lovelace-run` | Finished release profile (optimized) | 0 |
| Documentation corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS` | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS` | 0 |

The workspace total includes the 18 new tests; no existing test expectation was edited and no
existing production path changed.

## 6. What remains

* **Manifest mapping.** The seven rows are still `deferred` because this lane cannot edit
  `parity/`. A follow-up lane with parity scope should mark exactly the seven ids `ported` to the
  tests in section 4, regenerate `parity/manifest.toml`, `parity/manifest-map.json`,
  `parity/coverage.md` and `parity/rust-tests.json`, and run the manifest gate. The Rust inventory
  is now stale relative to the tree (18 new tests) until that regeneration runs. Row 2 is a
  composite: the closest single manifest target is the existing
  `lovelace-lang::suite_parity_batch::dxsemanticclosuretests_transform_xoverx_carriesnonzerocondition`
  (rewrite trace/rule/classification); the new
  `rational_functions::tests::cancel_conditions_are_the_same_atoms_as_non_zero_condition` supplies
  the API atom half and should be named in the note.
* **The `cancel` builtin row is separate.** `MetamorphicSetClosureTests/CancelBuiltin_ReducesAndRefusesToCancelWhenNoFactorIsCommon#0`
  is the measured `cancel` family row (`cancel((x^2+1)/(x-1))` returns `x + 1` in the existing
  language-layer univariate path). This lane added a correct library API and deliberately did not
  change the existing builtin, so that row still needs a language-layer fix/rewiring round.
* **Other matrix rows.** `SymbolicMatrix.SolveWithConditions`, `Rank`, the singular-inverse row
  and the other matrix-adjacent census rows remain outside this minimal inverse surface.
* **`Apart` is not ported.** `RationalFunctions.Apart`/partial fractions is not observed by any of
  the seven rows and is listed in the member map as not ported.
* **No runtime wiring.** The language engine does not consume the new modules yet; they are an
  additive library surface and their tests are the only callers. The corpus, fixtures and CLI are
  unchanged and green.
