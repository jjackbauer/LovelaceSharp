# Round 90 — the four deferred symbolic families (14 rows)

**Status: fixed and pinned, 14/14 rows closed, 0 blocked.** The four families the round-45
sweep filed as UNRUNNABLE because "no Rust API exists" are now either fixed (six production
causes) or closed by a test that drives the API which now does exist, against the oracle's own
observable. Measured 2026-11-12 (America/Sao_Paulo) against
`Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll` and
`rust/target/release/lovelace-run.exe`.

The scope was `rust/lovelace-symbolics/src` and `rust/lovelace-lang/src` (production and
tests). No `.cs`, fixture, corpus, `parity/`, `rust/lovelace-desktop` file or other document was
edited. Probe scripts and captured envelopes live under `rust/target/tmp/r90/` (git-ignored).

## 1. The 14 ids (extracted from parity/manifest.toml)

Every row below has `disposition = "deferred"` in `parity/manifest.toml` (the manifest is
generator-owned; this round did not edit it). The class names are the census classes the round-45
grouping used.

| # | Class | Id | Runner probe? |
|---|---|---|---|
| 1 | DomainFailureVocabulary | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DomainFailureVocabularyTests/DivisionByZero_IsAnEvaluationFailure#0` | yes, `1/0` |
| 2 | DomainFailureVocabulary | `.../DivisionByZero_OnTheRealPath_IsAnEvaluationFailure#0` | yes, `1/0.0` |
| 3 | DomainFailureVocabulary | `.../LogOfZero_IsAnEvaluationFailure#0` | yes, `evalf(log(0), 20)` |
| 4 | DomainFailureVocabulary | `.../ZeroToANegativePower_IsAnEvaluationFailure#0` | yes, `evalf(subs(0^x, x, -1), 20)` |
| 5 | RelationsAndFamilies | `.../RelationsAndFamiliesTests/Integrate_StructuredResult_ReportsConditions#0` | yes, `integrate_full(1/x, x)` |
| 6 | RelationsAndFamilies | `.../KleeneEvaluation_AndOrNot#0` | yes, the three Kleene scripts |
| 7 | RelationsAndFamilies | `.../LogicalCanonicalization#0` | partially: `not(not(...))`, constants, nested `and` (the constructor calls are library-only) |
| 8 | RelationsAndFamilies | `.../Piecewise_WithCompoundGuard_Evaluates#0` | no: neither runner has a `piecewise` builtin; the library API is the surface |
| 9 | IntegerPow | `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPowTests/Pow_GivenNegativeExponent_Throws#0` | no: the language operator is total (`2^(-1) = 0.5`), `Integer.Pow` is a library call |
| 10 | IntegerPow | `.../Pow_GivenZeroBase_Throws#0` | no, same reason |
| 11 | IntegerPow | `.../Pow_GivenZeroExponent_Throws#0` | no, same reason |
| 12 | ExactnessFlag | `.../ExactnessFlagTests/CachedFlag_EqualsTheLeafRule_OverTheWholeCorpus#0` | no: `Expr.IsExact` is the library API |
| 13 | ExactnessFlag | `.../RadicalsOfExactBases_AreExact_AndApproximateBasesAreNot#0` | language `inspect(x^1.5)` plus the library constructor |
| 14 | ExactnessFlag | `.../SolverSolutionExactness_AndValueExact_Agree#0` | yes, the five `solve_full` scripts |

```text
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DomainFailureVocabularyTests/DivisionByZero_IsAnEvaluationFailure#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DomainFailureVocabularyTests/DivisionByZero_OnTheRealPath_IsAnEvaluationFailure#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DomainFailureVocabularyTests/LogOfZero_IsAnEvaluationFailure#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DomainFailureVocabularyTests/ZeroToANegativePower_IsAnEvaluationFailure#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RelationsAndFamiliesTests/Integrate_StructuredResult_ReportsConditions#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RelationsAndFamiliesTests/KleeneEvaluation_AndOrNot#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RelationsAndFamiliesTests/LogicalCanonicalization#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RelationsAndFamiliesTests/Piecewise_WithCompoundGuard_Evaluates#0
Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPowTests/Pow_GivenNegativeExponent_Throws#0
Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPowTests/Pow_GivenZeroBase_Throws#0
Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPowTests/Pow_GivenZeroExponent_Throws#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ExactnessFlagTests/CachedFlag_EqualsTheLeafRule_OverTheWholeCorpus#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ExactnessFlagTests/RadicalsOfExactBases_AreExact_AndApproximateBasesAreNot#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ExactnessFlagTests/SolverSolutionExactness_AndValueExact_Agree#0
```

