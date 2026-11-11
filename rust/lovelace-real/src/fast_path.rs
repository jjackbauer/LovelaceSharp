//! The reference `Real` implementation's private fast-path layer, exposed as observables.
//!
//! The reference (`Lovelace.Real/Real.cs`) keeps a small-value fast-path layer beneath its public
//! trigonometric and arithmetic surface:
//!
//! * a 16-row special-angle table (`TrySpecialAngle`, built from `Sqrt2Half`/`Sqrt3Half`),
//! * an integer turn quotient for the [0, 2*pi) argument reduction (`WholeQuotient`,
//!   `ReduceToTwoPi`, `Truncate`),
//! * an exact half for pi-shaped values (`HalfOf`),
//! * a galloping trailing-zero strip (`Normalize`/`StripTrailingDecimalZeros`),
//! * an exponent alignment by decimal shift (`ToInteger`), and
//! * an integer series-termination test that carries its power of ten between terms
//!   (`IsBelowDecimalGuard`).
//!
//! `Lovelace.Real.Tests/RealTrigFastPathTests.cs` pins each of those against the expression it
//! replaces: `Truncate(Divide(value, twoPi))`, `Abs(term) < 10^-(guard+1)`, the public
//! `Real.Divide` half, and the digit-string strip. That is a library-only contract with no
//! wire-visible form, so the Rust workspace had no surface to assert it against and the ten census
//! rows of `RealTrigFastPathTests` stayed UNRUNNABLE.
//!
//! This module is that surface. It is observational only: every accessor either returns a value the
//! kernel itself produced, or re-expresses one reference fast path that the Rust kernel spells a
//! simpler way (documented per item). No production path calls into this module, so exposing it
//! cannot change an arithmetic result; the tests in `crate::fast_path_tests` pin every accessor
//! against the public expression it replaces, which is exactly what the C# rows assert.
//!
//! | Reference member (`Real.cs`) | Accessor |
//! |---|---|
//! | `Real.Exponent`, `Real.ToNatural()`, `Real.IsPeriodic`, `Int.IsNegative` | `exponent`, `magnitude`, `is_periodic`, `is_negative` |
//! | internal `new Real(Nat, bool, long)` | `raw` |
//! | `TrySpecialAngle`, `Sqrt2Half`, `Sqrt3Half` | `try_special_angle`, `sqrt2_half`, `sqrt3_half`, `special_angle_table` |
//! | `SpecialAngleCandidate` | `special_angle_candidate` |
//! | `ReduceToTwoPi`, `WholeQuotient`, `Truncate` | `reduce_to_two_pi`, `whole_quotient`, `truncate` |
//! | `HalfOf` | `half_of`, `half_path` |
//! | `Normalize`, `StripTrailingDecimalZeros` | `normalize`, `strip_trailing_decimal_zeros` |
//! | `ToInteger` | `to_integer` |
//! | `IsBelowDecimalGuard` (carried power) | `DecimalGuard`, `is_below_decimal_guard` |
//! | `CosTaylor`, `SinTaylor` | `cos_taylor`, `sin_taylor` |
//! | the `Sin`/`Cos` path decision | `trig_path`, `TrigPath` |
//!
//! Two conventions differ from the C# source and are visible at every call site:
//!
//! * The reference reads its precision from the thread-scoped `Real.MaxComputationDecimalPlaces`;
//!   the Rust kernel passes it as an explicit `places`/`digits` argument (there is no global
//!   precision state in this crate). Every accessor here takes that argument instead of reading a
//!   global, so a C# `using var scope = Real.WithPrecision(p, p)` becomes `places = p`.
//! * The reference's `ReduceToTwoPi(value, pi, twoPi)` takes `pi` and never reads it (the turn
//!   count is measured against `twoPi`); the accessor keeps the parameter for signature parity and
//!   documents it as unused, exactly as the kernel does.

use std::cmp::Ordering;

use lovelace_int::Int;
use lovelace_nat::Nat;

use crate::Real;
use crate::transcendental;

/// The series guard `Real.Sin`/`Real.Cos` add to the requested digits before running Taylor.
pub const SERIES_GUARD_DIGITS: usize = 10;

