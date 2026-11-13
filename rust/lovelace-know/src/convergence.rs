//! The C1-C4 convergence metrics and the explicit stopping criterion.
//!
//! * C1 plane saturation: the new-plane rate over the last K batches.
//! * C2 boundary stability: no unresolved boundary and every boundary confirmed at least twice.
//! * C3 prediction agreement: every held-out near-boundary probe must agree with its guard.
//! * C4 coverage: every boundary-adjacent plane must have the minimum support.
//!
//! All four are pure functions of the sample set, so a run is reproducible.

use std::collections::BTreeSet;

use crate::exact::ExactNumber;
use crate::model::{
    BoundaryEdge, Confidence, CoverageCell, KnowledgeConfig, Metrics, NumberDomain, SampleRecord,
    SamplingKind, frontier_kinds,
};
use crate::reducer::Reduction;

/// Computes the C1-C4 metrics and the stopping result for a reduction.
#[must_use]
pub fn measure(
    reduction: &Reduction,
    samples: &[SampleRecord],
    config: &KnowledgeConfig,
    new_planes_per_batch: &[i32],
    batch_sizes: &[i32],
) -> Metrics {
    // C1: plane saturation (new-plane rate over the last K batches).
    let window = usize::try_from(config.c1_window_batches).unwrap_or(0);
    let take = new_planes_per_batch.len().saturating_sub(window);
    let new_last_k: i32 = new_planes_per_batch[take..].iter().sum();
    let samples_take = batch_sizes.len().saturating_sub(window);
    let samples_last_k: i32 = batch_sizes[samples_take..].iter().sum();
    let rate = if samples_last_k == 0 {
        0.0
    } else {
        f64::from(new_last_k) / f64::from(samples_last_k)
    };
    let c1 = rate <= config.c1_new_plane_rate_threshold;

    // C2: boundary localization and stability.
    let no_unresolved = !reduction
        .frontiers
        .iter()
        .any(|frontier| frontier.kind == frontier_kinds::UNRESOLVED_BOUNDARY);
    let total_boundaries = i32::try_from(reduction.boundaries.len()).unwrap_or(i32::MAX);
    let stable_boundaries = i32::try_from(
        reduction
            .boundaries
            .iter()
            .filter(|boundary| boundary.confidence >= Confidence::Bounded)
            .count(),
    )
    .unwrap_or(i32::MAX);
    let c2 = no_unresolved && stable_boundaries == total_boundaries;

    // C3: prediction agreement on held-out near-boundary probes.
    let (held_out, agreed) = measure_c3(reduction, samples);
    let agreement = if held_out == 0 {
        0.0
    } else {
        f64::from(agreed) / f64::from(held_out)
    };
    let c3 = held_out > 0 && agreement >= config.c3_agreement_threshold;

    // C4: coverage (boundary-adjacent planes have the minimum support).
    let mut boundary_planes: BTreeSet<&str> = BTreeSet::new();
    for boundary in &reduction.boundaries {
        boundary_planes.insert(boundary.from_plane.as_str());
        boundary_planes.insert(boundary.to_plane.as_str());
    }
    let c4 = boundary_planes.iter().all(|sigma| {
        reduction
            .planes
            .iter()
            .find(|plane| plane.sigma == *sigma)
            .is_some_and(|plane| plane.support >= config.c4_min_support_per_plane)
    });

    let coverage = coverage(samples, config);

    let converged = c1 && c2 && c3 && c4;
    let stop_reason = if converged {
        "C1-C4 thresholds met".to_string()
    } else {
        let mut unmet: Vec<&str> = Vec::new();
        if !c1 {
            unmet.push("C1 plane saturation");
        }
        if !c2 {
            unmet.push("C2 boundary stability");
        }
        if !c3 {
            unmet.push("C3 prediction agreement");
        }
        if !c4 {
            unmet.push("C4 coverage");
        }
        format!("not converged: {}", unmet.join(", "))
    };

    Metrics {
        converged,
        stop_reason: Some(stop_reason),
        total_samples: i32::try_from(samples.len()).unwrap_or(i32::MAX),
        plane_count: i32::try_from(reduction.planes.len()).unwrap_or(i32::MAX),
        c1_new_planes_last_k: new_last_k,
        c1_new_plane_rate: rate,
        c1_saturated: c1,
        c2_total_boundaries: total_boundaries,
        c2_stable_boundaries: stable_boundaries,
        c2_stable: c2,
        c3_held_out_count: held_out,
        c3_agreed_count: agreed,
        c3_agreement: agreement,
        c3_agreed: c3,
        c4_covered: c4,
        coverage,
    }
}

