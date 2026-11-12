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
use crate::completion::{self, CompletionItem};
use crate::engine::{self, DEFAULT_PRECISION, EngineState, PlotCapture};
use crate::execution::{
    DesktopEvent, Diagnostic, DocumentId, EvaluatorFactory, EventSink, ExecutionHost, RunHandle,
    RunId, SequencedEvent, SourceRevision, SubmitError, WorkspaceSnapshot,
};
use crate::keyboard::WorkbenchCommand;
use crate::language;
use crate::run_state::{RunState, RunStateMachine, RunStateTransition};
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

/// Which toolbar controls the window must show for the active document's state.
///
/// The window builds its toolbar from this value, and `--gui-selftest` asserts on the same
/// value while a long run is in flight. That keeps the visible state a tested product decision
/// instead of a condition that only exists inside the egui closure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolbarState {
    /// Run is clickable (no run is in flight).
    pub run_enabled: bool,
    /// Cancel is on screen; true while a run is in flight.
    pub cancel_visible: bool,
    /// Cancel is clickable. It is false in `Cancelling`, where the request is already in.
    pub cancel_enabled: bool,
}

/// One pass over the typed event queue, as measured by [`Workbench::poll_report`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PollReport {
    /// Number of nonblocking `ExecutionHost::next_event` calls this pass made. Always at least
    /// one: the final call is the one that found the queue empty.
    pub calls: usize,
    /// Number of events that were applied to a document.
    pub applied: usize,
    /// Longest single `ExecutionHost::next_event` call in this pass.
    pub next_event_max: Duration,
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
    /// The structured diagnostic of the last failed run, with its source revision.
    ///
    /// It is kept only while [`Workbench::diagnostic`] can prove the revision still describes
    /// the buffer; an edit therefore hides the editor decoration instead of letting it point at
    /// text the failure never named.
    diagnostic: Option<Diagnostic>,
    /// The source revision the failure was produced for.
    diagnostic_revision: Option<SourceRevision>,
    /// The explicit run state and its transition log.
    run_state: RunStateMachine,
    /// Highest event delivery sequence this document has applied.
    ///
    /// [`ExecutionHost`] numbers every event it hands out, strictly increasing, and this
    /// workbench is the consumer. An event at or below this watermark has already been applied,
    /// so a repeated delivery is ignored instead of appending a second log line or publishing
    /// the same snapshot twice.
    last_sequence: u64,
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

    /// The current run state.
    pub fn state(&self) -> RunState {
        self.run_state.state()
    }

    /// The run currently in flight, or the run whose terminal result is displayed.
    #[allow(dead_code)] // host-facing identity accessor used by the workbench tests
    pub fn run_id(&self) -> Option<crate::execution::RunId> {
        self.run_state.run()
    }

    /// Every accepted run-state transition, oldest first.
    pub fn transitions(&self) -> &[RunStateTransition] {
        self.run_state.transitions()
    }

    /// Number of run-state transitions retained.
    pub fn transition_count(&self) -> usize {
        self.run_state.transition_count()
    }

    /// Number of accepted terminal run events in the transition log.
    #[allow(dead_code)] // invariant accessor exercised by the workbench tests
    pub fn terminal_run_event_count(&self) -> usize {
        self.run_state.terminal_run_event_count()
    }

    /// Duration of the first adjacent `from -> to` transition pair, when present.
    pub fn duration_between(
        &self,
        from: RunState,
        to: RunState,
    ) -> Option<std::time::Duration> {
        self.run_state.duration_between(from, to)
    }
}

/// The whole headless workbench: two or more documents sharing one [`ExecutionHost`].
pub struct Workbench {
    host: ExecutionHost,
    docs: Vec<WorkbenchDocument>,
    active: usize,
    engine_name: &'static str,
    plot_root: PathBuf,
    /// The active engine's own symbol table, captured once when the workbench was created.
    ///
    /// Completion uses it before the first run (keywords + the engine's registered functions
    /// with their parameter names). Once a run publishes a snapshot, that snapshot's capture
    /// adds the document's variables and user functions on top of it.
    engine_state: EngineState,
}

impl Default for Workbench {
    fn default() -> Workbench {
        Workbench::new()
    }
}

impl Workbench {
    /// Creates the shipped workbench: the real engine and two document tabs.
    pub fn new() -> Workbench {
        let engine = (engine::default_engine_factory())();
        let engine_name = engine.name();
        // The engine constructed to learn the name is also the one that donates its registry
        // to completion. No second engine is built and no builtin list is transcribed.
        let engine_state = engine.state().unwrap_or_default();
        let (plot_root, plot_error) = engine::prepare_plot_dir();
        let factory = plot_scoped_factory(
            Box::new(engine::default_engine_factory()),
            plot_root.clone(),
        );
        let mut workbench =
            Workbench::with_plot_root(factory, engine_name, plot_root.clone(), engine_state);
        workbench.log(
            0,
            LogLevel::Info,
            format!("Plot directory: {}.", plot_root.display()),
        );
        if let Some(message) = plot_error {
            workbench.log(0, LogLevel::Error, message);
        }
        workbench
    }

    /// Creates a workbench over an arbitrary engine factory (used by unit tests).
    ///
    /// The factory is invoked once to learn the engine's name and to capture the engine's own
    /// registry for completion. Constructing a throwaway engine is safe for every factory that
    /// can also serve runs; tests whose factory is stateful use
    /// [`Workbench::with_named_factory`] instead.
    #[allow(dead_code)] // exercised by the unit tests; the shipped constructor is `new`
    pub fn with_factory(factory: EvaluatorFactory) -> Workbench {
        let engine = (factory)();
        let engine_name = engine.name();
        let engine_state = engine.state().unwrap_or_default();
        let (plot_root, plot_error) = engine::prepare_plot_dir();
        let mut workbench = Workbench::with_plot_root(
            plot_scoped_factory(factory, plot_root.clone()),
            engine_name,
            plot_root.clone(),
            engine_state,
        );
        workbench.log(
            0,
            LogLevel::Info,
            format!("Plot directory: {}.", plot_root.display()),
        );
        if let Some(message) = plot_error {
            workbench.log(0, LogLevel::Error, message);
        }
        workbench
    }

    /// Creates a workbench over an arbitrary factory and an explicit engine name.
    ///
    /// This constructor deliberately does not call `factory`: it exists for factories whose
    /// construction blocks or mutates state, so it has no engine instance to capture a registry
    /// from. Completion still receives every symbol the document's last run published.
    #[allow(dead_code)] // used by the unit tests; the shipped constructors pick the engine itself
    pub fn with_named_factory(factory: EvaluatorFactory, engine_name: &'static str) -> Workbench {
        Workbench::with_plot_root(
            factory,
            engine_name,
            engine::default_plot_dir(),
            EngineState::default(),
        )
    }

