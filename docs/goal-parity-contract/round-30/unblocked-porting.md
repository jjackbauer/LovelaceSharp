# Round 30 — porting the rows the round-29 production fixes unblocked

Scope of THIS port: rust test code only (the four named crates), `parity/manifest-map.json`, and
this report. This port changed no production file, no `.cs` file, no fixture and no other document.
The single writer on `parity/manifest-map.json` re-read the file immediately before each write; the
final diff against HEAD changes exactly the 140 mapped rows (120 keys added, 20 replaced) and nothing
else.

The shared worktree also carries a concurrent production lane's changes (complex closed forms in
`rust/lovelace-symbolics`, the numeric power fallback and shape guards in
`rust/lovelace-lang/src/builtins.rs`, the help/pretty surfaces, and one corrected stale expectation in
`rust/lovelace-lang/src/latex_and_sqrt_parity.rs`). Those files are listed in section 8 as NOT changed
by this port; the final gates below measure the combined tree, whose Rust suite is green.

Baseline: HEAD `ab12078`, 4,975/6,209 ported (80.13 %), 118 excluded, 1,116 deferred,
`parity/rust-tests.json` suite green at 4,304 tests.

Result: **5,115/6,209 ported (82.38 %), 976 deferred, 118 excluded** — 140 new verified mappings:
the 135 ids in `docs/goal-parity-contract/round-29/blocker-ledger.md` section 5 plus five adjacent
rows the same production fixes also unblock (section 3).

## 1. Method

1. Section 5's ids were read from `parity/census.json` (project/class/method/case index), not from a
   name pattern, so each id maps to exactly the C# theory row or fact it names.
2. Every C# assertion was re-read from the C# source before mapping. A row was counted only when the
   Rust test makes the same assertion on the same input; where the existing round-29 test was weaker
   than the C# method (the periodic and solve records), a new test was appended that repeats the
   missing assertion, and the row points at the new test.
3. A mapped id resolves through `gate-manifest.mjs` to a test name in the measured
   `parity/rust-tests.json`, whose suite run recorded `exitCode: 0`.
4. The 12 boundary cells are covered by adding their four fractional literals to the canonical
   `MATRIX` table in `rust/lovelace-run/src/parity_batch.rs`; the existing `matrix_concat_axis`,
   `matrix_dft_n` and `matrix_range_bound` tests then drive them through the same `matrix_case`
   contract (recoverable refusal, exit 1, `InvalidArgument`/`TypeMismatch`, builtin prefix, no raw
   framework text). The table-size guard went from 260 to 272 cells.

## 2. The 135 ledger rows