/// The first trailing-zero strip step (`StripTrailingDecimalZeros`): 10^18 is the largest power of
/// ten that fits in one 64-bit limb, so it is the cheapest first probe.
pub const INITIAL_STRIP_STEP: i64 = 18;

/// Upper bound on the galloping strip step (`StripTrailingDecimalZeros`): 10^(2^40) is far past any
/// storable value, and the bound keeps the doubling from overflowing.
pub const MAX_STRIP_STEP: i64 = 1 << 40;

/// The reference's `Real.Exponent`: the decimal scale the value is stored at.
pub fn exponent(value: &Real) -> i64 {
    value.exp
}

/// The reference's `Real.ToNatural()`: the unsigned magnitude of every stored digit.
///
/// The digits are the value's coefficient, not its display: `8.000` stored at scale -3 has
/// magnitude 8000. Render it with `Nat::to_decimal` for the digit string the C# rows compare.
pub fn magnitude(value: &Real) -> Nat {
    value.int.magnitude().clone()
}

/// The reference's `Real.IsPeriodic`: the stored digits contain a repeating block once.
pub fn is_periodic(value: &Real) -> bool {
    value.period_len > 0
}

/// The reference's `Int.IsNegative(this)`: the stored sign, not a numeric comparison (`-0` stores no
/// sign, exactly as the reference's `Int` does).
pub fn is_negative(value: &Real) -> bool {
    value.int.is_negative()
}

/// The reference's internal `new Real(Nat magnitude, bool isNegative, long exponent)`: builds a
/// terminating value at an explicit scale **without normalising** it.
///
/// This is the constructor the C# fast-path rows use to place a value in a shape public arithmetic
/// would never produce (`new Real(Nat.Parse("1" + new string('0', 2000)), false, -2000)`), so the
/// strip is observed on a value that still has its trailing zeros.
pub fn raw(magnitude: Nat, negative: bool, exponent: i64) -> Real {
    let mut int = int_from_magnitude(&magnitude);
    if negative {
        int = int.negate();
    }
    Real {
        int,
        exp: exponent,
        period_start: -1,
        period_len: 0,
        inexact: false,
    }
}

/// `Real.Sqrt2Half`: sqrt(2)/2 at `places`, the constant the special-angle table's pi/4 row uses.
///
/// The reference caches this per precision (`CachedConstantAtPrecision`); the Rust kernel rebuilds
/// it on each table lookup with the same construction, so this accessor returns exactly the value
/// `try_special_angle` hands back for a pi/4 hit.
pub fn sqrt2_half(places: usize) -> Real {
    Real::from_i64(2)
        .sqrt(places)
        .div(&Real::from_i64(2), places + 5)
        .unwrap_or_else(Real::zero)
}

/// `Real.Sqrt3Half`: sqrt(3)/2 at `places`, the constant the table's pi/6 row uses.
pub fn sqrt3_half(places: usize) -> Real {
    Real::from_i64(3)
        .sqrt(places)
        .div(&Real::from_i64(2), places + 5)
        .unwrap_or_else(Real::zero)
}

/// The 16 table rows as `(num, den, sin, cos)`: the exact sin/cos of the multiples of pi/6 and pi/4
/// in [0, 2*pi), in the reference's table order.
///
/// The sqrt-based values are rebuilt at `places`, which is what the reference's per-precision
/// constant cache yields.
pub fn special_angle_table(places: usize) -> [(u64, u64, Real, Real); 16] {
    transcendental::special_angle_table(places)
}

/// `Real.SpecialAngleCandidate`: the angle `pi * num / den` denotes at pi's own scale, computed with
/// one integer division.
///
/// A denominator of 2^a*5^b is answered exactly (the reference's `TryTerminateExactly`), every other
/// one is the truncating division at `pi`'s scale -- which is what the public
/// `pi.mul(num).div(den, places)` produces once the ambient cap is reached.
pub fn special_angle_candidate(pi: &Real, num: u64, den: u64) -> Real {
    transcendental::special_angle_candidate(pi, num, den)
}

