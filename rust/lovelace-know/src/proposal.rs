//! The explicit proposal distribution q(z).
//!
//! A mixture of:
//! * deterministic one-dimensional sweeps (boundary discovery, at two coarse step sizes so
//!   that a boundary can be confirmed twice for C2);
//! * seeded Monte Carlo draws (breadth) whose weight is the reciprocal of the coordinate
//!   space size;
//! * frontier-guided bisection (refinement of unresolved boundary intervals);
//! * held-out validation probes one step outside each localized guard (C3).
//!
//! Every draw is a deterministic function of the seed and the current graph, so the whole
//! sample stream is reproducible.

use std::collections::BTreeSet;

use lovelace_int::{Int, Nat};

use crate::exact::ExactNumber;
use crate::model::{
    BoundaryEdge, Graph, KnowledgeConfig, NumberDomain, Operand, Operation, SamplingKind,
    frontier_kinds,
};
use crate::random::SplitMix64;

/// A sample coordinate to execute (before execution).
#[derive(Clone, Debug)]
pub struct SampleSpec {
    /// The script text handed to the runner.
    pub script: String,
    /// The operation under observation.
    pub op: Operation,
    /// The left operand.
    pub left: Operand,
    /// The right operand.
    pub right: Operand,
    /// The sweep family, when the sample belongs to one.
    pub sweep_id: Option<String>,
    /// The swept side, when an operand is swept.
    pub swept_side: Option<String>,
    /// The swept coordinate, when an operand is swept.
    pub axis_pos: Option<String>,
    /// How the sample entered the batch.
    pub sampling_kind: SamplingKind,
    /// The proposal density weight.
    pub weight: f64,
}

/// The deterministic phase scheduler for one convergence run.
#[derive(Clone, Debug)]
pub struct Proposal {
    sweeps: Vec<SampleSpec>,
    rng: SplitMix64,
    validated: BTreeSet<String>,
    sweep_cursor: usize,
    random_drawn: usize,
}

impl Proposal {
    /// Builds the proposal for a seed and configuration (the sweeps are static).
    #[must_use]
    pub fn new(seed: i64, config: &KnowledgeConfig) -> Self {
        let mut proposal = Self {
            sweeps: Vec::new(),
            rng: SplitMix64::from_seed(seed),
            validated: BTreeSet::new(),
            sweep_cursor: 0,
            random_drawn: 0,
        };
        proposal.build_sweeps(config);
        proposal
    }

    /// The number of static sweep coordinates.
    #[must_use]
    pub fn sweep_count(&self) -> usize {
        self.sweeps.len()
    }

    /// Produces the next batch, biased toward frontiers once breadth is exhausted.
    pub fn next_batch(
        &mut self,
        graph: Option<&Graph>,
        config: &KnowledgeConfig,
    ) -> Vec<SampleSpec> {
        let batch_size = usize::try_from(config.batch_size).unwrap_or(0);
        let mut batch: Vec<SampleSpec> = Vec::new();

        // Phase 1: deterministic sweeps (discover planes and candidate boundaries).
        while batch.len() < batch_size && self.sweep_cursor < self.sweeps.len() {
            batch.push(self.sweeps[self.sweep_cursor].clone());
            self.sweep_cursor += 1;
        }

        // Phase 2: random Monte Carlo breadth.
        let min_random = usize::try_from(config.min_random_samples).unwrap_or(0);
        while batch.len() < batch_size && self.random_drawn < min_random {
            let spec = self.random_spec(config);
            batch.push(spec);
            self.random_drawn += 1;
        }

        // Phases 3 and 4 only after breadth is exhausted: bias toward frontiers.
        let breadth_done =
            self.sweep_cursor >= self.sweeps.len() && self.random_drawn >= min_random;
        if breadth_done {
            if let Some(graph) = graph {
                // Phase 3: bisection refinement of unresolved boundary intervals.
                for frontier in &graph.frontiers {
                    if batch.len() >= batch_size {
                        break;
                    }
                    if frontier.kind != frontier_kinds::UNRESOLVED_BOUNDARY {
                        continue;
                    }
                    let (Some(operation), Some(anchor), Some(low), Some(high)) = (
                        frontier.operation.as_deref(),
                        frontier.anchor.as_deref(),
                        frontier.low.as_deref(),
                        frontier.high.as_deref(),
                    ) else {
                        continue;
                    };
                    let Some(op) = Operation::parse(operation) else {
                        continue;
                    };
                    for midpoint in midpoints(low, high) {
                        if batch.len() >= batch_size {
                            break;
                        }
                        batch.push(make_spec(
                            op,
                            Operand::natural(anchor.to_string()),
                            Operand::natural(midpoint),
                            Some("right"),
                            Some("refine"),
                            SamplingKind::Refine,
                            1.0,
                        ));
                    }
                }

                // Phase 4: held-out validation probes adjacent to localized guards.
                for boundary in &graph.boundaries {
                    if batch.len() >= batch_size {
                        break;
                    }
                    if boundary.guard.kind == "composite" {
                        continue;
                    }
                    if !self.validated.insert(boundary.id.clone()) {
                        continue;
                    }
                    for probe in validation_probes(boundary, graph) {
                        if batch.len() >= batch_size {
                            break;
                        }
                        batch.push(probe);
                    }
                }
            }
        }

        batch
    }

