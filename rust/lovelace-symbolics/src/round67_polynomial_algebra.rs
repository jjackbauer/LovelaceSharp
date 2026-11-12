//! Round 67: the polynomial content / primitive-part / gcd / lcm / resultant / discriminant /
//! factor census rows (`Lovelace.Symbolics.Tests.PolynomialsApiTests`), plus the zero, constant,
//! non-monic and negative-leading-coefficient edges the reference decides.
//!
//! The five census ids and the oracle probes are listed in
//! `docs/goal-parity-contract/round-67/polynomial-algebra.md`. Each test states the C# assertion
//! on the same values through the Rust `polynomials` facade; the expectation is never weakened.

use lovelace_int::Int;
use lovelace_nat::Nat;

use crate::polynomials::{content, discriminant, factor, gcd, lcm, primitive_part, resultant};
use crate::{Expr, Rat, poly_coeffs, square_free_univariate};

fn rat(n: i64, d: u64) -> Rat {
    Rat::new(Int::from_i64(n), Nat::from_u64(d))
}

fn x() -> Expr {
    Expr::sym("x")
}

fn x_pow(exponent: i64) -> Expr {
    Expr::pow(x(), Expr::num(exponent))
}

fn sum(terms: Vec<Expr>) -> Expr {
    terms.into_iter().fold(Expr::num(0), Expr::add)
}

fn product(terms: Vec<Expr>) -> Expr {
    terms.into_iter().fold(Expr::num(1), Expr::mul)
}

fn poly(expression: &Expr) -> Vec<Rat> {
    poly_coeffs(expression, "x").unwrap_or_else(|| panic!("expected a polynomial: {expression}"))
}

/// Coefficient-vector product, the test-side statement of `Polynomial.Multiply`.
fn multiply(a: &[Rat], b: &[Rat]) -> Vec<Rat> {
    let mut out = vec![Rat::zero(); a.len() + b.len() - 1];
    for (i, left) in a.iter().enumerate() {
        for (j, right) in b.iter().enumerate() {
            out[i + j] = out[i + j].add(&left.mul(right));
        }
    }
    out
}

/// Coefficient-vector power, the test-side statement of `Polynomial.Pow`.
fn power(base: &[Rat], exponent: u64) -> Vec<Rat> {
    let mut result = vec![Rat::one()];
    for _ in 0..exponent {
        result = multiply(&result, base);
    }
    result
}

// ---------------------------------------------------------------------------
// The five census rows.
// ---------------------------------------------------------------------------

/// `PolynomialsApiTests.cs:33 Content_PrimitivePart_SplitCoefficients`: 2*x^2 + 4*x + 6 has
/// content 2 and primitive part x^2 + 2*x + 3.
#[test]
fn content_primitive_part_split_coefficients() {
    let p = poly(&sum(vec![
        product(vec![Expr::num(2), x_pow(2)]),
        product(vec![Expr::num(4), x()]),
        Expr::num(6),
    ]));
    assert_eq!(content(&p), rat(2, 1));
    assert_eq!(primitive_part(&p), vec![rat(3, 1), rat(2, 1), rat(1, 1)]);
}

/// `PolynomialsApiTests.cs:56 Gcd_Lcm_AreConsistent`: gcd(x^2 - 1, x^2 + 2*x + 1) is the monic
/// x + 1, and the product of the gcd and lcm equals the product of the inputs.
#[test]
fn gcd_lcm_are_consistent() {
    let a = poly(&sum(vec![x_pow(2), Expr::num(-1)]));
    let b = poly(&sum(vec![
        x_pow(2),
        product(vec![Expr::num(2), x()]),
        Expr::num(1),
    ]));
    let g = gcd(&a, &b);
    assert_eq!(g, vec![rat(1, 1), rat(1, 1)], "monic x + 1");
    let l = lcm(&a, &b);
    assert_eq!(
        l,
        vec![rat(-1, 1), rat(-1, 1), rat(1, 1), rat(1, 1)],
        "(x - 1)*(x + 1)^2"
    );
    assert_eq!(multiply(&g, &l), multiply(&a, &b));
}

