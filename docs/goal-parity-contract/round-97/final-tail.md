# Round 97 — the final twelve mappable deferred rows

**Status: 12/12 rows closed and mapped (deferred 19 → 7).** Coverage is certified at
**6 084/6 209 (97.99 %)**: the only rows still deferred are the six `DifferentialOracleTests`
rows that need a live SymPy host, plus the empty `UnitTest1` placeholder. This report is the
deliverable for the twelve assigned classes; every assertion is the C# case's own observable.

Scope: `rust/lovelace-lang/src` and `rust/lovelace-symbolics/src` (production and tests),
`parity/manifest-map.json` and its generated artifacts, and this report. No `.cs`, fixture,
`parity/` gate script, desktop file or other doc was edited.

## 1. The twelve ids

| # | Census id | Rust test |
|---:|---|---|
| 1 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TimingScaleTests/OperationTiming_ExposesTheSameScaleAsItsDisplay#0` | `lovelace-lang::round97_tail::timingscaletests_operationtiming_exposes_the_same_scale_as_its_display` |
| 2 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxStructuredResultsTests/LinsolveFull_SingularSystem_ReportsStatusNotThrow#0` | `lovelace-lang::round97_tail::dxstructuredresultstests_linsolvefull_singularsystem_reportsstatusnotthrow` |
| 3 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.HyperbolicBuiltinsAndNegativeIntegerPowersTests/BuiltinCount_IncludesTheThreeHyperbolicFunctions#0` | `lovelace-lang::round97_tail::hyperbolicbuiltinsandnegativeintegerpowerstests_builtincount_includesthethreehyperbolicfunctions` |
| 4 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.InfiniteFamilyCompletenessTests/SympyOracle_ExpOverComplexes_IsAnImageSet_AndTheTemplateMatchesIt#0` | `lovelace-lang::round97_tail::infinitefamilycompletenesstests_sympyoracle_expovercomplexes_isanimageset_and_thetemplatematchesit` |
| 5 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.InverseBranchCandidateVerificationTests/EveryRepresentedSolutionSatisfiesTheOriginalEquation#4` | `lovelace-lang::round97_tail::inversebranchcandidateverificationtests_everyrepresentedsolutionsatisfiestheoriginalequation` |
| 6 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.LatexPrinterTests/KnownRendering_PowerOfAComplexConstant_IsAlreadyAGroup#0` | `lovelace-lang::round97_tail::latexprintertests_knownrendering_powerofacomplexconstant_isalreadyagroup` |
| 7 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.LatexSymbolNameEscapingTests/LatexSymbolName_InsideLargerExpressions_IsStillEscaped#0` | `lovelace-lang::round97_tail::latexsymbolnameescapingtests_latexsymbolname_insidelargerexpressions_isstillescaped` |
| 8 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MetamorphicSetClosureTests/CancelBuiltin_ReducesAndRefusesToCancelWhenNoFactorIsCommon#0` | `lovelace-lang::round97_tail::metamorphicsetclosuretests_cancelbuiltin_reducesandrefusestocancelwhennofactoriscommon` |
| 9 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PrettyParenthesesTests/ModelLevel_PowerOfAComplexConstant_HasADelimitedBase#0` | `lovelace-lang::round97_tail::prettyparenthesestests_modellevel_powerofacomplexconstant_hasadelimitedbase` |
| 10 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PropertyTests/MathIR_EvaluatesLikeSymbolic#0` | `lovelace-lang::round97_tail::propertytests_mathirevaluateslikesymbolic` |
| 11 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.SolveCompletenessPairingTests/EverySolveStatus_PublishesTheDocumentedPair_OnBothRecords#0` | `lovelace-lang::round97_tail::solvecompletenesspairingtests_everysolvestatus_publishesthedocumentedpair_onbothrecords` |
| 12 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.SystemSolveProofTests/NonlinearElimination_AndTheBoundary_AgreeWithSympySolve#0` | `lovelace-lang::round97_tail::systemsolveprooftests_nonlinearelimination_andtheboundary_agreewithsympysolve` |

All twelve ids resolve in the regenerated `parity/rust-tests.json` (lovelace-lang, 1 713 tests).

## 2. Per-row before/after

