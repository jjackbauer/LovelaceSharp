//! KNOW-008 / KNOW-010: the seeded reproduction of the frozen C# graph (seed 20240617).
//!
//! The reference file is knowledge-graph-demo.json at the repository root: the graph the C#
//! tool produced for the default configuration, driven by the C# runner. This test runs the
//! real Rust pipeline against the real Rust runner and compares the two graphs section by
//! section.
//!
//! The knowledge pipeline reproduces the reference exactly. Five of the 314 samples observe a
//! different runner result, because the two engines disagree on three expression semantics;
//! every one of those disagreements is listed and pinned below, so a change in the runner
//! makes this test fail loudly instead of silently drifting.

mod common;

use lovelace_know::converge;
use lovelace_know::model::{Graph, KnowledgeConfig};
use lovelace_know::runner::ProcessRunner;

fn load_reference() -> Graph {
    let text = std::fs::read_to_string(common::reference_graph_path())
        .expect("the frozen reference is present");
    let document = lovelace_json::Json::parse(&text).expect("the reference parses");
    Graph::from_json(&document).expect("the reference deserializes")
}

#[test]
fn reproduces_the_frozen_seeded_graph() {
    let config = KnowledgeConfig::default_config();
    assert_eq!(config.seed, 20_240_617, "the frozen seed");
    assert_eq!(config.batch_size, 64);
    assert_eq!(config.max_samples, 700);

    let runner = ProcessRunner::new(common::runner_path());
    let produced = converge::run(&config, &runner, None);
    let reference = load_reference();

    assert_eq!(produced.seed, reference.seed);
    assert_eq!(produced.version, reference.version);
    assert_eq!(produced.config.seed, reference.config.seed);
    assert_eq!(produced.config.batch_size, reference.config.batch_size);
    assert_eq!(produced.config.operations, reference.config.operations);
    assert_eq!(
        produced.config.sweep_operations,
        reference.config.sweep_operations
    );

    // ---------------------------------------------------------------------
    // 1. The sample stream: identical coordinates, order, provenance, weights.
    // ---------------------------------------------------------------------
    assert_eq!(
        produced.samples.len(),
        314,
        "the reference converged after 314 samples"
    );
    assert_eq!(produced.samples.len(), reference.samples.len());
    for (actual, expected) in produced.samples.iter().zip(reference.samples.iter()) {
        assert_eq!(actual.index, expected.index);
        assert_eq!(actual.script, expected.script);
        assert_eq!(actual.op, expected.op);
        assert_eq!(actual.left.literal, expected.left.literal);
        assert_eq!(actual.left.domain, expected.left.domain);
        assert_eq!(actual.right.literal, expected.right.literal);
        assert_eq!(actual.right.domain, expected.right.domain);
        assert_eq!(actual.sweep_id, expected.sweep_id);
        assert_eq!(actual.swept_side, expected.swept_side);
        assert_eq!(actual.axis_pos, expected.axis_pos);
        assert_eq!(actual.sampling_kind, expected.sampling_kind);
        assert_eq!(actual.weight, expected.weight);
    }

    // ---------------------------------------------------------------------
    // 2. The five runner-level observation divergences (not pipeline divergences).
    // ---------------------------------------------------------------------
    let divergences: Vec<i32> = produced
        .samples
        .iter()
        .zip(reference.samples.iter())
        .filter(|(actual, expected)| {
            actual.sigma != expected.sigma || actual.typed != expected.typed
        })
        .map(|(actual, _)| actual.index)
        .collect();
    assert_eq!(
        divergences,
        vec![164, 177, 207, 219, 231],
        "the documented runner-level divergences moved"
    );

    // 164: the C# engine binds unary minus tighter than ^, so it evaluates (-3)^0 and its
    // Integer.Pow rejects a non-positive exponent; the Rust engine evaluates -(3^0) = -1.
    // This is the only divergence that changes a plane.
    let sample = &produced.samples[164];
    assert_eq!(sample.script, "-3 ^ 0");
    assert_eq!(sample.sigma, "Integer");
    assert_eq!(sample.typed.as_deref(), Some("-1 (Integer)"));
    assert_eq!(
        reference.samples[164].sigma,
        "err|Exponent must be positive. (Parameter 'exponent')"
    );

    // 177 and 219: the same precedence difference on a negative base with an even exponent.
    assert_eq!(produced.samples[177].script, "-0.5 ^ 10");
    assert_eq!(
        produced.samples[177].typed.as_deref(),
        Some("-0.0009765625 (Real)")
    );
    assert_eq!(
        reference.samples[177].typed.as_deref(),
        Some("0.0009765625 (Real)")
    );
    assert_eq!(produced.samples[219].script, "-2 ^ 4");
    assert_eq!(
        produced.samples[219].typed.as_deref(),
        Some("-16 (Integer)")
    );
    assert_eq!(
        reference.samples[219].typed.as_deref(),
        Some("16 (Integer)")
    );

    // 207: same value (1/12), different typed rendering: C# expands the terminating decimal,
    // Rust prints the periodic literal. Both observations are the Real plane.
    assert_eq!(produced.samples[207].script, "0.(3) * 0.25");
    assert_eq!(
        produced.samples[207].typed.as_deref(),
        Some("0.08(3) (Real)")
    );
    assert!(
        reference.samples[207]
            .typed
            .as_deref()
            .expect("typed")
            .starts_with("0.08333333333333333")
    );

    // 231: the two engines disagree on the value of a real modulo: the Rust runner reports
    // the mathematically exact 10 % 0.5 == 0, the frozen reference reports 9.
    assert_eq!(produced.samples[231].script, "10 % 0.5");
    assert_eq!(produced.samples[231].typed.as_deref(), Some("0 (Real)"));
    assert_eq!(reference.samples[231].typed.as_deref(), Some("9 (Real)"));

    // ---------------------------------------------------------------------
    // 3. The reduced structure: identical wherever the observations agree.
    // ---------------------------------------------------------------------
    assert_eq!(produced.boundaries.len(), 18);
    assert_eq!(produced.boundaries.len(), reference.boundaries.len());
    for (actual, expected) in produced.boundaries.iter().zip(reference.boundaries.iter()) {
        assert_eq!(
            common::round_trip(&actual.to_json()),
            common::round_trip(&expected.to_json()),
            "boundary {}",
            actual.id
        );
    }

    // The reference carries one extra frontier: the low-support frontier for the error plane
    // that only the C# runner produces.
    let expected_frontiers: Vec<_> = reference
        .frontiers
        .iter()
        .filter(|frontier| frontier.kind != "low-support")
        .collect();
    assert_eq!(produced.frontiers.len(), expected_frontiers.len());
    assert_eq!(reference.frontiers.len(), produced.frontiers.len() + 1);
    for (actual, expected) in produced.frontiers.iter().zip(expected_frontiers) {
        assert_eq!(
            common::round_trip(&actual.to_json()),
            common::round_trip(&expected.to_json()),
            "frontier {}",
            actual.id
        );
    }

    // Planes: identical except the C#-only error plane, and the Integer plane, which has one
    // more member here (sample 164).
    assert_eq!(produced.planes.len(), reference.planes.len() - 1);
    for plane in &produced.planes {
        let expected = reference
            .planes
            .iter()
            .find(|candidate| candidate.sigma == plane.sigma)
            .unwrap_or_else(|| panic!("the reference has plane {}", plane.sigma));
        if plane.sigma == "Integer" {
            assert_eq!(plane.support, expected.support + 1);
            let mut expected_indices = expected.sample_indices.clone();
            let position = expected_indices
                .iter()
                .position(|index| *index > 164)
                .unwrap_or(expected_indices.len());
            expected_indices.insert(position, 164);
            assert_eq!(plane.sample_indices, expected_indices);
        } else {
            assert_eq!(
                common::round_trip(&plane.to_json()),
                common::round_trip(&expected.to_json()),
                "plane {}",
                plane.sigma
            );
        }
    }
    assert!(
        !produced
            .planes
            .iter()
            .any(|plane| plane.sigma.starts_with("err|Exponent"))
    );

    // ---------------------------------------------------------------------
    // 4. The convergence metrics.
    // ---------------------------------------------------------------------
    let metrics = produced
        .metrics
        .as_ref()
        .expect("the loop measured the graph");
    let expected = reference
        .metrics
        .as_ref()
        .expect("the reference measured it");
    assert_eq!(metrics.converged, expected.converged);
    assert!(metrics.converged);
    assert_eq!(metrics.stop_reason, expected.stop_reason);
    assert_eq!(metrics.total_samples, expected.total_samples);
    assert_eq!(metrics.c1_new_planes_last_k, expected.c1_new_planes_last_k);
    assert_eq!(metrics.c1_new_plane_rate, expected.c1_new_plane_rate);
    assert_eq!(metrics.c1_saturated, expected.c1_saturated);
    assert_eq!(metrics.c2_total_boundaries, expected.c2_total_boundaries);
    assert_eq!(metrics.c2_stable_boundaries, expected.c2_stable_boundaries);
    assert_eq!(metrics.c2_stable, expected.c2_stable);
    assert_eq!(metrics.c3_held_out_count, expected.c3_held_out_count);
    assert_eq!(metrics.c3_agreed_count, expected.c3_agreed_count);
    assert_eq!(metrics.c3_agreement, expected.c3_agreement);
    assert_eq!(metrics.c3_agreed, expected.c3_agreed);
    assert_eq!(metrics.c4_covered, expected.c4_covered);
    // The only metric that follows the plane count: 7 here, 8 in the reference.
    assert_eq!(metrics.plane_count, expected.plane_count - 1);
    assert_eq!(metrics.coverage.len(), expected.coverage.len());
    for (actual, reference_cell) in metrics.coverage.iter().zip(expected.coverage.iter()) {
        assert_eq!(
            common::round_trip(&actual.to_json()),
            common::round_trip(&reference_cell.to_json()),
            "coverage cell {}",
            actual.key
        );
    }
}
