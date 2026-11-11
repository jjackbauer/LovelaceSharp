# Round 48 - the RecordSchemas surface and the nine rows it unblocks

**Status: complete.** `rust/lovelace-run` now carries an additive record-schema registry
(`src/schemas.rs`) that is the Rust counterpart of `Lovelace.Suite.RecordSchemas`, plus nine ports
of `Lovelace.Suite.Tests.RecordSchemaTests` in `src/record_schema_batch.rs`. The nine census rows
listed in section 4 are now decidable by a passing Rust test. No `.cs` file, fixture, corpus
example, parity mapping or other documentation file was edited.

## 1. What round 45 recorded, and what was read

Round 45 deferred 9 rows under one missing API:

> 9 | RecordSchemas.For/Validate record-schema registry

The C# table is `Lovelace.Suite/RecordSchemas.cs` (139 lines): two records
(`RecordFieldSchema(Name, Kind)` and `RecordSchema(TypeName, Fields)`), a static registry
(`RecordSchemas.All`, `For`, `Validate`) with **21** entries, and the per-schema
`FieldNames`/`Validate` members. The contract is the test file
`Lovelace.Suite.Tests/RecordSchemaTests.cs` (338 lines, 9 census cases).

The registry is data, not reflection. Its observable surface is:

| C# member | Rust counterpart | Used by the nine rows |
|---|---|---|
| `RecordFieldSchema.Name` | `RecordFieldSchema::name: &'static str` | `SolverSchemas`, `DiagnosticSchema` (through `Fields`) |
| `RecordFieldSchema.Kind` | `RecordFieldSchema::kind: &'static str` | same |
| `RecordSchema.Fields` | `RecordSchema::fields: &'static [RecordFieldSchema]` | `SolverSchemas`, `DiagnosticSchema` |
| `RecordSchema.FieldNames` | `RecordSchema::field_names() -> Vec<&'static str>` | exercised through `Validate`; asserted directly by the ports |
| `RecordSchema.Validate(RecordValue)` | `RecordSchema::validate(&Json) -> Vec<String>` | `Validate_ReportsMissingAndUndeclaredFields` |
| `RecordSchemas.All` | `RecordSchemas::ALL` + `all()` | not directly asserted by these rows (registry completeness only) |
| `RecordSchemas.For(string) -> RecordSchema?` | `RecordSchemas::for_type(&str) -> Option<&'static RecordSchema>` | `UnknownRecordType`, `Validate`, `EveryProducedRecord`, both matrix theories |
| `RecordSchemas.Validate(RecordValue) -> IReadOnlyList<string>` | `RecordSchemas::validate(&Json) -> Vec<String>` | `UnknownRecordType`, `EveryProducedRecord`, both matrix theories |

`RecordValue`/`RecordField` are the C# runtime record types the tests construct and inspect. The
Rust equivalent consumes the runner's **already published** structured record value
`{"kind":"Record","type":...,"fields":[{"name":...,"value":...}]}`; no new runtime value type was
introduced, so the wire envelope is unchanged.

## 2. The additive surface

New file `rust/lovelace-run/src/schemas.rs` (371 lines), registered in `main.rs` as
`mod schemas;` (outside `cfg(test)`, with `#[allow(dead_code)]` because only the ports consume it
today). New file `rust/lovelace-run/src/record_schema_batch.rs` (ports), registered as
`#[cfg(test)] mod record_schema_batch;`.

| Item | C# counterpart | Contract |
|---|---|---|
| `RecordFieldSchema { name, kind }` + `new` | `RecordFieldSchema` | Field name exactly as emitted (snake_case) and the declared kind string, verbatim. |
| `RecordSchema { type_name, fields }` + `new` | `RecordSchema` | Type name and fields in wire order. |
| `RecordSchema::field_names()` | `FieldNames` | Required names, in declaration order. |
| `RecordSchema::validate(&Json)` | `Validate(RecordValue)` | Type-mismatch, missing-declared and undeclared-field violations, in the reference's order and wording. Empty means conformant. |
| `RecordSchemas::ALL`, `all()` | `All` | The 21 registered schemas in the reference's declaration order. |
| `RecordSchemas::for_type(name)` | `For` | `None` for an unregistered type. |
| `RecordSchemas::validate(&Json)` | `Validate` | Delegates to the known schema; **empty** for an unknown type. |

