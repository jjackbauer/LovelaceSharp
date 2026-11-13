//! The autonomous converge loop.
//!
//! sample, execute, canonicalize, reduce, merge, measure, (bias toward frontiers), repeat,
//! until the C1-C4 thresholds are met or the sample budget is exhausted. The loop is
//! deterministic for a fixed config and seed.

use std::collections::BTreeSet;

use crate::convergence;
use crate::model::{Graph, KnowledgeConfig};
use crate::proposal::Proposal;
use crate::reducer;
use crate::runner::{self, ScriptRunner};

/// Runs the loop and returns the reduced, measured graph.
#[must_use]
pub fn run(config: &KnowledgeConfig, runner: &dyn ScriptRunner, existing: Option<Graph>) -> Graph {
    let mut graph = existing.unwrap_or_else(|| Graph::new(config.clone()));
    let mut proposal = Proposal::new(config.seed, config);
    let mut existing_sigmas: BTreeSet<String> = graph
        .samples
        .iter()
        .map(|sample| sample.sigma.clone())
        .collect();
    let mut new_planes_per_batch: Vec<i32> = Vec::new();
    let mut batch_sizes: Vec<i32> = Vec::new();
    let mut next_index = i32::try_from(graph.samples.len()).unwrap_or(0);

    while i32::try_from(graph.samples.len()).unwrap_or(i32::MAX) < config.max_samples {
        let batch = proposal.next_batch(Some(&graph), config);
        if batch.is_empty() {
            break;
        }

        let records = runner::execute_batch(runner, &batch, next_index);
        next_index += i32::try_from(records.len()).unwrap_or(0);

        let mut new_planes = 0;
        for record in &records {
            if existing_sigmas.insert(record.sigma.clone()) {
                new_planes += 1;
            }
        }

        let batch_len = i32::try_from(records.len()).unwrap_or(0);
        graph.samples.extend(records);
        new_planes_per_batch.push(new_planes);
        batch_sizes.push(batch_len);

        reduce_and_measure(&mut graph, config, &new_planes_per_batch, &batch_sizes);
        if graph
            .metrics
            .as_ref()
            .is_some_and(|metrics| metrics.converged)
        {
            break;
        }
    }

    reduce_and_measure(&mut graph, config, &new_planes_per_batch, &batch_sizes);
    graph
}

fn reduce_and_measure(
    graph: &mut Graph,
    config: &KnowledgeConfig,
    new_planes_per_batch: &[i32],
    batch_sizes: &[i32],
) {
    let reduction = reducer::reduce(&graph.samples, config);
    let metrics = convergence::measure(
        &reduction,
        &graph.samples,
        config,
        new_planes_per_batch,
        batch_sizes,
    );
    graph.planes = reduction.planes;
    graph.boundaries = reduction.boundaries;
    graph.frontiers = reduction.frontiers;
    graph.metrics = Some(metrics);
}