/// `PolynomialsApiTests.cs:107 Resultant_MatchesKnownValues`: res(x^2 - 2, x - 1) = -1, two
/// polynomials sharing the root 1 have resultant zero, and x - 1 against x - 2 has resultant 1.
#[test]
fn resultant_matches_known_values() {
    let f1 = poly(&sum(vec![x_pow(2), Expr::num(-2)]));
    let g1 = poly(&sum(vec![x(), Expr::num(-1)]));
    assert_eq!(resultant(&f1, &g1), rat(-1, 1));

    let f2 = poly(&sum(vec![x(), Expr::num(-1)]));
    let g2 = poly(&sum(vec![x(), Expr::num(-1)]));
    assert_eq!(resultant(&f2, &g2), rat(0, 1));

    let f3 = poly(&sum(vec![x(), Expr::num(-1)]));
    let g3 = poly(&sum(vec![x(), Expr::num(-2)]));
    assert_eq!(resultant(&f3, &g3), rat(1, 1));
}

/// `PolynomialsApiTests.cs:126 Discriminant_MatchesKnownValues`: x^2 + 3*x + 2 has
/// discriminant 1 and x^2 - 2 has discriminant 8.
#[test]
fn discriminant_matches_known_values() {
    let q1 = poly(&sum(vec![
        x_pow(2),
        product(vec![Expr::num(3), x()]),
        Expr::num(2),
    ]));
    assert_eq!(discriminant(&q1), rat(1, 1));

    let q2 = poly(&sum(vec![x_pow(2), Expr::num(-2)]));
    assert_eq!(discriminant(&q2), rat(8, 1));
}

/// `PolynomialsApiTests.cs:85 Factor_QuarticWithRationalRoots_FourLinearFactors`: x^4 - 5*x^2 + 4
/// factors into four distinct monic linear factors, content 1, and the product reconstructs the
/// input. The exact factors are the oracle's ascending rational-root order.
#[test]
fn factor_quartic_with_rational_roots_four_linear_factors() {
    let p = poly(&sum(vec![
        x_pow(4),
        product(vec![Expr::num(-5), x_pow(2)]),
        Expr::num(4),
    ]));
    let factored = factor(&p);
    assert_eq!(factored.content, rat(1, 1));
    assert_eq!(factored.factors.len(), 4);
    for (factor, multiplicity) in &factored.factors {
        assert_eq!(*multiplicity, 1);
        assert_eq!(factor.len(), 2, "a linear factor, got {factor:?}");
    }
    assert_eq!(
        factored.factors,
        vec![
            (vec![rat(2, 1), rat(1, 1)], 1),
            (vec![rat(1, 1), rat(1, 1)], 1),
            (vec![rat(-1, 1), rat(1, 1)], 1),
            (vec![rat(-2, 1), rat(1, 1)], 1),
        ]
    );
    let mut reconstructed = vec![factored.content.clone()];
    for (factor, multiplicity) in &factored.factors {
        reconstructed = multiply(&reconstructed, &power(factor, *multiplicity));
    }
    assert_eq!(reconstructed, p, "content * factors reconstructs the input");
}

// ---------------------------------------------------------------------------
// The edge cases: zero, constants, non-monic inputs, negative leading coefficient.
// ---------------------------------------------------------------------------

/// The zero polynomial has content one and stays zero; a constant splits into its absolute value
/// and the signed unit; a rational coefficient set takes the gcd of the numerators over the lcm of
/// the denominators (1/3 + x/2 has content 1/6 and primitive part 2 + 3*x).
#[test]
fn content_and_primitive_part_of_zero_and_constants() {
    assert_eq!(content(&[rat(0, 1)]), rat(1, 1));
    assert_eq!(primitive_part(&[rat(0, 1)]), vec![rat(0, 1)]);

    assert_eq!(content(&[rat(6, 1)]), rat(6, 1));
    assert_eq!(primitive_part(&[rat(6, 1)]), vec![rat(1, 1)]);

    assert_eq!(content(&[rat(-6, 1)]), rat(6, 1));
    assert_eq!(primitive_part(&[rat(-6, 1)]), vec![rat(-1, 1)]);

    assert_eq!(content(&[rat(1, 3), rat(1, 2)]), rat(1, 6));
    assert_eq!(
        primitive_part(&[rat(1, 3), rat(1, 2)]),
        vec![rat(2, 1), rat(3, 1)]
    );

    // A non-monic linear polynomial: 2*x + 2 splits into content 2 and the primitive x + 1.
    assert_eq!(content(&[rat(2, 1), rat(2, 1)]), rat(2, 1));
    assert_eq!(
        primitive_part(&[rat(2, 1), rat(2, 1)]),
        vec![rat(1, 1), rat(1, 1)]
    );
}

