# LovelaceSharp — Full C# → Rust Parity Requirements (v2.0)

Migration acceptance contract. Transcribed from the requester's specification, which is the
authoritative source. Requirement IDs are extracted from this file by parity/gate-requirements.ps1;
the ledger in parity/requirements.toml must cover every ID found here.

> The central requirement is that the Rust ecosystem must be capable of replacing the entire C#
> LovelaceSharp ecosystem without losing observable functionality, correctness, interoperability,
> or tooling. The failure was primarily in translating those requirements into binding acceptance
> gates. This specification therefore concentrates on executable obligations, explicit coverage, and
> protecting the harness from silently narrowing the task.

## 1. Definition of parity

Six independent levels of compatibility.

| Level | Scope | Required compatibility |
|---|---|---|
| P0 | Numerical and language semantics | Exact |
| P1 | Textual rendering | Byte-for-byte where deterministic |
| P2 | Errors and diagnostics | Exact codes, categories, messages, positions |
| P3 | CLI and JSON protocol | Structural and behavioral equivalence |
| P4 | Extended ecosystem | Equivalent capabilities, plugins, artifacts, integrations |
| P5 | Performance and reliability | Measured against C# baselines |

A seventh dimension, P6 — Desktop usability, replaces the original web interface with equivalent
user-facing capabilities. Each dimension must be evaluated independently. For example, 212/214
corpus passes combined with 0/20 protocol fixtures must result in migration failure, regardless of
the percentage of passing checks overall.

### 1.1 Mandatory principles

- REQ-001 — Frozen reference: Establish an immutable C# oracle commit or tag. All comparisons must identify its SHA.
- REQ-002 — Complete inventory: Every C# production project, public capability, test, executable, and relevant artifact must have an explicit Rust disposition.
- REQ-003 — Observable equivalence: Compare results at public boundaries; internal algorithms and memory representations may differ.
- REQ-004 — Strict parity: Mathematical equivalence alone is insufficient when the public contract guarantees a specific textual or structured representation.
- REQ-005 — No silent exclusions: Unsupported features must appear in a machine-readable coverage manifest.
- REQ-006 — Explicit deviations: Intentional behavioral changes require a documented decision, a reason, affected tests, and the user's approval.
- REQ-007 — Independent verification: Implementation agents cannot be the sole authority validating their own work.
- REQ-008 — Fail-closed gates: Missing tests, missing reference binaries, skipped oracle runs, and unexecuted checks must not count as passes.
- REQ-009 — No premature cutover: C# remains the reference implementation until every mandatory gate passes.
- REQ-010 — Reproducibility: The final state must be reproducible from a clean checkout on CI infrastructure.

### 1.2 Permitted architectural differences

Base-10^9 versus binary limbs; Rust Result versus C# exceptions; Rust traits versus .NET
interfaces; different algorithms for multiplication, division, simplification, or symbolic solving;
different threading, allocation, caching, and ownership strategies; native desktop controls in place
of the Studio's HTTP routes and browser frontend. Architectural freedom must never become a
justification for removing a capability.

## 2. Nonnegotiable acceptance gates

FULL MIGRATION ACCEPTANCE: 214/214 documentation corpus; 20/20 protocol golden fixtures; 100% C#
test census dispositioned; 0 unapproved behavioral differences.

- At least 5,000 differential cases per pull request.
- At least 100,000 differential cases nightly.
- All mandatory property and proof tests passing.
- Every required CLI capability reproduced.
- All supported platforms build and pass acceptance tests.
- No missing modules or unexplained test exclusions.

The existing parity document estimates 2,087 C# test facts across the original baseline. Treat that
as a reference count, not a permanent constant: the census generator must establish the exact count
from the frozen commit, including parameterized test cases. A release passes only when all required
dimensions are green. There is no weighted average that allows a missing subsystem to be compensated
by additional tests elsewhere.

## 3. Immediate P0 requirements: runner and wire protocol

This is the first implementation priority because the existing Rust runner fails the independent
golden-fixture suite.

### 3.1 CLI argument compatibility

