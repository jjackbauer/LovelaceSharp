# Round 78 — assumption-scoped numeric evaluation

**Status: complete.** The focused scoped-eval tests, the full workspace suite, the library
build, corpus, fixtures, CLI, differential and library clippy gates are green (section 6). The
workspace suite was measured after the concurrent equality and kernel-cancel lanes had landed
their fixes; an earlier attempt failed in a desktop cancellation row that now passes.

Measured 2026-11-12 (America/Sao_Paulo) on the working tree. Scope: the additive production
surface in `rust/lovelace-lang/src/numeric.rs`, its test module
`rust/lovelace-lang/src/round78_scoped_eval.rs`, one `#[cfg(test)] mod` line in
`rust/lovelace-lang/src/lib.rs`, and this report. No `.cs` file, fixture, corpus, `parity/`
file or other document was edited.

## 1. The C# contract observed from the source and the four rows

The four rows are the `assumption-scoped numeric evaluation (EvaluateToNum with caller
assumptions)` block. Their C# source is `Lovelace.Symbolics.Tests/PiecewiseAssumptionTests.cs`.

| Census id | C# body | Caller scope | Guard | Observable |
|---|---|---|---|---|
| `.../MirroredSpelling_IsAlsoDecided#0` | lines 55-70 | `[x > 0]` | `0 < x` (constant-first) | `EvaluateToNum(piecewise(1 if guard, 2))` equals `1` |
| `.../ProvenGuard_TakesTheBranch#0` | lines 40-53 | `[x > 0]` | `x >= 0` | equals `1` |
| `.../RefutedGuard_TakesTheOtherwiseBranch#0` | lines 25-38 | `[x < 0]` | `x >= 0` | equals `2` |
| `.../WithoutAssumptions_TheGuardIsStillUndecidable#0` | lines 73-83 | none | `x >= 0` | throws `EvaluationException` |

The scope is installed by `ExprContext.WithAssumptions(AssumptionSet)`
(`Lovelace.Symbolics/Context.cs:125`), a flow-local `AsyncLocal` override that shadows the
context's base set. The evaluated member is
`Evaluation.EvaluateToNum(Expr, ExprContext, IReadOnlyDictionary<Symbol, Num>) -> Num`
(`Lovelace.Symbolics/Evaluation.cs:424`). Its relevant arms:

* `PiecewiseExpr` (`Evaluation.cs:487-499`): take the first branch whose guard is `True`, skip
  `False`, and throw `"Piecewise guard cannot be decided numerically."` for anything else;
  the otherwise value applies when every guard is `False`.
* `EvaluateCondition` (`Evaluation.cs:537-617`): for a bare `RelationExpr`, the assumption
  lattice is asked **before** numeric evaluation (`Evaluation.cs:572-586`,
  `ctx.Assumptions.Ask(atom)`); a decided answer returns immediately, otherwise the numeric
  fallback runs and a numeric failure becomes `Unknown`.
* `AssumptionRelationOf` (`Evaluation.cs:519-531`): the lattice stores symbol-op-constant
  only, so a mirrored spelling such as `0 < x` is flipped into `x > 0` before the query.

**Returned value and conditions.** C# returns the `Num` value only; there is no conditions field
on `EvaluateToNum`. The conditions the answer relied on are the caller's scope atoms that the
`Ask` call used — the caller still holds the scope, so the reference never has to publish them.
The additive Rust entry point below makes that reliance explicit in its return value.

**"No assumptions needed" case.** A relation with no applicable lattice atom falls through to
numeric evaluation; a guard such as `1 >= 0` is decided there, and no assumption participates.
In the four rows only `WithoutAssumptions_TheGuardIsStillUndecidable` is the empty-scope case,
and it is the refusal branch (unknown guard, unbound symbol). The additive API is also tested on
the successful "decided numerically, no assumption used" branch.

## 2. Home, and why

The production surface is in **`rust/lovelace-lang/src/numeric.rs`**.

* `numeric::evaluate` is already the port of `Evaluation.EvaluateToNum` (the module header says
  so), and `numeric::Num` is the port of the `Num` tier; `evaluate_condition` is the port of
  `EvaluateCondition`. The scoped entry point belongs beside them, not in `lovelace-symbolics`,
  which has no general numeric evaluator (only polynomial `eval_at` and `evalf_rootof`).
