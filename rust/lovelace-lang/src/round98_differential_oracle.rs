//! Round 98 — the six DifferentialOracle census rows.
//!
//! The C# rows compare the kernel against a LIVE SymPy 1.14.0 session (DifferentialOracleTests.cs +
//! OracleCorpus.cs + SympyOracle.cs). The Rust tests below embed that session's ground truth as
//! literal expected values (generated once by rust/target/tmp/differential_oracle.py; Python is
//! NOT a runtime dependency) and apply the SAME comparison rule:
//!
//! * derivatives, determinants, traces and inverse entries: canonical-print then reparse, numeric
//!   evaluation at the same rational sample points, real AND imaginary parts compared at 1e-13;
//! * solve: distinct-root count plus exact substitution back into the equation;
//! * roots: distinct values compared at 1e-20, plus exact substitution for closed forms;
//! * factorizations: the sorted degree:multiplicity profile and the expansion identity;
//! * limits: the value at 1e-20, or the exact infinity/DoesNotExist status with both sides.
//!
//! Census rows (one test each):
//! * Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_Derivatives_Agree#0
//! * Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_Solve_Agrees#0
//! * Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_Roots_Agree#0
//! * Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_Factorization_Agrees#0
//! * Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_Limits_Agree#0
//! * Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_MatrixOperations_Agree#0

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};

use lovelace_int::Int;
use lovelace_nat::Nat;
use lovelace_real::Real;
use lovelace_symbolics::solution_set::{SolveDomain, SolveStatus, solve_expression};
use lovelace_symbolics::symbolic_matrix::SymbolicMatrix;
use lovelace_symbolics::{Expr as Sym, Rat};

use crate::numeric::{self, Num};
use crate::{Engine, Value};

/// The derivative/matrix tolerance from OracleCompare.RealTolerance.
const REAL_TOLERANCE: &str = "0.0000000000001";

/// The roots/limits tolerance from OracleCompare.TightTolerance (1e-20).
const TIGHT_TOLERANCE: &str = "0.00000000000000000001";

fn eval(source: &str) -> Value {
    let mut engine = Engine::new();
    engine
        .eval(source)
        .unwrap_or_else(|error| panic!("`{source}` failed: {}", error.message))
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    match value {
        Value::Record(_, fields) => fields
            .iter()
            .find(|(candidate, _)| candidate == name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("record has no field `{name}`: {value:?}")),
        other => panic!("expected a record, got {other:?}"),
    }
}

fn enum_member(value: &Value) -> &str {
    match value {
        Value::Enum(_, member) => member,
        other => panic!("expected an enum, got {other:?}"),
    }
}

fn symbolic(value: &Value) -> &Sym {
    match value {
        Value::Symbolic(expression) => expression,
        other => panic!("expected a symbolic value, got {other:?}"),
    }
}

fn rat(text: &str) -> Rat {
    match text.split_once('/') {
        Some((numerator, denominator)) => Rat::from_parts(
            Int::from_decimal(numerator).expect("a decimal numerator"),
            Nat::from_decimal(denominator).expect("a decimal denominator"),
        ),
        None => Rat::from_i64(text.parse().expect("an integer")),
    }
}

fn rat_real(value: &Rat) -> Real {
    Num::Exact(value.clone())
        .to_real(50)
        .expect("a rational converts to Real")
}

fn components(value: &Num) -> (Real, Real) {
    match value {
        Num::Exact(value) => (rat_real(value), Real::zero()),
        Num::Real(value) => (value.clone(), Real::zero()),
        Num::Complex(value) => (value.re().clone(), value.im().clone()),
    }
}

fn show(value: &Num) -> String {
    let (real, imaginary) = components(value);
    format!(
        "{} + {}i",
        real.to_decimal_fixed(30),
        imaginary.to_decimal_fixed(30)
    )
}

