# Round 64 — `Diff` and `Integrate` rows unblocked

**Status: two of the four blocked .NET-Random-corpus rows are unblocked and tested. `Diff_MatchesNumericalDerivative` and `Integrate_DiffRoundTrips` both pass unignored, pinned by one new oracle-behaviour test each. The only production code is in `rust/lovelace-lang/src/numeric.rs` and `rust/lovelace-lang/src/builtins.rs`; no `.cs`, fixture, `parity/` file, existing test expectation or other report was changed.**

Measured against HEAD `6f2b853` and the pinned C# oracle `Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll` (.NET 10.0.3, built 2026-11-11 22:42). The in-process reference probe is `rust/target/tmp/dotnet-random-probe`.

## 1. The two rows and their ids

| Row | Census id | Round-62 blocker | Now |
|---|---|---|---|
| 3 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PropertyTests/Diff_MatchesNumericalDerivative#0` | `numeric::evaluate` has no Piecewise/Relation arm | UNBLOCKED |
| 4 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PropertyTests/Integrate_DiffRoundTrips#0` | `integrate(x^2*sin(x^3), x)` stays unevaluated | UNBLOCKED |

Both C# reference assertions pass in-process (`REFERENCE|Diff_MatchesNumericalDerivative|PASS`, `REFERENCE|Integrate_DiffRoundTrips|PASS`), and the integration probe reports `INTEG|5|MultiplyExpr` for the closed sixth integrand.

## 2. Oracle probes

`--eval` through the harness mangles the quoted script (a script as simple as `symbol("x")` reaches the oracle as an undefined variable `x` at position 0), so every probe below was run from a **`.ls` file with `--file`**; the envelopes are otherwise identical. This is a probing-tool detail, not an oracle defect, and it is recorded here so the next lane does not lose a cycle to it.

### 2.1 Numeric derivative of piecewise and relations

| Script (always `x = symbol("x"); ` first) | Oracle answer |
|---|---|
| `diff(abs(x), x)` | `piecewise(sign(x) if x != 0, diff(abs(x), x))` |
| `evalf(subs(diff(abs(x), x), x, 3/2), 50)` | `1` |
| `evalf(subs(diff(abs(x), x), x, -3/2), 50)` | `-1` |
| `evalf(subs(diff(abs(x), x), x, 1/4), 50)` | `1` |
| `evalf(subs(diff(abs(x), x), x, 2/3), 50)` | `1` |
| `diff(abs(sin(x)), x)` | `cos(x)*piecewise(sign(sin(x)) if sin(x) != 0, diff(abs(sin(x)), x))` |
| `evalf(subs(diff(abs(sin(x)), x), x, 1/3), 50)` | `0.94495694631473766438828400767588060784585269956514` |
| `evalf(subs(diff(abs(x), x), x, 0), 50)` | error: `Node of kind Derivative cannot be evaluated numerically.` (the guard is FALSE, so the otherwise branch is reached) |
| `evalf(1 < 2, 50)` | error: `Node of kind Relation cannot be evaluated numerically.` |
| `evalf(and(1 < 2, 2 < 3), 50)` / `evalf(not(1 < 2), 50)` | `1` / `0` |
| `piecewise(x > 0, x, -x)` | error: `Unknown function 'piecewise'.` (no source-level constructor; the node exists only in the calculus) |

The reference's `Evaluation.EvaluateToNum` has a `PiecewiseExpr` arm (Evaluation.cs:487-499), an And/Or/Not arm returning 1/0 (:500-506), and relation handling only inside `EvaluateCondition` (:537-617); a **bare** relation is deliberately not evaluable.

### 2.2 Integrate, and the reference inconsistency

