# Round 67 — Modus plugin seams

**Status: complete for 22 of the 23 deferred rows; 1 row is blocked on a Symbolics-only API; the 24th family member was already ported.**

The 24-row family is the set of `Lovelace.Suite.Tests` cases whose contract is the Modus
registration/dispatch seam, plus the adjacent `RecordValueTests` rows explicitly deferred as
"RecordValue member access + modus plugin seam", plus `SymbolicsPlugin.ParameterDomainOf`. One of
the 24 (`DspPluginTests/DspPlugin_GivenUntouchedKnob_ComputesAtFastBudget_AndPromotesWhenRaised#0`)
was already mapped in an earlier round; it is listed below for completeness and is not re-ported.

## 1. Where the seam lives, and why

The seam is implemented in **`rust/lovelace-lang`**, not `rust/lovelace-run`.

`Engine` in `lovelace-lang/src/lib.rs` is the Rust analogue of the reference `SuiteEngine` /
`Interpreter`: it owns the user function table, variables, precision flag, session assumptions and
the one `call` dispatch point. The static `registry::BUILTINS` table is metadata only; it has no
mutable plugin state. The runner (`lovelace-run/src/main.rs`) parses argv and prints the envelope and
owns no registry state at all. A plugin host therefore belongs beside `Engine`, exactly as the C#
`ModusHost` lives beside `Interpreter`.

The new module is `rust/lovelace-lang/src/modus.rs` (964 lines); `round67_modus.rs` holds the 22 new
tests. Production changes in `lib.rs`, `registry.rs` and `builtins.rs` are additive; the only
behavioural edits outside the new surface are the two RecordValue member-access messages (see §2.8)
and the bare-engine gate, which is opt-in and leaves `Engine::new()` unchanged.

## 2. Observed seam surface (member map)

