//! Round-21 DSP parity batch: the census rows the round-20 DSP builtins unblock.
//!
//! Every test is one C# census case (or a small group of theory rows that share one assertion).
//! The C# assertion is the contract. Where the reference test exercises a signal/DSPMath type that
//! the Rust language surface does not expose, the test composes the builtins that produce the same
//! observable vector and says so in the comment; a row whose assertion cannot be reproduced is
//! refused in docs/goal-parity-contract/round-21/dsp-porting-batch.md rather than weakened here.

use crate::{Engine, LangError, Value, structured};

fn engine() -> Engine {
    Engine::new()
}

fn eval(source: &str) -> Result<Value, LangError> {
    let mut e = engine();
    e.eval(source)
}

fn typed(source: &str) -> String {
    let value = eval(source).unwrap_or_else(|error| panic!("{source}: {}", error.message));
    value.typed()
}

fn error(source: &str) -> LangError {
    eval(source).expect_err(&format!("{source} must be refused"))
}

fn structured_elements(source: &str) -> Vec<lovelace_json::Json> {
    let mut e = engine();
    let value = e
        .eval(source)
        .unwrap_or_else(|error| panic!("{source}: {}", error.message));
    let projected = structured::project_value(&e, &value, None);
    projected
        .get("elements")
        .and_then(|e| e.as_arr())
        .unwrap_or_else(|| panic!("{source}: no elements in {projected:?}"))
        .to_vec()
}

fn element_field(source: &str, index: usize, field: &str) -> String {
    let elements = structured_elements(source);
    elements[index]
        .get(field)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("{source}: element {index} has no string {field}"))
        .to_string()
}

// ---------------------------------------------------------------------------
// Lovelace.Dsp.Tests/DspTests.cs -- signal generators and DspMath, through builtins
// ---------------------------------------------------------------------------

/// DspTests.cs:22 Impulse.Get(0) == 1, Get(1) == 0, Get(-1) == 0. The finite window carries
/// samples 0..n-1; delay by one places the sample at n = -1 (x[-1]) in slot 0, where the
/// reference signal is zero outside its support.
#[test]
fn impulse_get_zero_and_nonzero_is_one_only_at_zero() {
    assert_eq!(typed("impulse(2)"), "[1, 0] (Vector)");
    assert_eq!(typed("re(delay(impulse(1), 1)[0])"), "0 (Real)");
}

/// DspTests.cs:30 Step.Get(-1) == 0, Get(0) == 1, Get(5) == 1.
#[test]
fn step_get_negative_and_nonnegative_is_one_for_nonnegative() {
    assert_eq!(typed("step(6)"), "[1, 1, 1, 1, 1, 1] (Vector)");
    assert_eq!(typed("re(delay(step(1), 1)[0])"), "0 (Real)");
}

/// DspTests.cs:38 new Delay(3, Impulse).Get(3) == 1 and .Get(2) == 0.
#[test]
fn delay_given_impulse_and_k_shifts_signal() {
    assert_eq!(typed("delay(impulse(4), 3)[3]"), "1 (Complex)");
    assert_eq!(typed("delay(impulse(4), 3)[2]"), "0 (Complex)");
}

/// DspTests.cs:46 Scalar(2, Step).Get(0) == 2 and .Get(3) == 2. A one-tap convolution is the
/// pointwise scalar multiply on the same four samples.
#[test]
fn scalar_given_step_and_k_scales() {
    assert_eq!(typed("conv([1, 1, 1, 1], [2])"), "[2, 2, 2, 2] (Vector)");
}

/// DspTests.cs:70 PowerSeries(2, 0.5): Get(3) == 0.75, Get(0) == 0.
#[test]
fn powerseries_given_k_and_a_computes_kn_a_pow_n() {
    assert_eq!(typed("powerseries(2, 0.5, 4)[3]"), "0.75 (Complex)");
    assert_eq!(typed("powerseries(2, 0.5, 4)[0]"), "0 (Complex)");
}

