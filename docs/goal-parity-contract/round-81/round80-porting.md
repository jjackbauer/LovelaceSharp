# Round 81 — round-80 lane porting map flip

**Status: complete.** The 8 census rows the round-80 abs-rewrite and throwing-evaluator blocks
unblocked are mapped to their row tests, and the regenerated gate resolves 8/8. Coverage moved from
5,971/6,209 (96.17 %) to 5,979/6,209 (96.30 %); deferred 120 -> 112; excluded unchanged at 118.

Measured 2026-11-12 (America/Sao_Paulo) on the working tree. Scope: `parity/manifest-map.json`
plus the generated parity files and this report; no Rust, C#, fixture or other document was edited.

## 1. Pre-regeneration baseline — stale inventory

The checked-in inventory predated the round-80 Rust modules: `parity/rust-tests.json` recorded
`generatedAtUtc 2026-11-12T13:39:41Z`, `suite {exitCode 0, passed 5247, failed 0}`, 11 crates /
5,248 tests, and no `round80_*` test name. The map still deferred all eight rows, so the gate failed
on freshness alone:

```
"dispositions": { "ported": 5971, "consolidated": 0, "excluded": 118, "deferred": 120 },
"coveragePercent": 96.17,
"claimedRustTests": 5971,
"failures": ["rust-tests.json is stale: the Rust sources changed since the inventory was measured (run parity/gen-rust-tests.ps1)"]
FAIL: 1 issue(s)
```

**Pre-regen line: 5,971/6,209 ported (96.17 %), 118 excluded, 120 deferred, gate FAIL (stale
inventory: 5,247 passed / 5,248 listed, 0 `round80_*` names).**

## 2. First regeneration — re-established baseline

Commands: `parity/gen-rust-tests.ps1`, `node parity/gen-manifest.mjs`,
`parity/gate-manifest.ps1`. Observed lines:

```
crates=11 tests=5262 suiteExit=0 passed=5261 failed=0
```

```
{
  "rows": 6209,
  "dispositions": {
    "excluded": 118,
    "ported": 5971,
    "deferred": 120
  },
  "coveragePercent": 96.17,
  "rustTests": 5262
}
```

```
PASS (structural): 6209 test cases dispositioned; 5971 claim Rust coverage (96.17%).
```

The fresh inventory (`generatedAtUtc 2026-11-12T14:06:03Z`, suite exit 0) lists 5,262 tests, 14 more
than the stale 5,248: the 4 abs-rewrite and 10 throwing-evaluator tests. The measured workspace
total is **5,261 passed / 0 failed / 1 ignored** (5,262 listed); the difference is the pre-existing
ignored `lovelace_corpus::property_assertions::mathir_evaluates_like_symbolic`, which
`cargo test --workspace -- --ignored --list` lists as the workspace's only ignored test. All 14
`round80_*` names are in the inventory, and the recorded `sourcesSha256` matches the gate's walk of
the current sources.

After this first regeneration the baseline is truthful but unchanged in coverage:
**5,971/6,209 (96.17 %), 118 excluded, 120 deferred, gate PASS**.

## 3. The eight mappings

Each C# body was read at its census source line, and each mapped Rust test is the row's own test,
not a supporting/neighbour test. The four abs-rewrite tests map one-to-one. The throwing-evaluator
module contains 10 tests of which these four are the census rows; the other six are negative
controls and kind-boundary evidence (`the_same_expression_succeeds...`, `limit_of_the_same...`,
`limit_of_a_two_sided_pole...`, `the_fold_absorbs...`, `evaluate_to_num_propagates_both_kinds`,
`evaluate_to_num_reports_the_typed_domain_vocabulary`) and are not mapped. The class's other
public row, `Limit_TwoSidedPole_StillReportsOpposingSides#0`, was ported in round 34 and was not
touched.

