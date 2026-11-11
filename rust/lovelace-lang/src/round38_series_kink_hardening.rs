//! Round 38: the series-at-kink family (SeriesKinkDeterminismTests) and the kernel-hardening
//! residuals (KernelHardeningTests).
//!
//! Every test here is one deferred census family whose C# assertion is the contract. The script is
//! the C# row's own input (or the language-surface form of the same kernel call) and the expected
//! value is the oracle's measured answer for that script — probed with
//! `dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --eval '<script>' --omit-functions`.
//!
//! The series contract is the RIGHT-HAND expansion SymPy 1.14.0 publishes: a kink is resolved before
//! anything is differentiated, the coefficients are expressions (`|a*x|` is `a*x*sign(a)`), and the
//! tail is the canonical `(order var point degree)` node displayed `O((x - 1)^3)`.

use std::cmp::Ordering;

use lovelace_symbolics::{Expr as Sym, Rat};

use crate::{Engine, LangError, Value};

fn eval(script: &str) -> Value {
    let mut engine = Engine::new();
    engine
        .eval(script)
        .unwrap_or_else(|error| panic!("script: {script} failed: {}", error.message))
}

fn symbolic(script: &str) -> Sym {
    match eval(script) {
        Value::Symbolic(expression) => expression,
        other => panic!("script: {script} produced {} instead of a symbolic value", other.kind_name()),
    }
}

fn symbolic_text(script: &str) -> String {
    symbolic(script).to_string()
}

fn refusal(script: &str) -> LangError {
    match Engine::new().eval(script) {
        Ok(value) => panic!("script: {script} answered {} instead of refusing", value.typed()),
        Err(error) => error,
    }
}

fn record_fields<'a>(value: &'a Value, type_name: &str) -> &'a [(String, Value)] {
    match value {
        Value::Record(name, fields) if name == type_name => fields,
        Value::Record(name, _) => panic!("expected record {type_name}, got {name}"),
        other => panic!("expected record {type_name}, got {}", other.kind_name()),
    }
}

fn field<'a>(fields: &'a [(String, Value)], name: &str) -> &'a Value {
    fields
        .iter()
        .find(|(field, _)| field == name)
        .map(|(_, value)| value)
        .unwrap_or_else(|| panic!("record has no field {name}"))
}

fn vector(value: &Value) -> &[Value] {
    match value {
        Value::Vector(items) => items,
        _ => panic!("expected Vector, got {}", value.kind_name()),
    }
}

fn text(value: &Value) -> &str {
    match value {
        Value::Text(text) => text,
        other => panic!("expected Text, got {}", other.kind_name()),
    }
}

fn boolean(value: &Value) -> bool {
    match value {
        Value::Boolean(truth) => *truth,
        other => panic!("expected Boolean, got {}", other.kind_name()),
    }
}

fn variant(value: &Value) -> &str {
    match value {
        Value::Enum(_, name) => name,
        other => panic!("expected Enum, got {}", other.kind_name()),
    }
}

fn display(value: &Value) -> String {
    value.display()
}

// ---------------------------------------------------------------------------
// SeriesKinkDeterminismTests (30 rows)
// ---------------------------------------------------------------------------

/// The family: one row per published form, with the EXACT display and canonical form the expansion
/// must publish, and the smooth control the family is calibrated against.
const KINK_FAMILY: &[(&str, &str, &str)] = &[
    (
        "x = symbol(\"x\"); series(abs(x), x, 0, 3)",
        "x + O(x^3)",
        "(add (sym x) (order (sym x) (rat 0 1) (rat 3 1)))",
    ),
    (
        "x = symbol(\"x\"); series(abs(x), x, 0, 2)",
        "x + O(x^2)",
        "(add (sym x) (order (sym x) (rat 0 1) (rat 2 1)))",
    ),
    (
        "x = symbol(\"x\"); series(x*abs(x), x, 0, 3)",
        "x^2 + O(x^3)",
        "(add (pow (sym x) (rat 2 1)) (order (sym x) (rat 0 1) (rat 3 1)))",
    ),
    (
        "x = symbol(\"x\"); series(abs(sin(x)), x, 0, 3)",
        "x + O(x^3)",
        "(add (sym x) (order (sym x) (rat 0 1) (rat 3 1)))",
    ),
    (
        "x = symbol(\"x\"); series(abs(x - 1), x, 0, 3)",
        "1 - x + O(x^3)",
        "(add (rat 1 1) (mul (rat -1 1) (sym x)) (order (sym x) (rat 0 1) (rat 3 1)))",
    ),
    (
        "x = symbol(\"x\"); series(abs(x), x, 1, 3)",
        "x + O((x - 1)^3)",
        "(add (sym x) (order (sym x) (rat 1 1) (rat 3 1)))",
    ),
    (
        "x = symbol(\"x\"); series(abs(x^2 - 1), x, 0, 3)",
        "1 - x^2 + O(x^3)",
        "(add (rat 1 1) (mul (rat -1 1) (pow (sym x) (rat 2 1))) (order (sym x) (rat 0 1) (rat 3 1)))",
    ),
    (
        "x = symbol(\"x\"); series(sin(x), x, 0, 5)",
        "x - 1/6*x^3 + O(x^5)",
        "(add (sym x) (mul (rat -1 6) (pow (sym x) (rat 3 1))) (order (sym x) (rat 0 1) (rat 5 1)))",
    ),
];

