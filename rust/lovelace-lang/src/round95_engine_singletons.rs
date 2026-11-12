//! Round 95 — the language/engine singleton rows.
//!
//! Eleven deferred census rows name a class each; this module carries the LANGUAGE-tier halves and
//! the runner sibling (`lovelace-run/src/round95_singletons.rs`) carries the wire halves. Every
//! assertion below is the oracle-observed behaviour, measured on the pinned C# runner:
//!
//! * `Lovelace.Suite.Tests.BroadcastTests/ColumnVector_BroadcastsToMatrix#0` — `[[1],[2]] +
//!   [[10,20],[30,40]]` is `[[11, 21], [32, 42]]`; the two shapes broadcast right-aligned.
//! * `Lovelace.Suite.Tests.FunctionTests/Evaluate_GivenFunctionLocalVariable_DoesNotLeakToGlobal#0`
//!   — a function-local definition is gone after the call.
//! * `Lovelace.Suite.Tests.StatementTests/Evaluate_GivenVariableAssignmentInsideBlock_DoesNotLeak#0`
//!   — a braced block's definitions do not escape it, while an assignment to an existing outer
//!   binding still updates that binding.
//! * `Lovelace.Suite.Tests.RecursionDepthGuardTests/ADeepExpressionInsideTheRecursiveBody_IsRefused#0`
//!   — a 100-term body chain at ten call levels is a typed depth refusal.
//! * `Lovelace.Run.Tests.RecursionDepthEnvelopeTests/NestedBlocksInsideARecursiveBody_AreRefused_InsteadOfDying#0`
//!   — 120 nested `if`s walked at every one of five call levels is a typed depth refusal.
//! * `Lovelace.Run.Tests.ArityPropertyTests/DeclaredArity_TravelsWithEveryPluginBuiltin#0` — the
//!   descriptor's `MinArity`/`Variadic` travel with the function definition.
//! * `Lovelace.Suite.Tests.DspIntegerArgumentContractTests/TheExactlyIntegralRealRefusal_IsStillTheContractsContract#0`
//!   — the contract's wrong-kind refusal names the integer to write and stays
//!   `InvalidArgument`/`TypeMismatch`.
//! * `Lovelace.Run.Tests.LibraryProjectionAgreementTests/TheFacade_PublishesEveryStoredDigit_WhenTheDisplayBoundIsLower#0`
//!   — the structured projection publishes the digits the value STORES, whatever the display bound.
//!
//! `Lovelace.Representation.Tests/UnitTest1/Test1#0` has no counterpart here ON PURPOSE: its C#
//! body is empty, so it asserts nothing to port (see the round-95 report).

use lovelace_nat::Nat;

use crate::structured;
use crate::{Engine, Value};

/// Runs a probe on a thread with a generous stack, like the other depth probes: the interpreter's
/// walk is native recursion, and the test harness's default thread is too small to hold the
/// 120-deep nest the oracle measures (the refusal is not about the Rust stack size).
fn with_big<T: Send + 'static>(probe: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(probe)
        .expect("probe thread")
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}

/// The published evaluation refusal: `DepthExceeded`/`BudgetExceeded`, naming the budget and a
/// measured depth past it.
fn assert_depth_refusal(error: &crate::LangError, shape: &str) {
    assert_eq!(error.code, "DepthExceeded", "{shape}: {}", error.message);
    assert_eq!(
        error.category, "BudgetExceeded",
        "{shape}: {}",
        error.message
    );
    assert!(
        error
            .message
            .contains("exceeds the maximum supported evaluation depth of 512"),
        "{shape}: {}",
        error.message
    );
    let at = error
        .message
        .find("evaluation depth ")
        .unwrap_or_else(|| panic!("{shape}: names the evaluation depth: {}", error.message))
        + "evaluation depth ".len();
    let measured: usize = error.message[at..]
        .split(' ')
        .next()
        .and_then(|digits| digits.parse().ok())
        .unwrap_or_else(|| panic!("{shape}: a measured depth number: {}", error.message));
    assert!(
        measured > 512,
        "{shape}: the measured depth must be past the budget, was {measured}"
    );
}

fn decimals(rendering: &str) -> usize {
    let bare = rendering.split(' ').next().unwrap_or(rendering);
    match bare.find('.') {
        Some(dot) => bare.len() - dot - 1,
        None => 0,
    }
}

// ---------------------------------------------------------------------------
// Broadcast
// ---------------------------------------------------------------------------

