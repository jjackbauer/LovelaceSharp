//! Stage 1 tests: deterministic seeded generation and the proposal mixture.

mod common;

use lovelace_know::model::{Frontier, Graph, KnowledgeConfig, Operation, SamplingKind};
use lovelace_know::proposal::{Proposal, SampleSpec};

fn default_config() -> KnowledgeConfig {
    KnowledgeConfig::default_config()
}

fn batch(
    proposal: &mut Proposal,
    config: &KnowledgeConfig,
    graph: Option<&Graph>,
) -> Vec<SampleSpec> {
    proposal.next_batch(graph, config)
}

#[test]
fn builds_the_expected_number_of_sweeps() {
    let config = default_config();
    let proposal = Proposal::new(config.seed, &config);
    // 5 subtract anchors * 12 positions + 4 divide * 12 + 2 modulo * 12 + 1 greater * 12 + 1 less * 12
    assert_eq!(proposal.sweep_count(), 156);
}

#[test]
fn first_sweep_is_the_documented_coordinate() {
    let config = default_config();
    let mut proposal = Proposal::new(config.seed, &config);
    let batch = batch(&mut proposal, &config, None);
    assert_eq!(batch.len(), 64);
    assert_eq!(batch[0].script, "0 - 0");
    assert_eq!(batch[0].sweep_id.as_deref(), Some("subtract:0:right:h2"));
    assert_eq!(batch[0].swept_side.as_deref(), Some("right"));
    assert_eq!(batch[0].axis_pos.as_deref(), Some("0"));
    assert_eq!(batch[0].sampling_kind, SamplingKind::Sweep);
    assert_eq!(batch[0].weight, 1.0);
    // The h2 grid visits every second natural; the h3 grid follows it.
    assert_eq!(batch[1].axis_pos.as_deref(), Some("2"));
    assert_eq!(batch[1].sweep_id.as_deref(), Some("subtract:0:right:h2"));
    assert_eq!(batch[7].sweep_id.as_deref(), Some("subtract:0:right:h3"));
    assert_eq!(batch[7].axis_pos.as_deref(), Some("0"));
}

#[test]
fn same_seed_same_scripts() {
    let config = default_config();
    let mut first = Proposal::new(config.seed, &config);
    let mut second = Proposal::new(config.seed, &config);
    for _ in 0..4 {
        let left = batch(&mut first, &config, None);
        let right = batch(&mut second, &config, None);
        assert_eq!(left.len(), right.len());
        for (a, b) in left.iter().zip(right.iter()) {
            assert_eq!(a.script, b.script);
            assert_eq!(a.sweep_id, b.sweep_id);
            assert_eq!(a.sampling_kind, b.sampling_kind);
            assert_eq!(a.weight, b.weight);
        }
    }
}

#[test]
fn different_seed_different_scripts() {
    let config = default_config();
    let samples_for = |seed: i64| -> Vec<String> {
        let mut proposal = Proposal::new(seed, &config);
        let mut scripts = Vec::new();
        for _ in 0..4 {
            scripts.extend(
                batch(&mut proposal, &config, None)
                    .into_iter()
                    .map(|spec| spec.script),
            );
        }
        scripts
    };
    let reference = samples_for(config.seed);
    assert_eq!(reference, samples_for(config.seed));
    assert_ne!(reference, samples_for(config.seed + 1));
}

#[test]
fn random_draws_carry_the_proposal_weight() {
    let config = default_config();
    let mut proposal = Proposal::new(config.seed, &config);
    let mut random: Option<SampleSpec> = None;
    for _ in 0..4 {
        for spec in batch(&mut proposal, &config, None) {
            if spec.sampling_kind == SamplingKind::Random {
                random = Some(spec);
                break;
            }
        }
        if random.is_some() {
            break;
        }
    }
    let spec = random.expect("the breadth phase produces random draws");
    let operations = config.operations.len() as f64;
    let expected = 1.0
        / (operations
            * 3.0
            * 3.0
            * spec.left.domain.value_count(&config) as f64
            * spec.right.domain.value_count(&config) as f64);
    assert!((spec.weight - expected).abs() < 1e-18);
    assert_eq!(
        spec.script,
        format!(
            "{} {} {}",
            spec.left.literal,
            spec.op.symbol(),
            spec.right.literal
        )
    );
}

