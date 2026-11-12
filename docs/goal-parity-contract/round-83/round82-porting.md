# Round 83 — round-82 lane porting map flip

**Status: complete.** The 8 census rows the round-82 series-Laurent and evalf-precision blocks
unblocked are mapped to their row tests, and the regenerated gate resolves 8/8. Coverage moved from
5,979/6,209 (96.30 %) to 5,987/6,209 (96.42 %); deferred 112 -> 104; excluded unchanged at 118.
Series row #3 was already ported and was not touched.

Measured 2026-11-12 (America/Sao_Paulo) on the working tree. Scope: `parity/manifest-map.json`,
the generated parity files and this report; no Rust, C#, fixture, corpus or other document was
edited.

## 1. Pre-regeneration baseline — stale inventory

The checked-in inventory predated both round-82 blocks: `parity/rust-tests.json` recorded
`generatedAtUtc 2026-11-12T14:06:03Z`, `suite {exitCode 0, passed 5261, failed 0}`, 11 crates /
5,262 tests, `sourcesSha256 2f346250f7456e6cac672beea4a2509e850c830f237c3d38e5b4ce6b22eef012`,
and **zero** `round82_*` test names. The map and manifest still deferred the eight rows, so the
recorded coverage was:

**Pre-regen line: 5,979/6,209 ported (96.30 %), 118 excluded, 112 deferred; inventory STALE
(5,262 listed / 5,261 passed, 0 `round82_*` names), so the gate's freshness check cannot pass —
the stale `sourcesSha256` 2f34… is not the current source walk.**

The fresh inventory lists 16 more tests than the stale one: the two round-82 blocks' 14 (6 series
+ 4 language evalf + 4 runner evalf) plus two desktop `ui::app::tests` regression tests already in
the shared tree. The stale hash and the 14 missing `round82_*` names are what made the pre-regen
inventory untruthful.

## 2. First regeneration — re-established baseline

Commands: `parity/gen-rust-tests.ps1`, `node parity/gen-manifest.mjs`, `parity/gate-manifest.ps1`.
The checked-in `.ps1` scripts are unsigned in this environment, so they were invoked with
`powershell.exe -NoProfile -ExecutionPolicy Bypass -File`; the scripts were not modified. Observed
lines:

```
crates=11 tests=5278 suiteExit=0 passed=5277 failed=0
```

```
{
  "rows": 6209,
  "dispositions": {
    "excluded": 118,
    "ported": 5979,
    "deferred": 112
  },
  "coveragePercent": 96.3,
  "rustTests": 5278
}
```

```
PASS (structural): 6209 test cases dispositioned; 5979 claim Rust coverage (96.3%).
```

The fresh inventory (`generatedAtUtc 2026-11-12T14:37:32Z`, suite exit 0,
`sourcesSha256 be18e365d9754376c6d15bd9dc18ac69943b635da899d5cb731f819a9e6f8bde`) lists 5,278
tests: 5,277 passed plus the workspace's one ignored test
(`property_assertions::mathir_evaluates_like_symbolic`, the only entry of
`cargo test --workspace -- --ignored --list`). All 14 `round82_*` names are present. The baseline
is truthful but coverage is unchanged: **5,979/6,209 (96.30 %), 118 excluded, 112 deferred, gate
PASS**.

## 3. The eight mappings

Each C# body was read at its census source line; each mapped Rust test is the row's own test, not a
supporting/neighbour test. The four series rows are #0, #1, #2 and #4. **#3
(`series(sqrt(x), x, 0, 2)`) is NOT closed here**: it was already ported to
`lovelace-lang::symbolics_tail_round34::series_laurent_sqrt_at_zero_is_unchanged` in round 34 and
was left byte-identical.

In the evalf family, the two `Lovelace.Run.Tests` rows map to the two runner tests that assert
their payloads and the two `Lovelace.Symbolics.Tests` rows to the two language tests. The other
tests in those modules are supporting evidence, not census rows and not mapped: the series
order/Laurent boundaries, the `sin`-fold production rule, and the cap boundary/control pairs.

