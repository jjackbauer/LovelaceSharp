# Round 41 â€” clippy clean across the Rust workspace

Status: **complete**. `cargo clippy --workspace --all-targets -- -D warnings` now exits 0
with no diagnostics. `cargo fmt --all --check` exits 0. The full workspace suite, the
214-example corpus, the 20 protocol fixtures (both sides), the 16-row CLI matrix and
gate-manifest all pass. No C# file, fixture, parity mapping, or contract document was edited;
the only parity tree change is the regenerated `parity/rust-tests.json` inventory (see Â§6).

## 0. Result

| Measure | Before | After |
|---|---|---|
| `cargo clippy ... -D warnings` exit | 101 | **0** |
| Actionable diagnostics on the first reachable run | 28 (json 8, nat 20) behind 4 crate roll-ups | **0** |
| Reachable crates on that run | 2 (cargo stops dependants) | 11 |
| Workspace suite | not run past the failure | **4,808 passed, 0 failed** |

The brief's â€œapproximately 40 diagnosticsâ€ described only the first reachable layer. Because
cargo stops a crate whose dependency fails, clearing `lovelace-json` and `lovelace-nat`
exposed a further **53** in `lovelace-real`/`lovelace-symbolics`, **220** in `lovelace-lang`
(138 of them the single `result_large_err` lint), and roughly **115** in `lovelace-run`/
`lovelace-desktop` (mostly `to_string_in_format_args` and redundant closures). About 415
actionable diagnostics surfaced in total; all are fixed or covered by the allow list in Â§3.

## 1. Error classes and fixes

### 1a. Inherent `to_string` replaced by `Display`

| Type | Fix |
|---|---|
| `Json` | inherent `to_string` removed; `impl Display for Json` serialises compactly through the existing writer. All `.to_string()` call sites now use the std `ToString` blanket impl, byte-identically. |
| `Real` | public inherent `to_string` renamed to a private `render`; `Display` writes `render()`. `.to_string()` still resolves through `Display`. |
| `Rat` | inherent `to_string` removed; `Display` renders integer and fraction forms with the same branch logic. |
| `Expr` | inherent `to_string` renamed to a private `render`; `Display` writes `render()`; format-argument calls dropped their redundant `.to_string()`. |

This also removed the two deny-by-default `inherent_to_string_shadow_display` errors
(`Real`, `Expr`).

### 1b. Mechanical correctness lints

- explicit lifetimes elided on five reference operator impls (`Add/Sub/Mul/Div/Rem` for `&Nat`);
- needless borrows on `Nat::limbs()`, `as_decimal_str()`; needless range loops in `bz.rs`
  and three `groebner.rs` monomial operations rewritten with `iter_mut().enumerate()`;
- `manual_clamp` in `Nat::approx_digits`, `transcendental::trig_integer_digits` and an
  `evalf` digit boundary replaced with `clamp`; `manual_div_ceil` replaced with `div_ceil`;
  manual `RangeInclusive::contains` replaced with `contains`;
- `assertions_on_constants` moved into a `const { assert!(...) }` block, and the unused
  `MAX_TERMS` constant is now used by a compile-time bound assertion rather than deleted;
- redundant rebindings (`prime`, `snapshot`), unnecessary casts, unused assignments/variables/
  imports/mutability, `map_clone`/`iter_cloned_collect`, `useless_vec`, `format_in_format_args`,
  `question_mark`, `unwrap_or_default`, `while_let_loop`, `collapsible_if/match`,
  `redundant_guards`, `manual_range_contains`, `sort_by_key`, `bool_assert_comparison`,
  `wrong_self_convention` on two `Copy` fixed-point helpers, and two `&mut Vec` parameters
  narrowed to `&mut [_]` (only read/cloned there);
- `non_canonical_partial_ord_impl` on `Real` and `Fixed<W>`: the inherent comparison moved
  into `Ord::cmp` (identical body) and `partial_cmp` delegates to `Ord::cmp`;
- `type_complexity` resolved with `IntervalEnd`/`Bound`, `CarryCase`, and `MatrixCase` aliases;
- five missing docs added (three public polynomial helpers, two nat examples) and one orphan
  doc comment removed.

