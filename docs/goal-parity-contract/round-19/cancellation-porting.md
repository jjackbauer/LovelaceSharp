# Round 19 — porting the budget and cancellation rows

Scope: test code only. New files `rust/lovelace-run/src/round19_depth_batch.rs`,
`rust/lovelace-run/src/round19_run_batch.rs` and `rust/lovelace-lang/src/round19_batch.rs`;
`#[cfg(test)] mod` registrations in `main.rs`/`lib.rs`; `parity/manifest-map.json`. One
test-only helper change in `parity_batch::run_args` now mirrors `main`'s
`PlotDirectoryError` check. No production line was changed: every added or edited Rust item
compiles only under `#[cfg(test)]` except the map and this report.

## 1. Result

| Measure | Before | After |
|---|---:|---:|
| Manifest coverage | 4,336 / 6,209 (69.83 %) | **4,562 / 6,209 (73.47 %)** |
| Net new verified mappings | — | **+226** |
| Curated rows (map entries written) | — | 262 |
| Deferred rows | 1,755 | 1,529 |
| Excluded rows | 118 | 118 |
| Rust tests in the measured suite | 3,789 | 3,924 |
| New test functions | — | 135 (86 runner, 49 engine) |

The 262 curated rows cover 163 C# methods. Thirty-six of the curated rows already had a
disposition when the map was re-read, so the measured coverage delta is 226; those rows keep a
test that repeats the same C# assertion (the previous mapping was replaced with the
equivalent/stronger round-19 test). The map was re-read immediately before each write, as the
single-writer rule requires.

## 2. What was ported

### 2.1 Runner envelope rows (`Lovelace.Run.Tests`, 162 rows)

| Pool | Rows | New Rust tests |
|---|---:|---|
| `DepthGuardEnvelopeTests` | 16 | 9 |
| `ValueDepthEnvelopeTests` | 16 | 7 |
| `RecursionDepthEnvelopeTests` (14 of 15) | 14 | 4 |
| `RecursionBudgetCompositionEnvelopeTests` | 4 | 4 |
| `CancellationBudgetTests` + `PrintOutputOnFailureTests.TheCancelledPath` | 7 | existing round-18 tests |
| `EnginePrecisionDigitRefusalTests` | 16 | 7 |
| `BuiltinSurfaceContractTests` (evalf honours/refuses, capabilities) | 12 | 3 |
| `LibraryProjectionAgreementTests` | 13 | 5 |
| `InspectionExactnessTests` | 6 | 2 |
| `SourcePositionTests` | 4 | 4 |
| `CallSignatureDiagnosticsTests` | 3 | 3 |
| `UnknownFunctionEnvelopeTests` | 3 | 3 |
| `ArityValidationTests` | 4 | 2 |
| `SystemBuiltinArgumentShapeTests` (declared forms) | 3 | 1 |
| `WideArgumentRefusalTests` | 5 | 5 |
| `DivRemRecordTests` | 5 | 3 |
| `ParseErrorClassificationTests` | 5 | 5 |
| `PlotDirectoryErrorTests` + `PlotFileErrorTests` | 4 | 4 |
| `CommittedStateTests` | 1 | 1 |
| `StructuredRealPrecisionTests` | 3 | 2 |
| `EvalfDigitCapHonestyTests` | 2 | 2 |
| `IntegerArgumentBoundaryMatrixTests` | 7 | 2 |
| `PrintBudgetTokenBoundaryTests` + `PrintBudgetTotalityTests` | 3 | 3 |
| `FunctionArityMetadataTests`, `ArityPropertyTests` (e()), `BuiltinArgumentShapeSweepTests` (det) | 3 | 3 |
| `SolveSessionAssumptionTests` (controls) | 3 | 3 |

Every runner test drives the same `execute`/`envelope_for` pair `main` uses, through
`parity_batch::run_args`, so a wire assertion cannot pass on a flow the binary does not run.
Recursive scripts run on a 64 MiB test thread, matching the production thread's stack.

### 2.2 Engine rows (`Lovelace.Suite.Tests`, 72 rows)

