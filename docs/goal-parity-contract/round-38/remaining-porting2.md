# Round 38 — the remaining deferred rows (this lane's porting pass)

Status: complete for the rows below. 50 of the 520 deferred rows are mapped to a passing Rust test:
the 30-row series-at-kink family and 11 kernel-hardening rows the concurrent production lane
unblocked (its ids are in docs/goal-parity-contract/round-38/series-kink-hardening.md section 3),
plus 9 rows this lane probed, ported and mapped. The remaining 470 stay deferred with a named,
measured reason (section 3).

## 0. Population and result

| Measure | Rows |
|---|---:|
| Deferred census rows at round start | 520 |
| (a) mapped to a Rust test in this round | 50 |
|   — series-at-kink family (production lane) | 30 |
|   — kernel-hardening family (production lane, 11 of 22) | 11 |
|   — this lane's own probe-and-port pass | 9 |
| (b) re-probed and refused (measured divergence) | see section 3 |

The production lane's own report is the evidence for the 41 family rows; this lane verified that
each of those census ids was still deferred, that the named Rust test exists in the measured
inventory, and mapped it — nothing was mapped on assertion alone. Every (a) row resolves to a test
that passes in the measured suite (parity/tools/gate-manifest.mjs: failures []).

## 1. The family rows (41)

`docs/goal-parity-contract/round-38/series-kink-hardening.md` section 3 is the id-to-test table.
This lane applied it row by row: 8 + 8 + 7 + 4 + 1 + 2 = 30 `SeriesKinkDeterminismTests` ids to
the six `lovelace-lang::round38_series_kink_hardening` tests it names
(`kink_series_publishes_the_sympy_expansion_byte_identically_on_every_run`,
`kink_series_publishes_no_internal_symbol_and_no_unevaluated_derivative`,
`kink_series_takes_the_sympy_value_at_the_sample_point`,
`kink_series_neighbouring_shapes_follow_the_same_rule`,
`kink_series_truncation_gap_is_the_dropped_term_not_a_silent_approximation`,
`limit_engine_two_sided_kink_answers_are_unchanged`), and 3 + 8 = 11 `KernelHardeningTests`
ids to the nine it names (`cubic_roots_satisfy_polynomial_numerically`,
`rational_power_huge_exponent_is_exact`, `mathir_huge_exponent_lowers_and_evaluates`,
`numeric_to_symbolic_to_numeric_500_digits_round_trips`,
`real_constant_add_preserves_500_digits`, `real_constant_multiply_preserves_precision`,
`registered_function_arity_is_enforced`, `simplify_zero_over_x_cancels_with_condition`,
`solve_quartic_no_real_roots_default_domain_unevaluated_real_domain_empty`).

The rows the production lane refused stay deferred and are NOT claimed here (its section 4).


## 2. This lane's 9 rows

All nine tests live in `rust/lovelace-lang/src/round38_remaining.rs` (module
`lovelace-lang::round38_remaining`). The C# assertion is the contract; each row below states the
same value, record shape or refusal through the Rust surface.