/// DspTests.cs:78 Convolve([1,1], [1,1]) == [1, 2, 1].
#[test]
fn convolve_given_two_ones_produces_triangle() {
    assert_eq!(typed("conv([1, 1], [1, 1])"), "[1, 2, 1] (Vector)");
}

/// DspTests.cs:85 Convolve([1,2,3], [1,2,3]) == [1, 4, 10, 12, 9].
#[test]
fn convolve_given_123_self_convolution_produces_square() {
    assert_eq!(
        typed("conv([1, 2, 3], [1, 2, 3])"),
        "[1, 4, 10, 12, 9] (Vector)"
    );
}

/// DspTests.cs:92 MovingAverage(2, Step).Get(0) == 0.5, Get(1) == 1. The builtin is the same
/// causal zero-padded window over the same unit-step samples.
#[test]
fn movingaverage_given_step_returns_window_average() {
    assert_eq!(typed("movingavg(step(2), 2)"), "[0.5, 1] (Vector)");
}

/// DspTests.cs:100 ImpulseResponse([1], [1], 3) == [1, 0, 0]. A FIR impulse response is the
/// convolution of the unit impulse with the taps; the builtin convolution is that operation.
#[test]
fn impulse_response_given_unit_filter_passes_impulse() {
    assert_eq!(typed("conv([1, 0, 0], [1])"), "[1, 0, 0] (Vector)");
}

/// DspTests.cs:116 ImpulseResponse([1], [1,2,3], 5) == [1, 2, 3, 0, 0].
#[test]
fn impulse_response_given_fir_matches_convolution() {
    assert_eq!(
        typed("conv([1, 0, 0, 0, 0], [1, 2, 3])"),
        "[1, 2, 3, 0, 0, 0, 0] (Vector)"
    );
}

/// DspTests.cs:130 Sequence(0, 1, [5,7]): Get(-1) == 0, Get(2) == 0, and the two samples.
/// Delay by +/- one places the out-of-support samples in the window, where Sequence is zero.
#[test]
fn sequence_given_index_outside_support_returns_zero() {
    assert_eq!(typed("[5, 7][0]"), "5 (Natural)");
    assert_eq!(typed("[5, 7][1]"), "7 (Natural)");
    assert_eq!(typed("re(delay([5, 7], 1)[0])"), "0 (Real)");
    assert_eq!(typed("re(delay([5, 7], -1)[1])"), "0 (Real)");
}

// ---------------------------------------------------------------------------
// Lovelace.Dsp.Tests/FftTests.cs
// ---------------------------------------------------------------------------

/// FftTests.cs:18 Fft([1]) == [1].
#[test]
fn fft_given_impulse_n1_returns_identity() {
    assert_eq!(typed("fft([1])"), "[1] (Vector)");
}

/// FftTests.cs:22 Fft([1,0]) == [1,1].
#[test]
fn fft_given_impulse_n2_returns_all_ones() {
    assert_eq!(typed("fft([1, 0])"), "[1, 1] (Vector)");
}

/// FftTests.cs:26 Fft([1,1]) == [2,0].
#[test]
fn fft_given_constant_n2_returns_dc_only() {
    assert_eq!(typed("fft([1, 1])"), "[2, 0] (Vector)");
}

/// FftTests.cs:30 Fft([1,0,0,0]) == [1,1,1,1].
#[test]
fn fft_given_impulse_n4_returns_all_ones() {
    assert_eq!(typed("fft([1, 0, 0, 0])"), "[1, 1, 1, 1] (Vector)");
}

/// FftTests.cs:36 Fft([1,1,1,1]) == [4,0,0,0].
#[test]
fn fft_given_constant_n4_returns_dc_only() {
    assert_eq!(typed("fft([1, 1, 1, 1])"), "[4, 0, 0, 0] (Vector)");
}

