//! The registry-driven falsification gate and the complex-treatment audit.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use lovelace_real::Real;
use lovelace_symbolics::assumptions::{
    Assumption, AssumptionSet, Domain, SymbolPredicate, Tristate,
};
use lovelace_symbolics::{Expr, Rat};

use crate::complex::Complex;
use crate::numeric::Num;

use super::observation::{
    ComparisonTolerance, EvalFailure, PRECISION, default_eval, environment_text, fun, num_abs,
    num_cmp, num_is_zero, num_squared_magnitude, num_text, pi, x,
};
use super::probeset::{
    ComplexProbe, ProbePoint, complex_exclusion_witnesses, complex_probes, frozen,
};

/// One licensed environment and the instantiated left/right expressions compared there.
pub(crate) type LicensedShape = (AssumptionSet, Expr, Expr);

/// One rule the gate can sweep. Rust's simplifier has no queryable rewrite-rule registry, so the
/// gate takes its rules as data; the shipped registry below mirrors the reference's nine rules.
#[derive(Clone)]
pub(crate) struct GateRule {
    /// The stable rewrite-rule id.
    pub(crate) id: String,
    /// The instantiated left-hand expression.
    pub(crate) lhs: Expr,
    /// The instantiated right-hand expression, or the replacement's failure message.
    pub(crate) rhs: Result<Expr, String>,
    /// The rule's declared conditions.
    pub(crate) conditions: AssumptionSet,
    /// The rule's applicability predicate over a candidate environment.
    pub(crate) applicable: fn(&AssumptionSet) -> bool,
}

/// What the registry gate saw. Failures are values here, so a negative control can assert that
/// the gate reports a failure instead of passing silently.
#[derive(Clone, Debug, Default)]
pub(crate) struct GateReport {
    /// Every rule the gate was given.
    pub(crate) registered: Vec<String>,
    /// Every rule with at least one licensed environment.
    pub(crate) sampled: Vec<String>,
    /// Rules whose pattern could not be instantiated.
    pub(crate) uninstantiable: Vec<String>,
    /// Rules with no licensed environment.
    pub(crate) unlicensed: Vec<String>,
    /// Rules whose instantiated left side lost the probe symbol.
    pub(crate) degenerate: Vec<String>,
    /// Rules that threw during replacement, or points that threw during comparison.
    pub(crate) threw: Vec<String>,
    /// Rules whose identity was observed to fail.
    pub(crate) falsified: Vec<String>,
    /// Why an environment was not licensed.
    pub(crate) license_notes: Vec<String>,
    /// Assumption atom kinds the guard could not decide.
    pub(crate) unhandled_atoms: Vec<String>,
    /// How many points were actually compared per rule.
    pub(crate) compared: BTreeMap<String, usize>,
    /// Which regions contributed a compared point per rule.
    pub(crate) regions: BTreeMap<String, BTreeSet<String>>,
    /// Every complex comparison the gate actually made: rule, environment index and label.
    pub(crate) complex_compared: Vec<(String, usize, String)>,
    /// The shapes the gate used per rule.
    pub(crate) licensed_shapes: BTreeMap<String, Vec<LicensedShape>>,
}

impl GateReport {
    /// Unexpected exception from rule code, or a value disagreement: both are failures.
    pub(crate) fn failed(&self) -> bool {
        !self.threw.is_empty() || !self.falsified.is_empty()
    }

    /// Records one compared point.
    pub(crate) fn count(&mut self, rule_id: &str, region: &str) {
        *self.compared.entry(rule_id.to_string()).or_insert(0) += 1;
        self.regions
            .entry(rule_id.to_string())
            .or_default()
            .insert(region.to_string());
    }

    /// Every region that contributed a compared point, across the registry.
    pub(crate) fn all_covered_regions(&self) -> BTreeSet<String> {
        let mut all = BTreeSet::new();
        for regions in self.regions.values() {
            all.extend(regions.iter().cloned());
        }
        all
    }