/// `BroadcastTests.ColumnVector_BroadcastsToMatrix`: the audit's exact expression. The row was
/// deferred because Rust refused the [2, 1] operand against the [2, 2] one; the reference
/// broadcasts them right-aligned.
#[test]
fn broadcasttests_column_vector_broadcasts_to_matrix() {
    let mut engine = Engine::new();

    assert_eq!(
        engine
            .eval("[[1], [2]] + [[10, 20], [30, 40]]")
            .expect("the column vector broadcasts")
            .typed(),
        "[[11, 21], [32, 42]] (Array)"
    );

    // The already-ported siblings keep answering, so the fix widened the rule rather than
    // replacing it.
    assert_eq!(
        engine
            .eval("[1, 2] + [[1, 2], [3, 4]]")
            .expect("the row vector broadcasts")
            .typed(),
        "[[2, 4], [4, 6]] (Array)"
    );
    assert_eq!(
        engine
            .eval("[1, 2, 3] * 10")
            .expect("the scalar broadcasts")
            .typed(),
        "[10, 20, 30] (Vector)"
    );
}

/// The refusal the rule still owes: shapes whose axes disagree at neither 1 cannot broadcast.
#[test]
fn broadcasttests_incompatible_shapes_are_still_refused() {
    let mut engine = Engine::new();

    let error = engine
        .eval("[1, 2] + [1, 2, 3]")
        .expect_err("incompatible shapes are refused");

    assert_eq!(error.code, "InvalidOperation");
    assert!(
        error
            .message
            .contains("Operands could not be broadcast together with shapes [2] and [3]."),
        "{}",
        error.message
    );
}

// ---------------------------------------------------------------------------
// Scoping
// ---------------------------------------------------------------------------

/// `FunctionTests.Evaluate_GivenFunctionLocalVariable_DoesNotLeakToGlobal`: the function frame is
/// a scope of its own, so `local` is not a global variable after `f()`.
#[test]
fn functiontests_function_local_variable_does_not_leak_to_global() {
    let mut engine = Engine::new();

    engine.eval("func f() { local = 42 }").expect("definition");
    assert_eq!(engine.eval("f()").expect("call").display(), "42");

    assert!(
        engine.try_get_variable("local").is_none(),
        "a function-local definition is not a global: {:?}",
        engine.variables()
    );
    let error = engine.eval("local").expect_err("local is not visible");
    assert_eq!(error.message, "Undefined variable 'local'.");
}

/// `StatementTests.Evaluate_GivenVariableAssignmentInsideBlock_DoesNotLeak`: the block's own
/// definition is dropped with the block, while an assignment to an EXISTING outer binding updates
/// that binding (the reference's `Scope.Assign`).
#[test]
fn statementtests_block_assignment_does_not_leak() {
    let mut engine = Engine::new();

    assert_eq!(engine.eval("{ z = 99 }").expect("block").display(), "99");
    assert!(engine.try_get_variable("z").is_none());
    assert!(
        engine.eval("z").is_err(),
        "z is not visible after the block"
    );

    engine.eval("x = 10").expect("global");
    assert_eq!(
        engine.eval("{ x = 20 }; x").expect("update").display(),
        "20"
    );
    assert_eq!(
        engine
            .try_get_variable("x")
            .expect("x stays global")
            .display(),
        "20"
    );

    // The oracle DEFINES a top-level loop variable in the global scope (`for i in 1..3 {}; i`
    // answers 3), so the loop's name is published like any other global definition.
    engine.eval("for i in 1..3 { }").expect("loop");
    assert_eq!(
        engine.try_get_variable("i").expect("i is global").display(),
        "3"
    );
}

// ---------------------------------------------------------------------------
// Recursion depth
// ---------------------------------------------------------------------------

/// `RecursionDepthGuardTests.ADeepExpressionInsideTheRecursiveBody_IsRefused`: ten call levels
/// are fine on their own, but every level walks a 100-term expression chain, so the interpreter's
/// real depth is ten times that and the budget refuses it (it used to answer).
#[test]
fn recursiondepthguard_a_deep_expression_inside_the_recursive_body_is_refused() {
    with_big(|| {
        let mut engine = Engine::new();

        let deep = format!(
            "func f(n) {{ if (n == 0) {{ return 0 }}; return f(n - 1){} }}; f(10)",
            " + 1".repeat(100)
        );
        let error = engine.eval(&deep).expect_err("the deep body is refused");
        assert_depth_refusal(&error, "f(10) with a 100-term body chain");

        // The same body at four levels is inside the budget: the refusal is DEPTH, not the shape.
        let shallow = format!(
            "func g(n) {{ if (n == 0) {{ return 0 }}; return g(n - 1){} }}; g(4)",
            " + 1".repeat(100)
        );
        assert_eq!(
            engine.eval(&shallow).expect("g(4) answers").display(),
            "400"
        );
    });
}

