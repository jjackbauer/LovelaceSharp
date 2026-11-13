//! `--journey <ID>`: the mandatory desktop journeys, executed headlessly.
//!
//! Each journey is the executable form of one row in `docs/goal-desktop-v4/journeys.md`: the
//! script(s), the action sequence and the expected observable. The scripts go through the same
//! [`Workbench`] the window owns, and therefore through the same `ExecutionHost` path: every
//! value, print line, diagnostic, variable, function and plot below is read from the typed event
//! stream the window consumes, or from the same `Plot` pane the window paints.
//!
//! The mode is an acceptance harness, not a second product implementation: it adds no way to
//! evaluate source. It prints the observed values (result text, variables with kinds, revision,
//! reused count, diagnostics, the plot element census and the exported SVG's SHA-256) and exits
//! non-zero when the expected observable does not match. A mismatch is always a failure; no
//! expectation is relaxed to make a journey pass.
//!
//! Four of the first journeys need only the window's headless workbench. J-15 additionally drives
//! the real plot pane: it loads the engine's `PlotCapture.svg` through `Plot::set_svg`,
//! rasterises through the pane's resvg path, zooms/pans/fits through `PlotView` and exports the
//! captured bytes through `Plot::export_svg_to`.
//!
//! J-07 and J-08 certify the incremental cache (INC-001..INC-010): they read the per-statement
//! `StepCommitted { reused }` flags and the document's reuse counter from the same typed event
//! stream the window consumes, and compare the resulting values with the reference captures.

#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use egui::{Rect, pos2, vec2};

use crate::execution::{SourceRevision, WorkspaceSnapshot};
use crate::gui_selftest::plot_element_counts;
use crate::log_kind::LogKind;
use crate::run_state::RunState;
use crate::ui::plot::{MAX_EXPORT_SIDE, Plot, PlotView};
use crate::workbench::{Workbench, WorkbenchDocument};

/// One run's terminal budget; the desktop's own long-run check uses 30 s.
const RUN_TIMEOUT: Duration = Duration::from_secs(60);

/// The SHA-256 of the engine SVG this journey's plot script produces.
///
/// The value is the reference capture's own: `REF-plot` holds 15 000 bytes for
/// `plot(1..10, (1..10)^2)` and the current engine emits exactly the same document, so the hash
/// is a web-to-desktop parity assertion rather than a snapshot of whatever the engine happened
/// to write today.
const J15_SVG_SHA256: &str = "0b23d6f74c16bdd102cee23e9388d884a9212fc614cca55b9f814ea66271406c";

/// The 1000-digit `sqrt(2)` display the reference (`REF-digits`) publishes.
///
/// Pinned by SHA-256 rather than by a 1002-character literal: the digest proves every digit,
/// and it is the value `Get-FileHash`/the reference report use, so the round document can state
/// one short identity.
const J03_1000_DIGIT_SHA256: &str =
    "92fa8c84b033aaf1a67722abccd41dab07aa693b6d64398b6baaafa47636b63c";

/// The 20-place display of `sqrt(2)`: 20 decimal places with the trailing zero trimmed.
const J03_20_PLACE_DISPLAY: &str = "1.4142135623730950488";

/// Runs `--journey` for every id given after the flag.
///
/// Exit codes: 0 every journey passed, 1 at least one journey failed, 2 usage.
pub fn run(args: &[String]) -> i32 {
    let options = match Options::parse(args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}\n\n{}", usage());
            return 2;
        }
    };
    // Validate every id before any state is created or any line is printed: an unknown id is a
    // usage error (exit 2), not a journey failure, and must not depend on a writable temp folder.
    for id in &options.journeys {
        if !matches!(
            id.as_str(),
            "J-02" | "J-03" | "J-07" | "J-08" | "J-09" | "J-10" | "J-15"
        ) {
            eprintln!("unknown journey '{id}'\n\n{}", usage());
            return 2;
        }
    }
    let workbench = Workbench::new();
    println!(
        "JOURNEY session lovelace-desktop {} engine={} plot-dir={}",
        env!("CARGO_PKG_VERSION"),
        workbench.engine_name(),
        workbench.plot_dir().display()
    );
    // J-09 copies each run's plot and J-15 exports its raster/SVG artifacts, so both journeys
    // need the output directory before any evaluation starts. A plain `--journey J-02` must
    // still work even where the default temp directory is not writable, so the directory is
    // created only when a selected journey writes into it (DEF-004).
    let needs_out = options.journeys.iter().any(|id| needs_output_dir(id));
    let out = options.out.unwrap_or_else(|| {
        std::env::temp_dir()
            .join("lovelace-journey")
            .join(format!("run-{}", std::process::id()))
    });
    if needs_out {
        if let Err(error) = std::fs::create_dir_all(&out) {
            eprintln!("cannot create {}: {error}", out.display());
            return 1;
        }
    }
    let mut failed = 0usize;
    for id in &options.journeys {
        let outcome = match id.as_str() {
            "J-02" => journey_j02(),
            "J-03" => journey_j03(),
            "J-07" => journey_j07(),
            "J-08" => journey_j08(),
            "J-09" => journey_j09(&out),
            "J-10" => journey_j10(),
            "J-15" => journey_j15(&out),
            other => {
                eprintln!("unknown journey '{other}'\n\n{}", usage());
                return 2;
            }
        };
        if let Err(message) = outcome {
            failed += 1;
            println!("JOURNEY {id} FAIL {message}");
        }
    }
    println!(
        "JOURNEY summary journeys={} failed={failed}",
        options.journeys.len()
    );
    if failed == 0 { 0 } else { 1 }
}

/// The parsed `--journey` arguments.
struct Options {
    journeys: Vec<String>,
    out: Option<PathBuf>,
}

impl Options {
    fn parse(args: &[String]) -> Result<Options, String> {
        let mut journeys = Vec::new();
        let mut out = None;
        let mut index = 0usize;
        while index < args.len() {
            match args[index].as_str() {
                "--journey" => {
                    let id = args
                        .get(index + 1)
                        .ok_or_else(|| "--journey needs an id (for example J-02)".to_string())?;
                    journeys.push(id.clone());
                    index += 2;
                }
                "--out" => {
                    let path = args
                        .get(index + 1)
                        .ok_or_else(|| "--out needs a directory".to_string())?;
                    out = Some(PathBuf::from(path));
                    index += 2;
                }
                other if other.starts_with("--") => {
                    return Err(format!("unknown option '{other}'"));
                }
                other => {
                    journeys.push(other.to_string());
                    index += 1;
                }
            }
        }
        if journeys.is_empty() {
            return Err("no journey id given".to_string());
        }
        Ok(Options { journeys, out })
    }
}

