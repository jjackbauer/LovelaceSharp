//! Round 35: the Suite API tail, and the deferred rows the recent production unblocked.
//!
//! Every test below is one deferred census row (or a group of rows that assert the same case). The
//! C# assertion is the contract; where the Rust surface has a different shape (no CLR exception
//! types, no mutating ++/--, no decimal/f64 constructor) the test asserts the SAME value or refusal
//! through the Rust surface, and docs/goal-parity-contract/round-35/suite-tail.md records the
//! translation. A row whose measured Rust behaviour differs is not here; it is refused there.

use std::sync::Arc;

use crate::{help, Engine, LangError, Value};
use lovelace_int::Int;
use lovelace_nat::Nat;
use lovelace_real::Real;

fn eval(script: &str) -> Value {
    let mut engine = Engine::new();
    engine
        .eval(script)
        .unwrap_or_else(|error| panic!("script: {script} failed: {}", error.message))
}

fn eval_in(engine: &mut Engine, script: &str) -> Value {
    engine
        .eval(script)
        .unwrap_or_else(|error| panic!("script: {script} failed: {}", error.message))
}

fn refusal(script: &str) -> LangError {
    match Engine::new().eval(script) {
        Ok(value) => panic!("script: {script} answered {} instead of refusing", value.display()),
        Err(error) => error,
    }
}

fn nat(text: &str) -> Nat {
    Nat::from_decimal(text).unwrap_or_else(|| panic!("bad natural {text}"))
}

fn int(text: &str) -> Int {
    Int::from_decimal(text).unwrap_or_else(|| panic!("bad integer {text}"))
}

fn real(text: &str) -> Real {
    Real::from_str(text).unwrap_or_else(|| panic!("bad real {text}"))
}

fn one() -> Nat {
    Nat::from_u64(1)
}

// ===========================================================================
// Lovelace.Suite.Tests.HelpServiceTests (round-35 help aliases)
// ===========================================================================

#[test]
fn help_symbolics_lists_core_functions() {
    let text = help::category("symbolics");
    for name in ["symbol(", "simplify(", "expand(", "factor("] {
        assert!(text.contains(name), "missing {name} in:\n{text}");
    }
}

#[test]
fn help_solve_shows_signature_summary_example_and_see_also() {
    let text = help::function("solve").expect("solve has help");
    assert!(text.contains("solve(f, x [, domain])"), "{text}");
    assert!(text.contains("Solve"), "{text}");
    assert!(text.contains("solve(x^2 - 4 == 0, x)"), "{text}");
    assert!(text.contains("Returns: SolveResult"), "{text}");
    assert!(text.contains("See also:"), "{text}");
    assert!(text.contains("solve_full"), "{text}");
}

#[test]
fn help_compile_shows_descriptor() {
    let text = help::function("compile").expect("compile has help");
    assert!(text.contains("compile(f, params)"), "{text}");
    assert!(text.contains("MathIR"), "{text}");
    assert!(text.contains("compile_full"), "{text}");
}

#[test]
fn funcs_calculus_lists_calculus_functions() {
    let text = help::funcs(Some("calculus"));
    for name in ["diff(", "integrate(", "series(", "limit("] {
        assert!(text.contains(name), "missing {name} in:\n{text}");
    }
}

#[test]
fn funcs_all_is_categorized() {
    let text = help::funcs(None);
    for category in ["Symbolics", "Calculus", "Compilation", "DSP"] {
        assert!(text.contains(category), "missing {category} in:\n{text}");
    }
}

#[test]
fn help_unknown_argument_explains() {
    // The reference Lookup returns the "No category" sentence for an argument that names neither a
    // function nor a category. The Rust lookup answers None there (the caller can tell the
    // difference) and the same sentence is the category() renderer's answer.
    assert!(help::lookup("nonsense").is_none());
    assert!(help::category("nonsense").contains("No category"));
}

// ===========================================================================
// Lovelace.Suite.Tests.ArrayAllocationRefusalTests
// ===========================================================================

#[test]
fn zeros_ten_to_the_ninth_is_refused_without_allocating() {
    let error = refusal("zeros(1000000000)");
    assert_eq!(error.code, "AllocationRefused");
    assert_eq!(error.category, "BudgetExceeded");
    assert!(error.message.contains("zeros"), "{}", error.message);
}

