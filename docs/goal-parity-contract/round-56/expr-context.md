# Round 56 — the thread-scoped ExprContext block: eight mirrors and the additive context that decides them

**Status: the additive Rust context surface and all eight mirrors are in place; every gate is
green.** The eight census rows the round-50 re-sweep grouped under "thread-scoped
ExprContext/assumption scopes" now have direct Rust tests over a new, purely additive
`lovelace_lang::context` module. Per this lane's scope, no `.cs` file, fixture, `parity/` file or
other document was touched; the `parity/manifest-map.json` entries are still `deferred` and the
mapping proposal is section 4.

Measured 2026-11-11 (America/Sao_Paulo) on the current working tree.

## 1. What this round changed

* **New:** `rust/lovelace-lang/src/context.rs` — `Context`, `Symbol`, `FunctionId`,
  `AssumptionScope`, the two race-free identity tables and the thread-local scope stack.
* **New:** `rust/lovelace-lang/src/context_round56.rs` — the eight C# mirrors (section 4) plus
  three supporting tests (panic unwind, cross-thread invisibility, base-setter precedence).
* **Edited:** `rust/lovelace-lang/src/lib.rs` — three additive lines: `pub mod context;`,
  `mod context_round56;`, and the crate-private `Engine::set_assumptions` used by
  `Context::simplify`. No existing path changed.
* **Untouched:** every `.cs` file, fixture, corpus file, `parity/` file, `docs/` file other than
  this report, the runner envelope and the corpus/CLI wire behavior. The new context is not used
  by the engine, so the runtime contract is byte-for-byte unchanged.

## 2. What the C# source and tests observe

Sources read: `Lovelace.Symbolics/Context.cs` (the `ExprContext` class),
`Lovelace.Symbolics.Tests/AssumptionScopeIsolationTests.cs` (two rows) and
`Lovelace.Symbolics.Tests/ConcurrencyTests.cs` (six rows). The observed contract is:

1. **Base vs scoped value.** `ExprContext.Assumptions` returns the `AsyncLocal` scope value when
   a scope is active in the current flow, otherwise the context's base set. A scope is never a
   mutation of the base set.
2. **Push/pop.** `WithAssumptions(set)` captures the previous scope value (including `null` for
   "none"), installs `set`, and returns an `IDisposable`. `Dispose` restores the captured value
   and is idempotent (the second Dispose does nothing).
3. **Nesting.** An inner scope sees the inner set; after the inner Dispose the enclosing set is
   visible again; after the outermost Dispose the base set is visible.
4. **Concurrent isolation.** Two scopes open at the same moment in two flows each see their own
   set. Disposing one scope cannot restore a stale set into the other. The base set is untouched
   after both end. `AssumptionScopeIsActive` reports scope presence for the current flow.
5. **Setter precedence.** Assigning `ctx.Assumptions = set` updates the base set, but an active
   flow-local scope still overrides it until that scope ends.
6. **Identity tables.** `Symbol(name)` and `Function(name)` allocate a dense id under one lock;
   concurrent creation of 8 × 250 names produces 2,000 distinct ids; `Symbol(id)` /
   `Function(id)` round-trip the name; equality and ordering are by canonical name, not id.
7. **Shared-context kernels.** Parsing canonical text, simplifying, differentiating and lowering
   MathIR on one shared context agree with the serial results when run in parallel
   (`ParallelParse_AgreesWithSerial`, `ParallelSimplifyDiff_AgreeWithSerial`,
   `ParallelCompilation_AgreesWithSerial`). The context supplies the identity tables and, for
   simplification, the active assumptions.
8. **Unwind restoration.** C# uses `using`, so `Dispose` runs while an exception unwinds. The
   equivalent Rust requirement is that the RAII guard's `Drop` restores the enclosing set during
   `panic!` as well as ordinary block exit. No C# row spells this out; it is required by this
   task and pinned by a Rust test.

Deliberate representation differences (none of the eight rows observes them):