/// A negative leading coefficient is part of the content, not the primitive part: -2*x^2 + 4*x + 6
/// has content 2 and primitive part -x^2 + 2*x + 3.
#[test]
fn primitive_part_keeps_the_sign_of_a_negative_leading_coefficient() {
    let p = poly(&sum(vec![
        product(vec![Expr::num(-2), x_pow(2)]),
        product(vec![Expr::num(4), x()]),
        Expr::num(6),
    ]));
    assert_eq!(content(&p), rat(2, 1));
    assert_eq!(primitive_part(&p), vec![rat(3, 1), rat(2, 1), rat(-1, 1)]);
}

/// The gcd/lcm edges: zero is absorbed (gcd(0, p) is the monic p, either zero gives the zero lcm),
/// a constant operand has monic gcd 1, non-monic inputs normalize to a monic gcd, and the lcm of
/// constants is a*b/g, not the integer least common multiple.
#[test]
fn gcd_and_lcm_on_zero_constants_and_non_monic_inputs() {
    let a = poly(&sum(vec![x_pow(2), Expr::num(-1)]));
    let zero = vec![rat(0, 1)];
    assert_eq!(gcd(&zero, &a), vec![rat(-1, 1), rat(0, 1), rat(1, 1)]);
    assert_eq!(gcd(&a, &zero), vec![rat(-1, 1), rat(0, 1), rat(1, 1)]);
    assert_eq!(gcd(&zero, &zero), zero);
    assert_eq!(lcm(&zero, &a), vec![rat(0, 1)]);
    assert_eq!(lcm(&a, &zero), vec![rat(0, 1)]);

    let linear = poly(&sum(vec![product(vec![Expr::num(2), x()]), Expr::num(2)]));
    let wider = poly(&sum(vec![product(vec![Expr::num(4), x()]), Expr::num(4)]));
    assert_eq!(gcd(&linear, &wider), vec![rat(1, 1), rat(1, 1)]);
    assert_eq!(lcm(&linear, &wider), vec![rat(8, 1), rat(8, 1)]);

    assert_eq!(gcd(&[rat(6, 1)], &[rat(4, 1)]), vec![rat(1, 1)]);
    assert_eq!(lcm(&[rat(6, 1)], &[rat(4, 1)]), vec![rat(24, 1)]);

    let negated = poly(&sum(vec![
        product(vec![Expr::num(-1), x_pow(2)]),
        Expr::num(1),
    ]));
    let plus = poly(&sum(vec![x(), Expr::num(1)]));
    assert_eq!(gcd(&negated, &plus), vec![rat(1, 1), rat(1, 1)]);
}

/// The resultant edges: a zero operand answers zero, two constants answer one, and a constant
/// against a degree-n polynomial answers c^n.
#[test]
fn resultant_of_zero_and_constant_operands() {
    let zero = vec![rat(0, 1)];
    let linear = poly(&sum(vec![x(), Expr::num(-1)]));
    assert_eq!(resultant(&zero, &linear), rat(0, 1));
    assert_eq!(resultant(&linear, &zero), rat(0, 1));
    assert_eq!(resultant(&zero, &zero), rat(0, 1));
    assert_eq!(resultant(&[rat(3, 1)], &[rat(4, 1)]), rat(1, 1));
    assert_eq!(resultant(&[rat(3, 1)], &linear), rat(3, 1));
    assert_eq!(resultant(&linear, &[rat(3, 1)]), rat(3, 1));
    let quadratic = poly(&sum(vec![x_pow(2), Expr::num(-2)]));
    assert_eq!(resultant(&quadratic, &[rat(3, 1)]), rat(9, 1));
}