| Requirement | Surface | Acceptance |
|---|---|---|
| RUN-001 | --eval | Evaluate supplied source |
| RUN-002 | --file | Evaluate file, preserving source positions |
| RUN-003 | --stdin | Accept piped source and BOM handling |
| RUN-004 | Bare script path | Equivalent file execution |
| RUN-005 | --json, --text | Correct output modes |
| RUN-006 | --omit-functions, --omit-variables | Publish empty arrays without changing evaluation |
| RUN-007 | --functions [pattern] | Live registry query without evaluating a script |
| RUN-008 | --function <name> | Full builtin descriptor lookup |
| RUN-009 | --function-limit <n> | Bounded registry results and correct counts |
| RUN-010 | --cancel-after <ms> | Deadline, cancellation state, and budget accounting |
| RUN-011 | --print-budget <n> | Structured truncation with accurate metadata |
| RUN-012 | --plot-dir, --plot-file | Matching plot placement and naming |
| RUN-013 | --help, -h | Supported help surface |
| RUN-014 | Invalid arguments | Correct diagnostics, stderr, and exit status |

### 3.2 JSON envelope compatibility

- WIRE-001: Preserve protocolVersion, symbolicFormatVersion, and mathIrVersion with the same types and values as C#.
- WIRE-002: Match all success-envelope fields, including result, variables, functions, output, artifacts, revision, and timing data.
- WIRE-003: Match all error-envelope fields: code, category, message, recoverable, diagnostics, and applicable resolution information.
- WIRE-004: Preserve the distinction between an omitted optional JSON field, an explicit Null structured value, and an empty collection.
- WIRE-005: Match nested projections for every supported value kind.
- WIRE-006: Preserve declared enum types. SolveStatus.Partial and Completeness.Partial must remain distinguishable.
- WIRE-007: Preserve field order in typed records and ordering of published variables and functions.
- WIRE-008: Preserve human display, typed, and machine structured representations independently.
- WIRE-009: Capture print() output in the envelope; stdout must contain exactly one JSON document.
- WIRE-010: Publish the full timing contract, including structural duration and per-statement timing entries.
- WIRE-011: Preserve the C# runner's exit-code contract: success 0, script failure 1, usage error 2.
- WIRE-012: Preserve UTF-16 positions, including multibyte Unicode, surrogate pairs, BOM, and CRLF handling.
- WIRE-013: Match the C# serializer's non-ASCII escaping rules where byte compatibility is required.
- WIRE-014: Match structured truncation rules and differentiate depth, node, and precision caps.
- WIRE-015: Preserve artifact behavior for plots, including failure after a previous plotting statement.

Acceptance: all 20 existing fixtures pass structurally with only the reference's explicitly volatile
fields normalized; the original Lovelace.Run.Tests integration tests are reproduced or matched by
equivalent executable Rust tests; no unsupported or unknown structured value silently degrades to
plain text. The C# protocol document docs/symbolics/dsh-protocol.md is normative alongside the
fixtures.

## 4. Complete C# ecosystem coverage

A coverage ledger for every production project in the original solution. A missing dedicated Rust
crate is not automatically a defect: functionality may be consolidated, but the consolidation must
be explicit and its behavior tested.

| C# project | Required Rust-equivalent capabilities |
|---|---|
| Lovelace.Representation | Digit storage semantics, representation invariants, conversion behavior |
| Lovelace.Natural | Exact unsigned arithmetic, division, factorial, parsing, formatting |
| Lovelace.Integer | Signed arithmetic, normalization, exact division semantics |
| Lovelace.Rational | Normalized fractions, rational arithmetic, exact conversion |
| Lovelace.Real | Precision-aware exact decimals, repeating periods, transcendental operations |
| Lovelace.Complex | Arbitrary-precision complex values, fixed-width optimized representations, promotion |
| Lovelace.Array | Typed N-dimensional arrays and exact linear algebra |
| Lovelace.Abstractions | Modus plugin contracts, field abstractions, kernel registration |
| Lovelace.Statistics | Opt-in exact Real array-addition kernel |
| Lovelace.Suite | Language, execution engine, builtins, state, diagnostics, plotting |
| Lovelace.Symbolics | Symbolic algebra, calculus, solving, assumptions, canonicalization |
| Lovelace.MathIR | IR format, lowering, compilation, evaluation, batch execution |
| Lovelace.Dsp | Signal processing and all DSP plugin builtins |
| Lovelace.Console | Interactive REPL, history, commands, multiline input |
| Lovelace.Run | Complete programmatic runner and JSON protocol |
| Lovelace.Knowledge | Sampling, reduction, convergence, behavioral graph |
| Lovelace.Knowledge.Run | MGIR command-line interface and JSON protocol |
| Lovelace.Studio | Desktop-equivalent IDE, sessions, editor, inspectors, plotting |
| Lovelace.Proofs | Preserved Lean proofs and mapped Rust properties |

