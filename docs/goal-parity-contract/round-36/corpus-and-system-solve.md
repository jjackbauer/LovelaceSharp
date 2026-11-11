# Round 36 - Corpus closure and the system-solve family

Status: production fixed and verified on this tree. The two documentation-corpus failures are
closed (214/214), the 21-row system-solve family is implemented (Lex Gröbner elimination), and
tests pin every fixed behaviour. Two pre-existing precision tests outside this lane's files are
still red; section 5 records them with the evidence that they predate this work.

## 1. Scope and changed files

Production (this lane):

| File | Change |
|---|---|
| `rust/lovelace-lang/src/builtins.rs` | PowInt exponent emits a fresh Constant node; `linsolve` uses Bareiss elimination + back-substitution for symbolic systems; `solve_system` / `solve_system_full` use Lex Gröbner elimination with proof/abandon bookkeeping and the oracle's notes. |
| `rust/lovelace-symbolics/src/groebner.rs` | New sparse multivariate polynomial + Buchberger implementation (Lex/GrLex/GrRevLex, reduce, eliminate, monic, interreduce). |
| `rust/lovelace-symbolics/src/lib.rs` | Exact quadratic radical roots (positive non-square discriminant) so an eliminated univariate quadratic is not answered `Unevaluated`. |

Tests: `rust/lovelace-lang/src/corpus_and_system_solve_round36.rs` (10), `groebner.rs` tests
(7), and two pre-existing panic-sweep assertions updated to the new, stronger behaviour. No corpus
expectation, fixture, runner or `.cs` file was edited.

## 2. Corpus sym/071 - MathIR listing

The reference lowers a PowInt exponent with a FRESH Constant node every time
(`Lowering.Emit` calls `AddConstant`, which never consults the node cache). The constant TABLE is
deduplicated by value, the node is not. Rust cached the literal node by expression text, so the
coefficient 2 reused the exponent's node and every following index shifted by one.

Before (corpus runner):

```text
#!mathir 2
param x
const (rat 2 1)
const (rat 1 1)
const (rat 3 1)
Parameter 0 0
Constant 0 0
PowInt 0 0,1
Constant 0 1
Constant 0 2
Add 0 1,0
Mul 0 0,5
Add 0 4,6
Mul 0 0,7
Add 0 3,8
Mul 0 2,9
```

After (byte-identical to the C# oracle probe
`dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --file <script> --text`):

```text
#!mathir 2
param x
const (rat 2 1)
const (rat 1 1)
const (rat 3 1)
Parameter 0 0
Constant 0 0
PowInt 0 0,1
Constant 0 1
Constant 0 2
Constant 0 0
Add 0 5,0
Mul 0 0,6
Add 0 4,7
Mul 0 0,8
Add 0 3,9
Mul 0 2,10
```

## 3. Corpus sym/076 - linsolve shape

Rust solved by Cramer's rule, which evaluates the first coordinate to the corrected quotient
`-1/(x*y)`. The reference uses fraction-free Bareiss elimination and back-substitution, whose
intermediate `x/x` survives in the published product.

| | exact `--text` |
|---|---|
| oracle / expected | `[-x/x/(x*y), x/(x*y)] (Vector)` |
| Rust before | `[-1/(x*y), x/(x*y)] (Vector)` |
| Rust after | `[-x/x/(x*y), x/(x*y)] (Vector)` |

`linsolve_full(...).conditions` still publishes `[x*y != 0]`; the numeric Cramer path is kept for
all-numeric inputs, so no numeric rendering changed.

## 4. System-solve family (21 rows)

### Before (measured, from round 34)

| Script | Before |
|---|---|
| `solve_system_full([x^2 + y^2 == 1], [x, y])` | Unevaluated `system-solve.non-polynomial: only polynomial systems are supported in v1.` |
| `solve_system_full([x + y == 1, x + y == 2], [x, y])` | Unevaluated (same non-polynomial note) |
| `solve_system_full([x^2 + y == 1, x - y == 0], [x, y])` | Unevaluated (same non-polynomial note) |

### After (exact oracle text)