* C# uses `AsyncLocal<T>` (a scope follows an async flow across `await`); Rust has no async flow
  in this module, so the scope is **thread-local**. The observable guarantee is the same for
  threads: a scope is visible only on the thread that opened it, and each thread has its own
  stack. The guard is deliberately `!Send`, so it cannot be dropped on another thread.
* `ExprContext()` registers the core function registry; the Rust language registry is global and
  no observed member reads a per-context registry, so `Context::new()` starts empty.
* C# `Intern` hash-conses symbolic nodes on the context; the Rust symbolic expression is a value
  with no per-context pool, and none of the eight rows observes node identity.
* Dropping a guard removes exactly the layer it installed (by scope id), so out-of-order drops
  leave the remaining stack intact instead of restoring a stale value. The C# tests only exercise
  LIFO nesting.

## 3. Member map

| C# member | Rust member | Observable |
|---|---|---|
| `new ExprContext()` | `Context::new()` / `Default` | empty identity tables and empty base set |
| `Assumptions` getter | `Context::assumptions()` | innermost scope on this thread, else base set |
| `Assumptions` setter | `Context::set_assumptions(set)` | replaces the base set; active scope still wins |
| `WithAssumptions(set)` | `Context::with_assumptions(set)` | pushes a scope, returns an RAII guard |
| `AssumptionScope : IDisposable` | `AssumptionScope<'_>` with `Drop` | pops/restores on block exit and on panic unwind |
| `AssumptionScopeIsActive` (internal) | `Context::assumption_scope_is_active()` | true only while this thread has a scope |
| `Symbol(string)` | `Context::symbol(name)` | dense id, stable by name, race-free |
| `Symbol(int)` | `Context::symbol_by_id(id)` | `Option<Symbol>`; name round-trip |
| `Function(string)` | `Context::function(name)` | dense id, stable by name, race-free |
| `Function(int)` | `Context::function_by_id(id)` | `Option<FunctionId>`; name round-trip |
| `FunctionRegistry Functions` | not ported | global Rust registry; no row observes it |
| `Printing.CanonicalParse(text, ctx)` | `Context::canonical_parse(text)` | parses the typed S-expression form and interns every symbol/function it mentions |
| `Simplify.SimplifyExpr(e, ctx)` | `Context::simplify(expr)` | simplifies under the thread's active scope |
| `Calculus.Diff(e, x, ctx)` | `Context::diff(expr, "x")` | differentiation over the shared identity tables |
| `Lowering.Lower(e, ctx, params)` | `Context::lower(expr, params)` | MathIR lowering, same serial/parallel agreement |

`Symbol` and `FunctionId` also expose `id()`, `name()`, `Display`, name-based `Eq`/`Hash` and
name-based `Ord`, matching the C# structs' name equality and ordinal ordering.

## 4. The eight census ids and their Rust tests

All eight are currently `deferred` in `parity/manifest-map.json`; this lane was scoped not to edit
`parity/`. The proposed disposition for every row is `ported` to the named test.

