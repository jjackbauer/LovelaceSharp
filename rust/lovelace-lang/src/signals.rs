//! Discrete-time complex signals: the reference's Lovelace.Dsp.Signals surface.
//!
//! The reference models a signal as ISignal.Get(n) -> Complex. This module ports the finite
//! pieces the DSP tests pin: the unit impulse and step, a finite Sequence over an inclusive
//! support, and the point-wise Sum and Product combinators. A sample lives in the arbitrary
//! precision crate::complex::Complex, so no floating point is involved anywhere.
//!
//! The language's DSP builtins (impulse/step/delay/filter/...) sample these shapes over a
//! finite window and publish the resulting vector; the types here are the library surface the
//! reference's own DspTests exercise directly, including the Sequence constructor's length
//! argument refusal, which a runner script has no way to name.

use crate::complex::Complex;

/// A discrete-time complex signal: an arbitrary-precision value at each integer index.
pub trait Signal {
    /// The sample at index n; outside a finite support the value is zero.
    fn get(&self, n: i64) -> Complex;
}

/// Unit impulse: 1 at n == 0, zero elsewhere.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Impulse;

impl Signal for Impulse {
    fn get(&self, n: i64) -> Complex {
        if n == 0 {
            Complex::one()
        } else {
            Complex::zero()
        }
    }
}

/// Unit step: 1 for n >= 0, zero elsewhere.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Step;

impl Signal for Step {
    fn get(&self, n: i64) -> Complex {
        if n >= 0 {
            Complex::one()
        } else {
            Complex::zero()
        }
    }
}

/// A finite complex sequence over the inclusive support [lower, upper].
///
/// Get answers the stored sample inside the support and zero outside it, exactly the
/// reference's Sequence. The constructor carries the reference's ArgumentException as
/// [SequenceError]: the value count must be upper - lower + 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sequence {
    lower: i64,
    upper: i64,
    values: Vec<Complex>,
}

impl Sequence {
    /// Builds a sequence over [lower, upper] from exactly that many values.
    pub fn new(lower: i64, upper: i64, values: Vec<Complex>) -> Result<Sequence, SequenceError> {
        let expected = (upper as i128) - (lower as i128) + 1;
        if expected != values.len() as i128 {
            return Err(SequenceError {
                lower,
                upper,
                expected,
                got: values.len(),
            });
        }
        Ok(Sequence {
            lower,
            upper,
            values,
        })
    }

    /// The inclusive lower bound of the support.
    pub fn lower(&self) -> i64 {
        self.lower
    }

    /// The inclusive upper bound of the support.
    pub fn upper(&self) -> i64 {
        self.upper
    }

    /// The stored samples, in support order.
    pub fn values(&self) -> &[Complex] {
        &self.values
    }
}

impl Signal for Sequence {
    fn get(&self, n: i64) -> Complex {
        if n < self.lower || n > self.upper {
            return Complex::zero();
        }
        self.values[(n - self.lower) as usize].clone()
    }
}

/// The reference's Sequence length-mismatch ArgumentException.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequenceError {
    lower: i64,
    upper: i64,
    expected: i128,
    got: usize,
}

impl SequenceError {
    /// The required value count, upper - lower + 1.
    pub fn expected(&self) -> i128 {
        self.expected
    }

    /// The supplied value count.
    pub fn got(&self) -> usize {
        self.got
    }
}

impl std::fmt::Display for SequenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Sequence over [{},{}] requires {} value(s), but got {}. (Parameter 'values')",
            self.lower, self.upper, self.expected, self.got
        )
    }
}

impl std::error::Error for SequenceError {}

/// Point-wise sum: x(n) + y(n).
#[derive(Clone, Debug)]
pub struct Sum<X: Signal, Y: Signal> {
    x: X,
    y: Y,
}

impl<X: Signal, Y: Signal> Sum<X, Y> {
    /// Builds the point-wise sum of two signals.
    pub fn new(x: X, y: Y) -> Sum<X, Y> {
        Sum { x, y }
    }

    /// The first operand.
    pub fn x(&self) -> &X {
        &self.x
    }

    /// The second operand.
    pub fn y(&self) -> &Y {
        &self.y
    }
}

impl<X: Signal, Y: Signal> Signal for Sum<X, Y> {
    fn get(&self, n: i64) -> Complex {
        self.x.get(n).add(&self.y.get(n))
    }
}

/// Point-wise product: x(n) * y(n).
#[derive(Clone, Debug)]
pub struct Product<X: Signal, Y: Signal> {
    x: X,
    y: Y,
}

impl<X: Signal, Y: Signal> Product<X, Y> {
    /// Builds the point-wise product of two signals.
    pub fn new(x: X, y: Y) -> Product<X, Y> {
        Product { x, y }
    }

    /// The first operand.
    pub fn x(&self) -> &X {
        &self.x
    }

    /// The second operand.
    pub fn y(&self) -> &Y {
        &self.y
    }
}

impl<X: Signal, Y: Signal> Signal for Product<X, Y> {
    fn get(&self, n: i64) -> Complex {
        self.x.get(n).mul(&self.y.get(n))
    }
}
