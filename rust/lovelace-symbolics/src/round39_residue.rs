//! Round 39: the symbolics residue the round-38 measurement named - the immediate kernel-surface
//! rows this crate owns.
//!
//! Each test states the C# assertion (Lovelace.Symbolics.Tests/KernelHardeningTests.cs) against the
//! same operation on the same values, and pins the oracle's own display/canonical text where the
//! language can reach it. The census ids are listed in the round-39 report.

use crate::real_literal::RealLiteral;
use crate::{
    Expr, Rat, RelOp, RootOfError, SolveOutcome, diff, evalf_rootof, expand, expr_from_rat,
    nth_real_root, poly_coeffs, poly_to_expr, solve_polynomial, square_free_univariate, substitute,
};
use lovelace_int::Int;
use lovelace_nat::Nat;

fn rat(n: i64, d: u64) -> Rat {
    Rat::new(Int::from_i64(n), Nat::from_u64(d))
}

fn poly(coeffs: &[Rat], var: &str) -> Expr {
    poly_to_expr(coeffs, var)
}

/// Every solution the exact solver publishes, as expressions.
fn solution_expressions(coeffs: &[Rat]) -> Vec<Expr> {
    match solve_polynomial(coeffs) {
        SolveOutcome::Roots(roots) => roots.iter().map(|r| expr_from_rat(&r.value)).collect(),
        SolveOutcome::RootExpressions(exprs) => exprs,
        other => panic!("expected exact solutions, got {other:?}"),
    }
}

/// KernelHardeningTests.cs:350 `CubicRoots_SatisfyPolynomial_Symbolically` (#0 x^3 - 1, #1 x^3 - x):
/// substituting each solved root into the cubic and EXPANDING is the exact rational ZERO. The
/// Cardano roots share the cube-root base 1/2*(i*sqrt(3) - 1), so the expansion has to fold
/// (i*sqrt(3))^2 to -3 - which needs the product the Add coefficient scaling builds to stay FLAT.
#[test]
fn cubic_roots_satisfy_polynomial_symbolically() {
    for coeffs in [
        vec![rat(-1, 1), rat(0, 1), rat(0, 1), rat(1, 1)],
        vec![rat(0, 1), rat(-1, 1), rat(0, 1), rat(1, 1)],
    ] {
        let cubic = poly(&coeffs, "x");
        let solutions = solution_expressions(&coeffs);
        assert_eq!(
            solutions.len(),
            3,
            "a cubic has three exact roots: {solutions:?}"
        );
        for root in &solutions {
            let at = substitute(&cubic, "x", root);
            let expanded = expand(&at);
            assert_eq!(
                expanded,
                Expr::num(0),
                "root {} leaves a residual",
                root.to_string()
            );
        }
    }
}

/// KernelHardeningTests.cs:565 `Diff_Abs_CarriesNonzeroCondition`: the derivative of abs is a
/// PIECEWISE whose single branch is guarded by a `!=` relation, and the fallen-through branch is
/// the unevaluated derivative. The display and canonical text are the oracle's own for
/// `diff(abs(x), x)`.
#[test]
fn diff_abs_carries_nonzero_condition() {
    let x = Expr::sym("x");
    let d = diff(&Expr::Fun("abs".to_string(), vec![x.clone()]), "x");
    let (branches, otherwise) = d
        .as_piecewise()
        .expect("the derivative of abs is a piecewise");
    assert_eq!(branches.len(), 1);
    match &branches[0].0 {
        Expr::Rel(RelOp::Ne, _, zero) => assert_eq!(**zero, Expr::num(0)),
        other => panic!("the guard is a != relation, got {}", other.to_string()),
    }
    assert_eq!(branches[0].1.to_string(), "sign(x)");
    assert!(
        otherwise.as_derivative().is_some(),
        "the branch is an unevaluated derivative"
    );
    assert_eq!(
        d.to_string(),
        "piecewise(sign(x) if x != 0, diff(abs(x), x))"
    );
    assert_eq!(
        d.canonical(),
        "(pw ((ne (sym x) (rat 0 1)) (fn sign (sym x))) (der (x) (fn abs (sym x))))"
    );
}