fn measure_c3(reduction: &Reduction, samples: &[SampleRecord]) -> (i32, i32) {
    let validate: Vec<&SampleRecord> = samples
        .iter()
        .filter(|sample| sample.sampling_kind == SamplingKind::Validate)
        .collect();
    if validate.is_empty() {
        return (0, 0);
    }

    // Map each axis to its single non-composite guard boundary.
    let mut by_axis: std::collections::BTreeMap<String, &BoundaryEdge> =
        std::collections::BTreeMap::new();
    for boundary in &reduction.boundaries {
        if boundary.guard.kind == "composite" {
            continue;
        }
        let index = boundary
            .evidence
            .first()
            .map_or(-1, |evidence| evidence.sample_index);
        if index < 0 || index as usize >= samples.len() {
            continue;
        }
        let anchor = samples[index as usize].left.literal.clone();
        by_axis.insert(
            format!("{}|{}|{}", boundary.operation, anchor, boundary.swept_side),
            boundary,
        );
    }

    let mut held_out = 0;
    let mut agreed = 0;
    for sample in validate {
        let key = format!(
            "{}|{}|{}",
            sample.op.as_str(),
            sample.left.literal,
            sample.swept_side.as_deref().unwrap_or("right")
        );
        let Some(boundary) = by_axis.get(&key) else {
            continue;
        };
        held_out += 1;
        if predicts(boundary, sample) {
            agreed += 1;
        }
    }
    (held_out, agreed)
}

fn predicts(boundary: &BoundaryEdge, sample: &SampleRecord) -> bool {
    let Some(position) = sample
        .axis_pos
        .as_deref()
        .and_then(|literal| ExactNumber::parse(literal).ok())
    else {
        return false;
    };
    if boundary.guard.kind == "equality" {
        // Equality guard: an error exactly when the position is zero.
        let is_error = sample.sigma.starts_with("err|");
        return is_error == position.is_zero();
    }
    let Ok(threshold) = ExactNumber::parse(&boundary.guard.threshold) else {
        return false;
    };
    let predicted = if position > threshold {
        &boundary.to_plane
    } else {
        &boundary.from_plane
    };
    sample.sigma == *predicted
}

fn coverage(samples: &[SampleRecord], config: &KnowledgeConfig) -> Vec<CoverageCell> {
    let mut cells: Vec<CoverageCell> = Vec::new();
    for operation in &config.operations {
        for left in NumberDomain::all() {
            for right in NumberDomain::all() {
                cells.push(CoverageCell {
                    key: format!(
                        "{}|{}|{}",
                        operation.as_str(),
                        left.as_str(),
                        right.as_str()
                    ),
                    operation: operation.as_str().to_string(),
                    left_domain: left.as_str().to_string(),
                    right_domain: right.as_str().to_string(),
                    samples: 0,
                    weight: 0.0,
                });
            }
        }
    }

    for sample in samples {
        let key = format!(
            "{}|{}|{}",
            sample.op.as_str(),
            sample.left.domain.as_str(),
            sample.right.domain.as_str()
        );
        if let Some(cell) = cells.iter_mut().find(|cell| cell.key == key) {
            cell.samples += 1;
            cell.weight += sample.weight;
        }
    }

    cells.sort_by(|left, right| {
        left.operation
            .cmp(&right.operation)
            .then(left.left_domain.cmp(&right.left_domain))
            .then(left.right_domain.cmp(&right.right_domain))
    });
    cells
}
