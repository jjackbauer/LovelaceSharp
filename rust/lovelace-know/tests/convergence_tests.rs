//! Stage 5 tests: C1-C4 metrics, and full reduction/metric parity on the frozen reference.
//!
//! The reference graph stores the C# samples and the C# reduction of those samples, so
//! re-reducing them here tests the port without involving either runner.

mod common;

use lovelace_know::convergence;
use lovelace_know::model::{Graph, KnowledgeConfig, Operation};
use lovelace_know::reducer;

fn load_reference() -> (lovelace_json::Json, Graph) {
    let text = std::fs::read_to_string(common::reference_graph_path())
        .expect("the frozen reference is present");
    let document = lovelace_json::Json::parse(&text).expect("the reference parses");
    let graph = Graph::from_json(&document).expect("the reference deserializes");
    (document, graph)
}

fn array<'a>(document: &'a lovelace_json::Json, key: &str) -> &'a [lovelace_json::Json] {
    document
        .get(key)
        .and_then(lovelace_json::Json::as_arr)
        .unwrap_or_else(|| panic!("{key} is an array"))
}

#[test]
fn frozen_reference_reduces_to_the_stored_structure() {
    let (document, graph) = load_reference();
    let reduction = reducer::reduce(&graph.samples, &graph.config);

    let planes = array(&document, "planes");
    assert_eq!(reduction.planes.len(), planes.len(), "plane count");
    for (actual, expected) in reduction.planes.iter().zip(planes) {
        assert_eq!(
            common::round_trip(&actual.to_json()),
            expected.clone(),
            "plane {}",
            actual.sigma
        );
    }

    let boundaries = array(&document, "boundaries");
    assert_eq!(
        reduction.boundaries.len(),
        boundaries.len(),
        "boundary count"
    );
    for (actual, expected) in reduction.boundaries.iter().zip(boundaries) {
        assert_eq!(
            common::round_trip(&actual.to_json()),
            expected.clone(),
            "boundary {}",
            actual.id
        );
    }

    let frontiers = array(&document, "frontiers");
    assert_eq!(reduction.frontiers.len(), frontiers.len(), "frontier count");
    for (actual, expected) in reduction.frontiers.iter().zip(frontiers) {
        assert_eq!(
            common::round_trip(&actual.to_json()),
            expected.clone(),
            "frontier {}",
            actual.id
        );
    }
}

#[test]
fn frozen_reference_metrics_match() {
    let (document, graph) = load_reference();
    let reduction = reducer::reduce(&graph.samples, &graph.config);

    // The run's batch structure, read off the reference's sampling kinds: four full breadth
    // batches (156 sweeps + 100 random draws), the refinement batch (13 bisections plus the
    // four validation probes it had room for), then the remaining 41 validation probes.
    let sweeps = graph
        .samples
        .iter()
        .filter(|sample| sample.sampling_kind == lovelace_know::model::SamplingKind::Sweep)
        .count();
    let random = graph
        .samples
        .iter()
        .filter(|sample| sample.sampling_kind == lovelace_know::model::SamplingKind::Random)
        .count();
    let refine = graph
        .samples
        .iter()
        .filter(|sample| sample.sampling_kind == lovelace_know::model::SamplingKind::Refine)
        .count();
    let validate = graph
        .samples
        .iter()
        .filter(|sample| sample.sampling_kind == lovelace_know::model::SamplingKind::Validate)
        .count();
    assert_eq!((sweeps, random, refine, validate), (156, 100, 13, 45));

    let batch_sizes = vec![64, 64, 64, 64, 17, 41];
    let total: i32 = batch_sizes.iter().sum();
    assert_eq!(usize::try_from(total).expect("small"), graph.samples.len());
    let new_planes = new_planes_per_batch(&graph.samples, &batch_sizes);
    assert_eq!(
        new_planes.iter().sum::<i32>(),
        8,
        "all eight planes are discovered"
    );
    let metrics = convergence::measure(
        &reduction,
        &graph.samples,
        &graph.config,
        &new_planes,
        &batch_sizes,
    );
    assert!(metrics.converged);
    assert_eq!(metrics.c1_new_planes_last_k, 0);
    assert_eq!(metrics.c1_new_plane_rate, 0.0);
    let expected = document.get("metrics").expect("metrics are stored").clone();
    let actual = common::round_trip(&metrics.to_json());
    if let (lovelace_json::Json::Obj(expected), lovelace_json::Json::Obj(actual)) =
        (&expected, &actual)
    {
        for (key, value) in expected {
            assert_eq!(actual.get(key), Some(value), "metric {key}");
        }
        assert_eq!(actual.len(), expected.len(), "no extra metrics");
    } else {
        panic!("the metrics are objects");
    }
}