`oracle` is `dotnet Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll`; `rust` is
`rust/target/release/lovelace-run.exe`. "Before" values are the measured fields from the
round-45/50 sweeps and this round's pre-fix probes; "after" is the value both runners now publish
(or, for library-only rows, the value the Rust library now produces).

| # | Row | Before (measured) | After |
|---:|---|---|---|
| 1 | TimingScale | UNRUNNABLE: `OpTiming` had no elapsed time and no `ElapsedDisplay`/`ElapsedScale`; only the position/kind/output were recorded. | `OpTiming { elapsed: 3.5 ms }.elapsed_display() == "3.5 ms"`, `elapsed_scale() == (3.5, "ms")`; the engine measures every top-level statement. |
| 2 | DxStructuredResults | `$.exitCode`: `linsolve([[x,x],[x,x]],[1,1])` exited **0** with `result = [] (Vector)`; oracle exited **1** with code `InvalidOperation`, category `DomainError`, message `System is singular or underdetermined.` | Both runners: exit 1, `InvalidOperation`/`DomainError`, same message at the same position; `linsolve_full` still publishes `NoSolutions`/`complete:true`/`Complete`. |
| 3 | HyperbolicBuiltins | `no plugin-aware count API`; runner `--functions` total 123, C# Symbolics-only engine count 107. | `registry::builtin_count_for_plugins(&["Lovelace.Symbolics","Lovelace.MathIR"]) == 107`; total with DSP is 123; `asinh/acosh/atanh` are in the Symbolics set. |
| 4 | InfiniteFamilyCompleteness | Already matched: the only block was the C# row's SymPy session. Runner `solve_full(exp(x)-c == 0, x)` = `SolutionFamily(template: log(c) + 2*k*pi*i, parameter: k, period: 2*pi*i, parameter_domain: integer, exactness: ParametricExact)`; real `solve_full(exp(x)-5 == 0, x, real)` = `log(5)`. | Same values asserted by the mapped test, plus the residual `exp(log(c)+2*pi*k*i)-c` for every integer `k = -3..3` at `c = 5`. No production change was needed. |
| 5 | InverseBranchCandidateVerification #4 | Already matched (and the old round-33 comment that this row stayed `Unevaluated` was stale): `solve_full(sqrt(x^2+1)-2 == 0, x)` = `Solved`, complete, `represented_count = 2`, solutions `-1/2*sqrt(12)`, `1/2*sqrt(12)`. | Same values asserted, and each root substituted back into `sqrt(x^2+1)-2` leaves a zero residual. No production change was needed. |
| 6 | LatexPrinter | UNRUNNABLE: no model-level complex constant leaf, so the printer could not be driven to the C# shape. | Added `Expr::Complex`; `(1+1i)^x` renders `(1 + 1 i)^{x}` — the complex leaf is already a parenthesised LaTeX group, so no `\left...\right` pair is added. |
| 7 | LatexSymbolNameEscaping | The product `a%b * c&d` already escaped; the derivative node fell through to the generic call name and rendered `\operatorname{derivative}(f, x\_1)` instead of the C# `diff` spelling. | Product: `a\%b \cdot c\&d`; derivative: `\operatorname{diff}(f, x\_1)` (variable names escaped as names). |
| 8 | MetamorphicSetClosure | `cancel((x^2+1)/(x-1))` was `x + 1` in Rust (the univariate division ignored its remainder); oracle kept the unchanged quotient. | `cancel` now uses `RationalFunctions.Cancel` (exact common factor only): `(x^2+1)/(x-1)` unchanged, `(x^2-1)/(x-1)` → `x + 1`, `x/x` → `1`, matching the oracle on all three. |
| 9 | PrettyParentheses | UNRUNNABLE: no complex constant leaf; the power base could not be constructed by the language. | Added `Expr::Complex` and the power-base rule: `(1 + i)^x` and `(-1 + i)^x` — the base text is exactly one delimited group while the leaf stays an atom in the precedence table. |
| 10 | Property/MathIR | `MathIR disagrees for (add (rat 5 3) (mul (rat -1 1) (add (fn sin (sym x)) (mul (rat -1 1) (fn exp (sym x))))) (mul (rat 2 1) (sym x))) at x=1/3: direct = Real{int:…163898, exp:-50}; lowered = Real{int:…163897, exp:-50}` (1 ulp in the 50th decimal). | `numeric::evaluate` folds Add/Mul in the reference's canonical TermOrder, `IrProgram::lower` emits the same operand order, and `numeric::evaluate_bound` binds `x` instead of substituting it. Direct and lowered are both `…163897` at `x=1/3`, and the full 60-expression corpus (FNV `6 946 605 504 391 048 442`) agrees exactly at all three points. |
| 11 | SolveCompletenessPairing | `solve_system_full` on the 2^8 system published `Solved / complete:true / Complete` with the 128 collected records; oracle published `Partial / false / Partial`. | `SystemOutcome::PartialSolutions` publishes `Partial / false / Partial`, the same 128 solutions in enumeration order, and an empty diagnostics array. |
| 12 | SystemSolveProof | Already matched: only the C# SymPy comparison was unavailable. Both systems publish `Solved`, complete, two exact solutions; both inconsistent shapes publish `NoSolutions`, complete. | Same values asserted, with the C# 38/39-digit literals (`0.618…`, `-1.618…`, `±0.707…`) compared to `1e-24` and the exact `x = y` line relation pinned. No production change was needed. |

