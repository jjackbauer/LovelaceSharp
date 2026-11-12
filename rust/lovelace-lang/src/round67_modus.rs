//! Round-67 Modus plugin-seam member map: the deferred census rows whose C# contract is the
//! plugin registry, the payload channel, load-time refusals, field-kernel dispatch, descriptor
//! arity or a plugin-instance assumption scope.
//!
//! Every test names the C# case it mirrors. The expected values are the C# assertions, not a
//! paraphrase, and every evaluation goes through the public `Engine` surface the runner uses.

use std::sync::{Arc, Mutex};

use lovelace_nat::Nat;
use lovelace_real::Real;
use lovelace_symbolics::assumptions::{Assumption, AssumptionSet, SymbolPredicate};

use crate::context::Context;
use crate::modus::{
    ArrayOp, BuiltinDescriptor, Field, FieldKernel, ModusHost, ModusPlugin, Payload, ScalarResult,
};
use crate::{Engine, LangError, Value};

fn real(text: &str) -> Real {
    Real::from_str(text).unwrap_or_else(|| panic!("'{text}' is not a Real literal"))
}

fn refusal(engine: &mut Engine, source: &str) -> LangError {
    match engine.eval(source) {
        Err(error) => error,
        Ok(value) => panic!("'{source}' answered {}", value.display()),
    }
}

fn text(engine: &mut Engine, source: &str) -> String {
    engine
        .eval(source)
        .unwrap_or_else(|error| panic!("'{source}' failed: {}", error.message))
        .display()
}

// ---------------------------------------------------------------------------
// ModusTests (6 deferred rows): registration, kernel dispatch, the ScalarResult
// return channel and the Dsp plugin lifecycle.
// ---------------------------------------------------------------------------

struct AddRealKernel;

impl FieldKernel<Real> for AddRealKernel {
    fn try_elementwise(
        &self,
        op: ArrayOp,
        left: &[Real],
        right: &[Real],
        result: &mut [Real],
        field: Field,
    ) -> bool {
        assert_eq!(field, Field::Real);
        if op != ArrayOp::Add {
            return false;
        }
        for (slot, (a, b)) in result.iter_mut().zip(left.iter().zip(right)) {
            *slot = a.add(b);
        }
        true
    }
}

struct StatisticsPlugin;

impl ModusPlugin for StatisticsPlugin {
    fn name(&self) -> &str {
        "Lovelace.Statistics"
    }

    fn register(&mut self, context: &mut ModusHost) -> Result<(), LangError> {
        context.register_kernel(Box::new(AddRealKernel));
        Ok(())
    }
}

/// C# ModusTests.cs:13 `PluginLoad_RegistersKernel_AndDispatchRuns`: loading a plugin that
/// registers a Real Add kernel makes dispatch answer and write the summed slots.
#[test]
fn plugin_load_registers_kernel_and_dispatch_runs() {
    let mut engine = Engine::new();
    engine.load_plugin(&mut StatisticsPlugin).expect("load");

    let left = vec![real("1"), real("2"), real("3")];
    let right = vec![real("4"), real("5"), real("6")];
    let mut result = vec![real("0"); 3];
    assert!(engine.try_dispatch_kernel(ArrayOp::Add, &left, &right, &mut result));
    assert_eq!(result[0], real("5"));
    assert_eq!(result[1], real("7"));
    assert_eq!(result[2], real("9"));
}

/// C# ModusTests.cs:31 `RealAddKernel_GivenRealOperands_MatchesDirectRealAddition`.
#[test]
fn real_add_kernel_matches_direct_real_addition() {
    let mut engine = Engine::new();
    engine.load_plugin(&mut StatisticsPlugin).expect("load");

    let a = real("1.5");
    let b = real("2.25");
    let mut result = vec![real("0")];
    assert!(engine.try_dispatch_kernel(
        ArrayOp::Add,
        std::slice::from_ref(&a),
        std::slice::from_ref(&b),
        &mut result
    ));
    assert_eq!(result[0], a.add(&b));
}

