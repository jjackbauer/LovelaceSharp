# Round 76 — SymbolicMatrix rank and solve (4-row API block)

> **Status: implemented and gated.** The four deferred rows of the round-75 rank-11 block
> (`SymbolicMatrix inverse/rank/solve/solve_with_conditions API`) now have a Rust surface and a
> Rust test that asserts the C# contract. The `parity/manifest-map.json` flip is the parent
> lane's action: `parity/` is out of this round's scope and was not edited.

**Measured** 2026-11-12 (America/Sao_Paulo) against the pinned oracle and the workspace tree with
this round's files. The task's stated baseline was **5,191 passed / 1 ignored**; the observed
workspace run is **5,206 passed / 0 failed / 1 ignored** (this round adds 5 tests; the remaining
10 arrived from the concurrent `lovelace-lang` lane during the round).

## 1. What was blocked and what changed

Round-75 (§6, rank 11) records **4 rows** whose only carrier is a C# library object:
`SymbolicMatrix` had `Det` and `InverseWithConditions` only. This round adds, in
`rust/lovelace-symbolics/src/symbolic_matrix.rs`:

* `SymbolicMatrix::rank(&self) -> usize`
* `SymbolicMatrix::inverse(&self) -> Option<SymbolicMatrix>`
* `SymbolicMatrix::solve(&self, b: &[Expr]) -> Option<Vec<Expr>>`
* `SymbolicMatrix::solve_with_conditions(&self, b: &[Expr]) -> MatrixSolveResult`
* `MatrixSolveResult { vector, conditions, note }`
* private `solve_core` (the reference's `SolveCore`)

The diff to the existing file is **207 insertions, 0 deletions**: `det`,
`inverse_with_conditions`, `is_provably_zero` and `exact_divide` are untouched, and the
existing determinant/inverse tests still pass unchanged. Nothing in `lovelace-run`,
`lovelace-lang` or `lovelace-desktop` was edited.

## 2. Observed behaviour (the C# source is the contract)

Citations are to `Lovelace.Symbolics/Matrices/SymbolicMatrix.cs` and
`Lovelace.Symbolics.Tests/MatrixAdvanceTests.cs`.

### 2.1 Rank — generic over the symbolic domain

`Rank` (SymbolicMatrix.cs:180–229) is fraction-free **Bareiss elimination with full pivoting**,
not row-echelon only:

1. Search rows at or below the current row for the first entry at or right of the current column
   that is **not provably zero**. That entry is the pivot; its first such column is selected, the
   row is swapped into place, then the column is swapped into place (all *n* rows, including the
   ones already processed).
2. Count the pivot, update the trailing block with
   `(a[i,j]*pivot - a[i,c]*a[r,j]) / previous_pivot`, and zero the pivot column below the
   pivot. `previous_pivot` starts at 1 and becomes the pivot just used.
3. When no pivot exists, the rank is final. Rectangular matrices are supported; a 0-row or
   0-column matrix has rank 0.

**Zero-vs-unknown decision.** `IsProvablyZero` (SymbolicMatrix.cs:97–100) is a *structural*
test: a rational/integer zero, the canonical zero, or a numeric constant that evaluates to zero.
A symbolic entry is never "unknown" for pivot purposes: it counts as a pivot. The result is
therefore the **generic rank over the rational-function field Q(x)** — the size of the largest
minor that does not vanish identically. `is_provably_zero` in Rust is the structural half
(`Expr::Num(0)`); the crate's matrix module has no numeric evaluator, so a constant expression
that the C# evaluator would fold to zero is not folded here. For the census matrices (symbols and
integer constants) the two tests agree. This boundary is unchanged from the round-57
determinant/inverse port.

### 2.2 Inverse — preserved

`InverseWithConditions` (SymbolicMatrix.cs:235–256) is unchanged: adjugate over the determinant,
one `ExpressionProperty(det, NonZero)` condition, note `matrix is singular` and no condition
when the determinant is a provable zero. `Inverse` (SymbolicMatrix.cs:259–263) is the
convenience projection: it throws `InvalidOperationException` when the structured inverse has no
matrix. The Rust `inverse()` returns `None` in that case and is otherwise exactly
`inverse_with_conditions().matrix`.

### 2.3 Solve — particular solution, no free parameters

`SolveWithConditions` (SymbolicMatrix.cs:291–298) computes the determinant first, refuses when it
is a provable zero, and otherwise publishes the vector plus exactly the determinant NonZero
condition. `SolveCore` (SymbolicMatrix.cs:304–358) builds the augmented matrix, eliminates
**only the rows below the pivot** with the previous-pivot division, then back-substitutes with
`Exprs.Divide`.

The reference has **no free-parameter representation**: a singular or underdetermined square
system is refused (`matrix is singular` note, or the `System is singular or
underdetermined` exception when the pivot search fails), never parametrized. `solve` is the
convenience projection of `solve_with_conditions`.

### 2.4 Refusals

