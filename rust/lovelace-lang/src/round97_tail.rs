//! Round 97 — the final twelve mappable deferred census rows.
//!
//! One test per C# class below; every assertion is the oracle-observed behaviour named in
//! docs/goal-parity-contract/round-97/final-tail.md. The rows that were already reachable through
//! the runner (InfiniteFamilyCompleteness, InverseBranchCandidateVerification,
//! LatexSymbolNameEscaping, SystemSolveProof) are mapped to the tests that drive the same shapes;
//! the rest close a production gap the round-45/50 sweep measured.
//!
//! * TimingScaleTests — `OperationTiming.ElapsedDisplay` and `ElapsedScale` come from ONE selector.
//! * DxStructuredResultsTests — `linsolve` on a singular symbolic system is a typed InvalidOperation.
//! * HyperbolicBuiltinsAndNegativeIntegerPowersTests — a Symbolics+MathIR engine publishes 107 builtins.
//! * InfiniteFamilyCompletenessTests — `exp(x) = c` over the complexes is the ImageSet family
//!   `log(c) + 2*k*pi*i`; over the reals it is `log(5)`.
//! * InverseBranchCandidateVerificationTests #4 — `sqrt(x^2 + 1) = 2` is Solved with two roots.
//! * LatexPrinterTests — a complex-constant power base is already a LaTeX group.
//! * LatexSymbolNameEscapingTests — escaped names inside products and a derivative's variable list.
//! * MetamorphicSetClosureTests — `cancel` refuses to cancel an irreducible quotient.
//! * PrettyParenthesesTests — a complex-constant power base is one delimited group in ASCII.
//! * PropertyTests/MathIR_EvaluatesLikeSymbolic — direct evaluation and the lowered kernel agree
//!   exactly at 50 places over the pinned 60-expression corpus.
//! * SolveCompletenessPairingTests — the 2^8 system hits the 128-solution cap and publishes Partial.
//! * SystemSolveProofTests — both nonlinear systems enumerate their two exact solutions, and the
//!   inconsistent shapes are published as NoSolutions.

use std::collections::HashMap;
use std::time::Duration;

use lovelace_real::Real;
use lovelace_symbolics::{Expr as Sym, Rat};

use crate::mathir::{IrEvaluator, IrProgram};
use crate::numeric::{self, Num};
use crate::{Engine, OpTiming, Value};

/// The reference's precision scope for the property and system-solve rows (`WithPrecision(50, 20)`).
const PLACES: usize = 50;

fn eval(source: &str) -> Value {
    let mut engine = Engine::new();
    engine
        .eval(source)
        .unwrap_or_else(|error| panic!("`{source}` failed: {}", error.message))
}

fn eval_error(source: &str) -> crate::LangError {
    let mut engine = Engine::new();
    engine.eval(source).expect_err("the script must be refused")
}