| Cluster | Rows | Census ids | Rust test(s) |
|---|---:|---|---|
| LReal64 fixed-width tier | 58 | every deferred id in `Lovelace.Real.Tests.LReal64Tests` (Parse #0-#20, Add #0-#5, Subtract #0-#4, Multiply #0-#3, Divide #0-#10, Compare #0-#4, Add_Periodic, Add_Tenths, Divide_LongPeriod, Add_Overflow, Multiply_Overflow, Parse_TooManyDigits) | `lovelace-real::lreal_tests::lreal64_*` (12 functions, each iterating the method's own C# rows) |
| LReal128 fixed-width tier | 26 | every deferred id in `Lovelace.Real.Tests.LReal128Tests` (Parse #0-#9, Add #0-#3, Multiply #0-#2, Divide #0-#4, Add_Periodic, Add_Tenths, Multiply_Overflow, Divide_LongPeriod) | `lovelace-real::lreal_tests::lreal128_*` (8 functions) |
| Real periodic add/sub | 18 | `RealAddPeriodicExactTests` Add and Subtract at #582-#584, #606-#608, #630-#632 (q=19, s=17) | 18 new per-row tests in `lovelace-real::real_parity_batch` (`add_p*_q19_*_s17`, `sub_p*_q19_*_s17`) + the independent-rational `round29_real_batch::real_add_periodic_pairs_denote_the_exact_rational_sum` / `..._difference` property tests |
| Integer boundary cells | 12 | `IntegerArgumentBoundaryMatrixTests.EveryBoundaryCell...` #116-#119 (concat), #216-#219 (dft), #308-#311 (range) | `lovelace-run::parity_batch::matrix_concat_axis`, `matrix_dft_n`, `matrix_range_bound` |
| Integer inventory templates | 7 | `EveryInventoryTemplate_ReachesTheArgumentItNames` #5, #6, #7, #8, #11, #21, #30 | `lovelace-run::round29_run_batch::integer_boundary_inventory_templates_reach_the_argument_they_name` |
| inspect record | 10 | `InspectionExactnessTests` Inspect_AnswersExactnessAsABoolean #3, #5, #6; InspectExact_AgreesWithTheValuesOwnExactFlag #2, #4, #5; Inspect_KeepsNullWhereExactnessDoesNotApply #0, #1, #2; TruncatedQuotientAtTheBudget #0 | `lovelace-run::round29_run_batch::inspection_answers_exactness_as_a_boolean_for_every_numeric_kind`, `inspection_exact_agrees_with_the_values_own_exact_flag`, `inspection_keeps_null_where_exactness_does_not_apply`, `inspection_truncated_quotient_at_the_budget_is_not_exact` |
| solve folded relations | 4 | `SolveEmptySetTests` Solve / SolveFull / SolveSystem of a provably false equation; `SystemSolveRecordTests` SolveSystem_StatusCompleteAndCompleteness #2 | `lovelace-run::round30_run_batch::solve_of_a_provably_false_equation_publishes_the_solve_result_record` and `solve_full_of_a_provably_false_equation_publishes_no_solutions_complete` (new, full C# record assertions); `round29_run_batch::solve_system_of_a_folded_false_equation_answers_the_empty_set`; `round29_run_batch::solve_system_of_a_non_polynomial_system_is_an_unevaluated_record` |
| **total** | **135** | | |

## 3. Adjacent rows the same fixes unblock (5)

These were not in section 5 but fail/pass on the same production behaviour; each is asserted at the
same fidelity as its C# row.

| Row | Fix that unblocks it | Rust test |
|---|---|---|
| `Lovelace.Suite.Tests.Lovelace.Suite.Tests.StructuredRealExactnessTests/FixedWidthTruncation_IsNotPublishedAsAnExactReal#0` | new LReal64/LReal128 tier: the row parses `0.(0588235294117647)`, multiplies by 17, requires the product to differ from One, and requires `ToReal()` to stay inexact through `StructuredProjection` | `lovelace-lang::round30_batch::fixed_width_truncation_is_not_published_as_an_exact_real` (new) |
| `Lovelace.Suite.Tests.Lovelace.Suite.Tests.DspPluginTests/Dft_GivenLength_MatchesPaddedSignal#0` | integer-argument contract for dft: the optional length now pads/truncates, so `dft([1,2,3],4)` equals `dft([1,2,3,0])` | `lovelace-lang::suite_engine_batch::dft_given_length_matches_padded_signal` (new) |
| `Lovelace.Real.Tests.Lovelace.Real.Tests.RealDivideExactnessPropertyTests/DivisionAndItsMultiplicationTwin_AgreeOnValueAndExactness#0` | exact periodic add/sub/mul fraction: the periodic mul path no longer truncates the twin (1/7 * 1/23) | `lovelace-real::round30_real_batch::division_and_its_multiplication_twin_agree_on_value_and_exactness` (new, the full 10,000-combination C# property) |
| `Lovelace.Symbolics.Tests.Lovelace.Symbolics.Tests.DxStructuredResultsTests/Inspect_ArrayShapeRankAndElementDomain_AreStructural#0` | rebuilt inspect record: arrays publish shape, rank and element_domain from the value, not a symbolic summary | `lovelace-run::dx_batch::dx_inspect_array_shape_rank_and_element_domain_are_structural` (new) |
| `Lovelace.Run.Tests.Lovelace.Run.Tests.InspectionExactnessTests/Inspect_AnswersExactnessAsABoolean#4` | rebuilt inspect record + the current symbolic kernel: `inspect((-4)^(1/2))` answers type Symbolic, exact Boolean true | `lovelace-run::round29_run_batch::inspection_answers_exactness_as_a_boolean_for_every_numeric_kind` (one row added to the existing theory loop) |

The last row supersedes the round-29 ledger's note that it "still fails at EVALUATION". The current
release runner (measured: `inspect((-4)^(1/2))` -> `Inspection(type: Symbolic, ..., exact: True, pretty: 2*i)`)
and the in-process test both answer it; the row is counted, not weakened. The numeric `evalf` of the
same expression is still refused (section 6).

## 4. Tests appended

| File | Change |
|---|---|
| `rust/lovelace-real/src/lib.rs` | 18 per-row periodic tests appended to `real_parity_batch`; they call the existing `check_add_row`/`check_sub_row` helpers, which assert the exact rational and the Real-level equality |
| `rust/lovelace-real/src/round30_real_batch.rs` | new: the 10,000-combination division/multiplication-twin property |
| `rust/lovelace-run/src/round30_run_batch.rs` | new: the two solve record tests with the full C# field/kind assertions |
| `rust/lovelace-run/src/dx_batch.rs` | new: the array inspect structural test |
| `rust/lovelace-run/src/parity_batch.rs` | the 12 fractional boundary cells added to the `MATRIX` table; the size guard updated 260 -> 272 |
| `rust/lovelace-run/src/round29_run_batch.rs` | one row (`(-4)^(1/2)`, true) added to the inspect exactness theory |
| `rust/lovelace-lang/src/round30_batch.rs` | new: the fixed-width truncation exactness row |
| `rust/lovelace-lang/src/suite_engine_batch.rs` | new: the dft length == padded-signal row |
| `rust/lovelace-lang/src/lib.rs`, `rust/lovelace-run/src/main.rs` | `#[cfg(test)]` module registrations only |

24 new test functions plus one new row in an existing theory: real 19, run 3, lang 2. The measured
suite is **4,355 tests** (4,304 at the baseline; the difference is these 24 functions plus 27 tests
from the concurrent production lane's closed-form/help/pretty work). The full suite is green,
including `latex_and_sqrt_parity`.

## 5. Gates observed

```
$ powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1
crates=11 tests=4355 suiteExit=0 passed=4355 failed=0
(inventory sourcesSha256 = the measured tree's 28442e302aa4b060c0e0108521d9f138195fda79aac5960161d70580bbe402c1)

$ rust/target/release/lovelace-corpus.exe
FAIL sym/071    (pre-existing)
FAIL sym/076    (pre-existing)
corpus: 212 passed, 2 failed, 214 total

$ node parity/tools/gate-fixtures.mjs
oracle: 20/20
rust:   20/20
PASS: oracle 20/20, rust 20/20

$ node parity/tools/gate-manifest.mjs
{
  "censusCases": 6209,
  "manifestRows": 6209,
  "dispositions": { "ported": 5115, "consolidated": 0, "excluded": 118, "deferred": 976 },
  "coveragePercent": 82.38,
  "claimedRustTests": 5115,
  "projectsInCensus": 15,
  "failures": []
}
PASS (structural): 6209 test cases dispositioned; 5115 claim Rust coverage (82.38%).

$ node parity/tools/coverage-report.mjs
coverage.md written: 5115/6209 ported (82.38%)
```

The two corpus failures are the round-12 baseline (`sym/071`, and `sym/076` which renders
`-1/(x*y)` where C# renders `-x/x/(x*y)`); they are untouched by this round.

## 6. Refusals and what remains

- `Inspect_ReportsTheAssumptionsThatConstrainTheFreeSymbols#0` remains deferred: the inspect
  record's `assumptions` is still the empty vector because the engine has no `assume` bridge. Probe:
  `x = symbol("x"); y = symbol("y"); assume(x > 5); inspect(x^2 + y)` -> `assumptions: []`.
- `ExactnessFlagTests/InspectExact_IsFalse_WhenARealApproximationIsInTheTree#0` remains deferred:
  `inspect(x*sqrt(2)).exact` and `inspect(subs(x^2 + 1, x, sqrt(2))).exact` both answer `true`; the
  symbolic exactness flag is a different gap from the record schema.
- `TruncatingEvaluationStaysInexactTests/ExactControls_StayExact#0` and
  `EvalfTruncatedConstantExactnessTests/ExactControls_AreUntouched#0` remain deferred:
  `evalf((-4)^(1/2), 40)` is still refused with `Function 'i' has no numeric evaluator.` The symbolic
  value now exists, but the numeric evaluator does not consume it.
- Nine of the 11 `ComplexClosedFormTests` rows are now exercised by the production lane's new
  `rust/lovelace-symbolics/src/complex_closed_forms_round30.rs` (9 tests, green in the measured
  suite; the two quadratic-negative-discriminant rows are not in it), but this port did not map them:
  they are the concurrent production lane's row set, not one of the 140 ids this report claims. They
  remain deferred in the manifest.
- No production change was made by this port to force any of the above.

## 7. What could not be verified

- The C# xUnit suite itself cannot be executed in this environment (the VSTest testhost limitation
  recorded in `parity/README.md`); the C# source is the assertion contract and the pinned C# runner
  is the behavioural oracle. Every mapped test was transcribed from the C# source's own inputs and
  assertions, and the Rust suite was run.
- The production lane's correction of the stale expectation in
  `rust/lovelace-lang/src/latex_and_sqrt_parity.rs` (`x^(1/2)` -> `sqrt(x)`) is present in the final
  tree; I did not touch that file. The measured `lovelace-lang` run is green (1,140 passed), which
  includes that module.
- `cargo fmt` / clippy were not run: they are not part of this round's gate set.
- The differential runner was not run: its generator does not cover the changed surfaces; the wire
  probes and the mapped tests are the evidence.

## 8. Files changed

| File | Change |
|---|---|
| `parity/manifest-map.json` | 140 deferred ids mapped to ported Rust tests (120 new keys, 20 replaced), with the C# source line/data and the Rust assertion in each note |
| `rust/lovelace-real/src/lib.rs` | 18 periodic tests + module registration |
| `rust/lovelace-real/src/round30_real_batch.rs` | new test file |
| `rust/lovelace-run/src/parity_batch.rs` | 12 boundary cells + size guard |
| `rust/lovelace-run/src/round30_run_batch.rs` | new test file |
| `rust/lovelace-run/src/dx_batch.rs` | one adjacent inspect test |
| `rust/lovelace-run/src/round29_run_batch.rs` | one theory row |
| `rust/lovelace-run/src/main.rs` | test module registration |
| `rust/lovelace-lang/src/round30_batch.rs` | new test file |
| `rust/lovelace-lang/src/suite_engine_batch.rs` | one adjacent dft test |
| `rust/lovelace-lang/src/lib.rs` | test module registration |
| `docs/goal-parity-contract/round-30/unblocked-porting.md` | this report |
| `parity/manifest.toml`, `parity/rust-tests.json`, `parity/coverage.md` | regenerated by the gates |

Not changed by this port, but present in the measured tree because the concurrent production lane
landed them: `rust/lovelace-lang/src/builtins.rs`, `rust/lovelace-lang/src/help.rs`,
`rust/lovelace-lang/src/latex_and_sqrt_parity.rs`, `rust/lovelace-run/src/pretty_batch.rs`,
`rust/lovelace-symbolics/src/lib.rs` and `rust/lovelace-symbolics/src/complex_closed_forms_round30.rs`.
The inventory, suite and gates above were measured after those edits stopped (the source hash was
stable for 2.5 minutes before the final run, and the inventory's `sourcesSha256` matches the final
tree).