/// C# ModusTests.cs:49 `Dispatch_UnsupportedElementType_FallsBack`: a type with no exact field
/// identity is the reference backend's request, so dispatch reports false.
#[test]
fn dispatch_unsupported_element_type_falls_back() {
    let mut engine = Engine::new();
    engine.load_plugin(&mut StatisticsPlugin).expect("load");

    let left = [1i32, 2, 3];
    let right = [4i32, 5, 6];
    let mut result = [0i32; 3];
    assert!(!engine.try_dispatch_kernel(ArrayOp::Add, &left, &right, &mut result));
}

/// C# ModusTests.cs:64 `Kernel_DeclinesUnsupportedOp`: the Real kernel handles Add only, so a
/// Multiply request falls back to the reference backend.
#[test]
fn kernel_declines_unsupported_op() {
    let mut engine = Engine::new();
    engine.load_plugin(&mut StatisticsPlugin).expect("load");

    let left = vec![real("1"), real("2"), real("3")];
    let right = vec![real("4"), real("5"), real("6")];
    let mut result = vec![real("0"); 3];
    assert!(!engine.try_dispatch_kernel(ArrayOp::Multiply, &left, &right, &mut result));
}

struct ScalarResultPlugin;

impl ModusPlugin for ScalarResultPlugin {
    fn name(&self) -> &str {
        "ScalarResult"
    }

    fn register(&mut self, context: &mut ModusHost) -> Result<(), LangError> {
        context
            .register_builtin_scalar("flag", &["x"], |_| Ok(ScalarResult::from_boolean(true)))?;
        context
            .register_builtin_scalar("greet", &["x"], |_| Ok(ScalarResult::from_text("hello")))?;
        Ok(())
    }
}

/// C# ModusTests.cs:80 `ScalarResult_GivenBooleanAndText_ReturnValues`.
#[test]
fn scalar_result_boolean_and_text_return_values() {
    let mut engine = Engine::new();
    engine.load_plugin(&mut ScalarResultPlugin).expect("load");
    assert_eq!(engine.eval("flag(1)").unwrap().typed(), "True (Boolean)");
    assert_eq!(engine.eval("greet(1)").unwrap().typed(), "hello");
}

struct DspPlugin;

impl ModusPlugin for DspPlugin {
    fn name(&self) -> &str {
        "Lovelace.Dsp"
    }

    fn register(&mut self, _context: &mut ModusHost) -> Result<(), LangError> {
        // The Rust runtime ships the DSP kernels in the language crate; the plugin's descriptor
        // is the capability advertisement that makes them visible on a bare engine.
        Ok(())
    }
}

/// C# DspPluginTests.cs:17 `DspPlugin_GivenUnregisteredEngine_ThrowsUnknownFunction`.
#[test]
fn dsp_plugin_unregistered_engine_refuses_fft() {
    let mut engine = Engine::new_bare();
    let error = refusal(&mut engine, "fft([1, 0, 0, 0])");
    assert_eq!(error.code, "UnknownFunction");
    assert_eq!(error.message, "Unknown function 'fft'.");
    assert!(error.suggestions.is_empty());
}

/// C# ModusTests.cs:101 `DspPlugin_LoadedThroughModusSeam_RegistersDspBuiltins`: the load is
/// accepted, fft answers, and the function carries the plugin's own name while `abs` carries
/// none.
#[test]
fn dsp_plugin_loaded_through_modus_seam_registers_dsp_builtins() {
    let mut engine = Engine::new_bare();
    assert_eq!(DspPlugin.name(), "Lovelace.Dsp");
    engine.load_plugin(&mut DspPlugin).expect("load");

    assert_eq!(
        engine.eval("fft([1, 0, 0, 0])").unwrap().typed(),
        "[1, 1, 1, 1] (Vector)"
    );
    assert_eq!(engine.plugin_name_of("fft"), Some("Lovelace.Dsp"));
    assert_eq!(engine.plugin_name_of("abs"), None);
}

/// C# DspPluginTests.cs:255 `DspPlugin_LoadedTwice_Throws`.
#[test]
fn dsp_plugin_loaded_twice_is_refused() {
    let mut engine = Engine::new();
    engine.load_plugin(&mut DspPlugin).expect("first load");
    let error = engine.load_plugin(&mut DspPlugin).expect_err("second load");
    assert!(
        error.message.contains("already loaded"),
        "{}",
        error.message
    );
}