/// KernelHardeningTests.cs:553 `Diff_Floor_IsUnevaluatedDerivative_NotZero`: d floor(x)/dx must
/// never be stated as the smooth 0. Dividing by one (the chain rule's strip) keeps the node.
#[test]
fn diff_floor_is_unevaluated_derivative_not_zero() {
    let x = Expr::sym("x");
    let floor = Expr::Fun("floor".to_string(), vec![x.clone()]);
    let d = diff(&floor, "x");
    assert_ne!(d, Expr::num(0));
    assert!(d.as_derivative().is_some(), "got {}", d.to_string());
    let stripped = Expr::div(d.clone(), Expr::num(1));
    assert_eq!(stripped, d, "dividing by one keeps the derivative node");
    assert_eq!(d.to_string(), "diff(floor(x), x)");
    assert_eq!(d.canonical(), "(der (x) (fn floor (sym x)))");
    let second = diff(&d, "x");
    assert_eq!(second.canonical(), "(der (x x) (fn floor (sym x)))");
}

/// KernelHardeningTests.cs:576 `Diff_Piecewise_DifferentiatesBranches`: the branches and the
/// otherwise value are differentiated, the guards are carried unchanged.
#[test]
fn diff_piecewise_differentiates_branches() {
    let x = Expr::sym("x");
    let guard = Expr::relation(RelOp::Gt, x.clone(), Expr::num(0));
    let pw = Expr::piecewise(
        vec![(guard.clone(), Expr::pow(x.clone(), Expr::num(2)))],
        Expr::neg(Expr::pow(x.clone(), Expr::num(2))),
    );
    let d = diff(&pw, "x");
    let (branches, otherwise) = d
        .as_piecewise()
        .expect("a piecewise differentiates to a piecewise");
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].0, guard, "the guard is carried unchanged");
    assert_eq!(branches[0].1, Expr::mul(Expr::num(2), x.clone()));
    assert_eq!(otherwise, Expr::mul(Expr::num(-2), x));
}

/// KernelHardeningTests.cs:590 `FreeOf_DoesNotTreatPiecewiseAsConstant`: a symbol that occurs in
/// a branch (or in a guard) is not free, and an expression that only mentions another symbol is.
#[test]
fn free_of_does_not_treat_piecewise_as_constant() {
    let x = Expr::sym("x");
    let y = Expr::sym("y");
    let pw = Expr::piecewise(
        vec![(
            Expr::relation(RelOp::Gt, x.clone(), Expr::num(0)),
            x.clone(),
        )],
        y.clone(),
    );
    assert!(!pw.free_of("x"), "x occurs in a branch value");
    let guarded = Expr::piecewise(
        vec![(
            Expr::relation(RelOp::Gt, x.clone(), Expr::num(0)),
            y.clone(),
        )],
        y.clone(),
    );
    assert!(!guarded.free_of("x"), "x occurs in a guard");
    assert!(Expr::pow(y.clone(), Expr::num(2)).free_of("x"));
    let der = Expr::derivative(Expr::pow(y.clone(), Expr::num(2)), vec!["x".to_string()]);
    assert!(der.free_of("x"), "a derivative binds its variable");
}

/// KernelHardeningTests.cs:502 `HugeExponent_CompactForm_RoundTrips`: the compact carrier prints
/// `15e1000000000` and parses back to the same value; the digits are never materialized.
#[test]
fn huge_exponent_compact_form_round_trips() {
    let literal = RealLiteral::new(Int::from_i64(15), 1_000_000_000);
    assert_eq!(literal.to_string(), "15e1000000000");
    assert_eq!(
        RealLiteral::parse(&literal.to_string()),
        Some(literal.clone())
    );
    assert_eq!(
        literal.to_rational(),
        None,
        "a billion-digit power of ten is refused"
    );
    let small = RealLiteral::new(Int::from_i64(15), 3);
    assert_eq!(small.to_rational(), Some(rat(15_000, 1)));
    assert_eq!(
        RealLiteral::parse("15e-3").and_then(|l| l.to_rational()),
        Some(rat(15, 1000))
    );
    assert_eq!(
        RealLiteral::parse("7"),
        Some(RealLiteral::new(Int::from_i64(7), 0))
    );
    assert_eq!(
        RealLiteral::parse("1.5e3"),
        None,
        "a decimal significand is not compact"
    );
}