#[test]
fn zeros_element_product_beyond_the_budget_is_refused_by_name_and_limit() {
    let error = refusal("zeros(1000000000, 1000000000)");
    assert_eq!(error.code, "AllocationRefused");
    assert_eq!(error.category, "BudgetExceeded");
    assert!(error.message.contains("zeros"), "{}", error.message);
    assert!(error.message.contains("1000000000000000000"), "{}", error.message);
    assert!(error.message.contains("268435456"), "{}", error.message);
}

// ===========================================================================
// Lovelace.Suite.Tests.InputDepthGuardTests
// ===========================================================================

#[test]
fn flat_chain_one_past_the_budget_is_refused() {
    let accepted = format!("1{};", "-1".repeat(509));
    assert!(Engine::new().eval(&accepted).is_ok(), "509-term chain must parse");
    let refused = format!("1{};", "-1".repeat(510));
    let error = Engine::new().eval(&refused).expect_err("one past the tree budget is refused");
    assert_eq!(error.code, "DepthExceeded");
    assert!(error.message.contains("tree depth"), "{}", error.message);
    assert!(error.message.contains("512"), "{}", error.message);
}

#[test]
fn symbolic_construction_one_past_the_budget_is_refused() {
    let accepted = "x = symbol(\"x\"); e = x; ".to_string()
        + &"e = abs(e); ".repeat(255)
        + "1";
    Engine::new().eval(&accepted).expect("255 wrappers are accepted");

    let refused = "x = symbol(\"x\"); e = x; ".to_string()
        + &"e = abs(e); ".repeat(256)
        + "1";
    let error = Engine::new().eval(&refused).expect_err("256 wrappers are refused");
    assert_eq!(error.code, "DepthExceeded");
    assert!(error.message.contains("symbolic expression depth"), "{}", error.message);
    assert!(error.message.contains("256"), "{}", error.message);
}

// ===========================================================================
// Lovelace.Natural.Tests.NaturalIncrementTests — ++n (prefix) and n++ (postfix)
//
// The Rust Nat is an immutable value. ++n is Nat::add(1); n++ is the pair (the old value, the new
// value), which the test asserts exactly as the C# row does (old renders 5, the rebound n 6).
// ===========================================================================

fn inc(n: &Nat) -> Nat {
    n.add(&one())
}

#[test]
fn natural_increment_prefix_rows() {
    assert_eq!(inc(&Nat::zero()).to_decimal(), "1");
    assert_eq!(inc(&Nat::from_u64(9)).to_decimal(), "10");
    for (initial, expected) in [(99u64, "100"), (999, "1000"), (9999, "10000")] {
        assert_eq!(inc(&Nat::from_u64(initial)).to_decimal(), expected);
    }
    assert_eq!(inc(&Nat::from_u64(12345)).to_decimal(), "12346");

    let mut n = Nat::zero();
    for _ in 0..10 {
        n = inc(&n);
    }
    assert_eq!(n.to_decimal(), "10");
}

#[test]
fn natural_increment_postfix_rows() {
    let n = Nat::from_u64(5);
    let old = n.clone();
    let n = inc(&n);
    assert_eq!(old.to_decimal(), "5");
    assert_eq!(n.to_decimal(), "6");

    let z = Nat::zero();
    let old = z.clone();
    let z = inc(&z);
    assert_eq!(old.to_decimal(), "0");
    assert_eq!(z.to_decimal(), "1");
}

// ===========================================================================
// Lovelace.Natural.Tests.NaturalSubtractTests — --n (prefix) and n-- (postfix)
// ===========================================================================

fn dec(n: &Nat) -> Option<Nat> {
    n.sub(&one())
}

#[test]
fn natural_decrement_prefix_rows() {
    assert!(dec(&Nat::from_u64(1)).expect("1 - 1").is_zero());
    assert_eq!(dec(&Nat::from_u64(10)).expect("10 - 1").to_decimal(), "9");
    assert_eq!(dec(&Nat::from_u64(1000)).expect("1000 - 1").to_decimal(), "999");
    assert!(dec(&Nat::zero()).is_none(), "decrementing zero is refused (None)");
}

#[test]
fn natural_decrement_postfix_row() {
    let n = nat("5");
    let previous = n.clone();
    let n = dec(&n).expect("5 - 1");
    assert_eq!(previous.to_decimal(), "5");
    assert_eq!(n.to_decimal(), "4");
}

// ===========================================================================
// Lovelace.Natural.Tests.NaturalRandomizedCrossCheckTests — factorial (5 InlineData rows)
// ===========================================================================