/// C# DspPluginTests.cs:266 `PluginBuiltin_GivenCoreNameCollision_Throws`.
#[test]
fn plugin_builtin_core_name_collision_is_refused() {
    struct ShadowingPlugin;

    impl ModusPlugin for ShadowingPlugin {
        fn name(&self) -> &str {
            "Shadowing"
        }

        fn register(&mut self, context: &mut ModusHost) -> Result<(), LangError> {
            context.register_builtin("abs", &["x"], |_| Ok(Payload::Void))
        }
    }

    let mut engine = Engine::new();
    let error = engine
        .load_plugin(&mut ShadowingPlugin)
        .expect_err("shadowing abs");
    assert!(
        error.message.contains("already registered"),
        "{}",
        error.message
    );
}

// ---------------------------------------------------------------------------
// ModusArityTests (4 deferred rows): the descriptor is the one arity source of
// truth for exact, optional-tail, variadic and parameters-only registrations.
// ---------------------------------------------------------------------------

struct ArityPlugin;

impl ModusPlugin for ArityPlugin {
    fn name(&self) -> &str {
        "ArityProbe"
    }

    fn register(&mut self, context: &mut ModusHost) -> Result<(), LangError> {
        context.register_builtin_descriptor(
            BuiltinDescriptor::new(
                "pair",
                &["a", "b"],
                "Core",
                "Exactly two arguments.",
                "Text",
            ),
            |args| Ok(Payload::Text(format!("pair:{}", args.len()))),
        )?;
        context.register_builtin_descriptor(
            BuiltinDescriptor::new(
                "opt",
                &["a", "b"],
                "Core",
                "An optional trailing b.",
                "Text",
            )
            .with_min_arity(1),
            |args| Ok(Payload::Text(format!("opt:{}", args.len()))),
        )?;
        context.register_builtin_descriptor_scalar(
            BuiltinDescriptor::new(
                "variadic_min",
                &["a", "b"],
                "Core",
                "At least one argument; the tail repeats.",
                "Text",
            )
            .with_variadic(true)
            .with_min_arity(1),
            |args| {
                Ok(ScalarResult::from_text(format!(
                    "variadic_min:{}",
                    args.len()
                )))
            },
        )?;
        context.register_builtin_descriptor(
            BuiltinDescriptor::new(
                "variadic_two",
                &["a", "b"],
                "Core",
                "At least two arguments; the tail repeats.",
                "Text",
            )
            .with_variadic(true),
            |args| Ok(Payload::Text(format!("variadic_two:{}", args.len()))),
        )?;
        context.register_builtin("plain", &["a", "b"], |args| {
            Ok(Payload::Text(format!("plain:{}", args.len())))
        })?;
        Ok(())
    }
}

fn arity_engine() -> Engine {
    let mut engine = Engine::new();
    engine.load_plugin(&mut ArityPlugin).expect("load");
    engine
}

/// C# ModusArityTests.cs:55 `ExactCount_RequiresTheDeclaredCount`.
#[test]
fn exact_count_requires_the_declared_count() {
    let mut engine = arity_engine();
    assert_eq!(text(&mut engine, "pair(1, 2)"), "pair:2");

    let too_few = refusal(&mut engine, "pair(1)");
    assert_eq!(too_few.message, "pair(): expected 2 arguments; got 1.");
    assert_eq!(too_few.code, "InvalidArgument");
    assert_eq!(too_few.category, "TypeMismatch");
    assert_eq!(
        refusal(&mut engine, "pair(1, 2, 3)").message,
        "pair(): expected 2 arguments; got 3."
    );
}

/// C# ModusArityTests.cs:70 `ParametersOnlyRegistration_UsesTheSameRule`.
#[test]
fn parameters_only_registration_uses_the_same_rule() {
    let mut engine = arity_engine();
    assert_eq!(text(&mut engine, "plain(1, 2)"), "plain:2");
    assert_eq!(
        refusal(&mut engine, "plain(1)").message,
        "plain(): expected 2 arguments; got 1."
    );
}