| # | Seam (C#) | Observed behaviour | Rust member |
|---|---|---|---|
| 2.1 | **Registration / lifecycle** — `IModusPlugin`, `ModusHost.Load`, `_loadedPlugins` | A plugin is loaded once; the second load fails; registrations are stamped with the loading plugin's name; `Register` failures propagate but the plugin stays marked loaded (the C# `HashSet.Add` happens before `Register`). | `ModusHost::load`, `ModusPlugin::{name, register}`, `loaded_plugins`, `loading_plugin`; `PluginBuiltin.plugin`; `Engine::load_plugin`. Duplicate key is the plugin name (the C# key is the CLR type; the observable sentence "already loaded" is preserved). |
| 2.2 | **Lookup / core-name refusal** — `GuardName`, `Interpreter.Functions` | A plugin builtin may not shadow an existing core name; the refusal names the name and says `already registered`; plugin builtins are then resolvable by the language. | `ModusHost::register_inner` checks `registry::find(name)` and the host map; `ModusHost::contains`, `names`, `Engine::has_function`; dispatch is `Engine::call` → `ModusHost::call` before the static table. |
| 2.3 | **Capability advertisement** — `BuiltinDescriptor`, `FunctionDefinition.PluginName`, Dsp opt-in | A descriptor carries name/parameters/min-arity/variadic/category/summary/return kind; a loaded plugin advertises its names; a bare engine refuses `fft` with empty suggestions until the Dsp descriptor is loaded; core `abs` carries no plugin. | `BuiltinDescriptor` (+ `with_min_arity`/`with_variadic`), `ModusHost::descriptor_of`, `plugin_name_of`, `Engine::plugin_name_of`, `Engine::new_bare`, `Engine::plugin_enabled`, filtered `registry::suggest_among`. |
| 2.4 | **Call-site arity** — `FunctionDefinition.DeclaredArity`, `BuiltinArityException` | One validator computes lower/upper from `Parameters.Count`, `MinArity` (`-1` = exact) and `Variadic`; refusals keep the documented grammar: `pair(): expected 2 arguments; got 1.`, `opt(): expected 1 to 2 arguments; got 0.`, `variadic_min(): expected at least 1 argument; got 0.` | `PluginBuiltin::declared_arity`, `arity_failure`, `ErrorSignature` with parameters/min/max/variadic/actual; `Engine::call` routes host builtins through the same path. |
| 2.5 | **Payload channel** — `PayloadMap.Wrap/Unwrap`, `ScalarResult` | Arguments cross as payloads, not `Value`: Natural/Integer/Real/Complex/Symbolic/Boolean/Text/Record/Domain/Enum/Array/Void/Function; a returned payload is wrapped back; arrays keep shape; a rank-1 result becomes a Vector. Enum keeps BOTH type and member; Record keeps its type name and fields. | `Payload`, `PayloadArray`, `Payload::{from_value,into_value}`, `Arg`, `PluginArgs`, `ScalarResult::{from,from_boolean,from_text,from_array}`, `register_builtin_scalar`, `register_builtin_descriptor_scalar`, `register_array_builtin`. |
| 2.6 | **Dispatch hooks** — `IFieldKernel<T>`, `ArrayOp`, `ModusHost.TryDispatch` | Kernels are per exact scalar type; the core injects the field identity (Real/Integer/Natural); an unsupported element type or a declining kernel returns `false` so the reference backend runs; a missing `Add` arm makes `Multiply` fall back. | `ArrayOp`, `Field`, `FieldKernel<T>`, `KernelScalar`, `ModusHost::{register_kernel,try_dispatch}`, `Engine::{register_kernel,try_dispatch_kernel}`. |
| 2.7 | **Argument-shape guard** — `BuiltinArguments`, `ValueShapeException`, `BuiltinShapeException` | A failed coercion names the builtin, the 1-based position, the required shape and the arrived kind: `probe(): argument 2 must be an array or vector; got Natural.`; a body's own hard cast says `a value this builtin can use`; an engine-internal value is not disguised as an argument error. | `Arg` accessors attribute the failed payload to the argument index; `PluginError::{Shape,Cast,Domain,Internal}`; `plugin_failure` produces `InvalidArgument/TypeMismatch` for attributed failures and `InternalError/InternalInvariantFailure` for unattributed ones. |
| 2.8 | **Record member access** — `EvaluateMemberAsync` | A record miss names the type and available members; a non-record says `member access requires a record result` (and the Symbolic hint). These were the two deferred RecordValue rows; the C# sentence is reproduced exactly. | `Expr::Member` in `lib.rs` now emits `record 'DemoResult' has no member 'nope'. Available members: count, label, nested.` and `member 'length' is not available on type 'Vector': member access requires a record result.` |
| 2.9 | **Plugin-instance assumption scope** — `symbolics.Context.WithAssumptions`, shared plugin instance | A caller-installed scope has precedence INSIDE an engine evaluation and is gone afterwards; one plugin instance loaded into two engines never lets engine A's committed assumption decide engine B, even while A is inside its own evaluation. | `ModusHost::register_assumption_context`, `active_assumption_scope`; `Engine::eval` installs the scope for the evaluation and restores the engine's own set on the way out. `Context::with_assumptions` supplies the thread-local scope. |

## 3. The 24 census rows and what each unblocks

The family is 23 deferred rows plus one already-ported row. These are the 24 full census ids:

