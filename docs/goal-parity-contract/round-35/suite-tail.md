# Round 35 — the Suite API tail: per-row adjudication, and the deferred rows the recent production unblocked

Status: complete for the Suite tail and the re-attempts below. 123 previously unadjudicated
`Lovelace.Suite.Tests` rows were classified one by one: 14 are mappable now and ported with exact
assertions, 88 are blocked by a named production gap, and 21 need a capability external to the Rust
workspace. A further 74 deferred rows outside the Suite tail were re-attempted against the current
production and ported, and the final assumption/tri-state and RealLiteral production unblocked 36
more rows (section 4b), for 124 new verified mappings in total. The full suite, corpus, fixtures and
manifest gates are recorded in section 6.

The census counts 929 `Lovelace.Suite.Tests` cases; 806 were already dispositioned before this
round, and 133 were deferred (123 of them with no recorded reason at all). The round brief's
97-row figure undercounts the measured tail; this report classifies all 123 unadjudicated rows
and lists the 10 earlier deferrals separately in appendix B.

## 0. Population and result

| Measure | Rows |
|---|---:|
| Deferred `Lovelace.Suite.Tests` rows at round start | 133 |
| Of those: previously adjudicated (recorded reason) | 10 |
| Of those: unadjudicated at round start | 123 |
| (a) mappable now and ported | 14 |
| (b) blocked by a named production gap | 88 |
| (c) blocked by a missing external capability | 21 |
| Deferred rows outside the Suite tail re-attempted and ported | 74 |
| Assumption/tri-state and RealLiteral rows mapped after the final production | 36 |
| **New verified mappings in this round** | **124** |

Every (a) row is a Rust test that asserts the same case the C# assertion demands. Where the Rust
surface has a different shape — no CLR exception types, no mutating operators, no `decimal`/`f64`
constructor — the test asserts the same value or refusal through the Rust surface and the row's
note records the translation; a weaker proxy is recorded as (b), never as (a).

## 1. (a) Suite tail rows mapped now (14)

| Census row | Rust test |
|---|---|
| `Zeros_ElementProductBeyondTheBudget_IsRefusedByNameAndLimit#0` | `lovelace-lang::suite_tail_round35::zeros_element_product_beyond_the_budget_is_refused_by_name_and_limit` |
| `Zeros_TenToTheNinth_IsRefusedWithoutAllocating#0` | `lovelace-lang::suite_tail_round35::zeros_ten_to_the_ninth_is_refused_without_allocating` |
| `ConcurrentEngines_KeepTheirOwnAssumptions#0` | `lovelace-lang::suite_tail_round35::concurrent_engines_keep_their_own_assumptions` |
| `Funcs_All_IsCategorized#0` | `lovelace-lang::suite_tail_round35::funcs_all_is_categorized` |
| `Funcs_Calculus_ListsCalculusFunctions#0` | `lovelace-lang::suite_tail_round35::funcs_calculus_lists_calculus_functions` |
| `Help_Compile_ShowsDescriptor#0` | `lovelace-lang::suite_tail_round35::help_compile_shows_descriptor` |
| `Help_Solve_ShowsSignatureSummaryExampleAndSeeAlso#0` | `lovelace-lang::suite_tail_round35::help_solve_shows_signature_summary_example_and_see_also` |
| `Help_Symbolics_ListsCoreFunctions#0` | `lovelace-lang::suite_tail_round35::help_symbolics_lists_core_functions` |
| `Help_UnknownArgument_Explains#0` | `lovelace-lang::suite_tail_round35::help_unknown_argument_explains` |
| `FlatChainOnePastTheBudget_IsRefused#0` | `lovelace-lang::suite_tail_round35::flat_chain_one_past_the_budget_is_refused` |
| `SymbolicConstructionOnePastTheBudget_IsRefused#0` | `lovelace-lang::suite_tail_round35::symbolic_construction_one_past_the_budget_is_refused` |
| `OnePluginTwoEngines_AssumptionInA_DoesNotDecideB#0` | `lovelace-lang::suite_tail_round35::assumption_in_one_engine_does_not_decide_another` |
| `OnePluginTwoEngines_EveryAssumptionReaderAnswersForTheAskingEngine#0` | `lovelace-lang::suite_tail_round35::every_assumption_reader_answers_for_the_asking_engine` |
| `Evaluate_GivenFunctionDefinition_ExposesSignatureAndBody#0` | `lovelace-lang::suite_tail_round35::function_definition_exposes_signature_and_body` |

The mapped cases are the six help-surface rows the round-35 help aliases unblocked, the two array
allocation refusals, the two input-depth refusals, the function-definition signature row, and the
three engine-isolation rows the concurrent assumption-kernel lane unblocked.

## 2. (b) Blocked families (88 rows)

Ranked by rows. Each entry names the measured Rust behaviour against the C# contract.

