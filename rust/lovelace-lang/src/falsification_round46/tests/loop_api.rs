//! FalsificationLoopTests.cs: the thirteen loop-API rows.

use std::collections::BTreeSet;

use lovelace_symbolics::assumptions::Assumption;

use crate::{Engine, Value};

use super::*;

const PLANTED_RULE_ID: &str = "test.control-narrow-band-rule";

/// Where the planted defect's band sits relative to a coarse uniform grid.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FalsificationBandAlignment {
    /// The band contains a coarse-grid point.
    AlignedWithTheCoarseGrid,
    /// The band contains no coarse-grid point at the comparison grid.
    OffGrid,
}

fn band(alignment: FalsificationBandAlignment) -> (Rat, Rat) {
    match alignment {
        FalsificationBandAlignment::AlignedWithTheCoarseGrid => (rational(5, 4), rational(63, 50)),
        FalsificationBandAlignment::OffGrid => (rational(161, 128), rational(331, 256)),
    }
}

fn planted_band_shape(alignment: FalsificationBandAlignment, value: &Rat) -> RuleShape {
    let (low, high) = band(alignment);
    let environment = AssumptionSet::from_atoms([Assumption::Interval(
        x(),
        Some(Expr::Num(low.clone())),
        false,
        Some(Expr::Num(high.clone())),
        false,
    )]);
    let right = if value >= &low && value <= &high {
        Expr::add(x(), x())
    } else {
        x()
    };
    RuleShape {
        environment,
        left: x(),
        right,
    }
}

fn classify_planted(
    alignment: FalsificationBandAlignment,
    value: &Rat,
    why: &str,
) -> FalsificationProbe {
    let shape = planted_band_shape(alignment, value);
    let point = ProbePoint {
        region: "hypothesis".to_string(),
        label: rat_text(value),
        value: value.clone(),
        why: why.to_string(),
    };
    classify_at(PLANTED_RULE_ID, &shape, "x", &point, &tolerance()).1
}

struct PlantedRun {
    alignment: FalsificationBandAlignment,
    probes: Vec<FalsificationProbe>,
    defects: Vec<String>,
    classes: Vec<ObservationClass>,
    values: Vec<Rat>,
    keys: Vec<String>,
}

impl PlantedRun {
    fn new(alignment: FalsificationBandAlignment) -> PlantedRun {
        PlantedRun {
            alignment,
            probes: Vec::new(),
            defects: Vec::new(),
            classes: Vec::new(),
            values: Vec::new(),
            keys: Vec::new(),
        }
    }

    fn take(&mut self, value: &Rat, why: &str) {
        let key = rat_text(value);
        if self.keys.contains(&key) {
            return;
        }
        self.keys.push(key.clone());
        let probe = classify_planted(self.alignment, value, why);
        self.probes.push(probe.clone());
        self.classes.push(probe.class);
        self.values.push(value.clone());
        if probe.is_defect() {
            self.defects.push(format!(
                "{PLANTED_RULE_ID} at {key} threw an unexpected exception"
            ));
        }
    }
}

fn run_planted_band(alignment: FalsificationBandAlignment, seed: i64) -> FalsificationReport {
    let mut run = PlantedRun::new(alignment);
    for point in all() {
        run.take(&point.value, &point.why);
    }

    let (band_low, band_high) = band(alignment);
    let band_why = format!(
        "an approach to the planted band ({}, {})",
        rat_text(&band_low),
        rat_text(&band_high)
    );
    let approaches = [
        band_low.add(&Rat::from_i64(-1)),
        band_low.add(&rational(-1, 16)),
        band_high.add(&rational(1, 16)),
        band_high.add(&Rat::from_i64(1)),
        band_low
            .add(&band_high)
            .div(&Rat::from_i64(2))
            .expect("a midpoint"),
    ];
    for approach in approaches {
        run.take(&approach, &band_why);
    }

    let mut bias = BoundaryBias::new(BoundaryBiasSpec::default_spec(seed));
    let mut index = 0usize;
    while index + 1 < run.classes.len() {
        let first = run.classes[index];
        let second = run.classes[index + 1];
        let low = run.values[index].clone();
        let high = run.values[index + 1].clone();
        index += 1;
        if first == second || low >= high {
            continue;
        }
        for candidate in bias.broad_phase(&low, &high).unwrap_or_default() {
            let why = format!(
                "boundary bias between {} ({}) and {} ({})",
                rat_text(&low),
                first.name(),
                rat_text(&high),
                second.name()
            );
            run.take(&candidate, &why);
        }
        let boundary = {
            let mut classify = |value: &Rat| -> ObservationClass {
                classify_planted(alignment, value, "bisection midpoint").class
            };
            bias.narrow_phase(&low, &high, first, &mut classify)
        };
        for midpoint in boundary.1 {
            run.take(&midpoint, "bisection midpoint");
        }
    }

    let mut probes = run.probes;
    probes.sort_by(|left, right| (&left.region, &left.input).cmp(&(&right.region, &right.input)));
    FalsificationReport {
        generator_version: GENERATOR_VERSION.to_string(),
        sample_count: probes.len(),
        probes,
        defects: run.defects,
    }
}

