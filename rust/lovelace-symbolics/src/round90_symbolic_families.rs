//! Round 90 — the Symbolics half of the four deferred symbolic families.
//!
//! * `LogicalCanonicalization` (RelationsAndFamiliesTests.cs:25): the reference's
//!   `Exprs.And/Or/Not` constructors flatten, fold the exact 0/1 constants, collapse a double
//!   negation and sort their operands. This module pins those rules on the library surface.
//! * `CachedFlag_EqualsTheLeafRule_OverTheWholeCorpus` (ExactnessFlagTests.cs:97) and
//!   `RadicalsOfExactBases_AreExact_AndApproximateBasesAreNot` (ExactnessFlagTests.cs:219): the
//!   leaf rule, the numeric-exponent normalization and the radical/RootOf boundary.
//!
//! The oracle's observations are the C# assertions in those rows; the helpers below implement
//! the rule a second time so the kernel's cached flag is never checked against itself.

use lovelace_int::Int;
use lovelace_nat::Nat;

use crate::{Expr, RelOp};

fn rational(numerator: i64, denominator: u64) -> Expr {
    Expr::rational(&Int::from_i64(numerator), &Nat::from_u64(denominator))
}

fn imaginary_unit() -> Expr {
    Expr::Fun("i".to_string(), Vec::new())
}

fn quintic() -> Expr {
    let x = Expr::sym("x");
    Expr::sub(
        Expr::sub(Expr::pow(x.clone(), Expr::num(5)), x),
        Expr::num(1),
    )
}

/// The rule under test, implemented a second time and independently of the kernel's cached flag:
/// an expression is exact exactly when every leaf of it is in the exact basis.
fn exact_by_leaf_rule(expression: &Expr) -> bool {
    match expression {
        Expr::Real(_) => false,
        Expr::Num(_) | Expr::Sym(_) => true,
        Expr::Add(items) | Expr::Mul(items) => items.iter().all(exact_by_leaf_rule),
        Expr::Pow(left, right) | Expr::Div(left, right) | Expr::Rel(_, left, right) => {
            exact_by_leaf_rule(left) && exact_by_leaf_rule(right)
        }
        Expr::Fun(name, arguments) => match name.as_str() {
            // The imaginary unit and an algebraic number are exact; the transcendental and
            // indeterminate named constants are not.
            "i" => true,
            "rootof" => true,
            "pi" | "e" | "inf" => false,
            _ => arguments.iter().all(exact_by_leaf_rule),
        },
    }
}

// ---------------------------------------------------------------------------
// LogicalCanonicalization
// ---------------------------------------------------------------------------

/// RelationsAndFamiliesTests.cs:25 `LogicalCanonicalization`: every assertion in the C# row,
/// through the Rust constructors.
#[test]
fn logical_canonicalization() {
    let x = Expr::sym("x");
    let gt = Expr::relation(RelOp::Gt, x.clone(), Expr::num(0));
    let lt = Expr::relation(RelOp::Lt, x.clone(), Expr::num(1));

    // double negation collapses
    assert_eq!(Expr::logical_not(Expr::logical_not(gt.clone())), gt);
    // duplicates collapse, constants fold
    assert_eq!(Expr::and_all(vec![gt.clone(), gt.clone()]), gt);
    assert_eq!(Expr::and_all(vec![gt.clone(), Expr::num(1)]), gt);
    assert_eq!(
        Expr::and_all(vec![gt.clone(), Expr::num(0)]),
        Expr::num(0),
        "false is the conjunction's annihilator"
    );
    assert_eq!(Expr::or_all(vec![gt.clone(), Expr::num(1)]), Expr::num(1));
    assert_eq!(Expr::or_all(vec![Expr::num(0), Expr::num(0)]), Expr::num(0));
    assert_eq!(Expr::or_all(vec![gt.clone(), Expr::num(0)]), gt);
    assert_eq!(Expr::logical_not(Expr::num(0)), Expr::num(1));
    assert_eq!(Expr::logical_not(Expr::num(1)), Expr::num(0));
    // a single collected operand is the operand itself; an empty one is the identity
    assert_eq!(Expr::and_all(Vec::new()), Expr::num(1));
    assert_eq!(Expr::or_all(Vec::new()), Expr::num(0));

    // round-trip through canonical text and the canonical display order
    let combo = Expr::and_all(vec![gt.clone(), lt.clone()]);
    assert_eq!(combo.to_string(), "x < 1 and x > 0");
    let text = combo.canonical();
    assert_eq!(
        crate::canonical_parse(&text).expect("canonical text parses"),
        combo,
        "the canonical text must round-trip: {text}"
    );

    // nested same-kind nodes flatten instead of stacking (the reference's AndExpr operands)
    let y = Expr::sym("y");
    let ygt = Expr::relation(RelOp::Gt, y, Expr::num(0));
    let nested = Expr::and_all(vec![gt, Expr::and_all(vec![lt, ygt])]);
    match &nested {
        Expr::Fun(name, operands) => {
            assert_eq!(name, "and");
            assert_eq!(
                operands.len(),
                3,
                "the nested And must be flattened: {nested}"
            );
            assert!(
                operands
                    .iter()
                    .all(|operand| !matches!(operand, Expr::Fun(n, _) if n == "and"))
            );
        }
        other => panic!("expected a flattened And node, got {other:?}"),
    }
    assert_eq!(nested.node_count(), 10);
}

// ---------------------------------------------------------------------------
// ExactnessFlagTests — the leaf rule over a corpus
// ---------------------------------------------------------------------------

