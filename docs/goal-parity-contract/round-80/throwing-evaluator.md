# Round 80 — the throwing-evaluator block

**Status: complete.** The four deferred `Lovelace.Symbolics.Tests.ExpectedFailureBoundaryTests` rows in
the `Evaluation.EvaluateToNum/EvaluateToExpr with a throwing evaluator` block are reproduced by a new
additive library API. The focused tests pass 10/10, the workspace suite is green, and the corpus,
fixture, CLI, differential, clippy and lane-format gates are green (section 6). The manifest-map
flips are out of this lane's scope; the four census ids and their Rust test ids are listed in
section 4 for the mapping lane.

Measured 2026-11-12 (America/Sao_Paulo) on the working tree. Scope: the additive production
surface in `rust/lovelace-lang/src/evaluation_boundary.rs`, its test module
`rust/lovelace-lang/src/round80_throwing_evaluator.rs`, two declarations in
`rust/lovelace-lang/src/lib.rs`, and visibility-only edits in
`rust/lovelace-lang/src/numeric.rs` and `rust/lovelace-lang/src/builtins.rs`. No `.cs` file,
fixture, corpus, `parity/` file, `rust/lovelace-desktop` file or other document was edited.

## 1. The four rows and the reference failure modes

The four rows are the whole of `ExpectedFailureBoundaryTests` that is still deferred; the class's
other public row, `Limit_TwoSidedPole_StillReportsOpposingSides#0`, was ported in round 34 and is
used here as a negative control through the new entry point.

| # | Census row | C# call | Where the reference throws | Kind | Message |
|---:|---|---|---|---|---|
| 1 | `.../EvaluateToExpr_ExactArgument_ThrowingEvaluatorPropagates#0` | `Exprs.Function(boom, Integer(2))` then `Evaluation.EvaluateToExpr` (`ExpectedFailureBoundaryTests.cs:35-43`) | The registered numeric evaluator (`Evaluation.cs:694-718`, `:705-714`; fold at `:729-800`). The evaluator is the one `ContextWithThrowingFunction` registers at `ExpectedFailureBoundaryTests.cs:22-32`. | `InvalidOperationException` | `boom: deliberately unexpected failure` (`ExpectedFailureBoundaryTests.cs:17`) |
| 2 | `.../EvaluateToExpr_ApproximateArgument_ThrowingEvaluatorPropagates#0` | `Exprs.Function(boom, RealLiteral.FromRealExact(Real.Parse("0."+25 ones)))` then `EvaluateToExpr` (`ExpectedFailureBoundaryTests.cs:45-60`) | The approximate-argument fold (`Evaluation.cs:782-799`, the narrow catch at `:792-797`) and then the direct evaluator arm (`:705-714`); both catch only `EvaluationException`. | `InvalidOperationException` | `boom: deliberately unexpected failure` |
| 3 | `.../Limit_ThrowingEvaluatorPropagates#0` | `Limits.Limit(boom(x), x, 0, TwoSided)` (`ExpectedFailureBoundaryTests.cs:62-72`) | The direct-substitution step `LimitCore` (`Limits.cs:132-143`); its `catch (EvaluationException)` at `:145-149` does not catch the defect. | `InvalidOperationException` | `boom: deliberately unexpected failure` |
| 4 | `.../TryCompare_ReportsUnorderedInsteadOfThrowing#0` | `NumOps.TryCompare(2, 1+1i)` and `NumOps.Compare(2, 1+1i)` (`ExpectedFailureBoundaryTests.cs:93-108`) | `TryCompare` checks the complex tier first and reports `false` with comparison `0` (`Evaluation.cs:158-167`); `Compare` calls `ToReal`, which refuses a complex value (`:45-55`, `:148-153`). | `TryCompare`: none; `Compare`: `InvalidOperationException` | `TryCompare`: no exception, `unordered == 0`; `Compare`: `Complex value cannot convert to Real (use re()/im()).` |

The block's name also covers `EvaluateToNum`. The distinction the reference tests pin is the
**kind boundary**, not only the failure: `EvaluationException` is the kernel's typed "not defined
here" refusal, and the fold, limit and condition paths have narrowed catches for exactly that
kind. Any other exception is a defect. The Rust failure model mirrors it with
`EvaluationError::Evaluation` and `EvaluationError::InvalidOperation`; the kind is exposed as the
reference's type name by `EvaluationError::kind()`.

The domain-kind vocabulary this boundary separates, as read from the same source:

