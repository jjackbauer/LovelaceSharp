//! `--gui-selftest`: the shipped window's run path, driven headless on the real engine.
//!
//! The window and this mode share one implementation: [`Workbench`]. Each check creates a
//! workbench (two document tabs, the real `lovelace_lang::Engine` factory), edits a document's
//! source, calls [`Workbench::run_active`], polls the typed event stream with
//! [`Workbench::wait_until_idle`] and asserts on the document's published state. Nothing here
//! evaluates a script inline: every value, print line, diagnostic, plot and variable comes from
//! `ExecutionHost` events.
//!
//! One line is printed per check. The process exits non-zero when any check fails.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::engine::DEFAULT_PRECISION;
use crate::keyboard::{WorkbenchCommand, command_for_shortcut};
use crate::log_kind::LogKind;
use crate::run_state::RunState;
use crate::workbench::{Workbench, WorkbenchDocument};

type Check = fn() -> Result<String, String>;

const RUN_TIMEOUT: Duration = Duration::from_secs(30);
const CANCEL_TIMEOUT: Duration = Duration::from_secs(2);
/// The single long kernel the long-run and keyboard checks cancel.
///
/// It is ONE statement whose whole cost is an array triple loop (64 million exact multiply-adds
/// on 400x400 identity matrices, roughly 18 s unbudgeted in a debug build), and that loop polls
/// the cancellation seam. The cancel proof is therefore about work inside one kernel call, not
/// about statement boundaries. A single opaque primitive such as `sqrt(2)` at precision 200000
/// has no poll seam and is budgeted as uninterruptible in the round-9 report.
const LONG_RUN_SOURCE: &str = "matmul(eye(400), eye(400))";
/// Budget for one nonblocking `ExecutionHost::next_event` call.
const EVENT_LOOP_BUDGET: Duration = Duration::from_millis(50);
/// How long the event-loop sample runs while the long computation is in flight.
const EVENT_LOOP_SAMPLE: Duration = Duration::from_millis(300);

/// Runs every GUI check, prints one line per check and returns the process exit code.
pub fn run() -> i32 {
    let workbench = Workbench::new();
    println!(
        "GUI_SELFTEST lovelace-desktop {} engine={}",
        env!("CARGO_PKG_VERSION"),
        workbench.engine_name()
    );
    println!(
        "GUI_SELFTEST plot-dir {}",
        crate::engine::default_plot_dir().display()
    );
    let checks: [(&str, Check); 14] = [
        ("two-tabs", check_two_tabs),
        ("arithmetic", check_arithmetic),
        ("print", check_print),
        ("precision", check_precision),
        ("cancel", check_cancel),
        ("syntax-error", check_syntax_error),
        ("plot", check_plot),
        ("plot-dir", check_plot_dir),
        ("keyboard", check_keyboard),
        ("long-run", check_long_run),
        ("run-state-fast", check_run_state_fast),
        ("run-state-cancel", check_run_state_cancel),
        ("run-state-fail", check_run_state_fail),
        ("run-state-stale", check_run_state_stale),
    ];
    // The editor's own history is a Windows-only surface (egui is a Windows dependency), so the
    // check joins the list exactly when that surface is compiled in.
    #[cfg(windows)]
    let checks: Vec<(&str, Check)> = checks
        .into_iter()
        .chain([("undo-redo", check_undo_redo as Check)])
        .collect();
    #[cfg(not(windows))]
    let checks: Vec<(&str, Check)> = checks.into_iter().collect();

    let mut failed = 0usize;
    for &(name, check) in &checks {
        match check() {
            Ok(detail) => println!("GUI_SELFTEST ok {name}: {detail}"),
            Err(reason) => {
                failed += 1;
                println!("GUI_SELFTEST FAIL {name}: {reason}");
            }
        }
    }
    println!(
        "GUI_SELFTEST summary checks={} failed={failed}",
        checks.len()
    );
    if failed == 0 { 0 } else { 1 }
}

fn new_workbench() -> Workbench {
    Workbench::new()
}

fn run_to_idle(workbench: &mut Workbench, label: &str) -> Result<(), String> {
    if !workbench.run_active() {
        return Err(format!("{label}: run_active returned false"));
    }
    if !workbench.wait_until_idle(RUN_TIMEOUT) {
        return Err(format!("{label}: run did not reach a terminal state"));
    }
    Ok(())
}

fn snapshot(
    workbench: &Workbench,
    label: &str,
) -> Result<crate::execution::WorkspaceSnapshot, String> {
    workbench
        .documents()
        .first()
        .and_then(|document| document.snapshot.clone())
        .ok_or_else(|| format!("{label}: no workspace snapshot was published"))
}