    /// The one-line summary the reference prints.
    pub(crate) fn summary(&self) -> String {
        let regions: Vec<String> = self.all_covered_regions().into_iter().collect();
        format!(
            "registered={} sampled={} uninstantiable={} unlicensed={} degenerate={} threw={} falsified={} regions={}",
            self.registered.len(),
            self.sampled.len(),
            self.uninstantiable.len(),
            self.unlicensed.len(),
            self.degenerate.len(),
            self.threw.len(),
            self.falsified.len(),
            regions.join(",")
        )
    }
}

/// The assumption that the probe symbol ranges over the reals.
pub(crate) fn real_atom() -> Assumption {
    Assumption::SymbolDomain("x".to_string(), Domain::Real)
}

/// The assumption that the probe symbol is non-negative.
pub(crate) fn non_negative_atom() -> Assumption {
    Assumption::SymbolProperty("x".to_string(), SymbolPredicate::NonNegative)
}

/// The assumption that the probe symbol is nonzero.
pub(crate) fn non_zero_atom() -> Assumption {
    Assumption::SymbolProperty("x".to_string(), SymbolPredicate::NonZero)
}

/// The principal-strip membership the reference attaches to log(exp(z)).
pub(crate) fn strip_atom() -> Assumption {
    Assumption::Interval(
        fun("im", vec![x()]),
        Some(Expr::neg(pi())),
        true,
        Some(pi()),
        false,
    )
}

/// The finiteness condition the reference attaches to exp(log(x)).
pub(crate) fn finite_log_atom() -> Assumption {
    Assumption::ExpressionProperty(fun("log", vec![x()]), SymbolPredicate::Finite)
}

fn always(_: &AssumptionSet) -> bool {
    true
}

fn requires_real(env: &AssumptionSet) -> bool {
    env.ask(&real_atom()) == Tristate::True
}

fn requires_non_negative(env: &AssumptionSet) -> bool {
    env.ask(&non_negative_atom()) == Tristate::True
}

fn requires_real_not_non_negative(env: &AssumptionSet) -> bool {
    env.ask(&non_negative_atom()) != Tristate::True && env.ask(&real_atom()) == Tristate::True
}

fn requires_strip(env: &AssumptionSet) -> bool {
    env.ask(&strip_atom()) == Tristate::True
}

/// The nine shipped rules the reference registry exposes through Simplify.RulesForTesting.
pub(crate) fn gate_registry() -> Vec<GateRule> {
    let empty = AssumptionSet::empty();
    vec![
        GateRule {
            id: "trig.pythagorean-sin2-cos2".to_string(),
            lhs: Expr::add(
                Expr::pow(fun("cos", vec![x()]), Expr::num(2)),
                Expr::pow(fun("sin", vec![x()]), Expr::num(2)),
            ),
            rhs: Ok(Expr::num(1)),
            conditions: empty.clone(),
            applicable: always,
        },
        GateRule {
            id: "pow.sqrt-square-nonnegative".to_string(),
            lhs: Expr::pow(
                Expr::pow(x(), Expr::num(2)),
                Expr::Num(super::observation::rational(1, 2)),
            ),
            rhs: Ok(x()),
            conditions: AssumptionSet::from_atoms([non_negative_atom()]),
            applicable: requires_non_negative,
        },
        GateRule {
            id: "pow.sqrt-square-real".to_string(),
            lhs: Expr::pow(
                Expr::pow(x(), Expr::num(2)),
                Expr::Num(super::observation::rational(1, 2)),
            ),
            rhs: Ok(fun("abs", vec![x()])),
            conditions: AssumptionSet::from_atoms([real_atom()]),
            applicable: requires_real_not_non_negative,
        },
        GateRule {
            id: "rat.cancel-x-over-x".to_string(),
            lhs: Expr::div(x(), x()),
            rhs: Ok(Expr::num(1)),
            conditions: AssumptionSet::from_atoms([non_zero_atom()]),
            applicable: always,
        },
        GateRule {
            id: "rat.cancel-zero-over-x".to_string(),
            lhs: Expr::div(Expr::num(0), x()),
            rhs: Ok(Expr::num(0)),
            conditions: AssumptionSet::from_atoms([non_zero_atom()]),
            applicable: always,
        },
        GateRule {
            id: "logexp.exp-log".to_string(),
            lhs: fun("exp", vec![fun("log", vec![x()])]),
            rhs: Ok(x()),
            conditions: AssumptionSet::from_atoms([finite_log_atom()]),
            applicable: always,
        },
        GateRule {
            id: "logexp.log-exp".to_string(),
            lhs: fun("log", vec![fun("exp", vec![x()])]),
            rhs: Ok(x()),
            conditions: empty.clone(),
            applicable: requires_strip,
        },
        GateRule {
            id: "abs.abs-square".to_string(),
            lhs: Expr::pow(fun("abs", vec![x()]), Expr::num(2)),
            rhs: Ok(Expr::pow(x(), Expr::num(2))),
            conditions: AssumptionSet::from_atoms([real_atom()]),
            applicable: requires_real,
        },
        GateRule {
            id: "abs.abs-neg".to_string(),
            lhs: fun("abs", vec![Expr::neg(x())]),
            rhs: Ok(fun("abs", vec![x()])),
            conditions: empty,
            applicable: always,
        },
    ]
}