| Condition | Reference throw site | Kind | Message |
|---|---|---|---|
| Division by zero | `NumOps.Divide`, `Evaluation.cs:113-119` | `EvaluationException` | `Division by zero during numeric evaluation.` |
| `log(0)` | `NumOps.Ln`, `Evaluation.cs:311-333` (wraps the numeric `ArgumentException`) | `EvaluationException` | `Ln(x) requires x > 0. (Parameter 'x')` |
| `0` to a non-positive integer power | `EvaluateToNum` Power arm, `Evaluation.cs:467-474` | `EvaluationException` | `0 raised to a non-positive power.` |
| Missing symbol binding | `EvaluateToNum`, `Evaluation.cs:443-452` | `EvaluationException` | `No value for symbol '<name>'.` |
| Function with no numeric evaluator | `EvaluateToNum`, `Evaluation.cs:475-486` | `EvaluationException` | `Function '<name>' has no numeric evaluator.` |
| Infinity leaf | `EvaluateToNum`, `Evaluation.cs:436-442` | `EvaluationException` | `Infinity has no numeric value.` |
| Undecidable piecewise guard | `EvaluateToNum`, `Evaluation.cs:487-499` | `EvaluationException` | `Piecewise guard cannot be decided numerically.` |
| Undecidable logical condition | `EvaluateToNum`, `Evaluation.cs:500-506` | `EvaluationException` | `Logical condition cannot be decided numerically.` |
| Relation node | `EvaluateToNum` default arm, `Evaluation.cs:511-513` | `EvaluationException` | `Node of kind Relation cannot be evaluated numerically.` |
| Complex value on the ordered path | `NumOps.ToReal`, `Evaluation.cs:45-55` | `InvalidOperationException` | `Complex value cannot convert to Real (use re()/im()).` |

## 2. Home, and why

The production surface is **`rust/lovelace-lang/src/evaluation_boundary.rs`**.

* `crate::numeric` is already the port of `Evaluation.EvaluateToNum` and `Num` is the port of the
  reference's numeric tier; `crate::builtins::limit_result` is the only limit engine in the
  workspace. `lovelace-symbolics` has only polynomial `eval_at` and `evalf_rootof` — no general
  numeric evaluator and no limit engine — so it is not the home for this contract.
* The new module is additive rather than a refactor of `numeric::evaluate`. Threading a throwing
  function slot through `evaluate_with` would change the hot path's signature and error shape for
  every `evalf`/limit/integration caller; the new walk keeps `numeric::evaluate` byte-for-byte
  identical for successful evaluation. The duplicate walk is the price of preserving the existing
  error contract, and the tests pin both halves.
* `Num::is_zero` and `numeric::evaluate_function` gained visibility only; `Side`,
  `limit_result` and `limit_value_record` in `builtins.rs` gained `pub(crate)` visibility only.
  No existing function's body or signature changed in those two files.

## 3. Additive member map