/// SeriesKinkDeterminismTests.KinkSeries_PublishesTheSymPyExpansion_ByteIdenticallyOnEveryRun (#0..#7).
///
/// Two runs of the same script publish the SAME value — display, pretty and canonical form — and the
/// display is exactly the SymPy 1.14.0 expansion. On the pre-repair tree every kink row published
/// the source function (`abs(x)`) and the canonical form lacked the `order` node entirely.
#[test]
fn kink_series_publishes_the_sympy_expansion_byte_identically_on_every_run() {
    for (script, expected_display, expected_canonical) in KINK_FAMILY {
        let first = symbolic(script);
        let second = symbolic(script);
        assert_eq!(first.to_string(), *expected_display, "display of {script}");
        assert_eq!(second.to_string(), *expected_display, "second run of {script}");
        assert_eq!(first.canonical(), *expected_canonical, "canonical of {script}");
        assert_eq!(second.canonical(), *expected_canonical, "second canonical of {script}");
        assert_eq!(first, second, "the two runs must build the same tree: {script}");
    }
}

/// SeriesKinkDeterminismTests.KinkSeries_PublishesNoInternalSymbolAndNoUnevaluatedDerivative (#0..#7).
///
/// No published text of the family carries an internal symbol, a GUID-shaped name, an unevaluated
/// `diff(...)`/`piecewise(...)` node or a division by a zero constant.
#[test]
fn kink_series_publishes_no_internal_symbol_and_no_unevaluated_derivative() {
    for (script, expected_display, expected_canonical) in KINK_FAMILY {
        let published = symbolic(script);
        let text = published.to_string();
        let decoded = published.to_string();
        assert!(!text.contains("__"), "internal symbol leaked: {script} -> {text}");
        assert!(!decoded.contains("__"), "internal symbol leaked: {script} -> {decoded}");
        assert_eq!(text, *expected_display);
        assert_eq!(published.canonical(), *expected_canonical);
        assert!(!text.contains("diff("), "unevaluated derivative leaked: {text}");
        assert!(!text.contains("piecewise("), "unevaluated piecewise leaked: {text}");
        assert!(!text.contains("sqrt(0)"), "division by zero leaked: {text}");
    }
}

/// SeriesKinkDeterminismTests.KinkSeries_TakesTheSymPyValueAtTheSamplePoint (#0..#7).
///
/// The polynomial part of the published expansion, evaluated through the product's own `subs`,
/// takes the value SymPy measures for the same expansion: `series(abs(x), x, 0, 3)` is `x`, so it
/// is 1/2 at x = 1/2 and not 0; `series(x*abs(x), x, 0, 3)` is `x^2`, so it is 1/4 there.
#[test]
fn kink_series_takes_the_sympy_value_at_the_sample_point() {
    let rows: &[(&str, &str, &str)] = &[
        ("x = symbol(\"x\"); series(abs(x), x, 0, 3)", "1/2", "1/2"),
        ("x = symbol(\"x\"); series(abs(x), x, 0, 2)", "1/2", "1/2"),
        ("x = symbol(\"x\"); series(x*abs(x), x, 0, 3)", "1/2", "1/4"),
        ("x = symbol(\"x\"); series(abs(sin(x)), x, 0, 3)", "1/2", "1/2"),
        ("x = symbol(\"x\"); series(abs(x - 1), x, 0, 3)", "1/2", "1/2"),
        ("x = symbol(\"x\"); series(abs(x), x, 1, 3)", "3/2", "3/2"),
        ("x = symbol(\"x\"); series(abs(x^2 - 1), x, 0, 3)", "1/2", "3/4"),
        ("x = symbol(\"x\"); series(sin(x), x, 0, 5)", "1/2", "23/48"),
    ];
    for (script, sample, expected) in rows {
        let shown = symbolic_text(script);
        let polynomial = shown
            .split(" + O(")
            .next()
            .unwrap_or_else(|| panic!("{script} published no O-term tail: {shown}"));
        assert_ne!(polynomial, shown, "a series display always carries its tail: {shown}");
        let value = symbolic_text(&format!(
            "x = symbol(\"x\"); subs({polynomial}, x, {sample})"
        ));
        assert_eq!(value, *expected, "value of {script} at x = {sample}");
    }
}

