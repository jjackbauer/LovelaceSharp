//! The seven C# PropertyTests contracts, exercised against the exact generated corpora.
//!
//! Each test mirrors one row of Lovelace.Symbolics.Tests/PropertyTests.cs:
//! the same seed and corpus from property_corpus, the same assertion, translated onto the
//! Rust library surface that publishes the same value:
//!
//! * canonical text round-trip uses Context::canonical_parse (Printing.CanonicalParse).
//!   The reference asserts Assert.Same because its context interns every expression; the Rust
//!   Expr is a value, so the strongest available equivalent is structural equality plus
//!   canonical-text idempotence. Both are asserted.
//! * simplify/diff use Context::simplify / Context::diff.
//! * integrate and optimize have no public library function; they are reached through the
//!   engine builtins that expose the same kernels (integrate(...) and optimize(...)).
//! * solve uses lovelace_symbolics::solve_polynomial (the polynomial arm of Solvers.Solve).
//! * MathIR uses Context::lower and IrEvaluator, the Rust Lowering.Lower / IrEvaluator pair.
//!
//! Numeric evaluation is lovelace_lang::numeric::evaluate at 50 fractional digits, the
//! reference's Rl.WithPrecision(50, 20) working precision.

use std::collections::HashMap;

use lovelace_lang::Engine;
use lovelace_lang::Value;
use lovelace_lang::context::Context;
use lovelace_lang::mathir::IrEvaluator;
use lovelace_lang::numeric::{self, Num};
use lovelace_symbolics::assumptions::{Assumption, AssumptionSet, Domain};
use lovelace_symbolics::{Expr, Rat, SolveOutcome, expand, solve_polynomial, substitute};

use crate::property_corpus::{
    canonical_text_roundtrip as canonical_corpus, diff_matches_numerical_derivative as diff_corpus,
    integrate_diff_roundtrips as integrate_corpus, mathir_evaluates_like_symbolic as mathir_corpus,
    optimize_preserves_values as optimize_corpus, simplify_preserves_values as simplify_corpus,
    solve_quadratic_roots_satisfy_polynomial as solve_corpus,
};

const PLACES: usize = 50;

fn rat(numerator: i64, denominator: i64) -> Rat {
    Rat::from_i64(numerator)
        .div(&Rat::from_i64(denominator))
        .expect("nonzero denominator")
}

fn expr_at(value: &Num) -> Expr {
    match value {
        Num::Exact(value) => Expr::Num(value.clone()),
        Num::Real(value) => Expr::Real(value.to_string()),
        Num::Complex(_) => panic!("the generated corpora evaluate on the real line"),
    }
}

/// Evaluate a one-variable expression at a Num point, mirroring Evaluation.EvaluateToNum.
fn evaluate_at_num(expression: &Expr, value: &Num) -> Num {
    let substituted = substitute(expression, "x", &expr_at(value));
    numeric::evaluate(&substituted, PLACES).unwrap_or_else(|error| {
        panic!(
            "numeric evaluation failed for {}: {error}",
            substituted.canonical()
        )
    })
}

fn evaluate_at(expression: &Expr, point: &Rat) -> Num {
    evaluate_at_num(expression, &Num::Exact(point.clone()))
}

fn num_equal(left: &Num, right: &Num) -> bool {
    match (left, right) {
        (Num::Exact(a), Num::Exact(b)) => a == b,
        _ => left.to_real(PLACES) == right.to_real(PLACES),
    }
}

/// CanonicalText_RoundTripsAndReinterns.
///
/// BLOCKED twice. The reference asserts Assert.Same because ExprContext interns expressions;
/// Rust Context interns Symbol and Function ids only (context.rs), and Expr is a value enum, so
/// reference identity has no Rust spelling. The nearest executable contract, canonical-text
/// idempotence, also fails on 1 of the 200 generated expressions (item 49): the two mutually
/// commutative sum factors of a product are printed in one order by the constructed expression
/// and the other order after canonical_parse + canonical. Both are production gaps in the
/// symbolic printer/parser, outside rust/lovelace-corpus/src.
#[test]
#[ignore = "blocked: no Expr interning (Assert.Same) and canonical text is not idempotent for 1/200 items"]
fn canonical_text_round_trips_and_reinterns() {
    let context = Context::new();
    let corpus = canonical_corpus(20_260_909);
    for expression in &corpus.expressions {
        let text = expression.canonical();
        let back = context
            .canonical_parse(&text)
            .unwrap_or_else(|error| panic!("canonical text did not parse: {text}: {error}"));
        // The C# row's Assert.Same is an interning identity; Rust Expr is a value and has no
        // expression interning table, so the executable contract is the canonical-text
        // idempotence the identity exists to guarantee.
        assert_eq!(back.canonical(), text, "canonical text is not idempotent");
    }
}

