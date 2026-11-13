//! Stage 4 tests: plane clustering, boundary detection, guard fitting, and frontiers.
//!
//! Mirrors Lovelace.Knowledge.Tests/ReducerTests.cs.

mod common;

use lovelace_know::model::{Confidence, KnowledgeConfig, Operation, SampleRecord, SamplingKind};
use lovelace_know::reducer;

#[test]
fn cluster_same_sigma_shares_one_plane() {
    let config = KnowledgeConfig::default_config();
    let samples = vec![
        common::sweep_sample(
            0,
            Operation::Subtract,
            "0",
            "0",
            "right",
            "s:0:h2",
            "Natural",
        ),
        common::sweep_sample(
            1,
            Operation::Subtract,
            "0",
            "2",
            "right",
            "s:0:h2",
            "Natural",
        ),
    ];
    let reduction = reducer::reduce(&samples, &config);
    assert_eq!(reduction.planes.len(), 1);
    assert_eq!(reduction.planes[0].sigma, "Natural");
    assert_eq!(reduction.planes[0].support, 2);
    assert_eq!(reduction.planes[0].confidence, Confidence::Repeated);
    assert_eq!(reduction.planes[0].sample_indices, vec![0, 1]);
}

#[test]
fn detect_natural_subtraction_underflow_threshold_guard() {
    let config = KnowledgeConfig::default_config();
    let samples = vec![
        common::sweep_sample(
            0,
            Operation::Subtract,
            "0",
            "0",
            "right",
            "subtract:0:right:h2",
            "Natural",
        ),
        common::sweep_sample(
            1,
            Operation::Subtract,
            "0",
            "1",
            "right",
            "subtract:0:right:h2",
            "Integer",
        ),
        common::sweep_sample(
            2,
            Operation::Subtract,
            "0",
            "0",
            "right",
            "subtract:0:right:h3",
            "Natural",
        ),
        common::sweep_sample(
            3,
            Operation::Subtract,
            "0",
            "1",
            "right",
            "subtract:0:right:h3",
            "Integer",
        ),
    ];
    let reduction = reducer::reduce(&samples, &config);
    assert_eq!(reduction.boundaries.len(), 1);
    let boundary = &reduction.boundaries[0];
    assert_eq!(boundary.from_plane, "Natural");
    assert_eq!(boundary.to_plane, "Integer");
    assert_eq!(boundary.operation, "Subtract");
    assert_eq!(boundary.guard.kind, "threshold");
    assert_eq!(boundary.guard.relop, ">");
    assert_eq!(boundary.guard.threshold, "0");
    assert_eq!(boundary.guard.expression, "right > left");
    assert_eq!(boundary.confidence, Confidence::Conformant);
    // The two grids interleave on the same axis, so the differing adjacent pair is observed
    // once (0 -> 1) and the step-size tags from both grids count as two confirmations.
    assert_eq!(boundary.evidence.len(), 2);
    assert_eq!(boundary.evidence[0].side, "high");
    assert_eq!(boundary.evidence[0].axis_pos, "1");
    assert_eq!(boundary.evidence[1].side, "low");
    assert_eq!(boundary.evidence[1].axis_pos, "0");
    assert_eq!(boundary.id.len(), 16);
    assert!(
        reduction
            .frontiers
            .iter()
            .all(|frontier| frontier.kind != "unresolved-boundary")
    );
}

#[test]
fn detect_division_by_zero_equality_guard() {
    let config = KnowledgeConfig::default_config();
    let samples = vec![
        common::sweep_sample(
            0,
            Operation::Divide,
            "1",
            "0",
            "right",
            "divide:1:right:h2",
            "err|Cannot divide by zero.",
        ),
        common::sweep_sample(
            1,
            Operation::Divide,
            "1",
            "1",
            "right",
            "divide:1:right:h2",
            "Natural",
        ),
        common::sweep_sample(
            2,
            Operation::Divide,
            "1",
            "0",
            "right",
            "divide:1:right:h3",
            "err|Cannot divide by zero.",
        ),
        common::sweep_sample(
            3,
            Operation::Divide,
            "1",
            "1",
            "right",
            "divide:1:right:h3",
            "Natural",
        ),
    ];
    let reduction = reducer::reduce(&samples, &config);
    assert_eq!(reduction.boundaries.len(), 1);
    let boundary = &reduction.boundaries[0];
    assert_eq!(boundary.from_plane, "err|Cannot divide by zero.");
    assert_eq!(boundary.guard.kind, "equality");
    assert_eq!(boundary.guard.relop, "==");
    assert_eq!(boundary.guard.threshold, "0");
    assert_eq!(boundary.guard.expression, "right == 0");
}

