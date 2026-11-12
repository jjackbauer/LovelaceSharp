//! Round 75: deferred census rows the current runner now decides.
//!
//! Each test drives parity_batch::run_args - the runner's own parse/evaluate/envelope path - and
//! repeats the C# row's assertion on the field that carries it. The oracle (pinned C# runner)
//! publishes the same normalised envelope for the same script; the measurements are recorded in
//! docs/goal-parity-contract/round-75/probe-classification.md.

use super::parity_batch::*;
use lovelace_json::Json;

fn structured(r: &RunResult) -> &Json {
    get(get(&r.env, "result"), "structured")
}

fn canonical(r: &RunResult) -> &str {
    text(get(structured(r), "canonical"))
}

/// FactorMetamorphicTests.Factor_ExpandsBackToItsInput_OverTheWholeCorpus:
/// the C# corpus of 18 polynomials (nine with a negative leading coefficient) must satisfy
/// expand(factor(p)) == expand(p). The runner's `factor`/`expand` are the production path.
#[test]
fn factor_expands_back_to_its_input_over_the_whole_corpus() {
    const CORPUS: [&str; 18] = [
        "-x^2 + 1",
        "-x^3 + x",
        "-x^4 + 5*x^2 - 4",
        "-2*x^2 + 8",
        "-6*x^3 + 6*x",
        "-x^2 + 2",
        "-x^3 - x^2 + x + 1",
        "-x^2 - 2*x - 1",
        "-3*x^2 + 3",
        "x^2 - 1",
        "x^3 + 1",
        "x^4 - 5*x^2 + 4",
        "2*x^2 - 8",
        "6*x^3 - 6*x",
        "x^2 - 2",
        "x^3 + x^2 - x - 1",
        "x^3 - 3*x^2 + 3*x - 1",
        "x^4 - 1",
    ];
    let mut negative = 0;
    for source in CORPUS {
        if source.starts_with('-') {
            negative += 1;
        }
        let factored = run_args(&[
            "--eval",
            &format!("x = symbol(\"x\"); expand(factor({source}))"),
        ]);
        let expanded = run_args(&["--eval", &format!("x = symbol(\"x\"); expand({source})")]);
        assert_eq!(
            factored.exit, 0,
            "expand(factor({source})): {}",
            factored.env
        );
        assert_eq!(expanded.exit, 0, "expand({source}): {}", expanded.env);
        assert!(boolean(get(&factored.env, "ok")), "{}", factored.env);
        assert!(boolean(get(&expanded.env, "ok")), "{}", expanded.env);
        assert_eq!(
            canonical(&factored),
            canonical(&expanded),
            "expand(factor({source})) must equal expand({source})"
        );
    }
    // non-vacuity: the corpus really contains the negative-leading cases the row pins.
    assert!(negative >= 3, "corpus negative-leading cases: {negative}");
}

/// CapabilitiesBuiltinTests.ExactnessBestEffort_IsBackedByLiveRefusalsTheListDoesNotCarry:
/// `exactness` is BestEffort, and the five live refusals the C# body drives are each absent from
/// the advertised unsupported-operations list, with at least one caller-side InvalidArgument.
#[test]
fn exactness_best_effort_is_backed_by_live_refusals_the_list_does_not_carry() {
    let capabilities = run_args(&["--eval", "capabilities()"]);
    assert_eq!(capabilities.exit, 0, "{}", capabilities.env);
    let record = structured(&capabilities);
    assert_eq!(
        text(get(record_field(record, "exactness"), "value")),
        "BestEffort"
    );

    let operations = record_field(record, "unsupported_operations");
    let advertised: Vec<(String, String, String)> = array(get(operations, "elements"))
        .iter()
        .map(|entry| {
            (
                text(get(record_field(entry, "code"), "value")).to_string(),
                text(get(record_field(entry, "category"), "value")).to_string(),
                text(get(record_field(entry, "message"), "value")).to_string(),
            )
        })
        .collect();
    assert!(!advertised.is_empty());

    let unenumerated = [
        "mean(1);",
        "max(1,2);",
        "x = symbol(\"x\"); sum(x, 5);",
        "1 +;",
        "0^-1;",
    ];
    let mut observed: Vec<(String, String)> = Vec::new();
    for script in unenumerated {
        let run = run_args(&["--eval", script]);
        assert_eq!(run.exit, 1, "'{script}' must be refused: {}", run.env);
        assert!(!boolean(get(&run.env, "ok")), "'{script}': {}", run.env);
        let code = text(get(&run.env, "code")).to_string();
        let category = text(get(&run.env, "category")).to_string();
        let message = text(get(&run.env, "message")).to_string();
        assert!(
            !advertised
                .iter()
                .any(|(c, k, m)| *c == code && *k == category && *m == message),
            "'{script}' ({code}/{category}/\"{message}\") is advertised, so the list is no longer BestEffort"
        );
        observed.push((code, category));
    }

    // the decisive part: one of the five is a caller-side argument error, never an internal one.
    assert!(
        observed
            .iter()
            .any(|(code, category)| code == "InvalidArgument" && category == "TypeMismatch"),
        "observed refusals: {observed:?}"
    );
    assert!(
        !observed
            .iter()
            .any(|(code, category)| code == "InternalError"
                && category == "InternalInvariantFailure"),
        "observed refusals: {observed:?}"
    );
    assert!(
        !advertised
            .iter()
            .any(|(code, _, _)| code == "InternalError"),
        "the advertised list must not claim InternalError"
    );
}