fn usage() -> String {
    "usage: lovelace-desktop --journey <ID> [--journey <ID> ...] [--out <dir>]\n\
     \n\
     IDs: J-02 J-03 J-07 J-08 J-09 J-10 J-15\n\
     \n\
     Each journey runs its script through the shipped workbench/ExecutionHost path and exits\n\
     non-zero when the expected observable in docs/goal-desktop-v4/journeys.md does not match.\n\
     --out names the directory J-15 writes its exported SVG into (default: a temp directory).\n"
        .to_string()
}

/// Whether a journey writes artifacts into the `--out` directory.
///
/// J-09 copies each run's plot and J-15 exports its raster/SVG artifacts; the other journeys read
/// only the workbench's own snapshot and must keep working where the temp directory is not
/// writable. `--journey` creates the output directory before evaluating when this returns true.
fn needs_output_dir(id: &str) -> bool {
    matches!(id, "J-09" | "J-15")
}

// ---------------------------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------------------------

/// A journey's check log: prints each comparison and collects the mismatches.
struct Checks {
    id: &'static str,
    failures: Vec<String>,
}

impl Checks {
    fn new(id: &'static str) -> Checks {
        Checks {
            id,
            failures: Vec::new(),
        }
    }

    /// Records one contract comparison and prints both sides.
    fn expect(&mut self, what: &str, expected: impl Into<String>, observed: impl Into<String>) {
        let expected = expected.into();
        let observed = observed.into();
        let ok = expected == observed;
        println!(
            "JOURNEY {} check {what} expected={expected:?} observed={observed:?} {}",
            self.id,
            if ok { "ok" } else { "FAIL" }
        );
        if !ok {
            self.failures.push(format!(
                "{what}: expected {expected:?}, observed {observed:?}"
            ));
        }
    }

    /// Records a boolean predicate with its expected and observed descriptions.
    fn expect_that(
        &mut self,
        what: &str,
        expected: impl Into<String>,
        observed: impl Into<String>,
        ok: bool,
    ) {
        let expected = expected.into();
        let observed = observed.into();
        println!(
            "JOURNEY {} check {what} expected={expected:?} observed={observed:?} {}",
            self.id,
            if ok { "ok" } else { "FAIL" }
        );
        if !ok {
            self.failures.push(format!(
                "{what}: expected {expected:?}, observed {observed:?}"
            ));
        }
    }

    /// Prints the journey's verdict and returns the combined failure text.
    fn finish(self) -> Result<(), String> {
        if self.failures.is_empty() {
            println!("JOURNEY {} PASS", self.id);
            Ok(())
        } else {
            Err(self.failures.join(" | "))
        }
    }
}

fn new_workbench() -> Workbench {
    Workbench::new()
}

fn run_document(workbench: &mut Workbench, label: &str) -> Result<(), String> {
    if !workbench.run_active() {
        return Err(format!("{label}: run_active returned false"));
    }
    if !workbench.wait_until_idle(RUN_TIMEOUT) {
        return Err(format!("{label}: the run did not reach a terminal state"));
    }
    let state = workbench.documents()[0].state();
    if state != RunState::Finished {
        let snapshot = workbench.documents()[0]
            .snapshot
            .as_ref()
            .map(|snapshot| {
                snapshot
                    .diagnostic
                    .clone()
                    .unwrap_or_else(|| "no diagnostic".to_string())
            })
            .unwrap_or_else(|| "no snapshot".to_string());
        return Err(format!(
            "{label}: terminal state {state:?}, expected Finished ({snapshot})"
        ));
    }
    Ok(())
}

fn document(workbench: &Workbench) -> &WorkbenchDocument {
    &workbench.documents()[0]
}

fn snapshot(workbench: &Workbench, label: &str) -> Result<WorkspaceSnapshot, String> {
    workbench.documents()[0]
        .snapshot
        .clone()
        .ok_or_else(|| format!("{label}: no workspace snapshot was published"))
}

