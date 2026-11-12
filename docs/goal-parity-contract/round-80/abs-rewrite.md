# Round 80 — abs-rewrite family (4 rows)

**Status: complete.** All four deferred `abs-rewrite` rows now produce a full-envelope match
against the pinned oracle after normalising `revision`, `elapsed`, `elapsedTime` and
`timings`. Two production behaviours were wrong, not missing APIs: the traced `simplify_full`
pipeline never applied the real-domain `pow.sqrt-square-real` rewrite, and `substitute` rebuilt
function applications without the reference constructor's exact constant fold. Both were fixed in
`rust/lovelace-lang/src` and `rust/lovelace-symbolics/src`; no test expectation was changed or
weakened.

Measured 2026-11-12 (America/Sao_Paulo) against the pinned oracle
`1cc28f255eca6029849f880949fc7be1d40ab8af` and `rust/target/release/lovelace-run.exe`, at repo
root, via `--file <script> --omit-functions` on both runners.

## 1. The four rows, before and after

| # | Census id | Probe | Field | Before (Rust) | After (Rust) = oracle |
|---:|---|---|---|---|---|
| 1 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.AbsSymbolicRoundTripTests/Abs_Of_Symbol_Matches_The_Rewriters_Own_Abs_Node#0` | `x = symbol("x", real)`; `abs(x)`; `simplify_full(sqrt(x^2)).expression` | `$.result.display` / canonical of the third statement, and structural equality | `abs(x)` itself already matched; `simplify_full(sqrt(x^2)).expression` was `sqrt(x^2)`, canonical `(pow (pow (sym x) (rat 2 1)) (rat 1 2))`, so it was neither canonically nor structurally equal to the rewriter's `(fn abs (sym x))` | `simplify_full(sqrt(x^2)).expression` = `abs(x)`, canonical `(fn abs (sym x))`; canonical equality and structural `Expr::Fun("abs", [x])` equality both hold |
| 2 | `.../AbsSymbolicRoundTripTests/Simplify_Sqrt_Square_Under_Real_Is_Still_Abs_X#0` | `x = symbol("x", real)`; `simplify_full(sqrt(x^2))` | `$.result` (expression, changed, conditions, steps) | `TransformResult(status: Satisfied, original: sqrt(x^2), expression: sqrt(x^2), changed: False, conditions: [], steps: [])` | `TransformResult(status: Satisfied, original: sqrt(x^2), expression: abs(x), changed: True, conditions: [DomainCondition(variable: x, domain: real)], steps: [RewriteStep(rule_id: pow.sqrt-square-real, classification: DomainSpecific, before: sqrt(x^2), after: abs(x), required_conditions: [DomainCondition(variable: x, domain: real)])])` |
| 3 | `.../AbsSymbolicRoundTripTests/Subs_Abs_Symbolic_At_Negative_Value_Gives_Three#0` | `x = symbol("x")`; `subs(abs(x), x, -3)` | `$.result.display` / canonical / freeSymbols | `abs(-3)`, canonical `(fn abs (rat -3 1))`, domain real, freeSymbols `[]` | `3`, canonical `(rat 3 1)`, domain rational, freeSymbols `[]` |
| 4 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.UnsatisfiablePropagationTests/SimplifyFull_DomainCondition_CarriesADomainValueNotText#0` | `x = symbol("x", real)`; `simplify_full(sqrt(x^2))` | `$.result.conditions[0]` | conditions `[]` (no leaf; the C# body's `conditions[0]` would not exist) | one `DomainCondition` record: `variable` = Symbolic `x` (canonical `(sym x)`), `domain` = Domain `real`; the display carries no `SymbolDomainAssumption` |

After the fixes the four probes are **full-envelope matches** after the differential's normalisation
(not just matching display text): a sorted-key, volatile-field-stripped JSON comparison of the whole
envelope reported `rowA FULL MATCH`, `rowB FULL MATCH`, `rowC FULL MATCH`, `rowD FULL MATCH`.
`abs(x)` on its own, `simplify(sqrt(x^2))` under `x` real, `simplify_full(sqrt(x^2))` when
`x` is positive/nonnegative (the `pow.sqrt-square-nonnegative` path) and the non-real control
were already matching and were not changed (the positive/nonnegative probes still publish
`expression: x, conditions: [x >= 0], rule_id: pow.sqrt-square-nonnegative`).

