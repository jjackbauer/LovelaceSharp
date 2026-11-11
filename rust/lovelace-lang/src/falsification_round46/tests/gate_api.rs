//! FalsificationGateTests.cs: the nine gate-API rows.

use std::cmp::Ordering;

use super::*;

#[test]
fn registry_gate_every_registered_rule_is_sampled_on_every_region() {
    let rules = gate_registry();
    let report = run_gate(&rules);
    assert_eq!(rules.len(), 9);
    assert_eq!(report.registered.len(), 9);
    assert_eq!(report.sampled.len(), 9);
    let distinct: std::collections::BTreeSet<&String> = report.sampled.iter().collect();
    assert_eq!(distinct.len(), 9);
    assert!(
        report.uninstantiable.is_empty(),
        "{:?}",
        report.uninstantiable
    );
    assert!(report.unlicensed.is_empty(), "{:?}", report.unlicensed);
    assert!(report.degenerate.is_empty(), "{:?}", report.degenerate);
    assert!(report.threw.is_empty(), "{:?}", report.threw);
    assert!(report.falsified.is_empty(), "{:?}", report.falsified);
    assert!(!report.failed(), "{}", report.summary());
    for rule in &rules {
        assert!(
            report.compared.get(&rule.id).copied().unwrap_or(0) > 0,
            "rule '{}' was registered but no sample point was compared for it. {}",
            rule.id,
            report.summary()
        );
    }
    let covered = report.all_covered_regions();
    for region in required_regions() {
        assert!(
            covered.contains(region),
            "region '{region}' contributed no compared point. {}",
            report.summary()
        );
    }
}

#[test]
fn gate_fails_when_a_rule_throws() {
    let throwing = GateRule {
        id: "test.control-throwing-rule".to_string(),
        lhs: x(),
        rhs: Err(
            "InvalidOperationException: deliberate negative control: replacement throws"
                .to_string(),
        ),
        conditions: AssumptionSet::empty(),
        applicable: |_| true,
    };
    let report = run_gate(&[throwing]);
    assert!(
        !report.threw.is_empty(),
        "the gate must report failure when a rule throws; it reported none. {}",
        report.summary()
    );
    assert!(report.failed(), "{}", report.summary());
    assert!(
        report
            .threw
            .iter()
            .any(|line| line.contains("test.control-throwing-rule")
                && line.contains("InvalidOperationException")),
        "{:?}",
        report.threw
    );
    assert_eq!(report.sampled.len(), 1);
}

#[test]
fn gate_reports_falsification_when_a_rule_is_wrong() {
    let wrong = GateRule {
        id: "test.control-wrong-rule".to_string(),
        lhs: Expr::pow(Expr::pow(x(), Expr::num(2)), Expr::Num(rational(1, 2))),
        rhs: Ok(x()),
        conditions: AssumptionSet::empty(),
        applicable: |_| true,
    };
    let report = run_gate(&[wrong]);
    assert!(
        !report.falsified.is_empty(),
        "a wrong rule must be falsified. {}",
        report.summary()
    );
    assert!(report.failed(), "{}", report.summary());
    assert!(
        report
            .falsified
            .iter()
            .any(|line| line.contains("test.control-wrong-rule"))
    );
}

#[test]
fn gate_still_falsifies_a_complex_only_wrong_rule() {
    let wrong = GateRule {
        id: "test.control-complex-only-wrong-rule".to_string(),
        lhs: Expr::pow(x(), Expr::num(2)),
        rhs: Ok(Expr::pow(fun("abs", vec![x()]), Expr::num(2))),
        conditions: AssumptionSet::empty(),
        applicable: |_| true,
    };
    let report = run_gate(&[wrong]);
    assert!(
        !report.falsified.is_empty(),
        "a rule that is wrong only over the complexes must still be falsified. {}",
        report.summary()
    );
    assert!(
        report
            .falsified
            .iter()
            .any(|line| line.contains("test.control-complex-only-wrong-rule")
                && line.contains("complex:")),
        "{:?}",
        report.falsified
    );
    assert!(report.failed(), "{}", report.summary());
}

