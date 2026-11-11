//! Round 39 — additive equivalents of the Real API members the reference's tests exercise.
//!
//! The Rust representation states precision explicitly (a `digits` argument on `sqrt`, `pi`,
//! `div` and `pow`), where the reference keeps two ambient statics and a per-`AsyncLocal`
//! scope. This module adds the reference-shaped configuration surface without changing any
//! existing entry point: the statics and scopes are consulted only by the new `*_default` /
//! `*_checked` helpers and by the new display helpers. Every existing function keeps its
//! explicit-argument behaviour.

use std::cell::Cell;
use std::sync::atomic::{AtomicI64, Ordering};

use lovelace_int::Int;
use lovelace_nat::Nat;

use crate::Real;

/// `Real.DisplayDecimalPlaces` default (the reference's 100).
pub const DEFAULT_DISPLAY_DECIMAL_PLACES: i64 = 100;
/// `Real.MaxComputationDecimalPlaces` default (the reference's 1000).
pub const DEFAULT_MAX_COMPUTATION_DECIMAL_PLACES: i64 = 1000;

static DISPLAY_DECIMAL_PLACES: AtomicI64 = AtomicI64::new(DEFAULT_DISPLAY_DECIMAL_PLACES);
static MAX_COMPUTATION_DECIMAL_PLACES: AtomicI64 =
    AtomicI64::new(DEFAULT_MAX_COMPUTATION_DECIMAL_PLACES);

thread_local! {
    /// The reference's `AsyncLocal<long?>` display override. A Rust thread is the unit a test
    /// can isolate; the scope does not propagate to a child thread implicitly (see
    /// `precision_snapshot`/`with_precision_values` for the explicit capture a caller uses).
    static LOCAL_DISPLAY: Cell<Option<i64>> = const { Cell::new(None) };
    static LOCAL_MAX: Cell<Option<i64>> = const { Cell::new(None) };
}

/// An RAII precision scope, the Rust equivalent of the reference's `PrecisionScope`.
///
/// Dropping restores the values that were active on this thread when the scope was created, so
/// nesting restores in LIFO order.
pub struct PrecisionScope {
    saved_display: Option<i64>,
    saved_max: Option<i64>,
}

impl Drop for PrecisionScope {
    fn drop(&mut self) {
        LOCAL_DISPLAY.with(|slot| slot.set(self.saved_display));
        LOCAL_MAX.with(|slot| slot.set(self.saved_max));
    }
}

/// The error surface of the reference's `Real.Pow(Real)` refusals.
///
/// The reference throws `ArgumentOutOfRangeException` (with a parameter name and message) or
/// `NotImplementedException`. Rust reports the same refusal as a typed value; `message` and
/// `param_name` expose the reference's identity so a host can reproduce its diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RealPowError {
    /// `0` raised to a negative exponent: the integer path's `ArgumentOutOfRangeException`.
    ZeroBaseNegativeExponent,
    /// A nonzero base raised to a negative exponent: the reference's `NotImplementedException`.
    NegativeExponent,
    /// A non-integer exponent on a nonzero base: the reference's `NotImplementedException`.
    NonIntegerExponent,
    /// An exponent outside the `i64` range: the integer path's range refusal.
    ExponentOutOfRange,
}

impl RealPowError {
    /// The reference's diagnostic message for this refusal.
    pub fn message(self) -> &'static str {
        match self {
            RealPowError::ZeroBaseNegativeExponent => "Base cannot be zero.",
            RealPowError::NegativeExponent => "Exponent must be positive.",
            RealPowError::NonIntegerExponent => "Exponent must be an integer.",
            RealPowError::ExponentOutOfRange => "Exponent must be positive.",
        }
    }

    /// The reference's `ParamName` for the argument-out-of-range refusals.
    pub fn param_name(self) -> Option<&'static str> {
        match self {
            RealPowError::ZeroBaseNegativeExponent => Some("base"),
            RealPowError::NegativeExponent | RealPowError::ExponentOutOfRange => Some("exponent"),
            RealPowError::NonIntegerExponent => None,
        }
    }
}

