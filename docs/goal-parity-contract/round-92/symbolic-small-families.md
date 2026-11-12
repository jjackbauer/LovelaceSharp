# Round 92 — six small deferred symbolic families (12 rows)

**Status: 12/12 rows fixed or mapped, 0 blocked by the rows themselves; every gate is green on
the final tree.** Six of the twelve rows needed production work (the two recursion rows, the two
DxSemanticClosure rows, the missing solve-completeness mapping table and the missing rewrite proof
bridge); the other six were already equal at the row's own surface and are now pinned by a test
(the two complex provenance rows and the two linsolve rows) or supplied by the missing data surface
(the two proof-bridge rows). Other lanes were mid-edit while this round ran; §6 records the
transient blockers and the final re-run that cleared them.

Measured 2026-11-12 (America/Sao_Paulo) against
`Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll` and
`rust/target/release/lovelace-run.exe`. Probe scripts live under
`rust/target/tmp/r92/` (git-ignored). Scope was `rust/lovelace-symbolics/src` and
`rust/lovelace-lang/src`; no `.cs`, fixture, corpus, `parity/` or `rust/lovelace-desktop`
file was edited, and no document other than this report was written.

## 1. The 12 ids (extracted from parity/manifest.toml)

Every row below has `disposition = "deferred"` in `parity/manifest.toml` (the manifest is
generator-owned; this round did not edit it). Each of the six census classes has exactly these two
deferred rows.

```text
Lovelace.Complex.Tests/Lovelace.Complex.Tests.ComplexMathInexactArgumentProvenanceTests/SymbolicNodeAndNumericResult_AgreeForSine#0
Lovelace.Complex.Tests/Lovelace.Complex.Tests.ComplexMathInexactArgumentProvenanceTests/SymbolicNodeAndNumericResult_AgreeForTheSameExpression#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecursionBudgetCompositionTests/TheDeepestAcceptedCall_Answers_InEveryComposition#14
Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecursionBudgetCompositionTests/TheDeepestAcceptedCall_Answers_InEveryComposition#15
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxSemanticClosureTests/DomainOf_NegativeIntegerPower_IsRationalNotInteger#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxSemanticClosureTests/Solve_QuarticNoRealRoots_DefaultDomain_ReportsUnevaluated#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.LinsolveCheckTests/Solve_3x3_VerifiesByNumericSubstitution#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.LinsolveCheckTests/Solve_VerifiesByNumericSubstitution#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.SolveCompletenessMappingTests/BudgetExceeded_IsIncomplete_WithTheKernelCompletenessOfThePartialSubset#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.SolveCompletenessMappingTests/EveryStatusCarriesTheDocumentedPair#3
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RewriteProofBridgeTests/EveryObligation_IsStatedAndHonestAboutItsStatus#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RewriteProofBridgeTests/EveryShippedRule_HasExactlyOneProofObligation#0
```

| # | Class | Id (short) | Surface |
|---|---|---|---|
| 1 | ComplexMathInexactArgumentProvenance | `.../SymbolicNodeAndNumericResult_AgreeForSine#0` | library `Expr` + `EvaluateToNum` |
| 2 | ComplexMathInexactArgumentProvenance | `.../SymbolicNodeAndNumericResult_AgreeForTheSameExpression#0` | library `Expr` + `EvaluateToNum` |
| 3 | RecursionBudgetComposition | `.../TheDeepestAcceptedCall_Answers_InEveryComposition#14` | language `for` body |
| 4 | RecursionBudgetComposition | `.../TheDeepestAcceptedCall_Answers_InEveryComposition#15` | language `while` body |
| 5 | DxSemanticClosure | `.../DomainOf_NegativeIntegerPower_IsRationalNotInteger#0` | library `Domains.DomainOf` + assumptions |
| 6 | DxSemanticClosure | `.../Solve_QuarticNoRealRoots_DefaultDomain_ReportsUnevaluated#0` | library `Solvers.Solve` |
| 7 | LinsolveCheck | `.../Solve_3x3_VerifiesByNumericSubstitution#0` | library `SymbolicMatrix.Solve` |
| 8 | LinsolveCheck | `.../Solve_VerifiesByNumericSubstitution#0` | library `SymbolicMatrix.Solve` |
| 9 | SolveCompletenessMapping | `.../BudgetExceeded_IsIncomplete_WithTheKernelCompletenessOfThePartialSubset#0` | library mapping |
| 10 | SolveCompletenessMapping | `.../EveryStatusCarriesTheDocumentedPair#3` | library mapping |
| 11 | RewriteProofBridge | `.../EveryObligation_IsStatedAndHonestAboutItsStatus#0` | library registry + obligations |
| 12 | RewriteProofBridge | `.../EveryShippedRule_HasExactlyOneProofObligation#0` | library registry + obligations |