/// The C# NumberAgrees comparison: real and imaginary parts each under the tolerance.
fn assert_number_agrees(
    actual: &Num,
    expected_real: &str,
    expected_imaginary: &str,
    tolerance: &str,
    label: &str,
) {
    let (real, imaginary) = components(actual);
    let expected_real = Real::from_str(expected_real).expect("a decimal literal");
    let expected_imaginary = Real::from_str(expected_imaginary).expect("a decimal literal");
    let allowed = Real::from_str(tolerance).expect("a decimal tolerance");
    let real_difference = real.sub(&expected_real).abs();
    let imaginary_difference = imaginary.sub(&expected_imaginary).abs();
    assert!(
        real_difference.cmp(&allowed) == Ordering::Less,
        "{label}: MISMATCH — kernel {} vs sympy {} (real part)",
        show(actual),
        expected_real.to_decimal_fixed(30)
    );
    assert!(
        imaginary_difference.cmp(&allowed) == Ordering::Less,
        "{label}: MISMATCH — kernel {} vs sympy {}i (imaginary part)",
        show(actual),
        expected_imaginary.to_decimal_fixed(30)
    );
}

fn assert_real_agrees(actual: &Num, expected: &str, tolerance: &str, label: &str) {
    assert_number_agrees(actual, expected, "0", tolerance, label);
}

fn evaluate_at(expression: &Sym, bindings: &[(&str, &str)]) -> Num {
    let mut map: HashMap<String, Num> = HashMap::new();
    for (name, value) in bindings {
        map.insert((*name).to_string(), Num::Exact(rat(value)));
    }
    numeric::evaluate_bound(expression, &map, 50)
        .unwrap_or_else(|error| panic!("evaluation failed: {}", error.message))
}

fn expr(source: &str) -> Sym {
    let script = format!("x = symbol(\"x\"); y = symbol(\"y\"); {source}");
    match eval(&script) {
        Value::Symbolic(expression) => expression,
        Value::Integer(value) => {
            Sym::Num(Rat::from_i64(value.to_decimal().parse().expect("an i64")))
        }
        Value::Natural(value) => Sym::Num(Rat::from_parts(
            Int::from_decimal(&value.to_decimal()).expect("a decimal numerator"),
            Nat::from_u64(1),
        )),
        Value::Real(value) => Sym::Real(value.to_decimal_fixed(30)),
        other => panic!("`{source}` is not an expression: {other:?}"),
    }
}

fn matrix(rows: &[&[&str]]) -> SymbolicMatrix {
    SymbolicMatrix::from_rows(
        rows.iter()
            .map(|row| row.iter().map(|cell| expr(cell)).collect())
            .collect(),
    )
    .expect("a rectangular matrix")
}

/// The C# OracleCompare path for a closed-form kernel value: canonical print, reparse, evaluate.
fn reparse(expression: &Sym) -> Sym {
    let printed = expression.canonical();
    lovelace_symbolics::canonical_parse(&printed)
        .unwrap_or_else(|error| panic!("canonical form {printed} did not parse back: {error}"))
}

fn numeric_solution(value: &Sym) -> Num {
    match value {
        Sym::Fun(name, _) if name == "rootof" => Num::Exact(
            lovelace_symbolics::evalf_rootof(value, 30).expect("the RootOf numeric value"),
        ),
        _ => numeric::evaluate(value, 40)
            .unwrap_or_else(|error| panic!("solution did not evaluate: {}", error.message)),
    }
}

fn deduplicate_within(values: Vec<Num>, tolerance: &str) -> Vec<Num> {
    let allowed = Real::from_str(tolerance).expect("a decimal tolerance");
    let mut kept: Vec<Num> = Vec::new();
    for value in values {
        let duplicate = kept.iter().any(|previous| {
            let (previous_real, previous_imaginary) = components(previous);
            let (real, imaginary) = components(&value);
            previous_real.sub(&real).abs().cmp(&allowed) == Ordering::Less
                && previous_imaginary.sub(&imaginary).abs().cmp(&allowed) == Ordering::Less
        });
        if !duplicate {
            kept.push(value);
        }
    }
    kept
}

fn residual_is_zero(equation: &Sym, root: &Sym, label: &str) {
    let at = lovelace_symbolics::substitute(equation, "x", root);
    let expanded = lovelace_symbolics::expand(&at);
    assert!(
        matches!(&expanded, Sym::Num(value) if value.is_zero()),
        "{label}: kernel root {} does not satisfy the equation (expanded residual {})",
        root.canonical(),
        expanded.canonical()
    );
}

// ---------------------------------------------------------------------------
// SympyOracle_Derivatives_Agree#0
// ---------------------------------------------------------------------------