```text
x = symbol("x"); y = symbol("y"); solve_system_full([x + y == 1, x + y == 2], [x, y])
= SystemSolveResult(status: NoSolutions, domain: complex, complete: True, completeness: Complete, solutions: [], diagnostics: []) (SystemSolveResult)

x = symbol("x"); y = symbol("y"); solve_system_full([x^2 + y == 1, x - y == 0], [x, y])
= SystemSolveResult(status: Solved, domain: complex, complete: True, completeness: Complete, solutions: [SystemSolution(bindings: [Binding(name: x, value: 1/2*(-sqrt(5) - 1)), Binding(name: y, value: 1/2*(-sqrt(5) - 1))], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: 1/2*(sqrt(5) - 1)), Binding(name: y, value: 1/2*(sqrt(5) - 1))], conditions: [], exactness: Exact)], diagnostics: []) (SystemSolveResult)

x = symbol("x"); y = symbol("y"); solve_system_full([x^2 + y^2 == 1], [x, y])
= SystemSolveResult(status: Unevaluated, domain: complex, complete: False, completeness: Unknown, solutions: [], diagnostics: [Diagnostic(code: system-solve.unevaluated, category: UnsupportedOperation, message: system-solve.elimination-incomplete: the Gröbner basis has no polynomial in y alone, so the values of y are not enumerated here., recoverable: True, location: , details: [])]) (SystemSolveResult)

x = symbol("x"); y = symbol("y"); solve_system_full([x^2 + y^2 == 1, x*y == 0], [x, y])
= SystemSolveResult(status: Solved, domain: complex, complete: True, completeness: Complete, solutions: [SystemSolution(bindings: [Binding(name: x, value: 0), Binding(name: y, value: -1)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: -1), Binding(name: y, value: 0)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: 1), Binding(name: y, value: 0)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: 0), Binding(name: y, value: 1)], conditions: [], exactness: Exact)], diagnostics: []) (SystemSolveResult)
```

Dispositions now mirror the C# proof ledger: solutions => Solved/Complete with empty conditions per
solution; a DERIVED contradiction => NoSolutions/Complete with an empty diagnostics array; an
abandoned branch => Unevaluated/complete:false with one `system-solve.*` diagnostic. Empty equation
lists use `system-solve.no-equations`; non-polynomial equations use
`system-solve.non-polynomial`; a missing elimination polynomial and an unrepresentable univariate
root use the reference's own `elimination-incomplete` / `univariate-incomplete` sentences.

The quadratic-radical path is the same change that makes the parabola/circle bindings printable:
`solve_full(x^2 - 2 == 0, x)` now answers `-1/2*sqrt(8)` and `1/2*sqrt(8)` exactly as the oracle
does (the kernel previously fell through to `Unevaluated`).

### Rows expected to unblock

All 21 rows of the round-34 family `system-solve nonlinear/contradiction`:

```text
GroebnerTests/CircleHyperbola_GrLex#0
GroebnerTests/Eliminate_X_FromCircleHyperbola#0
GroebnerTests/LexOrder_OfOrderedMonomials#0
GroebnerTests/Reduce_NormalForm_IsIdempotent#0
GroebnerTests/SinglePolynomial_BasisIsItself#0
GroebnerTests/TrivialIdeal_UnitGenerators_ProducesOne#0
SystemSolveProofTests/Kernel_AbsenceOfSolutions_IsUnevaluatedWithAStableNote#0
SystemSolveProofTests/Kernel_DerivedContradiction_IsNoSolutionsWithoutANote#0
SystemSolveProofTests/Kernel_EmptyEquationList_IsATypedRefusal#0
SystemSolveProofTests/NonlinearElimination_FindsBothSolutionsOfCircleAndLine#0
SystemSolveProofTests/NonlinearElimination_FindsBothSolutionsOfParabolaAndLine#0
SystemSolveProofTests/Wire_DerivedContradiction_IsCompleteNoSolutions#0
SystemSolveProofTests/Wire_DerivedContradiction_IsCompleteNoSolutions#1
SystemSolveProofTests/Wire_DerivedContradiction_IsCompleteNoSolutions#2
SystemSolveProofTests/Wire_EmptyEquationList_IsARecord_NotAClrException#0
SystemSolveProofTests/Wire_NonlinearShapes_PublishTheirTwoSolutions#0
SystemSolveProofTests/Wire_NonlinearShapes_PublishTheirTwoSolutions#1
SystemSolveTests/CircleHyperbola_FourSolutions#0
SystemSolveTests/Inconsistent_NoSolutions#0
SystemSolveTests/NonPolynomial_ReportsNote#0
SystemSolveTests/ParabolaLine_TwoSolutions#0
```