#[test]
fn natural_factorial_rows() {
    let expected = [
        (0u64, "1"),
        (1, "1"),
        (5, "120"),
        (20, "2432902008176640000"),
        (100, "93326215443944152681699238856266700490715968264381621468592963895217599993229915608941463976156518286253697920827223758251185210916864000000000000000000000000"),
    ];
    for (n, text) in expected {
        assert_eq!(Nat::factorial(n).to_decimal(), text, "{n}!");
    }
}

// ===========================================================================
// Lovelace.Natural.Tests.NaturalMultiplyTests — concurrent reads of one shared operand
// ===========================================================================

#[test]
fn natural_concurrent_multiply_rows() {
    let shared = Arc::new(nat(&("1".to_string() + &"0".repeat(200))));
    let expected = "1".to_string() + &"0".repeat(400);
    let handles: Vec<_> = (0..16)
        .map(|_| {
            let shared = Arc::clone(&shared);
            let expected = expected.clone();
            std::thread::spawn(move || {
                let product = shared.mul(&shared);
                assert_eq!(product.to_decimal(), expected);
            })
        })
        .collect();
    for handle in handles {
        handle.join().expect("worker");
    }

    let a = nat("100000000000");
    let handles: Vec<_> = (0..32)
        .map(|_| {
            let a = a.clone();
            std::thread::spawn(move || assert_eq!(a.mul(&a).to_decimal(), "10000000000000000000000"))
        })
        .collect();
    for handle in handles {
        handle.join().expect("worker");
    }
}

// ===========================================================================
// Lovelace.Natural.Tests.NaturalRandomizedCrossCheckTests — fixed carry patterns and the
// overflow-alignment division. The expectation table was computed independently with BigInt.
// ===========================================================================

