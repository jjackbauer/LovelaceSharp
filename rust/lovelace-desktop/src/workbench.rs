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
use std::time::{Duration, Instant, SystemTime};

use crate::completion::{self, CompletionItem};
use crate::document_io::{self, LoadedFile, NewlineStyle};
use crate::engine::{self, DEFAULT_PRECISION, EngineState, PlotCapture};
use crate::execution::{
    DesktopEvent, Diagnostic, DocumentId, EvaluatorFactory, EventSink, ExecutionHost, RunHandle,
    RunId, SequencedEvent, SourceRevision, SubmitError, WorkspaceSnapshot,
};
use crate::keyboard::WorkbenchCommand;
use crate::language;
use crate::log_kind::{LogKind, LogLine};
use crate::recovery::RecoveryRecord;
use crate::run_state::{RunState, RunStateMachine, RunStateTransition};
use crate::settings::{MAX_PRECISION, MIN_PRECISION, SessionState, SessionTab};

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

/// The tab title a document with no file yet carries.
pub const UNTITLED_TITLE: &str = "untitled.lv";

/// The result of writing a document to disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveOutcome {
    /// The bytes were written: `path` is where, and `bytes` how many.
    Saved {
        /// The path the document now has.
        path: PathBuf,
        /// Number of bytes written.
        bytes: usize,
    },
    /// The document has no path yet, so the window must ask for one and call
    /// [`Workbench::save_as`].
    NeedsPath,
    /// The write failed; the message is logged and returned for the caller to surface.
    Failed(String),
}

/// The file name of `path`, or the whole path when it has none.
fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// The modification time of `path`, when it can be read.
///
/// DOC-005 records this in the recovery journal; a file that cannot be stat'ed (it was deleted
/// between the save and the stat, for example) simply has no recorded time.
fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

/// One document that a launch restore puts back, before it becomes a [`WorkbenchDocument`].
///
/// DOC-004/005: a restored document comes either from a file (read again, nothing written) or from
/// a recovery record (the journaled bytes, nothing executed). The two sources differ only in which
/// fields they fill in, so they share the installation path.
struct RestoredDocument {
    /// The identity the restored tab keeps (the session's, so its journal record stays keyed to it).
    id: DocumentId,
    /// Tab title.
    title: String,
    /// File path, when the tab showed a file.
    path: Option<PathBuf>,
    /// The buffer text to show.
    source: String,
    /// The text that counts as saved, so [`WorkbenchDocument::dirty`] is meaningful immediately.
    saved_source: String,
    /// Newline style, or `None` to detect it from the source.
    newline: Option<NewlineStyle>,
    /// Whether the file began with a UTF-8 byte-order mark.
    bom: bool,
    /// The source file's mtime when it was opened, when known.
    source_mtime: Option<SystemTime>,
    /// The file's own bytes, for a byte-identical clean save. `None` for a recovered buffer.
    original_bytes: Option<Vec<u8>>,
    /// Caret to restore, as 0-based row and column.
    caret: (usize, usize),
    /// Selection to restore.
    selection: Option<((usize, usize), (usize, usize))>,
}

/// What one session restore did (DOC-004).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RestoreReport {
    /// The identities of the documents that were put back, in tab order.
    pub restored: Vec<DocumentId>,
    /// Identities of the recovery records whose text the session restore consumed.
    ///
    /// A record is consumed when the tab it belongs to could only be restored from the journal
    /// (an unsaved buffer, or a file that has since disappeared). The window keeps tracking those
    /// records so they are deleted when the document is saved, closed or the app exits cleanly.
    pub recovered: Vec<u64>,
    /// One message per entry that could not be restored, with the reason.
    pub skipped: Vec<String>,
}

/// What one recovery did (DOC-005).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoverReport {
    /// The document the text was put into.
    pub document_id: DocumentId,
    /// Its index in the tab list.
    pub index: usize,
    /// True when an open tab was rewritten, false when the text arrived in a new tab.
    pub replaced: bool,
    /// Number of buffer bytes recovered.
    pub bytes: usize,
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
    /// DOC-002: the newline style recorded when the file was opened.
    ///
    /// New lines are written with [`NewlineStyle::ending`]; the text itself keeps the file's own
    /// endings because [`WorkbenchDocument::source`] was decoded without normalisation.
    pub newline: NewlineStyle,
    /// DOC-002: true when the file began with a UTF-8 byte-order mark, which save restores.
    pub bom: bool,
    /// DOC-005: the file's modification time when it was opened or last saved.
    ///
    /// The recovery record carries it so a recovered buffer still names the version of the file
    /// it was typed against; conflict detection (DOC-006) reads the same value.
    pub source_mtime: Option<SystemTime>,
    /// DOC-005: the caret to journal with the buffer, as 0-based row and column.
    ///
    /// The window copies its editor's caret here every frame; the workbench itself never derives
    /// one, so a headless restore can put the caret back exactly where the user left it.
    pub caret: (usize, usize),
    /// DOC-005: the selection to journal with the buffer: anchor and caret end, both 0-based.
    pub selection: Option<((usize, usize), (usize, usize))>,
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
    /// The document's log, oldest first. Every line carries its [`LogKind`] and renders the
    /// kind's visible marker (see [`LogLine::render`]).
    ///
    /// RUN-003: statement-scoped lines are appended only when the statement commits, so the log
    /// tail and the committed snapshot advance in the same [`Workbench::apply_event`] branch.
    pub logs: Vec<LogLine>,
    /// Statement-scoped log lines produced by the in-flight statement, not yet committed.
    ///
    /// A `[progress]` line (from `StepStarted`) and its `[stdout]` lines (from `Output`) wait
    /// here until the matching `StepCommitted` arrives. They are then flushed in their original
    /// order and the commit snapshot is published in the same branch: no pane can show a line
    /// from a statement whose effects the workspace does not have yet.
    pending_logs: Vec<LogLine>,
    /// The 0-based statement the in-flight statement-scoped events belong to.
    current_statement: Option<usize>,
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
    /// The in-flight run was submitted by the quick-eval input rather than by Run.
    ///
    /// The run uses the same document identity, revision and precision, but its source is the
    /// one-line expression, not the editor buffer. Its terminal snapshot therefore must not
    /// replace the document's workspace snapshot and a failure must not decorate the editor with
    /// coordinates from text the failure never named.
    quick_eval_run: Option<RunId>,
    started: Option<Instant>,
    saved_source: String,
    /// The bytes the document was last read from or written to.
    ///
    /// DOC-002: a buffer that still matches [`Self::saved_source`] writes these bytes verbatim, so
    /// an unmodified file round-trips even when no single separator describes it (a mixed-ending
    /// file, for example). `None` for a document that has never been written.
    original_bytes: Option<Vec<u8>>,
}

impl WorkbenchDocument {
    /// True when the buffer differs from the text last saved or loaded.
    pub fn dirty(&self) -> bool {
        self.source != self.saved_source
    }

    /// The tab title with the unsaved-changes marker: `"name.lv"` or `"name.lv *"`.
    ///
    /// DOC-002: one function produces the marker for the tab strip, the editor pane header and the
    /// window title, so the three cannot disagree about whether the buffer has unsaved changes.
    pub fn title_with_marker(&self) -> String {
        if self.dirty() {
            format!("{} *", self.title)
        } else {
            self.title.clone()
        }
    }

    /// The encoding badge: `"UTF-8 BOM · CRLF"`, `"UTF-8 · LF"`, and so on.
    pub fn encoding_label(&self) -> String {
        format!(
            "UTF-8{} · {}",
            if self.bom { " BOM" } else { "" },
            self.newline.label()
        )
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
    pub fn duration_between(&self, from: RunState, to: RunState) -> Option<std::time::Duration> {
        self.run_state.duration_between(from, to)
    }
}

/// The assembled state a frame renders, built from one document at one instant.
///
/// RUN-003: this exists so a pane cannot read a variable from the last commit, a revision from
/// the buffer and a log line from the next statement. [`Workbench::view`] fills every field from
/// the same document state and the same committed [`WorkspaceSnapshot`]; [`DocumentView`] then
/// answers the coherence questions the tests and the window pane use:
///
/// - `committed_steps` / `committed_statement()` are the committed snapshot's prefix;
/// - `last_committed_log_statement()` is the last statement the log names that is **inside**
///   that prefix (a terminal failure/cancellation attempt line names the next index and is
///   deliberately excluded);
/// - `last_log_statement()` is the last statement the log names at all.
///
/// The egui window renders `variables`/`functions`/`plot`/`logs`/`diagnostic` and the visible
/// `revision` from this one object.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentView {
    /// The source revision the committed snapshot belongs to.
    pub revision: SourceRevision,
    /// True once a run has published a workspace snapshot.
    pub has_snapshot: bool,
    /// Committed statements in that snapshot (the state's revision).
    pub committed_steps: usize,
    /// Statements the snapshot's run visited.
    pub total: usize,
    /// Display form of the last value-producing committed statement.
    pub result: String,
    /// The engine's structured projection of that same result, when the engine publishes one.
    ///
    /// RES-001: carried alongside [`Self::result`] from the one committed snapshot; the window's
    /// result tree is built only from this value.
    pub result_structured: Option<lovelace_json::Json>,
    /// SYM-001: the engine's symbolic inspection payload for that same committed result.
    ///
    /// Copied from the one committed snapshot, so editing the buffer cannot change it and a new
    /// committed result replaces it together with the result display and structured projection.
    pub result_inspection: Option<lovelace_json::Json>,
    /// Variables after the committed prefix, sorted by the engine.
    pub variables: Vec<(String, String)>,
    /// Functions after the committed prefix (the engine registry, WKS-002).
    pub functions: Vec<(String, Vec<String>)>,
    /// The kind-aware capture the completion catalogue reads.
    pub state: EngineState,
    /// The most recent plot produced by the committed prefix.
    pub plot: Option<PlotCapture>,
    /// Every print line produced by the committed prefix.
    pub output: Vec<String>,
    /// The log as the pane paints it: only lines whose statement committed (plus run-level and
    /// terminal lines).
    pub logs: Vec<LogLine>,
    /// The failure diagnostic that still describes the buffer, with its revision already checked.
    pub diagnostic: Option<Diagnostic>,
    /// The document's visible run state.
    pub run_state: RunState,
    /// Duration of the last run, in milliseconds.
    pub elapsed_ms: u64,
}

impl DocumentView {
    // These are the coherence predicates the RUN-003 tests and any host renderer use; the egui
    // window reads the fields directly. They stay on the assembled object on purpose.
    #[allow(dead_code)]
    /// The 1-based number of the last committed statement, when one committed.
    pub fn committed_statement(&self) -> Option<usize> {
        (self.committed_steps > 0).then_some(self.committed_steps)
    }

    #[allow(dead_code)]
    /// The 1-based number of the last statement any log line names.
    pub fn last_log_statement(&self) -> Option<usize> {
        self.logs
            .iter()
            .filter_map(|line| line.statement)
            .max()
            .map(|statement| statement + 1)
    }

    #[allow(dead_code)]
    /// The 1-based number of the last statement the log names **inside** the committed prefix.
    ///
    /// This is the pane-level form of RUN-003's rule: at rest after commit `n` this is `n`, and
    /// the variables/revision in the same object describe exactly statements `1..=n`.
    pub fn last_committed_log_statement(&self) -> Option<usize> {
        self.logs
            .iter()
            .filter_map(|line| line.statement)
            .filter(|statement| *statement < self.committed_steps)
            .max()
            .map(|statement| statement + 1)
    }

    #[allow(dead_code)]
    /// The display value of a variable in this view, if present.
    pub fn variable(&self, name: &str) -> Option<&str> {
        self.variables
            .iter()
            .find(|(variable, _)| variable == name)
            .map(|(_, value)| value.as_str())
    }
}