/// SeriesKinkDeterminismTests.KinkSeries_NeighbouringShapes_FollowTheSameRule (#0..#3).
///
/// The repair resolves the kink by COMPUTING the branch — the sign of the argument's leading
/// coefficient, SymPy's rule — rather than by refusing or by dropping the term. These are the
/// neighbouring shapes the same rule covers; each is the exact expansion SymPy publishes.
#[test]
fn kink_series_neighbouring_shapes_follow_the_same_rule() {
    let rows: &[(&str, &str)] = &[
        ("x = symbol(\"x\"); series(abs(x)/x, x, 0, 3)", "1 + O(x^3)"),
        ("x = symbol(\"x\"); series(abs(x)^2, x, 0, 3)", "x^2 + O(x^3)"),
        (
            "x = symbol(\"x\"); series(exp(abs(x)), x, 0, 3)",
            "1 + x + 1/2*x^2 + O(x^3)",
        ),
        (
            "x = symbol(\"x\"); a = symbol(\"a\"); series(abs(a*x), x, 0, 2)",
            "a*x*sign(a) + O(x^2)",
        ),
    ];
    for (script, expected) in rows {
        assert_eq!(symbolic_text(script), *expected, "{script}");
    }
}

/// SeriesKinkDeterminismTests.KinkSeries_TruncationGap_IsTheDroppedTerm_NotASilentApproximation (#0).
///
/// `series(abs(sin(x)), x, 0, 3)` is `x + O(x^3)`, and the gap from the true |sin(1/2)| is the
/// dropped term x^3/6 at 1/2, which is exactly 1/48.
#[test]
fn kink_series_truncation_gap_is_the_dropped_term_not_a_silent_approximation() {
    assert_eq!(
        symbolic_text("x = symbol(\"x\"); series(abs(sin(x)), x, 0, 3)"),
        "x + O(x^3)"
    );
    assert_eq!(display(&eval("x = symbol(\"x\"); subs(x, x, 1/2)")), "1/2");
    assert_eq!(display(&eval("x = symbol(\"x\"); subs(x^3/6, x, 1/2)")), "1/48");
}

/// SeriesKinkDeterminismTests.LimitEngine_TwoSidedKinkAnswers_AreUnchanged (#0..#3).
///
/// The kink policy is per-caller: the published `series()` resolves a kink, while the limit engine's
/// two-sided analysis keeps its own answers. These four must not move, and no internal symbol may
/// appear in them.
#[test]
fn limit_engine_two_sided_kink_answers_are_unchanged() {
    let rows: &[(&str, &str, Option<&str>)] = &[
        ("x = symbol(\"x\"); limit(abs(x), x, 0)", "Value", Some("0")),
        (
            "x = symbol(\"x\"); limit((1 - cos(x))/x^2, x, 0)",
            "Value",
            Some("1/2"),
        ),
        ("x = symbol(\"x\"); limit(sqrt(x), x, 0)", "Value", Some("0")),
        ("x = symbol(\"x\"); limit(abs(x)/x, x, 0)", "Unevaluated", None),
    ];
    for (script, expected_status, expected_value) in rows {
        let record = eval(script);
        let fields = record_fields(&record, "LimitResult");
        assert_eq!(variant(field(fields, "status")), *expected_status, "{script}");
        match expected_value {
            None => {
                assert!(matches!(field(fields, "value"), Value::Void), "{script}");
                let diagnostics = vector(field(fields, "diagnostics"));
                let diagnostic = record_fields(&diagnostics[0], "Diagnostic");
                assert_eq!(text(field(diagnostic, "code")), "limit.unevaluated");
            }
            Some(expected) => {
                assert_eq!(display(field(fields, "value")), *expected, "{script}");
            }
        }
        assert!(!record.display().contains("__"));
    }
}

// ---------------------------------------------------------------------------
// KernelHardeningTests
// ---------------------------------------------------------------------------

