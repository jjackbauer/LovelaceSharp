//! Round-69: the ParameterDomain projection on the Modus seam and the solution-family wire field.
//!
//! The deferred census row `ParameterDomainOf_MapsEveryDeclaredArm#0` is a direct library call.
//! This module wires the same projection through the Modus plugin payload channel and through the
//! `SolutionFamily` record the language emits, so both declared arms cross the seam as the integer
//! Domain and an undeclared ordinal is refused before a payload can exist.

use lovelace_symbolics::parameter_domain::{ParameterDomain, UndeclaredParameterDomain};

use crate::modus::{
    ModusHost, ModusPlugin, Payload, ScalarResult, parameter_domain_payload_of_ordinal,
};
use crate::{Engine, LangError, Value};

struct ParameterDomainPlugin;

impl ModusPlugin for ParameterDomainPlugin {
    fn name(&self) -> &str {
        "Lovelace.ParameterDomain"
    }

    fn register(&mut self, context: &mut ModusHost) -> Result<(), LangError> {
        context.register_builtin_scalar("parameter_domain", &[], |_| {
            Ok(ScalarResult::from(Payload::from_parameter_domain(
                ParameterDomain::Integers,
            )))
        })
    }
}

/// `ParameterDomainOf` projected onto the Modus payload channel: every declared arm answers the
/// integer Domain payload, and the plugin round-trip re-wraps it as a first-class Domain value.
#[test]
fn modus_seam_maps_every_declared_arm_to_the_integer_domain_payload() {
    for arm in ParameterDomain::ALL {
        assert_eq!(
            Payload::from_parameter_domain(arm),
            Payload::Domain("integer".to_string()),
            "{arm:?} must cross the Modus seam as the integer domain"
        );
    }

    let mut engine = Engine::new();
    engine
        .load_plugin(&mut ParameterDomainPlugin)
        .expect("load the parameter-domain plugin");
    let returned = engine.eval("parameter_domain()").expect("plugin return");
    assert_eq!(returned, Value::Domain("integer".to_string()));
    assert_eq!(returned.typed(), "integer (Domain)");
}

/// The refusal reaches the Modus seam: an undeclared ordinal never becomes a Domain payload.
#[test]
fn modus_seam_refuses_an_undeclared_parameter_domain() {
    let error = parameter_domain_payload_of_ordinal(99).expect_err("99 must be refused");
    assert_eq!(error, UndeclaredParameterDomain { ordinal: 99 });
    assert_eq!(
        error.to_string(),
        "No wire Domain is declared for this parameter domain. (Parameter 'domain') Actual value was 99."
    );

    for arm in ParameterDomain::ALL {
        assert_eq!(
            parameter_domain_payload_of_ordinal(arm.ordinal()),
            Ok(Payload::Domain("integer".to_string())),
            "{arm:?} must still answer its Domain payload"
        );
    }
}

/// The solution family's wire field is derived through the mapping, not hardcoded: the declared
/// `Integers` arm is the one every kernel family carries, and it publishes the integer domain.
#[test]
fn solve_full_family_parameter_domain_is_the_projected_integer_domain() {
    let mut engine = Engine::new();
    engine.eval(r#"x = symbol("x")"#).expect("symbol");
    let solved = engine
        .eval("solve_full(sin(x) == 0, x)")
        .expect("solve_full");
    let Value::Record(type_name, fields) = &solved else {
        panic!("expected a SolveResult record, got {}", solved.kind_name());
    };
    assert_eq!(type_name, "SolveResult");

    let families = fields
        .iter()
        .find(|(name, _)| name == "families")
        .map(|(_, value)| value)
        .expect("families field");
    let Value::Vector(families) = families else {
        panic!("families must be a vector, got {}", families.kind_name());
    };
    let Some(Value::Record(_, family_fields)) = families.first() else {
        panic!("expected one SolutionFamily record");
    };
    let parameter_domain = family_fields
        .iter()
        .find(|(name, _)| name == "parameter_domain")
        .map(|(_, value)| value)
        .expect("parameter_domain field");
    assert_eq!(
        parameter_domain,
        &Value::Domain("integer".to_string()),
        "the family's parameter_domain must be the mapping's Domain value"
    );
}