    /// Standalone Monte Carlo draws (used by the sample command).
    #[must_use]
    pub fn random_samples(seed: i64, config: &KnowledgeConfig, count: usize) -> Vec<SampleSpec> {
        let mut rng = SplitMix64::from_seed(seed);
        let mut specs = Vec::with_capacity(count);
        for _ in 0..count {
            let operation = config.operations[rng.next_bounded(config.operations.len())];
            let left = random_operand(&mut rng, config);
            let right = random_operand(&mut rng, config);
            let weight = random_weight(config, &left, &right);
            specs.push(make_spec(
                operation,
                left,
                right,
                None,
                None,
                SamplingKind::Random,
                weight,
            ));
        }
        specs
    }

    fn random_spec(&mut self, config: &KnowledgeConfig) -> SampleSpec {
        let operation = config.operations[self.rng.next_bounded(config.operations.len())];
        let left = random_operand(&mut self.rng, config);
        let right = random_operand(&mut self.rng, config);
        let weight = random_weight(config, &left, &right);
        make_spec(
            operation,
            left,
            right,
            None,
            None,
            SamplingKind::Random,
            weight,
        )
    }

    fn build_sweeps(&mut self, config: &KnowledgeConfig) {
        let naturals = sorted_literals(&config.natural_values);
        for operation in &config.sweep_operations {
            for anchor in anchors(*operation) {
                for step in [2usize, 3] {
                    let mut index = 0usize;
                    while index < naturals.len() {
                        let identifier = sweep_id(*operation, anchor, step);
                        self.sweeps.push(make_spec(
                            *operation,
                            Operand::natural((*anchor).to_string()),
                            Operand::natural(naturals[index].literal()),
                            Some("right"),
                            Some(identifier.as_str()),
                            SamplingKind::Sweep,
                            1.0,
                        ));
                        index += step;
                    }
                }
            }
        }
    }
}

/// A literal reduced to its exact value, so sweep order never depends on text.
#[derive(Clone, Debug)]
struct Literal {
    value: ExactNumber,
}

impl Literal {
    fn literal(&self) -> String {
        self.value.to_lovelace_literal()
    }
}

fn sorted_literals(literals: &[String]) -> Vec<Literal> {
    let mut values: Vec<Literal> = literals
        .iter()
        .map(|literal| Literal {
            value: ExactNumber::parse(literal).expect("the config literals are valid"),
        })
        .collect();
    values.sort_by(|left, right| left.value.cmp(&right.value));
    values
}

fn anchors(operation: Operation) -> &'static [&'static str] {
    match operation {
        Operation::Subtract => &["0", "1", "2", "5", "10"],
        Operation::Divide => &["1", "2", "5", "10"],
        Operation::Modulo => &["5", "10"],
        Operation::Greater | Operation::Less => &["5"],
        _ => &[],
    }
}

fn sweep_id(operation: Operation, anchor: &str, step: usize) -> String {
    format!(
        "{}:{}:right:h{}",
        operation.as_str().to_lowercase(),
        anchor,
        step
    )
}

fn random_operand(rng: &mut SplitMix64, config: &KnowledgeConfig) -> Operand {
    let domain = match rng.next_bounded(3) {
        0 => NumberDomain::Natural,
        1 => NumberDomain::Integer,
        _ => NumberDomain::Real,
    };
    let values = match domain {
        NumberDomain::Natural => &config.natural_values,
        NumberDomain::Integer => &config.integer_values,
        NumberDomain::Real => &config.real_values,
    };
    let literal = values[rng.next_bounded(values.len())].clone();
    Operand { domain, literal }
}