fn log_text(workbench: &Workbench) -> String {
    document(workbench)
        .logs
        .iter()
        .map(|line| line.render())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The kind-aware variable list as `name=display (kind)`, the form the report quotes.
fn variables_text(snapshot: &WorkspaceSnapshot) -> String {
    if snapshot.state.variables.is_empty() {
        return "(none)".to_string();
    }
    snapshot
        .state
        .variables
        .iter()
        .map(|variable| format!("{}={} ({})", variable.name, variable.display, variable.kind))
        .collect::<Vec<_>>()
        .join(", ")
}

fn function_text(snapshot: &WorkspaceSnapshot, name: &str) -> String {
    match snapshot
        .state
        .functions
        .iter()
        .find(|function| function.name == name)
    {
        Some(function) => format!(
            "{}({}) origin={} builtin={}",
            function.name,
            function.parameters.join(", "),
            function.origin().label(),
            function.is_builtin
        ),
        None => "(missing)".to_string(),
    }
}

/// The display form of one variable in a snapshot, or `(missing)`.
fn variable_display(snapshot: &WorkspaceSnapshot, name: &str) -> String {
    snapshot
        .state
        .variables
        .iter()
        .find(|variable| variable.name == name)
        .map(|variable| variable.display.clone())
        .unwrap_or_else(|| "(missing)".to_string())
}

/// How many digits follow the decimal point in `text` (0 when it has none).
fn fractional_digits(text: &str) -> usize {
    text.split_once('.')
        .map(|(_, fraction)| fraction.len())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------------------------
// J-02 - basic script
// ---------------------------------------------------------------------------------------------

/// `a = 2`, `b = a^10`, `b` -> result `1024`, variables `a`,`b` with kinds, log, cursor.
fn journey_j02() -> Result<(), String> {
    let mut checks = Checks::new("J-02");
    let source = "a = 2\nb = a^10\nb\n".to_string();
    let mut workbench = new_workbench();
    workbench.set_source(0, source.clone());
    run_document(&mut workbench, "J-02")?;
    let snapshot = snapshot(&workbench, "J-02")?;
    let logs = log_text(&workbench);
    let status = workbench.status_text(1, 1);

    println!("JOURNEY J-02 observed input={source:?}");
    println!("JOURNEY J-02 observed result={:?}", snapshot.result);
    println!(
        "JOURNEY J-02 observed variables=[{}]",
        variables_text(&snapshot)
    );
    println!(
        "JOURNEY J-02 observed revision={} reused={} committed={}/{} precision={}",
        snapshot.revision.0,
        document(&workbench).reused,
        snapshot.committed_steps,
        snapshot.total,
        document(&workbench).precision
    );
    println!("JOURNEY J-02 observed log={logs:?}");
    println!("JOURNEY J-02 observed cursor+status={status:?}");

    let a = snapshot
        .state
        .variables
        .iter()
        .find(|variable| variable.name == "a");
    let b = snapshot
        .state
        .variables
        .iter()
        .find(|variable| variable.name == "b");
    checks.expect("result", "1024", snapshot.result.clone());
    checks.expect(
        "variable a display",
        "2",
        a.map(|variable| variable.display.clone())
            .unwrap_or_else(|| "(missing)".to_string()),
    );
    checks.expect(
        "variable a kind",
        "Natural",
        a.map(|variable| variable.kind.clone())
            .unwrap_or_else(|| "(missing)".to_string()),
    );
    checks.expect(
        "variable b display",
        "1024",
        b.map(|variable| variable.display.clone())
            .unwrap_or_else(|| "(missing)".to_string()),
    );
    checks.expect(
        "variable b kind",
        "Natural",
        b.map(|variable| variable.kind.clone())
            .unwrap_or_else(|| "(missing)".to_string()),
    );
    checks.expect_that(
        "run log line",
        "contains Run started, = 1024 and Run finished",
        format!(
            "started={} result={} finished={}",
            logs.contains("Run started: 3 statements"),
            logs.contains("= 1024"),
            logs.contains("Run finished: 3 statements committed")
        ),
        logs.contains("Run started: 3 statements")
            && logs.contains("= 1024")
            && logs.contains("Run finished: 3 statements committed"),
    );
    checks.expect_that(
        "cursor position",
        "status names Ln 1, Col 1 and run Finished",
        status.clone(),
        status.contains("Ln 1, Col 1") && status.contains("run Finished"),
    );
    checks.expect("committed steps", "3", snapshot.committed_steps.to_string());
    checks.expect_that(
        "no diagnostic",
        "none",
        format!("{:?}", snapshot.diagnostic),
        snapshot.diagnostic.is_none(),
    );
    checks.expect(
        "snapshot revision matches the submitted source revision",
        SourceRevision(document(&workbench).revision).0.to_string(),
        snapshot.revision.0.to_string(),
    );
    checks.finish()
}

// ---------------------------------------------------------------------------------------------
// J-03 - precision
// ---------------------------------------------------------------------------------------------

/// The reference script: change precision to 20, evaluate `sqrt(2)`, then to 1000 and again.
fn j03_source() -> String {
    "setprecision(20)\nsqrt(2)\nsetprecision(1000)\nsqrt(2)\n".to_string()
}

/// `setprecision(20)` + `sqrt(2)`, then precision 1000 + rerun.
///
/// The observable is the display at each precision inside the same script: 20 decimal places
/// (19 after the display trims sqrt(2)'s trailing zero) and the full 1000. The second run is
/// submitted after the precision control moves from 20 to 1000; its reused count must stay 0,
/// because a precision change invalidates any cached rendering.
fn journey_j03() -> Result<(), String> {
    let mut checks = Checks::new("J-03");
    let source = j03_source();
    let mut workbench = new_workbench();
    workbench.set_source(0, source.clone());
    workbench.set_precision(0, 20);
    let visible_20 = workbench.status_text(1, 1);
    println!("JOURNEY J-03 observed input={source:?}");
    println!("JOURNEY J-03 observed visible precision before run 1={visible_20:?}");
    run_document(&mut workbench, "J-03 run 1")?;
    let first_results = document(&workbench).committed_results.clone();
    let first_snapshot = snapshot(&workbench, "J-03 run 1")?;
    let first_reused = document(&workbench).reused;
    let at20 = first_results.get(1).cloned().unwrap_or_default();
    let at1000 = first_results.get(3).cloned().unwrap_or_default();
    println!("JOURNEY J-03 observed run 1 statements={first_results:?}");
    println!(
        "JOURNEY J-03 observed run 1 diagnostics={:?}",
        first_snapshot.diagnostic
    );
    println!(
        "JOURNEY J-03 observed at 20 places={:?} digits={} sha256={}",
        at20,
        fractional_digits(&at20),
        sha256_hex(at20.as_bytes())
    );
    println!(
        "JOURNEY J-03 observed at 1000 places digits={} sha256={} prefix={:?}",
        fractional_digits(&at1000),
        sha256_hex(at1000.as_bytes()),
        at1000.chars().take(24).collect::<String>()
    );
    println!(
        "JOURNEY J-03 observed run 1 revision={} reused={first_reused} final result sha256={}",
        first_snapshot.revision.0,
        sha256_hex(first_snapshot.result.as_bytes())
    );

    checks.expect(
        "run 1 visible precision",
        "precision 20",
        if visible_20.contains("precision 20") {
            "precision 20".to_string()
        } else {
            visible_20.clone()
        },
    );
    checks.expect_that(
        "run 1 commits every statement",
        "4 results",
        format!("{} results", first_results.len()),
        first_results.len() == 4,
    );
    checks.expect(
        "sqrt(2) at precision 20",
        J03_20_PLACE_DISPLAY,
        at20.clone(),
    );
    checks.expect(
        "sqrt(2) at precision 20 renders 19 fractional digits",
        "19",
        fractional_digits(&at20).to_string(),
    );
    checks.expect(
        "sqrt(2) at precision 1000 renders 1000 fractional digits",
        "1000",
        fractional_digits(&at1000).to_string(),
    );
    checks.expect(
        "sqrt(2) at precision 1000 is the reference value",
        J03_1000_DIGIT_SHA256,
        sha256_hex(at1000.as_bytes()),
    );
    checks.expect(
        "the run's final result is the 1000-place value",
        sha256_hex(at1000.as_bytes()),
        sha256_hex(first_snapshot.result.as_bytes()),
    );
    checks.expect("run 1 reused count", "0", first_reused.to_string());

    // The precision control changes to 1000 and the same document runs again. The second run
    // must re-render at 1000 rather than reuse the 20-place value, so reused stays 0.
    workbench.set_precision(0, 1000);
    let visible_1000 = workbench.status_text(1, 1);
    run_document(&mut workbench, "J-03 run 2")?;
    let second_results = document(&workbench).committed_results.clone();
    let second_snapshot = snapshot(&workbench, "J-03 run 2")?;
    let second_reused = document(&workbench).reused;
    let second_at20 = second_results.get(1).cloned().unwrap_or_default();
    let second_at1000 = second_results.get(3).cloned().unwrap_or_default();
    println!("JOURNEY J-03 observed visible precision before run 2={visible_1000:?}");
    println!("JOURNEY J-03 observed run 2 statements={second_results:?}");
    println!(
        "JOURNEY J-03 observed run 2 revision={} reused={second_reused} final result sha256={}",
        second_snapshot.revision.0,
        sha256_hex(second_snapshot.result.as_bytes())
    );

    checks.expect(
        "run 2 visible precision",
        "precision 1000",
        if visible_1000.contains("precision 1000") {
            "precision 1000".to_string()
        } else {
            visible_1000.clone()
        },
    );
    checks.expect("run 2 sqrt(2) at precision 20", at20, second_at20);
    checks.expect(
        "run 2 sqrt(2) at precision 1000",
        J03_1000_DIGIT_SHA256,
        sha256_hex(second_at1000.as_bytes()),
    );
    checks.expect(
        "run 2 reused count after the precision change",
        "0",
        second_reused.to_string(),
    );
    checks.expect_that(
        "the precision change invalidated the cached result",
        "reused 0 and a re-rendered 1000-digit value",
        format!(
            "reused={second_reused} digits={}",
            fractional_digits(&second_at1000)
        ),
        second_reused == 0 && fractional_digits(&second_at1000) == 1000,
    );
    checks.finish()
}

// ---------------------------------------------------------------------------------------------
// J-07 - incremental reuse
// ---------------------------------------------------------------------------------------------

/// REF-incremental's script: three definitions plus the expression that consumes them.
fn j07_source() -> &'static str {
    "a = 2\nb = a + 1\nc = pi(100)\nb + c\n"
}

/// The same script with only its last statement edited.
fn j07_edited_source() -> &'static str {
    "a = 2\nb = a + 1\nc = pi(100)\nb + c + 1\n"
}