/// The environment ladder a rule may be licensed under: its own conditions, those plus realness
/// of the probe, and realness alone. A rule licensed under no environment is reported unlicensed.
pub(crate) fn environments(rule: &GateRule) -> Vec<AssumptionSet> {
    fn consider(envs: &mut Vec<AssumptionSet>, candidate: AssumptionSet) {
        if !envs.contains(&candidate) {
            envs.push(candidate);
        }
    }
    let mut envs = Vec::new();
    consider(&mut envs, rule.conditions.clone());
    if let Ok(with_real) = rule.conditions.add(real_atom()) {
        consider(&mut envs, with_real);
    }
    if let Ok(real_only) = AssumptionSet::empty().add(real_atom()) {
        consider(&mut envs, real_only);
    }
    envs
}

/// Whether a rule's own applicability and conditions are discharged under an environment.
pub(crate) fn licensed(rule: &GateRule, env: &AssumptionSet) -> Result<(), String> {
    if !(rule.applicable)(env) {
        return Err("NotApplicable".to_string());
    }
    if rule.conditions.is_unsatisfiable() {
        return Err("conditions unsatisfiable".to_string());
    }
    for atom in rule.conditions.atoms() {
        if env.ask(atom) != Tristate::True {
            return Err(format!("condition not provable: {}", atom.describe()));
        }
    }
    Ok(())
}

/// Runs the registry gate: every rule, every licensed environment, every probe.
pub(crate) fn run_gate(rules: &[GateRule]) -> GateReport {
    let mut report = GateReport::default();
    for rule in rules {
        report.registered.push(rule.id.clone());
        let lhs = rule.lhs.clone();
        if !lhs.to_string().contains('x') {
            report.degenerate.push(format!(
                "{} (pattern instantiation lost the probe: {})",
                rule.id, lhs
            ));
        }
        let mut shapes: Vec<LicensedShape> = Vec::new();
        let mut licensed_any = false;
        for (env_index, env) in environments(rule).into_iter().enumerate() {
            if let Err(why) = licensed(rule, &env) {
                report.license_notes.push(format!(
                    "{} under {}: {}",
                    rule.id,
                    environment_text(&env),
                    why
                ));
                continue;
            }
            licensed_any = true;
            match &rule.rhs {
                Err(message) => {
                    report
                        .threw
                        .push(format!("{} replacement: {}", rule.id, message));
                }
                Ok(rhs) => {
                    shapes.push((env.clone(), lhs.clone(), rhs.clone()));
                    for probe in frozen() {
                        compare_point(
                            &mut report,
                            &rule.id,
                            env_index,
                            &probe.region,
                            &probe.label,
                            &real_num(&probe),
                            &lhs,
                            rhs,
                            &env,
                            false,
                        );
                    }
                    for probe in complex_probes() {
                        compare_point(
                            &mut report,
                            &rule.id,
                            env_index,
                            &probe.region,
                            &probe.label,
                            &complex_num(&probe),
                            &lhs,
                            rhs,
                            &env,
                            true,
                        );
                    }
                }
            }
        }
        if licensed_any {
            report.sampled.push(rule.id.clone());
        } else {
            report.unlicensed.push(rule.id.clone());
        }
        report.licensed_shapes.insert(rule.id.clone(), shapes);
    }
    report
}

