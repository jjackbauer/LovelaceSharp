//! Round 31: the circular / infinite-family rows of InfiniteFamilyCompletenessTests.
//!
//! An equation whose solution set is INFINITE over the requested domain must not be published as
//! Solved/Complete with a finite list that represents one member, and a candidate that still
//! contains the solved variable is a circular answer, not a value. Every test below repeats the C#
//! assertion it is mapped to.

use lovelace_symbolics::{substitute, Expr as Sym};

use crate::{Engine, Value};

fn eval(script: &str) -> Value {
    Engine::new()
        .eval(script)
        .unwrap_or_else(|error| panic!("script failed: {}", error.message))
}

fn record_field<'a>(value: &'a Value, name: &str) -> &'a Value {
    match value {
        Value::Record(type_name, fields) => fields
            .iter()
            .find(|(field, _)| field.as_str() == name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("record {type_name} has no field")),
        other => panic!("expected a record, got {}", other.kind_name()),
    }
}

fn array_items(value: &Value) -> &[Value] {
    match value {
        Value::Vector(items) => items,
        Value::Array(array) => array.as_slice(),
        other => panic!("expected an array, got {}", other.kind_name()),
    }
}

fn enum_name(value: &Value) -> &str {
    match value {
        Value::Enum(_, variant) => variant,
        other => panic!("expected an enum, got {}", other.kind_name()),
    }
}

fn boolean(value: &Value) -> bool {
    match value {
        Value::Boolean(truth) => *truth,
        other => panic!("expected a Boolean, got {}", other.kind_name()),
    }
}

fn integer(value: &Value) -> i64 {
    match value {
        Value::Integer(n) => n.to_decimal().parse().unwrap_or(i64::MIN),
        other => panic!("expected an Integer, got {}", other.kind_name()),
    }
}

fn domain_name(value: &Value) -> &str {
    match value {
        Value::Domain(domain) => domain,
        other => panic!("expected a Domain, got {}", other.kind_name()),
    }
}

fn symbolic(value: &Value) -> &Sym {
    match value {
        Value::Symbolic(expression) => expression,
        other => panic!("expected a Symbolic value, got {}", other.kind_name()),
    }
}

fn solve(script: &str) -> Value {
    eval(script)
}

fn family_templates(result: &Value) -> Vec<String> {
    array_items(record_field(result, "families"))
        .iter()
        .map(|family| symbolic(record_field(family, "template")).to_string())
        .collect()
}

fn solution_values(result: &Value) -> Vec<String> {
    array_items(record_field(result, "solutions"))
        .iter()
        .map(|solution| symbolic(record_field(solution, "value")).to_string())
        .collect()
}

fn mentions_x(expression: &Sym) -> bool {
    fn walk(e: &Sym) -> bool {
        match e {
            Sym::Fun(name, _) if name == "rootof" => false,
            Sym::Sym(name) => name == "x",
            Sym::Num(_) | Sym::Real(_) => false,
            Sym::Add(items) | Sym::Mul(items) => items.iter().any(walk),
            Sym::Pow(base, exponent) => walk(base) || walk(exponent),
            Sym::Fun(_, args) => args.iter().any(walk),
            Sym::Div(a, b) => walk(a) || walk(b),
            Sym::Rel(_, a, b) => walk(a) || walk(b),
        }
    }
    walk(expression)
}

// ---------------------------------------------------------------------------
// 1. The reported defect: exp(x) = c over the complex field
// ---------------------------------------------------------------------------

#[test]
fn solve_full_exp_symbolic_complex_represents_the_whole_family() {
    let result = solve("x = symbol(\"x\"); c = symbol(\"c\"); solve_full(exp(x) == c, x)");

    assert_eq!(enum_name(record_field(&result, "status")), "Solved");
    assert!(boolean(record_field(&result, "complete")));
    assert_eq!(enum_name(record_field(&result, "completeness")), "Complete");
    assert!(array_items(record_field(&result, "solutions")).is_empty());

    let families = array_items(record_field(&result, "families"));
    assert_eq!(families.len(), 1);
    let family = &families[0];
    assert_eq!(
        domain_name(record_field(family, "parameter_domain")),
        "integer"
    );
    assert_eq!(enum_name(record_field(family, "exactness")), "ParametricExact");

    // exp(x) = 0 has no solution, so the family is valid under c != 0 and says so.
    let conditions = array_items(record_field(family, "conditions"));
    assert_eq!(conditions.len(), 1);
    assert_eq!(conditions[0].display(), "c != 0");
    assert_eq!(
        symbolic(record_field(family, "template")).to_string(),
        "log(c) + 2*k*pi*i"
    );
}