- ECO-001: Every row must resolve to one or more Rust modules, with tests linked to the corresponding C# capabilities.
- ECO-002: A complete parity run must prove that all required subsystems are present, even if their APIs have been reorganized.

### 4.1 Public API and integration compatibility

- ECO-003: Inventory all externally visible C# types and methods, not just language builtins.
- ECO-004: Identify external consumers: command-line scripts, applications, plugins, automation, and agent harnesses.
- ECO-005: Preserve the semantics of public operations, including defaults, error conditions, and value conversions.
- ECO-006: Provide equivalent Rust extension points for the C# abstractions that are intended to survive migration.
- ECO-007: Explicitly decide whether any .NET consumers require continued compatibility through a C ABI, FFI bridge, or C# wrapper.
- ECO-008: Inventory persisted formats and ensure existing files remain readable or have a verified migration path.

## 5. Numerical tower requirements

### 5.1 Natural and Integer

- NUM-001: All constructors and parsing forms match C# behavior.
- NUM-002: Canonical zero representation; no negative zero.
- NUM-003: Exact addition, subtraction, multiplication, division, and remainder.
- NUM-004: Signed division uses the same quotient and remainder convention as C#.
- NUM-005: Underflow, division by zero, invalid input, and overflow of bounded conversions produce equivalent failures.
- NUM-006: Exact power, factorial, and decimal shifting.
- NUM-007: Decimal parse/format round trips for arbitrarily large values.
- NUM-008: Identical results regardless of algorithm-selection threshold or worker count.
- NUM-009: Test every multiplication/division algorithm against an independent arithmetic oracle.
- NUM-010: Benchmark algorithm transitions, not merely steady-state performance.

For division, the core property must hold for every nonzero divisor: a = q*b + r, 0 <= r < b, for
unsigned operands, with the C#-defined signed convention tested separately.

### 5.2 Real and Rational

- REAL-001: Preserve arbitrary decimal precision and exactness metadata.
- REAL-002: Preserve repeating-decimal notation, including shortest-period canonicalization.
- REAL-003: Match exactness propagation through operations and conversions.
- REAL-004: Preserve rounding, truncation, guard-digit, and computation-cap semantics.
- REAL-005: Preserve precision changes during execution and their scope.
- REAL-006: Implement and verify sqrt, pi, powers, and supported transcendental functions.
- REAL-007: Test precision boundaries at 8, 16, 20, 50, 1,000, and 2,500 decimal places.
- REAL-008: Preserve the distinction between exact and inexact identity results.
- REAL-009: Implement normalized rational fractions with positive denominators and coprime numerator/denominator.
- REAL-010: Add direct Rational tests, since the C# project lacks a dedicated test suite.

### 5.3 Complex numbers

- CPLX-001: Match arithmetic and rendering across real and imaginary components.
- CPLX-002: Preserve arbitrary precision.
- CPLX-003: Support the C# fixed-width optimized representations where their observable performance contract matters.
- CPLX-004: Promote to arbitrary precision without silently saturating or losing exactness.
- CPLX-005: Verify optimized results against arbitrary-precision results.
- CPLX-006: Preserve real-to-complex coercion and domain behavior.

Different internal algorithms are acceptable. Different numerical answers, exactness metadata, or
public representations are not.

## 6. Arrays and linear algebra

- ARRAY-001: Preserve scalar, vector, matrix, and higher-rank array distinctions.
- ARRAY-002: Preserve shape, rank, strides, and element ordering.
- ARRAY-003: Match indexing, negative/invalid indices, partial indexing, and slicing.
- ARRAY-004: Implement reshape, transpose, squeeze, expand, flatten, concat, and append.
- ARRAY-005: Match broadcasting rules and their error messages.
- ARRAY-006: Match reductions, including axis behavior.
- ARRAY-007: Preserve typed-field arithmetic without silently falling back to machine floating point.
- ARRAY-008: Match dot products, matrix multiplication, determinants, inverses, trace, rank, and linear-system solving.
- ARRAY-009: Test zero-dimensional, empty, singular, non-square, and incompatible-shape cases.
- ARRAY-010: Verify matrix identities on generated exact inputs.

