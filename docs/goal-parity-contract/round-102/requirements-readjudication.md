# Round 102 — requirement-ledger re-adjudication

Date: 2026-11-12 (America/Sao_Paulo) · Repository HEAD: 112041b · Scope: `parity/requirements.json`
plus this report. The Rust tree, tests, fixtures, C# sources, and every other parity file were left
untouched.

## What was done

Every one of the 262 rows was re-read against evidence that exists in the current tree. A row was
moved to `met` only when a named Rust test, a named gate command with its exit code, or a named file
with the expected content proves the requirement. A row with partial evidence and a named remaining
gap is `partial`; a row with no supporting capability is `unmet`. No row was marked met on the
strength of a plan, a percentage, or an agent's claim.

The evidence was collected with these commands (all run in this round unless stated otherwise):

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo test --workspace` | exit 0 — 5 666 passed, 0 failed, 1 ignored |
| `cargo test --workspace --release` | exit 0 |
| `node parity/tools/gate-census.mjs` | exit 0 — 2 105 methods / 6 209 cases / 15 projects |
| `node parity/tools/gate-manifest.mjs` | exit 0 at 21:38 — 6 209/6 209 dispositioned (6 090 ported + 119 user-approved exclusions, 0 deferred); later red on a stale inventory (see caveats) |
| `node parity/tools/gate-projects.mjs` | exit 0 — 19/19 projects (4 port, 1 consolidated, 8 partial, 6 absent) |
| `node parity/tools/gate-requirements.mjs` | exit 0 after this edit — 262 rows, no met row without evidence |
| `node parity/tools/gate-fixtures.mjs` | exit 0 — oracle 20/20, rust 20/20 |
| `node parity/tools/gate-cli.mjs` | exit 0 — CLI matrix 16/16 |
| `parity/diff/run.mjs --cases 5000` | exit 0 (recorded in `parity/scorecard.json`, D4) |
| `cargo run --release -p lovelace-corpus` | exit 0 — corpus 214/214 (recorded in the scorecard, D9) |
| `powershell -File parity/mutate.ps1` | 9/12 detected in one run, 3 preconditions contaminated; GATE-001 and GATE-010 re-ran individually at 1/1 (see caveats) |
| `lake build` in `Lovelace.Proofs` | exit 0 — “Build completed successfully (9 jobs)” (Lean 4.33.1) |

## Result

| Status | Before | After | Change |
|---|---:|---:|---:|
| met | 48 | 180 | +132 |
| partial | 28 | 52 | +24 |
| unmet | 186 | 30 | -156 |
| **total** | **262** | **262** | 0 |

**132 rows moved to met; 0 rows were downgraded; the 48 previously met rows remain met.** The
`totals.byStatus` block was recomputed; `id`, `family`, `title`, `sourceLine`, `owner` and `phase`
were not touched (verified against `git show HEAD:parity/requirements.json`).

## Per-family counts

Before and after are `met / partial / unmet`.

| Family | Before M | Before P | Before U | After M | After P | After U | Δ met |
|---|---:|---:|---:|---:|---:|---:|---:|
| ARRAY | 0 | 0 | 10 | 10 | 0 | 0 | +10 |
| CLI | 0 | 0 | 7 | 0 | 0 | 7 | 0 |
| CPLX | 0 | 2 | 4 | 6 | 0 | 0 | +6 |
| DESK | 0 | 0 | 28 | 20 | 6 | 2 | +20 |
| DIFF | 9 | 0 | 1 | 9 | 1 | 0 | 0 |
| DSP | 0 | 1 | 9 | 8 | 2 | 0 | +8 |
| ECO | 0 | 2 | 6 | 1 | 3 | 4 | +1 |
| ENG | 2 | 4 | 6 | 9 | 3 | 0 | +7 |
| GATE | 0 | 1 | 11 | 11 | 1 | 0 | +11 |
| HARNESS | 3 | 4 | 5 | 7 | 5 | 0 | +4 |
| KNOW | 0 | 0 | 10 | 0 | 0 | 10 | 0 |
| LANG | 0 | 1 | 6 | 5 | 2 | 0 | +5 |
| MIR | 0 | 0 | 8 | 7 | 1 | 0 | +7 |
| NUM | 0 | 0 | 10 | 8 | 2 | 0 | +8 |
| PERF | 0 | 0 | 10 | 3 | 7 | 0 | +3 |
| PLUG | 1 | 1 | 6 | 7 | 1 | 0 | +6 |
| PROOF | 0 | 0 | 4 | 1 | 0 | 3 | +1 |
| REAL | 1 | 0 | 9 | 9 | 1 | 0 | +8 |
| REQ | 2 | 2 | 6 | 4 | 5 | 1 | +2 |
| RUN | 13 | 1 | 0 | 13 | 1 | 0 | 0 |
| RUST | 0 | 2 | 8 | 4 | 4 | 2 | +4 |
| STAT | 0 | 0 | 4 | 3 | 1 | 0 | +3 |
| SYM | 0 | 0 | 14 | 14 | 0 | 0 | +14 |
| TEST | 5 | 4 | 4 | 9 | 3 | 1 | +4 |
| WIRE | 12 | 3 | 0 | 12 | 3 | 0 | 0 |

## Rows moved to met, with citation

Each entry gives the first concrete citation in the row's `evidence` array; the array itself may
carry more.

### ARRAY — 10 moved to met

- **ARRAY-001** Preserve scalar, vector, matrix, and higher-rank array distinctions.
  - `cargo test -p lovelace-array` (61 tests, exit 0)
- **ARRAY-002** Preserve shape, rank, strides, and element ordering.
  - `lovelace-array::tests::FromVec_GivenCubeShapeAndEightValues_ExposesShapeRankNumelStrides`
- **ARRAY-003** Match indexing, negative/invalid indices, partial indexing, and slicing.
  - `lovelace-array::tests::Get_GivenFirstAxisOutOfRange_ReturnsNone`
- **ARRAY-004** Implement reshape, transpose, squeeze, expand, flatten, concat, and append.
  - `lovelace-array::tests::Reshape_GivenMatchingNumel_PreservesRowMajorOrder`
- **ARRAY-005** Match broadcasting rules and their error messages.
  - `lovelace-array::tests::ZipMapBroadcast_GivenScalar_ReusesValue`
- **ARRAY-006** Match reductions, including axis behavior.
  - `lovelace-array::tests::SumAxis_GivenMatrix_ReducesRequestedAxis`
- **ARRAY-007** Preserve typed-field arithmetic without silently falling back to machine floating point.
  - `lovelace-array::tests::FieldForI64_GivenOperations_MatchesIntegerArithmetic`
- **ARRAY-008** Match dot products, matrix multiplication, determinants, inverses, trace, rank, and linear-system solving.
  - `lovelace-lang::suite_parity_batch::arraytests_dot_innerproduct`
- **ARRAY-009** Test zero-dimensional, empty, singular, non-square, and incompatible-shape cases.
  - `lovelace-array::tests::Reductions_GivenEmptyArray_UseIdentitiesAndEmptyReduction`
- **ARRAY-010** Verify matrix identities on generated exact inputs.
  - `lovelace-lang::round66_resweep::acceptance_inverse_times_original_is_identity`

### CPLX — 6 moved to met

- **CPLX-001** Match arithmetic and rendering across real and imaginary components.
  - `lovelace-lang::complex_round24::complex_arithmetic_is_component_wise_and_exact`
- **CPLX-002** Preserve arbitrary precision.
  - `lovelace-lang::complex_round24::lcomplex128_covers_the_same_surface_at_38_digits`
- **CPLX-003** Support the C# fixed-width optimized representations where their observable performance contract matters.
  - `lovelace-lang::complex_round24::lcomplex64_parses_adds_and_conjugates_like_the_reference`
- **CPLX-004** Promote to arbitrary precision without silently saturating or losing exactness.
  - `lovelace-lang::complex_round24::lcomplex64_promotes_on_overflow_instead_of_rounding`
- **CPLX-005** Verify optimized results against arbitrary-precision results.
  - `lovelace-lang::complex_round24::limited_precision_takes_the_fixed_path_and_stays_exact`
- **CPLX-006** Preserve real-to-complex coercion and domain behavior.
  - `lovelace-lang::complex_round24::scalar_re_im_conj_promote_every_numeric_kind`

### DESK — 20 moved to met

- **DESK-002** Preserve editor, workspace, graph, and logs panes.
  - `lovelace-desktop::ui::app::tests::a_frame_renders_the_same_assembled_view_the_workbench_published`
- **DESK-003** Preserve the symbolic inspector.
  - `lovelace-desktop::inspector::tests::a_full_payload_reads_all_six_sections_from_the_engine_data`
- **DESK-004** Support multiple independent documents and engines.
  - `lovelace-desktop::workbench::tests::starts_with_two_independent_document_tabs`
- **DESK-005** Preserve per-document precision and execution state.
  - `lovelace-desktop::app::tests::precision_is_document_scoped`
- **DESK-006** Execute scripts asynchronously without blocking input or rendering.
  - `lovelace-desktop::execution::tests::t2_submit_does_not_block_and_run_started_arrives_quickly`
- **DESK-007** Support cancellation with consistent committed-state behavior.
  - `lovelace-desktop::execution::tests::t4_cancellation_stops_a_real_run_and_keeps_the_committed_snapshot`
- **DESK-008** Preserve incremental execution and visible statement status.
  - `lovelace-desktop::workbench::tests::every_committed_step_publishes_one_coherent_snapshot`
- **DESK-009** Restore window layout, documents, recent files, and settings.
  - `lovelace-desktop::settings::tests::session_tab_order_and_active_tab_round_trip_through_disk`
- **DESK-010** Implement crash recovery for unsaved buffers.
  - `lovelace-desktop::recovery::tests::a_record_round_trips_and_its_body_is_the_buffer_bytes`
- **DESK-011** Syntax highlighting.
  - `lovelace-desktop::ui::syntax` (11 tests: keywords, builtins, numbers, strings, interpolation, operators)
- **DESK-012** Context-aware autocomplete using the live builtin registry.
  - `lovelace-desktop::completion::tests::every_catalogue_entry_comes_from_one_of_the_live_sources`
- **DESK-015** Inline diagnostics with accurate source positions.
  - `lovelace-desktop::diagnostic_map` (8 tests: UTF-16 columns, CRLF, astral characters, combining marks)
- **DESK-017** Per-tab undo/redo and selection preservation.
  - `lovelace-desktop::ui::editor::tests::each_editor_has_its_own_history`
- **DESK-018** Efficient editing of large scripts.
  - rust/lovelace-desktop/src/ui/editor.rs: 'Virtualised code editor: only the visible rows are laid out'
- **DESK-019** Browse variables with their kinds and structured projections.
  - `lovelace-desktop::workspace_table::tests::variables_default_to_name_ascending_and_include_the_underscore_global`
- **DESK-020** Browse functions and help metadata.
  - `lovelace-desktop::workspace_table::tests::functions_show_parameters_origin_and_summary`
- **DESK-021** Inspect symbolic expression trees, assumptions, traces, and compiled MathIR.
  - `lovelace-desktop::inspector::tests::a_full_payload_reads_all_six_sections_from_the_engine_data`
- **DESK-022** Render SVG plots accurately.
  - `lovelace-desktop::ui::plot::tests::renders_clipping_transforms_dashed_paths_and_labels`
- **DESK-023** Support plot zoom, pan, fit, and export.
  - `lovelace-desktop::ui::plot::tests::zooming_at_a_known_point_keeps_that_point_stationary`
- **DESK-028** Demonstrate the absence of a localhost HTTP listener and embedded web runtime.
  - `lovelace-desktop::netcheck::tests::crate_sources_never_reference_networking_apis`

### DSP — 8 moved to met

- **DSP-001** Preserve all 16 C# DSP builtins and their signatures.
  - rust/lovelace-lang/src/registry.rs: exactly 16 descriptors carry plugin `Lovelace.Dsp`
- **DSP-002** Match convolution, DFT, FFT, filtering, and moving average.
  - `lovelace-lang::dsp_round21::convolve_given_two_ones_produces_triangle`
- **DSP-003** Match signal constructors, including impulse, step, cosine, exponential, power series, and noise.
  - `lovelace-lang::dsp_round20::impulse_given_lengths_one_and_four_are_unit_impulses`
- **DSP-004** Match delay, scale, and complex component operations.
  - `lovelace-lang::dsp_round20::delay_given_positive_zero_and_negative_offsets_shift_causally`
- **DSP-005** Preserve argument coercion and argument-position error reporting.
  - `lovelace-lang::dsp_round20::impulse_given_fractional_count_reports_the_exact_integer_contract`
- **DSP-006** Match precision-dependent optimized paths and arbitrary-precision fallbacks.
  - `lovelace-lang::fixed_dsp_tests::convolve64_given_nineteen_digits_keeps_the_width_and_promotes_at_twenty`
- **DSP-007** Preserve deterministic seeded noise generation.
  - `lovelace-lang::dsp_round20::noise_given_seed_zero_is_the_documented_deterministic_sequence`
- **DSP-010** Resolve the documented ambiguity in noise overloads explicitly, without silently changing the frozen C# behavior.
  - docs/goal-parity-contract/round-20/dsp-builtins.md: noise arity/seed contract (3 or 4 args, Int32 seed, short/long forms equal)

### ECO — 1 moved to met

- **ECO-006** Provide equivalent Rust extension points for the C# abstractions that are intended to survive migration.
  - rust/lovelace-lang/src/modus.rs: ModusPlugin/ModusHost seam with register_builtin, register_kernel and descriptor registration

### ENG — 7 moved to met

- **ENG-001** Preserve every published value kind and its widening rules.
  - `lovelace-lang::value_widen` (9 tests)
- **ENG-002** Match variables, assignments, scopes, closures, recursion, and function calls.
  - `lovelace-lang::suite_engine_batch::evaluate_given_assignment_exposes_variable_in_variables_dictionary`
- **ENG-003** Preserve the _ last-result variable.
  - `lovelace-lang::suite_engine_batch::evaluate_given_result_stores_in_underscore_variable`
- **ENG-004** Match loops, control flow, and statement evaluation order.
  - `lovelace-lang::suite_parity_batch::statementtests_evaluate_givenwhileloop_accumulatescount`
- **ENG-005** Preserve per-engine precision and plugin state isolation.
  - `lovelace-lang::context_round56::two_concurrent_scopes_are_isolated`
- **ENG-007** Preserve partial execution, failure, and cancellation behavior.
  - `lovelace-lang::cancellation_round18::a_cancelled_run_keeps_the_partial_state_it_committed`
- **ENG-008** Match recursion, allocation, rendering, and evaluation budgets.
  - `lovelace-lang::round19_batch::suite_recursion_at_the_deepest_accepted_depth_answers`

### GATE — 11 moved to met

- **GATE-001** Change a decimal period boundary.
  - `powershell -File parity/mutate.ps1 -Only GATE-001` exit 0: 1/1 detected, 1 reverted tree passes; detail `cargo-test:the_reference_truncates_a_period_that_closes_exactly_at_the_cap exit 101`
- **GATE-002** Change a numeric widening rule.
  - `powershell -File parity/mutate.ps1` run 2026-11-12: mutation `GATE-002` (Change a numeric widening rule) detected=True and restored=True; detail `corpus:lang/020 exit 1`
- **GATE-003** Break a Unicode source position.
  - `powershell -File parity/mutate.ps1` run 2026-11-12: mutation `GATE-003` (Break a Unicode source position) detected=True and restored=True; detail `fixtures:error_envelope exit 1`
- **GATE-004** Remove an envelope field.
  - `powershell -File parity/mutate.ps1` run 2026-11-12: mutation `GATE-004` (Remove an envelope field) detected=True and restored=True; detail `fixtures:symbolic exit 1`
- **GATE-005** Alter a diagnostic code.
  - `powershell -File parity/mutate.ps1` run 2026-11-12: mutation `GATE-005` (Alter a diagnostic code) detected=True and restored=True; detail `fixtures:error_envelope exit 1`
- **GATE-006** Change symbolic canonical ordering.
  - `powershell -File parity/mutate.ps1` run 2026-11-12: mutation `GATE-006` (Change symbolic canonical ordering) detected=True and restored=True; detail `fixtures:symbolic exit 1`
- **GATE-007** Break final-digit rounding.
  - `powershell -File parity/mutate.ps1` run 2026-11-12: mutation `GATE-007` (Break final-digit rounding) detected=True and restored=True; detail `corpus:sym/080 exit 1`
- **GATE-008** Duplicate captured print() output.
  - `powershell -File parity/mutate.ps1` run 2026-11-12: mutation `GATE-008` (Duplicate captured print() output) detected=True and restored=True; detail `fixtures:print_purity exit 1`
- **GATE-009** Remove builtin registry metadata.
  - `powershell -File parity/mutate.ps1` run 2026-11-12: mutation `GATE-009` (Remove builtin registry metadata) detected=True and restored=True; detail `cli:RUN-008 exit 1`
- **GATE-010** Disable a precision cap.
  - `powershell -File parity/mutate.ps1 -Only GATE-010` exit 0: 1/1 detected, 1 reverted tree passes; detail `cargo-test:the_smallest_over_cap_request_is_marked_too exit 101`
- **GATE-012** Run the gate with an empty or incomplete corpus.
  - `powershell -File parity/mutate.ps1` run 2026-11-12: mutation `GATE-012` (Run the gate with an empty or incomplete corpus) detected=True and restored=True; detail `diff-empty exit 2`

### HARNESS — 4 moved to met

- **HARNESS-008** Controlled scope changes: changes to a public contract or exclusion policy require user approval.
  - docs/goal-parity-contract/journal.md DEC-001 (cycle scope) and DEC-003 (user-delegated exclusions)
- **HARNESS-010** Final release auditor: a dedicated final gate evaluates the entire migration from the frozen source of truth.
  - `powershell -File parity/run-all.ps1` regenerates parity/scorecard.json from live gates and exits non-zero on any FAILED
- **HARNESS-011** Work preservation: each completed phase produces a reviewable commit or PR, test evidence, and an updated manifest.
  - git log: 61919a7 'Parity round 101: mutation anchors re-anchored...' and 112041b 'Final acceptance...'
- **HARNESS-012** No circular verification: a generated implementation and its generated tests cannot be the only two sources supporting a parity claim.
  - the C# oracle, 20 golden fixtures, the 5000-case differential and the SymPy oracle are independent of the Rust implementation

### LANG — 5 moved to met

- **LANG-002** Match operator precedence and associativity.
  - `lovelace-lang::suite_parity_batch::signprecedencetests_evaluate_signandpower_minustwosquared`
- **LANG-003** Match literal syntax, string interpolation, ranges, indexing, and slicing.
  - `lovelace-lang::suite_parity_batch::interpolationprinttests_evaluate_giveninterpolatedstring_formats`
- **LANG-005** Preserve source positions in UTF-16 code units.
  - `lovelace-run::script_source` (15 tests, incl. UTF-16 line/column)
- **LANG-006** Preserve parse errors, including codes, messages, and locations.
  - `lovelace-run::error_taxonomy_batch` (24 tests: parse-error codes, categories, messages and positions)
- **LANG-007** Reject excessive AST/input depth in the same manner as the reference.
  - `lovelace-lang::round19_batch::suite_depth_nesting_one_past_the_budget_is_refused_with_the_typed_depth_error`

### MIR — 7 moved to met

- **MIR-001** Support the complete versioned MathIR node set and text format.
  - rust/lovelace-lang/src/mathir.rs: versioned `#!mathir 2` text format, IrOpKind node set, param/const pools
