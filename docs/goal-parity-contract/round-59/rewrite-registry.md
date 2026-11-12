# Round 59 — the rewrite-registry block: nine public rules and seven mirrors

**Status: the seven `RewriteProtocolTests` census rows in the round-50
"rewrite rule registry with preconditions and trace API" block are unblocked. The additive Rust
surface is in `rust/lovelace-symbolics/src/rewrite_registry.rs`; every gate is green. Per this
lane's scope, no `.cs` file, fixture, `parity/` file or other document was touched, the manifest
entries are still `deferred`, and no existing simplification behaviour changed.

Measured 2026-11-11 (America/Sao_Paulo) on the working tree at `e7f1bda` plus the concurrent
lovelace-run lane's uncommitted `script_source.rs`.

## 1. What this round changed

* **New:** `rust/lovelace-symbolics/src/rewrite_registry.rs` — the reference's rewrite-rule
  registry as a public additive surface: `Pattern`, `Match`, `RuleApplicability`,
  `RuleClassification`, `RuleEvaluationException`, `RewriteRule`,
  `RewriteRuleRegistry`, `Budget`, `RewriteStep`, `RuleAttemptDiagnostic`,
  `ApplySink`, `RewriteEngine`, the nine shipped rules (`shipped_registry`),
  `shipped_rule_ids`, and the reference `Simplify.Transform` protocol over that registry
  (`transform`, `TransformOptions`, `TransformResult`, `TransformStatus`).
* **Edited:** `rust/lovelace-symbolics/src/lib.rs` — one additive `pub mod rewrite_registry;` line.
  No existing item was changed and no existing caller was touched.
* **Untouched:** every `.cs` file, fixture, corpus file, `parity/` file and other document; the
  language-layer simplifier (`builtins::simplify` / `simplify_traced`), the runner envelope, the
  registry's existing five step-provenance ids, and every other symbolics behaviour. The registry is
  not wired into the language engine; the tests are its only callers today.
* **Why the engine lives here:** the dependency direction is `lovelace-lang` -> `lovelace-symbolics`,
  not the reverse, so the registry cannot call the language simplifier. The reference application
  protocol is ported alongside the registry (the language engine stays the only production path),
  which is what makes ordered application, budget and trace observable without changing existing
  simplification behaviour.

## 2. What the C# source and tests observe

Sources read: `Lovelace.Symbolics/Rewriting.cs` (the protocol and engine),
`Lovelace.Symbolics/Simplify.cs` (the registry and options) and
`Lovelace.Symbolics.Tests/RewriteProtocolTests.cs` (the seven methods behind the seven rows).

### 2.1 Rule identity and registration order

The registry ships **nine** rules in five groups. Registration order is application order inside a
group; the phase order is `trig`, `power`, `rat`, `logexp`, `abs`.

| # | Rule id | Group | Classification | Pattern shape | Applicability | Conditions |
|---:|---|---|---|---|---|---|
| 1 | `trig.pythagorean-sin2-cos2` | trig | Universal | `Add[cos(x)^2, sin(x)^2]` | bound arguments equal | none |
| 2 | `pow.sqrt-square-nonnegative` | power | Conditional | `Pow[Pow(w, 2), 1/2]` | `w` is a symbol and `NonNegative(w)` is proven | `NonNegative(w)` |
| 3 | `pow.sqrt-square-real` | power | DomainSpecific | `Pow[Pow(w2, 2), 1/2]` | `w2` is a symbol, `NonNegative(w2)` is not proven, `Real(w2)` is proven | `Real(w2)` |
| 4 | `rat.cancel-x-over-x` | rat | Conditional | `Mul[cu, Pow(cv, -1)]` | `cu == cv` | `NonZero(cu)` |
| 5 | `rat.cancel-zero-over-x` | rat | Conditional | `Mul[0, Pow(cv2, -1)]` | always | `NonZero(cv2)` |
| 6 | `logexp.exp-log` | logexp | Conditional | `exp(log(lx))` | always | `Finite(log(lx))` |
| 7 | `logexp.log-exp` | logexp | Conditional | `log(exp(lx2))` | `im(lx2) in (-pi, pi)` is proven | none declared |
| 8 | `abs.abs-square` | abs | DomainSpecific | `Pow[abs(aw), 2]` | `aw` is a symbol and `Real(aw)` is proven | `Real(aw)` |
| 9 | `abs.abs-neg` | abs | Universal | `abs(-1 * aw2)` | always | none |