The audit's matrix_inverse_result and matrix_solve_result failures are high-priority correctness
investigations, not merely cosmetic projection differences.

## 7. Language and runtime semantics

### 7.1 Lexer and parser

- LANG-001: Match the complete token set, including Unicode edge cases.
- LANG-002: Match operator precedence and associativity.
- LANG-003: Match literal syntax, string interpolation, ranges, indexing, and slicing.
- LANG-004: Match whitespace, newline, comment, and escape handling.
- LANG-005: Preserve source positions in UTF-16 code units.
- LANG-006: Preserve parse errors, including codes, messages, and locations.
- LANG-007: Reject excessive AST/input depth in the same manner as the reference.

### 7.2 Execution engine

- ENG-001: Preserve every published value kind and its widening rules.
- ENG-002: Match variables, assignments, scopes, closures, recursion, and function calls.
- ENG-003: Preserve the _ last-result variable.
- ENG-004: Match loops, control flow, and statement evaluation order.
- ENG-005: Preserve per-engine precision and plugin state isolation.
- ENG-006: Implement consistent revision and state-publication semantics.
- ENG-007: Preserve partial execution, failure, and cancellation behavior.
- ENG-008: Match recursion, allocation, rendering, and evaluation budgets.
- ENG-009: Publish per-statement execution status and timing information.
- ENG-010: Produce deterministic output across repeated runs and different thread counts.
- ENG-011: Preserve the complete builtin help and capability catalogue.
- ENG-012: Test all language behaviors through public engine entry points, not only internal helpers.

### 7.3 Plugin architecture

- PLUG-001: Implement the functional equivalent of IModusPlugin and IModusContext.
- PLUG-002: Preserve explicit per-host plugin loading.
- PLUG-003: Preserve per-engine plugin state; assumptions from one engine must never leak into another.
- PLUG-004: Preserve builtin registration metadata: name, category, arity, parameters, signature, description, examples, and related functions.
- PLUG-005: Preserve typed-array field and kernel registration.
- PLUG-006: Keep value conversions owned by the host/kernel boundary.
- PLUG-007: Compare the complete C# and Rust registries automatically.
- PLUG-008: Verify plugin behavior under multiple concurrently active engines.

## 8. Symbolics and MathIR

### 8.1 Symbolic representation

- SYM-001: Preserve expression kinds, domains, assumptions, and exactness.
- SYM-002: Match canonical ordering and hashing-independent canonicalization.
- SYM-003: Preserve structural expression identity and interning behavior where observable.
- SYM-004: Preserve textual and canonical expression formats.
- SYM-005: Match simplification and rewriting with their applicable conditions.
- SYM-006: Preserve rewrite budgets, termination, trace ordering, and statuses.
- SYM-007: Match substitution, expansion, factoring, collection, and cancellation.
- SYM-008: Match differentiation, integration, limits, and series.
- SYM-009: Preserve solution sets, conditions, completeness, and domain restrictions.
- SYM-010: Match polynomial, nonlinear, system, and matrix-solving behaviors.
- SYM-011: Match assumptions, contradiction detection, and assumption isolation.
- SYM-012: Preserve all structured rich-result record types and field order.
- SYM-013: Verify the complete symbolic builtin registry, including metadata.
- SYM-014: Retain the existing SymPy differential oracle as an independent correctness check.

The two remaining documentation failures, sym/071 and sym/076, must either be fixed or recorded as
explicitly approved compatibility deviations. Their mathematical plausibility alone does not pass
strict parity.

### 8.2 MathIR

- MIR-001: Support the complete versioned MathIR node set and text format.
- MIR-002: Match lowering, optimization, type inference, and validation.
- MIR-003: Preserve common-subexpression handling and emitted ordering where published.
- MIR-004: Match lower, compile, compile_full, evalir, and evalir_batch.
- MIR-005: Preserve precision and cancellation behavior during evaluation.
- MIR-006: Verify direct execution against lowered-then-executed results.
- MIR-007: Reject malformed or unsupported IR identically.
- MIR-008: Add direct MathIR tests, addressing the original C# test-coverage gap.

## 9. DSP and specialized plugins

### 9.1 DSP

