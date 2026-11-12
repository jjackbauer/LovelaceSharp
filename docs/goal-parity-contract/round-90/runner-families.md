# Round 90 — five deferred runner/DSP families (13 rows)

**Status: fixed and pinned.** The 13 deferred rows across `PlotFileWriteRefusal` (3), `Dsp` (3),
`DenseArrayComplex` (3), `IntegerToString` (2) and `Parser` (2) now have the missing
production surface and one Rust test per row asserting the C# case's own observable. Measured
2026-11-12 (America/Sao_Paulo) against `Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll` and
`rust/target/release/lovelace-run.exe`. No manifest row was relabelled: this lane's scope
excludes `parity/`, so the parent applies the mappings listed in section 1.

## 1. The 13 ids and their mapped tests

| # | Census id | Mapped Rust test |
|---|---|---|
| 1 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.PlotFileWriteRefusalTests/TheSuiteDeclaresThePlotFileWriteRefusalTheRunnerClassifies#0` | `lovelace-lang::round90_runner_families::the_suite_declares_the_plot_file_write_refusal_the_runner_classifies` |
| 2 | `.../PlotFileUnderAMissingDirectory_IsRefusedByType#0` | `lovelace-run::round90_runner_families::plot_file_under_a_missing_directory_is_refused_by_type` |
| 3 | `.../EmptyPlotFile_IsRefusedByType#0` | `lovelace-run::round90_runner_families::empty_plot_file_is_refused_by_type` |
| 4 | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.DspTests/Sum_GivenImpulseAndStep_AddsPointwise#0` | `lovelace-lang::round90_runner_families::sum_given_impulse_and_step_adds_pointwise` |
| 5 | `.../Product_GivenImpulseAndStep_MultipliesPointwise#0` | `lovelace-lang::round90_runner_families::product_given_impulse_and_step_multiplies_pointwise` |
| 6 | `.../Sequence_GivenLengthMismatch_ThrowsArgumentException#0` | `lovelace-lang::round90_runner_families::sequence_given_length_mismatch_throws_argument_exception` |
| 7 | `Lovelace.Complex.Tests/Lovelace.Complex.Tests.DenseArrayComplexTests/DenseArrayOfComplex_GivenConstruction_ReportsComplexDTypeAndElementType#0` | `lovelace-lang::round90_runner_families::dense_array_of_complex_given_construction_reports_complex_dtype_and_element_type` |
| 8 | `.../DenseArrayOfComplex_GivenElements_GetElementRoundTrips#0` | `lovelace-lang::round90_runner_families::dense_array_of_complex_given_elements_get_element_round_trips` |
| 9 | `.../DenseArrayOfComplex_GivenContiguous_AsSpanIterates#0` | `lovelace-lang::round90_runner_families::dense_array_of_complex_given_contiguous_as_span_iterates` |
| 10 | `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerToStringTests/TryFormat_GivenSufficientBuffer_WritesRepresentationAndReturnsTrue#0` | `lovelace-lang::round90_runner_families::try_format_given_sufficient_buffer_writes_representation_and_returns_true` |
| 11 | `.../TryFormat_GivenInsufficientBuffer_ReturnsFalse#0` | `lovelace-lang::round90_runner_families::try_format_given_insufficient_buffer_returns_false` |
| 12 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ParserTests/Parse_GivenChainedAssignment_RightAssociative#0` | `lovelace-lang::round90_runner_families::parse_given_chained_assignment_right_associative` |
| 13 | `.../Parse_GivenUnaryPlus_ReturnsUnaryExprPlus#0` | `lovelace-lang::round90_runner_families::parse_given_unary_plus_returns_unary_expr_plus` |

## 2. Probes (oracle first)

Every runner probe used `--omit-functions` and the differential's normalisation
(`revision`, `elapsed`, `elapsedTime`, `timings` removed at any depth; key presence is
structural). The plot probes used the caller paths below; the parser probes used the C# test's
own source.