#[test]
fn refinement_bisects_unresolved_frontier_intervals() {
    let config = default_config();
    let mut proposal = Proposal::new(config.seed, &config);
    let mut graph = Graph::new(config.clone());
    graph.frontiers.push(Frontier {
        id: "frontier-1".to_string(),
        kind: "unresolved-boundary".to_string(),
        description: "boundary between Natural and Integer needs bisection".to_string(),
        operation: Some("Subtract".to_string()),
        swept_side: Some("right".to_string()),
        anchor: Some("0".to_string()),
        low: Some("0".to_string()),
        high: Some("3".to_string()),
    });

    let mut refine_specs: Vec<SampleSpec> = Vec::new();
    for _ in 0..6 {
        let produced = batch(&mut proposal, &config, Some(&graph));
        for spec in produced {
            if spec.sampling_kind == SamplingKind::Refine {
                refine_specs.push(spec);
            }
        }
        if !refine_specs.is_empty() {
            break;
        }
    }

    assert_eq!(
        refine_specs.len(),
        2,
        "the interior integer midpoints of 0..3 are 1 and 2"
    );
    assert_eq!(refine_specs[0].script, "0 - 1");
    assert_eq!(refine_specs[1].script, "0 - 2");
    assert_eq!(refine_specs[0].sweep_id.as_deref(), Some("refine"));
    assert_eq!(refine_specs[0].op, Operation::Subtract);
    assert_eq!(refine_specs[0].weight, 1.0);
}

#[test]
fn validation_probes_bracket_a_localized_boundary() {
    let config = default_config();
    let mut proposal = Proposal::new(config.seed, &config);
    let mut graph = Graph::new(config.clone());
    graph.samples.push(common::sweep_sample(
        0,
        Operation::Subtract,
        "0",
        "0",
        "right",
        "subtract:0:right:h2",
        "Natural",
    ));
    graph.samples.push(common::sweep_sample(
        1,
        Operation::Subtract,
        "0",
        "1",
        "right",
        "subtract:0:right:h2",
        "Integer",
    ));
    graph.boundaries.push(lovelace_know::model::BoundaryEdge {
        id: "boundary-1".to_string(),
        from_plane: "Natural".to_string(),
        to_plane: "Integer".to_string(),
        operation: "Subtract".to_string(),
        swept_side: "right".to_string(),
        guard: lovelace_know::model::Guard {
            variable: "right".to_string(),
            relop: ">".to_string(),
            threshold: "0".to_string(),
            expression: "right > left".to_string(),
            kind: "threshold".to_string(),
        },
        confidence: lovelace_know::model::Confidence::Conformant,
        evidence: vec![
            lovelace_know::model::BoundEvidence {
                side: "low".to_string(),
                sample_index: 0,
                axis_pos: "0".to_string(),
                sigma: "Natural".to_string(),
            },
            lovelace_know::model::BoundEvidence {
                side: "high".to_string(),
                sample_index: 1,
                axis_pos: "1".to_string(),
                sigma: "Integer".to_string(),
            },
        ],
    });

    let mut probes: Vec<SampleSpec> = Vec::new();
    for _ in 0..6 {
        let produced = batch(&mut proposal, &config, Some(&graph));
        for spec in produced {
            if spec.sampling_kind == SamplingKind::Validate {
                probes.push(spec);
            }
        }
        if !probes.is_empty() {
            break;
        }
    }

    // One step below the low bound is negative and is dropped; the bound positions and one
    // step above the high bound are kept.
    let scripts: Vec<&str> = probes.iter().map(|spec| spec.script.as_str()).collect();
    assert_eq!(scripts, vec!["0 - 0", "0 - 1", "0 - 2"]);
    assert_eq!(probes[0].sweep_id.as_deref(), Some("validate"));
    assert_eq!(probes[0].swept_side.as_deref(), Some("right"));
    assert_eq!(probes[0].axis_pos.as_deref(), Some("0"));
}
