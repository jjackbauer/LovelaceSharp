# Round 28 — Run/Suite porting from the round-27 fixes

Scope: tests only. New files `rust/lovelace-lang/src/round28_batch.rs` and
`rust/lovelace-run/src/round28_run_batch.rs`; a test-only module at the end of
`rust/lovelace-lang/src/registry.rs`; `#[cfg(test)] mod` registrations in `lib.rs`/`main.rs`;
`parity/manifest-map.json`; this report. No production line, `.cs` file, fixture or other doc was
changed. Every added Rust item compiles only under `#[cfg(test)]` except the map and this report.

## 1. Result

| Measure | Before | After |
|---|---:|---:|
| Manifest coverage | 4,874 / 6,209 (78.50 %) | **4,975 / 6,209 (80.13 %)** |
| Net new verified mappings | — | **+101** |
| Excluded rows | 118 | 118 |
| Deferred rows | 1,217 | **1,116** |
| Rust tests in the measured suite | 4,218 | **4,267** |
| New test functions | — | **49** (23 engine/registry, 26 runner) |

The 101 mappings are 94 `Lovelace.Run.Tests`/`Lovelace.Suite.Tests` rows plus 7 adjacent
`Lovelace.Symbolics.Tests` rows the same fixes unblock. The target was 250; **149 rows were not
mapped** because the C# assertion is not met by the current production tree (section 3). No row
was relabelled or mapped to a weaker assertion.

## 2. What was ported

### 2.1 `Lovelace.Run.Tests` (66 rows)

| Pool | Rows | Rust test(s) |
|---|---:|---|
| `IntegerArgumentBoundaryMatrixTests` | 29 | `deferred_boundary_cells_land_on_the_declared_outcome`, `deferred_inventory_templates_reach_the_argument_they_name`, `accepted_evalir_digits_change_the_answer` |
| `SolveSessionAssumptionTests` | 7 | seven tests (roots, equality, domain, one store, family condition, system store) |
| `PluginIntegerArgumentWireTests` | 6 | five tests (integral width, non-integral refusal, bound, wide, batch) |
| `WideArgumentRefusalTests` | 5 | `wide_argument_refusals_are_typed_argument_errors` |
| `EnvelopeDurationTests` | 5 | five envelope tests (multi/single/parse/file/success-vs-error) |
| `SurfacePositionAgreementTests` | 5 | `every_surface_reports_the_same_positions_for_the_same_text`, `file_surface_keeps_the_byte_order_mark_for_a_utf16_file` |
| `AllocationRefusalEnvelopeTests` | 3 | three budget-refusal tests |
| `EvalfDigitCapHonestyTests` | 2 | over-cap and smallest-over-cap markers |
| `SourcePositionTests` | 1 | round-27 `a_leading_byte_order_mark_is_source_text_not_a_parse_error` |
| `LibraryProjectionAgreementTests` | 1 | `a_clamped_value_is_marked_through_the_facade_as_it_is_on_the_cli` |
| `RegistryQueryTests` | 1 | `an_unknown_registry_name_is_the_same_refusal_a_call_produces` |
| `ArityPropertyTests` | 1 | round-27 `eye_accepts_a_column_count` |

### 2.2 `Lovelace.Suite.Tests` (28 rows)

| Pool | Rows | Rust test(s) |
|---|---:|---|
| `TimingTests` / `TimingScaleTests` | 14 | `timing_format_rows`, `timing_scale_picks_the_largest_whole_unit`, `timing_scale_and_format_always_agree` |
| `DescriptorExampleTests` | 4 | `descriptor_examples_that_were_refused_now_execute` (abs, assume_integer, assume_negative, assume_real) |
| `DspPluginTests` | 4 | abs spectrum, non-complex abs, accessors, re/im/conj |
| `NameSuggestionsTests` | 3 | registry test-only `edit_distance_is_the_structural_metric` |
| `ArrayTests` | 1 | `squeeze_of_a_singleton_dimension_is_a_vector` |
| `ArrayAllocationRefusalTests` | 1 | `zeros_shapes_within_the_budget_still_build` |

### 2.3 Adjacent `Lovelace.Symbolics.Tests` (7 rows)