| Census id | C# row assertion | Rust test (file:line) | Why this is the row's test |
|---|---|---|---|
| `Lovelace.Symbolics.Tests/.../SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#0` | `SeriesLaurentAndBranchPointTests.cs:54` InlineData `series(1/x, x, 0, 2)` -> display `x^-1 + O(x^2)`; the body rejects `sqrt(0)` and `/0` anywhere in the envelope | `lovelace-lang::round82_series_laurent::series_across_a_pole_keeps_the_laurent_leading_power` (`round82_series_laurent.rs:46`) | Same script through `Engine::eval`; asserts the same display and canonical `(add (pow (sym x) (rat -1 1)) (order …))`. The negative leading power IS the row. |
| `.../Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#1` | `SeriesLaurentAndBranchPointTests.cs:54` InlineData `series(1/x^2, x, 0, 2)` -> `x^-2 + O(x^2)`; same no-division-by-zero body | `lovelace-lang::round82_series_laurent::series_across_a_second_order_pole_keeps_two_negative_powers` (`:59`) | Same script; asserts `x^-2 + O(x^2)` and canonical `(add (pow (sym x) (rat -2 1)) (order …))`, i.e. the denominator's leading term survives a two-coefficient expansion. |
| `.../Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#2` | `SeriesLaurentAndBranchPointTests.cs:54` InlineData `series(sin(x)/x, x, 0, 3)` -> `1 - 1/6*x^2 + O(x^3)` | `lovelace-lang::round82_series_laurent::series_at_a_removable_singularity_keeps_the_hidden_quadratic_term` (`:72`) | Same script; asserts the same display and canonical `(add (rat 1 1) (mul (rat -1 6) (pow (sym x) (rat 2 1))) (order …))` — the hidden `x^2` term under a truthful `O(x^3)`. |
| `.../Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#4` | `SeriesLaurentAndBranchPointTests.cs:54` InlineData `series(sqrt(abs(x)), x, 0, 2)` -> `sqrt(x)`; the body rejects `sqrt(0)` and `/0` | `lovelace-lang::round82_series_laurent::series_at_a_branch_point_publishes_the_kink_resolved_source` (`:85`) | Same script; asserts display `sqrt(x)`, canonical `(pow (sym x) (rat 1 2))`, and no `abs`, `sqrt(0)` or `/0` — the kink-resolved source, not the unresolved input. |
| `Lovelace.Run.Tests/.../EvalfDigitCapHonestyTests/ARequestAtTheCap_IsNotMarked#0` | `EvalfDigitCapHonestyTests.cs:93` asserts `evalf(pi, 1000)` carries `Cap` (=1000) fractional digits and null `truncated`/`truncationReason`/`budget` | `lovelace-run::round82_evalf_precision::a_request_at_the_cap_is_honoured_in_full_and_unmarked` (`rust/lovelace-run/src/round82_evalf_precision.rs:87`) | Same script through the runner's own `parity_batch::run_args`; asserts the same digit count and the same three absent markers, plus the 100-place display and the `_` variable carrying the same digits. |
| `Lovelace.Run.Tests/.../StructuredRealPrecisionTests/EvalfCountAboveTheAmbientDisplayPrecision_CrossesInFull#0` | `StructuredRealPrecisionTests.cs:100` asserts 1,000 decimals, the `3.`+PiFirst60 prefix, the digits 981-1000 suffix, and `variables[_]` equal to the result value | `lovelace-run::round82_evalf_precision::evalf_count_above_the_ambient_display_precision_crosses_in_full` (`round82_evalf_precision.rs:128`) | Same script and the same four assertions through `run_args`; the value carries 1,000 digits while the display in force shows 100. |
| `Lovelace.Symbolics.Tests/.../EvalfTruncatedConstantExactnessTests/ExactControls_AreUntouched#0` | `EvalfTruncatedConstantExactnessTests.cs:125` asserts `evalf(1/2, 40)` is Real 0.5 exact with numerator/denominator, `2+3` exact 5, `sin(0)` exact Symbolic pretty `0`, and `evalf(sqrt(2), 40)` inexact with no rational form | `lovelace-lang::round82_evalf_precision::evalf_exact_controls_are_untouched_after_the_precision_repair` (`rust/lovelace-lang/src/round82_evalf_precision.rs:65`) | Same four expressions through `structured::project_value`; asserts the same fields and additionally canonical `(rat 0 1)`, domain `rational` and `nodeCount` 1 for `sin(0)`. |
| `Lovelace.Symbolics.Tests/.../EvalfWorkingPrecisionHonoursRequestedDecimalsTests/ValuesBelowTheComputationCap_StayInexact#0` | `EvalfWorkingPrecisionHonoursRequestedDecimalsTests.cs:174` loops `evalf(1/(3*10^1000), 30)`, `evalf(1/(3*10^1001), 30)`, `evalf(sin(pi(30)), 40)`: Real, value `0`, `exact` false, no numerator/denominator | `lovelace-lang::round82_evalf_precision::evalf_values_below_the_computation_cap_stay_inexact_zeros` (`round82_evalf_precision.rs:114`) | Same three expressions and the same five assertions through `project_value`, including the row's own `evalf(sin(pi(30)), 40)` — not the neighbouring exact `sin(0)` control. |