- DSP-001: Preserve all 16 C# DSP builtins and their signatures.
- DSP-002: Match convolution, DFT, FFT, filtering, and moving average.
- DSP-003: Match signal constructors, including impulse, step, cosine, exponential, power series, and noise.
- DSP-004: Match delay, scale, and complex component operations.
- DSP-005: Preserve argument coercion and argument-position error reporting.
- DSP-006: Match precision-dependent optimized paths and arbitrary-precision fallbacks.
- DSP-007: Preserve deterministic seeded noise generation.
- DSP-008: Preserve cancellation during long operations.
- DSP-009: Run the original DSP test suite and differential signal-processing benchmarks.
- DSP-010: Resolve the documented ambiguity in noise overloads explicitly, without silently changing the frozen C# behavior.

### 9.2 Statistics

Lovelace.Statistics is specifically an opt-in Real elementwise-addition kernel, not a comprehensive
statistics package.

- STAT-001: Preserve the plugin's identity and opt-in registration.
- STAT-002: Provide equivalent typed-array kernel dispatch.
- STAT-003: Preserve exact Real-field addition and active precision.
- STAT-004: Test fallback behavior and verify equivalence between the specialized and generic array paths.

## 10. Console and knowledge ecosystem

### 10.1 Interactive console

- CLI-001: Implement the interactive Lovelace REPL.
- CLI-002: Preserve multiline statement accumulation.
- CLI-003: Support editing, history, and keyboard navigation.
- CLI-004: Preserve interactive commands and help.
- CLI-005: Maintain engine state between commands.
- CLI-006: Match precision handling and _ result behavior.
- CLI-007: Match errors, cancellation, and output formatting.

### 10.2 MGIR/Knowledge

- KNOW-001: Port the observation-driven sampling pipeline.
- KNOW-002: Preserve deterministic seeded generation.
- KNOW-003: Implement observation canonicalization and behavioral-plane classification.
- KNOW-004: Implement reduction, convergence, boundaries, and frontiers.
- KNOW-005: Preserve graph persistence formats.
- KNOW-006: Implement the config, sample, reduce, converge, query, and help commands.
- KNOW-007: Match the JSON-over-stdio command protocol and exit codes.
- KNOW-008: Reproduce the seeded C# graph with seed 20240617.
- KNOW-009: Verify the Rust knowledge tool against the real Rust runner rather than a mock.
- KNOW-010: Compare graph outputs and convergence metrics against the frozen C# implementation.

## 11. Native desktop parity

The goal is not to reproduce ASP.NET endpoints. It is to preserve the capabilities users had
through Lovelace.Studio, while improving the native experience.

### 11.1 Core desktop behavior

- DESK-001: Deliver an actual native desktop shell on the agreed supported platforms.
- DESK-002: Preserve editor, workspace, graph, and logs panes.
- DESK-003: Preserve the symbolic inspector.
- DESK-004: Support multiple independent documents and engines.
- DESK-005: Preserve per-document precision and execution state.
- DESK-006: Execute scripts asynchronously without blocking input or rendering.
- DESK-007: Support cancellation with consistent committed-state behavior.
- DESK-008: Preserve incremental execution and visible statement status.
- DESK-009: Restore window layout, documents, recent files, and settings.
- DESK-010: Implement crash recovery for unsaved buffers.

### 11.2 Editor

- DESK-011: Syntax highlighting.
- DESK-012: Context-aware autocomplete using the live builtin registry.
- DESK-013: Bracket matching and automatic indentation.
- DESK-014: Find/replace, go-to-line, and editing shortcuts.
- DESK-015: Inline diagnostics with accurate source positions.
- DESK-016: Correct IME, clipboard, and Unicode handling.
- DESK-017: Per-tab undo/redo and selection preservation.
- DESK-018: Efficient editing of large scripts.

### 11.3 Inspector, plotting, and integration

- DESK-019: Browse variables with their kinds and structured projections.
- DESK-020: Browse functions and help metadata.
- DESK-021: Inspect symbolic expression trees, assumptions, traces, and compiled MathIR.
- DESK-022: Render SVG plots accurately.
- DESK-023: Support plot zoom, pan, fit, and export.
- DESK-024: Provide native open/save dialogs and drag-and-drop.
- DESK-025: Detect external file modifications.
- DESK-026: Support native keyboard accessibility and screen readers.
- DESK-027: Support system font scaling and high-DPI displays.
- DESK-028: Demonstrate the absence of a localhost HTTP listener and embedded web runtime.