| Script | Oracle CLI | Meaning |
|---|---|---|
| `integrate(x*cos(x^2), x)` | `1/2*sin(x^2)` | substitution closes |
| `integrate(x*sin(x^2), x)` | `-1/2*cos(x^2)` | substitution closes |
| `integrate(3*x^2*sin(x^3), x)` | `-cos(x^3)` | substitution closes via the `3*x^2` factor pair |
| `integrate(2*x*cos(x^2), x)` | `sin(x^2)` | existing table/chain row |
| `integrate(x^2*sin(x^3), x)` | **`integrate(sin(x^3)*x^2, x)`** | **C# CLI FAILS to close it** |
| `integrate(x^2*cos(x^3), x)` | `integrate(cos(x^3)*x^2, x)` | same C# CLI quirk |
| `integrate(x^2*exp(x^3), x)` | `integrate(exp(x^3)*x^2, x)` | same |
| `integrate(x^2*sin(2*x^3), x)` | `integrate(...)` | same |
| `integrate(x^3*sin(x^4), x)`, `x^4*sin(x^5)` | `integrate(...)` | same |
| `integrate(x^2*log(x^3), x)` | `integrate(...)` | correctly unevaluated (no `log` table-with-arg row) |
| `integrate_full(x*cos(x^2), x)` | SolvedExact, `1/2*sin(x^2)`, `method: substitution`, `verification_method: differentiation+expansion` | tier/method tokens |
| `integrate_full(3*x^2*sin(x^3), x)` | SolvedExact, `-cos(x^3)`, `method: substitution` | tier/method tokens |

**Reference inconsistency (not a Rust bug).** The C# **library** closes the constructed `x^2*sin(x^3)` — the reference test passes and the in-process probe prints `INTEG|5|MultiplyExpr` — while the C# **CLI** leaves the parsed literal unevaluated. The row's oracle is the library assertion (`PropertyTests.Integrate_DiffRoundTrips` builds the expression with `Exprs.Multiply`; it never parses the text), so the rule adopted here, per the parent's direction, is: **match the library, record the CLI divergence as a reference defect**. Rust has one numeric node kind, so `integrate(x^2*sin(x^3), x)` now closes to `-1/3*cos(x^3)`; that is the same class as the earlier C# `zeros` display defect.

Differential impact was checked before changing the code: none of the 200 generated cases (templates only use `integrate(2*x*cos(x^2), x)`, `integrate(x^2, x)`, `integrate(sin(x), x)`, `integrate(x*exp(x), x)`) nor any of the 1,758 stored regressions contains the literal `x^2*sin(x^3)`. The 306 `integrate*` regression files reduce to six unique scripts: `integrate(sin(x), x)`, `integrate(x*exp(x), x)`, `integrate(x^2, x)`, `integrate_full(exp(-x^2), x)`, `integrate_full(sin(x), x)`, `integrate_full(x^2, x)`. No stored case was excluded or edited.

## 3. Production changes

### 3.1 `rust/lovelace-lang/src/numeric.rs` — the missing numeric arms

