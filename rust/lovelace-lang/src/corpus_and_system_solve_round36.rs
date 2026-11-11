//! Round 36: the two documentation-corpus failures and the system-solve family.
//!
//! Every expected string below is the pinned C# oracle's exact `--text` output for the script
//! (probed through `dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll`), so a
//! passing test is a byte-level parity claim, not a proxy.

use crate::{Engine, Value};

fn eval(script: &str) -> Value {
    Engine::new()
        .eval(script)
        .unwrap_or_else(|error| panic!("script: {script} failed: {}", error.message))
}

fn display(script: &str) -> String {
    eval(script).display()
}

// ===========================================================================
// Corpus sym/071 — MathIR listing of the Horner form
// ===========================================================================

/// Documentation `Lovelace.Symbolics/README.md` section 13 (corpus sym/071).
///
/// The reference's PowInt lowering emits a FRESH Constant node for the exponent on every
/// occurrence (the constant TABLE is deduplicated, the node is not), so the literal 2 appears
/// once as the exponent node and once again as the coefficient node just before the first Add.
#[test]
fn corpus_sym_071_optimize_lower_emits_the_documented_mathir_listing() {
    let expected = "#!mathir 2\n\
param x\n\
const (rat 2 1)\n\
const (rat 1 1)\n\
const (rat 3 1)\n\
Parameter 0 0\n\
Constant 0 0\n\
PowInt 0 0,1\n\
Constant 0 1\n\
Constant 0 2\n\
Constant 0 0\n\
Add 0 5,0\n\
Mul 0 0,6\n\
Add 0 4,7\n\
Mul 0 0,8\n\
Add 0 3,9\n\
Mul 0 2,10";
    let script = "x = symbol(\"x\"); k = optimize(x^5 + 2*x^4 + 3*x^3 + x^2, [x]); lower(k, [x])";
    assert_eq!(display(script), expected);
}

// ===========================================================================
// Corpus sym/076 — linsolve keeps the Bareiss back-substitution shape
// ===========================================================================

/// Documentation `Lovelace.Symbolics/README.md` section 15 (corpus sym/076).
///
/// The reference solves by fraction-free Bareiss elimination, so the first value keeps the
/// intermediate `x/x` from back-substitution: `-x/x/(x*y)`, never Cramer's simplified
/// `-1/(x*y)`. The structured form publishes the same division and the det != 0 condition.
#[test]
fn corpus_sym_076_linsolve_keeps_the_bareiss_division_shape() {
    let script = "x = symbol(\"x\"); y = symbol(\"y\")";
    let mut engine = Engine::new();
    engine.eval(script).expect("symbols");
    assert_eq!(
        engine
            .eval("linsolve([[x, 1], [0, y]], [0, 1])")
            .expect("linsolve")
            .display(),
        "[-x/x/(x*y), x/(x*y)]"
    );
    assert_eq!(
        engine
            .eval("linsolve_full([[x, 1], [0, y]], [0, 1]).conditions")
            .expect("conditions")
            .display(),
        "[x*y != 0]"
    );
}

// ===========================================================================
// System-solve family — the five dispositions
// ===========================================================================

const CIRCLE_HYPERBOLA: &str = "x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x^2 + y^2 == 1, x*y == 0], [x, y])";
const CIRCLE_HYPERBOLA_ORACLE: &str = "SystemSolveResult(status: Solved, domain: complex, complete: True, completeness: Complete, solutions: [SystemSolution(bindings: [Binding(name: x, value: 0), Binding(name: y, value: -1)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: -1), Binding(name: y, value: 0)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: 1), Binding(name: y, value: 0)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: 0), Binding(name: y, value: 1)], conditions: [], exactness: Exact)], diagnostics: [])";

/// `SystemSolveTests/CircleHyperbola_FourSolutions` and the two descriptor examples.
#[test]
fn system_solve_circle_hyperbola_publishes_four_solutions() {
    assert_eq!(display(CIRCLE_HYPERBOLA), CIRCLE_HYPERBOLA_ORACLE);
}

const PARABOLA_LINE_ORACLE: &str = "SystemSolveResult(status: Solved, domain: complex, complete: True, completeness: Complete, solutions: [SystemSolution(bindings: [Binding(name: x, value: 1/2*(-sqrt(5) - 1)), Binding(name: y, value: 1/2*(-sqrt(5) - 1))], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: 1/2*(sqrt(5) - 1)), Binding(name: y, value: 1/2*(sqrt(5) - 1))], conditions: [], exactness: Exact)], diagnostics: [])";

/// `SystemSolveTests/ParabolaLine_TwoSolutions` and
/// `SystemSolveProofTests/NonlinearElimination_FindsBothSolutionsOfParabolaAndLine`.
#[test]
fn system_solve_parabola_line_finds_both_exact_radical_solutions() {
    assert_eq!(
        display("x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x^2 + y == 1, x - y == 0], [x, y])"),
        PARABOLA_LINE_ORACLE
    );
}