#[test]
fn frozen_battery_is_a_prefix_of_what_the_substrate_offers_in_its_own_order() {
    let all_probes = all();
    let frozen_probes = frozen();
    assert_eq!(frozen_probes.len(), 33);
    for (index, probe) in frozen_probes.iter().enumerate() {
        assert_eq!(all_probes[index].region, probe.region);
        assert_eq!(all_probes[index].label, probe.label);
        assert_eq!(all_probes[index].value, probe.value);
        assert_eq!(all_probes[index].why, probe.why);
    }

    let frozen_values: Vec<Rat> = frozen_probes
        .iter()
        .map(|probe| probe.value.clone())
        .collect();
    let mut cursor = 0usize;
    for legacy in legacy_points() {
        while cursor < frozen_values.len() && frozen_values[cursor] != legacy {
            cursor += 1;
        }
        assert!(
            cursor < frozen_values.len(),
            "legacy point {legacy} is missing from the frozen probe set."
        );
        cursor += 1;
    }

    assert!(all_probes.iter().all(|probe| !probe.why.trim().is_empty()));
    let generated_probes = generated();
    assert_eq!(generated_probes.len(), 27);
    assert_eq!(all_probes.len(), 60);
    assert!(
        generated_probes
            .iter()
            .all(|probe| probe.region == GENERATED_REGION)
    );
    let frozen_pairs: Vec<(String, String)> = frozen_probes
        .iter()
        .map(|probe| (probe.region.clone(), probe.label.clone()))
        .collect();
    assert!(
        generated_probes
            .iter()
            .all(|probe| !frozen_pairs.contains(&(probe.region.clone(), probe.label.clone())))
    );
    let distinct_pairs: BTreeSet<(String, String)> = generated_probes
        .iter()
        .map(|probe| (probe.region.clone(), probe.label.clone()))
        .collect();
    assert_eq!(distinct_pairs.len(), generated_probes.len());
    let distinct_values: BTreeSet<String> = generated_probes
        .iter()
        .map(|probe| rat_text(&probe.value))
        .collect();
    assert_eq!(distinct_values.len(), generated_probes.len());
    assert!(
        generated_probes
            .iter()
            .all(|probe| !probe.why.trim().is_empty())
    );
    let frozen_distinct: BTreeSet<(String, String)> = frozen_pairs.into_iter().collect();
    assert_eq!(frozen_distinct.len(), 33);
}

#[test]
fn classifier_names_each_class_and_never_folds_a_defect_into_one() {
    let zero = Num::Exact(Rat::zero());
    assert_eq!(
        ObservationClassifier::classify(&x(), &x(), "x", &zero, &tolerance()).class,
        ObservationClass::Agree
    );

    let disagree = ObservationClassifier::classify(
        &x(),
        &Expr::add(x(), Expr::num(1)),
        "x",
        &zero,
        &tolerance(),
    );
    assert_eq!(disagree.class, ObservationClass::Disagree);
    assert!(disagree.is_counterexample());

    let reciprocal = Expr::div(Expr::num(1), x());
    let undefined_left =
        ObservationClassifier::classify(&reciprocal, &x(), "x", &zero, &tolerance());
    assert_eq!(undefined_left.class, ObservationClass::UndefinedLeft);
    assert_eq!(undefined_left.left.outcome, SideOutcome::Undefined);
    assert_eq!(undefined_left.right.outcome, SideOutcome::Evaluated);

    let undefined_right =
        ObservationClassifier::classify(&x(), &reciprocal, "x", &zero, &tolerance());
    assert_eq!(undefined_right.class, ObservationClass::UndefinedRight);

    let unregistered = fun("no_such_function_at_all", vec![x()]);
    let unregistered_observation =
        ObservationClassifier::classify(&unregistered, &x(), "x", &zero, &tolerance());
    assert_eq!(
        unregistered_observation.left.outcome,
        SideOutcome::Undefined
    );
    assert_eq!(
        unregistered_observation.class,
        ObservationClass::UndefinedLeft
    );
    assert!(!unregistered_observation.is_defect());

    let threw = ClassifiedObservation {
        class: ObservationClass::BudgetOrRefusal,
        left: SideObservation {
            outcome: SideOutcome::Threw,
            value: None,
            detail: "InvalidOperationException: probe".to_string(),
        },
        right: SideObservation {
            outcome: SideOutcome::Evaluated,
            value: Some(zero),
            detail: String::new(),
        },
        delta_text: None,
    };
    assert!(threw.is_defect());
    assert!(!threw.is_counterexample());

    assert!(ObservationClassifier::is_typed_resource_refusal(
        &EvalFailure::Refused("probe".to_string())
    ));
    assert!(!ObservationClassifier::is_typed_resource_refusal(
        &EvalFailure::Undefined("a domain refusal is not a budget".to_string())
    ));
    assert!(!ObservationClassifier::is_typed_resource_refusal(
        &EvalFailure::Threw("nor is an ordinary failure".to_string())
    ));
}

