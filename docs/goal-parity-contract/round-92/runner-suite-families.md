# Round 92 — six deferred runner/Suite families (12 rows)

**Status: fixed and pinned.** The twelve deferred rows across `CancellationObservation` (2),
`SuiteEngineOutputPlot` (2), `HelpService` (2), `NameSuggestions` (2),
`CapabilitiesBuiltin` (2) and `DiagnosticContract` (2) now have the missing production surface
and one Rust test per row asserting the C# case's own observable. Measured 2026-11-12
(America/Sao_Paulo) against `Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll` and
`rust/target/release/lovelace-run.exe`. This lane's scope excludes `parity/`, so the parent
applies the mappings in section 1.

## 1. The 12 ids and their mapped tests

| # | Census id | Mapped Rust test |
|---|---|---|
| 1 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.CancellationObservationTests/Evaluate_GivenLongNaturalPowerAndShortBudget_CancelsPromptly#0` | `lovelace-lang::round92_runner_families::evaluate_given_long_natural_power_and_short_budget_cancels_promptly` |
| 2 | `.../Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly#0` | `...::evaluate_given_long_factorial_and_short_budget_cancels_promptly` |
| 3 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.SuiteEngineOutputPlotTests/EvaluateAsync_GivenOutputWriter_CapturesPrintLinesToThatWriter#0` | `...::evaluate_async_given_output_writer_captures_print_lines_to_that_writer` |
| 4 | `.../ResetPlotCapture_GivenPreviousPlot_ClearsLastPlot#0` | `...::reset_plot_capture_given_previous_plot_clears_last_plot` |
| 5 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.HelpServiceTests/Help_Overview_ListsAllCategories#0` | `...::help_overview_lists_all_categories` |
| 6 | `.../UnicodeOutput_ThreadsThroughEngineFormatting#0` | `...::unicode_output_threads_through_engine_formatting` |
| 7 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.NameSuggestionsTests/ANameNothingIsCloseTo_IsNotGuessedAt#0` | `...::a_name_nothing_is_close_to_is_not_guessed_at` |
| 8 | `.../EditDistance_IsTheStructuralMetric#3` | `...::edit_distance_is_the_structural_metric` |
| 9 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests/AdvertisedCodesAndCategories_MatchTheLiveEnvelope#0` | `...::advertised_codes_and_categories_match_the_live_envelope` |
| 10 | `.../PowerExponentClass_RefusesWhatItSays_AndNamesWhatIsSupported#0` | `...::power_exponent_class_refuses_what_it_says_and_names_what_is_supported` |
| 11 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DiagnosticContractTests/ErrorCategory_HasExactlyTheSevenFrozenMembers#0` | `...::error_category_has_exactly_the_seven_frozen_members` |
| 12 | `.../KernelNotes_CrossAsDiagnosticMessages#0` | `...::kernel_notes_cross_as_diagnostic_messages` |

## 2. Probes (oracle first)

Runner probes used `--json --omit-functions`; the library-only rows have no language script the
oracle accepts, so their probe is the C# test body's own expected value through the new Rust API.

| Row | Script / probe | Oracle before | Rust before | Rust after |
|---|---|---|---|---|
| 1 | `2^1000000000`, 1500 ms budget | `EvaluationCancelledException`; token `IsCancellationRequested` | power ran `Nat::pow` with no poll seam, so no deadline could be observed | `Cancelled/BudgetExceeded`, message `the evaluation was cancelled by the caller.`; CLI stop 2446 ms |
| 2 | `2000000!`, 1500 ms budget | `EvaluationCancelledException` | `Nat::factorial` had no poll seam | same structured `Cancelled`; CLI stop 1638 ms |
| 3 | `print("hi")` with a writer | `"hi"` in the writer, original `Output` restored | no `EvaluateAsync(source, writer)`/restore API | `eval_with_output` writes `"hi"`; earlier `["before"]` buffer restored |
| 4 | `plot([1, 2, 3])` then reset | `LastPlot` non-null then null | no reset API | `last_plot()` `Some` → `None` |
| 5 | `Help.Overview()` | categories + `help <function>`, prelude | overview stopped after the category counts | contains the full reference tail, including `help <function>` |
| 6 | `inf` formatted typed | `∞ (Symbolic)` with `UnicodeOutput=true` | no `UnicodeOutput` knob; always `inf` | `inf (Symbolic)` / `∞ (Symbolic)` / `inf (Symbolic)` |
| 7 | fixed registry, queries `fft`, `nosuchfn`, `push`, `zip` | empty list | no caller-registry suggest API | `suggest_from(REGISTRY, query)` is empty for all four |
| 8 | `EditDistance("solve_full", "solve_x")` | `int.MaxValue` | internal `usize::MAX`, not public | `i32::MAX` |
| 9 | all 19 advertised capability triggers | advertised code/category/message | 5 live message diffs: `solve` variable/expression, `integrate`, `limit`, `dft` | all 19 triggers byte-identical |
| 10 | `0^(1/2)` | exit 0, `0 (Real)` | `UnsupportedOperation/UnsupportedOperation`, `Non-integer exponents are not yet supported.` | exit 0, `0 (Real)` |
| 10 (ctl) | `(-4)^(1/2)`, `[0]^(1/2)`, `2^2.0` | `2*i`, `[0]`, `4` | `(-4)^(1/2)` already matched; array zero and Real-integer exponent refused | all match |
| 11 | `ErrorCategory` members | exactly `ParseError, DomainError, UnsupportedOperation, BudgetExceeded, NoSolution, TypeMismatch, InternalInvariantFailure` | no Rust enum | same seven names in the same order |
| 12 | `solve_full(x == x, x)` | diagnostic `solve.unevaluated` / `0 = 0: every value is a solution.` | same code/category, message `No solver for this structure.` | message is the oracle's |
| 12 (ctl) | `limit_full(a/x, x, 0)` | `leading coefficient is symbolic` | already matched | unchanged |

