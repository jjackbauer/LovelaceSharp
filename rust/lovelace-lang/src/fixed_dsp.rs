//! Fixed-point DSP kernels: the Rust port of the reference `Lovelace.Dsp.FixedDsp` surface.
//!
//! The reference has two DSP tiers. [`crate::builtins`]' DSP builtins are the arbitrary-precision
//! one; `FixedDsp` is the whole-array tier over the FIXED-WIDTH exact-decimal complex values
//! (`LComplex64` over `LReal64`, `LComplex128` over `LReal128`). Its members are linear
//! convolution, the direct-form difference equation (impulse and step response) and the causal
//! moving average. The point of the tier is its QUANTIZATION contract, which this module
//! reproduces member for member:
//!
//! * a value is `significand × 10^exponent` with at most 19 significant digits (`Cplx64`) or 38
//!   (`Cplx128`), exact, with trailing fractional zeros stripped and a canonical unsigned zero;
//! * every operation is EXACT: addition aligns at the smaller exponent and adds integers,
//!   multiplication forms the exact significand product, division runs the exact long division;
//! * a result that needs more digits than the width carries PROMOTES
//!   ([`FixedDspError::Promote`], the reference's `LRealPromoteException`) instead of rounding.
//!   The caller promotes to the arbitrary-precision tier, exactly as the reference's caller does;
//! * division whose exact quotient repeats stores the period (`1/3` is `0.(3)`), and a periodic
//!   operand is written out to a fixed working width before an operation (18 fractional places for
//!   the 64-bit width, 37 for the 128-bit one) and marked inexact; a result whose repeating block
//!   is at the working width is the boundary repeating and is kept as an inexact prefix;
//! * the only place a digit is ever altered is the all-nines period, which carries into the
//!   non-repeating prefix (the reference's `DetectAndNormalizePeriod`), and the zero rules:
//!   a zero is never signed, and `0 × anything` is the zero with the operands' provenance.
//!
//! ## Why the components are `lovelace_real` fixed reals
//!
//! The values here are [`Cplx<W>`] pairs of [`lovelace_real::lreal::Fixed`] components rather than
//! [`crate::complex::LComplex64`]. The `complex` module's fixed-width form states that it does not
//! model the reference's periodic expansions, while `FixedDsp.MovingAverage` divides by the window
//! with `LReal64.Divide`, which stores a repeating quotient as a period. The `lovelace_real` fixed
//! real IS the statement-for-statement port of `LReal64`/`LReal128` (working expansion, period
//! re-detection and all), so this tier keeps the reference's values for every input, not only for
//! the terminating ones. [`Cplx::to_complex`] promotes a result to the arbitrary-precision
//! [`Complex`] form and `complex::LComplex64::try_from_complex` narrows it back when it fits.
//!
//! ## Stated boundary
//!
//! The reference's kernels poll `KernelCancellation` once per output sample and once per inner
//! term. These kernels are pure: they take slices and return a value, with no cancellation seam and
//! no allocation refusal, because no caller in this workspace drives them yet. The language DSP
//! builtins keep the cancellation seam at their own call boundary.

use std::fmt;

use lovelace_real::lreal::{Fixed, LRealError, LRealWidth, W64, W128};

use crate::complex::Complex;

/// A fixed-width complex value: `re + im·i`, each component a fixed-width exact decimal
///
/// (`LReal64` or `LReal128`). This is the value shape of the reference's `LComplex64` /
/// `LComplex128` that `FixedDsp` operates on; the components are the period-preserving fixed
/// reals, so a quotient such as `1/3` is `0.(3)` rather than a promotion.
#[derive(Clone, Copy, Debug)]
pub struct Cplx<W: LRealWidth> {
    re: Fixed<W>,
    im: Fixed<W>,
}

/// The 19-significant-digit complex value: the shape of the reference's `LComplex64`.
pub type Cplx64 = Cplx<W64>;

