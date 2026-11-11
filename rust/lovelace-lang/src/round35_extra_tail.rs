//! Round 35: the assumption/tri-state and RealLiteral rows that the final production covers but
//! the two family modules did not assert directly. Every test here is a census row the family
//! report lists; where the kernel test asserts the same case it is used, and the supplements below
//! close the remaining assertion-shaped gaps (never a weaker proxy for a C# API that is absent).

use crate::{Engine, Value, structured};
use lovelace_symbolics::assumptions::{
    Assumption, AssumptionSet, Domain, SymbolPredicate, Tristate,
};
use lovelace_symbolics::{Expr as Sym, RelOp};

fn eval(script: &str) -> Value {
    Engine::new()
        .eval(script)
        .unwrap_or_else(|error| panic!("'{script}' failed: {}", error.message))
}

fn display(script: &str) -> String {
    eval(script).display()
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    match value {
        Value::Record(type_name, fields) => fields
            .iter()
            .find(|(field, _)| field.as_str() == name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("record {type_name} has no field '{name}'")),
        other => panic!("expected a record, got {}", other.kind_name()),
    }
}

fn projected(script: &str) -> lovelace_json::Json {
    let mut engine = Engine::new();
    let value = engine
        .eval(script)
        .unwrap_or_else(|error| panic!("'{script}' failed: {}", error.message));
    structured::project_value(&engine, &value, None)
}

fn text<'a>(json: &'a lovelace_json::Json, key: &str) -> Option<&'a str> {
    json.get(key).and_then(lovelace_json::Json::as_str)
}

// ---------------------------------------------------------------------------
// Kernel supplements
// ---------------------------------------------------------------------------

fn relation(op: RelOp, bound: i64) -> Assumption {
    Assumption::SymbolRelation("x".to_string(), op, Sym::num(bound))
}

#[test]
fn integer_intervals_that_hold_are_accepted() {
    let integer = AssumptionSet::empty()
        .add(Assumption::SymbolDomain("x".to_string(), Domain::Integer))
        .expect("consistent");
    // [1, 2] holds 1 and 2
    let non_empty = integer
        .add(relation(RelOp::Ge, 1))
        .expect("x >= 1")
        .add(relation(RelOp::Le, 2))
        .expect("x <= 2");
    assert!(!non_empty.is_unsatisfiable());
    // (1, 2] leaves 2 reachable
    let open = integer
        .add(relation(RelOp::Gt, 1))
        .expect("x > 1")
        .add(relation(RelOp::Le, 2))
        .expect("x <= 2");
    assert!(!open.is_unsatisfiable());
}

#[test]
fn predicate_lattice_rows() {
    // EvenOdd_ImplyInteger_OddImpliesNonZero
    let even = AssumptionSet::empty()
        .add(Assumption::SymbolProperty(
            "x".to_string(),
            SymbolPredicate::Even,
        ))
        .expect("consistent");
    let odd = AssumptionSet::empty()
        .add(Assumption::SymbolProperty(
            "x".to_string(),
            SymbolPredicate::Odd,
        ))
        .expect("consistent");
    assert_eq!(
        even.ask(&Assumption::SymbolDomain("x".to_string(), Domain::Integer)),
        Tristate::True
    );
    assert_eq!(
        odd.ask(&Assumption::SymbolDomain("x".to_string(), Domain::Integer)),
        Tristate::True
    );
    assert_eq!(
        odd.ask(&Assumption::SymbolProperty(
            "x".to_string(),
            SymbolPredicate::NonZero
        )),
        Tristate::True
    );
    assert_eq!(
        even.ask(&Assumption::SymbolProperty(
            "x".to_string(),
            SymbolPredicate::NonZero
        )),
        Tristate::Unknown
    );

    // Integer_DoesNotImply_EvenOddNonZero
    let integer = AssumptionSet::empty()
        .add(Assumption::SymbolDomain("x".to_string(), Domain::Integer))
        .expect("consistent");
    for property in [
        SymbolPredicate::Even,
        SymbolPredicate::Odd,
        SymbolPredicate::NonZero,
    ] {
        assert_eq!(
            integer.ask(&Assumption::SymbolProperty("x".to_string(), property)),
            Tristate::Unknown,
            "{property:?}"
        );
    }

    // DomainPredicates_AreRealGuarded
    let exp_x = Assumption::ExpressionProperty(
        Sym::Fun("exp".to_string(), vec![Sym::sym("x")]),
        SymbolPredicate::Positive,
    );
    assert_eq!(AssumptionSet::empty().ask(&exp_x), Tristate::Unknown);
    let real = AssumptionSet::empty()
        .add(Assumption::SymbolDomain("x".to_string(), Domain::Real))
        .expect("consistent");
    assert_eq!(real.ask(&exp_x), Tristate::True);
    let x_squared = Assumption::ExpressionProperty(
        Sym::pow(Sym::sym("x"), Sym::num(2)),
        SymbolPredicate::NonNegative,
    );
    assert_eq!(AssumptionSet::empty().ask(&x_squared), Tristate::Unknown);
}

#[test]
fn or_returns_false_when_every_disjunct_is_false() {
    let set = AssumptionSet::empty()
        .add(Assumption::SymbolProperty(
            "x".to_string(),
            SymbolPredicate::NonPositive,
        ))
        .expect("consistent")
        .add(relation(RelOp::Lt, 0))
        .expect("consistent");
    assert_eq!(
        set.ask(&Assumption::SymbolProperty(
            "x".to_string(),
            SymbolPredicate::NonNegative
        )),
        Tristate::False
    );
}

