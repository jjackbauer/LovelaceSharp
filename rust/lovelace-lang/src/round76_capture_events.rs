//! Round 76 — the `SuiteEngine` capture/event surface.
//!
//! The four deferred `Lovelace.Suite.Tests.SuiteEngineTests` rows in the
//! `SuiteEngine CaptureState, events and SetVariable/RemoveVariable API` block:
//!
//! * `CaptureState_IsImmutable#0`
//! * `VariableChanged_GivenAssignment_RaisesEvent#0`
//! * `FunctionDefined_GivenDefinition_RaisesEvent#0`
//! * `SetVariableAndRemoveVariable_Work#0`
//!
//! Each test drives the same operations as its C# row and asserts the same observable: an
//! immutable snapshot, the event name/order/payload, and the set/try-get/remove success and
//! refusal branches. `capture_state_records_variables_and_user_functions` is supporting
//! evidence for the snapshot fields the already-ported sibling row
//! (`CaptureState_GivenVariablesAndFunctions_ReturnsMatchingSnapshot#0`, round 50) exercises.

use std::sync::{Arc, Mutex};

use lovelace_nat::Nat;

use crate::{Engine, Value};

fn natural(n: u64) -> Value {
    Value::Natural(Nat::from_u64(n))
}

/// `SuiteEngineTests.cs:71 CaptureState_IsImmutable`: a snapshot taken before `SetVariable`
/// must keep its own variable count and must not grow the later name.
#[test]
fn capture_state_is_immutable() {
    let mut engine = Engine::new();
    let snapshot = engine.capture_state();
    let variable_count = snapshot.variables.len();

    engine.set_variable("x", natural(1));

    assert_eq!(snapshot.variables.len(), variable_count);
    assert!(!snapshot.variables.contains_key("x"));
}

/// `SuiteEngineTests.cs:84 VariableChanged_GivenAssignment_RaisesEvent`: evaluating `x = 7`
/// reports the name `x` to a listener. The result variable `_` is a second change, as in the
/// reference, which stores it through `SetVariable`.
#[test]
fn variable_changed_given_assignment_raises_event() {
    let mut engine = Engine::new();
    let names = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&names);
    engine.on_variable_changed(move |event| {
        seen.lock().expect("event log").push(event.name.clone());
    });

    engine.eval("x = 7").expect("assignment evaluates");

    assert!(
        names
            .lock()
            .expect("event log")
            .iter()
            .any(|name| name == "x")
    );
}

/// `SuiteEngineTests.cs:96 FunctionDefined_GivenDefinition_RaisesEvent`: `func g() { 1 }`
/// reports one definition whose name is `g`.
#[test]
fn function_defined_given_definition_raises_event() {
    let mut engine = Engine::new();
    let name: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let seen = Arc::clone(&name);
    engine.on_function_defined(move |event| {
        *seen.lock().expect("event log") = Some(event.definition.name.clone());
    });

    engine.eval("func g() { 1 }").expect("definition evaluates");

    assert_eq!(name.lock().expect("event log").as_deref(), Some("g"));
}

/// `SuiteEngineTests.cs:108 SetVariableAndRemoveVariable_Work`: a host-set variable is visible
/// through the non-throwing lookup, can be removed, and is then gone.
#[test]
fn set_variable_and_remove_variable_work() {
    let mut engine = Engine::new();
    engine.set_variable("x", natural(5));

    assert!(engine.try_get_variable("x").is_some());
    assert_eq!(
        engine.try_get_variable("x").expect("x exists").display(),
        "5"
    );

    assert!(engine.remove_variable("x"));
    assert!(!engine.variables().iter().any(|(name, _)| name == "x"));
}

/// The refusal/empty branch of the same surface: removing a name nothing defines returns false
/// and raises no event, and the lookup reports the absent value instead of throwing.
#[test]
fn removing_a_missing_variable_is_refused_without_an_event() {
    let mut engine = Engine::new();
    let events = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&events);
    engine.on_variable_changed(move |event| {
        seen.lock().expect("event log").push(event.name.clone());
    });

    assert!(!engine.remove_variable("x"));
    assert!(engine.try_get_variable("x").is_none());
    assert!(events.lock().expect("event log").is_empty());
}

/// The event order and payloads for one script: the assignment first, the definition second,
/// each with the value the reference carries and the removed flag false. The last statement is a
/// `func`, so the reference does not store a result variable and no third event is raised.
#[test]
fn events_are_ordered_and_carry_their_payloads() {
    let mut engine = Engine::new();
    let log = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&log);
    engine.on_variable_changed(move |event| {
        seen.lock().expect("event log").push(format!(
            "var:{}:{}:{}",
            event.name,
            event.value.display(),
            event.removed
        ));
    });
    let seen = Arc::clone(&log);
    engine.on_function_defined(move |event| {
        seen.lock()
            .expect("event log")
            .push(format!("fn:{}", event.definition.name));
    });

    engine
        .eval("x = 7; func g() { 1 }")
        .expect("script evaluates");

    assert_eq!(
        *log.lock().expect("event log"),
        vec!["var:x:7:false".to_string(), "fn:g".to_string()]
    );
}

/// The reference snapshot also records a removal, with the removed value and `removed: true`.
#[test]
fn removal_event_carries_the_removed_value() {
    let mut engine = Engine::new();
    let log = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&log);
    engine.on_variable_changed(move |event| {
        seen.lock().expect("event log").push((
            event.name.clone(),
            event.removed,
            event.value.display(),
        ));
    });

    engine.set_variable("x", natural(5));
    assert!(engine.remove_variable("x"));

    assert_eq!(
        *log.lock().expect("event log"),
        vec![
            ("x".to_string(), false, "5".to_string()),
            ("x".to_string(), true, "5".to_string())
        ]
    );
}

/// Supporting evidence for the snapshot fields the already-ported sibling row checks: `x` is a
/// Real with its display preserved, and `f` is a user function with its declared parameter list;
/// a builtin is present with the builtin flag and its declared parameters.
#[test]
fn capture_state_records_variables_and_user_functions() {
    let mut engine = Engine::new();
    engine
        .eval("x = 3.14; func f(y) = y * 2")
        .expect("script evaluates");

    let snapshot = engine.capture_state();
    assert_eq!(snapshot.revision, engine.revision());

    let x = snapshot.variables.get("x").expect("x captured");
    assert_eq!(x.name, "x");
    assert_eq!(x.kind, "Real");
    assert_eq!(x.display, "3.14");

    let f = snapshot.functions.get("f").expect("f captured");
    assert_eq!(f.parameters, vec!["y".to_string()]);
    assert!(!f.is_builtin);
    assert_eq!(f.plugin, None);

    let abs = snapshot
        .functions
        .get("abs")
        .expect("a core builtin is captured");
    assert!(abs.is_builtin);
    assert_eq!(abs.parameters, vec!["x".to_string()]);
}