/// The 38-significant-digit complex value: the shape of the reference's `LComplex128`.
pub type Cplx128 = Cplx<W128>;

impl<W: LRealWidth> Cplx<W> {
    /// Builds `re + im·i` from its two fixed-width components.
    pub fn new(re: Fixed<W>, im: Fixed<W>) -> Cplx<W> {
        Cplx { re, im }
    }

    /// Builds the purely real value `re + 0i`.
    pub fn from_real(re: Fixed<W>) -> Cplx<W> {
        Cplx {
            re,
            im: Fixed::zero(),
        }
    }

    /// The additive identity `0 + 0i`.
    pub fn zero() -> Cplx<W> {
        Cplx::from_real(Fixed::zero())
    }

    /// The multiplicative identity `1 + 0i`.
    pub fn one() -> Cplx<W> {
        Cplx::from_real(Fixed::one())
    }

    /// Builds the integer `value + 0i`; an integer wider than the fixed width promotes.
    pub fn from_i64(value: i64) -> Result<Cplx<W>, FixedDspError> {
        let re = Fixed::<W>::parse(&value.to_string()).map_err(FixedDspError::from)?;
        Ok(Cplx::from_real(re))
    }

    /// The real component.
    pub fn re(&self) -> &Fixed<W> {
        &self.re
    }

    /// The imaginary component.
    pub fn im(&self) -> &Fixed<W> {
        &self.im
    }

    /// True when both components are zero.
    pub fn is_zero(&self) -> bool {
        self.re.is_zero() && self.im.is_zero()
    }

    /// Component-wise addition, promoting when a component outgrows the width.
    pub fn add(&self, other: &Cplx<W>) -> Result<Cplx<W>, FixedDspError> {
        Ok(Cplx {
            re: self.re.add(&other.re).map_err(FixedDspError::from)?,
            im: self.im.add(&other.im).map_err(FixedDspError::from)?,
        })
    }

    /// Component-wise subtraction, promoting when a component outgrows the width.
    pub fn sub(&self, other: &Cplx<W>) -> Result<Cplx<W>, FixedDspError> {
        Ok(Cplx {
            re: self.re.sub(&other.re).map_err(FixedDspError::from)?,
            im: self.im.sub(&other.im).map_err(FixedDspError::from)?,
        })
    }

    /// FOIL multiplication in the reference's order:
    /// `(a.re·b.re − a.im·b.im) + (a.re·b.im + a.im·b.re)i`.
    pub fn mul(&self, other: &Cplx<W>) -> Result<Cplx<W>, FixedDspError> {
        let re = self
            .re
            .mul(&other.re)
            .map_err(FixedDspError::from)?
            .sub(&self.im.mul(&other.im).map_err(FixedDspError::from)?)
            .map_err(FixedDspError::from)?;
        let im = self
            .re
            .mul(&other.im)
            .map_err(FixedDspError::from)?
            .add(&self.im.mul(&other.re).map_err(FixedDspError::from)?)
            .map_err(FixedDspError::from)?;
        Ok(Cplx { re, im })
    }

    /// Component-wise division by a real scalar (the reference's `LComplex64 / LReal`): each
    /// component takes the exact quotient, which may be a periodic expansion.
    pub fn div_scalar(&self, scalar: &Fixed<W>) -> Result<Cplx<W>, FixedDspError> {
        Ok(Cplx {
            re: self.re.div(scalar).map_err(FixedDspError::from)?,
            im: self.im.div(scalar).map_err(FixedDspError::from)?,
        })
    }

    /// `1/z = conj(z) / |z|²`, the reference's `Reciprocal`.
    pub fn reciprocal(&self) -> Result<Cplx<W>, FixedDspError> {
        let magnitude_squared = self
            .re
            .mul(&self.re)
            .map_err(FixedDspError::from)?
            .add(&self.im.mul(&self.im).map_err(FixedDspError::from)?)
            .map_err(FixedDspError::from)?;
        Ok(Cplx {
            re: self
                .re
                .div(&magnitude_squared)
                .map_err(FixedDspError::from)?,
            im: self
                .im
                .neg()
                .div(&magnitude_squared)
                .map_err(FixedDspError::from)?,
        })
    }