- **MIR-002** Match lowering, optimization, type inference, and validation.
  - `lovelace-lang::mathir_round54::lowering_refuses_what_the_reference_refuses`
- **MIR-003** Preserve common-subexpression handling and emitted ordering where published.
  - rust/lovelace-lang/src/mathir.rs (structural CSE: 'subexpressions share their node')
- **MIR-004** Match lower, compile, compile_full, evalir, and evalir_batch.
  - `lovelace-lang::mathir_round54::kernel_named_and_positional_evaluate`
- **MIR-006** Verify direct execution against lowered-then-executed results.
  - `lovelace-lang::round97_tail::propertytests_mathirevaluateslikesymbolic`
- **MIR-007** Reject malformed or unsupported IR identically.
  - `lovelace-lang::mathir_typing_round55::deserialize_rejects_malformed_program`
- **MIR-008** Add direct MathIR tests, addressing the original C# test-coverage gap.
  - `lovelace-lang::mathir_round54` (13 tests)

### NUM — 8 moved to met

- **NUM-001** All constructors and parsing forms match C# behavior.
  - `lovelace-nat::tests::from_decimal_GivenFiveRoundTripLiterals_ReturnsOriginal`
- **NUM-002** Canonical zero representation; no negative zero.
  - `lovelace-int::tests::from_decimal_GivenNegativeZero_NormalizesToPositiveZero`