`to_string_in_format_args` (about 110 call sites across lang/run) and nine redundant closures
were applied with `cargo clippy --fix` after snapshotting the sources; the resulting diff was
reviewed and contains only the removal of redundant `.to_string()` calls and identity closures.

### 1c. Dead code

Zero-reference helpers and constants were deleted rather than allowed: `slice_indices_or_single`,
`numeric_total`, `solve_linear_system`, free `node_count`, `Real::shift_exp`,
`transcendental_round25::NAMES`, `real_parity_round10::nat`, the unused `r12_*` helpers,
`round19_batch`'s local `parse_expression`/`flat_chain_at_depth`/`recursion_with_deep_body`,
`round26_complex_math_parity::special_values`, `round35_extra_tail::display`,
`round19_run_batch::assert_timing_shape`/`project_value`, `pretty_batch::CORPUS`, and four
unused `Engine` accessors (`bounds_of`, `assumption_texts`, `positive_symbols`,
`nonzero_symbols`). Two test-only items were correctly scoped instead of deleted:
`symbolics_round23` is now `#[cfg(test)]` (it contains only tests), and `Engine::set_var` is
`#[cfg(test)]` (used only by a cancellation test). `engine::fake::boxed` is gated on
`#[cfg(not(feature = "real-engine"))]`, matching the factory that is its only user.

## 2. C#-mirrored test names preserved

`parity/manifest-map.json` keys on exact Rust test paths such as
`lovelace-json::tests::Parse_GivenFlatObject_PreservesFields` and
`lovelace-nat::newton::tests::Reciprocal_GivenRandomDivisors_IsTheExactFloor`. **None were
renamed.** Each affected test module carries a narrowly-scoped
`#[allow(non_snake_case)]` with a comment stating the manifest dependency. The rest of
`lovelace-nat` already used this pattern. gate-manifest still reports the same
4,808-test inventory and 5,720 claimed mappings.

## 3. Allow list added by this round (15 attributes)

| # | Attribute / scope | File | Justification |
|---:|---|---|---|
| 1 | `#![allow(clippy::result_large_err)]` (crate) | `rust/lovelace-lang/src/lib.rs` | `LangError` carries `ErrorSignature` inline by design as the wire-protocol error payload. Boxing would restructure construction/matching across the envelope path for a performance heuristic; explicitly deferred by the parent to protect the frozen wire contract. |
| 2 | `#[allow(clippy::should_implement_trait)]` on `Real::from_str` | `rust/lovelace-real/src/lib.rs` | The reference's `Real.FromString` is permissive and returns an option, not `Result`; `FromStr` cannot represent that contract and every ported call site uses this name. |
| 3-7 | `#[allow(clippy::should_implement_trait)]` on `Expr::{add,mul,neg,sub,div}` | `rust/lovelace-symbolics/src/lib.rs` | These are the reference's canonicalising binary operations called in associated form by the evaluator, not `std::ops` sugar; renaming would ripple through the ported surface. |
| 8 | `#[allow(non_snake_case)]` (test module) | `rust/lovelace-json/src/lib.rs` | Test names mirror the C# census method names; manifest-map keys on the exact path. |
| 9 | `#[allow(non_snake_case)]` (test module) | `rust/lovelace-nat/src/newton.rs` | Same reason; `lovelace-nat::newton::tests::...` paths are manifest keys. |
| 10 | `#[allow(clippy::large_enum_variant)]` on `Assumption` | `rust/lovelace-symbolics/src/assumptions.rs` | The variant payload mirrors the reference assumption forms and `Interval` stores its two optional bounds inline; boxing would change the published enum shape and allocate per interval for a stack-size heuristic. |
| 11 | `#[allow(clippy::large_enum_variant)]` on `IndexSpec` | `rust/lovelace-lang/src/builtins.rs` | A parsed index form carries its `Value` inline; boxing every ordinary single-index use for a stack-size heuristic is not warranted. |
| 12 | `#[allow(clippy::too_many_arguments)]` on `limit_record` | `rust/lovelace-lang/src/builtins.rs` | Mirrors the reference `LimitResult` constructor argument order; a struct would only move the same ten fields. |
| 13 | `#[allow(clippy::too_many_arguments)]` on `create_window` | `rust/lovelace-desktop/src/win32/ffi.rs` | Mirrors `CreateWindowExW`'s parameter list one-for-one; grouping geometry would obscure the SDK mapping. |
| 14 | `#![allow(clippy::upper_case_acronyms)]` (module) | `rust/lovelace-desktop/src/win32/ffi.rs` | Type names mirror the Windows SDK exactly (`HWND`, `WNDCLASSW`, ...); renaming would make the hand-written bindings harder to check against the headers. |
| 15 | `#[allow(clippy::approx_constant)]` on one test | `rust/lovelace-real/src/round39_real_tests.rs` | `3.14` is the double literal under test, not a stand-in for `PI`. |