/// A non-monic or negative leading coefficient scales/signs the resultant: res(2*x^2 - 4, x - 1)
/// is -2 and res(-x^2 + 2, x - 1) is 1.
#[test]
fn resultant_tracks_the_leading_coefficient_sign_and_scale() {
    let non_monic = poly(&sum(vec![
        product(vec![Expr::num(2), x_pow(2)]),
        Expr::num(-4),
    ]));
    let negated = poly(&sum(vec![
        product(vec![Expr::num(-1), x_pow(2)]),
        Expr::num(2),
    ]));
    let linear = poly(&sum(vec![x(), Expr::num(-1)]));
    assert_eq!(resultant(&non_monic, &linear), rat(-2, 1));
    assert_eq!(resultant(&negated, &linear), rat(1, 1));
}

/// The discriminant edges: zero answers zero, every constant answers one (the sign is irrelevant
/// for degree 0), and a linear polynomial answers one.
#[test]
fn discriminant_of_zero_constant_and_linear_polynomials() {
    assert_eq!(discriminant(&[rat(0, 1)]), rat(0, 1));
    assert_eq!(discriminant(&[rat(6, 1)]), rat(1, 1));
    assert_eq!(discriminant(&[rat(-6, 1)]), rat(1, 1));
    let linear = poly(&sum(vec![x(), Expr::num(-1)]));
    assert_eq!(discriminant(&linear), rat(1, 1));
    let scaled_linear = poly(&sum(vec![product(vec![Expr::num(3), x()]), Expr::num(-3)]));
    assert_eq!(discriminant(&scaled_linear), rat(1, 1));
}

/// The discriminant scales by c^(2n-2) and is sign-blind for a quadratic: 2*x^2 + 6*x + 4 and
/// -2*x^2 + 6*x - 4 both have discriminant 4; x^3 - x has 4 and 2*x^3 - 2*x has 64.
#[test]
fn discriminant_tracks_the_leading_coefficient_sign_and_scale() {
    let non_monic = poly(&sum(vec![
        product(vec![Expr::num(2), x_pow(2)]),
        product(vec![Expr::num(6), x()]),
        Expr::num(4),
    ]));
    let negated = poly(&sum(vec![
        product(vec![Expr::num(-2), x_pow(2)]),
        product(vec![Expr::num(6), x()]),
        Expr::num(-4),
    ]));
    assert_eq!(discriminant(&non_monic), rat(4, 1));
    assert_eq!(discriminant(&negated), rat(4, 1));

    let cubic = poly(&sum(vec![x_pow(3), product(vec![Expr::num(-1), x()])]));
    let scaled_cubic = poly(&sum(vec![
        product(vec![Expr::num(2), x_pow(3)]),
        product(vec![Expr::num(-2), x()]),
    ]));
    assert_eq!(discriminant(&cubic), rat(4, 1));
    assert_eq!(discriminant(&scaled_cubic), rat(64, 1));
}

/// The factor edges: the zero polynomial answers content zero and no factors; a nonzero constant
/// answers its absolute value as content and its signed unit as the one factor.
#[test]
fn factor_of_zero_and_constants() {
    let zero = factor(&[rat(0, 1)]);
    assert_eq!(zero.content, rat(0, 1));
    assert!(zero.factors.is_empty());

    let positive = factor(&[rat(6, 1)]);
    assert_eq!(positive.content, rat(6, 1));
    assert_eq!(positive.factors, vec![(vec![rat(1, 1)], 1)]);

    let negative = factor(&[rat(-6, 1)]);
    assert_eq!(negative.content, rat(6, 1));
    assert_eq!(negative.factors, vec![(vec![rat(-1, 1)], 1)]);
}