- **NUM-003** Exact addition, subtraction, multiplication, division, and remainder.
  - `lovelace-nat::tests::add_GivenCarryChainAcrossThreeLimbs_Propagates`
- **NUM-004** Signed division uses the same quotient and remainder convention as C#.
  - `lovelace-int::tests::div_rem_signed_GivenNegativeTenOverThree_YieldsNegativeThreeAndMagnitudeOne`
- **NUM-006** Exact power, factorial, and decimal shifting.
  - `lovelace-nat::tests::pow_GivenTwoToHundred_ReturnsKnownValue`
- **NUM-007** Decimal parse/format round trips for arbitrarily large values.
  - `lovelace-nat::tests::parse_then_format_GivenLargeLiteral_RoundTrips`
- **NUM-009** Test every multiplication/division algorithm against an independent arithmetic oracle.
  - `lovelace-nat::ntt::tests::agrees_with_schoolbook_on_random_lengths_up_to_300_limbs`
- **NUM-010** Benchmark algorithm transitions, not merely steady-state performance.
  - docs/goal-rust-perf/evidence.md EVD-042: transform crossover measured at 20k/40k/80k/120k digits (NTT vs Karatsuba)

### PERF — 3 moved to met

- **PERF-001** Benchmark identical inputs and compare matching output digests.
  - docs/goal-rust-perf/evidence.md EVD-036/EVD-046: paired Rust-vs-C# runs on identical inputs with output digests
