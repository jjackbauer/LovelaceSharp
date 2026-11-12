//! Round 66: the deferred precbench row the Real precision-scope surface now answers.

use crate::Real;

/// precbench.Tests/AccuracyTests.cs:76 WithPrecision_GivenScope_RestoresGlobalState: a
/// WithPrecision(7, ...) scope computes inside it and, once it ends, the global computation and
/// display decimal places are the ones that were active before the scope. The Rust surface is the
/// same RAII scope (Real::with_precision) with the same two published values.
#[test]
fn with_precision_scope_restores_the_global_state() {
    let saved_max = Real::max_computation_decimal_places();
    let saved_display = Real::display_decimal_places();

    {
        let _scope = Real::with_precision(7, 7);
        // the same computation the C# row performs inside the scope
        let two = Real::from_str("2").expect("2 is a decimal value");
        let _ = two.sqrt(7);
    }

    assert_eq!(
        Real::max_computation_decimal_places(),
        saved_max,
        "the computation cap must be restored when the scope ends"
    );
    assert_eq!(
        Real::display_decimal_places(),
        saved_display,
        "the display cap must be restored when the scope ends"
    );
}