| Census row | Rust test | Translation |
|---|---|---|
| `AssumptionAddScalingTests/Add_MixedAtomsInAnyInsertionOrder_YieldsTheSameCanonicalOrder#0` | `assumption_add_mixed_atoms_in_any_insertion_order_yields_the_same_canonical_order` | The reference's eight-atom Mixed() pool is built with the same five atom shapes (domain, property, relation, expression-property, interval); the stored order is ordinal by each atom's record rendering, asserted for the forward and reversed insertion order. Rust publishes that rendering as `Assumption::order_key`, the generated `ToString` the reference sorts on. |
| `AssumptionAddScalingTests/Add_OrderEqualsHistoricalAppendAndResort_ForEveryInsertionOrder#0` | `assumption_add_order_equals_historical_append_and_resort_for_every_insertion_order` | The incremental set must store exactly the sequence append-and-ordinal-resort produces. The reference draws fifty orders from `System.Random(20250825)`; Rust has no `System.Random`, so fifty deterministic orders are used with the same per-order assertion (the quantified claim is universal). |
| `SolveCompletenessPairingTests/SystemSolve_NoSolutions_IsACompleteAnswer#0` | `system_solve_no_solutions_is_a_complete_answer` | `solve_system_full([x + y == 1, x + y == 2], [x, y])` publishes `SystemSolveResult(status: NoSolutions, complete: True)` — an inconsistent system is a provably empty set, i.e. a complete answer. |
| `DiagnosticContractTests/UnsatisfiableTransformation_CarriesADomainErrorDiagnostic#0` | `unsatisfiable_transformation_carries_a_domain_error_diagnostic` | `assume(x == 0); simplify_full(x/x)` is `TransformResult(status: Unsatisfiable)` with `Diagnostic(code: transform.unsatisfiable-conditions, category: DomainError)`, not prose and not an empty array. |
| `UnsatisfiablePropagationTests/SimplifyFull_UnsatisfiableConditions_SerializesAsAnExplicitMarker#0` | `simplify_full_unsatisfiable_conditions_serialize_as_an_explicit_marker` | The conditions field (index 4) publishes one `UnsatisfiableConditions()` record with no fields — a structured marker, never the string "unsatisfiable". |
| `DebugEvalir/Lower_Evaluate_FiveTerm#0` | `evalir_lowers_optimizes_and_evaluates_a_five_term_polynomial` | The five-term polynomial is lowered, optimized, serialized and deserialized by `compile` and evaluated by `evalir` at x = 2 to 92 — the same number after the same round trip. |
| `PolynomialsApiTests/FromExpr_RoundTrip_DegreeCoefficientTerms#0` | `polynomial_coefficients_round_trip_degree_terms_and_coefficients` | `poly_coeffs` is the Rust polynomial carrier: degree 2, coefficients 1/2/1 at degrees 2/1/0, zero above the degree, three terms. |
| `PolynomialsApiTests/Divide_X3Minus1_By_XMinus1#0` | `polynomial_division_of_x_cubed_minus_one_by_x_minus_one` | Quotient x^2 + x + 1; the zero remainder is asserted by reconstructing numerator = quotient x divisor exactly (the same statement as r == 0, since `poly_div` publishes only the quotient). |
| `UsageDocumentationTests/DocumentedExample_MatchesEngine#75` | `documented_linsolve_example_matches_the_engine` | The usage guide's documented `linsolve([[x, 1], [0, y]], [0, 1])` rendering is now the engine's own: `[-x/x/(x*y), x/(x*y)] (Vector)`. The round-26 refusal measured a different canonical shape and no longer holds. |


## 3. Re-probed and refused (this lane's measurements)

These rows stay deferred with the reason below; none was approximated. Measured on the settled tree
with both runners on the same `--eval` argv, or against the live kernel API.

### 3a. The reference surface is not compiled in this crate

`rust/lovelace-lang/src/token.rs`, `parser.rs`, `ast.rs`, `value.rs` and `interp.rs` are
NOT modules of the crate (`lib.rs` declares `builtins`, `help`, `complex`,
`complex_math`, `numeric`, `registry`, `structured`, `syntax`). The live token/AST model
is `syntax.rs`, whose types differ:

| Candidate | Rows | Measurement |
|---|---:|---|
| `TokenTests/TokenKind_GivenAllValues_HasExpectedOrdinal#0..#19` | 20 | No `TokenKind` in the live crate. `syntax::Tok` has 8 variants (Num, Repr, Str, Interp, Ident, Punct, Semi, Eof), so the reference's declared ordinals cannot be asserted. Nineteen of twenty coincided with the dead file's enum; that enum is not the compiled surface. |
| `ScriptSourceTests/ToSemicolonStatements_*` | 7 | `normalize_source` lives only in the dead `token.rs`. The live `syntax::tokenize` treats a newline as a separator but exposes no string transform, so the row's exact-output assertion cannot be made. |
| `ParserTests/Parse_GivenUnaryPlus_ReturnsUnaryExprPlus` | 1 | The live `syntax::Expr` has `Neg`, `Not`, `Fact` and no unary-plus node (`+5` folds to `Number("5")`), so the asserted AST node does not exist. |
| `ParserTests/Parse_GivenChainedAssignment_RightAssociative` | 1 | Both runners' language layer refuse `x = y = 5` (`ParseError`), while the reference's parser accepts it. |

### 3b. Kernel-shape divergences (measured today)