### Probe evidence (selected)

```text
# before (round-45 measured families)
$.exitCode: linsolve([[x,x],[x,x]],[1,1]) exits 0 in Rust vs 1 in the oracle
$.result.display: cancel((x^2+1)/(x-1)) is x + 1 in Rust vs (x^2 + 1)/(x - 1)
8x2^8 system publishes Solved/complete True/Complete vs Partial/false/Partial

# after (this round, same --file scripts on both runners)
linsolve(...)          code=InvalidOperation category=DomainError message="System is singular or underdetermined."  (both)
cancel((x^2+1)/(x-1))  "(x^2 + 1)/(x - 1)"  (both)
cancel((x^2-1)/(x-1))  "x + 1"               (both)
cancel(x/x)            "1"                    (both)
2^8 system             status=Partial complete=false completeness=Partial solutions=128 diagnostics=0  (both)
```

The Property before/after is the exact panic message of the previously ignored
`lovelace-corpus::property_assertions::mathir_evaluates_like_symbolic` probe
(`…16898` direct vs `…16897` lowered); the production path is now covered inside
`lovelace-lang` by test #10 with the corpus pinned by its FNV hash.

## 3. Production changes

* `rust/lovelace-lang/src/lib.rs` — `OpTiming.elapsed` plus `elapsed_display()` /
  `elapsed_scale()` and the one `Timing.Scale` selector; each statement is timed with
  `Instant::now()`.
* `rust/lovelace-lang/src/builtins.rs` — singular `linsolve` is a typed
  `InvalidOperation`/`DomainError`; `cancel` uses `RationalFunctions.Cancel`;
  `SystemOutcome::PartialSolutions` and `system_partial_result` publish the enumeration cap;
  `cancel_full`'s exact-division guard is unchanged.
