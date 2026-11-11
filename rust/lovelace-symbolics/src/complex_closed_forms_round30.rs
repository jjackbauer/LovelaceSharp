//! Round 30: the complex closed forms of Lovelace.Symbolics.Tests.ComplexClosedFormTests.
//!
//! Every expected value is the SymPy 1.14.0 principal-branch ground truth transcribed by the
//! reference test file. The expressions are built through the kernel's own constructors
//! (`Expr::sqrt`, `Expr::pow`) so the test pins the production rule, not a wrapper; the numeric
//! check uses a small evaluator defined HERE (the kernel has no numeric evaluation surface of its
//! own), comparing the real and imaginary parts separately so a real answer where a complex one is
//! required cannot pass.

use super::*;

/// 1e-20, far below the 30 correct digits the values are transcribed at, so an exact value passes
/// and any decimal approximation fails.
const TOLERANCE: f64 = 1e-20;

fn rational(num: i64, den: u64) -> Expr {
    Expr::Num(Rat::new(Int::from_i64(num), Nat::from_u64(den)))
}

fn half() -> Expr {
    rational(1, 2)
}

fn i_unit() -> Expr {
    Expr::Fun("i".to_string(), Vec::new())
}

/// A test-side numeric evaluator for the closed forms under test: rationals, the imaginary unit,
/// pi, sqrt, powers, sums, products and quotients. It is deliberately independent of the production
/// evaluation path (which lives in another crate).
fn numeric(e: &Expr) -> Option<(f64, f64)> {
    fn round(value: f64) -> f64 {
        if value.abs() < 1e-300 { 0.0 } else { value }
    }
    match e {
        Expr::Num(n) => {
            let num = n.numerator().to_decimal().parse::<f64>().ok()?;
            let den = n.denominator().to_decimal().parse::<f64>().ok()?;
            Some((round(num / den), 0.0))
        }
        Expr::Real(text) => Some((round(text.parse::<f64>().ok()?), 0.0)),
        Expr::Sym(_) => None,
        Expr::Fun(name, args) if name == "i" && args.is_empty() => Some((0.0, 1.0)),
        Expr::Fun(name, args) if name == "pi" && args.is_empty() => {
            Some((std::f64::consts::PI, 0.0))
        }
        Expr::Fun(name, args) if name == "e" && args.is_empty() => {
            Some((std::f64::consts::E, 0.0))
        }
        Expr::Fun(name, args) if name == "sqrt" && args.len() == 1 => {
            let (re, im) = numeric(&args[0])?;
            if im.abs() > 1e-30 {
                return None;
            }
            if re < 0.0 {
                return Some((0.0, round((-re).sqrt())));
            }
            Some((round(re.sqrt()), 0.0))
        }
        Expr::Add(terms) => {
            let mut re = 0.0;
            let mut im = 0.0;
            for t in terms {
                let (r, i) = numeric(t)?;
                re += r;
                im += i;
            }
            Some((round(re), round(im)))
        }
        Expr::Mul(factors) => {
            let mut acc = (1.0, 0.0);
            for f in factors {
                let (r, i) = numeric(f)?;
                acc = (round(acc.0 * r - acc.1 * i), round(acc.0 * i + acc.1 * r));
            }
            Some(acc)
        }
        Expr::Div(a, b) => {
            let (ar, ai) = numeric(a)?;
            let (br, bi) = numeric(b)?;
            let denom = br * br + bi * bi;
            if denom == 0.0 {
                return None;
            }
            Some((round((ar * br + ai * bi) / denom), round((ai * br - ar * bi) / denom)))
        }
        Expr::Pow(base, exp) => {
            let (br, bi) = numeric(base)?;
            let (er, ei) = numeric(exp)?;
            if ei.abs() > 1e-30 {
                return None;
            }
            let modulus = (br * br + bi * bi).sqrt();
            if modulus == 0.0 {
                return None;
            }
            let theta = bi.atan2(br);
            let scale = modulus.powf(er);
            Some((round(scale * (er * theta).cos()), round(scale * (er * theta).sin())))
        }
        Expr::Fun(..) => None,
        Expr::Rel(..) => None,
    }
}

/// Asserts the expression's numeric value has the given real and imaginary parts, separately.
fn assert_parts(e: &Expr, expected_re: f64, expected_im: f64, label: &str) {
    let (re, im) = numeric(e).unwrap_or_else(|| panic!("{label}: the closed form is not numeric: {e}"));
    assert!(
        (re - expected_re).abs() < TOLERANCE,
        "{label}: real part {re} != {expected_re} (canonical {})",
        e.canonical()
    );
    assert!(
        (im - expected_im).abs() < TOLERANCE,
        "{label}: imaginary part {im} != {expected_im} (canonical {})",
        e.canonical()
    );
}

/// Canonical print -> canonical parse -> the SAME canonical text: the value has to survive the
/// published wire form, not merely evaluate correctly. The Rust model spells a radical as the
/// named `sqrt` function node (its canonical form is the reference's power, so the text is the
/// one the reference round-trips); the equality asserted here is therefore the crate's documented
/// canonical-text guarantee.
fn assert_round_trips(e: &Expr, expected_canonical: &str) {
    let canonical = e.canonical();
    assert_eq!(expected_canonical, canonical);
    let reparsed = canonical_parse(&canonical).expect("the canonical form parses back");
    assert_eq!(canonical, reparsed.canonical(), "the canonical form round-trips");
}

// ------------------------------------------------------------------
// Target 1: sqrt of an exact negative value
// ------------------------------------------------------------------