/// The exact numeric value of one real probe point.
pub(crate) fn real_num(probe: &ProbePoint) -> Num {
    Num::Exact(probe.value.clone())
}

/// The exact numeric value of one complex probe point.
pub(crate) fn complex_num(probe: &ComplexProbe) -> Num {
    Num::Complex(Complex::new(
        Real::from_str(&probe.re).expect("an exact real part"),
        Real::from_str(&probe.im).expect("an exact imaginary part"),
    ))
}

/// Compares one point under one environment and records what the gate observed.
fn compare_point(
    report: &mut GateReport,
    rule_id: &str,
    env_index: usize,
    region: &str,
    label: &str,
    value: &Num,
    lhs: &Expr,
    rhs: &Expr,
    env: &AssumptionSet,
    complex: bool,
) {
    for atom in env.atoms() {
        if !atom_holds(atom, value) {
            return;
        }
    }
    let left = match default_eval(lhs, "x", value) {
        Ok(value) => value,
        Err(EvalFailure::Undefined(_)) => return,
        Err(failure) => {
            report.threw.push(format!(
                "{} at {}:{}: {}",
                rule_id,
                region,
                label,
                failure.describe()
            ));
            return;
        }
    };
    let right = match default_eval(rhs, "x", value) {
        Ok(value) => value,
        Err(EvalFailure::Undefined(_)) => return,
        Err(failure) => {
            report.threw.push(format!(
                "{} at {}:{}: {}",
                rule_id,
                region,
                label,
                failure.describe()
            ));
            return;
        }
    };
    if complex {
        report
            .complex_compared
            .push((rule_id.to_string(), env_index, label.to_string()));
    }
    report.count(rule_id, region);
    if exceeds_tolerance(&left, &right) {
        report.falsified.push(format!(
            "{} at {}:{}: {} != {}",
            rule_id, region, label, lhs, rhs
        ));
    }
}

/// Whether |lhs - rhs| reaches the tolerance; a complex difference is decided on its exact
/// squared magnitude.
pub(crate) fn exceeds_tolerance(left: &Num, right: &Num) -> bool {
    let tolerance = ComparisonTolerance::ten_to_the_minus_eleven();
    let delta = match left.sub(right, PRECISION) {
        Ok(value) => value,
        Err(_) => return true,
    };
    if let Num::Complex(complex) = &delta {
        let squared = Num::Real(complex.magnitude_squared());
        let tolerance_squared = match tolerance.value.mul(&tolerance.value, PRECISION) {
            Ok(value) => value,
            Err(_) => return true,
        };
        return num_cmp(&squared, &tolerance_squared) != Some(Ordering::Less);
    }
    num_cmp(&num_abs(&delta), &tolerance.value) != Some(Ordering::Less)
}

/// Whether every atom of the environment holds at this numeric point.
pub(crate) fn atom_holds(atom: &Assumption, value: &Num) -> bool {
    match atom {
        Assumption::SymbolDomain(symbol, domain) => {
            if symbol != "x" {
                return false;
            }
            match domain {
                Domain::Integer | Domain::Rational | Domain::Real => {
                    !matches!(value, Num::Complex(_))
                }
                Domain::Complex => true,
            }
        }
        // A relation atom is not decidable at a point by this guard (the reference's default arm).
        Assumption::SymbolRelation(_, _, _) => false,
        Assumption::SymbolProperty(symbol, predicate) => {
            symbol == "x" && predicate_holds(*predicate, value)
        }
        Assumption::ExpressionProperty(expression, predicate) => {
            match default_eval(expression, "x", value) {
                Ok(evaluated) => predicate_holds(*predicate, &evaluated),
                Err(_) => false,
            }
        }
        Assumption::Interval(expression, lower, lower_open, upper, upper_open) => {
            let subject = match default_eval(expression, "x", value) {
                Ok(value) => value,
                Err(_) => return false,
            };
            if matches!(subject, Num::Complex(_)) {
                return false;
            }
            if let Some(bound) = lower {
                let bound_value = match default_eval(bound, "x", value) {
                    Ok(value) => value,
                    Err(_) => return false,
                };
                let Some(ordering) = num_cmp(&subject, &bound_value) else {
                    return false;
                };
                if ordering == Ordering::Less || (ordering == Ordering::Equal && *lower_open) {
                    return false;
                }
            }
            if let Some(bound) = upper {
                let bound_value = match default_eval(bound, "x", value) {
                    Ok(value) => value,
                    Err(_) => return false,
                };
                let Some(ordering) = num_cmp(&subject, &bound_value) else {
                    return false;
                };
                if ordering == Ordering::Greater || (ordering == Ordering::Equal && *upper_open) {
                    return false;
                }
            }
            true
        }
    }
}