impl std::fmt::Display for RealPowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RealPowError {}

/// The refusal of a square root of a negative value (the reference's `ArithmeticException`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NegativeSqrt;

impl std::fmt::Display for NegativeSqrt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the square root of a negative value is undefined")
    }
}

impl std::error::Error for NegativeSqrt {}

impl Real {
    /// Builds a real from an `f64` by parsing its shortest round-trip decimal representation,
    /// the Rust equivalent of the reference's `Real(double)` constructor. NaN and infinities
    /// have no finite decimal representation and are refused.
    pub fn from_f64(value: f64) -> Option<Real> {
        if !value.is_finite() {
            return None;
        }
        Real::from_str(&format!("{value}"))
    }

    /// Builds a real from a scaled integer: `mantissa * 10^-scale`.
    ///
    /// This is the representation-level equivalent of the reference's `Real(decimal)`
    /// constructor: the decimal literal it parses is exactly the mantissa with the decimal point
    /// inserted `scale` places from the right.
    pub fn from_decimal_scaled(mantissa: i128, scale: u32) -> Option<Real> {
        let negative = mantissa < 0;
        let mut digits = mantissa.unsigned_abs().to_string();
        if scale > 0 {
            let scale = scale as usize;
            if digits.len() <= scale {
                let padding = scale + 1 - digits.len();
                digits.insert_str(0, &"0".repeat(padding));
            }
            let split = digits.len() - scale;
            digits.insert(split, '.');
        }
        if negative {
            digits.insert(0, '-');
        }
        Real::from_str(&digits)
    }

    /// Parses an optional string. `None` is the reference's null input, refused by both
    /// `Parse` (throws) and `TryParse` (false); the Rust answer is `None` in both cases.
    pub fn from_str_opt(text: Option<&str>) -> Option<Real> {
        text.and_then(Real::from_str)
    }

    /// The decimal exponent: the value is `digits * 10^exponent`.
    pub fn exponent(&self) -> i64 {
        self.exp
    }

    /// Sets the decimal exponent directly (the reference exposes `Exponent { get; set; }`).
    /// This is a representation-level setter; it does not renormalise the value.
    pub fn set_exponent(&mut self, exponent: i64) {
        self.exp = exponent;
    }

    /// True when the value has a repeating decimal block.
    pub fn is_periodic(&self) -> bool {
        self.period_len > 0
    }

    /// The reference's `PeriodStart` value: zero for a non-periodic value.
    ///
    /// Rust stores the no-period sentinel as `-1` internally (the existing `period_start`
    /// accessor reports it); the reference's property reports `0`. This compatibility accessor
    /// exposes the reference-observable value without changing the existing accessor.
    pub fn period_start_or_zero(&self) -> i64 {
        self.period_start.max(0)
    }

    /// The unsigned magnitude the reference calls `ToNatural`.
    pub fn to_natural(&self) -> Nat {
        self.int.magnitude().clone()
    }

    /// True when the value is negative.
    pub fn is_negative(&self) -> bool {
        self.int.is_negative()
    }

    /// True when the value is strictly positive.
    pub fn is_positive(&self) -> bool {
        !self.int.is_negative() && !self.int.is_zero()
    }

    /// The reference's `Real.DisplayDecimalPlaces` (thread-local scope first, then the global).
    pub fn display_decimal_places() -> i64 {
        LOCAL_DISPLAY
            .with(|slot| slot.get())
            .unwrap_or_else(|| DISPLAY_DECIMAL_PLACES.load(Ordering::SeqCst))
    }

    /// Sets the global `Real.DisplayDecimalPlaces`; an active local scope still wins.
    pub fn set_display_decimal_places(value: i64) {
        DISPLAY_DECIMAL_PLACES.store(value, Ordering::SeqCst);
    }