/// Simplify_PreservesValues_UnderRealAssumption.
#[test]
fn simplify_preserves_values_under_real_assumption() {
    let context = Context::new();
    let assumptions = AssumptionSet::empty()
        .add(Assumption::SymbolDomain("x".to_string(), Domain::Real))
        .expect("x in Real is consistent");
    let _scope = context.with_assumptions(assumptions);
    let points = [rat(-7, 4), rat(-1, 3), rat(1, 5), rat(3, 2), rat(11, 4)];
    for expression in simplify_corpus(20_260_910).expressions {
        let simplified = context.simplify(expression.clone());
        for point in &points {
            let before = evaluate_at(&expression, point);
            let after = evaluate_at(&simplified, point);
            assert!(
                num_equal(&before, &after),
                "simplify changed {} at x={}",
                expression.canonical(),
                point
            );
        }
    }
}

/// Diff_MatchesNumericalDerivative.
///
/// BLOCKED: the generated derivative contains the reference's piecewise/relation
/// representation for abs. lovelace_lang::numeric::evaluate has no Piecewise/Relation arm and
/// refuses with "A relation has no numeric value"; the reference Evaluation.EvaluateToNum
/// evaluates the guard (Evaluation.cs:487-499, 572-617). This needs a production numeric-arm
/// change outside rust/lovelace-corpus/src.
#[test]
#[ignore = "blocked: numeric::evaluate has no Piecewise/Relation arm (lovelace-lang)"]
fn diff_matches_numerical_derivative() {
    let context = Context::new();
    let h_denominator = Rat::from_i64(10_000_000_000).mul(&Rat::from_i64(1_000_000_000));
    let h = Num::Real(
        Num::Exact(
            Rat::from_i64(1)
                .div(&h_denominator)
                .expect("nonzero denominator"),
        )
        .to_real(PLACES)
        .unwrap(),
    );
    let tolerance = Num::Exact(rat(1, 10_000_000_000));
    let two = Num::Exact(Rat::from_i64(2));
    let points = [rat(-3, 2), rat(1, 4), rat(2, 3)];
    for expression in diff_corpus(20_260_911).expressions {
        let derivative = context.diff(&expression, "x");
        for point in &points {
            let value = Num::Exact(point.clone());
            let plus = value.add(&h, PLACES).expect("add");
            let minus = value.sub(&h, PLACES).expect("sub");
            let f_plus = evaluate_at_num(&expression, &plus);
            let f_minus = evaluate_at_num(&expression, &minus);
            let denominator = h.mul(&two, PLACES).expect("mul");
            let numeric = f_plus
                .sub(&f_minus, PLACES)
                .expect("sub")
                .div(&denominator, PLACES)
                .expect("div");
            let symbolic = evaluate_at_num(&derivative, &value);
            let delta = numeric
                .sub(&symbolic, PLACES)
                .expect("sub")
                .to_real(PLACES)
                .expect("real difference")
                .abs();
            let bound = tolerance.to_real(PLACES).expect("real tolerance");
            assert!(
                delta.cmp(&bound) == std::cmp::Ordering::Less,
                "d/dx {} = {} disagrees at x={} (delta {})",
                expression.canonical(),
                derivative.canonical(),
                point,
                delta
            );
        }
    }
}

fn is_unevaluated_integral(expression: &Expr) -> bool {
    matches!(expression, Expr::Fun(name, _) if name == "integrate")
}

/// Integrate_DiffRoundTrips.
///
/// BLOCKED: the reference closes all six integrands; Rust integrates five. The sixth,
/// x^2*sin(x^3), stays an unevaluated integrate(...) (u-substitution is missing from
/// lovelace-lang's integrate kernel), so its derivative round-trip cannot be asserted.
#[test]
#[ignore = "blocked: integrate() leaves x^2*sin(x^3) unevaluated (lovelace-lang builtins)"]
fn integrate_diff_round_trips() {
    let context = Context::new();
    let mut engine = Engine::new();
    let mut closed_forms = 0usize;
    for (index, integrand) in integrate_corpus(20_260_912)
        .expressions
        .into_iter()
        .enumerate()
    {
        let script = format!("x = symbol(\"x\"); integrate({integrand}, x)");
        let value = engine.eval(&script).expect("integrate script");
        let Value::Symbolic(integral) = value else {
            panic!("integrate did not return a symbolic value: {value:?}");
        };
        if is_unevaluated_integral(&integral) {
            println!("unevaluated integrand {index}: {}", integrand.canonical());
            continue;
        }
        closed_forms += 1;
        let check = expand(&context.diff(&integral, "x"));
        let expected = expand(&integrand);
        assert_eq!(
            check.canonical(),
            expected.canonical(),
            "d/dx of the integral does not return the integrand"
        );
    }
    // The C# test's six integrands all produce a closed form (probed in-process against the
    // reference library: SymbolExpr/AddExpr/MultiplyExpr/FunctionExpr/FunctionExpr/MultiplyExpr).
    assert_eq!(
        closed_forms, 6,
        "a C# closed form stayed unevaluated in Rust"
    );
}