- `Sym::Fun("piecewise", …)` is read through `Expr::as_piecewise()` and evaluated branch by branch: first guard that is numerically TRUE selects the value, FALSE guards are skipped, all-FALSE falls to the otherwise value, and an UNDECIDABLE guard is refused with the reference's `Piecewise guard cannot be decided numerically.` — never coerced to a branch.
- `evaluate_condition` is the reference's `Evaluation.EvaluateCondition`: relations over three-valued logic (`Eq/Ne/Lt/Le/Gt/Ge`), `and`/`or`/`not` nodes, and the exact constants 0/1. Numeric failures inside a guard become Unknown, exactly as the reference catches `EvaluationException` there.
- `Num::try_compare` mirrors `NumOps.TryCompare`: exact/exact compares rationally, anything involving a complex value is unordered (`None` → Unknown), and an exact/real pair compares at the ambient precision.
- `and`/`or`/`not` at the numeric root answer the exact integers 1/0 (the reference's `EvaluateToNum` arm) and refuse Unknown with `Logical condition cannot be decided numerically.`
- `sign` gained a numeric evaluator (exact ±1/0, real ±1/0, complex z/|z|). It is required by the row: the abs derivative's TRUE branch is `sign(u)*u'`, and without it the guard would be evaluated and then the slope refused. This mirrors `NumOps.Sign` / the C# `sign` registration.
- **A bare `Sym::Rel` is still refused** by `evaluate` with the port's existing message. The reference refuses it too (`evalf(1 < 2, 50)` errors), so relations are readable only as conditions — adding a numeric value for a bare relation would have diverged from the oracle.

### 3.2 `rust/lovelace-lang/src/builtins.rs` — the by-substitution tier

`integrate_by_substitution` is a faithful port of `Calculus/Integrate.cs:398-444` (`TrySubstitution`):

- candidates are every single factor and every factor pair as the chain-rule derivative `gp`, with the remaining factors as `Wrap(rest)`;
- `MatchDerivative` returns scale 1 when `gp == g'`, else `1/c` when `SplitTerm(g') = (c, gp)`;
- pattern A: `rest = g^n` (n ≠ −1) → `scale * g^(n+1)/(n+1)`;
- pattern B: `rest = f(g)` for the table functions `sin, cos, exp, sinh, cosh` → `scale * F(g)` (F = −cos, sin, exp, cosh, sinh);
- the tier runs in `integrate_basic` after the coefficient/table rows and before integration by parts, matching the reference tier order, and is reachable through both `integrate` and `integrate_full`;
- `integrate_full` now reports `method: substitution` when that tier is what closes the integrand (previously every closed form said `table`), matching the oracle's `method` token for these cases. Existing `table` expectations (`integrate_full(x^2, x)` etc.) are unchanged because the substitution probe returns None for them.

Line anchors (current tree): `numeric.rs` Piecewise arm at :78-97, `evaluate_condition` at :130, `try_compare` at :202, `sign` evaluator at :726; `builtins.rs` `split_numeric_coefficient` at :6402, `match_integration_derivative` at :6416, `integrate_table_with_arg` at :6429, `integrate_by_substitution` at :6447, call site in `integrate_basic` at :6596.

## 4. Tests

| Test | Change |
|---|---|
| `property_assertions::diff_matches_numerical_derivative` | `#[ignore]` removed; doc updated to oracle evidence. Runs the row's own 60-expression × 3-point assertion. |
| `property_assertions::integrate_diff_round_trips` | `#[ignore]` removed; doc updated. Runs the row's six integrands and still requires six closed forms plus the derivative round trip. |
| `property_assertions::numeric_piecewise_relation_arms_match_the_oracle` | NEW. Pins the CLI-observed values: d/dx abs(x) = 1 at 3/2, 1/4, 2/3 and −1 at −3/2; TRUE/FALSE branch selection on a directly built piecewise; the undecidable-guard message; and the bare-relation refusal. |
| `property_assertions::integrate_by_substitution_matches_the_reference` | NEW. Pins `integrate(x^2*sin(x^3), x) = -1/3*cos(x^3)` (library oracle), the CLI-observed neighbours `x*cos(x^2) -> 1/2*sin(x^2)` and `3*x^2*sin(x^3) -> -cos(x^3)`, each with the differentiated round trip, plus `integrate_full(...).method = substitution`. |

`cargo test -p lovelace-corpus property_assertions`: **8 passed, 0 failed, 1 ignored** (the remaining ignored row is MathIR, a different lane). The other lane's canonical row also runs green here; its edits to this file and `property_corpus.rs` were preserved (canonical test unignored; canonical hashes 6,083,718,521,819,931,755 for CanonicalText 200/200 and 6,946,605,504,391,048,442 for MathIR 60/60).

## 5. Rust before/after probes

Before = `rust/target/release/lovelace-run.exe` built 22:33, before this change. After = debug build of the changed tree. Same `--file` scripts.

| Probe | Rust before | Rust after | Oracle |
|---|---|---|---|
| `evalf(subs(diff(abs(x), x), x, 3/2), 50)` | error `A relation has no numeric value; evaluate a number or an expression.` | `1` | `1` |
| `evalf(subs(diff(abs(x), x), x, -3/2), 50)` | same error | `-1` | `-1` |
| `integrate(x^2*sin(x^3), x)` | `integrate(sin(x^3)*x^2, x)` | `-1/3*cos(x^3)` | CLI: unevaluated; **library: closed form** |
| `integrate(x*cos(x^2), x)` | `integrate(x*cos(x^2), x)` | `1/2*sin(x^2)` | `1/2*sin(x^2)` |
| `integrate(3*x^2*sin(x^3), x)` | `integrate(3*sin(x^3)*x^2, x)` | `-cos(x^3)` | `-cos(x^3)` |
| `integrate(x^2*cos(x^3), x)` | unevaluated | `1/3*sin(x^3)` | CLI unevaluated; library closes |
| `integrate(x^2*exp(x^3), x)` | unevaluated | `1/3*exp(x^3)` | CLI unevaluated; library closes |
| `integrate(x^2*sin(2*x^3), x)` | unevaluated | `-1/6*cos(2*x^3)` | CLI unevaluated; library closes |
| `integrate(x^2*log(x^3), x)` | unevaluated | unevaluated | unevaluated |
| `integrate_full(x*cos(x^2), x)` | Unevaluated | SolvedExact, method `substitution` | SolvedExact, method `substitution` |
| `evalf(and(1 < 2, 2 < 3), 50)` | error `Logical operators need expressions.` (the **builtin** is reached before evalf; pre-existing language-surface difference) | unchanged | `1` |

