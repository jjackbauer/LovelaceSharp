//! The deterministic reducer: planes, boundaries, guards, and frontiers.
//!
//! Clustering is by canonical observation class. Boundaries are found by finite differences
//! along each sweep axis; a difference of exactly one integer step is localized and becomes a
//! boundary candidate, anything wider stays an unresolved frontier for the loop to bisect.
//! Guards are fitted from the bounding positions, and stable identifiers are FNV-1a hashes of
//! the structural key so they are reproducible across runs.

use std::collections::{BTreeMap, BTreeSet};

use lovelace_int::Int;

use crate::exact::ExactNumber;
use crate::model::{
    BoundEvidence, BoundaryEdge, Confidence, Frontier, Guard, KnowledgeConfig, NumberDomain,
    Operation, Plane, SampleRecord, SamplingKind, frontier_kinds,
};

/// The result of reducing a sample set into graph structure.
#[derive(Clone, Debug, Default)]
pub struct Reduction {
    /// The clustered behaviour planes, ordered by sigma.
    pub planes: Vec<Plane>,
    /// The localized boundaries, ordered by structural key.
    pub boundaries: Vec<BoundaryEdge>,
    /// The unresolved regions: unresolved boundaries, low-support planes, weak dimensions.
    pub frontiers: Vec<Frontier>,
}

/// Reduces a sample set into planes, boundaries, and frontiers.
#[must_use]
pub fn reduce(samples: &[SampleRecord], config: &KnowledgeConfig) -> Reduction {
    let planes = cluster_planes(samples);
    let (boundaries, mut frontiers) = detect_boundaries(samples, config);
    frontiers.extend(low_support_frontiers(&planes, config));
    frontiers.extend(weak_dimension_frontiers(samples));
    Reduction {
        planes,
        boundaries,
        frontiers,
    }
}

fn cluster_planes(samples: &[SampleRecord]) -> Vec<Plane> {
    let mut groups: BTreeMap<&str, Vec<&SampleRecord>> = BTreeMap::new();
    for sample in samples {
        groups
            .entry(sample.sigma.as_str())
            .or_default()
            .push(sample);
    }
    let mut planes = Vec::with_capacity(groups.len());
    for (sigma, group) in groups {
        let first = group[0];
        let confidence = if group.len() >= 2 {
            Confidence::Repeated
        } else {
            Confidence::Observed
        };
        let mut sample_indices: Vec<i32> = group.iter().map(|sample| sample.index).collect();
        sample_indices.sort_unstable();
        planes.push(Plane {
            sigma: sigma.to_string(),
            kind: first.kind.clone(),
            error_message: first.error_message.clone(),
            support: i32::try_from(group.len()).unwrap_or(i32::MAX),
            confidence,
            sample_indices,
        });
    }
    planes
}

struct Candidate {
    op: Operation,
    swept_side: String,
    anchor: String,
    plane_lo: String,
    plane_hi: String,
    axis_key: String,
    evidence: Vec<BoundEvidence>,
    step_sizes: BTreeSet<String>,
}

impl Candidate {
    fn add_evidence(&mut self, low: &SampleRecord, high: &SampleRecord) {
        self.evidence.push(BoundEvidence {
            side: "low".to_string(),
            sample_index: low.index,
            axis_pos: low.axis_pos.clone().unwrap_or_default(),
            sigma: low.sigma.clone(),
        });
        self.evidence.push(BoundEvidence {
            side: "high".to_string(),
            sample_index: high.index,
            axis_pos: high.axis_pos.clone().unwrap_or_default(),
            sigma: high.sigma.clone(),
        });
        if let Some(step) = step_of(low.sweep_id.as_deref()) {
            self.step_sizes.insert(step);
        }
        if let Some(step) = step_of(high.sweep_id.as_deref()) {
            self.step_sizes.insert(step);
        }
    }
}

fn step_of(sweep_id: Option<&str>) -> Option<String> {
    let sweep_id = sweep_id?;
    let index = sweep_id.rfind(":h")?;
    let tail = &sweep_id[index + 2..];
    if tail.is_empty() || !tail.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(sweep_id[index..].to_string())
}

fn axis_key(sample: &SampleRecord) -> String {
    format!(
        "{}|{}|{}",
        sample.op.as_str(),
        sample.left.literal,
        sample.swept_side.as_deref().unwrap_or("right")
    )
}