Two rules are Universal; the rest default to Conditional except the two DomainSpecific real-domain
rules. No shipped rule is Approximate or OptimizationOnly (the enums are complete anyway).

### 2.2 Patterns

`Pattern` is the reference `Pattern` hierarchy: literal structural equality, a single-node
wildcard (first binding wins), a sequence wildcard matching 0..n contiguous siblings of an Add, Mul
or function-argument list (at most one per list, leftmost-greedy split order), an n-ary Add/Mul
pattern, a power pattern, and a function-name/argument pattern. `Match` exposes single bindings and
sequence bindings, and a match with an unbound name fails rather than guessing.

### 2.3 Applicability is a value; only one exceptional state is caught

`RuleApplicability` has four values: `NotApplicable`, `Applicable`,
`ApplicableWithConditions` and `Unknown`. A pattern/precondition mismatch is a value, never
exception-based control flow. The engine fires on `Applicable` and `ApplicableWithConditions`;
`NotApplicable` and `Unknown` never license a rewrite in any mode. A probe may return the
declared-undecidable `RuleEvaluationException`; the engine catches exactly that payload, records
`Unknown` on the attempt diagnostic, and keeps walking. Every other error is a defect and
propagates (the C# `BoomException` case).

### 2.4 Application order, budget and provenance

The engine rebuilds children bottom-up in canonical order, then tries the rules at the rebuilt node
in registration order. The first applicable rule fires; the node is re-examined until no rule fires
or the budget is exhausted. The budget counts **applied rewrites**, not matching attempts, and is
identified as `rewrite_steps`. A trace is allocated only when requested; the condition builder is
consulted even without a trace because the conditions are semantics, not provenance. Diagnostics
are per-attempt and allocated only when requested. `transform` stops the phase loop as soon as
the budget is exhausted, so `MaxSteps: 0` applies nothing and reports `BudgetExceeded` with
`rewrite_steps` and no steps.

### 2.5 The effects this round mirrors

* `x/x` (a product `x * x^-1`) becomes `1` with one `SymbolProperty(x, NonZero)`
  condition and the `rat.cancel-x-over-x` step; trace disabled still yields the condition and
  allocates no steps.
* `sqrt(x^2)` (nested `Pow[Pow(x,2),1/2]`) becomes `abs(x)` under `x in Real`, with
  one `pow.sqrt-square-real` step classified DomainSpecific and a non-empty condition set.
  The nonnegative rule is tried first and does not fire; the real rule does.
* `exp(log(x))` becomes `x` carrying `Finite(log(x))`; `log(exp(x))` becomes `x`
  only when the principal-strip membership is proven.
* `sin(x)^2 + cos(x)^2` becomes `1`; `abs(-x)` becomes `abs(x)`.
* Applying one named rule directly is observable through `RewriteRuleRegistry::get` +
  `RewriteEngine::apply(_with)`; this is how the named-effect test drives the cancellation and
  absolute-value rules.

Deliberate representation differences (none of the seven rows observes them):

* C# uses exceptions; Rust returns `Result<RuleApplicability, RuleError>` from a probe, with
  `RuleError = Box<dyn Error + Send + Sync>`. The engine's catch set is exactly
  `RuleEvaluationException`, preserving the C# protocol rather than widening it.
* C# `ExprContext` carries interned symbols/functions and the assumption set; this crate's
  rules take the `AssumptionSet` directly. The seven rows set only assumptions, so no interning
  context is observed.
* C# has no explicit division node (a quotient is a negative power). Rust has `Expr::Div`; the
  engine preserves an explicit division node and still matches the reference product shapes.
* `transform` is the full (unsafe) reference protocol. The C# `SimplifyExpr` safe wrapper
  (`SafeWrap`) is already embodied by the language simplifier; it is not duplicated here and none
  of the seven rows observes it.

## 3. Member map

| C# member (`Rewriting.cs` / `Simplify.cs`) | Rust member | Observable |
|---|---|---|
| `RuleClassification` (Universal, Conditional, DomainSpecific, Approximate, OptimizationOnly) | `rewrite_registry::RuleClassification` + `name()` | kernel spelling; default is Conditional |
| `RuleApplicability` (NotApplicable, Applicable, ApplicableWithConditions, Unknown) | `RuleApplicability` + `name()` | the applicability value, never an error |
| `RuleEvaluationException` | `RuleEvaluationException` (Display + Error) | the one caught probe state |
| `RuleAttemptDiagnostic` (RuleId, PatternMatched, Precondition, RejectionReason) | `RuleAttemptDiagnostic` | one per attempted rule when diagnostics are requested |
| `RewriteStep` (RuleId, Classification, Before, After, Conditions) | `RewriteStep` (+ Display) | one per applied rewrite when tracing |
| `Pattern`: LiteralPat, WildPat, SeqWildPat, AddPat, MulPat, PowPat, FunPat | `Pattern`: Literal, Wild, SequenceWild, Add, Mul, Pow, Fun | structural shape matching |
| `Match`: Get, GetSeq, TryGet | `Match`: `get`, `get_seq`, `try_get` | bindings produced by a match |
| `RewriteRule` (Id, Group, Pattern, Applicability, Replacement, Classification, DeclaredConditions, ConditionBuilder) | `RewriteRule` + `new` + `with_classification` / `with_declared_conditions` / `with_condition_builder` | one rule; classification defaults to Conditional |
| `RewriteRule.Applicability` / `Replacement` | `applicability()` / `replacement()` | the predicate and the replacement builder |
| `RewriteRule.ConditionBuilder ?? DeclaredConditions` | `conditions_for()` / `has_condition_builder()` | the conditions one application requires |
| `RewriteRuleRegistry.Register` / `Group` / `All` | `RewriteRuleRegistry::register` / `group` / `all` (+ additive `get`) | registration order, group order, named lookup |
| `RewriteEngine.Budget` (MaxSteps, Steps, Exhausted, Kind) | `Budget` (`max_steps`, `steps`, `exhausted()`, `kind`) | counts applied rewrites; `rewrite_steps` |
| `RewriteEngine.Apply` overloads | `RewriteEngine::apply` / `apply_with` + `ApplySink` | ordered application; optional trace/conditions/diagnostics |
| `RewriteEngine.TryMatch` | `RewriteEngine::try_match` | one pattern against one expression |
| `Simplify.RulesForTesting(ctx)` | `shipped_registry().all()` | the nine rules, in registration order |
| `Simplify.ShippedRuleIds(ctx)` | `shipped_rule_ids()` | the nine ids, in registration order |
| `Simplify.Options` (Trace, MaxSteps, Diagnose) | `TransformOptions` | trace flag, step budget, diagnostics flag |
| `Simplify.Transform` | `transform(&Expr, &AssumptionSet, TransformOptions)` | expression + conditions + steps + budget + attempts |
| `TransformResult` (Expression, Conditions, Steps, BudgetExceeded, Status, BudgetKind) | `TransformResult` + `changed()` / `status()` | the transformed expression and its side conditions |
| `TransformStatus` (Satisfied, BudgetExceeded, Unsatisfiable) | `TransformStatus` + `name()` | structured disposition |
| phase order `trig, power, rat, logexp, abs` | the fixed group loop inside `transform` | groups run in that order, budget shared |

Not ported from the same file: the C# `ExprContext` interning/factory surface (not observed by these
seven rows) and the safe-mode `SafeWrap` wrapper (the language simplifier already implements safe
mode). The `RewriteProofs` proof-obligation bridge is the separate two-row block and was not
touched.