```
Lovelace.Suite.Tests/Lovelace.Suite.Tests.BuiltinArgumentShapeGuardTests/ACoercionFailureOnAnEngineInternalValue_IsNotDisguisedAsAnArgumentError#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.BuiltinArgumentShapeGuardTests/ARegisteredBuiltinThatCastsItsOwnPayload_IsAttributedToTheArgumentItReadLast#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.BuiltinArgumentShapeGuardTests/ARegisteredBuiltinThatCoercesItsArgument_IsAttributedToTheBuiltinAndThePosition#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/DspPlugin_GivenUnregisteredEngine_ThrowsUnknownFunction#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/DspPlugin_LoadedTwice_Throws#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/PluginBuiltin_GivenCoreNameCollision_Throws#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.EnumPayloadSeamTests/EnumValue_RoundTripsThroughTheModusSeam#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusArityTests/ExactCount_RequiresTheDeclaredCount#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusArityTests/MinArity_AllowsTheOptionalTail_AndStillRefusesLess#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusArityTests/ParametersOnlyRegistration_UsesTheSameRule#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusArityTests/Variadic_RemovesTheUpperBound_AndHonoursTheLowerOne#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusTests/Dispatch_UnsupportedElementType_FallsBack#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusTests/DspPlugin_LoadedThroughModusSeam_RegistersDspBuiltins#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusTests/Kernel_DeclinesUnsupportedOp#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusTests/PluginLoad_RegistersKernel_AndDispatchRuns#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusTests/RealAddKernel_GivenRealOperands_MatchesDirectRealAddition#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusTests/ScalarResult_GivenBooleanAndText_ReturnValues#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordValueTests/MemberAccess_OnNonRecord_ExplainsRequirement#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordValueTests/MemberAccess_UnknownField_NamesAvailableMembers#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordValueTests/Record_CrossesModusPluginBoundary#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.SharedPluginInstanceAssumptionIsolationTests/CallerInstalledAssumptionScope_KeepsPrecedenceInsideAnEngineEvaluation#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.SharedPluginInstanceAssumptionIsolationTests/OnePluginTwoEngines_AssumptionCommittedWhileAIsRunning_DoesNotDecideB#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxStructuredResultsTests/ParameterDomainOf_MapsEveryDeclaredArm#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/DspPlugin_GivenUntouchedKnob_ComputesAtFastBudget_AndPromotesWhenRaised#0
```

Every "mapped" row below is a new test in `rust/lovelace-lang/src/round67_modus.rs`.