/// The whole headless workbench: two or more documents sharing one [`ExecutionHost`].
pub struct Workbench {
    host: ExecutionHost,
    docs: Vec<WorkbenchDocument>,
    active: usize,
    engine_name: &'static str,
    plot_root: PathBuf,
    /// Paths the user opened or saved, newest first, without duplicates.
    ///
    /// DOC-002 requires Save As to update the recent list. DOC-004 owns persisting it across
    /// restarts; this list lives for the life of the workbench and is what the window's Recent
    /// menu shows.
    recent_files: Vec<String>,
    /// The id the next document created by this workbench receives.
    ///
    /// Ids are never reused: an event from a closed tab's run carries the old id, and a new tab
    /// must not answer to it. The counter starts above the two constructor documents.
    next_id: u64,
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
            LogKind::Info,
            format!("Plot directory: {}.", plot_root.display()),
        );
        if let Some(message) = plot_error {
            workbench.log(0, LogKind::Diagnostic, message);
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
            LogKind::Info,
            format!("Plot directory: {}.", plot_root.display()),
        );
        if let Some(message) = plot_error {
            workbench.log(0, LogKind::Diagnostic, message);
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
            recent_files: Vec::new(),
            next_id: 3,
            engine_state,
        };
        workbench.log(
            0,
            LogKind::Info,
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
        state.variables.sort_by(|a, b| a.name.cmp(&b.name));
        state.functions.sort_by(|a, b| a.name.cmp(&b.name));
        state
    }

    /// The completion catalogue for the active document, built from live sources.
    ///
    /// Keys/colour/insertion all read this one list; the window rebuilds it per frame so an
    /// engine snapshot landing or a document switch is reflected without a cache to invalidate.
    /// The window itself now takes one [`Workbench::completion_state`] capture and builds the
    /// catalogue from it, so the syntax highlighter can read the same builtin flags; this
    /// convenience form remains the host/test entry point.
    #[cfg_attr(not(test), allow(dead_code))]
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
                LogKind::Info,
                "The source changed while the run was in flight; the run was cancelled.",
            );
        } else if staled {
            self.log(
                index,
                LogKind::Info,
                "The displayed result is stale: the source changed after it finished.",
            );
        }
    }

    /// Loads a file into an existing document, recording its path and clean state.
    ///
    /// This is the "read into tab N" entry point for hosts and tests; the window itself opens
    /// files with [`Workbench::open`], which decides between reusing an empty tab and creating
    /// one. A read failure is logged and returned so the caller can surface it, and a file that is
    /// not UTF-8 is refused before the tab is touched: the decoder never mangles bytes it cannot
    /// read.
    #[allow(dead_code)] // kept as the "read into tab N" entry point for hosts and tests
    pub fn load_file(&mut self, index: usize, path: &Path) -> Result<(), String> {
        if index >= self.docs.len() {
            return Err("No document tab to open the file in.".to_string());
        }
        let loaded = match document_io::read(path) {
            Ok(loaded) => loaded,
            Err(message) => {
                self.log(index, LogKind::Diagnostic, message.clone());
                return Err(message);
            }
        };
        self.adopt_loaded(index, path, loaded);
        Ok(())
    }

    /// Opens `path` in a tab: the active tab when it is an untouched empty document, else a new
    /// one. Returns the index of the tab the file was opened in.
    ///
    /// A file that does not decode (not UTF-8, or UTF-16) is refused with the message logged and
    /// returned, and no tab is created or modified.
    pub fn open(&mut self, path: &Path) -> Result<usize, String> {
        let loaded = match document_io::read(path) {
            Ok(loaded) => loaded,
            Err(message) => {
                self.log(self.active, LogKind::Diagnostic, message.clone());
                return Err(message);
            }
        };
        let index = match self.pristine_active() {
            Some(index) => index,
            None => {
                let id = self.next_document_id();
                self.docs
                    .push(WorkbenchDocument::new(id, UNTITLED_TITLE, ""));
                self.docs.len() - 1
            }
        };
        self.adopt_loaded(index, path, loaded);
        self.active = index;
        Ok(index)
    }

    /// The active tab, when it is an empty document nothing has been written to.
    ///
    /// Open reuses this tab instead of leaving an empty "untitled" tab beside the file; every other
    /// tab (including the starter document) keeps its content and the file opens in a new tab.
    fn pristine_active(&self) -> Option<usize> {
        self.docs
            .get(self.active)
            .filter(|doc| doc.path.is_none() && !doc.dirty() && doc.source.is_empty())
            .map(|_| self.active)
    }

    /// Makes `index` show a file that has just been decoded.
    ///
    /// The document keeps the path, the decoded text (with the file's own line endings), the
    /// recorded newline style, the BOM flag and the file's own bytes. Its panes are cleared
    /// because the previous run described text that is no longer displayed.
    fn adopt_loaded(&mut self, index: usize, path: &Path, loaded: LoadedFile) {
        let title = file_name_of(path);
        let Some(doc) = self.docs.get_mut(index) else {
            return;
        };
        if doc.state().is_in_flight() {
            if let Some(handle) = doc.run.as_ref() {
                let _ = self.host.cancel(handle);
            }
            doc.run = None;
            doc.quick_eval_run = None;
            doc.started = None;
        }
        let was_active = doc.state() != RunState::Idle;
        doc.title = title;
        doc.path = Some(path.to_path_buf());
        doc.source = loaded.text;
        doc.saved_source = doc.source.clone();
        doc.newline = loaded.newline;
        doc.bom = loaded.bom;
        doc.source_mtime = file_mtime(path);
        doc.caret = (0, 0);
        doc.selection = None;
        doc.original_bytes = Some(loaded.bytes);
        doc.revision = doc.revision.saturating_add(1);
        doc.snapshot = None;
        doc.last_plot = None;
        doc.pending_logs.clear();
        doc.current_statement = None;
        doc.committed = 0;
        doc.total = 0;
        doc.elapsed_ms = 0;
        if was_active {
            doc.run_state.reset("document loaded from disk");
        }
        let bytes = self.docs[index].source.len();
        let detail = self.docs[index].encoding_label();
        self.add_recent(path);
        self.log(
            index,
            LogKind::Info,
            format!(
                "Opened {} ({bytes} bytes, {detail}).",
                path.display()
            ),
        );
    }

    /// Adds a new empty document tab and selects it. Returns its index.
    pub fn new_document(&mut self) -> usize {
        let id = self.next_document_id();
        let title = self.next_untitled_title();
        let precision = self.docs[self.active].precision;
        let mut document = WorkbenchDocument::new(id, &title, "");
        document.precision = precision;
        self.docs.push(document);
        let index = self.docs.len() - 1;
        self.active = index;
        self.log(index, LogKind::Info, format!("New document {title}."));
        index
    }

    /// The id a document created now would receive.
    fn next_document_id(&mut self) -> DocumentId {
        let id = DocumentId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    /// A tab title no open document uses: `untitled.lv`, then `untitled-2.lv`, and so on.
    fn next_untitled_title(&self) -> String {
        for sequence in 1..=self.docs.len() + 2 {
            let candidate = if sequence == 1 {
                UNTITLED_TITLE.to_string()
            } else {
                format!("untitled-{sequence}.lv")
            };
            if !self.docs.iter().any(|doc| doc.title == candidate) {
                return candidate;
            }
        }
        format!("untitled-{}.lv", self.docs.len() + 3)
    }

    /// Writes a document to its own path, or reports that it needs one.
    pub fn save(&mut self, index: usize) -> SaveOutcome {
        let Some(path) = self.docs.get(index).and_then(|doc| doc.path.clone()) else {
            return SaveOutcome::NeedsPath;
        };
        self.write_document(index, path)
    }

    /// Writes a document to `path` and adopts it as the document's file.
    ///
    /// DOC-002: Save As updates the path, the tab title and the recent list, and the tab is clean
    /// afterwards because the bytes on disk are the bytes in the buffer.
    pub fn save_as(&mut self, index: usize, path: PathBuf) -> SaveOutcome {
        self.write_document(index, path)
    }

    /// The shared write: pick the bytes, write them, adopt the path and clear the dirty marker.
    fn write_document(&mut self, index: usize, path: PathBuf) -> SaveOutcome {
        let prepared = match self.docs.get(index) {
            None => Err("No document is open.".to_string()),
            Some(doc) => {
                // DOC-002: an unmodified document writes the bytes it was read from, so a BOM, a
                // mixed-ending file or any other byte survives; only an edited buffer is
                // re-encoded, and even then its text (with the file's line endings) is written
                // verbatim.
                let bytes = match (doc.dirty(), doc.original_bytes.as_ref()) {
                    (false, Some(original)) => original.clone(),
                    _ => document_io::encode(&doc.source, doc.bom),
                };
                Ok((bytes, doc.source.clone(), doc.bom))
            }
        };
        let (bytes, source, bom) = match prepared {
            Ok(prepared) => prepared,
            Err(message) => return SaveOutcome::Failed(message),
        };
        match document_io::write(&path, &bytes) {
            Ok(written) => {
                {
                    let doc = &mut self.docs[index];
                    doc.path = Some(path.clone());
                    doc.title = file_name_of(&path);
                    doc.saved_source = source;
                    doc.original_bytes = Some(bytes);
                    doc.newline = NewlineStyle::detect(&doc.source);
                    doc.bom = bom;
                    doc.source_mtime = file_mtime(&path);
                }
                let detail = self.docs[index].encoding_label();
                self.add_recent(&path);
                self.log(
                    index,
                    LogKind::Info,
                    format!("Saved {} ({written} bytes, {detail}).", path.display()),
                );
                SaveOutcome::Saved {
                    path,
                    bytes: written,
                }
            }
            Err(message) => {
                self.log(index, LogKind::Diagnostic, message.clone());
                SaveOutcome::Failed(message)
            }
        }
    }

    /// Closes the tab with this identity, cancelling any run it owns.
    ///
    /// This is the post-decision step: the window calls it only after the tab's unsaved changes
    /// have been saved or explicitly discarded (DOC-003). Closing the last tab leaves a fresh
    /// empty document, because the window is never tab-less.
    pub fn close_document(&mut self, id: DocumentId) -> bool {
        let Some(index) = self.docs.iter().position(|doc| doc.id == id) else {
            return false;
        };
        if let Some(handle) = self.docs[index].run.as_ref() {
            let _ = self.host.cancel(handle);
        }
        let title = self.docs[index].title.clone();
        self.docs.remove(index);
        if self.docs.is_empty() {
            let id = self.next_document_id();
            self.docs
                .push(WorkbenchDocument::new(id, UNTITLED_TITLE, ""));
            self.active = 0;
        } else if self.active > index {
            self.active -= 1;
        } else if self.active >= self.docs.len() {
            self.active = self.docs.len() - 1;
        }
        self.log(self.active, LogKind::Info, format!("Closed {title}."));
        true
    }

    /// The identities of the tabs with unsaved changes, in tab order.
    ///
    /// DOC-003: the window asks about exactly these documents, so a clean tab never shows a prompt.
    pub fn dirty_documents(&self) -> Vec<DocumentId> {
        self.docs
            .iter()
            .filter(|doc| doc.dirty())
            .map(|doc| doc.id)
            .collect()
    }

    /// The index of the tab with this identity, if it is open.
    pub fn document_index(&self, id: DocumentId) -> Option<usize> {
        self.docs.iter().position(|doc| doc.id == id)
    }

    /// Paths the user opened or saved, newest first.
    pub fn recent_files(&self) -> &[String] {
        &self.recent_files
    }

    /// Records a path as most recently used, without duplicates and capped.
    fn add_recent(&mut self, path: &Path) {
        let path = path.to_string_lossy().into_owned();
        self.recent_files.retain(|existing| existing != &path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(crate::settings::MAX_RECENT_FILES);
    }

    /// DOC-004: adopts the persisted recent list when the application starts.
    ///
    /// Paths are deduplicated, capped and pruned of files that no longer exist, in that order of
    /// authority: an entry that survives keeps its position, so a file the user has not touched
    /// does not jump to the top merely because the list was loaded.
    pub fn set_recent_files(&mut self, paths: Vec<String>) {
        let mut loaded: Vec<String> = Vec::new();
        for path in paths {
            let path = path.trim();
            if path.is_empty() || !Path::new(path).exists() {
                continue;
            }
            if loaded.iter().any(|existing| existing == path) {
                continue;
            }
            loaded.push(path.to_string());
        }
        loaded.truncate(crate::settings::MAX_RECENT_FILES);
        self.recent_files = loaded;
    }

    /// DOC-005: stores the caret and selection the window's editor currently shows.
    ///
    /// The workbench never computes a caret; the window copies its editor's position here before
    /// each journal pass, so a recovery record names exactly where the user was typing.
    pub fn set_caret(
        &mut self,
        index: usize,
        caret: (usize, usize),
        selection: Option<((usize, usize), (usize, usize))>,
    ) {
        if let Some(document) = self.docs.get_mut(index) {
            document.caret = caret;
            document.selection = selection;
        }
    }

    /// DOC-004: the tab order, the active tab and each tab's file, for the settings file.
    ///
    /// A pathless tab keeps its document identity so the next launch can look for its recovery
    /// record; a tab with a file records the path, so it is restored by reading that file again.
    pub fn session_state(&self) -> SessionState {
        SessionState::new(
            self.docs
                .iter()
                .map(|document| SessionTab {
                    path: document
                        .path
                        .as_ref()
                        .map(|path| path.to_string_lossy().into_owned()),
                    document_id: document.id.0,
                })
                .collect(),
            self.active,
        )
    }

    /// DOC-004: replaces the tab list with the last session's tabs, in their original order.
    ///
    /// A tab with a file is restored by **reading** the file (never by executing it and never by
    /// writing it), so an unchanged file comes back byte-for-byte as the codec decoded it. A tab
    /// without a file is restored only when a DOC-005 recovery record covers it: the journal's
    /// text is what the user was typing, and putting it back performs no I/O on any document file.
    /// An entry that cannot be restored is reported in [`RestoreReport::skipped`] and the launch
    /// continues; a session whose every entry is unreadable leaves one fresh empty document.
    pub fn restore_session(
        &mut self,
        session: &SessionState,
        records: &[RecoveryRecord],
    ) -> RestoreReport {
        let mut report = RestoreReport::default();
        for document in &mut self.docs {
            if let Some(handle) = document.run.take() {
                let _ = self.host.cancel(&handle);
            }
        }
        self.docs.clear();
        self.active = 0;
        // The session's active tab is mapped to the restored tab it became, not to the same
        // numeric index: a skipped entry must not silently move the selection to a neighbour.
        let mut active_index: Option<usize> = None;

        for (entry_index, tab) in session.documents.iter().enumerate() {
            let record = records
                .iter()
                .find(|record| record.document_id == tab.document_id);
            let restored = match &tab.path {
                Some(path_text) => {
                    let path = PathBuf::from(path_text);
                    match document_io::read(&path) {
                        Ok(loaded) => Some(RestoredDocument {
                            id: DocumentId(tab.document_id),
                            title: file_name_of(&path),
                            path: Some(path.clone()),
                            source: loaded.text.clone(),
                            saved_source: loaded.text,
                            newline: Some(loaded.newline),
                            bom: loaded.bom,
                            source_mtime: file_mtime(&path),
                            original_bytes: Some(loaded.bytes),
                            caret: (0, 0),
                            selection: None,
                        }),
                        // The file is gone. If its journal record still has the text, that text
                        // is all that is left of the document, so restore it instead of dropping
                        // the tab; the record names the path it was being edited from.
                        Err(message) => match record {
                            Some(record) => {
                                report.recovered.push(record.document_id);
                                Some(Self::restored_from_record(record, DocumentId(tab.document_id)))
                            }
                            None => {
                                report.skipped.push(message);
                                None
                            }
                        },
                    }
                }
                None => match record {
                    Some(record) => {
                        report.recovered.push(record.document_id);
                        Some(Self::restored_from_record(record, DocumentId(tab.document_id)))
                    }
                    None => {
                        report.skipped.push(format!(
                            "the unsaved buffer for document {} has no recovery record",
                            tab.document_id
                        ));
                        None
                    }
                },
            };
            if let Some(restored) = restored {
                let index = self.install_document(restored);
                report.restored.push(self.docs[index].id);
                if entry_index == session.active {
                    active_index = Some(index);
                } else if active_index.is_none() && entry_index > session.active {
                    // The active entry was skipped; the nearest following restored tab is the
                    // closest thing to what the user had selected.
                    active_index = Some(index);
                }
            }
        }

        if self.docs.is_empty() {
            let id = self.next_document_id();
            self.docs
                .push(WorkbenchDocument::new(id, UNTITLED_TITLE, ""));
        }
        let highest = self.docs.iter().map(|doc| doc.id.0).max().unwrap_or(0);
        self.next_id = self.next_id.max(highest.saturating_add(1));
        self.active = active_index.unwrap_or(self.docs.len() - 1);

        let count = report.restored.len();
        let total = session.documents.len();
        self.log(
            self.active,
            LogKind::Info,
            format!(
                "Restored {count} of {total} document(s) from the last session; nothing was run \
                 and no file was written."
            ),
        );
        for skipped in &report.skipped {
            self.log(
                self.active,
                LogKind::Diagnostic,
                format!("Session restore skipped a tab: {skipped}"),
            );
        }
        report
    }

    /// DOC-005: puts one recovery record's text into a tab, before any save can overwrite it.
    ///
    /// The text goes into the tab that already represents the record's document when one is open
    /// (the session restore usually recreated it from disk, and the edit is what is missing), or
    /// into a new tab otherwise. The file on disk is not touched in either case, and nothing is
    /// executed: the recovered text is only a buffer until the user saves it.
    pub fn recover(&mut self, record: &RecoveryRecord) -> RecoverReport {
        let existing = self
            .docs
            .iter()
            .position(|document| document.id.0 == record.document_id)
            .or_else(|| {
                record.path.as_ref().and_then(|path| {
                    self.docs
                        .iter()
                        .position(|document| document.path.as_deref() == Some(path.as_path()))
                })
            });
        let bytes = record.buffer_bytes();
        if let Some(index) = existing {
            if let Some(handle) = self.docs[index].run.take() {
                let _ = self.host.cancel(&handle);
            }
            let title = self.docs[index].title.clone();
            let id = self.docs[index].id;
            {
                let document = &mut self.docs[index];
                let was_active = document.state() != RunState::Idle;
                document.source = record.source.clone();
                document.caret = record.caret;
                document.selection = record.selection;
                document.source_mtime = record.source_mtime;
                document.revision = document.revision.saturating_add(1);
                document.snapshot = None;
                document.last_plot = None;
                document.diagnostic = None;
                document.diagnostic_revision = None;
                document.pending_logs.clear();
                document.current_statement = None;
                document.committed = 0;
                document.total = 0;
                document.elapsed_ms = 0;
                if was_active {
                    document.run_state.reset("document recovered from the journal");
                }
            }
            self.active = index;
            self.log(
                index,
                LogKind::Info,
                format!("Recovered {title} from the recovery journal ({bytes} bytes); nothing was written."),
            );
            RecoverReport {
                document_id: id,
                index,
                replaced: true,
                bytes,
            }
        } else {
            let id = self.next_document_id();
            let restored = Self::restored_from_record(record, id);
            let index = self.install_document(restored);
            self.active = index;
            let title = self.docs[index].title.clone();
            self.log(
                index,
                LogKind::Info,
                format!(
                    "Recovered {title} from the recovery journal ({bytes} bytes); nothing was executed."
                ),
            );
            RecoverReport {
                document_id: id,
                index,
                replaced: false,
                bytes,
            }
        }
    }

    /// The [`RestoredDocument`] a recovery record describes.
    fn restored_from_record(record: &RecoveryRecord, id: DocumentId) -> RestoredDocument {
        // The disk text is the "saved" side of the dirty comparison: a recovered buffer whose
        // text happens to equal the file's is not dirty, one that differs is.
        let saved_source = record
            .path
            .as_deref()
            .and_then(|path| document_io::read(path).ok())
            .map(|loaded| loaded.text)
            .unwrap_or_default();
        RestoredDocument {
            id,
            title: record.title.clone(),
            path: record.path.clone(),
            source: record.source.clone(),
            saved_source,
            // The journal holds the buffer's own line endings, so the style is detected from it.
            newline: None,
            bom: record.bom,
            source_mtime: record.source_mtime,
            // A recovered buffer was never written from this process, so there are no file bytes
            // to write back verbatim: a save re-encodes the text the user recovered.
            original_bytes: None,
            caret: record.caret,
            selection: record.selection,
        }
    }

    /// Appends one restored document and returns its index.
    fn install_document(&mut self, restored: RestoredDocument) -> usize {
        let mut document =
            WorkbenchDocument::new(restored.id, &restored.title, &restored.source);
        document.path = restored.path;
        document.newline = restored
            .newline
            .unwrap_or_else(|| NewlineStyle::detect(&restored.source));
        document.bom = restored.bom;
        document.source_mtime = restored.source_mtime;
        document.caret = restored.caret;
        document.selection = restored.selection;
        document.saved_source = restored.saved_source;
        document.original_bytes = restored.original_bytes;
        self.docs.push(document);
        self.docs.len() - 1
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
            LogKind::Info,
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
            self.log(index, LogKind::Info, "A run is already in progress.");
            return false;
        }
        if doc.source.trim().is_empty() {
            self.log(
                index,
                LogKind::Info,
                "Nothing to run: the document is empty.",
            );
            return false;
        }
        let source = doc.source.clone();
        match self.begin_run(index, source) {
            Ok((run_id, revision)) => {
                self.log(
                    index,
                    LogKind::Info,
                    format!("Run queued (revision {}, run {}).", revision.0, run_id.0),
                );
                true
            }
            Err(SubmitError::Busy) => {
                self.log(
                    index,
                    LogKind::Info,
                    "The execution host is still finishing the previous run.",
                );
                false
            }
            Err(SubmitError::Spawn(message)) => {
                self.log(
                    index,
                    LogKind::Diagnostic,
                    format!("Could not start the run worker: {message}"),
                );
                false
            }
        }
    }

    /// Requests a quick evaluation of `expression` in the active document.
    ///
    /// RUN-002: this goes through exactly the same [`ExecutionHost::submit`] call as
    /// [`Workbench::run_active`], with the active document's id, source revision and precision,
    /// but the submitted source is the one-line expression instead of the editor buffer. The
    /// buffer and its revision are never touched.
    ///
    /// LOG-003: the field is the caller's, so this only reports whether the request was accepted;
    /// the window clears its input only on `true`. A refusal while a run is in flight leaves the
    /// expression unlogged, because no run was submitted to echo.
    ///
    /// The run is log-and-status only for the document panes: every run constructs a fresh
    /// engine, so its snapshot describes the expression and cannot replace the workspace snapshot
    /// of a document whose source was never evaluated. The `[echo]`, `[result]`, `[stdout]`,
    /// `[diagnostic]` and `[timing]` lines still come from the normal typed event stream.
    pub fn quick_eval_active(&mut self, expression: &str) -> bool {
        let expression = expression.trim();
        if expression.is_empty() {
            return false;
        }
        let index = self.active;
        if self.docs[index].state().is_in_flight() {
            self.log(
                index,
                LogKind::Info,
                "Quick eval not submitted: a run is already in progress; the expression was kept.",
            );
            return false;
        }
        let expression = expression.to_string();
        match self.begin_run(index, expression.clone()) {
            Ok((run_id, revision)) => {
                self.docs[index].quick_eval_run = Some(run_id);
                self.log(index, LogKind::Echo, format!(">> {expression}"));
                self.log(
                    index,
                    LogKind::Info,
                    format!(
                        "Quick eval queued (revision {}, run {}).",
                        revision.0, run_id.0
                    ),
                );
                true
            }
            Err(SubmitError::Busy) => {
                self.log(
                    index,
                    LogKind::Info,
                    "Quick eval not submitted: the execution host is still finishing the previous run; the expression was kept.",
                );
                false
            }
            Err(SubmitError::Spawn(message)) => {
                self.log(
                    index,
                    LogKind::Diagnostic,
                    format!("Could not start the quick-eval worker: {message}"),
                );
                false
            }
        }
    }

    /// Submits `source` for document `index` through the one [`ExecutionHost`] entry point both
    /// Run and quick eval use, and records the accepted run on the document.
    ///
    /// The caller has already validated the request (non-empty source, no run in flight) and is
    /// responsible for the log line describing what it submitted.
    fn begin_run(
        &mut self,
        index: usize,
        source: String,
    ) -> Result<(RunId, SourceRevision), SubmitError> {
        let doc = &self.docs[index];
        let revision = SourceRevision(doc.revision);
        let precision = doc.precision;
        let handle = self.host.submit(doc.id, revision, source, precision)?;
        let run_id = handle.run();
        {
            let doc = &mut self.docs[index];
            doc.run = Some(handle);
            doc.quick_eval_run = None;
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
        Ok((run_id, revision))
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
            self.log(index, LogKind::Info, "Cancellation requested.");
        }
        cancellation_requested
    }

    /// True while any run is in flight.
    pub fn is_running(&self) -> bool {
        self.docs.iter().any(|doc| doc.state().is_in_flight())
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
        // A quick-eval run carries the document's id and revision but evaluated the one-line
        // expression, not the buffer. Its terminal snapshot and diagnostic must not claim the
        // editor's text; both panes stay tied to the document's own run. The log lines are the
        // same normal typed events as any other run.
        let quick_eval = self.docs[index].quick_eval_run == Some(tracked_run);

        match &sequenced.event {
            DesktopEvent::RunStarted { total, .. } => {
                // A run starts with an empty statement window: anything still buffered belongs
                // to a delivery that never committed and must not leak into this run's log.
                self.drop_pending(index);
                crate::run_trace::mark(
                    "run_started_applied",
                    format!("total={} document={}", total, id.0),
                );
                let mut started = false;
                let mut cancellation_pending = false;
                {
                    let doc = &mut self.docs[index];
                    if doc.state() == RunState::Queued {
                        started = doc
                            .run_state
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
                    self.log(index, LogKind::Info, detail);
                } else if cancellation_pending {
                    self.log(
                        index,
                        LogKind::Info,
                        "Run started while cancellation was pending; waiting for the terminal event.",
                    );
                }
                true
            }
            DesktopEvent::StepStarted {
                index: statement,
                label,
                ..
            } => {
                // LOG-002 progress: the workbench already receives one StepStarted per
                // top-level statement, so the log routes that existing event instead of
                // inventing progress. RUN-003: the line is buffered and only appended when the
                // statement commits, so it cannot name a statement whose effects are not in the
                // workspace yet.
                let total = self.docs[index].total;
                self.docs[index].current_statement = Some(*statement);
                let line = LogLine::new(
                    LogKind::Progress,
                    format!("statement {}/{}: {}", statement + 1, total, label),
                )
                .at_statement(*statement);
                self.push_pending(index, line);
                true
            }
            DesktopEvent::StepCommitted {
                index: statement,
                snapshot,
                ..
            } => {
                // RUN-003: this one branch advances every pane. The event's snapshot is the
                // coherent committed prefix; the statement's buffered progress/stdout lines are
                // appended first, then the snapshot is published and the committed counter moves.
                // A stale (abandoned) run keeps its lines buffered for the discard notice and
                // never publishes a snapshot that describes replaced source.
                if stale {
                    return true;
                }
                debug_assert_eq!(
                    snapshot.committed_steps,
                    statement + 1,
                    "a commit snapshot must describe exactly the statements before and including it"
                );
                self.flush_pending(index);
                if !quick_eval {
                    self.publish_snapshot(index, snapshot.clone());
                }
                self.docs[index].committed = snapshot.committed_steps;
                true
            }
            DesktopEvent::Output { text, .. } => {
                // RUN-003: buffered with the statement it belongs to and flushed at its commit.
                let line = LogLine::new(LogKind::Stdout, text.clone());
                let line = match self.docs[index].current_statement {
                    Some(statement) => line.at_statement(statement),
                    None => line,
                };
                self.push_pending(index, line);
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
                // The terminal snapshot is the same coherent object the terminal publish wrote;
                // flushing first guarantees the log tail's statement lines are exactly the
                // committed prefix the snapshot names.
                self.flush_pending(index);
                let accepted = self.docs[index].sink.apply(sequenced);
                if !accepted {
                    return self.discard_terminal(
                        index,
                        tracked_run,
                        "the result did not belong to the document's current revision.",
                    );
                }
                if !quick_eval {
                    self.publish_snapshot(index, snapshot.clone());
                }
                {
                    let doc = &mut self.docs[index];
                    // A successful document run is the only outcome that clears the failure
                    // decoration: the buffer it describes is still current and now evaluates
                    // cleanly. A quick eval did not evaluate the buffer, so the decoration for
                    // the document's last failed run remains truthful.
                    if !quick_eval {
                        doc.diagnostic = None;
                        doc.diagnostic_revision = None;
                    }
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
                self.log(index, LogKind::Result, format!("= {result}"));
                self.log(
                    index,
                    LogKind::Timing,
                    format!(
                        "Run finished: {} statements committed in {} ms.",
                        snapshot.committed_steps, elapsed
                    ),
                );
                true
            }
            DesktopEvent::Cancelled {
                committed_steps,
                snapshot,
                ..
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
                self.flush_pending(index);
                if !quick_eval {
                    self.publish_snapshot(index, snapshot.clone());
                }
                {
                    let doc = &mut self.docs[index];
                    debug_assert_eq!(
                        snapshot.committed_steps, *committed_steps,
                        "the Cancelled count and its snapshot must name the same committed prefix"
                    );
                    doc.committed = snapshot.committed_steps;
                    let transitioned = doc.run_state.finish(
                        tracked_run,
                        RunState::Cancelled,
                        "Cancelled event applied",
                    );
                    debug_assert!(transitioned, "a terminal event must end the in-flight run");
                }
                let elapsed = self.close_run(index);
                let text = if quick_eval {
                    format!(
                        "Run cancelled after {} statements in {} ms.",
                        committed_steps, elapsed
                    )
                } else {
                    format!(
                        "Run cancelled after {} statements in {} ms; the committed statements remain in the workspace.",
                        committed_steps, elapsed
                    )
                };
                self.log(index, LogKind::Timing, text);
                true
            }
            DesktopEvent::Failed {
                diagnostic,
                snapshot,
                ..
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
                self.flush_pending(index);
                if !quick_eval {
                    self.publish_snapshot(index, snapshot.clone());
                }
                {
                    let doc = &mut self.docs[index];
                    // A failed quick eval names a position in the expression, not in the
                    // document; it is logged but must not decorate the editor.
                    if !quick_eval {
                        doc.diagnostic = Some(diagnostic.clone());
                        doc.diagnostic_revision = Some(sequenced.revision);
                    }
                    doc.committed = snapshot.committed_steps;
                    let transitioned =
                        doc.run_state
                            .finish(tracked_run, RunState::Failed, "Failed event applied");
                    debug_assert!(transitioned, "a terminal event must end the in-flight run");
                }
                let elapsed = self.close_run(index);
                self.log(
                    index,
                    LogKind::Diagnostic,
                    format!("Diagnostic: {}", diagnostic.render()),
                );
                let text = if quick_eval {
                    format!(
                        "Run failed after {} committed statements in {} ms.",
                        snapshot.committed_steps, elapsed
                    )
                } else {
                    format!(
                        "Run failed after {} committed statements in {} ms; the committed prefix remains in the workspace.",
                        snapshot.committed_steps, elapsed
                    )
                };
                self.log(index, LogKind::Timing, text);
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
        doc.quick_eval_run = None;
        // A terminal event ends the run's statement window; any line still buffered describes a
        // statement that never committed and must not leak into a later run.
        doc.pending_logs.clear();
        doc.current_statement = None;
        elapsed
    }

    /// Retires a run whose terminal event cannot be published, without publishing its snapshot.
    ///
    /// The run was in flight and is now over, so the state machine still accepts exactly one
    /// terminal transition. `Cancelled` is the truthful state: the run was abandoned by the
    /// source edit (or its result failed the revision check), not completed for this buffer.
    /// The abandoned run's buffered statement lines are still appended: the log narrates what
    /// was attempted, while no snapshot is published and therefore no pane claims its effects.
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
        self.flush_pending(index);
        let _ = self.close_run(index);
        self.log(
            index,
            LogKind::Info,
            format!("Run result discarded: {reason}"),
        );
        true
    }

    /// Records a snapshot and adopts exactly the plot it carries.
    ///
    /// RUN-003: the plot is part of the snapshot, so a commit without a plot (a new run whose
    /// committed prefix has not drawn one) must clear the previous run's plot instead of leaving
    /// a picture that belongs to different variables and a different revision.
    fn publish_snapshot(&mut self, index: usize, snapshot: WorkspaceSnapshot) {
        self.docs[index].last_plot = snapshot.plot.clone();
        self.docs[index].snapshot = Some(snapshot);
    }

    /// Appends a log line of `kind` to the active document.
    ///
    /// DOC-004/005: the window reports settings, session-restore and journal problems through the
    /// same log the Logs pane shows, so a skipped record or an unwritable settings file is visible
    /// in the product instead of only on stderr.
    pub fn log_active(&mut self, kind: LogKind, text: impl Into<String>) {
        let active = self.active;
        self.log(active, kind, text);
    }

    /// Appends a log line of `kind`, enforcing [`LOG_CAP`].
    fn log(&mut self, index: usize, kind: LogKind, text: impl Into<String>) {
        let Some(doc) = self.docs.get_mut(index) else {
            return;
        };
        Self::push_line(doc, LogLine::new(kind, text));
    }

    /// Appends one already-built line, enforcing [`LOG_CAP`].
    fn push_line(doc: &mut WorkbenchDocument, line: LogLine) {
        if doc.logs.len() >= LOG_CAP {
            doc.logs.remove(0);
        }
        doc.logs.push(line);
    }

    /// Buffers a statement-scoped line until the statement commits.
    fn push_pending(&mut self, index: usize, line: LogLine) {
        if let Some(doc) = self.docs.get_mut(index) {
            doc.pending_logs.push(line);
        }
    }

    /// Appends every buffered statement line in production order and closes the statement window.
    fn flush_pending(&mut self, index: usize) {
        let Some(doc) = self.docs.get_mut(index) else {
            return;
        };
        doc.current_statement = None;
        let pending = std::mem::take(&mut doc.pending_logs);
        for line in pending {
            Self::push_line(doc, line);
        }
    }

    /// Drops buffered statement lines without appending them.
    fn drop_pending(&mut self, index: usize) {
        if let Some(doc) = self.docs.get_mut(index) {
            doc.pending_logs.clear();
            doc.current_statement = None;
        }
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
    #[allow(dead_code)] // host-facing accessor; the window renders `active_view().plot`
    pub fn active_plot(&self) -> Option<&PlotCapture> {
        self.docs[self.active].last_plot.as_ref()
    }

    /// The active document's log lines.
    #[allow(dead_code)] // host-facing accessor; the window renders `active_view().logs`
    pub fn active_logs(&self) -> &[LogLine] {
        &self.docs[self.active].logs
    }

    /// The active document's last snapshot, if any.
    pub fn active_snapshot(&self) -> Option<&WorkspaceSnapshot> {
        self.docs[self.active].snapshot.as_ref()
    }

    /// Assembles the state the panes render for one document.
    ///
    /// Every pane-relevant field is copied from this document in one call: the committed
    /// snapshot (variables, functions, state, plot, output, result, source revision and
    /// committed/total statement counts), the log, the revision-gated diagnostic and the run
    /// state. A consumer that holds the returned value holds one coherent state.
    pub fn view(&self, index: usize) -> Option<DocumentView> {
        let document = self.docs.get(index)?;
        let snapshot = document.snapshot.as_ref();
        Some(DocumentView {
            revision: snapshot
                .map(|snapshot| snapshot.revision)
                .unwrap_or(SourceRevision(0)),
            has_snapshot: snapshot.is_some(),
            committed_steps: snapshot
                .map(|snapshot| snapshot.committed_steps)
                .unwrap_or(0),
            total: snapshot.map(|snapshot| snapshot.total).unwrap_or(0),
            result: snapshot
                .map(|snapshot| snapshot.result.clone())
                .unwrap_or_default(),
            result_structured: snapshot.and_then(|snapshot| snapshot.result_structured.clone()),
            result_inspection: snapshot.and_then(|snapshot| snapshot.result_inspection.clone()),
            variables: snapshot
                .map(|snapshot| snapshot.variables.clone())
                .unwrap_or_default(),
            functions: snapshot
                .map(|snapshot| snapshot.functions.clone())
                .unwrap_or_default(),
            state: snapshot
                .map(|snapshot| snapshot.state.clone())
                .unwrap_or_default(),
            plot: snapshot.and_then(|snapshot| snapshot.plot.clone()),
            output: snapshot
                .map(|snapshot| snapshot.output.clone())
                .unwrap_or_default(),
            logs: document.logs.clone(),
            diagnostic: self.diagnostic(index).cloned(),
            run_state: document.state(),
            elapsed_ms: document.elapsed_ms,
        })
    }

    /// The active document's assembled view (the state the window renders this frame).
    pub fn active_view(&self) -> DocumentView {
        self.view(self.active)
            .expect("the workbench always has an active document")
    }

    /// The active document's failure diagnostic, while it still describes the buffer.
    #[allow(dead_code)] // host-facing accessor; the window renders `active_view().diagnostic`
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
            newline: NewlineStyle::detect(source),
            bom: false,
            source_mtime: None,
            caret: (0, 0),
            selection: None,
            source: source.to_string(),
            precision: DEFAULT_PRECISION,
            revision: 0,
            sink: EventSink::new(),
            snapshot: None,
            last_plot: None,
            logs: Vec::new(),
            pending_logs: Vec::new(),
            current_statement: None,
            diagnostic: None,
            diagnostic_revision: None,
            run_state: RunStateMachine::new(),
            last_sequence: 0,
            committed: 0,
            total: 0,
            elapsed_ms: 0,
            run: None,
            quick_eval_run: None,
            started: None,
            saved_source: source.to_string(),
            original_bytes: None,
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
    use crate::document_io::LineEnding;
    use crate::engine::{EngineFunction, Evaluator, FakeEngine};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    fn fake_workbench() -> Workbench {
        Workbench::with_factory(Box::new(|| Box::new(FakeEngine::new())))
    }

    /// The log lines a run produced, from its own "Run queued" line onward.
    ///
    /// The constructor's plot-directory line is environment-dependent (a read-only APPDATA adds
    /// a startup [`LogKind::Diagnostic`]), so the event-sequence tests slice at the run boundary
    /// instead of pinning the whole startup log.
    fn run_log(document: &WorkbenchDocument) -> &[LogLine] {
        let start = document
            .logs
            .iter()
            .position(|line| line.text.starts_with("Run queued"))
            .expect("a run must log its queue line");
        &document.logs[start..]
    }

    /// The lines a quick-eval submission produced, from its own echo line onward.
    fn quick_eval_log(document: &WorkbenchDocument) -> &[LogLine] {
        let start = document
            .logs
            .iter()
            .rposition(|line| line.kind == LogKind::Echo)
            .expect("an accepted quick eval must log its echo line");
        &document.logs[start..]
    }

    fn builtin(name: &str, parameters: &[&str]) -> EngineFunction {
        EngineFunction {
            name: name.to_string(),
            parameters: parameters.iter().map(|p| p.to_string()).collect(),
            is_builtin: true,
            plugin: None,
            summary: None,
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
        let mut workbench =
            Workbench::with_factory(Box::new(|| Box::new(crate::engine::RealEngine::new())));
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

    /// WKS-001/WKS-002 with the shipped engine: after a run that defines a user `func` and a
    /// variable, the committed snapshot publishes `_` with its kind and display, the function
    /// table is built from the registry capture (builtins + plugin builtins + the user
    /// definition), and the table's count agrees with the snapshot and with a fresh engine's
    /// registry plus that one user definition.
    #[test]
    #[cfg(feature = "real-engine")]
    fn the_real_snapshot_publishes_underscore_and_the_registry_count_for_the_tables() {
        let mut workbench =
            Workbench::with_factory(Box::new(|| Box::new(crate::engine::RealEngine::new())));
        workbench.set_source(0, "func aaa(x) = x * 2\na = 21\naaa(a)\n".to_string());
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(10)));
        assert_eq!(workbench.documents()[0].state(), RunState::Finished);

        let view = workbench.active_view();
        let underscore = view
            .state
            .variables
            .iter()
            .find(|variable| variable.name == "_")
            .expect("the engine publishes the answer global in its capture");
        assert_eq!(underscore.kind, "Natural");
        assert_eq!(underscore.display, "42");
        let a = view
            .state
            .variables
            .iter()
            .find(|variable| variable.name == "a")
            .expect("the assigned variable is in the capture");
        assert_eq!(a.kind, "Natural");
        assert_eq!(a.display, "21");
        assert!(
            view.variables.iter().any(|(name, _)| name == "_"),
            "the snapshot's variable list also names _: {:?}",
            view.variables
        );

        let table = crate::workspace_table::FunctionTable::new(&view.state);
        let fresh_registry = crate::engine::RealEngine::new()
            .state()
            .expect("a fresh real engine publishes its registry");
        assert_eq!(
            table.rows.len(),
            view.state.functions.len(),
            "the table rows are the snapshot's registry capture"
        );
        assert_eq!(
            table.rows.len(),
            view.functions.len(),
            "the displayed count equals the snapshot's function count"
        );
        assert_eq!(
            table.rows.len(),
            fresh_registry.functions.len() + 1,
            "the current registry plus the one user definition"
        );

        let user = table.rows.iter().find(|row| row.name == "aaa").unwrap();
        assert_eq!(user.origin, crate::engine::FunctionOrigin::User);
        assert_eq!(user.parameters_text(), "x");
        let builtin = table.rows.iter().find(|row| row.name == "abs").unwrap();
        assert_eq!(builtin.origin, crate::engine::FunctionOrigin::Builtin);
        let plugin = table.rows.iter().find(|row| row.name == "acos").unwrap();
        assert_eq!(plugin.origin, crate::engine::FunctionOrigin::Plugin);
    }

    /// RES-001 end to end with the shipped engine: one run publishes the structured projection
    /// of its last committed result in the snapshot/view, every captured variable carries the
    /// engine's projection too, and the typed tree is built from that data (never the display).
    #[test]
    #[cfg(feature = "real-engine")]
    fn the_real_snapshot_publishes_the_structured_result_and_variable_projections() {
        use crate::results_tree::{ResultProjection, StructuredNode};

        let mut workbench =
            Workbench::with_factory(Box::new(|| Box::new(crate::engine::RealEngine::new())));
        workbench.set_source(
            0,
            "m = [[1, 2], [3, 4]]\nx = symbol(\"x\")\ninv_full([[x, 1], [1, x]])\n".to_string(),
        );
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(10)));
        assert_eq!(workbench.documents()[0].state(), RunState::Finished);

        let view = workbench.active_view();
        let structured = view
            .result_structured
            .as_ref()
            .expect("the committed result carries the engine's projection");
        assert_eq!(
            structured.get("kind").and_then(lovelace_json::Json::as_str),
            Some("Record"),
            "the last result is the inv_full record: {structured:?}"
        );
        let tree = ResultProjection::new(view.result.clone(), Some(structured.clone()))
            .tree()
            .expect("the structured result is a tree");
        let StructuredNode::Record(record) = tree else {
            panic!("the committed result must render as a record");
        };
        assert_eq!(record.type_name, "MatrixInverseResult");
        assert_eq!(
            record.field_names(),
            [
                "status",
                "complete",
                "completeness",
                "inverse",
                "conditions",
                "diagnostics"
            ]
        );
        let StructuredNode::Array(inverse) = record.field("inverse").unwrap() else {
            panic!("the inverse field is the nested 2x2 matrix");
        };
        assert_eq!(inverse.shape, [2, 2]);

        let m = view
            .state
            .variables
            .iter()
            .find(|variable| variable.name == "m")
            .expect("m is in the committed capture");
        let m_tree = StructuredNode::from_structured(
            m.structured
                .as_ref()
                .expect("the variable carries the engine projection"),
        )
        .expect("the variable projection is a tree");
        let StructuredNode::Array(m) = m_tree else {
            panic!("m is a 2x2 array");
        };
        assert_eq!(m.shape, [2, 2]);
        assert_eq!(m.cell(1, 1).unwrap().headline(), "Natural 4");

        // The display is beside the projection, and the tree above was built without it: the
        // snapshot carries both fields from the same commit.
        assert!(
            view.result.contains("MatrixInverseResult"),
            "the engine's display text is available alongside: {:?}",
            view.result
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
            [RunState::Queued, RunState::Running, RunState::Finished],
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

    /// LOG-002: a known successful run fed through the real `ExecutionHost` event path must
    /// yield the expected kind sequence, in order, with no kind silently dropped. The run slice
    /// starts at its queue line, then start, one progress line per statement, the print's stdout
    /// line, the result and the terminal timing line.
    #[test]
    fn a_successful_run_logs_the_expected_kind_sequence_in_order() {
        let mut workbench = fake_workbench();
        workbench.set_source(0, "print(\"hi\")\n6 * 7\n".to_string());
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        assert_eq!(workbench.documents()[0].state(), RunState::Finished);

        let document = &workbench.documents()[0];
        let logs = run_log(document);
        let kinds = logs.iter().map(|line| line.kind).collect::<Vec<_>>();
        assert_eq!(
            kinds,
            vec![
                LogKind::Info,
                LogKind::Info,
                LogKind::Progress,
                LogKind::Stdout,
                LogKind::Progress,
                LogKind::Result,
                LogKind::Timing,
            ],
            "logs were {:?}",
            document.logs
        );
        assert_eq!(logs[2].text, "statement 1/2: print(\"hi\")");
        assert_eq!(logs[3].text, "hi");
        assert_eq!(logs[4].text, "statement 2/2: 6 * 7");
        assert_eq!(logs[5].text, "= 42");
        assert!(
            logs[6].text.starts_with("Run finished:"),
            "the timing line was {:?}",
            logs[6]
        );
        assert!(logs[6].text.contains(" ms."));
        for line in &document.logs {
            assert!(
                line.render().starts_with(line.kind.marker()),
                "the painted line lost its marker: {line:?}"
            );
        }
    }

    /// LOG-002: a deliberate error must append exactly one diagnostic line followed by the
    /// timing line, after the progress and stdout lines from the statements that did commit.
    #[test]
    fn a_failing_run_logs_diagnostic_and_timing_after_progress_and_stdout() {
        let mut workbench = fake_workbench();
        workbench.set_source(0, "print(\"hi\")\n1 / 0\n".to_string());
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        assert_eq!(workbench.documents()[0].state(), RunState::Failed);

        let document = &workbench.documents()[0];
        let logs = run_log(document);
        let kinds = logs.iter().map(|line| line.kind).collect::<Vec<_>>();
        assert_eq!(
            kinds,
            vec![
                LogKind::Info,
                LogKind::Info,
                LogKind::Progress,
                LogKind::Stdout,
                LogKind::Progress,
                LogKind::Diagnostic,
                LogKind::Timing,
            ],
            "logs were {:?}",
            document.logs
        );
        assert_eq!(logs[3].text, "hi");
        assert!(
            logs[5].text.starts_with("Diagnostic: ")
                && logs[5].text.contains("Cannot divide by zero."),
            "the diagnostic line was {:?}",
            logs[5]
        );
        assert!(
            logs[6]
                .text
                .starts_with("Run failed after 1 committed statements in "),
            "the failed timing line was {:?}",
            logs[6]
        );
        assert!(
            logs[6].text.contains(" ms;"),
            "the failed timing line was {:?}",
            logs[6]
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
                .any(|line| line.kind == LogKind::Stdout && line.text == "x")
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
            .filter(|line| line.kind == LogKind::Stdout)
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
            .filter(|line| line.kind == LogKind::Stdout)
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
            .any(|line| line.kind == LogKind::Stdout && line.text == "one")
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
            .filter(|line| line.kind == LogKind::Stdout)
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
            .filter(|line| line.kind == LogKind::Stdout && line.text == "tick")
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
            document.logs.iter().any(
                |line| line.kind == LogKind::Diagnostic && line.text.starts_with("Diagnostic:")
            )
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
                .any(|line| line.kind == LogKind::Diagnostic),
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
            [RunState::Queued, RunState::Running, RunState::Finished]
        );
        assert!(
            document
                .duration_between(RunState::Queued, RunState::Running)
                .is_some()
        );
        assert!(
            document
                .duration_between(RunState::Running, RunState::Finished)
                .is_some()
        );
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

    /// RUN-002: a quick eval submits through the same `ExecutionHost` entry point as Run, for
    /// the active document and with its precision, but never writes the editor buffer or bumps
    /// its revision. The expression's result reaches the log.
    #[test]
    fn quick_eval_runs_against_the_active_document_without_touching_its_buffer_or_revision() {
        let mut workbench = fake_workbench();
        workbench.set_source(0, "a = 1\n".to_string());
        workbench.set_active(1);
        workbench.set_source(1, "b = 2\n".to_string());
        workbench.set_active_precision(250);
        let source_before = workbench.documents()[1].source.clone();
        let revision_before = workbench.documents()[1].revision;
        let inactive_revision = workbench.documents()[0].revision;

        assert!(workbench.quick_eval_active("6 * 7"));
        assert!(
            workbench.wait_until_idle(Duration::from_secs(5)),
            "the quick-eval run must reach a terminal state"
        );

        let document = &workbench.documents()[1];
        assert_eq!(
            document.source, source_before,
            "quick eval must not change the active document's buffer"
        );
        assert_eq!(
            document.revision, revision_before,
            "quick eval must not advance the active document's revision"
        );
        assert_eq!(
            workbench.documents()[0].revision, inactive_revision,
            "quick eval must not touch the inactive document"
        );
        assert_eq!(document.state(), RunState::Finished);
        assert!(
            document.snapshot.is_none(),
            "an expression-only run must not replace the document's workspace snapshot"
        );

        let echoes: Vec<&LogLine> = document
            .logs
            .iter()
            .filter(|line| line.kind == LogKind::Echo)
            .collect();
        assert_eq!(echoes.len(), 1, "one submission must log one echo");
        assert_eq!(echoes[0].text, ">> 6 * 7");
        assert_eq!(echoes[0].render(), "[echo] >> 6 * 7");
        let run = quick_eval_log(document);
        assert!(
            run.iter()
                .any(|line| line.kind == LogKind::Result && line.text == "= 42"),
            "quick-eval log was {run:?}"
        );
        assert!(
            run.iter().any(|line| {
                line.kind == LogKind::Timing && line.text.starts_with("Run finished: 1 statements")
            }),
            "quick-eval log was {run:?}"
        );
        assert!(
            run.iter()
                .any(|line| line.kind == LogKind::Info && line.text.contains("precision 250")),
            "the run must carry the active document's current precision: {run:?}"
        );
        assert!(
            !workbench.documents()[0]
                .logs
                .iter()
                .any(|line| line.kind == LogKind::Echo),
            "the inactive document must not receive the quick-eval log lines"
        );
    }

    /// A failed quick eval logs its structured diagnostic like any run, but must not decorate
    /// the editor: the diagnostic's line and column name the expression, not the document text.
    #[test]
    fn a_failing_quick_eval_logs_its_diagnostic_without_decorating_the_document() {
        let mut workbench = fake_workbench();
        workbench.set_source(0, "a = 1\n".to_string());
        assert!(workbench.quick_eval_active("1 / 0"));
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));

        let document = &workbench.documents()[0];
        assert_eq!(document.state(), RunState::Failed);
        assert!(
            document.snapshot.is_none(),
            "the expression snapshot must not become the document workspace"
        );
        assert!(
            workbench.active_diagnostic().is_none(),
            "the expression's position must not decorate the editor"
        );
        assert!(
            document.logs.iter().any(|line| {
                line.kind == LogKind::Diagnostic && line.text.contains("Cannot divide by zero.")
            }),
            "logs were {:?}",
            document.logs
        );
        assert!(
            document.logs.iter().any(|line| {
                line.kind == LogKind::Timing
                    && line.text.starts_with("Run failed after")
                    && line.text.ends_with(" ms.")
                    && !line.text.contains("workspace")
            }),
            "an expression-only failure must not claim a workspace prefix: {:?}",
            document.logs
        );
        assert_eq!(
            document
                .logs
                .iter()
                .filter(|line| line.kind == LogKind::Echo)
                .count(),
            1
        );
    }

    /// LOG-003 / LOG-002: every accepted submission emits exactly one echo line carrying the
    /// submitted text, in submission order. Two submissions therefore produce two echoes, each
    /// once, and each result follows its own echo.
    #[test]
    fn each_quick_eval_submission_emits_exactly_one_echo_line_for_its_own_text() {
        let mut workbench = fake_workbench();
        assert!(workbench.quick_eval_active("6 * 7"));
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        assert!(workbench.quick_eval_active("2 + 3"));
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));

        let document = &workbench.documents()[0];
        let echoes: Vec<&str> = document
            .logs
            .iter()
            .filter(|line| line.kind == LogKind::Echo)
            .map(|line| line.text.as_str())
            .collect();
        assert_eq!(echoes, [">> 6 * 7", ">> 2 + 3"]);
        for expression in [">> 6 * 7", ">> 2 + 3"] {
            assert_eq!(
                echoes.iter().filter(|text| **text == expression).count(),
                1,
                "{expression} must be echoed exactly once"
            );
        }
        let results: Vec<&str> = document
            .logs
            .iter()
            .filter(|line| line.kind == LogKind::Result)
            .map(|line| line.text.as_str())
            .collect();
        assert_eq!(results, ["= 42", "= 5"]);
    }

    /// LOG-003: a submit refused because a run is in flight must not clear the caller's field,
    /// must not fake an echo line for a run the host never accepted, and must log an `[info]`
    /// advisory that the expression was kept.
    #[test]
    fn a_quick_eval_refused_while_busy_keeps_the_expression_and_logs_an_info_advisory() {
        let mut workbench = slow_workbench(Duration::from_millis(20));
        workbench.set_source(0, "a = 1\n".to_string());
        assert!(workbench.run_active());
        assert_eq!(workbench.documents()[0].state(), RunState::Queued);

        assert!(
            !workbench.quick_eval_active("6 * 7"),
            "a busy execution host must refuse the quick eval"
        );
        assert_eq!(
            workbench.documents()[0].state(),
            RunState::Queued,
            "a refused quick eval must not disturb the in-flight run"
        );
        let document = &workbench.documents()[0];
        assert!(
            !document
                .logs
                .iter()
                .any(|line| line.kind == LogKind::Echo),
            "a refused submission has no run to echo"
        );
        assert!(
            document.logs.iter().any(|line| {
                line.kind == LogKind::Info
                    && line.text.contains("Quick eval not submitted")
                    && line.text.contains("kept")
            }),
            "logs were {:?}",
            document.logs
        );
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
    }

    /// RUN-003: after **every** committed statement the assembled view must name the same
    /// statement in its variables, source revision, committed count and log tail.
    ///
    /// The test drains the real `ExecutionHost` stream one event at a time and inspects
    /// [`Workbench::active_view`] immediately after each `StepCommitted`, before the next event
    /// (even a `SubProgress` or another statement's `StepStarted`) is applied. A pane that
    /// advanced on a separate event, or a log line appended before its statement's effects were
    /// in the snapshot, cannot pass: at commit `n` the view must equal exactly the state after
    /// statement `n`, with `last_log_statement() == Some(n)`.
    #[test]
    fn every_committed_step_publishes_one_coherent_snapshot() {
        let mut workbench = fake_workbench();
        let source = "a = 1\nb = a + 1\nprint(\"p\")\nb\n";
        workbench.set_source(0, source.to_string());
        let revision = SourceRevision(workbench.documents()[0].revision);
        assert!(workbench.run_active());

        let labels = ["a = 1", "b = a + 1", "print(\"p\")", "b"];
        let expected_variables = [
            vec![("a".to_string(), "1".to_string())],
            vec![
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "2".to_string()),
            ],
            vec![
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "2".to_string()),
            ],
            vec![
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "2".to_string()),
            ],
        ];
        let expected_results = ["1", "2", "Void", "2"];

        let deadline = Instant::now() + Duration::from_secs(5);
        let mut commits = 0usize;
        loop {
            assert!(
                Instant::now() < deadline,
                "the run never delivered a terminal event"
            );
            let event = workbench
                .host
                .next_event(Duration::from_secs(5))
                .expect("the worker must publish its events");
            let committed = matches!(&event.event, DesktopEvent::StepCommitted { .. });
            let terminal = matches!(
                &event.event,
                DesktopEvent::Finished { .. }
                    | DesktopEvent::Cancelled { .. }
                    | DesktopEvent::Failed { .. }
            );
            assert!(
                workbench.apply_event(&event),
                "the first delivery of {:?} must be applied",
                event.event
            );
            if committed {
                let n = commits + 1;
                let view = workbench.active_view();
                assert_eq!(
                    view.committed_steps, n,
                    "after statement {n} the committed count must be {n}, not {}",
                    view.committed_steps
                );
                assert_eq!(view.committed_statement(), Some(n));
                assert_eq!(
                    view.revision, revision,
                    "the committed snapshot must carry the revision the run was submitted for"
                );
                assert_eq!(view.total, 4);
                assert_eq!(
                    view.variables, expected_variables[n - 1],
                    "variables after statement {n} were {:?}",
                    view.variables
                );
                assert_eq!(view.result, expected_results[n - 1]);
                let state_variables: Vec<(String, String)> = view
                    .state
                    .variables
                    .iter()
                    .map(|variable| (variable.name.clone(), variable.display.clone()))
                    .collect();
                assert_eq!(
                    state_variables, expected_variables[n - 1],
                    "the completion capture must describe the same committed prefix"
                );
                assert_eq!(
                    view.functions.len(),
                    view.state.functions.len(),
                    "the pane list and the completion capture must be the same commit"
                );
                assert!(view.functions.iter().any(|(name, _)| name == "print"));
                assert_eq!(
                    view.output.len(),
                    if n >= 3 { 1 } else { 0 },
                    "print output must appear with its own commit"
                );
                assert_eq!(view.diagnostic, None);
                // The log tail names exactly the statement the variables describe.
                assert_eq!(
                    view.last_committed_log_statement(),
                    Some(n),
                    "the log's last committed line must be statement {n}; logs: {:?}",
                    view.logs
                );
                assert_eq!(
                    view.last_log_statement(),
                    Some(n),
                    "no log line may name a later statement after commit {n}; logs: {:?}",
                    view.logs
                );
                let progress = view
                    .logs
                    .iter()
                    .rev()
                    .find(|line| line.kind == LogKind::Progress)
                    .expect("every commit flushes its progress line");
                assert_eq!(
                    progress.text,
                    format!("statement {n}/4: {}", labels[n - 1])
                );
                assert_eq!(progress.statement, Some(n - 1));
                assert!(
                    view.logs
                        .iter()
                        .all(|line| line.statement.map_or(true, |statement| statement < n)),
                    "after commit {n} the log still holds a later statement's line: {:?}",
                    view.logs
                );
                commits += 1;
            }
            if terminal {
                break;
            }
        }

        assert_eq!(commits, 4, "the script has exactly four statements");
        let view = workbench.active_view();
        assert_eq!(view.run_state, RunState::Finished);
        assert_eq!(view.committed_steps, 4);
        assert_eq!(view.variables, expected_variables[3]);
        assert_eq!(view.result, "2");
        assert_eq!(view.output, vec!["p".to_string()]);
        assert_eq!(view.last_committed_log_statement(), Some(4));
        assert_eq!(view.last_log_statement(), Some(4));
        // The stdout line belongs to statement 3 and follows its progress line.
        let progress_three = view
            .logs
            .iter()
            .position(|line| line.kind == LogKind::Progress && line.statement == Some(2))
            .expect("statement 3's progress line");
        let stdout = view
            .logs
            .iter()
            .position(|line| line.kind == LogKind::Stdout && line.text == "p")
            .expect("the print's stdout line");
        assert!(
            progress_three < stdout,
            "the print's own progress line must precede its output"
        );
    }

    /// RUN-003 plot clause: the plot in the assembled view is the committed snapshot's plot.
    ///
    /// A later run that draws nothing must not leave the previous run's picture beside the new
    /// run's variables and revision.
    #[test]
    fn a_new_run_without_a_plot_clears_the_previous_run_plot_in_the_view() {
        let mut workbench = fake_workbench();
        workbench.set_source(0, "plot(\"trend\", 1, 4, 2)\na = 1\nb = 2\n".to_string());
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        let view = workbench.active_view();
        assert_eq!(view.run_state, RunState::Finished);
        let plot = view
            .plot
            .as_ref()
            .expect("the plot produced by statement 1 must be in the committed snapshot");
        assert_eq!(plot.title, "trend");
        assert!(plot.svg.contains("<svg"));
        assert_eq!(view.last_committed_log_statement(), Some(3));
        assert_eq!(
            workbench.documents()[0]
                .last_plot
                .as_ref()
                .map(|plot| plot.title.as_str()),
            Some("trend")
        );

        workbench.set_source(0, "a = 7\nb = 8\n".to_string());
        let revision = SourceRevision(workbench.documents()[0].revision);
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));
        let view = workbench.active_view();
        assert_eq!(view.run_state, RunState::Finished);
        assert_eq!(view.revision, revision);
        assert_eq!(view.committed_steps, 2);
        assert_eq!(
            view.variables,
            vec![
                ("a".to_string(), "7".to_string()),
                ("b".to_string(), "8".to_string()),
            ]
        );
        assert_eq!(
            view.plot, None,
            "a plot from the previous run would not belong to these variables"
        );
        assert!(
            workbench.documents()[0].last_plot.is_none(),
            "the document's plot field must agree with the committed snapshot"
        );
    }

    /// RUN-003: an error at statement 3 must leave the view on the committed prefix (statements
    /// 1-2) while the diagnostic and the log name statement 3.
    ///
    /// `c = 1 / 0` fails before the assignment inserts `c`, so a half-applied variable from the
    /// failed statement would show up as a `c` in `variables`; the single assembled view is
    /// asserted exactly, including the absence of `c`.
    #[test]
    fn a_failure_at_statement_three_keeps_the_committed_prefix_and_names_statement_three() {
        let mut workbench = fake_workbench();
        workbench.set_source(0, "a = 1\nb = a + 1\nc = 1 / 0\nb\n".to_string());
        let revision = SourceRevision(workbench.documents()[0].revision);
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));

        let view = workbench.active_view();
        assert_eq!(view.run_state, RunState::Failed);
        assert_eq!(
            view.revision, revision,
            "the committed prefix must stay on the submitted revision"
        );
        assert_eq!(view.committed_steps, 2);
        assert_eq!(view.committed_statement(), Some(2));
        assert_eq!(view.total, 4);
        assert_eq!(
            view.variables,
            vec![
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "2".to_string()),
            ],
            "the failed statement must not half-apply a variable"
        );
        assert_eq!(view.variable("c"), None);
        assert_eq!(view.result, "2", "the last committed value is kept");
        assert_eq!(view.output, Vec::<String>::new());
        assert_eq!(
            view.last_committed_log_statement(),
            Some(2),
            "the last committed log line is statement 2; logs: {:?}",
            view.logs
        );
        assert_eq!(
            view.last_log_statement(),
            Some(3),
            "the log narrates the failed attempt at statement 3"
        );
        let diagnostic = view
            .diagnostic
            .as_ref()
            .expect("a failed run must publish the structured diagnostic");
        assert_eq!(diagnostic.statement_index, 2, "0-based statement 3");
        assert_eq!((diagnostic.line, diagnostic.column), (3, 1));
        assert_eq!(diagnostic.message, "Cannot divide by zero.");
        assert!(view.logs.iter().any(|line| {
            line.kind == LogKind::Diagnostic
                && line.text.contains("Cannot divide by zero.")
                && line.text.contains("line 3, column 1")
        }));
        assert!(view.logs.iter().any(|line| {
            line.kind == LogKind::Timing
                && line.text.contains("Run failed after 2 committed statements")
        }));
        // The same view supplies the completion capture, and it agrees: only a and b.
        let names: Vec<&str> = view
            .state
            .variables
            .iter()
            .map(|variable| variable.name.as_str())
            .collect();
        assert_eq!(names, ["a", "b"]);
    }

    /// The fake engine with its third evaluation held open until the run is cancelled.
    ///
    /// The worker's cancellation check happens at statement boundaries; a kernel that observes
    /// the live flag returns an error, which the worker recognises as the requested
    /// cancellation. This makes "cancelled after statement 2" deterministic instead of a race.
    struct CancelAtThirdEngine {
        inner: FakeEngine,
        calls: usize,
        cancel_flag: Option<Arc<AtomicBool>>,
        entered: Arc<AtomicBool>,
    }

    impl CancelAtThirdEngine {
        fn new(entered: Arc<AtomicBool>) -> CancelAtThirdEngine {
            CancelAtThirdEngine {
                inner: FakeEngine::new(),
                calls: 0,
                cancel_flag: None,
                entered,
            }
        }
    }

    impl Evaluator for CancelAtThirdEngine {
        fn eval(&mut self, src: &str) -> Result<String, String> {
            self.calls += 1;
            if self.calls == 3 {
                self.entered.store(true, Ordering::Release);
                let deadline = Instant::now() + Duration::from_secs(5);
                loop {
                    if self
                        .cancel_flag
                        .as_ref()
                        .map(|flag| flag.load(Ordering::Acquire))
                        .unwrap_or(false)
                    {
                        break;
                    }
                    assert!(
                        Instant::now() < deadline,
                        "the test never cancelled the in-flight statement"
                    );
                    std::thread::sleep(Duration::from_millis(1));
                }
                return Err("Cancelled.".to_string());
            }
            self.inner.eval(src)
        }

        fn set_cancel_flag(&mut self, flag: Option<Arc<AtomicBool>>) {
            self.cancel_flag = flag;
        }

        fn set_precision(&mut self, decimals: usize) {
            self.inner.set_precision(decimals);
        }

        fn precision(&self) -> usize {
            self.inner.precision()
        }

        fn variables(&self) -> Vec<(String, String)> {
            self.inner.variables()
        }

        fn functions(&self) -> Vec<(String, Vec<String>)> {
            self.inner.functions()
        }

        fn state(&self) -> Option<EngineState> {
            self.inner.state()
        }

        fn take_output(&mut self) -> Vec<String> {
            self.inner.take_output()
        }

        fn last_plot(&self) -> Option<PlotCapture> {
            self.inner.last_plot()
        }
    }

    /// RUN-003: cancelling after statement 2 must keep exactly the committed prefix in every
    /// field of the assembled view, with no variable from the cancelled statement 3.
    #[test]
    fn cancelling_after_statement_two_keeps_the_same_coherent_prefix() {
        let entered = Arc::new(AtomicBool::new(false));
        let factory_entered = Arc::clone(&entered);
        let mut workbench = Workbench::with_factory(Box::new(move || {
            Box::new(CancelAtThirdEngine::new(Arc::clone(&factory_entered)))
        }));
        workbench.set_source(0, "a = 1\nb = a + 1\nc = 3\nb\n".to_string());
        let revision = SourceRevision(workbench.documents()[0].revision);
        assert!(workbench.run_active());

        // Wait until the worker is inside statement 3 *and* the workbench has applied its
        // StepStarted, so the cancelled attempt is a real third-statement window.
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            workbench.poll();
            if entered.load(Ordering::Acquire)
                && workbench.documents()[0].current_statement == Some(2)
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "statements 1-2 never committed and statement 3 never started"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            workbench.cancel_active(),
            "the in-flight run must accept the cancellation"
        );
        assert!(workbench.wait_until_idle(Duration::from_secs(5)));

        let view = workbench.active_view();
        assert_eq!(view.run_state, RunState::Cancelled);
        assert_eq!(view.revision, revision);
        assert_eq!(view.committed_steps, 2);
        assert_eq!(view.committed_statement(), Some(2));
        assert_eq!(view.total, 4);
        assert_eq!(
            view.variables,
            vec![
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "2".to_string()),
            ],
            "the cancelled statement must not appear in the workspace"
        );
        assert_eq!(view.variable("c"), None);
        assert_eq!(view.result, "2");
        assert_eq!(
            view.last_committed_log_statement(),
            Some(2),
            "the last committed log line is statement 2; logs: {:?}",
            view.logs
        );
        assert_eq!(
            view.last_log_statement(),
            Some(3),
            "the log narrates the cancelled attempt at statement 3"
        );
        assert_eq!(view.diagnostic, None, "a cancellation is not a diagnostic");
        assert!(view.logs.iter().any(|line| {
            line.kind == LogKind::Timing
                && line.text
                    .contains("Run cancelled after 2 statements")
        }));
        let names: Vec<&str> = view
            .state
            .variables
            .iter()
            .map(|variable| variable.name.as_str())
            .collect();
        assert_eq!(names, ["a", "b"]);
    }

    // -----------------------------------------------------------------------------------------
    // DOC-002 / DOC-003: the file lifecycle. Every test drives the same entry points the window
    // calls (`open`, `save`, `save_as`, `new_document`, `close_document`); the native dialogs are
    // the window's job, so the paths are supplied directly here.
    // -----------------------------------------------------------------------------------------

    /// A file in the test scratch directory, unique per test and process.
    fn scratch(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join("lovelace-desktop-workbench-file-tests");
        std::fs::create_dir_all(&directory).expect("scratch directory");
        directory.join(format!("{name}-{}.lv", std::process::id()))
    }

    /// True when `bytes` contains a carriage return only as part of a CRLF pair.
    fn keeps_every_line_crlf(bytes: &[u8]) -> bool {
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'\r' {
                if bytes.get(index + 1) != Some(&b'\n') {
                    return false;
                }
                index += 2;
            } else if bytes[index] == b'\n' {
                return false;
            } else {
                index += 1;
            }
        }
        true
    }

    #[test]
    fn opening_a_bom_crlf_file_records_the_bom_the_style_and_the_file_name() {
        let path = scratch("open-bom");
        let bytes = b"\xEF\xBB\xBFa = 1\r\nb = 2\r\n";
        std::fs::write(&path, bytes).expect("fixture");

        let mut workbench = fake_workbench();
        let index = workbench.open(&path).expect("open");

        let document = workbench.document(index).expect("opened tab");
        assert_eq!(document.path.as_deref(), Some(path.as_path()));
        assert_eq!(document.title, path.file_name().unwrap().to_string_lossy());
        assert!(!document.dirty(), "a freshly opened file is clean");
        assert!(document.bom, "the BOM must be recorded");
        assert_eq!(document.newline, NewlineStyle::CrLf);
        assert_eq!(document.source, "a = 1\r\nb = 2\r\n", "no normalisation");
        assert_eq!(document.encoding_label(), "UTF-8 BOM · CRLF");
        assert_eq!(document.title_with_marker(), document.title);
        assert_eq!(
            workbench.recent_files().first().map(String::as_str),
            Some(path.to_string_lossy().as_ref()),
            "opening records the path as recently used"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_crlf_file_that_is_not_modified_is_saved_byte_identically() {
        let path = scratch("save-crlf");
        let bytes = b"a = 1\r\nb = 2\r\n";
        std::fs::write(&path, bytes).expect("fixture");

        let mut workbench = fake_workbench();
        let index = workbench.open(&path).expect("open");
        assert_eq!(
            workbench.save(index),
            SaveOutcome::Saved {
                path: path.clone(),
                bytes: bytes.len()
            }
        );
        assert_eq!(
            std::fs::read(&path).expect("read back"),
            bytes,
            "saving an unmodified CRLF file must not rewrite its line endings"
        );
        assert!(!workbench.document(index).expect("tab").dirty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_bom_file_keeps_its_bom_when_it_is_saved_unmodified() {
        let path = scratch("save-bom");
        let bytes = b"\xEF\xBB\xBFplot([1], [2], \"bom\")\r\n";
        std::fs::write(&path, bytes).expect("fixture");

        let mut workbench = fake_workbench();
        let index = workbench.open(&path).expect("open");
        assert!(matches!(
            workbench.save(index),
            SaveOutcome::Saved { bytes: written, .. } if written == bytes.len()
        ));
        let written = std::fs::read(&path).expect("read back");
        assert_eq!(written, bytes);
        assert_eq!(&written[..3], &[0xEF, 0xBB, 0xBF], "the BOM is still there");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_mixed_ending_file_round_trips_when_it_is_saved_unmodified() {
        let path = scratch("save-mixed");
        let bytes = b"one\r\ntwo\nthree\r\n";
        std::fs::write(&path, bytes).expect("fixture");

        let mut workbench = fake_workbench();
        let index = workbench.open(&path).expect("open");
        assert_eq!(
            workbench.document(index).expect("tab").newline,
            NewlineStyle::Mixed {
                first: LineEnding::CrLf
            }
        );
        assert!(matches!(workbench.save(index), SaveOutcome::Saved { .. }));
        assert_eq!(
            std::fs::read(&path).expect("read back"),
            bytes,
            "no separator describes this file, so only its own bytes are correct"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_edit_then_save_clears_the_dirty_marker_and_keeps_the_files_endings() {
        let path = scratch("save-dirty");
        std::fs::write(&path, b"a = 1\r\nb = 2\r\n").expect("fixture");

        let mut workbench = fake_workbench();
        let index = workbench.open(&path).expect("open");
        let id = workbench.document(index).expect("tab").id;
        assert!(workbench.dirty_documents().is_empty(), "a fresh tab is clean");

        workbench.set_source(index, "a = 1\r\nb = 3\r\n".to_string());
        let document = workbench.document(index).expect("tab");
        assert!(document.dirty(), "the edit must show");
        assert_eq!(
            document.title_with_marker(),
            format!("{} *", document.title),
            "the dirty marker is part of the title"
        );
        assert_eq!(workbench.dirty_documents(), vec![id]);

        assert!(matches!(workbench.save(index), SaveOutcome::Saved { .. }));
        let document = workbench.document(index).expect("tab");
        assert!(!document.dirty(), "saving clears the marker");
        assert_eq!(document.title_with_marker(), document.title);
        assert!(workbench.dirty_documents().is_empty());
        let written = std::fs::read(&path).expect("read back");
        assert_eq!(written, b"a = 1\r\nb = 3\r\n");
        assert!(
            keeps_every_line_crlf(&written),
            "the edited file is still pure CRLF: {:?}",
            String::from_utf8_lossy(&written)
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_as_updates_the_path_the_title_and_the_recent_list() {
        let path = scratch("save-as");
        let _ = std::fs::remove_file(&path);

        let mut workbench = fake_workbench();
        let index = workbench.new_document();
        let id = workbench.document(index).expect("new tab").id;
        workbench.set_source(index, "x = 41\nx + 1\n".to_string());
        assert_eq!(
            workbench.save(index),
            SaveOutcome::NeedsPath,
            "a document with no file must ask for a path, not invent one"
        );
        assert_eq!(
            workbench.document(index).expect("tab").title_with_marker(),
            "untitled-2.lv *"
        );

        let outcome = workbench.save_as(index, path.clone());
        assert!(
            matches!(
                &outcome,
                SaveOutcome::Saved { path: saved_path, bytes } if *saved_path == path && *bytes == 13
            ),
            "Save As writes the 13-byte buffer to the chosen path: {outcome:?}"
        );
        let document = workbench.document(index).expect("tab");
        assert_eq!(document.path.as_deref(), Some(path.as_path()));
        assert_eq!(document.title, path.file_name().unwrap().to_string_lossy());
        assert_eq!(document.title_with_marker(), document.title);
        assert_eq!(document.newline, NewlineStyle::Lf, "an LF buffer stays LF");
        assert!(!document.bom);
        assert_eq!(
            workbench.recent_files().first().map(String::as_str),
            Some(path.to_string_lossy().as_ref())
        );
        assert_eq!(std::fs::read(&path).expect("read back"), b"x = 41\nx + 1\n");
        assert_eq!(
            workbench.document_index(id),
            Some(index),
            "Save As does not move the tab"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn saving_an_untitled_document_twice_keeps_the_recent_list_deduplicated() {
        let path = scratch("save-twice");
        let _ = std::fs::remove_file(&path);

        let mut workbench = fake_workbench();
        let index = workbench.new_document();
        workbench.save_as(index, path.clone());
        workbench.set_source(index, "y = 1\n".to_string());
        workbench.save(index);

        let recent = workbench.recent_files();
        assert_eq!(
            recent
                .iter()
                .filter(|entry| entry.as_str() == path.to_string_lossy())
                .count(),
            1,
            "the same path is recorded once, newest first: {recent:?}"
        );
        assert_eq!(
            recent.first().map(String::as_str),
            Some(path.to_string_lossy().as_ref())
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_non_utf8_file_is_refused_with_a_clear_message_and_no_tab_is_created() {
        let path = scratch("refuse-non-utf8");
        let bytes = b"a = 1\nb = \xE9\n";
        std::fs::write(&path, bytes).expect("fixture");

        let mut workbench = fake_workbench();
        let before = workbench.documents().len();
        let message = workbench.open(&path).expect_err("a non-UTF-8 file must be refused");
        assert!(
            message.contains("not UTF-8") && message.contains("re-save it as UTF-8"),
            "the message must say what is wrong and what to do: {message}"
        );
        assert!(
            message.contains(&path.display().to_string()),
            "the message must name the file: {message}"
        );
        assert_eq!(
            workbench.documents().len(),
            before,
            "a refused file must not create an empty tab"
        );
        assert_eq!(
            std::fs::read(&path).expect("the file is untouched"),
            bytes,
            "a refused file must never be rewritten"
        );
        let logged = workbench.active_document().logs.iter().any(|line| {
            line.kind == LogKind::Diagnostic && line.text.contains("not UTF-8")
        });
        assert!(logged, "the refusal must be in the log");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn opening_reuses_an_empty_active_tab_and_adds_one_for_a_document_with_content() {
        let path = scratch("reuse-tab");
        std::fs::write(&path, b"a = 1\n").expect("fixture");

        let mut workbench = fake_workbench();
        // The starter tab has content, so Open adds a tab instead of replacing it.
        let opened = workbench.open(&path).expect("open");
        assert_eq!(opened, 2);
        assert_eq!(workbench.documents().len(), 3);
        assert_eq!(workbench.active(), opened);

        // A fresh untitled tab is empty, so the next Open reuses it.
        let index = workbench.new_document();
        assert_eq!(index, 3);
        let opened = workbench.open(&path).expect("open");
        assert_eq!(opened, index, "an untouched empty tab is reused");
        assert_eq!(workbench.documents().len(), 4);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_new_document_is_selected_untitled_and_clean_until_it_is_edited() {
        let mut workbench = fake_workbench();
        let index = workbench.new_document();
        assert_eq!(workbench.active(), index);
        {
            let document = workbench.document(index).expect("new tab");
            assert_eq!(document.title, "untitled-2.lv");
            assert_eq!(document.source, "");
            assert_eq!(document.path, None);
            assert!(!document.dirty(), "a new empty buffer is not dirty");
            assert!(!document.bom);
        }
        assert!(workbench.dirty_documents().is_empty());
        let id = workbench.document(index).expect("new tab").id;

        workbench.set_source(index, "z = 1\n".to_string());
        assert!(workbench.dirty_documents().contains(&id));
        assert_eq!(
            workbench.new_document(),
            index + 1,
            "a second new document gets the next free title"
        );
        assert_eq!(
            workbench.document(index + 1).expect("second new tab").title,
            "untitled-3.lv"
        );
    }

    #[test]
    fn closing_a_tab_selects_a_neighbour_and_the_last_tab_leaves_a_fresh_document() {
        let mut workbench = fake_workbench();
        let first = workbench.document(0).expect("first tab").id;
        let second = workbench.document(1).expect("second tab").id;
        workbench.set_active(1);

        assert!(workbench.close_document(second));
        assert_eq!(workbench.documents().len(), 1);
        assert_eq!(workbench.active(), 0, "the remaining tab becomes active");
        assert_eq!(workbench.document(0).expect("remaining").id, first);
        assert!(!workbench.close_document(second), "it is already closed");

        assert!(workbench.close_document(first));
        assert_eq!(
            workbench.documents().len(),
            1,
            "the window is never tab-less"
        );
        let replacement = workbench.document(0).expect("replacement tab");
        assert_eq!(replacement.title, "untitled.lv");
        assert_eq!(replacement.source, "");
        assert!(!replacement.dirty());
        assert_ne!(replacement.id, first, "the replacement is a new document");
    }

    // -----------------------------------------------------------------------------------------
    // DOC-004 / DOC-005: recent files, session restore and the crash-recovery journal.
    // -----------------------------------------------------------------------------------------

    /// A temporary directory unique to one test in this process.
    fn session_scratch(name: &str) -> PathBuf {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "lovelace-desktop-session-{}-{}-{name}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&directory).expect("scratch directory");
        directory
    }

    #[test]
    fn adopting_the_saved_recent_list_dedupes_prunes_and_keeps_the_order() {
        let directory = session_scratch("recent-adopt");
        let first = directory.join("first.lv");
        let second = directory.join("second.lv");
        let missing = directory.join("missing.lv");
        std::fs::write(&first, "a = 1\n").expect("fixture");
        std::fs::write(&second, "b = 2\n").expect("fixture");
        let _ = std::fs::remove_file(&missing);

        let mut workbench = fake_workbench();
        workbench.set_recent_files(vec![
            second.to_string_lossy().into_owned(),
            "   ".to_string(),
            missing.to_string_lossy().into_owned(),
            first.to_string_lossy().into_owned(),
            second.to_string_lossy().into_owned(),
        ]);
        assert_eq!(
            workbench.recent_files(),
            &[
                second.to_string_lossy().into_owned(),
                first.to_string_lossy().into_owned()
            ],
            "the existing paths keep their order, the missing one is pruned and the duplicate dropped"
        );
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn reopening_a_file_moves_it_to_the_top_of_the_recent_list() {
        let directory = session_scratch("recent-reopen");
        let first = directory.join("first.lv");
        let second = directory.join("second.lv");
        std::fs::write(&first, "a = 1\n").expect("fixture");
        std::fs::write(&second, "b = 2\n").expect("fixture");
        let mut workbench = fake_workbench();

        workbench.open(&first).expect("open the first file");
        workbench.open(&second).expect("open the second file");
        assert_eq!(workbench.recent_files()[0], second.to_string_lossy());
        workbench.open(&first).expect("open the first file again");
        assert_eq!(
            workbench.recent_files()[0],
            first.to_string_lossy(),
            "a re-opened file moves to the top"
        );
        assert_eq!(
            workbench
                .recent_files()
                .iter()
                .filter(|path| path.as_str() == first.to_string_lossy())
                .count(),
            1,
            "and it is not duplicated"
        );
        assert_eq!(workbench.recent_files().len(), 2);
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_session_lists_the_tabs_in_order_with_their_paths_and_the_active_tab() {
        let directory = session_scratch("session-state");
        let first = directory.join("first.lv");
        let second = directory.join("second.lv");
        std::fs::write(&first, "a = 1\n").expect("fixture");
        std::fs::write(&second, "b = 2\n").expect("fixture");
        let mut workbench = fake_workbench();
        let first_index = workbench.open(&first).expect("open the first file");
        let second_index = workbench.open(&second).expect("open the second file");
        workbench.new_document();
        workbench.set_active(second_index);

        let session = workbench.session_state();
        let ids: Vec<u64> = workbench.documents().iter().map(|doc| doc.id.0).collect();
        assert_eq!(
            session.documents.iter().map(|tab| tab.document_id).collect::<Vec<_>>(),
            ids,
            "the session keeps tab order"
        );
        assert_eq!(session.active, second_index, "the active tab is recorded");
        assert_eq!(
            session.documents[first_index].path.as_deref(),
            Some(first.to_string_lossy().as_ref())
        );
        assert_eq!(
            session.documents[second_index].path.as_deref(),
            Some(second.to_string_lossy().as_ref())
        );
        assert_eq!(
            session.documents.last().expect("the new tab").path,
            None,
            "an unsaved buffer keeps its place with no path"
        );
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn restoring_the_last_session_reopens_the_same_tabs_in_order_without_running_or_writing() {
        let directory = session_scratch("session-restore");
        let first = directory.join("first.lv");
        let second = directory.join("second.lv");
        let third = directory.join("third.lv");
        let first_bytes = b"a = 1\n";
        let second_bytes = b"plot([1], [4], \"two\")\r\n";
        let third_bytes = b"z = 3\n";
        std::fs::write(&first, first_bytes).expect("fixture");
        std::fs::write(&second, second_bytes).expect("fixture");
        std::fs::write(&third, third_bytes).expect("fixture");

        let session = {
            let mut workbench = fake_workbench();
            workbench.open(&first).expect("open first");
            workbench.open(&second).expect("open second");
            workbench.open(&third).expect("open third");
            workbench.set_active(3);
            workbench.session_state()
        };

        let mut relaunched = fake_workbench();
        let report = relaunched.restore_session(&session, &[]);
        assert_eq!(report.restored.len(), 3);
        assert_eq!(
            report.skipped.len(),
            2,
            "the two pathless starter tabs have no journal and are not restored: {:?}",
            report.skipped
        );
        assert!(
            report.skipped.iter().all(|skipped| skipped.contains("no recovery record")),
            "{:?}",
            report.skipped
        );
        assert_eq!(
            relaunched
                .documents()
                .iter()
                .map(|doc| doc.title.as_str())
                .collect::<Vec<_>>(),
            vec!["first.lv", "second.lv", "third.lv"],
            "the tabs come back in their original order"
        );
        assert_eq!(relaunched.active(), 1, "the active tab comes back");
        assert_eq!(
            relaunched.active_document().title,
            "second.lv",
            "the active tab is the tab the user had selected, not a shifted neighbour"
        );
        assert_eq!(relaunched.documents()[1].source, "plot([1], [4], \"two\")\r\n");
        assert_eq!(
            relaunched.documents()[1].newline,
            NewlineStyle::CrLf,
            "a restored file keeps its newline style"
        );
        for (path, bytes) in [
            (&first, first_bytes.as_slice()),
            (&second, second_bytes.as_slice()),
            (&third, third_bytes.as_slice()),
        ] {
            assert_eq!(
                std::fs::read(path).expect("read the file back"),
                bytes,
                "restoring must never write a file"
            );
        }
        assert!(
            relaunched.documents().iter().all(|doc| doc.state() == RunState::Idle),
            "restoring must not run anything"
        );
        assert!(
            relaunched
                .documents()
                .iter()
                .flat_map(|doc| doc.logs.iter())
                .all(|line| !line.text.starts_with("Run queued")),
            "no document may have queued a run"
        );
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn restoring_a_session_never_constructs_an_engine() {
        // A factory that counts its own invocations: a run would construct the engine, so a count
        // of zero proves the restore only read files and built buffers.
        let directory = session_scratch("session-no-engine");
        let file = directory.join("quiet.lv");
        std::fs::write(&file, "a = 1\n").expect("fixture");
        let session = {
            let mut workbench = fake_workbench();
            workbench.open(&file).expect("open");
            workbench.set_active(0);
            workbench.session_state()
        };
        let calls = Arc::new(AtomicU64::new(0));
        let counter = calls.clone();
        let mut workbench = Workbench::with_named_factory(
            Box::new(move || {
                counter.fetch_add(1, Ordering::SeqCst);
                Box::new(FakeEngine::new())
            }),
            "counting",
        );
        let report = workbench.restore_session(&session, &[]);
        assert_eq!(report.restored.len(), 1);
        assert_eq!(calls.load(Ordering::SeqCst), 0, "no engine was constructed");
        assert!(workbench.documents().iter().all(|doc| doc.state() == RunState::Idle));
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_pathless_session_tab_restores_only_when_the_journal_covers_it() {
        let directory = session_scratch("session-pathless");
        let session = SessionState::new(
            vec![SessionTab {
                path: None,
                document_id: 5,
            }],
            0,
        );
        let mut workbench = fake_workbench();
        let report = workbench.restore_session(&session, &[]);
        assert!(report.restored.is_empty());
        assert_eq!(report.skipped.len(), 1);
        assert!(report.skipped[0].contains("no recovery record"));
        assert_eq!(
            workbench.documents().len(),
            1,
            "an unrestorable session still leaves the window one empty document"
        );
        assert_eq!(workbench.documents()[0].title, "untitled.lv");
        assert_eq!(workbench.documents()[0].source, "");

        let record = RecoveryRecord {
            document_id: 5,
            revision: 2,
            title: "untitled-2.lv".to_string(),
            path: None,
            bom: false,
            caret: (1, 4),
            selection: Some(((0, 0), (1, 4))),
            source_mtime: None,
            saved_at: SystemTime::UNIX_EPOCH,
            source: "x = 1\ny = 2\n".to_string(),
        };
        let mut workbench = fake_workbench();
        let report = workbench.restore_session(&session, std::slice::from_ref(&record));
        assert_eq!(report.recovered, vec![5], "the journal record was consumed");
        assert_eq!(workbench.documents().len(), 1);
        let document = &workbench.documents()[0];
        assert_eq!(document.source, "x = 1\ny = 2\n");
        assert_eq!(document.title, "untitled-2.lv");
        assert!(document.dirty(), "a recovered unsaved buffer is dirty");
        assert_eq!(document.caret, (1, 4));
        assert_eq!(document.selection, Some(((0, 0), (1, 4))));
        assert_eq!(document.id, DocumentId(5), "the identity the journal is keyed by");
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_journaled_dirty_buffer_produces_a_record_immediately_and_is_throttled_for_a_second() {
        use crate::recovery::{RecoveryJournal, record_path};

        let directory = session_scratch("journal-debounce");
        let recovery_root = directory.join("recovery");
        let mut workbench = fake_workbench();
        let mut journal = RecoveryJournal::new(recovery_root.clone());
        assert_eq!(journal.debounce(), Duration::from_secs(1));
        let document_id = workbench.documents()[0].id.0;
        let path = record_path(&recovery_root, document_id);
        let start = Instant::now();

        workbench.set_source(0, "a = 1\n".to_string());
        workbench.set_caret(0, (0, 5), Some(((0, 2), (0, 5))));
        let report = journal.sync(workbench.documents(), start);
        assert_eq!(report.written, vec![document_id], "the first edit is journaled at once");
        assert!(path.exists());
        let first = RecoveryRecord::parse(&std::fs::read(&path).expect("read")).expect("parses");
        assert_eq!(first.source, "a = 1\n");
        assert_eq!(first.source.as_bytes(), b"a = 1\n", "byte-identical to the buffer");
        assert_eq!(first.caret, (0, 5));
        assert_eq!(first.selection, Some(((0, 2), (0, 5))));
        assert_eq!(first.path, None);

        // A keystroke 500 ms later is inside the window: the record must not be rewritten.
        workbench.set_source(0, "a = 2\n".to_string());
        let report = journal.sync(workbench.documents(), start + Duration::from_millis(500));
        assert!(report.written.is_empty(), "typing must not rewrite per keystroke");
        assert_eq!(
            std::fs::read(&path).expect("read"),
            first.to_bytes(),
            "the record on disk is still the first revision"
        );
        // The window must ask for a frame when the window elapses: the trailing write cannot wait
        // for the next keystroke, or a kill after typing stops would lose more than one window.
        assert_eq!(
            journal.pending_delay(workbench.documents(), start + Duration::from_millis(500)),
            Some(Duration::from_millis(500)),
            "the window knows when the throttled revision becomes due"
        );

        // Once the window has elapsed the newest text is flushed, so a kill loses at most one
        // second of typing.
        let report = journal.sync(workbench.documents(), start + Duration::from_millis(1_000));
        assert_eq!(report.written, vec![document_id]);
        let second = RecoveryRecord::parse(&std::fs::read(&path).expect("read")).expect("parses");
        assert_eq!(second.source, "a = 2\n");
        assert_eq!(
            journal.pending_delay(workbench.documents(), start + Duration::from_millis(1_000)),
            None,
            "nothing is outstanding once the newest revision is on disk"
        );

        // Saving the document makes it clean, and its record is deleted.
        workbench.save_as(0, directory.join("saved.lv"));
        let report = journal.sync(workbench.documents(), start + Duration::from_secs(2));
        assert_eq!(report.deleted, vec![document_id]);
        assert!(!path.exists(), "a clean document has no recovery record");
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_journaled_record_names_the_sources_mtime_at_open() {
        let directory = session_scratch("journal-mtime");
        let file = directory.join("mtime.lv");
        std::fs::write(&file, "a = 1\n").expect("fixture");
        let expected = std::fs::metadata(&file)
            .and_then(|metadata| metadata.modified())
            .expect("the file has an mtime");
        let mut workbench = fake_workbench();
        let index = workbench.open(&file).expect("open");
        let document = workbench.document(index).expect("tab");
        assert_eq!(document.source_mtime, Some(expected), "open records the mtime");
        workbench.set_source(index, "a = 2\n".to_string());
        let record =
            RecoveryRecord::from_document(workbench.document(index).expect("tab"), SystemTime::now());
        assert_eq!(record.source_mtime, Some(expected));
        assert_eq!(record.path, Some(file.clone()));
        assert!(
            String::from_utf8_lossy(&record.to_bytes()).contains("mtime_unix_nanos:"),
            "the record file states the mtime it was opened at"
        );
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_crash_leaves_the_record_and_the_next_launch_restores_the_exact_text_and_original_path() {
        use crate::recovery::{self as recovery_mod, RecoveryJournal};

        let directory = session_scratch("crash");
        let recovery_root = directory.join("recovery");
        let file = directory.join("notes.lv");
        let on_disk = b"a = 1\nb = 2\n";
        std::fs::write(&file, on_disk).expect("fixture");

        let session;
        let record;
        {
            // "The process": a workbench and its journal. It is dropped below without ever calling
            // the clean-exit path, which is exactly what a forced kill leaves behind.
            let mut workbench = fake_workbench();
            let index = workbench.open(&file).expect("open");
            workbench.set_source(index, "a = 1\nb = 3\nc = 4\n".to_string());
            workbench.set_caret(index, (2, 5), None);
            session = workbench.session_state();
            let mut journal = RecoveryJournal::new(recovery_root.clone());
            let report = journal.sync(workbench.documents(), Instant::now());
            assert_eq!(report.written.len(), 1, "the dirty buffer was journaled");
        }
        assert_eq!(
            std::fs::read(&file).expect("their file"),
            on_disk,
            "journaling an unsaved edit never writes the document"
        );

        let scanned = recovery_mod::scan(&recovery_root);
        assert_eq!(scanned.warnings, Vec::<String>::new());
        assert_eq!(scanned.records.len(), 1, "the record survived the simulated crash");
        record = scanned.records[0].clone();
        assert_eq!(record.path, Some(file.clone()), "the record reports the original path");
        assert_eq!(record.title, "notes.lv");
        assert_eq!(
            record.source, "a = 1\nb = 3\nc = 4\n",
            "the record is byte-identical to the buffer the user saw"
        );

        // Next launch: the session restores the file from disk, then the recovery prompt's answer
        // puts the missing edit back into that tab. The file itself is not touched.
        let mut relaunched = fake_workbench();
        let restore = relaunched.restore_session(&session, &scanned.records);
        assert_eq!(restore.restored.len(), 1, "the file's tab is back from disk");
        assert!(
            restore.recovered.is_empty(),
            "a readable file is restored from disk, leaving its record for the prompt"
        );
        let recovered = relaunched.recover(&record);
        assert!(recovered.replaced, "the text goes back into the tab that owns the file");
        let document = relaunched.document(recovered.index).expect("tab");
        assert_eq!(document.source, "a = 1\nb = 3\nc = 4\n", "the exact text is restored");
        assert_eq!(document.path, Some(file.clone()));
        assert_eq!(document.caret, (2, 5));
        assert!(document.dirty());
        assert_eq!(document.title_with_marker(), "notes.lv *");
        assert_eq!(
            std::fs::read(&file).expect("their file"),
            on_disk,
            "recovering must not overwrite the file"
        );
        assert!(document.state() == RunState::Idle, "recovering runs nothing");
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_clean_exit_deletes_only_the_records_the_session_owns() {
        use crate::recovery::{RecoveryJournal, record_path};

        let directory = session_scratch("clean-exit");
        let recovery_root = directory.join("recovery");
        let mut journal = RecoveryJournal::new(recovery_root.clone());
        let mut workbench = fake_workbench();
        let id = workbench.documents()[0].id.0;
        workbench.set_source(0, "a = 1\n".to_string());
        journal.sync(workbench.documents(), Instant::now());
        assert!(record_path(&recovery_root, id).exists());

        // A record nobody in this session owns is left alone: quitting must not destroy an
        // unanswered recovery prompt from an earlier crash.
        let foreign = record_path(&recovery_root, 99);
        std::fs::write(&foreign, b"foreign").expect("foreign record");

        let report = journal.clean_exit(workbench.documents());
        assert_eq!(report.deleted, vec![id]);
        assert!(!record_path(&recovery_root, id).exists());
        assert!(foreign.exists(), "only this session's records are cleaned up");
        let _ = std::fs::remove_dir_all(&directory);
    }
}
