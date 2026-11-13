//! Transcript tests for the interactive REPL.
//!
//! Every test drives the built binary through a pipe — the same redirected-stdin flow a user or
//! an agent uses — and asserts the exact transcript after normalising only the volatile elapsed
//! suffix (the reference does the same: its own transcript tests assert lines that end with a
//! variable timing). The transcripts below are the reference console session's text; see
//! docs/goal-parity-contract/round-103/repl.md.

use std::io::Write;
use std::process::{Command, Stdio};

const BANNER: &str = "LovelaceSharp REPL v1.0.0\nArbitrary-precision math scripting, vector math, and plotting.\nType 'help' for a list of statements, operators, functions, and commands.\n\n";

fn help_overview() -> String {
    [
        "LovelaceSharp — help",
        "─────────────────────────────────────────────────",
        "Categories:",
        "  Language         1 function(s)",
        "  Arrays           23 function(s)",
        "  Numerics         10 function(s)",
        "  Symbolics        40 function(s)",
        "  Calculus         10 function(s)",
        "  Solving          4 function(s)",
        "  Linear Algebra   7 function(s)",
        "  Optimization     2 function(s)",
        "  Compilation      5 function(s)",
        "  DSP              16 function(s)",
        "  Introspection    5 function(s)",
        "",
        "help <category>       list a category's functions",
        "help <function>       signature, summary, examples",
        "funcs [category]      categorized function listing",
        "vars / clear / delete / run <file> / set precision|display|pretty",
        "Language statements & operators: docs/Language.md",
        "",
        "Every function example runs after this prelude:",
        "  x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\")",
    ]
    .join("\n")
}

fn help_solve() -> String {
    [
        "solve(f, x [, domain])",
        "",
        "Solves an equation for x and returns the SAME SolveResult record solve_full returns: status, domain (a Domain value), complete/completeness, per-solution conditions/multiplicity/exactness, parametric families and diagnostics. The default domain is Complex; pass real for real solutions only.",
        "",
        "Examples:",
        "    solve(x^2 - 4 == 0, x)",
        "    solve(x^2 + 1 == 0, x, real)",
        "",
        "Returns: SolveResult",
        "See also: solve_full, solve_system, linsolve",
        "Plugin: Lovelace.Symbolics",
    ]
    .join("\n")
}

const KNOWN_CATEGORIES: &str = "Language, Arrays, Numerics, Symbolics, Calculus, Solving, Linear Algebra, Optimization, Compilation, DSP, Introspection, Core";

/// Runs the REPL with `stdin` piped in and returns the transcript with every scaled elapsed
/// suffix replaced by `[T]`.
fn run_repl(args: &[&str], stdin: &str) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lovelace-run"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cannot start lovelace-run");

    let mut pipe = child.stdin.take().expect("the child has no stdin");
    let input = stdin.to_string();
    let writer = std::thread::spawn(move || {
        // A session that exits (exit/quit) stops reading; a broken pipe is expected and ignored.
        let _ = pipe.write_all(input.as_bytes());
    });

    let output = child
        .wait_with_output()
        .expect("lovelace-run did not finish");
    writer.join().expect("the stdin writer panicked");

    assert_eq!(
        output.status.code(),
        Some(0),
        "the REPL must exit 0; stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        output.stderr.is_empty(),
        "the REPL wrote to stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    normalise_elapsed(&String::from_utf8(output.stdout).expect("stdout is not UTF-8"))
}