#[test]
fn kernel_exp_symbolic_complex_every_family_member_solves() {
    let result = solve("x = symbol(\"x\"); c = symbol(\"c\"); solve_full(exp(x) == c, x)");
    let family = &array_items(record_field(&result, "families"))[0];
    let template = symbolic(record_field(family, "template")).clone();
    let parameter = symbolic(record_field(family, "parameter")).to_string();
    assert_eq!(parameter, "k");
    assert!(!mentions_x(&template), "a family template must not contain x");

    // For a concrete c = 3 every member of the family is a solution of exp(x) - 3 = 0 ...
    let equation = Sym::Fun("exp".to_string(), vec![Sym::sym("x")]);
    let equation = Sym::sub(equation, Sym::num(3));
    let mut members: Vec<Sym> = Vec::new();
    for k in [-3i64, -1, 0, 1, 4] {
        let member = substitute(&template, &parameter, &Sym::num(k));
        let member = substitute(&member, "c", &Sym::num(3));
        assert!(
            crate::builtins::residual_is_zero(&equation, "x", &member) == Some(true),
            "k = {k} gives {member}, which does not solve exp(x) = 3"
        );
        members.push(member);
    }

    // ... and k = 0 and k = 1 are DIFFERENT solutions, so a one-element set omits the rest.
    let mut distinct: Vec<String> = members.iter().map(|m| m.to_string()).collect();
    distinct.sort();
    distinct.dedup();
    assert!(distinct.len() >= 4, "the family members must be pairwise distinct");
}

#[test]
fn solve_full_exp_numeric_complex_is_not_the_principal_value_alone() {
    let result = solve("x = symbol(\"x\"); solve_full(exp(x) == 5, x)");

    assert_eq!(enum_name(record_field(&result, "status")), "Solved");
    assert!(boolean(record_field(&result, "complete")));
    assert!(array_items(record_field(&result, "solutions")).is_empty());
    let families = array_items(record_field(&result, "families"));
    assert_eq!(families.len(), 1);
    assert_eq!(
        domain_name(record_field(&families[0], "parameter_domain")),
        "integer"
    );
    assert!(!mentions_x(symbolic(record_field(&families[0], "template"))));
    assert_eq!(family_templates(&result), vec!["log(5) + 2*k*pi*i"]);
}

// ---------------------------------------------------------------------------
// 2. The real domain keeps its (correct) answer
// ---------------------------------------------------------------------------

#[test]
fn real_domain_exp_keeps_the_single_solution() {
    let positive = solve("x = symbol(\"x\"); solve_full(exp(x) == 5, x, real)");
    assert_eq!(enum_name(record_field(&positive, "status")), "Solved");
    assert_eq!(
        enum_name(record_field(&positive, "completeness")),
        "Complete"
    );
    assert_eq!(solution_values(&positive), vec!["log(5)"]);
    assert!(array_items(record_field(&positive, "families")).is_empty());
    let solution = &array_items(record_field(&positive, "solutions"))[0];
    assert_eq!(
        array_items(record_field(solution, "conditions"))[0].display(),
        "1 != 0"
    );
    assert_eq!(
        array_items(record_field(&positive, "common_conditions"))[0].display(),
        "1 != 0"
    );

    let negative = solve("x = symbol(\"x\"); solve_full(exp(x) == -5, x, real)");
    assert_eq!(enum_name(record_field(&negative, "status")), "NoSolutions");
    assert!(array_items(record_field(&negative, "families")).is_empty());

    let zero = solve("x = symbol(\"x\"); solve_full(exp(x) == 0, x, real)");
    assert_eq!(enum_name(record_field(&zero, "status")), "NoSolutions");
    assert!(array_items(record_field(&zero, "families")).is_empty());
}

#[test]
fn real_domain_exp_symbolic_keeps_its_behaviour_and_never_carries_the_complex_period() {
    let result = solve("x = symbol(\"x\"); c = symbol(\"c\"); solve_full(exp(x) == c, x, real)");

    assert_eq!(
        match record_field(&result, "domain") {
            Value::Domain(domain) => domain.as_str(),
            other => panic!("expected a Domain, got {}", other.kind_name()),
        },
        "real"
    );
    assert_eq!(enum_name(record_field(&result, "status")), "Solved");
    assert_eq!(solution_values(&result).len(), 1);
    assert!(array_items(record_field(&result, "families")).is_empty());
}