/// FftTests.cs:42/47/50 AssertMatchesDft(n): the transform pair agrees on a deterministic complex
/// signal. The language has no complex-literal constructor, so the signal is the complex output of
/// the same transform over a ramp; both real and imaginary parts are exercised and the asserted
/// fact (fft == dft, element for element) is identical.
fn fft_matches_dft(n: usize) {
    let ramp = (1..=n)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let script = format!("x = fft([{ramp}]); fft(x) == dft(x)");
    assert_eq!(typed(&script), "True (Boolean)", "{script}");
}

#[test]
fn fft_given_structured_n8_matches_dft() {
    fft_matches_dft(8);
}

#[test]
fn fft_given_structured_n16_matches_dft() {
    fft_matches_dft(16);
}

#[test]
fn fft_given_structured_n64_matches_dft() {
    fft_matches_dft(64);
}

/// FftTests.cs:54: the FFT of cos(2*pi*n/8) has peaks of magnitude squared 16 at bins 1 and 7 and
/// zeros elsewhere, within 1e-41. The tolerance is a decimal literal because the language's e is
/// Euler's number, not a scientific-notation exponent.
#[test]
fn fft_given_real_cosine_returns_expected_spectral_peaks() {
    let tol = format!("0.{}1", "0".repeat(40));
    let power = |k: usize| {
        format!(
            "setprecision(50); abs(re(fft(cosine(1/8, 0, 8))[{k}])^2 + im(fft(cosine(1/8, 0, 8))[{k}])^2{}) < {tol}",
            if k == 1 || k == 7 { " - 16" } else { "" }
        )
    };
    for k in 0..8 {
        assert_eq!(
            typed(&power(k)),
            "True (Boolean)",
            "bin {k} is not within the reference tolerance"
        );
    }
}

// ---------------------------------------------------------------------------
// Lovelace.Dsp.Tests/FourierTests.cs
// ---------------------------------------------------------------------------

/// FourierTests.cs:18 Rl.Exp(0) == 1.
#[test]
fn exp_given_zero_is_exactly_one() {
    assert_eq!(typed("re(exponential(0, 1)[0])"), "1 (Real)");
}

/// FourierTests.cs:22 Rl.Exp(1).ToString() starts "2.71828182845904523536".
#[test]
fn exp_given_one_matches_e() {
    assert!(typed("re(exponential(1, 2)[1])").starts_with("2.71828182845904523536"));
}

/// FourierTests.cs:26 Complex.Zero.Exp(30) == Complex.One.
#[test]
fn complex_exp_given_zero_is_exactly_one() {
    assert_eq!(typed("exponential(0, 1)[0]"), "1 (Complex)");
}

/// FourierTests.cs:29 Dft([1,0,0,0]) == [1,1,1,1].
#[test]
fn dft_given_impulse_n4_returns_all_ones() {
    assert_eq!(typed("dft([1, 0, 0, 0])"), "[1, 1, 1, 1] (Vector)");
}

/// FourierTests.cs:36 Dft([1,1,1,1]) == [4,0,0,0].
#[test]
fn dft_given_constant_n4_returns_dc_only() {
    assert_eq!(typed("dft([1, 1, 1, 1])"), "[4, 0, 0, 0] (Vector)");
}

/// FourierTests.cs:43 Dft([1,0,0]) == [1,1,1].
#[test]
fn dft_given_impulse_n3_returns_all_ones() {
    assert_eq!(typed("dft([1, 0, 0])"), "[1, 1, 1] (Vector)");
}

/// FourierTests.cs:50 Cosine(1/4, 0, 30).Get(0) == Complex.One.
#[test]
fn cosine_given_n0_is_exactly_one() {
    assert_eq!(typed("cosine(1/4, 0, 1)[0]"), "1 (Complex)");
}

/// FourierTests.cs:57 Exponential(i, 30).Get(1) == cos(1) + i*sin(1); the reference asserts the
/// leading decimal digits of both parts.
#[test]
fn exponential_given_i_matches_euler() {
    let value = typed("exponential(fft([0, 1, 0, 0])[3], 2)[1]");
    let (re, im) = value
        .split_once(" + ")
        .unwrap_or_else(|| panic!("not a two-part complex value: {value}"));
    assert!(re.starts_with("0.5403023058681397174"), "{re}");
    assert!(im.starts_with("0.8414709848078965066"), "{im}");
}