### 11.4 Desktop acceptance

Headless application-state tests are necessary but insufficient. The release gate must include
automated widget interaction tests; real window-launch tests; screenshot comparisons at multiple DPI
settings; keyboard and IME tests; file persistence and crash-recovery tests; accessibility
verification; manual visual acceptance on every supported platform.

DECISION REQUIRED: The original specification requires Windows, Linux, and macOS. If Windows-only is
now acceptable, record that as a deliberate scope change. Otherwise, the current Win32 shell cannot
be the final desktop implementation.

## 12. Complete parity verification infrastructure

### 12.1 Frozen oracle

- TEST-001: Pin a C# oracle SHA/tag.
- TEST-002: Build and execute the C# reference in CI.
- TEST-003: Prevent untracked reference changes.
- TEST-004: Record approved oracle amendments.
- TEST-005: Archive oracle binaries or build inputs for reproducibility.

### 12.2 Test census and coverage manifest

- TEST-006: Generate parity/census.json from all original C# tests.
- TEST-007: Maintain parity/manifest.toml mapping every C# test to a Rust equivalent or approved exclusion.
- TEST-008: Resolve each mapped Rust test to an actual test that ran.
- TEST-009: Detect missing, duplicate, renamed, or ignored tests.
- TEST-010: Require explicit reasons for every exclusion.
- TEST-011: Disallow weakening assertions while porting tests.
- TEST-012: Report coverage separately by subsystem.
- TEST-013: Require the inventory to remain complete when the oracle changes.

### 12.3 Differential execution

A dedicated system submits equivalent inputs to both implementations.

- DIFF-001: Share the same generated input across both implementations.
- DIFF-002: Generate valid and invalid syntax, numerical boundaries, arrays, symbolic expressions, and errors.
- DIFF-003: Vary precision, budgets, plugin sets, and execution modes.
- DIFF-004: Compare exact results, kinds, metadata, diagnostics, and envelopes.
- DIFF-005: Ignore only explicitly documented volatile fields.
- DIFF-006: Minimize failing inputs automatically.
- DIFF-007: Save reproducible seeds and minimized examples.
- DIFF-008: Promote every verified mismatch into permanent regression coverage.
- DIFF-009: Run at least 5,000 comparisons per PR and 100,000 nightly.
- DIFF-010: Fail when either oracle cannot execute.

### 12.4 Property-based testing

Preserve the twelve property families from the original parity specification: arithmetic identities
and reconstruction; parse/format round trips; canonical decimal periods; precision-aware square-root
bounds; matrix algebra identities; array shape invariants; symbolic canonicalization independence;
rewrite correctness and termination; MathIR equivalence; structured projection round trips;
cross-thread and cross-run determinism.

### 12.5 Proof compatibility

- PROOF-001: Preserve the pinned Lean project and ensure it builds.
- PROOF-002: Map each relevant theorem to Rust implementation-boundary property tests.
- PROOF-003: Document the correspondence between the Rust numeric representation and the proof model.
- PROOF-004: Include exhaustive small-domain arithmetic verification in CI.

These Rust property tests do not, by themselves, constitute a formal proof of the Rust code.

### 12.6 Falsify the verification system

Before accepting the parity gate, deliberately inject faults.

- GATE-001: Change a decimal period boundary.
- GATE-002: Change a numeric widening rule.
- GATE-003: Break a Unicode source position.
- GATE-004: Remove an envelope field.
- GATE-005: Alter a diagnostic code.
- GATE-006: Change symbolic canonical ordering.
- GATE-007: Break final-digit rounding.
- GATE-008: Duplicate captured print() output.
- GATE-009: Remove builtin registry metadata.
- GATE-010: Disable a precision cap.
- GATE-011: Delete a supposedly ported test.
- GATE-012: Run the gate with an empty or incomplete corpus.

Every mutation must be detected by the expected gate. Otherwise, the gate itself is defective.

## 13. Performance, reliability, and Rust engineering quality

### 13.1 Performance

- PERF-001: Benchmark identical inputs and compare matching output digests.
- PERF-002: Use warmed executions and account for C# JIT effects.
- PERF-003: Report distributions rather than a single sample.
- PERF-004: Benchmark both implementations on the same machine and software environment.
- PERF-005: Measure arithmetic at small, medium, large, and algorithm-transition sizes.
- PERF-006: Include the original numeric, DSP, array, precision, and symbolic benchmark families.
- PERF-007: Profile allocations, peak memory, startup time, and latency.
- PERF-008: Record all results, inputs, build profiles, and hardware details.
- PERF-009: Meet the existing migration target of at least 80% of reference throughput, unless a specific deviation is approved.
- PERF-010: Never replace a correctness failure with a performance-based justification.