| Case | C# | Rust |
|---|---|---|
| singular determinant | `MatrixSolveResult(null, null, Empty, "matrix is singular")`; `Solve` throws | vector `None`, empty conditions, note `matrix is singular`; `solve` returns `None` |
| non-square | `Det` throws `Determinant requires a square matrix.` | note `determinant requires a square matrix` (same text as the existing inverse refusal) |
| failed pivot search | throws `System is singular or underdetermined.` | note `system is singular or underdetermined`, empty conditions (unreachable after the determinant pre-check) |
| right-hand side length ≠ rows | indexes out of range (undefined failure) | typed note `right-hand side length does not match the matrix` |
| singular/non-square inverse | `Inverse` throws | `inverse()` returns `None` |

The right-hand-side note is the one refusal added beyond the reference (the C# path is an
undefined `IndexOutOfRangeException`); it is documented here and asserted in the refusal test.

## 3. Member map

| C# (`SymbolicMatrix.cs`) | Rust (`symbolic_matrix.rs`) | Status |
|---|---|---|
| `Rows`, `Columns`, `this[r,c]`, `From` | `rows()`, `columns()`, `get()`, `from_rows` | existing |
| `Det(ExprContext)` (:125) | `det()` | existing, unchanged |
| `IsProvablyZero` (:97) | `is_provably_zero` | existing, unchanged |
| `ExactDiv` (:106) | `exact_divide` | existing, unchanged |
| `Rank(ExprContext)` (:180) | `rank()` | **new** |
| `InverseWithConditions` (:235) | `inverse_with_conditions()` | existing, unchanged |
| `Inverse` (:259) | `inverse()` | **new** |
| `Solve(SymbolicMatrix, Expr[], ExprContext)` (:280) | `solve(&[Expr])` (method; the static receiver is folded into `&self`) | **new** |
| `SolveWithConditions(Expr[], ExprContext)` (:291) | `solve_with_conditions(&[Expr])` | **new** |
| `SolveCore` (:304, private) | `solve_core` (private) | **new** |
| `CollectSymbols` (:360) | `symbols()` | existing |
| `LinearSystem` (:301) | — | alias not ported; no census row uses it |
| `MatrixSolveResult(Matrix, Vector, Conditions, Note)` (:427) | `MatrixInverseResult` + `MatrixSolveResult` | split; each operation carries the fields it publishes |
| `ExpressionPropertyAssumption(det, NonZero)` | `Assumption::ExpressionProperty(det, NonZero)` | existing vocabulary |

`MatrixSolveResult` is a record with both `Matrix` and `Vector`; Rust keeps two result
types because the inverse surface never publishes a vector and the solve surface never publishes a
matrix. Each field the reference reads for the corresponding operation is present with the same
meaning.

## 4. The four census ids and their tests

All four ids were `deferred` in `parity/manifest-map.json` when this round started (the
singular-inverse sibling row was already ported by round 66 and is not part of the four).

| Census id | C# case | Rust test |
|---|---|---|
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MatrixAdvanceTests/Rank_GenericOverSymbols_And_Numeric#0` | MatrixAdvanceTests.cs:16 | `lovelace-symbolics::round76_matrix::rank_generic_over_symbols_and_numeric` |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MatrixAdvanceTests/InverseWithConditions_NonSingular_MatchesInverse_And_CarriesCondition#0` | MatrixAdvanceTests.cs:40 | `lovelace-symbolics::round76_matrix::inverse_with_conditions_non_singular_matches_inverse_and_carries_condition` |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MatrixAdvanceTests/SolveWithConditions_MatchesSolve_And_CarriesCondition#0` | MatrixAdvanceTests.cs:67 | `lovelace-symbolics::round76_matrix::solve_with_conditions_matches_solve_and_carries_condition` |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MatrixAdvanceTests/Inverse_Invertible_BehaviorPreserved#0` | MatrixAdvanceTests.cs:86 | `lovelace-symbolics::round76_matrix::inverse_invertible_behavior_preserved` |

## 5. Tests

`rust/lovelace-symbolics/src/round76_matrix.rs` (new, 5 tests; registered in
`rust/lovelace-symbolics/src/lib.rs`):