| Pool | Rows | Rust test(s) |
|---|---:|---|
| `EvalirDigitArgumentContractTests` | 5 | `evalir_integral_digit_counts_are_still_honoured`, `two_evalir_places_answer_two_places`, `evalf_digit_count_refusal_is_unchanged` |
| `AbsSymbolicRoundTripTests` | 1 | `abs_of_an_integer_is_still_the_magnitude` |
| `CapabilitiesBuiltinTests` | 1 | `capabilities_returns_a_record_named_capabilities_result` |

### 2.4 What the round-27 fixes actually unblocked

Measured, not assumed: evalir/evalir_batch honour the digit count and refuse non-integral counts;
`eye(rows, cols)`, `mean(a, axis)` and `norm(a, axis)` answer their two-argument forms; the wide
index/order/dimension taxonomy crosses as `InvalidArgument`/`TypeMismatch`; the solve session store
filters roots and domains; a leading BOM and CRLF are mapped into the caller's offsets on all three
surfaces; the evalf 1000-place clamp is marked `truncated`/`digit-cap`/`budget:1000`; and the
runner's one duration selector makes `elapsed` and `elapsedTime` agree on success and failure.

## 3. Refusals (rows left deferred)

202 Run/Suite rows remain deferred: **67** in `Lovelace.Run.Tests` and **135** in
`Lovelace.Suite.Tests`. The pools below were re-probed; each C# assertion still fails on a named
production difference, so each row stays `deferred`.

### 3.1 `Lovelace.Run.Tests` (67)

| Pool | Rows | Why the C# assertion is not met |
|---|---:|---|
| `IntegerArgumentBoundaryMatrixTests` | 19 | `concat(...,2.5)`, `dft(...,2.5)` and `1..2.5` are ACCEPTED (the arguments are floored); `sum`/`prod`/`min`/`max` refuse a Text axis as `InvalidOperation` "Axis must be an integer." without naming the builtin; the corresponding inventory-template cells therefore fail their anchor/prefix assertion. |
| `InspectionExactnessTests` | 11 | `inspect(x).exact` is `true` for `sqrt(2)`, `1/1009` and `evalf(...)`, where C# requires `false`; `inspect` refuses complex and non-symbolic values instead of answering a Null-kind row. |
| `BuiltinArgumentShapeSweepTests` | 7 | `matmul(1,1)`, `symbol(1)`, `assume(1)` and `transpose(1,1)` still cross as `DomainError` prose/arity (not `matmul(): argument 1 …`); `assume_real("y")` is refused; the registry sweep also hits `transpose`'s declared 1..2 versus its 2-argument arity refusal. |
| `SystemBuiltinArgumentShapeTests` | 7 | `solve_system`/`solve_system_full`/`jacobian` refuse wrong shapes with prose ("solve_system needs variable symbols."), not the documented `InvalidArgument` position/expected-shape sentence. |
| `LimitShortFormRecordTests` | 3 | `limit(a/x,x,0)` still reports "coefficient does not evaluate at the point", not "leading coefficient is symbolic". |
| `SolveEmptySetTests` | 3 | `solve(0 == 1, x)`, `solve_full(2 == 3, x)` and `solve_system([0 == 1], …)` still answer "solve needs an equation" because the relation is folded before the kernel; only the folded-Boolean control passes. |
| `ArityPropertyTests` | 2 | the live-registry arity sweep still sees declarations/validators that disagree (e.g. `transpose`); `FunctionDefinition` does not carry `MinArity`/`Variadic` (the registry is a static table). |
| `PlotFileErrorTests` | 2 | a missing plot directory and an empty plot file are accepted/created instead of refused by name. |
| `ArityValidationTests` | 1 | `compile_full(x^2+1)` answers "compile needs an expression and parameters.", not `compile_full(): expected 2 arguments; got 1.` |
| `BuiltinSurfaceContractTests` | 1 | `assume_integer(x); assumptions()` publishes an EMPTY atom list; the C# row requires the DomainCondition atom and its canonical form. |
| `CallSignatureDiagnosticsTests` | 1 | the registry advertises `filter` but the dispatcher answers UnknownFunction, so one of the five probes has no signature. |
| `ErrorCategoryMirrorTests` | 1 | no Rust enum mirror of the C# `ErrorCategory` exists. |
| `EvalfDigitCapHonestyTests` | 1 | `evalf(pi, 1000)` still carries 100 fractional digits, not 1000. |
| `GoldenFixtureRecorder` | 1 | the row is not a test (it records fixtures). |
| `LibraryProjectionAgreementTests` | 1 | the Rust engine has one precision knob, not a separate display bound, so the C# façade row (stored 1100 digits, display 100) cannot be reproduced. |
| `PlotDirectoryErrorTests` | 1 | an empty plot directory is accepted. |
| `RecursionDepthEnvelopeTests` | 1 | nested blocks inside a recursive body are accepted instead of refused by the depth guard. |
| `StdoutPurityTests` | 1 | the row asserts a real-process stdout byte count; a `src` test has no process surface. |
| `StructuredRealPrecisionTests` | 1 | same `evalf(pi, 1000)` 100-digit production difference. |
| `SystemSolveRecordTests` | 1 | `solve_system([sin(x) == 0], [x])` answers "No solver for this system." instead of `Unevaluated`/`Unknown`. |
| `UnknownFunctionEnvelopeTests` | 1 | `--text` writes to stderr in `main`; `run_args` builds only the JSON envelope. |

