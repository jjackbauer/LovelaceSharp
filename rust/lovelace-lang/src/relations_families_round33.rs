//! Round 33: the family rows of RelationsAndFamiliesTests that the round-31 periodic/exponential
//! solver unblocked (SinFamily_MembersSatisfyEquation_Numerically #0..#2, Solve_CosHalf_ReturnsTwoFamilies,
//! Solve_ExpZero_Empty, Solve_SinBeyondRange_EmptyWithNote).
//!
//! The C# rows drive the kernel's SolveResult (SolutionKind, Status, Note); the language's solve_full
//! record carries the same contract (status, completeness, families, diagnostics), and the numeric
//! member check reuses the kernel's own 40-digit residual predicate.

use lovelace_symbolics::{Expr as Sym, substitute};

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
            .unwrap_or_else(|| panic!("record {type_name} has no field {name}")),
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

fn text(value: &Value) -> &str {
    match value {
        Value::Text(text) => text,
        other => panic!("expected a Text value, got {}", other.kind_name()),
    }
}

fn symbolic(value: &Value) -> &Sym {
    match value {
        Value::Symbolic(expression) => expression,
        other => panic!("expected a Symbolic value, got {}", other.kind_name()),
    }
}

fn status(result: &Value) -> &str {
    enum_name(record_field(result, "status"))
}

fn families(result: &Value) -> &[Value] {
    array_items(record_field(result, "families"))
}

fn family_templates(result: &Value) -> Vec<String> {
    families(result)
        .iter()
        .map(|family| symbolic(record_field(family, "template")).to_string())
        .collect()
}

fn diagnostics(result: &Value) -> &[Value] {
    array_items(record_field(result, "diagnostics"))
}

// ---------------------------------------------------------------------------
// Solve_SinZero family members: every member of k*pi satisfies sin(x) = 0
// ---------------------------------------------------------------------------

/// SinFamily_MembersSatisfyEquation_Numerically #0/#1/#2: the C# theory substitutes k = 0, 1, 2 into
/// the family template and requires |sin(member)| < 10^-19 at 50 digits. The Rust analogue evaluates
/// the same residual at the kernel's 40 digits (a stricter 10^-24 budget), so a passing check is at
/// least as strong.
#[test]
fn sin_family_members_satisfy_the_equation_numerically() {
    let result = eval("x = symbol(\"x\"); solve_full(sin(x) == 0, x)");
    assert_eq!(status(&result), "Solved");
    let families = families(&result);
    assert_eq!(families.len(), 1, "sin(x) = 0 has the single family k*pi");
    assert_eq!(family_templates(&result), vec!["k*pi"]);

    let family = &families[0];
    let template = symbolic(record_field(family, "template")).clone();
    let parameter = symbolic(record_field(family, "parameter")).to_string();
    assert_eq!(parameter, "k");
    let equation = Sym::Fun("sin".to_string(), vec![Sym::sym("x")]);
    for k in [0i64, 1, 2] {
        let member = substitute(&template, &parameter, &Sym::num(k));
        assert_eq!(
            crate::builtins::residual_is_zero(&equation, "x", &member),
            Some(true),
            "k = {k} gives {member}, and sin({member}) is not zero"
        );
    }
}

// ---------------------------------------------------------------------------
// Two families for cos(x) = 1/2, and the empty/range classifications
// ---------------------------------------------------------------------------

/// Solve_CosHalf_ReturnsTwoFamilies: the periodic inverse publishes both branches.
#[test]
fn solve_cos_half_returns_two_families() {
    let result = eval("x = symbol(\"x\"); solve_full(cos(x) == 1/2, x)");
    assert_eq!(status(&result), "Solved");
    assert_eq!(families(&result).len(), 2);
}

/// Solve_ExpZero_Empty: exp(x) = 0 has no solution; the empty answer is a NoSolutions record with a
/// diagnostic, not a family and not an Unevaluated non-answer.
#[test]
fn solve_exp_zero_is_empty() {
    let result = eval("x = symbol(\"x\"); solve_full(exp(x) == 0, x)");
    assert_eq!(status(&result), "NoSolutions");
    assert!(families(&result).is_empty());
    assert!(array_items(record_field(&result, "solutions")).is_empty());
    let diags = diagnostics(&result);
    assert!(!diags.is_empty(), "the empty answer states why");
    assert!(!text(record_field(&diags[0], "message")).trim().is_empty());
}

/// Solve_SinBeyondRange_EmptyWithNote: |c| > 1 has no real solution (NoSolutions with a note), while
/// over the complex field the solutions exist but are not representable (Unevaluated with a note) --
/// the two must not be conflated.
#[test]
fn solve_sin_beyond_range_is_empty_over_the_reals_and_unevaluated_over_the_complexes() {
    let real = eval("x = symbol(\"x\"); solve_full(sin(x) == 2, x, real)");
    assert_eq!(status(&real), "NoSolutions");
    assert!(
        !diagnostics(&real).is_empty(),
        "the empty answer carries a note"
    );

    let complex = eval("x = symbol(\"x\"); solve_full(sin(x) == 2, x)");
    assert_eq!(status(&complex), "Unevaluated");
    assert!(
        !diagnostics(&complex).is_empty(),
        "the complex non-answer carries a note"
    );
}