## 4. The seven census ids and their Rust tests

All seven are currently `deferred` in `parity/manifest-map.json`; this lane was scoped not to edit
`parity/`. The proposed disposition for every row is `ported`. The manifest `rust` id is the
measured inventory id `lovelace-symbolics::rewrite_registry::tests::<name>`.

| # | Census id | Rust test | What the mirror asserts |
|---:|---|---|---|
| 1 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RewriteProtocolTests/Budget_CountsAppliedRewrites_AndStopsFurtherGroups#0` | `budget_counts_applied_rewrites_and_stops_further_groups` | `transform(x/x, MaxSteps: 0)` is `BudgetExceeded`, kind `rewrite_steps`, no steps, unchanged expression |
| 2 | `…/DomainSpecificRule_DeclaresItsCondition#0` | `domain_specific_rule_declares_its_condition` | `x in Real`; `sqrt(x^2)` -> `abs(x)`; one `pow.sqrt-square-real` step, DomainSpecific, non-empty conditions |
| 3 | `…/EveryRule_HasAStableUniqueId_AndUniversalRulesAreUnconditional#0` | `every_rule_has_a_stable_unique_id_and_universal_rules_are_unconditional` | nine distinct ids in registration order; Universal rules have no declared conditions and no condition builder |
| 4 | `…/RuleEvaluationException_IsTheOnlyCaughtState#0` | `rule_evaluation_exception_is_the_only_caught_state` | the declared-undecidable probe is caught; expression unchanged; the matched attempt is `Unknown` with the probe message |
| 5 | `…/ThrowingPrecondition_Propagates#0` | `throwing_precondition_propagates` | a non-`RuleEvaluationException` defect propagates out of `apply`, never becomes `NotApplicable` |
| 6 | `…/TraceDisabled_KeepsConditionsAndAllocatesNoSteps#0` | `trace_disabled_keeps_conditions_and_allocates_no_steps` | `x/x` -> `1`; empty steps; non-empty `x != 0` conditions |
| 7 | `…/UnknownApplicability_NeverFires#0` | `unknown_applicability_never_fires` | a rule that answers `Unknown` leaves the node unchanged |