#[test]
fn natural_adversarial_carry_patterns() {
    let cases: &[(&str, &str, &str, &str, Option<&str>, &str, &str)] = &[
    ("9999999999999999999999999999999999999999", "9999999999999999999999999999999999999999", "19999999999999999999999999999999999999998", "99999999999999999999999999999999999999980000000000000000000000000000000000000001", Some("0"), "1", "0"),
    ("9999999999999999999999999999999999999999", "10000000000000000000000000000000000000000", "19999999999999999999999999999999999999999", "99999999999999999999999999999999999999990000000000000000000000000000000000000000", None, "0", "9999999999999999999999999999999999999999"),
    ("9999999999999999999999999999999999999999", "18446744073709551615", "10000000000000000000018446744073709551614", "184467440737095516149999999999999999999981553255926290448385", Some("9999999999999999999981553255926290448384"), "542101086242752217033", "2098486950404341704"),
    ("9999999999999999999999999999999999999999", "18446744073709551616", "10000000000000000000018446744073709551615", "184467440737095516159999999999999999999981553255926290448384", Some("9999999999999999999981553255926290448383"), "542101086242752217003", "13399722918938673151"),
    ("9999999999999999999999999999999999999999", "340282366920938463463374607431768211455", "10340282366920938463463374607431768211454", "3402823669209384634633746074317682114549659717633079061536536625392568231788545", Some("9659717633079061536536625392568231788544"), "29", "131811359292784559562136384478721867804"),
    ("10000000000000000000000000000000000000000", "9999999999999999999999999999999999999999", "19999999999999999999999999999999999999999", "99999999999999999999999999999999999999990000000000000000000000000000000000000000", Some("1"), "1", "1"),
    ("10000000000000000000000000000000000000000", "10000000000000000000000000000000000000000", "20000000000000000000000000000000000000000", "100000000000000000000000000000000000000000000000000000000000000000000000000000000", Some("0"), "1", "0"),
    ("10000000000000000000000000000000000000000", "18446744073709551615", "10000000000000000000018446744073709551615", "184467440737095516150000000000000000000000000000000000000000", Some("9999999999999999999981553255926290448385"), "542101086242752217033", "2098486950404341705"),
    ("10000000000000000000000000000000000000000", "18446744073709551616", "10000000000000000000018446744073709551616", "184467440737095516160000000000000000000000000000000000000000", Some("9999999999999999999981553255926290448384"), "542101086242752217003", "13399722918938673152"),
    ("10000000000000000000000000000000000000000", "340282366920938463463374607431768211455", "10340282366920938463463374607431768211455", "3402823669209384634633746074317682114550000000000000000000000000000000000000000", Some("9659717633079061536536625392568231788545"), "29", "131811359292784559562136384478721867805"),
    ("18446744073709551615", "9999999999999999999999999999999999999999", "10000000000000000000018446744073709551614", "184467440737095516149999999999999999999981553255926290448385", None, "0", "18446744073709551615"),
    ("18446744073709551615", "10000000000000000000000000000000000000000", "10000000000000000000018446744073709551615", "184467440737095516150000000000000000000000000000000000000000", None, "0", "18446744073709551615"),
    ("18446744073709551615", "18446744073709551615", "36893488147419103230", "340282366920938463426481119284349108225", Some("0"), "1", "0"),
    ("18446744073709551615", "18446744073709551616", "36893488147419103231", "340282366920938463444927863358058659840", None, "0", "18446744073709551615"),
    ("18446744073709551615", "340282366920938463463374607431768211455", "340282366920938463481821351505477763070", "6277101735386680763495507056286727952620534092958556749825", None, "0", "18446744073709551615"),
    ("18446744073709551616", "9999999999999999999999999999999999999999", "10000000000000000000018446744073709551615", "184467440737095516159999999999999999999981553255926290448384", None, "0", "18446744073709551616"),
    ("18446744073709551616", "10000000000000000000000000000000000000000", "10000000000000000000018446744073709551616", "184467440737095516160000000000000000000000000000000000000000", None, "0", "18446744073709551616"),
    ("18446744073709551616", "18446744073709551615", "36893488147419103231", "340282366920938463444927863358058659840", Some("1"), "1", "1"),
    ("18446744073709551616", "18446744073709551616", "36893488147419103232", "340282366920938463463374607431768211456", Some("0"), "1", "0"),
    ("18446744073709551616", "340282366920938463463374607431768211455", "340282366920938463481821351505477763071", "6277101735386680763835789423207666416083908700390324961280", None, "0", "18446744073709551616"),
    ("340282366920938463463374607431768211455", "9999999999999999999999999999999999999999", "10340282366920938463463374607431768211454", "3402823669209384634633746074317682114549659717633079061536536625392568231788545", None, "0", "340282366920938463463374607431768211455"),
    ("340282366920938463463374607431768211455", "10000000000000000000000000000000000000000", "10340282366920938463463374607431768211455", "3402823669209384634633746074317682114550000000000000000000000000000000000000000", None, "0", "340282366920938463463374607431768211455"),
    ("340282366920938463463374607431768211455", "18446744073709551615", "340282366920938463481821351505477763070", "6277101735386680763495507056286727952620534092958556749825", Some("340282366920938463444927863358058659840"), "18446744073709551617", "0"),
    ("340282366920938463463374607431768211455", "18446744073709551616", "340282366920938463481821351505477763071", "6277101735386680763835789423207666416083908700390324961280", Some("340282366920938463444927863358058659839"), "18446744073709551615", "18446744073709551615"),
    ("340282366920938463463374607431768211455", "340282366920938463463374607431768211455", "680564733841876926926749214863536422910", "115792089237316195423570985008687907852589419931798687112530834793049593217025", Some("0"), "1", "0"),
    ];
    for (x, y, sum, product, difference, quotient, remainder) in cases {
        let a = nat(x);
        let b = nat(y);
        assert_eq!(a.add(&b).to_decimal(), *sum, "{x} + {y}");
        assert_eq!(a.mul(&b).to_decimal(), *product, "{x} * {y}");
        if let Some(difference) = difference {
            assert_eq!(a.sub(&b).expect("a >= b").to_decimal(), *difference, "{x} - {y}");
        }
        let (q, r) = a.div_rem(&b).expect("nonzero divisor");
        assert_eq!(q.to_decimal(), *quotient, "{x} / {y}");
        assert_eq!(r.to_decimal(), *remainder, "{x} % {y}");
    }
}

#[test]
fn natural_overflow_alignment_division() {
    let dividend = nat("9999999999999999999999999999999999999999")
        .mul(&nat("18446744073709551616"));
    let divisor = nat("1000000000000000000000000000000");
    let (q, r) = dividend.div_rem(&divisor).expect("nonzero divisor");
    assert_eq!(q.to_decimal(), "184467440737095516159999999999");
    assert_eq!(r.to_decimal(), "999999999981553255926290448384");
}