/// `RecursionDepthEnvelopeTests.NestedBlocksInsideARecursiveBody_AreRefused_InsteadOfDying`:
/// five call levels with 120 nested `if`s around the recursive call. A call-level cap admits the
/// five levels; the interpreter's depth is 120 per level, so the budget must refuse it instead of
/// the process dying.
#[test]
fn recursiondepthenvelope_nested_blocks_inside_a_recursive_body_are_refused() {
    with_big(|| {
        let mut engine = Engine::new();

        let nested = format!(
            "func f(n) {{ if (n == 0) {{ return 0 }}; {}return f(n - 1){} }}; f(5)",
            "if (1 == 1) { ".repeat(120),
            " }".repeat(120)
        );
        let error = engine
            .eval(&nested)
            .expect_err("the nested body is refused");
        assert_depth_refusal(&error, "120 nested ifs at 5 call levels");

        // One call level of the same 120-deep nest is admitted: the nesting alone is inside the
        // budget, and it is the recursion that multiplies it past the bound.
        let one_level = format!(
            "func h(n) {{ if (n == 0) {{ return 0 }}; {}return h(n - 1){} }}; h(1)",
            "if (1 == 1) { ".repeat(120),
            " }".repeat(120)
        );
        assert_eq!(
            engine.eval(&one_level).expect("h(1) answers").display(),
            "0"
        );
    });
}

// ---------------------------------------------------------------------------
// Declared arity
// ---------------------------------------------------------------------------

/// `ArityPropertyTests.DeclaredArity_TravelsWithEveryPluginBuiltin`: the registry descriptor's
/// `MinArity` and `Variadic` are carried by the function definition itself, so the one call-site
/// validator can enforce what the descriptor declares.
#[test]
fn arityproperty_declared_arity_travels_with_every_plugin_builtin() {
    let engine = Engine::new();
    let snapshot = engine.capture_state();
    let mut checked = 0;

    for (name, function) in &snapshot.functions {
        if !function.is_builtin {
            continue;
        }
        let declared = crate::registry::find(name)
            .unwrap_or_else(|| panic!("{name}: a registered builtin has a descriptor"));
        if declared.plugin.is_none() {
            continue;
        }
        checked += 1;
        assert_eq!(
            function.min_arity, declared.min_arity,
            "{name}: the descriptor's declared MinArity ({}) is not carried by the function",
            declared.min_arity
        );
        assert_eq!(
            function.variadic, declared.variadic,
            "{name}: the descriptor's declared Variadic ({}) is not carried by the function",
            declared.variadic
        );
        assert_eq!(
            function.parameters.len(),
            declared.parameter_count,
            "{name}: the declared parameter count travels too"
        );
    }

    assert!(
        checked >= 40,
        "expected the plugin descriptors to cover the registry, saw {checked}"
    );
}

/// The same declaration travels through a HOST registration: a descriptor registered with a
/// shorter minimum and a variadic tail is what the definition publishes.
#[test]
fn arityproperty_a_host_descriptor_carries_its_declaration() {
    let mut engine = Engine::new();
    let descriptor = crate::modus::BuiltinDescriptor::new(
        "host_shape",
        &["a", "rest"],
        "Tests",
        "A host builtin whose declaration is not its parameter count.",
        "Natural",
    )
    .with_min_arity(1)
    .with_variadic(true);
    engine
        .register_builtin_descriptor(descriptor, |_args| {
            Ok(crate::modus::Payload::Natural(Nat::from_u64(1)))
        })
        .expect("the host registration succeeds");

    let snapshot = engine.capture_state();
    let function = &snapshot.functions["host_shape"];
    assert_eq!(function.min_arity, 1);
    assert!(function.variadic);
    assert_eq!(
        function.parameters,
        vec!["a".to_string(), "rest".to_string()]
    );
}

// ---------------------------------------------------------------------------
// The exact-integer contract
// ---------------------------------------------------------------------------

/// `DspIntegerArgumentContractTests.TheExactlyIntegralRealRefusal_IsStillTheContractsContract`:
/// the contract's wrong-KIND refusal is a published library shape that names the integer to
/// write, and the engine's own call sites produce it when they do not normalise the payload.
#[test]
fn dspintegerargumentcontract_the_exactly_integral_real_refusal_is_the_contracts_contract() {
    let mut engine = Engine::new();

    // The measured boundary normalises `3.0` to the integer it denotes, exactly as the C# row
    // documents; the contract is therefore pinned where it is reachable.
    assert_eq!(
        engine
            .eval("movingavg([1, 2, 3, 4, 5], 3.0)")
            .expect("an integral Real count is the integer it denotes")
            .typed(),
        engine
            .eval("movingavg([1, 2, 3, 4, 5], 3)")
            .expect("the integer form")
            .typed()
    );

    // A call site that pins the kind refuses through the contract: the recoverable argument pair,
    // naming the value to write instead. The contract constructor itself is pinned by
    // `argument_contract::tests`; this is the same shape observed through the engine.
    let error = engine
        .eval("eye(2.0)")
        .expect_err("an integral Real is the wrong KIND for a pinned dimension");
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
    assert!(
        error.message.contains("write it as 2"),
        "the refusal names the integer to write: {}",
        error.message
    );
}