| Family | Rows | Measured behaviour |
|---|---:|---|
| `IndeterminateExponentialLimitTests` | 8 | `limit((1 + 1/x)^x, x, inf)` answers `e`, `e^2`, `e` in the oracle and `Unevaluated` (`coefficient does not evaluate at the point`) in Rust; the wire-value rows fail with them. |
| `AcceptanceTests/Integrate_Table_AllVerify`, `DebugIntegrate` | 2 | `integrate_full(sin(x), x)`, `exp(3*x)`, `1/(x^2 - 1)` and `x*exp(x)` answer `Unevaluated` in Rust; `integrate_full(3*x^2 + 2*x + 1, x)` answers `x^3 + x` (verified: False) where the oracle answers `x^3 + x^2 + x`. |
| Solver radical shapes (`Solve_QuarticWithRealRoots`, `solve(x^4 + 1)`, `(x+1)^3 == 8`) | 3 | `solve(x^4 - 5*x^2 == 0, x)` publishes three `rootof` solutions with `Partial` where the oracle publishes `±1/2*sqrt(20)` and 0 with `Solved`; `solve_full(x^4 + 1 == 0, x)` is `Partial`, not the oracle's `Unevaluated`; `(x+1)^3 == 8` returns a different closed form. |
| Abs family (`Diff_Of_Abs_Symbolic`, `Subs_Abs`, `Simplify_Sqrt_Square_Under_Real`, `Transform_*`) | 4 | `diff(abs(x), x)` answers `0` where the oracle answers `piecewise(sign(x) if x != 0, diff(abs(x), x))`; `subs(abs(x), x, -3)` answers `abs(-3)` where the oracle answers `3`; `simplify_full(sqrt(x^2))` under `x real` is unchanged (`changed: False`) where the oracle rewrites to `abs(x)` with a `DomainCondition`. |
| `DxStructuredResultsTests/EvalirBatch_*` | 4 | `evalir` refuses a second free symbol: `evalir supports one free symbol.` for every two-parameter kernel. |
| `InvFull_ExposesDetCondition`, `LinsolveFull_SingularSystem`, `ParameterDomainOf_MapsEveryDeclaredArm` | 3 | `inv_full([[x, 0], [0, y]])` publishes `conditions: []` where the oracle publishes `[x*y != 0]`; `linsolve_full` nests its solutions as one row where the oracle flattens them; `linsolve` on a singular system returns `[]` where the oracle throws; `ParameterDomainOf` has no Rust surface. |
| `MatrixAdvanceTests/Rank_GenericOverSymbols_And_Numeric` | 1 | The two symbolic ranks match (2, 1), but `matrix_rank([[1, 2], [2, 4]])` is refused by the oracle ("requires a symbolic matrix") and answered 1 by Rust, so the numeric half cannot be asserted. |
| `Hyperbolic.../ZeroToTheMinusOne`, `BuiltinCount`, `HaveCompleteDescriptors#0..#2` | 5 | `0^-1` is `DivisionByZero/DomainError` "Cannot divide by zero." in Rust and `InvalidArgument/TypeMismatch` "Base cannot be zero. (Parameter 'base')" in the oracle; the builtin count is 123 in Rust and 107 in the reference; the descriptor rows need help entries for asinh/acosh/atanh, absent from `help.rs`. |
| `CapabilitiesBuiltinTests/PowerExponentClass...` | 1 | `0^(1/2)` must be supported with display `0`; Rust refuses it with `UnsupportedOperation` "Non-integer exponents are not yet supported." |
| `CapabilitiesBuiltinTests/ExactnessBestEffort...` | 1 | Two probes must be refused: `max(1,2)` answers 1 and `x = symbol("x"); sum(x, 5)` answers a Symbolic value in Rust. |
| `Capabilities.../Capabilities_IsOneRegisteredBuiltin`, `Help_Overview`, `Value_API_CarriesTheEnumKind` | 3 | These assert `PluginName`, `CaptureState`, `InterpreterBuiltinDescriptors`, the C# `ValueKind` ordinals and help-overview text ("help <function>"); none exists on the Rust surface. |
| `CompilationTests` (8), `MathIrTypingTests` (7), `PropertyTests` (7) | 22 | Multi-parameter kernels, `IrProgram.Deserialize`, `Validate`/`Infer` typing and the `System.Random` corpora have no Rust surface; `evalir` also answers `0.75` (Real) where the reference keeps `3/4`. |
| `ConcurrencyTests` (6), `AssumptionScopeIsolationTests` (2), `RealAsyncLocalTests` (4) | 12 | `ExprContext` id maps, thread-scoped assumption scopes and `AsyncLocal` precision scopes do not exist; assumptions are values. |
| `MetamorphicSetClosureTests` (9), `Falsification*` (32) | 41 | The rewrite registry, its falsification gate and the loop's sample-region data are C#-side: `cancel((x^2+1)/(x-1))` answers `x + 1` in Rust and the unchanged quotient in the oracle, `cancel_full` has no dispatcher arm, and no Rust API exposes rule preconditions or a falsification harness. |
| `Dsp` (16), `Plot`/`PlotFile*` (10), `Modus`/`DspPlugin`/`DescriptorExample` (16) | 42 | Fixed-point `Cplx64/128`, `DspMath.Filter/ImpulseResponse`, the plot renderer model, the Modus seam and the descriptor surface are absent. `fft([])` refuses where the oracle returns empty, `fft([1,1,1])` answers `[3, 0, 0]` where the oracle refuses, and `plot(sin(x))` succeeds under a writable directory instead of the typed refusal. |
| `Natural` (40), `Integer` (9), `Real` residual (47) | 96 | CLR identity, `System.Random` corpora, cached-string reference identity, global `DisplayDigits`/`Precision`/`DisplayDecimalPlaces` scopes, `f64`/`decimal` constructors, `TryFormat`/`Span` overloads, `Real.Exponent`/`PeriodStart`/`PeriodLength` and the `PiSegment`/batch/async entry points have no Rust counterpart. |


