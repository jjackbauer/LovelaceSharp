//! The headless workbench state behind the native window and `--gui-selftest`.
//!
//! This module is the single place where the product's document state meets the tested
//! [`ExecutionHost`]. The egui window owns one [`Workbench`] and calls three methods every
//! frame: [`Workbench::poll`] to apply the typed event stream, [`Workbench::run_active`] for the
//! Run button and [`Workbench::cancel_active`] for the Cancel button. `--gui-selftest` owns the
//! same [`Workbench`] and drives exactly those methods, which is what makes the headless mode a
//! real test of the window's run path rather than a second implementation.
//!
//! Per document, the workbench keeps:
//!
//! - its own [`DocumentId`] and therefore its own `ExecutionHost` run identity;
//! - its source text and source revision (the revision sent with a run);
//! - its precision (carried by `submit`, applied to the run engine by the worker);
//! - its typed [`EventSink`], last [`WorkspaceSnapshot`], last engine plot and log;
//! - its run phase and terminal outcome.
//!
//! No pane data is invented here. Variables, functions, prints, diagnostics and the SVG all come
//! from [`WorkspaceSnapshot`] events published by the execution core. A pane with no snapshot or
//! no plot stays empty and the UI renders its empty state.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::app::LogLevel;
use crate::engine::{self, PlotCapture, DEFAULT_PRECISION};
use crate::execution::{
    DesktopEvent, DocumentId, EvaluatorFactory, EventSink, ExecutionHost, RunHandle,
    SequencedEvent, SourceRevision, SubmitError, WorkspaceSnapshot,
};
use crate::settings::{MAX_PRECISION, MIN_PRECISION};

/// Maximum number of log entries retained per document; the oldest are evicted first.
pub const LOG_CAP: usize = 500;

/// The source of the first document, so the window opens on a real runnable script instead of a
/// blank buffer. It is ordinary editable text, not precomputed pane data.
pub const STARTER_SCRIPT: &str = "\
# Lovelace Desktop. Press Run to evaluate this document.
plot([1, 4, 2], [4, 5, 6], \"welcome\")
a = 2
b = a + 10
b
";

/// One line in a document's log pane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogLine {
    /// Severity, reusing the headless application's vocabulary.
    pub level: LogLevel,
    /// Message text without a trailing newline.
    pub text: String,
}

/// Whether a run is in flight for the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunPhase {
    /// No run is in flight.
    Idle,
    /// A run was submitted and has not delivered its terminal event.
    Running,
    /// Cancellation was accepted and the terminal event has not arrived yet.
    Cancelling,
}

impl RunPhase {
    /// The word shown in the status strip for this phase.
    pub fn label(self) -> &'static str {
        match self {
            RunPhase::Idle => "Idle",
            RunPhase::Running => "Running",
            RunPhase::Cancelling => "Cancelling",
        }
    }
}

/// The terminal result of the document's last run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunOutcome {
    /// The run evaluated every statement.
    Finished,
    /// The run stopped on its cancellation token.
    Cancelled,
    /// A statement failed (or the engine panicked).
    Failed,
}

impl RunOutcome {
    /// The word shown in the status strip after the run.
    pub fn label(self) -> &'static str {
        match self {
            RunOutcome::Finished => "Finished",
            RunOutcome::Cancelled => "Cancelled",
            RunOutcome::Failed => "Failed",
        }
    }
}