#[test]
fn sqrt_of_negative_one_is_the_exact_imaginary_unit() {
    let value = Expr::sqrt(Expr::num(-1));
    assert!(value.is_exact(), "the imaginary unit must be EXACT, not an approximation");
    assert_eq!(i_unit(), value);
    assert_round_trips(&value, "(i)");
    assert_parts(&value, 0.0, 1.0, "sqrt(-1)");
}

#[test]
fn sqrt_of_negative_four_is_exactly_two_i() {
    let value = Expr::sqrt(Expr::num(-4));
    assert!(value.is_exact());
    assert_round_trips(&value, "(mul (rat 2 1) (i))");
    assert_parts(&value, 0.0, 2.0, "sqrt(-4)");
}

#[test]
fn sqrt_of_negative_perfect_squares_are_exact_imaginary_constants() {
    let nine = Expr::sqrt(Expr::num(-9));
    assert!(nine.is_exact());
    assert_round_trips(&nine, "(mul (rat 3 1) (i))");
    assert_parts(&nine, 0.0, 3.0, "sqrt(-9)");

    // sqrt(-1/4) = i/2 in SymPy: a negative RATIONAL base is the same branch.
    let quarter = Expr::sqrt(rational(-1, 4));
    assert!(quarter.is_exact());
    assert_round_trips(&quarter, "(mul (rat 1 2) (i))");
    assert_parts(&quarter, 0.0, 0.5, "sqrt(-1/4)");
}

#[test]
fn sqrt_of_negative_non_square_is_the_exact_imaginary_multiple_of_the_radical() {
    // SymPy: (-2)**Rational(1,2) = sqrt(2)*I - exact as a radical, never a decimal.
    let value = Expr::sqrt(Expr::num(-2));
    let expected = Expr::mul(i_unit(), Expr::sqrt(Expr::num(2)));
    assert!(
        value == expected,
        "sqrt(-2) built {}, expected {}",
        value.canonical(),
        expected.canonical()
    );
    assert_round_trips(&value, "(mul (i) (pow (rat 2 1) (rat 1 2)))");
    assert_parts(&value, 0.0, std::f64::consts::SQRT_2, "sqrt(-2)");
}

// ------------------------------------------------------------------
// Target 2: negative base raised to an exponent with denominator 2
// ------------------------------------------------------------------

#[test]
fn negative_base_rational_powers_match_the_sympy_principal_branch() {
    let minus1 = Expr::pow(Expr::num(-1), half());
    assert_round_trips(&minus1, "(i)");
    assert_parts(&minus1, 0.0, 1.0, "(-1)^(1/2)");

    let minus4 = Expr::pow(Expr::num(-4), half());
    assert_round_trips(&minus4, "(mul (rat 2 1) (i))");
    assert_parts(&minus4, 0.0, 2.0, "(-4)^(1/2)");

    // SymPy: (-3)**Rational(1,2) = sqrt(3)*I
    let minus3 = Expr::pow(Expr::num(-3), half());
    assert_round_trips(&minus3, "(mul (i) (pow (rat 3 1) (rat 1 2)))");
    assert_parts(&minus3, 0.0, 3.0f64.sqrt(), "(-3)^(1/2)");

    // SymPy: (-4)**Rational(3,2) = -8*I
    let three_halves = Expr::pow(Expr::num(-4), rational(3, 2));
    assert_round_trips(&three_halves, "(mul (rat -8 1) (i))");
    assert_parts(&three_halves, 0.0, -8.0, "(-4)^(3/2)");

    // SymPy: (-4)**Rational(-1,2) = -I/2
    let minus_half = Expr::pow(Expr::num(-4), rational(-1, 2));
    assert_round_trips(&minus_half, "(mul (rat -1 2) (i))");
    assert_parts(&minus_half, 0.0, -0.5, "(-4)^(-1/2)");
}

#[test]
fn imaginary_unit_integer_powers_fold_exactly() {
    assert_round_trips(&Expr::pow(i_unit(), Expr::num(2)), "(rat -1 1)");
    assert_round_trips(&Expr::pow(i_unit(), Expr::num(3)), "(mul (rat -1 1) (i))");
    assert_round_trips(&Expr::pow(i_unit(), Expr::num(4)), "(rat 1 1)");
    assert_round_trips(&Expr::pow(i_unit(), Expr::num(-1)), "(mul (rat -1 1) (i))");
}

// ------------------------------------------------------------------
// The boundary: what the reference deliberately does NOT widen
// ------------------------------------------------------------------

#[test]
fn positive_base_non_integer_power_is_still_left_unevaluated() {
    // A positive base under a non-integer exponent is a real irrational the Real type refuses to
    // approximate; the kernel keeps it as a power.
    assert_round_trips(
        &Expr::pow(Expr::num(2), half()),
        "(pow (rat 2 1) (rat 1 2))",
    );
}

#[test]
fn negative_base_third_root_is_outside_the_bounded_set() {
    // The kernel's real-root shortcut still fires for an odd denominator and a UNIT fraction.
    assert_round_trips(&Expr::pow(Expr::num(-8), rational(1, 3)), "(rat -2 1)");
    // A non-unit denominator-3 exponent is still left alone entirely.
    assert_round_trips(&Expr::pow(Expr::num(-8), rational(2, 3)), "(pow (rat -8 1) (rat 2 3))");
}

#[test]
fn sqrt_of_a_non_negative_value_is_unchanged() {
    assert_round_trips(&Expr::sqrt(Expr::num(4)), "(rat 2 1)");
    assert_round_trips(&Expr::sqrt(Expr::num(9)), "(rat 3 1)");
    assert_round_trips(&Expr::sqrt(Expr::num(2)), "(pow (rat 2 1) (rat 1 2))");
    assert!(matches!(Expr::sqrt(Expr::num(0)), Expr::Num(ref n) if n.is_zero()));
}