#[test]
fn tolerance_is_a_parameter_the_same_values_decide_differently_under_different_tolerances() {
    let lhs = Expr::add(x(), Expr::Num(rational(1, 1000)));
    let value = Num::Exact(Rat::zero());
    let tight = ObservationClassifier::classify(
        &lhs,
        &x(),
        "x",
        &value,
        &ComparisonTolerance::of_rational(rational(1, 10_000)),
    );
    let loose = ObservationClassifier::classify(
        &lhs,
        &x(),
        "x",
        &value,
        &ComparisonTolerance::of_rational(rational(1, 100)),
    );
    assert_eq!(tight.class, ObservationClass::Disagree);
    assert_eq!(loose.class, ObservationClass::Agree);
    assert!(
        tight
            .delta_text
            .as_deref()
            .unwrap_or_default()
            .contains("1/1000")
    );
    assert_eq!(
        ComparisonTolerance::of_rational(rational(1, 10_000)).text,
        "1/10000"
    );
}

#[test]
fn probe_record_round_trips_through_its_deterministic_text_form() {
    let why = "a rationale with a | pipe, a \\ backslash and\n a newline".to_string();
    let original = FalsificationProbe {
        rule_id: "rule.with|a-pipe".to_string(),
        region: "near-pole".to_string(),
        label: "x = 1/10^6".to_string(),
        input: "1/1000000".to_string(),
        is_complex: false,
        class: ObservationClass::Disagree,
        left_outcome: SideOutcome::Evaluated,
        right_outcome: SideOutcome::Evaluated,
        left: "sqrt(x^2)".to_string(),
        right: "|x|".to_string(),
        left_value: "1/1000000".to_string(),
        right_value: "1/1000000".to_string(),
        delta_text: "|delta| = 0".to_string(),
        precision: 40,
        tolerance: "1/100000000000".to_string(),
        environment: "{x > 0}".to_string(),
        generator_version: GENERATOR_VERSION.to_string(),
        why: why.clone(),
    };
    let text = original.to_text();
    let parsed = FalsificationProbe::parse(&text).expect("the probe round-trips");
    assert_eq!(original, parsed);
    assert_eq!(text, parsed.to_text());
    assert_eq!(parsed.why, why);
    assert!(text.contains("near-pole"));

    let report = FalsificationReport {
        generator_version: GENERATOR_VERSION.to_string(),
        sample_count: 1,
        probes: vec![original],
        defects: vec!["a defect | with a pipe".to_string()],
    };
    let reparsed = FalsificationReport::parse(&report.to_text()).expect("the report round-trips");
    assert_eq!(report.fingerprint(), reparsed.fingerprint());
    assert_eq!(
        report.counterexamples().len(),
        reparsed.counterexamples().len()
    );
    assert_eq!(report.defects, reparsed.defects);
}