Supporting tests added with the seven mirrors:

| Rust test | Coverage |
|---|---|
| `registry_lists_the_nine_reference_rules_in_registration_order` | group sizes 1/2/2/2/2, power order, `shipped_rule_ids` agreement, named lookup/refusal |
| `named_rule_application_rewrites_with_its_condition` | applying `rat.cancel-x-over-x` alone yields `1` + `NonZero(x)`; `abs.abs-neg` alone yields `abs(x)` |
| `shipped_patterns_match_the_reference_shapes_and_first_match_wins` | all nine patterns match their concrete reference shape; two applicable rules on one node resolve by registration order |

## 5. Gate outputs

Commands ran with the repository toolchain environment (`RUSTUP_HOME`, `CARGO_HOME`, `PATH`, `TMP`,
`TEMP`); Node gates ran from the repository root. The release runner was rebuilt before the
fixture and CLI gates.

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| New module | `cargo test -p lovelace-symbolics rewrite_registry` | `10 passed; 0 failed; 128 filtered out` | 0 |
| Symbolics crate | `cargo test -p lovelace-symbolics -- --quiet` | `138 passed; 0 failed` (baseline 128 + 10 here) | 0 |
| Workspace suite | `cargo test --workspace -- --quiet` | `5,054 passed; 0 failed`, 19 suites | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean, no warnings | 0 |
| Release runner | `cargo build --release -p lovelace-run` | Finished release profile | 0 |
| Documentation corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS` | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS` | 0 |

The workspace total is 5,054 = the 5,029 committed baseline + the 10 tests added here + 15 tests
present in the concurrent lovelace-run lane's uncommitted `script_source.rs`. The 15 are not
this lane's change and no existing test expectation was edited. `cargo test --workspace -- --list`
counts 5,054 test entries across 19 suites.

## 6. What remains

* **Manifest mapping.** The seven rows are still `deferred` because this lane cannot edit
  `parity/`. A follow-up lane with parity scope should mark exactly the seven ids `ported` to
  the tests in section 4, regenerate `parity/manifest.toml`, `parity/manifest-map.json`,
  `parity/coverage.md` and `parity/rust-tests.json`, and run the manifest gate. The Rust inventory
  is stale relative to the tree (10 new tests) until that regeneration runs.
* **The two proof-bridge rows are separate.** `RewriteProofBridgeTests/EveryObligation_IsStatedAndHonestAboutItsStatus#0`
  and `…/EveryShippedRule_HasExactlyOneProofObligation#0` need the proof-obligation bridge, not just the rule
  list. `shipped_rule_ids` supplies the identity half that bridge will consume; the obligation records
  and their status mapping are still absent, so those two rows remain deferred.
* **No runtime wiring.** The language engine does not consume the new registry; it is an additive
  library surface and its tests are the only callers. Existing `simplify` / `simplify_full` behaviour,
  the runner envelope, the corpus, fixtures and CLI are unchanged.
* **Safe mode is not duplicated.** The C# `SimplifyExpr` safe wrapper is already the language
  simplifier's policy; the new `transform` is the full `Transform` protocol and no row here observes
  safe-mode licensing.