#[test]
fn sampling_is_a_superset_of_the_original_points_and_every_probe_is_documented() {
    for legacy in legacy_points() {
        assert!(
            all_points().contains(&legacy),
            "legacy point {legacy} was dropped"
        );
    }
    assert!(frozen().iter().all(|probe| !probe.why.trim().is_empty()));
    assert!(
        complex_probes()
            .iter()
            .all(|probe| !probe.why.trim().is_empty())
    );

    let near_pole = frozen()
        .into_iter()
        .filter(|probe| probe.region == "near-pole")
        .collect::<Vec<_>>();
    assert!(!near_pole.is_empty());
    for probe in &near_pole {
        assert!(
            probe.value >= rational(-1, 1_000_000) && probe.value <= rational(1, 1_000_000),
            "near-pole point must lie within epsilon = 1e-6 of the pole at 0: {}",
            probe.label
        );
    }

    let branch_cut = frozen()
        .into_iter()
        .filter(|probe| probe.region == "near-branch-cut")
        .collect::<Vec<_>>();
    assert!(branch_cut.iter().any(|probe| probe.value.is_negative()));
    assert!(branch_cut.iter().any(|probe| !probe.value.is_negative()));

    let discontinuity = frozen()
        .into_iter()
        .filter(|probe| probe.region == "near-discontinuity")
        .collect::<Vec<_>>();
    assert!(discontinuity.iter().any(|probe| probe.value.is_negative()));
    assert!(discontinuity.iter().any(|probe| !probe.value.is_negative()));

    let boundary = frozen()
        .into_iter()
        .filter(|probe| probe.region == "assumption-boundary")
        .collect::<Vec<_>>();
    assert!(boundary.iter().any(|probe| probe.value.is_zero()));
    assert!(boundary.iter().any(|probe| probe.value.is_negative()));
    assert!(
        boundary
            .iter()
            .any(|probe| !probe.value.is_negative() && !probe.value.is_zero())
    );

    let complex = complex_probes();
    assert!(complex.len() >= 8);
    assert!(complex.iter().any(|probe| probe.im != "0"));
    assert!(complex.iter().any(|probe| probe.im == "0"));
}

#[test]
fn assumption_boundary_guard_flips_exactly_at_zero() {
    let atom = non_negative_atom();
    assert!(atom_holds(&atom, &Num::Exact(rational(1, 1_000_000))));
    assert!(atom_holds(&atom, &Num::Exact(Rat::zero())));
    assert!(!atom_holds(&atom, &Num::Exact(rational(-1, 1_000_000))));
}

#[test]
fn complex_treatment_partitions_the_registry_with_explicit_counts() {
    let rules = gate_registry();
    let report = run_gate(&rules);
    let treatments = complex_treatment_audit(&rules, &report);
    assert_eq!(rules.len(), 9);
    assert_eq!(treatments.len(), 9);
    let distinct: std::collections::BTreeSet<&String> = treatments
        .iter()
        .map(|treatment| &treatment.rule_id)
        .collect();
    assert_eq!(distinct.len(), 9);

    let mut sampled: Vec<String> = treatments
        .iter()
        .filter(|treatment| treatment.disposition == ComplexDisposition::ComplexSampled)
        .map(|treatment| treatment.rule_id.clone())
        .collect();
    sampled.sort();
    let mut excluded: Vec<String> = treatments
        .iter()
        .filter(|treatment| treatment.disposition == ComplexDisposition::ComplexExclusionProven)
        .map(|treatment| treatment.rule_id.clone())
        .collect();
    excluded.sort();
    let expected_sampled = vec![
        "abs.abs-neg",
        "logexp.exp-log",
        "rat.cancel-x-over-x",
        "rat.cancel-zero-over-x",
        "trig.pythagorean-sin2-cos2",
    ];
    let expected_excluded = vec![
        "abs.abs-square",
        "logexp.log-exp",
        "pow.sqrt-square-nonnegative",
        "pow.sqrt-square-real",
    ];
    assert_eq!(sampled, expected_sampled);
    assert_eq!(excluded, expected_excluded);
    assert_eq!(sampled.len(), 5);
    assert_eq!(excluded.len(), 4);
    assert_eq!(rules.len(), sampled.len() + excluded.len());

    for treatment in &treatments {
        if treatment.disposition == ComplexDisposition::ComplexSampled {
            assert!(
                treatment.sampled_points > 0,
                "'{}' is complex-sampled but compared no complex point",
                treatment.rule_id
            );
        } else {
            assert_eq!(treatment.sampled_points, 0);
        }
        assert_eq!(
            treatment.unexplained_skips, 0,
            "'{}' skipped complex points with no stated reason: {:?}",
            treatment.rule_id, treatment.unexplained_detail
        );
    }
    assert!(report.unhandled_atoms.is_empty());
    assert!(report.threw.is_empty());
    assert!(report.falsified.is_empty());
    assert!(!report.failed(), "{}", report.summary());
}

