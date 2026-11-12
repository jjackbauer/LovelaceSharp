//! Round 66: the deferred census rows the runner's own envelope now decides.
//!
//! Each test drives parity_batch::run_args - the runner's parse/evaluate/envelope path - and
//! asserts the C# row's contract on the field that renders it. The oracle (pinned C# runner)
//! publishes the same normalised envelope for the same script (measured in this round's sweep).

use super::parity_batch::*;
use lovelace_json::Json;

fn structured(r: &RunResult) -> &Json {
    get(get(&r.env, "result"), "structured")
}

/// SolveCompletenessMappingTests.cs:67 LanguageStatuses_ReportExactlyWhatTheMappingReturns, the
/// Unevaluated row: solve_full(x^4 + 1 == 0, x) publishes status Unevaluated with
/// complete false and completeness Unknown - the documented pair for that status. The C# row also
/// reads the pair back through SolveCompletenessMapping.Of; the mapping is a total function whose
/// output for Unevaluated is exactly this triple, so the record's own pair is the mapping's output
/// asserted where the language publishes it.
#[test]
fn solve_language_statuses_publish_the_unevaluated_pair() {
    let r = run_args(&["--eval", "x = symbol(\"x\"); solve_full(x^4 + 1 == 0, x)"]);
    assert_eq!(r.exit, 0, "{}", r.env);
    assert!(boolean(get(&r.env, "ok")), "{}", r.env);
    let record = structured(&r);
    assert_eq!(text(get(record, "type")), "SolveResult");

    let status = record_field(record, "status");
    assert_eq!(text(get(status, "kind")), "Enum");
    assert_eq!(text(get(status, "type")), "SolveStatus");
    assert_eq!(text(get(status, "value")), "Unevaluated");

    let complete = record_field(record, "complete");
    assert_eq!(text(get(complete, "kind")), "Boolean");
    assert_eq!(text(get(complete, "value")), "false");

    let completeness = record_field(record, "completeness");
    assert_eq!(text(get(completeness, "kind")), "Enum");
    assert_eq!(text(get(completeness, "type")), "Completeness");
    assert_eq!(text(get(completeness, "value")), "Unknown");
}

/// DspTests.cs:109 ImpulseResponse_GivenFirstOrderIir_MatchesHandComputed: the first-order IIR
/// y[n] = 0.1*x[n] + 0.9*y[n-1] has the hand-computed impulse response
/// [0.1, 0.09, 0.081, 0.0729]. The runner's difference-equation filter publishes exactly that
/// vector for the same coefficients (a = [1, -0.9], b = [0.1], length 4).
#[test]
fn dsp_impulse_response_of_a_first_order_iir_matches_the_hand_computed_sequence() {
    let r = run_args(&["--eval", "filter([1, -0.9], [0.1], 4)"]);
    assert_eq!(r.exit, 0, "{}", r.env);
    assert!(boolean(get(&r.env, "ok")), "{}", r.env);
    let result = get(&r.env, "result");
    assert_eq!(text(get(result, "kind")), "Vector");
    assert_eq!(text(get(result, "display")), "[0.1, 0.09, 0.081, 0.0729]");
    let elements = array(get(structured(&r), "elements"));
    assert_eq!(elements.len(), 4, "four samples");
    // the C# row compares Complex values: each sample is exact with the imaginary part zero.
    let real: Vec<&str> = elements.iter().map(|e| text(get(e, "re"))).collect();
    assert_eq!(real, vec!["0.1", "0.09", "0.081", "0.0729"]);
    for (index, element) in elements.iter().enumerate() {
        assert_eq!(text(get(element, "kind")), "Complex", "sample {index}");
        assert_eq!(text(get(element, "im")), "0", "sample {index}");
        assert!(boolean(get(element, "exact")), "sample {index}");
    }
}