struct DerivativeCase {
    expression: &'static str,
    /// (point, sympy real part, sympy imaginary part), N(..., 30).
    points: &'static [(&'static str, &'static str, &'static str)],
}

const DERIVATIVES: &[DerivativeCase] = &[
    DerivativeCase {
        expression: "x^7",
        points: &[
            ("1/3", "0.00960219478737997256515775034294", "0"),
            ("-1/2", "0.109375000000000000000000000000", "0"),
            ("7/5", "52.7067520000000000000000000000", "0"),
        ],
    },
    DerivativeCase {
        expression: "sin(x^2)",
        points: &[
            ("1/3", "0.662555672359261280868626202047", "0"),
            ("-1/2", "-0.968912421710644784144595449494", "0"),
            ("7/5", "-1.06246494140683265580838260325", "0"),
        ],
    },
    DerivativeCase {
        expression: "exp(-x^2)",
        points: &[
            ("1/3", "-0.596559544542913183054293028847", "0"),
            ("-1/2", "0.778800783071404868245170266978", "0"),
            ("7/5", "-0.394403578578925989214320091069", "0"),
        ],
    },
    DerivativeCase {
        expression: "x*log(x)",
        points: &[
            ("1/3", "-0.0986122886681096913952452369225", "0"),
            (
                "-1/2",
                "0.306852819440054690582767878542",
                "3.14159265358979323846264338328",
            ),
            ("7/5", "1.33647223662121293050459341022", "0"),
        ],
    },
    DerivativeCase {
        expression: "sin(x)/x",
        points: &[
            ("1/3", "-0.109881432221157204395244744381", "0"),
            ("-1/2", "0.162537030636066568860588575655", "0"),
            ("7/5", "-0.381375372412307584997973127307", "0"),
        ],
    },
    DerivativeCase {
        expression: "tan(x^3)",
        points: &[
            ("1/3", "0.333790999179746492481090015847", "0"),
            ("-1/2", "0.761841910010786112211782624635", "0"),
            ("7/5", "6.91702668304069031278752914719", "0"),
        ],
    },
];