| # | Census id (abbreviated) | Status | Rust evidence |
|---:|---|---|---|
| 1 | `…/BuiltinArgumentShapeGuardTests/ACoercionFailureOnAnEngineInternalValue_IsNotDisguisedAsAnArgumentError#0` | mapped | `coercion_failure_on_an_engine_internal_value_stays_internal` |
| 2 | `…/ARegisteredBuiltinThatCastsItsOwnPayload_IsAttributedToTheArgumentItReadLast#0` | mapped | `registered_builtin_that_casts_its_payload_is_attributed_to_the_last_argument` |
| 3 | `…/ARegisteredBuiltinThatCoercesItsArgument_IsAttributedToTheBuiltinAndThePosition#0` | mapped | `registered_builtin_that_coerces_its_argument_is_attributed_to_the_position` |
| 4 | `…/DspPluginTests/DspPlugin_GivenUnregisteredEngine_ThrowsUnknownFunction#0` | mapped | `dsp_plugin_unregistered_engine_refuses_fft` (uses the explicit `Engine::new_bare`) |
| 5 | `…/DspPluginTests/DspPlugin_LoadedTwice_Throws#0` | mapped | `dsp_plugin_loaded_twice_is_refused` |
| 6 | `…/DspPluginTests/PluginBuiltin_GivenCoreNameCollision_Throws#0` | mapped | `plugin_builtin_core_name_collision_is_refused` |
| 7 | `…/EnumPayloadSeamTests/EnumValue_RoundTripsThroughTheModusSeam#0` | mapped | `enum_value_round_trips_through_the_modus_seam` |
| 8 | `…/ModusArityTests/ExactCount_RequiresTheDeclaredCount#0` | mapped | `exact_count_requires_the_declared_count` |
| 9 | `…/ModusArityTests/MinArity_AllowsTheOptionalTail_AndStillRefusesLess#0` | mapped | `min_arity_allows_the_optional_tail_and_still_refuses_less` |
| 10 | `…/ModusArityTests/ParametersOnlyRegistration_UsesTheSameRule#0` | mapped | `parameters_only_registration_uses_the_same_rule` |
| 11 | `…/ModusArityTests/Variadic_RemovesTheUpperBound_AndHonoursTheLowerOne#0` | mapped | `variadic_removes_the_upper_bound_and_honours_the_lower_one` |
| 12 | `…/ModusTests/Dispatch_UnsupportedElementType_FallsBack#0` | mapped | `dispatch_unsupported_element_type_falls_back` |
| 13 | `…/ModusTests/DspPlugin_LoadedThroughModusSeam_RegistersDspBuiltins#0` | mapped | `dsp_plugin_loaded_through_modus_seam_registers_dsp_builtins` |
| 14 | `…/ModusTests/Kernel_DeclinesUnsupportedOp#0` | mapped | `kernel_declines_unsupported_op` |
| 15 | `…/ModusTests/PluginLoad_RegistersKernel_AndDispatchRuns#0` | mapped | `plugin_load_registers_kernel_and_dispatch_runs` |
| 16 | `…/ModusTests/RealAddKernel_GivenRealOperands_MatchesDirectRealAddition#0` | mapped | `real_add_kernel_matches_direct_real_addition` |
| 17 | `…/ModusTests/ScalarResult_GivenBooleanAndText_ReturnValues#0` | mapped | `scalar_result_boolean_and_text_return_values` |
| 18 | `…/RecordValueTests/MemberAccess_OnNonRecord_ExplainsRequirement#0` | mapped | `member_access_on_a_non_record_explains_the_requirement` |
| 19 | `…/RecordValueTests/MemberAccess_UnknownField_NamesAvailableMembers#0` | mapped | `member_access_unknown_field_names_available_members` |
| 20 | `…/RecordValueTests/Record_CrossesModusPluginBoundary#0` | mapped | `record_crosses_the_modus_plugin_boundary` |
| 21 | `…/SharedPluginInstanceAssumptionIsolationTests/CallerInstalledAssumptionScope_KeepsPrecedenceInsideAnEngineEvaluation#0` | mapped | `caller_installed_assumption_scope_keeps_precedence_inside_an_engine_evaluation` |
| 22 | `…/SharedPluginInstanceAssumptionIsolationTests/OnePluginTwoEngines_AssumptionCommittedWhileAIsRunning_DoesNotDecideB#0` | mapped | `assumption_committed_while_a_is_running_does_not_decide_b` |
| 23 | `Lovelace.Symbolics.Tests/DxStructuredResultsTests/ParameterDomainOf_MapsEveryDeclaredArm#0` | **blocked** | The row calls the internal `SymbolicsPlugin.ParameterDomainOf(ParameterDomain)`, including an undeclared `(ParameterDomain)99` that must throw. Both the `ParameterDomain` enum and the total mapping live in `lovelace-symbolics`; that crate is outside this lane's `run`/`lang` scope. A `lang`-local copy would be a shim, not the kernel's own mapping, so it is not written. |
| 24 | `…/DspPluginTests/DspPlugin_GivenUntouchedKnob_ComputesAtFastBudget_AndPromotesWhenRaised#0` | already ported | `lovelace-run::dsp_round21::dsp_plugin_untouched_knob_computes_at_fast_budget_and_promotes` (mapped before this round; the family's 24th member, not a deferred row). |

The 22 mapped ids above are the ones this round unblocks. Id 23 is the one row that cannot be
reproduced without the Symbolics `ParameterDomain` capability. Id 24 already had coverage.

## 4. What the tests assert

The new module is a direct mirror of the C# case bodies, with the C# assertion translated to the
Rust observable:

* registration + kernel dispatch: `StatisticsPlugin` registers a Real `Add` kernel; dispatch writes
  `5, 7, 9`, a direct `1.5 + 2.25` comparison, `i32` falls back, and `Multiply` declines;
* the ScalarResult channel returns `True (Boolean)` and `hello`;
* Dsp lifecycle: bare engine refuses `fft` with exactly `Unknown function 'fft'.` and no
  suggestions, a loaded Dsp descriptor makes `fft([1,0,0,0])` render `[1, 1, 1, 1] (Vector)`,
  `fft` advertises `Lovelace.Dsp` while `abs` advertises none, a second load says
  `already loaded`, and a plugin that tries to register `abs` says `already registered`;
* descriptor arity: exact, parameters-only, optional tail and variadic messages above, plus
  `InvalidArgument`/`TypeMismatch` on the exact-count refusal;
* payload/record/enum: the EnumEcho plugin receives an `Enum("SolveStatus","Solved")` payload and
  returns an Enum value; a plugin-produced `PluginResult` keeps its record type and `answer` field;
  the two RecordValue diagnostics and the unknown-field member list match the C# sentences;
* assumption scope: a `Context::with_assumptions` scope makes `simplify(x > 0)` `True` only inside
  the scope; a second engine asks from inside the first engine's `hold_gate` builtin and still
  answers `x > 0`, then engine A remains `True` after the gate.

## 5. Gates (run 2026-11-12, America/Sao_Paulo)

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| focused | `cargo test -p lovelace-lang round67_modus` | `22 passed; 0 failed` | 0 |
| workspace | `cargo test --workspace` | `5,136 passed; 0 failed; 1 ignored` over 21 binaries (5098 baseline + 22 Modus + 16 concurrent polynomial-lane tests) | 0 |
| compile check | `cargo check -p lovelace-lang --tests` | `Finished dev profile`, 0 warnings | 0 |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings` | `Finished dev profile` | 0 |
| fmt | `cargo fmt --all -- --check` | no diff | 0 |
| corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20` / `rust: 20/20` / `PASS` | 0 |
| CLI | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16` / `PASS` | 0 |
| differential | `node parity/diff/run.mjs --cases 200 --seed 1` | generated `200 match / 0 mismatch / 0 error`; replayed `1758 match / 0 mismatch / 0 error`; `harness exit: 0` | 0 |

## 6. Residual

* **`ParameterDomainOf_MapsEveryDeclaredArm#0` remains deferred.** It needs a public/total
  `ParameterDomain` mapping inside `lovelace-symbolics`; the two declared arms both map to
  `MathDomain.Integer` and the undeclared value must be refused. This lane did not add a
  `lovelace-lang` duplicate because that would not be the kernel's own projection.
* The Rust runner ships one binary with all builtins. `Engine::new()` therefore keeps every
  advertised builtin, while the new `Engine::new_bare()` / `load_plugin` pair is the explicit
  equivalent of the reference's opt-in engine. The Dsp descriptor is descriptor-only: the kernels
  are already compiled into the language crate, so loading it enables the advertised names and
  stamps their plugin; no assembly loading is claimed.
* No `.cs`, fixture, corpus, `parity/` file or manifest was edited. `parity/manifest-map.json` is
  unchanged, so the parent still has to map the 22 new tests to their census ids.
* `round66_resweep.rs` received a formatting-only normalisation to the workspace's edition-2024
  `rustfmt`; no assertion changed.

## 7. Files

| File | Change |
|---|---|
| `rust/lovelace-lang/src/modus.rs` | new: plugin host, payload channel, descriptors, arity, kernels, scope |
| `rust/lovelace-lang/src/round67_modus.rs` | new: 22 tests mirroring the C# cases |
| `rust/lovelace-lang/src/lib.rs` | additive Engine fields/methods, bare-engine gate, host dispatch, scope install/restore, RecordValue messages |
| `rust/lovelace-lang/src/registry.rs` | `suggest_among` (capability-filtered suggestions) |
| `rust/lovelace-lang/src/builtins.rs` | `unknown_function_among` (uses the filtered suggestion list) |
| `rust/lovelace-lang/src/round66_resweep.rs` | formatting only |