| Row | Script / argv | Oracle before | Rust before | Rust after |
|---|---|---|---|---|
| 2 | `plot([1, 2, 3])` `--plot-dir rust/target/tmp/plot-missing-dir --plot-file nope/x.svg` | `PlotFileError/TypeMismatch`, message `Cannot write the plot file 'rust/target/tmp/plot-missing-dir\nope/x.svg': Could not find a part of the path 'C:\...\nope\x.svg'.` | same code/category, message `...: The system cannot find the path specified. (os error 3)` | **byte-identical to the oracle**; full normalised envelope diff NONE |
| 3 | `plot([1, 2, 3])` `--plot-dir rust/target/tmp/plot-empty-dir --plot-file ""` | `PlotFileError/TypeMismatch`, message `Cannot write the plot file 'rust/target/tmp/plot-empty-dir': Access to the path 'C:\...\plot-empty-dir' is denied.` | same code/category, message `Cannot write the plot file 'rust/target/tmp/plot-empty-dir\': The system cannot find the path specified. (os error 3)` (note the trailing separator and OS text) | **byte-identical to the oracle**; full normalised envelope diff NONE |
| 2/3 success control | `--plot-file ok.svg` | exit 0, Text path, SVG artifact | exit 0, same | unchanged; full normalised envelope diff NONE |
| 12 | `x = y = 5` | exit 0, result `5 (Natural)`, variables `_`, `x`, `y` | exit 1 `ParseError`: `Expected ';' or end of input but found '=' at position 6.` | exit 0, result `5 (Natural)`, variables `_`, `x`, `y`; normalised diff NONE |
| 12 (same defect) | `abs(x = -5)` | exit 0, result `5 (Integer)`, `x = -5` | exit 1 `ParseError`: `Expected 'RParen' but found '=' at position 6.` | matches the oracle |
| 13 | `+5` | exit 0, `5 (Natural)` | exit 0, `5 (Natural)` (wire already matched) | unchanged; the missing piece was the AST node |
| 4/5 | `impulse(2) + step(2)` / `impulse(2) * step(2)` | exit 1 `InvalidOperation/DomainError` (operator not supported for Complex) | exit 1, different Complex-operator prose | unchanged (no runner surface names the signal combinators); the library row is pinned directly |
| 7-9 | `complex(1, 2)` | exit 1 `InvalidArgument/TypeMismatch`: `complex(): expected 0 arguments; got 2.` | identical | unchanged; the library row is pinned directly |
| 10/11 | no runner surface (Integer.TryFormat is a library buffer API) | — | — | pinned directly |

The Dsp, DenseArray and IntegerToString rows are the UNRUNNABLE class: the C# case constructs a
library value the language surface has no constructor for. The probes above show that absence on
both runners; the tests assert the C# body's values through the new library API instead of
inventing a script the oracle does not accept.

## 3. Production fixes