/// REF-incremental's `pi(100)` display, byte for byte.
const J07_PI100: &str = "3.1415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170679";
/// REF-incremental's `b + c` result.
const J07_RESULT: &str = "6.1415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170679";
/// REF-incremental's edited `b + c + 1` result.
const J07_EDITED_RESULT: &str = "7.1415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170679";

/// `a = 2`; `b = a + 1`; `c = pi(100)`; `b + c`, run twice with the last statement edited.
///
/// The expected observable is REF-incremental's: `reused=3` on the second run, identical values
/// for the three unchanged statements, and a reused statement is never reported as recomputed
/// (the per-statement flags are read from the typed `StepCommitted` events).
fn journey_j07() -> Result<(), String> {
    let mut checks = Checks::new("J-07");
    let first_source = j07_source().to_string();
    let edited_source = j07_edited_source().to_string();
    let mut workbench = new_workbench();
    workbench.set_source(0, first_source.clone());
    println!("JOURNEY J-07 observed input={first_source:?}");

    run_document(&mut workbench, "J-07 run 1")?;
    let first = snapshot(&workbench, "J-07 run 1")?;
    let first_reused = document(&workbench).reused;
    let first_flags = document(&workbench).committed_reused.clone();
    let first_results = document(&workbench).committed_results.clone();
    println!(
        "JOURNEY J-07 observed run 1 revision={} reused={first_reused} flags={first_flags:?} committed={}/{} results={first_results:?}",
        first.revision.0, first.committed_steps, first.total
    );
    println!(
        "JOURNEY J-07 observed run 1 result={:?} variables=[{}]",
        first.result,
        variables_text(&first)
    );

    workbench.set_source(0, edited_source.clone());
    run_document(&mut workbench, "J-07 run 2")?;
    let second = snapshot(&workbench, "J-07 run 2")?;
    let second_reused = document(&workbench).reused;
    let second_flags = document(&workbench).committed_reused.clone();
    let second_results = document(&workbench).committed_results.clone();
    println!("JOURNEY J-07 observed edited input={edited_source:?}");
    println!(
        "JOURNEY J-07 observed run 2 revision={} reused={second_reused} flags={second_flags:?} committed={}/{} results={second_results:?}",
        second.revision.0, second.committed_steps, second.total
    );
    println!(
        "JOURNEY J-07 observed run 2 result={:?} variables=[{}]",
        second.result,
        variables_text(&second)
    );

    checks.expect("run 1 reused count", "0", first_reused.to_string());
    checks.expect(
        "run 2 reused count (REF-incremental)",
        "3",
        second_reused.to_string(),
    );
    checks.expect(
        "run 2 per-statement reuse flags",
        "[true, true, true, false]",
        format!("{second_flags:?}"),
    );
    checks.expect_that(
        "the reused count is exactly the number of reused statements",
        second_reused.to_string(),
        second_flags
            .iter()
            .filter(|flag| **flag)
            .count()
            .to_string(),
        second_reused == second_flags.iter().filter(|flag| **flag).count(),
    );
    checks.expect("run 1 result", J07_RESULT, first.result.clone());
    checks.expect("run 1 c", J07_PI100, variable_display(&first, "c"));
    checks.expect("run 2 result", J07_EDITED_RESULT, second.result.clone());
    checks.expect("run 2 c", J07_PI100, variable_display(&second, "c"));
    for name in ["a", "b", "c"] {
        checks.expect(
            &format!("the reused statement's value for {name} is identical"),
            variable_display(&first, name),
            variable_display(&second, name),
        );
    }
    checks.expect(
        "the three reused statements publish the same committed results",
        format!("{:?}", &first_results[..3]),
        format!("{:?}", &second_results[..3]),
    );
    checks.expect(
        "run 2 committed steps",
        "4",
        second.committed_steps.to_string(),
    );
    checks.expect("run 2 total statements", "4", second.total.to_string());
    checks.finish()
}