    /// Complex division `a · (1/b)`, the reference's `LComplex64 / LComplex64`.
    pub fn div(&self, other: &Cplx<W>) -> Result<Cplx<W>, FixedDspError> {
        self.mul(&other.reciprocal()?)
    }

    /// The exact arbitrary-precision value (the reference's `ToComplex`).
    pub fn to_complex(&self) -> Complex {
        Complex::new(self.re.to_real(), self.im.to_real())
    }
}

impl<W: LRealWidth> PartialEq for Cplx<W> {
    fn eq(&self, other: &Cplx<W>) -> bool {
        self.re == other.re && self.im == other.im
    }
}

impl<W: LRealWidth> Eq for Cplx<W> {}

impl<W: LRealWidth> fmt::Display for Cplx<W> {
    /// The reference's `LComplex64.ToString` shape: a zero part is omitted, and a nonzero pair is
    /// `re + im·i` with the imaginary sign spelled out.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.im.is_zero() {
            return f.write_str(&self.re.to_string());
        }
        if self.re.is_zero() {
            return write!(f, "{}i", self.im);
        }
        let im_negative = self.im.is_negative();
        let im_absolute = if im_negative { self.im.neg() } else { self.im };
        write!(
            f,
            "{} {} {}i",
            self.re,
            if im_negative { "-" } else { "+" },
            im_absolute
        )
    }
}

/// Why a fixed-point DSP kernel could not produce a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixedDspError {
    /// A component needed more significant digits than the width carries. The reference throws
    /// `LRealPromoteException`; the caller promotes to the arbitrary-precision form instead of
    /// rounding.
    Promote,
    /// A divisor was zero. The reference throws `DivideByZeroException`; reachable when `a[0]` is
    /// zero.
    DivideByZero,
    /// The denominator coefficient vector `a` was empty. The reference throws `ArgumentException`.
    EmptyDenominator,
    /// The moving-average window was not positive. The reference throws
    /// `ArgumentOutOfRangeException`.
    WindowNotPositive,
}

impl From<LRealError> for FixedDspError {
    fn from(error: LRealError) -> FixedDspError {
        match error {
            LRealError::Promote => FixedDspError::Promote,
            LRealError::DivideByZero => FixedDspError::DivideByZero,
            // A value the fixed real cannot parse is one the width cannot hold.
            LRealError::Parse => FixedDspError::Promote,
        }
    }
}

impl fmt::Display for FixedDspError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            FixedDspError::Promote => {
                "the result requires more significant digits than the fixed width carries; promote to the arbitrary-precision form."
            }
            FixedDspError::DivideByZero => "cannot divide by zero.",
            FixedDspError::EmptyDenominator => {
                "the denominator coefficient vector 'a' must not be empty."
            }
            FixedDspError::WindowNotPositive => "window must be positive.",
        };
        f.write_str(text)
    }
}

impl std::error::Error for FixedDspError {}

/// Linear convolution `y[n] = Σ_m x[m]·h[n−m]` over fixed-width components.
///
/// An empty input gives the empty vector (the reference's `Count == 0` guard). Each output sample
/// is accumulated in ascending `m` order, so a promotion happens at the same term the reference's
/// does.
pub fn convolve<W: LRealWidth>(
    x: &[Cplx<W>],
    h: &[Cplx<W>],
) -> Result<Vec<Cplx<W>>, FixedDspError> {
    if x.is_empty() || h.is_empty() {
        return Ok(Vec::new());
    }
    let length = x.len() + h.len() - 1;
    let mut result = Vec::with_capacity(length);
    for n in 0..length {
        let lo = n.saturating_sub(h.len() - 1);
        let hi = n.min(x.len() - 1);
        let mut sum = Cplx::zero();
        for m in lo..=hi {
            sum = sum.add(&x[m].mul(&h[n - m])?)?;
        }
        result.push(sum);
    }
    Ok(result)
}