/// `Real.TrySpecialAngle`: `Some((sin, cos))` when `x` is exactly one of the 16 table angles at
/// `pi`'s scale, `None` otherwise.
///
/// The comparison is against `pi` as the caller holds it, and a non-periodic `x` whose last stored
/// place sits above the candidate's is rejected before the comparison -- the two observables
/// `NonSpecialAngles_AreNotCapturedByTheTable` pins.
pub fn try_special_angle(x: &Real, pi: &Real, places: usize) -> Option<(Real, Real)> {
    transcendental::try_special_angle(x, pi, places)
}

/// `Real.Truncate`: the integer part of `value`, toward zero, at scale 0.
///
/// The Rust kernel does not keep the reference's private `Truncate` beside `operator%`; its
/// `truncate_toward_zero` (used by the turn-quotient fallback) is the same function, and this
/// accessor is that kernel helper -- the public expression `WholeQuotient` is pinned against.
pub fn truncate(value: &Real) -> Real {
    transcendental::truncate_toward_zero(value)
}

/// `Real.WholeQuotient`: the integer part of `left / right` (truncated toward zero) with one integer
/// division instead of `Divide`'s remainder-tracking decimal loop.
///
/// This is the kernel's own function (`transcendental::whole_quotient`), the quotient the [0, 2*pi)
/// reduction uses, so the row's equality with `Truncate(Divide(left, right))` is the live contract
/// rather than a restatement of it.
pub fn whole_quotient(left: &Real, right: &Real) -> Real {
    transcendental::whole_quotient(left, right)
}

/// `Real.ReduceToTwoPi`: `value` folded into [0, 2*pi) with the integer turn quotient.
///
/// `pi` is accepted for signature parity with the C# row and is **unused**: the reference's
/// `ReduceToTwoPi` measures every turn against `two_pi` and never reads its `pi` parameter either.
/// The kernel's own reduction is called, so the returned value is the one `Real::sin`/`Real::cos`
/// reduce with.
pub fn reduce_to_two_pi(value: &Real, pi: &Real, two_pi: &Real) -> Real {
    let _ = pi;
    transcendental::reduce_to_two_pi_whole(value, two_pi)
}

/// Which half of `Real.HalfOf` a value takes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HalfPath {
    /// The value is stored at exactly the ambient computation scale, so one place down of
    /// `magnitude * 5` is the exact half (the N23 fast path).
    ExactShift,
    /// Anything else keeps the general division by two (the reference's `value / new Real(2)`).
    GeneralDivision,
}

/// `Real.HalfOf`: exactly `value / 2`, with the reference's two branches.
///
/// The Rust kernel spells this as a division by two (`transcendental::half_of`); the reference
/// shifts the magnitude one decimal place when the value is stored at the ambient computation
/// scale. This accessor is the reference's fast path with the general division as its fallback, and
/// `half_path` reports which branch was taken. `HalfOf_MatchesTheDividedForm_ForPiShapedValues` pins
/// both branches against the public division.
///
/// `places` is the C# `MaxComputationDecimalPlaces` of the row's scope.
pub fn half_of(value: &Real, places: usize) -> Real {
    match half_path(value, places) {
        HalfPath::ExactShift => {
            let doubled = magnitude(value).mul(&Nat::from_u64(5));
            let halved = normalize(&raw(doubled, is_negative(value), value.exp - 1));
            if value.is_exact() {
                halved
            } else {
                halved.mark_inexact()
            }
        }
        HalfPath::GeneralDivision => value
            .div(&Real::from_i64(2), places)
            .unwrap_or_else(Real::zero),
    }
}

/// Which branch of `Real.HalfOf` `value` takes: the exact shift only when the value is terminating
/// and stored at exactly `-places`.
pub fn half_path(value: &Real, places: usize) -> HalfPath {
    if value.period_len == 0 && value.exp == -(places as i64) {
        HalfPath::ExactShift
    } else {
        HalfPath::GeneralDivision
    }
}

/// `Real.Normalize`: the canonical trailing-zero-free form of a terminating value.
///
/// This is the reference's *binary* strip -- galloping decimal division on the magnitude, no digit
/// rendering -- whereas the kernel's `Real::normalized` strips one zero at a time. Both produce the
/// same canonical value, which is exactly the equivalence
/// `Normalize_BinaryStrip_MatchesTheDigitStringStrip` states.
pub fn normalize(value: &Real) -> Real {
    if value.period_len > 0 || value.exp >= 0 || value.is_zero() {
        return value.clone();
    }
    let (magnitude, stripped) =
        strip_trailing_decimal_zeros(&magnitude(value), value.exp.unsigned_abs());
    if stripped == 0 {
        return value.clone();
    }
    raw(magnitude, is_negative(value), value.exp + stripped as i64)
}