- **PERF-002** Use warmed executions and account for C# JIT effects.
  - docs/goal-rust-perf/state.md: 'Fair, warmed comparisons against the reference now exist for every metric'
- **PERF-004** Benchmark both implementations on the same machine and software environment.
  - docs/goal-rust-perf/evidence.md EVD-036/EVD-046: Rust and C# harnesses interleaved on the same machine and session

### PLUG — 6 moved to met

- **PLUG-001** Implement the functional equivalent of IModusPlugin and IModusContext.
  - rust/lovelace-lang/src/modus.rs: ModusPlugin/ModusHost with registration, descriptors and arity
- **PLUG-002** Preserve explicit per-host plugin loading.
  - `lovelace-lang::round67_modus::dsp_plugin_unregistered_engine_refuses_fft`
- **PLUG-003** Preserve per-engine plugin state; assumptions from one engine must never leak into another.
  - `lovelace-lang::round67_modus::assumption_committed_while_a_is_running_does_not_decide_b`
- **PLUG-005** Preserve typed-array field and kernel registration.
  - rust/lovelace-lang/src/modus.rs: ModusHost::register_kernel<T: KernelScalar> and FieldKernel
- **PLUG-006** Keep value conversions owned by the host/kernel boundary.
  - `lovelace-lang::round67_modus::enum_value_round_trips_through_the_modus_seam`
- **PLUG-008** Verify plugin behavior under multiple concurrently active engines.
  - `lovelace-lang::context_round56::parallel_function_creation_no_duplicate_ids`

### PROOF — 1 moved to met

- **PROOF-001** Preserve the pinned Lean project and ensure it builds.
  - `lake build` in Lovelace.Proofs exit 0: 'Build completed successfully (9 jobs)' (Lean v4.33.1 installed from the pinned lean-toolchain)

### REAL — 8 moved to met

- **REAL-001** Preserve arbitrary decimal precision and exactness metadata.
  - `lovelace-real::tests::Pi_GivenFifty_TruncatesRatherThanRounds`
- **REAL-002** Preserve repeating-decimal notation, including shortest-period canonicalization.
  - `lovelace-real::real_parity_batch::div_GivenOneBySeven_ReturnsPeriod142857`
- **REAL-003** Match exactness propagation through operations and conversions.
  - `lovelace-real::round30_real_batch::division_and_its_multiplication_twin_agree_on_value_and_exactness`
- **REAL-004** Preserve rounding, truncation, guard-digit, and computation-cap semantics.
  - `lovelace-real::round34_inexact_division::the_reference_truncates_a_period_that_closes_exactly_at_the_cap`
- **REAL-005** Preserve precision changes during execution and their scope.
  - `lovelace-real::round39_real_tests::pi_given_nested_precision_scopes_outer_scope_restored_after_inner_disposes`
- **REAL-006** Implement and verify sqrt, pi, powers, and supported transcendental functions.
  - `lovelace-real::real_parity_batch::sqrt_GivenFive_Matches1000KnownDigits`
- **REAL-008** Preserve the distinction between exact and inexact identity results.
  - `lovelace-real::round31_trig::special_angles_are_still_exact_at_every_precision`
- **REAL-010** Add direct Rational tests, since the C# project lacks a dedicated test suite.
  - `lovelace-real::real_parity_round7` (1089 tests: conversion, periodic arithmetic, parse/format)

### REQ — 2 moved to met

- **REQ-008** Fail-closed gates: Missing tests, missing reference binaries, skipped oracle runs, and unexecuted checks must not count as passes.
  - `node parity/tools/gate-manifest.mjs` exit 0 refuses a stale inventory and a claimed test that is not in it
- **REQ-009** No premature cutover: C# remains the reference implementation until every mandatory gate passes.
  - `powershell -File parity/build-oracle.ps1 -Verify` exit 0: C# oracle remains the reference

### RUST — 4 moved to met

- **RUST-001** cargo fmt --check passes.
  - `cargo fmt --all -- --check` exit 0 (run 2026-11-12 against the current tree)
