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

use crate::app::LogLevel;
use crate::engine::DEFAULT_PRECISION;
use crate::keyboard::{WorkbenchCommand, command_for_shortcut};
use crate::workbench::{RunOutcome, RunPhase, Workbench};

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
    let checks: [(&str, Check); 10] = [
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
    ];

    let mut failed = 0usize;
    for (name, check) in checks {
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
    for document in workbench.documents() {
        if document.snapshot.is_some() || document.last_plot.is_some() {
            return Err(format!("{} must start with empty panes", document.title));
        }
        if document.phase != RunPhase::Idle {
            return Err(format!(
                "{} starts in phase {:?}",
                document.title, document.phase
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
    if document.outcome != Some(RunOutcome::Finished) {
        return Err(format!("outcome was {:?}", document.outcome));
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
        .any(|line| line.level == LogLevel::Output && line.text == "x")
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
        if workbench.documents()[0].phase == RunPhase::Idle {
            return Err(format!(
                "the 50-statement run finished before a statement could be cancelled (outcome {:?})",
                workbench.documents()[0].outcome
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
    if !workbench.wait_until_idle(CANCEL_TIMEOUT) {
        return Err("the cancelled run did not reach a terminal state within 2 s".to_string());
    }
    let elapsed = cancel_at.elapsed();
    let document = &workbench.documents()[0];
    if document.outcome != Some(RunOutcome::Cancelled) {
        return Err(format!(
            "terminal outcome was {:?}, expected Cancelled",
            document.outcome
        ));
    }
    if document.phase != RunPhase::Idle {
        return Err(format!("cancelled run left phase {:?}", document.phase));
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
    if document.outcome != Some(RunOutcome::Failed) {
        return Err(format!(
            "outcome was {:?}, expected Failed",
            document.outcome
        ));
    }
    if document.phase != RunPhase::Idle {
        return Err(format!("failed run left phase {:?}", document.phase));
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
        "{diagnostic} (outcome {:?}, phase {:?})",
        document.outcome, document.phase
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
    let parsed = crate::svg::parse(&plot.svg);
    if parsed.polylines.len() != 1 {
        return Err(format!(
            "the engine SVG parsed {} polylines, expected 1",
            parsed.polylines.len()
        ));
    }
    if parsed.polylines[0].points.len() != 3 {
        return Err(format!(
            "the engine SVG parsed {} points, expected 3",
            parsed.polylines[0].points.len()
        ));
    }
    Ok(format!(
        "{} SVG bytes, title {:?}, {} polyline with {} points",
        plot.svg.len(),
        plot.title,
        parsed.polylines.len(),
        parsed.polylines[0].points.len()
    ))
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
    if document.outcome != Some(RunOutcome::Cancelled) {
        return Err(format!(
            "keyboard: terminal outcome was {:?}, expected Cancelled",
            document.outcome
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

    Ok(format!(
        "Ctrl+Enter -> Run (status \"run Running\", Run disabled, Cancel visible); Esc -> run Cancelled in {:.2} ms; toolbar restored",
        elapsed.as_secs_f64() * 1000.0
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
    if document.outcome != Some(RunOutcome::Cancelled) {
        return Err(format!(
            "long-run: terminal outcome was {:?}, expected Cancelled",
            document.outcome
        ));
    }
    if document.phase != RunPhase::Idle {
        return Err(format!(
            "long-run: cancelled run left phase {:?}",
            document.phase
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