The neighbouring probes that remain different are **not** these rows and were left alone:
`simplify_full(sqrt((x+1)^2))` reports `changed: True` with the same expression in the oracle
(a general `changed` flag detail of the reference transform) while Rust reports `False`, and
`simplify_full(abs(x)^2)` applies the reference's `abs.abs-square` rule, which no deferred census
row in this family drives. No rule was added for them.

## 2. Production fixes

### 2.1 `pow.sqrt-square-real` in the traced simplifier

`rust/lovelace-lang/src/builtins.rs:11436-11466` (inside `simplify_traced`, the production path
`simplify_full` calls). The safe `simplify` already rewrote `sqrt(x^2) -> abs(x)` under a real
assumption, and the traced path already had `pow.sqrt-square-nonnegative`; it had no real-domain
branch. The new branch fires only when:

* the node is `sqrt(w^2)` (`w` = a single symbol, matching the shipped registry's applicability
  predicate),
* `w` is **not** known nonnegative (otherwise the nonnegative rule must win, as the oracle does),
* `w` is provably real in the session's assumptions.

It produces `Expr::Fun("abs", [w])`, records the `RewriteStep` with rule id
`pow.sqrt-square-real`, classification `DomainSpecific`, and the same `DomainCondition` leaf the
reference publishes (reusing `assumption_condition_leaf`). The condition is lifted into the
result's `conditions` by the existing `step_conditions`, so the structured payload, the display
and the trace are produced by the existing machinery, not a second policy. Firing the rule without
the real premise would define `sqrt((-1)^2)` as `-1`, so the guard is the contract.

### 2.2 Exact function folding on substitution