- **RUST-002** Clippy passes with warnings treated as errors.
  - `cargo clippy --workspace --all-targets -- -D warnings` exit 0 (run 2026-11-12)
- **RUST-003** All tests pass in debug and release profiles.
  - `cargo test --workspace` exit 0: 5666 passed, 0 failed, 1 ignored
- **RUST-005** Every unsafe block is isolated, justified, and reviewed.
  - rust/lovelace-desktop/src/main.rs: unsafe confined to src/win32 FFI; `#![cfg_attr(not(windows), forbid(unsafe_code))]`

### STAT — 3 moved to met

- **STAT-002** Provide equivalent typed-array kernel dispatch.
  - rust/lovelace-lang/src/modus.rs: ModusHost::register_kernel / try_dispatch_kernel<T: KernelScalar>
- **STAT-003** Preserve exact Real-field addition and active precision.
  - `lovelace-lang::round67_modus::real_add_kernel_matches_direct_real_addition`
- **STAT-004** Test fallback behavior and verify equivalence between the specialized and generic array paths.
  - `lovelace-lang::round67_modus::dispatch_unsupported_element_type_falls_back`

### SYM — 14 moved to met

- **SYM-001** Preserve expression kinds, domains, assumptions, and exactness.
  - `cargo test -p lovelace-symbolics` (196 tests, exit 0)
- **SYM-002** Match canonical ordering and hashing-independent canonicalization.
  - `lovelace-symbolics::printing_round16::canonical_order_is_invariant_under_canonical_parse`
- **SYM-003** Preserve structural expression identity and interning behavior where observable.
  - `lovelace-symbolics::round66_resweep::acceptance_interned_equal_structure_is_the_same_value`
- **SYM-004** Preserve textual and canonical expression formats.
  - `lovelace-symbolics::printing_round16::canonical_parse_reprints_every_expression_shape_identically`
- **SYM-005** Match simplification and rewriting with their applicable conditions.
  - `lovelace-symbolics::rational_functions::tests::cancel_reduces_the_documented_corpus`
- **SYM-006** Preserve rewrite budgets, termination, trace ordering, and statuses.
  - `lovelace-symbolics::rewrite_registry::tests::budget_counts_applied_rewrites_and_stops_further_groups`
- **SYM-007** Match substitution, expansion, factoring, collection, and cancellation.
  - `lovelace-symbolics::rational_functions` (13 tests: cancel/collect/factor)
- **SYM-008** Match differentiation, integration, limits, and series.
  - `lovelace-symbolics::tests::Series_GivenExp_MatchesFactorialCoefficients`
- **SYM-009** Preserve solution sets, conditions, completeness, and domain restrictions.
  - `lovelace-lang::suite_parity_batch::solvecompletenesspairingtests_partial_isnevercomplete`
- **SYM-010** Match polynomial, nonlinear, system, and matrix-solving behaviors.
  - `lovelace-symbolics::groebner` (7 tests)
- **SYM-011** Match assumptions, contradiction detection, and assumption isolation.
  - `lovelace-symbolics::assumptions::tests::contradiction_detection_is_neither_missed_nor_invented`
- **SYM-012** Preserve all structured rich-result record types and field order.
  - `lovelace-run::record_schema_batch::every_produced_record_conforms_to_its_declared_schema`
- **SYM-013** Verify the complete symbolic builtin registry, including metadata.
  - `lovelace-lang::help::tests::every_builtin_has_complete_metadata`
- **SYM-014** Retain the existing SymPy differential oracle as an independent correctness check.
  - `.github/workflows/ci.yml` job `sympy-oracle` installs SymPy and runs DifferentialOracleTests with LOVELACE_REQUIRE_SYMPY=1

### TEST — 4 moved to met

- **TEST-002** Build and execute the C# reference in CI.
  - .github/workflows/ci.yml job `fast-tests` runs 14 dotnet test projects; job `aot-smoke` publishes Lovelace.Run Native AOT and executes the published binary against solve/partial/array/print/error scripts (exit 0 required)
- **TEST-007** Maintain parity/manifest.toml mapping every C# test to a Rust equivalent or approved exclusion.
  - `node parity/tools/gate-manifest.mjs` exit 0: 6209/6209 cases dispositioned (6090 ported, 119 user-approved exclusions, 0 deferred)
- **TEST-010** Require explicit reasons for every exclusion.
  - `node parity/tools/gate-manifest.mjs` exit 0: 119 exclusions, each with a non-empty reason and owner=user
- **TEST-013** Require the inventory to remain complete when the oracle changes.
  - `node parity/tools/gate-census.mjs` requires census.oracle.commit == parity/oracle.json commit and one census project per *.Tests directory

## Rows still partial (52)

Each entry names the gap that keeps it open; the full note is in `requirements.json`.

**DESK**

- **DESK-001** Deliver an actual native desktop shell on the agreed supported platforms. — A native Windows shell exists and its tests pass, but the desktop-v4 readiness gate still fails (148 blockers) and Linux/macOS are deferred by user-approved DEV-002; delivery on the agreed platform is not certified.
- **DESK-016** Correct IME, clipboard, and Unicode handling. — Unicode selection and CRLF/clipboard-model behaviour are tested; actual IME composition in the egui editor has no automated test.
- **DESK-024** Provide native open/save dialogs and drag-and-drop. — Native open/save dialogs are implemented and tested; drag-and-drop of files is not implemented (no dropped-file handler in the crate).
- **DESK-025** Detect external file modifications. — The source mtime is recorded at open, but no external-change comparison or prompt exists (a grep for conflict/external detection finds only the comment), so changes made on disk by another program are not detected.
- **DESK-026** Support native keyboard accessibility and screen readers. — Keyboard access and AccessKit/UIA names exist, but the desktop-v4 A11Y rows are NOT_TESTED and no screen-reader verification has been run.
- **DESK-027** Support system font scaling and high-DPI displays. — Layout clamping and egui's scale factor exist; no DPI 100/150/200% test evidence is present (the desktop readiness gate still reports TST-014 NOT_TESTED), so system scaling is unverified.

**DIFF**

- **DIFF-009** Run at least 5,000 comparisons per PR and 100,000 nightly. — The 5,000-case PR slice is clean; no 100,000-case nightly run or nightly workflow exists, so the nightly half of the requirement is not met.

**DSP**

- **DSP-008** Preserve cancellation during long operations. — Cancellation is implemented and tested for generic long kernels; no DSP-specific test (fft/conv/filter) exercises cancellation.
- **DSP-009** Run the original DSP test suite and differential signal-processing benchmarks. — The original DSP test suite is ported; no Rust-vs-C# differential signal-processing benchmark exists (dspbench measures the C# implementation only).

**ECO**