fn detect_boundaries(
    samples: &[SampleRecord],
    config: &KnowledgeConfig,
) -> (Vec<BoundaryEdge>, Vec<Frontier>) {
    // Fitting samples: sweep and refine (held-out validate samples are excluded).
    let mut by_axis: BTreeMap<String, Vec<&SampleRecord>> = BTreeMap::new();
    for sample in samples {
        if sample.sweep_id.is_none()
            || sample.axis_pos.is_none()
            || sample.sampling_kind == SamplingKind::Validate
        {
            continue;
        }
        by_axis.entry(axis_key(sample)).or_default().push(sample);
    }

    let mut candidates: BTreeMap<String, Candidate> = BTreeMap::new();
    let mut frontiers: Vec<Frontier> = Vec::new();
    let mut ordered_axes: BTreeMap<String, Vec<&SampleRecord>> = BTreeMap::new();

    for (key, axis_samples) in &by_axis {
        let mut ordered: Vec<&SampleRecord> = axis_samples.clone();
        ordered.sort_by(|left, right| {
            let left_position = parse_axis(left);
            let right_position = parse_axis(right);
            left_position
                .cmp(&right_position)
                .then(left.index.cmp(&right.index))
        });
        if ordered.len() < 2 {
            continue;
        }
        ordered_axes.insert(key.clone(), ordered.clone());
        let anchor = ordered[0].left.literal.clone();
        let swept_side = ordered[0]
            .swept_side
            .clone()
            .unwrap_or_else(|| "right".to_string());
        let op = ordered[0].op;

        for pair in ordered.windows(2) {
            let low = pair[0];
            let high = pair[1];
            if low.sigma == high.sigma {
                continue;
            }
            let low_position = parse_axis(low);
            let high_position = parse_axis(high);
            let one_step = high_position.num().sub(low_position.num()) == Int::from_i64(1);
            let localized = low_position.is_integer() && high_position.is_integer() && one_step;
            if !localized {
                frontiers.push(Frontier {
                    id: stable_id(&format!("frontier:{key}:{}->{}", low.sigma, high.sigma)),
                    kind: frontier_kinds::UNRESOLVED_BOUNDARY.to_string(),
                    description: format!(
                        "boundary between {} and {} needs bisection",
                        low.sigma, high.sigma
                    ),
                    operation: Some(op.as_str().to_string()),
                    swept_side: Some(swept_side.clone()),
                    anchor: Some(anchor.clone()),
                    low: low.axis_pos.clone(),
                    high: high.axis_pos.clone(),
                });
                continue;
            }
            let candidate_key = format!("{key}|{}|{}", low.sigma, high.sigma);
            let candidate = candidates
                .entry(candidate_key)
                .or_insert_with(|| Candidate {
                    op,
                    swept_side: swept_side.clone(),
                    anchor: anchor.clone(),
                    plane_lo: low.sigma.clone(),
                    plane_hi: high.sigma.clone(),
                    axis_key: key.clone(),
                    evidence: Vec::new(),
                    step_sizes: BTreeSet::new(),
                });
            candidate.add_evidence(low, high);
        }
    }

    let mut boundaries = Vec::with_capacity(candidates.len());
    for (key, candidate) in &candidates {
        let axis = ordered_axes
            .get(&candidate.axis_key)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let guard = fit_guard(
            candidate.op,
            &candidate.swept_side,
            &candidate.anchor,
            &candidate.plane_lo,
            &candidate.plane_hi,
            axis,
        );
        let confirmations = i32::try_from(candidate.step_sizes.len()).unwrap_or(i32::MAX);
        let confidence = if confirmations >= config.c2_min_confirmations {
            Confidence::Conformant
        } else {
            Confidence::Bounded
        };
        let mut evidence = candidate.evidence.clone();
        evidence.sort_by(|left, right| {
            left.side
                .cmp(&right.side)
                .then(left.sample_index.cmp(&right.sample_index))
        });
        boundaries.push(BoundaryEdge {
            id: stable_id(&format!("boundary:{key}")),
            from_plane: candidate.plane_lo.clone(),
            to_plane: candidate.plane_hi.clone(),
            operation: candidate.op.as_str().to_string(),
            swept_side: candidate.swept_side.clone(),
            guard,
            confidence,
            evidence,
        });
    }

    (boundaries, frontiers)
}