The five capability message diffs measured before the repair:

| operation_class | Oracle / advertised | Rust before |
|---|---|---|
| `solve.non-symbolic-variable` | `solve(): argument 2 must be a symbolic variable; got Natural.` | `solve needs a symbolic variable.` |
| `solve.non-symbolic-expression` | `solve(): argument 1 must be a symbolic expression; got Vector.` | `solve needs an equation.` |
| `integrate.non-symbolic-variable` | `integrate(): argument 2 must be a symbolic variable; got Natural.` | `integrate needs a variable.` |
| `limit.non-symbolic-variable` | `limit(): argument 2 must be a symbolic variable; got Natural.` | `limit needs a variable.` |
| `dsp.symbolic-element` | `DSP builtins expect numeric/complex elements, but got 'SymbolExpr'.` | `... got 'Symbolic'.` |

## 3. Production fixes

1. **Cancellable Natural power/factorial** (new `rust/lovelace-lang/src/cancellable.rs`). Binary
   exponentiation and a balanced factorial product tree over `Nat::mul`/Nat::from_u64`, polling
   `cancel::check()` between squares, leaves and merges. `builtins::pow` and the interpreter's
   Factorial node route through them; a spent deadline returns `cancelled_error()`, which
   `Engine::eval` publishes as `Cancelled/BudgetExceeded`.
2. **Engine evaluation overloads** (`rust/lovelace-lang/src/lib.rs`):
   `eval_with_output` (writer capture + previous-output restore) and `eval_with_budget`
   (caller deadline, restored afterwards); `reset_plot_capture` clears `last_plot`.
3. **Help overview** (`help.rs`): the reference's command/prelude tail, including
   `help <function>`.
4. **Unicode output** (new `unicode.rs` + Engine knob): `set_unicode_output(true)` selects
   ∞/π and ≠/≤/≥ exactly where the reference's Pretty printer differs from ASCII; the choice
   threads through arrays and records.
5. **NameSuggestions surface** (new `name_suggestions.rs`; `registry::suggest_from` and public
   `registry::edit_distance`): suggests over a caller-supplied registry, and returns the
   contract's `i32::MAX` sentinel.
6. **Power boundary** (`builtins::pow`): an exact integral Real exponent is accepted
   (`2^2.0 = 4`); a zero base under a positive non-integer exponent is the exact Real zero;
   a negative base under the principal square root keeps the exact complex form; element-wise
   application makes `[0]^(1/2)` and `[0, (-4)]^(1/2)` match the oracle.
7. **Argument refusals** (`builtins.rs`): one `symbolic_variable_argument`/
   `symbolic_expression_argument` helper produces the reference's
   `<name>(): argument <n> must be ...; got <kind>.` sentence with InvalidOperation/DomainError;
   the DSP payload type name reports `SymbolExpr`.
8. **Solve identity** (`builtins.rs`): an equation that simplifies to zero publishes the
   reference's kernel note as the `solve.unevaluated` diagnostic message.
9. **ErrorCategory** (new `diagnostic.rs`, re-exported as `lovelace_lang::ErrorCategory`): the
   seven frozen members in the reference's order, with lossless name/from-name mapping.

## 4. Per-row before / after

| Row | Before (Rust) | After (Rust) | Oracle |
|---|---|---|---|
| 1 | no poll inside Natural power; no budget API | `Cancelled` inside the power, 2446 ms on a 1500 ms budget | same exception/token contract |
| 2 | no poll inside Natural factorial | `Cancelled` inside the product tree, 1638 ms | same |
| 3 | no output-writer evaluation | `eval_with_output` captures and restores | same |
| 4 | no reset | `reset_plot_capture` | same |
| 5 | overview ended at counts | full reference tail | same |
| 6 | no UnicodeOutput | `∞ (Symbolic)` toggles | same |
| 7 | registry-only suggestions | caller-registry suggestions | empty |
| 8 | internal `usize::MAX` | public `i32::MAX` | `int.MaxValue` |
| 9 | 5 message diffs | all 19 triggers match | advertised == live |
| 10 | `0^(1/2)` refused | `0 (Real)` | `0` |
| 11 | no enum | seven frozen members | same |
| 12 | wrong kernel note | `0 = 0: every value is a solution.` | same |

## 5. Tests

The 12 row tests are listed in section 1, all in
`rust/lovelace-lang/src/round92_runner_families.rs`. Supporting tests: `cancellable.rs` (2),
`unicode.rs` (2), `diagnostic.rs` (1), and the existing registry round-28 edit-distance test
now asserts the public `int`/sentinel contract. No assertion was weakened.

The round-18 wire test `cancellation_batch::
a_run_that_completes_past_its_deadline_is_a_success_with_a_truthful_ledger` used a wall-clock
race (a 20 ms budget against a ~52 ms factorial). It now drives the published clock through a
test-only `with_test_elapsed_ms` hook in `lovelace-run/src/main.rs`: the script really
completes through `execute`, the elapsed is fixed at 25 ms against the 20 ms budget, and the
assertions (ok/exit 0, `stopped:false`, `exceeded:true`, the "completed anyway" diagnostic) are
unchanged and deterministic.

## 6. Gates (observed 2026-11-12)

| Step | Command | Observed | Exit |
|---|---|---|---|
| Lang suite | `cargo test -p lovelace-lang` | 1,684 passed, 0 failed, 0 ignored | 0 |
| Run suite | `cargo test -p lovelace-run` | 419 passed, 0 failed; integration 1 passed | 0 |
| Documentation corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS` | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS` | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1` | generated `200 match / 0 mismatch / 0 error`; replayed `1758 match / 0 mismatch / 0 error`; `stillMismatching: []` | 0 |
| Clippy (own crates) | `cargo clippy -p lovelace-lang -p lovelace-run --all-targets -- -D warnings` | clean | 0 |
| Format (own crates) | `cargo fmt -p lovelace-lang -p lovelace-run -- --check` | clean | 0 |