// ---------------------------------------------------------------------------------------------
// J-08 - dependency invalidation
// ---------------------------------------------------------------------------------------------

/// The function chain J-08 redefines: `f` is called by `g`, which is called by `h`, which `y` calls.
fn j08_function_source(body: &str) -> String {
    format!(
        "func f(x) = {body}\nfunc g(x) = f(x) * 2\nfunc h(x) = g(x) + 1\ny = h(3)\nz = pi(50)\n"
    )
}

/// `a = 2`; `b = a + 1`; `c = pi(100)`; then change `a`; then redefine `func f`.
///
/// The expected observable is that transitive dependents recompute (`b` after `a`; `y` after
/// `f` through `g` and `h`) while independent statements stay reused (`c`, `z`), reported one
/// statement at a time.
fn journey_j08() -> Result<(), String> {
    let mut checks = Checks::new("J-08");
    let mut workbench = new_workbench();

    // Phase 1: a variable edit.
    let first_source = "a = 2\nb = a + 1\nc = pi(100)\n";
    let edited_source = "a = 5\nb = a + 1\nc = pi(100)\n";
    workbench.set_source(0, first_source.to_string());
    run_document(&mut workbench, "J-08 variable run 1")?;
    let before = snapshot(&workbench, "J-08 variable run 1")?;
    println!(
        "JOURNEY J-08 observed variable run 1 reused={} b={:?} c={:?}",
        document(&workbench).reused,
        variable_display(&before, "b"),
        variable_display(&before, "c")
    );
    let original_c = variable_display(&before, "c");

    workbench.set_source(0, edited_source.to_string());
    run_document(&mut workbench, "J-08 variable run 2")?;
    let after = snapshot(&workbench, "J-08 variable run 2")?;
    let variable_first_flags = document(&workbench).committed_reused.clone();
    println!(
        "JOURNEY J-08 observed variable run 2 reused={} flags={variable_first_flags:?} a={:?} b={:?} c={:?}",
        document(&workbench).reused,
        variable_display(&after, "a"),
        variable_display(&after, "b"),
        variable_display(&after, "c")
    );

    checks.expect(
        "changing a: b recomputes and c reuses",
        "[false, false, true]",
        format!("{variable_first_flags:?}"),
    );
    checks.expect(
        "changing a: reused count",
        "1",
        document(&workbench).reused.to_string(),
    );
    checks.expect(
        "changing a: b sees the new a",
        "6",
        variable_display(&after, "b"),
    );
    checks.expect(
        "changing a: c is unchanged",
        original_c,
        variable_display(&after, "c"),
    );

    // Phase 2: a function redefinition.
    let function_source = j08_function_source("x + 1");
    let function_edited = j08_function_source("x + 10");
    workbench.set_source(0, function_source.clone());
    run_document(&mut workbench, "J-08 function run 1")?;
    let before = snapshot(&workbench, "J-08 function run 1")?;
    println!(
        "JOURNEY J-08 observed function run 1 reused={} y={:?} z={:?}",
        document(&workbench).reused,
        variable_display(&before, "y"),
        variable_display(&before, "z")
    );
    let original_z = variable_display(&before, "z");

    workbench.set_source(0, function_edited.clone());
    run_document(&mut workbench, "J-08 function run 2")?;
    let after = snapshot(&workbench, "J-08 function run 2")?;
    let function_flags = document(&workbench).committed_reused.clone();
    println!("JOURNEY J-08 observed function edited input={function_edited:?}");
    println!(
        "JOURNEY J-08 observed function run 2 reused={} flags={function_flags:?} y={:?} z={:?}",
        document(&workbench).reused,
        variable_display(&after, "y"),
        variable_display(&after, "z")
    );

    checks.expect(
        "redefining f: both definitions evaluate and the call site recomputes",
        "[false, false, false, false, true]",
        format!("{function_flags:?}"),
    );
    checks.expect(
        "redefining f: reused count",
        "1",
        document(&workbench).reused.to_string(),
    );
    checks.expect_that(
        "redefining f: the reuse count equals the reused statements",
        document(&workbench).reused.to_string(),
        function_flags
            .iter()
            .filter(|flag| **flag)
            .count()
            .to_string(),
        document(&workbench).reused == function_flags.iter().filter(|flag| **flag).count(),
    );
    checks.expect(
        "redefining f: y is recomputed through g and h",
        "27",
        variable_display(&after, "y"),
    );
    checks.expect(
        "redefining f: z is reused",
        original_z,
        variable_display(&after, "z"),
    );
    checks.expect(
        "redefining f: the recomputed y is not the old value",
        "9",
        variable_display(&before, "y"),
    );
    checks.finish()
}

// ---------------------------------------------------------------------------------------------
// J-09 - side effects
// ---------------------------------------------------------------------------------------------