#[test]
fn detect_unresolved_boundary_marks_frontier() {
    let config = KnowledgeConfig::default_config();
    let samples = vec![
        common::sweep_sample(
            0,
            Operation::Subtract,
            "0",
            "0",
            "right",
            "subtract:0:right:h2",
            "Natural",
        ),
        common::sweep_sample(
            1,
            Operation::Subtract,
            "0",
            "4",
            "right",
            "subtract:0:right:h2",
            "Integer",
        ),
    ];
    let reduction = reducer::reduce(&samples, &config);
    assert!(reduction.boundaries.is_empty());
    let unresolved: Vec<_> = reduction
        .frontiers
        .iter()
        .filter(|frontier| frontier.kind == "unresolved-boundary")
        .collect();
    assert_eq!(unresolved.len(), 1);
    assert_eq!(unresolved[0].operation.as_deref(), Some("Subtract"));
    assert_eq!(unresolved[0].anchor.as_deref(), Some("0"));
    assert_eq!(unresolved[0].low.as_deref(), Some("0"));
    assert_eq!(unresolved[0].high.as_deref(), Some("4"));
    assert!(unresolved[0].description.contains("needs bisection"));
}

#[test]
fn low_support_plane_becomes_a_frontier() {
    let config = KnowledgeConfig::default_config();
    let samples = vec![common::sweep_sample(
        0,
        Operation::Power,
        "-3",
        "0",
        "right",
        "power:-3:right:h2",
        "err|Exponent must be positive. (Parameter 'exponent')",
    )];
    let reduction = reducer::reduce(&samples, &config);
    let low_support: Vec<_> = reduction
        .frontiers
        .iter()
        .filter(|frontier| frontier.kind == "low-support")
        .collect();
    assert_eq!(low_support.len(), 1);
    assert!(low_support[0].description.contains("has support 1 (< 2)"));
    assert_eq!(low_support[0].id.len(), 16);
}

#[test]
fn unsampled_arithmetic_cells_become_weak_dimensions() {
    let config = KnowledgeConfig::default_config();
    let samples: Vec<SampleRecord> = Vec::new();
    let reduction = reducer::reduce(&samples, &config);
    let weak: Vec<_> = reduction
        .frontiers
        .iter()
        .filter(|frontier| frontier.kind == "weak-dimension")
        .collect();
    assert_eq!(
        weak.len(),
        54,
        "six arithmetic operations over nine domain pairs"
    );
    let first = &weak[0];
    assert_eq!(first.operation.as_deref(), Some("Add"));
    assert_eq!(first.description, "unsampled cell + over Natural x Natural");
}

#[test]
fn stable_ids_are_deterministic_and_distinct() {
    let first = reducer::stable_id("boundary:Subtract|0|right|Natural|Integer");
    let second = reducer::stable_id("boundary:Subtract|0|right|Natural|Integer");
    let other = reducer::stable_id("boundary:Subtract|1|right|Natural|Integer");
    assert_eq!(first, second);
    assert_ne!(first, other);
    assert_eq!(first.len(), 16);
    assert!(first.chars().all(|character| character.is_ascii_hexdigit()));
}

#[test]
fn validation_samples_are_excluded_from_fitting() {
    let config = KnowledgeConfig::default_config();
    let mut samples = vec![
        common::sweep_sample(
            0,
            Operation::Subtract,
            "0",
            "0",
            "right",
            "subtract:0:right:h2",
            "Natural",
        ),
        common::sweep_sample(
            1,
            Operation::Subtract,
            "0",
            "1",
            "right",
            "subtract:0:right:h2",
            "Integer",
        ),
    ];
    let mut validate =
        common::validation_sample(2, Operation::Subtract, "0", "4", "right", "Integer");
    validate.sampling_kind = SamplingKind::Validate;
    samples.push(validate);
    let reduction = reducer::reduce(&samples, &config);
    assert_eq!(reduction.boundaries.len(), 1);
    assert_eq!(reduction.boundaries[0].evidence.len(), 2);
}