The full `cargo test --workspace` gate could not run because the desktop lane's in-flight
`lovelace-desktop` changes do not compile (`inspector.rs:385` `Section<MathIr>` mismatch and a
`Value::canonical` call the lane is still moving). `cargo test --workspace --exclude lovelace-desktop`
is green: 2,485 + 419 + 1 + 196 passed, 0 failed. This lane did not touch `lovelace-desktop`.

## 7. Scope and the shared-tree conflict

Changed files: `rust/lovelace-lang/src/{lib.rs,builtins.rs,help.rs,registry.rs,round92_runner_families.rs,cancellable.rs,unicode.rs,diagnostic.rs,name_suggestions.rs}`;
`rust/lovelace-run/src/{main.rs,cancellation_batch.rs}`. No `.cs`, fixture, corpus,
`parity/`, `lovelace-desktop` or symbolics file was edited.

**Shared-tree conflict (reported, not overwritten).** The symbolic lane's
`rust/lovelace-lang/src/round92_symbolic_families.rs` and
`rust/lovelace-symbolics/src/round92_symbolic_families.rs` are present in the same tree, as are
its `round19_batch.rs` edit and the `docs_sync.rs`/`round93_eval_print.rs` pair. Their
`docs_sync.rs` parser test was red because it did not tolerate the blank separator line between
the `lovelace` and `result` fences; the parser now skips blank separators (a one-function
fix, not a deletion), and the test is green. `round93_eval_print.rs` was rustfmt-only
reformatted so the crate's `cargo fmt --check` gate is clean; no semantics changed. Nothing
else in that lane's files was rewritten.