fn record_type(value: &Value) -> &str {
    match value {
        Value::Record(name, _) => name,
        other => panic!("expected a record, got {other:?}"),
    }
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

fn boolean(value: &Value) -> bool {
    match value {
        Value::Boolean(value) => *value,
        other => panic!("expected a boolean, got {other:?}"),
    }
}

fn vector(value: &Value) -> &[Value] {
    match value {
        Value::Vector(values) => values,
        other => panic!("expected a vector, got {other:?}"),
    }
}

fn symbolic(value: &Value) -> &Sym {
    match value {
        Value::Symbolic(expression) => expression,
        other => panic!("expected a symbolic value, got {other:?}"),
    }
}

fn integer_text(value: &Value) -> String {
    match value {
        Value::Integer(value) => value.to_decimal(),
        other => panic!("expected an integer, got {other:?}"),
    }
}

fn rat(numerator: i64, denominator: i64) -> Rat {
    Rat::from_i64(numerator)
        .div(&Rat::from_i64(denominator))
        .expect("nonzero denominator")
}

fn residual_is_zero(residual: &Sym, label: &str) {
    let value = numeric::evaluate(residual, 40)
        .unwrap_or_else(|error| panic!("{label}: residual evaluation failed: {}", error.message));
    let tolerance = Real::from_str("0.000000000000000000000001").expect("tolerance");
    let magnitude = match value {
        Num::Exact(rational) => {
            assert!(
                rational.is_zero(),
                "{label}: exact residual {rational} is not zero"
            );
            return;
        }
        Num::Real(real) => real.abs(),
        Num::Complex(complex) => complex.magnitude(40),
    };
    assert!(
        magnitude.cmp(&tolerance) == std::cmp::Ordering::Less,
        "{label}: residual magnitude is not zero: {}",
        magnitude.to_decimal_fixed(40)
    );
}

// ---------------------------------------------------------------------------
// TimingScaleTests/OperationTiming_ExposesTheSameScaleAsItsDisplay#0
// ---------------------------------------------------------------------------

#[test]
fn timingscaletests_operationtiming_exposes_the_same_scale_as_its_display() {
    // C#: new OperationTiming(0, Value.Void, string.Empty, TimeSpan.FromMilliseconds(3.5)) has
    // ElapsedDisplay "3.5 ms" and ElapsedScale (3.5, "ms").
    let timing = OpTiming {
        position: 0,
        result_kind: "Void".to_string(),
        has_output: false,
        elapsed: Duration::from_nanos(3_500_000),
    };
    assert_eq!("3.5 ms", timing.elapsed_display());
    assert_eq!((3.5, "ms"), timing.elapsed_scale());

    // The selector table the C# TimingScaleTests drives: largest whole unit, two decimals,
    // whole nanoseconds at the bottom of the scale.
    let cases = [
        (Duration::ZERO, "0 ns", (0.0, "ns")),
        (Duration::from_nanos(500), "500 ns", (500.0, "ns")),
        (Duration::from_nanos(500_000), "500 µs", (500.0, "µs")),
        (Duration::from_nanos(12_570_000), "12.57 ms", (12.57, "ms")),
        (Duration::from_nanos(2_500_000_000), "2.5 s", (2.5, "s")),
        (Duration::from_secs(90), "1.5 min", (1.5, "min")),
        (Duration::from_secs(7_200), "2 h", (2.0, "h")),
    ];
    for (elapsed, display, scale) in cases {
        let timing = OpTiming {
            position: 0,
            result_kind: "Void".to_string(),
            has_output: false,
            elapsed,
        };
        assert_eq!(display, timing.elapsed_display(), "{elapsed:?}");
        assert_eq!(scale, timing.elapsed_scale(), "{elapsed:?}");
    }
}

// ---------------------------------------------------------------------------
// DxStructuredResultsTests/LinsolveFull_SingularSystem_ReportsStatusNotThrow#0
// ---------------------------------------------------------------------------

#[test]
fn dxstructuredresultstests_linsolvefull_singularsystem_reportsstatusnotthrow() {
    let record = eval("x = symbol(\"x\"); linsolve_full([[x, x], [x, x]], [1, 1])");
    assert_eq!("MatrixSolveResult", record_type(&record));
    assert_eq!("NoSolutions", enum_member(field(&record, "status")));
    assert!(boolean(field(&record, "complete")));
    assert_eq!("Complete", enum_member(field(&record, "completeness")));
    assert!(vector(field(&record, "solutions")).is_empty());

    // The convenience projection keeps its throwing behaviour (InvalidOperation/DomainError).
    let error = eval_error("x = symbol(\"x\"); linsolve([[x, x], [x, x]], [1, 1])");
    assert_eq!(
        "System is singular or underdetermined.",
        error.message.as_str()
    );
    assert_eq!("InvalidOperation", error.code);
    assert_eq!("DomainError", error.category);
}

// ---------------------------------------------------------------------------
// HyperbolicBuiltinsAndNegativeIntegerPowersTests/BuiltinCount_IncludesTheThreeHyperbolicFunctions#0
// ---------------------------------------------------------------------------

#[test]
fn hyperbolicbuiltinsandnegativeintegerpowerstests_builtincount_includesthethreehyperbolicfunctions()
 {
    // C#: a SuiteEngine with SymbolicsPlugin + MathIRPlugin publishes 107 builtins.
    assert_eq!(
        107,
        crate::registry::builtin_count_for_plugins(&["Lovelace.Symbolics", "Lovelace.MathIR"])
    );
    // The runner loads the DSP plugin too, and its 123 is the count the --functions surface
    // reports; the difference is exactly the 16 DSP descriptors, not a drift in the core set.
    assert_eq!(123, crate::registry::BUILTINS.len());
    for name in ["asinh", "acosh", "atanh"] {
        let info = crate::registry::find(name).unwrap_or_else(|| panic!("{name} is registered"));
        assert_eq!(Some("Lovelace.Symbolics"), info.plugin);
    }
}

// ---------------------------------------------------------------------------
// InfiniteFamilyCompletenessTests/SympyOracle_ExpOverComplexes_IsAnImageSet_AndTheTemplateMatchesIt#0
// ---------------------------------------------------------------------------

#[test]
fn infinitefamilycompletenesstests_sympyoracle_expovercomplexes_isanimageset_and_thetemplatematchesit()
 {
    let record = eval("x = symbol(\"x\"); c = symbol(\"c\"); solve_full(exp(x) - c == 0, x)");
    assert_eq!("SolveResult", record_type(&record));
    assert_eq!("Solved", enum_member(field(&record, "status")));
    assert!(boolean(field(&record, "complete")));
    assert_eq!("Complete", enum_member(field(&record, "completeness")));
    assert!(vector(field(&record, "solutions")).is_empty());

    let families = vector(field(&record, "families"));
    assert_eq!(1, families.len());
    let family = &families[0];
    assert_eq!(
        "log(c) + 2*k*pi*i",
        symbolic(field(family, "template")).to_string()
    );
    assert_eq!(
        "(add (fn log (sym c)) (mul (rat 2 1) (sym k) (pi) (i)))",
        symbolic(field(family, "template")).canonical()
    );
    assert_eq!("k", symbolic(field(family, "parameter")).to_string());
    assert_eq!("2*pi*i", symbolic(field(family, "period")).to_string());
    assert_eq!(
        "integer",
        match field(family, "parameter_domain") {
            Value::Domain(domain) => domain.as_str(),
            other => panic!("expected a domain, got {other:?}"),
        }
    );
    assert_eq!("ParametricExact", enum_member(field(family, "exactness")));

    // The C# row's concrete check: every integer member k in -3..3 solves exp(x) = c. Evaluate
    // the residual at c = 5, the value the SymPy script uses for the real branch.
    let template = symbolic(field(family, "template")).clone();
    let equation = Sym::sub(
        Sym::Fun("exp".to_string(), vec![Sym::sym("x")]),
        Sym::num(5),
    );
    for k in -3..=3 {
        let member = lovelace_symbolics::substitute(&template, "k", &Sym::num(i64::from(k)));
        let member = lovelace_symbolics::substitute(&member, "c", &Sym::num(5));
        let residual = lovelace_symbolics::substitute(&equation, "x", &member);
        residual_is_zero(&residual, &format!("family member k = {k}"));
    }

    // ... and over the reals SymPy answers {log(5)}: the kernel publishes that one solution.
    let real = eval("x = symbol(\"x\"); solve_full(exp(x) - 5 == 0, x, real)");
    assert_eq!("Solved", enum_member(field(&real, "status")));
    let solutions = vector(field(&real, "solutions"));
    assert_eq!(1, solutions.len());
    assert_eq!(
        "log(5)",
        symbolic(field(&solutions[0], "value")).to_string()
    );
}

// ---------------------------------------------------------------------------
// InverseBranchCandidateVerificationTests/EveryRepresentedSolutionSatisfiesTheOriginalEquation#4
// ---------------------------------------------------------------------------

#[test]
fn inversebranchcandidateverificationtests_everyrepresentedsolutionsatisfiestheoriginalequation() {
    // C# theory row #4 is "sqrt(x^2 + 1) - 2", expected count 2, finite.
    let record = eval("x = symbol(\"x\"); solve_full(sqrt(x^2 + 1) - 2 == 0, x)");
    assert_eq!("Solved", enum_member(field(&record, "status")));
    assert!(boolean(field(&record, "complete")));
    assert_eq!("2", integer_text(field(&record, "represented_count")));

    let solutions = vector(field(&record, "solutions"));
    assert_eq!(2, solutions.len());
    let mut values: Vec<String> = solutions
        .iter()
        .map(|solution| symbolic(field(solution, "value")).to_string())
        .collect();
    values.sort();
    assert_eq!(vec!["-1/2*sqrt(12)", "1/2*sqrt(12)"], values);

    let equation = Sym::sub(
        Sym::Fun(
            "sqrt".to_string(),
            vec![Sym::add(Sym::pow(Sym::sym("x"), Sym::num(2)), Sym::num(1))],
        ),
        Sym::num(2),
    );
    for solution in solutions {
        let value = symbolic(field(solution, "value"));
        let residual = lovelace_symbolics::substitute(&equation, "x", value);
        residual_is_zero(&residual, &format!("sqrt(x^2 + 1) - 2 at {value}"));
    }
}

// ---------------------------------------------------------------------------
// LatexPrinterTests/KnownRendering_PowerOfAComplexConstant_IsAlreadyAGroup#0
// ---------------------------------------------------------------------------

#[test]
fn latexprintertests_knownrendering_powerofacomplexconstant_isalreadyagroup() {
    let one = Rat::one();
    let positive = Sym::pow(Sym::complex(one.clone(), one.clone()), Sym::sym("x"));
    // C#: the complex constant renders as the already-delimited group "(1 + 1 i)", so the power
    // adds no left...ight pair.
    assert_eq!("(1 + 1 i)^{x}", positive.latex());
    let negative = Sym::pow(Sym::complex(Rat::from_i64(-1), one), Sym::sym("x"));
    assert_eq!("(-1 + 1 i)^{x}", negative.latex());
}

// ---------------------------------------------------------------------------
// LatexSymbolNameEscapingTests/LatexSymbolName_InsideLargerExpressions_IsStillEscaped#0
// ---------------------------------------------------------------------------

#[test]
fn latexsymbolnameescapingtests_latexsymbolname_insidelargerexpressions_isstillescaped() {
    let product = Sym::mul(Sym::sym("a%b"), Sym::sym("c&d"));
    assert_eq!("a\\%b \\cdot c\\&d", product.latex());
    let derivative = Sym::Fun(
        "derivative".to_string(),
        vec![Sym::sym("f"), Sym::sym("x_1")],
    );
    assert_eq!("\\operatorname{diff}(f, x\\_1)", derivative.latex());
}

// ---------------------------------------------------------------------------
// MetamorphicSetClosureTests/CancelBuiltin_ReducesAndRefusesToCancelWhenNoFactorIsCommon#0
// ---------------------------------------------------------------------------

#[test]
fn metamorphicsetclosuretests_cancelbuiltin_reducesandrefusestocancelwhennofactoriscommon() {
    assert_eq!(
        "1 (Symbolic)",
        eval("x = symbol(\"x\"); cancel(x/x)").typed()
    );
    assert_eq!(
        "x + 1 (Symbolic)",
        eval("x = symbol(\"x\"); cancel((x^2-1)/(x-1))").typed()
    );
    // The irreducible quotient comes back unchanged, not silently reduced to x + 1.
    assert_eq!(
        "(x^2 + 1)/(x - 1) (Symbolic)",
        eval("x = symbol(\"x\"); cancel((x^2+1)/(x-1))").typed()
    );
}

// ---------------------------------------------------------------------------
// PrettyParenthesesTests/ModelLevel_PowerOfAComplexConstant_HasADelimitedBase#0
// ---------------------------------------------------------------------------

#[test]
fn prettyparenthesestests_modellevel_powerofacomplexconstant_hasadelimitedbase() {
    let one = Rat::one();
    let positive = Sym::pow(Sym::complex(one.clone(), one.clone()), Sym::sym("x"));
    assert_eq!("(1 + i)^x", positive.to_string());
    let negative = Sym::pow(Sym::complex(Rat::from_i64(-1), one), Sym::sym("x"));
    assert_eq!("(-1 + i)^x", negative.to_string());
}

// ---------------------------------------------------------------------------
// PropertyTests/MathIR_EvaluatesLikeSymbolic#0
// ---------------------------------------------------------------------------

/// The 60 canonical expressions the C# generator produces for seed 20260915. The joined text's
/// FNV-1a 64 is asserted below against the pinned corpus hash, so this list cannot drift from
/// `Lovelace.Symbolics.Tests/PropertyTests.RandomExpr`.
const MATHIR_CORPUS: [&str; 60] = [
    "(fn sin (fn sin (rat -5 3)))",
    "(rat 0 1)",
    "(fn exp (add (rat -3 1) (sym x)))",
    "(rat 2 1)",
    "(mul (rat 2 1) (sym x))",
    "(rat 1 1)",
    "(add (fn abs (pow (sym x) (rat 2 1))) (mul (rat -1 1) (fn exp (fn exp (sym x)))))",
    "(fn sin (fn abs (sym x)))",
    "(fn exp (rat -3 2))",
    "(rat 0 1)",
    "(fn abs (add (fn exp (rat -1 3)) (mul (rat -1 1) (add (rat -1 1) (sym x)))))",
    "(fn exp (add (fn exp (sym x)) (mul (rat -2 1) (sym x))))",
    "(rat 0 1)",
    "(pow (fn abs (fn abs (sym x))) (rat 3 1))",
    "(fn abs (fn abs (sym x)))",
    "(pow (fn exp (sym x)) (rat 2 1))",
    "(add (fn exp (rat 1 1)) (mul (rat -1 1) (fn sin (pow (sym x) (rat 4 1)))))",
    "(add (rat -11 2) (fn sin (rat -2 1)))",
    "(rat -1 2)",
    "(pow (add (rat -4 1) (mul (rat -1 1) (add (rat 5 1) (sym x)))) (rat 4 1))",
    "(add (fn abs (sym x)) (fn abs (pow (sym x) (rat 2 1))))",
    "(rat -125 1)",
    "(rat 0 1)",
    "(add (fn exp (rat 5 1)) (fn sin (rat -5 3)))",
    "(fn sin (rat -1 4))",
    "(add (rat -1 3) (fn sin (rat 1 4)))",
    "(rat 1 1)",
    "(mul (sym x) (fn abs (sym x)))",
    "(add (rat 5 3) (mul (rat -1 1) (add (fn sin (sym x)) (mul (rat -1 1) (fn exp (sym x))))) (mul (rat 2 1) (sym x)))",
    "(mul (fn abs (sym x)) (fn abs (add (rat 5 2) (sym x))) (fn exp (sym x)))",
    "(add (rat -3 1) (fn sin (rat 3 4)))",
    "(mul (add (rat -16 9) (fn exp (sym x))) (add (fn exp (sym x)) (pow (sym x) (rat 4 1))))",
    "(fn exp (sym x))",
    "(rat 1 1)",
    "(fn sin (fn sin (rat -1 4)))",
    "(mul (add (rat -1 1) (mul (rat -4 1) (sym x))) (add (fn exp (sym x)) (mul (rat -1 1) (sym x))))",
    "(rat 9 2)",
    "(rat 25 16)",
    "(fn sin (rat -7 2))",
    "(rat 3 1)",
    "(rat 4 1)",
    "(fn abs (sym x))",
    "(fn abs (fn exp (rat 1 1)))",
    "(fn exp (sym x))",
    "(rat 27 1)",
    "(mul (rat -1 1) (add (rat 1 3) (sym x)))",
    "(rat 1 1)",
    "(add (fn exp (fn sin (sym x))) (mul (rat -1 1) (add (fn sin (rat -1 1)) (mul (rat -1 1) (fn exp (rat -5 1))))))",
    "(mul (fn abs (pow (sym x) (rat 2 1))) (fn exp (rat -5 1)))",
    "(fn abs (fn sin (rat -1 1)))",
    "(fn sin (mul (rat -5 4) (sym x)))",
    "(fn exp (sym x))",
    "(pow (sym x) (rat 2 1))",
    "(rat 1 1)",
    "(add (pow (fn sin (rat 5 2)) (rat 4 1)) (mul (fn sin (rat -1 1)) (add (rat -5 1) (mul (rat -1 1) (sym x)))))",
    "(fn abs (sym x))",
    "(add (rat 1 2) (mul (rat -1 1) (fn exp (rat 2 1))))",
    "(fn abs (sym x))",
    "(mul (rat 4 1) (add (rat 3 1) (sym x)))",
    "(pow (sym x) (rat 3 1))",
];

fn fnv1a64(text: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn num_equal(left: &Num, right: &Num) -> bool {
    match (left, right) {
        (Num::Exact(a), Num::Exact(b)) => a == b,
        _ => left.to_real(PLACES) == right.to_real(PLACES),
    }
}

#[test]
fn propertytests_mathirevaluateslikesymbolic() {
    let joined = MATHIR_CORPUS.join("\n");
    assert_eq!(
        6_946_605_504_391_048_442,
        fnv1a64(&joined),
        "the pinned MathIR corpus must be the oracle's 60 expressions"
    );
    let x = "x".to_string();
    let points = [rat(-3, 4), rat(1, 3), rat(7, 5)];
    for canonical in MATHIR_CORPUS {
        let expression = lovelace_symbolics::canonical_parse(canonical)
            .unwrap_or_else(|error| panic!("{canonical}: {error}"));
        assert_eq!(canonical, expression.canonical());
        let program = IrProgram::lower(&expression, std::slice::from_ref(&x))
            .unwrap_or_else(|error| panic!("{canonical}: lower failed: {}", error.message));
        for point in &points {
            let mut bindings: HashMap<String, Num> = HashMap::new();
            bindings.insert(x.clone(), Num::Exact(point.clone()));
            let direct = numeric::evaluate_bound(&expression, &bindings, PLACES)
                .unwrap_or_else(|error| panic!("{canonical} at {point}: {error}"));
            let lowered = IrEvaluator::evaluate_at(&program, &bindings, PLACES)
                .unwrap_or_else(|error| panic!("{canonical} at {point}: {error}"));
            assert!(
                num_equal(&direct, &lowered),
                "MathIR disagrees for {canonical} at x={point}: direct={direct:?}, lowered={lowered:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// SolveCompletenessPairingTests/EverySolveStatus_PublishesTheDocumentedPair_OnBothRecords#0
// ---------------------------------------------------------------------------

#[test]
fn solvecompletenesspairingtests_everysolvestatus_publishesthedocumentedpair_onbothrecords() {
    // The 2^8 system hits the reference's 128-solution enumeration cap, which is the ONE way
    // SystemSolveResult publishes Partial/complete:false/Partial.
    let script = "x1 = symbol(\"x1\"); x2 = symbol(\"x2\"); x3 = symbol(\"x3\"); \
                  x4 = symbol(\"x4\"); x5 = symbol(\"x5\"); x6 = symbol(\"x6\"); \
                  x7 = symbol(\"x7\"); x8 = symbol(\"x8\"); \
                  solve_system_full([x1^2 == 1, x2^2 == 1, x3^2 == 1, x4^2 == 1, \
                                     x5^2 == 1, x6^2 == 1, x7^2 == 1, x8^2 == 1], \
                                    [x1, x2, x3, x4, x5, x6, x7, x8])";
    let record = eval(script);
    assert_eq!("SystemSolveResult", record_type(&record));
    assert_eq!("Partial", enum_member(field(&record, "status")));
    assert!(!boolean(field(&record, "complete")));
    assert_eq!("Partial", enum_member(field(&record, "completeness")));
    let solutions = vector(field(&record, "solutions"));
    assert_eq!(128, solutions.len());
    assert!(vector(field(&record, "diagnostics")).is_empty());
    for solution in solutions {
        assert_eq!(8, vector(field(solution, "bindings")).len());
    }
}

// ---------------------------------------------------------------------------
// SystemSolveProofTests/NonlinearElimination_AndTheBoundary_AgreeWithSympySolve#0
// ---------------------------------------------------------------------------

fn assert_close_to(expression: &Sym, expected: &str, label: &str) {
    let actual = numeric::evaluate(expression, 60)
        .unwrap_or_else(|error| panic!("{label}: evaluation failed: {}", error.message))
        .to_real(60)
        .unwrap_or_else(|| panic!("{label}: not real"));
    let expected = Real::from_str(expected).unwrap_or_else(|| panic!("{label}: bad expectation"));
    let tolerance = Real::from_str("0.000000000000000000000001").expect("tolerance");
    assert!(
        actual.sub(&expected).abs().cmp(&tolerance) == std::cmp::Ordering::Less,
        "{label}: {} differs from {expected}",
        actual.to_decimal_fixed(40)
    );
}

#[test]
fn systemsolveprooftests_nonlinearelimination_andtheboundary_agreewithsympysolve() {
    // sympy.solve([x**2 + y - 1, x - y], [x, y], dict=True)
    let parabola = eval(
        "x = symbol(\"x\"); y = symbol(\"y\"); \
         solve_system_full([x^2 + y == 1, x - y == 0], [x, y])",
    );
    assert_eq!("Solved", enum_member(field(&parabola, "status")));
    assert!(boolean(field(&parabola, "complete")));
    let solutions = vector(field(&parabola, "solutions"));
    assert_eq!(2, solutions.len());
    assert!(vector(field(&parabola, "diagnostics")).is_empty());
    let mut values: Vec<&Sym> = Vec::new();
    for solution in solutions {
        let x = symbolic(field(&vector(field(solution, "bindings"))[0], "value"));
        let y = symbolic(field(&vector(field(solution, "bindings"))[1], "value"));
        assert_eq!(x, y, "the line x - y = 0 forces x = y");
        values.push(x);
    }
    assert!(
        values
            .iter()
            .any(|value| value.to_string() == "1/2*(sqrt(5) - 1)")
    );
    assert_close_to(
        values[0],
        "-1.6180339887498948482045868343656381177",
        "parabola-",
    );
    assert_close_to(
        values[1],
        "0.61803398874989484820458683436563811772",
        "parabola+",
    );

    // sympy.solve([x**2 + y**2 - 1, y - x], [x, y], dict=True)
    let circle = eval(
        "x = symbol(\"x\"); y = symbol(\"y\"); \
         solve_system_full([x^2 + y^2 == 1, y - x == 0], [x, y])",
    );
    assert_eq!("Solved", enum_member(field(&circle, "status")));
    let solutions = vector(field(&circle, "solutions"));
    assert_eq!(2, solutions.len());
    let values: Vec<&Sym> = solutions
        .iter()
        .map(|solution| symbolic(field(&vector(field(solution, "bindings"))[0], "value")))
        .collect();
    assert_close_to(
        values[0],
        "-0.7071067811865475244008443621048490393",
        "circle-",
    );
    assert_close_to(
        values[1],
        "0.70710678118654752440084436210484903928",
        "circle+",
    );

    // The boundary's other side: SymPy reports zero solutions, and the kernel must PROVE the
    // emptiness it reports (NoSolutions, complete, no diagnostics).
    for (label, script) in [
        (
            "x + y - 1, x + y - 2",
            "x = symbol(\"x\"); y = symbol(\"y\"); \
             solve_system_full([x + y == 1, x + y == 2], [x, y])",
        ),
        (
            "x*y - 1, x",
            "x = symbol(\"x\"); y = symbol(\"y\"); \
             solve_system_full([x*y == 1, x == 0], [x, y])",
        ),
    ] {
        let record = eval(script);
        assert_eq!(
            "NoSolutions",
            enum_member(field(&record, "status")),
            "{label}"
        );
        assert!(boolean(field(&record, "complete")), "{label}");
        assert!(vector(field(&record, "solutions")).is_empty(), "{label}");
        assert!(vector(field(&record, "diagnostics")).is_empty(), "{label}");
    }
}