## 2. Probes (oracle first)

Each probe ran the documented prelude (`x = symbol("x")`, `y = symbol("y")` where used) as a
`--file ... --omit-functions` script. The full envelopes were compared with the differential's
four volatile keys removed (`revision`, `elapsed`, `elapsedTime`, `timings`); the table records
the field that carried the row.

| Row | Probe | Oracle | Rust before | Rust after |
|---|---|---|---|---|
| 1 | library `evaluate_to_num(Div(1,0))`; language `1/0` | `EvaluationException` with `"Division by zero during numeric evaluation."`; envelope `DivisionByZero/DomainError "Cannot divide by zero."` | identical | identical (test added) |
| 2 | library `evaluate_to_num(Div(1, Real 0.0))`; language `1/0.0` | `EvaluationException` with `"Division by zero during numeric evaluation."`; envelope `DivisionByZero/DomainError "Cannot divide a Real by zero."` | library identical; language envelope `message:"Cannot divide by zero."` | library identical; language envelope `"Cannot divide a Real by zero."` |
| 3 | library `evaluate_to_num(log(0))`; language `evalf(log(0), 20)` | `EvaluationException` / `EvaluationError`, `"Ln(x) requires x > 0. (Parameter 'x')"` | identical | identical (test added) |
| 4 | library `evaluate_to_num(0^-1)`; language `evalf(subs(0^x, x, -1), 20)` | `EvaluationException`; envelope `EvaluationError/DomainError "0 raised to a non-positive power."` | library identical; language envelope `InvalidOperation/DomainError`, same message | library identical; language envelope `EvaluationError/DomainError` |
| 5 | `integrate_full(1/x, x)` | `IntegrationResult(status: SolvedConditional, expression: log(x), conditions: [x > 0], ...)` | `status: SolvedExact`, `conditions: []` | oracle-identical |
| 6 | `evalf(subs(and(x > 0, x < 1), x, 1/2), 20)` / `... x, 2 ...` / `evalf(and(...), 20)` | `1` / `0` / `ok:false EvaluationError "Logical condition cannot be decided numerically."` | `1` / `0` / `InvalidOperation`, same message | `1` / `0` / `EvaluationError` |
| 7 | `inspect(not(not(x > 0)))` | canonical `(gt (sym x) (rat 0 1))`, nodeCount 3 | `(not (not (gt ...)))`, nodeCount 5 | oracle-identical |
| 7 | `inspect(and(x > 0, 1))` | `(gt (sym x) (rat 0 1))`, nodeCount 3 | `(and (rat 1 1) (gt ...))`, nodeCount 5 | oracle-identical |
| 7 | `inspect(and(x > 0, 0))` | `(rat 0 1)`, nodeCount 1 | `(and (rat 0 1) (gt ...))`, nodeCount 5 | oracle-identical |
| 7 | `inspect(or(0, 0))` | `(rat 0 1)`, nodeCount 1 | `(or (rat 0 1) (rat 0 1))`, nodeCount 3 | oracle-identical |
| 7 | `inspect(and(x > 0, and(x < 1, y > 0)))` | flat 3-operand And, nodeCount 10 | nested And operand, nodeCount 11 | flat 3-operand And, nodeCount 10 |
| 8 | library `Expr::piecewise` + `Evaluation.EvaluateToNum` / MathIR lowering | branch 1 at `x = 1/2`, 0 at `x = 2` (C# test) | numeric evaluation already correct; MathIR guard lowering already correct | same, pinned by test |
| 9 | `2^(-1)` (control) | `0.5 (Real)` | identical | identical |
| 10 | `0^3` (control) | `0 (Natural)` | identical | identical |
| 11 | `2^0` (control) | `1 (Natural)` | identical | identical |
| 12 | library `Expr::is_exact` over the C# corpus | the leaf rule | already correct | same, pinned by test |
| 13 | library `Expr::pow(x, Expr::real("1.5"))` | equals `x^(3/2)`; `IsExact` true | `Pow(x, Real("1.5"))`: canonical `(pow (sym x) (real 1.5))`, `is_exact() == false` | canonical `(pow (sym x) (rat 3 2))`, `is_exact() == true` |
| 14 | the five `solve_full` scripts | each `Solution`'s `value.exact` == `(exactness != Approximate)` | identical | identical (test added) |

Notes on rows 9–11: the reference has **two** integer powers with different contracts.
`Integer.Pow(Integer)` (Integer.cs:414) refuses a zero base and a non-positive exponent with
`ArgumentOutOfRangeException`; the language operator is total at its special points (`2^0 = 1`,
`0^3 = 0`, `2^-1 = 0.5`). The three runner controls therefore MATCH on both runners before and
after, and the closed row is the checked library contract, which Rust's `Int::pow(u64)` cannot
express.

## 3. Production fixes

1. **Real zero-divisor vocabulary** — `rust/lovelace-lang/src/builtins.rs:292` adds
   `divide_by_zero_real` and `div` (`:305`) selects it when an operand was Real. The reference's
   `Real.Divide` names its tier ("Cannot divide a Real by zero.", Real.cs:915); the exact
   integer/rational path keeps the generic sentence. `rust/lovelace-lang/src/value.rs:391` makes
   the same distinction on the scalar path.
2. **Zero to a non-positive power is EvaluationError** — `rust/lovelace-lang/src/numeric.rs:524`.
   The reference's `EvaluateToNum` raises `EvaluationException` (Evaluation.cs:472); a plain
   `LangError` crossed as InvalidOperation/DomainError.
3. **Kleene/piecewise Unknown is EvaluationError** — `rust/lovelace-lang/src/numeric.rs:148,167`.
   Same root cause: `EvaluateCondition`'s Unknown is a typed domain refusal
   (Evaluation.cs:496/504), not a defect.
4. **Integration conditions** — `rust/lovelace-lang/src/builtins.rs:1664` (status/conditions) with
   the reference's `LogArguments` walk at `:6632` and the condition projection at `:6663`.
   A log antiderivative is `SolvedConditional` with `f > 0` atoms; no log stays `SolvedExact`.
5. **Logical constructors** — `rust/lovelace-symbolics/src/lib.rs:598` (`Expr::and_all`),
   `:604` (`or_all`), `:612` (`logical_not`), `:623` (`logical_connective`). They flatten a
   nested same-kind node, fold the exact 0/1 constants, collapse a double negation, deduplicate and
   store operands in the canonical order, matching `Exprs.And/Or/Not` (Constructors.cs:646-731).
   The language builtins (`rust/lovelace-lang/src/builtins.rs:1612`) and the unary `not`
   (`rust/lovelace-lang/src/lib.rs:1497`) now build through them.
6. **Numeric exponent normalization** — `rust/lovelace-symbolics/src/lib.rs:481` (`Expr::pow`
   normalizes an exact Real exponent) and `:5424` (`real_text_to_rational`, the Rust spelling of
   `RealLiteral.ToRational`). This is the reference's "every numeric exponent is a RationalConstant"
   rule (Constructors.cs:477), so `x^1.5` and `x^(3/2)` are one node and both exact.
7. **Additive library surfaces for the rows whose API did not exist**:
   `rust/lovelace-lang/src/numeric.rs:637` `integer_pow` (the checked `Integer.Pow(Integer)`
   contract, refusals in the reference's order), and
   `rust/lovelace-lang/src/evaluation_boundary.rs:509,526` make `Tristate` and
   `evaluate_condition` public (the Kleene row's own surface).

The `NumOps`-flavoured rows 1–4 are driven through
`evaluation_boundary::evaluate_to_num`, the additive typed evaluator path that already existed
("the library-level path used by the expected-failure boundary rows"); no new evaluator was
written for them.

## 4. Tests (one per row; the assertions are the oracle's)

New module `rust/lovelace-lang/src/round90_symbolic_families.rs` (12 tests) and
`rust/lovelace-symbolics/src/round90_symbolic_families.rs` (3 tests). No test expectation was
changed to fit the implementation.

| Row | Test | Asserts |
|---|---|---|
| 1 | `division_by_zero_is_an_evaluation_failure` | `EvaluationException` kind + reference message on `Div(1,0)`; envelope code/category/message |
| 2 | `division_by_zero_on_the_real_path_is_an_evaluation_failure` | same kind on `Div(1, Real 0.0)`; envelope message is the Real sentence |
| 3 | `log_of_zero_is_an_evaluation_failure` | `EvaluationException` + `"Ln(x) requires x > 0. (Parameter 'x')"`; envelope |
| 4 | `zero_to_a_negative_power_is_an_evaluation_failure` | `EvaluationException` + `"0 raised to a non-positive power."`; envelope EvaluationError |
| 5 | `integrate_structured_result_reports_conditions` | `1/x` → SolvedConditional, expression `log(x)`, one condition `x > 0` (canonical `(gt (sym x) (rat 0 1))`); `x^5` SolvedExact no conditions; `exp(x^2)` Unevaluated |
| 6 | `kleene_evaluation_and_or_not` | direct `Tristate` True/False/Unknown on the And; 1/0 from `EvaluateToNum`; Unknown is a typed error; Not and Or cases |
| 7 | `logical_canonicalization` (symbolics) + `logical_canonicalization_through_the_engine` (lang) | all C# assertions: double negation, duplicate/constant folding, Or duals, pretty `"x < 1 and x > 0"`, canonical round-trip, 3-operand flatten with nodeCount 10 |
| 8 | `piecewise_with_compound_guard_evaluates` | branch 1 at x=1/2, 0 at x=2 and at the boundary x=1, through numeric evaluation AND the MathIR kernel |
| 9 | `pow_given_negative_exponent_throws` | InvalidArgument/TypeMismatch, `"Exponent must be positive. (Parameter 'exponent')"` |
| 10 | `pow_given_zero_base_throws` | InvalidArgument/TypeMismatch, `"Base cannot be zero. (Parameter 'base')"`; base checked first (`0.Pow(0)` names the base); the three ported controls (2^10, (-2)^4, (-2)^3) still answer |
| 11 | `pow_given_zero_exponent_throws` | InvalidArgument/TypeMismatch, exponent message |
| 12 | `cached_flag_equals_the_leaf_rule_over_the_whole_corpus` | kernel `is_exact` == an independently implemented leaf rule over the C# corpus, both directions, with exact and inexact members present |
| 13 | `radicals_of_exact_bases_are_exact_and_approximate_bases_are_not` | sqrt(2)/sqrt(x)/x^(3/2)/2^(1/3)/i·sqrt(3) exact; approximate base and symbolic approximate exponent inexact; `pow(x, Real(1.5)) == pow(x, 3/2)` and exact; RootOf exact |
| 14 | `solver_solution_exactness_and_value_exact_agree` | every published Solution's `value.is_exact()` == `(exactness != "Approximate")`; Exact published, Approximate not |

## 5. Gates (observed 2026-11-12)

| Step | Command | Observed | Exit |
|---|---|---|---|
| Symbolics suite | `cargo test -p lovelace-symbolics` | 190 passed, 0 failed (3 new) | 0 |
| Language suite | `cargo test -p lovelace-lang` | 1,648 passed, 0 failed (12 new) | 0 |
| Workspace suite | `cargo test --workspace` | `<pending>` | |
| Documentation corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20, rust: 20/20` | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16` | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1` | generated 200/200, replayed 1,758/1,758, 0 mismatch, 0 error | 0 |
| Clippy | `cargo clippy -p lovelace-symbolics -p lovelace-lang --all-targets -- -D warnings` | clean | 0 |
| Format | `cargo fmt -p lovelace-symbolics -p lovelace-lang -- --check` | clean | 0 |

## 6. Scope, adjacent differences, and what remains

Changed files: `rust/lovelace-symbolics/src/lib.rs`, the two new round-90 test modules and their
registrations, and in `rust/lovelace-lang/src`: `builtins.rs`, `numeric.rs`, `value.rs`,
`evaluation_boundary.rs`, `lib.rs`. No `.cs`, fixture, corpus, `parity/`, `rust/lovelace-desktop`
or other document was edited; probe files live under `rust/target/tmp/r90/`.

Adjacent differences seen while probing but **not** part of these 14 rows:

- `integrate_full(1/(x + 1), x)`: the oracle answers `log(x + 1)` with condition `x + 1 > 0`;
  Rust returns Unevaluated (its table has no shifted-denominator entry). This is the pre-existing
  integration-tier gap, unchanged here.
- The language has no `piecewise` builtin on either runner ("Unknown function 'piecewise'"), so
  the piecewise row's surface is the library API; that is the surface the C# row itself uses.

**Rows left blocked: 0.** All 14 ids are closed: rows 5, 6, 7 and 13 have production fixes, rows
2 and 4 have their runner-envelope vocabulary fixed (their library surface was already
equivalent), rows 9–11 gained the checked `Integer.Pow` API, and rows 1, 3, 8, 12 and 14 were
already equivalent at the row's own surface and are now pinned by a test.