/// The primitive part's leading coefficient moves into the content when factors are extracted:
/// -x^2 + 1 has content -1 and the two factors x + 1 and x - 1; 2*x^2 + x has content 2 and the
/// monic factors x + 1/2 and x; 4*x^2 - 4 has content 4.
#[test]
fn factor_keeps_the_leading_coefficient_in_the_content() {
    let negated = factor(&[rat(1, 1), rat(0, 1), rat(-1, 1)]);
    assert_eq!(negated.content, rat(-1, 1));
    assert_eq!(
        negated.factors,
        vec![
            (vec![rat(1, 1), rat(1, 1)], 1),
            (vec![rat(-1, 1), rat(1, 1)], 1),
        ]
    );

    let non_monic = factor(&[rat(0, 1), rat(1, 1), rat(2, 1)]);
    assert_eq!(non_monic.content, rat(2, 1));
    assert_eq!(
        non_monic.factors,
        vec![
            (vec![rat(1, 2), rat(1, 1)], 1),
            (vec![rat(0, 1), rat(1, 1)], 1),
        ]
    );

    let scaled = factor(&[rat(-4, 1), rat(0, 1), rat(4, 1)]);
    assert_eq!(scaled.content, rat(4, 1));
    assert_eq!(
        scaled.factors,
        vec![
            (vec![rat(1, 1), rat(1, 1)], 1),
            (vec![rat(-1, 1), rat(1, 1)], 1),
        ]
    );

    // A repeated root keeps its multiplicity and the negative content: -2*(x - 1)^2.
    let repeated = factor(&[rat(-2, 1), rat(4, 1), rat(-2, 1)]);
    assert_eq!(repeated.content, rat(-2, 1));
    assert_eq!(repeated.factors, vec![(vec![rat(-1, 1), rat(1, 1)], 2)]);

    // The positive repeated case: 2*(x - 1)^2 has content 2 and the same single factor.
    let positive_repeated = factor(&[rat(2, 1), rat(-4, 1), rat(2, 1)]);
    assert_eq!(positive_repeated.content, rat(2, 1));
    assert_eq!(
        positive_repeated.factors,
        vec![(vec![rat(-1, 1), rat(1, 1)], 2)]
    );

    // A non-monic quartic: 2*(x^4 - 5*x^2 + 4) has content 2 and the same four linear factors.
    let non_monic_quartic = factor(&[rat(8, 1), rat(0, 1), rat(-10, 1), rat(0, 1), rat(2, 1)]);
    assert_eq!(non_monic_quartic.content, rat(2, 1));
    assert_eq!(
        non_monic_quartic.factors,
        vec![
            (vec![rat(2, 1), rat(1, 1)], 1),
            (vec![rat(1, 1), rat(1, 1)], 1),
            (vec![rat(-1, 1), rat(1, 1)], 1),
            (vec![rat(-2, 1), rat(1, 1)], 1),
        ]
    );
}

/// The square-free edges that the factor row depends on: the zero polynomial decomposes into
/// nothing, a nonzero constant into the unit once.
#[test]
fn square_free_of_a_constant_is_the_unit() {
    assert!(square_free_univariate(&[rat(0, 1)]).is_empty());
    assert_eq!(
        square_free_univariate(&[rat(6, 1)]),
        vec![(vec![rat(1, 1)], 1)]
    );
    assert_eq!(
        square_free_univariate(&[rat(-6, 1)]),
        vec![(vec![rat(1, 1)], 1)]
    );
}

/// Square-free factors are monic, so a non-monic or negative leading coefficient is stripped into
/// the (implicit) content instead of being carried by the factor.
#[test]
fn square_free_normalises_non_monic_and_negative_inputs() {
    assert_eq!(
        square_free_univariate(&[rat(6, 1), rat(4, 1), rat(2, 1)]),
        vec![(vec![rat(3, 1), rat(2, 1), rat(1, 1)], 1)]
    );
    assert_eq!(
        square_free_univariate(&[rat(1, 1), rat(0, 1), rat(-1, 1)]),
        vec![(vec![rat(-1, 1), rat(0, 1), rat(1, 1)], 1)]
    );
    assert_eq!(
        square_free_univariate(&[rat(0, 1), rat(1, 1), rat(2, 1)]),
        vec![(vec![rat(0, 1), rat(1, 2), rat(1, 1)], 1)]
    );
    assert_eq!(
        square_free_univariate(&[rat(2, 1), rat(-4, 1), rat(2, 1)]),
        vec![(vec![rat(-1, 1), rat(1, 1)], 2)]
    );
}