fn normalise_elapsed(transcript: &str) -> String {
    transcript
        .split('\n')
        .map(|line| {
            if let Some(position) = line.rfind("   [") {
                let tail = &line[position + 4..];
                let scaled = [" ns]", " µs]", " ms]", " s]", " min]", " h]"]
                    .iter()
                    .any(|unit| tail.ends_with(unit));
                if scaled {
                    return format!("{}   [T]", &line[..position]);
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn session(body: &str) -> String {
    format!("{BANNER}{body}")
}

#[test]
fn multiline_accumulation_runs_the_block_after_braces_balance() {
    let transcript = run_repl(&["--repl"], "func g(x) {\n  x + 1\n}\ng(2)\n");
    let expected =
        session("» func g(x) {\n…   x + 1\n… }\n» g(2)\n= 3 (Natural)   [T]\n» \nBye!\n");
    assert_eq!(transcript, expected);
}

#[test]
fn state_persists_between_commands_and_underscore_tracks_the_last_result() {
    let transcript = run_repl(&["--repl"], "a = 5\nfunc f(x) = x^2\n_ * 3\nf(4)\nvars\n");
    let expected = session(concat!(
        "» a = 5\n",
        "= 5 (Natural)   [T]\n",
        "» func f(x) = x^2\n",
        "» _ * 3\n",
        "= 15 (Natural)   [T]\n",
        "» f(4)\n",
        "= 16 (Natural)   [T]\n",
        "» vars\n",
        "  _ = 16 (Natural)\n",
        "  a = 5 (Natural)\n",
        "» \n",
        "Bye!\n",
    ));
    assert_eq!(transcript, expected);
}

#[test]
fn precision_knobs_apply_to_later_commands() {
    let transcript = run_repl(
        &["--repl"],
        "sqrt(2)\nset display 5\nsqrt(2)\nset precision 7\nsqrt(2)\n",
    );
    let expected = session(concat!(
        "» sqrt(2)\n",
        "= 1.4142135623730950488016887242096980785696718753769480731766797379907324784621070388503875343276415727 (Real)   [T]\n",
        "» set display 5\n",
        "Display digits set to 5.\n",
        "» sqrt(2)\n",
        "= 1.41421 (Real)   [T]\n",
        "» set precision 7\n",
        "Computation precision set to 7 decimal places.\n",
        "» sqrt(2)\n",
        "= 1.4142135 (Real)   [T]\n",
        "» \n",
        "Bye!\n",
    ));
    assert_eq!(transcript, expected);
}

#[test]
fn errors_render_the_caret_and_the_session_keeps_accepting_input() {
    let transcript = run_repl(&["--repl"], "[1,2;3,4]\ndiff(x, 3)\n2+2\n");
    let expected = session(concat!(
        "» [1,2;3,4]\n",
        "[1,2;3,4]\n",
        "    ^\n",
        "Error: Expected 'RBracket' but found ';' at position 4.   [T]\n",
        "» diff(x, 3)\n",
        "Error: Undefined variable 'x'.   [T]\n",
        "» 2+2\n",
        "= 4 (Natural)   [T]\n",
        "» \n",
        "Bye!\n",
    ));
    assert_eq!(transcript, expected);
}

#[test]
fn cancellation_is_rendered_and_the_session_continues() {
    let transcript = run_repl(
        &["--repl", "--cancel-after", "200"],
        "sum(1..1000000000)\n1+1\n",
    );
    let expected = session(concat!(
        "» sum(1..1000000000)\n",
        "Error: the evaluation was cancelled by the caller.   [T]\n",
        "» 1+1\n",
        "= 2 (Natural)   [T]\n",
        "» \n",
        "Bye!\n",
    ));
    assert_eq!(transcript, expected);
}

#[test]
fn interactive_commands_and_help_match_the_reference_rendering() {
    let transcript = run_repl(
        &["--repl"],
        "help\nhelp solve\nhelp nosuchfn\nfuncs nosuch\nset pretty unicode\ninf\nset pretty ascii\ninf\nset display 4\npi\nclear\nvars\ndelete n\n",
    );
    let expected = session(&format!(
        "» help\n{}\n» help solve\n{}\n» help nosuchfn\nNo category 'nosuchfn'. Known categories: {KNOWN_CATEGORIES}.\n» funcs nosuch\nNo category or function matching 'nosuch'. Known categories: {KNOWN_CATEGORIES}.\n» set pretty unicode\nPretty output: unicode.\n» inf\n= ∞ (Symbolic)   [T]\n» set pretty ascii\nPretty output: ascii.\n» inf\n= inf (Symbolic)   [T]\n» set display 4\nDisplay digits set to 4.\n» pi\n= 3.1415 (Real)   [T]\n» clear\nAll variables cleared.\n» vars\n(no variables defined)\n» delete n\nVariable 'n' is not defined.\n» \nBye!\n",
        help_overview(),
        help_solve(),
    ));
    assert_eq!(transcript, expected);
}

#[test]
fn run_executes_a_script_file_and_prints_its_output_before_the_result() {
    let path = std::env::temp_dir().join(format!("lovelace-repl-run-{}.ls", std::process::id()));
    std::fs::write(&path, "print(\"hello\")\n1 + 2\n").expect("cannot write the run script");
    let command = format!("run \"{}\"", path.display());
    let transcript = run_repl(&["--repl"], &format!("{command}\n"));
    let _ = std::fs::remove_file(&path);

    let expected = session(&format!(
        "» {command}\nhello\n= 3 (Natural)   [T]\n» \nBye!\n"
    ));
    assert_eq!(transcript, expected);
}

#[test]
fn a_missing_run_file_reports_the_reference_clr_sentence() {
    let path =
        std::env::temp_dir().join(format!("lovelace-repl-missing-{}.ls", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let command = format!("run \"{}\"", path.display());
    let transcript = run_repl(&["--repl"], &format!("{command}\n"));

    let absolute = std::path::absolute(&path)
        .map(|value| value.to_string_lossy().to_string())
        .expect("cannot absolute-path the missing file");
    let expected = session(&format!(
        "» {command}\nError: Could not find file '{absolute}'.   [T]\n» \nBye!\n"
    ));
    assert_eq!(transcript, expected);
}

#[test]
fn exit_and_end_of_input_terminate_with_bye() {
    let exit = run_repl(&["--repl"], "exit\n1+1\n");
    assert_eq!(exit, session("» exit\nBye!\n"));
    assert!(!exit.contains("= 2 (Natural)"));

    let quit = run_repl(&["--repl"], "quit\n1+1\n");
    assert_eq!(quit, session("» quit\nBye!\n"));

    let eof = run_repl(&["--repl"], "1+1\n");
    assert_eq!(eof, session("» 1+1\n= 2 (Natural)   [T]\n» \nBye!\n"));
}

#[test]
fn continuation_end_of_input_still_prints_bye_once() {
    let transcript = run_repl(&["--repl"], "func g(x) {\n");
    let expected = session("» func g(x) {\n… \nBye!\n");
    assert_eq!(transcript, expected);
}

#[test]
fn repl_rejects_a_script_source_on_the_command_line() {
    let output = Command::new(env!("CARGO_BIN_EXE_lovelace-run"))
        .args(["--repl", "--eval", "1+1"])
        .output()
        .expect("cannot start lovelace-run");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--repl cannot be combined with a script source"),
        "unexpected stderr: {stderr}"
    );
}