/// The reference's predicate table at one numeric point.
pub(crate) fn predicate_holds(predicate: SymbolPredicate, value: &Num) -> bool {
    let sign = num_cmp(value, &Num::Exact(Rat::zero()));
    match predicate {
        SymbolPredicate::Finite => true,
        SymbolPredicate::NonZero => !num_is_zero(value),
        SymbolPredicate::Positive => sign == Some(Ordering::Greater),
        SymbolPredicate::NonNegative => matches!(sign, Some(Ordering::Greater | Ordering::Equal)),
        SymbolPredicate::Negative => sign == Some(Ordering::Less),
        SymbolPredicate::NonPositive => matches!(sign, Some(Ordering::Less | Ordering::Equal)),
        // Even and Odd are not decidable at these points.
        _ => false,
    }
}

/// How one registered rule is treated in the complex region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ComplexDisposition {
    /// At least one complex probe satisfied every assumption atom and was compared.
    ComplexSampled,
    /// No complex probe was compared; every skip carries a named atom and a counterexample was seen.
    ComplexExclusionProven,
}

/// What the complex-treatment audit observed for one rule.
#[derive(Clone, Debug)]
pub(crate) struct ComplexTreatment {
    /// The rule's stable id.
    pub(crate) rule_id: String,
    /// The disposition.
    pub(crate) disposition: ComplexDisposition,
    /// Complex sweep points the gate actually compared for this rule.
    pub(crate) sampled_points: usize,
    /// Licensed environments the gate used.
    pub(crate) envs_considered: usize,
    /// (environment, witness) pairs skipped because a named atom failed.
    pub(crate) skips_attributed_to_named_atom: usize,
    /// Pairs not compared because the identity is not defined there.
    pub(crate) skips_attributed_to_domain: usize,
    /// Pairs skipped with no stated reason.
    pub(crate) unexplained_skips: usize,
    /// The unexplained skip descriptions.
    pub(crate) unexplained_detail: Vec<String>,
    /// The exclusion witness's label.
    pub(crate) witness_label: String,
    /// The exclusion witness's region.
    pub(crate) witness_region: String,
    /// The exclusion witness's point rendering.
    pub(crate) witness_point: String,
    /// The exclusion witness's environment rendering.
    pub(crate) witness_env: String,
    /// The atoms that blocked the guard at the witness.
    pub(crate) blocking_atoms: Vec<String>,
    /// The left-hand rendering at the witness.
    pub(crate) witness_lhs: String,
    /// The right-hand rendering at the witness.
    pub(crate) witness_rhs: String,
    /// The left-hand value at the witness.
    pub(crate) witness_lhs_value: String,
    /// The right-hand value at the witness.
    pub(crate) witness_rhs_value: String,
    /// |lhs - rhs|^2 at the witness.
    pub(crate) witness_delta_squared: Option<Num>,
}

impl ComplexTreatment {
    fn new(rule_id: &str) -> ComplexTreatment {
        ComplexTreatment {
            rule_id: rule_id.to_string(),
            disposition: ComplexDisposition::ComplexExclusionProven,
            sampled_points: 0,
            envs_considered: 0,
            skips_attributed_to_named_atom: 0,
            skips_attributed_to_domain: 0,
            unexplained_skips: 0,
            unexplained_detail: Vec::new(),
            witness_label: String::new(),
            witness_region: String::new(),
            witness_point: String::new(),
            witness_env: String::new(),
            blocking_atoms: Vec::new(),
            witness_lhs: String::new(),
            witness_rhs: String::new(),
            witness_lhs_value: String::new(),
            witness_rhs_value: String::new(),
            witness_delta_squared: None,
        }
    }
}

