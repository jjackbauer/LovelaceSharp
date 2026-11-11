# Round 12 — Suite / Symbolics / Real batch: 104 verified mappings (75 net-new)

Scope: `rust/lovelace-lang/src/suite_parity_batch.rs`, `rust/lovelace-real/src/real_parity_round10.rs`
(tests only, appended to already-wired modules — no `lib.rs` wiring was added) and
`parity/manifest-map.json`. No production line, no `.cs`, no fixture, no corpus example changed.

## 1. Result

Coverage moved from **3,916/6,209 (63.07 %)** to **3,991/6,209 (64.28 %)** — **75 net-new
mappings**, backed by **76 new Rust tests** (68 in `lovelace-lang`, 8 in `lovelace-real`). The
batch **claims 104 census rows**: 29 of them already carried a `ported` disposition (they are now
also backed by this batch's tests, which assert the same contract), so the observable gain is 75.

| C# test project | Cases | Before | After | Delta |
|---|---:|---:|---:|---:|
| Lovelace.Suite.Tests | 929 | 564 | 580 | **+16** |
| Lovelace.Symbolics.Tests | 1,226 | 399 | 452 | **+53** |
| Lovelace.Real.Tests | 2,537 | 2,296 | 2,302 | **+6** |
| **batch total** | | | | **+75 net / 104 rows** |

Per class (rows claimed): SignPrecedenceTests 15 · SuiteEngineTests 4 ·
SuiteEngineOutputPlotTests 1 · DiagnosticContractTests 10 · DxSemanticClosureTests 15 ·
HyperbolicBuiltinsAndNegativeIntegerPowersTests 3 · SolveCompletenessMappingTests 6 ·
SolveCompletenessPairingTests 5 · ExactnessFlagTests 22 · InfiniteFamilyCompletenessTests 1 ·
RelationsAndFamiliesTests 3 · AcceptanceTests 7 · RealDivideTests 2 · RealPowZeroBaseTests 6 ·
RealConstructorTests 2.

## 2. What was ported

* **Sign precedence (15).** Six AST-shape rows (`-x^2` is `Neg(Pow)`, `(-x)^2` is
  `Pow(Neg)`, `2^-1`, `-1..2`, `1..10^2`, `-5!`) and seven evaluation rows, plus `subs(-x^2,
  x, 3) = -9` and the canonical-form guard (`(pow (sym x) (rat 2 1))`, never `(pow (mul`). The
  eighth evaluation row (`+2^2`) is refused: this crate has no unary-plus production.
* **Engine surface (5).** `abs` resolves as a registered builtin, `1 + 1` stores `2 (Natural)`
  under `_`, `parse("x = 1; y = 2; x + y")` has three statements, `parse_expression("1 + 2")` is
  `Binary(Add)`, and `plot(1..3, [1, 4, 9])` captures an SVG starting with `<svg`.
* **Rich-result diagnostics (10).** All seven records (Solve/SystemSolve/Limit/Integration/
  Transform/Optimization/Compilation) end with a `diagnostics` **array** whose shape[0] is the
  element count; the Partial solve carries `Diagnostic(code, category, message, recoverable,
  location, details)` with `solve.unrepresented-roots` and an `ErrorCategory` member; Complete
  solve and Satisfied transform carry empty arrays.
* **Solver semantics (26).** Simplify safety (`x/x`, `exp(log(x))`, `sin^2+cos^2`, the
  non-zero assumption), the `rat.cancel-x-over-x` Conditional step, the solver domain contract
  (complex vs real, unsupported `integer`/`rational`), cubic real-domain filtering, principal
  branch `-3,1`, and the published `(status, complete, completeness)` pairing for Solved /
  NoSolutions / Partial on both records.
* **Exactness (22).** The 20-row leaf rule (every elementary function of an exact argument is
  exact), `inspect(e).exact` over ten exact closed forms, and the quintic's RootOf roots as
  AlgebraicExact.
* **Relations and families (4).** The assumption display/atom count, `sin(x) = 0` family
  (`k*pi`, parameter `k`, Integer domain) via both `solve` and `solve_full`.
* **Acceptance corpus (7).** Canonical invariants, differentiation, factoring (with expand-back),
  the quartic/quadratic root sets, series coefficients, and `det`.
* **Real (10).** Division scale/period metadata for `1/0.9` and `1/0.99`, zero-base power
  refusals (negative) and zeros (positive), and the copy/assign deep-copy facts.

## 3. Refusals (each measured, not assumed)

| Pool | What fails | Evidence |
|---|---|---|
| SignPrecedenceTests | `+2^2` | `ParseError "Unexpected token '+'"` |
| SuiteEngineTests | 10 rows | no CaptureState snapshot, no elapsed/statement timing display, no events, no SetVariable/RemoveVariable, no persistent Diagnostics |
| SuiteEngineOutputPlotTests | 2 rows | no output-writer parameter, no ResetPlotCapture |
| DiagnosticContractTests | 4 rows | no ErrorCategory enum type; the 420-redex script **overflows the stack**; `solve_full(x == x)` says "No solver for this structure." not "0 = 0: every value is a solution."; `simplify_full(x/x)` under `assume(x == 0)` is Satisfied, not Unsatisfiable |
| DxSemanticClosureTests | 14 rows | prover returns symbolic relations instead of True/False; no AssumptionSet/Tristate/MaxSteps/trace API; `x^4+1` is Partial not Unevaluated; multiplicity 1 not 2; `x^-1` domain complex not rational |
| Hyperbolic… | 11 rows | asinh/acosh/atanh unregistered; no help surface; `2^(1/2)` message differs; `x^-1` renders `x^(-1)`; `0^-1` message differs |
| ExactnessFlagTests | 7 rows | no approximation (RealConstant) leaf and no RootOf node; `inspect(sqrt(2)).exact` is true; driving `solve_full(x^3-2…)` **panics** in lovelace-symbolics (index out of bounds) |
| InfiniteFamilyCompletenessTests | 18 rows | exp/sin/cos inverse families report Unevaluated; `x*exp(x)==1` reports Solved (circular); `sqrt(x)==x` Unevaluated |
| RelationsAndFamiliesTests | 10 rows | no Tristate/canonical-parser/IR/numeric model API; `cos(x)=1/2`, `sin(x)=2`, `exp(x)=0` report Unevaluated; `integrate_full(1/x)` is SolvedExact without the x>0 condition |
| AcceptanceTests | 12 rows | no NumOps/canonical parser/IR/interning/assumptions; product-rule expand differs in factor order; Hessian mixed partials not structurally equal |
| RealDivideTests | 3 rows | `Real::div` emits one guard digit and marks the quotient inexact instead of the exact reference value |
| RealConstructorTests / RealPow / RealPi / RealSqrt / RealModulus / RealToString / RealProperties / RealAsyncLocal / RealParse / RealPiSegment | 78 rows | no f64/decimal constructors, no modulus, no Pi/Sqrt refusal paths (usize/zero-returning), no DisplayDecimalPlaces/MaxComputationDecimalPlaces knobs, no TryFormat, no async/batch, no precision scopes, no PiSegment |

## 4. Observed gates

```text
powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1   # exit 0
  crates=11 tests=3639 suiteExit=0 passed=3639 failed=0

node parity/gen-manifest.mjs                                         # exit 0
  { "rows": 6209, "dispositions": { "excluded": 118, "ported": 3991, "deferred": 2100 },
    "coveragePercent": 64.28, "rustTests": 3639 }

node parity/tools/gate-manifest.mjs                                  # exit 0
  { "censusCases": 6209, "manifestRows": 6209,
    "dispositions": { "ported": 3991, "consolidated": 0, "excluded": 118, "deferred": 2100 },
    "coveragePercent": 64.28, "claimedRustTests": 3991, "projectsInCensus": 15, "failures": [] }
  PASS (structural): 6209 test cases dispositioned; 3991 claim Rust coverage (64.28%).

node parity/tools/coverage-report.mjs                                # exit 0
  coverage.md written: 3991/6209 ported (64.28%)

powershell -File parity/gate-fixtures.ps1                            # exit 0
  oracle: 20/20 / rust: 20/20 / PASS: oracle 20/20, rust 20/20

cargo run --release -p lovelace-corpus                               # exit 1 (pre-existing)
  corpus: 212 passed, 2 failed, 214 total    (sym/071, sym/076)
```

`-ExecutionPolicy Bypass` was required because this host refuses unsigned scripts
(`gen-rust-tests.ps1 is not digitally signed`); the command is otherwise the documented one.

## 5. Divergences found (production is out of scope; not fixed)

1. A 420-term `simplify_full` overflows the stack (STATUS_STACK_OVERFLOW) instead of returning a
   BudgetExceeded record.
2. `solve_full(x^3 - 2 == 0, x, real)` panics: index out of bounds at
   `rust/lovelace-symbolics/src/lib.rs:2382`.
3. The inverse-family solver is incomplete: `exp(x) = c`, `sin(x) = 1/2`, `cos(x) = 1/2`,
   `tan(x) = 1`, `sin(x) = 2`, `exp(x) = 0` report Unevaluated; only `sin(x) = 0` publishes
   the family.
4. Root multiplicity is lost (`x^2 - 2x + 1` reports multiplicity 1).
5. `Real::div` emits one guard digit beyond the requested budget and marks the quotient inexact.
6. The infallible readers used by the reference facts (`evalf` of constants, `sqrt` of a
   negative, `pi(0)`) are absent: Rust refuses, answers zero, or publishes a different flag.

## 6. Files changed

| file | change |
|---|---|
| `rust/lovelace-lang/src/suite_parity_batch.rs` | +68 tests appended (Round 12 section) |
| `rust/lovelace-real/src/real_parity_round10.rs` | +8 tests appended (Round 12 section) |
| `parity/manifest-map.json` | 104 `ported` rows (notes carry C# file:line and the Rust test) |
| `parity/manifest.toml`, `parity/coverage.md` | regenerated (not hand-edited) |
| `docs/goal-parity-contract/round-12/suite-real-batch.md` | this report |

## 7. Not verified / limits

- **The C# tests were not executed** (the VSTest host cannot run in this environment; see
  `parity/README.md`). Every expected value is the C# assertion transcribed from the source, and
  every Rust assertion was executed green.
- **Concurrent writes.** The error-taxonomy implementer owns
  `rust/lovelace-lang/src/{lib,builtins,syntax}.rs` and `rust/lovelace-run/src/main.rs` this
  round and changed them while this batch ran (e.g. `LangError` gained
  `ErrorSignature/suggestions/hints/registry_size`). This batch therefore appended only to
  already-wired test modules; the full workspace suite was re-run after those edits and is green.
- The 214-example corpus stays at 212/214 with the two pre-existing failures; no corpus example
  was touched.
- A few refusals are behavioural gaps rather than missing APIs (solver families, multiplicity,
  div guard digit); they would need production work, which this batch is not allowed to make.