/// `print` and `plot` twice: each run executes its effects, one line per print within a run.
fn journey_j09(out: &Path) -> Result<(), String> {
    let mut checks = Checks::new("J-09");
    let source =
        "print(\"alpha\")\nplot(1..10, (1..10)^2, \"side effects\")\nprint(\"beta\")\n".to_string();
    let mut workbench = new_workbench();
    workbench.set_source(0, source.clone());
    println!("JOURNEY J-09 observed input={source:?}");

    let mut run_plots: Vec<(String, usize, String)> = Vec::new();
    for run in 1..=2u8 {
        run_document(&mut workbench, &format!("J-09 run {run}"))?;
        let snapshot = snapshot(&workbench, &format!("J-09 run {run}"))?;
        let output = snapshot.output.clone();
        let stdout_lines = document(&workbench)
            .logs
            .iter()
            .filter(|line| line.kind == LogKind::Stdout)
            .count();
        println!(
            "JOURNEY J-09 observed run {run} revision={} reused={} diagnostics={:?}",
            snapshot.revision.0,
            document(&workbench).reused,
            snapshot.diagnostic
        );
        let plot = snapshot
            .plot
            .as_ref()
            .ok_or_else(|| format!("J-09 run {run}: the run produced no plot capture"))?;
        let on_disk = std::fs::read(&plot.path).unwrap_or_default();
        // A copy in the evidence directory: the engine reuses `run-N/plot.svg` across workbenches,
        // so the appdata copy is not durable evidence on its own.
        let copy_path = out.join(format!("J-09-run{run}.svg"));
        std::fs::write(&copy_path, plot.svg.as_bytes())
            .map_err(|error| format!("J-09: cannot write {}: {error}", copy_path.display()))?;
        println!(
            "JOURNEY J-09 observed run {run} output={output:?} stdout-log-lines={stdout_lines} plot={:?} copy={:?} bytes={} sha256={}",
            plot.path,
            copy_path,
            plot.svg.len(),
            sha256_hex(plot.svg.as_bytes())
        );

        checks.expect_that(
            &format!("run {run} executed both print statements"),
            "[\"alpha\", \"beta\"]",
            format!("{output:?}"),
            output == vec!["alpha".to_string(), "beta".to_string()],
        );
        checks.expect_that(
            &format!("run {run} has no duplicated print line"),
            "one `alpha` and one `beta`",
            format!(
                "alpha={} beta={}",
                output.iter().filter(|line| *line == "alpha").count(),
                output.iter().filter(|line| *line == "beta").count()
            ),
            output.iter().filter(|line| *line == "alpha").count() == 1
                && output.iter().filter(|line| *line == "beta").count() == 1,
        );
        checks.expect_that(
            &format!("run {run} executed the plot statement"),
            "an SVG on disk matching the capture",
            format!(
                "bytes={} on_disk={} equal={}",
                plot.svg.len(),
                on_disk.len(),
                on_disk == plot.svg.as_bytes()
            ),
            plot.svg.contains("<svg") && !on_disk.is_empty() && on_disk == plot.svg.as_bytes(),
        );
        checks.expect(
            &format!("run {run} stdout log lines"),
            (run as usize * 2).to_string(),
            stdout_lines.to_string(),
        );
        run_plots.push((
            plot.path.clone(),
            plot.svg.len(),
            sha256_hex(plot.svg.as_bytes()),
        ));
    }
    let (first_path, first_bytes, first_hash) = &run_plots[0];
    let (second_path, second_bytes, second_hash) = &run_plots[1];
    checks.expect_that(
        "each run wrote a fresh plot file",
        "two different engine paths",
        format!("{first_path:?} vs {second_path:?}"),
        first_path != second_path,
    );
    checks.expect_that(
        "both runs rendered the same deterministic effect",
        "identical byte counts and SHA-256",
        format!("{first_bytes}/{first_hash} vs {second_bytes}/{second_hash}"),
        first_bytes == second_bytes && first_hash == second_hash,
    );
    checks.finish()
}

// ---------------------------------------------------------------------------------------------
// J-10 - clear-variables semantics
// ---------------------------------------------------------------------------------------------

/// A user function and a variable, then Clear Variables: the variable goes, the function stays.
fn journey_j10() -> Result<(), String> {
    let mut checks = Checks::new("J-10");
    let source = "func f(x) = x + 1\ny = 41\nf(y)\n".to_string();
    let mut workbench = new_workbench();
    workbench.set_source(0, source.clone());
    println!("JOURNEY J-10 observed input={source:?}");
    run_document(&mut workbench, "J-10")?;
    let before = snapshot(&workbench, "J-10")?;
    let before_variables = variables_text(&before);
    let before_functions = before.functions.len();
    let before_f = function_text(&before, "f");
    let y = before
        .state
        .variables
        .iter()
        .find(|variable| variable.name == "y")
        .cloned();
    println!(
        "JOURNEY J-10 observed before clear result={:?}",
        before.result
    );
    println!("JOURNEY J-10 observed before clear variables=[{before_variables}]");
    println!("JOURNEY J-10 observed before clear f={before_f} functions={before_functions}");
    println!(
        "JOURNEY J-10 observed before clear revision={} reused={} diagnostics={:?}",
        before.revision.0,
        document(&workbench).reused,
        before.diagnostic
    );
    checks.expect("result", "42", before.result.clone());
    checks.expect(
        "y is defined before the clear",
        "41 (Natural)",
        y.as_ref()
            .map(|variable| format!("{} ({})", variable.display, variable.kind))
            .unwrap_or_else(|| "(missing)".to_string()),
    );
    checks.expect(
        "f is a user function before the clear",
        "f(x) origin=user builtin=false",
        before_f.clone(),
    );

    let removed = workbench
        .clear_variables(0)
        .ok_or_else(|| "J-10: Clear Variables was refused on a finished workspace".to_string())?;
    let after = snapshot(&workbench, "J-10")?;
    let after_variables = variables_text(&after);
    let after_f = function_text(&after, "f");
    let after_functions = after.functions.len();
    let log = log_text(&workbench);
    println!(
        "JOURNEY J-10 observed after clear removed={removed} variables=[{after_variables}] f={after_f} functions={after_functions} result={:?}",
        after.result
    );
    println!(
        "JOURNEY J-10 observed after clear diagnostics={:?}",
        after.diagnostic
    );
    println!(
        "JOURNEY J-10 observed after clear log contains={:?}",
        if log.contains("Variables cleared") {
            "Variables cleared"
        } else {
            "(missing)"
        }
    );

    checks.expect_that(
        "the variable is gone",
        "0 variables",
        format!("{} variables: [{after_variables}]", after.variables.len()),
        after.variables.is_empty() && after.state.variables.is_empty(),
    );
    checks.expect_that(
        "the kind-aware variable projection is empty too",
        "0 variables",
        after.state.variables.len().to_string(),
        after.state.variables.is_empty(),
    );
    checks.expect(
        "the user function is retained",
        "f(x) origin=user builtin=false",
        after_f,
    );
    checks.expect(
        "the function table count is unchanged",
        before_functions.to_string(),
        after_functions.to_string(),
    );
    checks.expect(
        "the last result is untouched by the clear",
        before.result.clone(),
        after.result.clone(),
    );
    checks.expect_that(
        "the clear is logged",
        "Variables cleared: N removed; M functions retained.",
        if log.contains("Variables cleared") {
            "Variables cleared".to_string()
        } else {
            "(missing)".to_string()
        },
        log.contains("Variables cleared"),
    );
    checks.finish()
}