/// The complex half of the gate, kept out of the gate's verdicts.
pub(crate) fn complex_treatment_audit(
    rules: &[GateRule],
    report: &GateReport,
) -> Vec<ComplexTreatment> {
    let tolerance = ComparisonTolerance::ten_to_the_minus_eleven();
    let tolerance_squared = tolerance
        .value
        .mul(&tolerance.value, PRECISION)
        .expect("a real tolerance");
    let mut treatments = Vec::new();
    for rule in rules {
        let mut treatment = ComplexTreatment::new(&rule.id);
        let Some(shapes) = report.licensed_shapes.get(&rule.id) else {
            continue;
        };
        treatment.envs_considered = shapes.len();
        treatment.sampled_points = report
            .complex_compared
            .iter()
            .filter(|(id, _, _)| id == &rule.id)
            .count();

        for (env_index, (env, lhs, rhs)) in shapes.iter().enumerate() {
            for probe in complex_probes() {
                let value = complex_num(&probe);
                let blocked: Vec<String> = env
                    .atoms()
                    .iter()
                    .filter(|atom| !atom_holds(atom, &value))
                    .map(|atom| atom.describe())
                    .collect();
                if !blocked.is_empty() {
                    treatment.skips_attributed_to_named_atom += 1;
                    continue;
                }
                if report.complex_compared.contains(&(
                    rule.id.clone(),
                    env_index,
                    probe.label.clone(),
                )) {
                    continue;
                }
                if evaluate_pair(lhs, rhs, &value).is_none() {
                    treatment.skips_attributed_to_domain += 1;
                    continue;
                }
                treatment.unexplained_skips += 1;
                treatment.unexplained_detail.push(format!(
                    "{} at {} (every atom held)",
                    environment_text(env),
                    probe.label
                ));
            }
        }

        if treatment.sampled_points == 0 {
            'witness: for (env, lhs, rhs) in shapes.iter() {
                for witness in complex_exclusion_witnesses() {
                    let value = complex_num(&witness);
                    let blocking: Vec<String> = env
                        .atoms()
                        .iter()
                        .filter(|atom| !atom_holds(atom, &value))
                        .map(|atom| atom.describe())
                        .collect();
                    if blocking.is_empty() {
                        continue;
                    }
                    let Some((left, right)) = evaluate_pair(lhs, rhs, &value) else {
                        continue;
                    };
                    let delta = left.sub(&right, PRECISION).expect("a subtraction");
                    let delta_squared = num_squared_magnitude(&delta);
                    if num_cmp(&delta_squared, &tolerance_squared) == Some(Ordering::Less) {
                        continue;
                    }
                    treatment.witness_label = witness.label.clone();
                    treatment.witness_region = witness.region.clone();
                    treatment.witness_point = format!(
                        "z = {} (re={}, im={})",
                        witness.label, witness.re, witness.im
                    );
                    treatment.witness_env = environment_text(env);
                    treatment.blocking_atoms = blocking;
                    treatment.witness_lhs = lhs.to_string();
                    treatment.witness_rhs = rhs.to_string();
                    treatment.witness_lhs_value = num_text(&left);
                    treatment.witness_rhs_value = num_text(&right);
                    treatment.witness_delta_squared = Some(delta_squared);
                    break 'witness;
                }
            }
        }
        treatment.disposition = if treatment.sampled_points > 0 {
            ComplexDisposition::ComplexSampled
        } else {
            ComplexDisposition::ComplexExclusionProven
        };
        treatments.push(treatment);
    }
    treatments
}

/// Both sides at one complex point, or None when the identity is not defined there.
fn evaluate_pair(lhs: &Expr, rhs: &Expr, value: &Num) -> Option<(Num, Num)> {
    let left = default_eval(lhs, "x", value).ok()?;
    let right = default_eval(rhs, "x", value).ok()?;
    Some((left, right))
}