// ---------------------------------------------------------------------------
// 3. The rule is the inverse-family rule, not an exp special case
// ---------------------------------------------------------------------------

#[test]
fn the_other_inverses_keep_their_own_honest_answer() {
    let sin_half = solve("x = symbol(\"x\"); solve_full(sin(x) == 1/2, x)");
    assert_eq!(enum_name(record_field(&sin_half, "status")), "Solved");
    assert_eq!(array_items(record_field(&sin_half, "families")).len(), 2);
    for family in array_items(record_field(&sin_half, "families")) {
        assert_eq!(
            domain_name(record_field(family, "parameter_domain")),
            "integer"
        );
    }
    assert_eq!(
        family_templates(&sin_half),
        vec!["asin(1/2) + 2*k*pi", "pi - asin(1/2) + 2*k*pi"]
    );

    let cos_half = solve("x = symbol(\"x\"); solve_full(cos(x) == 1/2, x)");
    assert_eq!(enum_name(record_field(&cos_half, "status")), "Solved");
    assert_eq!(array_items(record_field(&cos_half, "families")).len(), 2);
    assert_eq!(
        family_templates(&cos_half),
        vec!["acos(1/2) + 2*k*pi", "-acos(1/2) + 2*k*pi"]
    );

    let tan_one = solve("x = symbol(\"x\"); solve_full(tan(x) == 1, x)");
    assert_eq!(enum_name(record_field(&tan_one, "status")), "Solved");
    assert_eq!(array_items(record_field(&tan_one, "families")).len(), 1);
    assert_eq!(family_templates(&tan_one), vec!["atan(1) + k*pi"]);

    // log is a function (principal branch), so log(x) = c has exactly one solution e^c.
    let log_c = solve("x = symbol(\"x\"); c = symbol(\"c\"); solve_full(log(x) == c, x)");
    assert_eq!(enum_name(record_field(&log_c, "status")), "Solved");
    assert_eq!(enum_name(record_field(&log_c, "completeness")), "Complete");
    assert_eq!(solution_values(&log_c), vec!["exp(c)"]);
    assert!(array_items(record_field(&log_c, "families")).is_empty());
    for solution in array_items(record_field(&log_c, "solutions")) {
        assert!(!mentions_x(symbolic(record_field(solution, "value"))));
    }
}

// ---------------------------------------------------------------------------
// 4. Circular candidates are not values and never enter a Solved set
// ---------------------------------------------------------------------------

#[test]
fn circular_candidates_are_not_represented_and_are_not_a_complete_set() {
    // (equation, domain argument) - the ten rows of the C# CircularEquations theory.
    let cases: [(&str, &str); 10] = [
        ("exp(x) == x", ""),
        ("log(x) == x", ""),
        ("x - log(x) == 0", ""),
        ("x - sin(x) == 0", ""),
        ("x - cos(x) == 0", ""),
        ("x*exp(x) == 1", ""),
        ("x + exp(x) == 0", ""),
        ("exp(x) == x", ", real"),
        ("log(x) == x", ", real"),
        ("x + exp(x) == 0", ", real"),
    ];
    for (equation, domain) in cases {
        let script = format!("x = symbol(\"x\"); solve_full({equation}, x{domain})");
        let result = solve(&script);
        let status = enum_name(record_field(&result, "status"));
        assert_ne!(status, "Solved", "{script} must not be Solved");
        assert_ne!(
            status, "NoSolutions",
            "{script} has solutions; nothing is proven empty"
        );
        assert!(!boolean(record_field(&result, "complete")), "{script} claims complete");
        assert_ne!(
            enum_name(record_field(&result, "completeness")),
            "Complete",
            "{script} claims Complete"
        );
        for solution in array_items(record_field(&result, "solutions")) {
            let value = symbolic(record_field(solution, "value"));
            assert!(
                !value.canonical().contains("(sym x)"),
                "{script} published the circular value {value}"
            );
        }
        for family in array_items(record_field(&result, "families")) {
            let template = symbolic(record_field(family, "template"));
            assert!(
                !template.canonical().contains("(sym x)"),
                "{script} published the circular template {template}"
            );
        }
    }
}