| # | Family | Rows | Measured Rust behaviour vs the C# assertion |
|---:|---|---:|---|
| 1 | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) | 20 | The live tokenizer publishes `syntax::Tok` (Num/Repr/Str/Interp/Ident/Punct(String)/Semi/Eof); there is no 20-member `TokenKind` ordinal enum. The `token.rs` file that has one is not declared in `lib.rs` and is not compiled. |
| 2 | no RecordSchemas registry (For/Validate) | 9 | No `RecordSchemas::For`/`Validate`; a record is `Value::Record(name, fields)` and nothing declares or validates the field schema. |
| 3 | no public Value.Widen/WidenPair (widening is internal to builtins::binary) | 9 | `Value` publishes no `Widen`/`WidenPair`; numeric widening happens inside `builtins::binary` and is not callable on two values. |
| 4 | no ScriptSource.ToSemicolonStatements/normalize_source API | 7 | There is no public source-normalisation function. Newline→semicolon handling lives in `syntax::tokenize`, and the BOM strip in `Engine::eval_inner`; neither returns the normalised text the C# row asserts. |
| 5 | no PlotModel/PlotSeries/PlotValue/SvgPlotRenderer surface (engine plot only) | 4 | The renderer model is not a Rust API; only the `plot` builtin exists. Periodic-value expansion, huge-natural conversion and the linear/spline model pair cannot be stated. |
| 6 | no StructuralEquality/FirstDifference API | 4 | No `StructuralEquality.ValuesEqual`/`FirstDifference`; `Value` has derived `PartialEq` and symbolic canonical equality, but no differing-path diagnostic. |
| 7 | no CaptureState snapshot API | 3 | `Engine` has no `CaptureState`; variables and functions are read through `variables()`/`functions()`. |
| 8 | filter has no dispatcher arm; complex arithmetic names widening, not re()/im()/conj()/abs() | 3 | Measured: `filter([1,-0.5],[1],4)` → `Unknown function 'filter'. Did you mean 'filter'?` (the name is in the registry, not in the dispatcher). `fft(...)[1] + 1` → `Cannot widen from Natural to Complex: only numeric kinds (Natural, Integer, Real) support widening.` |
| 9 | PlotFileError message does not name the caller path; empty plot args are CLI usage errors | 3 | Measured: an unwritable plot file yields `Cannot write plot: Access is denied. (os error 5).` (no caller path), and an empty `--plot-file`/`--plot-dir` value is rejected by the argument parser as a usage error (exit 2). |
| 10 | 2^1000000000 and 2000000! do not observe the deadline (return values, not Cancelled) | 2 | Measured with `set_cancel_after(1500 ms)`: both evaluations return a value; `was_cancelled()` stays false and no `Cancelled` failure is published. |
| 11 | suggest() reads the live registry (fft is registered); far-distance sentinel is usize::MAX, C# int.MaxValue | 2 | `suggest("fft")` consults the live registry where `fft` exists; `edit_distance` is capped with `usize::MAX`, not `int.MaxValue` (round-28 already refused the sentinel row). |
| 12 | no unary-plus AST node; assignment is not an expression production (chained assignment) | 2 | `+5` parses to the literal `5` (identity, no `Unary(Plus)` node), and `x = y = 5` is refused because assignment is parsed only at statement level. |
| 13 | member-access refusals do not name the record type or its available members | 2 | Measured: `r.nope` → `No field named 'nope'.`; `v.length` → `Cannot read field 'length' from a value of kind 'Vector'.` Neither names the record type or its members. |
| 14 | loop statements evaluate to Void; the reference's for/while value is the last body value | 2 | Measured: the for/while compositions of `f(85)` answer Void, while the C# row expects the recursion value 0. |
| 15 | structured::project_value has no depth guard | 2 | The projection recurses over the value with no depth check; only `eval`/display enforce the 1024-level budget, so the structured-projection refusal rows cannot hold. |
| 16 | no ArgumentContractException/ArgumentContractFailure surface | 1 | The language refuses the argument, but there is no contract object carrying Failure/Builtin/Parameter/Received. |
| 17 | function-local assignments leak into the global variable table (measured) | 1 | Measured: after `func f() { local = 42 }` and `f()`, `variables()` contains `local`; the C# engine scopes function locals. |
| 18 | help overview lacks the literal 'help <function>' hint | 1 | `help::overview()` lists categories and counts but has no `help <function>` line. |
| 19 | Engine has no UnicodeOutput toggle | 1 | There is no unicode output mode; `inf` always renders as `inf`. |
| 20 | engine plot collapses huge-close x values onto one pixel (measured points 40.000,... twice) | 1 | Measured: `plot([10^20, 10^20+1], [0,1])` writes `points="40.000,360.000 40.000,40.000"`; the two x values are not distinct, which is exactly what the C# row forbids. |
| 21 | f(10) with a 100-term body answers 1000 instead of refusing the evaluation depth | 1 | Measured: the deep-body recursive function answers 1000; the C# row expects the evaluation-depth refusal. |
| 22 | block-local assignments leak into the global variable table (measured) | 1 | Measured: after `{ z = 99 }`, `variables()` contains `z`; the C# engine scopes block locals. |
| 23 | eval() takes no output writer (output is drained by take_output) | 1 | There is no writer parameter; captured print output is read back with `take_output()`. |
| 24 | no reset_plot_capture API | 1 | `last_plot()` exists; there is no reset. |
| 25 | OpTiming publishes no elapsed time | 1 | `OpTiming` publishes `position`, `result_kind` and `has_output` only; there is no elapsed duration, display string or scale. |
| 26 | no FunctionDefined event surface | 1 | The engine has no event/delegate surface. |
| 27 | no public SetVariable/RemoveVariable (set_var is crate-private and add-only) | 1 | The concurrent lane added a crate-private `set_var`; there is no public set/remove API and no snapshot to test immutability against. |
| 28 | no VariableChanged event surface | 1 | The engine has no event/delegate surface. |
| 29 | OpTiming publishes no elapsed/scale pair | 1 | Same as the elapsed rows: position/kind/output only. |

## 3. (c) Missing external capability (21 rows)

These rows assert types or seams of the C# host architecture that the Rust workspace does not have
and that a production fix inside the current crates cannot provide; they are not behaviour gaps in
the language kernel.