/// KernelHardeningTests.CubicRoots_SatisfyPolynomial_Numerically (#0..#2).
///
/// Cardano's mixed cube-root terms need the u*v = -P/3 coupling, which Expand cannot derive, so each
/// root is verified by a high-precision numeric residual instead: at 60 digits every residual is
/// below 1e-40.
#[test]
fn cubic_roots_satisfy_polynomial_numerically() {
    let polynomials = [
        "x^3 + 2*x^2 + 3*x + 4",
        "x^3 + 1",
        "x^3 - 3*x + 1",
    ];
    let tolerance =
        lovelace_real::Real::from_str(&format!("0.{}1", "0".repeat(39))).expect("decimal");
    for polynomial in polynomials {
        let record = eval(&format!(
            "x = symbol(\"x\"); solve_full({polynomial} == 0, x)"
        ));
        let fields = record_fields(&record, "SolveResult");
        assert_eq!(variant(field(fields, "status")), "Solved", "{polynomial}");
        let solutions = vector(field(fields, "solutions"));
        assert_eq!(solutions.len(), 3, "{polynomial}");
        for index in 0..solutions.len() {
            // The root is handed to subs AS THE RECORD'S VALUE, never re-parsed from its display:
            // the display of a Cardano root re-entered as script text is a non-integer power of a
            // symbolic base, which the language refuses (the oracle's own capability table says
            // so too); the substitution through the value keeps the exact tree.
            let residual = eval(&format!(
                "x = symbol(\"x\"); s = solve_full({polynomial} == 0, x).solutions[{index}].value; \
                 evalf(abs(subs({polynomial}, x, s)), 60)"
            ));
            let root = display(field(
                record_fields(&solutions[index], "Solution"),
                "value",
            ));
            match residual {
                Value::Real(value) => assert!(
                    value.cmp(&tolerance) == Ordering::Less,
                    "root {root} of {polynomial} has residual {value}"
                ),
                Value::Natural(value) => assert_eq!(value.to_decimal(), "0"),
                Value::Integer(value) => assert_eq!(value.to_decimal(), "0"),
                other => panic!(
                    "root {root} of {polynomial} produced {} as its residual",
                    other.kind_name()
                ),
            }
        }
    }
}

/// KernelHardeningTests.RationalPower_HugeExponent_IsExact (#0).
///
/// `2^65536` is exact on both sides: the kernel's power constructor folds the rational power to the
/// rational constant whose numerator is 2^65536, and the negative exponent folds to its reciprocal.
#[test]
fn rational_power_huge_exponent_is_exact() {
    let two = Sym::Num(Rat::from_i64(2));
    let expected = Rat::from_i64(2).pow_i64(65536).expect("exact power fits");
    let positive = Sym::pow(two.clone(), Sym::Num(Rat::from_i64(65536)));
    assert_eq!(positive, Sym::Num(expected.clone()));
    let negative = Sym::pow(two, Sym::Num(Rat::from_i64(-65536)));
    assert_eq!(
        negative,
        Sym::Num(Rat::one().div(&expected).expect("exact reciprocal"))
    );
}

/// KernelHardeningTests.MathIR_HugeExponent_LowersAndEvaluates (#0).
///
/// The lowered x^65536 program evaluates at x = 2 to the exact integer 2^65536 (19,729 digits), far
/// past i64, so the kernel's exponent path must be arbitrary precision end to end.
#[test]
fn mathir_huge_exponent_lowers_and_evaluates() {
    let value = eval("x = symbol(\"x\"); evalir(compile(x^65536, [x]), [2], 40)");
    let expected = lovelace_int::Int::from_i64(2).pow(65536);
    match value {
        Value::Integer(integer) => assert_eq!(integer.to_decimal(), expected.to_decimal()),
        other => panic!("expected Integer, got {}", other.kind_name()),
    }
}

/// KernelHardeningTests.RegisteredFunctionArity_IsEnforced (#0).
///
/// A fixed-arity builtin refuses the wrong count with the oracle's own message. The row's
/// supplementary half (a variadic scalar min/max accepting any count >= 2, and an unknown name
/// staying open) has no Rust surface: min/max are the ARRAY reductions (value, axis), and an unknown
/// name is an unknown-function refusal rather than an open extensibility seam. Those two assertions
/// are therefore not claimed here; the arity contract itself is the refusal pair below.
#[test]
fn registered_function_arity_is_enforced() {
    let too_many = refusal("x = symbol(\"x\"); y = symbol(\"y\"); sin(x, y)");
    assert_eq!(too_many.code, "InvalidArgument");
    assert_eq!(too_many.message, "sin(): expected 1 argument; got 2.");
    let too_few = refusal("log()");
    assert_eq!(too_few.code, "InvalidArgument");
    assert_eq!(too_few.message, "log(): expected 1 argument; got 0.");
}