### 13.2 Quality and safety

- RUST-001: cargo fmt --check passes.
- RUST-002: Clippy passes with warnings treated as errors.
- RUST-003: All tests pass in debug and release profiles.
- RUST-004: No floating point is used in the exact numeric value path.
- RUST-005: Every unsafe block is isolated, justified, and reviewed.
- RUST-006: Dependencies are audited for license and security advisories.
- RUST-007: Public APIs have usable documentation and examples.
- RUST-008: Build and test workflows work from a clean checkout.
- RUST-009: Reproducible release artifacts are produced for every supported target.
- RUST-010: CI tests real binaries through their published entry points.

The current zero-dependency strategy can remain if desired, but it should be an explicit engineering
decision, not a permanent consequence of the original development environment's inability to access
crates.io.

## 14. DSH orchestration requirements

- HARNESS-001 — Immutable top-level objective: the migration's definition of done cannot be narrowed by an implementation agent.
- HARNESS-002 — Coverage-driven planning: select work from unsatisfied parity requirements.
- HARNESS-003 — Independent falsification: every significant implementation claim is checked independently.
- HARNESS-004 — Evidence provenance: claims must include commits, commands, exit codes, and test artifacts.
- HARNESS-005 — Distinguish progress from completion.
- HARNESS-006 — Persistent failures: unresolved differences remain in the ledger across rounds and context compaction.
- HARNESS-007 — No fabricated green: missing tools, skipped checks, and unavailable platforms are reported as unverified or failed.
- HARNESS-008 — Controlled scope changes: changes to a public contract or exclusion policy require user approval.
- HARNESS-009 — Reproducible benchmarking.
- HARNESS-010 — Final release auditor: a dedicated final gate evaluates the entire migration from the frozen source of truth.
- HARNESS-011 — Work preservation: each completed phase produces a reviewable commit or PR, test evidence, and an updated manifest.
- HARNESS-012 — No circular verification: a generated implementation and its generated tests cannot be the only two sources supporting a parity claim.

## 15. Recommended implementation order

| Phase | Priority | Deliverable | Exit gate |
|---|---|---|---|
| 0. Freeze and inventory | Critical | Pinned C# oracle, complete coverage manifest | All C# capabilities accounted for |
| 1. Protocol recovery | Critical | Complete lovelace-run compatibility | 20/20 fixtures and runner tests |
| 2. Differential harness | Critical | Independent cross-language comparison | 5,000-case PR slice clean |
| 3. Language and numeric gaps | High | Semantic fixes, runtime and array contracts | Required property and unit tests green |
| 4. Symbolics and MathIR | High | Canonical and structured-result equivalence | 214/214 corpus; differential cases clean |
| 5. Missing plugins and tools | High | DSP, Statistics, REPL, Knowledge | Full subsystem coverage and graph parity |
| 6. Desktop completion | High | Usable native IDE on agreed platforms | UI acceptance suite green |
| 7. Proofs and performance | High | Lean mappings and comprehensive benchmarks | Proof and performance gates green |
| 8. Release cutover | Critical | Reproducible Rust distribution | Full scorecard green |

Some phases can be developed in parallel. Their acceptance criteria cannot be weakened or considered
satisfied merely because downstream work has started.

## 16. Final definition of done

- Every required C# project and capability has a verified Rust equivalent or approved disposition.
- Every C# test obligation is mapped, and every mapped Rust test actually executes.
- All 214 documentation examples match.
- All 20 protocol fixtures match.
- Every required CLI flag and registry operation behaves equivalently.
- No unapproved differences remain in the differential test suite.
- All required property tests pass.
- All required Lean theorem mappings are verified.
- The complete Knowledge pipeline is operational and produces equivalent artifacts.
- The desktop meets the agreed functional and platform requirements.
- Performance thresholds and reliability budgets are satisfied or formally waived.
- The final scorecard runs automatically and fails closed.
- A clean checkout can reproduce the full verification process.
- The C# reference remains recoverable and reproducible after cutover.
