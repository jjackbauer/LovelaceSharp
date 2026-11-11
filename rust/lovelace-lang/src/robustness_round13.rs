//! Round 13 robustness regression tests: the four production defects the porting programme
//! found (solver panic, unbounded rewriting, lost root multiplicity, inexact exact quotients).
//!
//! Every expected value here is the frozen oracle's observed record for the same script; the
//! oracle invocations are recorded in docs/goal-parity-contract/round-13/robustness-fixes.md.

use lovelace_real::Real;

use crate::{Engine, Value};

fn eval(script: &str) -> Value {
    let mut engine = Engine::new();
    engine.set_precision(1000);
    match engine.eval(script) {
        Ok(value) => value,
        Err(err) => panic!("script failed unexpectedly: {script}: {}", err.message),
    }
}

fn record_field<'a>(value: &'a Value, name: &str) -> &'a Value {
    match value {
        Value::Record(_, fields) => fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("record has no field {name}")),
        other => panic!("expected a record, got {}", other.kind_name()),
    }
}

fn enum_variant(value: &Value) -> &str {
    match value {
        Value::Enum(_, variant) => variant,
        other => panic!("expected an enum, got {}", other.kind_name()),
    }
}

fn integral(value: &Value) -> i64 {
    match value {
        Value::Integer(n) => n.to_decimal().parse().expect("integer"),
        other => panic!("expected an integer, got {}", other.kind_name()),
    }
}

fn boolean(value: &Value) -> bool {
    match value {
        Value::Boolean(b) => *b,
        other => panic!("expected a boolean, got {}", other.kind_name()),
    }
}

fn items(value: &Value) -> &Vec<Value> {
    match value {
        Value::Vector(values) => values,
        other => panic!("expected a vector, got {}", other.kind_name()),
    }
}

fn real_of(script: &str) -> Real {
    match eval(script) {
        Value::Real(value) => value,
        other => panic!("{script} produced {} instead of Real", other.kind_name()),
    }
}

// ---------------------------------------------------------------------------
// Defect 1 — the cubic solver panicked with an index-out-of-bounds.
// ---------------------------------------------------------------------------

#[test]
fn round13_solve_cubic_without_rational_root_returns_the_oracle_record() {
    // Oracle: SolveResult(status: Solved, domain: real, solutions: [2^(1/3) AlgebraicExact]).
    let result = eval("x = symbol(\"x\"); solve_full(x^3 - 2, x, real)");
    assert_eq!(enum_variant(record_field(&result, "status")), "Solved");
    assert!(boolean(record_field(&result, "complete")));
    let solutions = items(record_field(&result, "solutions"));
    assert_eq!(solutions.len(), 1, "x^3 - 2 has one real root");
    assert_eq!(integral(record_field(&solutions[0], "multiplicity")), 1);
    assert_eq!(
        enum_variant(record_field(&solutions[0], "exactness")),
        "AlgebraicExact"
    );
    assert_eq!(
        record_field(&solutions[0], "value").display(),
        "2^(1/3)",
        "the solver must publish the Cardano root, not a refusal or a panic"
    );
}

#[test]
fn round13_solve_cubic_without_rational_root_never_panics_in_any_branch() {
    // disc > 0 (one real root) and disc < 0 (three real roots) both used to reach coeffs[4].
    for (script, expected) in [
        ("x = symbol(\"x\"); solve_full(x^3 - 2, x)", 3),
        ("x = symbol(\"x\"); solve_full(x^3 + x + 1, x, real)", 1),
        ("x = symbol(\"x\"); solve_full(x^3 - 3*x + 1, x, real)", 3),
    ] {
        let result = eval(script);
        assert_eq!(
            enum_variant(record_field(&result, "status")),
            "Solved",
            "{script}"
        );
        assert_eq!(
            items(record_field(&result, "solutions")).len(),
            expected,
            "{script}"
        );
    }
}

// ---------------------------------------------------------------------------
// Defect 3 — root multiplicity was lost in SolveResult solutions.
// ---------------------------------------------------------------------------

#[test]
fn round13_expanded_repeated_root_reports_its_multiplicity() {
    for (script, multiplicity) in [
        ("x = symbol(\"x\"); solve_full(x^2 - 2*x + 1 == 0, x)", 2),
        (
            "x = symbol(\"x\"); solve_full(x^3 - 3*x^2 + 3*x - 1 == 0, x)",
            3,
        ),
    ] {
        let result = eval(script);
        let solutions = items(record_field(&result, "solutions"));
        assert_eq!(solutions.len(), 1, "{script}: one distinct root");
        assert_eq!(
            integral(record_field(&solutions[0], "multiplicity")),
            multiplicity,
            "{script}: multiplicity must come from the factorization, not the literal 1"
        );
        assert_eq!(record_field(&solutions[0], "value").display(), "1");
    }
}

#[test]
fn round13_factored_power_matches_the_oracles_inverse_branch() {
    // The oracle's structural dispatch does not expand a power of a sum: (x - 1)^2 == 0 is
    // answered by the inverse branch and publishes TWO multiplicity-1 solutions (the expanded
    // spelling publishes one multiplicity-2 solution). Both spellings now match the oracle.
    let result = eval("x = symbol(\"x\"); solve_full((x - 1)^2 == 0, x)");
    let solutions = items(record_field(&result, "solutions"));
    assert_eq!(solutions.len(), 2);
    for solution in solutions {
        assert_eq!(record_field(solution, "value").display(), "1");
        assert_eq!(integral(record_field(solution, "multiplicity")), 1);
    }
}

