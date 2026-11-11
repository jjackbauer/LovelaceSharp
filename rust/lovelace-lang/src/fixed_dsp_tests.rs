//! Round-48 fixed-point DSP parity: the ten `FixedDspTests` census rows, mirrored.
//!
//! Every test names the C# case it mirrors (`Lovelace.Dsp.Tests/FixedDspTests.cs`) and asserts the
//! same concrete sample array and the same expected values the C# assertion states. The two
//! `MatchesDspMath` rows additionally assert parity with the arbitrary-precision form: the class
//! result is computed with [`Complex`] arithmetic (the Rust counterpart of `DspMath`) and compared
//! with the fixed result promoted through [`Cplx::to_complex`], exactly as the C# case compares
//! `DspMath` output with `fixedResult.Select(v => v.ToComplex())`.

use lovelace_real::lreal::{LReal64, LReal128};

use crate::complex::{Complex, LComplex64};
use crate::fixed_dsp::{
    Cplx, Cplx64, Cplx128, FixedDspError, convolve, impulse_response, moving_average, step_response,
};

fn r64(text: &str) -> LReal64 {
    LReal64::parse(text).unwrap_or_else(|_| panic!("LReal64 literal {text}"))
}

fn r128(text: &str) -> LReal128 {
    LReal128::parse(text).unwrap_or_else(|_| panic!("LReal128 literal {text}"))
}

fn c64(value: i64) -> Cplx64 {
    Cplx64::from_i64(value).expect("an i64 fits the 19-digit width")
}

fn c128(value: i64) -> Cplx128 {
    Cplx128::from_i64(value).expect("an i64 fits the 38-digit width")
}

fn re64(text: &str) -> Cplx64 {
    Cplx64::from_real(r64(text))
}

fn re128(text: &str) -> Cplx128 {
    Cplx128::from_real(r128(text))
}

fn class(value: i64) -> Complex {
    Complex::from_i64(value)
}

/// `DspMath.Convolve`: the same summation order over arbitrary-precision components.
fn class_convolve(x: &[Complex], h: &[Complex]) -> Vec<Complex> {
    if x.is_empty() || h.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::new();
    for n in 0..(x.len() + h.len() - 1) {
        let lo = n.saturating_sub(h.len() - 1);
        let hi = n.min(x.len() - 1);
        let mut sum = Complex::zero();
        for m in lo..=hi {
            sum = sum.add(&x[m].mul(&h[n - m]));
        }
        result.push(sum);
    }
    result
}

/// `new DspMath.MovingAverage(window, sequence)` sampled over the whole sequence.
fn class_moving_average(x: &[Complex], window: usize) -> Vec<Complex> {
    let divisor = class(window as i64);
    (0..x.len())
        .map(|n| {
            let mut sum = Complex::zero();
            for k in (n + 1).saturating_sub(window)..=n {
                sum = sum.add(&x[k]);
            }
            sum.div(&divisor)
                .expect("a positive window is a nonzero divisor")
        })
        .collect()
}

// ---------------------------------------------------------------------------
// FixedDspTests.cs:21 Convolve64_GivenTwoOnes_ProducesTriangle
// ---------------------------------------------------------------------------

#[test]
fn convolve64_given_two_ones_produces_triangle() {
    let x = [c64(1), c64(1)];
    let result = convolve(&x, &x).expect("two one-tap operands fit the width");
    assert_eq!(result, vec![c64(1), c64(2), c64(1)]);

    // The same three values narrowed into the existing fixed-width complex port, which is the
    // type the C# case names (LComplex64): [One, Parse("2"), One].
    let narrowed: Vec<LComplex64> = result
        .iter()
        .map(|value| {
            LComplex64::try_from_complex(&value.to_complex())
                .expect("an exact 19-digit decimal fits the fixed-width complex form")
        })
        .collect();
    assert_eq!(
        narrowed,
        vec![
            LComplex64::parse("1").unwrap(),
            LComplex64::parse("2").unwrap(),
            LComplex64::parse("1").unwrap(),
        ]
    );
}

// ---------------------------------------------------------------------------
// FixedDspTests.cs:30 Convolve64_Given123SelfConvolution_MatchesDspMath
// ---------------------------------------------------------------------------

#[test]
fn convolve64_given_123_self_convolution_matches_dsp_math() {
    let x = [c64(1), c64(2), c64(3)];
    let fixed = convolve(&x, &x).expect("[1,2,3] self-convolution fits 19 digits");
    assert_eq!(fixed, vec![c64(1), c64(4), c64(10), c64(12), c64(9)]);

    let class_result = class_convolve(
        &[class(1), class(2), class(3)],
        &[class(1), class(2), class(3)],
    );
    let promoted: Vec<Complex> = fixed.iter().map(Cplx::to_complex).collect();
    assert_eq!(promoted, class_result);
}