#[test]
fn the_legitimate_inverse_branch_still_represents_its_values() {
    let result = solve("x = symbol(\"x\"); solve_full(sqrt(x) == x, x)");

    assert_eq!(enum_name(record_field(&result, "status")), "Solved");
    assert!(boolean(record_field(&result, "complete")));
    let values = solution_values(&result);
    assert_eq!(values.len(), 2);
    assert_eq!(values, vec!["0", "1"]);
    let equation = Sym::sub(
        Sym::pow(
            Sym::sym("x"),
            Sym::rational(
                &lovelace_int::Int::from_i64(1),
                &lovelace_nat::Nat::from_u64(2),
            ),
        ),
        Sym::sym("x"),
    );
    for solution in array_items(record_field(&result, "solutions")) {
        let value = symbolic(record_field(solution, "value"));
        assert!(!mentions_x(value));
        assert_eq!(
            crate::builtins::residual_is_zero(&equation, "x", value),
            Some(true),
            "the represented value {value} does not solve sqrt(x) = x"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Nothing else moves (the acceptance controls)
// ---------------------------------------------------------------------------

#[test]
fn acceptance_controls_nothing_else_moves() {
    let quadratic = solve("x = symbol(\"x\"); solve_full(x^2 - 4 == 0, x)");
    assert_eq!(enum_name(record_field(&quadratic, "status")), "Solved");
    assert!(boolean(record_field(&quadratic, "complete")));
    assert_eq!(array_items(record_field(&quadratic, "solutions")).len(), 2);

    let no_real = solve("x = symbol(\"x\"); solve_full(x^2 + 1 == 0, x, real)");
    assert_eq!(enum_name(record_field(&no_real, "status")), "NoSolutions");
    assert!(boolean(record_field(&no_real, "complete")));

    let extraneous = solve("x = symbol(\"x\"); solve_full(sqrt(x) + 2 == 0, x)");
    assert_eq!(enum_name(record_field(&extraneous, "status")), "Unevaluated");
    assert!(!boolean(record_field(&extraneous, "complete")));
    assert_eq!(integer(record_field(&extraneous, "represented_count")), 0);

    let quartic = solve("x = symbol(\"x\"); solve_full(x^4 - x^2 - 1 == 0, x)");
    assert_eq!(enum_name(record_field(&quartic, "status")), "Partial");
    assert!(!boolean(record_field(&quartic, "complete")));
    assert_eq!(integer(record_field(&quartic, "represented_count")), 2);
    assert_eq!(integer(record_field(&quartic, "unrepresented_count")), 2);
}

// ---------------------------------------------------------------------------
// 6. The oracle: the complex answer is an ImageSet, not one value
// ---------------------------------------------------------------------------

/// The C# row drives SymPy live (solveset(exp(x) - c, x, S.Complexes) is an ImageSet over the
/// integers). The Rust workspace has no SymPy bridge, so the assertable part of that oracle row is
/// the SHAPE it measured plus the two algebraic facts it proves: the answer is a single family
/// parameterised by the integers with the period 2*pi*i, and every member of that family solves the
/// equation for a concrete c (the facts above). This test pins the shape.
#[test]
fn exp_over_complexes_is_an_imageset_not_one_value() {
    let result = solve("x = symbol(\"x\"); c = symbol(\"c\"); solve_full(exp(x) == c, x)");
    let family = &array_items(record_field(&result, "families"))[0];
    assert_eq!(
        domain_name(record_field(family, "parameter_domain")),
        "integer"
    );
    assert_eq!(
        symbolic(record_field(family, "period")).canonical(),
        "(mul (rat 2 1) (pi) (i))"
    );
    assert_eq!(
        symbolic(record_field(family, "template")).canonical(),
        "(add (fn log (sym c)) (mul (rat 2 1) (sym k) (pi) (i)))"
    );

    // Over the REALS the same solver answers the single value log(c), like SymPy's {log(c)}.
    let real = solve("x = symbol(\"x\"); c = symbol(\"c\"); solve_full(exp(x) == c, x, real)");
    assert_eq!(solution_values(&real), vec!["log(c)"]);
    assert!(array_items(record_field(&real, "families")).is_empty());
}

// ---------------------------------------------------------------------------
// The two rows whose C# assertion is the KERNEL contract, restated with the
// numeric side the reference checks (the 40-digit residual)
// ---------------------------------------------------------------------------

/// `Kernel_ExpSymbolic_Complex_EveryFamilyMemberSolves`: the kernel-level record is Solved /
/// Complete with the family as its whole answer, every member solves exp(x) = 3 for the same k
/// values the C# row uses, and k = 0 and k = 1 are DIFFERENT solutions (the C# row asserts this
/// numerically with `NumOps.IsZero(one - zero)`, so the Rust analogue evaluates the difference at
/// the same 40 digits through the same residual predicate).
#[test]
fn kernel_exp_symbolic_complex_every_family_member_solves_in_a_complete_set() {
    let result = solve("x = symbol(\"x\"); c = symbol(\"c\"); solve_full(exp(x) == c, x)");

    assert_eq!(enum_name(record_field(&result, "status")), "Solved");
    assert_eq!(
        enum_name(record_field(&result, "completeness")),
        "Complete"
    );
    assert!(boolean(record_field(&result, "complete")));
    assert!(array_items(record_field(&result, "solutions")).is_empty());

    let families = array_items(record_field(&result, "families"));
    assert_eq!(families.len(), 1);
    let family = &families[0];
    assert_eq!(
        domain_name(record_field(family, "parameter_domain")),
        "integer"
    );
    let template = symbolic(record_field(family, "template")).clone();
    let parameter = symbolic(record_field(family, "parameter")).to_string();
    assert!(!mentions_x(&template), "a family template must not contain x");

    // For a concrete c = 3, every member of the family is a solution of exp(x) - 3 = 0 ...
    let equation = Sym::sub(
        Sym::Fun("exp".to_string(), vec![Sym::sym("x")]),
        Sym::num(3),
    );
    let mut members: Vec<Sym> = Vec::new();
    for k in [-3i64, -1, 0, 1, 4] {
        let member = substitute(&template, &parameter, &Sym::num(k));
        let member = substitute(&member, "c", &Sym::num(3));
        assert_eq!(
            crate::builtins::residual_is_zero(&equation, "x", &member),
            Some(true),
            "k = {k} gives {member}, which does not solve exp(x) = 3"
        );
        members.push(member);
    }

    // ... and k = 0 and k = 1 are DIFFERENT solutions, so a one-element set omits the rest.
    let zero = &members[2];
    let one = &members[3];
    let difference = Sym::sub(one.clone(), zero.clone());
    assert_eq!(
        crate::builtins::residual_is_zero(&Sym::sym("x"), "x", &difference),
        Some(false),
        "the family's k = 0 and k = 1 members must be distinct solutions (their difference is \
         {difference}, evaluated as zero)"
    );
}

/// `RealDomain_ExpKeepsTheSingleSolution`: the real domain keeps the one solution of exp(x) = 5,
/// which satisfies the original equation at 40 digits; a negative right-hand side and zero both
/// stay NoSolutions with no family and no leaked complex period.
#[test]
fn real_domain_exp_keeps_the_single_solution_that_satisfies_the_equation() {
    let positive = solve("x = symbol(\"x\"); solve_full(exp(x) == 5, x, real)");
    assert_eq!(enum_name(record_field(&positive, "status")), "Solved");
    assert_eq!(
        enum_name(record_field(&positive, "completeness")),
        "Complete"
    );
    let solutions = array_items(record_field(&positive, "solutions"));
    assert_eq!(solutions.len(), 1);
    assert!(array_items(record_field(&positive, "families")).is_empty());
    let value = symbolic(record_field(&solutions[0], "value"));
    let equation = Sym::sub(
        Sym::Fun("exp".to_string(), vec![Sym::sym("x")]),
        Sym::num(5),
    );
    assert_eq!(
        crate::builtins::residual_is_zero(&equation, "x", value),
        Some(true),
        "the represented value {value} does not solve exp(x) = 5"
    );

    let negative = solve("x = symbol(\"x\"); solve_full(exp(x) == -5, x, real)");
    assert_eq!(enum_name(record_field(&negative, "status")), "NoSolutions");
    assert!(array_items(record_field(&negative, "families")).is_empty());

    let zero = solve("x = symbol(\"x\"); solve_full(exp(x) == 0, x, real)");
    assert_eq!(enum_name(record_field(&zero, "status")), "NoSolutions");
    assert!(array_items(record_field(&zero, "families")).is_empty());
}