## 2. Probes and per-row before/after

The differential's four volatile keys (`revision`, `elapsed`, `elapsedTime`, `timings`) were
removed before comparing envelopes. Library-only rows cannot be driven by either CLI (the C# rows
themselves construct `Expr`s and call the kernel API), so they were measured at the library
surface and, where the language has an analogue, that analogue was also run on both runners.

| Row | Probe | Oracle | Rust before | Rust after |
|---|---|---|---|---|
| 1 | library `Expr.Real(pi(30))` + `sin`; CLI analogue `evalf(sin(pi(30)), 30)` | node `IsExact=false`; numeric `IsExact=false`, value = `ComplexMath.Sin(pi30)`; CLI `structured.exact=false`, display `0` | same API was reachable: node and numeric both inexact, CLI same | unchanged; pinned by test |
| 2 | library `Expr.Real(pi(30))` + `cos`; CLI analogue `evalf(cos(pi(30)), 30)` | node/numeric `IsExact=false`, value = `ComplexMath.Cos(pi30)` = `-1`; CLI `exact=false`, display `-1` | same | unchanged; pinned by test |
| 3 | `func f(n) { if (n == 0) { return 0 }; return f(n - 1) }; for i in 1..5 { f(85) }` | `$.result = {kind:"Natural", display:"0", structured:{value:"0", exact:true}}`; `_.kind=Natural`; last timing `resultKind:"Natural"` | no `result` key (loop answered `Void`); `_` absent; last timing `resultKind:"Void"` | `$.result = Natural 0`, `_ = Natural 0`, last timing `Natural` |
| 4 | `...; i = 0; while (i < 5) { i = i + 1; f(85) }` | same `Natural 0` result, `_ = Natural 0` | no `result`; last timing `Void` | `Natural 0`, `_ = Natural 0` |
| 5 | library `DomainOf(x^-1)` under `x : Integer`; control `DomainOf(x^2)` | `Rational`; control `Integer` | no assumption-scoped API; `Expr::domain()` ignored the scope and returned `Complex` for `x^-1` | `domain_of(&x^-1, assumptions) = Rational`; `x^2 = Integer` |
| 6 | library `Solve(x^4+1 == 0, x)`; CLI `solve_full(x^4 + 1 == 0, x)` | `Status=Unevaluated`, `Complete=false`, `Completeness=Unknown`, note/reason `complex algebraic roots not supported (RootOf is real-only in v1).`, `UnrepresentedCount=4`; CLI already `Unevaluated/false/Unknown` | `solve_polynomial_named` answered `Partial([])` with no note; CLI was already `Unevaluated` | `solve_expression` answers `Unevaluated` with the note and count 4; real domain `NoSolutions/"no real solutions"` |
| 7 | library 3x3 `SymbolicMatrix::Solve`; CLI analogue `linsolve(...)` | the C# row substitutes `x=2,y=3` and asserts rows `2*v0+v1=1`, `v0+2*v1+v2=0`, `v1+3*v2=1` | API already existed; no test pinned the row; numeric substitution verifies | test passes; residuals exactly 0/1 |
| 8 | library 2x2 `SymbolicMatrix::Solve`; solution `[-x/x/(x*y), x/(x*y)]` | C# substitutes two `(x,y)` pairs and asserts `x*v0+v1=0`, `y*v1=1` | API already existed; no test pinned the row; substitution verifies | test passes for `(2,3)` and `(-1/2,5/2)` |
| 9 | mapping `Of(BudgetExceeded)` and `Of(BudgetExceeded, Partial)` | `(false, Unknown)`; `(false, Partial)` | no mapping API | `SolveCompletenessMapping::of` / `of_with_partial_subset` return those pairs |
| 10 | mapping `Of(Unevaluated)` (row 3 of the InlineData), plus all four statuses; CLI `solve_full(x^4+1)` | `Unevaluated -> (false, Unknown)`; `Solved/NoSolutions -> (true, Complete)`, `Partial -> (false, Partial)`; CLI triple `Unevaluated/false/Unknown` | no mapping API; CLI triple already matched | mapping table returns all documented pairs; CLI unchanged |
| 11 | compare sorted `shipped_rule_ids()` with sorted obligation rule ids | the oracle has 9 shipped rules, each with exactly one obligation | registry already exposed the 9 ids; no obligation table existed | `proof_bridge::proof_obligations()` has the 9 ids; sorted lists equal |
| 12 | each obligation: non-empty statement; `Theorem` null unless `MachineChecked` | 9 statements, all `Unproven`, all theorem null | no obligation table existed | test asserts non-empty statements, null theorems for non-machine-checked, all 9 `Unproven` |

