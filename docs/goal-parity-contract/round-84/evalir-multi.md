# Round 84 — the evalir-multi family

**Status: complete.** The four deferred `evalir-multi` census rows of
`Lovelace.Symbolics.Tests/DxStructuredResultsTests` are closed on the Rust side. The Rust
language surface now compiles a multi-parameter kernel with the reference's MathIR lowering and
typing, reads its own IR back through `IrProgram.Deserialize` + `IrEvaluator`, and answers or
refuses exactly as the oracle does. Every row's own bare-statement script produces an identical
normalised envelope on both runners (exit code included). No C# file, fixture, corpus file,
`parity/` file, desktop file or other document was edited; the four ids are listed in §1 and are
ready to flip to `ported` in `parity/manifest-map.json` by the round that owns that map.

## 1. Result — the four rows

| Census id (project/class/method#case) | Rust test | Contract |
|---|---|---|
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxStructuredResultsTests/EvalirBatch_AcceptsNestedRowValues#0` | `lovelace-lang::round84_evalir_multi::evalir_batch_accepts_nested_row_values` | a two-parameter kernel accepts `[[1,2],[3,4]]` and answers `[3, 7]`; its `result_type` is `unknown` |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxStructuredResultsTests/EvalirBatch_FlatInputNotAWholeNumberOfRows_IsRejected#0` | `lovelace-lang::round84_evalir_multi::evalir_batch_flat_input_not_a_whole_number_of_rows_is_rejected` | `[1,2,3]` for a 2-wide kernel is the documented whole-rows refusal |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxStructuredResultsTests/EvalirBatch_RowOrientedInput_EvaluatesRowByRow#0` | `lovelace-lang::round84_evalir_multi::evalir_batch_row_oriented_input_evaluates_row_by_row` | `[[1,2],[3,4]]` pairs row by row and answers `[3, 13]` |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxStructuredResultsTests/EvalirBatch_RowWidthMismatch_IsRejectedWithTheDocumentedShape#0` | `lovelace-lang::round84_evalir_multi::evalir_batch_row_width_mismatch_is_rejected_with_the_documented_shape` | three values in a row of a 2-parameter kernel is the documented row-width refusal |

The four ids are exactly the family listed by rounds 45/50/66/75 as `evalir-multi` (4 rows);
round-75 measured them at `compile_full(...).result_type` and
`evalir_batch(compile(x*y+1,[x,y]), ...)`.

## 2. Method and probes

The row's own script (the same call the C# body makes, without the `print` wrapper so the last
statement IS the call) was executed through both binaries with the differential's argv:

```powershell
dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --eval <row> --omit-functions
rust/target/release/lovelace-run.exe --eval <row> --omit-functions
```

Envelopes were compared structurally with only the four volatile fields replaced
(`revision`, `elapsed`, `elapsedTime`, `timings`; key presence structural), exactly as
`parity/diff/compare.mjs` does. The before values below were measured on the pre-fix runner in
this session (and match the messages recorded by rounds 45, 66 and 75); the after values are the
post-fix runs. For the two failing rows the diagnostic position differs from a `print(...)`
wrapper (the oracle attributes a builtin exception to the *statement* start, Rust to the call
site), so the row's bare-statement form is used: there the two positions coincide and the full
envelope is identical.

## 3. Before / after, per row