/// Solve_QuadraticRoots_SatisfyPolynomial.
#[test]
fn solve_quadratic_roots_satisfy_polynomial() {
    let corpus = solve_corpus(20_260_913);
    for item in &corpus.items {
        let mut parts = item
            .split(',')
            .map(|part| part.parse::<i64>().expect("integer"));
        let a = parts.next().expect("a");
        let b = parts.next().expect("b");
        let c = parts.next().expect("c");
        let outcome = solve_polynomial(&[Rat::from_i64(c), Rat::from_i64(b), Rat::from_i64(a)]);
        let roots: Vec<Expr> = match outcome {
            SolveOutcome::Roots(roots) => roots
                .into_iter()
                .map(|root| Expr::Num(root.value))
                .collect(),
            SolveOutcome::RootExpressions(roots) => roots,
            SolveOutcome::RootExpressionsCount(roots, _) => roots,
            other => panic!("quadratic {item} was not solved exactly: {other:?}"),
        };
        assert_eq!(roots.len(), 2, "quadratic {item} must have two exact roots");
        let x = Expr::sym("x");
        let polynomial = Expr::add(
            Expr::add(
                Expr::mul(Expr::num(a), Expr::pow(x.clone(), Expr::num(2))),
                Expr::mul(Expr::num(b), x),
            ),
            Expr::num(c),
        );
        for root in roots {
            let at = expand(&substitute(&polynomial, "x", &root));
            let Expr::Num(value) = at else {
                panic!(
                    "root {} did not substitute to a constant: {at:?}",
                    root.canonical()
                );
            };
            assert!(
                value.is_zero(),
                "root {} is not a root of {item}",
                root.canonical()
            );
        }
    }
}

/// Optimize_PreservesValues.
#[test]
fn optimize_preserves_values() {
    let mut engine = Engine::new();
    let points = [rat(-2, 3), rat(1, 2), rat(5, 4)];
    for expression in optimize_corpus(20_260_914).expressions {
        let script = format!("x = symbol(\"x\"); optimize({expression}, [x])");
        let value = engine.eval(&script).expect("optimize script");
        let Value::Symbolic(optimized) = value else {
            panic!("optimize did not return a symbolic value: {value:?}");
        };
        for point in &points {
            let before = evaluate_at(&expression, point);
            let after = evaluate_at(&optimized, point);
            assert!(
                num_equal(&before, &after),
                "optimize changed {} at x={}",
                expression.canonical(),
                point
            );
        }
    }
}

/// MathIR_EvaluatesLikeSymbolic.
///
/// BLOCKED: at x=1/3 on the second generated expression, numeric::evaluate and IrEvaluator
/// differ by one unit in the 50th decimal place (…898 vs …897); the reference asserts exact
/// NumOps.Compare equality. This is a production numeric/IR parity gap outside this crate.
#[test]
#[ignore = "blocked: numeric::evaluate and IrEvaluator differ by 1 ulp at 50 places (lovelace-lang)"]
fn mathir_evaluates_like_symbolic() {
    let context = Context::new();
    let points = [rat(-3, 4), rat(1, 3), rat(7, 5)];
    for expression in mathir_corpus(20_260_915).expressions {
        let program = context
            .lower(&expression, &["x".to_string()])
            .expect("lowering");
        for point in &points {
            let direct = evaluate_at(&expression, point);
            let mut bindings = HashMap::new();
            bindings.insert("x".to_string(), Num::Exact(point.clone()));
            let lowered =
                IrEvaluator::evaluate_at(&program, &bindings, PLACES).expect("IR evaluation");
            assert!(
                num_equal(&direct, &lowered),
                "MathIR disagrees for {} at x={}: direct={:?}, lowered={:?}",
                expression.canonical(),
                point,
                direct.to_real(PLACES),
                lowered.to_real(PLACES)
            );
        }
    }
}