* The assumption engine is `lovelace_symbolics::assumptions::AssumptionSet`, already a direct
  dependency of `lovelace-lang`; the new code calls its existing `ask` and `atoms` methods and
  changes neither.
* `numeric` is a public module, so hosts can call the entry point without a new crate or a
  language builtin. The engine's ambient scope plumbing (`Engine::assumptions`, the modus
  caller-scope in `Engine::eval`) is untouched.

## 3. Additive member map

| C# member / mechanism | Rust member | Location |
|---|---|---|
| `ExprContext.WithAssumptions(AssumptionSet)` | explicit `scope: &AssumptionSet` argument (no ambient install needed) | `numeric.rs:61` |
| `Evaluation.EvaluateToNum(...) -> Num` | `numeric::evaluate_scoped(expr, places, scope) -> Result<ScopedEvaluation, LangError>` (new) | `numeric.rs:61` |
| — (new result shape) | `numeric::ScopedEvaluation { value: Num, conditions: AssumptionSet }` | `numeric.rs:40` |
| `Evaluation.EvaluateToNum` value-only path | `numeric::evaluate(expr, places) -> Result<Num, LangError>` (signature and behaviour unchanged; delegates with an empty scope) | `numeric.rs:75` |
| `Evaluation.AssumptionRelationOf` | `assumption_relation_of(op, left, right) -> Option<Assumption>` (private; mirrored spelling flipped) | `numeric.rs:282` |
| `ctx.Assumptions.Ask(atom)` in `EvaluateCondition` | `decided_relation(...)` calls `AssumptionSet::ask` (existing engine, unchanged) | `numeric.rs:304` |
| `Evaluation.EvaluateCondition` relation arm | `evaluate_condition_with(...)` (private; consults the scope first, then numeric) | `numeric.rs:190` |
| `PiecewiseExpr` arm | the `piecewise` arm of `evaluate_with` (private) | `numeric.rs:82` |
| — (reliance report) | `minimal_witness(scope, atom, decision)` (private): removes one atom at a time while `ask` still returns the same `True`/`False` | `numeric.rs:323` |

Semantics of the new result:

* `conditions` is the **minimal subset of the caller-supplied scope** that decided a guard, in
  `AssumptionSet`'s canonical order. A guard decided from `[x > 0]` reports `x > 0` even when
  the guard itself was the weaker `x >= 0`; a non-deciding atom (for example an unrelated
  `y > 0`) is not reported.
* A guard decided numerically (for example `1 >= 0`), and an expression with no guards,
  report an **empty** condition set: no assumptions were needed.
* A relation that is not of the lattice's symbol-vs-constant shape falls through to numeric
  evaluation exactly as before.
* An undecided guard refuses with the reference's message
  `"Piecewise guard cannot be decided numerically."`; the symbol with no binding still refuses
  with its own message when it is reached outside a guard.
* An `AssumptionSet::unsatisfiable()` scope cannot license a branch: a piecewise guard carrying
  that answer is refused, matching the C# arm, which accepts only `True`/`False`.