/// FourierTests.cs:67 Noise(1,0,42,20): two instances agree at Get(0)/Get(1), and Get(3) has both
/// parts in [0, 1).
#[test]
fn noise_given_same_seed_is_reproducible_and_bounded() {
    let a = typed("noise(1, 0, 42, 4)");
    let b = typed("noise(1, 0, 42, 4)");
    assert_eq!(a, b);
    let re = element_field("noise(1, 0, 42, 4)", 3, "re");
    let im = element_field("noise(1, 0, 42, 4)", 3, "im");
    let re: f64 = re.parse().expect("a decimal real part");
    let im: f64 = im.parse().expect("a decimal imaginary part");
    assert!((0.0..1.0).contains(&re), "{re}");
    assert!((0.0..1.0).contains(&im), "{im}");
}

// ---------------------------------------------------------------------------
// Lovelace.Dsp.Tests/TrigTests.cs -- the reference runs at 100 computation digits (TestPrecision.cs),
// so the cosine builtin is driven at the same precision and the returned Real parts are compared.
// sin(x) is cos(pi/2 - x), which is the frequency 1/4 - x/(2*pi) at sample 1.
// ---------------------------------------------------------------------------

fn real_cos_at_100(freq: &str) -> String {
    typed(&format!("setprecision(100); re(cosine({freq}, 0, 2)[1])"))
}

/// TrigTests.cs:14 Sin(0) == 0.
#[test]
fn sin_given_zero_is_exactly_zero() {
    assert_eq!(real_cos_at_100("1/4"), "0 (Real)");
}

/// TrigTests.cs:18 Sin(pi/6) == 0.5.
#[test]
fn sin_given_pi_over_6_is_exactly_one_half() {
    assert_eq!(real_cos_at_100("1/6"), "0.5 (Real)");
}

/// TrigTests.cs:22 Sin(pi/4) == sqrt(2)/2.
#[test]
fn sin_given_pi_over_4_is_sqrt2_over_2() {
    assert_eq!(
        real_cos_at_100("1/8"),
        typed("setprecision(100); sqrt(2)/2")
    );
}

/// TrigTests.cs:26 Sin(pi/2) == 1.
#[test]
fn sin_given_pi_over_2_is_exactly_one() {
    assert_eq!(real_cos_at_100("0"), "1 (Real)");
}

/// TrigTests.cs:30 Sin(pi) == 0.
#[test]
fn sin_given_pi_is_exactly_zero() {
    assert_eq!(real_cos_at_100("-1/4"), "0 (Real)");
}

/// TrigTests.cs:34 Sin(3*pi/2) == -1.
#[test]
fn sin_given_three_pi_over_2_is_exactly_minus_one() {
    assert_eq!(real_cos_at_100("-1/2"), "-1 (Real)");
}

/// TrigTests.cs:38 Cos(0) == 1.
#[test]
fn cos_given_zero_is_exactly_one() {
    assert_eq!(real_cos_at_100("0"), "1 (Real)");
}

/// TrigTests.cs:42 Cos(pi/3) == 0.5.
#[test]
fn cos_given_pi_over_3_is_exactly_one_half() {
    assert_eq!(real_cos_at_100("1/6"), "0.5 (Real)");
}

/// TrigTests.cs:46 Cos(pi/6) == sqrt(3)/2.
#[test]
fn cos_given_pi_over_6_is_sqrt3_over_2() {
    assert_eq!(
        real_cos_at_100("1/12"),
        typed("setprecision(100); sqrt(3)/2")
    );
}

/// TrigTests.cs:50 Cos(pi) == -1.
#[test]
fn cos_given_pi_is_exactly_minus_one() {
    assert_eq!(real_cos_at_100("1/2"), "-1 (Real)");
}

/// TrigTests.cs:54 Sin(-pi/6) == -Sin(pi/6).
#[test]
fn sin_given_negative_angle_is_negated() {
    assert_eq!(real_cos_at_100("1/3"), "-0.5 (Real)");
}

