# Round 76 — SuiteEngine capture/events

**Status: complete for the four deferred rows.** The block is
`SuiteEngine CaptureState, events and SetVariable/RemoveVariable API`: a host can take an
immutable `(variables, functions, revision)` snapshot, subscribe to the variable-change and
function-defined events, and set / try-get / remove a global variable through the engine. The
surface is additive; the existing evaluation, revision and wire envelope are unchanged.

Measured 2026-11-12 (America/Sao_Paulo) against the pinned oracle
`1cc28f255eca6029849f880949fc7be1d40ab8af` and the release runner
`rust/target/release/lovelace-run.exe`. The oracle was read (not rebuilt) for the source contract;
all gates below were run on the working tree.

## 1. Home crate: `rust/lovelace-lang`, not `rust/lovelace-run`

`Engine` in `rust/lovelace-lang/src/lib.rs` is the Rust analogue of the reference `SuiteEngine`
over `Interpreter`: it owns the variable store (`vars`), the user-function table (`funcs`), the
revision counter, precision, the Modus plugin host and the one evaluation/assignment path.
`rust/lovelace-run/src/main.rs` only parses argv and prints the wire envelope; it owns no engine
state. A capture/event surface therefore belongs beside `Engine`, exactly as the C#
`SuiteEngine.CaptureState`, `VariableChanged` and `FunctionDefined` live on the suite façade
(delegating to `Interpreter`). No production change was needed in `lovelace-run`.

## 2. Observed surface

### 2.1 Capture fields

The C# `SuiteEngine.CaptureState()` (`Lovelace.Suite/SuiteEngine.cs:357-370`) builds a
`StateSnapshot` (`Lovelace.Suite/Diagnostics.cs:50-64`) from the live interpreter tables under
the engine precision scope:

| C# member | Observed | Rust member |
|---|---|---|
| `StateSnapshot.Revision` | The interpreter revision at capture time (`long`). | `StateSnapshot::revision: u64`. |
| `StateSnapshot.Variables` | Dictionary `name -> StateVariable`. Captures every global; `true`/`false` are constants, not globals. | `StateSnapshot::variables: BTreeMap<String, StateVariable>`, sorted by name. |
| `StateVariable.Name` / `.Kind` / `.Display` | The name, the published `ValueKind`, and `ValueFormatter.Format(value)` at display precision. | `StateVariable { name, kind, display }`; `kind` is `Value::kind_name()`, `display` is `Engine::format` under the engine display precision. |
| `StateSnapshot.Functions` | Dictionary `name -> StateFunction`, including user functions **and** core/plugin builtins (the interpreter’s one `_functions` table). | `StateSnapshot::functions: BTreeMap<String, StateFunction>`; `registry::BUILTINS` filtered by the engine’s plugin gate, then Modus registrations, then user `funcs` (a user definition wins a name collision, as the C# single table does). |
| `StateFunction.Name` / `.Parameters` / `.IsBuiltin` / `.Plugin` | Name, declared parameter names, builtin flag and registering plugin. | `StateFunction { name, parameters, is_builtin, plugin }`. |
| `StateFunction.Span` | The user definition’s `SourceSpan?`. | **Omitted**: the Rust `Stmt::Func` carries no source span, so there is no counterpart to publish. |

The snapshot maps are owned copies: a later `SetVariable` cannot change a capture (the row
`CaptureState_IsImmutable` is exactly that assertion).

### 2.2 Event kinds, order and identity

C# sources: the events are declared on `Interpreter` (`Interpreter.cs:201-205`) and surfaced by
`SuiteEngine` (`SuiteEngine.cs:184-196`); payloads are `VariableChangedEventArgs` and
`FunctionDefinedEventArgs` (`Diagnostics.cs:12-33`); the raise sites are
`RaiseVariableChanged` / `RaiseFunctionDefined` (`Interpreter.cs:2532-2542`).

| Kind | When the reference raises it | Payload / identity | Rust |
|---|---|---|---|
| `VariableChanged` — define/reassign | A global assignment during evaluation (`EvaluateAssignAsync` -> `DefineInScope` only when the write target is the global scope, `Interpreter.cs:456-468`), an explicit `SetVariable`, and the implicit `_` result store after a successful non-void evaluation (`SuiteEngine.cs:309-310` -> `SetVariable`). | `Name`, `Value`, `Removed = false`. | `VariableChanged { name, value, removed: false }`; raised from `Expr::Assign`, the `_` store at the end of `eval_text`, and `Engine::set_variable`. |
| `VariableChanged` — remove | Only when the name existed (`Interpreter.Remove`, `Interpreter.cs:260-270`). | `Name`, the removed `Value`, `Removed = true`. | `VariableChanged { name, value: old, removed: true }`; raised from `Engine::remove_variable` only on the `Some` branch. |
| `FunctionDefined` | A script `FunctionStatement` (`Interpreter.cs:1305-1307`), an explicit `DefineFunction`, and `RegisterBuiltin` (including plugin registrations through `ModusHost`). | `Definition.Name` (plus parameters / builtin / plugin on the definition). | `FunctionDefined { definition: StateFunction }`; raised from `Stmt::Func`, `Engine::register_builtin` and `Engine::register_builtin_descriptor`. |

**Order.** The reference raises each event at the mutation point, synchronously, so the listener
order is evaluation order: an assignment’s event before a later statement’s event, and the `_`
result store after every statement event. Listeners on one event run in subscription order, one
payload per mutation, with no coalescing. The Rust implementation fires from the same points and
in the same order; `events_are_ordered_and_carry_their_payloads` pins
`x = 7; func g() { 1 }` as `VariableChanged(x, 7)` then `FunctionDefined(g)`, and
`VariableChanged_GivenAssignment_RaisesEvent`’s `x = 7` produces `x` then `_`.

### 2.3 Refusal and empty cases

| Case | Reference | Rust |
|---|---|---|
| Remove a missing variable | `Remove` returns `false`; no revision bump, no event. | `remove_variable` returns false; no event, no revision bump. |
| Lookup a missing variable | `TryGetVariable` returns false with a null out value. | `try_get_variable` returns `None`. |
| Capture before any state | Variables empty; functions are the engine’s builtins; revision is the current counter. | The same; an empty engine’s `variables` map is empty and the snapshot is frozen. |
| No subscribers | A raise with no handler is a no-op. | An empty listener vector returns before building the payload; evaluation is unchanged. |

## 3. Member map

| # | C# member (source) | Rust member | Semantics |
|---|---|---|---|
| 3.1 | `SuiteEngine.CaptureState()` (`SuiteEngine.cs:357`) | `Engine::capture_state()` | Builds an owned `StateSnapshot` from the current variables, functions and revision. |
| 3.2 | `StateSnapshot` / `StateVariable` (`Diagnostics.cs:36,50`) | `StateSnapshot` / `StateVariable` | Same fields for variables and revision; maps sorted by name; functions as 3.3. |
| 3.3 | `StateFunction` (`Diagnostics.cs:39`) | `StateFunction` | Name, parameters, builtin flag, plugin; `Span` omitted because Rust does not track it. |
| 3.4 | `SuiteEngine.VariableChanged` (`SuiteEngine.cs:185`) | `Engine::on_variable_changed` / `VariableChanged` | Subscribes a listener; `Send` is required because `Engine` is held behind an `Arc<Mutex<…>>` by the plugin seam. |
| 3.5 | `SuiteEngine.FunctionDefined` (`SuiteEngine.cs:192`) | `Engine::on_function_defined` / `FunctionDefined` | Subscribes a listener; payload carries the definition identity. |
| 3.6 | `SuiteEngine.SetVariable(string, Value)` (`SuiteEngine.cs:329`) | `Engine::set_variable(&str, Value)` | Stores/overwrites, moves the revision once, raises `removed: false`. |
| 3.7 | `SuiteEngine.TryGetVariable(string, out Value)` (`SuiteEngine.cs:332`) | `Engine::try_get_variable(&str) -> Option<&Value>` | Non-throwing lookup; `None` is the false branch. |
| 3.8 | `SuiteEngine.RemoveVariable(string)` (`SuiteEngine.cs:335`) | `Engine::remove_variable(&str) -> bool` | Removes; true + removal event when present, false with no event when absent. |
| 3.9 | Live `Variables` / `Functions` / `Revision` (`SuiteEngine.cs:176-179,168`) | Existing `Engine::variables`, `variable_values`, `functions`, `revision` | Unchanged; the snapshot is the immutable view. |

## 4. The four census ids

All four are `Lovelace.Suite.Tests/Lovelace.Suite.Tests.SuiteEngineTests/…`, deferred in
`parity/manifest.toml` with the block reason "SuiteEngine CaptureState, events and
SetVariable/RemoveVariable API". Each id is unblocked by exactly one new test:

| # | Census id | C# source | Rust test |
|---:|---|---|---|
| 1 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.SuiteEngineTests/CaptureState_IsImmutable#0` | `SuiteEngineTests.cs:72` (census sourceLine 72) | `lovelace-lang::round76_capture_events::capture_state_is_immutable` |
| 2 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.SuiteEngineTests/VariableChanged_GivenAssignment_RaisesEvent#0` | `SuiteEngineTests.cs:84` | `lovelace-lang::round76_capture_events::variable_changed_given_assignment_raises_event` |
| 3 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.SuiteEngineTests/FunctionDefined_GivenDefinition_RaisesEvent#0` | `SuiteEngineTests.cs:96` | `lovelace-lang::round76_capture_events::function_defined_given_definition_raises_event` |
| 4 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.SuiteEngineTests/SetVariableAndRemoveVariable_Work#0` | `SuiteEngineTests.cs:109` | `lovelace-lang::round76_capture_events::set_variable_and_remove_variable_work` |

The sibling row
`CaptureState_GivenVariablesAndFunctions_ReturnsMatchingSnapshot#0` is **already ported**
(round 50, `lovelace-run::round50_resweep::capture_state_publishes_variables_and_user_functions`);
it is not one of the four and is not re-ported. `capture_state_records_variables_and_user_functions`
is supporting API-level evidence for the same snapshot fields (x as `Real` with display `3.14`;
f as a non-builtin with parameters `["y"]`; `abs` present as a builtin).

`parity/manifest-map.json` is outside this round’s permitted scope and was not edited; the four
test ids above are the mapping targets.

## 5. Tests

New module: `rust/lovelace-lang/src/round76_capture_events.rs` (declared
`#[cfg(test)] mod round76_capture_events;`). Eight tests:

| Test | C# row | Asserts |
|---|---|---|
| `capture_state_is_immutable` | 1 | Snapshot variable count stays 0 after `SetVariable("x", 1)`; the snapshot does not contain `x`. |
| `variable_changed_given_assignment_raises_event` | 2 | Evaluating `x = 7` reports a `VariableChanged` whose name is `x`. |
| `function_defined_given_definition_raises_event` | 3 | Evaluating `func g() { 1 }` reports one `FunctionDefined` whose definition name is `g`. |
| `set_variable_and_remove_variable_work` | 4 | `SetVariable("x", 5)` is visible via `try_get_variable` as display `5`; `remove_variable("x")` returns true and `x` is gone. |
| `removing_a_missing_variable_is_refused_without_an_event` | refusal/empty | `remove_variable("x")` on an empty engine returns false, no event fires, and the lookup is `None`. |
| `events_are_ordered_and_carry_their_payloads` | order/payload | `x = 7; func g() { 1 }` logs exactly `var:x:7:false` then `fn:g`; the `func` statement is Void, so no `_` event. |
| `removal_event_carries_the_removed_value` | payload | Set then remove logs `(x, false, 5)` then `(x, true, 5)`. |
| `capture_state_records_variables_and_user_functions` | supporting (sibling) | Snapshot revision equals the engine revision; `x` is `Real`/`3.14`; `f` has parameters `["y"]`, `is_builtin = false`, no plugin; `abs` is a builtin with parameter `x`. |

The listeners use `Arc<Mutex<…>>` because `Engine` requires `Send` listeners (it is held behind
`Arc<Mutex<Engine>>` by the plugin closure seam); this is a Rust ownership requirement, not a
change to the observable payload.

## 6. Gates

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Focused | `cargo test -p lovelace-lang round76_capture_events` | `8 passed; 0 failed; 0 ignored`; 1554 filtered out | 0 |
| Workspace | `cargo test --workspace` | `5,206 passed; 0 failed; 1 ignored` over 22 result blocks; `lovelace-lang` reports 1,562 (1,554 before this round’s 8); no failure line; ended after doc-tests | 0 |
| Corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS` | 0 |
| CLI | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS` | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1` | generated **200 match / 0 mismatch / 0 error**; replayed **1758 match / 0 mismatch / 0 error**; total 1958; `stillMismatching=0`; harness exit 0 | 0 |
| Clippy (crate) | `cargo clippy -p lovelace-lang --all-targets -- -D warnings` | clean, `Finished` | 0 |
| Clippy (workspace, desktop excluded) | `cargo clippy --workspace --all-targets --exclude lovelace-desktop -- -D warnings` | clean, `Finished` | 0 |
| Format (crate) | `cargo fmt -p lovelace-lang -- --check` | clean | 0 |

**Workspace-wide clippy/format caveat.** The whole-workspace commands do not reach green in this
working tree, and every diagnostic is in `rust/lovelace-desktop`, outside this lane’s scope:

* `cargo clippy --workspace --all-targets -- -D warnings` reports
  `rust/lovelace-desktop/src/execution.rs:652` (`manual_unwrap_or_default`) and
  `rust/lovelace-desktop/src/ui/plot.rs:98` (`manual_clamp`);
* `cargo fmt --all -- --check` lists only `rust/lovelace-desktop/src/…` files.

`cargo clippy --workspace --all-targets --exclude lovelace-desktop -- -D warnings` exits 0, and
`cargo clippy -p lovelace-lang --all-targets -- -D warnings` exits 0;
`cargo fmt -p lovelace-lang -- --check` exits 0. No desktop or symbolics file was edited by
this round (another lane owns them).

The workspace test log is `rust/target/tmp/round76-workspace.log`. Its total is above the
5,191 baseline because it also contains concurrent-lane tests; the `lovelace-lang` package
itself is exactly the 1,554 pre-round total plus this round’s 8 (`1,562`).

## 7. Scope and boundaries

Changed files: `rust/lovelace-lang/src/lib.rs` and the new
`rust/lovelace-lang/src/round76_capture_events.rs`, plus this report. No `.cs` file, fixture,
corpus file, `parity/` file, or other document was edited. The production change is additive:
new public types/methods and listener fields; the existing `Expr::Assign`, `Stmt::Func`, `_` store
and `register_builtin` paths only call a raise helper that returns immediately when no listener is
subscribed. The wire envelope and the existing one-per-evaluation revision bump are untouched.

Boundaries, stated rather than hidden:

1. **`Span`** is not published in `StateFunction` (no Rust source-span field on `Stmt::Func`).
2. **Plugin-load definitions**: the reference raises `FunctionDefined` for every builtin a
   `LoadPlugin` registers (its `ModusHost` calls `Interpreter.RegisterBuiltin`). The Rust
   `Engine::load_plugin` registers through `ModusHost` directly and does not surface those
   registrations as listener events, so the event log covers script definitions and explicit host
   `register_builtin`/`register_builtin_descriptor` calls. This is outside the four rows; mapping
   plugin-load event order would require an order log in `ModusHost`.
3. **Flat-store locals**: the Rust engine currently has one flat variable store (the separate
   STILL-DIFFERENT row `FunctionTests/Evaluate_GivenFunctionLocalVariable_DoesNotLeakToGlobal`).
   A block/function-local assignment therefore leaks *and* raises a `VariableChanged` event; the
   reference would define a child-scope binding and raise nothing. Global assignments (the four
   rows, and the `_` store) match. The raise site is commented to move with a future scope model.
4. **Host `DefineFunction`** has no Rust public method; script `func` and host `register_builtin`
   are the definition sources the four rows drive, and both raise.
5. **Revision on script mutations** stays the existing one-per-evaluation counter to keep the
   wire output frozen; the new explicit `set_variable`/`remove_variable` move it once, like the
   reference. `CaptureState_IsImmutable` does not assert a revision value.