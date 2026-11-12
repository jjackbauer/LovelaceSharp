# Round 95 — language/engine singleton rows

Scope: the eleven deferred census rows whose ids name a language/engine singleton class. The
oracle is the pinned C# runner (`dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll
--eval/--file --omit-functions`); every "oracle" value below was measured against it in this
round. The manifest is read-only for this lane, so the ids are reported here for the parent to
re-disposition.

Id extraction: the 37 deferred rows in `parity/manifest.toml` were filtered by the CLASS SEGMENT of
the id, not a substring. "Function" matched two ids; only
`Lovelace.Suite.Tests.FunctionTests/...` is this lane's row —
`Lovelace.Symbolics.Tests.HyperbolicBuiltinsAndNegativeIntegerPowersTests/...` merely contains the
word "Functions" and belongs to the symbolic lane. One row per named class, eleven rows total.

## The rows

| # | Row id (parity/manifest.toml) | Class | Before | After | Status |
|---|---|---|---|---|---|
| 1 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.BroadcastTests/ColumnVector_BroadcastsToMatrix#0` | Broadcast | `ok=false`, exit 1, `InvalidOperation`/`DomainError`, "Operands could not be broadcast together with shapes [2, 1] and [2, 2]." | `ok=true`, `[[11, 21], [32, 42]] (Array)` | closed |
| 2 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.FunctionTests/Evaluate_GivenFunctionLocalVariable_DoesNotLeakToGlobal#0` | Function | `variables` = [`_`, `local`] | `variables` = [`_`] | closed |
| 3 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.StatementTests/Evaluate_GivenVariableAssignmentInsideBlock_DoesNotLeak#0` | Statement | `variables` = [`_`, `z`] | `variables` = [`_`] | closed |
| 4 | `Lovelace.Run.Tests/Lovelace.Run.Tests.RecursionDepthEnvelopeTests/NestedBlocksInsideARecursiveBody_AreRefused_InsteadOfDying#0` | RecursionDepthEnvelope | `ok=true`, exit 0, result 0 | `ok=false`, exit 1, `DepthExceeded`/`BudgetExceeded`, recoverable | closed |
| 5 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecursionDepthGuardTests/ADeepExpressionInsideTheRecursiveBody_IsRefused#0` | RecursionDepthGuard | ten levels measured 60 budget units (call levels only), so the call answered 1000 | `ok=false`, `DepthExceeded`/`BudgetExceeded` | closed |
| 6 | `Lovelace.Run.Tests/Lovelace.Run.Tests.ArityPropertyTests/DeclaredArity_TravelsWithEveryPluginBuiltin#0` | ArityProperty | no Rust surface: `StateFunction` published no declared arity | `StateFunction.min_arity`/`variadic` carry the descriptor; 80 plugin builtins checked | closed |
| 7 | `Lovelace.Run.Tests/Lovelace.Run.Tests.ErrorCategoryMirrorTests/EveryMirroredTaxonomy_EqualsTheEnum#0` | ErrorCategoryMirror | enum existed; no mirrored arrays to compare | three family mirrors equal `ErrorCategory::ALL` | closed |
| 8 | `Lovelace.Run.Tests/Lovelace.Run.Tests.GoldenFixtureRecorder/RecordGoldenFixtures#0` | GoldenFixtureRecorder | no recorder: only the comparison existed | recorder beside the comparison, same normalise, inert unless `LOVELACE_RECORD_GOLDENS=1` | closed |
| 9 | `Lovelace.Run.Tests/Lovelace.Run.Tests.LibraryProjectionAgreementTests/TheFacade_PublishesEveryStoredDigit_WhenTheDisplayBoundIsLower#0` | LibraryProjectionAgreement | no display-only bound; the audit's two-bound shape was not expressible through the library | `set_display_precision` + full-digit projection; façade and CLI publish the same 1 100 digits | closed |
| 10 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspIntegerArgumentContractTests/TheExactlyIntegralRealRefusal_IsStillTheContractsContract#0` | DspIntegerArgumentContract | the wrong-kind refusal was an inline string; no contract constructor | `argument_contract::not_an_integer_kind`/`not_an_integer`; engine reachable via `eye(2.0)` | closed |
| 11 | `Lovelace.Representation.Tests/Lovelace.Representation.Tests.UnitTest1/Test1#0` | UnitTest1 | empty C# body, no assertion | no test written | **unmappable** |