#[test]
fn loop_is_deterministic_same_seed_produces_a_byte_identical_report() {
    let first = run_planted_band(
        FalsificationBandAlignment::AlignedWithTheCoarseGrid,
        20_260_916,
    );
    let second = run_planted_band(
        FalsificationBandAlignment::AlignedWithTheCoarseGrid,
        20_260_916,
    );
    assert_eq!(first.fingerprint(), second.fingerprint());
    assert_eq!(first.to_text(), second.to_text());
    assert_eq!(first.sample_count, second.sample_count);
    let other_seed = run_planted_band(FalsificationBandAlignment::OffGrid, 20_260_917);
    assert_ne!(first.fingerprint(), other_seed.fingerprint());
}

fn run_fingerprint_in_child_process(executable: &std::path::Path) -> String {
    let output = std::process::Command::new(executable)
        .args([
            "--exact",
            "falsification_round46::tests::loop_api::falsification_fingerprint_child",
            "--nocapture",
        ])
        .output()
        .expect("the fingerprint child runs");
    assert!(
        output.status.success(),
        "the child fingerprint run exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .find(|line| line.contains("FALSIFIER-FINGERPRINT="))
        .map(|line| line.trim().to_string())
        .unwrap_or_else(|| panic!("the child printed no fingerprint line: {stdout}"))
}

#[test]
fn loop_is_deterministic_across_processes() {
    let executable = std::env::current_exe().expect("the test binary is on disk");
    assert!(executable.exists(), "the test binary must be on disk");
    let first = run_fingerprint_in_child_process(&executable);
    let second = run_fingerprint_in_child_process(&executable);
    assert_eq!(first, second);
    assert!(first.starts_with("FALSIFIER-FINGERPRINT="));
}

#[test]
fn falsification_fingerprint_child() {
    let report = run_planted_band(FalsificationBandAlignment::OffGrid, 20_260_916);
    println!("FALSIFIER-FINGERPRINT={}", report.fingerprint());
}

#[test]
fn generator_brackets_a_known_boundary_the_natural_subtraction_widening() {
    let kind_of = |right: i64| -> ObservationClass {
        let mut engine = Engine::new();
        let value = engine
            .eval(&format!("5 - {right}"))
            .expect("the engine evaluates the subtraction");
        if matches!(value, Value::Natural(_)) {
            ObservationClass::Agree
        } else {
            ObservationClass::Disagree
        }
    };
    for right in 0..=5 {
        assert_eq!(kind_of(right), ObservationClass::Agree);
    }
    for right in 6..=10 {
        assert_eq!(kind_of(right), ObservationClass::Disagree);
    }

    let mut bias = BoundaryBias::new(BoundaryBiasSpec::default_spec(20_260_916));
    let offered = bias
        .broad_phase(&rational(0, 1), &rational(12, 1))
        .expect("a valid bracket");
    assert!(!offered.is_empty());
    for point in &offered {
        assert!(
            *point > rational(0, 1) && *point < rational(12, 1),
            "a generated point must lie strictly inside its bracket: got {}",
            rat_text(point)
        );
    }

    let low = rational(5, 1);
    let high = rational(6, 1);
    let mut at = |point: &Rat| -> ObservationClass {
        match point.to_i64() {
            Some(right) => kind_of(right),
            None => ObservationClass::Agree,
        }
    };
    assert_eq!(at(&low), ObservationClass::Agree);
    assert_eq!(at(&rational(11, 2)), ObservationClass::Agree);
    let (boundary, midpoints) = bias.narrow_phase(&low, &high, ObservationClass::Agree, &mut at);
    assert_eq!(boundary, high);
    assert_eq!(at(&boundary), ObservationClass::Disagree);
    assert!(midpoints.is_empty());
}

#[test]
fn generator_brackets_a_non_integral_boundary_and_every_point_is_strictly_inside() {
    let mut bias = BoundaryBias::new(BoundaryBiasSpec {
        seed: 20_260_916,
        ladder: BiasStepLadder::default_ladder(),
        samples_per_level: 3,
        max_narrow_steps: 64,
    });
    let third = rational(1, 3);
    for step in [2_i64, 3] {
        let low = rational(0, 12);
        let high = rational(step, 12);
        let offered = bias.broad_phase(&low, &high).expect("a valid bracket");
        for candidate in &offered {
            assert!(
                *candidate > low && *candidate < high,
                "step {step}: {} must lie strictly inside ({}, {})",
                rat_text(candidate),
                rat_text(&low),
                rat_text(&high)
            );
        }
        let mut side = |value: &Rat| -> ObservationClass {
            if value < &third {
                ObservationClass::Agree
            } else {
                ObservationClass::Disagree
            }
        };
        let (boundary, midpoints) =
            bias.narrow_phase(&low, &high, ObservationClass::Agree, &mut side);
        assert!(
            boundary >= low && boundary <= high,
            "step {step}: the returned bound must stay inside its bracket; got {}",
            rat_text(&boundary)
        );
        assert!(
            midpoints
                .iter()
                .all(|midpoint| *midpoint >= low && *midpoint <= high)
        );
        assert!(
            midpoints
                .iter()
                .all(|midpoint| side(midpoint) == ObservationClass::Agree
                    || side(midpoint) == ObservationClass::Disagree)
        );
    }
}

#[test]
fn generator_broad_phase_sequence_is_stable_under_the_same_seed_and_moves_with_it() {
    let first = BoundaryBias::new(BoundaryBiasSpec::default_spec(7))
        .broad_phase(&Rat::from_i64(-2), &Rat::from_i64(3))
        .expect("a valid bracket");
    let second = BoundaryBias::new(BoundaryBiasSpec::default_spec(7))
        .broad_phase(&Rat::from_i64(-2), &Rat::from_i64(3))
        .expect("a valid bracket");
    assert_eq!(
        first.iter().map(rat_text).collect::<Vec<String>>(),
        second.iter().map(rat_text).collect::<Vec<String>>()
    );
    let other = BoundaryBias::new(BoundaryBiasSpec::default_spec(8))
        .broad_phase(&Rat::from_i64(-2), &Rat::from_i64(3))
        .expect("a valid bracket");
    assert_ne!(
        first
            .iter()
            .map(rat_text)
            .collect::<Vec<String>>()
            .join(","),
        other
            .iter()
            .map(rat_text)
            .collect::<Vec<String>>()
            .join(",")
    );
}

#[test]
fn planted_defect_is_falsified_and_the_report_names_the_region_and_the_input() {
    let report = run_planted_band(
        FalsificationBandAlignment::AlignedWithTheCoarseGrid,
        20_260_916,
    );
    assert!(!report.counterexamples().is_empty());
    assert!(report.defects.is_empty());
    let first = report.counterexamples()[0];
    assert_eq!(first.rule_id, PLANTED_RULE_ID);
    assert!(!first.region.trim().is_empty());
    assert!(!first.input.trim().is_empty());
    assert!(!first.why.trim().is_empty());
    assert_eq!(first.class, ObservationClass::Disagree);
    assert!(report.failed());
}

#[test]
fn registry_loop_classifies_every_shipped_rule_and_finds_no_defect() {
    let mut lines = Vec::new();
    let mut observed: BTreeSet<ObservationClass> = BTreeSet::new();
    let spec = BoundaryBiasSpec::default_spec(20_260_916);
    let mut rules = gate_registry();
    rules.sort_by(|left, right| left.id.cmp(&right.id));
    for rule in &rules {
        let shape = RuleShape {
            environment: rule.conditions.clone(),
            left: rule.lhs.clone(),
            right: rule.rhs.clone().expect("a shipped replacement"),
        };
        let report = FalsificationLoop::run(&rule.id, &shape, "x", &all(), &spec, &tolerance());
        lines.push(format!(
            "{} samples={} counterexamples={}",
            rule.id,
            report.sample_count,
            report.counterexamples().len()
        ));
        for probe in &report.probes {
            observed.insert(probe.class);
        }
        assert!(report.defects.is_empty(), "{}", report.defects.join(" | "));
        assert!(
            report.counterexamples().is_empty(),
            "{}",
            report
                .counterexamples()
                .iter()
                .map(|probe| probe.to_text())
                .collect::<Vec<String>>()
                .join(" | ")
        );
    }
    assert_eq!(lines.len(), 9);
    assert!(observed.contains(&ObservationClass::Agree));

    let one_over_x = Expr::div(Expr::num(1), x());
    let at_zero = ObservationClassifier::classify(
        &one_over_x,
        &x(),
        "x",
        &Num::Exact(Rat::zero()),
        &tolerance(),
    );
    assert_eq!(at_zero.class, ObservationClass::UndefinedLeft);
    observed.insert(at_zero.class);
    assert!(observed.contains(&ObservationClass::UndefinedLeft));
}

fn measure_at_seed(seed: i64, budget: usize) -> (i64, i64, Rat, Rat) {
    let (band_low, band_high) = band(FalsificationBandAlignment::OffGrid);
    let low = Rat::from_i64(-2);
    let high = Rat::from_i64(3);
    let gap = |value: &Rat| -> Rat {
        if value < &band_low {
            band_low.add(&value.neg())
        } else if value > &band_high {
            value.add(&band_high.neg())
        } else {
            Rat::zero()
        }
    };
    let classify = |value: &Rat, why: &str| -> ObservationClass {
        classify_planted(FalsificationBandAlignment::OffGrid, value, why).class
    };

    // ---- uniform arm: one exact grid over the whole interval, in order ----
    let uniform_step = rational(1, 12);
    let mut uniform = -1_i64;
    let mut uniform_gap = high.add(&low.neg());
    let mut calls = 0usize;
    for k in 0..=60 {
        if calls >= budget {
            break;
        }
        let value = low.add(&uniform_step.mul(&Rat::from_i64(k)));
        if value > high {
            break;
        }
        calls += 1;
        let current_gap = gap(&value);
        if current_gap < uniform_gap {
            uniform_gap = current_gap;
        }
        if classify(&value, "uniform grid point") == ObservationClass::Disagree {
            uniform = calls as i64;
            break;
        }
    }

    // ---- biased arm: a seeded coarse ladder, then the generator inside the tightest bracket ----
    let mut biased = -1_i64;
    let mut biased_gap = high.add(&low.neg());
    calls = 0;
    let mut rng = DeterministicRng::new(seed);
    let mut bracket_low = low.clone();
    let mut bracket_high = high.clone();
    for step in [4_i64, 8, 16] {
        if biased >= 0 || calls >= budget {
            break;
        }
        let jitter = rng.next_int(step as usize);
        for k in 1..=3 {
            if calls >= budget {
                break;
            }
            let index = (k as i64) * step - jitter as i64;
            if index <= 0 || index >= step * 4 {
                continue;
            }
            let value = low.add(&Rat::from_i64(4).mul(&rational(index, step)));
            calls += 1;
            let current_gap = gap(&value);
            if current_gap < biased_gap {
                biased_gap = current_gap;
            }
            if classify(&value, &format!("biased ladder point at step {step}"))
                == ObservationClass::Disagree
            {
                biased = calls as i64;
                break;
            }
            if value > bracket_low && value < band_low {
                bracket_low = value.clone();
            }
            if value < bracket_high && value > band_high {
                bracket_high = value.clone();
            }
        }
    }

    if biased < 0 && calls < budget && bracket_low < bracket_high {
        let mut bias = BoundaryBias::new(BoundaryBiasSpec::default_spec(seed));
        for candidate in bias
            .broad_phase(&bracket_low, &bracket_high)
            .unwrap_or_default()
        {
            if calls >= budget {
                break;
            }
            calls += 1;
            let current_gap = gap(&candidate);
            if current_gap < biased_gap {
                biased_gap = current_gap;
            }
            if classify(&candidate, "boundary-bias candidate") == ObservationClass::Disagree {
                biased = calls as i64;
                break;
            }
        }
    }

    (biased, uniform, biased_gap, uniform_gap)
}

#[test]
fn hypothesis_boundary_bias_beats_uniform_on_a_planted_defect() {
    let seeds = [1_i64, 2, 3, 5, 8, 13];
    let budget = 30usize;
    let mut biased_wins = 0;
    let mut uniform_wins = 0;
    let mut ties = 0;
    let mut misses = 0;
    let mut rows = Vec::new();
    for seed in seeds {
        let (biased, uniform, biased_gap, uniform_gap) = measure_at_seed(seed, budget);
        rows.push(format!(
            "seed={seed} biased={biased} uniform={uniform} biasedGap={} uniformGap={}",
            rat_text(&biased_gap),
            rat_text(&uniform_gap)
        ));
        if biased < 0 && uniform < 0 {
            misses += 1;
            continue;
        }
        if biased >= 0 && uniform < 0 {
            biased_wins += 1;
        } else if uniform >= 0 && biased < 0 {
            uniform_wins += 1;
        } else if biased < uniform {
            biased_wins += 1;
        } else if uniform < biased {
            uniform_wins += 1;
        } else {
            ties += 1;
        }
    }
    assert_eq!(biased_wins, 0);
    assert_eq!(uniform_wins, 0);
    assert_eq!(misses, 6);
    assert!(
        biased_wins <= uniform_wins,
        "HYPOTHESIS now HOLDS and this pinned verdict must be updated: biasedWins={biased_wins}, uniformWins={uniform_wins}, ties={ties}. {}",
        rows.join(" | ")
    );
}