#[test]
fn sympyoracle_derivatives_agree() {
    for case in DERIVATIVES {
        let derivative = lovelace_symbolics::diff(&expr(case.expression), "x");
        let reparsed = reparse(&derivative);
        for (point, expected_real, expected_imaginary) in case.points {
            let value = evaluate_at(&reparsed, &[("x", point)]);
            assert_number_agrees(
                &value,
                expected_real,
                expected_imaginary,
                REAL_TOLERANCE,
                &format!("diff({}) at x = {point}", case.expression),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// SympyOracle_Solve_Agrees#0
// ---------------------------------------------------------------------------

struct SolveCase {
    expression: &'static str,
    /// sympy.solve's distinct-root count.
    count: usize,
    /// The C# corpus declares Unevaluated exactly where sympy's set is empty.
    declared_unevaluated: bool,
}

const SOLVE_CASES: &[SolveCase] = &[
    SolveCase {
        expression: "x^2 - 4*x + 4",
        count: 1,
        declared_unevaluated: false,
    },
    SolveCase {
        expression: "x^2 - 5*x + 6",
        count: 2,
        declared_unevaluated: false,
    },
    SolveCase {
        expression: "x^3 + 1",
        count: 3,
        declared_unevaluated: false,
    },
    SolveCase {
        expression: "x^4 - 5*x^2 + 4",
        count: 4,
        declared_unevaluated: false,
    },
    SolveCase {
        expression: "x^2 + 1",
        count: 2,
        declared_unevaluated: false,
    },
    SolveCase {
        expression: "x^2 + 4",
        count: 2,
        declared_unevaluated: false,
    },
    SolveCase {
        expression: "sqrt(x) + 2",
        count: 0,
        declared_unevaluated: true,
    },
];

#[test]
fn sympyoracle_solve_agrees() {
    for case in SOLVE_CASES {
        let equation = expr(case.expression);
        let set = solve_expression(&equation, "x", SolveDomain::Complex);
        let label = format!("solve({})", case.expression);
        if case.declared_unevaluated {
            assert_eq!(
                set.status,
                SolveStatus::Unevaluated,
                "{label}: the kernel answered {:?}, not the declared non-answer",
                set.status
            );
            assert!(
                !set.completeness_pair().0,
                "{label}: a declared non-answer must not claim completeness"
            );
        } else {
            assert_eq!(
                set.status,
                SolveStatus::Solved,
                "{label}: the kernel answered {:?}",
                set.status
            );
        }
        let distinct: BTreeSet<String> = set
            .solutions
            .iter()
            .map(|solution| solution.canonical())
            .collect();
        assert_eq!(
            distinct.len(),
            case.count,
            "{label}: kernel {} distinct root(s) vs sympy {}",
            distinct.len(),
            case.count
        );
        for solution in &set.solutions {
            residual_is_zero(&equation, solution, &label);
        }
    }
}

// ---------------------------------------------------------------------------
// SympyOracle_Roots_Agree#0
// ---------------------------------------------------------------------------

struct RootsCase {
    expression: &'static str,
    /// sympy.real_roots, distinct values at 30 significant digits.
    values: &'static [&'static str],
}

const ROOTS_CASES: &[RootsCase] = &[
    RootsCase {
        expression: "x^4 - 5*x^2 + 4",
        values: &[
            "-2.00000000000000000000000000000",
            "-1.00000000000000000000000000000",
            "1.00000000000000000000000000000",
            "2.00000000000000000000000000000",
        ],
    },
    RootsCase {
        expression: "x^2 - 2",
        values: &[
            "-1.41421356237309504880168872421",
            "1.41421356237309504880168872421",
        ],
    },
    RootsCase {
        expression: "x^4 - 2",
        values: &[
            "-1.18920711500272106671749997056",
            "1.18920711500272106671749997056",
        ],
    },
    RootsCase {
        expression: "x^5 - 3*x + 1",
        values: &[
            "-1.38879198440725418280005669419",
            "0.334734141943352687075098962473",
            "1.21464804269846180398582838932",
        ],
    },
    RootsCase {
        expression: "x^3 - 2",
        values: &["1.25992104989487316476721060728"],
    },
    RootsCase {
        expression: "x^3 - 3*x^2 + 3*x - 1",
        values: &["1.00000000000000000000000000000"],
    },
    RootsCase {
        expression: "x^5 - x - 1",
        values: &["1.16730397826141868425604589985"],
    },
    RootsCase {
        expression: "x^2 + 1",
        values: &[],
    },
];

#[test]
fn sympyoracle_roots_agree() {
    for case in ROOTS_CASES {
        let equation = expr(case.expression);
        let set = solve_expression(&equation, "x", SolveDomain::Real);
        let label = format!("real_roots({})", case.expression);
        assert!(
            matches!(set.status, SolveStatus::Solved | SolveStatus::NoSolutions),
            "{label}: the kernel made no definite real-root claim (status {:?})",
            set.status
        );
        let numeric: Vec<Num> = set.solutions.iter().map(numeric_solution).collect();
        let values = deduplicate_within(numeric, TIGHT_TOLERANCE);
        assert_eq!(
            values.len(),
            case.values.len(),
            "{label}: kernel {} distinct real root(s) vs sympy {}",
            values.len(),
            case.values.len()
        );
        for (index, (actual, expected)) in values.iter().zip(case.values.iter()).enumerate() {
            assert_real_agrees(
                actual,
                expected,
                TIGHT_TOLERANCE,
                &format!("{label}: root {index}"),
            );
        }
        for solution in &set.solutions {
            if !matches!(solution, Sym::Fun(name, _) if name == "rootof") {
                residual_is_zero(&equation, solution, &label);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SympyOracle_Factorization_Agrees#0
// ---------------------------------------------------------------------------

struct FactorCase {
    expression: &'static str,
    /// sympy.factor_list's sorted ddd:ddd degree:multiplicity profile.
    profile: &'static str,
}

const FACTOR_CASES: &[FactorCase] = &[
    FactorCase {
        expression: "x^2 - 5*x + 6",
        profile: "001:001;001:001",
    },
    FactorCase {
        expression: "x^3 + 1",
        profile: "001:001;002:001",
    },
    FactorCase {
        expression: "x^4 - 5*x^2 + 4",
        profile: "001:001;001:001;001:001;001:001",
    },
    FactorCase {
        expression: "x^3 - 3*x^2 + 3*x - 1",
        profile: "001:003",
    },
    FactorCase {
        expression: "2*x^2 - 8",
        profile: "001:001;001:001",
    },
    FactorCase {
        expression: "x^4 - 1",
        profile: "001:001;001:001;002:001",
    },
    FactorCase {
        expression: "x^2 - 2",
        profile: "002:001",
    },
    FactorCase {
        expression: "6*x^3 - 6*x",
        profile: "001:001;001:001;001:001",
    },
    FactorCase {
        expression: "x^3 + x^2 - x - 1",
        profile: "001:001;001:002",
    },
];

fn degree_of(expression: &Sym, var: &str) -> i64 {
    lovelace_symbolics::poly_coeffs(expression, var)
        .map(|coeffs| coeffs.len() as i64 - 1)
        .unwrap_or(-999)
}

fn degree_profile(factored: &Sym, var: &str) -> String {
    let mut parts = Vec::new();
    fn visit(expression: &Sym, var: &str, parts: &mut Vec<String>) {
        match expression {
            Sym::Mul(factors) => {
                for factor in factors {
                    visit(factor, var, parts);
                }
            }
            Sym::Pow(base, exponent) => {
                let power = match exponent.as_ref() {
                    Sym::Num(value) if value.is_integer() => value.to_i64(),
                    _ => None,
                };
                if let Some(power) = power.filter(|power| *power > 1) {
                    parts.push(format!("{:03}:{:03}", degree_of(base, var), power));
                    return;
                }
                parts.push(format!("{:03}:{:03}", degree_of(expression, var), 1));
            }
            Sym::Num(_) | Sym::Real(_) | Sym::Complex(_, _) => {}
            other => parts.push(format!("{:03}:{:03}", degree_of(other, var), 1)),
        }
    }
    visit(factored, var, &mut parts);
    parts.sort();
    parts.join(";")
}

#[test]
fn sympyoracle_factorization_agrees() {
    for case in FACTOR_CASES {
        let original = expr(case.expression);
        let factored = expr(&format!("factor({})", case.expression));
        let label = format!("factor({})", case.expression);
        assert_eq!(
            degree_profile(&factored, "x"),
            case.profile,
            "{label}: kernel factor degree profile differs from sympy.factor_list"
        );
        assert_eq!(
            lovelace_symbolics::expand(&factored).canonical(),
            lovelace_symbolics::expand(&original).canonical(),
            "{label}: the kernel factorization does not expand back to the input"
        );
    }
}

// ---------------------------------------------------------------------------
// SympyOracle_Limits_Agree#0
// ---------------------------------------------------------------------------

enum LimitExpectation {
    /// A finite value at 30 significant digits.
    Finite(&'static str),
    PlusInfinity,
    MinusInfinity,
    /// The two sides disagree: left -inf, right +inf.
    DoesNotExist,
}

struct LimitCase {
    expression: &'static str,
    point: &'static str,
    direction: &'static str,
    expectation: LimitExpectation,
}

const LIMIT_CASES: &[LimitCase] = &[
    LimitCase {
        expression: "sin(x)/x",
        point: "0",
        direction: "two",
        expectation: LimitExpectation::Finite("1.00000000000000000000000000000"),
    },
    LimitCase {
        expression: "(1 - cos(x))/x^2",
        point: "0",
        direction: "two",
        expectation: LimitExpectation::Finite("0.500000000000000000000000000000"),
    },
    LimitCase {
        expression: "(x^3 - 8)/(x - 2)",
        point: "2",
        direction: "two",
        expectation: LimitExpectation::Finite("12.0000000000000000000000000000"),
    },
    LimitCase {
        expression: "(x^2 - 1)/(x - 1)",
        point: "1",
        direction: "two",
        expectation: LimitExpectation::Finite("2.00000000000000000000000000000"),
    },
    LimitCase {
        expression: "tan(x)/x",
        point: "0",
        direction: "two",
        expectation: LimitExpectation::Finite("1.00000000000000000000000000000"),
    },
    LimitCase {
        expression: "1/x^2",
        point: "0",
        direction: "two",
        expectation: LimitExpectation::PlusInfinity,
    },
    LimitCase {
        expression: "1/(x - 1)^2",
        point: "1",
        direction: "two",
        expectation: LimitExpectation::PlusInfinity,
    },
    LimitCase {
        expression: "1/x",
        point: "0",
        direction: "right",
        expectation: LimitExpectation::PlusInfinity,
    },
    LimitCase {
        expression: "1/x",
        point: "0",
        direction: "left",
        expectation: LimitExpectation::MinusInfinity,
    },
    LimitCase {
        expression: "1/x",
        point: "0",
        direction: "two",
        expectation: LimitExpectation::DoesNotExist,
    },
    LimitCase {
        expression: "1/(x - 1)",
        point: "1",
        direction: "two",
        expectation: LimitExpectation::DoesNotExist,
    },
    LimitCase {
        expression: "(3*x^2 + 5*x)/(2*x^2 - x)",
        point: "inf",
        direction: "two",
        expectation: LimitExpectation::Finite("1.50000000000000000000000000000"),
    },
    LimitCase {
        expression: "(3*x^2 + 5*x)/(2*x^2 - x)",
        point: "-inf",
        direction: "two",
        expectation: LimitExpectation::Finite("1.50000000000000000000000000000"),
    },
    LimitCase {
        expression: "x/(x^2 + 1)",
        point: "inf",
        direction: "two",
        expectation: LimitExpectation::Finite("0"),
    },
    LimitCase {
        expression: "(2*x^3 - x)/(x^3 + 1)",
        point: "inf",
        direction: "two",
        expectation: LimitExpectation::Finite("2.00000000000000000000000000000"),
    },
    LimitCase {
        expression: "x^3 - 2*x",
        point: "inf",
        direction: "two",
        expectation: LimitExpectation::PlusInfinity,
    },
    LimitCase {
        expression: "x^3 - 2*x",
        point: "-inf",
        direction: "two",
        expectation: LimitExpectation::MinusInfinity,
    },
    LimitCase {
        expression: "(1 + 1/x)^x",
        point: "inf",
        direction: "two",
        expectation: LimitExpectation::Finite("2.71828182845904523536028747135"),
    },
    LimitCase {
        expression: "(1 + 1/x)^x",
        point: "-inf",
        direction: "two",
        expectation: LimitExpectation::Finite("2.71828182845904523536028747135"),
    },
    LimitCase {
        expression: "(1 + x)^(1/x)",
        point: "0",
        direction: "two",
        expectation: LimitExpectation::Finite("2.71828182845904523536028747135"),
    },
    LimitCase {
        expression: "(1 + x)^(1/x)",
        point: "0",
        direction: "left",
        expectation: LimitExpectation::Finite("2.71828182845904523536028747135"),
    },
    LimitCase {
        expression: "(1 + 2/x)^x",
        point: "inf",
        direction: "two",
        expectation: LimitExpectation::Finite("7.38905609893065022723042746058"),
    },
    LimitCase {
        expression: "(1 + 1/x)^(3*x)",
        point: "inf",
        direction: "two",
        expectation: LimitExpectation::Finite("20.0855369231876677409285296546"),
    },
    LimitCase {
        expression: "(1 + 3/x)^(x/2)",
        point: "inf",
        direction: "two",
        expectation: LimitExpectation::Finite("4.48168907033806482260205546012"),
    },
];

fn limit_record(expression: &str, point: &str, direction: &str) -> Value {
    let function = match direction {
        "left" => "limit_left",
        "right" => "limit_right",
        _ => "limit_full",
    };
    eval(&format!(
        "x = symbol(\"x\"); {function}({expression}, x, {point})"
    ))
}

fn assert_infinity_value(value: &Value, negative: bool, label: &str) {
    let expression = symbolic(value);
    let infinity = Sym::Fun("inf".to_string(), Vec::new());
    let expected = if negative {
        Sym::neg(infinity)
    } else {
        infinity
    };
    assert_eq!(
        *expression,
        expected,
        "{label}: side value {} is not {}inf",
        expression.canonical(),
        if negative { "-" } else { "+" }
    );
}

#[test]
fn sympyoracle_limits_agree() {
    for case in LIMIT_CASES {
        let record = limit_record(case.expression, case.point, case.direction);
        let status = enum_member(field(&record, "status"));
        let label = format!(
            "limit({}, x -> {}, {})",
            case.expression, case.point, case.direction
        );
        match case.expectation {
            LimitExpectation::Finite(expected) => {
                assert_eq!(
                    status, "Value",
                    "{label}: kernel status {status} vs sympy {expected}"
                );
                let value = symbolic(field(&record, "value"));
                let number = numeric::evaluate(value, 40).unwrap_or_else(|error| {
                    panic!(
                        "{label}: the kernel value did not evaluate: {}",
                        error.message
                    )
                });
                assert_real_agrees(&number, expected, TIGHT_TOLERANCE, &label);
            }
            LimitExpectation::PlusInfinity => {
                assert_eq!(
                    status, "PlusInfinity",
                    "{label}: kernel status {status} vs sympy oo"
                );
            }
            LimitExpectation::MinusInfinity => {
                assert_eq!(
                    status, "MinusInfinity",
                    "{label}: kernel status {status} vs sympy -oo"
                );
            }
            LimitExpectation::DoesNotExist => {
                assert_eq!(
                    status, "DoesNotExist",
                    "{label}: kernel status {status} vs sympy one-sided disagreement"
                );
                assert_infinity_value(field(&record, "left"), true, &label);
                assert_infinity_value(field(&record, "right"), false, &label);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SympyOracle_MatrixOperations_Agree#0
// ---------------------------------------------------------------------------

struct MatrixValueCase {
    rows: &'static [&'static [&'static str]],
    /// (x, y, sympy value), evaluated at 30 significant digits.
    points: &'static [(&'static str, &'static str, &'static str)],
}

const DETERMINANT_CASES: &[MatrixValueCase] = &[
    MatrixValueCase {
        rows: &[&["x", "1"], &["y", "x"]],
        points: &[
            ("1/3", "7/5", "-1.28888888888888888888888888889"),
            ("2", "-3/4", "4.75000000000000000000000000000"),
        ],
    },
    MatrixValueCase {
        rows: &[&["x", "1", "0"], &["0", "x", "1"], &["1", "0", "x"]],
        points: &[
            ("1/3", "7/5", "1.03703703703703703703703703704"),
            ("2", "-3/4", "9.00000000000000000000000000000"),
        ],
    },
    MatrixValueCase {
        rows: &[&["2", "0", "1"], &["1", "3", "2"], &["1", "1", "4"]],
        points: &[
            ("1/3", "7/5", "18.0000000000000000000000000000"),
            ("2", "-3/4", "18.0000000000000000000000000000"),
        ],
    },
];

const TRACE_CASES: &[MatrixValueCase] = &[
    MatrixValueCase {
        rows: &[&["x", "1"], &["y", "x"]],
        points: &[
            ("1/3", "7/5", "0.666666666666666666666666666667"),
            ("2", "-3/4", "4.00000000000000000000000000000"),
        ],
    },
    MatrixValueCase {
        rows: &[&["2", "0", "1"], &["1", "3", "2"], &["1", "1", "4"]],
        points: &[
            ("1/3", "7/5", "9.00000000000000000000000000000"),
            ("2", "-3/4", "9.00000000000000000000000000000"),
        ],
    },
];

struct RankCase {
    rows: &'static [&'static [&'static str]],
    rank: usize,
}

const RANK_CASES: &[RankCase] = &[
    RankCase {
        rows: &[&["1", "2"], &["2", "4"]],
        rank: 1,
    },
    RankCase {
        rows: &[&["1", "2", "3"], &["4", "5", "6"], &["7", "8", "9"]],
        rank: 2,
    },
    RankCase {
        rows: &[&["2", "0", "1"], &["1", "3", "2"], &["1", "1", "3"]],
        rank: 3,
    },
    RankCase {
        rows: &[&["x", "1"], &["0", "y"]],
        rank: 2,
    },
    RankCase {
        rows: &[&["x", "x"], &["x", "x"]],
        rank: 1,
    },
];

struct InverseCase {
    rows: &'static [&'static [&'static str]],
    /// (x, y, row-major entry values), evaluated at 30 significant digits.
    points: &'static [(&'static str, &'static str, &'static [&'static str])],
}

const INVERSE_CASES: &[InverseCase] = &[
    InverseCase {
        rows: &[&["x", "1"], &["y", "x"]],
        points: &[
            (
                "1/3",
                "7/5",
                &[
                    "-0.258620689655172413793103448276",
                    "0.775862068965517241379310344828",
                    "1.08620689655172413793103448276",
                    "-0.258620689655172413793103448276",
                ],
            ),
            (
                "2",
                "-3/4",
                &[
                    "0.421052631578947368421052631579",
                    "-0.210526315789473684210526315789",
                    "0.157894736842105263157894736842",
                    "0.421052631578947368421052631579",
                ],
            ),
        ],
    },
    InverseCase {
        rows: &[&["x", "2"], &["3", "y"]],
        points: &[
            (
                "1/3",
                "7/5",
                &[
                    "-0.253012048192771084337349397590",
                    "0.361445783132530120481927710843",
                    "0.542168674698795180722891566265",
                    "-0.0602409638554216867469879518072",
                ],
            ),
            (
                "2",
                "-3/4",
                &[
                    "0.100000000000000000000000000000",
                    "0.266666666666666666666666666667",
                    "0.400000000000000000000000000000",
                    "-0.266666666666666666666666666667",
                ],
            ),
        ],
    },
    InverseCase {
        rows: &[&["2", "0", "1"], &["1", "3", "2"], &["1", "1", "4"]],
        points: &[
            (
                "1/3",
                "7/5",
                &[
                    "0.555555555555555555555555555556",
                    "0.0555555555555555555555555555556",
                    "-0.166666666666666666666666666667",
                    "-0.111111111111111111111111111111",
                    "0.388888888888888888888888888889",
                    "-0.166666666666666666666666666667",
                    "-0.111111111111111111111111111111",
                    "-0.111111111111111111111111111111",
                    "0.333333333333333333333333333333",
                ],
            ),
            (
                "2",
                "-3/4",
                &[
                    "0.555555555555555555555555555556",
                    "0.0555555555555555555555555555556",
                    "-0.166666666666666666666666666667",
                    "-0.111111111111111111111111111111",
                    "0.388888888888888888888888888889",
                    "-0.166666666666666666666666666667",
                    "-0.111111111111111111111111111111",
                    "-0.111111111111111111111111111111",
                    "0.333333333333333333333333333333",
                ],
            ),
        ],
    },
];

#[test]
fn sympyoracle_matrixoperations_agree() {
    for case in DETERMINANT_CASES {
        let determinant = matrix(case.rows).det().expect("a square matrix");
        let reparsed = reparse(&determinant);
        for (x, y, expected) in case.points {
            let value = evaluate_at(&reparsed, &[("x", x), ("y", y)]);
            assert_real_agrees(
                &value,
                expected,
                REAL_TOLERANCE,
                &format!("det at (x, y) = ({x}, {y})"),
            );
        }
    }
    for case in TRACE_CASES {
        let trace = matrix(case.rows).trace();
        let reparsed = reparse(&trace);
        for (x, y, expected) in case.points {
            let value = evaluate_at(&reparsed, &[("x", x), ("y", y)]);
            assert_real_agrees(
                &value,
                expected,
                REAL_TOLERANCE,
                &format!("trace at (x, y) = ({x}, {y})"),
            );
        }
    }
    for case in RANK_CASES {
        assert_eq!(
            matrix(case.rows).rank(),
            case.rank,
            "rank of {:?}",
            case.rows
        );
    }
    for case in INVERSE_CASES {
        let inverse = matrix(case.rows).inverse().expect("an invertible matrix");
        let mut entries: Vec<Sym> = Vec::new();
        for row in 0..inverse.rows() {
            for column in 0..inverse.columns() {
                entries.push(reparse(inverse.get(row, column).expect("in range")));
            }
        }
        for (x, y, expected) in case.points {
            assert_eq!(
                expected.len(),
                entries.len(),
                "the corpus entry count matches the inverse's order"
            );
            for (index, (entry, expected_value)) in entries.iter().zip(expected.iter()).enumerate()
            {
                let value = evaluate_at(entry, &[("x", x), ("y", y)]);
                assert_real_agrees(
                    &value,
                    expected_value,
                    REAL_TOLERANCE,
                    &format!("inverse entry {index} at (x, y) = ({x}, {y})"),
                );
            }
        }
    }
}