The two proof-bridge rows are a **map, not invented behaviour**: the registry ids and all nine
statements already matched the oracle's data; the missing Rust surface was the obligation table
itself. No rewrite rule changed.

## 3. Production changes

| File | Change |
|---|---|
| `rust/lovelace-lang/src/lib.rs` | `while`/`for` now answer the last completed body value instead of `Void` (rows 3-4). `break`/`continue` keep the previous completed value; an empty loop still answers `Void`. |
| `rust/lovelace-symbolics/src/lib.rs` | `Expr::domain_under(&AssumptionSet)` and `domain_of(&Expr, &AssumptionSet)`; `Expr::domain()` delegates with the empty scope. Symbol domains come from the narrowest assumption atom; a non-integer power is real under a proven non-negative symbol. (row 5) |
| `rust/lovelace-symbolics/src/solution_set.rs` (new) | `SolveDomain`, `SolveStatus`, `Completeness`, `SolutionSet`, `solve_expression`/`solve_polynomial_set` with the reference's default-domain policy, and `SolveCompletenessMapping`. (rows 6, 9, 10) |
| `rust/lovelace-symbolics/src/proof_bridge.rs` (new) | `ProofStatus`, `ProofObligation`, `proof_obligations`, `proof_obligation_for`, `bridge_is_in_step`; the reference's nine statements are data. (rows 11-12) |
| `rust/lovelace-lang/src/round19_batch.rs` | Removed the stale skip that exempted the `for`/`while` compositions from the existing deepest-accepted-call test; the test now asserts the oracle's `0` for all 16 shapes. |

No production behaviour changed for rows 1, 2, 7 and 8: their APIs already agreed with the oracle;
this round pins them. Row 10's language triple also already agreed; only the mapping API was
missing.

## 4. Tests (one per row)

`rust/lovelace-lang/src/round92_symbolic_families.rs` (6):

- `complexmathinexactargumentprovenancetests_symbolicnodeandnumericresult_agree_for_the_same_expression` — row 2
- `complexmathinexactargumentprovenancetests_symbolicnodeandnumericresult_agree_for_sine` — row 1
- `linsolvechecktests_solve_verifiesbynumericsubstitution` — row 8
- `linsolvechecktests_solve3x3_verifiesbynumericsubstitution` — row 7
- `recursionbudgetcompositiontests_thedeepestacceptedcall_answers_in_a_for_body` — row 3
- `recursionbudgetcompositiontests_thedeepestacceptedcall_answers_in_a_while_body` — row 4