    /// The reference's `Real.MaxComputationDecimalPlaces` (thread-local scope first, then the
    /// global).
    pub fn max_computation_decimal_places() -> i64 {
        LOCAL_MAX
            .with(|slot| slot.get())
            .unwrap_or_else(|| MAX_COMPUTATION_DECIMAL_PLACES.load(Ordering::SeqCst))
    }

    /// Sets the global `Real.MaxComputationDecimalPlaces`; an active local scope still wins.
    pub fn set_max_computation_decimal_places(value: i64) {
        MAX_COMPUTATION_DECIMAL_PLACES.store(value, Ordering::SeqCst);
    }

    /// Creates a scope that overrides both precision settings on the current thread.
    pub fn with_precision(computation: i64, display: i64) -> PrecisionScope {
        let saved_max = LOCAL_MAX.with(|slot| slot.replace(Some(computation)));
        let saved_display = LOCAL_DISPLAY.with(|slot| slot.replace(Some(display)));
        PrecisionScope {
            saved_display,
            saved_max,
        }
    }

    /// The effective `(computation, display)` pair on the current thread, for a child task that
    /// cannot inherit a Rust thread-local and must capture the scope explicitly.
    pub fn precision_snapshot() -> (i64, i64) {
        (
            Self::max_computation_decimal_places(),
            Self::display_decimal_places(),
        )
    }

    /// Restores a captured precision pair on the current thread.
    pub fn with_precision_values(computation: i64, display: i64) -> PrecisionScope {
        Real::with_precision(computation, display)
    }

    /// `PiTo` with the reference's validation: `digits` must be positive and no larger than
    /// the active computation cap.
    pub fn pi_checked(digits: i64) -> Option<Real> {
        let cap = Self::max_computation_decimal_places();
        if digits <= 0 || digits > cap {
            return None;
        }
        Some(Real::pi(digits as usize))
    }

    /// The reference's `Sqrt(Real)` contract: a negative input is refused rather than
    /// silently answered with zero. The Rust `sqrt` keeps its existing zero-returning
    /// behaviour; this checked entry point is the additive refusal surface.
    pub fn sqrt_checked(&self) -> Result<Real, NegativeSqrt> {
        if self.is_negative() {
            return Err(NegativeSqrt);
        }
        let digits = Self::max_computation_decimal_places().max(1) as usize;
        Ok(self.sqrt(digits))
    }

    /// The reference's batch `Sqrt(IReadOnlyList<Real>)`: an empty batch is empty, and a
    /// negative element refuses the whole batch (the reference throws `ArithmeticException`).
    pub fn sqrt_batch(values: &[Real]) -> Result<Vec<Real>, NegativeSqrt> {
        let mut out = Vec::with_capacity(values.len());
        for value in values {
            out.push(value.sqrt_checked()?);
        }
        Ok(out)
    }

    /// Divides at the active computation cap, the ambient default the reference's `/` uses.
    pub fn div_default(&self, other: &Real) -> Option<Real> {
        let digits = Self::max_computation_decimal_places().max(1) as usize;
        self.div(other, digits)
    }

    /// The reference's `Real.Pow(Real)` contract.
    ///
    /// A negative exponent on a nonzero base is the reference's `NotImplementedException`; a
    /// negative exponent on zero is the integer path's `ArgumentOutOfRangeException`; a zero
    /// base with a positive (even fractional) exponent is zero.
    pub fn pow_real_checked(&self, exponent: &Real) -> Result<Real, RealPowError> {
        if let Some(integer) = exponent.to_int_exact() {
            let text = integer.to_decimal();
            let exp = text
                .parse::<i64>()
                .map_err(|_| RealPowError::ExponentOutOfRange)?;
            if exp < 0 {
                if self.is_zero() {
                    return Err(RealPowError::ZeroBaseNegativeExponent);
                }
                return Err(RealPowError::NegativeExponent);
            }
            return self.pow_int(exp).ok_or(RealPowError::ExponentOutOfRange);
        }
        if self.is_zero() {
            if exponent.is_negative() {
                return Err(RealPowError::ZeroBaseNegativeExponent);
            }
            return Ok(Real::zero());
        }
        Err(RealPowError::NonIntegerExponent)
    }