// ---------------------------------------------------------------------------------------------
// J-15 - plot presentation
// ---------------------------------------------------------------------------------------------

/// `plot(1..10, (1..10)^2)`: census, zoom, pan, fit, raster and byte-identical SVG export.
fn journey_j15(out: &Path) -> Result<(), String> {
    let mut checks = Checks::new("J-15");
    let source = "plot(1..10, (1..10)^2)\n".to_string();
    let mut workbench = new_workbench();
    workbench.set_source(0, source.clone());
    println!("JOURNEY J-15 observed input={source:?}");
    run_document(&mut workbench, "J-15")?;
    let snapshot = snapshot(&workbench, "J-15")?;
    println!(
        "JOURNEY J-15 observed revision={} reused={} diagnostics={:?}",
        snapshot.revision.0,
        document(&workbench).reused,
        snapshot.diagnostic
    );
    let capture = snapshot
        .plot
        .clone()
        .ok_or_else(|| "J-15: the run produced no plot capture".to_string())?;
    let counts = plot_element_counts(&capture.svg);
    let hash = sha256_hex(capture.svg.as_bytes());
    println!(
        "JOURNEY J-15 observed plot title={:?} path={:?} bytes={} rect={} line={} text={} polyline={} sha256={hash}",
        capture.title,
        capture.path,
        capture.svg.len(),
        counts[0],
        counts[1],
        counts[2],
        counts[3]
    );

    checks.expect("SVG byte count", "15000", capture.svg.len().to_string());
    checks.expect(
        "SVG element census rect/line/text/polyline",
        "2/13/11/1",
        format!("{}/{}/{}/{}", counts[0], counts[1], counts[2], counts[3]),
    );
    checks.expect("SVG SHA-256", J15_SVG_SHA256, hash.clone());

    // The product's plot pane, not a look-alike: the same Plot::set_svg the window calls.
    let mut pane = Plot::new();
    if !pane.set_svg(&capture.svg) {
        return Err(format!(
            "J-15: the plot pane rejected the engine SVG: {:?}",
            pane.error
        ));
    }
    let pane_rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 800.0));
    pane.canvas = pane_rect;
    pane.fit_scale = PlotView::fit_scale(pane_rect, pane.svg_size());
    let fit_view = pane.view;
    let fit_percent = pane.zoom_percent();
    println!(
        "JOURNEY J-15 observed pane parsed svg={}x{} points fit_scale={:.5} fit_view zoom={:.4} pan=({:.2},{:.2}) percent={:.1}%",
        pane.svg_w,
        pane.svg_h,
        pane.fit_scale,
        fit_view.zoom,
        fit_view.pan.x,
        fit_view.pan.y,
        fit_percent
    );
    checks.expect("parsed canvas width", "800", format!("{:.0}", pane.svg_w));
    checks.expect("parsed canvas height", "600", format!("{:.0}", pane.svg_h));
    checks.expect_that(
        "Fit is the identity view",
        "zoom 1, pan 0",
        format!(
            "zoom={} pan=({},{})",
            fit_view.zoom, fit_view.pan.x, fit_view.pan.y
        ),
        fit_view == PlotView::IDENTITY,
    );

    pane.zoom_step(1.25);
    let zoomed = pane.view;
    println!(
        "JOURNEY J-15 observed zoomed view zoom={:.4} pan=({:.2},{:.2}) percent={:.1}% scale={:.4}",
        zoomed.zoom,
        zoomed.pan.x,
        zoomed.pan.y,
        pane.zoom_percent(),
        zoomed.scale(pane.fit_scale)
    );
    checks.expect_that(
        "zoom changes the view transform",
        "zoom 1.25 after one step, different from Fit",
        format!(
            "zoom={} pan=({},{})",
            zoomed.zoom, zoomed.pan.x, zoomed.pan.y
        ),
        (zoomed.zoom - 1.25).abs() < 1e-6 && zoomed != fit_view,
    );
    checks.expect_that(
        "zoom changes the rendered scale and percentage",
        "a strictly larger scale and percentage than Fit",
        format!(
            "scale {:.4} -> {:.4}, {:.1}% -> {:.1}%",
            fit_view.scale(pane.fit_scale),
            zoomed.scale(pane.fit_scale),
            fit_percent,
            pane.zoom_percent()
        ),
        zoomed.scale(pane.fit_scale) > fit_view.scale(pane.fit_scale)
            && pane.zoom_percent() > fit_percent,
    );

    let panned = zoomed.pan_by(
        vec2(-40.0, 30.0),
        pane_rect,
        pane.svg_size(),
        pane.fit_scale,
    );
    pane.view = panned;
    println!(
        "JOURNEY J-15 observed pan by (-40, 30): pan=({:.2},{:.2})",
        panned.pan.x, panned.pan.y
    );
    checks.expect_that(
        "pan moves the zoomed document",
        "pan (-40, 30)",
        format!("({:.2},{:.2})", panned.pan.x, panned.pan.y),
        (panned.pan.x + 40.0).abs() < 1e-3 && (panned.pan.y - 30.0).abs() < 1e-3,
    );
    pane.fit();
    println!(
        "JOURNEY J-15 observed fit restores zoom={:.4} pan=({:.2},{:.2})",
        pane.view.zoom, pane.view.pan.x, pane.view.pan.y
    );
    checks.expect_that(
        "Fit restores the identity view",
        "zoom 1, pan 0",
        format!(
            "zoom={} pan=({},{})",
            pane.view.zoom, pane.view.pan.x, pane.view.pan.y
        ),
        pane.view == PlotView::IDENTITY,
    );

    let raster_1x = pane
        .raster_pixmap(1.0, MAX_EXPORT_SIDE as f32)
        .ok_or_else(|| "J-15: the pane could not rasterise the SVG at 1x".to_string())?;
    let raster_2x = pane
        .raster_pixmap(2.0, MAX_EXPORT_SIDE as f32)
        .ok_or_else(|| "J-15: the pane could not rasterise the SVG at 2x".to_string())?;
    println!(
        "JOURNEY J-15 observed raster 1x={}x{}px 2x={}x{}px",
        raster_1x.0.width(),
        raster_1x.0.height(),
        raster_2x.0.width(),
        raster_2x.0.height()
    );
    checks.expect(
        "raster at 1x",
        "800x600",
        format!("{}x{}", raster_1x.0.width(), raster_1x.0.height()),
    );
    checks.expect(
        "raster at 2x (HiDPI)",
        "1600x1200",
        format!("{}x{}", raster_2x.0.width(), raster_2x.0.height()),
    );
    let raster_path = out.join("J-15-plot-2x.png");
    raster_2x
        .0
        .save_png(&raster_path)
        .map_err(|error| format!("J-15: cannot write {}: {error}", raster_path.display()))?;
    println!(
        "JOURNEY J-15 observed raster artifact={:?} bytes={}",
        raster_path,
        std::fs::metadata(&raster_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0)
    );

    let export_path = out.join("J-15-export.svg");
    let exported = pane
        .export_svg_to(&export_path)
        .map_err(|error| format!("J-15: SVG export failed: {error}"))?;
    let exported_bytes = std::fs::read(&export_path)
        .map_err(|error| format!("J-15: cannot read {}: {error}", export_path.display()))?;
    let engine_bytes = std::fs::read(&capture.path).unwrap_or_default();
    let export_hash = sha256_hex(&exported_bytes);
    println!(
        "JOURNEY J-15 observed exported={:?} bytes={exported} sha256={export_hash} byte_identical_to_capture={} engine_file_identical={}",
        export_path,
        exported_bytes == capture.svg.as_bytes(),
        engine_bytes == capture.svg.as_bytes()
    );
    checks.expect("exported byte count", "15000", exported.to_string());
    checks.expect("exported SVG SHA-256", J15_SVG_SHA256, export_hash);
    checks.expect_that(
        "exported SVG is byte-identical to the captured SVG",
        "equal bytes",
        format!(
            "{} bytes exported, {} bytes captured",
            exported_bytes.len(),
            capture.svg.len()
        ),
        exported_bytes == capture.svg.as_bytes(),
    );
    checks.expect_that(
        "the engine's own plot file holds the captured SVG",
        "equal bytes",
        format!(
            "{} bytes on disk, {} bytes captured",
            engine_bytes.len(),
            capture.svg.len()
        ),
        engine_bytes == capture.svg.as_bytes(),
    );
    checks.finish()
}