The map reasons record the block lift (the Laurent-aware series path; the computation-precision
constants and inexact-constant fold) and the notes cite the C# line, the Rust test line and the
round-82 report section (`series-laurent.md` / `evalf-precision.md` section 6).

## 4. Second regeneration — 8/8 resolve

The map was re-read immediately before writing and guarded by its expected sha256
`9058e5e628f97f7abe35077e87d0b67932c4f83b174f8b984ce81c90cd27ab4b`; the write produced
`4f27a7f6a81787f47aee21c572c6b4aab8dcd01301b5fb3c47595b5cdd213d8e`. Exactly the 8 keys changed
and no other key was touched, verified by comparing every parsed entry before and after; each new
`rust` id was confirmed to exist in the fresh inventory. `node parity/gen-manifest.mjs`,
`parity/gate-manifest.ps1` and `node parity/tools/coverage-report.mjs` were run again:

```
{
  "rows": 6209,
  "dispositions": {
    "excluded": 118,
    "ported": 5987,
    "deferred": 104
  },
  "coveragePercent": 96.42,
  "rustTests": 5278
}
```

```
{
  "censusCases": 6209,
  "manifestRows": 6209,
  "dispositions": {
    "ported": 5987,
    "consolidated": 0,
    "excluded": 118,
    "deferred": 104
  },
  "coveragePercent": 96.42,
  "claimedRustTests": 5987,
  "projectsInCensus": 15,
  "failures": []
}
PASS (structural): 6209 test cases dispositioned; 5987 claim Rust coverage (96.42%).
```

```
coverage.md written: 5987/6209 ported (96.42%)
```

Targeted check over the regenerated manifest and the measured inventory:

```
RESOLVED 8/8; suite exit 0; unresolved: none
```

Focused reruns after the flip: `cargo test -p lovelace-lang round82_` -> 10 passed, 0 failed;
`cargo test -p lovelace-run round82_evalf` -> 4 passed, 0 failed. Series row #3 still resolves to
`lovelace-lang::symbolics_tail_round34::series_laurent_sqrt_at_zero_is_unchanged`. Per project:
Lovelace.Symbolics.Tests 1,166 -> 1,172 ported (95.11 % -> 95.60 %), deferred 60 -> 54;
Lovelace.Run.Tests 744 -> 746 ported (99.07 % -> 99.33 %), deferred 7 -> 5.

**Final coverage line: 5,987/6,209 ported (96.42 %), 118 excluded, 104 deferred.**

## 5. Scope

Changed by this round:

* `parity/manifest-map.json` — 8 entries flipped `deferred` -> `ported` with their round-82 Rust
  test ids; no other key changed.
* `parity/manifest.toml`, `parity/rust-tests.json`, `parity/coverage.md` — regenerated.
* `docs/goal-parity-contract/round-83/round82-porting.md` — this report.

No `.rs`, `.cs`, fixture, corpus or other document was edited. The working tree also carries the
round-82 lanes' uncommitted Rust sources and test modules; they were read only as the measured
inventory.