// ---------------------------------------------------------------------------
// Library projection agreement
// ---------------------------------------------------------------------------

/// `LibraryProjectionAgreementTests.TheFacade_PublishesEveryStoredDigit_WhenTheDisplayBoundIsLower`:
/// the audit's shape — computation 1 100, display 100 — through the library façade. The structured
/// value carries the 1 100 digits the value stores; the human rendering stays at the display bound,
/// and the truncation fields stay absent because nothing was cut.
#[test]
fn libraryprojectionagreement_the_facade_publishes_every_stored_digit() {
    const PI_FIRST_60: &str = "3.141592653589793238462643383279502884197169399375105820974944";
    const PI_1081_TO_1100: &str = "24972177528347913151";

    let mut engine = Engine::new();
    engine.set_precision(1100);
    engine.set_display_precision(100);

    let value = engine.eval("pi(1100)").expect("pi(1100)");
    let dto = structured::project_value(&engine, &value, None);
    let rendered = dto
        .get("value")
        .and_then(lovelace_json::Json::as_str)
        .expect("a Real publishes its value");

    assert_eq!(decimals(rendered), 1100, "every stored digit crosses");
    assert!(
        rendered.starts_with(PI_FIRST_60),
        "the first 60 digits: {rendered}"
    );
    assert!(
        rendered.ends_with(PI_1081_TO_1100),
        "the tail only a 1 100-place request has: {rendered}"
    );
    // nothing was cut, so nothing may be marked as cut
    assert!(dto.get("truncated").is_none());
    assert!(dto.get("truncationReason").is_none());
    assert!(dto.get("budget").is_none());

    // and the DISPLAY bound stays a display bound: the fix moved the payload, not the display
    assert_eq!(decimals(&engine.format(&value)), 100);

    // a Real nested in a container crosses in full too — the room belongs to the projection
    let nested = engine.eval("[pi(1100), e(1100)]").expect("nested");
    let elements = structured::project_value(&engine, &nested, None);
    let Some(lovelace_json::Json::Arr(items)) = elements.get("elements") else {
        panic!("the array publishes its elements");
    };
    for item in items {
        let text = item
            .get("value")
            .and_then(lovelace_json::Json::as_str)
            .expect("each element publishes its value");
        assert_eq!(decimals(text), 1100);
    }
}

/// The complement: an exact Real is still published as its rational with no truncation fields, and
/// a value that stores fewer digits than the display bound is neither cut nor padded.
#[test]
fn libraryprojectionagreement_untruncated_values_are_neither_cut_nor_padded() {
    let mut engine = Engine::new();
    engine.set_precision(1100);
    engine.set_display_precision(100);

    let eighth = engine.eval("1/8").expect("1/8");
    let exact = structured::project_value(&engine, &eighth, None);
    assert_eq!(
        exact.get("value").and_then(lovelace_json::Json::as_str),
        Some("0.125")
    );
    assert!(matches!(
        exact.get("exact"),
        Some(lovelace_json::Json::Bool(true))
    ));
    assert!(exact.get("truncated").is_none());

    let pi30 = engine.eval("pi(30)").expect("pi(30)");
    let short = structured::project_value(&engine, &pi30, None);
    let rendered = short
        .get("value")
        .and_then(lovelace_json::Json::as_str)
        .expect("a Real publishes its value");
    assert_eq!(decimals(rendered), 30);
}

/// A value the ENGINE clamped still says so: `evalf`'s documented 1 000-place cap crosses with
/// the digits the value really has and the truncation fields naming the cap.
#[test]
fn libraryprojectionagreement_a_clamped_value_is_marked() {
    let mut engine = Engine::new();
    engine.set_precision(5000);
    engine.set_display_precision(100);

    let value = engine.eval("evalf(sqrt(2), 5000)").expect("evalf");
    let dto = structured::project_value(&engine, &value, None);
    let rendered = dto
        .get("value")
        .and_then(lovelace_json::Json::as_str)
        .expect("a Real publishes its value");

    assert_eq!(decimals(rendered), 1000);
    assert!(matches!(
        dto.get("truncated"),
        Some(lovelace_json::Json::Bool(true))
    ));
    assert_eq!(
        dto.get("truncationReason")
            .and_then(lovelace_json::Json::as_str),
        Some("digit-cap")
    );
    assert_eq!(
        dto.get("budget"),
        Some(&lovelace_json::Json::Num("1000".to_string()))
    );
}

/// Keeps the `Value` import live and pins the published kind name the projection reads.
#[test]
fn value_kind_is_the_published_name() {
    let value: Value = Value::Natural(Nat::from_u64(1));
    assert_eq!(value.kind_name(), "Natural");
}
