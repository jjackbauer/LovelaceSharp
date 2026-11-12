# Round 78 — StructuralEquality

**Status: complete for the four deferred rows.** The block is
`StructuralEquality.FirstDifference path reporting` (round-75 rank 9): the reference's deep,
diagnostic structural equality over `Value`. Rust gains
`lovelace-lang::structural_equality::StructuralEquality` with `values_equal`, `records_equal`,
`first_difference` and `first_difference_from`. The surface is additive: it changes no Display,
ordering, wire envelope or existing equality implementation.

Measured 2026-11-12 (America/Sao_Paulo) against the pinned oracle
`1cc28f255eca6029849f880949fc7be1d40ab8af` and `rust/target/release/lovelace-run.exe`.

## 1. Where the contract lives

The reference class is `Lovelace.Suite.StructuralEquality` (`Lovelace.Suite/StructuralEquality.cs`),
a static class over `Lovelace.Suite.Value`; its five census rows are
`Lovelace.Suite.Tests.StructuralEqualityTests` (`Lovelace.Suite.Tests/StructuralEqualityTests.cs`).
It is a library-only surface: no runner script observes it, which is why round 75 classified the
four deferred rows UNRUNNABLE rather than STILL-DIFFERENT.

The Rust home is therefore `rust/lovelace-lang`, whose `Value` enum (`src/lib.rs:181`) is the
counterpart of the reference `Value`. The runner owns no values or comparison path.

## 2. Observed equality semantics

Read from `StructuralEquality.cs` and its tests; the messages and paths below are the exact
reference strings.

| # | Aspect | Reference behaviour (source) | Rust surface |
|---|---|---|---|
| 1 | Kind first | A kind mismatch short-circuits before any payload is read: `${path}: kind {a.Kind} != {b.Kind}` (`:27-28`). Natural 1 and Integer 1 are different; Integer 1 and Text "1" are different. | `kind_tag` compares the stored variant, then emits the same sentence. |
| 2 | Numeric | Same-kind Natural/Integer/Real compare by NUMERIC value through `NumericOps.Compare` (`:32-35`); unequal answers `${path}: {kind} value differs`. | `Nat::cmp`, `Int::cmp`, `Real::cmp` (the latter is exact value, not digit spelling). |
| 3 | Complex | Real and imaginary parts are compared separately and named: `${path}.re: differs`, `${path}.im: differs` (`:36-40`). | Same, componentwise on the two `Real` payloads. |
| 4 | Boolean / Text | `${path}: boolean differs`; Text uses ordinal string equality and says `${path}: text differs` (`:41-45`). | Same. |
| 5 | Enum | Equal iff BOTH the declared enum type and the member match. A type mismatch says `${path}: enum type {a} != {b}`; a member mismatch says `${path}: enum {type} value {a} != {b}` (`:46-55`). `SolveStatus.Partial` is not `Completeness.Partial`. | Same, on the stored `(type, member)` pair. |
| 6 | Domain / Function | Domain says `${path}: domain differs`; Function compares by NAME and says `${path}: function differs` (`:56-59`). | Same. Function identity is the name, exactly as the reference observes it. |
| 7 | Symbolic | Equal iff `Printing.CanonicalPrint(a) == Printing.CanonicalPrint(b)`; a difference says `${path}: symbolic canonical differs` (`:60-63`). The pretty text is never read. | `Expr::canonical()` text equality; same message. |
| 8 | Vector / Array | Shape is compared first (`${path}: shape [{a}] != [{b}]` with no spaces), then elements in the reference's flattened row-major order, each at `${path}[{i}]` (`:64-79`). Vector and Array are distinct kinds. | `Value::Vector` shape `[len]`; `Value::Array` shape from `NdArray::shape()`; elements via `as_slice()` (row-major). |
| 9 | Record | Type name first (`${path}: record type {a} != {b}`), then field count (`${path}: {type} has {n} fields, other has {m}`), then each field's NAME positionally (`${path}.{type}[{i}]: field name {a} != {b}`) and each field's VALUE recursively at `${path}.{name}` (`:80-95`). Field order is part of the structure. | Same, on `Value::Record(type, Vec<(name, value)>)`. |
| 10 | Void / path | `Void` (and every valueless kind) is equal to itself and falls through the default `return null` (`:96-98`). The root path is `$`; nested paths append `.field`, `[index]`, `.re`/`.im`. | Same; `first_difference` uses `$`, `first_difference_from` takes an explicit root. |

### 2.1 Assumptions do not participate

The reference comparison has no `ExprContext`, no `AssumptionSet` and no simplifier: it reads the
two values only. Canonical construction is assumption-free by invariant
(`Constructors.cs:10-11`), and `Printing.CanonicalPrint` is a pure function of the tree. The Rust
implementation is likewise assumption-free. This is an observed NON-interaction, and it is pinned
by a test: under `x > 0`, `abs(x)` and `x` denote the same number but remain structurally
different, while an identical expression remains equal under any ambient assumption set.