/// IIR/FIR impulse response (direct form, zero initial state) over fixed-width components.
///
/// The difference equation is evaluated in the reference's exact order: the feedback sum
/// `−Σ_{j≥1} a[j]·y[j−1]` first, then the feed-forward sum `Σ_j b[j]·x[j]`, then the division by
/// `a[0]`. The order matters because each term is quantized to the fixed width and any term may
/// promote before the final value is known.
pub fn impulse_response<W: LRealWidth>(
    a: &[Cplx<W>],
    b: &[Cplx<W>],
    n: usize,
) -> Result<Vec<Cplx<W>>, FixedDspError> {
    difference_equation(a, b, n, false)
}

/// IIR/FIR step response (direct form, zero initial state) over fixed-width components.
///
/// The reference pre-fills the whole input history with one, so the first published sample already
/// carries the steady-state feed-forward sum `Σ b[j]`.
pub fn step_response<W: LRealWidth>(
    a: &[Cplx<W>],
    b: &[Cplx<W>],
    n: usize,
) -> Result<Vec<Cplx<W>>, FixedDspError> {
    difference_equation(a, b, n, true)
}

fn difference_equation<W: LRealWidth>(
    a: &[Cplx<W>],
    b: &[Cplx<W>],
    n: usize,
    step_input: bool,
) -> Result<Vec<Cplx<W>>, FixedDspError> {
    if a.is_empty() {
        return Err(FixedDspError::EmptyDenominator);
    }
    let mut x = vec![
        if step_input {
            Cplx::one()
        } else {
            Cplx::zero()
        };
        b.len()
    ];
    if !step_input && !b.is_empty() {
        x[0] = Cplx::one();
    }
    let mut y = vec![Cplx::zero(); a.len()];
    let mut response = Vec::with_capacity(n);
    for _ in 0..n {
        let mut acc = Cplx::zero();
        for j in 1..a.len() {
            acc = acc.sub(&a[j].mul(&y[j - 1])?)?;
        }
        for j in 0..b.len() {
            acc = acc.add(&b[j].mul(&x[j])?)?;
        }
        let sample = acc.div(&a[0])?;
        response.push(sample);
        for j in (1..a.len()).rev() {
            y[j] = y[j - 1];
        }
        y[0] = sample;
        if !step_input {
            for j in (1..b.len()).rev() {
                x[j] = x[j - 1];
            }
            if !b.is_empty() {
                x[0] = Cplx::zero();
            }
        }
    }
    Ok(response)
}

/// Moving average over exactly `window` samples ending at `n`, over fixed-width components.
///
/// The divisor is the window as a fixed-width value and each output sample is
/// `sum ÷ window` component by component, so a quotient whose exact expansion repeats keeps its
/// period (`1/3` is `0.(3)`) instead of promoting. A non-positive window is refused before any
/// sample is produced, matching the reference's argument guard.
pub fn moving_average<W: LRealWidth>(
    x: &[Cplx<W>],
    window: i64,
) -> Result<Vec<Cplx<W>>, FixedDspError> {
    if window <= 0 {
        return Err(FixedDspError::WindowNotPositive);
    }
    let divisor = Fixed::<W>::parse(&window.to_string()).map_err(FixedDspError::from)?;
    let mut result = Vec::with_capacity(x.len());
    for (n, _) in x.iter().enumerate() {
        let mut sum = Cplx::zero();
        // The reference walks k from n−window+1 and skips the negative indices; starting at zero
        // adds the same samples in the same order without walking a huge negative range.
        let lo = (n as i64 + 1).saturating_sub(window).max(0);
        for k in lo..=n as i64 {
            sum = sum.add(&x[k as usize])?;
        }
        result.push(sum.div_scalar(&divisor)?);
    }
    Ok(result)
}