/// One document tab: its source, its run identity and the state published by its last run.
pub struct WorkbenchDocument {
    /// The document's stable identity, and the id every event carries.
    pub id: DocumentId,
    /// Tab title (a file name or an "untitled" name).
    pub title: String,
    /// File path, when the document came from disk.
    pub path: Option<PathBuf>,
    /// Source text currently in the editor.
    pub source: String,
    /// Precision passed to the next run's engine.
    pub precision: usize,
    /// Source revision; incremented by every edit and sent with `submit`.
    pub revision: u64,
    /// Applies the typed event stream for this document.
    pub sink: EventSink,
    /// The last coherent snapshot published by a terminal event.
    pub snapshot: Option<WorkspaceSnapshot>,
    /// The most recent plot produced by the engine; kept until a newer plot replaces it.
    pub last_plot: Option<PlotCapture>,
    /// The document's log, oldest first.
    pub logs: Vec<LogLine>,
    /// Whether a run is in flight.
    pub phase: RunPhase,
    /// The terminal result of the last run, if one has completed.
    pub outcome: Option<RunOutcome>,
    /// Number of statements committed by the current or last run.
    pub committed: usize,
    /// Number of statements the current or last run will visit.
    pub total: usize,
    /// Wall-clock time from submit to terminal, in milliseconds.
    pub elapsed_ms: u64,
    run: Option<RunHandle>,
    started: Option<Instant>,
    saved_source: String,
}

impl WorkbenchDocument {
    /// True when the buffer differs from the text last saved or loaded.
    pub fn dirty(&self) -> bool {
        self.source != self.saved_source
    }
}

/// The whole headless workbench: two or more documents sharing one [`ExecutionHost`].
pub struct Workbench {
    host: ExecutionHost,
    docs: Vec<WorkbenchDocument>,
    active: usize,
    engine_name: &'static str,
}

impl Default for Workbench {
    fn default() -> Workbench {
        Workbench::new()
    }
}

impl Workbench {
    /// Creates the shipped workbench: the real engine and two document tabs.
    pub fn new() -> Workbench {
        let engine_name = (engine::default_engine_factory())().name();
        let plot_root = engine::default_plot_dir();
        let factory = plot_scoped_factory(
            Box::new(engine::default_engine_factory()),
            plot_root.clone(),
        );
        let mut workbench = Workbench::with_named_factory(factory, engine_name);
        workbench.log(
            0,
            LogLevel::Info,
            format!("Plot directory: {}.", plot_root.display()),
        );
        workbench
    }

    /// Creates a workbench over an arbitrary engine factory (used by unit tests).
    ///
    /// The factory is invoked once to learn the engine's name. Constructing a throwaway engine
    /// is safe for every factory that can also serve runs; tests whose factory is stateful use
    /// [`Workbench::with_named_factory`] instead.
    #[allow(dead_code)] // exercised by the unit tests; the shipped constructor is `new`
    pub fn with_factory(factory: EvaluatorFactory) -> Workbench {
        let engine_name = (factory)().name();
        let plot_root = engine::default_plot_dir();
        let mut workbench = Workbench::with_named_factory(
            plot_scoped_factory(factory, plot_root.clone()),
            engine_name,
        );
        workbench.log(
            0,
            LogLevel::Info,
            format!("Plot directory: {}.", plot_root.display()),
        );
        workbench
    }

    /// Creates a workbench over an arbitrary factory and an explicit engine name.
    pub fn with_named_factory(factory: EvaluatorFactory, engine_name: &'static str) -> Workbench {
        let host = ExecutionHost::new(factory);
        let mut workbench = Workbench {
            host,
            docs: vec![
                WorkbenchDocument::new(DocumentId(1), "welcome.lv", STARTER_SCRIPT),
                WorkbenchDocument::new(DocumentId(2), "untitled.lv", ""),
            ],
            active: 0,
            engine_name,
        };
        workbench.log(
            0,
            LogLevel::Info,
            format!(
                "Lovelace Desktop ready. Engine: {}. Two documents open.",
                engine_name
            ),
        );
        workbench
    }