    /// Renders the value at an explicit display width, the value-level equivalent of the
    /// reference's `ToString()` under `DisplayDecimalPlaces`.
    ///
    /// A periodic value publishes its block in full. A terminating value with no more fractional
    /// digits than `digits` publishes its canonical form. A terminating value with more stored
    /// fractional digits publishes exactly `digits` fractional places (zero-padded when the
    /// significant digits start further right), which is the reference's display window.
    pub fn to_string_with_display(&self, digits: usize) -> String {
        if self.period_len > 0 || self.exp >= 0 {
            return self.to_string();
        }
        if self.frac_digits() <= digits {
            return self.to_string();
        }
        self.to_decimal_fixed(digits)
    }

    /// Writes the display form at the active `DisplayDecimalPlaces` into a byte buffer.
    /// Returns `Some(bytes_written)`, or `None` (writing nothing) when the buffer is too small
    /// -- the reference's `TryFormat` contract.
    pub fn write_display(&self, out: &mut [u8]) -> Option<usize> {
        let digits = Self::display_decimal_places().max(0) as usize;
        let text = self.to_string_with_display(digits);
        if out.len() < text.len() {
            return None;
        }
        out[..text.len()].copy_from_slice(text.as_bytes());
        Some(text.len())
    }
}

fn int_from_nat(value: &Nat) -> Int {
    Int::from_decimal(&value.to_decimal()).unwrap_or_else(Int::zero)
}

/// The reference's `Real.PiSegment(long termStart, long termEnd)`: the Chudnovsky binary
/// splitting triple `(P, Q, T)` over the half-open term range `[term_start, term_end)`.
///
/// Satisfies the merge identity `T(a,b) = T(a,m)*Q(m,b) + P(a,m)*T(m,b)`; a single term is
/// `a_k`, `b_k`, and `(-1)^k * a_k * (A + B*k)`. `None` for an empty, negative, or
/// inverted range (the reference has no validation because its caller always passes a valid
/// range).
pub fn pi_segment(term_start: i64, term_end: i64) -> Option<(Nat, Nat, Int)> {
    const A: u64 = 13_591_409;
    const B: u64 = 545_140_134;
    const C3: &str = "262537412640768000";

    if term_start < 0 || term_end <= term_start {
        return None;
    }
    if term_end - term_start == 1 {
        let k = term_start as u64;
        if k == 0 {
            return Some((
                Nat::from_u64(1),
                Nat::from_u64(1),
                Int::from_decimal(&A.to_string()).expect("A"),
            ));
        }
        let k6 = 6 * k;
        let k3 = 3 * k;
        let first = Nat::from_u64(k6 * (k6 - 1) * (k6 - 2));
        let second = Nat::from_u64((k6 - 3) * (k6 - 4) * (k6 - 5));
        let ak = first.mul(&second);
        let third = Nat::from_u64((k3) * (k3 - 1) * (k3 - 2));
        let cube = Nat::from_u64(k * k * k);
        let c3 = Nat::from_decimal(C3).expect("C3");
        let bk = third.mul(&cube).mul(&c3);
        let factor = Nat::from_u64(A + B * k);
        let magnitude = ak.mul(&factor);
        let mut tk = int_from_nat(&magnitude);
        if k % 2 == 1 {
            tk = tk.negate();
        }
        return Some((ak, bk, tk));
    }
    let mid = (term_start + term_end) / 2;
    let (lp, lq, lt) = pi_segment(term_start, mid)?;
    let (rp, rq, rt) = pi_segment(mid, term_end)?;
    let p = lp.mul(&rp);
    let q = lq.mul(&rq);
    let t = lt.mul(&int_from_nat(&rq)).add(&int_from_nat(&lp).mul(&rt));
    Some((p, q, t))
}