// ===========================================================================
// Lovelace.Natural.Tests.NaturalDecimalCacheTests — lazy materialization under concurrency.
// (The identity assertions on the cached STRING are refused: Rust returns an owned String.)
// ===========================================================================

#[test]
fn natural_concurrent_arithmetic_on_lazy_parsed_values() {
    let a = nat("1234567890123456789012345678901234567890");
    let b = nat("9876543210987654321098765432109876543210");
    let sum = "11111111101111111110111111111011111111100";
    let product = "12193263113702179522618503273386678859448712086533622923332237463801111263526900";
    let handles: Vec<_> = (0..64)
        .map(|_| {
            let a = a.clone();
            let b = b.clone();
            std::thread::spawn(move || {
                assert_eq!(a.add(&b).to_decimal(), sum);
                assert_eq!(a.mul(&b).to_decimal(), product);
            })
        })
        .collect();
    for handle in handles {
        handle.join().expect("worker");
    }
}

// ===========================================================================
// Lovelace.Integer.Tests.IntegerIncrementTests — Increment/Decrement and the prefix operators
// (the mutating operators are expressed as the immutable value they produce).
// ===========================================================================

fn inc_int(n: &Int) -> Int {
    n.add(&Int::from_i64(1))
}

fn dec_int(n: &Int) -> Int {
    n.sub(&Int::from_i64(1))
}

#[test]
fn integer_increment_decrement_rows() {
    assert_eq!(inc_int(&int("5")).to_decimal(), "6");
    assert!(inc_int(&int("-1")).is_zero());
    assert_eq!(inc_int(&int("0")).to_decimal(), "1");
    assert_eq!(dec_int(&int("5")).to_decimal(), "4");
    assert_eq!(dec_int(&int("0")).to_decimal(), "-1");
    assert_eq!(dec_int(&int("-2")).to_decimal(), "-3");

    // PreIncrementOperator / PreDecrementOperator: the new value is both bound and returned.
    let x = inc_int(&int("5"));
    assert_eq!(x.to_decimal(), "6");
    let y = inc_int(&int("5"));
    assert_eq!(y.to_decimal(), "6");
    let x = dec_int(&int("5"));
    assert_eq!(x.to_decimal(), "4");
    let y = dec_int(&int("5"));
    assert_eq!(y.to_decimal(), "4");
}

// ===========================================================================
// Lovelace.Integer.Tests.IntegerPredicateTests — IsEvenInteger / IsOddInteger
// ===========================================================================

fn is_even(n: &Int) -> bool {
    n.div_rem_signed(&int("2")).expect("2 is nonzero").1.is_zero()
}

#[test]
fn integer_parity_predicate_rows() {
    assert!(is_even(&int("4")));
    assert!(!is_even(&int("3")));
    assert!(is_even(&int("-4")));
    assert!(is_even(&int("0")));

    assert!(!is_even(&int("7")));
    assert!(!is_even(&int("-3")));
    assert!(is_even(&int("6")));
    assert!(is_even(&int("0")));
}

// ===========================================================================
// Lovelace.Integer.Tests.IntegerConstructorTests — the natural / signed-magnitude constructors
// ===========================================================================

#[test]
fn integer_constructor_rows() {
    // Integer(Natural): always non-negative.
    let from_nat = Int::from_decimal(&Nat::from_u64(99).to_decimal()).expect("99");
    assert!(!from_nat.is_negative());
    assert_eq!(from_nat.to_decimal(), "99");

    // Integer(zero Natural) is zero.
    let zero = Int::from_decimal(&Nat::zero().to_decimal()).expect("0");
    assert!(zero.is_zero());

    // Integer(Natural, isNegative: true) with a nonzero magnitude is negative.
    let negative = int("-5");
    assert!(negative.is_negative());

    // Integer(zero Natural, isNegative: true) normalises to positive zero.
    let negative_zero = int("-0");
    assert!(negative_zero.is_zero());
    assert!(!negative_zero.is_negative());
}

// ===========================================================================
// Lovelace.Integer.Tests.IntegerFactorialTests — a negative Integer is a named domain refusal
// ===========================================================================

#[test]
fn integer_factorial_of_negative_is_refused() {
    let error = refusal("(-1)!");
    assert!(error.message.contains("Factorial is not defined for negative integers"), "{}", error.message);
}

// ===========================================================================
// Lovelace.Array.Tests.NdArrayTests — ArrayMath through the language builtins
// ===========================================================================