/// KernelHardeningTests.cs:257 `YunSquareFree_MultiplicitiesAreCorrect`: x^2*(x - 1) decomposes
/// into (x - 1) once and x twice, and the factors multiply back to the input exactly.
#[test]
fn yun_square_free_multiplicities_are_correct() {
    let p = vec![rat(0, 1), rat(0, 1), rat(-1, 1), rat(1, 1)];
    let factors = square_free_univariate(&p);
    assert_eq!(factors.len(), 2, "got {factors:?}");
    assert_eq!(factors[0], (vec![rat(-1, 1), rat(1, 1)], 1), "(x - 1) once");
    assert_eq!(factors[1], (vec![rat(0, 1), rat(1, 1)], 2), "x twice");
    let square_free = vec![rat(-1, 1), rat(0, 1), rat(1, 1)];
    assert_eq!(
        square_free_univariate(&square_free),
        vec![(square_free.clone(), 1)]
    );
    assert_eq!(
        square_free_univariate(&[rat(1, 1), rat(-2, 1), rat(1, 1)]),
        vec![(vec![rat(-1, 1), rat(1, 1)], 2)]
    );
}

/// KernelHardeningTests.cs:286 `RootOf_CloseRoots_AllFoundAndIndexedStably`: the three real roots
/// of x*(x - 1/1000)*(x - 1) are counted and each index isolates the right one, ascending.
#[test]
fn rootof_close_roots_all_found_and_indexed_stably() {
    let coeffs = vec![rat(0, 1), rat(1, 1000), rat(-1001, 1000), rat(1, 1)];
    assert_eq!(crate::real_root_count(&coeffs), 3);
    let tolerance = rat(1, 10_000_000_000_000_000_000);
    for (index, expected) in [(0usize, rat(0, 1)), (1, rat(1, 1000)), (2, rat(1, 1))] {
        let value = nth_real_root(&coeffs, index).expect("an indexed real root");
        let difference = value.add(&expected.neg()).abs();
        assert!(
            difference.cmp(&tolerance) == std::cmp::Ordering::Less,
            "index {index} answered {} instead of {}",
            value.to_string(),
            expected.to_string()
        );
    }
    let defining = poly(&coeffs, "x");
    for index in 0..3i64 {
        let node = Expr::Fun(
            "rootof".to_string(),
            vec![defining.clone(), Expr::num(index)],
        );
        assert_eq!(
            node.to_string(),
            format!("rootof({}, {index})", defining.to_string())
        );
        let value = evalf_rootof(&node, 30).expect("the numeric value of the RootOf node");
        let expected = nth_real_root(&coeffs, index as usize).expect("the same root");
        // the node evaluation is asked for 30 digits and the bare call for 40: both approximate
        // the same root to far better than the row tolerance.
        let difference = value.add(&expected.neg()).abs();
        assert!(
            difference.cmp(&tolerance) == std::cmp::Ordering::Less,
            "the node answered {} and the bare call {}",
            value.to_string(),
            expected.to_string()
        );
    }
    assert_eq!(
        nth_real_root(&coeffs, 3),
        Err(RootOfError::IndexOutOfRange { index: 3, count: 3 })
    );
}

/// KernelHardeningTests.cs:323 `RootOf_NoRealRoots_FailsExplicitly`: x^2 + 1 has no real root, so
/// evaluating its RootOf fails explicitly instead of answering a complex number.
#[test]
fn rootof_no_real_roots_fails_explicitly() {
    let coeffs = vec![rat(1, 1), rat(0, 1), rat(1, 1)];
    assert_eq!(nth_real_root(&coeffs, 0), Err(RootOfError::NoRealRoots));
    let node = Expr::Fun("rootof".to_string(), vec![poly(&coeffs, "x"), Expr::num(0)]);
    assert_eq!(evalf_rootof(&node, 20), Err(RootOfError::NoRealRoots));
    assert!(RootOfError::NoRealRoots.message().contains("no real root"));
}

/// The numeric surface agrees with the exact radicals the solver publishes: the roots of
/// x^2 - 5 are +-1/2*sqrt(20) = +-sqrt(5).
#[test]
fn rootof_evaluates_the_quadratic_radicals() {
    let coeffs = vec![rat(-5, 1), rat(0, 1), rat(1, 1)];
    let plus = nth_real_root(&coeffs, 1).expect("the positive root");
    let minus = nth_real_root(&coeffs, 0).expect("the negative root");
    let expect = rat(2236067977499789696, 1_000_000_000_000_000_000);
    let delta = plus.add(&expect.neg()).abs();
    assert!(
        delta.cmp(&rat(1, 1_000_000_000_000_000_000)) == std::cmp::Ordering::Less,
        "sqrt(5) is 2.2360679774997896..., got {}",
        plus.to_string()
    );
    assert_eq!(minus, plus.neg());
    let defining = poly(&coeffs, "x");
    assert_eq!(poly_coeffs(&defining, "x"), Some(coeffs));
}