* The private `Tristate` gains the `Unsatisfiable` member (the reference's fourth state), and
  the And/Or/Not and logical-value arms mirror the C# comparisons exactly.

No existing function's signature changed, and `evaluate` now performs the same walk with an
empty scope. The old behaviour is pinned by the pre-existing test suite and by the refusal half
of `without_assumptions_the_guard_is_still_undecidable`.

## 4. Tests and the census ids they unblock

`rust/lovelace-lang/src/round78_scoped_eval.rs` (registered at `lib.rs:2402`) contains:

| Census id (appendix 9, `probe-classification.md:357-360`) | Rust test id | What it asserts |
|---|---|---|
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PiecewiseAssumptionTests/MirroredSpelling_IsAlsoDecided#0` | `lovelace-lang::round78_scoped_eval::mirrored_spelling_is_also_decided` | guard `0 < x` under `[x > 0]` yields `1`; conditions equal `[x > 0]` |
| `.../ProvenGuard_TakesTheBranch#0` | `lovelace-lang::round78_scoped_eval::proven_guard_takes_the_branch` | guard `x >= 0` under `[x > 0]` yields `1`; conditions equal `[x > 0]` |
| `.../RefutedGuard_TakesTheOtherwiseBranch#0` | `lovelace-lang::round78_scoped_eval::refuted_guard_takes_the_otherwise_branch` | guard `x >= 0` under `[x < 0]` yields `2`; conditions equal `[x < 0]` |
| `.../WithoutAssumptions_TheGuardIsStillUndecidable#0` | `lovelace-lang::round78_scoped_eval::without_assumptions_the_guard_is_still_undecidable` | both `evaluate` and `evaluate_scoped(..., empty)` throw the piecewise-guard message |

The fifth test, `a_guard_decided_without_assumptions_reports_none`, is supporting evidence for
the "no assumptions needed" branch: `1 >= 0` under a scope holding an unrelated atom yields the
branch value with empty conditions. It is not mapped to a census row.

The full census ids are the four listed in `docs/goal-parity-contract/round-75/probe-classification.md`
appendix §9 (ranks 47-50, block rank 6).

## 5. Focused verification

```
cargo test -p lovelace-lang round78_scoped_eval -- --nocapture
running 5 tests
test round78_scoped_eval::without_assumptions_the_guard_is_still_undecidable ... ok
test round78_scoped_eval::a_guard_decided_without_assumptions_reports_none ... ok
test round78_scoped_eval::refuted_guard_takes_the_otherwise_branch ... ok
test round78_scoped_eval::proven_guard_takes_the_branch ... ok
test round78_scoped_eval::mirrored_spelling_is_also_decided ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 1562 filtered out
```

`cargo check -p lovelace-lang --tests` and `cargo check -p lovelace-lang --lib` both exit 0;
`cargo clippy -p lovelace-lang --lib -- -D warnings` exits 0.

## 6. Gates

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Focused | `cargo test -p lovelace-lang round78_scoped_eval` (rust/) | 5 passed / 0 failed | 0 |
| Corpus | `cargo run --release -p lovelace-corpus` (rust/) | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Fixtures | `node parity/tools/gate-fixtures.mjs` (repo root) | `oracle: 20/20`, `rust: 20/20`, `PASS` | 0 |
| CLI | `node parity/tools/gate-cli.mjs` (repo root) | `CLI matrix: 16/16`, `PASS` | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1` (repo root) | generated **200 match / 0 mismatch / 0 error**; replayed **1758 match / 0 mismatch / 0 error**; `firstDifferenceHistogram: []`; harness exit 0 | 0 |
| Workspace | `cargo test --workspace` (rust/) | **5,247 passed; 0 failed; 1 ignored** over 21 result blocks (5,219 + 28 from the concurrent lanes and this lane's 5), exit 0 | 0 |
| Clippy (workspace minus desktop) | `cargo clippy --workspace --all-targets --exclude lovelace-desktop -- -D warnings` (rust/) | no diagnostics, `Finished` | 0 |
| Clippy (library) | `cargo clippy -p lovelace-lang --lib -- -D warnings` (rust/) | no diagnostics, `Finished` | 0 |
| Format (files) | `rustfmt --edition 2024 --check src/numeric.rs src/round78_scoped_eval.rs` (rust/lovelace-lang/) | no diff | 0 |
| Format (workspace) | `cargo fmt --all -- --check` (rust/) | fails only in concurrent lanes' files: `lovelace-desktop/src/engine.rs`, `lovelace-desktop/src/execution.rs`, `lovelace-symbolics/src/cancel.rs`; no scoped-eval file appears | external |

## 7. Boundaries and residual

Changed by this round:

* `rust/lovelace-lang/src/numeric.rs` — new `ScopedEvaluation`/`evaluate_scoped`; the existing
  `evaluate` delegates on an empty scope; private scope threading and the assumption-arm helpers.
* `rust/lovelace-lang/src/round78_scoped_eval.rs` — new, 5 tests.
* `rust/lovelace-lang/src/lib.rs` — one `#[cfg(test)] mod round78_scoped_eval;` line (the
  concurrent equality lane added its own `round78_structural_equality` line beneath it; both
  are present).
* `docs/goal-parity-contract/round-78/scoped-eval.md` — this report.

Not touched: any `.cs` file, `parity/`, `fixtures`, corpus, `rust/lovelace-desktop`, or any
other document. The new entry point is library surface only; no language builtin was added and
`evalf` still calls the value-only `evaluate`.