| Row | Probe | Before (oracle vs rust) | After (both) |
|---|---|---|---|
| `EvalirBatch_AcceptsNestedRowValues#0` | `compile_full(x + y, [x, y])`, field `result_type` (`$.result.structured.fields[name=result_type].value.value`) | oracle **"unknown"** vs rust **"Integer"** | **"unknown"**; full normalised envelope identical |
| `EvalirBatch_AcceptsNestedRowValues#0` | `evalir_batch(compile_full(x + y, [x, y]), [[1, 2], [3, 4]], 40)` | oracle exit **0**, `[3, 7] (Vector)` vs rust exit **1** `InvalidOperation/DomainError` **"evalir supports one free symbol."** | exit **0**, `[3, 7] (Vector)`; envelope identical |
| `EvalirBatch_RowOrientedInput_EvaluatesRowByRow#0` | `evalir_batch(compile(x*y + 1, [x, y]), [[1, 2], [3, 4]], 40)` | oracle exit **0**, `[3, 13] (Vector)` vs rust exit **1** "evalir supports one free symbol." | exit **0**, `[3, 13] (Vector)`; envelope identical |
| `EvalirBatch_RowWidthMismatch_IsRejectedWithTheDocumentedShape#0` | `evalir_batch(compile(x*y + 1, [x, y]), [[1, 2, 5], [3, 4, 6]], 40)` | oracle message **"evalir_batch(): kernel expects 2 parameters per row; row 0 contains 3."** vs rust **"evalir supports one free symbol."** | that exact message, `InvalidOperation/DomainError`, exit **1**; envelope identical |
| `EvalirBatch_FlatInputNotAWholeNumberOfRows_IsRejected#0` | `evalir_batch(compile(x*y + 1, [x, y]), [1, 2, 3], 40)` | oracle message **"evalir_batch(): kernel expects 2 parameters per row; got 3 values, which is not a whole number of rows."** vs rust **"evalir supports one free symbol."** | that exact message, `InvalidOperation/DomainError`, exit **1**; envelope identical |

A second mapping probe from the same family also matches: `evalir(compile(x - y, [x, y]), [10, 3], 40)`
is `7 (Integer)` while `evalir(compile(x - y, [y, x]), [10, 3], 40)` is `-7 (Integer)`, so the
values are bound by POSITION in declaration order, not by free-symbol name.

## 4. The fix

`rust/lovelace-lang/src/builtins.rs`

* `compile`/`compile_full` no longer build the record from the ad-hoc `ir_text` emitter. They call
  the MathIR compiler (`crate::mathir::compile_ordered`), which enforces the reference's
  **"Compilation requires at least one parameter."** contract, lowers, and runs
  `IrTyping::validate`. `result_type` is now the FORWARD inference's last node type
  (`IrTyping::infer`) — `unknown` when it is unconstrained, never a guess from the symbolic domain.