| Census id | C# row assertion | Rust test | Why this is the row's test |
|---|---|---|---|
| `Lovelace.Symbolics.Tests/.../AbsSymbolicRoundTripTests/Abs_Of_Symbol_Matches_The_Rewriters_Own_Abs_Node#0` | `AbsSymbolicRoundTripTests.cs:24`: `abs(x)` prints `abs(x)`; `simplify_full(sqrt(x^2)).expression` is canonically equal to the applied node and the applied node equals `Exprs.Function(abs, Symbol x)` | `lovelace-lang::round80_abs_rewrite::abs_of_symbol_matches_the_rewriters_own_abs_node` (`round80_abs_rewrite.rs:53`) | Same script under `x` real; asserts the same display, canonical `(fn abs (sym x))`, and structural `Expr::Fun("abs", [x])` — the node the rewrites themselves construct. |
| `.../AbsSymbolicRoundTripTests/Simplify_Sqrt_Square_Under_Real_Is_Still_Abs_X#0` | `AbsSymbolicRoundTripTests.cs:88`: `type(simplify_full(sqrt(x^2)))` is `TransformResult`; its `expression` prints `abs(x)` | `lovelace-lang::round80_abs_rewrite::simplify_sqrt_square_under_real_is_still_abs_x` (`:76`) | Same script; asserts the same type and expression, and additionally `changed` True, one `DomainCondition(variable: x, domain: real)`, and exactly one `pow.sqrt-square-real` / `DomainSpecific` step. |
| `.../AbsSymbolicRoundTripTests/Subs_Abs_Symbolic_At_Negative_Value_Gives_Three#0` | `AbsSymbolicRoundTripTests.cs:73`: `subs(abs(x), x, -3)` prints `3`, canonically `(rat 3 1)`, no free symbols | `lovelace-lang::round80_abs_rewrite::subs_abs_symbolic_at_negative_value_gives_three` (`:113`) | Same script and the same three assertions (display, canonical, empty `free_symbols`); rejects `abs(-3)`. |
| `.../UnsatisfiablePropagationTests/SimplifyFull_DomainCondition_CarriesADomainValueNotText#0` | `UnsatisfiablePropagationTests.cs:80`: `conditions[0]` is a `DomainCondition` record with `variable` (Symbolic) and `domain` (Domain Real), and no `SymbolDomainAssumption` text | `lovelace-lang::round80_abs_rewrite::simplify_full_domain_condition_carries_a_domain_value_not_text` (`:128`) | Same script; asserts exactly one `DomainCondition`, the two field names and kinds, canonical `(sym x)`, domain display `real`, and the absence of `SymbolDomainAssumption`. |
| `.../ExpectedFailureBoundaryTests/EvaluateToExpr_ExactArgument_ThrowingEvaluatorPropagates#0` | `ExpectedFailureBoundaryTests.cs:35`: `EvaluateToExpr(boom(2))` throws `InvalidOperationException` with `boom: deliberately unexpected failure` | `lovelace-lang::round80_throwing_evaluator::evaluate_to_expr_exact_argument_throwing_evaluator_propagates` (`round80_throwing_evaluator.rs:94`) | Same expression shape through `evaluation_boundary::evaluate_to_expr`; asserts the same kind string, message and error variant. |
| `.../ExpectedFailureBoundaryTests/EvaluateToExpr_ApproximateArgument_ThrowingEvaluatorPropagates#0` | `ExpectedFailureBoundaryTests.cs:46`: `0.` followed by 25 ones selects the approximate fold; the same exception and message escape | `lovelace-lang::round80_throwing_evaluator::evaluate_to_expr_approximate_argument_throwing_evaluator_propagates` (`:109`) | Same 25-digit Real argument; asserts the same kind and message from the approximate-argument branch. |
| `.../ExpectedFailureBoundaryTests/Limit_ThrowingEvaluatorPropagates#0` | `ExpectedFailureBoundaryTests.cs:63`: `Limits.Limit(boom(x), x, 0, TwoSided)` throws `InvalidOperationException` with the boom message | `lovelace-lang::round80_throwing_evaluator::limit_throwing_evaluator_propagates` (`:125`) | Same limit shape through `evaluation_boundary::limit`; asserts the same kind and message from the direct-substitution step. |
| `.../ExpectedFailureBoundaryTests/TryCompare_ReportsUnorderedInsteadOfThrowing#0` | `ExpectedFailureBoundaryTests.cs:94`: `TryCompare` orders 2 vs 3, reports 2 vs `1+1i` as false/0, and `Compare(2, 1+1i)` throws `InvalidOperationException` | `lovelace-lang::round80_throwing_evaluator::try_compare_reports_unordered_instead_of_throwing` (`:146`) | Same ordering and unordered-pair assertions, plus the ordered path's exact `Complex value cannot convert to Real (use re()/im()).` message. |

The map reasons record the block lift (the `pow.sqrt-square-real` rewrite and exact function fold;
the additive `evaluation_boundary` surface) and the notes cite the C# line, the Rust test line and
the round-80 report section (`abs-rewrite.md` / `throwing-evaluator.md` section 3 / section 4).

## 4. Second regeneration — 8/8 resolve

After the map write (`parity/manifest-map.json` sha256
`69770df3f87d6c146c458a9d722acfde98cab6124f8c29a7756bee67d47004d3` ->
`9058e5e628f97f7abe35077e87d0b67932c4f83b174f8b984ce81c90cd27ab4b`; exactly the 8 keys changed and
no other key was touched, verified by comparing the parsed objects), `node parity/gen-manifest.mjs`,
`parity/gate-manifest.ps1` and `node parity/tools/coverage-report.mjs` were run again:

```
{
  "rows": 6209,
  "dispositions": {
    "excluded": 118,
    "ported": 5979,
    "deferred": 112
  },
  "coveragePercent": 96.3,
  "rustTests": 5262
}
```

```
"dispositions": { "ported": 5979, "consolidated": 0, "excluded": 118, "deferred": 112 },
"coveragePercent": 96.3,
"claimedRustTests": 5979,
"failures": []
PASS (structural): 6209 test cases dispositioned; 5979 claim Rust coverage (96.3%).
```

```
coverage.md written: 5979/6209 ported (96.30%)
```

Targeted check over the generated manifest and the measured inventory:

```
RESOLVED 8/8; suite exit 0; unresolved: none
```

Per project: Lovelace.Symbolics.Tests 1,158 -> 1,166 ported (94.45 % -> 95.11 %); deferred 68 -> 60.

**Final coverage line: 5,979/6,209 ported (96.30 %), 118 excluded, 112 deferred.**

## 5. Scope

Changed by this round:

* `parity/manifest-map.json` — 8 entries flipped `deferred` -> `ported` with their round-80 Rust
  test ids; no other key changed.
* `parity/manifest.toml`, `parity/rust-tests.json`, `parity/coverage.md` — regenerated.
* `docs/goal-parity-contract/round-81/round80-porting.md` — this report.

No `.rs`, `.cs`, fixture, corpus or other document was edited; the round-80 Rust files and test
modules remain the two lanes' uncommitted work, used here only as the measured inventory.