The 21 schemas and every field name, order and kind string are byte-for-byte the C# table:
`SolveResult`, `Solution`, `SolutionFamily`, `SystemSolveResult`, `MatrixInverseResult`,
`MatrixSolveResult`, `SystemSolution`, `Binding`, `TransformResult`, `RewriteStep`, `LimitResult`,
`IntegrationResult`, `OptimizationResult`, `CompilationResult`, `ParameterInfo`, `Diagnostic`,
`Inspection`, `DomainCondition`, `FiniteCondition`, `PredicateCondition`, `IntervalCondition`.

Required/optional and null semantics are the declared kind strings, as in C#: every named field is
required to be present; a value may be Null only where the kind says so (`Boolean|Null`,
`Symbolic|Null`, `Record|Null`, `Domain|Null`, `Text|Null`, `Integer|Null`, `Array|Null`). As in
the reference, `validate` checks presence and undeclared names only - it does not enforce that a
present value matches its declared kind. The frozen Diagnostic form is exactly `code`, `category`,
`message`, `recoverable`, `location` (`Record|Null`), `details` (`Array`).

## 3. The nine Rust tests

All live in `rust/lovelace-run/src/record_schema_batch.rs` and drive the runner's own
`parity_batch::envelope` / `run_args` pair, so the records validated are the records the binary
publishes.

| C# case | Rust test |
|---|---|
| `DiagnosticSchema_DeclaresTheSixFrozenFields` | `record_schema_batch::diagnostic_schema_declares_the_six_frozen_fields` |
| `EveryProducedRecord_ConformsToItsDeclaredSchema` | `record_schema_batch::every_produced_record_conforms_to_its_declared_schema` |
| `SingularMatrix_...` #0 (`inv_full`) | `record_schema_batch::singular_matrix_inv_full_is_a_complete_no_solutions_carrying_a_structured_diagnostic` |
| `SingularMatrix_...` #1 (`linsolve_full`) | `record_schema_batch::singular_matrix_linsolve_full_is_a_complete_no_solutions_carrying_a_structured_diagnostic` |
| `SolvedMatrix_...` #0 (`inv_full`) | `record_schema_batch::solved_matrix_inv_full_is_complete_with_an_empty_diagnostics_array` |
| `SolvedMatrix_...` #1 (`linsolve_full`) | `record_schema_batch::solved_matrix_linsolve_full_is_complete_with_an_empty_diagnostics_array` |
| `SolverSchemas_DeclareTheStructuralKinds` | `record_schema_batch::solver_schemas_declare_the_structural_kinds` |
| `UnknownRecordType_IsReportedAsUnknown_NotAsConforming` | `record_schema_batch::unknown_record_type_is_reported_as_unknown_not_as_conforming` |
| `Validate_ReportsMissingAndUndeclaredFields` | `record_schema_batch::validate_reports_missing_and_undeclared_fields` |

`EveryProducedRecord` runs all 17 scripts from the C# corpus through the runner, recursively
collects every Record at depth <= 6 (through `fields[].value` and array `elements`), validates each
against the registry, and asserts the seen set contains all 17 structural names the C# case names:
`SolveResult`, `Solution`, `SolutionFamily`, `SystemSolveResult`, `SystemSolution`, `Binding`,
`TransformResult`, `RewriteStep`, `LimitResult`, `IntegrationResult`, `OptimizationResult`,
`CompilationResult`, `ParameterInfo`, `Inspection`, `MatrixInverseResult`, `MatrixSolveResult`,
`Diagnostic`. The corpus also produces `UnsatisfiableConditions` (from the refuted-rewrite row);
it is unregistered, exactly as in C#, and therefore validates vacuously - the test does not hide
that by asserting it.

The two matrix theories assert the full vocabulary on the real records: `SolveStatus` enum
`NoSolutions`/`Solved`, `complete` true, `Completeness` `Complete`, the payload and `conditions`
Arrays, and either exactly one structured `Diagnostic` (`matrix.singular`, `ErrorCategory`
`NoSolution`, non-empty message, recoverable, `location` Null, empty `details`) or an empty
diagnostics Array.

## 4. The nine census ids this round unblocks