// ---------------------------------------------------------------------------
// FixedDspTests.cs:40 Convolve128_Given123SelfConvolution_MatchesDspMath
// ---------------------------------------------------------------------------

#[test]
fn convolve128_given_123_self_convolution_matches_dsp_math() {
    let x = [c128(1), c128(2), c128(3)];
    let fixed = convolve(&x, &x).expect("[1,2,3] self-convolution fits 38 digits");
    assert_eq!(fixed, vec![c128(1), c128(4), c128(10), c128(12), c128(9)]);

    let class_result = class_convolve(
        &[class(1), class(2), class(3)],
        &[class(1), class(2), class(3)],
    );
    let promoted: Vec<Complex> = fixed.iter().map(Cplx::to_complex).collect();
    assert_eq!(promoted, class_result);
}

// ---------------------------------------------------------------------------
// FixedDspTests.cs:50 Convolve64_GivenWideOperands_ThrowsPromoteException
// ---------------------------------------------------------------------------

#[test]
fn convolve64_given_wide_operands_throws_promote_exception() {
    // 19 significant digits; the square needs 38, more than the 64-bit width carries.
    let big = re64(&"9".repeat(19));
    assert_eq!(convolve(&[big], &[big]), Err(FixedDspError::Promote));
}

// ---------------------------------------------------------------------------
// FixedDspTests.cs:57 Convolve128_GivenWideOperands_ThrowsPromoteException
// ---------------------------------------------------------------------------

#[test]
fn convolve128_given_wide_operands_throws_promote_exception() {
    // 38 significant digits; the square needs 76, more than the 128-bit width carries.
    let big = re128(&"9".repeat(38));
    assert_eq!(convolve(&[big], &[big]), Err(FixedDspError::Promote));
}

// ---------------------------------------------------------------------------
// FixedDspTests.cs:64 ImpulseResponse64_GivenFir_MatchesTaps
// ---------------------------------------------------------------------------

#[test]
fn impulse_response64_given_fir_matches_taps() {
    let a = [c64(1)];
    let b = [c64(1), c64(2), c64(3)];
    let result = impulse_response(&a, &b, 5).expect("the FIR recurrence fits the width");
    assert_eq!(result, vec![c64(1), c64(2), c64(3), c64(0), c64(0)]);
}

// ---------------------------------------------------------------------------
// FixedDspTests.cs:76 ImpulseResponse128_GivenFir_MatchesTaps
// ---------------------------------------------------------------------------

#[test]
fn impulse_response128_given_fir_matches_taps() {
    let a = [c128(1)];
    let b = [c128(1), c128(2), c128(3)];
    let result = impulse_response(&a, &b, 5).expect("the FIR recurrence fits the width");
    assert_eq!(result, vec![c128(1), c128(2), c128(3), c128(0), c128(0)]);
}

// ---------------------------------------------------------------------------
// FixedDspTests.cs:88 MovingAverage64_GivenPowerOfTwoWindow_MatchesDspMath
// ---------------------------------------------------------------------------

#[test]
fn moving_average64_given_power_of_two_window_matches_dsp_math() {
    // FixedDspTests.cs samples (i % 7) for i in 0..8: [0, 1, 2, 3, 4, 5, 6, 0], window 4.
    let samples: Vec<i64> = (0..8).map(|i| i % 7).collect();
    let fixed_input: Vec<Cplx64> = samples.iter().map(|v| c64(*v)).collect();
    let fixed = moving_average(&fixed_input, 4).expect("quarters terminate inside the width");
    assert_eq!(
        fixed,
        vec![
            re64("0"),
            re64("0.25"),
            re64("0.75"),
            re64("1.5"),
            re64("2.5"),
            re64("3.5"),
            re64("4.5"),
            re64("3.75"),
        ]
    );

    let class_input: Vec<Complex> = samples.iter().map(|v| class(*v)).collect();
    let class_result = class_moving_average(&class_input, 4);
    let promoted: Vec<Complex> = fixed.iter().map(Cplx::to_complex).collect();
    assert_eq!(promoted, class_result);
}

// ---------------------------------------------------------------------------
// FixedDspTests.cs:104 MovingAverage128_GivenPowerOfTwoWindow_MatchesDspMath
// ---------------------------------------------------------------------------

#[test]
fn moving_average128_given_power_of_two_window_matches_dsp_math() {
    let samples: Vec<i64> = (0..8).map(|i| i % 7).collect();
    let fixed_input: Vec<Cplx128> = samples.iter().map(|v| c128(*v)).collect();
    let fixed = moving_average(&fixed_input, 4).expect("quarters terminate inside the width");
    assert_eq!(
        fixed,
        vec![
            re128("0"),
            re128("0.25"),
            re128("0.75"),
            re128("1.5"),
            re128("2.5"),
            re128("3.5"),
            re128("4.5"),
            re128("3.75"),
        ]
    );

    let class_input: Vec<Complex> = samples.iter().map(|v| class(*v)).collect();
    let class_result = class_moving_average(&class_input, 4);
    let promoted: Vec<Complex> = fixed.iter().map(Cplx::to_complex).collect();
    assert_eq!(promoted, class_result);
}