1. **Parser — assignment is an expression level, unary plus is an AST node**
   (`rust/lovelace-lang/src/syntax.rs`). `Parser::expression` now delegates to a new
   right-associative `assignment` level (the reference's `ParseAssignment`), so `x = y = 5`,
   `abs(x = -5)` and `(x = 5)` parse exactly as the oracle does. `Expr::Plus` was added and is
   produced by both the `ParsePower` sign branch and `ParseUnary`; evaluation is the identity.
   `ast_depth` counts the new node. The one existing Rust-specific assertion that `+2^2` was a
   bare `Binary` was **strengthened** to `Plus(Pow)`, which is the reference's AST.
2. **Typed plot-file refusal** (new `rust/lovelace-lang/src/plot_file.rs`). `write_plot_file`
   performs the reference's `File.WriteAllText` site and returns a public
   `PlotFileWriteException` carrying the requested path, the absolute path, the classified
   failure and the CLR inner sentence; `to_lang_error` is the single PlotFileError/TypeMismatch
   classification. `builtins::builtins_plot` now routes through it. An empty file name is the
   directory itself (no trailing separator), which is the access-denial case; a missing parent is
   the path-not-found case.
3. **Signal library** (new `rust/lovelace-lang/src/signals.rs`). The reference's `ISignal`,
   `Impulse`, `Step`, finite `Sequence` (with the length-mismatch `SequenceError`), `Sum` and
   `Product`, all over the arbitrary-precision `Complex`.
4. **Typed dense arrays** (new `rust/lovelace-array/src/dense.rs`, re-exported by the crate).
   `DType`, `Precision` and the packed `DenseArray<T>` with `dtype`/`element_type`/`rank`/
   `shape`/`numel`/`is_contiguous`/`get_element`/`as_span`/`as_contiguous`.
5. **TryFormat** (new `rust/lovelace-lang/src/display_format.rs`). The `TryFormat` trait for
   `Int`: `Some(count)` writes the canonical decimal, `None` writes nothing (the reference's
   false/0-written branch). The trait lives in the language crate because this lane's scope
   excludes `lovelace-int`; the method call reads as the reference's `n.TryFormat(buffer)`.

## 4. Per-row before / after

| Row | Before (Rust) | After (Rust) | Oracle |
|---|---|---|---|
| 1 | no `PlotFileWriteException`; the write error was assembled inline in `builtins` | public typed exception + `to_lang_error` = PlotFileError/TypeMismatch | declares the type and the runner classification |
| 2 | OS sentence, no typed API | typed path-not-found, message byte-identical | matches |
| 3 | trailing-separator path, OS sentence, no typed API | typed access-denied, directory path, message byte-identical | matches |
| 4 | no `Sum` signal | `Sum(Impulse, Step).get(0) = 2`, `get(1) = 1` | same |
| 5 | no `Product` signal | `Product(Impulse, Step).get(0) = 1`, `get(1) = 0` | same |
| 6 | no `Sequence` | `Sequence::new(0, 2, [1, 1])` refuses (3 required, 2 supplied) | `ArgumentException` |
| 7 | no `DType`/`DenseArray` | dtype Complex, element type Complex, numel 2, contiguous | same |
| 8 | no typed array | `get_element(0) = 1+2i`, `get_element(1) = 3-4i` | same |
| 9 | no typed array | span `[1+2i, 3-4i]` | same |
| 10 | `Int::to_decimal` only | 10-char buffer writes `-42`, 3 written | true, 3 written |
| 11 | no buffer contract | 3-char buffer refuses, nothing written | false, 0 written |
| 12 | `ParseError` at the second `=` | `Assign(x, Assign(y, 5))`; evaluation 5, x and y set | same |
| 13 | plus consumed as identity, no AST node | `Expr::Plus(5)` | `UnaryExpr(UnaryOp.Plus)` |

## 5. Tests

The 13 row tests are listed in section 1 (11 in
`rust/lovelace-lang/src/round90_runner_families.rs`, 2 in
`rust/lovelace-run/src/round90_runner_families.rs`). Supporting production tests added with the
new modules: three `DenseArray` tests in `dense.rs`, three `PlotFileWriteException` tests in
`plot_file.rs`, two `TryFormat` tests in `display_format.rs`. No assertion was weakened; the
one pre-existing AST assertion in `suite_surface_round14.rs` was strengthened from "a Binary
node" to the reference's `Plus(Pow)` shape.

## 6. Gates (observed 2026-11-12)

| Step | Command | Observed | Exit |
|---|---|---|---|
| Workspace suite | `cargo test --workspace` | **5,428 passed, 0 failed, 1 ignored** over 21 binaries; own crates alone: 2,117 passed (array 61, lang 1,636, run 419, integration 1) | 0 |
| Documentation corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS` | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS` | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1` | generated **200 match / 0 mismatch / 0 error**; replayed **1758 match / 0 mismatch / 0 error**; `stillMismatching=0` | 0 |
| Clippy (own crates) | `cargo clippy -p lovelace-lang -p lovelace-array -p lovelace-run --all-targets -- -D warnings` | clean | 0 |
| Format (own files) | `rustfmt --edition 2024 --check <our files>` | clean | 0 |

## 7. Scope and the shared-tree conflict

Changed files: `rust/lovelace-array/src/lib.rs` and the new `dense.rs`;
`rust/lovelace-lang/src/lib.rs`, `syntax.rs`, `builtins.rs`, `suite_lex_parse_batch.rs`,
`suite_surface_round14.rs` and the new `signals.rs`, `plot_file.rs`, `display_format.rs`,
`round90_runner_families.rs`; `rust/lovelace-run/src/main.rs` and the new
`round90_runner_families.rs`. No `.cs`, fixture, corpus, `parity/`, `lovelace-desktop` or
symbolics file was edited.

**Shared-tree conflict (reported, not overwritten).** While this round ran, the
relations/symbolics lane added `rust/lovelace-symbolics/src/round90_symbolic_families.rs` and
`rust/lovelace-lang/src/round90_symbolic_families.rs`, and edited `rust/lovelace-lang/src/lib.rs`
and `rust/lovelace-lang/src/builtins.rs`. Our edits are disjoint (our module declaration and
`Expr::Plus` arm/ast-depth arm in `lib.rs`; the plot-write block at `builtins.rs:4992`); their
declaration and their `builtins.rs:1611` change were preserved. The first full workspace run
failed only on **their** in-flight `Expr::canonical_parse` reference in
`lovelace-symbolics/src/round90_symbolic_families.rs` (exit 101); after they fixed it, the
retry was green: **5,428 passed / 0 failed / 1 ignored over 21 binaries** (exit 0). Crate-wide
`cargo fmt` cannot be used while their new file is unformatted, so format evidence is per-file
for this lane's files (`rustfmt --check` clean on all of them).

Rows left blocked by this lane: **none of the 13**. Rows 1-13 are fixed; the parent only needs to
apply the section-1 mappings to `parity/manifest.toml`.