### 2.2 Positive and negative cases, measured on both binaries

Canonical strings below were read with `inspect(<expr>).canonical` from
`dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll` and
`rust/target/release/lovelace-run.exe`; every language-level expression pair is byte-identical in
the two runtimes. The last row uses the Rust `Div` node, which has no C# counterpart (the
reference folds a quotient into a reciprocal power at construction), so that row is Rust-only.

| Case | Canonical form | Structural equality | Why |
|---|---|---|---|
| `x + y` vs `y + x` | both `(add (sym x) (sym y))` | **true** | Commutative canonicalisation sorts the terms. |
| `x * y` vs `y * x` | both `(mul (sym x) (sym y))` | **true** | Unordered operands sort to one factor sequence. |
| `x + x` vs `2*x` | both `(mul (rat 2 1) (sym x))` | **true** | Like terms are collected at construction. |
| `(x + 1)^2` vs `x^2 + 2*x + 1` | `(pow (add (rat 1 1) (sym x)) (rat 2 1))` vs `(add (rat 1 1) (pow (sym x) (rat 2 1)) (mul (rat 2 1) (sym x)))` | **false** | Mathematically equal, structurally different: equality is not algebraic. |
| `sin(x)^2 + cos(x)^2` vs `1` | `(add (pow (fn cos (sym x)) (rat 2 1)) (pow (fn sin (sym x)) (rat 2 1)))` vs `(rat 1 1)` | **false** | A trigonometric identity is not a structural rewrite. |
| rational `1/2` vs real `0.5` | `(rat 1 2)` vs `(real 0.5)` | **false** | Same mathematical value, different symbolic node kinds. |
| `abs(x)` vs `x` under `x > 0` | `(fn abs (sym x))` vs `(sym x)` | **false** | Assumptions are not consulted (2.1). |
| Natural 2 vs Integer 2 vs Symbolic 2 | all display `2`; `(rat 2 1)` for the symbolic one | **false** | Kind-first: `$: kind Natural != Integer`, `$: kind Integer != Symbolic`. |
| rational `-1/2` leaf vs quotient node `Div(-1, 2)` | `(rat -1 2)` vs a reciprocal-power product | **false** | Same pretty text `-1/2`, different canonical form: pretty text is not the key. |

The "vice versa" direction (structurally equal but mathematically different) is not reachable for
canonical expressions: equal canonical forms are the same typed tree. The nearest observable case
is the same DISPLAY with different structure (last row), which is correctly unequal. Function
values are a documented identity limit: the reference compares only `FunctionDefinition.Name`, so
two definitions that share a name compare equal even when their bodies differ; Rust stores only
the name, so it matches that observation.

## 3. Before / after