### 3.2 `Lovelace.Suite.Tests` (135)

| Pool | Rows | Why the C# assertion is not met |
|---|---:|---|
| `TokenTests` | 20 | the C# `TokenKind` ordinals 0..19 do not exist: the active Rust token model is `syntax::Tok` with 9 variants and no discriminants. |
| `RecordSchemaTests` | 9 | no record-schema registry / `RecordSchemas.Validate`; the C# row asserts field names and declared kinds. |
| `ValueTests` | 9 | the active `Value` exposes no public `Widen`/`WidenPair`, and `ToString` does not prefix the kind label. |
| `HelpServiceTests` | 8 | no HelpService surface (overview/signature/see-also text); the registry table has metadata but no renderer. |
| `DescriptorExampleTests` | 7 | `assume_clear`, `scale`, `filter` and `ndims` are advertised but unknown to the dispatcher (or absent), and `cancel_full` is unknown. |
| `DspPluginTests` | 7 | no plugin seam (load-once/collision/unregistered-engine), `filter` is absent, and `dft(..., n)` does not pad to the requested length. |
| `ScriptSourceTests` | 7 | no `ScriptSource.ToSemicolonStatements` preprocessing API; the engine normalises internally but does not publish the normalized text. |
| `SuiteEngineTests` | 7 | no CaptureState/events/SetVariable/RemoveVariable API, and `OpTiming` carries no elapsed duration. |
| `ModusTests` | 6 | no Modus plugin seam (kernel registration, dispatch, scalar results). |
| `PlotTests` | 5 | the C# model/renderer API is not public; the huge-close-X script collapses to one f64 coordinate, and there is no linear-interpolation option. |
| `ModusArityTests` | 4 | no Modus registration/arity surface. |
| `SharedPluginInstanceAssumptionIsolationTests` | 4 | no shared plugin-instance seam. |
| `StructuralEqualityTests` | 4 | `Value` has no `FirstDifference` path/kind reporting API. |
| `ArrayAllocationRefusalTests` | 3 | the C# rows assert the exact `Lovelace.Suite.ArrayAllocationRefusedException` type and its `RequestedElements`/`Limit` properties; Rust publishes a `LangError` code only. |
| `BuiltinArgumentShapeGuardTests` | 3 | no `RegisterBuiltin` API to construct the faulting plugin builtins the rows attribute. |
| `PlotFileWriteRefusalTests` | 3 | no plot-file write-refusal type; the suite accepts/creates the file. |
| `RecordValueTests` | 3 | member-access messages do not name the record type/available members, and the Modus record channel is absent. |
| `StructuredRealExactnessTests` | 3 | no `LReal64` fixed-width tier; `2^-100000` is published `exact:false`; the 18-place truncation display differs. |
| `CancellationObservationTests` | 2 | the Natural power/factorial loops are outside the polling scopes, so a short budget does not cancel promptly. |
| `InputDepthGuardTests` | 2 | no symbolic-construction guard, and the flat chain one past the budget parses before evaluation refuses it. |
| `NameSuggestionsTests` | 2 | the `fft` no-guess row uses a supplied registry (the live registry contains `fft`, so `suggest` is not empty), and the `int.MaxValue` edit-distance sentinel is `usize::MAX` in Rust. |
| `ParserTests` | 2 | chained assignment is a ParseError and unary plus is folded to a Natural (no UnaryExprPlus node). |
| `RecursionBudgetCompositionTests` | 2 | `for`/`while` compositions return `Void` where C# answers the deepest call's value. |
| `SuiteEngineOutputPlotTests` | 2 | no per-call output-writer parameter and no `ResetPlotCapture` API. |
| `ValueDepthGuardTests` | 2 | the structured projection has no value-depth budget. |
| `ArrayTests` | 1 | `CaptureState` is absent. |
| `BroadcastTests` | 1 | `[[1],[2]] + [[10,20],[30,40]]` is refused; C# broadcasts the column vector. |
| `CrossEngineIsolationTests` | 1 | `simplify(x > 0)` returns the symbolic relation, not the Boolean the C# row asserts. |
| `DspIntegerArgumentContractTests` | 1 | the row constructs `ArgumentContractException.NotAnIntegerKind(...)` directly; no such Rust type exists. |
| `EnumPayloadSeamTests` | 1 | no Modus payload seam. |
| `FunctionTests` | 1 | a function-local assignment leaks into the global variables table. |
| `RecursionDepthGuardTests` | 1 | a deep expression inside a recursive body is accepted. |
| `StatementTests` | 1 | a block assignment leaks into the engine variables. |
| `TimingScaleTests` | 1 | `OperationTiming.ElapsedDisplay`/`ElapsedScale` has no Rust equivalent (`OpTiming` has no elapsed field). |