// ---------------------------------------------------------------------------------------------
// SHA-256
// ---------------------------------------------------------------------------------------------

/// The SHA-256 round constants.
const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// The SHA-256 of `bytes`, lowercase hex.
///
/// The crate deliberately carries no hash dependency, and the journey evidence quotes the same
/// digest `Get-FileHash -Algorithm SHA256` prints, so the one digest the acceptance mode needs is
/// implemented here. It is unit-tested against the NIST vectors.
fn sha256_hex(bytes: &[u8]) -> String {
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut message = bytes.to_vec();
    let bit_length = (bytes.len() as u64).wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());

    for chunk in message.chunks_exact(64) {
        let mut schedule = [0u32; 64];
        for (word, bytes) in chunk.chunks_exact(4).enumerate() {
            schedule[word] = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        }
        for word in 16..64 {
            let s0 = schedule[word - 15].rotate_right(7)
                ^ schedule[word - 15].rotate_right(18)
                ^ (schedule[word - 15] >> 3);
            let s1 = schedule[word - 2].rotate_right(17)
                ^ schedule[word - 2].rotate_right(19)
                ^ (schedule[word - 2] >> 10);
            schedule[word] = schedule[word - 16]
                .wrapping_add(s0)
                .wrapping_add(schedule[word - 7])
                .wrapping_add(s1);
        }
        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];
        for word in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(choose)
                .wrapping_add(SHA256_K[word])
                .wrapping_add(schedule[word]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }
    state.iter().map(|word| format!("{word:08x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_the_nist_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn fractional_digits_counts_only_digits_after_the_point() {
        assert_eq!(fractional_digits("1024"), 0);
        assert_eq!(fractional_digits(J03_20_PLACE_DISPLAY), 19);
        assert_eq!(fractional_digits("(void)"), 0);
    }

    #[test]
    fn a_mismatch_is_a_failure_and_a_match_is_not() {
        let mut checks = Checks::new("J-XX");
        checks.expect("result", "1024", "1024");
        assert!(checks.finish().is_ok(), "an exact match must pass");

        let mut checks = Checks::new("J-XX");
        checks.expect("result", "1024", "1204");
        assert!(
            checks.finish().is_err(),
            "a mismatched observable must fail instead of passing silently"
        );
    }

    #[test]
    fn only_the_artifact_writing_journeys_need_the_output_directory() {
        for id in ["J-02", "J-03", "J-07", "J-08", "J-10"] {
            assert!(
                !needs_output_dir(id),
                "{id} reads only the workbench snapshot and must not require a writable temp directory"
            );
        }
        for id in ["J-09", "J-15"] {
            assert!(
                needs_output_dir(id),
                "{id} writes artifacts into --out and must create the directory first (DEF-004)"
            );
        }
        assert!(!needs_output_dir("J-04"), "an unknown id writes nothing");
    }

    #[test]
    fn options_require_a_journey_and_reject_unknown_flags() {
        assert!(Options::parse(&[]).is_err());
        assert!(Options::parse(&["--journey".to_string()]).is_err());
        assert!(Options::parse(&["--unknown".to_string()]).is_err());
        let parsed = Options::parse(&[
            "--journey".to_string(),
            "J-02".to_string(),
            "J-15".to_string(),
            "--out".to_string(),
            "C:\\tmp\\journeys".to_string(),
        ])
        .expect("the two ids and the out directory parse");
        assert_eq!(
            parsed.journeys,
            vec!["J-02".to_string(), "J-15".to_string()]
        );
        assert_eq!(parsed.out, Some(PathBuf::from("C:\\tmp\\journeys")));
    }
}