Before this round there was no Rust `StructuralEquality` at all (round 75: "no Rust API exists to
assert the row's contract"). `Value` derived `PartialEq`, which answers true/false but cannot name
a nested difference and compares `Expr`s structurally rather than by canonical text. After:

| Census case | Before | After |
|---|---|---|
| `DifferenceInANestedField_IsNamedByPath#0` | `sample(2) != sample(3)`; no path. | `$.solutions[1].multiplicity: Integer value differs`. |
| `DifferentKinds_AreReportedByKind#0` | `Integer(1) != Text("1")`; no reason. | `$: kind Integer != Text`. |
| `DifferentShape_IsReportedAsShape#0` | `[1, 2] != [1]`; no reason. | `$: shape [2] != [1]`. |
| `SymbolicEquality_IsByCanonicalForm_NotPrettyText#0` | derived `Expr` equality (operand-order sensitive; no diagnostic). | canonical-text equality, `None` for equal values, `$: symbolic canonical differs` otherwise. |

The API is purely additive: `Value`'s `PartialEq`, `Ord` and `display()` are untouched, so every
existing Display/ordering/wire test is unchanged by construction.

## 4. Member map

| Reference member | Source | Rust member |
|---|---|---|
| `ValuesEqual(Value, Value)` | `StructuralEquality.cs:15` | `StructuralEquality::values_equal(&Value, &Value) -> bool` |
| `RecordsEqual(RecordValue, RecordValue)` | `:18` | `StructuralEquality::records_equal(&Value, &Value) -> bool` (records only; Rust has no separate record wrapper) |
| `FirstDifference(Value, Value, string path = "$")` | `:25` | `StructuralEquality::first_difference(&Value, &Value) -> Option<String>` and `first_difference_from(&Value, &Value, &str) -> Option<String>` |
| `ValueKind` switch | `:30-98` | the match in `structural_equality.rs`; every `Value` variant is covered |

## 5. The four census ids

All four are `Lovelace.Suite.Tests/Lovelace.Suite.Tests.StructuralEqualityTests/…`, deferred in
`parity/manifest.toml` with the block reason "StructuralEquality.FirstDifference path reporting".
Each is unblocked by one same-name Rust test:

| # | Census id | C# line | Rust test |
|---:|---|---:|---|
| 1 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.StructuralEqualityTests/DifferenceInANestedField_IsNamedByPath#0` | 32 | `lovelace-lang::round78_structural_equality::difference_in_a_nested_field_is_named_by_path` |
| 2 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.StructuralEqualityTests/DifferentKinds_AreReportedByKind#0` | 64 | `lovelace-lang::round78_structural_equality::different_kinds_are_reported_by_kind` |
| 3 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.StructuralEqualityTests/DifferentShape_IsReportedAsShape#0` | 40 | `lovelace-lang::round78_structural_equality::different_shape_is_reported_as_shape` |
| 4 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.StructuralEqualityTests/SymbolicEquality_IsByCanonicalForm_NotPrettyText#0` | 50 | `lovelace-lang::round78_structural_equality::symbolic_equality_is_by_canonical_form_not_pretty_text` |

The fifth row of the class, `IdenticalRecords_AreEqual#0` (C# line 25), is **already ported**
(`lovelace-lang::round19_batch::suite_structural_identical_records_are_equal`); it is not one of
the four and is not re-ported. `parity/manifest-map.json` is outside this round's scope and was
not edited; the four `rust` ids above are the mapping targets for the porting round.

## 6. Tests

New module `rust/lovelace-lang/src/round78_structural_equality.rs` (declared
`#[cfg(test)] mod round78_structural_equality;`), **15 tests**: the four row mirrors plus eleven
supporting tests that pin the semantics in §2.

| Test | Kind | Asserts |
|---|---|---|
| `difference_in_a_nested_field_is_named_by_path` | row 1 | `sample(2)` vs `sample(3)` names `$.solutions[1].multiplicity` exactly. |
| `different_kinds_are_reported_by_kind` | row 2 | Integer 1 vs Text "1" answers `$: kind Integer != Text`. |
| `different_shape_is_reported_as_shape` | row 3 | Vector [1,2] vs [1] answers `$: shape [2] != [1]`. |
| `symbolic_equality_is_by_canonical_form_not_pretty_text` | row 4 | `x+x` equals `2*x` canonically (and self-difference is `None`); a rational leaf and a quotient with the same display `-1/2` are unequal. |
| `commutative_operands_canonicalise_to_one_form` | supporting | `x+y`/`y+x` and `x*y`/`y*x` are equal. |
| `mathematically_equal_expressions_with_different_structure_are_unequal` | supporting | `(x+1)^2` vs expanded, and the Pythagorean identity vs 1, are unequal. |
| `rational_and_real_leaves_are_different_node_kinds` | supporting | `1/2` rational vs `0.5` real leaf differ canonically. |
| `assumptions_do_not_change_structural_equality` | supporting | An `x > 0` `AssumptionSet` proves its atom, yet `abs(x)` vs `x` stays unequal and `x` vs `x` equal. |
| `same_display_different_kinds_are_unequal` | supporting | Natural 2, Integer 2 and Symbolic 2 all display `2` but differ by kind. |
| `record_identity_is_type_name_field_count_and_field_order` | supporting | Record type, field count and positional field-name diagnostics. |
| `enum_identity_includes_the_declared_type` | supporting | Enum type mismatch and member mismatch diagnostics. |
| `complex_parts_are_named_by_path` | supporting | An imaginary-part difference answers `$.im: differs`. |
| `array_shape_and_element_paths_are_named` | supporting | 2×2 vs 2×2 element 3 answers `$[3]: Natural value differs`; 2×2 vs 2×3 answers `$: shape [2,2] != [2,3]`. |
| `void_and_clamped_reals_follow_the_reference_kinds` | supporting | Void equals Void and differs by kind from Natural; a `RealClamped` equals the same numeric `Real`. |
| `records_equal_requires_records_and_the_root_path_is_overridable` | supporting | `records_equal` accepts records and refuses non-records; `first_difference_from` honours an explicit root path. |

## 7. Gates

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Focused | `cargo test -p lovelace-lang structural_equality` | **15 passed; 0 failed; 0 ignored** (1,569 filtered out) | 0 |
| Check | `cargo check -p lovelace-lang --tests` | clean, `Finished` | 0 |
| Workspace | `cargo test --workspace` | **5,244 passed; 0 failed; 1 ignored** over 22 result blocks (baseline 5,219 passed / 1 ignored; the +25 is this lane's 15 plus the concurrent scoped-eval and cancellation lanes' tests in the same working tree); no failure line; ended after doc-tests. Measured before the formatting-only sweep described in §8. | 0 |
| Package after fmt | `cargo test -p lovelace-lang` | **1,584 passed; 0 failed; 0 ignored** after the formatting sweep | 0 |
| Corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS` | 0 |
| CLI | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS` | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1` | generated **200/200 match**, replayed **1758/1758 match**; 0 mismatch, 0 error; `stillMismatching=0`; `harness exit: 0` | 0 |
| Clippy (crate) | `cargo clippy -p lovelace-lang --all-targets -- -D warnings` | clean, `Finished` | 0 |
| Clippy (workspace, desktop excluded) | `cargo clippy --workspace --all-targets --exclude lovelace-desktop -- -D warnings` | clean, `Finished` | 0 |
| Format | `cargo fmt -p lovelace-lang -- --check` | clean after the authorized package sweep; the sweep and the concurrent doc-comment edit it exposed are recorded in §8 | 0 |

## 8. Scope and boundaries

Changed files:

- `rust/lovelace-lang/src/structural_equality.rs` — new production module (the surface).
- `rust/lovelace-lang/src/round78_structural_equality.rs` — new test module (15 tests).
- `rust/lovelace-lang/src/lib.rs` — this lane's semantic edits are two declarations only:
  `pub mod structural_equality;` (line 24) and `#[cfg(test)] mod round78_structural_equality;`
  (line 2426). The same file also carries the concurrent scoped-eval lane's
  `mod round78_scoped_eval;` (line 2421) and the cancellation lane's `Engine`/`kernel_cancel_round9`
  changes. The authorized package-wide fmt (below) additionally normalized the cancellation lane's
  import order in this file; no token or logic was changed by this lane.
- `docs/goal-parity-contract/round-78/structural-equality.md` — this report.

**Formatting sweep (authorized after the gates above).** `cargo fmt -p lovelace-lang` was
authorized by the coordinator because the package fmt check was red on the cancellation lane's
uncommitted files. Pre-sweep hashes: `lib.rs` `D71A0B10…` and `kernel_cancel_round9.rs`
`BB7C5017…`. The sweep changed exactly two files, both formatting-only:

- `rust/lovelace-lang/src/lib.rs` — the two `std::sync` imports (`Arc`, `atomic::AtomicBool`)
  reordered. No token or logic change.
- `rust/lovelace-lang/src/kernel_cancel_round9.rs` — the same import order, and the
  `setter.join().expect(...)` call wrapped over three lines. No token or logic change.

During the sweep a THIRD change appeared in `lib.rs` that rustfmt cannot produce: the
`kernel_cancel_round9` doc comment changed from "kernel (`sqrt(2)` at precision 200000) with the
structured `Cancelled` failure inside 2 s" to "polling kernel (`matmul`) … The opaque `sqrt(2)` at
precision 200000 is documented as uninterruptible, not claimed as cancelled". That is the
cancellation lane's concurrent prose edit, not this lane's output. Nothing was reverted:
`git checkout -- lib.rs` would also discard that lane's working-tree code and the scoped-eval
declarations. After the sweep, `cargo fmt -p lovelace-lang -- --check` exits 0,
`cargo clippy -p lovelace-lang --all-targets -- -D warnings` exits 0, and
`cargo test -p lovelace-lang` reports **1,584 passed / 0 failed / 0 ignored**.

No `.cs` file, fixture, corpus file, `parity/` file, `rust/lovelace-desktop` file or other
document was edited. No assumptions/eval production file was edited; the supporting assumption
test only READS `AssumptionSet::empty` / `add` / `ask`.

Boundaries, stated rather than hidden:

1. **Two Real spellings.** Rust has `Value::Real` and `Value::RealClamped`; the reference has one
   `ValueKind.Real`. The new surface treats both as kind `Real` and compares the numeric payload,
   ignoring the clamp metadata (which has no reference counterpart). A test pins this.
2. **Array vs Vector.** Rust's wire `kind_name()` calls a rank-1 `Value::Array` a `Vector`, but the
   structural kind follows the STORED variant, matching the reference's separate `ValueKind.Vector`
   and `ValueKind.Array`. A rank-1 `Value::Array` is therefore structurally an `Array` here; the
   language's normal vector path stores `Value::Vector`, so observable program values are unaffected.
3. **Function identity is the name.** As in the reference, two `Value::Function`s are equal when
   their names are equal, even if a later definition replaces the body. This is the reference's
   own limit, not an approximation introduced here.
4. **No mapping edit.** The four census ids are listed in §5; flipping them in
   `parity/manifest-map.json` and regenerating the manifest is the porting round's job.
