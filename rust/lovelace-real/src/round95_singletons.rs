//! Round 95 numeric singletons in lovelace-real: the two deferred Real rows.
//!
//! Each test restates the C# row's observable against the Rust surface.

use lovelace_nat::Nat;

use crate::Real;

/// C# StructuredRealExactnessTests.TerminatingQuotientBeyondTheDigitBudget_IsCertifiedExact#2:
/// `2^-100000` is the exact quotient 1 / 2^100000 at every budget, not a truncation.
#[test]
fn terminating_quotient_beyond_the_digit_budget_is_certified_exact() {
    let denominator = Nat::from_u64(2).pow(100_000);
    let value = Real::from_fraction(&Nat::from_u64(1), &denominator, 1000)
        .expect("a nonzero denominator divides");

    assert!(value.is_exact(), "2^-100000 was not certified exact");
    let (numerator, published_denominator) = value.to_rational();
    assert_eq!("1", numerator.to_decimal());
    assert_eq!(denominator, published_denominator);
    assert!(
        value.to_decimal_fixed(4).starts_with("0.0000"),
        "2^-100000 rendered as {}",
        value.to_decimal_fixed(4)
    );
}

/// C# RealTrigFastPathTests.Cos_AtDefaultPrecision_StaysInteractive: at the default 1000-place
/// precision one cosine must stay under the 10 s budget (the row's own bound; it guards the
/// per-term decimal-rendering regression, not machine speed).
#[test]
fn cos_at_default_precision_stays_interactive() {
    let argument = Real::from_str("0.5").expect("0.5");
    let _ = argument.cos(1000); // warm the code path

    let start = std::time::Instant::now();
    let value = argument.cos(1000);
    let elapsed = start.elapsed();

    assert!(!value.is_periodic());
    assert!(
        elapsed < std::time::Duration::from_secs(10),
        "cos(0.5) at 1000 places took {elapsed:?}"
    );
}