| Pool | Rows | New Rust tests |
|---|---:|---|
| `InputDepthGuardTests` | 5 | 5 |
| `RecursionDepthGuardTests` | 5 | 5 |
| `RecursionBudgetCompositionTests` | 31 | 3 |
| `ValueDepthGuardTests` | 6 | 6 |
| `ArrayKernelCancellationTests` | 6 | existing round-18 tests |
| `NameSuggestionsTests` (nothing-close / dropped synonym) | 4 | 2 |
| `RecordValueTests` | 3 | 3 |
| `PlotTests` | 8 | 8 |
| `StructuralEqualityTests.IdenticalRecords_AreEqual` | 1 | 1 |
| `EnumPayloadSeamTests.Value_API_CarriesTheEnumKind` | 1 | 1 |
| `SuiteEngineTests` (timings, error position) | 2 | 2 |

The engine tests arm the same thread-local deadline the runner arms
(`Engine::set_cancel_after`) and assert the reference's verdict and the C# wall-clock fence.

### 2.3 Round-8 refusals that now reproduce

The round-8 refusal table was re-checked. Twenty-eight rows now satisfy the C# assertion and were
ported: empty reductions (`sum([])`=0, `prod([])`=1), both step-slice rows, `len(1..0)`=0,
the five typed-array promotion/narrowing rows, substitution of an exact rational, the
`evalf(1/3,50)` payload, the sqrt/setprecision/pi arity sentences, Integer and negative
factorial, `divrem(-7,-3)`, `100/5`, the wide setprecision count, and the seven
`TruncatedIrrational_IsNotCertifiedExact` rows. Twenty-seven of these already had a ported
disposition from a later round (the round-19 test replaces that mapping and repeats the same
assertion), so they add one net row; they are still listed because the round-8 table named them.

## 3. Observed verification lines

```text
$ powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1
crates=11 tests=3924 suiteExit=0 passed=3924 failed=0

$ node parity/gen-manifest.mjs
{ "rows": 6209,
  "dispositions": { "excluded": 118, "ported": 4562, "deferred": 1529 },
  "coveragePercent": 73.47, "rustTests": 3924 }

$ node parity/tools/gate-manifest.mjs
PASS (structural): 6209 test cases dispositioned; 4562 claim Rust coverage (73.47%).

$ node parity/tools/coverage-report.mjs
coverage.md written: 4562/6209 ported (73.47%)

$ node parity/tools/gate-fixtures.mjs
oracle: 20/20
rust:   20/20
PASS: oracle 20/20, rust 20/20

$ cargo run --release -p lovelace-corpus
corpus: 212 passed, 2 failed, 214 total   (sym/071, sym/076 — unchanged)
```

## 4. Refusals (rows left deferred)

The target was 250 new mappings; **226 net** were verified (24 short). The rows below were
examined and refused because the Rust assertion would have been weaker than the C# one, or the
capability is still a production gap. All remain `deferred`; none was relabelled.