* `lower` uses `IrProgram::lower_ordered` and trims the serializer's trailing newline.
* `evalir`/`evalir_batch` no longer parse the IR into a single-symbol polynomial. They read the
  text (or a CompilationResult's `ir` field) through `IrProgram::deserialize` — the same
  version gate + `IrTyping::validate` the reference runs — then evaluate with
  `IrEvaluator::evaluate_at` / `evaluate_batch_at` and publish `Num::into_value`. Multi-parameter
  kernels therefore exist at all.
* The list/row contract is the reference's `BatchRows`: `[]` → **"values must not be empty."**;
  rank > 2 → **"a flat list or a list of rows (rank 2); got rank N."**; rank 2 → row width must
  equal the kernel width (**"kernel expects N parameters per row; row 0 contains M."**); a flat
  list must hold a whole number of rows (**"… got K values, which is not a whole number of
  rows."**); a zero-width flat list is the reference's `DivisionByZero` /
  **"Attempted to divide by zero."**. `evalir`'s scalar arity message is
  **"evalir(): kernel expects N parameters; got K values."**, and the scalar-cast refusal keeps the
  reference's `InvalidArgument/TypeMismatch` attribution.
* The dead single-symbol path and its helpers (`parse_ir_text`, the builtin-local `ir_text`/
  `emit_ir`, `result_type`/`polynomial_is_integral`) were removed.

`rust/lovelace-lang/src/mathir.rs`

* `IrProgram::lower_ordered` / `compile_ordered`: the language's lowering sorts Add/Multiply
  children with `canonical_order` (the reference's storage order) instead of this crate's
  display-ordered `Sym` storage; the library `lower`/`compile` stay unchanged for the existing
  MathIR mirrors. This is what keeps `lower`'s published text byte-identical to the oracle
  (e.g. the `sym/071` Horner listing and `lower(x^2 + 1, [x])`).
* Duplicate parameter names bind to their **LAST** declaration position, as the reference's
  name→index dictionary does; validation then refuses the out-of-range operand.
* The ordered lowering also emits the reference's BINARY left-fold for sums/products and
  `Reciprocal`/`Sqrt` for `x^-1`/`x^(1/2)`. Before this round the language emitted
  `Add 0 0,1,2` for `x+y+z` and `PowInt` for `1/x`, which `IrProgram::deserialize` would
  reject; the new text is the same text the oracle publishes and the same text the reader accepts.

## 5. Boundaries

| Boundary | Probe | Oracle | Rust before | Rust after |
|---|---|---|---|---|
| zero parameters | `compile(x, [])`, `compile_full(x, [])` | `InvalidArgument/TypeMismatch` "Compilation requires at least one parameter. (Parameter 'parameters')" | exit 0: `CompilationResult` with `parameters: []`, `result_type: Integer` | the oracle refusal |
| zero parameters | `evalir(lower(x-x, []), [], 40)` | `0 (Integer)` | "evalir needs numeric inputs." | `0 (Integer)` |
| zero parameters | `evalir_batch(lower(x-x, []), [7], 40)` | `DivisionByZero/DomainError` "Attempted to divide by zero." | exit 0, `[0]` | the oracle refusal |
| zero parameters | `evalir_batch(lower(x-x, []), [], 40)` | "evalir_batch(): values must not be empty." | exit 0, `[]` | the oracle refusal |
| one parameter | `evalir(compile(x^2+1,[x]), [2], 40)`; `evalir_batch(..., [1,2,3], 40)`; `evalir_batch(..., [[1],[2]], 40)` | `5`; `[2,5,10]`; `[2,5]` | same | same (guard) |
| one parameter | `compile_full(x^2+1,[x]).result_type` | `Integer` | `Integer` | `Integer` |
| duplicate names | `lower(x, [x, x])` | `#!mathir 2\nparam x\nparam x\nParameter 0 1` | `…Parameter 0 0` | the oracle text |
| duplicate names | `compile(x, [x, x])`, `compile_full(x, [x, x])`, `evalir(lower(x, [x, x]), [2, 3], 40)` | `InvalidOperation/DomainError` "IR validation: node 0 (Parameter) references out-of-range operand 1." | exit 0 (compile accepted) / "evalir needs numeric inputs." | the oracle refusal |

All boundary probes above, in their bare-statement form, also produce identical normalised
envelopes on both runners after the fix.

## 6. Tests

New module `rust/lovelace-lang/src/round84_evalir_multi.rs` (8 tests), registered in
`rust/lovelace-lang/src/lib.rs`:

1. `evalir_batch_accepts_nested_row_values` — row #1 (row-shaped batch, `result_type`, and the
   contrast that a constant-bearing kernel still infers `Integer`).
2. `evalir_batch_row_oriented_input_evaluates_row_by_row` — row #2 (row-shaped and flat
   whole-row forms).
3. `evalir_batch_row_width_mismatch_is_rejected_with_the_documented_shape` — row #3 (exact
   sentence, code and category).
4. `evalir_batch_flat_input_not_a_whole_number_of_rows_is_rejected` — row #4 (exact sentence,
   code and category).
5. `evalir_multi_parameter_binds_values_in_declaration_order` — position binding both ways, the
   single-row list form and the scalar arity refusal.
6. `evalir_zero_parameter_boundary_is_the_compilation_refusal_and_a_constant_kernel` — the zero
   boundary: compile refusal, constant zero-parameter kernel, zero-width batch refusals.