#[test]
fn complex_exclusion_every_real_only_rule_has_an_observed_complex_counterexample() {
    let rules = gate_registry();
    let report = run_gate(&rules);
    let treatments = complex_treatment_audit(&rules, &report);
    let mut excluded: Vec<&ComplexTreatment> = treatments
        .iter()
        .filter(|treatment| treatment.disposition == ComplexDisposition::ComplexExclusionProven)
        .collect();
    excluded.sort_by(|left, right| left.rule_id.cmp(&right.rule_id));
    assert_eq!(excluded.len(), 4);
    let expected = vec![
        "abs.abs-square",
        "logexp.log-exp",
        "pow.sqrt-square-nonnegative",
        "pow.sqrt-square-real",
    ];
    let observed: Vec<&str> = excluded
        .iter()
        .map(|treatment| treatment.rule_id.as_str())
        .collect();
    assert_eq!(observed, expected);

    let tolerance = tolerance();
    let tolerance_squared = tolerance
        .value
        .mul(&tolerance.value, PRECISION)
        .expect("a real tolerance");
    for treatment in &excluded {
        assert!(
            !treatment.witness_label.is_empty(),
            "'{}' is complex-excluded without a witnessed complex counterexample",
            treatment.rule_id
        );
        assert!(!treatment.blocking_atoms.is_empty());
        let delta_squared = treatment
            .witness_delta_squared
            .as_ref()
            .expect("a witness difference");
        assert!(
            num_cmp(delta_squared, &tolerance_squared) != Some(Ordering::Less),
            "'{}' must genuinely fail at {}: |lhs-rhs|^2 was below the gate's tolerance",
            treatment.rule_id,
            treatment.witness_point
        );
        assert!(
            num_cmp(delta_squared, &Num::Exact(Rat::one())) == Some(Ordering::Greater),
            "'{}' witnessed difference must be O(1), not rounding noise: |lhs-rhs|^2 = {}",
            treatment.rule_id,
            num_text(delta_squared)
        );
    }

    for (rule_id, witness_label) in [
        ("abs.abs-square", "1+i"),
        ("logexp.log-exp", "5i"),
        ("pow.sqrt-square-nonnegative", "-1+i"),
        ("pow.sqrt-square-real", "1+i"),
    ] {
        assert_eq!(
            excluded
                .iter()
                .find(|treatment| treatment.rule_id == rule_id)
                .expect("the rule is excluded")
                .witness_label,
            witness_label
        );
    }

    assert!(report.falsified.is_empty());
    assert!(!report.failed(), "{}", report.summary());
}

#[test]
fn complex_exclusion_log_exp_is_stopped_by_realness_not_by_undefinedness() {
    let strip = strip_atom();
    let realness = real_atom();
    assert!(atom_holds(&strip, &complex_value("1", "1")));
    assert!(!atom_holds(&realness, &complex_value("1", "1")));
    assert!(!atom_holds(&strip, &complex_value("0", "5")));
}