#[test]
fn unsatisfiable_sentinel_is_sticky() {
    let unsatisfiable = AssumptionSet::unsatisfiable();
    assert!(unsatisfiable.is_unsatisfiable());
    assert!(!AssumptionSet::empty().is_unsatisfiable());
    let added = unsatisfiable
        .add(Assumption::SymbolProperty(
            "x".to_string(),
            SymbolPredicate::Positive,
        ))
        .expect("sticky");
    assert!(added.is_unsatisfiable());
}

#[test]
fn the_same_atom_from_two_contexts_is_stored_once() {
    let atoms = [
        Assumption::SymbolDomain("t".to_string(), Domain::Real),
        Assumption::SymbolDomain("t".to_string(), Domain::Real),
        Assumption::SymbolProperty("a".to_string(), SymbolPredicate::Positive),
    ];
    let set = atoms.into_iter().fold(AssumptionSet::empty(), |set, atom| {
        set.add(atom).expect("consistent")
    });
    assert_eq!(set.atoms().len(), 2);
    let rendered: Vec<String> = set.atoms().iter().map(Assumption::describe).collect();
    assert_eq!(rendered, vec!["t in Real", "a is Positive"]);
}

// ---------------------------------------------------------------------------
// RealLiteral supplements (the language surface + the published projection)
// ---------------------------------------------------------------------------

#[test]
fn evalf_of_a_truncated_constant_is_never_an_exact_rational() {
    let expected = [
        ("evalf(pi(1), 1)", "3.1"),
        ("evalf(pi(1), 5)", "3.1"),
        ("evalf(pi(1), 40)", "3.1"),
        ("evalf(pi(30), 5)", "3.14159"),
        ("evalf(pi(30), 29)", "3.14159265358979323846264338327"),
        ("evalf(pi(30), 30)", "3.141592653589793238462643383279"),
        ("evalf(pi(30), 40)", "3.141592653589793238462643383279"),
        ("evalf(pi(30), 100)", "3.141592653589793238462643383279"),
        ("evalf(e(1), 5)", "2.7"),
        ("evalf(e(1), 40)", "2.7"),
        ("evalf(e(30), 7)", "2.7182818"),
        ("evalf(e(30), 29)", "2.71828182845904523536028747135"),
        ("evalf(e(30), 40)", "2.718281828459045235360287471352"),
        ("evalf(e(30), 100)", "2.718281828459045235360287471352"),
    ];
    for (script, digits) in expected {
        let json = projected(script);
        assert_eq!(text(&json, "value"), Some(digits), "{script}");
        assert_eq!(
            json.get("exact"),
            Some(&lovelace_json::Json::Bool(false)),
            "{script}"
        );
        assert!(json.get("numerator").is_none(), "{script}: numerator");
        assert!(json.get("denominator").is_none(), "{script}: denominator");
    }
}

#[test]
fn functions_of_a_truncated_value_are_inexact_with_no_rational_form() {
    let expressions = [
        "evalf(sin(pi(1)*1/6), 40)",
        "evalf(cos(pi(1)*1/6), 40)",
        "evalf(sin(pi(1)*1/4), 40)",
        "evalf(atan(1), 40)",
        "evalf(sin(pi(30)/6), 40)",
        "evalf(cos(pi(30)/6), 40)",
        "evalf(sin(pi(30)/4), 40)",
        "evalf(atan(pi(30)/4), 40)",
        "evalf(exp(pi(30)/10), 40)",
        "evalf(sin(pi(45)/6), 40)",
        "evalf(sin(pi(30)/6), 100)",
        "evalf(subs(x + 1, x, pi(30)), 40)",
    ];
    for script in expressions {
        let full = format!("x = symbol(\"x\"); {script}");
        let json = projected(&full);
        assert_eq!(
            json.get("kind").and_then(lovelace_json::Json::as_str),
            Some("Real"),
            "{script}"
        );
        assert_eq!(
            json.get("exact"),
            Some(&lovelace_json::Json::Bool(false)),
            "{script}"
        );
        assert!(json.get("numerator").is_none(), "{script}: numerator");
        assert!(json.get("denominator").is_none(), "{script}: denominator");
    }
}

#[test]
fn truncating_exact_controls_stay_exact() {
    let sqrt4 = projected("evalf(sqrt(4), 40)");
    assert_eq!(
        sqrt4.get("kind").and_then(lovelace_json::Json::as_str),
        Some("Integer")
    );
    assert_eq!(text(&sqrt4, "value"), Some("2"));

    let periodic = projected("1/17");
    assert_eq!(text(&periodic, "value"), Some("0.(0588235294117647)"));
    assert_eq!(
        periodic.get("exact"),
        Some(&lovelace_json::Json::Bool(true))
    );
    assert_eq!(text(&periodic, "numerator"), Some("1"));
    assert_eq!(text(&periodic, "denominator"), Some("17"));

    let complex_root = projected("evalf((-4)^(1/2), 40)");
    assert_eq!(
        complex_root
            .get("kind")
            .and_then(lovelace_json::Json::as_str),
        Some("Complex")
    );
    assert_eq!(text(&complex_root, "re"), Some("0"));
    assert_eq!(text(&complex_root, "im"), Some("2"));

    let dft0 = projected("dft([1,2,3])[0]");
    assert_eq!(text(&dft0, "re"), Some("6"));
    assert_eq!(text(&dft0, "im"), Some("0"));
}

#[test]
fn elementary_functions_of_an_approximate_argument_are_inexact() {
    for function in ["sin", "cos", "exp", "log"] {
        let script = format!("x = symbol(\"x\"); inspect({function}(pi(30)))");
        let inspection = eval(&script);
        assert_eq!(
            field(&inspection, "exact").display(),
            "False",
            "{function}(pi(30)) must stay inexact"
        );
    }
}