/// `Real.StripTrailingDecimalZeros`: removes up to `cap` trailing decimal zeros from `value` and
/// reports how many it removed.
///
/// A zero can only be stripped when the magnitude divides by ten exactly; the step starts at
/// `INITIAL_STRIP_STEP` (10^18, one 64-bit limb), doubles after each success and halves after each
/// failure, capped at `MAX_STRIP_STEP`. A long zero run therefore costs O(log) divisions and no
/// decimal rendering at all.
pub fn strip_trailing_decimal_zeros(value: &Nat, cap: u64) -> (Nat, u64) {
    let mut value = value.clone();
    let mut stripped: u64 = 0;
    let mut step: i64 = INITIAL_STRIP_STEP;
    while stripped < cap && step > 0 {
        let trial = (step as u64).min(cap - stripped);
        let divisor = ten_to_the(trial);
        let Some((quotient, remainder)) = value.div_rem(&divisor) else {
            break;
        };
        if remainder.is_zero() {
            value = quotient;
            stripped += trial;
            if step < MAX_STRIP_STEP {
                step <<= 1;
            }
        } else if trial > 1 {
            step = (trial >> 1) as i64;
        } else {
            break; // the magnitude's last digit is not a zero
        }
    }
    (value, stripped)
}

/// `Real.ToInteger(zeros)`: the stored magnitude shifted `zeros` decimal places, with the value's
/// stored sign.
///
/// This is the exponent alignment every addition runs; the reference now shifts the binary magnitude
/// (`Nat.ShiftLeftDecimal`) instead of padding the rendered digits, and
/// `ToInteger_BinaryShift_MatchesTheDigitPadding` pins the two against each other.
pub fn to_integer(value: &Real, zeros: u64) -> Int {
    let shifted = magnitude(value).mul_pow10(zeros as usize);
    let mut int = int_from_magnitude(&shifted);
    if is_negative(value) {
        int = int.negate();
    }
    int
}

/// The kernel's series-termination expression: `|term| < 10^-(guard+1)`.
///
/// This is `transcendental::below_decimal_guard`, the predicate the live Taylor loops break on. The
/// reference's integer version carries its power of ten between terms; `DecimalGuard` exposes that
/// form.
pub fn is_below_decimal_guard(term: &Real, guard: usize) -> bool {
    transcendental::below_decimal_guard(term, guard)
}

/// `Real.IsBelowDecimalGuard` with its carried state: the reference's integer
/// `|term| < 10^-(guard+1)` test, plus the `ref Nat? tenPower, ref long tenPowerExponent` pair the
/// series loops hand from term to term.
///
/// The carried power is observable (`DecimalGuard::carried_power`,
/// `DecimalGuard::carried_exponent`) because
/// `SeriesTermination_IntegerTest_TracksTheCarriedPowerAcrossManyTerms` asserts it keeps up with the
/// falling term exponents. Every verdict equals `is_below_decimal_guard`, which is the expression
/// the reference replaced.
#[derive(Clone, Debug, Default)]
pub struct DecimalGuard {
    power: Option<Nat>,
    exponent: i64,
}

impl DecimalGuard {
    /// A guard with no carried power: the state the reference starts each series with.
    pub fn new() -> DecimalGuard {
        DecimalGuard::default()
    }

    /// The reference's `IsBelowDecimalGuard(term, guard, ref tenPower, ref tenPowerExponent)`.
    pub fn is_below(&mut self, term: &Real, guard: usize) -> bool {
        if term.is_zero() {
            return true; // 0 < 10^-(guard+1)
        }
        let k = -((guard as i64) + 1) - term.exp;
        if k <= 0 {
            return false; // |term| >= 10^Exponent >= 10^-(guard+1)
        }
        if self.power.is_none() || self.exponent > k {
            self.power = Some(ten_to_the(k as u64));
            self.exponent = k;
        } else if self.exponent < k {
            let growth = ten_to_the((k - self.exponent) as u64);
            let grown = self
                .power
                .as_ref()
                .expect("carried power present")
                .mul(&growth);
            self.power = Some(grown);
            self.exponent = k;
        }
        let power = self.power.as_ref().expect("carried power present");
        term.int.magnitude().cmp(power) == Ordering::Less
    }