| # | Census id | Rust test | What the mirror asserts |
|---:|---|---|---|
| 1 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.AssumptionScopeIsolationTests/NestedScopes_RestoreTheEnclosingScopeOnDispose#0` | `context_round56::nested_scopes_restore_the_enclosing_scope_on_dispose` | outer `x > 0` visible, inner `x < 0` visible and refutes the outer query, outer visible after inner drop, base empty after outer drop |
| 2 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.AssumptionScopeIsolationTests/TwoConcurrentScopes_AreIsolated#0` | `context_round56::two_concurrent_scopes_are_isolated` | two threads enter scopes behind a `Barrier`; positive flow answers `True`, negative flow answers `False`, base empty afterwards |
| 3 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ConcurrencyTests/AssumptionScopes_RestoreOnDispose#0` | `context_round56::assumption_scopes_restore_on_dispose` | worker-thread `x in Real` scope simplifies `sqrt(x^2)` to `abs(x)`, scope drop restores the baseline `sqrt(x^2)`, atoms empty |
| 4 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ConcurrencyTests/ParallelCompilation_AgreesWithSerial#0` | `context_round56::parallel_compilation_agrees_with_serial` | one polynomial lowered on 16 threads; every serialized program equals the serial one |
| 5 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ConcurrencyTests/ParallelFunctionCreation_NoDuplicateIds#0` | `context_round56::parallel_function_creation_no_duplicate_ids` | 8 × 250 names give 2,000 unique ids; every id resolves back to its name |
| 6 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ConcurrencyTests/ParallelParse_AgreesWithSerial#0` | `context_round56::parallel_parse_agrees_with_serial` | 40 canonical texts parse to the same canonical results serially and in parallel; `x` is interned |
| 7 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ConcurrencyTests/ParallelSimplifyDiff_AgreeWithSerial#0` | `context_round56::parallel_simplify_diff_agree_with_serial` | 40 `sin(x^2)^k` expressions; canonical diff and simplify renderings agree serially and in parallel |
| 8 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ConcurrencyTests/ParallelSymbolCreation_NoDuplicateIds#0` | `context_round56::parallel_symbol_creation_no_duplicate_ids` | 8 × 250 names give 2,000 unique ids; id/name round-trip; equality by name and repeated interning is stable |

Supporting tests, not tied to a census row but required by the task's observations:

| Rust test | Why it exists |
|---|---|
| `context_round56::a_scope_restores_the_base_set_when_the_thread_unwinds` | required unwind/panic restoration; `catch_unwind` panics inside the guard and the base set is empty afterwards |
| `context_round56::a_scope_is_invisible_to_another_thread` | direct cross-thread independence: an observer thread reads the base set while the opener is inside its scope |
| `context_round56::the_base_setter_does_not_override_an_active_scope` | the C# setter/scope precedence rule, then the new base set after the scope ends |

Census ids expected to move from `deferred` to `ported`: exactly the eight rows above. No other
census id is touched.

## 5. Gate outputs

Commands ran with the repository toolchain environment (`RUSTUP_HOME`, `CARGO_HOME`, `PATH`,
`TMP`); Node gates ran from the repository root. The release runner was rebuilt before the
fixture and CLI gates.

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| New mirrors | `cargo test -p lovelace-lang context_round56` | `11 passed; 0 failed; 1504 filtered out` | 0 |
| Workspace suite | `cargo test --workspace -- --quiet` | 19 suites, `PASSED=5011`, `FAILED=0` (baseline 5,000 + 11 new) | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean, no warnings | 0 |
| Release runner | `cargo build --release -p lovelace-run` | Finished release profile (optimized) | 0 |
| Documentation corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS` | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS` | 0 |

The workspace total includes the 11 new tests; the previous baseline was 5,000. No existing
test expectation was edited.

## 6. What remains

* **Manifest mapping.** The eight rows are still `deferred` because this lane cannot edit
  `parity/`. A follow-up lane with parity scope should mark exactly the eight ids `ported` to the
  tests in section 4, regenerate `parity/manifest.toml` and `parity/coverage.md`, and run
  `node parity/tools/gate-manifest.mjs`. That step moves 8 rows from deferred to ported; it does
  not change production code.
* **Not ported, not observed by these rows.** The C# `Functions` registry, `Intern` hash-consing
  pool and the `AsyncLocal` flow semantics across `await` remain outside the additive module. A
  future row that observes a scope crossing a thread or an async task would need a different
  carrier than the thread-local stack.
* **Runtime use.** The language engine does not consume `Context` yet; the module is an additive
  library surface and its tests are the only callers. The corpus, fixtures and CLI are unchanged
  and green.
* **No regression surface.** The only production edits outside the two new files are the module
  declaration, the test-module declaration and the crate-private `Engine::set_assumptions` setter
  used by `Context::simplify`.
