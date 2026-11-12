//! Round 95 numeric singletons: the two Suite-level rows whose surface is the language engine.
//!
//! One test per census row, asserting the oracle-observed observable.

use crate::{Engine, Value};

fn eval(script: &str) -> Value {
    let mut engine = Engine::new();
    engine
        .eval(script)
        .unwrap_or_else(|error| panic!("script: {script} failed: {}", error.message))
}

/// C# NdArrayTests.MatMul_Batched: ArrayMath.MatMul on two rank-3 arrays multiplies the matching
/// 2-D slices. The oracle publishes [[[7, 10], [15, 22]], [[67, 78], [91, 106]]].
#[test]
fn matmul_batched_multiplies_each_three_dimensional_slice() {
    let value =
        eval("matmul([[[1, 2], [3, 4]], [[5, 6], [7, 8]]], [[[1, 2], [3, 4]], [[5, 6], [7, 8]]])");
    assert_eq!(
        "[[[7, 10], [15, 22]], [[67, 78], [91, 106]]]",
        value.display()
    );
}

/// C# ArrayAllocationRefusalTests.TheSuiteDeclaresTheAllocationRefusalTheRunnerClassifies: the suite
/// declares the refusal type and the runner classifies it. The Rust surface declares the refusal as
/// the published AllocationRefused/BudgetExceeded code pair rather than a CLR exception type.
#[test]
fn suite_declares_the_allocation_refusal_the_runner_classifies() {
    let error = match Engine::new().eval("zeros(1000000000)") {
        Ok(value) => panic!("zeros(1000000000) answered {}", value.display()),
        Err(error) => error,
    };
    assert_eq!("AllocationRefused", error.code);
    assert_eq!("BudgetExceeded", error.category);
    assert!(error.message.contains("zeros"), "{}", error.message);
}