### 3c. Rows whose own script now matches but whose deferred id is a different row

| Candidate | Rows | Measurement |
|---|---:|---|
| `RecursionDepthEnvelopeTests/ADeepRecursion...` | 1 | Already ported. Its deferred sibling `NestedBlocksInsideARecursiveBody_AreRefused_InsteadOfDying#0` is REFUSED: `func f(n) { ... 120 x if (1 == 1) { ... return f(n-1) ... } }; f(5)` answers `0` in Rust where the oracle refuses `DepthExceeded`. |
| `EvalfDigitCapHonestyTests/ARequestBeyondTheCap_IsMarkedAsClamped` | 1 | Already ported. Its deferred sibling `ARequestAtTheCap_IsNotMarked#0` is REFUSED: `evalf(pi, 1000)` publishes a structured value with 100 fractional digits (the display bound), where the reference asserts the cap's 1000. |
| `UnsatisfiablePropagationTests/ConditionPayload_IsStructuredForEveryReachableAtomKind` | 1 | Already ported (round 34). |
| `AssumptionSoundnessTests/PropertyAndRelationAtoms_Contradict` | 1 | REFUSED: `AssumptionSet::add(SymbolProperty(x, NonZero))` after `SymbolRelation(x, Eq, 0)` is ACCEPTED (the reverse order is caught), so the row's "both insertion orders" assertion fails. |
| `ExactnessFlagTests/CachedFlag_EqualsTheLeafRule_OverTheWholeCorpus`, `RadicalsOfExactBases...` | 2 | The corpus needs `I`, complex constants, `RootOf` and a derivative node the Rust `Expr` does not have; the radical row pins the canonicalisation `x^1.5 == x^(3/2)`, which `Expr::pow` does not perform. |
| `PolynomialsApiTests/Content/Gcd/SquareFree/Factor/Resultant/Discriminant` | 6 | No content/primitive-part, gcd/lcm, square-free, resultant or discriminant API in `lovelace-symbolics`. |
| `UsageDocumentationTests/DocumentedExample#61,#70,#71,#73,#99` | 5 | The census display name truncates the script and expected result, so the row cannot be tied to a guide block without guessing. |

## 4. Verification

Run on the settled tree (production sources last edited 13:53; this lane's module 13:57). Every line
below is the observed output.

| Gate | Command | Observed result |
|---|---|---|
| Suite / inventory | `powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1` | `crates=11 tests=4717 suiteExit=0 passed=4717 failed=0` |
| Manifest | `node parity/gen-manifest.mjs` | `rows 6209: ported 5621, deferred 470, excluded 118; coverage 90.53 %` |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `PASS (structural): 6209 test cases dispositioned; 5621 claim Rust coverage (90.53 %).` failures: `[]` |
| Coverage | `node parity/tools/coverage-report.mjs` | `coverage.md written: 5621/6209 ported (90.53 %)` |
| Corpus | `rust/target/release/lovelace-corpus.exe` | `corpus: 214 passed, 0 failed, 214 total` |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `PASS: oracle 20/20, rust 20/20` |

Per-project movement (from parity/coverage.md): `Lovelace.Symbolics.Tests` 961 -> 1,011
(78.38 % -> 82.46 %), deferred 265 -> 215; every other project unchanged; TOTAL 5,571 -> 5,621
(89.72 % -> 90.53 %), deferred 520 -> 470, excluded 118 unchanged.

The one flake observed: `assumptions_round35::eight_hundred_atoms_stay_ordered_and_fast` (a
wall-clock tripwire, 2000 ms debug ceiling) failed once under full parallel load and passed on the
rerun (0.78 s alone); it is a pre-existing round-35 test, not this round's.