| C# member / mechanism | Rust member | Location |
|---|---|---|
| `EvaluationException` | `EvaluationException` / `EvaluationError::Evaluation` (`kind() == "EvaluationException"`) | `evaluation_boundary.rs:37,102` |
| `InvalidOperationException` at the boundary | `InvalidOperationException` / `EvaluationError::InvalidOperation` | `evaluation_boundary.rs:69,104` |
| `FunctionDefinition.NumericEvaluator` | `NumericEvaluator` trait (blanket impl for `Fn(&str, &[Num], usize)`) | `evaluation_boundary.rs:162` |
| `CoreFunctions.Register` table | `builtin_numeric_function` (the built-in elementary table, same `evaluate_function`) | `evaluation_boundary.rs:192` |
| `Evaluation.EvaluateToExpr` | `evaluate_to_expr(expr, places, bindings, functions)` | `evaluation_boundary.rs:285` |
| `Evaluation.EvaluateToNum` | `evaluate_to_num(expr, places, bindings, functions)` | `evaluation_boundary.rs:386` |
| `Evaluation.EvaluateCondition` | private `evaluate_condition` over an empty assumption scope | `evaluation_boundary.rs:518` |
| `NumOps.Compare` | `compare(&left, &right, places) -> Result<Ordering, EvaluationError>` | `evaluation_boundary.rs:219` |
| `NumOps.TryCompare` | `try_compare(...) -> Option<Ordering>` (`None` is the reference's `false` + `0`) | `evaluation_boundary.rs:239` |
| `Limits.Limit` | `limit(expr, var, point, LimitDirection, functions) -> Result<Value, EvaluationError>` | `evaluation_boundary.rs:617` |
| `LimitDirection` | `LimitDirection { TwoSided, FromLeft, FromRight }` | `evaluation_boundary.rs:598` |
| `NumOps.IsZero` | `Num::is_zero` (visibility only) | `numeric.rs:358` |
| `LimitCore`'s series fallback / `LimitResult.Of` | `builtins::limit_result` / `builtins::limit_value_record` (visibility only) | `builtins.rs:7027,7246` |

Semantics of the additive path:

* `evaluate_to_expr` folds a function application only when every argument is a numeric constant,
  exactly as `EvaluateToExpr`. A typed `Evaluation` from the evaluator leaves the node symbolic
  (the reference's narrowed catch); an `InvalidOperation` escapes.
* `evaluate_to_num` evaluates the whole tree, including the piecewise, And/Or/Not, named-constant
  and relation arms. It calls the host evaluator first, then the built-in table when the host
  returns `None`. Both kinds propagate, as in the reference's `EvaluateToNum`.
* `limit` runs the reference's direct-substitution guard with the host evaluator. A typed
  `Evaluation` falls through to the existing symbolic engine (a pole or branch point is not a
  defect); an `InvalidOperation` escapes. An EXACT substitution value is the limit (the
  reference's `LimitResult.Of`); an approximate or complex value falls through to the engine, so no
  existing successful answer changes. `±inf` points run the same probe on the `x = 1/t` transform
  the reference's `AtInfinity` uses, then delegate to the existing engine.
* `compare` refuses a complex operand with the reference's exact kind and message. `try_compare`
  reports that pair as `None` before reaching the refusal, so the ordered path keeps the defect.

## 4. Tests and the census ids they unblock

`rust/lovelace-lang/src/round80_throwing_evaluator.rs` contains 10 tests. The four mapped rows:

| Census id | Rust test id | What it asserts |
|---|---|---|
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ExpectedFailureBoundaryTests/EvaluateToExpr_ExactArgument_ThrowingEvaluatorPropagates#0` | `lovelace-lang::round80_throwing_evaluator::evaluate_to_expr_exact_argument_throwing_evaluator_propagates` | `evaluate_to_expr` on `boom(2)` returns the `InvalidOperation` kind with `boom: deliberately unexpected failure`. |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ExpectedFailureBoundaryTests/EvaluateToExpr_ApproximateArgument_ThrowingEvaluatorPropagates#0` | `lovelace-lang::round80_throwing_evaluator::evaluate_to_expr_approximate_argument_throwing_evaluator_propagates` | the same expression with a 25-digit Real argument propagates the same kind and message. |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ExpectedFailureBoundaryTests/Limit_ThrowingEvaluatorPropagates#0` | `lovelace-lang::round80_throwing_evaluator::limit_throwing_evaluator_propagates` | `limit(boom(x), x, 0, TwoSided)` propagates the same kind and message from the substitution step. |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ExpectedFailureBoundaryTests/TryCompare_ReportsUnorderedInsteadOfThrowing#0` | `lovelace-lang::round80_throwing_evaluator::try_compare_reports_unordered_instead_of_throwing` | `try_compare(2, 3)` orders, `try_compare(2, 1+1i)` is `None`, and `compare(2, 1+1i)` is the `InvalidOperation` kind with the exact complex-to-real message. |

The six supporting tests, none of which is a separate census row:

* `the_same_expression_succeeds_when_the_evaluator_does_not_throw` — the negative control: the
  same `boom(2)` with an answering evaluator folds to `7`, and the built-in table still folds
  `abs(-3)` to `3`.
* `limit_of_the_same_expression_succeeds_when_the_evaluator_answers` — the same `boom(x)` at 0
  answers `7` under the answering evaluator.
* `limit_of_a_two_sided_pole_still_reports_opposing_sides` — the class's other public control:
  `limit(1/x, x, 0)` is still `DoesNotExist` through the new entry point.
* `the_fold_absorbs_the_domain_kind_and_leaves_the_node_symbolic` — a typed `Evaluation` from the
  evaluator leaves `boom(2)` symbolic, the reference's narrowed catch.
* `evaluate_to_num_propagates_both_kinds` — the `EvaluateToNum` half of the block name keeps both
  kinds and their messages.
* `evaluate_to_num_reports_the_typed_domain_vocabulary` — division by zero, `log(0)` and `0^-1`
  report `EvaluationException` with the reference's exact messages (the table in section 1).

## 5. Focused verification

```text
cargo test -p lovelace-lang round80_throwing_evaluator
running 10 tests
...
test result: ok. 10 passed; 0 failed; 0 ignored; 1588 filtered out; finished in 0.00s
```

## 6. Gates

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Focused | `cargo test -p lovelace-lang round80_throwing_evaluator` (rust/) | 10 passed, 0 failed, 1588 filtered out | 0 |
| Workspace | `cargo test --workspace` (rust/) | 5,261 passed; 0 failed; 1 ignored over 21 result blocks | 0 |
| Corpus | `cargo run --release -p lovelace-corpus` (rust/) | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Fixtures | `node parity/tools/gate-fixtures.mjs` (repo root) | `oracle: 20/20`, `rust: 20/20`, `PASS` | 0 |
| CLI | `node parity/tools/gate-cli.mjs` (repo root) | `CLI matrix: 16/16`, `PASS` | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1` (repo root) | generated `200 match / 0 mismatch / 0 error`; regressions `1758 match / 0 mismatch / 0 error`; `harness exit: 0`; `parity/diff/runs/20261112T135801Z-seed1` | 0 |
| Clippy (crate) | `cargo clippy -p lovelace-lang --all-targets -- -D warnings` (rust/) | no diagnostics | 0 |
| Clippy (workspace minus desktop) | `cargo clippy --workspace --all-targets --exclude lovelace-desktop -- -D warnings` (rust/) | no diagnostics | 0 |
| Format (lane files) | `rustfmt --edition 2024 --check src/evaluation_boundary.rs src/round80_throwing_evaluator.rs` (rust/lovelace-lang/) | no diff | 0 |
| Format (crate) | `cargo fmt -p lovelace-lang -- --check` (rust/) | no diff | 0 |
| Format (workspace) | `cargo fmt --all -- --check` (rust/) | exit 1; diffs only in `lovelace-desktop/src/engine.rs` and `execution.rs` (another lane's files); no `lovelace-lang` file appears | 1 (other lane) |

The workspace total is the task baseline (5,247 passed / 1 ignored) plus this lane's 10 tests and
four concurrent abs-rewrite tests: 5,261 passed / 1 ignored. The one ignored test is the
pre-existing `lovelace_corpus::property_assertions::mathir_evaluates_like_symbolic` row.

## 7. Scope, shared files and residual

Changed by this round:

* `rust/lovelace-lang/src/evaluation_boundary.rs` — new production module.
* `rust/lovelace-lang/src/round80_throwing_evaluator.rs` — new, 10 tests.
* `rust/lovelace-lang/src/lib.rs` — `pub mod evaluation_boundary;` and one `#[cfg(test)] mod
  round80_throwing_evaluator;` declaration. The concurrent abs-rewrite lane's
  `round80_abs_rewrite` declaration is preserved.
* `rust/lovelace-lang/src/numeric.rs` — `Num::is_zero` is public and `evaluate_function` is
  `pub(crate)`; no body changed.
* `rust/lovelace-lang/src/builtins.rs` — `Side`, `limit_result` and `limit_value_record` are
  `pub(crate)`; no body changed. The abs-rewrite lane's `simplify_traced` edit is preserved.
* `docs/goal-parity-contract/round-80/throwing-evaluator.md` — this report.

Not touched: any `.cs` file, `fixtures`, corpus, `parity/`, `rust/lovelace-desktop`, or any other
document. The manifest-map flips for the four ids are left to the mapping lane.

Residual, stated rather than hidden:

* The new API is empty-scope: the reference's `EvaluateCondition` consults the ambient
  `ExprContext.Assumptions`, and this entry point has no scope argument (the four rows install
  none). The scoped entry point from round 78 (`numeric::evaluate_scoped`) remains the assumption
  path.
* `evaluate_to_expr` consults the host evaluator before the built-in exact-fold table, so a host
  that claims a built-in name (for example `sin` at 0) overrides that fold. Returning `None` defers
  to the table; the reference's registry cannot shadow a built-in because duplicate registration
  throws.
* `limit` publishes only an EXACT direct-substitution value; an approximate or complex value is
  delegated to the existing series engine so the engine's existing exactness answer is unchanged.
* The Rust `Expr::Fun("piecewise")`/logical nodes are the reference's dedicated node kinds; the
  new EvaluateToExpr leaves them opaque like the reference, and EvaluateToNum implements their
  arms. `Expr::Div` (a Rust-only explicit quotient node) is traversed although the reference has no
  such node; this only widens substitution inside a tree shape the reference spells as a product.