| Capability | Rows | Why external |
|---|---:|---|
| `IModusPlugin` / `IModusContext` plugin ABI | 10 | `ModusTests` + `ModusArityTests` assert plugin registration, descriptor arity enforcement and kernel dispatch. The Rust engine grows one builtin enum instead of a plugin ABI (the same disposition as `Lovelace.Abstractions`, absent in `parity/projects.toml`). |
| Opt-in DSP plugin registration | 3 | `DspPlugin_GivenUnregisteredEngine`, `DspPlugin_LoadedTwice`, `PluginBuiltin_GivenCoreNameCollision`: the Rust engine has no load/unload seam because the DSP builtins are always registered. |
| Host-registered builtin seam | 3 | `BuiltinArgumentShapeGuardTests` registers a builtin from the test and asserts the host's coercion guard attributes the failure; there is no host registration API. |
| CLR type identity | 1 | `ArrayAllocationRefusalTests.TheSuiteDeclaresTheAllocationRefusalTheRunnerClassifies` reflects on a CLR exception type name. |
| Modus payload seam | 2 | `Record_CrossesModusPluginBoundary` and `EnumPayloadSeamTests.EnumValue_RoundTripsThroughTheModusSeam` need the plugin boundary. |
| Caller-installed `ExprContext` assumption scope | 1 | `CallerInstalledAssumptionScope_KeepsPrecedenceInsideAnEngineEvaluation` asserts a scope object the Rust symbolics API does not publish. |
| `RegisterBuiltin` hold-gate | 1 | `OnePluginTwoEngines_AssumptionCommittedWhileAIsRunning...` needs a host builtin to hold an engine mid-evaluation. |

## 4. Re-attempted deferred rows outside the Suite tail (74 mapped)

The recent production (digit-store, inexact-division semantics, LReal tiers, complex closed forms,
circular families, trig precision, help aliases, and the assumption kernel that landed while this
round was running) was probed against these deferred rows. The 74 rows below now assert the C#
contract through the allowed crate surfaces; rows the probes still refused are listed in section 5.

| Project | Family | Rows | Rust test(s) |
|---|---|---:|---|
| Lovelace.Natural.Tests | increment/decrement (prefix and postfix) | NaturalIncrementTests (9) + NaturalSubtractTests (5) | `natural_increment_prefix_rows`, `natural_increment_postfix_rows`, `natural_decrement_prefix_rows`, `natural_decrement_postfix_row` |
| Lovelace.Natural.Tests | factorial (5 InlineData rows) | NaturalRandomizedCrossCheckTests | `natural_factorial_rows` |
| Lovelace.Natural.Tests | concurrent multiplication | NaturalMultiplyTests (2) | `natural_concurrent_multiply_rows` |
| Lovelace.Natural.Tests | fixed carry/division cross-checks | NaturalRandomizedCrossCheckTests (2) | `natural_adversarial_carry_patterns`, `natural_overflow_alignment_division` |
| Lovelace.Natural.Tests | lazy parse under concurrency | NaturalDecimalCacheTests | `natural_concurrent_arithmetic_on_lazy_parsed_values` |
| Lovelace.Integer.Tests | increment/decrement and pre-operators | IntegerIncrementTests (8) | `integer_increment_decrement_rows` |
| Lovelace.Integer.Tests | parity predicates | IntegerPredicateTests (8) | `integer_parity_predicate_rows` |
| Lovelace.Integer.Tests | natural / signed-magnitude constructors | IntegerConstructorTests (4) | `integer_constructor_rows` |
| Lovelace.Integer.Tests | factorial of a negative integer | IntegerFactorialTests | `integer_factorial_of_negative_is_refused` |
| Lovelace.Integer.Tests | unary plus | IntegerNegationTests (3) | `integer_unary_plus_rows` |
| Lovelace.Array.Tests | sum / min / max / mean / norm / dot / cross / matmul / det / inverse / trace | NdArrayTests (7) | `array_sum_all_and_axis`, `array_min_max`, `array_mean_and_norm`, `array_dot_and_cross`, `array_matmul_2x2`, `array_det_inverse_trace`, `array_inverse_singular_is_refused` |
| Lovelace.Real.Tests | Real operator% (8 rows) | RealModulusTests | `real_modulus_rows` |
| Lovelace.Real.Tests | pi digit-count refusals | RealPiTests (2) | `real_pi_refuses_zero_and_negative_digit_counts` |
| Lovelace.Real.Tests | decimal constructors | RealConstructorTests (3) | `real_decimal_constructor_rows` |
| Lovelace.Run.Tests | unknown-function text surface | UnknownFunctionEnvelopeTests | `unknown_function_text_surface_carries_the_same_answer` |
| Lovelace.Run.Tests | real-process stdout purity | StdoutPurityTests | `real_process_stdout_is_one_json_document_and_print_stays_in_output` |
| Lovelace.Run.Tests | limit short-form refusal diagnostics | LimitShortFormRecordTests (3) | `limit_short_form_refusals_carry_the_diagnostic` |
| Lovelace.Run.Tests | structured assumption atoms | BuiltinSurfaceContractTests | `assumptions_returns_structured_atoms_with_display_and_domain_condition` |

Translation notes for the 74 rows:

- The immutable `Nat`/`Int` value types express the C# mutating `++`/`--` as the pair (old value,
  rebound new value); both facts the C# row asserts are asserted.
- `Nat::factorial`/`Int` constructors take the same values the C# row constructs; a negative
  `Integer` factorial is the language's named domain refusal.
- The fixed random-derived rows (`Factorial` InlineData, adversarial carry patterns, overflow
  alignment) use the exact constants the C# row pins; the BigInt expectations were computed
  independently. The five `System.Random` corpus rows remain refused (family 22).