`rust/lovelace-symbolics/src/lib.rs:2255-2325` (new `fold_exact_function`/`rat_integer`) and
`lib.rs:2353-2359` (`substitute`'s `Fun` arm). The reference's `Exprs.Function` constructor
constant-folds a total function of exact rational arguments, so
`Evaluation.Substitute`'s `Exprs.Function(abs, Num(-3))` is the exact rational `3`. Rust's
`substitute` rebuilt `Expr::Fun` verbatim, so `subs(abs(x), x, -3)` answered `abs(-3)`.
Substitution now rebuilds through the same exact fold (the reference's table: `abs`, `sign`,
`floor`, `ceil`, `min`/`max` and the zero/one identities for `exp`, `sin`, `cos`, `tan`,
`log`, `atan`, `asin`, `acos`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`).
Only exact `Expr::Num` arguments fold; approximate `Real` leaves do not, matching the constructor's
`allExact` gate. This is the reference construction contract, not a special case for this row.

## 3. Tests added (one per row)

New module `rust/lovelace-lang/src/round80_abs_rewrite.rs` (declared at
`rust/lovelace-lang/src/lib.rs:2430-2436`). Each test drives the row's own script through the
language surface (`Engine::eval`) and asserts the oracle-observed values above.

| Row | Test | What it pins |
|---:|---|---|
| 1 | `round80_abs_rewrite::abs_of_symbol_matches_the_rewriters_own_abs_node` (`round80_abs_rewrite.rs:53`) | `abs(x)` display/canonical; `simplify_full(sqrt(x^2)).expression` canonical equality and structural equality with `Expr::Fun("abs", [x])` |
| 2 | `round80_abs_rewrite::simplify_sqrt_square_under_real_is_still_abs_x` (`:76`) | `type(...)` = `TransformResult`; expression `abs(x)`; `changed` True; `conditions` `[DomainCondition(variable: x, domain: real)]`; exactly one step with rule id `pow.sqrt-square-real` / classification `DomainSpecific` |
| 3 | `round80_abs_rewrite::subs_abs_symbolic_at_negative_value_gives_three` (`:113`) | display `3`, canonical `(rat 3 1)`, no free symbols |
| 4 | `round80_abs_rewrite::simplify_full_domain_condition_carries_a_domain_value_not_text` (`:128`) | condition is a `DomainCondition` record with fields `variable` (Symbolic) and `domain` (Domain `real`); no raw `SymbolDomainAssumption` text |

The four census ids are listed in section 1; they are the rows the porting round can now flip from
`deferred` to `ported` in `parity/manifest-map.json`. That map was **not** edited here (out of
this lane's scope, per the round-74/79 split).

## 4. Gates

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Focused | `cargo test -p lovelace-lang round80_abs_rewrite` | **4 passed; 0 failed; 0 ignored** (1,594 filtered) | 0 |
| Workspace | `cargo test --workspace` | **5,261 passed; 0 failed; 1 ignored** (final shared-tree run: 5,247 baseline + this lane's 4 + 10 concurrent evaluator-lane tests; an earlier run before those landed was 5,251 passed / 1 ignored; every result block ok) | 0 |
| Corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS` | 0 |
| CLI | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS` | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1` | generated **200 match / 0 mismatch / 0 error**; replayed **1,758 match / 0 mismatch / 0 error**; `mismatches` file empty; `firstDifferenceHistogram: []`; `harness exit: 0` | 0 |
| Clippy | `cargo clippy --workspace --all-targets --exclude lovelace-desktop -- -D warnings` | `Finished`, no warnings | 0 |
| Format | `cargo fmt -p lovelace-lang -p lovelace-symbolics -- --check` | clean after the sweep noted in `5` | 0 |

The run directory is `parity/diff/runs/20261112T134859Z-seed1`; its `monitor.json` records
`generated {total: 200, match: 200, mismatch: 0, error: 0}` and
`regressions {total: 1758, match: 1758, mismatch: 0, error: 0}`.

## 5. Scope, shared files and boundaries

Changed by this round:

* `rust/lovelace-lang/src/builtins.rs` — **shared file**: the evaluator lane already had working
  tree changes in this file (the `evaluation_boundary` limit wrapper: `Side`, `limit_result`
  and `limit_value_record` made `pub(crate)`). This lane's only edit is the added
  `pow.sqrt-square-real` block in `simplify_traced` (`builtins.rs:11436-11466`); the evaluator
  lane's hunks were not touched.
* `rust/lovelace-lang/src/lib.rs` — **shared file**: one `#[cfg(test)] mod round80_abs_rewrite;`
  declaration plus doc comment. The evaluator lane's `pub mod evaluation_boundary;` and
  `mod round80_throwing_evaluator;` declarations remain.
* `rust/lovelace-symbolics/src/lib.rs` — `fold_exact_function`/`rat_integer` and the
  `substitute` `Fun` arm.
* `rust/lovelace-symbolics/src/cancel.rs` — **formatting only** (two hunks: import order and one
  `assert!` line wrap). It is committed and was the only pre-existing file in the edited packages
  that the current rustfmt rejected, so it had to be normalised for the package format gate; no
  token or logic changed. Disclosed rather than hidden.
* `rust/lovelace-lang/src/round80_abs_rewrite.rs` — new (4 tests).
* `docs/goal-parity-contract/round-80/abs-rewrite.md` — this report.

No `.cs` file, fixture, corpus file, `parity/` file, `rust/lovelace-desktop` file or other
document was edited. The workspace `cargo fmt --all -- --check` still reports pre-existing diffs in
`rust/lovelace-desktop/src/engine.rs` and `execution.rs`; that package is explicitly out of this
lane's scope, so the format gate is reported for the two edited packages.

## 6. Reproduction

```powershell
$repo = 'C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME = "$repo\.toolchain\rustup"; $env:CARGO_HOME = "$repo\.toolchain\cargo"
$env:PATH = "$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP = "$repo\rust\target\tmp"; $env:TEMP = $env:TMP
Set-Location "$repo\rust"

# Rows 1 / 2 / 4 probe
'x = symbol("x", real)', 'simplify_full(sqrt(x^2))' | Set-Content row.ls
# Row 3 probe
'x = symbol("x")', 'subs(abs(x), x, -3)' | Set-Content subs.ls

dotnet exec "$repo\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll" --file row.ls --omit-functions
"$repo\rust\target\release\lovelace-run.exe" --file row.ls --omit-functions
dotnet exec "$repo\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll" --file subs.ls --omit-functions
"$repo\rust\target\release\lovelace-run.exe" --file subs.ls --omit-functions

cargo test -p lovelace-lang round80_abs_rewrite
cargo test --workspace
cargo run --release -p lovelace-corpus
Set-Location $repo
node parity/tools/gate-fixtures.mjs
node parity/tools/gate-cli.mjs
node parity/diff/run.mjs --cases 200 --seed 1
```