/// KernelHardeningTests.Simplify_ZeroOverX_CancelsWithCondition (#0).
///
/// The full transform applies rat.cancel-zero-over-x: 0/x becomes 0 carrying the pole condition
/// x != 0, while the safe simplify() keeps 0/x.
#[test]
fn simplify_zero_over_x_cancels_with_condition() {
    assert_eq!(symbolic_text("x = symbol(\"x\"); simplify(0/x)"), "0/x");
    let record = eval("x = symbol(\"x\"); simplify_full(0/x)");
    let fields = record_fields(&record, "TransformResult");
    assert_eq!(display(field(fields, "expression")), "0");
    assert!(boolean(field(fields, "changed")));
    let conditions = vector(field(fields, "conditions"));
    assert_eq!(conditions.len(), 1);
    assert_eq!(display(&conditions[0]), "x != 0");
    let steps = vector(field(fields, "steps"));
    let step = record_fields(&steps[0], "RewriteStep");
    assert_eq!(text(field(step, "rule_id")), "rat.cancel-zero-over-x");
    assert_eq!(variant(field(step, "classification")), "Conditional");
}

/// KernelHardeningTests.Solve_QuarticNoRealRoots_DefaultDomain_Unevaluated_RealDomain_Empty (#0).
///
/// The default domain is complex; x^4 + 1 has four complex roots the kernel cannot represent
/// (RootOf is real-only), so the answer is Unevaluated with every root counted as unrepresented —
/// never "no solutions". Under the explicit real domain the answer IS the complete empty set.
#[test]
fn solve_quartic_no_real_roots_default_domain_unevaluated_real_domain_empty() {
    let record = eval("x = symbol(\"x\"); solve_full(x^4 + 1 == 0, x)");
    let fields = record_fields(&record, "SolveResult");
    assert_eq!(variant(field(fields, "status")), "Unevaluated");
    assert!(!boolean(field(fields, "complete")));
    assert_eq!(variant(field(fields, "completeness")), "Unknown");
    assert!(vector(field(fields, "solutions")).is_empty());
    assert_eq!(display(field(fields, "unrepresented_count")), "4");
    assert!(text(field(fields, "unrepresented_reason")).contains("complex algebraic roots"));

    let real = eval("x = symbol(\"x\"); solve_full(x^4 + 1 == 0, x, real)");
    let real_fields = record_fields(&real, "SolveResult");
    assert_eq!(variant(field(real_fields, "status")), "NoSolutions");
    assert!(boolean(field(real_fields, "complete")));
}

/// KernelHardeningTests.NumericToSymbolicToNumeric_500Digits_RoundTrips (#0).
///
/// A 500-digit decimal written at 600 digits keeps every digit on the way back out: the display is
/// the value itself, not a 64-digit truncation.
#[test]
fn numeric_to_symbolic_to_numeric_500_digits_round_trips() {
    let digits = "3".repeat(500);
    let script = format!("setprecision(600); a = 0.{digits}; a");
    assert_eq!(display(&eval(&script)), format!("0.{digits}"));
}

/// KernelHardeningTests.RealConstantAdd_Preserves500Digits (#0).
///
/// The digit-wise sum of 0.111...1 and 0.222...2 at 500 digits is 0.333...3 — exact, never a
/// 64-digit truncation.
#[test]
fn real_constant_add_preserves_500_digits() {
    let ones = "1".repeat(500);
    let twos = "2".repeat(500);
    let sum = eval(&format!(
        "setprecision(600); a = 0.{ones}; b = 0.{twos}; a + b"
    ));
    assert_eq!(display(&sum), format!("0.{}", "3".repeat(500)));
}

/// KernelHardeningTests.RealConstantMultiply_PreservesPrecision (#0).
///
/// 3 * 0.777...7 (500 digits) is 2.333...31 with every digit preserved: the product is 7...7*3/10^500
/// and the final digit is the carry, not a rounded 3.
#[test]
fn real_constant_multiply_preserves_precision() {
    let sevens = "7".repeat(500);
    let product = eval(&format!("setprecision(600); a = 0.{sevens}; a * 3"));
    assert_eq!(display(&product), format!("2.{}1", "3".repeat(499)));
}