- `ArrayMath.*` and `Real.%` are asserted through the language builtins that publish the same
  operation (the Rust `lovelace-array` crate has no ArrayMath module; the math lives in
  `lovelace-lang`'s builtins). The values and refusals are the C# assertions' own.
- The Real `%` rows use Real literals (`7.0 % 3.0`), because the language narrows an all-integer
  quotient/remainder back to Natural; the C# rows construct `Real.Parse("7")`.
- The `Lovelace.Run.Tests` envelope rows are built in-process through `envelope_for` — the same
  function the binary's `run()` calls — except stdout purity and the `--text` surface, which spawn
  the real binary because those assertions are about the process.

Oracle cross-check for the mapped language rows: the C# oracle runner (`dotnet exec
Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll`) was executed on the same scripts and agreed with
the Rust answers used above — `7.0 % 3.0` → Real 1, `2.5 % 1.2` → Real 0.1, `0.0 % 5.0` →
Real 0, `det([[1,2],[3,4]])` → Integer -2, `sum([[1,2],[3,4]], 0)` → Vector [4, 6], and `pi(0)` →
`InvalidArgument/TypeMismatch` with the message both runners publish.

## 4b. Assumption/tri-state and RealLiteral rows (36 mapped after the final production)

The concurrent production lane's final report (`docs/goal-parity-contract/round-35/assumptions-realliteral.md`)
landed the tri-state `AssumptionSet` and the `Expr::Real` leaf. The 33 + 33 ledger rows were
re-checked against the C# assertions; 36 are mapped below, and the remainder is refused with the
measured reason in section 5.

| Family | Rows | Rust test(s) |
|---|---:|---|
| assumption language surface (rewrites, tri-state answers, inspect, scaling) | 8 | `lovelace-lang::assumptions_round35::*` (`simplify_sqrt_square...`, `simplify_relation...` for both AskRelation rows, `compatible_relations...`, `inspect_reports...`, `simplify_abs_square...`, `simplify_log_exp...`, `eight_hundred_atoms...`) |
| assumption kernel (truth table, contradictions, domains, intervals, sentinel) | 11 | `lovelace-symbolics::assumptions::tests::*` (`bound_truth_table...`, `contradiction_detection...`, `domain_atoms_merge...`, `interval_bounds...`, `real_valued_bounds...`, `unsatisfiable_set...`, `integer_interval_emptiness...` for 4 rows, `interval_condition_im_zero...`) |
| assumption supplements (dedup, sentinel, non-empty integer interval, predicate lattice, Or) | 7 | `lovelace-lang::round35_extra_tail::*` (`the_same_atom...`, `unsatisfiable_sentinel...`, `integer_intervals_that_hold...`, `predicate_lattice_rows` for 3 rows, `or_returns_false...`) |
| RealLiteral/RealConstant (leaf exactness, evalf of approximate arguments) | 10 | `lovelace-lang::real_constant_round35::*` (2 rows) and `lovelace-lang::round35_extra_tail::*` (`elementary_functions...` for sin/cos/exp/log, `truncating_exact_controls...`, `functions_of_a_truncated_value...` for #0/#1/#11) |

The measured translation limits: the C# `RealLiteral.FromRealExact(...).ToReal()` round-trip has no
Rust type — the `Expr::Real` leaf is the carrier, and the rows that construct a 500-digit literal and
add/multiply it through the kernel API stay deferred (no real-constant arithmetic API). The
substitution/leaf/exactness/evalf rows are asserted end to end. The ScopeIsolation tests have no
Rust `WithAssumptions` scope to restore, so both stay deferred rather than being asserted through
two engines; the kernel's per-engine isolation is already covered by the Suite-tail rows.

## 5. Probed and refused (measured divergences)

Every candidate below was measured on the current tree and refused; a weaker proxy would have
violated the contract rule.

| Candidate | Measured behaviour |
|---|---|
| Real.Sqrt batch rows (5) | `sqrt([4,9,16,25])` → `sqrt needs a number.`; `sqrt` has no batch/broadcast form. |
| Real.Sqrt negative rows (2 incl. batch) | `sqrt(-1)` → `i` (Symbolic); `sqrt(-4.0)` → `2*i`; the C# contract is an ArithmeticException. |
| Dsp.Fft empty / non-power-of-two (2) | `fft([])` → `dft needs a non-empty vector.`; `fft([1,1,1])` → `[3,0,0]` (the Rust fft is a DFT and accepts any length). |
| CancellationObservation (2) | `2^1000000000` and `2000000!` return values under a 1500 ms deadline; no Cancelled failure. |
| PlotValue huge natural / periodic (2) | No PlotValue model; the conversion cannot be invoked. |
| SvgPlotRenderer linear/spline (2) | No interpolation selector on the `plot` builtin; only the default dense polyline exists. |
| RecordValue unknown-field / non-record (2) | Messages do not name the record type or members (section 2). |
| PlotFileWriteRefusal (3) | Unwritable paths are auto-created (missing parent succeeds) or named without the caller path; empty values are usage errors. |
| EvalfDigitCapHonesty.ARequestAtTheCap (1) | `evalf(pi,1000)` publishes 100 digits (the ambient display cap), not 1000; the guarded row above it is the same gap. |
| StructuredRealPrecision.EvalfCountAboveTheAmbientDisplayPrecision (1) | Same measured truncation at 100 digits. |
| Real.Pi exceeding the max digits (1) | The Rust `pi` argument is capped at 1000 but there is no `MaxComputationDecimalPlaces` scope to pin the threshold. |
| Cross-engine/locals scoping (4) | Function/block locals leak; measured `variables()` contains `local`/`z`. |
| Recursion compositions / deep body (3) | For/while statements answer Void; the deep-body recursion answers 1000 instead of refusing. |
| Real divide leading-zero-run (3) | The Real exposes no Exponent/PeriodStart/PeriodLength/ToNatural accessors; only the string and exactness could be checked, which is weaker. |
| Run.Tests plot dir/file envelopes (5) | The runner creates missing plot directories and accepts empty values differently; the C# envelope/path assertions do not hold. |
| Run.Tests CallSignature / Arity sweeps (5+) | `compile_full(x^2+1)` reports `InvalidOperation`/`compile needs an expression and parameters.` instead of the declared-arity message; the `filter` probe cannot even reach arity. |
| AssumptionAddScaling Add_MixedAtoms / Add_OrderEqualsHistorical | Measured: mixed-kind atoms are NOT insertion-order independent (`forward != backward` on the 8-atom set), and the 50-order row needs the C# `System.Random(20250825)` sequence (family 22, unreproducible). |
| AssumptionScopeIsolation (2) | No `WithAssumptions`/`ExprContext` scope: nested-scope restore and two-concurrent-scope isolation cannot be stated; two-engine isolation is a different mechanism. |
| AssumptionSoundness PropertyAndRelationAtoms | Measured: the NonZero/Eq pair is accepted when the relation is inserted first (`backward` does not contradict); the C# row demands every pair in both orders. |
| DomainOf_NegativeIntegerPower / ParameterDomainOf | No published `DomainOf` language surface and no `ParameterDomain` enum (the family report's own two deferrals). |
| EvalfTruncatedConstant ExactControls / EvalfWorking ValuesBelowTheCap | Measured: `sin(0)` stays the Symbolic `sin(0)` (C# asserts pretty `0`), and `evalf(sin(pi(30)),40)` is `5.028841971e-31`, not `0`. |
| RealLiteral C#-API rows (CachedFlag corpus, Radicals, HugeExponent/500-digit add/multiply, NegativePower x7, FunctionOfATruncatedArgument, LiteralRoundTrip_KeepsExact, SymbolicTreeFlag x4) | No `RealLiteral` struct or real-constant arithmetic/kernel-readback API in Rust; the leaf covers substitution/exactness/evalf, not these constructors. ElementaryFunction abs/ceil (#4/#5) also need the kernel function path. |

## 6. Verification

| Gate | Command | Observed result |
|---|---|---|
| Suite | `cargo test --workspace` | 4,653 passed; 0 failed (exit 0) |
| Inventory | `powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1` | `crates=11 tests=4653 suiteExit=0 passed=4653 failed=0` |
| Manifest | `node parity/gen-manifest.mjs` | rows 6209: ported 5489, deferred 602, excluded 118; coverage 88.40 % |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `PASS (structural): 6209 test cases dispositioned; 5489 claim Rust coverage (88.4 %).` failures: [] |
| Coverage | `node parity/tools/coverage-report.mjs` | `coverage.md written: 5489/6209 ported (88.40 %)` |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `PASS: oracle 20/20, rust 20/20` |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, PASS |
| Corpus | `cargo run --release -p lovelace-corpus` | `corpus: 212 passed, 2 failed, 214 total` (sym/071 and sym/076, the documented pre-existing divergences) |
| Structural gates | census / projects / requirements | census PASS 2105 methods / 6209 cases; projects PASS 19; requirements PASS 262 |

The corpus run must be made with `TMP`/`TEMP` set to `rust/target/tmp` (the task environment's
values): the host's default `%LocalAppData%\Temp` ACL denies the plot write, which otherwise adds a
spurious `lang/076` failure ("Cannot write plot: Access is denied"). With the documented environment
the corpus is 212/214 and the two failures are the known sym/071 and sym/076 divergences.

The suite ran on the stable tree after the concurrent production lane's final edit; the 4,653 tests
include this round's 51 tests (`suite_tail_round35` 38, `round35_run_batch` 4,
`round35_extra_tail` 9) and the assumption/RealLiteral lane's 39 tests.

## Appendix A — every Suite tail row (123)

| Census row | Class | Cat | Family |
|---|---|---|---|
| `TheSuiteDeclaresTheAllocationRefusalTheRunnerClassifies#0` | ArrayAllocationRefusalTests | c | asserts the existence of a CLR exception type (no CLR type identity in Rust) |
| `Zeros_ElementProductBeyondTheBudget_IsRefusedByNameAndLimit#0` | ArrayAllocationRefusalTests | a | mapped |
| `Zeros_TenToTheNinth_IsRefusedWithoutAllocating#0` | ArrayAllocationRefusalTests | a | mapped |
| `CaptureState_ReportsArrayKind#0` | ArrayTests | b | no CaptureState snapshot API |
| `ACoercionFailureOnAnEngineInternalValue_IsNotDisguisedAsAnArgumentError#0` | BuiltinArgumentShapeGuardTests | c | no host-registered builtin seam to attribute a coercion failure |
| `ARegisteredBuiltinThatCastsItsOwnPayload_IsAttributedToTheArgumentItReadLast#0` | BuiltinArgumentShapeGuardTests | c | no host-registered builtin seam to attribute a coercion failure |
| `ARegisteredBuiltinThatCoercesItsArgument_IsAttributedToTheBuiltinAndThePosition#0` | BuiltinArgumentShapeGuardTests | c | no host-registered builtin seam to attribute a coercion failure |
| `Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly#0` | CancellationObservationTests | b | 2^1000000000 and 2000000! do not observe the deadline (return values, not Cancelled) |
| `Evaluate_GivenLongNaturalPowerAndShortBudget_CancelsPromptly#0` | CancellationObservationTests | b | 2^1000000000 and 2000000! do not observe the deadline (return values, not Cancelled) |
| `ConcurrentEngines_KeepTheirOwnAssumptions#0` | CrossEngineIsolationTests | a | mapped |
| `TheExactlyIntegralRealRefusal_IsStillTheContractsContract#0` | DspIntegerArgumentContractTests | b | no ArgumentContractException/ArgumentContractFailure surface |
| `ComplexArithmetic_GivenComplex_ThrowsActionableError#0` | DspPluginTests | b | filter has no dispatcher arm; complex arithmetic names widening, not re()/im()/conj()/abs() |
| `DspPlugin_GivenUnregisteredEngine_ThrowsUnknownFunction#0` | DspPluginTests | c | no opt-in plugin registration seam (DSP builtins are always registered) |
| `DspPlugin_LoadedTwice_Throws#0` | DspPluginTests | c | no opt-in plugin registration seam (DSP builtins are always registered) |
| `Filter_GivenLength_ReturnsImpulseResponse#0` | DspPluginTests | b | filter has no dispatcher arm; complex arithmetic names widening, not re()/im()/conj()/abs() |
| `Filter_GivenSignal_MatchesConvolution#0` | DspPluginTests | b | filter has no dispatcher arm; complex arithmetic names widening, not re()/im()/conj()/abs() |
| `PluginBuiltin_GivenCoreNameCollision_Throws#0` | DspPluginTests | c | no opt-in plugin registration seam (DSP builtins are always registered) |
| `EnumValue_RoundTripsThroughTheModusSeam#0` | EnumPayloadSeamTests | c | no Modus payload seam (Enum exists, but not across a plugin boundary) |
| `Evaluate_GivenFunctionLocalVariable_DoesNotLeakToGlobal#0` | FunctionTests | b | function-local assignments leak into the global variable table (measured) |
| `Funcs_All_IsCategorized#0` | HelpServiceTests | a | mapped |
| `Funcs_Calculus_ListsCalculusFunctions#0` | HelpServiceTests | a | mapped |
| `Help_Compile_ShowsDescriptor#0` | HelpServiceTests | a | mapped |
| `Help_Overview_ListsAllCategories#0` | HelpServiceTests | b | help overview lacks the literal 'help <function>' hint |
| `Help_Solve_ShowsSignatureSummaryExampleAndSeeAlso#0` | HelpServiceTests | a | mapped |
| `Help_Symbolics_ListsCoreFunctions#0` | HelpServiceTests | a | mapped |
| `Help_UnknownArgument_Explains#0` | HelpServiceTests | a | mapped |
| `UnicodeOutput_ThreadsThroughEngineFormatting#0` | HelpServiceTests | b | Engine has no UnicodeOutput toggle |
| `FlatChainOnePastTheBudget_IsRefused#0` | InputDepthGuardTests | a | mapped |
| `SymbolicConstructionOnePastTheBudget_IsRefused#0` | InputDepthGuardTests | a | mapped |
| `ExactCount_RequiresTheDeclaredCount#0` | ModusArityTests | c | no IModusPlugin/IModusContext host seam |
| `MinArity_AllowsTheOptionalTail_AndStillRefusesLess#0` | ModusArityTests | c | no IModusPlugin/IModusContext host seam |
| `ParametersOnlyRegistration_UsesTheSameRule#0` | ModusArityTests | c | no IModusPlugin/IModusContext host seam |
| `Variadic_RemovesTheUpperBound_AndHonoursTheLowerOne#0` | ModusArityTests | c | no IModusPlugin/IModusContext host seam |
| `Dispatch_UnsupportedElementType_FallsBack#0` | ModusTests | c | no IModusPlugin/IModusContext host seam |
| `DspPlugin_LoadedThroughModusSeam_RegistersDspBuiltins#0` | ModusTests | c | no IModusPlugin/IModusContext host seam |
| `Kernel_DeclinesUnsupportedOp#0` | ModusTests | c | no IModusPlugin/IModusContext host seam |
| `PluginLoad_RegistersKernel_AndDispatchRuns#0` | ModusTests | c | no IModusPlugin/IModusContext host seam |
| `RealAddKernel_GivenRealOperands_MatchesDirectRealAddition#0` | ModusTests | c | no IModusPlugin/IModusContext host seam |
| `ScalarResult_GivenBooleanAndText_ReturnValues#0` | ModusTests | c | no IModusPlugin/IModusContext host seam |
| `ANameNothingIsCloseTo_IsNotGuessedAt#0` | NameSuggestionsTests | b | suggest() reads the live registry (fft is registered); far-distance sentinel is usize::MAX, C# int.MaxValue |
| `EditDistance_IsTheStructuralMetric#3` | NameSuggestionsTests | b | suggest() reads the live registry (fft is registered); far-distance sentinel is usize::MAX, C# int.MaxValue |
| `Parse_GivenChainedAssignment_RightAssociative#0` | ParserTests | b | no unary-plus AST node; assignment is not an expression production (chained assignment) |
| `Parse_GivenUnaryPlus_ReturnsUnaryExprPlus#0` | ParserTests | b | no unary-plus AST node; assignment is not an expression production (chained assignment) |
| `EmptyPlotFile_IsRefusedByType#0` | PlotFileWriteRefusalTests | b | PlotFileError message does not name the caller path; empty plot args are CLI usage errors |
| `PlotFileUnderAMissingDirectory_IsRefusedByType#0` | PlotFileWriteRefusalTests | b | PlotFileError message does not name the caller path; empty plot args are CLI usage errors |
| `TheSuiteDeclaresThePlotFileWriteRefusalTheRunnerClassifies#0` | PlotFileWriteRefusalTests | b | PlotFileError message does not name the caller path; empty plot args are CLI usage errors |
| `PlotValue_GivenHugeNatural_ToRealPreservesDigitsExactly#0` | PlotTests | b | no PlotModel/PlotSeries/PlotValue/SvgPlotRenderer surface (engine plot only) |
| `PlotValue_GivenPeriodicReal_ExpandsToDouble#0` | PlotTests | b | no PlotModel/PlotSeries/PlotValue/SvgPlotRenderer surface (engine plot only) |
| `SvgPlotRenderer_GivenCubicSpline_PassesThroughDataEndpoints#0` | PlotTests | b | no PlotModel/PlotSeries/PlotValue/SvgPlotRenderer surface (engine plot only) |
| `SvgPlotRenderer_GivenHugeCloseXValues_KeepsPointsDistinct#0` | PlotTests | b | engine plot collapses huge-close x values onto one pixel (measured points 40.000,... twice) |
| `SvgPlotRenderer_GivenLinearInterpolation_EmitsPolyline#0` | PlotTests | b | no PlotModel/PlotSeries/PlotValue/SvgPlotRenderer surface (engine plot only) |
| `DiagnosticSchema_DeclaresTheSixFrozenFields#0` | RecordSchemaTests | b | no RecordSchemas registry (For/Validate) |
| `EveryProducedRecord_ConformsToItsDeclaredSchema#0` | RecordSchemaTests | b | no RecordSchemas registry (For/Validate) |
| `SingularMatrix_IsACompleteNoSolutions_CarryingAStructuredDiagnostic#0` | RecordSchemaTests | b | no RecordSchemas registry (For/Validate) |
| `SingularMatrix_IsACompleteNoSolutions_CarryingAStructuredDiagnostic#1` | RecordSchemaTests | b | no RecordSchemas registry (For/Validate) |
| `SolvedMatrix_IsComplete_WithAnEmptyDiagnosticsArray#0` | RecordSchemaTests | b | no RecordSchemas registry (For/Validate) |
| `SolvedMatrix_IsComplete_WithAnEmptyDiagnosticsArray#1` | RecordSchemaTests | b | no RecordSchemas registry (For/Validate) |
| `SolverSchemas_DeclareTheStructuralKinds#0` | RecordSchemaTests | b | no RecordSchemas registry (For/Validate) |
| `UnknownRecordType_IsReportedAsUnknown_NotAsConforming#0` | RecordSchemaTests | b | no RecordSchemas registry (For/Validate) |
| `Validate_ReportsMissingAndUndeclaredFields#0` | RecordSchemaTests | b | no RecordSchemas registry (For/Validate) |
| `MemberAccess_OnNonRecord_ExplainsRequirement#0` | RecordValueTests | b | member-access refusals do not name the record type or its available members |
| `MemberAccess_UnknownField_NamesAvailableMembers#0` | RecordValueTests | b | member-access refusals do not name the record type or its available members |
| `Record_CrossesModusPluginBoundary#0` | RecordValueTests | c | no Modus plugin boundary |
| `TheDeepestAcceptedCall_Answers_InEveryComposition#14` | RecursionBudgetCompositionTests | b | loop statements evaluate to Void; the reference's for/while value is the last body value |
| `TheDeepestAcceptedCall_Answers_InEveryComposition#15` | RecursionBudgetCompositionTests | b | loop statements evaluate to Void; the reference's for/while value is the last body value |
| `ADeepExpressionInsideTheRecursiveBody_IsRefused#0` | RecursionDepthGuardTests | b | f(10) with a 100-term body answers 1000 instead of refusing the evaluation depth |
| `ToSemicolonStatements_GivenBlock_PreservesNewlinesInsideBraces#0` | ScriptSourceTests | b | no ScriptSource.ToSemicolonStatements/normalize_source API |
| `ToSemicolonStatements_GivenCrlf_NormalizesLineEndings#0` | ScriptSourceTests | b | no ScriptSource.ToSemicolonStatements/normalize_source API |
| `ToSemicolonStatements_GivenLeadingBom_StripsIt#0` | ScriptSourceTests | b | no ScriptSource.ToSemicolonStatements/normalize_source API |
| `ToSemicolonStatements_GivenNewlineAfterBlock_EmitsSemicolon#0` | ScriptSourceTests | b | no ScriptSource.ToSemicolonStatements/normalize_source API |
| `ToSemicolonStatements_GivenNewlineSeparatedStatements_JoinsWithSemicolons#0` | ScriptSourceTests | b | no ScriptSource.ToSemicolonStatements/normalize_source API |
| `ToSemicolonStatements_GivenStringWithNewline_PreservesIt#0` | ScriptSourceTests | b | no ScriptSource.ToSemicolonStatements/normalize_source API |
| `ToSemicolonStatements_GivenTrailingSemicolon_SuppressesBlankSeparator#0` | ScriptSourceTests | b | no ScriptSource.ToSemicolonStatements/normalize_source API |
| `CallerInstalledAssumptionScope_KeepsPrecedenceInsideAnEngineEvaluation#0` | SharedPluginInstanceAssumptionIsolationTests | c | no ExprContext assumption-scope API for caller-installed precedence |
| `OnePluginTwoEngines_AssumptionCommittedWhileAIsRunning_DoesNotDecideB#0` | SharedPluginInstanceAssumptionIsolationTests | c | no RegisterBuiltin/hold-gate seam for the interleaved evaluation |
| `OnePluginTwoEngines_AssumptionInA_DoesNotDecideB#0` | SharedPluginInstanceAssumptionIsolationTests | a | mapped |
| `OnePluginTwoEngines_EveryAssumptionReaderAnswersForTheAskingEngine#0` | SharedPluginInstanceAssumptionIsolationTests | a | mapped |
| `Evaluate_GivenVariableAssignmentInsideBlock_DoesNotLeak#0` | StatementTests | b | block-local assignments leak into the global variable table (measured) |
| `DifferenceInANestedField_IsNamedByPath#0` | StructuralEqualityTests | b | no StructuralEquality/FirstDifference API |
| `DifferentKinds_AreReportedByKind#0` | StructuralEqualityTests | b | no StructuralEquality/FirstDifference API |
| `DifferentShape_IsReportedAsShape#0` | StructuralEqualityTests | b | no StructuralEquality/FirstDifference API |
| `SymbolicEquality_IsByCanonicalForm_NotPrettyText#0` | StructuralEqualityTests | b | no StructuralEquality/FirstDifference API |
| `EvaluateAsync_GivenOutputWriter_CapturesPrintLinesToThatWriter#0` | SuiteEngineOutputPlotTests | b | eval() takes no output writer (output is drained by take_output) |
| `ResetPlotCapture_GivenPreviousPlot_ClearsLastPlot#0` | SuiteEngineOutputPlotTests | b | no reset_plot_capture API |
| `CaptureState_GivenVariablesAndFunctions_ReturnsMatchingSnapshot#0` | SuiteEngineTests | b | no CaptureState snapshot API |
| `CaptureState_IsImmutable#0` | SuiteEngineTests | b | no CaptureState snapshot API |
| `Evaluate_GivenFunctionDefinition_ExposesSignatureAndBody#0` | SuiteEngineTests | a | mapped |
| `EvaluateAsync_GivenScript_RecordsElapsedTime#0` | SuiteEngineTests | b | OpTiming publishes no elapsed time |
| `FunctionDefined_GivenDefinition_RaisesEvent#0` | SuiteEngineTests | b | no FunctionDefined event surface |
| `SetVariableAndRemoveVariable_Work#0` | SuiteEngineTests | b | no public SetVariable/RemoveVariable (set_var is crate-private and add-only) |
| `VariableChanged_GivenAssignment_RaisesEvent#0` | SuiteEngineTests | b | no VariableChanged event surface |
| `OperationTiming_ExposesTheSameScaleAsItsDisplay#0` | TimingScaleTests | b | OpTiming publishes no elapsed/scale pair |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#0` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#1` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#10` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#11` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#12` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#13` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#14` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#15` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#16` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#17` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#18` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#19` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#2` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#3` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#4` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#5` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#6` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#7` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#8` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `TokenKind_GivenAllValues_HasExpectedOrdinal#9` | TokenTests | b | token model has no TokenKind ordinal surface (live Tok is Num/Repr/Str/Interp/Ident/Punct/Semi/Eof) |
| `ADeepRecordChain_IsRefusedTheSameWay#0` | ValueDepthGuardTests | b | structured::project_value has no depth guard |
| `ValueOnePastTheBudget_IsRefusedByTheStructuredProjection#0` | ValueDepthGuardTests | b | structured::project_value has no depth guard |
| `Value_ToString_GivenNatural_PrefixesKindLabel#0` | ValueTests | b | no public Value.Widen/WidenPair (widening is internal to builtins::binary) |
| `Widen_GivenIntegerToReal_ReturnsRealValue#0` | ValueTests | b | no public Value.Widen/WidenPair (widening is internal to builtins::binary) |
| `Widen_GivenNaturalToInteger_ReturnsIntegerValue#0` | ValueTests | b | no public Value.Widen/WidenPair (widening is internal to builtins::binary) |
| `Widen_GivenNaturalToReal_ReturnsRealValue#0` | ValueTests | b | no public Value.Widen/WidenPair (widening is internal to builtins::binary) |
| `Widen_GivenSameKind_ReturnsSameValue#0` | ValueTests | b | no public Value.Widen/WidenPair (widening is internal to builtins::binary) |
| `Widen_GivenWiderToNarrower_ThrowsInvalidOperationException#0` | ValueTests | b | no public Value.Widen/WidenPair (widening is internal to builtins::binary) |
| `WidenPair_GivenNaturalAndInteger_ReturnsBothAsInteger#0` | ValueTests | b | no public Value.Widen/WidenPair (widening is internal to builtins::binary) |
| `WidenPair_GivenNaturalAndReal_ReturnsBothAsReal#0` | ValueTests | b | no public Value.Widen/WidenPair (widening is internal to builtins::binary) |
| `WidenPair_GivenSameKind_ReturnsUnchanged#0` | ValueTests | b | no public Value.Widen/WidenPair (widening is internal to builtins::binary) |

## Appendix B — previously adjudicated Suite rows still deferred (10)

| Census row | Class | Recorded reason |
|---|---|---|
| `ColumnVector_BroadcastsToMatrix#0` | BroadcastTests | [[1], [2]] + [[10, 20], [30, 40]] is refused ("Operands could not be broadcast together with shapes [2, 1] and [2, 2]"); C# broadcasts the column to [[11, 21], [32, 42]]. |
| `Example_Executes#107` | DescriptorExampleTests | Rust refuses the snippet: Unknown function 'scale'. |
| `Example_Executes#108` | DescriptorExampleTests | Rust refuses the snippet: Unknown function 'scale'. |
| `Example_Executes#11` | DescriptorExampleTests | Rust refuses the snippet: Unknown function 'assume_clear'. |
| `Example_Executes#21` | DescriptorExampleTests | Rust refuses the snippet: Unknown function 'cancel_full'. |
| `Example_Executes#22` | DescriptorExampleTests | Rust refuses the snippet: Unknown function 'cancel_full'. |
| `Example_Executes#53` | DescriptorExampleTests | Rust refuses the snippet: Unknown function 'filter'. |
| `Example_Executes#86` | DescriptorExampleTests | Rust refuses the snippet: Unknown function 'ndims'. |
| `TerminatingQuotientBeyondTheDigitBudget_IsCertifiedExact#2` | StructuredRealExactnessTests | C# certifies "2^-100000" as exact with the full power-of-two/five denominator; Rust publishes exact=false. |
| `TruncatedQuotientAtTheBudget_IsNotCertifiedExact#0` | StructuredRealExactnessTests | C# asserts value "0.000991080277502477" (18-place truncation at setprecision(18)); Rust renders "0.0009910802775024777006937561942517343904856293359762140733…" (display truncation width differs). |