/// TrigTests.cs:58 Sin(pi/5) starts "0.58778525229247312916".
#[test]
fn sin_given_general_angle_matches_known_value() {
    let value = real_cos_at_100("3/20");
    assert!(value.starts_with("0.58778525229247312916"), "{value}");
}

// ---------------------------------------------------------------------------
// Lovelace.Dsp.Tests/NoFloatingPointTests.cs -- the same source scan over the same directories
// ---------------------------------------------------------------------------

fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn has_word(line: &str, word: &str) -> bool {
    let bytes = line.as_bytes();
    let mut start = 0;
    while let Some(offset) = line[start..].find(word) {
        let at = start + offset;
        let before_ok = at == 0 || !is_ident(bytes[at - 1]);
        let after = at + word.len();
        let after_ok = after >= bytes.len() || !is_ident(bytes[after]);
        if before_ok && after_ok {
            return true;
        }
        start = at + 1;
    }
    false
}

fn scan_sources(
    dir: &std::path::Path,
    mut violation: impl FnMut(&str, usize, &str) -> bool,
) -> Vec<String> {
    fn walk(
        dir: &std::path::Path,
        violation: &mut impl FnMut(&str, usize, &str) -> bool,
        out: &mut Vec<String>,
    ) {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name == "obj" || name == "bin" || name.starts_with('.') {
                    continue;
                }
                walk(&path, violation, out);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("cs") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for (index, line) in text.lines().enumerate() {
                if violation(&path.display().to_string(), index + 1, line) {
                    out.push(format!("{}:{}: {}", path.display(), index + 1, line.trim()));
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, &mut violation, &mut out);
    out
}

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// NoFloatingPointTests.cs:15 Dsp and Complex sources contain no double/float/System.Numerics.Complex.
#[test]
fn dsp_and_complex_sources_contain_no_floating_point() {
    let root = repo_root();
    let mut violations = Vec::new();
    for dir in ["Lovelace.Dsp", "Lovelace.Complex"] {
        violations.extend(scan_sources(&root.join(dir), |_, _, line| {
            has_word(line, "double")
                || has_word(line, "float")
                || line.contains("System.Numerics.Complex")
        }));
    }
    assert!(violations.is_empty(), "{violations:?}");
}

/// NoFloatingPointTests.cs:28 Real source contains no System.Numerics.Complex, Math.Cos/Sin/Exp/Tan
/// or Random.NextDouble.
#[test]
fn real_source_contains_no_ieee_transcendentals_or_complex() {
    let root = repo_root();
    let needles = [
        "System.Numerics.Complex",
        "Math.Cos",
        "Math.Sin",
        "Math.Exp",
        "Math.Tan",
        "Random.NextDouble",
    ];
    let violations = scan_sources(&root.join("Lovelace.Real"), |_, _, line| {
        needles.iter().any(|needle| line.contains(needle))
    });
    assert!(violations.is_empty(), "{violations:?}");
}

// ---------------------------------------------------------------------------
// Lovelace.Suite.Tests/DspIntegerArgumentContractTests.cs -- the wire-observable half of the
// argument contract (the CLR exception subtype is not published; code/category/message is).
// ---------------------------------------------------------------------------

/// DspIntegerArgumentContractTests.cs:36 TheIntegerArgumentsThatWorked_StillWork.
#[test]
fn the_integer_arguments_that_worked_still_work() {
    assert_eq!(
        typed("movingavg([1, 2, 3, 4, 5], 3)"),
        "[0.(3), 1, 2, 3, 4] (Vector)"
    );
    assert_eq!(typed("delay([1, 2, 3, 4], 1)"), "[0, 1, 2, 3] (Vector)");
    assert_eq!(typed("impulse(2)"), "[1, 0] (Vector)");
    assert_eq!(typed("impulse(4)"), "[1, 0, 0, 0] (Vector)");
    assert_eq!(typed("len(step(4))"), "4 (Natural)");
    assert_eq!(typed("dft([0, 1])"), "[1, -1] (Vector)");
}

/// DspIntegerArgumentContractTests.cs:59 ANonIntegralRealCount_IsRefusedAndNamed, four rows.
#[test]
fn non_integral_real_count_is_refused_and_named() {
    let rows = [
        ("movingavg([1, 2, 3, 4, 5], 3.7)", "movingavg", "w", "3.7"),
        ("delay([1, 2, 3, 4], 1.5)", "delay", "k", "1.5"),
        ("impulse(2.5)", "impulse", "n", "2.5"),
        ("step(-0.5)", "step", "n", "-0.5"),
    ];
    for (source, builtin, parameter, literal) in rows {
        let err = error(source);
        assert_eq!(err.code, "InvalidArgument", "{source}");
        assert_eq!(err.category, "TypeMismatch", "{source}");
        assert!(
            err.message.starts_with(&format!("{builtin}():")),
            "{source}: {}",
            err.message
        );
        assert!(
            err.message.contains(&format!("({parameter})")),
            "{source}: {}",
            err.message
        );
        assert!(err.message.contains(literal), "{source}: {}", err.message);
        assert!(
            err.message.contains("exact integer"),
            "{source}: {}",
            err.message
        );
    }
}

/// DspIntegerArgumentContractTests.cs:88 AnExactlyIntegralRealCount_IsTheIntegerItDenotes.
#[test]
fn an_exactly_integral_real_count_is_the_integer_it_denotes() {
    assert_eq!(
        typed("movingavg([1, 2, 3, 4, 5], 3.0)"),
        typed("movingavg([1, 2, 3, 4, 5], 3)")
    );
}

/// DspIntegerArgumentContractTests.cs:119 ACountWiderThanInt64_IsRefusedByBound.
#[test]
fn a_count_wider_than_int64_is_refused_by_bound() {
    let too_wide = "1000000000000000000000000000000";
    let err = error(&format!("movingavg([1, 2, 3], {too_wide})"));
    assert_eq!(err.code, "InvalidArgument");
    assert_eq!(err.category, "TypeMismatch");
    assert!(err.message.contains(too_wide), "{}", err.message);
    assert!(
        err.message.contains("9223372036854775807"),
        "{}",
        err.message
    );
}

/// DspIntegerArgumentContractTests.cs:134 ANegativeSampleCount_IsRefusedAgainstTheNonNegativeBound.
#[test]
fn a_negative_sample_count_is_refused_against_the_nonnegative_bound() {
    let err = error("impulse(-1)");
    assert_eq!(err.code, "InvalidArgument");
    assert_eq!(err.category, "TypeMismatch");
    assert!(err.message.contains("non-negative"), "{}", err.message);
    assert!(err.message.contains("(n)"), "{}", err.message);
    assert!(err.message.contains("-1"), "{}", err.message);
}

/// DspIntegerArgumentContractTests.cs:147 ASeedOutsideTheInt32Range_IsRefusedAgainstItsOwnBound.
#[test]
fn a_seed_outside_the_int32_range_is_refused_against_its_own_bound() {
    let outside = "3000000000";
    let err = error(&format!("noise(1, 0, {outside}, 4)"));
    assert_eq!(err.code, "InvalidArgument");
    assert_eq!(err.category, "TypeMismatch");
    assert!(err.message.contains("(seed)"), "{}", err.message);
    assert!(err.message.contains(outside), "{}", err.message);
    assert!(err.message.contains("-2147483648"), "{}", err.message);
    assert!(err.message.contains("2147483647"), "{}", err.message);
}

/// DspIntegerArgumentContractTests.cs:174 AnIntegerArgumentRefusal_IsAnArgumentException_AndNeverAnInvalidOperation,
/// five rows: the wire pair is the argument family, never the domain family.
#[test]
fn an_integer_argument_refusal_is_an_argument_error_never_an_invalid_operation() {
    let rows = [
        "movingavg([1, 2, 3, 4, 5], 3.7)",
        "movingavg([1, 2, 3], 1000000000000000000000000000000)",
        "delay([1, 2, 3, 4], 1.5)",
        "impulse(-1)",
        "noise(1, 0, 3000000000, 4)",
    ];
    for source in rows {
        let err = error(source);
        assert_eq!(err.code, "InvalidArgument", "{source}");
        assert_eq!(err.category, "TypeMismatch", "{source}");
        assert_ne!(err.code, "InvalidOperation", "{source}");
        assert!(
            err.message.starts_with("movingavg(")
                || err.message.starts_with("delay(")
                || err.message.starts_with("impulse(")
                || err.message.starts_with("noise("),
            "{source}: {}",
            err.message
        );
    }
}

/// DspIntegerArgumentContractTests.cs:196 TheReader_AcceptsEveryPayloadTheCoreCanSend. The reader
/// itself is private; the same accepted/rejected payloads are driven through the call sites that
/// consume it, which is where the contract is observable.
#[test]
fn the_reader_accepts_every_payload_the_core_can_send() {
    assert_eq!(typed("impulse(3)"), "[1, 0, 0] (Vector)");
    assert_eq!(typed("delay([1], -7)"), "[0] (Vector)");
    assert_eq!(typed("impulse(2.0)"), "[1, 0] (Vector)");
    assert_eq!(typed("delay([1], 9223372036854775807)"), "[0] (Vector)");
    assert_eq!(typed("delay([1], -9223372036854775808)"), "[0] (Vector)");
    assert_eq!(error("impulse(3.7)").code, "InvalidArgument");
    assert_eq!(
        error("impulse(1000000000000000000000000000000)").code,
        "InvalidArgument"
    );
    assert_eq!(typed("impulse(0.0)"), "[] (Vector)");
}

// ---------------------------------------------------------------------------
// Lovelace.Suite.Tests/DescriptorExampleTests.cs -- the eight descriptor examples for the new
// builtins. The C# assertion is "the snippet executes on a fresh engine primed with the documented
// prelude"; each row below is that same snippet.
// ---------------------------------------------------------------------------

fn assert_descriptor_example_executes(source: &str) {
    let mut e = engine();
    e.eval("x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\");")
        .unwrap_or_else(|error| panic!("descriptor prelude: {}", error.message));
    e.eval(source)
        .unwrap_or_else(|error| panic!("descriptor example {source}: {}", error.message));
}

#[test]
fn descriptor_example_step_executes() {
    assert_descriptor_example_executes("step(4)");
}

#[test]
fn descriptor_example_cosine_executes() {
    assert_descriptor_example_executes("cosine(0.25, 0, 8)");
}

#[test]
fn descriptor_example_delay_executes() {
    assert_descriptor_example_executes("delay([1, 2, 3], 1)");
}

#[test]
fn descriptor_example_exponential_executes() {
    assert_descriptor_example_executes("exponential(0.5, 4)");
}

#[test]
fn descriptor_example_impulse_executes() {
    assert_descriptor_example_executes("impulse(4)");
}

#[test]
fn descriptor_example_movingavg_executes() {
    assert_descriptor_example_executes("movingavg([1, 2, 3], 2)");
}

#[test]
fn descriptor_example_noise_executes() {
    assert_descriptor_example_executes("noise(1, 0, 4, 3)");
}

#[test]
fn descriptor_example_powerseries_executes() {
    assert_descriptor_example_executes("powerseries(2, 0.5, 4)");
}

// ---------------------------------------------------------------------------
// Lovelace.Run.Tests/ArityPropertyTests.cs -- the two documented noise forms
// ---------------------------------------------------------------------------

/// ArityPropertyTests.cs:207-208 noise(1, 0, 4) and noise(1, 0, 4, 3) stay callable.
#[test]
fn noise_short_and_long_forms_stay_callable() {
    assert_eq!(typed("noise(1, 0, 4)"), typed("noise(1, 0, 0, 4)"));
    assert_eq!(
        element_field("noise(1, 0, 4, 3)", 0, "re"),
        element_field("noise(1, 0, 4, 3)", 0, "re")
    );
}