/// C# ModusArityTests.cs:79 `MinArity_AllowsTheOptionalTail_AndStillRefusesLess`.
#[test]
fn min_arity_allows_the_optional_tail_and_still_refuses_less() {
    let mut engine = arity_engine();
    assert_eq!(text(&mut engine, "opt(1)"), "opt:1");
    assert_eq!(text(&mut engine, "opt(1, 2)"), "opt:2");
    assert_eq!(
        refusal(&mut engine, "opt()").message,
        "opt(): expected 1 to 2 arguments; got 0."
    );
}

/// C# ModusArityTests.cs:89 `Variadic_RemovesTheUpperBound_AndHonoursTheLowerOne`.
#[test]
fn variadic_removes_the_upper_bound_and_honours_the_lower_one() {
    let mut engine = arity_engine();
    assert_eq!(text(&mut engine, "variadic_min(1)"), "variadic_min:1");
    assert_eq!(
        text(&mut engine, "variadic_min(1, 2, 3, 4)"),
        "variadic_min:4"
    );
    assert_eq!(
        refusal(&mut engine, "variadic_min()").message,
        "variadic_min(): expected at least 1 argument; got 0."
    );

    assert_eq!(text(&mut engine, "variadic_two(1, 2)"), "variadic_two:2");
    assert_eq!(
        text(&mut engine, "variadic_two(1, 2, 3, 4, 5)"),
        "variadic_two:5"
    );
    assert_eq!(
        refusal(&mut engine, "variadic_two(1)").message,
        "variadic_two(): expected at least 2 arguments; got 1."
    );
}

// ---------------------------------------------------------------------------
// BuiltinArgumentShapeGuardTests (3 deferred rows): the ONE call-site guard
// attributes a coercion to the argument and an engine-internal value to the
// internal failure.
// ---------------------------------------------------------------------------

/// C# BuiltinArgumentShapeGuardTests.cs:33
/// `ARegisteredBuiltinThatCoercesItsArgument_IsAttributedToTheBuiltinAndThePosition`.
#[test]
fn registered_builtin_that_coerces_its_argument_is_attributed_to_the_position() {
    let mut engine = Engine::new();
    engine
        .register_builtin("probe", &["a", "b"], |args| {
            let array = args.get(1).expect("argument 2").as_array()?;
            Ok(Payload::Text(array.elements.len().to_string()))
        })
        .expect("register");

    let error = refusal(&mut engine, "probe(1, 2)");
    assert_eq!(
        error.message,
        "probe(): argument 2 must be an array or vector; got Natural."
    );
    assert!(!error.message.contains("cast"));
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
}

/// C# BuiltinArgumentShapeGuardTests.cs:49
/// `ARegisteredBuiltinThatCastsItsOwnPayload_IsAttributedToTheArgumentItReadLast`.
#[test]
fn registered_builtin_that_casts_its_payload_is_attributed_to_the_last_argument() {
    let mut engine = Engine::new();
    engine
        .register_builtin("probe", &["a", "b"], |args| {
            let text = args.get(1).expect("argument 2").cast_text()?;
            Ok(Payload::Text(text.to_string()))
        })
        .expect("register");

    let error = refusal(&mut engine, "probe(1, 2)");
    assert_eq!(
        error.message,
        "probe(): argument 2 must be a value this builtin can use; got Natural."
    );
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
}

/// C# BuiltinArgumentShapeGuardTests.cs:67
/// `ACoercionFailureOnAnEngineInternalValue_IsNotDisguisedAsAnArgumentError`.
#[test]
fn coercion_failure_on_an_engine_internal_value_stays_internal() {
    let mut engine = Engine::new();
    engine
        .register_builtin("bug", &["a", "b"], |args| {
            let _read = args.get(0).expect("argument 1");
            let internal = Payload::Void;
            let _array = internal.as_array()?;
            Ok(Payload::Text("unreachable".to_string()))
        })
        .expect("register");

    let error = refusal(&mut engine, "bug(1, 2)");
    assert_eq!(error.code, "InternalError");
    assert_eq!(error.category, "InternalInvariantFailure");
    assert!(!error.message.contains("argument"), "{}", error.message);
}

