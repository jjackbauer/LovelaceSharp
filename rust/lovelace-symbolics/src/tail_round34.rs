//! Round 34: kernel-level rows from the Symbolics tail that the crate's own public API expresses.
//!
//! Two census rows are kernel arithmetic, not engine scripts: the rational-normalization row
//! (AcceptanceTests) and two polynomial-root rows (KernelHardeningTests). Each test states the C#
//! assertion against the same operation on the same values.

use crate::{Rat, SolveOutcome, real_root_count, solve_polynomial};
use lovelace_int::Int;
use lovelace_nat::Nat;

fn rat(n: i64, d: u64) -> Rat {
    Rat::new(Int::from_i64(n), Nat::from_u64(d))
}

/// AcceptanceTests.cs:249 — rational normalization and arithmetic.
#[test]
fn acceptance_rational_normalization_and_arithmetic() {
    let two_fourths = rat(2, 4);
    assert_eq!(two_fourths, rat(1, 2));
    assert_eq!(two_fourths.to_string(), "1/2");
    assert_eq!(rat(5, 6), rat(1, 2).add(&rat(1, 3)));
    assert_eq!(rat(1, 3), Rat::from_i64(2).div(&Rat::from_i64(6)).unwrap());
}

/// KernelHardeningTests.cs:497 — the square-free part of (x - 1)^2 has exactly one real root.
#[test]
fn kernel_hardening_root_of_multiple_root_is_square_free_normalized() {
    // -(x - 1)^2 = -1 + 2x - x^2, coefficients ascending.
    assert_eq!(real_root_count(&[rat(-1, 1), rat(2, 1), rat(-1, 1)]), 1);
}

/// KernelHardeningTests.cs:475 — the rational roots include 0, 1/1000 and 1.
#[test]
fn kernel_hardening_rational_roots_include_zero() {
    // x*(x - 1/1000)*(x - 1) = x^3 - (1001/1000)x^2 + (1/1000)x
    let outcome = solve_polynomial(&[rat(0, 1), rat(1, 1000), rat(-1001, 1000), rat(1, 1)]);
    let roots = match outcome {
        SolveOutcome::Roots(roots) => roots,
        other => panic!("expected exact rational roots, got {other:?}"),
    };
    let values: Vec<Rat> = roots.iter().map(|root| root.value.clone()).collect();
    assert!(values.contains(&rat(0, 1)), "{values:?}");
    assert!(values.contains(&rat(1, 1000)), "{values:?}");
    assert!(values.contains(&rat(1, 1)), "{values:?}");
}