| Census id | Rust test |
|---|---|
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/DiagnosticSchema_DeclaresTheSixFrozenFields#0` | `lovelace-run::record_schema_batch::diagnostic_schema_declares_the_six_frozen_fields` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/EveryProducedRecord_ConformsToItsDeclaredSchema#0` | `lovelace-run::record_schema_batch::every_produced_record_conforms_to_its_declared_schema` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/SingularMatrix_IsACompleteNoSolutions_CarryingAStructuredDiagnostic#0` | `lovelace-run::record_schema_batch::singular_matrix_inv_full_is_a_complete_no_solutions_carrying_a_structured_diagnostic` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/SingularMatrix_IsACompleteNoSolutions_CarryingAStructuredDiagnostic#1` | `lovelace-run::record_schema_batch::singular_matrix_linsolve_full_is_a_complete_no_solutions_carrying_a_structured_diagnostic` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/SolvedMatrix_IsComplete_WithAnEmptyDiagnosticsArray#0` | `lovelace-run::record_schema_batch::solved_matrix_inv_full_is_complete_with_an_empty_diagnostics_array` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/SolvedMatrix_IsComplete_WithAnEmptyDiagnosticsArray#1` | `lovelace-run::record_schema_batch::solved_matrix_linsolve_full_is_complete_with_an_empty_diagnostics_array` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/SolverSchemas_DeclareTheStructuralKinds#0` | `lovelace-run::record_schema_batch::solver_schemas_declare_the_structural_kinds` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/UnknownRecordType_IsReportedAsUnknown_NotAsConforming#0` | `lovelace-run::record_schema_batch::unknown_record_type_is_reported_as_unknown_not_as_conforming` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/Validate_ReportsMissingAndUndeclaredFields#0` | `lovelace-run::record_schema_batch::validate_reports_missing_and_undeclared_fields` |

Measured inventory (`cargo test -p lovelace-run -- --list`) lists all nine names exactly as
written above, so the manifest map can copy them verbatim.

## 5. Gate outputs

Environment: `RUSTUP_HOME`/`CARGO_HOME` under `.toolchain`, `TMP` under `rust/target/tmp`.
Commands were run from `rust/` unless noted.

| Gate | Command | Observed |
|---|---|---|
| Workspace tests | `cargo test --workspace` | **4,952 passed, 0 failed**. This lane contributes 9 of them; the concurrent DSP lane's 17 tests were compiled in this run. An earlier run before those 17 compiled measured 4,935 (4,926 baseline + 9). |
| Documentation corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total`, exit 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS: oracle 20/20, rust 20/20`, exit 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS: every RUN-001..RUN-014 row behaves like the oracle`, exit 0 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, no warnings |
| Formatting, this crate | `cargo fmt -p lovelace-run -- --check` | exit 0 (both new files formatted by `cargo fmt -p lovelace-run`) |
| Formatting, workspace | `cargo fmt --all -- --check` | exit 0. An earlier run failed only on `rust/lovelace-lang/src/fixed_dsp.rs` and `fixed_dsp_tests.rs` while the concurrent DSP lane was mid-edit; that lane then formatted its files and the gate is clean. No `lovelace-run` file ever appeared in a diff. |

The final measurement of every gate above is green. No file outside this lane's scope was edited
to obtain it.

## 6. Scope and what remains

Changed files:

| File | Change |
|---|---|
| `rust/lovelace-run/src/schemas.rs` | new additive registry (production module) |
| `rust/lovelace-run/src/record_schema_batch.rs` | new nine-test port module |
| `rust/lovelace-run/src/main.rs` | two module registrations only |
| `docs/goal-parity-contract/round-48/record-schemas.md` | this report |

Not touched: any `.cs` file, `Lovelace.Run.Tests/fixtures`, `parity/` (no manifest, census or
coverage edit), the corpus, `rust/lovelace-lang` (the DSP lane's files), or any other document.

Remaining work, owned elsewhere:

1. **Manifest mapping.** The nine rows above are not yet `ported` in `parity/manifest-map.json`.
   Mapping them moves coverage 5,788 -> 5,797 of 6,209 (93.22 % -> 93.36 %) and is a separate
   parity-only edit, outside this lane's scope.
2. **Wire enforcement.** The registry is additive and validates published structured records; the
   runner does not emit schema violations on the wire (the C# registry is likewise a library
   surface, not an envelope field). If a future host wants schema drift as a diagnostic, it can
   consume `RecordSchemas::validate` without changing this contract.