// ---------------------------------------------------------------------------
// EnumPayloadSeamTests (1 deferred row) and RecordValueTests (3 deferred rows).
// ---------------------------------------------------------------------------

struct EnumEchoPlugin {
    last: Arc<Mutex<Option<Payload>>>,
}

impl ModusPlugin for EnumEchoPlugin {
    fn name(&self) -> &str {
        "EnumEcho"
    }

    fn register(&mut self, context: &mut ModusHost) -> Result<(), LangError> {
        let last = Arc::clone(&self.last);
        context.register_builtin("enum_echo", &["v"], move |args| {
            let payload = args.get(0).expect("argument 1").payload().clone();
            *last.lock().expect("echo slot") = Some(payload.clone());
            Ok(payload)
        })
    }
}

/// C# EnumPayloadSeamTests.cs:42 `EnumValue_RoundTripsThroughTheModusSeam`: the argument crosses
/// as an enum payload (both components) and the echoed payload is re-wrapped as an enum value.
#[test]
fn enum_value_round_trips_through_the_modus_seam() {
    let mut engine = Engine::new();
    let last = Arc::new(Mutex::new(None));
    engine
        .load_plugin(&mut EnumEchoPlugin {
            last: Arc::clone(&last),
        })
        .expect("load");

    let returned = engine
        .eval(r#"x = symbol("x"); enum_echo(solve_full(x^2 - 4 == 0, x).status)"#)
        .expect("enum echo");
    let payload = last.lock().expect("echo slot").clone().expect("payload");
    let (type_name, member) = payload.as_enum().expect("enum payload");
    assert_eq!(type_name, "SolveStatus");
    assert_eq!(member, "Solved");
    assert_eq!(returned.kind_name(), "Enum");
    assert_eq!(returned.display(), "Solved");
    assert_eq!(returned.typed(), "Solved");
}

struct RecordPlugin;

impl ModusPlugin for RecordPlugin {
    fn name(&self) -> &str {
        "RecordTestPlugin"
    }

    fn register(&mut self, context: &mut ModusHost) -> Result<(), LangError> {
        context.register_builtin("plug", &[], |_| {
            Ok(Payload::Record(
                "PluginResult".to_string(),
                vec![("answer".to_string(), Payload::Natural(Nat::from_u64(5)))],
            ))
        })
    }
}

/// C# RecordValueTests.cs:75 `Record_CrossesModusPluginBoundary`: a plugin-produced record keeps
/// its type name and its field is readable through member access.
#[test]
fn record_crosses_the_modus_plugin_boundary() {
    let mut engine = Engine::new();
    engine.load_plugin(&mut RecordPlugin).expect("load");
    let result = engine.eval("plug()").expect("plug");
    match &result {
        Value::Record(type_name, fields) => {
            assert_eq!(type_name, "PluginResult");
            assert_eq!(fields.len(), 1);
        }
        other => panic!("expected a record, got {}", other.kind_name()),
    }
    assert_eq!(engine.eval("plug().answer").unwrap().display(), "5");
}

/// The reference's host builtin channel (`SuiteEngine.RegisterBuiltin`) is the same registry, so
/// a record producer registered directly behaves like a plugin producer. Shared setup for the
/// two member-access rows below (C# RecordValueTests.cs:44 and :55).
fn maker_engine() -> Engine {
    let mut engine = Engine::new();
    engine
        .register_builtin("maker", &["n"], |args| {
            let count = args.get(0).expect("argument 1").as_natural()?.clone();
            Ok(Payload::Record(
                "DemoResult".to_string(),
                vec![
                    ("count".to_string(), Payload::Natural(count)),
                    ("label".to_string(), Payload::Text("hello".to_string())),
                    (
                        "nested".to_string(),
                        Payload::Record(
                            "Inner".to_string(),
                            vec![("flag".to_string(), Payload::Boolean(true))],
                        ),
                    ),
                ],
            ))
        })
        .expect("register maker");
    engine
}

/// C# RecordValueTests.cs:44 `MemberAccess_UnknownField_NamesAvailableMembers`.
#[test]
fn member_access_unknown_field_names_available_members() {
    let mut engine = maker_engine();
    engine.eval("r = maker(3)").expect("maker");
    let error = refusal(&mut engine, "r.nope");
    assert_eq!(
        error.message,
        "record 'DemoResult' has no member 'nope'. Available members: count, label, nested."
    );
}

/// C# RecordValueTests.cs:55 `MemberAccess_OnNonRecord_ExplainsRequirement`.
#[test]
fn member_access_on_a_non_record_explains_the_requirement() {
    let mut engine = maker_engine();
    engine.eval("v = [1, 2, 3]").expect("vector");
    let error = refusal(&mut engine, "v.length");
    assert!(
        error
            .message
            .contains("member access requires a record result"),
        "{}",
        error.message
    );
}

// ---------------------------------------------------------------------------
// SharedPluginInstanceAssumptionIsolationTests (2 deferred rows): a plugin-instance
// caller scope outranks the engine inside the evaluation, and a committed
// assumption on one engine never decides another.
// ---------------------------------------------------------------------------

struct SymbolicsContextPlugin {
    context: Arc<Context>,
}

impl ModusPlugin for SymbolicsContextPlugin {
    fn name(&self) -> &str {
        "Lovelace.Symbolics"
    }

    fn register(&mut self, host: &mut ModusHost) -> Result<(), LangError> {
        host.register_assumption_context(Arc::clone(&self.context));
        Ok(())
    }
}

/// C# SharedPluginInstanceAssumptionIsolationTests.cs:115
/// `CallerInstalledAssumptionScope_KeepsPrecedenceInsideAnEngineEvaluation`.
#[test]
fn caller_installed_assumption_scope_keeps_precedence_inside_an_engine_evaluation() {
    let context = Arc::new(Context::new());
    let mut engine = Engine::new();
    engine
        .load_plugin(&mut SymbolicsContextPlugin {
            context: Arc::clone(&context),
        })
        .expect("load");
    engine.eval(r#"x = symbol("x")"#).expect("symbol");

    let scoped = AssumptionSet::empty()
        .add(Assumption::SymbolProperty(
            "x".to_string(),
            SymbolPredicate::Positive,
        ))
        .expect("consistent scope");
    {
        let _scope = context.with_assumptions(scoped);
        assert_eq!(
            engine.eval("simplify(x > 0)").expect("scoped simplify"),
            Value::Boolean(true)
        );
    }
    // the scope is gone: the engine's own (empty) session is what answers again
    assert_eq!(
        engine
            .eval("simplify(x > 0)")
            .expect("unscoped simplify")
            .display(),
        "x > 0"
    );
}

/// C# SharedPluginInstanceAssumptionIsolationTests.cs:75
/// `OnePluginTwoEngines_AssumptionCommittedWhileAIsRunning_DoesNotDecideB`: A's assumption is
/// committed before `hold_gate` runs, and B asks from inside that builtin — B answers for B.
#[test]
fn assumption_committed_while_a_is_running_does_not_decide_b() {
    let context = Arc::new(Context::new());
    let mut a = Engine::new();
    a.load_plugin(&mut SymbolicsContextPlugin {
        context: Arc::clone(&context),
    })
    .expect("load a");
    let mut b_engine = Engine::new();
    b_engine
        .load_plugin(&mut SymbolicsContextPlugin {
            context: Arc::clone(&context),
        })
        .expect("load b");
    let b = Arc::new(Mutex::new(b_engine));

    let b_gate = Arc::clone(&b);
    a.register_builtin("hold_gate", &[], move |_| {
        let mut b = b_gate.lock().expect("engine b");
        let answer = b
            .eval(r#"x = symbol("x"); simplify(x > 0)"#)
            .expect("b simplify");
        assert_eq!(answer.display(), "x > 0");
        Ok(Payload::Boolean(true))
    })
    .expect("register gate");

    a.eval(r#"x = symbol("x"); assume(x > 0); hold_gate()"#)
        .expect("a with gate");
    assert_eq!(a.eval("simplify(x > 0)").unwrap().display(), "True");
}