7. `evalir_one_parameter_boundary_keeps_the_single_lane_contract` — the one-parameter boundary.
8. `evalir_duplicate_parameter_names_bind_last_and_compilation_refuses` — duplicate-name lowering
   and the validation refusal through compile, compile_full and evalir.

No test expectation was weakened: every expected string/value is the oracle's measured answer, and
the row tests assert the C# body's own values (`[3, 7]`, `[3, 13]`, both exact refusal
sentences, and `kind == Record`).

## 7. Gates

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Focused | `cargo test -p lovelace-lang round84_evalir_multi` | **8 passed; 0 failed; 0 ignored** (1,608 filtered out) | 0 |
| Workspace | `cargo test --workspace` | **5,307 passed; 0 failed; 1 ignored** across 21 result blocks. This round adds 8 tests: the first workspace run mid-session observed 5,298 (the 5,290 baseline + these 8), and the final run observed 5,307 because concurrent lanes landed 9 more tests in the shared tree; every block is `ok` and this round's 8 are present | 0 |
| Corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS` | 0 |
| CLI | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS` | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize --concurrency 10` | generated **200 match / 0 mismatch / 0 error**; regressions **1,758 match / 0 mismatch / 0 error**; `harness exit: 0`; run `parity/diff/runs/20261112T153742Z-seed1` | 0 |
| Clippy | `cargo clippy -p lovelace-lang -p lovelace-run --all-targets -- -D warnings` | `Finished`, no diagnostics | 0 |
| Format | `cargo fmt -p lovelace-lang -p lovelace-run -- --check` | clean | 0 |

`monitor.json` in the differential run records
`generated {total: 200, match: 200, mismatch: 0, error: 0}` and
`regressions {total: 1758, match: 1758, mismatch: 0, error: 0}`; the error-category cells are
24/24 generated and 326/326 replayed.

## 8. Scope and changed files

* `rust/lovelace-lang/src/builtins.rs` — compile/compile_full/lower/evalir/evalir_batch arms and the
  evalir helpers; the dead single-symbol IR path removed.
* `rust/lovelace-lang/src/mathir.rs` — last-position parameter binding, `lower_ordered` /
  `compile_ordered`, ordered n-ary emission.
* `rust/lovelace-lang/src/round84_evalir_multi.rs` — new (8 tests).
* `rust/lovelace-lang/src/lib.rs` — one `#[cfg(test)] mod round84_evalir_multi;` declaration
  plus its doc comment.
* `docs/goal-parity-contract/round-84/evalir-multi.md` — this report.

No `.cs` file, fixture, corpus file, `parity/manifest-map.json`, `parity/manifest.toml`, desktop
file or other document was edited. Running the gates wrote their own artifacts under
`parity/diff/runs/20261112T153742Z-seed1` and `rust/target/tmp`; no tracked `parity/` file
changed.

## 9. Reproduction

```powershell
$repo = 'C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME = "$repo\.toolchain\rustup"; $env:CARGO_HOME = "$repo\.toolchain\cargo"
$env:PATH = "$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP = "$repo\rust\target\tmp"; $env:TEMP = $env:TMP
Set-Location $repo

# one row, both runners (bare-statement form; normalise revision/elapsed/elapsedTime/timings)
$row = 'x = symbol("x"); y = symbol("y"); evalir_batch(compile(x*y + 1, [x, y]), [[1, 2], [3, 4]], 40)'
dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --eval $row --omit-functions
rust/target/release/lovelace-run.exe --eval $row --omit-functions

# gates
Set-Location "$repo\rust"
cargo test -p lovelace-lang round84_evalir_multi
cargo test --workspace
cargo run --release -p lovelace-corpus
cargo clippy -p lovelace-lang -p lovelace-run --all-targets -- -D warnings
cargo fmt -p lovelace-lang -p lovelace-run -- --check
Set-Location $repo
node parity/tools/gate-fixtures.mjs
node parity/tools/gate-cli.mjs
node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize --concurrency 10
```