fn random_weight(config: &KnowledgeConfig, left: &Operand, right: &Operand) -> f64 {
    let operations = config.operations.len() as f64;
    let left_values = left.domain.value_count(config) as f64;
    let right_values = right.domain.value_count(config) as f64;
    1.0 / (operations * 3.0 * 3.0 * left_values * right_values)
}

fn make_spec(
    operation: Operation,
    left: Operand,
    right: Operand,
    swept_side: Option<&str>,
    sweep_id: Option<&str>,
    sampling_kind: SamplingKind,
    weight: f64,
) -> SampleSpec {
    let script = format!("{} {} {}", left.literal, operation.symbol(), right.literal);
    let axis_pos = match swept_side {
        Some("right") => Some(right.literal.clone()),
        Some("left") => Some(left.literal.clone()),
        _ => None,
    };
    SampleSpec {
        script,
        op: operation,
        left,
        right,
        sweep_id: sweep_id.map(str::to_string),
        swept_side: swept_side.map(str::to_string),
        axis_pos,
        sampling_kind,
        weight,
    }
}

/// Integer midpoint(s) strictly between two natural literals.
fn midpoints(low_literal: &str, high_literal: &str) -> Vec<String> {
    let (Ok(low), Ok(high)) = (
        ExactNumber::parse(low_literal),
        ExactNumber::parse(high_literal),
    ) else {
        return Vec::new();
    };
    if !low.is_integer() || !high.is_integer() || low >= high {
        return Vec::new();
    }
    let difference = high.num().sub(low.num());
    let (half, remainder) = difference
        .div_rem(&Nat::from_u64(2))
        .expect("two is not zero");
    let midpoint = low.num().add(&half);
    let mut candidates = vec![ExactNumber::new(midpoint.clone(), Nat::from_u64(1))];
    if !remainder.is_zero() {
        candidates.push(ExactNumber::new(
            midpoint.add(&Int::from_i64(1)),
            Nat::from_u64(1),
        ));
    }
    let mut result: Vec<String> = Vec::new();
    for candidate in candidates.into_iter().flatten() {
        if candidate > low
            && candidate < high
            && !result
                .iter()
                .any(|seen| seen == &candidate.to_lovelace_literal())
        {
            result.push(candidate.to_lovelace_literal());
        }
    }
    result
}

/// Held-out probes one step outside each side of a localized boundary.
fn validation_probes(boundary: &BoundaryEdge, graph: &Graph) -> Vec<SampleSpec> {
    let Some(first) = boundary.evidence.first() else {
        return Vec::new();
    };
    let index = first.sample_index;
    if index < 0 || index as usize >= graph.samples.len() {
        return Vec::new();
    }
    let anchor = graph.samples[index as usize].left.literal.clone();

    let mut low: Vec<ExactNumber> = boundary
        .evidence
        .iter()
        .filter(|evidence| evidence.side == "low")
        .filter_map(|evidence| ExactNumber::parse(&evidence.axis_pos).ok())
        .collect();
    let mut high: Vec<ExactNumber> = boundary
        .evidence
        .iter()
        .filter(|evidence| evidence.side == "high")
        .filter_map(|evidence| ExactNumber::parse(&evidence.axis_pos).ok())
        .collect();
    if low.is_empty() || high.is_empty() {
        return Vec::new();
    }
    low.sort();
    high.sort();
    let position_low = low.last().expect("non-empty").clone();
    let position_high = high.first().expect("non-empty").clone();

    let mut probes: Vec<ExactNumber> = Vec::new();
    let one_below = position_low.subtract(&ExactNumber::one());
    if !one_below.is_negative() {
        probes.push(one_below);
    }
    probes.push(position_low.clone());
    probes.push(position_high.clone());
    probes.push(position_high.add(&ExactNumber::one()));

    let mut unique: Vec<ExactNumber> = Vec::new();
    for probe in probes {
        if !unique.contains(&probe) {
            unique.push(probe);
        }
    }

    let Some(operation) = Operation::parse(&boundary.operation) else {
        return Vec::new();
    };
    unique
        .into_iter()
        .map(|probe| {
            let literal = probe.to_lovelace_literal();
            make_spec(
                operation,
                Operand::natural(anchor.clone()),
                Operand::natural(literal),
                Some(boundary.swept_side.as_str()),
                Some("validate"),
                SamplingKind::Validate,
                1.0,
            )
        })
        .collect()
}
