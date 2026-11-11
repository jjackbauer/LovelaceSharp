//! The seeded falsification loop: classify, filter by the rule's own claim, refine boundaries.

use lovelace_symbolics::assumptions::{Assumption, AssumptionSet};
use lovelace_symbolics::{Expr, Rat};

use crate::numeric::Num;

use super::boundary::{BoundaryBias, BoundaryBiasSpec};
use super::observation::{
    GENERATOR_VERSION, ObservationClass, ObservationClassifier, PRECISION, default_eval, num_cmp,
    num_text, rat_text,
};
use super::probe::{FalsificationProbe, FalsificationReport};
use super::probeset::ProbePoint;

/// The environment a rule was licensed under, paired with the expressions actually compared there.
#[derive(Clone, Debug)]
pub(crate) struct RuleShape {
    /// The rule's own claim about its input.
    pub(crate) environment: AssumptionSet,
    /// The left-hand expression.
    pub(crate) left: Expr,
    /// The right-hand expression.
    pub(crate) right: Expr,
}

impl RuleShape {
    /// The environment rendered deterministically.
    pub(crate) fn environment_text(&self) -> String {
        super::observation::environment_text(&self.environment)
    }
}

/// The loop's computation precision, as the existing gate uses it.
pub(crate) const LOOP_PRECISION: i64 = PRECISION as i64;

/// Classifies one exact rational input and describes it as a probe record.
pub(crate) fn classify_at(
    rule_id: &str,
    shape: &RuleShape,
    variable: &str,
    point: &ProbePoint,
    tolerance: &super::observation::ComparisonTolerance,
) -> (ObservationClass, FalsificationProbe) {
    let value = Num::Exact(point.value.clone());
    let classified =
        ObservationClassifier::classify(&shape.left, &shape.right, variable, &value, tolerance);
    let probe = FalsificationProbe {
        rule_id: rule_id.to_string(),
        region: point.region.clone(),
        label: point.label.clone(),
        input: rat_text(&point.value),
        is_complex: false,
        class: classified.class,
        left_outcome: classified.left.outcome,
        right_outcome: classified.right.outcome,
        left: shape.left.to_string(),
        right: shape.right.to_string(),
        left_value: classified
            .left
            .value
            .as_ref()
            .map(num_text)
            .unwrap_or_default(),
        right_value: classified
            .right
            .value
            .as_ref()
            .map(num_text)
            .unwrap_or_default(),
        delta_text: classified.delta_text.clone().unwrap_or_default(),
        precision: LOOP_PRECISION,
        tolerance: tolerance.text.clone(),
        environment: shape.environment_text(),
        generator_version: GENERATOR_VERSION.to_string(),
        why: point.why.clone(),
    };
    (classified.class, probe)
}

/// Whether every atom of the rule's declared environment holds at this exact rational input.
pub(crate) fn assumption_holds(shape: &RuleShape, variable: &str, point: &ProbePoint) -> bool {
    for atom in shape.environment.atoms() {
        if !atom_holds_at(atom, &point.value, variable) {
            return false;
        }
    }
    true
}

/// Whether one assumption atom holds at one exact rational input. An atom this harness cannot
/// decide counts as NOT holding, so an undecidable guard never becomes a counterexample.
pub(crate) fn atom_holds_at(atom: &Assumption, value: &Rat, variable: &str) -> bool {
    let Assumption::Interval(expression, lower, lower_open, upper, upper_open) = atom else {
        return false;
    };
    if expression != &Expr::sym(variable) {
        return false;
    }
    let binding = Num::Exact(value.clone());
    let subject = match default_eval(expression, variable, &binding) {
        Ok(value) => value,
        Err(_) => return false,
    };
    if let Some(bound) = lower {
        let bound_value = match default_eval(bound, variable, &binding) {
            Ok(value) => value,
            Err(_) => return false,
        };
        let Some(ordering) = num_cmp(&subject, &bound_value) else {
            return false;
        };
        if ordering == std::cmp::Ordering::Less
            || (ordering == std::cmp::Ordering::Equal && *lower_open)
        {
            return false;
        }
    }
    if let Some(bound) = upper {
        let bound_value = match default_eval(bound, variable, &binding) {
            Ok(value) => value,
            Err(_) => return false,
        };
        let Some(ordering) = num_cmp(&subject, &bound_value) else {
            return false;
        };
        if ordering == std::cmp::Ordering::Greater
            || (ordering == std::cmp::Ordering::Equal && *upper_open)
        {
            return false;
        }
    }
    true
}