fn fit_guard(
    op: Operation,
    swept_side: &str,
    anchor: &str,
    plane_lo: &str,
    plane_hi: &str,
    axis: &[&SampleRecord],
) -> Guard {
    let mut low_positions: Vec<ExactNumber> = axis
        .iter()
        .filter(|sample| sample.sigma == plane_lo)
        .map(|sample| parse_axis(sample))
        .collect();
    let mut high_positions: Vec<ExactNumber> = axis
        .iter()
        .filter(|sample| sample.sigma == plane_hi)
        .map(|sample| parse_axis(sample))
        .collect();
    low_positions.sort();
    high_positions.sort();
    let position_lo = low_positions
        .last()
        .cloned()
        .unwrap_or_else(ExactNumber::zero);
    let position_hi = high_positions
        .first()
        .cloned()
        .unwrap_or_else(ExactNumber::zero);

    // Equality guard: an error plane at position 0 for division or modulo.
    if plane_lo.starts_with("err|")
        && position_lo.is_zero()
        && matches!(op, Operation::Divide | Operation::Modulo)
    {
        return Guard {
            variable: swept_side.to_string(),
            relop: "==".to_string(),
            threshold: "0".to_string(),
            expression: format!("{swept_side} == 0"),
            kind: "equality".to_string(),
        };
    }

    // Threshold guard: the two planes are uniform on each side of position_lo / position_hi.
    let uniform_low = axis.iter().all(|sample| {
        let position = parse_axis(sample);
        position > position_lo || sample.sigma == plane_lo
    });
    let uniform_high = axis.iter().all(|sample| {
        let position = parse_axis(sample);
        position < position_hi || sample.sigma == plane_hi
    });

    if uniform_low && uniform_high {
        let anchor_value = ExactNumber::parse(anchor).unwrap_or_else(|_| ExactNumber::zero());
        let anchor_side = if swept_side == "right" {
            "left"
        } else {
            "right"
        };
        let expression = if position_lo == anchor_value {
            format!("{swept_side} > {anchor_side}")
        } else {
            format!("{swept_side} > {}", position_lo.to_lovelace_literal())
        };
        return Guard {
            variable: swept_side.to_string(),
            relop: ">".to_string(),
            threshold: position_lo.to_lovelace_literal(),
            expression,
            kind: "threshold".to_string(),
        };
    }

    Guard {
        variable: swept_side.to_string(),
        relop: "?".to_string(),
        threshold: String::new(),
        expression: "non-uniform boundary".to_string(),
        kind: "composite".to_string(),
    }
}

fn parse_axis(sample: &SampleRecord) -> ExactNumber {
    sample
        .axis_pos
        .as_deref()
        .and_then(|literal| ExactNumber::parse(literal).ok())
        .unwrap_or_else(ExactNumber::zero)
}

fn low_support_frontiers(planes: &[Plane], config: &KnowledgeConfig) -> Vec<Frontier> {
    planes
        .iter()
        .filter(|plane| plane.support < config.c4_min_support_per_plane)
        .map(|plane| Frontier {
            id: stable_id(&format!("frontier:lowsupport:{}", plane.sigma)),
            kind: frontier_kinds::LOW_SUPPORT.to_string(),
            description: format!(
                "plane {} has support {} (< {})",
                plane.sigma, plane.support, config.c4_min_support_per_plane
            ),
            operation: None,
            swept_side: None,
            anchor: None,
            low: None,
            high: None,
        })
        .collect()
}

fn weak_dimension_frontiers(samples: &[SampleRecord]) -> Vec<Frontier> {
    let mut sampled: BTreeSet<String> = BTreeSet::new();
    for sample in samples {
        if !sample.op.is_arithmetic() {
            continue;
        }
        sampled.insert(format!(
            "{}|{}|{}",
            sample.op.as_str(),
            sample.left.domain.as_str(),
            sample.right.domain.as_str()
        ));
    }

    let mut arithmetic: Vec<Operation> = vec![
        Operation::Add,
        Operation::Subtract,
        Operation::Multiply,
        Operation::Divide,
        Operation::Modulo,
        Operation::Power,
    ];
    arithmetic.sort_by(|left, right| left.as_str().cmp(right.as_str()));

    let mut frontiers: Vec<Frontier> = Vec::new();
    for op in arithmetic {
        for left in NumberDomain::all() {
            for right in NumberDomain::all() {
                let key = format!("{}|{}|{}", op.as_str(), left.as_str(), right.as_str());
                if sampled.contains(&key) {
                    continue;
                }
                frontiers.push(Frontier {
                    id: stable_id(&format!("frontier:weak:{key}")),
                    kind: frontier_kinds::WEAK_DIMENSION.to_string(),
                    description: format!(
                        "unsampled cell {} over {} x {}",
                        op.symbol(),
                        left.as_str(),
                        right.as_str()
                    ),
                    operation: Some(op.as_str().to_string()),
                    swept_side: None,
                    anchor: None,
                    low: None,
                    high: None,
                });
            }
        }
    }
    frontiers
}

/// A deterministic stable identifier: FNV-1a 64-bit over the UTF-8 key, as 16 hex digits.
#[must_use]
pub fn stable_id(input: &str) -> String {
    let mut hash: u64 = 14_695_981_039_346_656_037;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    format!("{hash:016x}")
}