| Test | Concrete matrices | Asserted |
|---|---|---|
| `rank_generic_over_symbols_and_numeric` | `[[x,1],[y,x]]`, `[[x,x],[x,x]]`, `[[1,2],[2,4]]`, 3×3 identity | ranks 2, 1, 1, 3 (the four C# assertions) |
| `inverse_with_conditions_non_singular_matches_inverse_and_carries_condition` | `[[x,1],[0,y]]` | matrix present, no note, exactly one `ExpressionProperty(x*y, NonZero)` atom, equal to `inverse()`, `A·A⁻¹ = I` at (2,3) |
| `solve_with_conditions_matches_solve_and_carries_condition` | `[[x,1],[0,y]]`, `b = [1,1]` | vector present, no note, exactly the determinant NonZero atom, entrywise equal to `solve()`, solution (1/3, 1/3) at (2,3), both residual rows zero |
| `inverse_invertible_behavior_preserved` | `[[x,1],[0,y]]` | entry-by-entry equality with `inverse_with_conditions()`; concrete entries (1/2, −1/6, 0, 1/3) at (2,3) |
| `rank_and_solve_refusal_cases` (supporting, not a census row) | zero 2×2, 0×0, zero-column, 2×3 dependent, 3×2 staircase, singular 2×2, non-square, singular inverse | rank 0/0/0/1/2; singular solve note + empty conditions; non-square note; length-mismatch note; `inverse()` None |

The four census tests assert every assertion of their C# bodies (the C# bodies assert matrix
equality, non-empty conditions, null notes, and elementwise vector equality; the Rust tests assert
the same facts plus the concrete values the C# test leaves implicit).

## 6. Gates

Commands run from the stated directory with
`RUSTUP_HOME=.toolchain\rustup; CARGO_HOME=.toolchain\cargo; TMP=rust\target\tmp`.

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Focused | `cargo test -p lovelace-symbolics round76_matrix` (rust/) | `5 passed; 0 failed; 0 ignored; 176 filtered out` | 0 |
| Package | `cargo test -p lovelace-symbolics` (rust/) | `181 passed; 0 failed; 0 ignored` (176 before the round) | 0 |
| Workspace | `cargo test --workspace` (rust/) | **5,206 passed; 0 failed; 1 ignored** over 22 result blocks | 0 |
| Corpus | `cargo run --release -p lovelace-corpus` (rust/) | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Fixtures | `node parity/tools/gate-fixtures.mjs` (repo root) | `oracle: 20/20`, `rust: 20/20`, `PASS` | 0 |
| CLI | `node parity/tools/gate-cli.mjs` (repo root) | `CLI matrix: 16/16`, `PASS` | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1` (repo root) | `GENERATED 200 200 0 0`, `REPLAYED 1758 1758 0 0`, `stillMismatching=0`, `new=0`, `harness exit: 0` | 0 |
| Clippy (package) | `cargo clippy -p lovelace-symbolics --all-targets -- -D warnings` (rust/) | no diagnostics, `Finished` | 0 |
| Clippy (workspace minus desktop) | `cargo clippy --workspace --all-targets --exclude lovelace-desktop -- -D warnings` (rust/) | no diagnostics, `Finished` | 0 |
| Format (package) | `cargo fmt -p lovelace-symbolics -- --check` (rust/) | no diff | 0 |

**Workspace-wide clippy and format are red only in `rust/lovelace-desktop`**, which this round
did not touch and which the concurrent desktop lane is editing (`git status` shows its
working-tree changes). The full `cargo clippy --workspace --all-targets -- -D warnings` reports
two lints there — `execution.rs:652` (`manual_unwrap_or_default`) and `ui/plot.rs:98`
(`manual_clamp`) — and `cargo fmt --all -- --check` reports diffs in `engine.rs`,
`execution.rs`, `gui_selftest.rs`, `main.rs`, `settings.rs`, `statements.rs`, `ui/` and
`workbench.rs`. The formatting diffs were already present in the committed desktop files at the
start of this round (the fmt check reported them while `git status` was still clean for
`rust/lovelace-desktop`); the clippy lints are in the same files, which are outside this round's
permitted scope. Every crate touched or depended on by this round is clippy-clean and fmt-clean.

The differential ran against `rust/target/release/lovelace-run.exe` sha256
`d2d5067a…c2bbb4`, built before this round; the runner source was not changed by this round and
the new API is additive library surface, so the measured 200/1,758 result is the unchanged runner
baseline. The corpus run rebuilt `lovelace-symbolics` and `lovelace-lang` in release with the new
code and stayed 214/214.

## 7. Scope, boundaries and residual

Changed files:

* `rust/lovelace-symbolics/src/symbolic_matrix.rs` — +207 lines, 0 deletions (new surface only).
* `rust/lovelace-symbolics/src/round76_matrix.rs` — new, 5 tests.
* `rust/lovelace-symbolics/src/lib.rs` — +4 lines (test-module declaration).
* `docs/goal-parity-contract/round-76/matrix-rank-solve.md` — this report.

No `.cs` file, fixture, corpus file, `parity/` file, `manifest.toml` or other document was
edited. `rust/lovelace-run`, `rust/lovelace-lang` and `rust/lovelace-desktop` were not
touched; the concurrent `lovelace-lang` and `lovelace-desktop` changes visible in `git status`
(the language lane's `round76_capture_events` module and the desktop lane's working tree) are the
other lanes' work.

Boundaries left explicit:

* **Structural zero test.** The reference also folds numeric constants through its evaluator; the
  Rust pivot test is structural. No census matrix depends on the difference.
* **Exceptions become values.** `InvalidOperationException` paths (singular inverse, singular or
  underdetermined solve, non-square determinant) are published as `None` plus a note, the
  convention the round-57 inverse already used.
* **Right-hand-side length** is a typed refusal rather than the reference's undefined
  `IndexOutOfRangeException`.
* **No free parameters**: the reference has no parametric solve on this surface, so neither does
  the port.
* **Map flip is external**: the four ids are listed in §4; `parity/manifest-map.json` was not
  edited, by scope.