#[test]
fn array_sum_all_and_axis() {
    assert_eq!(eval("sum([[1, 2], [3, 4]])").display(), "10");
    assert_eq!(eval("sum([[1, 2], [3, 4]], 0)").typed(), "[4, 6] (Vector)");
    assert_eq!(eval("sum([[1, 2], [3, 4]], 1)").typed(), "[3, 7] (Vector)");
}

#[test]
fn array_min_max() {
    assert_eq!(eval("min([3, 1, 4, 1])").display(), "1");
    assert_eq!(eval("max([3, 1, 4, 1])").display(), "4");
}

#[test]
fn array_mean_and_norm() {
    assert_eq!(eval("mean([1, 2, 3])").display(), "2");
    assert_eq!(eval("norm([3, 4])").display(), "5");
}

#[test]
fn array_dot_and_cross() {
    assert_eq!(eval("dot([1, 2, 3], [4, 5, 6])").display(), "32");
    assert_eq!(eval("cross([1, 0, 0], [0, 1, 0])").typed(), "[0, 0, 1] (Vector)");
}

#[test]
fn array_matmul_2x2() {
    assert_eq!(
        eval("matmul([[1, 2], [3, 4]], [[5, 6], [7, 8]])").typed(),
        "[[19, 22], [43, 50]] (Array)"
    );
}

#[test]
fn array_det_inverse_trace() {
    assert_eq!(eval("det([[1, 2], [3, 4]])").display(), "-2");
    assert_eq!(eval("trace([[1, 2], [3, 4]])").display(), "5");
    assert_eq!(eval("inv([[1, 2], [3, 4]])").typed(), "[[-2, 1], [1.5, -0.5]] (Array)");
}

#[test]
fn array_inverse_singular_is_refused() {
    let error = refusal("inv([[1, 2], [2, 4]])");
    assert_eq!(error.code, "InvalidOperation");
    assert!(error.message.contains("singular"), "{}", error.message);
}

// ===========================================================================
// Lovelace.Real.Tests.RealModulusTests — operator% on Real operands
// ===========================================================================

fn assert_real(script: &str, expected: &str) {
    let value = eval(script);
    let Value::Real(actual) = &value else {
        panic!("'{script}' answered {} ({})", value.display(), value.kind_name());
    };
    assert_eq!(actual, &real(expected), "'{script}' => {}", actual.to_string());
}

#[test]
fn real_modulus_rows() {
    assert_real("7.0 % 3.0", "1");
    assert_real("2.0 % 5.0", "2");
    assert_eq!(eval("0.0 % 5.0").display(), "0");
    assert_real("-7.0 % 3.0", "-1");
    assert_real("7.0 % -3.0", "1");
    assert_real("2.5 % 1.2", "0.1");
    assert_eq!(eval("6.0 % 3.0").display(), "0");
    let error = refusal("5.0 % 0.0");
    assert_eq!(error.code, "DivisionByZero");
}

// ===========================================================================
// Lovelace.Real.Tests.RealPiTests — the digit count is validated (0 and negative are refused)
// ===========================================================================

#[test]
fn real_pi_refuses_zero_and_negative_digit_counts() {
    for script in ["pi(0)", "pi(-1)"] {
        let error = refusal(script);
        assert_eq!(error.code, "InvalidArgument", "{script}: {}", error.message);
        assert!(error.message.contains("digits"), "{script}: {}", error.message);
    }
}

// ===========================================================================
// Lovelace.Real.Tests.RealConstructorTests — the decimal constructors are the string values
// ===========================================================================

#[test]
fn real_decimal_constructor_rows() {
    // Constructor_GivenDecimalMaxValue_ParsesWithoutThrow
    let max = "79228162514264337593543950335";
    assert_eq!(real(max).to_string(), max);

    // Constructor_GivenNegativeDecimal_ParsesCorrectly
    let negative = real("-3.14");
    assert!(negative.to_string().starts_with('-'));
    assert_eq!(negative, real("-3.14"));

    // Constructor_GivenZeroDecimal_ReturnsZero
    assert!(real("0").is_zero());
}

// ===========================================================================
// Lovelace.Suite.Tests.CrossEngineIsolationTests and the shared-plugin isolation rows.
//
// The Rust engine owns its assumption set (there is no plugin instance to share), so the same
// behavioural contract is asserted on two engines: one engine's assumptions never decide another's
// query, and every reader (simplify, inspect, assumptions) answers for the asking engine.
// ===========================================================================