const CIRCLE_LINE_ORACLE: &str = "SystemSolveResult(status: Solved, domain: complex, complete: True, completeness: Complete, solutions: [SystemSolution(bindings: [Binding(name: x, value: -1/2*sqrt(2)), Binding(name: y, value: -1/2*sqrt(2))], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: 1/2*sqrt(2)), Binding(name: y, value: 1/2*sqrt(2))], conditions: [], exactness: Exact)], diagnostics: [])";

/// `SystemSolveProofTests/NonlinearElimination_FindsBothSolutionsOfCircleAndLine`.
#[test]
fn system_solve_circle_line_finds_both_exact_radical_solutions() {
    assert_eq!(
        display("x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x^2 + y^2 == 1, y == x], [x, y])"),
        CIRCLE_LINE_ORACLE
    );
}

const INCONSISTENT_ORACLE: &str = "SystemSolveResult(status: NoSolutions, domain: complex, complete: True, completeness: Complete, solutions: [], diagnostics: [])";

/// `SystemSolveTests/Inconsistent_NoSolutions` and the wire contradiction rows: a DERIVED
/// contradiction is NoSolutions/complete with no diagnostic to excuse it.
#[test]
fn system_solve_derived_contradictions_are_complete_no_solutions() {
    for script in [
        "x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x + y == 1, x + y == 2], [x, y])",
        "x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x*y == 1, x == 0], [x, y])",
        "x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x^2 - 1 == 0, x^2 - 4 == 0], [x, y])",
        "x = symbol(\"x\"); solve_system_full([x - 1 == 0, x - 2 == 0], [x])",
    ] {
        assert_eq!(display(script), INCONSISTENT_ORACLE, "{script}");
    }
}

/// `SystemSolveTests/NonPolynomial_ReportsNote`: the non-polynomial gate names itself.
#[test]
fn system_solve_non_polynomial_reports_the_stable_note() {
    assert_eq!(
        display("x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([sin(x) == y], [x, y])"),
        "SystemSolveResult(status: Unevaluated, domain: complex, complete: False, completeness: Unknown, solutions: [], diagnostics: [Diagnostic(code: system-solve.unevaluated, category: UnsupportedOperation, message: system-solve.non-polynomial: only polynomial systems are supported in v1., recoverable: True, location: , details: [])])"
    );
}

/// `SystemSolveProofTests/Wire_EmptyEquationList_IsARecord_NotAClrException`.
#[test]
fn system_solve_empty_equation_list_is_a_typed_refusal() {
    assert_eq!(
        display("x = symbol(\"x\"); solve_system_full([], [x])"),
        "SystemSolveResult(status: Unevaluated, domain: complex, complete: False, completeness: Unknown, solutions: [], diagnostics: [Diagnostic(code: system-solve.unevaluated, category: UnsupportedOperation, message: system-solve.no-equations: the system has no equations, so every assignment of the variables is a solution; the solution set is not enumerable here., recoverable: True, location: , details: [])])"
    );
}

/// `SystemSolveProofTests/Wire_AbsenceOfSolutions_IsUnevaluatedWithADiagnostic` (free-variable
/// and quartic cases): an abandoned branch is Unevaluated and says why.
#[test]
fn system_solve_abandoned_branches_are_unevaluated_with_the_oracle_note() {
    assert_eq!(
        display("x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x^2 + y^2 == 1], [x, y])"),
        "SystemSolveResult(status: Unevaluated, domain: complex, complete: False, completeness: Unknown, solutions: [], diagnostics: [Diagnostic(code: system-solve.unevaluated, category: UnsupportedOperation, message: system-solve.elimination-incomplete: the Gröbner basis has no polynomial in y alone, so the values of y are not enumerated here., recoverable: True, location: , details: [])])"
    );
    assert_eq!(
        display("x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x^4 + 1 == 0, y == x], [x, y])"),
        "SystemSolveResult(status: Unevaluated, domain: complex, complete: False, completeness: Unknown, solutions: [], diagnostics: [Diagnostic(code: system-solve.unevaluated, category: UnsupportedOperation, message: system-solve.univariate-incomplete: the elimination polynomial in y has no representable root, so this branch is not decided., recoverable: True, location: , details: [])])"
    );
}

/// The public `solve_system` and `solve_system_full` surfaces publish the SAME record (the
/// reference's plugin registers one implementation for both names).
#[test]
fn system_solve_plain_and_full_publish_the_same_record() {
    for body in [
        "[x + y == 1, x - y == 3]",
        "[x^2 + y == 1, x - y == 0]",
        "[x + y == 1, x + y == 2]",
    ] {
        let plain = display(&format!("x = symbol(\"x\"); y = symbol(\"y\"); solve_system({body}, [x, y])"));
        let full = display(&format!("x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full({body}, [x, y])"));
        assert_eq!(plain, full, "{body}");
    }
}