The workspace total moved from the round-62 baseline (5,074 passed / 4 ignored) to **5,083 passed / 1 ignored**: the three unignored rows (this lane's Diff and Integrate, the other lane's CanonicalText), the two new tests here, and four tests added by the canonical-order lane. Only `mathir_evaluates_like_symbolic` remains ignored in the whole workspace.

The `evalf(and(...))` and `evalf(1 < 2, 50)` surface differences are pre-existing and outside these rows: Rust's `and` builtin evaluates its operands to Boolean values and refuses them before the numeric tier sees a logical node, and `evalf` refuses the Boolean that `1 < 2` already produced ("evalf needs a number"). The numeric-tier arms added here are exercised through piecewise guards and the `diff(abs(u), x)` slope, which is what the rows need.

## 6. Gate outputs (observed this round)

| Gate | Command | Observed | Exit |
|---|---|---|---|
| Full suite | `cargo test --workspace` | 5,083 passed, 0 failed, 1 ignored (21 test binaries; only MathIR remains ignored) | 0 |
| Corpus crate | `cargo test -p lovelace-corpus` | 18 passed, 0 failed, 1 ignored | 0 |
| Documentation corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | oracle 20/20, rust 20/20 | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16` | 0 |
| Differential (generated) | `node parity/diff/run.mjs --cases 200` | generated 200/200 match, replayed 1758/1758 match, 0 mismatch, 0 error; harness exit 0 | 0 |
| Differential (replay) | `node parity/diff/run.mjs --regressions` | replayed 1873/1873 (the combined run skips 115 duplicates); harness exit 0 | 0 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | Finished, no warnings | 0 |
| Format | `cargo fmt --all -- --check` | no diff | 0 |

## 7. Residual

- Rows 1 (CanonicalText) and 7 (MathIR) remain other lanes' blockers; this lane does not touch them. With rows 3 and 4 unignored and row 1 unignored by the other lane, the remaining ignored blocker probe is MathIR.
- The C# CLI's substitution quirk (parsed integer-exponent powers in the single-factor candidate do not match the derivative's power node, while the factor-pair path does) is a reference defect. It should be added to the known-deviation list with the probe pair in §2.2; Rust must keep closing the library-correct form.
- Pre-existing surface differences listed at the end of §5 (logical operands / bare relation through `evalf`) are not regressions and are not fixed here.

## 8. Reproduction

    $repo = 'C:\Users\ricar\dev\LovelaceSharp'
    $env:RUSTUP_HOME = "$repo\.toolchain\rustup"
    $env:CARGO_HOME  = "$repo\.toolchain\cargo"
    $env:PATH = "$repo\.toolchain\cargo\bin;" + $env:PATH
    $env:TMP = "$repo\rust\target\tmp"; $env:TEMP = $env:TMP
    Set-Location "$repo\rust"
    cargo test -p lovelace-corpus property_assertions
    cargo test --workspace
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all -- --check
    cargo run --release -p lovelace-corpus
    Set-Location $repo
    node parity/tools/gate-fixtures.mjs
    node parity/tools/gate-cli.mjs
    node parity/diff/run.mjs --cases 200
    node parity/diff/run.mjs --regressions

Oracle probes (write the script to a `.ls` file; do not pass it through harness `--eval`):

    dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --file probe.ls --omit-functions --omit-variables

In-process reference probe:

    rust/target/tmp/dotnet-random-probe/bin/Release/net10.0/DotNetRandomProbe.exe