Corpus ids: `sym/071`, `sym/076`.

### Test map

| Census rows | Rust test |
|---|---|
| 6 GroebnerTests rows | `lovelace-symbolics::groebner::tests::{circle_hyperbola_grlex_basis_has_the_documented_three_elements, eliminate_x_from_circle_hyperbola_is_y_cubed_minus_y, lex_basis_of_ordered_monomials_keeps_three_elements, reduce_normal_form_is_idempotent, a_single_polynomial_is_its_own_basis, trivial_ideal_from_x_and_x_plus_one_is_one}` |
| `Kernel_DerivedContradiction...`, `Wire_DerivedContradiction...#0-#2`, `SystemSolveTests/Inconsistent...` | `lovelace-lang::corpus_and_system_solve_round36::system_solve_derived_contradictions_are_complete_no_solutions` |
| `Kernel_AbsenceOfSolutions...` | `...::system_solve_abandoned_branches_are_unevaluated_with_the_oracle_note` |
| `Kernel_EmptyEquationList...`, `Wire_EmptyEquationList...` | `...::system_solve_empty_equation_list_is_a_typed_refusal` |
| `NonlinearElimination...CircleAndLine`, `Wire_NonlinearShapes...#1` | `...::system_solve_circle_line_finds_both_exact_radical_solutions` |
| `NonlinearElimination...ParabolaAndLine`, `Wire_NonlinearShapes...#0`, `ParabolaLine_TwoSolutions` | `...::system_solve_parabola_line_finds_both_exact_radical_solutions` |
| `CircleHyperbola_FourSolutions` | `...::system_solve_circle_hyperbola_publishes_four_solutions` |
| `NonPolynomial_ReportsNote` | `...::system_solve_non_polynomial_reports_the_stable_note` |
| corpus sym/071 / sym/076 | `...::corpus_sym_071_optimize_lower_emits_the_documented_mathir_listing` / `...::corpus_sym_076_linsolve_keeps_the_bareiss_division_shape` |

`solve_system` and `solve_system_full` are pinned to publish the same record by
`...::system_solve_plain_and_full_publish_the_same_record`.

## 5. Gates observed on this tree

| Gate | Command | Result |
|---|---|---|
| Corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `PASS: oracle 20/20, rust 20/20` |
| CLI | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`; `PASS: every RUN-001..RUN-014 row behaves like the oracle` |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize` | generated 159 match / 41 mismatch / 0 error; replayed 774 match / 978 mismatch / 0 error; `new=0`. The same 200-case run at 12:17 (before this lane) had identical counts, so the delta is 0. |
| New tests | `cargo test -p lovelace-lang corpus_and_system_solve_round36` | 10 passed, 0 failed |
| New tests | `cargo test -p lovelace-symbolics groebner` | 7 passed, 0 failed |
| Workspace | `cargo test --workspace --no-fail-fast` | 4,691 passed, 2 failed (19 suites) |

The two workspace failures are pre-existing and outside every file this lane changed:

```text
dsp_round21::cos_given_pi_over_6_is_sqrt3_over_2
robustness_round13::round13_terminating_quotient_longer_than_the_budget_stays_exact
```

Evidence that they predate this work: the release test binary built at 12:01 (before this lane's
first edit), `rust/target/release/deps/lovelace_lang-efe395ab20442daa.exe`, fails both tests with
the same outputs. Both are Real precision / DSP-angle paths in `lovelace-real` / `lovelace-lang` DSP code that this lane did not touch. They need the precision lane, not a weakened assertion here.

The manifest gate is expected to be stale until the orchestrator regenerates the inventory and maps
the 21 rows above; this lane is forbidden to edit `parity/`.

## 6. Remains after this lane

- Portable mapping of the 21 census ids (inventory regeneration + manifest), owned by the
  orchestrator because `parity/` is out of scope here.
- The two pre-existing precision tests above (separate lane).
- The wider `solve complex algebraic roots` family is still blocked where the reference needs
  `RootOf` for non-real roots; this lane added only the positive-discriminant exact-radical case
  required by the system solver.