// ---------------------------------------------------------------------------
// FixedDspTests.cs:121 MovingAverage64_GivenNonPositiveWindow_ThrowsArgumentOutOfRange
// ---------------------------------------------------------------------------

#[test]
fn moving_average64_given_non_positive_window_throws_argument_out_of_range() {
    assert_eq!(
        moving_average(&[c64(1)], 0),
        Err(FixedDspError::WindowNotPositive)
    );
    assert_eq!(
        moving_average(&[c64(1)], -3),
        Err(FixedDspError::WindowNotPositive)
    );
}

// ---------------------------------------------------------------------------
// The difference-equation step path (FixedDsp.StepResponse), not a census row: the reference
// pre-fills the input history with one, so the first sample is already the steady state.
// ---------------------------------------------------------------------------

#[test]
fn step_response64_given_fir_prefills_the_step_history() {
    let a = [c64(1)];
    let b = [c64(1), c64(2), c64(3)];
    let result = step_response(&a, &b, 5).expect("the FIR step response fits the width");
    assert_eq!(result, vec![c64(6), c64(6), c64(6), c64(6), c64(6)]);
}

/// The direct-form recurrence with a denominator: y[n] = x[n] + 0.5·y[n−1], i.e. the
/// FixedDspTests.cs:64 recurrence with a[1] = −1/2, so the impulse response halves each step.
#[test]
fn impulse_response64_given_iir_matches_the_direct_form_recurrence() {
    let a = [c64(1), re64("-0.5")];
    let b = [c64(1)];
    let result = impulse_response(&a, &b, 4).expect("halving terminates inside the width");
    assert_eq!(
        result,
        vec![re64("1"), re64("0.5"), re64("0.25"), re64("0.125")]
    );
}

/// The final divide by a[0] goes through the reference's complex division (a[0]·Reciprocal), so a
/// scalar gain is the reciprocal of the coefficient: a = [2], b = [1] gives the impulse response
/// [1/2, 0, 0].
#[test]
fn impulse_response64_given_a_scalar_denominator_divides_by_the_tap() {
    let a = [c64(2)];
    let b = [c64(1)];
    let result = impulse_response(&a, &b, 3).expect("the reciprocal terminates inside the width");
    assert_eq!(result, vec![re64("0.5"), re64("0"), re64("0")]);
}

/// FOIL order: (1 + i)(1 − i) = 2, with the real part computed as 1·1 − 1·(−1) and the imaginary
/// part as 1·(−1) + 1·1.
#[test]
fn convolve64_given_complex_operands_foils_in_the_reference_order() {
    let left = Cplx64::new(r64("1"), r64("1"));
    let right = Cplx64::new(r64("1"), r64("-1"));
    let result = convolve(&[left], &[right]).expect("the product fits the width");
    assert_eq!(result, vec![c64(2)]);
    assert_eq!(result[0].to_string(), "2");
}

/// The quantization rule the fixed-width complex form in `complex` cannot express: the moving
/// average divides by the window with `LReal64.Divide`, which stores a repeating quotient as a
/// period. 1/3 is 0.(3), 2/3 is 0.(6), 3/3 is 1.
#[test]
fn moving_average64_given_a_three_sample_window_keeps_periodic_quotients() {
    let fixed = moving_average(&[c64(1), c64(1), c64(1)], 3).expect("periods fit the width");
    let rendered: Vec<String> = fixed.iter().map(|v| v.to_string()).collect();
    assert_eq!(rendered, vec!["0.(3)", "0.(6)", "1"]);
    // A periodic component is not an exact decimal, so the promoted form keeps the period too.
    assert_eq!(fixed[0].to_complex().to_string(), "0.(3)");
}

/// The width boundary: 19 nines × 1 stays inside the 19-digit width, × 10 needs 20 digits and
/// promotes instead of rounding.
#[test]
fn convolve64_given_nineteen_digits_keeps_the_width_and_promotes_at_twenty() {
    let big = re64(&"9".repeat(19));
    assert_eq!(
        convolve(&[big], &[c64(1)]).expect("times one stays inside the width"),
        vec![big]
    );
    assert_eq!(
        convolve(&[big], &[c64(10)]),
        Err(FixedDspError::Promote),
        "nineteen nines times ten needs twenty significant digits"
    );
}

/// The reference's argument guard: an empty denominator vector is refused before any sample.
#[test]
fn impulse_response64_given_empty_denominator_is_refused() {
    assert_eq!(
        impulse_response::<lovelace_real::lreal::W64>(&[], &[c64(1)], 3),
        Err(FixedDspError::EmptyDenominator)
    );
}