/// ExactnessFlagTests.cs:97 `CachedFlag_EqualsTheLeafRule_OverTheWholeCorpus`: the kernel's
/// cached flag equals the independent leaf rule over the same corpus shapes, in both directions.
#[test]
fn cached_flag_equals_the_leaf_rule_over_the_whole_corpus() {
    let x = Expr::sym("x");
    let y = Expr::sym("y");
    let approx = Expr::real("1.5");
    let corpus = vec![
        Expr::num(1),
        rational(2, 3),
        x.clone(),
        imaginary_unit(),
        Expr::add(rational(1, 2), Expr::mul(rational(-3, 4), imaginary_unit())),
        Expr::Fun("rootof".to_string(), vec![quintic(), Expr::num(0)]),
        Expr::add(x.clone(), Expr::num(1)),
        Expr::mul(Expr::num(2), x.clone()),
        Expr::pow(x.clone(), Expr::num(2)),
        Expr::pow(x.clone(), rational(1, 2)),
        Expr::pow(x.clone(), rational(3, 2)),
        Expr::pow(Expr::num(2), rational(1, 2)),
        Expr::pow(Expr::num(2), rational(1, 3)),
        Expr::mul(imaginary_unit(), Expr::pow(Expr::num(3), rational(1, 2))),
        Expr::Fun("abs".to_string(), vec![x.clone()]),
        Expr::Fun("sin".to_string(), vec![x.clone()]),
        Expr::Fun("cos".to_string(), vec![x.clone()]),
        Expr::Fun("exp".to_string(), vec![x.clone()]),
        Expr::Fun("log".to_string(), vec![x.clone()]),
        Expr::Fun("tan".to_string(), vec![x.clone()]),
        Expr::Fun("atan".to_string(), vec![x.clone()]),
        Expr::Fun("sinh".to_string(), vec![x.clone()]),
        Expr::Fun("min".to_string(), vec![x.clone(), y.clone()]),
        Expr::relation(
            RelOp::Lt,
            x.clone(),
            Expr::Fun("exp".to_string(), vec![y.clone()]),
        ),
        Expr::derivative(
            Expr::Fun("sin".to_string(), vec![x.clone()]),
            vec!["x".to_string()],
        ),
        // and now the same shapes with an approximation in them
        approx.clone(),
        Expr::add(x.clone(), approx.clone()),
        Expr::mul(x.clone(), approx.clone()),
        Expr::pow(approx.clone(), Expr::num(2)),
        Expr::pow(x.clone(), Expr::add(x.clone(), approx.clone())),
        Expr::pow(x.clone(), Expr::sym("pi")),
        Expr::Fun("sin".to_string(), vec![approx.clone()]),
        Expr::Fun("exp".to_string(), vec![approx.clone()]),
        Expr::Fun("min".to_string(), vec![x.clone(), approx.clone()]),
        Expr::relation(RelOp::Lt, x.clone(), approx.clone()),
        Expr::derivative(
            Expr::Fun("sin".to_string(), vec![approx]),
            vec!["x".to_string()],
        ),
    ];

    for expression in &corpus {
        assert_eq!(
            exact_by_leaf_rule(expression),
            expression.is_exact(),
            "the cached flag disagrees with the leaf rule on {}",
            expression.canonical()
        );
        // and the second direction, so a rule that is uniformly true cannot pass
        if !exact_by_leaf_rule(expression) {
            assert!(
                !expression.is_exact(),
                "{} carries an approximation leaf",
                expression.canonical()
            );
        }
    }

    // the approximation cases are present, so the check above is not vacuous
    assert!(
        corpus
            .iter()
            .any(|expression| !exact_by_leaf_rule(expression))
    );
    assert!(corpus.iter().any(exact_by_leaf_rule));
}

// ---------------------------------------------------------------------------
// ExactnessFlagTests — radicals and RootOf
// ---------------------------------------------------------------------------

/// ExactnessFlagTests.cs:219 `RadicalsOfExactBases_AreExact_AndApproximateBasesAreNot`: the
/// radical boundary, the numeric-exponent normalization and the RootOf leaf.
#[test]
fn radicals_of_exact_bases_are_exact_and_approximate_bases_are_not() {
    let x = Expr::sym("x");
    let y = Expr::sym("y");
    let approx = Expr::real("1.5");

    // sqrt(2): the exact closed form, both spellings
    assert!(Expr::sqrt(Expr::num(2)).is_exact());
    assert!(Expr::pow(Expr::num(2), rational(1, 2)).is_exact());
    // sqrt(x), x^(3/2), 2^(1/3), i*sqrt(3)
    assert!(Expr::pow(x.clone(), rational(1, 2)).is_exact());
    assert!(Expr::pow(x.clone(), rational(3, 2)).is_exact());
    assert!(Expr::pow(Expr::num(2), rational(1, 3)).is_exact());
    assert!(Expr::sqrt(Expr::num(-3)).is_exact());

    // a radical of an approximate base is still an approximation
    assert!(!Expr::pow(approx.clone(), rational(1, 2)).is_exact());
    // ... and so is a radical whose SYMBOLIC exponent carries one
    assert!(!Expr::pow(x.clone(), Expr::add(y, Expr::real("0.5"))).is_exact());

    // The boundary the C# row pins: a NUMERIC exponent is canonicalized to a rational constant
    // before the flag is computed, so x^1.5 and x^(3/2) are the SAME node and both are exact.
    assert_eq!(
        Expr::pow(x.clone(), approx.clone()),
        Expr::pow(x.clone(), rational(3, 2))
    );
    assert!(Expr::pow(x.clone(), approx.clone()).is_exact());
    assert_eq!(
        Expr::pow(x.clone(), approx).canonical(),
        "(pow (sym x) (rat 3 2))"
    );

    // RootOf: an algebraic number, exact at construction
    assert!(Expr::Fun("rootof".to_string(), vec![quintic(), Expr::num(0)]).is_exact());
}