- **ECO-001** Every row must resolve to one or more Rust modules, with tests linked to the corresponding C# capabilities. — Every project row has a disposition and 98.08% of test cases have a Rust test; six projects are absent, so not every row resolves to a Rust module.
- **ECO-002** A complete parity run must prove that all required subsystems are present, even if their APIs have been reorganized. — A parity run does prove presence/absence, and six required subsystems (Abstractions, Statistics, Console, Knowledge, Knowledge.Run, Proofs) are absent, so complete presence is disproved, not proven.
- **ECO-005** Preserve the semantics of public operations, including defaults, error conditions, and value conversions. — Public-boundary semantics are compared broadly, but with no public C# API inventory (ECO-003) 'all public operations' is not covered.

**ENG**

- **ENG-006** Implement consistent revision and state-publication semantics. — Revision publication is implemented, but a failed or cancelled run still publishes a different committed variable set than the oracle (OQ-013).
- **ENG-010** Produce deterministic output across repeated runs and different thread counts. — Determinism is tested for specific paths and measured across worker budgets; no test runs the same script across different thread counts and compares the published envelope.
- **ENG-012** Test all language behaviors through public engine entry points, not only internal helpers. — The runner suite exercises public entry points, but the 1719-test language suite still includes direct internal-helper tests; there is no check that every language behavior is reachable through the public engine.

**GATE**

- **GATE-011** Delete a supposedly ported test. — The mutation exists and gate-manifest is designed to catch a deleted/renamed claimed test, but it could not be validated today: a concurrent desktop session modified rust/lovelace-desktop, making the measured inventory stale, so the manifest gate is red before the mutation and f…

**HARNESS**

- **HARNESS-002** Coverage-driven planning: select work from unsatisfied parity requirements. — Work is visibly driven by ledger gaps, but the selection is manual; no tool derives the next batch from the ledger.
- **HARNESS-003** Independent falsification: every significant implementation claim is checked independently. — Independent falsification exists and several claims were re-run; not every significant claim is re-derived by a second party.
- **HARNESS-005** Distinguish progress from completion. — Progress and completion are reported separately; the ledger still has no automated progress metric beyond raw counts.
- **HARNESS-007** No fabricated green: missing tools, skipped checks, and unavailable platforms are reported as unverified or failed. — Unavailable checks are recorded honestly; the desktop state still carries 123 NOT_TESTED rows, so the reporting discipline is exercised rather than complete.
- **HARNESS-009** Reproducible benchmarking. — A reproducible benchmark harness and recorded results exist; there is no automated rerun that fails when numbers or hardware metadata are missing.

**LANG**

- **LANG-001** Match the complete token set, including Unicode edge cases. — The complete token set is enumerated and exercised; Unicode *edge cases in the lexer* (non-ASCII identifiers/literals) have no dedicated test, only output-glyph tests.
- **LANG-004** Match whitespace, newline, comment, and escape handling. — Whitespace, newline and comment handling are tested; string escape handling (lib.rs::unescape) has no dedicated test, so the 'escape handling' clause is not evidenced.

**MIR**

- **MIR-005** Preserve precision and cancellation behavior during evaluation. — Precision is honoured and exactness tiers are preserved, but no test cancels a long MathIR evaluation, so the cancellation clause is unverified.

**NUM**

- **NUM-005** Underflow, division by zero, invalid input, and overflow of bounded conversions produce equivalent failures. — Division-by-zero and bounded-conversion failures match. The integer power contract still diverges: rust/lovelace-int/src/lib.rs::pow returns 1 for exponent 0 (and 0 for 0^positive) while IntegerPowTests asserts the oracle throws (OBS-010/OQ-009).
- **NUM-008** Identical results regardless of algorithm-selection threshold or worker count. — Algorithm-selection paths are cross-checked; identical results across worker counts are measured in the perf evidence, not asserted by a test.

**PERF**

- **PERF-003** Report distributions rather than a single sample. — Mean and median are reported; no distribution (percentiles/min/max or per-iteration samples) is recorded.
- **PERF-005** Measure arithmetic at small, medium, large, and algorithm-transition sizes. — Transition and large sizes are measured; the small/medium range is only used incidentally (the benchmark harness takes a size argument), with no recorded small/medium matrix.
- **PERF-006** Include the original numeric, DSP, array, precision, and symbolic benchmark families. — The numeric family is covered; DSP, array, precision and symbolic benchmark families have no Rust harness.
- **PERF-007** Profile allocations, peak memory, startup time, and latency. — Latency and one CPU profile exist; allocations, peak memory, startup time and a systematic latency report are absent.
- **PERF-008** Record all results, inputs, build profiles, and hardware details. — Results and build profiles are recorded; hardware details (CPU/RAM) and the raw per-iteration inputs are not.
- **PERF-009** Meet the existing migration target of at least 80% of reference throughput, unless a specific deviation is approved. — Most metrics are at or above the 80% target; parse is at ~31% of the reference throughput and no deviation has been approved.
- **PERF-010** Never replace a correctness failure with a performance-based justification. — No correctness failure has been replaced by a performance justification; the rule is recorded but no automated gate enforces it.

**PLUG**

- **PLUG-007** Compare the complete C# and Rust registries automatically. — The registry is compared through the capabilities fixture; there is still no dedicated C#-vs-Rust registry diff tool.

**REAL**

- **REAL-007** Test precision boundaries at 8, 16, 20, 50, 1,000, and 2,500 decimal places. — Boundaries at 8, 16, 20, 50 and 1000 places are exercised; no 2500-place test exists (grep for 2500 in rust/lovelace-real returns nothing).

**REQ**

- **REQ-002** Complete inventory: Every C# production project, public capability, test, executable, and relevant artifact must have an explicit Rust disposition. — Projects and test cases are inventoried with dispositions; public C# types/methods are still not enumerated one by one (ECO-003).
- **REQ-003** Observable equivalence: Compare results at public boundaries; internal algorithms and memory representations may differ. — Verification compares public boundaries, and internal representations differ freely; documented public differences remain (parity/final-acceptance.md lists four; OQ-013 committed-state), so observable equivalence is not complete.
- **REQ-004** Strict parity: Mathematical equivalence alone is insufficient when the public contract guarantees a specific textual or structured representation. — Textual/structured comparison is enforced, but the four recorded deviations in docs/goal-parity-contract/final-acceptance.md are not user-approved and no gate fails on them.
- **REQ-006** Explicit deviations: Intentional behavioral changes require a documented decision, a reason, affected tests, and the user's approval. — Deviations are documented with reasons, but the four parity deviations have no recorded user approval and no affected-test list; the desktop deviations are separate from contract-v2.
- **REQ-007** Independent verification: Implementation agents cannot be the sole authority validating their own work. — Independent falsification exists (intent audit, mutation gate, gate re-runs); not every implementation claim is re-derived by a second party, and implementation agents still author most evidence.