fn new_planes_per_batch(
    samples: &[lovelace_know::model::SampleRecord],
    batch_sizes: &[i32],
) -> Vec<i32> {
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    let mut new_planes = Vec::new();
    let mut start = 0usize;
    for size in batch_sizes {
        let end = start + usize::try_from(*size).expect("a small batch");
        let mut fresh = 0;
        for sample in &samples[start..end] {
            if seen.insert(sample.sigma.as_str()) {
                fresh += 1;
            }
        }
        new_planes.push(fresh);
        start = end;
    }
    new_planes
}

#[test]
fn c1_window_uses_the_last_k_batches() {
    let config = KnowledgeConfig::default_config();
    let samples = Vec::new();
    let reduction = reducer::reduce(&samples, &config);
    let saturated = convergence::measure(
        &reduction,
        &samples,
        &config,
        &[5, 0, 0, 0],
        &[64, 64, 64, 64],
    );
    assert_eq!(saturated.c1_new_planes_last_k, 0);
    assert!(saturated.c1_saturated);
    let growing = convergence::measure(
        &reduction,
        &samples,
        &config,
        &[5, 5, 5, 5],
        &[64, 64, 64, 64],
    );
    assert_eq!(growing.c1_new_planes_last_k, 15);
    assert!(!growing.c1_saturated);
}

#[test]
fn unresolved_frontier_blocks_c2() {
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
            "4",
            "right",
            "s:0:h2",
            "Integer",
        ),
    ];
    let reduction = reducer::reduce(&samples, &config);
    let metrics = convergence::measure(&reduction, &samples, &config, &[], &[]);
    assert!(!metrics.c2_stable);
    assert_eq!(metrics.c2_total_boundaries, 0);
    assert!(
        metrics
            .stop_reason
            .expect("reason")
            .contains("C2 boundary stability")
    );
}

#[test]
fn c3_counts_only_matched_probes_and_their_agreement() {
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
    samples.push(common::validation_sample(
        2,
        Operation::Subtract,
        "0",
        "0",
        "right",
        "Natural",
    ));
    samples.push(common::validation_sample(
        3,
        Operation::Subtract,
        "0",
        "1",
        "right",
        "Integer",
    ));
    samples.push(common::validation_sample(
        4,
        Operation::Subtract,
        "0",
        "2",
        "right",
        "Natural",
    ));
    let reduction = reducer::reduce(&samples, &config);
    assert_eq!(reduction.boundaries.len(), 1);

    let metrics = convergence::measure(&reduction, &samples, &config, &[], &[]);
    assert_eq!(metrics.c3_held_out_count, 3);
    assert_eq!(metrics.c3_agreed_count, 2);
    assert!((metrics.c3_agreement - 2.0 / 3.0).abs() < 1e-12);
    assert!(!metrics.c3_agreed);
    assert!(!metrics.converged);
}

#[test]
fn c4_requires_support_on_every_boundary_plane() {
    let config = KnowledgeConfig::default_config();
    // One sample on each side, so both boundary-adjacent planes have support 1.
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
    ];
    let reduction = reducer::reduce(&samples, &config);
    assert_eq!(reduction.boundaries.len(), 1);
    let metrics = convergence::measure(&reduction, &samples, &config, &[], &[]);
    assert!(!metrics.c4_covered);
    assert!(!metrics.converged);
    assert!(metrics.stop_reason.expect("reason").contains("C4 coverage"));
}

#[test]
fn coverage_has_one_cell_per_operation_and_domain_pair() {
    let config = KnowledgeConfig::default_config();
    let samples = vec![common::sweep_sample(
        0,
        Operation::Subtract,
        "0",
        "0",
        "right",
        "subtract:0:right:h2",
        "Natural",
    )];
    let reduction = reducer::reduce(&samples, &config);
    let metrics = convergence::measure(&reduction, &samples, &config, &[], &[]);
    assert_eq!(metrics.coverage.len(), 12 * 9);
    assert_eq!(metrics.coverage[0].operation, "Add");
    let subtract_natural_natural = metrics
        .coverage
        .iter()
        .find(|cell| cell.key == "Subtract|Natural|Natural")
        .expect("the sampled cell exists");
    assert_eq!(subtract_natural_natural.samples, 1);
    assert_eq!(subtract_natural_natural.weight, 1.0);
}