## 1. Broadcast — column vector broadcasts to matrix

Probe script: `[[1], [2]] + [[10, 20], [30, 40]]`.

| Runner | ok | exit | display/typed | code |
|---|---|---|---|---|
| oracle | true | 0 | `[[11, 21], [32, 42]] (Array)` | — |
| rust before | false | 1 | — | `InvalidOperation`/`DomainError` |
| rust after | true | 0 | `[[11, 21], [32, 42]] (Array)` | — |

The exact differing field was `$.exitCode` (1 vs 0) with the broadcast refusal message. Cause:
`builtins::elementwise` refused every pair of container shapes that differed; only the vector-vs-
array arm broadcast. The fix gives every container pair ONE right-aligned broadcast
(`builtins::broadcast_shape`, `broadcast_elementwise`, `broadcast_element_at`), so [2, 1] against
[2, 2] broadcasts, a length-1 vector broadcasts across a longer one, and shapes that disagree at
neither 1 still refuse. The vector/vector result stays a Vector; every pair involving an Array is
an Array. `[1, 2] + [1, 2, 3]` still refuses, as the oracle does.

## 2. Function-local variable does not leak

Probe script: `func f() { local = 42 }; f()` with `--omit-functions` (variables visible).

| Runner | `variables` names | result |
|---|---|---|
| oracle | [`_`] | 42 |
| rust before | [`_`, `local`] | 42 |
| rust after | [`_`] | 42 |

The exact differing field was the length of `$.variables` (2 vs 1). Cause: the engine had one flat
store; assignments always wrote it. The fix adds a lexical scope chain
(`Engine.scopes`): a user call pushes a frame whose parent is the global store (the reference's
`new Scope(_global)`), so a callee never sees its caller's block locals and its own definitions are
dropped when the call returns.

## 3. Block assignment does not leak

Probe script: `{ z = 99 }`.

| Runner | `variables` names | result |
|---|---|---|
| oracle | [`_`] | 99 |
| rust before | [`_`, `z`] | 99 |
| rust after | [`_`] | 99 |

Same cause and fix as row 2. The reference semantics are preserved exactly: assignment updates the
nearest existing binding (measured: `x = 10; { x = 20 }; x` is 20 and `x` is global), a new name
binds in the innermost scope, a top-level `for` variable is defined globally (measured on the
oracle: `for i in 1..3 { }; i` answers 3), and only a GLOBAL write raises `VariableChanged`.

## 4. Nested blocks inside a recursive body are refused instead of dying

Probe script: `func f(n) { if (n == 0) { return 0 }; <120 nested ifs> return f(n - 1) <120 }>
f(5)`.

| Runner | ok | exit | code/category |
|---|---|---|---|
| oracle | false | 1 | `DepthExceeded`/`BudgetExceeded`, message "evaluation depth 513 exceeds … 512" |
| rust before | true | 0 | — (answered 0) |
| rust after | false | 1 | `DepthExceeded`/`BudgetExceeded`, recoverable |

The exact differing field was `$.exitCode` (0 vs 1). Cause: the budget charged call levels only
(6 units each), so five call levels measured 30 and the 600 statement levels were invisible.