**RUN**

- **RUN-003** -stdin — Accept piped source and BOM handling — Piped source matches; BOM handling remains a recorded oracle/spec conflict (OQ-002), so the row pins the oracle behaviour rather than resolving it.

**RUST**

- **RUST-004** No floating point is used in the exact numeric value path. — No number is represented or compared in floating point and arithmetic/parsing are exact; one f64 log10 estimate remains in the Nat multiplication split heuristic.
- **RUST-006** Dependencies are audited for license and security advisories. — The dependency set is small and visible; no `cargo audit`/license report has been produced.
- **RUST-007** Public APIs have usable documentation and examples. — Docs and one example exist, but not every public API has a runnable example or a doc test (doc-tests run 0 tests).
- **RUST-008** Build and test workflows work from a clean checkout. — The build/test workflow works in this prepared checkout; a clean checkout is not demonstrated (the toolchain is vendored under .toolchain and Cargo.lock is modified).

**STAT**

- **STAT-001** Preserve the plugin's identity and opt-in registration. — The identity and opt-in registration are modelled by a test-local plugin (round67_modus.rs); no production Statistics plugin is shipped.

**TEST**

- **TEST-005** Archive oracle binaries or build inputs for reproducibility. — The commit and the DLL hash are recorded; neither the DLL nor the build inputs are archived, so the oracle binary still depends on a local build.
- **TEST-009** Detect missing, duplicate, renamed, or ignored tests. — Missing, duplicate and stale cases are detected. Renamed tests are caught only through the stale-inventory hash, and an ignored test is not rejected: rust-tests.json records 1 ignored test and the gate does not distinguish it.
- **TEST-011** Disallow weakening assertions while porting tests. — Spot checks and one systematic re-open exist, but there is no gate that rejects a ported test whose assertions are weaker than the C# original.

**WIRE**

- **WIRE-013** Match the C# serializer's non-ASCII escaping rules where byte compatibility is required. — Parsed JSON matches and both writers escape non-ASCII as \uXXXX, but no gate compares raw output bytes, so byte compatibility is not proven.
- **WIRE-014** Match structured truncation rules and differentiate depth, node, and precision caps. — Depth and node budgets are exercised and differentiated; a separate precision-cap truncation is not exercised through the wire.
- **WIRE-015** Preserve artifact behavior for plots, including failure after a previous plotting statement. — Artifact kind/path/naming and failure-after-plot behaviour match; the SVG bytes are recorded as byte-identical in round-87 but no automated gate compares them.

## Rows still unmet (30)


**CLI**

- **CLI-001** Implement the interactive Lovelace REPL. — No interactive Lovelace REPL exists: Lovelace.Console is dispositioned absent in parity/projects.toml and its 15 test cases are approved exclusions, not ports.
- **CLI-002** Preserve multiline statement accumulation. — No interactive Lovelace REPL exists: Lovelace.Console is dispositioned absent in parity/projects.toml and its 15 test cases are approved exclusions, not ports.
- **CLI-003** Support editing, history, and keyboard navigation. — No interactive Lovelace REPL exists: Lovelace.Console is dispositioned absent in parity/projects.toml and its 15 test cases are approved exclusions, not ports.
- **CLI-004** Preserve interactive commands and help. — No interactive Lovelace REPL exists: Lovelace.Console is dispositioned absent in parity/projects.toml and its 15 test cases are approved exclusions, not ports.
- **CLI-005** Maintain engine state between commands. — No interactive Lovelace REPL exists: Lovelace.Console is dispositioned absent in parity/projects.toml and its 15 test cases are approved exclusions, not ports.
- **CLI-006** Match precision handling and _ result behavior. — No interactive Lovelace REPL exists: Lovelace.Console is dispositioned absent in parity/projects.toml and its 15 test cases are approved exclusions, not ports.
- **CLI-007** Match errors, cancellation, and output formatting. — No interactive Lovelace REPL exists: Lovelace.Console is dispositioned absent in parity/projects.toml and its 15 test cases are approved exclusions, not ports.

**DESK**

- **DESK-013** Bracket matching and automatic indentation. — No bracket matching or automatic indentation is implemented; a grep of rust/lovelace-desktop/src for bracket/indent finds neither a matcher nor an auto-indent action (only unrelated message text).
- **DESK-014** Find/replace, go-to-line, and editing shortcuts. — No find/replace, go-to-line or editing-shortcut surface exists; the keyboard module binds run/cancel/file/undo only and a grep for find/replace/goto finds none.

**ECO**

- **ECO-003** Inventory all externally visible C# types and methods, not just language builtins. — Only test methods are enumerated (census). No inventory of externally visible C# types and methods exists.
- **ECO-004** Identify external consumers: command-line scripts, applications, plugins, automation, and agent harnesses. — No inventory of external consumers (scripts, applications, plugins, automation, agent harnesses) exists.
- **ECO-007** Explicitly decide whether any .NET consumers require continued compatibility through a C ABI, FFI bridge, or C# wrapper. — No decision is recorded on .NET consumer compatibility (C ABI, FFI bridge or C# wrapper).
- **ECO-008** Inventory persisted formats and ensure existing files remain readable or have a verified migration path. — No inventory of persisted C# formats (Studio sessions/settings, knowledge graphs, plots) and no migration path for them.

**KNOW**