fn log_text(workbench: &Workbench, index: usize) -> String {
    workbench
        .documents()
        .get(index)
        .map(|document| {
            document
                .logs
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn expect_log(
    workbench: &Workbench,
    index: usize,
    label: &str,
    predicate: impl Fn(&str) -> bool,
) -> Result<(), String> {
    let text = log_text(workbench, index);
    if predicate(&text) {
        Ok(())
    } else {
        Err(format!("{label}: logs were {text:?}"))
    }
}

/// The observed state sequence since `from` (a transition-log index taken before the run),
/// starting with the state the first logged transition came from.
fn state_sequence(document: &WorkbenchDocument, from: usize) -> Vec<RunState> {
    let transitions = document.transitions();
    let start = transitions
        .get(from)
        .map(|transition| transition.from)
        .unwrap_or_else(|| document.state());
    let mut sequence = vec![start];
    if from < transitions.len() {
        sequence.extend(transitions[from..].iter().map(|transition| transition.to));
    }
    sequence
}

/// The sequence rendered as the exact status-strip labels, joined by ` -> `.
fn state_labels(sequence: &[RunState]) -> String {
    sequence
        .iter()
        .map(|state| state.label())
        .collect::<Vec<_>>()
        .join(" -> ")
}

fn expect_state_sequence(
    document: &WorkbenchDocument,
    from: usize,
    expected: &[RunState],
    label: &str,
) -> Result<Vec<RunState>, String> {
    let observed = state_sequence(document, from);
    if observed != expected {
        return Err(format!(
            "{label}: expected sequence {}, observed {}",
            state_labels(expected),
            state_labels(&observed)
        ));
    }
    Ok(observed)
}

/// Number of terminal run events (Finished/Cancelled/Failed) logged since `from`.
fn terminal_run_events(document: &WorkbenchDocument, from: usize) -> usize {
    document
        .transitions()
        .iter()
        .skip(from.min(document.transitions().len()))
        .filter(|transition| transition.to.is_terminal_run_event())
        .count()
}

/// The measured duration of an adjacent state pair, or a readable "not observed".
fn duration_text(document: &WorkbenchDocument, from: RunState, to: RunState) -> String {
    match document.duration_between(from, to) {
        Some(duration) => format!("{:.3} ms", duration.as_secs_f64() * 1000.0),
        None => "not observed".to_string(),
    }
}

fn check_two_tabs() -> Result<String, String> {
    let mut workbench = new_workbench();
    if workbench.documents().len() != 2 {
        return Err(format!(
            "expected two tabs at start, found {}",
            workbench.documents().len()
        ));
    }
    let first_id = workbench.documents()[0].id;
    let second_id = workbench.documents()[1].id;
    if first_id == second_id {
        return Err("both tabs share one ExecutionHost document id".to_string());
    }
    if workbench.active() != 0 {
        return Err(format!("active tab is {}", workbench.active()));
    }
    let idle_status = workbench.status_text(1, 1);
    if !idle_status.contains("run Idle") {
        return Err(format!(
            "the no-run state must show \"run Idle\": {idle_status:?}"
        ));
    }
    for document in workbench.documents() {
        if document.snapshot.is_some() || document.last_plot.is_some() {
            return Err(format!("{} must start with empty panes", document.title));
        }
        if document.state() != RunState::Idle {
            return Err(format!(
                "{} starts in state {:?}",
                document.title,
                document.state()
            ));
        }
    }

    workbench.set_source(0, "a = 1\n".to_string());
    run_to_idle(&mut workbench, "tab one")?;
    let first_vars = workbench.documents()[0]
        .snapshot
        .as_ref()
        .ok_or("tab one published no snapshot")?
        .variables
        .clone();
    if !first_vars.iter().any(|(name, _)| name == "a") {
        return Err(format!("tab one variables were {first_vars:?}"));
    }

    workbench.set_active(1);
    workbench.set_source(1, "b = 2\n".to_string());
    run_to_idle(&mut workbench, "tab two")?;
    let second_vars = workbench.documents()[1]
        .snapshot
        .as_ref()
        .ok_or("tab two published no snapshot")?
        .variables
        .clone();
    if !second_vars.iter().any(|(name, _)| name == "b") {
        return Err(format!("tab two variables were {second_vars:?}"));
    }
    if first_vars
        != workbench.documents()[0]
            .snapshot
            .as_ref()
            .unwrap()
            .variables
    {
        return Err("the run in tab two changed tab one's variables".to_string());
    }
    Ok(format!(
        "document ids {:?}/{:?}, precision {}, independent snapshots",
        first_id, second_id, DEFAULT_PRECISION
    ))
}

fn check_arithmetic() -> Result<String, String> {
    let mut workbench = new_workbench();
    workbench.set_source(0, "a = 2\nb = a + 10\nb\n".to_string());
    run_to_idle(&mut workbench, "arithmetic")?;
    let document = &workbench.documents()[0];
    if document.state() != RunState::Finished {
        return Err(format!("state was {:?}", document.state()));
    }
    let snapshot = snapshot(&workbench, "arithmetic")?;
    if snapshot.result != "12" {
        return Err(format!("result was {:?}, expected \"12\"", snapshot.result));
    }
    if snapshot.committed_steps != 3 {
        return Err(format!(
            "committed steps were {}, expected 3",
            snapshot.committed_steps
        ));
    }
    let b = snapshot.variables.iter().find(|(name, _)| name == "b");
    let Some((_, value)) = b else {
        return Err(format!(
            "variables do not contain b: {:?}",
            snapshot.variables
        ));
    };
    expect_log(&workbench, 0, "arithmetic", |text| {
        text.contains("Run started") && text.contains("= 12") && text.contains("Run finished")
    })?;
    let status = workbench.status_text(1, 1);
    if !status.contains("rev 1")
        || !status.contains("precision 1000")
        || !status.contains("Ln 1, Col 1")
        || !status.contains("run Finished")
    {
        return Err(format!("status strip was {status:?}"));
    }
    Ok(format!(
        "result {0}, steps {1}, b = {2}, status [{3}]",
        snapshot.result, snapshot.committed_steps, value, status
    ))
}

fn check_print() -> Result<String, String> {
    let mut workbench = new_workbench();
    workbench.set_source(0, "print(\"x\")\n1+1\n".to_string());
    run_to_idle(&mut workbench, "print")?;
    let snapshot = snapshot(&workbench, "print")?;
    if snapshot.output != vec!["x".to_string()] {
        return Err(format!("print output was {:?}", snapshot.output));
    }
    if snapshot.result != "2" {
        return Err(format!("result was {:?}, expected \"2\"", snapshot.result));
    }
    let logs = &workbench.documents()[0].logs;
    if !logs
        .iter()
        .any(|line| line.kind == LogKind::Stdout && line.text == "x")
    {
        return Err(format!("the print line is missing from the log: {logs:?}"));
    }
    if !logs.iter().any(|line| line.text == "= 2") {
        return Err(format!("the terminal result line is missing: {logs:?}"));
    }
    Ok(format!(
        "output {:?}, result {}, committed {}",
        snapshot.output, snapshot.result, snapshot.committed_steps
    ))
}

fn check_precision() -> Result<String, String> {
    let mut workbench = new_workbench();
    workbench.set_source(0, "pi\n".to_string());

    workbench.set_precision(0, 8);
    if workbench.documents()[0].precision != 8 {
        return Err(format!(
            "precision after the control was {}",
            workbench.documents()[0].precision
        ));
    }
    run_to_idle(&mut workbench, "precision 8")?;
    let eight = snapshot(&workbench, "precision 8")?.result;

    workbench.set_precision(0, 20);
    run_to_idle(&mut workbench, "precision 20")?;
    let twenty = snapshot(&workbench, "precision 20")?.result;

    if !eight.starts_with("3.1415926") {
        return Err(format!("pi at precision 8 was {eight:?}"));
    }
    if !twenty.starts_with("3.141592653589793") {
        return Err(format!("pi at precision 20 was {twenty:?}"));
    }
    if eight.len() != 10 || twenty.len() != 22 {
        return Err(format!(
            "expected 8 and 20 fractional digits, got lengths {} and {} ({eight:?} / {twenty:?})",
            eight.len(),
            twenty.len()
        ));
    }
    if eight == twenty {
        return Err("the precision control did not change what the engine rendered".to_string());
    }
    Ok(format!(
        "pi at 8 = {eight}, at 20 = {twenty} (engine set_precision via submit)"
    ))
}

fn check_cancel() -> Result<String, String> {
    let mut workbench = new_workbench();
    let source = (0..50)
        .map(|index| format!("v{index} = sqrt(2)"))
        .collect::<Vec<_>>()
        .join("\n");
    workbench.set_source(0, source);
    if !workbench.run_active() {
        return Err("run_active returned false".to_string());
    }

    let deadline = Instant::now() + RUN_TIMEOUT;
    while workbench.documents()[0].committed == 0 {
        workbench.poll();
        if workbench.documents()[0].state() == RunState::Idle {
            return Err(format!(
                "the 50-statement run finished before a statement could be cancelled (state {:?})",
                workbench.documents()[0].state()
            ));
        }
        if Instant::now() >= deadline {
            return Err("no statement was committed within 30 s".to_string());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let committed_before = workbench.documents()[0].committed;
    let cancel_at = Instant::now();
    if !workbench.cancel_active() {
        return Err("cancel_active returned false".to_string());
    }
    if workbench.documents()[0].state() != RunState::Cancelling {
        return Err(format!(
            "cancel request left state {:?}, expected Cancelling",
            workbench.documents()[0].state()
        ));
    }
    if !workbench.wait_until_idle(CANCEL_TIMEOUT) {
        return Err("the cancelled run did not reach a terminal state within 2 s".to_string());
    }
    let elapsed = cancel_at.elapsed();
    let document = &workbench.documents()[0];
    if document.state() != RunState::Cancelled {
        return Err(format!(
            "terminal state was {:?}, expected Cancelled",
            document.state()
        ));
    }
    if elapsed >= CANCEL_TIMEOUT {
        return Err(format!("cancel took {elapsed:?}, expected under 2 s"));
    }
    if document.committed < 1 || document.committed >= 50 {
        return Err(format!(
            "committed {} of 50 statements; expected the committed prefix",
            document.committed
        ));
    }
    let snapshot = snapshot(&workbench, "cancel")?;
    if snapshot.committed_steps != document.committed {
        return Err(format!(
            "snapshot has {} committed steps, document says {}",
            snapshot.committed_steps, document.committed
        ));
    }
    if !snapshot.variables.iter().any(|(name, _)| name == "v0") {
        return Err(format!(
            "the committed statement is not visible: {:?}",
            snapshot.variables
        ));
    }
    expect_log(&workbench, 0, "cancel", |text| {
        text.contains("Run cancelled after")
    })?;
    if !workbench.status_text(1, 1).contains("Cancelled") {
        return Err(format!(
            "status strip does not say Cancelled: {:?}",
            workbench.status_text(1, 1)
        ));
    }
    Ok(format!(
        "cancelled after {0} of 50 statements in {1:.2} ms (first committed {2})",
        document.committed,
        elapsed.as_secs_f64() * 1000.0,
        committed_before
    ))
}

fn check_syntax_error() -> Result<String, String> {
    let mut workbench = new_workbench();
    workbench.set_source(0, "a = 2\n1 +\n".to_string());
    run_to_idle(&mut workbench, "syntax-error")?;
    let document = &workbench.documents()[0];
    if document.state() != RunState::Failed {
        return Err(format!("state was {:?}, expected Failed", document.state()));
    }
    let snapshot = snapshot(&workbench, "syntax-error")?;
    let diagnostic = snapshot
        .diagnostic
        .as_deref()
        .ok_or("the failed snapshot carries no diagnostic")?;
    if diagnostic.trim().is_empty() {
        return Err("the diagnostic is empty".to_string());
    }
    if !diagnostic.contains("line ") || !diagnostic.contains("column ") {
        return Err(format!("diagnostic has no position: {diagnostic:?}"));
    }
    expect_log(&workbench, 0, "syntax-error", |text| {
        text.contains("Diagnostic:") && text.contains("Run failed")
    })?;
    if workbench.is_running() {
        return Err("the workbench did not return to idle".to_string());
    }
    Ok(format!(
        "{diagnostic} (state {:?}, no run in flight)",
        document.state()
    ))
}

fn check_plot() -> Result<String, String> {
    let mut workbench = new_workbench();
    workbench.set_source(0, "plot([1, 4, 2], [4, 5, 6], \"selftest\")\n".to_string());
    run_to_idle(&mut workbench, "plot")?;
    let document = &workbench.documents()[0];
    let plot = document
        .last_plot
        .as_ref()
        .ok_or("the engine produced no plot capture")?;
    if !plot.svg.contains("<svg") {
        return Err(format!("the plot SVG is {:?}", plot.svg));
    }
    if !plot.svg.contains("polyline") {
        return Err("the engine SVG has no polyline".to_string());
    }
    // DEF-003: the pre-round-87 engine emitted a 3-vertex polyline stub; the current engine
    // (the round-87 plot surface in lovelace-lang) publishes an 800x600 chart with a frame, both
    // axes, tick geometry, tick labels, the title text and ONE densely sampled series polyline.
    // This data has x = [1, 4, 2], which is not strictly increasing, so the renderer samples a
    // parametric Catmull-Rom instead of a natural spline. The assertion below pins that current
    // shape (counts, sample count and the three data vertices), not the obsolete 3-point stub.
    let counts = plot_element_counts(&plot.svg);
    let expected_counts = [2usize, 14, 13, 1];
    if counts != expected_counts {
        return Err(format!(
            "the engine SVG's element counts are rect {} / line {} / text {} / polyline {}, expected {} / {} / {} / {}",
            counts[0], counts[1], counts[2], counts[3],
            expected_counts[0], expected_counts[1], expected_counts[2], expected_counts[3]
        ));
    }
    let parsed = crate::svg::parse(&plot.svg);
    if parsed.width != 800 * crate::svg::MILLI || parsed.height != 600 * crate::svg::MILLI {
        return Err(format!(
            "the engine SVG canvas is {} by {} milli-points, expected 800000 by 600000",
            parsed.width, parsed.height
        ));
    }
    if parsed.polylines.len() != 1 {
        return Err(format!(
            "the engine SVG parsed {} polylines, expected 1",
            parsed.polylines.len()
        ));
    }
    let points = &parsed.polylines[0].points;
    // 1 + ceil(segment1_len_px / 2) + ceil(segment2_len_px / 2) = 1 + 338 + 241 for the mapped
    // (1,4) -> (4,5) -> (2,6) path; the density is the engine's published sampling contract.
    if points.len() != 580 {
        return Err(format!(
            "the engine SVG parsed {} sampled points, expected 580 (the dense parametric spline)",
            points.len()
        ));
    }
    // The polyline still represents the three plotted points: (1,4) first, (4,5) at the branch,
    // (2,6) last, at the same mapped coordinates the engine's own tick geometry uses.
    let expected_vertices = [
        crate::svg::Point { x: 101_818, y: 522_500 },
        crate::svg::Point { x: 738_181, y: 297_500 },
        crate::svg::Point { x: 313_939, y: 72_500 },
    ];
    for vertex in expected_vertices {
        if !points.contains(&vertex) {
            return Err(format!(
                "the engine SVG's series does not pass through ({}, {}) milli-points",
                vertex.x, vertex.y
            ));
        }
    }
    if !parsed.texts.iter().any(|text| text.text == "selftest") {
        return Err("the engine SVG has no title text".to_string());
    }
    Ok(format!(
        "{} SVG bytes, title {:?}, {} rect / {} lines / {} texts / 1 polyline, {} sampled points through the three data vertices",
        plot.svg.len(),
        plot.title,
        counts[0],
        counts[1],
        counts[2],
        points.len()
    ))
}

/// Counts the element kinds the engine's plot renderer publishes, one line per element.
///
/// The engine writes each element on its own line, so counting trimmed line prefixes cannot be
/// confused by a point coordinate or a label that happens to spell a tag name.
fn plot_element_counts(svg: &str) -> [usize; 4] {
    let mut counts = [0usize; 4];
    for line in svg.lines() {
        let tag = line.trim_start();
        if tag.starts_with("<rect") {
            counts[0] += 1;
        } else if tag.starts_with("<line") {
            counts[1] += 1;
        } else if tag.starts_with("<text") {
            counts[2] += 1;
        } else if tag.starts_with("<polyline") {
            counts[3] += 1;
        }
    }
    counts
}

/// Resolves a path the same way the product does, for component-wise comparisons.
fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// True when `path` is `base` or a descendant of it.
fn is_within(base: &Path, path: &Path) -> bool {
    let base = canonical(base);
    let path = canonical(path);
    path.starts_with(&base)
}

/// The long script: ONE genuine long kernel, so the cancel check exercises cancellation inside a
/// kernel call rather than between statements.
fn long_run_source() -> String {
    LONG_RUN_SOURCE.to_string()
}

fn check_plot_dir() -> Result<String, String> {
    let cwd = std::env::current_dir()
        .map_err(|error| format!("plot-dir: no current directory: {error}"))?;
    let mut workbench = new_workbench();
    let plot_root = workbench.plot_dir().to_path_buf();
    let (prepared, creation_error) = crate::engine::prepare_plot_dir();
    if prepared != plot_root {
        return Err(format!(
            "plot-dir: the workbench uses {} but the resolver returned {}",
            plot_root.display(),
            prepared.display()
        ));
    }
    if let Some(message) = creation_error {
        return Err(format!(
            "plot-dir: the app-scoped directory is unusable: {message}"
        ));
    }
    if !plot_root.is_dir() {
        return Err(format!(
            "plot-dir: {} does not exist after preparation",
            plot_root.display()
        ));
    }
    if is_within(&cwd, &plot_root) {
        return Err(format!(
            "plot-dir: resolved directory {} is under the working directory {}",
            plot_root.display(),
            cwd.display()
        ));
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        let appdata = PathBuf::from(appdata);
        if !is_within(&appdata, &plot_root) {
            return Err(format!(
                "plot-dir: {} is not under %APPDATA% ({})",
                plot_root.display(),
                appdata.display()
            ));
        }
    }

    workbench.set_source(0, "plot([1, 4, 2], [4, 5, 6], \"plotdir\")\n".to_string());
    run_to_idle(&mut workbench, "plot-dir")?;
    let plot = workbench.documents()[0]
        .last_plot
        .as_ref()
        .ok_or("plot-dir: the engine produced no plot capture")?;
    let written = PathBuf::from(&plot.path);
    if !is_within(&plot_root, &written) {
        return Err(format!(
            "plot-dir: engine wrote {} outside the resolved directory {}",
            written.display(),
            plot_root.display()
        ));
    }
    if !written.is_file() {
        return Err(format!(
            "plot-dir: engine reported {} but the file does not exist",
            written.display()
        ));
    }
    let on_disk = std::fs::read_to_string(&written)
        .map_err(|error| format!("plot-dir: cannot read {}: {error}", written.display()))?;
    if !on_disk.contains("<svg") || on_disk != plot.svg {
        return Err(format!(
            "plot-dir: {} does not hold the captured SVG ({} bytes on disk, {} captured)",
            written.display(),
            on_disk.len(),
            plot.svg.len()
        ));
    }
    expect_log(&workbench, 0, "plot-dir", |text| {
        text.contains(&format!("Plot directory: {}.", plot_root.display()))
    })?;

    Ok(format!(
        "resolved {} (outside cwd {}), plot file {} ({} bytes), logged at startup",
        plot_root.display(),
        cwd.display(),
        written.display(),
        on_disk.len()
    ))
}

fn check_keyboard() -> Result<String, String> {
    let mut workbench = new_workbench();
    workbench.set_source(0, long_run_source());
    // The run-start breakdown printed below comes from the same marks the native window writes
    // with LOVELACE_RUN_TRACE; in-memory mode keeps the selftest output self-contained.
    crate::run_trace::reset();
    crate::run_trace::enable_memory();

    let run = command_for_shortcut(true, false, true)
        .ok_or("keyboard: Ctrl+Enter did not map to a command")?;
    if run != WorkbenchCommand::Run {
        return Err(format!("keyboard: Ctrl+Enter mapped to {:?}", run.label()));
    }
    if !workbench.command(run) {
        return Err("keyboard: Ctrl+Enter did not start a run".to_string());
    }
    // Let the worker publish RunStarted so the assertions below describe a run that really
    // exists, not only the phase set on submit.
    let started = Instant::now();
    while workbench.documents()[0].total == 0 {
        workbench.poll();
        if Instant::now() - started >= CANCEL_TIMEOUT {
            return Err("keyboard: the run never published RunStarted".to_string());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let running = workbench.status_text(1, 1);
    if !running.contains("run Running") {
        return Err(format!("keyboard: status strip was {running:?}"));
    }
    let toolbar = workbench.toolbar_state();
    if toolbar.run_enabled || !toolbar.cancel_visible || !toolbar.cancel_enabled {
        return Err(format!(
            "keyboard: toolbar while running was {toolbar:?}; expected a disabled Run and a visible Cancel"
        ));
    }

    let cancel =
        command_for_shortcut(false, true, false).ok_or("keyboard: Esc did not map to a command")?;
    if cancel != WorkbenchCommand::Cancel {
        return Err(format!("keyboard: Esc mapped to {:?}", cancel.label()));
    }
    let cancel_at = Instant::now();
    if !workbench.command(cancel) {
        return Err("keyboard: Esc did not request cancellation".to_string());
    }
    if !workbench.wait_until_idle(CANCEL_TIMEOUT) {
        return Err(format!(
            "keyboard: the run did not reach a terminal state within {} ms",
            CANCEL_TIMEOUT.as_millis()
        ));
    }
    let elapsed = cancel_at.elapsed();
    let document = &workbench.documents()[0];
    if document.state() != RunState::Cancelled {
        return Err(format!(
            "keyboard: terminal state was {:?}, expected Cancelled",
            document.state()
        ));
    }
    if !workbench.status_text(1, 1).contains("run Cancelled") {
        return Err(format!(
            "keyboard: status strip was {:?}",
            workbench.status_text(1, 1)
        ));
    }
    let idle = workbench.toolbar_state();
    if !idle.run_enabled || idle.cancel_visible {
        return Err(format!(
            "keyboard: toolbar after the run was {idle:?}; expected Run enabled and no Cancel"
        ));
    }

    let breakdown = crate::run_trace::breakdown_from("command_run_enter")
        .unwrap_or_else(|| "no run-start marks were recorded".to_string());
    crate::run_trace::reset();
    Ok(format!(
        "Ctrl+Enter -> Run (status \"run Running\", Run disabled, Cancel visible); Esc -> run Cancelled in {:.2} ms; toolbar restored; run-start trace [{breakdown}]",
        elapsed.as_secs_f64() * 1000.0
    ))
}

/// EDT-012: the native editor's per-document undo/redo, driven through the same shortcut
/// mapping and command entry point the window's editor uses.
///
/// The check performs two typed edits with a newline between them (so they are two separate
/// bursts and the buffer is `ab\ncd`), then a Ctrl+Z / Ctrl+Y round trip and a second
/// Ctrl+Z / Ctrl+Shift+Z round trip. Every step prints the observed buffer and 0-based caret.
#[cfg(windows)]
fn check_undo_redo() -> Result<String, String> {
    use crate::history::{HistoryCommand, command_for_shortcut};
    use crate::ui::editor::TextEditor;

    let mut editor = TextEditor::from_source("");
    let mut steps: Vec<String> = Vec::new();
    let snap = |label: &str, editor: &TextEditor| {
        format!(
            "{label}: buffer {:?}, caret ({}, {})",
            editor.source(),
            editor.caret_row,
            editor.caret_col
        )
    };

    editor.insert_str("ab");
    steps.push(snap("type \"ab\"", &editor));
    editor.newline();
    steps.push(snap("Enter", &editor));
    editor.insert_str("cd");
    steps.push(snap("type \"cd\"", &editor));

    if editor.source() != "ab\ncd" || (editor.caret_row, editor.caret_col) != (1, 2) {
        return Err(format!(
            "undo-redo: typing `ab`, Enter and `cd` produced {:?} at caret ({}, {}); expected {:?} at (1, 2)",
            editor.source(),
            editor.caret_row,
            editor.caret_col,
            "ab\ncd"
        ));
    }
    if editor.history.undo_steps() != 3 {
        return Err(format!(
            "undo-redo: two typed bursts plus Enter made {} undo steps, expected 3",
            editor.history.undo_steps()
        ));
    }

    let undo = command_for_shortcut(true, false, false, true)
        .ok_or("undo-redo: Ctrl+Z did not map to a history command")?;
    if undo != HistoryCommand::Undo {
        return Err(format!("undo-redo: Ctrl+Z mapped to {}", undo.label()));
    }
    if !editor.command(undo) {
        return Err("undo-redo: Ctrl+Z found nothing to undo".to_string());
    }
    steps.push(snap("Ctrl+Z", &editor));
    if editor.source() != "ab\n" || (editor.caret_row, editor.caret_col) != (1, 0) {
        return Err(format!(
            "undo-redo: Ctrl+Z left {:?} at caret ({}, {}); expected {:?} at (1, 0)",
            editor.source(),
            editor.caret_row,
            editor.caret_col,
            "ab\n"
        ));
    }
    if editor.history.redo_steps() != 1 {
        return Err(format!(
            "undo-redo: Ctrl+Z left {} redo steps, expected 1",
            editor.history.redo_steps()
        ));
    }

    let redo = command_for_shortcut(false, true, false, true)
        .ok_or("undo-redo: Ctrl+Y did not map to a history command")?;
    if redo != HistoryCommand::Redo {
        return Err(format!("undo-redo: Ctrl+Y mapped to {}", redo.label()));
    }
    if !editor.command(redo) {
        return Err("undo-redo: Ctrl+Y found nothing to redo".to_string());
    }
    steps.push(snap("Ctrl+Y", &editor));
    if editor.source() != "ab\ncd" || (editor.caret_row, editor.caret_col) != (1, 2) {
        return Err(format!(
            "undo-redo: Ctrl+Y left {:?} at caret ({}, {}); expected {:?} at (1, 2)",
            editor.source(),
            editor.caret_row,
            editor.caret_col,
            "ab\ncd"
        ));
    }

    // The second undo spelling: Ctrl+Shift+Z must redo what Ctrl+Z undid.
    if !editor.command(undo) {
        return Err("undo-redo: the second Ctrl+Z found nothing to undo".to_string());
    }
    steps.push(snap("Ctrl+Z (again)", &editor));
    let redo_shift = command_for_shortcut(true, false, true, true)
        .ok_or("undo-redo: Ctrl+Shift+Z did not map to a history command")?;
    if redo_shift != HistoryCommand::Redo {
        return Err(format!(
            "undo-redo: Ctrl+Shift+Z mapped to {}",
            redo_shift.label()
        ));
    }
    if !editor.command(redo_shift) {
        return Err("undo-redo: Ctrl+Shift+Z found nothing to redo".to_string());
    }
    steps.push(snap("Ctrl+Shift+Z", &editor));
    if editor.source() != "ab\ncd" || (editor.caret_row, editor.caret_col) != (1, 2) {
        return Err(format!(
            "undo-redo: Ctrl+Shift+Z left {:?} at caret ({}, {})",
            editor.source(),
            editor.caret_row,
            editor.caret_col
        ));
    }

    // A per-document history never crosses editors: undoing in a second editor leaves this
    // one's buffer and its still-available steps untouched.
    let editor_before = editor.source();
    let mut other = TextEditor::from_source("");
    other.insert_str("other");
    if !other.undo() {
        return Err("undo-redo: the second editor had nothing to undo".to_string());
    }
    if other.source() != "" || editor.source() != editor_before || editor.history.undo_steps() != 3
    {
        return Err(format!(
            "undo-redo: undo in the second editor changed the first (second {:?}, first {:?}, {} steps left)",
            other.source(),
            editor.source(),
            editor.history.undo_steps()
        ));
    }

    for step in &steps {
        println!("GUI_SELFTEST undo-redo {step}");
    }
    Ok(format!(
        "two typed bursts around Enter, 3 steps; Ctrl+Z -> {:?} caret (1,0); Ctrl+Y -> {:?} caret (1,2); Ctrl+Z/Ctrl+Shift+Z round trip; per-editor histories independent",
        "ab\n", "ab\ncd"
    ))
}

fn check_long_run() -> Result<String, String> {
    let mut workbench = new_workbench();
    workbench.set_source(0, long_run_source());

    let submit_started = Instant::now();
    if !workbench.run_active() {
        return Err("long-run: run_active returned false".to_string());
    }
    let submit = submit_started.elapsed();
    if submit > EVENT_LOOP_BUDGET {
        return Err(format!(
            "long-run: submitting took {:.3} ms, the window would have frozen",
            submit.as_secs_f64() * 1000.0
        ));
    }
    if !workbench.active_is_running() {
        return Err("long-run: the run was already terminal after submit".to_string());
    }

    // The window calls poll() once per frame; poll_report() is that same loop with the host
    // call timed above. Sample it for 300 ms while the heavy statement is in flight and assert
    // no single next-event call blocked for the frame budget.
    let sample_started = Instant::now();
    let mut calls = 0usize;
    let mut applied = 0usize;
    let mut max_event = Duration::ZERO;
    let mut frames = 0usize;
    while sample_started.elapsed() < EVENT_LOOP_SAMPLE {
        let report = workbench.poll_report();
        calls += report.calls;
        applied += report.applied;
        max_event = max_event.max(report.next_event_max);
        frames += 1;
        if !workbench.active_is_running() {
            return Err(format!(
                "long-run: the run finished {} ms into the 300 ms responsiveness sample",
                sample_started.elapsed().as_millis()
            ));
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    if max_event > EVENT_LOOP_BUDGET {
        return Err(format!(
            "long-run: one next-event call blocked for {:.3} ms (budget {} ms)",
            max_event.as_secs_f64() * 1000.0,
            EVENT_LOOP_BUDGET.as_millis()
        ));
    }
    if applied == 0 {
        return Err("long-run: no typed event arrived during the sample".to_string());
    }
    let status_while_running = workbench.status_text(1, 1);
    if !status_while_running.contains("run Running") {
        return Err(format!(
            "long-run: status strip while running was {status_while_running:?}"
        ));
    }

    let cancel_started = Instant::now();
    if !workbench.cancel_active() {
        return Err("long-run: cancel_active returned false".to_string());
    }
    if !workbench.wait_until_idle(CANCEL_TIMEOUT) {
        return Err(format!(
            "long-run: cancellation did not reach a terminal state within {} ms",
            CANCEL_TIMEOUT.as_millis()
        ));
    }
    let cancel = cancel_started.elapsed();
    if cancel >= CANCEL_TIMEOUT {
        return Err(format!(
            "long-run: cancellation took {:.3} ms",
            cancel.as_secs_f64() * 1000.0
        ));
    }
    let document = &workbench.documents()[0];
    if document.state() != RunState::Cancelled {
        return Err(format!(
            "long-run: terminal state was {:?}, expected Cancelled",
            document.state()
        ));
    }
    if !workbench.status_text(1, 1).contains("run Cancelled") {
        return Err(format!(
            "long-run: status strip was {:?}",
            workbench.status_text(1, 1)
        ));
    }
    expect_log(&workbench, 0, "long-run", |text| {
        text.contains("Run cancelled after")
    })?;

    Ok(format!(
        "submit returned in {} us; 300 ms sample: {} frames, {} next-event calls, {} applied, max {:.3} us; cancel terminal in {:.2} ms after {} of {} statements",
        submit.as_micros(),
        frames,
        calls,
        applied,
        max_event.as_secs_f64() * 1_000_000.0,
        cancel.as_secs_f64() * 1000.0,
        document.committed,
        document.total
    ))
}

/// (a) A fast successful run: Idle -> Queued -> Running -> Finished, one terminal event.
fn check_run_state_fast() -> Result<String, String> {
    let mut workbench = new_workbench();
    workbench.set_source(0, "a = 2\nb = a + 10\nb\n".to_string());
    let from = workbench.documents()[0].transition_count();
    if !workbench.run_active() {
        return Err("run-state-fast: run_active returned false".to_string());
    }
    if workbench.documents()[0].state() != RunState::Queued {
        return Err(format!(
            "run-state-fast: submit accepted but state was {:?}, expected Queued",
            workbench.documents()[0].state()
        ));
    }
    let queued_status = workbench.status_text(1, 1);
    if !queued_status.contains("run Queued") {
        return Err(format!(
            "run-state-fast: status strip after submit was {queued_status:?}"
        ));
    }
    if !workbench.wait_until_idle(RUN_TIMEOUT) {
        return Err("run-state-fast: the run did not reach a terminal state".to_string());
    }
    let document = &workbench.documents()[0];
    if document.state() != RunState::Finished {
        return Err(format!(
            "run-state-fast: terminal state was {:?}, expected Finished",
            document.state()
        ));
    }
    let sequence = expect_state_sequence(
        document,
        from,
        &[
            RunState::Idle,
            RunState::Queued,
            RunState::Running,
            RunState::Finished,
        ],
        "run-state-fast",
    )?;
    if terminal_run_events(document, from) != 1 {
        return Err(format!(
            "run-state-fast: observed {} terminal run events, expected exactly 1",
            terminal_run_events(document, from)
        ));
    }
    expect_log(&workbench, 0, "run-state-fast", |text| {
        text.contains("Run finished")
    })?;
    Ok(format!(
        "sequence {}; Queued {}; Running {}; terminal run events 1",
        state_labels(&sequence),
        duration_text(document, RunState::Queued, RunState::Running),
        duration_text(document, RunState::Running, RunState::Finished)
    ))
}

/// (b) A cancelled long run: Idle -> Queued -> Running -> Cancelling -> Cancelled.
fn check_run_state_cancel() -> Result<String, String> {
    let mut workbench = new_workbench();
    workbench.set_source(0, long_run_source());
    let from = workbench.documents()[0].transition_count();
    if !workbench.run_active() {
        return Err("run-state-cancel: run_active returned false".to_string());
    }
    if workbench.documents()[0].state() != RunState::Queued {
        return Err(format!(
            "run-state-cancel: submit accepted but state was {:?}, expected Queued",
            workbench.documents()[0].state()
        ));
    }

    // Wait until the worker's RunStarted has been applied, then let the single long kernel
    // start so the cancellation lands inside it rather than at a statement boundary.
    let deadline = Instant::now() + RUN_TIMEOUT;
    while workbench.documents()[0].state() == RunState::Queued {
        workbench.poll();
        if Instant::now() >= deadline {
            return Err("run-state-cancel: the worker never reported RunStarted".to_string());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    if workbench.documents()[0].state() != RunState::Running {
        return Err(format!(
            "run-state-cancel: state before cancel was {:?}, expected Running",
            workbench.documents()[0].state()
        ));
    }
    std::thread::sleep(Duration::from_millis(100));
    if workbench.documents()[0].state() != RunState::Running {
        return Err(
            "run-state-cancel: the long run finished before it could be cancelled".to_string(),
        );
    }

    let cancel_started = Instant::now();
    if !workbench.cancel_active() {
        return Err("run-state-cancel: cancel_active returned false".to_string());
    }
    if workbench.documents()[0].state() != RunState::Cancelling {
        return Err(format!(
            "run-state-cancel: cancel request left state {:?}, expected Cancelling",
            workbench.documents()[0].state()
        ));
    }
    let cancelling_status = workbench.status_text(1, 1);
    if !cancelling_status.contains("run Cancelling") {
        return Err(format!(
            "run-state-cancel: status strip after cancel was {cancelling_status:?}"
        ));
    }
    if !workbench.wait_until_idle(CANCEL_TIMEOUT) {
        return Err(
            "run-state-cancel: the cancelled run did not reach a terminal state".to_string(),
        );
    }
    let cancel_wall = cancel_started.elapsed();
    let document = &workbench.documents()[0];
    if document.state() != RunState::Cancelled {
        return Err(format!(
            "run-state-cancel: terminal state was {:?}, expected Cancelled",
            document.state()
        ));
    }
    let sequence = expect_state_sequence(
        document,
        from,
        &[
            RunState::Idle,
            RunState::Queued,
            RunState::Running,
            RunState::Cancelling,
            RunState::Cancelled,
        ],
        "run-state-cancel",
    )?;
    if terminal_run_events(document, from) != 1 {
        return Err(format!(
            "run-state-cancel: observed {} terminal run events, expected exactly 1",
            terminal_run_events(document, from)
        ));
    }
    if document
        .duration_between(RunState::Cancelling, RunState::Cancelled)
        .is_none()
    {
        return Err("run-state-cancel: the Cancelling transition was not logged".to_string());
    }
    expect_log(&workbench, 0, "run-state-cancel", |text| {
        text.contains("Run cancelled after")
    })?;
    Ok(format!(
        "sequence {}; Queued {}; Running {}; Cancelling {} (wall {:.3} ms); terminal run events 1",
        state_labels(&sequence),
        duration_text(document, RunState::Queued, RunState::Running),
        duration_text(document, RunState::Running, RunState::Cancelling),
        duration_text(document, RunState::Cancelling, RunState::Cancelled),
        cancel_wall.as_secs_f64() * 1000.0
    ))
}

/// (c) A failed run: Idle -> Queued -> Running -> Failed, one terminal event.
fn check_run_state_fail() -> Result<String, String> {
    let mut workbench = new_workbench();
    workbench.set_source(0, "a = 2\n1 +\n".to_string());
    let from = workbench.documents()[0].transition_count();
    if !workbench.run_active() {
        return Err("run-state-fail: run_active returned false".to_string());
    }
    if workbench.documents()[0].state() != RunState::Queued {
        return Err(format!(
            "run-state-fail: submit accepted but state was {:?}, expected Queued",
            workbench.documents()[0].state()
        ));
    }
    if !workbench.wait_until_idle(RUN_TIMEOUT) {
        return Err("run-state-fail: the run did not reach a terminal state".to_string());
    }
    let document = &workbench.documents()[0];
    if document.state() != RunState::Failed {
        return Err(format!(
            "run-state-fail: terminal state was {:?}, expected Failed",
            document.state()
        ));
    }
    let sequence = expect_state_sequence(
        document,
        from,
        &[
            RunState::Idle,
            RunState::Queued,
            RunState::Running,
            RunState::Failed,
        ],
        "run-state-fail",
    )?;
    if terminal_run_events(document, from) != 1 {
        return Err(format!(
            "run-state-fail: observed {} terminal run events, expected exactly 1",
            terminal_run_events(document, from)
        ));
    }
    if !workbench.status_text(1, 1).contains("run Failed") {
        return Err(format!(
            "run-state-fail: status strip was {:?}",
            workbench.status_text(1, 1)
        ));
    }
    expect_log(&workbench, 0, "run-state-fail", |text| {
        text.contains("Diagnostic:") && text.contains("Run failed")
    })?;
    Ok(format!(
        "sequence {}; Queued {}; Running {}; status \"run Failed\"; terminal run events 1",
        state_labels(&sequence),
        duration_text(document, RunState::Queued, RunState::Running),
        duration_text(document, RunState::Running, RunState::Failed)
    ))
}

/// (d) An edit after a successful run: Idle -> Queued -> Running -> Finished -> Stale, and a
/// second edit keeps Stale without adding another transition or resurrecting Finished.
fn check_run_state_stale() -> Result<String, String> {
    let mut workbench = new_workbench();
    workbench.set_source(0, "a = 1\na\n".to_string());
    let from = workbench.documents()[0].transition_count();
    if !workbench.run_active() {
        return Err("run-state-stale: run_active returned false".to_string());
    }
    if !workbench.wait_until_idle(RUN_TIMEOUT) {
        return Err("run-state-stale: the run did not reach a terminal state".to_string());
    }
    if workbench.documents()[0].state() != RunState::Finished {
        return Err(format!(
            "run-state-stale: state after the run was {:?}, expected Finished",
            workbench.documents()[0].state()
        ));
    }

    workbench.set_source(0, "a = 2\na\n".to_string());
    if workbench.documents()[0].state() != RunState::Stale {
        return Err(format!(
            "run-state-stale: an edit after Finished left state {:?}, expected Stale",
            workbench.documents()[0].state()
        ));
    }
    let stale_status = workbench.status_text(1, 1);
    if !stale_status.contains("run Stale") {
        return Err(format!(
            "run-state-stale: status strip after the edit was {stale_status:?}"
        ));
    }
    let after_first_edit = workbench.documents()[0].transition_count();
    workbench.set_source(0, "a = 3\na\n".to_string());
    if workbench.documents()[0].state() != RunState::Stale {
        return Err(format!(
            "run-state-stale: a second edit left state {:?}, expected Stale",
            workbench.documents()[0].state()
        ));
    }
    if workbench.documents()[0].transition_count() != after_first_edit {
        return Err("run-state-stale: a second edit added another transition".to_string());
    }

    let document = &workbench.documents()[0];
    let sequence = expect_state_sequence(
        document,
        from,
        &[
            RunState::Idle,
            RunState::Queued,
            RunState::Running,
            RunState::Finished,
            RunState::Stale,
        ],
        "run-state-stale",
    )?;
    if terminal_run_events(document, from) != 1 {
        return Err(format!(
            "run-state-stale: observed {} terminal run events, expected exactly 1",
            terminal_run_events(document, from)
        ));
    }
    let stale_index = document
        .transitions()
        .iter()
        .position(|transition| transition.to == RunState::Stale)
        .ok_or("run-state-stale: the Stale transition is missing")?;
    if document.transitions()[stale_index + 1..]
        .iter()
        .any(|transition| transition.to == RunState::Finished)
    {
        return Err("run-state-stale: Finished reappeared after Stale".to_string());
    }
    Ok(format!(
        "sequence {}; Queued {}; Running {}; Finished displayed {} before the edit; second edit kept Stale and added no transition; terminal run events 1",
        state_labels(&sequence),
        duration_text(document, RunState::Queued, RunState::Running),
        duration_text(document, RunState::Running, RunState::Finished),
        duration_text(document, RunState::Finished, RunState::Stale)
    ))
}