| Pool | Rows | Why the C# assertion is not met |
|---|---:|---|
| `IntegerArgumentBoundaryMatrixTests` | 86 | DSP integer arguments (`movingavg`, `delay`, `impulse`, `step`, `exponential`, `cosine`, `powerseries`, `noise`) are unknown to the dispatcher; `mean`/`norm` reject 2 arguments; `eye(rows, cols)` rejects 2 arguments; `evalir`/`evalir_batch` ignore the digit count; `zeros(-1)` is accepted; `series(...,-1)` is accepted; the index overflow crosses as `InvalidOperation` without the named value. |
| `PluginIntegerArgumentWireTests` | 13 | `evalir` ignores 2.9/0/-5/10^30 digit counts; the DSP builtins are absent. |
| `InspectionExactnessTests` | 11 | `inspect(sqrt(2)).exact` is true where C# says false; complex values are rejected by `inspect`; the Null-kind rows are rejected; the truncated quotient is reported exact. |
| `SystemBuiltinArgumentShapeTests` | 7 | wrong-shape system calls still cross as `InvalidOperation/DomainError` with prose, not `InvalidArgument/TypeMismatch` naming position/expected shape. |
| `WideArgumentRefusalTests` | 5 | `zeros(-1)`/`zeros(2,-1)` are accepted; the wide index crosses without the count and Int64 bound; `series(...,-1)` is accepted; the wide series order crosses as `InvalidOperation`. |
| `SolveSessionAssumptionTests` | 7 | the session store is not consulted by `solve`; `assume_real`/`assume_integer` are unknown to the dispatcher. |
| `ArityPropertyTests` | 5 | the live registry's `eye`/`mean`/`norm` arity messages violate the documented grammar; the plugin short forms (`noise`) are absent. |
| `BuiltinArgumentShapeSweepTests` | 7 | `matmul(1,1)`, `symbol(1)`, `assume(1)` and `transpose(1,1)` do not cross as the documented argument error; the two whole-registry sweeps collect grammar violations. |
| `EnvelopeDurationTests` | 5 | `elapsed` is always rendered in ms while `elapsedTime.unit` selects µs/ms from one scale; for sub-millisecond failures the two surfaces disagree, which the C# assertion forbids. |
| `SurfacePositionAgreementTests` | 5 | the runner's `--eval` BOM is tokenized as an unexpected character; `--file` does not decode UTF-16 in the in-process helper; `--stdin` is not reachable from a `src` unit test. |
| `LimitShortFormRecordTests` | 3 | `limit(a/x, x, 0)` reports "coefficient does not evaluate at the point", not the documented "leading coefficient is symbolic". |
| `SolveEmptySetTests` | 4 | `0 == 1` is folded to a Boolean before the kernel; `solve` still answers "solve needs an equation". |
| `AllocationRefusalEnvelopeTests` (Run) / `ArrayAllocationRefusalTests` (Suite) | 7 | no allocation budget/refusal type exists; the multi-GB request would be attempted. |
| `PlotDirectoryErrorTests.EmptyPlotDirectory`, `PlotFileErrorTests` (2) | 3 | an empty plot directory is accepted; a plot file under a missing directory is created instead of refused; the empty-name refusal does not name the path. |
| `SourcePositionTests.LeadingBom` | 1 | the BOM is a parse error rather than an offset-1 position. |
| `UnknownFunctionEnvelopeTests.TheTextSurface` | 1 | `--text` writes to stderr inside `main`; `run_args` builds the JSON envelope only. |
| `CallSignatureDiagnosticsTests.EveryArityRefusalAgreesWithTheRegistryEntry` | 1 | the registry exposes `filter` but the dispatcher answers UnknownFunction, so one of the five probes has no signature. |
| `ArityValidationTests.TooFewArguments` | 1 | `compile_full(x^2+1)` answers "compile needs an expression and parameters.", not the documented `compile_full(): expected 2 arguments; got 1.` |
| `StructuredRealPrecisionTests.EvalfCountAboveTheAmbientDisplayPrecision` | 1 | `evalf(pi, 1000)` carries 100 digits. |
| `EvalfDigitCapHonestyTests` (3) | 3 | the 1000-place digit cap drops digits without `truncated`/`truncationReason`/`budget`; `evalf(pi,1000)` carries 100 digits. |
| `LibraryProjectionAgreementTests` (2) | 2 | no separate display bound to prove the facade keeps the computation digits; the digit-cap truncation fields are absent. |
| `BuiltinSurfaceContractTests` (2) | 2 | `evalf(sin(1), 1)` refuses ("evalf needs a number"); `assumptions()` publishes the atom as Text, not a Symbolic carrier. |
| `ErrorCategoryMirrorTests`, `StdoutPurityTests`, `GoldenFixtureRecorder` | 3 | no Rust enum mirror, no process surface from a `src` test, and the recorder is not a test. |
| Suite depth/model rows | 6 | `SymbolicConstructionOnePastTheBudget` (no construction guard), `FlatChainOnePastTheBudget` (parse succeeds; eval refuses), `ADeepExpressionInsideTheRecursiveBody` (Rust accepts), the two `for`/`while` composition values (Rust `Void`), `ValueOnePastTheBudget_IsRefusedByTheStructuredProjection`, `ADeepRecordChain`. |
| Suite other rows | 6 | Modus plugin/record registration (2), unknown/non-record member messages (2), `CaptureState`/events (2). |
| `CancellationObservationTests` (2), `ArrayAllocationRefusalTests` (4) | 6 | the Natural power/factorial loops are outside the polling scopes; no allocation budget. |

## 5. What could not be verified

- The 250 target: 226 net new mappings are verified; the remaining 24 are in the refusal table
  above and need production work (DSP dispatch/arity, digit-count plumbing, allocation budget,
  session assumptions, BOM/UTF-16 surfaces, digit-cap truncation markers). They were not mapped
  to weaker tests.
- The full suite, corpus and fixtures were run by the orchestrator-side commands above; the
  corpus still fails the two pre-existing rows `sym/071` and `sym/076`, unchanged.
- C#-only assertions with no Rust analogue were not mapped: CLR exception type/message equality,
  `ValueKind` enum ordinals, `StructuralEquality.FirstDifference` path strings, Modus payload
  identity, and the process stdout byte count.