- **KNOW-001** Port the observation-driven sampling pipeline. — No Knowledge/MGIR module or crate exists (parity/projects.toml dispositions Lovelace.Knowledge and Lovelace.Knowledge.Run absent; a grep of rust/ for knowledge/sampling/convergence finds nothing), so the observation-driven sampling pipeline is not implemented.
- **KNOW-002** Preserve deterministic seeded generation. — No Knowledge/MGIR module or crate exists (parity/projects.toml dispositions Lovelace.Knowledge and Lovelace.Knowledge.Run absent; a grep of rust/ for knowledge/sampling/convergence finds nothing), so the deterministic seeded generation is not implemented.
- **KNOW-003** Implement observation canonicalization and behavioral-plane classification. — No Knowledge/MGIR module or crate exists (parity/projects.toml dispositions Lovelace.Knowledge and Lovelace.Knowledge.Run absent; a grep of rust/ for knowledge/sampling/convergence finds nothing), so the observation canonicalization and behavioral-plane classification is not imp…
- **KNOW-004** Implement reduction, convergence, boundaries, and frontiers. — No Knowledge/MGIR module or crate exists (parity/projects.toml dispositions Lovelace.Knowledge and Lovelace.Knowledge.Run absent; a grep of rust/ for knowledge/sampling/convergence finds nothing), so the reduction, convergence, boundaries and frontiers is not implemented.
- **KNOW-005** Preserve graph persistence formats. — No Knowledge/MGIR module or crate exists (parity/projects.toml dispositions Lovelace.Knowledge and Lovelace.Knowledge.Run absent; a grep of rust/ for knowledge/sampling/convergence finds nothing), so the graph persistence formats is not implemented.
- **KNOW-006** Implement the config, sample, reduce, converge, query, and help commands. — No Knowledge/MGIR module or crate exists (parity/projects.toml dispositions Lovelace.Knowledge and Lovelace.Knowledge.Run absent; a grep of rust/ for knowledge/sampling/convergence finds nothing), so the config/sample/reduce/converge/query/help commands is not implemented.
- **KNOW-007** Match the JSON-over-stdio command protocol and exit codes. — No Knowledge/MGIR module or crate exists (parity/projects.toml dispositions Lovelace.Knowledge and Lovelace.Knowledge.Run absent; a grep of rust/ for knowledge/sampling/convergence finds nothing), so the JSON-over-stdio command protocol and exit codes is not implemented.
- **KNOW-008** Reproduce the seeded C# graph with seed 20240617. — No Knowledge/MGIR module or crate exists (parity/projects.toml dispositions Lovelace.Knowledge and Lovelace.Knowledge.Run absent; a grep of rust/ for knowledge/sampling/convergence finds nothing), so the reproducing the seeded C# graph (seed 20240617) is not implemented.
- **KNOW-009** Verify the Rust knowledge tool against the real Rust runner rather than a mock. — No Knowledge/MGIR module or crate exists (parity/projects.toml dispositions Lovelace.Knowledge and Lovelace.Knowledge.Run absent; a grep of rust/ for knowledge/sampling/convergence finds nothing), so the verifying a Rust knowledge tool against the real runner is not implemented.
- **KNOW-010** Compare graph outputs and convergence metrics against the frozen C# implementation. — No Knowledge/MGIR module or crate exists (parity/projects.toml dispositions Lovelace.Knowledge and Lovelace.Knowledge.Run absent; a grep of rust/ for knowledge/sampling/convergence finds nothing), so the comparing graph outputs and convergence metrics with frozen C# is not imple…

**PROOF**

- **PROOF-002** Map each relevant theorem to Rust implementation-boundary property tests. — No mapping from the Lean theorems to Rust implementation-boundary property tests exists.
- **PROOF-003** Document the correspondence between the Rust numeric representation and the proof model. — No document relates the Rust numeric representation (base-2^64 limbs / exact decimals) to the Lean proof model.
- **PROOF-004** Include exhaustive small-domain arithmetic verification in CI. — No exhaustive small-domain arithmetic verification runs in CI; the C#/Rust suites use sampled cases.

**REQ**

- **REQ-010** Reproducibility: The final state must be reproducible from a clean checkout on CI infrastructure. — No CI workflow builds or tests the Rust workspace from a clean checkout (.github/workflows/ci.yml covers the C# suites only), so clean-checkout reproduction on CI is not demonstrated.

**RUST**

- **RUST-009** Reproducible release artifacts are produced for every supported target. — No release artifacts or reproducible-build pipeline exist for any target; only local debug/release builds under rust/target.
- **RUST-010** CI tests real binaries through their published entry points. — No Rust CI job exists (.github/workflows/ci.yml is C#-only), so no CI test exercises the published Rust binaries.

**TEST**

- **TEST-004** Record approved oracle amendments. — No oracle-amendment record exists: a grep of docs and parity for an amendment log finds only the contract line that requires one.

## Rows that could not be adjudicated

None were left untouched: all 262 rows have a status, an evidence array, and a note.

One row could not be *validated* to met today: **GATE-011** (delete a supposedly ported test). Its
mutation exists and gate-manifest resolves every claimed Rust test, but the mutation harness reported
`NOT VALIDATED: manifest was already red before the mutation` because `parity/rust-tests.json` had
become stale relative to the concurrent desktop edits. It is `partial`, not met, until the inventory
is regenerated on a quiet tree and the mutation is re-run.

## Caveats and environment findings

1. **A concurrent desktop session was writing the tree.** Between 21:55 and 22:00 another session
   modified seven files under `rust/lovelace-desktop/src` and added `incremental.rs`. This made
   `parity/rust-tests.json` stale, so `gate-manifest.mjs` now exits 1 with
   “rust-tests.json is stale”. All Rust quality and suite measurements above were taken before those
   writes (HEAD 112041b). The concurrent work is uncommitted and outside this round's scope.
2. **The single 12-mutation run reported 9/12.** GATE-001 and GATE-010 failed their *precondition*
   (the target test was reported already red) during that run; both tests pass in isolation and both
   mutations re-ran individually at 1/1 detected, 1/1 restored. GATE-011 is the one real casualty of
   the stale inventory. The full-run failure is therefore recorded as contamination, not as a gate
   defect, but the report does not claim a clean 12/12 run today.
3. **TEST-002 is configuration evidence.** `.github/workflows/ci.yml` builds and runs the C#
   reference (including the AOT publish-and-execute job), but no CI run log is stored in the
   repository, so the row is evidenced by the workflow file rather than a recorded run.
4. **Desktop capability vs desktop release.** Twenty DESK rows are met on named headless/component
   tests; `tools/desktop-gate.ps1 -Readiness` still exits 1 with 148 blocking problems
   (47/171 P0 PASS, 123 NOT_TESTED). DESK-001, DESK-016, DESK-024, DESK-025, DESK-026 and DESK-027
   are partial for that reason; DESK-013 and DESK-014 are unmet because the features do not exist.
5. **SYM-014 is met with a qualification.** The SymPy oracle is retained in C# CI and Rust tests
   compare against recorded SymPy 1.14 values; there is no live SymPy bridge on the Rust side
   (CAP-003), so the oracle is not executed against Rust in CI.
6. **PROOF-001 required a toolchain install.** The pinned Lean 4.33.1 toolchain was downloaded during
   this round; `lake build` then completed successfully. PROOF-002..004 remain unmet (no theorem
   mapping, representation correspondence, or exhaustive CI verification).

## Reproduction

```powershell
node parity/tools/gate-requirements.mjs
node parity/tools/gate-census.mjs
node parity/tools/gate-fixtures.mjs
node parity/tools/gate-cli.mjs
powershell -NoProfile -ExecutionPolicy Bypass -File parity/mutate.ps1
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --workspace --release
```

The report is intentionally explicit about the two commands that are red *at the time of writing*
(`gate-manifest` and the full `mutate.ps1` run) and why: a stale inventory caused by an uncommitted
concurrent writer, not by a change made in this round.