#[test]
fn concurrent_engines_keep_their_own_assumptions() {
    let mut a = Engine::new();
    let mut b = Engine::new();
    eval_in(&mut a, "x = symbol(\"x\"); assume(x > 0)");
    eval_in(&mut b, "x = symbol(\"x\"); assume(x < 0)");
    assert_eq!(eval_in(&mut a, "simplify(x > 0)").display(), "True");
    assert_eq!(eval_in(&mut b, "simplify(x < 0)").display(), "True");
    assert_eq!(eval_in(&mut a, "simplify(x < 0)").display(), "False");
    assert_eq!(eval_in(&mut b, "simplify(x > 0)").display(), "False");
}

#[test]
fn assumption_in_one_engine_does_not_decide_another() {
    let mut a = Engine::new();
    let mut b = Engine::new();
    eval_in(&mut a, "x = symbol(\"x\"); assume(x > 0)");
    assert_eq!(eval_in(&mut a, "simplify(x > 0)").display(), "True");

    assert_eq!(eval_in(&mut b, "x = symbol(\"x\"); simplify(x > 0)").display(), "x > 0");
    assert_eq!(eval_in(&mut b, "simplify(x < 0)").display(), "x < 0");
    assert_eq!(eval_in(&mut a, "simplify(x > 0)").display(), "True");
    assert_eq!(eval_in(&mut a, "simplify(x < 0)").display(), "False");
}

#[test]
fn every_assumption_reader_answers_for_the_asking_engine() {
    fn items(value: &Value) -> &[Value] {
        match value {
            Value::Vector(items) => items,
            other => panic!("expected a vector, got {}", other.kind_name()),
        }
    }

    let mut a = Engine::new();
    let mut b = Engine::new();
    eval_in(&mut a, "x = symbol(\"x\"); assume(x > 0)");
    eval_in(&mut b, "x = symbol(\"x\")");

    assert!(!items(&eval_in(&mut a, "inspect(x).assumptions")).is_empty());
    assert!(items(&eval_in(&mut b, "inspect(x).assumptions")).is_empty());
    assert!(!items(&eval_in(&mut a, "assumptions().assumptions")).is_empty());
    assert!(items(&eval_in(&mut b, "assumptions().assumptions")).is_empty());

    eval_in(&mut b, "assume_clear()");
    assert!(!items(&eval_in(&mut a, "assumptions().assumptions")).is_empty());
}

// ===========================================================================
// Lovelace.Suite.Tests.SuiteEngineTests — a user function exposes its signature, and its body is
// what evaluating it runs.
// ===========================================================================

#[test]
fn function_definition_exposes_signature_and_body() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "func f(a, b) { a + b }");
    let functions = engine.functions();
    let (name, parameters) = functions
        .iter()
        .find(|(name, _)| name == "f")
        .unwrap_or_else(|| panic!("f is not in the function table: {functions:?}"));
    assert_eq!(name, "f");
    assert_eq!(parameters, &["a".to_string(), "b".to_string()]);
    // A non-empty body is what makes the call answer: the defined function is not a builtin.
    assert_eq!(eval_in(&mut engine, "f(1, 2)").display(), "3");
}

// ===========================================================================
// Lovelace.Integer.Tests.IntegerNegationTests — unary plus returns a copy with the same value and
// sign. The Rust Integer has no operator+ overload, so the language's unary plus (which is the
// identity) is asserted on Integer operands.
// ===========================================================================

#[test]
fn integer_unary_plus_rows() {
    fn integer_value(script: &str) -> Int {
        match eval(script) {
            Value::Integer(value) => value,
            other => panic!("'{script}' answered {} ({})", other.display(), other.kind_name()),
        }
    }

    // GivenPositiveValue_ReturnsCopyWithSameSign
    let positive = integer_value("x = 0 - 5 + 10; +x");
    assert_eq!(positive.to_decimal(), "5");
    assert_eq!(positive.sign(), 1);

    // GivenNegativeValue_ReturnsCopyWithSameSign
    let negative = integer_value("x = -5; +x");
    assert_eq!(negative.to_decimal(), "-5");
    assert_eq!(negative.sign(), -1);

    // GivenZero_ReturnsZero
    let zero = integer_value("x = 0 * -1; +x");
    assert!(zero.is_zero());
    assert!(!zero.is_negative());
}