* `rust/lovelace-lang/src/registry.rs` — `builtin_count_for_plugins`.
* `rust/lovelace-lang/src/numeric.rs` — canonical Add/Mul fold order and
  `evaluate_bound` (the reference's `EvaluateToNum(e, ctx, bindings)`).
* `rust/lovelace-lang/src/mathir.rs` — `IrProgram::lower` emits the reference's canonical
  operand order; `Expr::Complex` lowers to a `(cplx …)` constant and evaluates.
* `rust/lovelace-symbolics/src/lib.rs` — `Expr::Complex(Rat, Rat)`, its canonical/pretty/
  LaTeX/debug renderings, exactness/domain/node/rank arms, canonical parse for `(cplx …)`,
  the pretty power-base grouping and the LaTeX derivative arm.
* Supporting exhaustive-match arms in both crates (`assumptions.rs`, `rewrite_registry.rs`,
  `series_kink.rs`, `rational_functions.rs`, `symbolic_matrix.rs`, `context.rs`,
  `evaluation_boundary.rs`, `print_budget.rs`, test-side evaluators).

## 4. Tests

`rust/lovelace-lang/src/round97_tail.rs` contains one test per id (12 tests). Every test states
the C# assertion: typed renderings, statuses, record types, exact roots, the 107/123 counts,
the canonical family, the three cancel renderings, the complex printings, the pinned MathIR
corpus and the two system-solve solution pairs. No existing test expectation was weakened.

Two existing tests were **corrected** because the production fix changed the reference-shaped
IR order: `mathir_round54`'s four `compile_kernel`/`compile` IR-text expectations and the
`typing_infers_the_documented_tiers` vector now use the canonical Constant-first order the C#
`Lowering.Lower` emits (verified on the oracle with `lower(x^2+1, [x])`: `Constant 0 0`
before `Parameter 0 0`). This is an oracle-backed correction, not a relaxation.

## 5. Gates (observed)

```text
cargo test --workspace
  61 + 0 + 18 + 0 + 317 + 129 + 7 + 1713 + 264 + 2487 + 429 + 1 + 196 passed, 0 failed, 1 ignored
  => 5 622 passed / 1 ignored (the tree also contains 40 desktop-lane tests; the round-96
     baseline was 5 570 and this round adds the 12 tests above)

powershell -File parity/gen-rust-tests.ps1
  crates=11 tests=5623 suiteExit=0 passed=5622 failed=0

node parity/gen-manifest.mjs
  rows=6209 ported=6084 deferred=7 excluded=118 coveragePercent=97.99 rustTests=5623

node parity/tools/gate-manifest.mjs
  PASS (structural): 6209 test cases dispositioned; 6084 claim Rust coverage (97.99%). failures=[]

node parity/tools/coverage-report.mjs
  coverage.md written: 6084/6209 ported (97.99%)

node parity/tools/gate-fixtures.mjs
  oracle: 20/20; rust: 20/20; PASS: oracle 20/20, rust 20/20

node parity/tools/gate-cli.mjs
  CLI matrix: 16/16; PASS: every RUN-001..RUN-014 row behaves like the oracle

node parity/diff/run.mjs --cases 200 --seed 1
  generated=200 replayed=1758 total=1958
  GENERATED 200 match 0 mismatch 0 error
  REPLAYED 1758 match 0 mismatch 0 error
  stillMismatching=0 new=0; harness exit 0

cargo run --release -p lovelace-corpus
  corpus: 214 passed, 0 failed, 214 total

cargo fmt -p lovelace-lang -p lovelace-symbolics --check        exit 0
cargo clippy -p lovelace-lang -p lovelace-symbolics --all-targets -- -D warnings   exit 0
```

**Workspace-wide fmt/clippy are not green, and the failure is entirely in the concurrently
edited desktop lane, not in this lane's crates.** Observed:

* `cargo clippy --workspace --all-targets -- -D warnings` stops in `lovelace-desktop`:
  `history.rs:124` (`derivable_impls`), `run_state.rs:349` (`match_like_matches_macro`),
  `ui/plot.rs:1445` (`needless_borrows_for_generic_args`), `workbench.rs:2072` and `3467`
  (`unnecessary_map_or`).
* `cargo fmt --all --check` reports diffs throughout `rust/lovelace-desktop/src`
  (`workbench.rs`, `ui/*.rs`, `gui_selftest.rs`, …).

Per the task's instruction, those files were not touched; the failing gates are reported here
and the lane stops on the desktop boundary. The workspace test suite itself is green because
the desktop package's 317 tests pass.

## 6. Files touched

* `rust/lovelace-lang/src/{lib,builtins,registry,numeric,mathir,context,evaluation_boundary,print_budget,mathir_round54,circular_family_round31}.rs`
* `rust/lovelace-lang/src/round97_tail.rs` (new)
* `rust/lovelace-symbolics/src/{lib,assumptions,rewrite_registry,series_kink,rational_functions,symbolic_matrix,complex_closed_forms_round30,round90_symbolic_families}.rs`
* `parity/manifest-map.json` (12 curated entries: deferred → ported)
* Generated: `parity/manifest.toml`, `parity/rust-tests.json`, `parity/coverage.md`
* This report.

Not touched: any `.cs` file, fixture, `parity/` gate script, `rust/lovelace-desktop`, or doc
outside this report.