Pre-existing allows were left untouched (`needless_range_loop` in `lovelace-lang`;
`non_snake_case` in the older `lovelace-nat` test modules; `unsafe_code`/`non_snake_case`/
`dead_code` in the Win32 module).

## 4. Behaviour-preservation notes

- Every `Display` replacement produces the same string as the removed inherent method; all
  former call sites resolve through `ToString` to the same formatter.
- `Real`/`Rat` comparison bodies were moved verbatim into `Ord::cmp`; `.cmp()` call sites
  resolve to the same implementation.
- Slice/`iter_mut`/sort/`clamp`/`div_ceil`/`while let`/`?` rewrites are local and
  equivalent; the deleted items had zero references (verified by grep and by the 4,808-test
  inventory being unchanged).
- No test function was renamed or removed. The regenerated inventory reports the same
  `crates=11 tests=4808`.
- Observation: on the first full-suite run the wall-clock ceiling test
  `assumptions_round35::eight_hundred_atoms_stay_ordered_and_fast` measured 2.37 s against its
  2 s ceiling while the suite ran in parallel. It passes isolated (0.72 s) and in the two
  subsequent full runs (including the inventory regeneration run), which both exited 0.

## 5. Final gate lines

```
cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
  exit 0 (no diagnostics)

cargo fmt --manifest-path rust/Cargo.toml --all --check
  exit 0

cargo test --manifest-path rust/Cargo.toml --workspace
  4,808 passed, 0 failed (generator: crates=11 tests=4808 suiteExit=0 passed=4808 failed=0)

cargo run --release -p lovelace-corpus
  corpus: 214 passed, 0 failed, 214 total

node parity/tools/gate-fixtures.mjs
  oracle: 20/20
  rust: 20/20
  PASS: oracle 20/20, rust 20/20

node parity/tools/gate-cli.mjs
  CLI matrix: 16/16
  PASS: every RUN-001..RUN-014 row behaves like the oracle

node parity/tools/gate-manifest.mjs
  PASS (structural): 6209 test cases dispositioned; 5720 claim Rust coverage (92.12%);
  failures: []

node parity/tools/gate-census.mjs
  PASS: 2105 methods / 6209 cases across 15 projects

node parity/tools/gate-projects.mjs
  PASS: 19 C# projects dispositioned

node parity/tools/gate-requirements.mjs
  PASS: 262 requirements accounted for
```

## 6. Parity-manifest integrity

`parity/manifest-map.json` and every mapping row are **unchanged**. The manifest gate failed
its freshness check only because the generated `parity/rust-tests.json` records a SHA-256 of
the Rust sources, which necessarily changes when clippy fixes edit them. Re-running the
gate's own documented remedy (`parity/gen-rust-tests.ps1`) rewrote that generated file; its
diff is limited to `generatedAtUtc` and `sourcesSha256` (suite totals and all 11 crate test
lists are identical). No `.cs` file, fixture, mapping, or contract document was edited.

## 7. Change set

55 Rust source files changed, +377/âˆ’845 lines; the deletions are dominated by the unused
helpers and the duplicate `root_expr` block, not by logic. No generated binary artifacts were
kept.

