//! Round-61 DSP family: the seven round-50 census rows, each pinned to the pinned C# oracle's
//! observable behaviour.
//!
//! The family was measured as "runs fine but diverges" in
//! docs/goal-parity-contract/round-50/resweep.md. The four `filter` rows were already answered
//! correctly by the round-51 dispatcher arms; the three measured divergences were the empty FFT,
//! the non-power-of-two FFT, and the truncated Complex widening refusal. Each test below names the
//! census row it pins and asserts the oracle-observed field/value, never a Rust-specific shape.

use crate::{Engine, LangError, Value};

fn eval(source: &str) -> Result<Value, LangError> {
    Engine::new().eval(source)
}

fn typed(source: &str) -> String {
    eval(source)
        .unwrap_or_else(|error| panic!("{source}: {}", error.message))
        .typed()
}

fn error(source: &str) -> LangError {
    eval(source).expect_err(&format!("{source} must be refused"))
}

/// `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.DspTests/Filter_GivenFir_MatchesConvolution#0`
///
/// DspTests.cs:140 filters a=[1], b=[1,2,3], x=[1,1,1,1] and asserts every filtered sample equals
/// the matching prefix sample of Convolve(x, b). The oracle answers `[1, 3, 6, 6]`.
#[test]
fn filter_fir_signal_matches_the_convolution_prefix() {
    assert_eq!(
        typed("filter([1], [1, 2, 3], [1, 1, 1, 1])"),
        "[1, 3, 6, 6] (Vector)"
    );
    for i in 0..4 {
        assert_eq!(
            typed(&format!("filter([1], [1, 2, 3], [1, 1, 1, 1])[{i}]")),
            typed(&format!("conv([1, 1, 1, 1], [1, 2, 3])[{i}]")),
            "filtered sample {i} must equal the convolution prefix"
        );
    }
}

/// `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.DspTests/Filter_GivenIir_MatchesImpulseResponseConvolution#0`
///
/// DspTests.cs:156 filters a=[1,-0.5], b=[1], x=[1,1,1,1] and asserts the prefix equality against
/// Convolve(x, ImpulseResponse(a, b, len)). The oracle answers `[1, 1.5, 1.75, 1.875]`.
#[test]
fn filter_iir_signal_matches_the_impulse_response_convolution() {
    assert_eq!(
        typed("filter([1, -0.5], [1], [1, 1, 1, 1])"),
        "[1, 1.5, 1.75, 1.875] (Vector)"
    );
    for i in 0..4 {
        assert_eq!(
            typed(&format!("filter([1, -0.5], [1], [1, 1, 1, 1])[{i}]")),
            typed(&format!(
                "conv([1, 1, 1, 1], filter([1, -0.5], [1], 4))[{i}]"
            )),
            "filtered sample {i} must equal the impulse-response convolution prefix"
        );
    }
}

/// `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/Filter_GivenLength_ReturnsImpulseResponse#0`
///
/// DspPluginTests.cs:145 asserts the typed result of `filter([1, -0.5], [1], 4)` is
/// `[1, 0.5, 0.25, 0.125] (Vector)`; the oracle publishes exactly that.
#[test]
fn filter_scalar_length_returns_the_impulse_response() {
    assert_eq!(
        typed("filter([1, -0.5], [1], 4)"),
        "[1, 0.5, 0.25, 0.125] (Vector)"
    );
}

/// `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/Filter_GivenSignal_MatchesConvolution#0`
///
/// DspPluginTests.cs:117 compares the direct `filter([1, -0.5], [1], [1, 1, 1, 1])` prefix with
/// `conv([1, 1, 1, 1], filter([1, -0.5], [1], 4))`. The oracle answers the same prefix
/// `[1, 1.5, 1.75, 1.875]`.
#[test]
fn filter_signal_matches_the_convolution_with_its_impulse_response() {
    assert_eq!(
        typed("filter([1, -0.5], [1], [1, 1, 1, 1])"),
        "[1, 1.5, 1.75, 1.875] (Vector)"
    );
    for i in 0..4 {
        for projection in ["re", "im"] {
            assert_eq!(
                typed(&format!(
                    "{projection}(filter([1, -0.5], [1], [1, 1, 1, 1])[{i}])"
                )),
                typed(&format!(
                    "{projection}(conv([1, 1, 1, 1], filter([1, -0.5], [1], 4))[{i}])"
                )),
                "direct filter element {i} ({projection}) must equal the convolution prefix"
            );
        }
    }
}

/// `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FftTests/Fft_GivenEmpty_ReturnsEmpty#0`
///
/// FftTests.cs:82 asserts `DspMath.Fft([])` is empty. The oracle publishes `[] (Vector)`; the
/// Rust path used to refuse the empty vector through the generic DFT.
#[test]
fn fft_of_an_empty_signal_is_the_empty_transform() {
    assert_eq!(typed("fft([])"), "[] (Vector)");
    // DspMath.Dft has the same N == 0 early return, so the sibling surface is pinned too.
    assert_eq!(typed("dft([])"), "[] (Vector)");
}

/// `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FftTests/Fft_GivenNonPowerOfTwo_ThrowsArgumentException#0`
///
/// FftTests.cs:75 asserts `DspMath.Fft([1,1,1])` throws ArgumentException. The oracle publishes
/// InvalidArgument / TypeMismatch with this exact sentence; Rust used to answer `[3, 0, 0]`.
#[test]
fn fft_of_a_non_power_of_two_length_is_a_typed_refusal() {
    let failure = error("fft([1, 1, 1])");
    assert_eq!(failure.code, "InvalidArgument");
    assert_eq!(failure.category, "TypeMismatch");
    assert_eq!(failure.position, 0);
    assert_eq!(
        failure.message,
        "FFT length must be a power of two, but got 3. (Parameter 'x')"
    );
}

/// `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/ComplexArithmetic_GivenComplex_ThrowsActionableError#0`
///
/// DspPluginTests.cs:82 asserts the Complex widening refusal names `re()/im()/conj()/abs()`. The
/// oracle publishes InvalidOperation / DomainError with the full Value.Widen hint; Rust used to
/// stop after "... support widening.".
#[test]
fn complex_arithmetic_refusal_names_the_bridge_builtins() {
    let failure = error("fft([0, 1, 0, 0])[1] + 1");
    assert_eq!(failure.code, "InvalidOperation");
    assert_eq!(failure.category, "DomainError");
    assert_eq!(failure.position, 0);
    assert!(
        failure.message.contains("re()/im()/conj()/abs()"),
        "the refusal must name the bridge builtins: {}",
        failure.message
    );
    assert_eq!(
        failure.message,
        "Cannot widen from Natural to Complex: only numeric kinds (Natural, Integer, Real) support widening. Reductions over Complex arrays (sum/mean/dot/norm/matmul) are not supported; use re()/im()/conj()/abs() to bridge back to Real."
    );
}