The remaining 501 `Lovelace.Symbolics.Tests` rows are outside this round's target; the seven
adjacent rows above were ported because the round-27 digit/assumption fixes reach them. The
`SeriesKinkDeterminismTests` family was re-probed and refused: `series(abs(x), …)` returns the
input unchanged, so every expected expansion differs.

## 4. Observed verification lines

```text
$ powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1
crates=11 tests=4267 suiteExit=0 passed=4267 failed=0

$ node parity/gen-manifest.mjs
{ "rows": 6209,
  "dispositions": { "excluded": 118, "ported": 4975, "deferred": 1116 },
  "coveragePercent": 80.13, "rustTests": 4267 }

$ node parity/tools/gate-manifest.mjs
PASS (structural): 6209 test cases dispositioned; 4975 claim Rust coverage (80.13%).

$ node parity/tools/coverage-report.mjs
coverage.md written: 4975/6209 ported (80.13%)

$ cargo run --release -p lovelace-corpus
corpus: 212 passed, 2 failed, 214 total   (sym/071, sym/076 — unchanged; exit 1 is those two)

$ node parity/tools/gate-fixtures.mjs
oracle: 20/20
rust:   20/20
PASS: oracle 20/20, rust 20/20

$ node parity/tools/gate-cli.mjs
CLI matrix: 16/16
PASS: every RUN-001..RUN-014 row behaves like the oracle
```

## 5. What could not be verified

- **The 250 target.** 101 net new mappings are verified; 149 are short. The remaining Run/Suite rows
  are not deferred for lack of tests but because the C# assertion is contradicted by a concrete
  production behaviour (section 3); mapping them would have required a weaker assertion, which the
  contract forbids.
- **C# test execution.** The harness cannot run `dotnet test` in this environment (CAP-001); the
  contract's observables were probed against the pinned runner and the Rust tests repeat the
  assertions directly.
- **The two pre-existing corpus failures** (`sym/071`, `sym/076`) are unchanged and outside this
  round's scope.
- **Process-level rows** (`StdoutPurityTests`, the process-boundary allocation/plot rows) cannot be
  reproduced from a `src` unit test because `run_args` builds the envelope in-process; they stay
  deferred.
- **Token/Value/ScriptSource rows** need production surfaces the active crate does not expose
  (the stale `token.rs`/`value.rs` files are not compiled into the crate).