    /// The carried 10^k, or `None` while no term needed a power of ten.
    pub fn carried_power(&self) -> Option<&Nat> {
        self.power.as_ref()
    }

    /// The carried exponent `k` of that power of ten.
    pub fn carried_exponent(&self) -> i64 {
        self.exponent
    }

    /// Drops the carried power, returning the guard to `DecimalGuard::new`.
    pub fn reset(&mut self) {
        self.power = None;
        self.exponent = 0;
    }
}

/// `Real.CosTaylor`: `cos(x) = 1 - x^2/2! + x^4/4! - ...` at `guard` places, stopped by
/// `is_below_decimal_guard`.
pub fn cos_taylor(x: &Real, guard: usize) -> Real {
    transcendental::cos_taylor_guarded(x, guard)
}

/// `Real.SinTaylor`: `sin(x) = x - x^3/3! + x^5/5! - ...` at `guard` places, stopped by
/// `is_below_decimal_guard`.
pub fn sin_taylor(x: &Real, guard: usize) -> Real {
    transcendental::sin_taylor_guarded(x, guard)
}

/// Which of `Real.Sin`/`Real.Cos`'s arithmetic paths a value takes.
#[derive(Clone, Debug, PartialEq)]
pub enum TrigPath {
    /// The value is one of the 16 special angles: the table answers with these exact values.
    SpecialAngle {
        /// The exact sine the table returns.
        sin: Real,
        /// The exact cosine the table returns.
        cos: Real,
    },
    /// The value is not in the table: the kernel falls through to the reduction and Taylor series.
    Taylor {
        /// The value reduced into [0, 2*pi) against the working-width pi, the argument the series is
        /// folded from.
        reduced: Real,
    },
}

/// Which path `Real.Sin`/`Real.Cos` takes for `value` at `digits` places.
///
/// The decision is the kernel's own: the table is asked first against the caller's pi (the
/// reference's ordering note in `Real.Sin`), then against the argument-aware pi. A table hit is
/// reported with the exact values it returns; a miss is reported as `Taylor` with the reduction the
/// series starts from. Negative values follow the kernel's odd/even symmetry: the path is classified
/// on the magnitude, exactly as the kernel recurses on it.
pub fn trig_path(value: &Real, digits: usize) -> TrigPath {
    let digits = digits.max(1);
    let magnitude_value = if value.cmp(&Real::zero()) == Ordering::Less {
        value.neg()
    } else {
        value.clone()
    };
    let ambient_pi = Real::pi(digits);
    let ambient_two_pi = ambient_pi.mul(&Real::from_i64(2));
    let ambient_x = transcendental::reduce_to_two_pi_whole(&magnitude_value, &ambient_two_pi);
    if let Some((sin, cos)) = transcendental::try_special_angle(&ambient_x, &ambient_pi, digits) {
        return TrigPath::SpecialAngle { sin, cos };
    }
    let working = transcendental::trig_reduction_places(&magnitude_value, digits);
    let pi = Real::pi(transcendental::trig_pi_places(
        &magnitude_value,
        digits,
        working,
    ));
    let two_pi = pi.mul(&Real::from_i64(2));
    let reduced = transcendental::reduce_to_two_pi_whole(&magnitude_value, &two_pi);
    if let Some((sin, cos)) = transcendental::try_special_angle(&reduced, &pi, working) {
        return TrigPath::SpecialAngle { sin, cos };
    }
    TrigPath::Taylor { reduced }
}

/// The reference's `TenToThe`: 10^k by binary exponentiation.
fn ten_to_the(exponent: u64) -> Nat {
    Nat::from_u64(10).pow(exponent)
}

/// The reference's `Int` from an unsigned magnitude (`new Int(magnitude)`).
fn int_from_magnitude(magnitude: &Nat) -> Int {
    Int::from_decimal(&magnitude.to_decimal()).unwrap_or_else(Int::zero)
}