/// The loop's own accumulator: it de-duplicates by (rule, input, environment), filters by the
/// rule's claim, and refines every in-order class change with the boundary generator.
struct LoopRun<'a> {
    rule_id: &'a str,
    shape: &'a RuleShape,
    variable: &'a str,
    tolerance: &'a super::observation::ComparisonTolerance,
    probes: Vec<FalsificationProbe>,
    defects: Vec<String>,
    keys: Vec<String>,
    classes: Vec<ObservationClass>,
    values: Vec<Rat>,
}

impl<'a> LoopRun<'a> {
    fn new(
        rule_id: &'a str,
        shape: &'a RuleShape,
        variable: &'a str,
        tolerance: &'a super::observation::ComparisonTolerance,
    ) -> LoopRun<'a> {
        LoopRun {
            rule_id,
            shape,
            variable,
            tolerance,
            probes: Vec::new(),
            defects: Vec::new(),
            keys: Vec::new(),
            classes: Vec::new(),
            values: Vec::new(),
        }
    }

    fn take(&mut self, point: &ProbePoint, log_class: bool) {
        let key = format!(
            "{}@R:{}#{}",
            self.rule_id,
            rat_text(&point.value),
            self.shape.environment_text()
        );
        if self.keys.contains(&key) {
            return;
        }
        if !assumption_holds(self.shape, self.variable, point) {
            return;
        }
        let (class, probe) = classify_at(
            self.rule_id,
            self.shape,
            self.variable,
            point,
            self.tolerance,
        );
        self.keys.push(key);
        self.probes.push(probe.clone());
        if log_class {
            self.classes.push(class);
            self.values.push(point.value.clone());
        }
        if probe.is_defect() {
            self.defects.push(format!(
                "{} at {}:{} threw an unexpected exception ({}/{}) - a defect, not a class",
                self.rule_id,
                point.region,
                point.label,
                probe.left_outcome.name(),
                probe.right_outcome.name()
            ));
        }
    }
}

/// The seeded loop over an identity.
pub(crate) struct FalsificationLoop;

impl FalsificationLoop {
    /// Walks the probe set once, and refines every in-order class change with the generator.
    pub(crate) fn run(
        rule_id: &str,
        shape: &RuleShape,
        variable: &str,
        points: &[ProbePoint],
        spec: &BoundaryBiasSpec,
        tolerance: &super::observation::ComparisonTolerance,
    ) -> FalsificationReport {
        let mut state = LoopRun::new(rule_id, shape, variable, tolerance);
        for point in points {
            state.take(point, true);
        }

        let mut bias = BoundaryBias::new(spec.clone());
        let mut index = 0usize;
        while index + 1 < state.classes.len() {
            let first = state.classes[index];
            let second = state.classes[index + 1];
            let low = state.values[index].clone();
            let high = state.values[index + 1].clone();
            index += 1;
            if first == second || low >= high {
                continue;
            }
            match bias.broad_phase(&low, &high) {
                Ok(candidates) => {
                    for candidate in candidates {
                        let point = ProbePoint {
                            region: "generated-boundary".to_string(),
                            label: rat_text(&candidate),
                            value: candidate,
                            why: format!(
                                "boundary bias between {} ({}) and {} ({}), seed {}",
                                rat_text(&low),
                                first.name(),
                                rat_text(&high),
                                second.name(),
                                spec.seed
                            ),
                        };
                        state.take(&point, false);
                    }
                }
                Err(message) => {
                    state
                        .defects
                        .push(format!("{} boundary bias: {}", rule_id, message));
                    continue;
                }
            }

            let mut offered = Vec::new();
            let boundary = {
                let mut classify = |value: &Rat| -> ObservationClass {
                    let point = ProbePoint {
                        region: "generated-boundary".to_string(),
                        label: rat_text(value),
                        value: value.clone(),
                        why: "bisection midpoint".to_string(),
                    };
                    classify_at(rule_id, shape, variable, &point, tolerance).0
                };
                bias.narrow_phase(&low, &high, first, &mut classify)
            };
            offered.extend(boundary.1);
            for midpoint in offered {
                let point = ProbePoint {
                    region: "generated-boundary".to_string(),
                    label: rat_text(&midpoint),
                    value: midpoint,
                    why: "bisection midpoint".to_string(),
                };
                state.take(&point, false);
            }
        }

        let mut probes = state.probes;
        probes.sort_by(|left, right| {
            (&left.rule_id, &left.region, &left.input, &left.environment).cmp(&(
                &right.rule_id,
                &right.region,
                &right.input,
                &right.environment,
            ))
        });
        FalsificationReport {
            generator_version: GENERATOR_VERSION.to_string(),
            sample_count: probes.len(),
            probes,
            defects: state.defects,
        }
    }
}