`rust/lovelace-symbolics/src/round92_symbolic_families.rs` (6):

- `dxsemanticclosuretests_domainof_negativeintegerpower_isrationalnotinteger` — row 5
- `dxsemanticclosuretests_solve_quartic_no_real_roots_default_domain_reports_unevaluated` — row 6
- `solvecompletenessmappingtests_budgetexceeded_is_incomplete_with_the_kernel_completeness` — row 9
- `solvecompletenessmappingtests_everystatuscarriesthedocumentedpair` — row 10
- `rewriteproofbridgetests_everyshippedrule_hasexactlyoneproofobligation` — row 12
- `rewriteproofbridgetests_everyobligation_isstatedandhonestaboutitsstatus` — row 11

Measured in isolation:

```text
cargo test -p lovelace-symbolics round92
  6 passed; 0 failed; 0 ignored; 190 filtered out

cargo test -p lovelace-lang round92
  6 passed; 0 failed; 0 ignored; 1653 filtered out
```

## 5. Gates (final tree)

| Gate | Command | Observed | Exit |
|---|---|---|---|
| Workspace suite | `cargo test --workspace --no-fail-fast` | **5,508 passed, 0 failed, 1 ignored** over 21 binaries; includes this round's 12 tests and the concurrent lanes' additions | 0 |
| Symbolics suite | `cargo test -p lovelace-symbolics` | 196 passed, 0 failed, 0 ignored (6 new) | 0 |
| Language suite | part of the workspace run | 1,684 passed, 0 failed (6 new) | 0 |
| Language round-92 tests | `cargo test -p lovelace-lang round92` | 6 passed, 0 failed | 0 |
| Documentation corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20` | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16` | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1` | generated 200/200 match, regressions 1,758/1,758 match, 0 mismatch, 0 error; `harnessExit: 0` | 0 |
| Clippy | `cargo clippy -p lovelace-symbolics -p lovelace-lang --all-targets -- -D warnings` | clean | 0 |
| Format | `cargo fmt -p lovelace-symbolics -p lovelace-lang -- --check` | clean | 0 |

The release runner was rebuilt on the final tree (`cargo build --release -p lovelace-run`) and
re-probed after the loop fix; both recursion envelopes now equal the oracle (rows 3-4). The
workspace count is higher than the 5,452 baseline because concurrent lanes added their own tests;
this round contributed 12 of the 5,508 and changed no test expectation.

## 6. Concurrent-lane activity

The task warned that another lane edits `lovelace-run`. During this round three adjacent lanes were
mid-edit in `lovelace-run`, `lovelace-lang` and `lovelace-desktop` at once. I did not touch or
overwrite any of their files; my changes in `lovelace-lang/src/lib.rs` are exactly the loop
value block (around lines 1323-1380) plus the test-module registration. Transient states observed
while they worked, and cleared on the final re-run:

- `lovelace-desktop/src/ui/app.rs:3130` `error[E0061]` (five-argument
  `paint_result_section`);
- `docs_sync::tests::parser_reads_examples_and_snippets_like_the_reference` failing 0 vs 1 in
  the lane's new `docs_sync.rs`;
- `cancellation_batch::a_run_that_completes_past_its_deadline_is_a_success_with_a_truthful_ledger`
  timing out past a 20 ms budget in one run;
- `assert!(MAX_SUGGESTIONS == 3)` triggering `clippy::assertions-on-constants` in the lane's
  `round92_runner_families.rs`;
- unformatted runner-lane files.

All five cleared when the lanes finished; the final commands above are the re-runs on the settled
tree.

**Rows left blocked: 0.** All twelve census ids are closed at their own surface.