// ---------------------------------------------------------------------------
// Defect 2 — the 420-redex transformation overflowed the stack instead of
// returning the oracle's budgeted TransformResult.
// ---------------------------------------------------------------------------

#[test]
fn round13_simplify_420_redexes_returns_budget_exceeded_instead_of_aborting() {
    // The C# DiagnosticContractTests row: 420 independent exp(log(x + k)) redexes against the
    // documented MaxSteps = 400. The oracle publishes BudgetExceeded with 400 steps and 400
    // conditions; the regression is that this call returns at all (it must never abort the
    // process) and that the budget — not the process stack — is what stops it.
    let terms: Vec<String> = (1..=420).map(|k| format!("exp(log(x+{k}))")).collect();
    let script = format!("x = symbol(\"x\"); simplify_full({})", terms.join(" + "));
    let result = eval(&script);
    assert_eq!(result.kind_name(), "Record");
    assert_eq!(
        enum_variant(record_field(&result, "status")),
        "BudgetExceeded"
    );
    assert!(boolean(record_field(&result, "budget_exceeded")));
    assert_eq!(
        record_field(&result, "budget_kind").display(),
        "rewrite_steps"
    );
    assert!(boolean(record_field(&result, "changed")));
    assert_eq!(items(record_field(&result, "steps")).len(), 400);
    assert_eq!(items(record_field(&result, "conditions")).len(), 400);
    let diagnostics = items(record_field(&result, "diagnostics"));
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        record_field(&diagnostics[0], "code").display(),
        "transform.budget-exceeded"
    );
    assert_eq!(
        enum_variant(record_field(&diagnostics[0], "category")),
        "BudgetExceeded"
    );
    // The partial expression is a genuine partial result: the 400 rewrites that fitted are
    // reflected (the rewritten sum collapses to 400*x + constant) and the remaining redexes are
    // still exp(log(...)) terms, exactly the shape the oracle publishes.
    let expression = record_field(&result, "expression").display();
    assert!(
        expression.contains("400*x"),
        "the 400 applied rewrites are missing from {expression}"
    );
    assert!(
        expression.contains("exp(log("),
        "the unrewritten tail is missing from {expression}"
    );
}

#[test]
fn round13_simplify_below_the_budget_stays_satisfied() {
    let result = eval("x = symbol(\"x\"); simplify_full(sin(x)^2 + cos(x)^2)");
    assert_eq!(enum_variant(record_field(&result, "status")), "Satisfied");
    assert!(!boolean(record_field(&result, "budget_exceeded")));
    assert_eq!(record_field(&result, "expression").display(), "1");
}

#[test]
fn round13_simplify_exp_log_carries_the_finite_condition_record() {
    // The rewrite that makes the 420-redex input reach the budget: simplify_full (not the safe
    // simplify) applies exp(log(u)) -> u and records FiniteCondition(log(u)).
    let result = eval("x = symbol(\"x\"); simplify_full(exp(log(x)))");
    assert_eq!(record_field(&result, "expression").display(), "x");
    let conditions = items(record_field(&result, "conditions"));
    assert_eq!(conditions.len(), 1);
    assert_eq!(conditions[0].kind_name(), "Record");
    assert_eq!(
        record_field(&conditions[0], "expression").display(),
        "log(x)"
    );
}

// ---------------------------------------------------------------------------
// Defect 4 — Real::div emitted a guard digit and denied exactness.
// ---------------------------------------------------------------------------

#[test]
fn round13_exact_terminating_quotients_are_exact_with_no_guard_digit() {
    // Oracle probes: 1/4 -> 0.25 (1/4), 1/8 -> 0.125 (1/8), 10/3 -> 3.(3) (10/3),
    // 1/3 -> 0.(3) (1/3). All four are exactly representable, so all four are exact.
    for (script, display, numerator, denominator) in [
        ("1/4", "0.25", 1u64, 4u64),
        ("1/8", "0.125", 1, 8),
        ("10/3", "3.(3)", 10, 3),
        ("1/3", "0.(3)", 1, 3),
    ] {
        let value = real_of(script);
        assert!(value.is_exact(), "{script} was marked inexact");
        assert_eq!(value.to_string(), display, "{script} display");
        let (num, den) = value.to_rational();
        assert_eq!(
            num.to_decimal(),
            numerator.to_string(),
            "{script} numerator"
        );
        assert_eq!(
            den.to_decimal(),
            denominator.to_string(),
            "{script} denominator"
        );
    }
}

#[test]
fn round13_terminating_quotient_longer_than_the_budget_stays_exact() {
    // The old digit loop stored budget+1 digits (a guard digit) and marked the exact terminating
    // quotient inexact. 1/(2^4000) has a 4000-digit terminating expansion and must stay exact.
    let value = real_of("1/(2^4000)");
    assert!(value.is_exact(), "1/(2^4000) was marked inexact");
    let (numerator, denominator) = value.to_rational();
    assert_eq!(numerator.to_decimal(), "1");
    let expected = lovelace_nat::Nat::from_u64(2).pow(4000);
    assert_eq!(denominator, expected);

    // The control: a NON-terminating quotient beyond the budget is truncated to exactly the
    // budget and is the one that is allowed to say inexact.
    let shortened = real_of("1/(3*10^1000)");
    assert!(
        !shortened.is_exact(),
        "a non-terminating quotient beyond the budget must say inexact"
    );
}