    /// Creates a workbench over a factory, a name, the plot root its runs write under and the
    /// registry snapshot completion starts from.
    fn with_plot_root(
        factory: EvaluatorFactory,
        engine_name: &'static str,
        plot_root: PathBuf,
        engine_state: EngineState,
    ) -> Workbench {
        let host = ExecutionHost::new(factory);
        let mut workbench = Workbench {
            host,
            docs: vec![
                WorkbenchDocument::new(DocumentId(1), "welcome.lv", STARTER_SCRIPT),
                WorkbenchDocument::new(DocumentId(2), "untitled.lv", ""),
            ],
            active: 0,
            engine_name,
            plot_root,
            engine_state,
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

    /// The application-scoped directory this workbench's runs write plots into.
    pub fn plot_dir(&self) -> &Path {
        &self.plot_root
    }

    /// The toolbar controls the window must show for the active document right now.
    pub fn toolbar_state(&self) -> ToolbarState {
        let state = self.docs[self.active].state();
        let in_flight = state.is_in_flight();
        ToolbarState {
            run_enabled: !in_flight,
            cancel_visible: in_flight,
            cancel_enabled: matches!(state, RunState::Queued | RunState::Running),
        }
    }

    /// Applies one keyboard command (Ctrl+Enter or Esc) to the active document.
    pub fn command(&mut self, command: WorkbenchCommand) -> bool {
        match command {
            WorkbenchCommand::Run => {
                crate::run_trace::mark("command_run_enter", "Ctrl+Enter command");
                self.run_active()
            }
            WorkbenchCommand::Cancel => self.cancel_active(),
        }
    }

    /// The name of the engine every run constructs.
    pub fn engine_name(&self) -> &'static str {
        self.engine_name
    }

    /// The active engine's registry capture, taken when the workbench was created.
    #[allow(dead_code)] // host-facing accessor; the window reads completion_catalogue instead
    pub fn engine_state(&self) -> &EngineState {
        &self.engine_state
    }

    /// Everything completion may offer for the active document.
    ///
    /// The engine's registry (builtins with their declared parameter names) is the base. The
    /// active document's last committed snapshot is overlaid field for field, so its variables
    /// carry the kinds the engine reported and its user `func` definitions arrive already
    /// marked `is_builtin: false`. A symbol the run published replaces the registry entry of
    /// the same name; the snapshot describes the document, the registry describes the engine.
    pub fn completion_state(&self) -> EngineState {
        let mut state = self.engine_state.clone();
        if let Some(snapshot) = self.active_snapshot() {
            for variable in &snapshot.state.variables {
                match state
                    .variables
                    .iter_mut()
                    .find(|existing| existing.name == variable.name)
                {
                    Some(existing) => *existing = variable.clone(),
                    None => state.variables.push(variable.clone()),
                }
            }
            for function in &snapshot.state.functions {
                match state
                    .functions
                    .iter_mut()
                    .find(|existing| existing.name == function.name)
                {
                    Some(existing) => *existing = function.clone(),
                    None => state.functions.push(function.clone()),
                }
            }
        }
        state.variables
            .sort_by(|a, b| a.name.cmp(&b.name));
        state
            .functions
            .sort_by(|a, b| a.name.cmp(&b.name));
        state
    }

    /// The completion catalogue for the active document, built from live sources.
    ///
    /// Keys/colour/insertion all read this one list; the window rebuilds it per frame so an
    /// engine snapshot landing or a document switch is reflected without a cache to invalidate.
    pub fn completion_catalogue(&self) -> Vec<CompletionItem> {
        let state = self.completion_state();
        completion::build_catalogue(language::KEYWORDS, &state.functions, &state.variables)
    }

    /// Replaces the captured registry (tests use it to provide a deterministic engine surface).
    #[cfg(test)]
    pub(crate) fn set_engine_state(&mut self, state: EngineState) {
        self.engine_state = state;
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
    /// The source revision advancing drives two state-machine edges:
    ///
    /// - while a run is `Queued` or `Running`, the edit cancels it and moves to `Cancelling`; its
    ///   terminal event will arrive but the result describes text that no longer exists, so it is
    ///   discarded and the run ends in `Cancelled`;
    /// - while `Finished`, the displayed result no longer describes the buffer, so the state
    ///   becomes `Stale`. A second edit while `Stale` is a no-op: it cannot resurrect `Finished`.
    pub fn set_source(&mut self, index: usize, source: String) {
        let Some(doc) = self.docs.get_mut(index) else {
            return;
        };
        if doc.source == source {
            return;
        }
        doc.source = source;
        doc.revision = doc.revision.saturating_add(1);
        let state = doc.state();
        let run = doc.run_state.run();
        let mut cancelled = false;
        let mut staled = false;
        match state {
            RunState::Queued | RunState::Running => {
                if let Some(handle) = doc.run.as_ref() {
                    let _ = self.host.cancel(handle);
                }
                if let Some(run) = run {
                    cancelled = doc
                        .run_state
                        .cancel(run, "source changed while the run was in flight");
                }
            }
            RunState::Finished => {
                if let Some(run) = run {
                    staled = doc
                        .run_state
                        .stale(run, "source revision advanced after a finished result");
                }
            }
            _ => {}
        }
        if cancelled {
            self.log(
                index,
                LogLevel::Warn,
                "The source changed while the run was in flight; the run was cancelled.",
            );
        } else if staled {
            self.log(
                index,
                LogLevel::Info,
                "The displayed result is stale: the source changed after it finished.",
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
        if doc.state().is_in_flight() {
            if let Some(handle) = doc.run.as_ref() {
                let _ = self.host.cancel(handle);
            }
            doc.run = None;
            doc.started = None;
        }
        let was_active = doc.state() != RunState::Idle;
        doc.title = title.clone();
        doc.path = Some(path.to_path_buf());
        doc.source = text;
        doc.saved_source = doc.source.clone();
        doc.revision = doc.revision.saturating_add(1);
        doc.snapshot = None;
        doc.last_plot = None;
        doc.committed = 0;
        doc.total = 0;
        doc.elapsed_ms = 0;
        if was_active {
            doc.run_state.reset("document loaded from disk");
        }
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
    /// is evaluated on the caller's thread and the window never blocks. When `submit` is accepted
    /// the state machine enters [`RunState::Queued`] synchronously; [`RunState::Running`] follows
    /// when the typed `RunStarted` event is polled.
    pub fn run_active(&mut self) -> bool {
        let index = self.active;
        let Some(doc) = self.docs.get(index) else {
            return false;
        };
        if doc.state().is_in_flight() {
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
                {
                    let doc = &mut self.docs[index];
                    doc.run = Some(handle);
                    doc.committed = 0;
                    doc.total = 0;
                    doc.elapsed_ms = 0;
                    doc.started = Some(Instant::now());
                    let queued = doc.run_state.begin(run_id, "submit accepted");
                    debug_assert!(queued, "a submit must enter Queued");
                    if crate::run_trace::enabled() {
                        crate::run_trace::mark(
                            "run_phase_set",
                            format!("document={} run={} phase=Queued", doc.id.0, run_id.0),
                        );
                    }
                }
                self.log(
                    index,
                    LogLevel::Info,
                    format!("Run queued (revision {}, run {}).", revision.0, run_id.0),
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
    ///
    /// The state moves to [`RunState::Cancelling`] synchronously when the host accepts the
    /// request; the terminal `Cancelled` event is applied by a later [`Workbench::poll`].
    pub fn cancel_active(&mut self) -> bool {
        let index = self.active;
        let cancellation_requested;
        {
            let Some(doc) = self.docs.get_mut(index) else {
                return false;
            };
            let Some(handle) = doc.run.as_ref() else {
                return false;
            };
            if !handle.is_active() {
                return false;
            }
            if !matches!(doc.state(), RunState::Queued | RunState::Running) {
                return false;
            }
            if !self.host.cancel(handle) {
                return false;
            }
            let Some(run) = doc.run_state.run() else {
                return false;
            };
            cancellation_requested = doc.run_state.cancel(run, "cancellation requested");
        }
        if cancellation_requested {
            self.log(index, LogLevel::Info, "Cancellation requested.");
        }
        cancellation_requested
    }

    /// True while any run is in flight.
    pub fn is_running(&self) -> bool {
        self.docs
            .iter()
            .any(|doc| doc.state().is_in_flight())
    }

    /// True while the active document's run is in flight.
    pub fn active_is_running(&self) -> bool {
        self.docs[self.active].state().is_in_flight()
    }

    /// Drains every queued event without blocking and applies it to its document.
    ///
    /// Returns true when at least one event was applied, so the window knows to repaint. The
    /// caller decides when to call this: the window calls it at the top of every frame and
    /// schedules another frame while a run is in flight.
    pub fn poll(&mut self) -> bool {
        self.poll_report().applied > 0
    }

    /// Drains the typed event queue and measures how long each nonblocking host call took.
    ///
    /// This is exactly the loop [`Workbench::poll`] runs, exposed so `--gui-selftest` can prove
    /// the window's next-event path stays responsive while a long computation is in flight. The
    /// final call is the one that observes the queue empty, so `calls` is never zero.
    pub fn poll_report(&mut self) -> PollReport {
        let mut report = PollReport::default();
        loop {
            let started = Instant::now();
            let next = self.host.next_event(Duration::ZERO);
            let elapsed = started.elapsed();
            report.calls += 1;
            report.next_event_max = report.next_event_max.max(elapsed);
            let Some(sequenced) = next else {
                return report;
            };
            if self.apply_event(&sequenced) {
                report.applied += 1;
            }
        }
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
        // Idempotent consumption: the host numbers events in delivery order, so an event at or
        // below the document's watermark has already been applied. Ignoring it here is what
        // keeps a repeated delivery from duplicating an Output log line or a snapshot.
        if sequenced.sequence <= self.docs[index].last_sequence {
            return false;
        }
        self.docs[index].last_sequence = sequenced.sequence;
        let Some(tracked_run) = self.docs[index].run.as_ref().map(|handle| handle.run()) else {
            return false;
        };
        if tracked_run != sequenced.event.run() {
            return false;
        }
        // A run whose source revision is older than the document's current revision describes
        // text the user has already edited away. Discard its result rather than publishing it.
        let stale = sequenced.revision < SourceRevision(self.docs[index].revision);

        match &sequenced.event {
            DesktopEvent::RunStarted { total, .. } => {
                crate::run_trace::mark(
                    "run_started_applied",
                    format!("total={} document={}", total, id.0),
                );
                let mut started = false;
                let mut cancellation_pending = false;
                {
                    let doc = &mut self.docs[index];
                    if doc.state() == RunState::Queued {
                        started =
                            doc.run_state
                                .started(tracked_run, "worker reported RunStarted");
                    } else if doc.state() == RunState::Cancelling {
                        cancellation_pending = true;
                    }
                    doc.total = *total;
                }
                if started {
                    let detail = format!(
                        "Run started: {} statements, engine {}, precision {}.",
                        total, self.engine_name, self.docs[index].precision
                    );
                    self.log(index, LogLevel::Info, detail);
                } else if cancellation_pending {
                    self.log(
                        index,
                        LogLevel::Warn,
                        "Run started while cancellation was pending; waiting for the terminal event.",
                    );
                }
                true
            }
            DesktopEvent::StepStarted { .. } => true,
            DesktopEvent::StepCommitted {
                index: statement, ..
            } => {
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
                    return self.discard_terminal(
                        index,
                        tracked_run,
                        "the source changed while the run was in flight.",
                    );
                }
                if !self.docs[index].state().is_in_flight() {
                    return false;
                }
                let accepted = self.docs[index].sink.apply(sequenced);
                if !accepted {
                    return self.discard_terminal(
                        index,
                        tracked_run,
                        "the result did not belong to the document's current revision.",
                    );
                }
                self.publish_snapshot(index, snapshot.clone());
                {
                    let doc = &mut self.docs[index];
                    // A successful run is the only outcome that clears the failure decoration:
                    // the buffer it describes is still current and now evaluates cleanly.
                    doc.diagnostic = None;
                    doc.diagnostic_revision = None;
                    doc.committed = snapshot.committed_steps;
                    let transitioned = doc.run_state.finish(
                        tracked_run,
                        RunState::Finished,
                        "Finished event applied",
                    );
                    debug_assert!(transitioned, "a terminal event must end the in-flight run");
                }
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
            DesktopEvent::Cancelled {
                committed_steps, ..
            } => {
                if stale {
                    return self.discard_terminal(
                        index,
                        tracked_run,
                        "the source changed while the run was in flight.",
                    );
                }
                if !self.docs[index].state().is_in_flight() {
                    return false;
                }
                let snapshot = self.host.last_snapshot(id);
                if let Some(snapshot) = snapshot {
                    self.publish_snapshot(index, snapshot);
                }
                {
                    let doc = &mut self.docs[index];
                    doc.committed = *committed_steps;
                    let transitioned = doc.run_state.finish(
                        tracked_run,
                        RunState::Cancelled,
                        "Cancelled event applied",
                    );
                    debug_assert!(transitioned, "a terminal event must end the in-flight run");
                }
                let elapsed = self.close_run(index);
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
                    return self.discard_terminal(
                        index,
                        tracked_run,
                        "the source changed while the run was in flight.",
                    );
                }
                if !self.docs[index].state().is_in_flight() {
                    return false;
                }
                let snapshot = self.host.last_snapshot(id);
                if let Some(snapshot) = snapshot {
                    self.publish_snapshot(index, snapshot);
                }
                {
                    let doc = &mut self.docs[index];
                    doc.diagnostic = Some(diagnostic.clone());
                    doc.diagnostic_revision = Some(sequenced.revision);
                    let transitioned =
                        doc.run_state
                            .finish(tracked_run, RunState::Failed, "Failed event applied");
                    debug_assert!(transitioned, "a terminal event must end the in-flight run");
                }
                let elapsed = self.close_run(index);
                self.log(
                    index,
                    LogLevel::Error,
                    format!("Diagnostic: {}", diagnostic.render()),
                );
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
    ///
    /// The run state is intentionally not touched here: the caller has already applied the one
    /// terminal state transition (or the abandoned-run transition).
    fn close_run(&mut self, index: usize) -> u64 {
        let doc = &mut self.docs[index];
        let elapsed = doc
            .started
            .take()
            .map(|started| started.elapsed().as_millis() as u64)
            .unwrap_or(0);
        doc.elapsed_ms = elapsed;
        doc.run = None;
        elapsed
    }

    /// Retires a run whose terminal event cannot be published, without publishing its snapshot.
    ///
    /// The run was in flight and is now over, so the state machine still accepts exactly one
    /// terminal transition. `Cancelled` is the truthful state: the run was abandoned by the
    /// source edit (or its result failed the revision check), not completed for this buffer.
    fn discard_terminal(&mut self, index: usize, run: RunId, reason: &str) -> bool {
        if !self.docs[index].state().is_in_flight() {
            return false;
        }
        let transitioned = self.docs[index].run_state.finish(
            run,
            RunState::Cancelled,
            "terminal event not published; the run was abandoned",
        );
        if !transitioned {
            return false;
        }
        let _ = self.close_run(index);
        self.log(
            index,
            LogLevel::Warn,
            format!("Run result discarded: {reason}"),
        );
        true
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
        format!(
            "{}  |  rev {}  |  precision {}  |  Ln {}, Col {}  |  run {}",
            doc.title,
            doc.revision,
            doc.precision,
            line,
            column,
            doc.state().label()
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

    /// The active document's failure diagnostic, while it still describes the buffer.
    pub fn active_diagnostic(&self) -> Option<&Diagnostic> {
        self.diagnostic(self.active)
    }

    /// A document's failure diagnostic, while its revision is the document's current revision.
    ///
    /// The revision check is what makes the editor decoration honest: a diagnostic names a line
    /// and column in the text that was run, so an edit (including deleting the offending line)
    /// makes it inapplicable. The log line remains, but no marker can point at stale text and no
    /// marker can point past the shortened buffer.
    pub fn diagnostic(&self, index: usize) -> Option<&Diagnostic> {
        let document = self.docs.get(index)?;
        let revision = document.diagnostic_revision?;
        if revision != SourceRevision(document.revision) {
            return None;
        }
        document.diagnostic.as_ref()
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
            diagnostic: None,
            diagnostic_revision: None,
            run_state: RunStateMachine::new(),
            last_sequence: 0,
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
    use crate::completion::CompletionKind;
    use crate::engine::{EngineFunction, Evaluator, FakeEngine};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    fn fake_workbench() -> Workbench {
        Workbench::with_factory(Box::new(|| Box::new(FakeEngine::new())))
    }

    fn builtin(name: &str, parameters: &[&str]) -> EngineFunction {
        EngineFunction {
            name: name.to_string(),
            parameters: parameters.iter().map(|p| p.to_string()).collect(),
            is_builtin: true,
        }
    }

    #[test]
    fn completion_catalogue_merges_the_engine_registry_with_the_last_committed_snapshot() {
        let mut workbench = fake_workbench();
        // The registry the engine published at startup: one builtin the fake itself does not
        // own. This is exactly how the real engine's capture_state reaches completion.
        workbench.set_engine_state(EngineState {
            variables: Vec::new(),
            functions: vec![builtin("sqrt", &["x"])],
        });
        workbench.set_source(0, "a = 1".to_string());
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        assert_eq!(workbench.documents()[0].state(), RunState::Finished);

        let catalogue = workbench.completion_catalogue();
        let find = |label: &str| {
            catalogue
                .iter()
                .find(|item| item.label == label)
                .unwrap_or_else(|| panic!("{label} is missing from the completion catalogue"))
        };

        // From the engine's registry.
        let sqrt = find("sqrt");
        assert_eq!(sqrt.kind, CompletionKind::Builtin);
        assert_eq!(sqrt.detail, "sqrt(x)");
        // From the document's last committed snapshot, with the kind the engine reported.
        let a = find("a");
        assert_eq!(a.kind, CompletionKind::Variable);
        assert_eq!(a.detail, "a (Natural)");
        // From the language surface.
        assert_eq!(find("while").kind, CompletionKind::Keyword);
        // From the fake engine's own snapshot functions.
        assert!(catalogue.iter().any(|item| item.label == "abs"));
        // No duplicates anywhere in the merged catalogue.
        let mut labels: Vec<&str> = catalogue.iter().map(|item| item.label.as_str()).collect();
        let before = labels.len();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), before, "the catalogue must be deduplicated");
    }

    /// Proves the whole completion source chain with the shipped engine: startup registry ->
    /// completion before any run, then a script-defined function -> last committed snapshot.
    #[test]
    #[cfg(feature = "real-engine")]
    fn completion_catalogue_offers_the_real_registry_and_a_script_defined_function() {
        let mut workbench = Workbench::with_factory(Box::new(|| {
            Box::new(crate::engine::RealEngine::new())
        }));
        // Before any run the registry alone supplies the functions (and their signatures).
        let before = workbench.completion_catalogue();
        assert!(
            before
                .iter()
                .any(|item| item.label == "sqrt" && item.detail == "sqrt(x)"),
            "the startup engine registry must already reach completion"
        );

        workbench.set_source(0, "func myfunc(a, b) = a + b".to_string());
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(10)));
        assert_eq!(
            workbench.documents()[0].state(),
            RunState::Finished,
            "the definition must run before it can complete"
        );

        let catalogue = workbench.completion_catalogue();
        let myfunc = catalogue
            .iter()
            .find(|item| item.label == "myfunc")
            .expect("the script-defined function is offered");
        assert_eq!(
            myfunc.kind,
            CompletionKind::Function,
            "a script func is marked distinctly from a builtin"
        );
        assert_eq!(myfunc.detail, "myfunc(a, b)");
        assert_eq!(
            catalogue
                .iter()
                .find(|item| item.label == "sqrt")
                .expect("the registry survives the run")
                .kind,
            CompletionKind::Builtin
        );
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

    /// Prints one line per evaluation, after a delay, so the test can replay a real Output event
    /// to the consumer while the run is still in flight.
    struct SlowPrintEngine {
        pending: Vec<String>,
        delay: Duration,
        calls: usize,
    }

    impl Evaluator for SlowPrintEngine {
        fn eval(&mut self, _src: &str) -> Result<String, String> {
            self.calls += 1;
            std::thread::sleep(self.delay);
            self.pending.push(format!("line{}", self.calls));
            Ok(format!("v{}", self.calls))
        }

        fn take_output(&mut self) -> Vec<String> {
            std::mem::take(&mut self.pending)
        }
    }

    /// Prints `one`, `two` and `three` for the three print statements, then blocks inside one
    /// kernel call that polls the live cancellation seam. The kernel never reaches a statement
    /// boundary, so only the flag `ExecutionHost::cancel` sets can stop it.
    struct CancellableKernelEngine {
        pending: Vec<String>,
        cancel: Option<Arc<AtomicBool>>,
        kernel_entered: Arc<AtomicBool>,
    }

    impl Evaluator for CancellableKernelEngine {
        fn eval(&mut self, src: &str) -> Result<String, String> {
            for line in ["one", "two", "three"] {
                if src.contains(&format!("print(\"{line}\")")) {
                    self.pending.push(line.to_string());
                    return Ok("Void".to_string());
                }
            }
            self.kernel_entered.store(true, Ordering::SeqCst);
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                if self
                    .cancel
                    .as_ref()
                    .map_or(false, |flag| flag.load(Ordering::Acquire))
                {
                    return Err("Cancelled inside the kernel.".to_string());
                }
                if Instant::now() >= deadline {
                    return Err("The kernel observed no cancellation within 10 s.".to_string());
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }

        fn set_cancel_flag(&mut self, flag: Option<Arc<AtomicBool>>) {
            self.cancel = flag;
        }

        fn take_output(&mut self) -> Vec<String> {
            std::mem::take(&mut self.pending)
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
        Workbench::with_factory(Box::new(move || Box::new(SlowEngine { delay })))
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
        assert_eq!(document.state(), RunState::Finished);
        assert_eq!(
            document
                .transitions()
                .iter()
                .map(|transition| transition.to)
                .collect::<Vec<_>>(),
            [
                RunState::Queued,
                RunState::Running,
                RunState::Finished
            ],
            "a finished run must log Queued, Running and exactly one Finished"
        );
        assert_eq!(document.terminal_run_event_count(), 1);
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
        assert!(
            document
                .logs
                .iter()
                .any(|line| line.text.starts_with("Run started"))
        );
        assert!(document.logs.iter().any(|line| line.text == "= 12"));
        assert!(
            document
                .logs
                .iter()
                .any(|line| line.text.starts_with("Run finished"))
        );
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
        assert!(
            workbench.documents()[0]
                .logs
                .iter()
                .any(|line| line.level == LogLevel::Output && line.text == "x")
        );
    }

    /// LOG-001 byte clause, consumer side: the log line and the snapshot's output entry must be
    /// the engine's bytes, not a trimmed or re-joined rendering of them.
    #[test]
    fn print_output_reaches_the_log_and_snapshot_byte_for_byte() {
        let source = "print(\"a\r\nb\")\nprint(\"c\nd\")\nprint(\"e\rf\")\n";
        let mut workbench = fake_workbench();
        workbench.set_source(0, source.to_string());
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));

        let document = &workbench.documents()[0];
        assert_eq!(document.state(), RunState::Finished);
        let logged = document
            .logs
            .iter()
            .filter(|line| line.level == LogLevel::Output)
            .map(|line| line.text.as_bytes().to_vec())
            .collect::<Vec<_>>();
        assert_eq!(
            logged,
            vec![b"a\r\nb".to_vec(), b"c\nd".to_vec(), b"e\rf".to_vec()],
            "the log holds exactly the three printed lines, byte-for-byte"
        );

        let snapshot = document.snapshot.as_ref().expect("a finished snapshot");
        let snapshot_output = snapshot
            .output
            .iter()
            .map(|line| line.as_bytes().to_vec())
            .collect::<Vec<_>>();
        assert_eq!(
            snapshot_output, logged,
            "the snapshot output and the log lines must be the same bytes in the same order"
        );

        let concatenated = logged.iter().flatten().copied().collect::<Vec<u8>>();
        assert_eq!(
            concatenated,
            b"a\r\nbc\nde\rf".to_vec(),
            "no desktop layer may normalise CRLF/CR or insert a separator"
        );
    }

    /// LOG-001 repeated-delivery clause: the consumer must be idempotent. Replaying a real
    /// Output event (and then the terminal event) must not append a second log line, re-draining
    /// a finished run must change nothing, and the snapshot output must stay exactly once.
    #[test]
    fn replayed_events_do_not_duplicate_log_lines_or_snapshot_output() {
        let mut workbench = Workbench::with_named_factory(
            Box::new(|| {
                Box::new(SlowPrintEngine {
                    pending: Vec::new(),
                    delay: Duration::from_millis(120),
                    calls: 0,
                })
            }),
            "slow-print",
        );
        workbench.set_source(0, "a = 1\nb = 2\n".to_string());
        assert!(workbench.run_active());

        let mut replayed_output = false;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            assert!(
                Instant::now() < deadline,
                "the run did not deliver its terminal event within 5 s"
            );
            let event = workbench
                .host
                .next_event(Duration::from_secs(5))
                .expect("the worker must publish its events");
            let is_output = matches!(event.event, DesktopEvent::Output { .. });
            let is_terminal = matches!(
                event.event,
                DesktopEvent::Finished { .. }
                    | DesktopEvent::Cancelled { .. }
                    | DesktopEvent::Failed { .. }
            );
            assert!(
                workbench.apply_event(&event),
                "the first delivery of {:?} must be applied",
                event.event
            );
            if is_output && !replayed_output {
                replayed_output = true;
                // The very same SequencedEvent, delivered a second time to the same consumer.
                assert!(
                    !workbench.apply_event(&event),
                    "a repeated Output delivery must be ignored, not logged twice"
                );
            }
            if is_terminal {
                assert!(
                    !workbench.apply_event(&event),
                    "a repeated terminal delivery must be ignored"
                );
                break;
            }
        }
        assert!(replayed_output, "no Output event was delivered");

        // Re-draining after the run finished must not append anything either.
        for _ in 0..3 {
            assert!(
                !workbench.poll(),
                "an idle queue must yield no applied events"
            );
        }
        assert!(workbench.wait_until_idle(Duration::from_secs(2)));

        let document = &workbench.documents()[0];
        assert_eq!(document.state(), RunState::Finished);
        assert_eq!(document.terminal_run_event_count(), 1);
        let logged = document
            .logs
            .iter()
            .filter(|line| line.level == LogLevel::Output)
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            logged,
            vec!["line1", "line2"],
            "each printed line must appear exactly once, in order"
        );
        let snapshot = document.snapshot.as_ref().expect("a finished snapshot");
        assert_eq!(
            snapshot.output,
            vec!["line1".to_string(), "line2".to_string()],
            "the snapshot output must not duplicate the replayed line"
        );
    }

    /// LOG-001 cancellation clause: the three print statements commit before the run blocks in
    /// the kernel. Cancelling while it is blocked must keep each printed line exactly once, in
    /// order, publish the same output in the snapshot, and end in exactly one `Cancelled`.
    #[test]
    fn cancelling_after_the_first_print_keeps_each_printed_line_exactly_once() {
        let kernel_entered = Arc::new(AtomicBool::new(false));
        let entered = Arc::clone(&kernel_entered);
        let mut workbench = Workbench::with_named_factory(
            Box::new(move || {
                Box::new(CancellableKernelEngine {
                    pending: Vec::new(),
                    cancel: None,
                    kernel_entered: Arc::clone(&entered),
                })
            }),
            "cancellable-kernel",
        );
        workbench.set_source(
            0,
            "print(\"one\")\nprint(\"two\")\nprint(\"three\")\nspin()\n".to_string(),
        );
        assert!(workbench.run_active());

        let deadline = Instant::now() + Duration::from_secs(5);
        while !workbench.documents()[0]
            .logs
            .iter()
            .any(|line| line.level == LogLevel::Output && line.text == "one")
        {
            workbench.poll();
            assert!(
                Instant::now() < deadline,
                "the first print was never observed; logs: {:?}",
                workbench.documents()[0].logs
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        // Cancel only once the run is inside the kernel: the three prints are committed by then,
        // so a re-emitted prefix line would be visible in the final log.
        while !kernel_entered.load(Ordering::SeqCst) {
            workbench.poll();
            assert!(Instant::now() < deadline, "the kernel was never entered");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            workbench.cancel_active(),
            "cancel must be accepted while the kernel runs"
        );
        assert_eq!(workbench.documents()[0].state(), RunState::Cancelling);
        assert!(workbench.wait_until_idle(Duration::from_secs(2)));

        let document = &workbench.documents()[0];
        assert_eq!(document.state(), RunState::Cancelled);
        assert_eq!(document.terminal_run_event_count(), 1);
        let logged = document
            .logs
            .iter()
            .filter(|line| line.level == LogLevel::Output)
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            logged,
            vec!["one", "two", "three"],
            "each printed line must appear exactly once, in order"
        );
        for line in ["one", "two", "three"] {
            assert_eq!(
                logged.iter().filter(|text| **text == line).count(),
                1,
                "{line} was duplicated in the log"
            );
        }
        let snapshot = document
            .snapshot
            .as_ref()
            .expect("a cancelled run publishes its committed snapshot");
        assert_eq!(
            snapshot.output,
            vec!["one".to_string(), "two".to_string(), "three".to_string()],
            "the cancelled snapshot must carry the same once-only output"
        );
        assert_eq!(snapshot.committed_steps, 3);
        assert!(
            document
                .logs
                .iter()
                .any(|line| line.text.starts_with("Run cancelled after")),
            "logs were {:?}",
            document.logs
        );
    }

    /// LOG-001 rerun clause: one print statement run twice yields exactly two log occurrences and
    /// a one-line snapshot after each run - never three or four.
    #[test]
    fn rerunning_the_same_printing_script_logs_exactly_one_line_per_run() {
        let mut workbench = fake_workbench();
        workbench.set_source(0, "print(\"tick\")\n".to_string());
        for run in 1..=2 {
            assert!(workbench.run_active(), "run {run} must be accepted");
            assert!(workbench.wait_until_idle(Duration::from_secs(5)));
            let document = &workbench.documents()[0];
            assert_eq!(document.state(), RunState::Finished);
            let snapshot = document.snapshot.as_ref().expect("a finished snapshot");
            assert_eq!(
                snapshot.output,
                vec!["tick".to_string()],
                "run {run}'s snapshot must contain exactly its own print"
            );
        }
        let occurrences = workbench.documents()[0]
            .logs
            .iter()
            .filter(|line| line.level == LogLevel::Output && line.text == "tick")
            .count();
        assert_eq!(
            occurrences, 2,
            "two runs with one print each must produce exactly two log occurrences, not {occurrences}"
        );
        assert_eq!(
            workbench.documents()[0]
                .snapshot
                .as_ref()
                .expect("the second run's snapshot")
                .output
                .len(),
            1,
            "the second run's snapshot must not accumulate the first run's output"
        );
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
        assert_eq!(workbench.documents()[0].state(), RunState::Cancelling);
        assert!(
            workbench.status_text(1, 1).contains("run Cancelling"),
            "status strip was {:?}",
            workbench.status_text(1, 1)
        );
        assert!(workbench.wait_until_idle(Duration::from_secs(2)));
        let elapsed = cancel_at.elapsed();
        assert!(elapsed < Duration::from_secs(2), "cancel took {elapsed:?}");

        let document = &workbench.documents()[0];
        assert_eq!(document.state(), RunState::Cancelled);
        assert_eq!(
            document
                .transitions()
                .iter()
                .map(|transition| transition.to)
                .collect::<Vec<_>>(),
            [
                RunState::Queued,
                RunState::Running,
                RunState::Cancelling,
                RunState::Cancelled
            ],
            "a cancelled run must log Queued, Running, Cancelling and exactly one Cancelled"
        );
        assert_eq!(document.terminal_run_event_count(), 1);
        assert!(document.committed >= 1);
        assert!(document.committed < 50);
        let snapshot = document.snapshot.as_ref().expect("cancelled snapshot");
        assert_eq!(snapshot.committed_steps, document.committed);
        assert_eq!(
            snapshot.variables,
            vec![("committed".to_string(), "yes".to_string())]
        );
        assert!(
            document
                .logs
                .iter()
                .any(|line| line.text.starts_with("Run cancelled after"))
        );
        assert!(workbench.status_text(1, 1).contains("Cancelled"));
    }

    #[test]
    fn a_failed_run_yields_a_diagnostic_and_returns_to_idle() {
        let mut workbench = fake_workbench();
        workbench.set_source(0, "y = 4\n1 / 0\n".to_string());
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        let document = &workbench.documents()[0];
        assert_eq!(document.state(), RunState::Failed);
        assert_eq!(
            document
                .transitions()
                .iter()
                .map(|transition| transition.to)
                .collect::<Vec<_>>(),
            [RunState::Queued, RunState::Running, RunState::Failed],
            "a failed run must log Queued, Running and exactly one Failed"
        );
        assert_eq!(document.terminal_run_event_count(), 1);
        let snapshot = document.snapshot.as_ref().expect("failed snapshot");
        assert_eq!(snapshot.committed_steps, 1);
        assert!(snapshot.diagnostic.is_some());
        // The editor decoration is driven by the structured diagnostic, not by the log text.
        let diagnostic = workbench
            .diagnostic(0)
            .expect("a failed run must publish its structured diagnostic");
        assert_eq!(
            diagnostic.line, 2,
            "the failing second statement starts on line 2"
        );
        assert_eq!(diagnostic.column, 1);
        assert_eq!(diagnostic.statement_index, 1);
        assert_eq!(diagnostic.byte_offset, 6);
        assert_eq!(
            diagnostic.message, "Cannot divide by zero.",
            "the engine message must be carried verbatim"
        );
        assert!(
            document
                .logs
                .iter()
                .any(|line| line.level == LogLevel::Error && line.text.starts_with("Diagnostic:"))
        );
        assert!(workbench.status_text(1, 1).contains("Failed"));
    }

    #[test]
    fn a_successful_run_clears_the_failure_decoration_and_an_edit_hides_it() {
        // A stateful engine that fails once and then succeeds, so the second run can prove the
        // success-clears rule without changing the source (a source edit hides the decoration for
        // a different reason: the revision no longer matches).
        struct FailOnceEngine {
            calls: Arc<AtomicU64>,
        }
        impl Evaluator for FailOnceEngine {
            fn eval(&mut self, _src: &str) -> Result<String, String> {
                let call = self.calls.fetch_add(1, Ordering::SeqCst);
                if call == 1 {
                    Err("Cannot divide by zero.".to_string())
                } else {
                    Ok("ok".to_string())
                }
            }
        }

        let calls = Arc::new(AtomicU64::new(0));
        let counter = Arc::clone(&calls);
        let mut workbench = Workbench::with_factory(Box::new(move || {
            Box::new(FailOnceEngine {
                calls: Arc::clone(&counter),
            })
        }));
        workbench.set_source(0, "y = 4\n1 / 0\n".to_string());

        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        assert_eq!(workbench.documents()[0].state(), RunState::Failed);
        assert_eq!(
            workbench.diagnostic(0).map(|d| d.line),
            Some(2),
            "the first run's diagnostic is drawn"
        );

        // The same source now runs successfully; a success is the one outcome that clears the
        // decoration while the buffer has not changed.
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        assert_eq!(workbench.documents()[0].state(), RunState::Finished);
        assert_eq!(
            workbench.diagnostic(0),
            None,
            "a successful run clears the failure decoration"
        );
        assert!(
            workbench.documents()[0]
                .logs
                .iter()
                .any(|line| line.level == LogLevel::Error),
            "the failed run's log line remains even though the decoration cleared"
        );

        // Deleting the offending line is a source edit: the diagnostic is gated on the revision
        // it was produced for, so it can never point at the shortened buffer.
        let mut edited = fake_workbench();
        edited.set_source(0, "y = 4\n1 / 0\nthird = 3\n".to_string());
        let failed_revision = edited.documents()[0].revision;
        assert!(edited.run_active());
        assert!(edited.wait_until_idle(Duration::from_secs(5)));
        assert_eq!(edited.documents()[0].state(), RunState::Failed);
        assert!(edited.diagnostic(0).is_some());
        edited.set_source(0, "y = 4\nthird = 3\n".to_string());
        assert!(
            edited.documents()[0].revision > failed_revision,
            "an edit must advance the revision the diagnostic was gated on"
        );
        assert_eq!(
            edited.diagnostic(0),
            None,
            "deleting the offending line hides the marker instead of retargeting it"
        );
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
        assert!(
            workbench.documents()[0]
                .snapshot
                .as_ref()
                .unwrap()
                .variables
                .iter()
                .any(|(name, _)| name == "a")
        );

        workbench.set_active(1);
        workbench.set_source(1, "b = 2\n".to_string());
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        assert!(
            workbench.documents()[1]
                .snapshot
                .as_ref()
                .unwrap()
                .variables
                .iter()
                .any(|(name, _)| name == "b")
        );
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
        let mut workbench =
            Workbench::with_factory(Box::new(|| Box::new(PlotOnceEngine { calls: 0 })));
        workbench.set_source(0, "plot([1, 2])\na = 1\nb = 2\n".to_string());
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        let document = &workbench.documents()[0];
        assert_eq!(document.state(), RunState::Finished);
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
    fn toolbar_state_tracks_the_active_run_and_the_status_strip() {
        let mut workbench = slow_workbench(Duration::from_millis(20));
        let source = (0..50)
            .map(|index| format!("step = {index}"))
            .collect::<Vec<_>>()
            .join("\n");
        workbench.set_source(0, source);

        let idle = workbench.toolbar_state();
        assert!(idle.run_enabled);
        assert!(!idle.cancel_visible);
        assert!(!idle.cancel_enabled);

        assert!(workbench.run_active());
        assert_eq!(
            workbench.documents()[0].state(),
            RunState::Queued,
            "submit must enter Queued synchronously"
        );
        assert!(
            workbench.status_text(1, 1).contains("run Queued"),
            "status strip after submit was {:?}",
            workbench.status_text(1, 1)
        );
        let deadline = Instant::now() + Duration::from_secs(2);
        while workbench.documents()[0].state() == RunState::Queued {
            workbench.poll();
            assert!(
                Instant::now() < deadline,
                "the worker never reported RunStarted"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(workbench.documents()[0].state(), RunState::Running);
        let running = workbench.toolbar_state();
        assert!(!running.run_enabled, "Run must be disabled while running");
        assert!(
            running.cancel_visible,
            "Cancel must be visible while running"
        );
        assert!(
            running.cancel_enabled,
            "Cancel must be enabled while running"
        );
        assert!(
            workbench.status_text(1, 1).contains("run Running"),
            "status strip was {:?}",
            workbench.status_text(1, 1)
        );

        assert!(workbench.cancel_active());
        assert_eq!(workbench.documents()[0].state(), RunState::Cancelling);
        assert!(
            !workbench.toolbar_state().cancel_enabled,
            "Cancel must be disabled once the request is already in"
        );
        assert!(workbench.wait_until_idle(Duration::from_secs(2)));
        assert_eq!(workbench.documents()[0].state(), RunState::Cancelled);
        let cancelled = workbench.toolbar_state();
        assert!(cancelled.run_enabled, "Run returns after a run is terminal");
        assert!(!cancelled.cancel_visible, "Cancel leaves with the run");
        assert!(workbench.status_text(1, 1).contains("run Cancelled"));
    }

    #[test]
    fn keyboard_commands_use_the_same_run_and_cancel_entry_points() {
        let mut workbench = slow_workbench(Duration::from_millis(10));
        workbench.set_source(0, "a = 1\nb = 2\nc = 3\n".to_string());
        assert!(workbench.command(crate::keyboard::WorkbenchCommand::Run));
        assert!(workbench.active_is_running());
        assert!(workbench.command(crate::keyboard::WorkbenchCommand::Cancel));
        assert!(workbench.wait_until_idle(Duration::from_secs(2)));
        assert_eq!(workbench.documents()[0].state(), RunState::Cancelled);
        assert!(
            !workbench.command(crate::keyboard::WorkbenchCommand::Cancel),
            "cancelling an idle document is a no-op"
        );
    }

    #[test]
    fn plot_dir_is_application_scoped_and_not_under_the_working_directory() {
        let workbench = fake_workbench();
        assert_eq!(workbench.plot_dir(), engine::default_plot_dir());
        let cwd = std::env::current_dir().expect("current directory");
        assert!(
            !workbench.plot_dir().starts_with(&cwd),
            "plot directory {} must not be under the working directory {}",
            workbench.plot_dir().display(),
            cwd.display()
        );
        let logged = workbench.documents()[0]
            .logs
            .iter()
            .find(|line| line.text.starts_with("Plot directory:"))
            .map(|line| line.text.clone())
            .expect("the plot directory is logged at startup");
        assert!(
            logged.contains(&workbench.plot_dir().display().to_string()),
            "logged plot line {logged:?} does not name {}",
            workbench.plot_dir().display()
        );
    }

    #[test]
    fn poll_report_measures_a_nonblocking_next_event_path_while_running() {
        let mut workbench = slow_workbench(Duration::from_millis(20));
        let source = (0..20)
            .map(|index| format!("step = {index}"))
            .collect::<Vec<_>>()
            .join("\n");
        workbench.set_source(0, source);
        assert!(workbench.run_active());

        let mut reports = 0;
        let mut max = Duration::ZERO;
        let mut applied = 0;
        while workbench.active_is_running() && reports < 8 {
            let report = workbench.poll_report();
            assert!(report.calls >= 1);
            max = max.max(report.next_event_max);
            applied += report.applied;
            reports += 1;
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(reports >= 1);
        assert!(
            max < Duration::from_millis(50),
            "the next-event path blocked for {max:?}"
        );
        assert!(applied >= 1, "no typed event was seen during the sample");
        let _ = workbench.cancel_active();
        assert!(workbench.wait_until_idle(Duration::from_secs(2)));
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
        assert_eq!(workbench.documents()[0].state(), RunState::Queued);
        workbench.set_source(0, "c = 3\n".to_string());
        assert_eq!(workbench.documents()[0].state(), RunState::Cancelling);
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        assert!(
            workbench.documents()[0].snapshot.is_none(),
            "a run whose source was replaced must not publish its snapshot"
        );
        assert_eq!(
            workbench.documents()[0].state(),
            RunState::Cancelled,
            "an abandoned run must still end in exactly one terminal state"
        );
        assert_eq!(workbench.documents()[0].terminal_run_event_count(), 1);
        let texts = workbench.documents()[0]
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

    #[test]
    fn queued_running_finished_are_all_logged_with_distinct_labels() {
        let mut workbench = fake_workbench();
        workbench.set_source(0, "a = 1\n".to_string());
        assert!(workbench.run_active());
        assert_eq!(
            workbench.documents()[0].state(),
            RunState::Queued,
            "submit must enter Queued synchronously"
        );
        assert!(
            workbench.status_text(1, 1).contains("run Queued"),
            "status strip was {:?}",
            workbench.status_text(1, 1)
        );
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        let document = &workbench.documents()[0];
        assert_eq!(document.state(), RunState::Finished);
        assert_eq!(
            document
                .transitions()
                .iter()
                .map(|transition| transition.to)
                .collect::<Vec<_>>(),
            [
                RunState::Queued,
                RunState::Running,
                RunState::Finished
            ]
        );
        assert!(document.duration_between(RunState::Queued, RunState::Running).is_some());
        assert!(document.duration_between(RunState::Running, RunState::Finished).is_some());
    }

    #[test]
    fn editing_a_finished_result_marks_it_stale_and_a_second_edit_keeps_it_stale() {
        let mut workbench = fake_workbench();
        workbench.set_source(0, "a = 1\n".to_string());
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        assert_eq!(workbench.documents()[0].state(), RunState::Finished);

        let before = workbench.documents()[0].transition_count();
        workbench.set_source(0, "a = 2\n".to_string());
        assert_eq!(
            workbench.documents()[0].state(),
            RunState::Stale,
            "an edit after a finished result must mark the result stale"
        );
        assert_eq!(workbench.documents()[0].transition_count(), before + 1);
        assert!(
            workbench.status_text(1, 1).contains("run Stale"),
            "status strip was {:?}",
            workbench.status_text(1, 1)
        );

        let stale_count = workbench.documents()[0].transition_count();
        workbench.set_source(0, "a = 3\n".to_string());
        assert_eq!(
            workbench.documents()[0].state(),
            RunState::Stale,
            "a second edit must not move away from Stale"
        );
        assert_eq!(
            workbench.documents()[0].transition_count(),
            stale_count,
            "a second edit must not add another transition"
        );
        let document = &workbench.documents()[0];
        assert_eq!(
            document
                .transitions()
                .iter()
                .map(|transition| transition.to)
                .collect::<Vec<_>>(),
            [
                RunState::Queued,
                RunState::Running,
                RunState::Finished,
                RunState::Stale
            ]
        );
        let stale_index = document
            .transitions()
            .iter()
            .position(|transition| transition.to == RunState::Stale)
            .expect("the Stale transition");
        assert!(
            document.transitions()[stale_index + 1..]
                .iter()
                .all(|transition| transition.to != RunState::Finished),
            "Finished must never reappear after Stale"
        );
        assert_eq!(document.terminal_run_event_count(), 1);
    }

    #[test]
    fn a_duplicate_terminal_event_cannot_produce_a_second_terminal_transition() {
        let mut workbench = fake_workbench();
        workbench.set_source(0, "a = 1\n".to_string());
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        let document = &workbench.documents()[0];
        let run = document.run_id().expect("the finished run id is retained");
        let snapshot = document.snapshot.clone().expect("finished snapshot");
        let document_id = document.id;
        let revision = SourceRevision(document.revision);
        let transitions = document.transition_count();
        assert_eq!(document.terminal_run_event_count(), 1);

        let duplicate = SequencedEvent {
            sequence: 999,
            revision,
            event: DesktopEvent::Finished {
                document: document_id,
                run,
                snapshot,
            },
        };
        assert!(
            !workbench.apply_event(&duplicate),
            "a terminal event for a closed run must be refused"
        );
        let document = &workbench.documents()[0];
        assert_eq!(document.transition_count(), transitions);
        assert_eq!(document.terminal_run_event_count(), 1);
        assert_eq!(document.state(), RunState::Finished);
    }
}