    /// The name of the engine every run constructs.
    pub fn engine_name(&self) -> &'static str {
        self.engine_name
    }

    /// The active tab index.
    pub fn active(&self) -> usize {
        self.active
    }

    /// All document tabs.
    pub fn documents(&self) -> &[WorkbenchDocument] {
        &self.docs
    }

    /// One document tab.
    pub fn document(&self, index: usize) -> Option<&WorkbenchDocument> {
        self.docs.get(index)
    }

    /// Selects a tab. A missing index is ignored.
    pub fn set_active(&mut self, index: usize) {
        if index < self.docs.len() {
            self.active = index;
        }
    }

    /// The active document, if there is one.
    pub fn active_document(&self) -> &WorkbenchDocument {
        &self.docs[self.active]
    }

    /// Replaces a document's source and bumps its source revision.
    ///
    /// Editing while a run is in flight cancels that run: its results describe text that no
    /// longer exists, and the worker must not publish them as the current document's state.
    pub fn set_source(&mut self, index: usize, source: String) {
        let Some(doc) = self.docs.get_mut(index) else {
            return;
        };
        if doc.source == source {
            return;
        }
        doc.source = source;
        doc.revision = doc.revision.saturating_add(1);
        let cancelled = if doc.phase == RunPhase::Running {
            match doc.run.as_ref() {
                Some(handle) if self.host.cancel(handle) => {
                    doc.phase = RunPhase::Cancelling;
                    true
                }
                _ => false,
            }
        } else {
            false
        };
        if cancelled {
            self.log(
                index,
                LogLevel::Warn,
                "The source changed while the run was in flight; the run was cancelled.",
            );
        }
    }

    /// Loads a file into a document, recording its path and clean state.
    ///
    /// This is used by the window when it is started with a script path. A read failure is
    /// logged and returned so the caller can surface it.
    pub fn load_file(&mut self, index: usize, path: &Path) -> Result<(), String> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => {
                let message = format!("Cannot open {}: {error}", path.display());
                self.log(index, LogLevel::Error, message.clone());
                return Err(message);
            }
        };
        let title = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        let Some(doc) = self.docs.get_mut(index) else {
            return Err("No document tab to open the file in.".to_string());
        };
        doc.title = title.clone();
        doc.path = Some(path.to_path_buf());
        doc.source = text;
        doc.saved_source = doc.source.clone();
        doc.revision = doc.revision.saturating_add(1);
        doc.snapshot = None;
        doc.last_plot = None;
        doc.outcome = None;
        self.log(index, LogLevel::Info, format!("Opened {}.", path.display()));
        Ok(())
    }

    /// Sets the active document's precision.
    ///
    /// The value is clamped to the supported range and carried by the next `submit`. The
    /// execution worker calls `Evaluator::set_precision` with exactly this value before it
    /// evaluates the first statement, so the engine that renders the results is the one whose
    /// precision this control changed.
    pub fn set_precision(&mut self, index: usize, decimals: usize) {
        let decimals = decimals.clamp(MIN_PRECISION, MAX_PRECISION);
        let Some(doc) = self.docs.get_mut(index) else {
            return;
        };
        if doc.precision == decimals {
            return;
        }
        doc.precision = decimals;
        let title = doc.title.clone();
        self.log(
            index,
            LogLevel::Info,
            format!("Precision for {title} set to {decimals} digits."),
        );
    }

    /// Sets the active tab's precision.
    pub fn set_active_precision(&mut self, decimals: usize) {
        self.set_precision(self.active, decimals);
    }

    /// Requests a run of the active document.
    ///
    /// This only calls [`ExecutionHost::submit`], which spawns the worker and returns: no source
    /// is evaluated on the caller's thread and the window never blocks.
    pub fn run_active(&mut self) -> bool {
        let index = self.active;
        let Some(doc) = self.docs.get(index) else {
            return false;
        };
        if doc.phase != RunPhase::Idle {
            self.log(index, LogLevel::Warn, "A run is already in progress.");
            return false;
        }
        if doc.source.trim().is_empty() {
            self.log(
                index,
                LogLevel::Warn,
                "Nothing to run: the document is empty.",
            );
            return false;
        }
        let revision = SourceRevision(doc.revision);
        let source = doc.source.clone();
        let precision = doc.precision;
        match self.host.submit(doc.id, revision, source, precision) {
            Ok(handle) => {
                let run_id = handle.run();
                let doc = &mut self.docs[index];
                doc.run = Some(handle);
                doc.phase = RunPhase::Running;
                doc.outcome = None;
                doc.committed = 0;
                doc.total = 0;
                doc.elapsed_ms = 0;
                doc.started = Some(Instant::now());
                self.log(
                    index,
                    LogLevel::Info,
                    format!("Run requested (revision {}, run {}).", revision.0, run_id.0),
                );
                true
            }
            Err(SubmitError::Busy) => {
                self.log(
                    index,
                    LogLevel::Warn,
                    "The execution host is still finishing the previous run.",
                );
                false
            }
            Err(SubmitError::Spawn(message)) => {
                self.log(
                    index,
                    LogLevel::Error,
                    format!("Could not start the run worker: {message}"),
                );
                false
            }
        }
    }

    /// Requests cancellation of the active document's run.
    pub fn cancel_active(&mut self) -> bool {
        let index = self.active;
        let Some(doc) = self.docs.get_mut(index) else {
            return false;
        };
        let Some(handle) = doc.run.as_ref() else {
            return false;
        };
        if !handle.is_active() {
            return false;
        }
        if !self.host.cancel(handle) {
            return false;
        }
        doc.phase = RunPhase::Cancelling;
        self.log(index, LogLevel::Info, "Cancellation requested.");
        true
    }

    /// True while any run is in flight.
    pub fn is_running(&self) -> bool {
        self.docs.iter().any(|doc| doc.phase != RunPhase::Idle)
    }

    /// True while the active document's run is in flight.
    pub fn active_is_running(&self) -> bool {
        self.docs[self.active].phase != RunPhase::Idle
    }

    /// Drains every queued event without blocking and applies it to its document.
    ///
    /// Returns true when at least one event was applied, so the window knows to repaint. The
    /// caller decides when to call this: the window calls it at the top of every frame and
    /// schedules another frame while a run is in flight.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Some(sequenced) = self.host.next_event(Duration::ZERO) {
            if self.apply_event(&sequenced) {
                changed = true;
            }
        }
        changed
    }

    /// Polls until no run is in flight or `timeout` elapses. Returns true when the workbench
    /// became idle. `--gui-selftest` uses this; the window polls once per frame instead.
    pub fn wait_until_idle(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            self.poll();
            if !self.is_running() {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// Applies one sequenced event. Returns false when the event belonged to no tracked run.
    fn apply_event(&mut self, sequenced: &SequencedEvent) -> bool {
        let id = sequenced.event.document();
        let Some(index) = self.docs.iter().position(|doc| doc.id == id) else {
            return false;
        };
        let tracked_run = self.docs[index].run.as_ref().map(|handle| handle.run());
        if tracked_run != Some(sequenced.event.run()) {
            return false;
        }
        // A run whose source revision is older than the document's current revision describes
        // text the user has already edited away. Discard its result rather than publishing it.
        let stale = sequenced.revision < SourceRevision(self.docs[index].revision);

        match &sequenced.event {
            DesktopEvent::RunStarted { total, .. } => {
                let detail = format!(
                    "Run started: {} statements, engine {}, precision {}.",
                    total, self.engine_name, self.docs[index].precision
                );
                self.docs[index].total = *total;
                self.log(index, LogLevel::Info, detail);
                true
            }
            DesktopEvent::StepStarted { .. } => true,
            DesktopEvent::StepCommitted { index: statement, .. } => {
                self.docs[index].committed = statement + 1;
                true
            }
            DesktopEvent::Output { text, .. } => {
                self.log(index, LogLevel::Output, text.clone());
                true
            }
            DesktopEvent::SubProgress { .. } => true,
            DesktopEvent::Finished { snapshot, .. } => {
                if stale {
                    self.finish_stale(index);
                    return true;
                }
                let accepted = self.docs[index].sink.apply(sequenced);
                if !accepted {
                    self.finish_stale(index);
                    return true;
                }
                self.publish_snapshot(index, snapshot.clone());
                let doc = &mut self.docs[index];
                doc.outcome = Some(RunOutcome::Finished);
                doc.committed = snapshot.committed_steps;
                let elapsed = self.close_run(index);
                let result = snapshot.result.clone();
                self.log(index, LogLevel::Result, format!("= {result}"));
                self.log(
                    index,
                    LogLevel::Info,
                    format!(
                        "Run finished: {} statements committed in {} ms.",
                        snapshot.committed_steps, elapsed
                    ),
                );
                true
            }
            DesktopEvent::Cancelled { committed_steps, .. } => {
                if stale {
                    self.finish_stale(index);
                    return true;
                }
                let snapshot = self.host.last_snapshot(id);
                if let Some(snapshot) = snapshot {
                    self.publish_snapshot(index, snapshot);
                }
                let elapsed = self.close_run(index);
                self.docs[index].outcome = Some(RunOutcome::Cancelled);
                self.docs[index].committed = *committed_steps;
                self.log(
                    index,
                    LogLevel::Info,
                    format!(
                        "Run cancelled after {} statements in {} ms; the committed statements remain in the workspace.",
                        committed_steps, elapsed
                    ),
                );
                true
            }
            DesktopEvent::Failed { diagnostic, .. } => {
                if stale {
                    self.finish_stale(index);
                    return true;
                }
                let snapshot = self.host.last_snapshot(id);
                if let Some(snapshot) = snapshot {
                    self.publish_snapshot(index, snapshot);
                }
                let elapsed = self.close_run(index);
                self.docs[index].outcome = Some(RunOutcome::Failed);
                self.log(index, LogLevel::Error, format!("Diagnostic: {diagnostic}"));
                self.log(
                    index,
                    LogLevel::Error,
                    format!(
                        "Run failed after {} committed statements in {} ms; the committed prefix remains in the workspace.",
                        self.docs[index].committed, elapsed
                    ),
                );
                true
            }
        }
    }

    /// Closes the current run, records its wall time and returns the elapsed milliseconds.
    fn close_run(&mut self, index: usize) -> u64 {
        let doc = &mut self.docs[index];
        let elapsed = doc
            .started
            .take()
            .map(|started| started.elapsed().as_millis() as u64)
            .unwrap_or(0);
        doc.elapsed_ms = elapsed;
        doc.phase = RunPhase::Idle;
        doc.run = None;
        elapsed
    }

    /// Retires a stale run without publishing its snapshot.
    fn finish_stale(&mut self, index: usize) {
        self.close_run(index);
        self.log(
            index,
            LogLevel::Warn,
            "Run result discarded: the source changed while the run was in flight.",
        );
    }

    /// Records a snapshot and adopts its plot, if it produced one.
    fn publish_snapshot(&mut self, index: usize, snapshot: WorkspaceSnapshot) {
        if let Some(plot) = snapshot.plot.clone() {
            self.docs[index].last_plot = Some(plot);
        }
        self.docs[index].snapshot = Some(snapshot);
    }

    /// Appends a log line, enforcing [`LOG_CAP`].
    fn log(&mut self, index: usize, level: LogLevel, text: impl Into<String>) {
        let Some(doc) = self.docs.get_mut(index) else {
            return;
        };
        if doc.logs.len() >= LOG_CAP {
            doc.logs.remove(0);
        }
        doc.logs.push(LogLine {
            level,
            text: text.into(),
        });
    }

    /// The status strip: document, revision, precision, cursor and run state.
    pub fn status_text(&self, line: usize, column: usize) -> String {
        let doc = &self.docs[self.active];
        let run = if doc.phase == RunPhase::Idle {
            doc.outcome.map(RunOutcome::label).unwrap_or("Idle")
        } else {
            doc.phase.label()
        };
        format!(
            "{}  |  rev {}  |  precision {}  |  Ln {}, Col {}  |  run {}",
            doc.title, doc.revision, doc.precision, line, column, run
        )
    }

    /// The active document's last plot, if any.
    pub fn active_plot(&self) -> Option<&PlotCapture> {
        self.docs[self.active].last_plot.as_ref()
    }

    /// The active document's log lines.
    pub fn active_logs(&self) -> &[LogLine] {
        &self.docs[self.active].logs
    }

    /// The active document's last snapshot, if any.
    pub fn active_snapshot(&self) -> Option<&WorkspaceSnapshot> {
        self.docs[self.active].snapshot.as_ref()
    }
}

impl WorkbenchDocument {
    fn new(id: DocumentId, title: &str, source: &str) -> WorkbenchDocument {
        WorkbenchDocument {
            id,
            title: title.to_string(),
            path: None,
            source: source.to_string(),
            precision: DEFAULT_PRECISION,
            revision: 0,
            sink: EventSink::new(),
            snapshot: None,
            last_plot: None,
            logs: Vec::new(),
            phase: RunPhase::Idle,
            outcome: None,
            committed: 0,
            total: 0,
            elapsed_ms: 0,
            run: None,
            started: None,
            saved_source: source.to_string(),
        }
    }
}

/// Wraps an engine factory so every run gets its own writable plot directory.
///
/// One engine is created per run, so a monotonically increasing `run-N` directory also stops
/// concurrent documents from writing the same `plot.svg` at the same time.
fn plot_scoped_factory(inner: EvaluatorFactory, root: PathBuf) -> EvaluatorFactory {
    let sequence = Arc::new(AtomicU64::new(0));
    Box::new(move || {
        let run = sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let directory = root.join(format!("run-{run}"));
        let mut evaluator = inner();
        evaluator.set_plot_dir(&directory.to_string_lossy());
        evaluator
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Evaluator, FakeEngine};
    use std::sync::atomic::{AtomicBool, Ordering};

    fn fake_workbench() -> Workbench {
        Workbench::with_factory(Box::new(|| Box::new(FakeEngine::new())))
    }

    /// An engine that sleeps in every evaluation, so cancellation has a real statement
    /// boundary to observe.
    struct SlowEngine {
        delay: Duration,
    }

    impl Evaluator for SlowEngine {
        fn eval(&mut self, _src: &str) -> Result<String, String> {
            std::thread::sleep(self.delay);
            Ok("ok".to_string())
        }

        fn variables(&self) -> Vec<(String, String)> {
            vec![("committed".to_string(), "yes".to_string())]
        }
    }

    /// An engine that reports a plot only after its first evaluation, then reports none. This
    /// mirrors `lovelace_lang::Engine`, which resets its captured plot at the start of every
    /// `eval`; the execution core evaluates one statement at a time.
    struct PlotOnceEngine {
        calls: usize,
    }

    impl Evaluator for PlotOnceEngine {
        fn eval(&mut self, _src: &str) -> Result<String, String> {
            self.calls += 1;
            Ok("ok".to_string())
        }

        fn last_plot(&self) -> Option<PlotCapture> {
            if self.calls == 1 {
                Some(PlotCapture {
                    path: "plot.svg".to_string(),
                    title: "first-statement plot".to_string(),
                    svg: "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"10\"></svg>"
                        .to_string(),
                })
            } else {
                None
            }
        }
    }

    fn slow_workbench(delay: Duration) -> Workbench {
        Workbench::with_factory(Box::new(move || {
            Box::new(SlowEngine { delay })
        }))
    }

    #[test]
    fn starts_with_two_independent_document_tabs() {
        let workbench = fake_workbench();
        assert_eq!(workbench.documents().len(), 2);
        assert_eq!(workbench.documents()[0].id, DocumentId(1));
        assert_eq!(workbench.documents()[1].id, DocumentId(2));
        assert_eq!(workbench.active(), 0);
        assert_eq!(workbench.documents()[0].precision, DEFAULT_PRECISION);
        assert_ne!(
            workbench.documents()[0].source,
            workbench.documents()[1].source,
            "the starter document must be real source, not a copy of the empty tab"
        );
        assert!(!workbench.documents()[0].dirty());
    }

    #[test]
    fn a_finished_run_publishes_result_steps_variables_and_logs() {
        let mut workbench = fake_workbench();
        workbench.set_source(0, "a = 2\nb = a + 10\nb\n".to_string());
        let revision = workbench.documents()[0].revision;
        assert_eq!(revision, 1, "one edit bumps the revision once");
        assert!(workbench.run_active());
        assert!(workbench.active_is_running(), "submit must only schedule");
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));

        let document = &workbench.documents()[0];
        assert_eq!(document.outcome, Some(RunOutcome::Finished));
        let snapshot = document.snapshot.as_ref().expect("a finished snapshot");
        assert_eq!(snapshot.result, "12");
        assert_eq!(snapshot.committed_steps, 3);
        assert!(
            snapshot
                .variables
                .iter()
                .any(|(name, value)| name == "b" && value == "12"),
            "variables were {:?}",
            snapshot.variables
        );
        assert!(document
            .logs
            .iter()
            .any(|line| line.text.starts_with("Run started")));
        assert!(document
            .logs
            .iter()
            .any(|line| line.text == "= 12"));
        assert!(document
            .logs
            .iter()
            .any(|line| line.text.starts_with("Run finished")));
    }

    #[test]
    fn print_output_reaches_the_log_and_the_snapshot() {
        let mut workbench = fake_workbench();
        workbench.set_source(0, "print(\"x\")\n1+1\n".to_string());
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        let snapshot = workbench.documents()[0]
            .snapshot
            .as_ref()
            .expect("a finished snapshot");
        assert_eq!(snapshot.result, "2");
        assert_eq!(snapshot.output, vec!["x".to_string()]);
        assert!(workbench.documents()[0]
            .logs
            .iter()
            .any(|line| line.level == LogLevel::Output && line.text == "x"));
    }

    #[test]
    fn cancellation_keeps_the_committed_prefix() {
        let mut workbench = slow_workbench(Duration::from_millis(20));
        let source = (0..50)
            .map(|index| format!("step = {index}"))
            .collect::<Vec<_>>()
            .join("\n");
        workbench.set_source(0, source);
        assert!(workbench.run_active());

        let deadline = Instant::now() + Duration::from_secs(5);
        while workbench.documents()[0].committed == 0 {
            workbench.poll();
            assert!(
                Instant::now() < deadline,
                "no statement was committed within 5 s"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        let cancel_at = Instant::now();
        assert!(workbench.cancel_active());
        assert_eq!(workbench.documents()[0].phase, RunPhase::Cancelling);
        assert!(workbench.wait_until_idle(Duration::from_secs(2)));
        let elapsed = cancel_at.elapsed();
        assert!(elapsed < Duration::from_secs(2), "cancel took {elapsed:?}");

        let document = &workbench.documents()[0];
        assert_eq!(document.outcome, Some(RunOutcome::Cancelled));
        assert_eq!(document.phase, RunPhase::Idle);
        assert!(document.committed >= 1);
        assert!(document.committed < 50);
        let snapshot = document.snapshot.as_ref().expect("cancelled snapshot");
        assert_eq!(snapshot.committed_steps, document.committed);
        assert_eq!(
            snapshot.variables,
            vec![("committed".to_string(), "yes".to_string())]
        );
        assert!(document
            .logs
            .iter()
            .any(|line| line.text.starts_with("Run cancelled after")));
        assert!(workbench.status_text(1, 1).contains("Cancelled"));
    }

    #[test]
    fn a_failed_run_yields_a_diagnostic_and_returns_to_idle() {
        let mut workbench = fake_workbench();
        workbench.set_source(0, "y = 4\n1 / 0\n".to_string());
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        let document = &workbench.documents()[0];
        assert_eq!(document.outcome, Some(RunOutcome::Failed));
        assert_eq!(document.phase, RunPhase::Idle);
        let snapshot = document.snapshot.as_ref().expect("failed snapshot");
        assert_eq!(snapshot.committed_steps, 1);
        assert!(snapshot.diagnostic.is_some());
        assert!(document
            .logs
            .iter()
            .any(|line| line.level == LogLevel::Error && line.text.starts_with("Diagnostic:")));
        assert!(workbench.status_text(1, 1).contains("Failed"));
    }

    #[test]
    fn precision_is_document_scoped_and_clamped() {
        let mut workbench = fake_workbench();
        workbench.set_precision(0, 8);
        assert_eq!(workbench.documents()[0].precision, 8);
        assert_eq!(workbench.documents()[1].precision, DEFAULT_PRECISION);
        workbench.set_precision(1, 0);
        assert_eq!(workbench.documents()[1].precision, MIN_PRECISION);
        workbench.set_precision(0, usize::MAX);
        assert_eq!(workbench.documents()[0].precision, MAX_PRECISION);
    }

    #[test]
    fn tab_isolation_running_one_document_does_not_touch_the_other() {
        let mut workbench = fake_workbench();
        workbench.set_source(0, "a = 1\n".to_string());
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        assert!(workbench.documents()[0]
            .snapshot
            .as_ref()
            .unwrap()
            .variables
            .iter()
            .any(|(name, _)| name == "a"));

        workbench.set_active(1);
        workbench.set_source(1, "b = 2\n".to_string());
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        assert!(workbench.documents()[1]
            .snapshot
            .as_ref()
            .unwrap()
            .variables
            .iter()
            .any(|(name, _)| name == "b"));
        assert_eq!(
            workbench.documents()[0]
                .snapshot
                .as_ref()
                .unwrap()
                .variables
                .iter()
                .filter(|(name, _)| name == "b")
                .count(),
            0,
            "the second document's run must not leak into the first"
        );
        assert_eq!(workbench.documents()[0].revision, 1);
        assert_eq!(workbench.documents()[1].revision, 1);
    }

    #[test]
    fn a_plot_from_an_earlier_statement_survives_later_statements() {
        let mut workbench = Workbench::with_factory(Box::new(|| {
            Box::new(PlotOnceEngine { calls: 0 })
        }));
        workbench.set_source(0, "plot([1, 2])\na = 1\nb = 2\n".to_string());
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        let document = &workbench.documents()[0];
        assert_eq!(document.outcome, Some(RunOutcome::Finished));
        let snapshot = document.snapshot.as_ref().expect("finished snapshot");
        assert_eq!(snapshot.committed_steps, 3);
        let plot = snapshot
            .plot
            .as_ref()
            .expect("the plot produced by statement 1 must survive statements 2 and 3");
        assert_eq!(plot.title, "first-statement plot");
        assert_eq!(
            document.last_plot.as_ref().map(|plot| plot.title.as_str()),
            Some("first-statement plot")
        );
    }

    #[test]
    fn stale_events_from_an_abandoned_run_are_discarded() {
        let panicked = std::sync::Arc::new(AtomicBool::new(false));
        let flag = std::sync::Arc::clone(&panicked);
        let mut workbench = Workbench::with_named_factory(
            Box::new(move || {
                let engine: Box<dyn Evaluator> = if !flag.swap(true, Ordering::SeqCst) {
                    Box::new(SlowEngine {
                        delay: Duration::from_millis(50),
                    })
                } else {
                    Box::new(FakeEngine::new())
                };
                engine
            }),
            "slow",
        );
        workbench.set_source(0, "a = 1\nb = 2\n".to_string());
        assert!(workbench.run_active());
        workbench.set_source(0, "c = 3\n".to_string());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        assert!(
            workbench.documents()[0].snapshot.is_none(),
            "a run whose source was replaced must not publish its snapshot"
        );
        let texts = workbench
            .documents()[0]
            .logs
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>();
        assert!(
            texts
                .iter()
                .any(|text| text.contains("Run result discarded")),
            "logs were {texts:?}"
        );
    }
}