The fix replaces the call-level-only counter with the reference's walk budget: a frame costs 4
units, one unit per statement executed (a Block is grouping and costs nothing), and one unit per
descending expression node (a leaf and a call argument's subtree cost nothing). The base of the
measurement is the first user frame plus everything the top-level statement charged to reach it, so
sibling statements and enclosing composition do not move the boundary.

The boundary matrix was measured against the oracle and reproduced exactly:

| shape (per call level) | oracle max depth | rust after max depth |
|---|---|---|
| `f(n-1)` | 85 | 85 |
| `f(n-1) + 1` | 72 | 72 |
| `f(n-1) + 1 + 1` | 63 | 63 |
| `f(n-1) + 1 + 1 + 1` | 56 | 56 |
| call inside 1 / 2 / 3 nested ifs | 72 / 63 / 56 | 72 / 63 / 56 |
| `while (1 == 1) { return f(n-1) }` | 72 | 72 |
| `if (1 == 1) return f(n-1)` | 72 | 72 |
| bare block `{ return f(n-1) }` | 85 | 85 |
| no-argument recursion `return f()` | 85 | 85 |

Constant f(85) answers and f(86) refuses on both runners; the already-ported composition rows
(`round19_batch`) stay green.

## 5. A deep expression inside the recursive body is refused

Probe script: `func f(n) { if (n == 0) { return 0 }; return f(n - 1) <100 × " + 1"> }
f(10)`.

| Runner | ok | exit | code/category |
|---|---|---|---|
| oracle | false | 1 | `DepthExceeded`/`BudgetExceeded` |
| rust before | true | 0 | — (answered 1000; the body expressions were invisible to the budget) |
| rust after | false | 1 | `DepthExceeded`/`BudgetExceeded` |

The oracle control `g(4)` with the same 100-term body answers 400 on both runners, so the refusal
is the recursion depth, not the shape.

## 6. Declared arity travels with every plugin builtin

The C# row reads `FunctionDefinition.MinArity`/`Variadic` and compares them with the descriptor.
Rust's counterpart is the snapshot's `StateFunction`; before this round it published name,
parameters, builtin flag and plugin only, so the property had no surface to be asserted on.

The fix adds `min_arity: usize` and `variadic: bool` to `StateFunction` and populates them from
the declaration everywhere a definition is published: the static registry (`registry::BuiltinInfo`),
a descriptor-carrying host registration (`register_builtin_descriptor`), a plain host registration
(parameters are the exact arity) and a user definition. The test walks the snapshot's plugin
builtins and requires the two fields (and the parameter count) to equal the registry declaration:
80 builtins checked (the whole plugin registry).

## 7. Every mirrored taxonomy equals the enum

`diagnostic::ErrorCategory` already froze the seven names. What was missing was the C# row's shape:
three literal arrays in the envelope-test families plus the one place that compares them with the
enum. The three mirrors now live beside the families they guard:
`round28_run_batch::MIRRORED_ERROR_CATEGORIES` (allocation refusal),
`plot_surface_wire::MIRRORED_ERROR_CATEGORIES` (plot directory) and
`round90_runner_families::MIRRORED_ERROR_CATEGORIES` (plot file). The test in
`round95_singletons` compares all three with `ErrorCategory::ALL` in the frozen order.

## 8. Golden-fixture recorder

The C# recorder re-records the goldens from the current runner using the SAME `TestSupport.Normalise`
the comparison uses, inert unless `LOVELACE_RECORD_GOLDENS=1`, writing the source fixture a
reviewer sees in the diff. The Rust recorder (`golden_batch::record_all`) is that recorder: it
runs each fixture through the runner's own execute/envelope pair, applies the same `normalise`,
writes `<volatile>` placeholders to `Lovelace.Run.Tests/fixtures/<name>.json`, and returns the
names it rewrote. The corpus list is now one shared constant, so the comparison, the recorder and
the "every fixture has a corpus row" check cannot disagree.

Two tests pin it: the recorder is inert (writes nothing, reports nothing) when the variable is
unset, and a re-record of every golden is content-identical to the golden on disk after the same
normalisation — so a re-record is a reviewable no-op, never a silent rewrite.

## 9. Library projection agreement

The audit's shape is `ComputationDecimalPlaces = 1100; DisplayDecimalPlaces = 100` then
`pi(1100)`: the structured value must carry all 1 100 stored digits, the human rendering stays at
100, and nothing may claim truncation.

Rust already published stored digits through the CLI/structured projection, but the two bounds
could not be expressed through the library (one `setprecision` knob) and the projection inherited
whatever display scope the caller had installed. The fix adds `Engine::set_display_precision`
(a display-only bound) and makes `structured::project_value` render a Real at its stored digits
regardless of the ambient display scope.

Measured through the façade: value 1 100 digits, starts `3.14159…` and ends `24972177528347913151`,
no `truncated`/`truncationReason`/`budget` fields, display rendering 100 digits. The CLI's
`setprecision(1100); pi(1100)` payload equals the façade's payload byte for byte. Untruncated
values are neither cut nor padded (`1/8` → `0.125` exact rational; `pi(30)` → 30 digits), and a
value the engine really clamped still crosses with `truncated: true`, reason `digit-cap`, budget
1000.

## 10. The exactly-integral-Real refusal is the contract's contract

The C# row constructs `ArgumentContractException.NotAnIntegerKind("movingavg", "w", …, "3")` and
asserts the failure kind, the "write it as 3" message and the `ArgumentException` base. Rust's
refusal existed but only as an inline string in a private helper, so the contract could not be
asserted without a builtin that happened to reach it.

The fix publishes `argument_contract::not_an_integer_kind` and `argument_contract::not_an_integer`
(the recoverable `InvalidArgument`/`TypeMismatch` pair) and routes `exact_integer_argument`
through them; existing messages are byte-identical (the index row's "got the Text." included).

Measured: the contract constructor names the integer ("write it as 3"); the engine's pinned call
sites produce the same shape — `eye(2.0)` → `InvalidArgument`/`TypeMismatch`, "got the Real 2,
which is integral — write it as 2." — while the DSP boundary normalises an exactly integral Real
(`movingavg(…, 3.0)` answers what `movingavg(…, 3)` answers), exactly as the C# row documents.

## 11. UnitTest1 — unmappable (needs a disposition decision)

`Lovelace.Representation.Tests/UnitTest1.cs` is a template placeholder:

```csharp
public class UnitTest1
{
    [Fact]
    public void Test1()
    {

    }
}
```

The body is empty: no assertion, no observable, no oracle value. There is nothing to port and
nothing to assert, and inventing a test would manufacture a contract the row does not have. This
row is **unmappable** and needs a disposition decision (delete the placeholder, or exclude it with
a recorded rationale) rather than code. No test was written.

## Gates (observed 2026-11-12, repo root)

| Gate | Command | Observed | Exit |
|---|---|---|---|
| Workspace | `cargo test --workspace` | **5 570 passed, 0 failed, 1 ignored** (lang 1 701, run 429+1, corpus 61, desktop 277, others) | 0 |
| Corpus | `rust/target/release/lovelace-corpus.exe` (with `TMP` set) | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, PASS | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, PASS | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1` | generated **200 match / 0 mismatch / 0 error**; replayed **1 758 match / 0 mismatch / 0 error**; `stillMismatching: []`, `new: 0` | 0 |
| Clippy | `cargo clippy -p lovelace-lang -p lovelace-run --all-targets -- -D warnings` | clean | 0 |
| Format | `cargo fmt -p lovelace-lang -p lovelace-run -- --check` | clean | 0 |

The corpus run needs the lane's temp directory to exist (`rust/target/tmp`); without it the
`plot` corpus row fails on the default Windows temp path, unrelated to any code change (re-run
with the environment from the task: 214/214).

## Scope and the shared tree

Changed files:

* `rust/lovelace-lang/src/lib.rs` — scope chain, walk budget, `StateFunction` declared arity,
  `set_display_precision`, module declarations.
* `rust/lovelace-lang/src/builtins.rs` — right-aligned broadcast; the contract constructors are
  used by `exact_integer_argument`.
* `rust/lovelace-lang/src/structured.rs` — Real payload at stored digits.
* `rust/lovelace-lang/src/argument_contract.rs` (new) — the published refusal constructors.
* `rust/lovelace-lang/src/round95_engine_singletons.rs` (new) — the language-tier row tests.
* `rust/lovelace-run/src/round95_singletons.rs` (new) — the wire-tier row tests.
* `rust/lovelace-run/src/golden_batch.rs` — shared corpus, recorder, recorder tests.
* `rust/lovelace-run/src/main.rs` — the new test module declaration.
* `rust/lovelace-run/src/round28_run_batch.rs`, `plot_surface_wire.rs`,
  `round90_runner_families.rs` — the three taxonomy mirrors.

No `.cs`, fixture, `parity/`, `lovelace-desktop` or docs file other than this report was edited.

Shared-tree note: the numeric lane works in the same files this round does, and nothing of
theirs was overwritten. `rust/lovelace-lang/src/builtins.rs` also carries their batched-matmul
row (`matmul`, hunks around line 4760+); `rust/lovelace-lang/src/lib.rs` also carries their
`mod round95_singletons;` declaration at the bottom (their test module lives in
`rust/lovelace-lang/src/round95_singletons.rs`). My additions are declared separately, both edits
coexist, and `cargo test --workspace` is green with both present. No nat/int/real/array file was
edited by this lane.
